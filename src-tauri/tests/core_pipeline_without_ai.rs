//! **AI가 없어도 제품은 제품이다** (INV-8 · `phase-prompt/04` 성공 기준 2).
//!
//! `tests/ai_note_commands.rs`가 AI 경계 쪽에서 "provider 없음은 실패가 아니다"를 판정한다면,
//! 이 파일은 **반대쪽에서** 같은 불변식을 판정한다 — AI가 하나도 설정되지 않았을 때 핵심 세
//! 경로가 실제로 끝까지 도는가.
//!
//! ```text
//! 1. 녹음      시작 → 정지 → 파일 확정 → 레코드 영속화        (R-002)
//! 2. 전사      정지 뒤의 orchestration → Transcript 추가       (§7.1 · §7.2)
//! 3. 열람      Recordings 목록 · Recording 상세 · Transcript   (§5 A · §5 C)
//! ```
//!
//! 그리고 그 셋이 **세 가지 AI 상태 어디에서도 막히지 않는다**는 것까지 본다.
//!
//! ```text
//! provider 미설정   고른 provider가 아예 없다        `notConfigured` — failure가 없다
//! 연결 불가          고른 provider가 응답하지 않는다  §13의 실패이지만 AI 밖으로 번지지 않는다
//! 생성 실패          노트를 만들다 실패했다           같음
//! ```
//!
//! **실제 Whisper도 실제 AI 서버도 네트워크도 요구하지 않는다** (§18 · A-TRANS-001).
//! 대체 구현이 들어가는 자리는 넷뿐이다 — 마이크 · 시계 · 전사 엔진 · AI provider. 파일도
//! 저장소도 정지 경로도 제품 코드 그대로 실행된다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다. Tauri 런타임도 창도 하드웨어도 없다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use molt_note_lib::ai::testing::FakeNoteAiProvider;
use molt_note_lib::ai::NoteAiProvider;
use molt_note_lib::audio::{CaptureFormat, OpenCapture, SampleSink, SampleSource};
use molt_note_lib::commands::{
    finish_recording, AiNoteStatusPayload, NoteGenerator, Recorder, Storage,
    StoppedRecordingPayload, Transcriber, TranscriptionStatusPayload,
};
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{
    Failure, FailureKind, NoteType, ProcessingStatus, RecordingId, Settings, Transcript,
};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::clock::Clock;
use molt_note_lib::transcription::{RawSegment, RawTranscription, StubEngine};

/// 가짜 마이크는 어떤 키를 줘도 같은 값을 낸다.
const DEVICE_KEY: &str = "0:가짜 마이크";

/// 모델 디렉터리에 두는 자리표시자 파일의 이름. **실제 whisper 모델이 아니다** (A-TRANS-001).
const MODEL_FILE: &str = "ggml-base.bin";

/// 한 번에 보내는 샘플 수. 확정된 파일이 유효 최소치를 넘도록 넉넉히 보낸다.
const CHUNK: usize = 1_600;

/// 배경 작업이 끝나기를 기다리는 한계. 넘으면 테스트가 실패한다 — 매달리지 않는다.
const FINISH_TIMEOUT: Duration = Duration::from_secs(30);

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 자리 ----------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-core-without-ai-{}-{}-{}",
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
            format: CaptureFormat::pcm_16bit(16_000, 1),
            stop: Box::new(move || {
                *sink.lock().expect("통로를 놓을 수 있어야 한다") = None;
                Ok(())
            }),
        })
    }
}

/// 녹음 · 전사 · 열람이 한 자리에서 일어나는 앱 하나.
///
/// **AI 노트 실행자도 함께 들고 있다.** 이 파일의 요지가 "AI가 있든 없든 나머지가 돈다"이므로
/// 둘이 같은 앱 데이터 디렉터리를 공유해야 의미가 있다.
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
    /// 정지 직후에 전사가 걸리는 앱 하나. **AI 설정은 하나도 저장하지 않는다** (INV-8).
    fn new(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.path().join("app-data"));
        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

        let models_dir = app_data_dir
            .ensure_models_dir()
            .expect("사전 조건: 모델 디렉터리");
        std::fs::write(models_dir.join(MODEL_FILE), b"not a real model")
            .expect("사전 조건: 자리표시자 모델을 둔다");

        let microphone = ControlledMicrophone::new();
        let clock = TestClock::default();
        let fixture = Self {
            recorder: Recorder::with_clock(app_data_dir.clone(), microphone.clone(), clock.clone()),
            transcriber: Transcriber::with_engine(
                app_data_dir.clone(),
                StubEngine::returning(spoken()),
            ),
            storage,
            microphone,
            clock,
            app_data_dir,
            _root: root,
        };

        // 정지 → 전사가 한 번에 이어지도록 자동 전사를 켠다. **AI 값은 건드리지 않는다** —
        // `Settings::DEFAULT`의 `ai_provider`·`ai_model`은 `None`이고 그것이 기본이자 정상 상태다.
        fixture.save_settings(Settings {
            automatic_transcription: true,
            transcription_model: Some(MODEL_FILE.to_string()),
            ..Settings::DEFAULT
        });
        fixture
    }

    fn save_settings(&self, settings: Settings) {
        settings::save(&self.connection(), &settings).expect("사전 조건: 설정을 저장한다");
    }

    fn settings(&self) -> Settings {
        settings::load(&self.connection()).expect("설정을 읽을 수 있어야 한다")
    }

    fn connection(&self) -> rusqlite::Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    /// **설정이 고른** provider로 노트를 만드는 실행자. 실제 앱이 지나는 경로 그대로다.
    fn configured_generator(&self) -> NoteGenerator {
        NoteGenerator::configured_in(self.app_data_dir.clone())
    }

    /// 주어진 provider로 노트를 만드는 실행자. 실제 앱과 경로는 같고 provider만 다르다.
    fn generator_with(&self, provider: impl NoteAiProvider + 'static) -> NoteGenerator {
        NoteGenerator::with_provider(self.app_data_dir.clone(), provider)
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

    fn transcripts(&self, recording_id: &str) -> Vec<Transcript> {
        store::list_transcripts(&self.connection(), &RecordingId::new(recording_id))
            .expect("Transcript 목록을 읽을 수 있어야 한다")
    }

    /// 전사가 끝날 때까지 상태를 물어보며 기다린다. 끝난 상태를 돌려준다.
    fn wait_for_transcription(&self) -> TranscriptionStatusPayload {
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

/// 노트 생성이 끝날 때까지 상태를 물어보며 기다린다.
fn wait_for_note(generator: &NoteGenerator) -> AiNoteStatusPayload {
    let deadline = Instant::now() + FINISH_TIMEOUT;
    loop {
        let status = generator.status().expect("상태를 물어볼 수 있어야 한다");
        if status.state != "running" {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "노트 생성이 끝나기를 기다리다 시간이 지났다"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

/// 엔진이 냈다고 가정하는 원시 출력. **센티초다** (ADR-0007 §10). 실제 추론 결과가 아니다.
fn spoken() -> RawTranscription {
    RawTranscription {
        language: Some("ko".to_owned()),
        segments: vec![RawSegment {
            start_centiseconds: 100,
            end_centiseconds: 240,
            text: Some(" AI 없이도 전사는 끝까지 간다".to_owned()),
        }],
    }
}

/// 이 파일이 말하는 "AI가 안 되는 세 상황". 셋 다 §13의 서로 다른 실패다.
fn ai_that_does_not_work() -> [(&'static str, FakeNoteAiProvider, FailureKind); 3] {
    [
        (
            "연결 불가",
            FakeNoteAiProvider::unreachable(),
            FailureKind::AiProviderUnreachable,
        ),
        (
            "모델 없음",
            FakeNoteAiProvider::without_models(),
            FailureKind::AiModelUnavailable,
        ),
        (
            "생성 실패 (schema 불일치)",
            FakeNoteAiProvider::generating_text("죄송하지만 JSON으로 답할 수 없습니다"),
            FailureKind::AiResponseUnusable,
        ),
    ]
}

/// 세 경로가 지금 이 앱에서 실제로 도는지 한 번에 확인한다.
///
/// **이 함수가 이 파일의 판정 그 자체다.** 어느 AI 상태에서 불러도 같은 답이 나와야 한다.
fn assert_the_three_paths_work(fixture: &Fixture, label: &str) {
    // 1. 녹음 — 시작 · 정지 · 파일 확정 · 레코드 영속화 (R-002).
    let stopped = fixture.record_and_stop();
    let audio = PathBuf::from(&stopped.recording.audio_path);

    assert!(audio.is_file(), "{label}: 확정된 파일이 있어야 한다: {audio:?}");
    assert_eq!(
        stopped.recording.audio_path, stopped.capture.output_path,
        "{label}: 레코드는 방금 확정된 그 파일을 가리킨다"
    );
    assert_eq!(
        stopped.capture.byte_size,
        std::fs::metadata(&audio)
            .expect("파일을 읽을 수 있어야 한다")
            .len(),
        "{label}: 보고된 크기는 파일시스템에서 읽은 값이다"
    );
    assert_eq!(stopped.recording.duration_ms, 5_000, "{label}");
    assert!(!stopped.recording.title.trim().is_empty(), "{label}");

    // 2. 전사 — 정지가 건 orchestration이 끝까지 간다 (§7.1 · §7.2).
    let transcribed = fixture.wait_for_transcription();
    assert_eq!(
        transcribed.state, "done",
        "{label}: 전사가 끝나야 한다 (failure={:?})",
        transcribed.failure
    );
    assert_eq!(
        transcribed.recording_id.as_deref(),
        Some(stopped.recording.id.as_str()),
        "{label}: 걸린 전사는 방금 저장된 그 녹음이다"
    );
    let transcript_id = transcribed
        .transcript_id
        .expect("추가된 Transcript를 가리켜야 한다");

    let stored = fixture.transcripts(&stopped.recording.id);
    assert_eq!(stored.len(), 1, "{label}: Transcript가 하나 추가된다");
    assert_eq!(
        stored[0].segments[0].start_ms, 1_000,
        "{label}: 센티초 100은 밀리초 1000이다 — AI 상태가 단위를 바꾸지 않는다"
    );

    // 3. 열람 — 목록 · 상세 · Transcript 탭이 읽는 값 전부.
    let listed = fixture
        .storage
        .list_recordings()
        .expect("목록을 읽을 수 있어야 한다");
    let detail = fixture
        .storage
        .recording(&stopped.recording.id)
        .expect("상세를 읽을 수 있어야 한다")
        .expect("방금 저장한 녹음이 있어야 한다");
    let tab = fixture
        .storage
        .transcript(&transcript_id)
        .expect("Transcript를 읽을 수 있어야 한다")
        .expect("추가된 Transcript가 있어야 한다");
    let notes = fixture
        .storage
        .ai_notes(&transcript_id)
        .expect("노트 목록을 읽는 것도 실패가 아니다");

    assert!(
        listed.iter().any(|item| item.id == stopped.recording.id),
        "{label}: 방금 저장한 녹음이 목록에 있다"
    );
    assert_eq!(detail.transcription_status, "done", "{label}");
    assert_eq!(
        detail.current_transcript_id.as_deref(),
        Some(transcript_id.as_str()),
        "{label}: current가 방금 만든 Transcript를 가리킨다 (§7.2)"
    );
    assert_eq!(tab.segments.len(), 1, "{label}: 문장을 읽을 수 있다");
    assert!(!tab.raw_text.trim().is_empty(), "{label}");
    assert!(
        !tab.engine.trim().is_empty() && !tab.model.trim().is_empty(),
        "{label}: 무엇으로 만들어진 문장인지가 남는다 (provenance · §7)"
    );
    assert!(
        notes.is_empty(),
        "{label}: 노트가 없는 것은 빈 목록이지 실패가 아니다"
    );
}

// --- provider를 고르지 않았다 (INV-8) ----------------------------------------------------

#[test]
fn recording_transcription_and_reading_all_run_with_no_ai_provider_configured() {
    // 이 Phase의 성공 기준 2 그 자체다. 아무 AI 설정도 저장한 적 없는 앱에서 세 경로가 끝까지 간다.
    let fixture = Fixture::new("no-provider");
    assert_eq!(
        fixture.settings().ai_provider,
        None,
        "사전 조건: 고른 provider가 없다"
    );
    assert_eq!(fixture.settings().ai_model, None, "사전 조건: 고른 모델도 없다");

    assert_the_three_paths_work(&fixture, "provider 미설정");

    // 그리고 그 상태는 여전히 "고르지 않음"이다 — 이 경로들이 무엇도 대신 골라 주지 않는다.
    assert_eq!(fixture.settings().ai_provider, None);
}

#[test]
fn having_no_provider_is_a_normal_state_rather_than_an_error() {
    // INV-8을 타입으로 적은 자리다 — `provider_status`에는 실패 채널이 아예 없다.
    let fixture = Fixture::new("state-not-error");
    let generator = fixture.configured_generator();

    let status = generator.provider_status(&fixture.settings());

    assert_eq!(status.state, "notConfigured");
    assert_eq!(status.failure, None, "고르지 않은 것은 실패가 아니다");
    assert_eq!(status.provider_id, None);
    assert_eq!(status.provider_name, None);
    assert_eq!(status.locality, None, "나가는 곳이 없으므로 말할 것도 없다");
    assert!(status.models.is_empty());
}

#[test]
fn asking_for_a_note_without_a_provider_leaves_the_recording_and_its_transcript_untouched() {
    // §13: 사용자가 그 상태에서 굳이 눌렀을 때의 답이다. 조용히 무시되지도, 원본을 건드리지도 않는다.
    let fixture = Fixture::new("start-without-provider");
    let stopped = fixture.record_and_stop();
    let transcribed = fixture.wait_for_transcription();
    assert_eq!(transcribed.state, "done", "사전 조건: 전사가 끝나 있다");

    let before = fixture.transcripts(&stopped.recording.id);
    let audio_before = std::fs::read(&stopped.recording.audio_path).expect("오디오를 읽는다");

    let generator = fixture.configured_generator();
    generator
        .start(&stopped.recording.id, NoteType::Meeting)
        .expect("provider가 없다는 이유로 거절하지 않는다");
    let finished = wait_for_note(&generator);

    assert_eq!(finished.state, "failed");
    assert_eq!(
        finished.failure.map(|failure| failure.kind),
        Some(FailureKind::AiProviderNotConfigured)
    );

    // 아무것도 시도하지 못했으므로 레코드에 실패를 남기지 않는다.
    let after = fixture
        .storage
        .recording(&stopped.recording.id)
        .expect("상세를 읽는다")
        .expect("녹음이 남아 있어야 한다");
    assert_eq!(after.ai_status, "none", "한 적 없는 실패가 녹음에 붙지 않는다");
    assert_eq!(after.transcription_status, "done", "남의 상태를 옮기지 않는다");
    assert_eq!(fixture.transcripts(&stopped.recording.id), before, "전사가 그대로다");
    assert_eq!(
        std::fs::read(&stopped.recording.audio_path).expect("오디오를 읽는다"),
        audio_before,
        "원본 오디오도 그대로다 (INV-1)"
    );
}

// --- AI가 안 되는 것이 나머지를 막지 않는다 (INV-8 · §13) ---------------------------------

#[test]
fn an_ai_provider_that_does_not_work_blocks_none_of_the_three_paths() {
    // 연결 불가 · 모델 없음 · 생성 실패 — 셋 다 §13의 실패이지만, 그 실패는 AI 상태 하나에
    // 머문다. 실패를 **실제로 겪은 뒤에** 세 경로를 다시 처음부터 돌린다.
    for (label, provider, expected) in ai_that_does_not_work() {
        let fixture = Fixture::new("ai-broken");

        // 먼저 노트를 만들 재료를 갖춘 녹음 하나를 만든다.
        let stopped = fixture.record_and_stop();
        assert_eq!(
            fixture.wait_for_transcription().state,
            "done",
            "{label}: 사전 조건 — 전사가 끝나 있다"
        );

        // 그리고 AI를 실제로 실패시킨다.
        let generator = fixture.generator_with(provider);
        generator
            .start(&stopped.recording.id, NoteType::Meeting)
            .expect("시작은 provider의 상태와 무관하게 접수된다");
        let finished = wait_for_note(&generator);

        assert_eq!(finished.state, "failed", "{label}");
        let failure = finished.failure.expect("무엇이 막혔는지 말한다");
        assert_eq!(failure.kind, expected, "{label}");
        assert!(failure.source_data_safe, "{label}: 원본은 안전하다 (INV-3)");
        assert_eq!(
            fixture
                .storage
                .recording(&stopped.recording.id)
                .expect("상세를 읽는다")
                .expect("녹음이 있어야 한다")
                .ai_status,
            "failed",
            "{label}: 실패가 상태로 남는다 (§13)"
        );

        // 실패한 **뒤에** 세 경로를 처음부터 다시 돌린다 — 새 녹음 · 새 전사 · 새 열람이다.
        assert_the_three_paths_work(&fixture, label);

        // 앞선 녹음도 그대로 남아 있다. AI 실패가 다른 녹음을 흔들지 않는다.
        let listed = fixture.storage.list_recordings().expect("목록을 읽는다");
        assert_eq!(listed.len(), 2, "{label}: 두 녹음이 함께 남는다");
        assert!(
            listed.iter().any(|item| item.id == stopped.recording.id),
            "{label}: 실패를 겪은 녹음도 목록에 그대로 있다"
        );
    }
}

#[test]
fn a_recording_that_never_touched_ai_keeps_the_untried_ai_status() {
    // `none`은 "아직 시도하지 않았다"는 정상 상태이며 오류가 아니다 (§7 · INV-8).
    let fixture = Fixture::new("untried");

    let stopped = fixture.record_and_stop();
    assert_eq!(fixture.wait_for_transcription().state, "done");

    let recording = store::load_recording(
        &fixture.connection(),
        &RecordingId::new(&stopped.recording.id),
    )
    .expect("녹음을 읽을 수 있어야 한다")
    .expect("녹음이 남아 있어야 한다");

    assert_eq!(recording.ai_status, ProcessingStatus::None);
    assert_eq!(recording.transcription_status, ProcessingStatus::Done);
    assert_eq!(recording.notion_status, ProcessingStatus::None);
}
