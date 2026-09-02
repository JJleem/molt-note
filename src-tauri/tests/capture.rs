//! 캡처 시작 → 정지 → 파일 확정 → 보고가 command 경계까지 도달하는지 본다
//! (ADR-0003 · PRODUCT-SPEC §6.1 · §13).
//!
//! **이 파일은 실제 마이크도 마이크 권한도 요구하지 않는다.** 실제 장치에서 샘플을 받는
//! 부분이 [`SampleSource`] 뒤에 있으므로, 여기서는 그 자리에 정해진 샘플을 보내는 구현을
//! 넣는다 (§18). 그 경계 바깥은 **제품 코드가 그대로 실행된다** — 경로 결정도, WAV writer도,
//! 파일 확정도, 크기 읽기도 진짜다.
//!
//! 실제 장치에서만 알 수 있는 것(권한 프롬프트 · 어떤 장치가 실제로 열리는가 · 녹음된 소리가
//! 들리는가 · 실제 컨테이너와 코덱)은 사람이 확인하며, **이 테스트가 그것을 대신 판정하지
//! 않는다** (ADR-0003 §12).
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread;

use molt_note_lib::audio::{CaptureFormat, OpenCapture, SampleSource};
use molt_note_lib::commands::Capture;
use molt_note_lib::domain::{Failure, FailureKind};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;

/// 테스트가 고른 장치 키. 가짜 마이크는 어떤 키를 줘도 같은 값을 낸다.
const DEVICE_KEY: &str = "0:가짜 마이크";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 앱 데이터 루트. Drop 시 지운다.
///
/// 경로가 `std::env::temp_dir()`에서만 나오므로 테스트가 실제 앱 데이터 위치를 건드리지 않는다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-capture-test-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        Self(std::env::temp_dir().join(unique))
    }

    fn app_data_dir(&self) -> AppDataDirectory {
        AppDataDirectory::new(self.0.join("app-data"))
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 정해진 샘플을 보내는 가짜 마이크. 실제 하드웨어를 부르는 코드는 이 파일에 없다.
struct FakeMicrophone {
    label: String,
    format: CaptureFormat,
    chunks: Vec<Vec<i16>>,
    /// 정지할 때 돌려줄 실패. 캡처가 중간에 끊긴 경우를 만들 때 쓴다.
    interrupted: Option<Failure>,
}

impl FakeMicrophone {
    /// 짧은 소리를 내는 마이크 하나.
    fn speaking() -> Self {
        Self {
            label: "가짜 마이크".to_string(),
            format: CaptureFormat::pcm_16bit(16_000, 1),
            chunks: vec![vec![1_000; 512], vec![-1_000; 512], vec![0; 512]],
            interrupted: None,
        }
    }

    /// 캡처가 도중에 끊기는 마이크.
    fn cut_short(failure: Failure) -> Self {
        Self {
            interrupted: Some(failure),
            ..Self::speaking()
        }
    }

    /// 이 마이크가 보내는 샘플의 총 개수.
    fn sample_count(&self) -> usize {
        self.chunks.iter().map(Vec::len).sum()
    }
}

impl SampleSource for FakeMicrophone {
    fn open(&self, _device_key: &str, samples: SyncSender<Vec<i16>>) -> Result<OpenCapture, Failure> {
        let chunks = self.chunks.clone();
        let sending = thread::spawn(move || {
            for chunk in chunks {
                if samples.send(chunk).is_err() {
                    break;
                }
            }
        });

        let interrupted = self.interrupted.clone();
        Ok(OpenCapture {
            device_label: self.label.clone(),
            format: self.format,
            stop: Box::new(move || {
                let _ = sending.join();
                match interrupted {
                    Some(failure) => Err(failure),
                    None => Ok(()),
                }
            }),
        })
    }
}

/// 장치를 열지 못하는 경계 구현. 실제 원인은 권한이거나 다른 앱의 점유다.
struct UnavailableMicrophone;

impl SampleSource for UnavailableMicrophone {
    fn open(&self, _device_key: &str, _samples: SyncSender<Vec<i16>>) -> Result<OpenCapture, Failure> {
        Err(Failure::retryable(
            FailureKind::AudioDevice,
            "입력 장치를 열지 못했다. 다른 앱이 쓰고 있거나 마이크 권한이 없을 수 있다.",
        )
        .with_detail("device busy"))
    }
}

/// 디렉터리 안의 `.wav` 파일 전부. 없으면 빈 목록이다.
fn wav_files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match std::fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("wav"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

#[test]
fn a_capture_that_stops_reports_the_device_the_path_the_format_and_the_size() {
    // Phase 2A의 성공 기준 그 자체다 (ADR-0003 §12 · phase-prompt/02a).
    let temp = TempRoot::new("report");
    let microphone = FakeMicrophone::speaking();
    let expected_samples = microphone.sample_count();
    let app_data_dir = temp.app_data_dir();
    let capture = Capture::with_source(app_data_dir.clone(), microphone);

    capture.start(DEVICE_KEY).expect("녹음을 시작할 수 있어야 한다");
    let report = capture.stop().expect("녹음을 정지할 수 있어야 한다");

    assert_eq!(report.device_label, "가짜 마이크", "장치 이름");
    assert!(!report.output_path.is_empty(), "출력 경로");
    assert_eq!(report.container, "WAV", "컨테이너");
    assert_eq!(report.sample_rate_hz, 16_000);
    assert_eq!(report.channels, 1);
    assert_eq!(report.bits_per_sample, 16);

    // 포맷 문장에 네 가지가 모두 들어 있다.
    for fact in ["16000", "mono", "16-bit", "WAV"] {
        assert!(
            report.format.contains(fact),
            "포맷 문장에 {fact}가 없다: {}",
            report.format
        );
    }

    // 보고된 크기는 **실제로 만들어진 파일**의 크기다.
    let written = PathBuf::from(&report.output_path);
    assert!(written.is_file(), "보고된 경로에 파일이 있어야 한다");
    let on_disk = std::fs::metadata(&written).expect("파일을 읽을 수 있어야 한다").len();
    assert_eq!(report.byte_size, on_disk);
    assert!(
        report.byte_size >= (expected_samples * 2) as u64,
        "보낸 샘플이 파일에 들어 있어야 한다: {} byte",
        report.byte_size
    );
}

#[test]
fn the_output_file_lives_under_the_app_data_directory() {
    // 경로는 AppDataDirectory에서 파생된다 — 캡처가 자기 마음대로 자리를 정하지 않는다.
    let temp = TempRoot::new("app-data");
    let app_data_dir = temp.app_data_dir();
    let capture = Capture::with_source(app_data_dir.clone(), FakeMicrophone::speaking());

    capture.start(DEVICE_KEY).expect("녹음을 시작할 수 있어야 한다");
    let report = capture.stop().expect("녹음을 정지할 수 있어야 한다");

    let written = PathBuf::from(&report.output_path);
    assert!(
        written.starts_with(app_data_dir.root()),
        "출력이 앱 데이터 디렉터리 밖에 있다: {}",
        written.display()
    );
    assert_eq!(
        written.parent(),
        Some(app_data_dir.recordings_dir().as_path()),
        "녹음은 앱 데이터 디렉터리의 녹음 자리에 놓인다"
    );
}

#[test]
fn a_second_capture_does_not_overwrite_the_first() {
    // 두 녹음이 같은 초에 끝나도 앞선 파일이 사라지지 않는다 (INV-1).
    let temp = TempRoot::new("no-overwrite");
    let app_data_dir = temp.app_data_dir();
    let capture = Capture::with_source(app_data_dir.clone(), FakeMicrophone::speaking());

    capture.start(DEVICE_KEY).expect("첫 녹음 시작");
    let first = capture.stop().expect("첫 녹음 정지");
    capture.start(DEVICE_KEY).expect("두 번째 녹음 시작");
    let second = capture.stop().expect("두 번째 녹음 정지");

    assert_ne!(first.output_path, second.output_path);
    assert!(PathBuf::from(&first.output_path).is_file(), "첫 파일이 남아 있어야 한다");
    assert!(PathBuf::from(&second.output_path).is_file());
    assert_eq!(wav_files(&app_data_dir.recordings_dir()).len(), 2);
    assert!(first.byte_size > 0 && second.byte_size > 0);
}

#[test]
fn a_device_that_cannot_be_opened_reaches_the_user_as_the_shared_failure_contract() {
    let temp = TempRoot::new("unavailable");
    let app_data_dir = temp.app_data_dir();
    let capture = Capture::with_source(app_data_dir.clone(), UnavailableMicrophone);

    let failure = capture.start(DEVICE_KEY).expect_err("장치를 열지 못한 것은 실패다");

    assert_eq!(failure.kind, FailureKind::AudioDevice, "저장소 실패와 구분된다");
    assert!(!failure.message.is_empty(), "보여줄 문장이 있어야 한다");
    assert!(failure.source_data_safe, "아무것도 쓰지 않았다");
    assert!(failure.retryable, "권한을 주거나 다른 앱을 닫으면 성공할 수 있다");
    assert_eq!(failure.detail.as_deref(), Some("device busy"));

    assert!(
        wav_files(&app_data_dir.recordings_dir()).is_empty(),
        "열지 못했으면 파일도 만들어지지 않는다"
    );
}

#[test]
fn a_failure_to_open_serializes_into_the_shape_the_frontend_reads() {
    let temp = TempRoot::new("serialized");
    let capture = Capture::with_source(temp.app_data_dir(), UnavailableMicrophone);

    let failure = capture.start(DEVICE_KEY).expect_err("실패해야 한다");
    let json = serde_json::to_value(&failure).expect("직렬화할 수 있어야 한다");

    // `src/ipc/failure.ts`의 FailureKind union에 같은 문자열이 있어야 한다.
    assert_eq!(json["kind"], "audioDevice");
    assert_eq!(json["sourceDataSafe"], true);
    assert_eq!(json["retryable"], true);
}

#[test]
fn a_capture_that_was_cut_short_says_so_and_keeps_what_was_recorded() {
    // 끊긴 것을 성공으로 보고하지 않는다. 그렇다고 이미 녹음된 것을 숨기지도 않는다.
    let temp = TempRoot::new("cut-short");
    let app_data_dir = temp.app_data_dir();
    let capture = Capture::with_source(
        app_data_dir.clone(),
        FakeMicrophone::cut_short(
            Failure::retryable(FailureKind::AudioDevice, "녹음이 중간에 끊겼다.")
                .with_detail("stream error"),
        ),
    );

    capture.start(DEVICE_KEY).expect("녹음을 시작할 수 있어야 한다");
    let failure = capture.stop().expect_err("끊긴 캡처는 실패로 보고된다");

    assert_eq!(failure.kind, FailureKind::AudioDevice);
    assert!(failure.retryable);
    assert_eq!(failure.detail.as_deref(), Some("stream error"));

    let written = wav_files(&app_data_dir.recordings_dir());
    assert_eq!(written.len(), 1, "그때까지의 녹음은 파일로 남는다");
    assert!(
        failure.message.contains(&written[0].display().to_string()),
        "어디에 남았는지 사용자가 알 수 있어야 한다: {}",
        failure.message
    );
    assert!(
        std::fs::metadata(&written[0]).expect("파일을 읽을 수 있어야 한다").len() > 0
    );
}

#[test]
fn stopping_without_starting_is_a_failure_not_a_panic() {
    let temp = TempRoot::new("not-recording");
    let capture = Capture::with_source(temp.app_data_dir(), FakeMicrophone::speaking());

    let failure = capture.stop().expect_err("녹음 중이 아니면 정지할 것이 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(!failure.retryable, "다시 정지해도 결과는 같다");
    assert!(failure.source_data_safe);
}

#[test]
fn starting_twice_does_not_throw_away_the_recording_already_running() {
    let temp = TempRoot::new("already-recording");
    let app_data_dir = temp.app_data_dir();
    let capture = Capture::with_source(app_data_dir.clone(), FakeMicrophone::speaking());

    capture.start(DEVICE_KEY).expect("첫 녹음 시작");
    let failure = capture.start(DEVICE_KEY).expect_err("이미 녹음 중이다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(failure.source_data_safe, "진행 중인 녹음을 건드리지 않는다");

    // 첫 녹음은 그대로 살아 있고 정상적으로 끝난다.
    let report = capture.stop().expect("첫 녹음은 그대로 끝낼 수 있어야 한다");
    assert!(report.byte_size > 0);
    assert_eq!(wav_files(&app_data_dir.recordings_dir()).len(), 1);
}

#[test]
fn a_place_that_cannot_hold_recordings_becomes_a_failure_the_user_can_read() {
    // 녹음 디렉터리가 있어야 할 자리에 파일이 있다.
    let temp = TempRoot::new("blocked");
    let app_data_dir = temp.app_data_dir();
    std::fs::create_dir_all(app_data_dir.root()).expect("사전 조건: 루트를 만든다");
    std::fs::write(app_data_dir.recordings_dir(), "디렉터리가 아니다")
        .expect("사전 조건: 자리를 막는다");

    let capture = Capture::with_source(app_data_dir, FakeMicrophone::speaking());
    let failure = capture.start(DEVICE_KEY).expect_err("파일 위에 녹음을 둘 수는 없다");

    assert_eq!(failure.kind, FailureKind::Storage, "장치 실패와 구분된다");
    assert!(!failure.message.is_empty());
    assert!(failure.source_data_safe, "아무것도 쓰지 못했다");
    assert!(failure.retryable);
}
