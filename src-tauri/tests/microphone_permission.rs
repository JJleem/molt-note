//! 마이크 권한이 캡처 시작을 실제로 막는지 본다 (PRODUCT-SPEC §13 · §14.3 · INV-10).
//!
//! **이 파일에는 마이크도, 실제 마이크 권한도 없다.** 권한을 묻는 자리가
//! [`MicrophonePermission`] 뒤에 있으므로 여기서는 그 자리에 정해진 상태를 돌려주는 구현을
//! 넣는다 (§18). 그 경계 바깥은 **제품 코드가 그대로 실행된다** — 시작 경로도, 파일 확정도,
//! 실패를 만드는 규칙도 진짜다.
//!
//! 확인하는 것은 세 가지다.
//!
//! ```text
//! 허용    캡처가 시작되고 파일이 생긴다
//! 거부    캡처가 시작되지 않는다 — 장치를 열지도, 파일을 만들지도 않는다
//! 미결정  캡처를 막지는 않되, 장치를 열지 못하면 권한 안내로 도달한다 (UNVERIFIED 폴백)
//! ```
//!
//! **실제 TCC 프롬프트가 뜨는지, macOS가 실제로 어떤 상태를 돌려주는지는 이 테스트가
//! 판정하지 않는다.** 그것은 사람이 번들된 앱에서 확인할 항목이다
//! (`docs/ADR-0005-microphone-permission.md` §6).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use molt_note_lib::audio::{CaptureFormat, OpenCapture, SampleSink, SampleSource};
use molt_note_lib::commands::Recorder;
use molt_note_lib::domain::{Failure, FailureKind};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::microphone::{MicrophoneAccess, MicrophonePermission};

/// 테스트가 고른 장치 키. 가짜 마이크는 어떤 키를 줘도 같은 값을 낸다.
const DEVICE_KEY: &str = "0:가짜 마이크";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 앱 데이터 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-permission-test-{}-{}-{}",
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

/// 테스트가 권한 상태를 정하는 경계 구현. 실제 OS에 묻는 코드는 이 파일에 없다.
///
/// 물어본 횟수를 센다 — **장치를 열기 전에 물었는지**를 이것으로 본다.
struct FakePermission {
    access: Mutex<MicrophoneAccess>,
    asked: AtomicUsize,
}

impl FakePermission {
    /// `Recorder`가 들고 갈 수 있고 테스트도 계속 볼 수 있게 공유 값으로 만든다.
    fn new(access: MicrophoneAccess) -> Arc<Self> {
        Arc::new(Self {
            access: Mutex::new(access),
            asked: AtomicUsize::new(0),
        })
    }

    fn asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }

    /// 사용자가 시스템 설정에서 접근을 허용한 상황.
    fn allow(&self) {
        *self.access.lock().expect("권한 상태를 빌릴 수 있어야 한다") = MicrophoneAccess::Granted;
    }
}

/// `Recorder`에게 넘기는 손잡이. 넘긴 뒤에도 테스트가 같은 값을 계속 본다.
struct SharedPermission(Arc<FakePermission>);

impl SharedPermission {
    fn to(permission: &Arc<FakePermission>) -> Self {
        Self(Arc::clone(permission))
    }
}

impl MicrophonePermission for SharedPermission {
    fn status(&self) -> MicrophoneAccess {
        *self.0.access.lock().expect("권한 상태를 빌릴 수 있어야 한다")
    }

    fn request(&self) -> MicrophoneAccess {
        self.0.asked.fetch_add(1, Ordering::SeqCst);
        self.status()
    }
}

/// 정해진 샘플을 보내는 가짜 마이크. **열린 횟수를 센다.**
struct FakeMicrophone {
    opened: AtomicUsize,
}

impl FakeMicrophone {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            opened: AtomicUsize::new(0),
        })
    }

    fn opened(&self) -> usize {
        self.opened.load(Ordering::SeqCst)
    }
}

/// 같은 이유의 손잡이. 마이크를 넘긴 뒤에도 열린 횟수를 볼 수 있어야 한다.
struct SharedMicrophone(Arc<FakeMicrophone>);

impl SharedMicrophone {
    fn to(microphone: &Arc<FakeMicrophone>) -> Self {
        Self(Arc::clone(microphone))
    }
}

impl SampleSource for SharedMicrophone {
    fn open(&self, _device_key: &str, samples: SampleSink) -> Result<OpenCapture, Failure> {
        self.0.opened.fetch_add(1, Ordering::SeqCst);

        let sending = thread::spawn(move || {
            for chunk in [vec![1_000i16; 512], vec![-1_000i16; 512]] {
                if samples.send(chunk).is_err() {
                    break;
                }
            }
        });

        Ok(OpenCapture {
            device_label: "가짜 마이크".to_string(),
            format: CaptureFormat::pcm_16bit(16_000, 1),
            stop: Box::new(move || {
                let _ = sending.join();
                Ok(())
            }),
        })
    }
}

/// 장치를 열지 못하는 경계 구현. 실제 원인은 권한일 수도, 다른 앱의 점유일 수도 있다.
struct UnavailableMicrophone;

impl SampleSource for UnavailableMicrophone {
    fn open(&self, _device_key: &str, _samples: SampleSink) -> Result<OpenCapture, Failure> {
        Err(
            Failure::retryable(FailureKind::AudioDevice, "고른 입력 장치를 열지 못했다.")
                .with_detail("device unavailable"),
        )
    }
}

/// 디렉터리 안의 `.wav` 파일 전부. 디렉터리가 없으면 빈 목록이다.
fn wav_files(directory: &Path) -> Vec<PathBuf> {
    match std::fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("wav"))
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[test]
fn a_denied_microphone_stops_the_capture_before_anything_is_opened_or_written() {
    // 이 Task의 핵심 주장이다 — 거부 상태에서는 캡처가 **시작되지 않는다.**
    let temp = TempRoot::new("denied");
    let app_data_dir = temp.app_data_dir();
    let microphone = FakeMicrophone::new();
    let permission = FakePermission::new(MicrophoneAccess::Denied);
    let recorder =
        Recorder::with_source(app_data_dir.clone(), SharedMicrophone::to(&microphone))
            .with_microphone(SharedPermission::to(&permission));

    let failure = recorder
        .start(DEVICE_KEY)
        .expect_err("거부된 상태에서는 시작되지 않아야 한다");

    assert_eq!(failure.kind, FailureKind::MicrophonePermission);
    assert_eq!(permission.asked(), 1, "장치보다 권한을 먼저 묻는다");
    assert_eq!(microphone.opened(), 0, "장치를 열지 않는다");
    assert!(
        wav_files(&app_data_dir.recordings_dir()).is_empty(),
        "파일을 만들지 않는다"
    );

    // 진행 중인 녹음도 남지 않는다 — 화면은 여전히 "시작 전"을 본다.
    let status = recorder.status().expect("상태를 물을 수 있어야 한다");
    assert_eq!(status.state, "idle");
    assert_eq!(status.elapsed_ms, 0);
}

#[test]
fn a_denied_microphone_tells_the_user_what_to_do_and_is_not_a_recording_setup_failure() {
    // 사용자가 읽고 행동할 수 있어야 한다 (§13). 그리고 녹음 초기화 실패와 구분돼야 한다 —
    // 다른 장치를 고르거나 앱을 다시 시작해서 풀리는 문제가 아니기 때문이다.
    let temp = TempRoot::new("denied-message");
    let permission = FakePermission::new(MicrophoneAccess::Denied);
    let recorder = Recorder::with_source(temp.app_data_dir(), SharedMicrophone::to(&FakeMicrophone::new()))
        .with_microphone(SharedPermission::to(&permission));

    let failure = recorder.start(DEVICE_KEY).expect_err("거부");

    assert_ne!(failure.kind, FailureKind::AudioDevice, "장치 실패가 아니다");
    assert_ne!(failure.kind, FailureKind::Storage, "초기화 실패가 아니다");
    assert!(!failure.retryable, "설정을 바꾸지 않으면 결과가 같다");
    assert!(failure.source_data_safe, "저장된 것을 건드리지 않았다");
    assert!(
        failure.message.contains("마이크") && failure.message.contains("허용"),
        "무엇을 해야 하는지 없다: {}",
        failure.message
    );

    // 어디를 열어야 하는지는 platform 경계가 만들어 넣는다. macOS에서는 그 경로가 문장에 있다.
    if cfg!(target_os = "macos") {
        assert!(
            failure.message.contains("시스템 설정")
                && failure.message.contains("개인정보 보호 및 보안"),
            "설정 경로가 없다: {}",
            failure.message
        );
    }
}

#[test]
fn a_granted_microphone_records_exactly_as_before() {
    let temp = TempRoot::new("granted");
    let microphone = FakeMicrophone::new();
    let permission = FakePermission::new(MicrophoneAccess::Granted);
    let recorder =
        Recorder::with_source(temp.app_data_dir(), SharedMicrophone::to(&microphone))
            .with_microphone(SharedPermission::to(&permission));

    recorder
        .start(DEVICE_KEY)
        .expect("허용된 상태에서는 시작돼야 한다");
    let report = recorder.stop().expect("정지할 수 있어야 한다");

    assert_eq!(permission.asked(), 1);
    assert_eq!(microphone.opened(), 1, "장치를 연다");
    assert!(
        PathBuf::from(&report.output_path).is_file(),
        "파일이 실제로 만들어진다"
    );
}

#[test]
fn allowing_access_afterwards_makes_the_next_start_work() {
    // 거부는 영구적인 앱 상태가 아니다. 사용자가 설정에서 허용하면 다음 시작이 그대로 된다.
    let temp = TempRoot::new("allow-later");
    let microphone = FakeMicrophone::new();
    let permission = FakePermission::new(MicrophoneAccess::Denied);
    let recorder =
        Recorder::with_source(temp.app_data_dir(), SharedMicrophone::to(&microphone))
            .with_microphone(SharedPermission::to(&permission));

    recorder.start(DEVICE_KEY).expect_err("아직 거부돼 있다");
    permission.allow();
    recorder.start(DEVICE_KEY).expect("허용한 뒤에는 시작된다");

    assert_eq!(permission.asked(), 2, "시작할 때마다 새로 묻는다");
    assert_eq!(microphone.opened(), 1);
    recorder.stop().expect("정지할 수 있어야 한다");
}

#[test]
fn an_undetermined_state_does_not_block_a_microphone_that_opens() {
    // "모른다"를 "거부"로 접지 않는다. 판정 수단이 없다는 이유로 녹음을 막으면, 권한이 멀쩡한
    // 사용자가 앱을 쓸 수 없게 된다.
    let temp = TempRoot::new("undetermined-ok");
    let microphone = FakeMicrophone::new();
    let permission = FakePermission::new(MicrophoneAccess::Undetermined);
    let recorder =
        Recorder::with_source(temp.app_data_dir(), SharedMicrophone::to(&microphone))
            .with_microphone(SharedPermission::to(&permission));

    recorder
        .start(DEVICE_KEY)
        .expect("미결정 상태가 시작을 막지 않는다");

    assert_eq!(microphone.opened(), 1);
    recorder.stop().expect("정지할 수 있어야 한다");
}

#[test]
fn an_undetermined_state_that_cannot_open_the_device_points_at_the_permission_setting() {
    // 신뢰할 수 있는 판정 수단이 없는 동안의 폴백이다 (ADR-0005 §4 · UNVERIFIED).
    // 원래의 기술적 원인은 사라지지 않는다.
    let temp = TempRoot::new("undetermined-fail");
    let permission = FakePermission::new(MicrophoneAccess::Undetermined);
    let recorder = Recorder::with_source(temp.app_data_dir(), UnavailableMicrophone)
        .with_microphone(SharedPermission::to(&permission));

    let failure = recorder.start(DEVICE_KEY).expect_err("장치를 열지 못했다");

    assert_eq!(failure.kind, FailureKind::MicrophonePermission);
    assert!(
        failure.message.contains("마이크"),
        "{}",
        failure.message
    );
    let detail = failure.detail.expect("기술적 원인이 남아야 한다");
    assert!(detail.contains("device unavailable"), "{detail}");
}

#[test]
fn a_device_that_fails_while_access_is_granted_stays_a_device_failure() {
    // 권한이 있는데도 장치를 열지 못했다면 권한 안내를 보여선 안 된다 — 사용자가 없는 문제를
    // 고치려 하게 된다.
    let temp = TempRoot::new("granted-fail");
    let permission = FakePermission::new(MicrophoneAccess::Granted);
    let recorder = Recorder::with_source(temp.app_data_dir(), UnavailableMicrophone)
        .with_microphone(SharedPermission::to(&permission));

    let failure = recorder.start(DEVICE_KEY).expect_err("장치를 열지 못했다");

    assert_eq!(failure.kind, FailureKind::AudioDevice);
    assert_eq!(failure.message, "고른 입력 장치를 열지 못했다.");
}
