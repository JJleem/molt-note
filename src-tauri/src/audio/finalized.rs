//! 확정된 녹음 파일을 확인한다 (PRODUCT-SPEC §6의 R-002).
//!
//! **Stop의 성공은 "API가 resolve됐다"가 아니라 "파일이 확정됐다"를 뜻한다.** 그러려면
//! 최소한 네 가지가 성립해야 한다.
//!
//! ```text
//! 1. 파일 경로가 실제로 존재한다          ← 이 모듈
//! 2. 파일 크기가 유효 최소치를 넘는다      ← 이 모듈
//! 3. 포맷을 알고 있다                    ← 이 모듈
//! 4. Recording 메타데이터가 영속화됐다     ← crate::commands
//! ```
//!
//! 앞의 셋이 여기 있다. 세 번째는 "우리가 어떤 형식으로 썼다고 믿는다"가 아니라 **확정된
//! 파일을 다시 열어 그 형식이 맞는지 확인한다** — 헤더를 쓰지 못한 파일과 온전한 파일은
//! 크기만으로는 구분되지 않기 때문이다 (ADR-0003 §5.5).
//!
//! ## 이 모듈은 파일을 지우지 않는다
//!
//! 확인에 실패해도 **파일은 그 자리에 그대로 남는다** (INV-4 · R-004). 확인 실패는
//! "이 파일은 온전하지 않다"는 사실을 사용자에게 알리는 일이며, 그 사실을 파일을 없애는
//! 방식으로 처리하지 않는다. 그래서 이 모듈의 모든 실패 문장은 **어느 파일인지**를 담는다 —
//! 지우지 않는 파일은 사용자가 찾을 수 있어야 한다.
//!
//! 정책 전체(파일과 레코드 중 무엇을 먼저 쓰는가 · 어긋난 두 상태를 어떻게 다루는가)는
//! `docs/ADR-0004-recording-session-lifecycle.md`에 있다.

use std::fs;
use std::path::{Path, PathBuf};

use crate::audio::capture::{CaptureFormat, EXTENSION};
use crate::domain::{Failure, FailureKind};

/// 확정된 녹음 파일이 넘어야 하는 최소 크기(byte).
///
/// 16-bit PCM WAV의 표준 RIFF 헤더는 44 byte다. 그보다 크지 않으면 소리가 한 프레임도
/// 들어 있지 않다는 뜻이므로 이 값이 "유효 최소치"의 경계다.
///
/// **크기만으로 판정하지는 않는다.** 헤더 길이는 writer가 정하는 값이므로 여기에만 기대면
/// 추정이 된다. [`verify`]는 파일을 다시 열어 실제로 담긴 프레임 수까지 확인한다.
pub const MIN_FINALIZED_BYTES: u64 = 45;

/// 확인을 마친 녹음 파일에 대한 사실.
///
/// **여기 있는 값은 전부 파일시스템과 파일 자신에게서 읽은 것이다.** 캡처가 그랬을 것이라고
/// 기대한 값이 아니다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAudio {
    /// 확인된 파일의 경로.
    pub path: PathBuf,
    /// 레코드에 남길 형식 식별자(`wav`). 파일을 다시 읽어 확인한 뒤에만 붙는다.
    pub format: &'static str,
    /// 파일시스템에서 읽은 크기(byte).
    pub byte_size: u64,
    /// 파일에 실제로 들어 있는 샘플 프레임 수(채널당). 0이면 소리가 없다.
    pub frames: u64,
}

/// 확정된 파일이 정말 쓸 수 있는 녹음인지 확인한다 (R-002).
///
/// `expected`는 캡처가 만들었다고 보고한 형식이다. 파일에서 읽은 형식이 그것과 다르면
/// **다르다는 사실 자체가 실패다** — 어느 쪽이 맞는지 이 함수가 고르지 않는다.
///
/// 어떤 실패 경로도 파일을 지우지 않으며, 모든 실패 문장은 파일의 자리를 담는다 (INV-4).
pub fn verify(path: &Path, expected: CaptureFormat) -> Result<VerifiedAudio, Failure> {
    let metadata = fs::metadata(path).map_err(|error| {
        // 확정했다고 보고된 파일이 없다. 성공처럼 보이는 실패이므로 여기서 걸러진다.
        broken(path, "녹음 파일을 찾지 못했다").with_detail(error)
    })?;
    if !metadata.is_file() {
        return Err(broken(path, "녹음 파일이 있어야 할 자리에 파일이 없다"));
    }

    let byte_size = metadata.len();
    if byte_size < MIN_FINALIZED_BYTES {
        return Err(broken(
            path,
            format!("녹음 파일에 소리가 들어 있지 않다({byte_size} byte)"),
        ));
    }

    // 다시 열어 본다. 열리지 않으면 우리는 이 파일의 포맷을 아는 것이 아니다.
    let reader = hound::WavReader::open(path)
        .map_err(|error| broken(path, "녹음 파일을 다시 읽지 못했다").with_detail(error))?;

    let spec = reader.spec();
    if spec.channels != expected.channels
        || spec.sample_rate != expected.sample_rate_hz
        || spec.bits_per_sample != expected.bits_per_sample
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err(
            broken(path, "녹음 파일의 형식이 녹음한 형식과 다르다").with_detail(format!(
                "expected={} actual={}",
                expected.describe(),
                describe(spec)
            )),
        );
    }

    let frames = u64::from(reader.duration());
    if frames == 0 {
        return Err(broken(path, "녹음된 소리가 없다"));
    }

    Ok(VerifiedAudio {
        path: path.to_path_buf(),
        format: EXTENSION,
        byte_size,
        frames,
    })
}

/// 레코드가 가리키는 오디오 파일이 지금 그 자리에 있는가.
///
/// **확인만 한다.** 없다고 해서 레코드를 지우거나 고치지 않고, 있다고 해서 파일을 건드리지도
/// 않는다 (INV-3 · INV-4). "레코드는 있는데 audio가 없다"를 감지하는 수단이 이것이다.
pub fn audio_is_present(path: &str) -> bool {
    Path::new(path).is_file()
}

/// 확정에 실패한 파일 하나에 대한 실패.
///
/// **문장에 언제나 경로가 들어간다.** 이 파일은 지워지지 않으므로 사용자가 찾을 수 있어야
/// 한다. 재시도 가능하지 않다 — 같은 파일을 다시 확인해도 답은 같고, 이미 끝난 녹음을
/// 다시 정지할 수도 없다.
fn broken(path: &Path, message: impl std::fmt::Display) -> Failure {
    Failure::permanent(
        FailureKind::Storage,
        format!("{message}: {}", path.display()),
    )
    // 방금 녹음한 것을 신뢰할 수 없다는 뜻이다 (§13의 "원본 데이터는 안전한가").
    .with_source_data_at_risk()
}

/// 파일에서 읽은 형식을 사람이 읽는 문장으로. 실패의 기술적 원인에만 쓴다.
fn describe(spec: hound::WavSpec) -> String {
    let sample_format = match spec.sample_format {
        hound::SampleFormat::Int => "int",
        hound::SampleFormat::Float => "float",
    };
    format!(
        "{} Hz · {}ch · {}-bit {}",
        spec.sample_rate, spec.channels, spec.bits_per_sample, sample_format
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 이 테스트들이 쓰는 형식. 캡처가 보고하는 값과 같은 모양이다.
    const FORMAT: CaptureFormat = CaptureFormat::pcm_16bit(16_000, 1);

    /// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let unique = format!(
                "molt-note-finalized-{}-{}-{}",
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

    /// 실제 WAV 파일 하나를 만든다. 샘플 수가 0이면 헤더만 있는 파일이 된다.
    fn wav_file(directory: &Path, name: &str, format: CaptureFormat, samples: usize) -> PathBuf {
        let path = directory.join(name);
        let spec = hound::WavSpec {
            channels: format.channels,
            sample_rate: format.sample_rate_hz,
            bits_per_sample: format.bits_per_sample,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("사전 조건: 파일을 만든다");
        for index in 0..samples {
            writer
                .write_sample(index as i16)
                .expect("사전 조건: 샘플을 쓴다");
        }
        writer.finalize().expect("사전 조건: 파일을 확정한다");
        path
    }

    #[test]
    fn a_finalized_file_that_holds_sound_is_verified_with_facts_read_from_the_file() {
        let temp = TempDir::new("good");
        let path = wav_file(temp.path(), "good.wav", FORMAT, 512);

        let verified = verify(&path, FORMAT).expect("확정된 파일은 확인을 통과해야 한다");

        assert_eq!(verified.path, path);
        assert_eq!(verified.format, "wav", "레코드에 남는 형식 식별자다");
        assert_eq!(verified.frames, 512, "파일에서 읽은 프레임 수다");
        assert_eq!(
            verified.byte_size,
            fs::metadata(&path).expect("파일을 읽을 수 있어야 한다").len(),
            "크기는 파일시스템에서 읽은 값이다"
        );
        assert!(verified.byte_size > MIN_FINALIZED_BYTES);
    }

    #[test]
    fn a_path_that_holds_no_file_is_a_failure_that_names_the_path() {
        // 1번 조건: 파일 경로가 실제로 존재한다.
        let temp = TempDir::new("missing");
        let path = temp.path().join("없는-녹음.wav");

        let failure = verify(&path, FORMAT).expect_err("없는 파일은 확인을 통과할 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(
            failure.message.contains(&path.display().to_string()),
            "어느 파일인지 보인다: {}",
            failure.message
        );
        assert!(!failure.retryable);
    }

    #[test]
    fn a_directory_in_the_place_of_the_file_is_not_mistaken_for_a_recording() {
        let temp = TempDir::new("directory");
        let path = temp.path().join("directory.wav");
        fs::create_dir_all(&path).expect("사전 조건: 디렉터리를 둔다");

        let failure = verify(&path, FORMAT).expect_err("디렉터리는 녹음 파일이 아니다");

        assert!(failure.message.contains(&path.display().to_string()));
        assert!(path.is_dir(), "확인에 실패해도 있던 것을 지우지 않는다");
    }

    #[test]
    fn a_file_smaller_than_the_minimum_is_refused_and_left_where_it_is() {
        // 2번 조건: 파일 크기가 유효 최소치를 넘는다.
        let temp = TempDir::new("too-small");
        let path = temp.path().join("truncated.wav");
        fs::write(&path, [0_u8; 10]).expect("사전 조건: 잘린 파일을 둔다");

        let failure = verify(&path, FORMAT).expect_err("헤더도 못 되는 파일은 녹음이 아니다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(!failure.source_data_safe, "방금 녹음한 것을 신뢰할 수 없다");
        assert!(failure.message.contains(&path.display().to_string()));
        assert!(path.is_file(), "실패해도 파일을 지우지 않는다 (INV-4)");
        assert_eq!(
            fs::metadata(&path).expect("파일이 남아 있어야 한다").len(),
            10
        );
    }

    #[test]
    fn a_file_with_a_header_but_no_sound_is_refused() {
        // 크기 검사를 통과하더라도 소리가 없으면 녹음이 아니다.
        let temp = TempDir::new("silent");
        let path = wav_file(temp.path(), "empty.wav", FORMAT, 0);

        let failure = verify(&path, FORMAT).expect_err("소리가 없는 파일은 녹음이 아니다");

        assert!(failure.message.contains(&path.display().to_string()));
        assert!(path.is_file(), "실패해도 파일을 지우지 않는다 (INV-4)");
    }

    #[test]
    fn a_file_that_cannot_be_read_back_is_refused_rather_than_assumed_to_be_wav() {
        // 3번 조건: 포맷을 알고 있다. 크기만 큰 파일을 WAV로 취급하지 않는다.
        let temp = TempDir::new("not-wav");
        let path = temp.path().join("not-wav.wav");
        fs::write(&path, [7_u8; 4_096]).expect("사전 조건: WAV가 아닌 파일을 둔다");

        let failure = verify(&path, FORMAT).expect_err("읽을 수 없는 파일의 포맷은 알 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(failure.detail.is_some(), "기술적 원인이 함께 실린다");
        assert!(path.is_file(), "실패해도 파일을 지우지 않는다 (INV-4)");
    }

    #[test]
    fn a_file_whose_format_differs_from_the_capture_is_refused() {
        // 기대와 실제가 다르면 어느 쪽이 맞는지 고르지 않고 그 사실을 알린다.
        let temp = TempDir::new("mismatch");
        let path = wav_file(temp.path(), "stereo.wav", CaptureFormat::pcm_16bit(48_000, 2), 64);

        let failure = verify(&path, FORMAT).expect_err("다른 형식의 파일은 확인을 통과할 수 없다");

        let detail = failure.detail.expect("기대값과 실제값이 함께 남는다");
        assert!(detail.contains("48000"), "실제 형식이 보인다: {detail}");
        assert!(detail.contains("16000"), "기대한 형식이 보인다: {detail}");
        assert!(path.is_file(), "실패해도 파일을 지우지 않는다 (INV-4)");
    }

    #[test]
    fn the_presence_check_answers_without_changing_anything() {
        let temp = TempDir::new("presence");
        let path = wav_file(temp.path(), "present.wav", FORMAT, 32);
        let absent = temp.path().join("absent.wav");

        assert!(audio_is_present(&path.display().to_string()));
        assert!(!audio_is_present(&absent.display().to_string()));
        assert!(!audio_is_present(""), "빈 경로는 있는 파일이 아니다");
        assert!(
            !audio_is_present(&temp.path().display().to_string()),
            "디렉터리는 오디오 파일이 아니다"
        );

        assert!(path.is_file(), "확인은 파일을 건드리지 않는다");
        assert!(!absent.exists(), "확인은 없는 파일을 만들지 않는다");
    }
}
