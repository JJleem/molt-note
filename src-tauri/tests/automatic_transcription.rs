//! **자동 전사는 설정이 켜져 있을 때만 시작된다** (`phase-prompt/03` 요구 4 · ADR-0007 §8.2.3).
//!
//! 여기서 판정하는 것은 전사의 품질도, 동시성도 아니라 **무엇이 무엇을 시작시키는가**다.
//!
//! ```text
//! automatic_transcription = ON   → 정지가 성공한 뒤 전사가 시작된다
//! automatic_transcription = OFF  → 시작되지 않는다 (기본값이 OFF다)
//! 수동 전사                       → 설정과 무관하게 언제나 가능하다
//! automatic_processing            → 다른 값이다. 켜도 전사가 시작되지 않는다
//! ```
//!
//! 정지 경로는 제품 코드 그대로 지난다 — 파일이 실제로 쓰이고, 확인되고, 레코드가 저장된다.
//! 대체 구현이 들어가는 자리는 세 곳뿐이다: **마이크 · 시계 · 전사 엔진.** 실제 whisper도
//! 모델 파일도 필요하지 않다 (§18 · `phase-prompt/03` 요구 8 · 9).
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다. Tauri 런타임도 창도 하드웨어도 없다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use molt_note_lib::audio::{CaptureFormat, OpenCapture, SampleSink, SampleSource};
use molt_note_lib::commands::{
    finish_recording, Recorder, Storage, StoppedRecordingPayload, Transcriber,
    TranscriptionStatusPayload,
};
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{Failure, FailureKind, RecordingId, Settings, Transcript};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::clock::Clock;
use molt_note_lib::transcription::{RawSegment, RawTranscription, StubEngine};

/// 가짜 마이크는 어떤 키를 줘도 같은 값을 낸다.
const DEVICE_KEY: &str = "0:가짜 마이크";

/// 모델 디렉터리에 두는 자리표시자 파일의 이름. 실제 모델이 아니다.
const MODEL_FILE: &str = "ggml-base.bin";

/// 한 번에 보내는 샘플 수. 확정된 파일이 유효 최소치를 넘도록 넉넉히 보낸다.
const CHUNK: usize = 1_600;

/// 배경 전사가 끝나기를 기다리는 한계. 넘으면 테스트가 실패한다 — 매달리지 않는다.
const FINISH_TIMEOUT: Duration = Duration::from_secs(30);

/// "시작되지 않았다"를 확인하기 전에 배경 스레드에게 주는 시간.
///
/// 전사가 걸렸다면 이 시간 안에 상태가 `idle`에서 벗어난다 — 엔진이 값을 그대로 돌려주는
/// double이기 때문이다. 그래도 시작되지 않는다면 그것은 늦은 것이 아니라 걸리지 않은 것이다.
const SETTLE: Duration = Duration::from_millis(300);

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 자리 ----------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-automatic-transcription-{}-{}-{}",
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

/// 테스트가 시각을 정하는 시계.
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

/// 테스트가 "지금 말한다"고 정할 때만 샘플을 보내는 마이크.
#[derive(Clone)]
struct ControlledMicrophone {
    sink: Arc<Mutex<Option<SampleSink>>>,
}

impl ControlledMicrophone {
    fn new() -> Self {
        Self {
            sink: Arc::new(Mutex::new(None)),
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
        Ok(OpenCapture {
            device_label: "가짜 마이크".to_string(),
            // 전사가 요구하는 것과 같은 16 kHz mono다 — 이 테스트는 리샘플링을 판정하지 않는다.
            format: CaptureFormat::pcm_16bit(16_000, 1),
            stop: Box::new(move || {
                *sink.lock().expect("통로를 놓을 수 있어야 한다") = None;
                Ok(())
            }),
        })
    }
}

/// 녹음 · 저장 · 전사가 한 자리에서 일어나는 앱 하나.
struct Fixture {
    /// Drop 시 임시 디렉터리를 지운다.
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    recorder: Recorder,
    storage: Storage,
    transcriber: Transcriber,
    microphone: ControlledMicrophone,
    clock: TestClock,
}

impl Fixture {
    /// 모델이 실제로 놓여 있는 앱. 전사가 걸리면 끝까지 간다.
    fn new(label: &str) -> Self {
        let fixture = Self::without_model(label);
        let models_dir = fixture
            .app_data_dir
            .ensure_models_dir()
            .expect("사전 조건: 모델 디렉터리");
        std::fs::write(models_dir.join(MODEL_FILE), b"not a real model")
            .expect("사전 조건: 자리표시자 모델을 둔다");
        fixture.save_settings(Settings {
            transcription_model: Some(MODEL_FILE.to_string()),
            ..Settings::DEFAULT
        });
        fixture
    }

    /// 모델을 고르지 않은 앱. **그것이 앱의 기본 상태다** (ADR-0007 §8.2).
    fn without_model(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.path().join("app-data"));
        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

        let microphone = ControlledMicrophone::new();
        let clock = TestClock::default();

        Self {
            recorder: Recorder::with_clock(
                app_data_dir.clone(),
                microphone.clone(),
                clock.clone(),
            ),
            transcriber: Transcriber::with_engine(app_data_dir.clone(), StubEngine::returning(spoken())),
            storage,
            microphone,
            clock,
            app_data_dir,
            _root: root,
        }
    }

    /// 설정을 저장한다. 화면이 지나는 경로와 같은 저장소를 쓴다.
    fn save_settings(&self, settings: Settings) {
        let connection = self.connection();
        settings::save(&connection, &settings).expect("사전 조건: 설정을 저장한다");
    }

    fn settings(&self) -> Settings {
        settings::load(&self.connection()).expect("설정을 읽을 수 있어야 한다")
    }

    /// 소리가 들어 있는 녹음 하나를 시작해서 정지까지 마친다. **제품 경로 그대로다.**
    fn record_and_stop(&self) -> StoppedRecordingPayload {
        self.recorder
            .start(DEVICE_KEY)
            .expect("녹음을 시작할 수 있어야 한다");
        self.microphone.speak(1_000);
        self.clock.advance(5_000);
        finish_recording(&self.recorder, &self.storage, &self.transcriber, None)
            .expect("정지가 성공해야 한다")
    }

    fn connection(&self) -> rusqlite::Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    fn transcripts(&self, recording_id: &str) -> Vec<Transcript> {
        store::list_transcripts(&self.connection(), &RecordingId::new(recording_id))
            .expect("Transcript 목록을 읽을 수 있어야 한다")
    }

    /// 전사가 끝날 때까지 상태를 물어보며 기다린다. 끝난 상태를 돌려준다.
    fn wait_for_finish(&self) -> TranscriptionStatusPayload {
        let deadline = Instant::now() + FINISH_TIMEOUT;
        loop {
            let status = self
                .transcriber
                .status()
                .expect("상태를 물어볼 수 있어야 한다");
            if status.state != "running" {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "전사가 끝나기를 기다리다 시간이 지났다"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

/// 엔진이 냈다고 가정하는 원시 출력. **센티초다** (ADR-0007 §10).
fn spoken() -> RawTranscription {
    RawTranscription {
        language: Some("ko".to_owned()),
        segments: vec![RawSegment {
            start_centiseconds: 100,
            end_centiseconds: 240,
            text: Some(" 자동으로 걸린 전사".to_owned()),
        }],
    }
}

// --- ON일 때만 시작된다 ---------------------------------------------------------------

#[test]
fn a_stop_starts_a_transcription_only_when_automatic_transcription_is_on() {
    let fixture = Fixture::new("on");
    fixture.save_settings(Settings {
        automatic_transcription: true,
        transcription_model: Some(MODEL_FILE.to_string()),
        ..Settings::DEFAULT
    });

    let stopped = fixture.record_and_stop();
    let finished = fixture.wait_for_finish();

    assert_eq!(finished.state, "done", "정지 뒤에 전사가 걸려야 한다");
    assert_eq!(
        finished.recording_id.as_deref(),
        Some(stopped.recording.id.as_str()),
        "걸린 전사는 방금 저장된 그 녹음이다"
    );
    let transcripts = fixture.transcripts(&stopped.recording.id);
    assert_eq!(transcripts.len(), 1, "Transcript가 하나 추가된다 (§7.1)");
    assert_eq!(
        transcripts[0].segments[0].start_ms, 1_000,
        "센티초 100은 밀리초 1000이다 — 자동 경로에서도 단위가 바뀌지 않는다"
    );
}

#[test]
fn a_stop_starts_nothing_when_automatic_transcription_is_off() {
    let fixture = Fixture::new("off");
    fixture.save_settings(Settings {
        automatic_transcription: false,
        transcription_model: Some(MODEL_FILE.to_string()),
        ..Settings::DEFAULT
    });

    let stopped = fixture.record_and_stop();
    thread::sleep(SETTLE);

    assert_eq!(
        fixture.transcriber.status().expect("상태를 읽는다").state,
        "idle",
        "꺼져 있으면 정지가 전사를 걸지 않는다"
    );
    assert!(
        fixture.transcripts(&stopped.recording.id).is_empty(),
        "걸리지 않은 전사가 Transcript를 만들 수는 없다"
    );
    // 녹음 자체는 그대로 저장돼 있다 — 전사를 걸지 않는 것과 녹음이 실패하는 것은 다르다.
    assert_eq!(
        fixture
            .storage
            .list_recordings()
            .expect("목록을 읽는다")
            .len(),
        1
    );
}

#[test]
fn the_default_settings_do_not_start_a_transcription_after_a_stop() {
    // 아무것도 저장한 적 없는 앱이다. 기본값이 OFF이므로 여기서도 전사는 걸리지 않는다.
    let fixture = Fixture::new("default");
    fixture.save_settings(Settings {
        transcription_model: Some(MODEL_FILE.to_string()),
        ..Settings::DEFAULT
    });
    assert!(
        !fixture.settings().automatic_transcription,
        "사전 조건: 기본값은 OFF다"
    );

    let stopped = fixture.record_and_stop();
    thread::sleep(SETTLE);

    assert_eq!(
        fixture.transcriber.status().expect("상태를 읽는다").state,
        "idle"
    );
    assert!(fixture.transcripts(&stopped.recording.id).is_empty());
}

#[test]
fn automatic_processing_is_a_different_toggle_and_does_not_start_a_transcription() {
    // 하나의 boolean에 두 의미가 겹치지 않는다. 후처리 토글을 켠 것은 전사를 켠 것이 아니다.
    let fixture = Fixture::new("other-toggle");
    fixture.save_settings(Settings {
        automatic_processing: true,
        automatic_transcription: false,
        transcription_model: Some(MODEL_FILE.to_string()),
        ..Settings::DEFAULT
    });

    let stopped = fixture.record_and_stop();
    thread::sleep(SETTLE);

    assert_eq!(
        fixture.transcriber.status().expect("상태를 읽는다").state,
        "idle",
        "automatic_processing이 자동 전사를 켜지 않는다"
    );
    assert!(fixture.transcripts(&stopped.recording.id).is_empty());
    assert!(
        fixture.settings().automatic_processing,
        "다른 토글의 값이 이 경로 때문에 바뀌지도 않는다"
    );
}

// --- 수동 전사는 설정과 무관하다 -------------------------------------------------------

#[test]
fn a_manual_transcription_works_while_automatic_transcription_is_off() {
    // 자동으로 시작하지 않는 것과 전사할 수 없는 것은 다른 말이다.
    let fixture = Fixture::new("manual");
    fixture.save_settings(Settings {
        automatic_transcription: false,
        transcription_model: Some(MODEL_FILE.to_string()),
        ..Settings::DEFAULT
    });

    let stopped = fixture.record_and_stop();
    thread::sleep(SETTLE);
    assert_eq!(
        fixture.transcriber.status().expect("상태를 읽는다").state,
        "idle",
        "사전 조건: 자동으로는 걸리지 않았다"
    );

    // 사용자가 직접 건다. 이 경로는 설정을 보지 않는다.
    fixture
        .transcriber
        .start(&stopped.recording.id)
        .expect("수동 전사는 설정과 무관하게 시작할 수 있어야 한다");
    let finished = fixture.wait_for_finish();

    assert_eq!(finished.state, "done");
    assert_eq!(fixture.transcripts(&stopped.recording.id).len(), 1);
    assert!(
        !fixture.settings().automatic_transcription,
        "수동 전사가 설정을 켜 두지 않는다"
    );
}

// --- 모델이 없는 상태 (ADR-0007 §8.2.3) -----------------------------------------------

#[test]
fn a_missing_model_is_reported_as_a_failure_and_the_toggle_is_left_as_the_user_set_it() {
    // 앱이 사용자의 설정을 대신 고치지 않는다. 실행할 수 없다는 사실은 §13의 실패로 드러난다.
    let fixture = Fixture::without_model("no-model");
    fixture.save_settings(Settings {
        automatic_transcription: true,
        ..Settings::DEFAULT
    });

    let stopped = fixture.record_and_stop();
    let finished = fixture.wait_for_finish();

    assert_eq!(finished.state, "failed");
    let failure = finished.failure.expect("실패가 상태에 실려 와야 한다");
    assert_eq!(failure.kind, FailureKind::TranscriptionModelMissing);
    assert!(!failure.retryable, "모델을 먼저 둬야 풀린다");
    assert!(failure.source_data_safe, "원본은 그대로다");

    let settings = fixture.settings();
    assert!(
        settings.automatic_transcription,
        "모델이 없다는 이유로 앱이 토글을 뒤집지 않는다"
    );
    assert_eq!(
        settings.transcription_model, None,
        "모델을 대신 골라 주지도 않는다"
    );

    // 녹음도 파일도 그대로 남는다 (INV-1 · INV-3).
    assert!(Path::new(&stopped.recording.audio_path).is_file());
    assert!(fixture.transcripts(&stopped.recording.id).is_empty());
}

#[test]
fn a_failed_stop_does_not_start_a_transcription() {
    // 전사는 **저장된 녹음**에 대해서만 걸린다. 확인되지 않은 파일을 전사하지 않는다.
    let fixture = Fixture::new("failed-stop");
    fixture.save_settings(Settings {
        automatic_transcription: true,
        transcription_model: Some(MODEL_FILE.to_string()),
        ..Settings::DEFAULT
    });

    // 소리 없는 녹음이다 — 정지가 실패한다 (R-002).
    fixture
        .recorder
        .start(DEVICE_KEY)
        .expect("녹음을 시작할 수 있어야 한다");
    fixture.clock.advance(1_000);
    finish_recording(&fixture.recorder, &fixture.storage, &fixture.transcriber, None)
        .expect_err("사전 조건: 빈 녹음은 성공이 아니다");
    thread::sleep(SETTLE);

    assert_eq!(
        fixture.transcriber.status().expect("상태를 읽는다").state,
        "idle",
        "정지가 실패했으면 전사도 걸리지 않는다"
    );
    assert!(
        fixture
            .storage
            .list_recordings()
            .expect("목록을 읽는다")
            .is_empty(),
        "사전 조건: 저장된 녹음이 없다"
    );
}
