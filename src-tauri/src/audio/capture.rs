//! 캡처 → 정지 → 파일 확정 (PRODUCT-SPEC §6.1의 `file finalization` · ADR-0003 §5.5).
//!
//! 이 모듈도 [`super::devices`]와 같은 방식으로 갈라져 있고, **그 경계가 이 파일의 목적이다.**
//!
//! ```text
//! SampleSource   실제 장치가 있어야 답할 수 있는 부분 (구현은 system_capture 하나)
//! 그 밖의 전부   값과 임시 디렉터리만 있으면 검증되는 부분
//! ```
//!
//! 그래서 **파일이 만들어지고 확정되는 경로 전체가 마이크 없이 테스트된다** — 가짜 샘플
//! 소스를 [`SampleSource`] 자리에 넣으면 나머지는 실제 코드가 그대로 지나간다 (§18).
//!
//! **start / stop 한 쌍이 전부다.** pause/resume · 재시작 영속성 · 재생 · Recording 레코드
//! 생성은 여기에 없다 — 전부 Phase 2B다 (ADR-0003 §14).
//!
//! ## 만들어지는 포맷 — 확인된 것과 확인되지 않은 것
//!
//! **확인된 것**: 이 코드는 `hound`로 **16-bit PCM WAV(RIFF)** 를 쓴다. 샘플레이트와 채널
//! 수는 지어내지 않고 **열린 장치가 알려준 값을 그대로** 쓴다 ([`CaptureFormat`]).
//!
//! **UNVERIFIED**: 실제 장치가 어떤 샘플레이트·채널을 주는지, 그래서 만들어진 파일이
//! whisper 입력 요구(16kHz mono 16-bit · §14.4)와 맞는지는 이 코드가 알지 못한다.
//! 리샘플링도 다운믹스도 여기에 없다. **그 판정은 사람의 장치 검증 몫이다**
//! (`docs/ADR-0003-recording-engine.md` §12 항목 7).

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread::JoinHandle;

use crate::domain::{Failure, FailureKind};

/// 이 경로가 만드는 컨테이너. 코드가 실제로 쓰는 것이며 추정이 아니다.
pub const CONTAINER: &str = "WAV";

/// 이 경로가 쓰는 샘플 하나의 비트 수. 샘플을 `i16`으로 쓰므로 16이다.
pub const BITS_PER_SAMPLE: u16 = 16;

/// 출력 파일 확장자.
const EXTENSION: &str = "wav";

/// 같은 이름을 몇 번까지 비켜 볼 것인가. 여기까지 전부 차 있으면 이름을 만들지 못한 것이다.
const MAX_PATH_ATTEMPTS: u32 = 1_000;

/// 아직 파일에 쓰지 못한 샘플 덩어리를 몇 개까지 쌓아 둘 것인가.
///
/// 무제한 큐를 쓰면 디스크가 느릴 때 메모리가 대신 자란다. 한도를 두고, **넘치면 조용히
/// 버리지 않고 실패로 알린다** — 유실을 숨기는 것이 이 제품에서 가장 나쁜 결과다 (R-005).
pub const SAMPLE_QUEUE_CAPACITY: usize = 256;

/// 캡처가 실제로 만들어 낸 형식.
///
/// **장치가 알려준 값을 그대로 담는다.** 기대값으로 덮어쓰지 않는다 — 기대와 실제가 다르면
/// 그것이 바로 사람이 확인해야 할 사실이기 때문이다 (ADR-0003 §12 항목 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureFormat {
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl CaptureFormat {
    /// 이 경로가 쓰는 16-bit PCM 형식. 샘플레이트와 채널 수는 장치가 알려준 값이다.
    pub const fn pcm_16bit(sample_rate_hz: u32, channels: u16) -> Self {
        Self {
            sample_rate_hz,
            channels,
            bits_per_sample: BITS_PER_SAMPLE,
        }
    }

    /// 컨테이너 이름.
    pub const fn container(&self) -> &'static str {
        CONTAINER
    }

    /// 사람이 읽는 형식 문장. **네 가지가 모두 들어간다** —
    /// 샘플레이트 · 채널 수 · 비트 심도 · 컨테이너.
    ///
    /// 사람이 실제 장치 검증에서 §14.4의 whisper 입력 요구와 대조하는 값이 이 문자열이다.
    pub fn describe(&self) -> String {
        format!(
            "{} Hz · {} · {}-bit PCM · {}",
            self.sample_rate_hz,
            self.channel_label(),
            self.bits_per_sample,
            self.container(),
        )
    }

    /// 채널 수를 사람이 읽는 말로. 모르는 수는 지어내지 않고 그대로 적는다.
    fn channel_label(&self) -> String {
        match self.channels {
            1 => "mono".to_string(),
            2 => "stereo".to_string(),
            other => format!("{other}ch"),
        }
    }
}

/// 정지 후에 사용자에게 돌려주는 보고 값.
///
/// Phase 2A의 성공 기준이 그대로 필드가 된다 —
/// **장치 이름 · 출력 경로 · 포맷 · 파일 크기(byte)**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureReport {
    /// 실제로 열린 장치의 이름. 고른 이름이 아니라 **열린 장치가 알려준 이름**이다.
    pub device_label: String,
    /// 확정된 파일의 경로.
    pub output_path: PathBuf,
    /// 실제로 만들어진 형식.
    pub format: CaptureFormat,
    /// 확정된 파일의 크기(byte). 파일시스템에서 읽은 값이며 계산한 값이 아니다.
    pub byte_size: u64,
}

/// 열린 캡처 하나. [`SampleSource`]가 돌려주는 값이다.
pub struct OpenCapture {
    /// 실제로 열린 장치의 이름.
    pub device_label: String,
    /// 장치가 알려준 형식.
    pub format: CaptureFormat,
    /// 장치를 멈춘다. **캡처가 도중에 끊겼다면 그 사실을 여기서 돌려준다.**
    pub stop: Box<dyn FnOnce() -> Result<(), Failure> + Send>,
}

/// 실제 장치에서 샘플을 받는 경계.
///
/// 실제 구현은 [`crate::audio::system_capture`] 하나뿐이고, 테스트는 자신의 구현을 넣는다 —
/// **자동 테스트가 실제 마이크나 마이크 권한의 존재를 전제하지 않는 이유가 이 trait이다** (§18).
pub trait SampleSource {
    /// `device_key`가 가리키는 장치를 열고 캡처를 시작한다.
    ///
    /// 들어오는 샘플은 `samples`로 보낸다. 큐가 가득 차면 **버리지 말고** 정지 시점에
    /// 실패로 알린다 ([`OpenCapture::stop`]).
    ///
    /// 장치를 열지 못하면 실패다. 이때 파일은 아직 만들어지지 않았다.
    fn open(&self, device_key: &str, samples: SyncSender<Vec<i16>>) -> Result<OpenCapture, Failure>;
}

/// 진행 중인 캡처 하나. [`Self::stop`]이 파일을 확정하고 보고 값을 만든다.
pub struct ActiveCapture {
    device_label: String,
    format: CaptureFormat,
    output_path: PathBuf,
    stop_device: Box<dyn FnOnce() -> Result<(), Failure> + Send>,
    /// 파일에 쓰는 쪽으로 가는 통로. 이것을 닫는 것이 "더 이상 샘플이 없다"는 신호다.
    samples: SyncSender<Vec<i16>>,
    writer: JoinHandle<Result<(), Failure>>,
}

impl ActiveCapture {
    /// 캡처를 멈추고 파일을 확정한 뒤 보고 값을 만든다.
    ///
    /// 순서가 중요하다 — **장치를 먼저 멈추고 · 통로를 닫고 · 쓰는 쪽이 끝나기를 기다린 뒤**
    /// 크기를 읽는다. 그래야 보고된 크기가 확정된 파일의 크기다.
    pub fn stop(self) -> Result<CaptureReport, Failure> {
        let Self {
            device_label,
            format,
            output_path,
            stop_device,
            samples,
            writer,
        } = self;

        // 1. 장치를 멈춘다. 더 이상 새 샘플이 들어오지 않는다.
        let stopped = stop_device();

        // 2. 통로를 닫는다. 쓰는 쪽은 남은 것을 전부 쓰고 스스로 끝난다.
        drop(samples);

        // 3. 파일이 확정될 때까지 기다린다. 쓰는 쪽이 죽었더라도 앱은 죽지 않는다.
        let written = match writer.join() {
            Ok(result) => result,
            Err(_) => Err(Failure::permanent(
                FailureKind::Storage,
                "녹음 파일을 확정하지 못했다. 저장된 파일이 온전하지 않을 수 있다.",
            )
            .with_source_data_at_risk()),
        };
        written?;

        // 4. 끊긴 캡처도 그때까지의 소리는 파일에 남아 확정됐다. 어디에 남았는지 함께 알린다 —
        //    실패했다는 이유로 이미 녹음된 것을 사용자에게서 숨기지 않는다 (INV-1).
        if let Err(failure) = stopped {
            return Err(Failure {
                message: format!(
                    "{} 그때까지 녹음된 파일은 남아 있다: {}",
                    failure.message,
                    output_path.display()
                ),
                ..failure
            });
        }

        let byte_size = file_size(&output_path)?;
        Ok(CaptureReport {
            device_label,
            output_path,
            format,
            byte_size,
        })
    }
}

/// 장치를 열고 캡처를 시작한다.
///
/// `directory`는 호출자가 이미 준비해 둔 디렉터리다 — **이 함수는 경로를 스스로 정하지 않는다.**
/// 오디오 파일이 어디에 놓이는지는 [`crate::platform::app_data_dir`]가 결정한다 (INV-10).
///
/// 장치를 먼저 열고 그다음에 파일을 만든다. 그래서 **장치를 열지 못하면 파일이 생기지 않는다.**
pub fn start(
    source: &dyn SampleSource,
    device_key: &str,
    directory: &Path,
    stem: &str,
) -> Result<ActiveCapture, Failure> {
    let output_path = output_path(directory, stem)?;
    let (sender, receiver) = sync_channel::<Vec<i16>>(SAMPLE_QUEUE_CAPACITY);

    let open = source.open(device_key, sender.clone())?;

    let file = match WavFile::create(&output_path, open.format) {
        Ok(file) => file,
        Err(failure) => {
            // 장치는 이미 열려 있다. 파일을 만들지 못했으면 장치를 도로 닫는다.
            let _ = (open.stop)();
            return Err(failure);
        }
    };

    let writer = match std::thread::Builder::new()
        .name("molt-note-capture-writer".to_string())
        .spawn(move || drain(receiver, file))
    {
        Ok(handle) => handle,
        Err(error) => {
            let _ = (open.stop)();
            return Err(Failure::retryable(
                FailureKind::Storage,
                "녹음을 시작하지 못했다. 파일에 쓸 준비를 끝내지 못했다.",
            )
            .with_detail(error));
        }
    };

    Ok(ActiveCapture {
        device_label: open.device_label,
        format: open.format,
        output_path,
        stop_device: open.stop,
        samples: sender,
        writer,
    })
}

/// 출력 파일 경로를 정한다. **같은 입력이면 같은 값이고, 이미 있는 파일을 가리키지 않는다.**
///
/// 이름이 겹치면 뒤에 번호를 붙여 비켜 간다. 덮어쓰기는 하지 않는다 — 앞선 녹음을
/// 새 녹음이 지우는 경로를 만들지 않는다 (INV-1 · INV-4).
pub fn output_path(directory: &Path, stem: &str) -> Result<PathBuf, Failure> {
    for attempt in 0..MAX_PATH_ATTEMPTS {
        let name = match attempt {
            0 => format!("{stem}.{EXTENSION}"),
            taken => format!("{stem}-{}.{EXTENSION}", taken + 1),
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(Failure::permanent(
        FailureKind::Storage,
        "녹음 파일을 둘 이름을 찾지 못했다. 같은 이름의 파일이 이미 너무 많다.",
    ))
}

/// 출력 파일 이름의 뿌리.
///
/// 시각을 **값으로 받는다.** 그래서 이름을 만드는 규칙이 시계 없이 그대로 검증된다.
pub fn file_stem(unix_seconds: u64) -> String {
    format!("capture-{unix_seconds}")
}

/// 확정된 파일의 크기(byte). **파일시스템에서 읽는다** — 쓴 샘플 수로 계산하지 않는다.
///
/// 계산한 값은 "파일이 실제로 그만큼 쓰였다"를 말해 주지 못한다. 사람이 확인해야 하는 것은
/// 후자다 (ADR-0003 §12 항목 5).
pub fn file_size(path: &Path) -> Result<u64, Failure> {
    fs::metadata(path).map(|meta| meta.len()).map_err(|error| {
        Failure::retryable(
            FailureKind::Storage,
            format!("녹음 파일을 확인하지 못했다: {}", path.display()),
        )
        .with_detail(error)
    })
}

/// 들어오는 샘플을 파일에 쓰고, 통로가 닫히면 파일을 확정한다.
///
/// **이 함수 안에 panic 경로가 없다.** 여기서 죽으면 파일이 확정되지 않은 채 남는다.
fn drain(receiver: Receiver<Vec<i16>>, mut file: WavFile) -> Result<(), Failure> {
    for chunk in receiver {
        file.write(&chunk)?;
    }
    file.finish()
}

/// 쓰는 중인 WAV 파일 하나.
///
/// WAV(RIFF)는 길이를 헤더에 갖는 포맷이므로 **정상 종료해야 재생 가능한 파일이 된다**
/// (ADR-0003 §5.5). 그 종료가 [`Self::finish`]다.
struct WavFile {
    writer: hound::WavWriter<BufWriter<fs::File>>,
}

impl WavFile {
    /// 파일을 만들고 헤더를 쓴다.
    fn create(path: &Path, format: CaptureFormat) -> Result<Self, Failure> {
        let spec = hound::WavSpec {
            channels: format.channels,
            sample_rate: format.sample_rate_hz,
            bits_per_sample: format.bits_per_sample,
            sample_format: hound::SampleFormat::Int,
        };

        hound::WavWriter::create(path, spec)
            .map(|writer| Self { writer })
            .map_err(|error| {
                Failure::retryable(
                    FailureKind::Storage,
                    format!("녹음 파일을 만들지 못했다: {}", path.display()),
                )
                .with_detail(error)
            })
    }

    /// 샘플 덩어리 하나를 쓴다.
    fn write(&mut self, samples: &[i16]) -> Result<(), Failure> {
        for sample in samples {
            self.writer.write_sample(*sample).map_err(|error| {
                Failure::retryable(FailureKind::Storage, "녹음을 파일에 쓰지 못했다.")
                    .with_detail(error)
                    .with_source_data_at_risk()
            })?;
        }
        Ok(())
    }

    /// 파일을 확정한다. 여기까지 성공해야 재생 가능한 파일이다.
    fn finish(self) -> Result<(), Failure> {
        self.writer.finalize().map_err(|error| {
            Failure::retryable(
                FailureKind::Storage,
                "녹음 파일을 확정하지 못했다. 저장된 파일이 온전하지 않을 수 있다.",
            )
            .with_detail(error)
            .with_source_data_at_risk()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let unique = format!(
                "molt-note-capture-{}-{}-{}",
                label,
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_same_directory_and_stem_always_give_the_same_path() {
        let temp = TempDir::new("deterministic");

        let first = output_path(temp.path(), "capture-42").expect("경로를 정할 수 있어야 한다");
        let second = output_path(temp.path(), "capture-42").expect("경로를 정할 수 있어야 한다");

        assert_eq!(first, second);
        assert_eq!(first, temp.path().join("capture-42.wav"));
        assert_eq!(first.parent(), Some(temp.path()));
    }

    #[test]
    fn a_path_is_never_one_that_already_holds_a_file() {
        // 같은 초에 두 번 녹음해도 앞선 녹음을 덮어쓰지 않는다.
        let temp = TempDir::new("no-overwrite");

        let first = output_path(temp.path(), "capture-42").expect("첫 경로");
        fs::write(&first, "앞선 녹음").expect("사전 조건: 앞선 녹음을 둔다");

        let second = output_path(temp.path(), "capture-42").expect("두 번째 경로");

        assert_ne!(second, first);
        assert!(!second.exists(), "이미 있는 파일을 가리키지 않는다");
        assert_eq!(
            fs::read_to_string(&first).expect("앞선 녹음이 남아 있어야 한다"),
            "앞선 녹음"
        );
    }

    #[test]
    fn the_path_keeps_stepping_aside_while_names_are_taken() {
        let temp = TempDir::new("stepping-aside");
        let mut chosen = Vec::new();

        for _ in 0..4 {
            let path = output_path(temp.path(), "capture-42").expect("경로를 정할 수 있어야 한다");
            assert!(!chosen.contains(&path), "같은 경로를 두 번 주지 않는다");
            fs::write(&path, "녹음").expect("자리를 차지한다");
            chosen.push(path);
        }

        assert_eq!(chosen.len(), 4);
        for path in &chosen {
            assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("wav"));
        }
    }

    #[test]
    fn a_different_stem_gives_a_different_path() {
        let temp = TempDir::new("distinct-stem");

        let one = output_path(temp.path(), &file_stem(1)).expect("경로 하나");
        let other = output_path(temp.path(), &file_stem(2)).expect("다른 경로");

        assert_ne!(one, other);
    }

    #[test]
    fn the_file_stem_is_made_from_the_time_it_is_given() {
        assert_eq!(file_stem(0), "capture-0");
        assert_eq!(file_stem(1_772_000_000), "capture-1772000000");
        assert_eq!(file_stem(7), file_stem(7), "같은 시각이면 같은 이름이다");
        assert_ne!(file_stem(7), file_stem(8));
    }

    #[test]
    fn the_format_sentence_carries_all_four_facts() {
        let format = CaptureFormat::pcm_16bit(16_000, 1);

        let sentence = format.describe();

        assert!(sentence.contains("16000"), "샘플레이트: {sentence}");
        assert!(sentence.contains("mono"), "채널 수: {sentence}");
        assert!(sentence.contains("16-bit"), "비트 심도: {sentence}");
        assert!(sentence.contains(CONTAINER), "컨테이너: {sentence}");
        assert_eq!(format.bits_per_sample, BITS_PER_SAMPLE);
        assert_eq!(format.container(), "WAV");
    }

    #[test]
    fn the_format_sentence_reports_the_channel_count_it_was_given() {
        // 기대값으로 덮어쓰지 않는다 — 다르면 그것이 사람이 확인해야 할 사실이다.
        assert!(CaptureFormat::pcm_16bit(48_000, 2).describe().contains("stereo"));
        assert!(CaptureFormat::pcm_16bit(48_000, 4).describe().contains("4ch"));
        assert!(CaptureFormat::pcm_16bit(44_100, 1).describe().contains("44100"));
    }

    #[test]
    fn the_reported_size_is_read_from_the_file_that_was_written() {
        let temp = TempDir::new("size");
        let path = temp.path().join("written.wav");
        fs::write(&path, [0_u8; 137]).expect("사전 조건: 파일을 쓴다");

        assert_eq!(file_size(&path).expect("크기를 읽을 수 있어야 한다"), 137);
    }

    #[test]
    fn a_missing_file_becomes_a_failure_the_user_can_read() {
        let temp = TempDir::new("missing");
        let path = temp.path().join("absent.wav");

        let failure = file_size(&path).expect_err("없는 파일의 크기는 읽을 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(
            failure.message.contains(&path.display().to_string()),
            "어느 파일인지 보인다: {}",
            failure.message
        );
        assert!(failure.detail.is_some(), "기술적 원인이 함께 실린다");
        assert!(failure.retryable);
    }

    #[test]
    fn a_wav_file_that_is_finalized_can_report_a_size_larger_than_its_header() {
        // 파일 확정 경로만 따로 지나 본다. 하드웨어도 스레드도 필요하지 않다.
        let temp = TempDir::new("finalize");
        let path = temp.path().join("finalized.wav");
        let format = CaptureFormat::pcm_16bit(16_000, 1);

        let mut file = WavFile::create(&path, format).expect("파일을 만들 수 있어야 한다");
        file.write(&[0, 1, -1, 32_767, -32_768]).expect("샘플을 쓸 수 있어야 한다");
        file.finish().expect("확정할 수 있어야 한다");

        let size = file_size(&path).expect("크기를 읽을 수 있어야 한다");
        assert!(size > 10, "헤더만 있는 파일이 아니다: {size} byte");
        assert!(
            size >= 5 * u64::from(BITS_PER_SAMPLE / 8),
            "쓴 샘플이 파일에 들어 있다: {size} byte"
        );
    }

    #[test]
    fn a_file_that_cannot_be_created_becomes_a_failure_instead_of_a_panic() {
        // 디렉터리가 없다. 존재하지 않는 자리에 파일을 만들 수는 없다.
        let temp = TempDir::new("uncreatable");
        let path = temp.path().join("없는-디렉터리").join("output.wav");

        let failure = WavFile::create(&path, CaptureFormat::pcm_16bit(16_000, 1))
            .err()
            .expect("파일을 만들 수 없어야 한다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(!failure.message.is_empty(), "보여줄 문장이 있어야 한다");
        assert!(failure.source_data_safe, "아무것도 쓰지 못했다");
        assert!(failure.detail.is_some());
    }
}
