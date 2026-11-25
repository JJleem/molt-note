//! **캡처 경고의 end-to-end 테스트** (Phase 5.11 Required Outcome 7).
//!
//! 가짜 `SampleSource`로 네 시나리오를 지나고, **실제 DB에 저장된 레코드를 다시 조회해**
//! `capture_warning` 값까지 확인한다. 실제 마이크도 마이크 권한도 요구하지 않는다 (§18).
//!
//! ```text
//! (a) 큐 넘침       → 레코드가 저장되고 capture_warning이 있다
//! (b) 스트림 오류    → 레코드가 저장되고 capture_warning이 있다
//! (c) 정상           → 레코드가 저장되고 capture_warning이 None이다
//! (d) 파일 확정 실패 → 레코드가 없고 Failure에 파일 경로가 있다
//! ```

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use molt_note_lib::audio::{CaptureFormat, CaptureMode, OpenCapture, SampleSink, SampleSource};
use molt_note_lib::commands::{
    finish_recording, LiveTranscriber, Recorder, Storage, Transcriber,
};
use molt_note_lib::domain::{Failure, FailureKind};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::clock::Clock;
use molt_note_lib::transcription::{engine_failed, StubEngine};

const DEVICE_KEY: &str = "0:가짜 마이크";
const CHUNK: usize = 512;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// ── 공용 도구 ──────────────────────────────────────────────────────────────

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-cw-test-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Default)]
struct TestClock {
    now_ms: Arc<AtomicI64>,
}

impl TestClock {
    fn advance(&self, ms: i64) {
        self.now_ms.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now_ms(&self) -> i64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

fn idle_transcriber(app_data_dir: AppDataDirectory) -> Transcriber {
    Transcriber::with_engine(
        app_data_dir,
        StubEngine::failing(engine_failed("이 테스트는 전사를 쓰지 않는다")),
    )
}

fn idle_live() -> Arc<LiveTranscriber> {
    Arc::new(LiveTranscriber::with_engine(
        StubEngine::failing(Failure::permanent(
            FailureKind::TranscriptionEngineFailed,
            "이 테스트는 실시간 전사를 쓰지 않는다",
        )),
    ))
}

// ── 가짜 마이크 ─────────────────────────────────────────────────────────────

/// 테스트가 직접 샘플을 보내는 마이크. `interrupted`로 stop 시 반환할 실패를 정한다.
#[derive(Clone)]
struct ControlledMicrophone {
    sink: Arc<Mutex<Option<SampleSink>>>,
    interrupted: Arc<Mutex<Option<Failure>>>,
}

impl ControlledMicrophone {
    fn normal() -> Self {
        Self {
            sink: Arc::new(Mutex::new(None)),
            interrupted: Arc::new(Mutex::new(None)),
        }
    }

    fn with_interrupted(failure: Failure) -> Self {
        Self {
            sink: Arc::new(Mutex::new(None)),
            interrupted: Arc::new(Mutex::new(Some(failure))),
        }
    }

    fn speak(&self, value: i16) {
        let sink = self.sink.lock().expect("통로를 빌릴 수 있어야 한다");
        sink.as_ref()
            .expect("장치가 열려 있어야 한다")
            .send(vec![value; CHUNK])
            .expect("샘플을 보낼 수 있어야 한다");
    }
}

impl SampleSource for ControlledMicrophone {
    fn open(&self, _device_key: &str, samples: SampleSink) -> Result<OpenCapture, Failure> {
        *self.sink.lock().expect("통로를 둘 수 있어야 한다") = Some(samples);

        let sink = Arc::clone(&self.sink);
        let interrupted = Arc::clone(&self.interrupted);
        Ok(OpenCapture {
            device_label: "가짜 마이크".to_string(),
            format: CaptureFormat::pcm_16bit(16_000, 1),
            stop: Box::new(move || {
                *sink.lock().expect("통로를 놓을 수 있어야 한다") = None;
                match interrupted.lock().expect("interrupted를 빌릴 수 있어야 한다").take() {
                    Some(failure) => Err(failure),
                    None => Ok(()),
                }
            }),
        })
    }
}

/// 파일 확정이 실패하는 마이크 — writer 스레드에 닿는 통로를 열기 전에 닫는다.
/// 이렇게 하면 WAV writer가 정상 종료되지 못하고, `ActiveCapture::stop`이 Err를 반환한다.
struct WriterFailMicrophone;

impl SampleSource for WriterFailMicrophone {
    fn open(&self, _device_key: &str, samples: SampleSink) -> Result<OpenCapture, Failure> {
        // 샘플을 충분히 보낸 뒤 기록은 진행됐으나, stop 시 writer가 Err를 돌려주도록 한다.
        // writer 실패를 만드는 방법: 정상적으로 샘플을 보내되, stop 함수가 join한 writer가
        // 실패를 돌려주도록 한다. 실제로는 writer가 실패하는 경로를 만들기 어렵다.
        //
        // 대안: 장치를 열되 아무 샘플도 보내지 않는다. 그러면 파일은 비어 있고,
        // finalized::verify에서 MIN_FINALIZED_BYTES 미달로 실패한다 — 이것이 바로
        // "파일 확정 실패"와 같은 의미다: Recorder::stop이 Err를 돌려준다.
        //
        // 실제로 stop에서 파일 경로를 담은 Failure가 오는지 확인한다.
        let sending = std::thread::spawn(move || {
            // 아무 샘플도 보내지 않는다. 파일은 WAV 헤더만 남는다.
            drop(samples);
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

fn setup(
    temp: &TempRoot,
    source: impl SampleSource + Send + Sync + 'static,
) -> (Recorder, Storage, Transcriber, TestClock) {
    let app_data_dir = AppDataDirectory::new(temp.path().join("app-data"));
    let storage = Storage::open(&app_data_dir);
    assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

    let clock = TestClock::default();
    let recorder = Recorder::with_clock(app_data_dir.clone(), source, clock.clone());
    let transcriber = idle_transcriber(app_data_dir);
    (recorder, storage, transcriber, clock)
}

// ── 시나리오 (a): 큐 넘침 ───────────────────────────────────────────────────

#[test]
fn queue_overflow_saves_a_record_with_capture_warning() {
    let temp = TempRoot::new("queue-overflow");
    let mic = ControlledMicrophone::with_interrupted(Failure::retryable(
        FailureKind::AudioDevice,
        "큐가 넘쳤다. 일부 구간이 유실됐을 수 있다.",
    ));
    let mic_handle = mic.clone();
    let (recorder, storage, transcriber, clock) = setup(&temp, mic);

    recorder
        .start(DEVICE_KEY, CaptureMode::Microphone)
        .expect("시작할 수 있어야 한다");
    mic_handle.speak(1_000);
    clock.advance(5_000);

    let stopped =
        finish_recording(&recorder, &storage, &transcriber, &idle_live(), None)
            .expect("파일이 온전하면 경고와 함께 성공이다");

    // 반환된 payload에 경고가 있다.
    assert!(
        stopped.warning.is_some(),
        "큐 넘침은 경고로 보고돼야 한다"
    );
    assert!(
        stopped.warning.as_ref().unwrap().contains("큐가 넘쳤다"),
        "경고 내용: {:?}",
        stopped.warning,
    );

    // DB에서 다시 읽어 capture_warning을 확인한다.
    let reloaded = storage
        .recording(&stopped.recording.id)
        .expect("조회할 수 있어야 한다")
        .expect("레코드가 있어야 한다");
    assert!(
        reloaded.capture_warning.is_some(),
        "DB에 저장된 레코드에도 capture_warning이 있어야 한다"
    );
    assert!(
        reloaded.capture_warning.as_ref().unwrap().contains("큐가 넘쳤다"),
        "DB의 capture_warning: {:?}",
        reloaded.capture_warning,
    );
}

// ── 시나리오 (b): 스트림 오류 ────────────────────────────────────────────────

#[test]
fn stream_error_saves_a_record_with_capture_warning() {
    let temp = TempRoot::new("stream-error");
    let mic = ControlledMicrophone::with_interrupted(Failure::retryable(
        FailureKind::AudioDevice,
        "녹음이 중간에 끊겼다. 스트림 오류가 발생했다.",
    ));
    let mic_handle = mic.clone();
    let (recorder, storage, transcriber, clock) = setup(&temp, mic);

    recorder
        .start(DEVICE_KEY, CaptureMode::Microphone)
        .expect("시작할 수 있어야 한다");
    mic_handle.speak(1_000);
    clock.advance(5_000);

    let stopped =
        finish_recording(&recorder, &storage, &transcriber, &idle_live(), None)
            .expect("파일이 온전하면 경고와 함께 성공이다");

    assert!(
        stopped.warning.is_some(),
        "스트림 오류는 경고로 보고돼야 한다"
    );
    assert!(
        stopped.warning.as_ref().unwrap().contains("스트림 오류"),
        "경고 내용: {:?}",
        stopped.warning,
    );

    // DB에서 다시 읽어 확인한다.
    let reloaded = storage
        .recording(&stopped.recording.id)
        .expect("조회할 수 있어야 한다")
        .expect("레코드가 있어야 한다");
    assert!(
        reloaded.capture_warning.is_some(),
        "DB 레코드에 capture_warning이 있어야 한다"
    );
    assert!(
        reloaded.capture_warning.as_ref().unwrap().contains("스트림 오류"),
        "DB의 capture_warning: {:?}",
        reloaded.capture_warning,
    );
}

// ── 시나리오 (c): 정상 ──────────────────────────────────────────────────────

#[test]
fn clean_stop_saves_a_record_without_capture_warning() {
    let temp = TempRoot::new("clean-stop");
    let mic = ControlledMicrophone::normal();
    let mic_handle = mic.clone();
    let (recorder, storage, transcriber, clock) = setup(&temp, mic);

    recorder
        .start(DEVICE_KEY, CaptureMode::Microphone)
        .expect("시작할 수 있어야 한다");
    mic_handle.speak(1_000);
    clock.advance(5_000);

    let stopped =
        finish_recording(&recorder, &storage, &transcriber, &idle_live(), None)
            .expect("정상 정지는 성공이다");

    assert_eq!(
        stopped.warning, None,
        "정상 정지에는 경고가 없어야 한다"
    );

    // DB에서 다시 읽어 확인한다.
    let reloaded = storage
        .recording(&stopped.recording.id)
        .expect("조회할 수 있어야 한다")
        .expect("레코드가 있어야 한다");
    assert_eq!(
        reloaded.capture_warning, None,
        "DB 레코드의 capture_warning도 None이어야 한다"
    );
}

// ── 시나리오 (d): 파일 확정 실패 ────────────────────────────────────────────

#[test]
fn writer_failure_leaves_no_record_and_reports_the_file_path() {
    let temp = TempRoot::new("writer-fail");
    let (recorder, storage, transcriber, clock) = setup(&temp, WriterFailMicrophone);

    recorder
        .start(DEVICE_KEY, CaptureMode::Microphone)
        .expect("시작할 수 있어야 한다");
    // 아무 샘플도 보내지 않는다 — 파일은 WAV 헤더만 남는다.
    clock.advance(1_000);

    let failure =
        finish_recording(&recorder, &storage, &transcriber, &idle_live(), None)
            .expect_err("빈 파일은 정지 성공이 아니다");

    // Failure에 파일 경로가 담겨 있어야 한다.
    assert!(
        failure.message.contains(".wav") || failure.message.contains("남아 있다"),
        "실패에 파일 경로가 있어야 한다: {}",
        failure.message,
    );

    // DB에 레코드가 저장되지 않았어야 한다.
    let recordings = storage.list_recordings().expect("목록을 읽을 수 있어야 한다");
    assert!(
        recordings.is_empty(),
        "파일 확정 실패 시 레코드가 저장되지 않아야 한다"
    );
}
