//! AI 노트 **command 경계**의 계약 (`phase-prompt/04` 요구 2 · 8 · 9 · 15 · 16 ·
//! PRODUCT-SPEC §12 · §13 · INV-5 · INV-8 · INV-9 · `docs/ADR-0008-note-ai-provider.md`).
//!
//! 여기서 판정하는 것은 노트의 품질이 아니라 **경계의 성질** 넷이다.
//!
//! ```text
//! 1. provider가 없거나 닿지 않는 것이 실패가 아니라 상태값이다        (INV-8 · §13)
//! 2. 그 상태에서도 녹음 · 전사 · 열람이 그대로 동작한다              (INV-8)
//! 3. 생성이 도는 동안 다른 command와 상태 조회가 계속 답한다          (요구 16 · R-001)
//! 4. 로컬/외부 구분과 provenance가 화면까지 그대로 도달한다           (INV-5 · §7.3)
//! ```
//!
//! **실제 AI 서버도 모델도 실제 Whisper 추론 결과도 요구하지 않는다** (§18 · A-TRANS-001).
//! provider 자리에는 계약이 같은 test double이 서고, Transcript는 손으로 쓴 fixture다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다. Tauri 런타임도 창도 하드웨어도 없다.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use molt_note_lib::ai::testing::{
    sample_note, FakeNoteAiProvider, FAKE_MODEL_ID, FAKE_PROVIDER_ID,
};
use molt_note_lib::ai::{
    Availability, Locality, NoteAiProvider, NoteGeneration, NoteRequest, ProviderDescriptor,
};
use molt_note_lib::commands::{
    AiNoteStatusPayload, NoteGenerator, Storage, StructuredNotePayload,
};
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{
    Failure, FailureKind, NoteType, ProcessingStatus, Recording, RecordingId, Settings, Transcript,
    TranscriptId, TranscriptSegment,
};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;

/// 배경 스레드가 끝나기를 기다리는 한계. 넘으면 테스트가 실패한다 — 매달리지 않는다.
const FINISH_TIMEOUT: Duration = Duration::from_secs(30);

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 멈춰 있는 provider ----------------------------------------------------------------

/// 생성 도중에 **멈춰 서는** provider.
///
/// 로컬 모델의 생성이 오래 걸린다는 사실을 그대로 흉내 낸다 — 다만 걸리는 시간이 테스트의
/// 통제 아래 있다. [`Controls::wait_until_inside`]가 돌아온 시점에 이 provider는 확실히
/// `generate_note` 안에 있고, [`Controls::release`] 전까지 거기서 나오지 않는다.
struct GatedProvider {
    inner: Arc<Gate>,
}

struct Gate {
    entered: Mutex<Sender<()>>,
    release: Mutex<Receiver<()>>,
    calls: Mutex<usize>,
}

/// 멈춰 있는 provider를 조종하는 손잡이. 테스트 스레드가 들고 있는다.
struct Controls {
    entered: Receiver<()>,
    release: Sender<()>,
}

impl GatedProvider {
    fn new() -> (Self, Controls) {
        let (entered_tx, entered_rx) = channel();
        let (release_tx, release_rx) = channel();

        let provider = Self {
            inner: Arc::new(Gate {
                entered: Mutex::new(entered_tx),
                release: Mutex::new(release_rx),
                calls: Mutex::new(0),
            }),
        };
        let controls = Controls {
            entered: entered_rx,
            release: release_tx,
        };

        (provider, controls)
    }

    fn handle(&self) -> Arc<Gate> {
        Arc::clone(&self.inner)
    }
}

impl Gate {
    fn call_count(&self) -> usize {
        *self.calls.lock().expect("호출 기록을 잠근다")
    }
}

impl Controls {
    /// provider가 생성 안으로 들어갈 때까지 기다린다.
    ///
    /// **이 호출이 돌아온 뒤부터가 관찰 구간이다** — provider는 아직 노트를 내지 않았다.
    fn wait_until_inside(&self) {
        self.entered
            .recv_timeout(FINISH_TIMEOUT)
            .expect("provider가 생성에 들어가야 한다");
    }

    fn release(&self) {
        self.release.send(()).expect("provider가 아직 살아 있어야 한다");
    }
}

impl NoteAiProvider for GatedProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            // 실제 벤더 식별자와 겹치지 않는다. 이 값이 저장된 행은 테스트가 만든 것이다.
            id: "gated-note-ai-provider".to_owned(),
            name: "멈춰 있는 테스트 provider".to_owned(),
            locality: Locality::Local,
        }
    }

    fn availability(&self) -> Availability {
        Availability::Ready {
            models: vec!["gated-model".to_owned()],
        }
    }

    fn generate_note(&self, request: &NoteRequest<'_>) -> Result<NoteGeneration, Failure> {
        *self.inner.calls.lock().expect("호출 기록을 잠근다") += 1;

        self.inner
            .entered
            .lock()
            .expect("신호 통로를 잠근다")
            .send(())
            .expect("테스트가 신호를 받고 있어야 한다");
        self.inner
            .release
            .lock()
            .expect("신호 통로를 잠근다")
            .recv_timeout(FINISH_TIMEOUT)
            .expect("테스트가 provider를 풀어 줘야 한다");

        Ok(NoteGeneration {
            note: sample_note(request.mode),
            model: "gated-model".to_owned(),
        })
    }
}

// --- 자리 --------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-ai-note-commands-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
        Self(path)
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// "오디오"로 둘 바이트. AI 경로는 이 파일을 열지 않으므로 내용은 아무것이나 좋다 (INV-6).
const AUDIO_BYTES: &[u8] = b"molt-note fixture audio bytes";

/// 노트를 걸 수 있는 앱 데이터 디렉터리 하나 — DB · 녹음 레코드 · 오디오 파일 · Transcript.
struct Fixture {
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    recording_id: String,
    transcript_id: TranscriptId,
    audio_path: PathBuf,
}

impl Fixture {
    /// current Transcript를 가진 Recording 하나 (§7.2의 기본 입력).
    fn new(label: &str) -> Self {
        let fixture = Self::without_transcript(label);
        fixture.append_transcript();
        fixture
    }

    /// Transcript가 하나도 없는 Recording — current도 없다 (§7.2의 정상 상태).
    fn without_transcript(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.0.join("app-data"));
        app_data_dir.ensure().expect("사전 조건: 앱 데이터 디렉터리");

        let recordings_dir = app_data_dir
            .ensure_recordings_dir()
            .expect("사전 조건: 녹음 디렉터리");
        let audio_path = recordings_dir.join("recording.wav");
        fs::write(&audio_path, AUDIO_BYTES).expect("사전 조건: 오디오 파일을 만든다");

        let fixture = Self {
            _root: root,
            app_data_dir,
            recording_id: "rec-ai-note".to_owned(),
            transcript_id: TranscriptId::new("tr-ai-note"),
            audio_path,
        };

        store::insert_recording(
            &fixture.connection(),
            &Recording {
                id: RecordingId::new(&fixture.recording_id),
                title: "3DGS Study #04".to_owned(),
                created_at: "2026-09-03T10:00:00.000Z".to_owned(),
                updated_at: "2026-09-03T10:00:00.000Z".to_owned(),
                duration_ms: 3_151_000,
                audio_path: fixture.audio_path.to_str().expect("경로 문자열").to_owned(),
                audio_format: "wav".to_owned(),
                microphone: Some("MacBook Pro Microphone".to_owned()),
                current_transcript_id: None,
                // 전사는 이미 끝난 상태다. AI가 이 값을 옮기지 않는 것이 확인 대상이다.
                transcription_status: ProcessingStatus::Done,
                ai_status: ProcessingStatus::None,
                notion_status: ProcessingStatus::None,
            },
        )
        .expect("사전 조건: 녹음 레코드를 저장한다");

        fixture
    }

    /// fixture Transcript 하나를 추가하고 current로 지정한다. **실제 전사 결과가 아니다**
    /// (A-TRANS-001).
    fn append_transcript(&self) {
        let mut connection = self.connection();
        store::append_transcript(
            &mut connection,
            &Transcript {
                id: self.transcript_id.clone(),
                recording_id: RecordingId::new(&self.recording_id),
                language: Some("ko".to_owned()),
                segments: vec![TranscriptSegment {
                    start_ms: 0,
                    end_ms: 2_000,
                    text: "오늘 회의에서는 다음 분기 일정을 이야기했다".to_owned(),
                }],
                raw_text: "오늘 회의에서는 다음 분기 일정을 이야기했다".to_owned(),
                created_at: "2026-09-03T10:01:00.000Z".to_owned(),
                engine: "fixture-engine".to_owned(),
                model: "fixture-model".to_owned(),
            },
        )
        .expect("사전 조건: Transcript를 추가한다");

        store::set_current_transcript(
            &connection,
            &RecordingId::new(&self.recording_id),
            Some(&self.transcript_id),
            "2026-09-03T10:02:00.000Z",
        )
        .expect("사전 조건: current Transcript를 지정한다");
    }

    /// 설정 값을 저장한다. 사용자가 설정 화면에서 고른 것과 같은 자리다.
    fn save_settings(&self, settings: Settings) {
        settings::save(&self.connection(), &settings).expect("사전 조건: 설정을 저장한다");
    }

    /// **설정이 고른** provider로 노트를 만드는 실행자. 실제 앱이 지나는 경로 그대로다.
    fn configured_generator(&self) -> NoteGenerator {
        NoteGenerator::configured_in(self.app_data_dir.clone())
    }

    /// 주어진 provider로 노트를 만드는 실행자. 실제 앱과 경로는 같고 provider만 다르다.
    fn generator_with(&self, provider: impl NoteAiProvider + 'static) -> NoteGenerator {
        NoteGenerator::with_provider(self.app_data_dir.clone(), provider)
    }

    /// 앱이 들고 있는 것과 같은 저장소. **생성과 별개의 연결이다.**
    fn storage(&self) -> Storage {
        let storage = Storage::open(&self.app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");
        storage
    }

    fn connection(&self) -> rusqlite::Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    fn recording(&self) -> Recording {
        store::load_recording(&self.connection(), &RecordingId::new(&self.recording_id))
            .expect("녹음을 읽을 수 있어야 한다")
            .expect("녹음이 남아 있어야 한다")
    }

    fn transcript(&self) -> Transcript {
        store::load_transcript(&self.connection(), &self.transcript_id)
            .expect("Transcript를 읽을 수 있어야 한다")
            .expect("Transcript가 남아 있어야 한다")
    }

    fn audio_bytes(&self) -> Vec<u8> {
        fs::read(&self.audio_path).expect("오디오 파일을 읽을 수 있어야 한다")
    }
}

/// 생성이 끝날 때까지 상태를 물어보며 기다린다. 끝난 상태를 돌려준다.
fn wait_for_finish(generator: &NoteGenerator) -> AiNoteStatusPayload {
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

// --- provider가 없는 것은 오류가 아니다 (INV-8) ----------------------------------------

#[test]
fn a_provider_that_was_never_chosen_is_a_state_not_a_failure() {
    let fixture = Fixture::new("not-configured");
    let generator = fixture.configured_generator();

    // 이 호출에는 실패 채널이 아예 없다 — `Result`가 아니다. 그것이 INV-8을 타입으로 적는 방법이다.
    let status = generator.provider_status(&Settings::DEFAULT);

    assert_eq!(status.state, "notConfigured");
    assert_eq!(status.failure, None, "고르지 않은 것은 실패가 아니다");
    assert_eq!(status.provider_id, None);
    assert_eq!(status.provider_name, None);
    assert_eq!(status.locality, None, "나가는 곳이 없으므로 말할 것도 없다");
    assert!(status.models.is_empty());
}

#[test]
fn an_identifier_this_app_cannot_build_is_not_swapped_for_another_provider() {
    // 알아볼 수 없는 값이 저장돼 있어도 다른 provider를 대신 고르지 않는다 (INV-9 · §12).
    let fixture = Fixture::new("unknown-identifier");
    let generator = fixture.configured_generator();

    let status = generator.provider_status(&Settings {
        ai_provider: Some("어떤-다른-provider".to_owned()),
        ..Settings::DEFAULT
    });

    assert_eq!(status.state, "notConfigured");
    assert_eq!(status.provider_id, None);
    assert_eq!(status.failure, None);
}

#[test]
fn recording_transcript_and_reading_all_work_with_no_ai_provider_at_all() {
    // 이 Phase의 가장 중요한 검증 항목 중 하나다 (phase-prompt/04 Verification Boundary).
    let fixture = Fixture::new("core-pipeline-without-ai");
    let storage = fixture.storage();

    let recordings = storage.list_recordings().expect("목록을 읽을 수 있어야 한다");
    let transcript = storage
        .transcript(fixture.transcript_id.as_str())
        .expect("Transcript를 읽을 수 있어야 한다")
        .expect("저장된 Transcript가 있어야 한다");
    let settings = storage.settings().expect("설정을 읽을 수 있어야 한다");
    let notes = storage
        .ai_notes(fixture.transcript_id.as_str())
        .expect("노트 목록을 읽는 것도 실패가 아니다");

    assert_eq!(recordings.len(), 1);
    assert_eq!(recordings[0].ai_status, "none", "시도한 적 없음이 정상 상태다");
    assert_eq!(transcript.segments.len(), 1);
    assert_eq!(settings.ai_provider, None, "고르지 않은 것이 기본값이다");
    assert!(notes.is_empty(), "노트가 없는 것은 빈 목록이지 실패가 아니다");
}

#[test]
fn asking_for_a_note_without_a_provider_is_accepted_and_answered_as_a_state() {
    // §13: 사용자가 그 상태에서 굳이 생성을 요청했을 때의 답이다. **command의 Err가 아니다** —
    // 조용히 무시되지도 않는다.
    let fixture = Fixture::new("start-without-provider");
    let generator = fixture.configured_generator();

    let accepted = generator
        .start(&fixture.recording_id, NoteType::Meeting)
        .expect("provider가 없다는 이유로 거절하지 않는다");
    assert_eq!(accepted.state, "running", "시작은 접수 사실을 돌려준다");

    let finished = wait_for_finish(&generator);
    let failure = finished.failure.expect("무엇이 막혔는지 말한다");

    assert_eq!(finished.state, "failed");
    assert_eq!(failure.kind, FailureKind::AiProviderNotConfigured);
    assert!(!failure.retryable, "설정에서 골라야 풀린다");
    assert!(failure.source_data_safe, "아무것도 건드리지 않았다");
    assert!(!failure.message.trim().is_empty(), "그대로 화면에 띄울 수 있다");

    // 아무것도 시도하지 못했으므로 레코드에 실패를 남기지 않는다 — 사용자가 한 적 없는 실패가
    // 녹음에 붙지 않는다 (`ai::run`의 NoTranscriptYet과 같은 규칙).
    assert_eq!(fixture.recording().ai_status, ProcessingStatus::None);
    assert_eq!(
        fixture.recording().transcription_status,
        ProcessingStatus::Done,
        "남의 파이프라인 상태도 그대로다"
    );
    assert_eq!(fixture.audio_bytes(), AUDIO_BYTES, "원본 오디오도 그대로다 (INV-1)");
}

#[test]
fn a_chosen_provider_without_a_chosen_model_does_not_pick_one_by_itself() {
    // 모델을 대신 골라 주면 provenance가 기록이 아니라 추정이 된다 (§7.3 · ADR-0008 §11.1).
    let fixture = Fixture::new("no-model-chosen");
    fixture.save_settings(Settings {
        ai_provider: Some(molt_note_lib::ai::ollama::PROVIDER_ID.to_owned()),
        ai_model: None,
        ..Settings::DEFAULT
    });
    let generator = fixture.configured_generator();

    generator
        .start(&fixture.recording_id, NoteType::Summary)
        .expect("시작은 접수된다");
    let finished = wait_for_finish(&generator);

    assert_eq!(finished.state, "failed");
    assert_eq!(
        finished.failure.map(|failure| failure.kind),
        Some(FailureKind::AiProviderNotConfigured),
    );
    assert_eq!(fixture.recording().ai_status, ProcessingStatus::None);
}

// --- provider가 자기 자신에 대해 말하는 것이 화면까지 간다 (INV-5) ------------------------

#[test]
fn whether_the_transcript_leaves_the_device_reaches_the_screen() {
    // 사용자가 알아야 하는 것은 "데이터가 나가는가"이고, 그 답을 provider가 스스로 말한다
    // (§12 · 요구 15). 화면이 그것을 표시하려면 값이 이 경계를 지나야 한다.
    let fixture = Fixture::new("locality");

    let local = fixture
        .generator_with(FakeNoteAiProvider::ready())
        .provider_status(&Settings::DEFAULT);
    let external = fixture
        .generator_with(FakeNoteAiProvider::ready().with_locality(Locality::External))
        .provider_status(&Settings::DEFAULT);

    assert_eq!(local.locality.as_deref(), Some("local"));
    assert_eq!(external.locality.as_deref(), Some("external"));
    assert_ne!(local.locality, external.locality, "둘이 구분되지 않으면 소용없다");

    assert_eq!(local.state, "ready");
    assert_eq!(local.provider_id.as_deref(), Some(FAKE_PROVIDER_ID));
    assert_eq!(local.models, vec![FAKE_MODEL_ID.to_owned()]);
    assert_eq!(local.failure, None);
}

#[test]
fn a_server_that_does_not_answer_and_a_server_without_models_are_different_states() {
    // §13: 서버를 켜는 것과 모델을 받는 것은 사용자가 할 일이 다르다. 하나의 boolean으로
    // 뭉개면 화면이 구분해 안내할 수 없다 (ADR-0008 §4.2).
    let fixture = Fixture::new("availability-states");

    let unreachable = fixture
        .generator_with(FakeNoteAiProvider::unreachable())
        .provider_status(&Settings::DEFAULT);
    let without_models = fixture
        .generator_with(FakeNoteAiProvider::without_models())
        .provider_status(&Settings::DEFAULT);

    assert_eq!(unreachable.state, "unavailable");
    let failure = unreachable.failure.expect("왜 닿지 못하는지 말한다");
    assert_eq!(failure.kind, FailureKind::AiProviderUnreachable);
    assert!(failure.retryable, "서버를 켜고 다시 물어보면 된다");
    assert!(unreachable.models.is_empty());
    // 닿지 못하는 상태에서도 무엇을 골랐는지는 여전히 말할 수 있다.
    assert_eq!(unreachable.provider_id.as_deref(), Some(FAKE_PROVIDER_ID));
    assert_eq!(unreachable.locality.as_deref(), Some("local"));

    assert_eq!(without_models.state, "noModels");
    assert_eq!(without_models.failure, None, "응답했다 — 실패가 아니다");
    assert!(without_models.models.is_empty());
}

// --- 만들어진 노트가 화면까지 온다 (§7.3) -----------------------------------------------

#[test]
fn a_generated_note_comes_back_through_the_command_surface_with_its_provenance() {
    let fixture = Fixture::new("generated-note");
    let generator = fixture.generator_with(FakeNoteAiProvider::ready());

    generator
        .start(&fixture.recording_id, NoteType::Meeting)
        .expect("생성을 시작할 수 있어야 한다");
    let finished = wait_for_finish(&generator);

    assert_eq!(finished.state, "done");
    assert_eq!(finished.mode.as_deref(), Some("meeting"));
    let note_id = finished.ai_note_id.expect("새로 저장된 노트를 가리킨다");

    let storage = fixture.storage();
    let one = storage
        .ai_note(&note_id)
        .expect("노트를 읽을 수 있어야 한다")
        .expect("방금 저장된 노트가 있어야 한다");
    let listed = storage
        .ai_notes(fixture.transcript_id.as_str())
        .expect("목록을 읽을 수 있어야 한다");

    assert_eq!(listed.len(), 1, "생성 한 번에 노트 하나가 남는다");
    assert_eq!(listed[0], one, "두 읽기 경로가 같은 값을 준다");

    // provenance는 §7.3이 요구하는 것 전부다. 어느 값도 추정이 아니다.
    assert_eq!(one.recording_id, fixture.recording_id);
    assert_eq!(one.transcript_id, fixture.transcript_id.as_str());
    assert_eq!(one.mode, "meeting");
    assert_eq!(one.provider, FAKE_PROVIDER_ID);
    assert_eq!(one.model, FAKE_MODEL_ID);
    assert!(!one.prompt_version.trim().is_empty(), "promptVersion이 실제로 남는다");
    assert!(!one.generated_at.trim().is_empty());

    // 본문은 봉투가 풀린 채로 온다 — 화면은 저장 형식도 schema 버전도 알지 않는다.
    let StructuredNotePayload::Meeting(meeting) = &one.note else {
        panic!("요청한 mode의 노트가 와야 한다");
    };
    assert!(!meeting.overview.trim().is_empty());

    // 직렬화된 모양이 화면과의 계약이다 — mode로 갈라 읽을 수 있어야 한다.
    let wire = serde_json::to_value(&one).expect("payload는 직렬화된다");
    assert_eq!(wire["note"]["mode"], "meeting");
    assert!(wire["note"]["keyDiscussions"].is_array());
    assert_eq!(wire["transcriptId"], fixture.transcript_id.as_str());

    // 입력은 그대로다 (INV-1 · INV-2).
    assert_eq!(fixture.transcript().raw_text, "오늘 회의에서는 다음 분기 일정을 이야기했다");
    assert_eq!(fixture.audio_bytes(), AUDIO_BYTES);
    assert_eq!(fixture.recording().ai_status, ProcessingStatus::Done);
}

#[test]
fn regenerating_adds_a_note_instead_of_replacing_the_one_that_is_there() {
    // ADR-0008 §9.2: 재생성은 대체가 아니라 추가다. 그래서 이전 노트의 provenance가 남는다.
    let fixture = Fixture::new("regenerate");
    let generator = fixture.generator_with(FakeNoteAiProvider::ready());

    generator
        .start(&fixture.recording_id, NoteType::Summary)
        .expect("첫 생성");
    let first = wait_for_finish(&generator);
    generator
        .start(&fixture.recording_id, NoteType::Summary)
        .expect("앞의 생성이 끝난 뒤에는 다시 시작할 수 있다");
    let second = wait_for_finish(&generator);

    let notes = fixture
        .storage()
        .ai_notes(fixture.transcript_id.as_str())
        .expect("목록을 읽을 수 있어야 한다");

    assert_eq!(first.state, "done");
    assert_eq!(second.state, "done");
    assert_ne!(first.ai_note_id, second.ai_note_id, "서로 다른 노트다");
    assert_eq!(notes.len(), 2, "이전 노트가 지워지지 않는다");
    assert_eq!(
        notes.iter().map(|note| note.id.clone()).collect::<Vec<_>>(),
        vec![
            first.ai_note_id.expect("첫 노트"),
            second.ai_note_id.expect("둘째 노트"),
        ],
        "만들어진 순서 그대로 온다"
    );
}

#[test]
fn a_recording_without_a_transcript_is_a_state_of_its_own_not_a_failure() {
    // §7.2: 재료가 아직 없는 것은 실패가 아니다. 사용자가 할 일은 전사를 먼저 돌리는 것이다.
    let fixture = Fixture::without_transcript("no-transcript");
    let generator = fixture.generator_with(FakeNoteAiProvider::ready());

    generator
        .start(&fixture.recording_id, NoteType::Study)
        .expect("시작은 접수된다");
    let finished = wait_for_finish(&generator);

    assert_eq!(finished.state, "noTranscript");
    assert_eq!(finished.failure, None, "실패가 아니다");
    assert_eq!(finished.mode.as_deref(), Some("study"));
    assert_eq!(
        fixture.recording().ai_status,
        ProcessingStatus::None,
        "아무것도 시도하지 않았으므로 상태도 옮기지 않는다"
    );
}

#[test]
fn unknown_identifiers_are_empty_answers_not_failures() {
    let fixture = Fixture::new("unknown-ids");
    let storage = fixture.storage();

    assert_eq!(
        storage.ai_note("없는-노트").expect("빈 답도 정상이다"),
        None
    );
    assert!(storage
        .ai_notes("없는-전사")
        .expect("빈 답도 정상이다")
        .is_empty());
}

// --- 생성이 다른 호출을 붙잡지 않는다 (요구 16 · R-001) ----------------------------------

#[test]
fn a_status_query_answers_while_the_provider_is_still_generating() {
    let fixture = Fixture::new("status-while-running");
    let (provider, controls) = GatedProvider::new();
    let gate = provider.handle();
    let generator = fixture.generator_with(provider);

    let accepted = generator
        .start(&fixture.recording_id, NoteType::Meeting)
        .expect("생성을 시작할 수 있어야 한다");
    assert_eq!(accepted.state, "running", "시작은 접수 사실을 돌려준다");

    // 여기부터가 관찰 구간이다 — provider는 생성 안에 들어갔고 아직 나오지 않았다.
    controls.wait_until_inside();

    let started = Instant::now();
    for _ in 0..5 {
        let status = generator.status().expect("상태를 물어볼 수 있어야 한다");
        assert_eq!(status.state, "running");
        assert_eq!(status.recording_id.as_deref(), Some(fixture.recording_id.as_str()));
        assert_eq!(status.mode.as_deref(), Some("meeting"));
        assert_eq!(status.ai_note_id, None, "아직 노트가 없다");
        assert_eq!(status.failure, None);
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "상태 조회가 생성을 기다렸다: {elapsed:?}"
    );
    assert_eq!(gate.call_count(), 1, "사전 조건: provider는 여전히 그 한 번의 생성 안에 있다");

    controls.release();
    assert_eq!(wait_for_finish(&generator).state, "done");
}

#[test]
fn other_commands_answer_while_a_note_is_being_generated() {
    // 생성이 앱의 저장소 연결을 붙들면, 스레드를 만들어 놓고 저장소에서 다시 막는 셈이 된다.
    let fixture = Fixture::new("other-commands");
    let (provider, controls) = GatedProvider::new();
    let generator = fixture.generator_with(provider);
    let storage = fixture.storage();

    generator
        .start(&fixture.recording_id, NoteType::Meeting)
        .expect("생성을 시작할 수 있어야 한다");
    controls.wait_until_inside();

    let started = Instant::now();
    let listed = storage.list_recordings().expect("목록을 읽을 수 있어야 한다");
    let settings = storage.settings().expect("설정을 읽을 수 있어야 한다");
    let transcript = storage
        .transcript(fixture.transcript_id.as_str())
        .expect("전사를 읽을 수 있어야 한다");
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "다른 command가 생성을 기다렸다: {elapsed:?}"
    );
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].ai_status, "running",
        "화면이 읽는 상태도 이미 저장돼 있다 (§7)"
    );
    assert_eq!(settings.ai_provider, None);
    assert!(transcript.is_some(), "열람은 생성과 무관하게 답한다");

    controls.release();
    assert_eq!(wait_for_finish(&generator).state, "done");
}

#[test]
fn starting_a_second_generation_is_refused_instead_of_being_ignored() {
    // 여러 Recording 일괄 AI 처리 큐는 이 Phase의 범위 밖이다 (§16 DEFERRED). 줄을 세우는
    // 대신 거절하며, 거절이 진행 중인 생성을 흔들지 않는다.
    let fixture = Fixture::new("duplicate");
    let (provider, controls) = GatedProvider::new();
    let gate = provider.handle();
    let generator = fixture.generator_with(provider);

    generator
        .start(&fixture.recording_id, NoteType::Meeting)
        .expect("첫 시작은 성공한다");
    controls.wait_until_inside();

    let failure = generator
        .start(&fixture.recording_id, NoteType::Study)
        .expect_err("생성 중에는 다시 시작할 수 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(!failure.message.trim().is_empty(), "그대로 화면에 띄울 수 있다");
    assert!(failure.source_data_safe, "아무것도 건드리지 않았다");
    assert_eq!(
        generator.status().expect("상태를 읽는다").mode.as_deref(),
        Some("meeting"),
        "거부가 진행 중인 생성을 흔들지 않는다"
    );

    controls.release();
    assert_eq!(wait_for_finish(&generator).state, "done");
    assert_eq!(gate.call_count(), 1, "거부된 요청이 생성을 한 번 더 돌리지 않는다");
    assert_eq!(
        fixture
            .storage()
            .ai_notes(fixture.transcript_id.as_str())
            .expect("목록을 읽는다")
            .len(),
        1,
        "노트도 하나뿐이다"
    );
}
