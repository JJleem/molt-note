//! **AI 노트 생성이 실패해도 잃는 것이 없고, 실패는 상태로 남아 다시 시도할 수 있다**
//! (PRODUCT-SPEC §13 · INV-1 · INV-2 · INV-3 · `docs/ADR-0008-note-ai-provider.md` §13).
//!
//! `tests/ai_note_run.rs`가 orchestration 함수 하나를 두고 같은 성질을 판정한다면, 이 파일은
//! **화면이 실제로 지나는 경로 전부**를 지나며 판정한다.
//!
//! ```text
//! NoteGenerator.start ─→ 배경 스레드 ─→ ai::run::generate ─→ OllamaProvider ─→ HttpTransport
//!   (command 경계)                        (영속화 규칙)         (실제 adapter)      ↑ 여기만 double
//! ```
//!
//! **대체 구현이 들어가는 자리는 HTTP 왕복 하나뿐이다.** 실행자도, 배경 스레드도, orchestration도,
//! 벤더 adapter도, 저장소도 제품 코드 그대로다 — 그래서 여기서 확인되는 것은 "double이 실패를
//! 흉내 냈다"가 아니라 **실패가 실제 경로를 지나 무엇을 남기는가**다.
//!
//! 판정 대상은 §13이 요구하는 세 질문이며, provider가 실패하는 **세 가지 방식** 각각에 대해
//! 같은 답이 나와야 한다.
//!
//! ```text
//! 무엇이 실패했는가       §13의 서로 다른 실패 셋 — 뭉개지지 않는다
//! 원본은 안전한가         Recording · Transcript · segment · 오디오 파일이 그대로다
//! 다시 시도할 수 있는가   상태가 남고, 상황이 풀리면 같은 요청이 성공한다
//! ```
//!
//! **실제 Ollama 프로세스도 네트워크도 실제 Whisper 추론 결과도 요구하지 않는다**
//! (§18 · A-TRANS-001). Transcript는 손으로 쓴 fixture다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use molt_note_lib::ai::ollama::testing::{StubReply, StubServer, MODEL_IN_THE_LIST};
use molt_note_lib::ai::ollama::OllamaProvider;
use molt_note_lib::commands::{AiNoteStatusPayload, NoteGenerator, Storage};
use molt_note_lib::db::{self, store};
use molt_note_lib::domain::{
    AiNote, FailureKind, NoteType, ProcessingStatus, Recording, RecordingId, Transcript,
    TranscriptId, TranscriptSegment,
};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use rusqlite::Connection;

/// 배경 스레드가 끝나기를 기다리는 한계. 넘으면 테스트가 실패한다 — 매달리지 않는다.
const FINISH_TIMEOUT: Duration = Duration::from_secs(30);

/// 테스트가 "사용자가 설정한 주소"로 쓰는 값. **실재하지 않는다** — `.invalid`는 어떤 이름
/// 해석도 성공하지 않도록 예약된 TLD이며, 그래서 이 파일은 소켓을 열 수 없다.
const CONFIGURED_BASE_URL: &str = "http://configured-host.invalid:65535";

/// "오디오"로 둘 바이트. AI 경로는 이 파일을 열지 않으므로 내용은 아무것이나 좋다 —
/// 확인하는 것은 **그대로 남아 있는가**뿐이다 (INV-1 · INV-6).
const AUDIO_BYTES: &[u8] = b"molt-note fixture audio bytes";

const TRANSCRIPT_TEXT: &str = "오늘 회의에서는 다음 분기 일정과 남은 과제를 이야기했다.";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-ai-failure-recovery-{}-{}-{}",
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

/// 노트를 만들 재료를 갖춘 앱 하나 — DB · 녹음 레코드 · 오디오 파일 · current Transcript.
struct Fixture {
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    recording_id: RecordingId,
    transcript_id: TranscriptId,
    audio_path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
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
            recording_id: RecordingId::new("rec-ai-failure"),
            transcript_id: TranscriptId::new("tr-ai-failure"),
            audio_path,
        };

        let mut connection = fixture.connection();
        store::insert_recording(
            &connection,
            &Recording {
                id: fixture.recording_id.clone(),
                title: "3DGS Study #04".to_owned(),
                created_at: "2026-09-03T10:00:00.000Z".to_owned(),
                updated_at: "2026-09-03T10:00:00.000Z".to_owned(),
                duration_ms: 3_151_000,
                audio_path: fixture.audio_path.to_str().expect("경로 문자열").to_owned(),
                audio_format: "wav".to_owned(),
                microphone: Some("MacBook Pro Microphone".to_owned()),
                current_transcript_id: None,
                // 전사는 이미 끝난 상태다. AI 실패가 이 값을 옮기지 않는 것이 확인 대상이다.
                transcription_status: ProcessingStatus::Done,
                ai_status: ProcessingStatus::None,
                notion_status: ProcessingStatus::None,
            },
        )
        .expect("사전 조건: 녹음 레코드를 저장한다");

        // **실제 전사 결과가 아니라 fixture다** (A-TRANS-001).
        store::append_transcript(
            &mut connection,
            &Transcript {
                id: fixture.transcript_id.clone(),
                recording_id: fixture.recording_id.clone(),
                language: Some("ko".to_owned()),
                segments: vec![
                    TranscriptSegment {
                        start_ms: 0,
                        end_ms: 2_000,
                        text: TRANSCRIPT_TEXT.to_owned(),
                    },
                    TranscriptSegment {
                        start_ms: 2_000,
                        end_ms: 4_000,
                        text: "결론은 다음 주에 다시 정리하기로 했다.".to_owned(),
                    },
                ],
                raw_text: format!("{TRANSCRIPT_TEXT} 결론은 다음 주에 다시 정리하기로 했다."),
                created_at: "2026-09-03T10:01:00.000Z".to_owned(),
                engine: "fixture-engine".to_owned(),
                model: "fixture-model".to_owned(),
            },
        )
        .expect("사전 조건: Transcript를 추가한다");

        store::set_current_transcript(
            &connection,
            &fixture.recording_id,
            Some(&fixture.transcript_id),
            "2026-09-03T10:02:00.000Z",
        )
        .expect("사전 조건: current Transcript를 지정한다");

        fixture
    }

    /// 그 서버를 상대하는 실행자 하나. **실행자도 orchestration도 adapter도 제품 코드다.**
    fn generator_talking_to(&self, server: StubServer) -> NoteGenerator {
        NoteGenerator::with_provider(
            self.app_data_dir.clone(),
            OllamaProvider::new(CONFIGURED_BASE_URL, MODEL_IN_THE_LIST, Arc::new(server)),
        )
    }

    /// 앱이 들고 있는 것과 같은 저장소. 화면이 목록과 상세를 읽는 자리다.
    fn storage(&self) -> Storage {
        let storage = Storage::open(&self.app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");
        storage
    }

    fn connection(&self) -> Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    fn recording(&self) -> Recording {
        store::load_recording(&self.connection(), &self.recording_id)
            .expect("녹음을 읽을 수 있어야 한다")
            .expect("녹음이 남아 있어야 한다")
    }

    fn notes(&self) -> Vec<AiNote> {
        store::list_ai_notes_for_transcript(&self.connection(), &self.transcript_id)
            .expect("AI Note 목록을 읽을 수 있어야 한다")
    }

    fn audio_bytes(&self) -> Vec<u8> {
        fs::read(&self.audio_path).expect("오디오 파일을 읽을 수 있어야 한다")
    }

    /// `transcripts`와 `transcript_segments`의 **모든 행 · 모든 열**을 한 문자열로 뜬다.
    ///
    /// domain 타입으로 비교하면 저장소가 복원하지 않는 열이 바뀌어도 알 수 없다. 테이블을
    /// 그대로 읽으므로 **바이트 단위의 동일성**을 말할 수 있다 (INV-2).
    fn transcript_tables(&self) -> String {
        let connection = self.connection();
        let mut dump = String::new();

        let mut transcripts = connection
            .prepare(
                "SELECT id, recording_id, language, raw_text, created_at, engine, model
                 FROM transcripts ORDER BY id",
            )
            .expect("transcripts를 질의할 수 있어야 한다");
        let rows = transcripts
            .query_map([], |row| {
                Ok(format!(
                    "transcript|{}|{}|{:?}|{}|{}|{}|{}\n",
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .expect("transcripts 행을 읽을 수 있어야 한다");
        for row in rows {
            dump.push_str(&row.expect("transcripts 행 하나를 읽는다"));
        }

        let mut segments = connection
            .prepare(
                "SELECT transcript_id, ordinal, start_ms, end_ms, text
                 FROM transcript_segments ORDER BY transcript_id, ordinal",
            )
            .expect("transcript_segments를 질의할 수 있어야 한다");
        let rows = segments
            .query_map([], |row| {
                Ok(format!(
                    "segment|{}|{}|{}|{}|{}\n",
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .expect("transcript_segments 행을 읽을 수 있어야 한다");
        for row in rows {
            dump.push_str(&row.expect("transcript_segments 행 하나를 읽는다"));
        }

        dump
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

/// provider가 실패하는 **세 가지 방식**. 사용자가 할 수 있는 일이 셋 다 다르다 (§13).
///
/// ```text
/// 미실행        서버가 응답하지 않는다        서버를 켜고 다시 누르면 된다        재시도 ✓
/// 모델 없음     서버는 살아 있는데 모델이 없다  모델을 받아야 한다                 재시도 ✗
/// 잘못된 응답   응답을 노트로 읽을 수 없다     다시 만들면 달라질 수 있다          재시도 ✓
/// ```
fn ways_a_provider_fails() -> [(&'static str, StubServer, FailureKind, bool); 3] {
    [
        (
            "미실행",
            StubServer::refusing(),
            FailureKind::AiProviderUnreachable,
            true,
        ),
        (
            "모델 없음",
            StubServer::ready().with_models(Vec::new()),
            FailureKind::AiModelUnavailable,
            false,
        ),
        (
            "잘못된 응답",
            StubServer::ready().with_generate(StubReply::GeneratedText(
                "죄송하지만 JSON으로 답할 수 없습니다".to_owned(),
            )),
            FailureKind::AiResponseUnusable,
            true,
        ),
    ]
}

// --- 세 실패 각각이 아무것도 잃지 않는다 (INV-1 · INV-2 · INV-3) --------------------------

#[test]
fn every_way_a_provider_can_fail_leaves_the_recording_and_the_transcript_exactly_as_they_were() {
    for (label, server, expected_kind, retryable) in ways_a_provider_fails() {
        let fixture = Fixture::new("intact");
        let before = fixture.recording();
        let transcripts_before = fixture.transcript_tables();
        let audio_before = fixture.audio_bytes();

        let generator = fixture.generator_talking_to(server);
        generator
            .start(fixture.recording_id.as_str(), NoteType::Meeting)
            .expect("시작은 provider의 상태와 무관하게 접수된다");
        let finished = wait_for_finish(&generator);

        // 1. 무엇이 실패했는가 (§13).
        assert_eq!(finished.state, "failed", "{label}");
        let failure = finished.failure.expect("무엇이 막혔는지 말한다");
        assert_eq!(failure.kind, expected_kind, "{label}");
        assert_eq!(failure.retryable, retryable, "{label}: 다시 눌러 볼 값이 있는가");
        assert!(
            !failure.message.trim().is_empty(),
            "{label}: 그대로 화면에 띄울 수 있는 문장이어야 한다"
        );

        // 2. 원본은 안전한가 (INV-1 · INV-2 · INV-3).
        assert!(failure.source_data_safe, "{label}");

        let after = fixture.recording();
        assert_eq!(after.id, before.id, "{label}");
        assert_eq!(after.title, before.title, "{label}");
        assert_eq!(after.created_at, before.created_at, "{label}");
        assert_eq!(after.duration_ms, before.duration_ms, "{label}");
        assert_eq!(after.audio_path, before.audio_path, "{label}");
        assert_eq!(after.audio_format, before.audio_format, "{label}");
        assert_eq!(after.microphone, before.microphone, "{label}");
        assert_eq!(
            after.current_transcript_id, before.current_transcript_id,
            "{label}: 실패가 current를 옮기지 않는다 (§7.2)"
        );
        assert_eq!(
            after.transcription_status, before.transcription_status,
            "{label}: 남의 파이프라인 상태를 옮기지 않는다"
        );
        assert_eq!(after.notion_status, before.notion_status, "{label}");

        assert_eq!(
            fixture.transcript_tables(),
            transcripts_before,
            "{label}: Transcript와 segment의 모든 행이 그대로다 (INV-2)"
        );
        assert_eq!(fixture.audio_bytes(), audio_before, "{label}: 오디오도 그대로다");
        assert!(
            fixture.notes().is_empty(),
            "{label}: 반쯤 채워진 노트를 남기지 않는다"
        );

        // 3. 실패가 **상태로** 남는다 — 화면이 그릴 재료가 저장돼 있다 (§13).
        assert_eq!(after.ai_status, ProcessingStatus::Failed, "{label}");
        let listed = fixture
            .storage()
            .list_recordings()
            .expect("목록을 읽을 수 있어야 한다");
        assert_eq!(
            listed[0].ai_status, "failed",
            "{label}: 목록과 상세가 읽는 값에도 남는다"
        );
        assert_eq!(
            generator.status().expect("상태를 다시 읽는다").failure,
            Some(failure),
            "{label}: 다시 물어봐도 같은 실패가 온다 — 한 번 읽고 사라지지 않는다"
        );
    }
}

// --- 실패한 뒤에도 다시 시도할 수 있다 (§13) ----------------------------------------------

#[test]
fn a_note_can_still_be_generated_after_each_of_the_three_failures() {
    // 실패는 막다른 길이 아니다. 사용자가 서버를 켜거나 모델을 받은 뒤 같은 버튼을 다시
    // 누르는 것이 이 테스트다 — 그래서 두 번째 실행자는 **응답하는 서버**를 상대한다.
    for (label, server, _kind, _retryable) in ways_a_provider_fails() {
        let fixture = Fixture::new("retry");
        let transcripts_before = fixture.transcript_tables();

        let failing = fixture.generator_talking_to(server);
        failing
            .start(fixture.recording_id.as_str(), NoteType::Summary)
            .expect("첫 시도는 접수된다");
        assert_eq!(
            wait_for_finish(&failing).state,
            "failed",
            "{label}: 사전 조건 — 첫 시도가 실패한다"
        );
        assert_eq!(fixture.recording().ai_status, ProcessingStatus::Failed, "{label}");

        // 상황이 풀렸다. 같은 녹음 · 같은 mode를 다시 요청한다.
        let working = fixture.generator_talking_to(StubServer::ready());
        working
            .start(fixture.recording_id.as_str(), NoteType::Summary)
            .expect("실패한 뒤에도 다시 시작할 수 있다");
        let finished = wait_for_finish(&working);

        assert_eq!(finished.state, "done", "{label}: 재시도가 성공한다");
        let note_id = finished.ai_note_id.expect("새로 저장된 노트를 가리킨다");
        assert_eq!(
            fixture.recording().ai_status,
            ProcessingStatus::Done,
            "{label}: 실패 상태가 성공으로 덮인다"
        );

        // 노트는 하나다 — 실패한 시도가 반쯤 채워진 행을 남기지 않았다는 뜻이다.
        let notes = fixture.notes();
        assert_eq!(notes.len(), 1, "{label}");
        assert_eq!(notes[0].id.as_str(), note_id, "{label}");
        assert_eq!(notes[0].note_type, NoteType::Summary, "{label}");
        assert_eq!(
            notes[0].transcript_id, fixture.transcript_id,
            "{label}: 실제로 입력에 쓴 Transcript가 남는다 (§7.3)"
        );
        assert!(!notes[0].model.trim().is_empty(), "{label}: provenance가 남는다");

        // 실패도 성공도 원본을 건드리지 않았다.
        assert_eq!(
            fixture.transcript_tables(),
            transcripts_before,
            "{label}: Transcript가 그대로다 (INV-2)"
        );
        assert_eq!(fixture.audio_bytes(), AUDIO_BYTES, "{label}: 오디오도 그대로다");
    }
}

#[test]
fn a_failure_does_not_take_away_the_note_that_was_already_there() {
    // ADR-0008 §9.2 · §13: 실패가 이전 노트를 지우지 않는다. 그래서 사용자는 실패한 뒤에도
    // 이미 갖고 있던 노트를 그대로 본다.
    let fixture = Fixture::new("keeps-earlier-note");

    let working = fixture.generator_talking_to(StubServer::ready());
    working
        .start(fixture.recording_id.as_str(), NoteType::Meeting)
        .expect("첫 생성");
    assert_eq!(wait_for_finish(&working).state, "done", "사전 조건: 노트 하나가 있다");
    let kept = fixture.notes();
    assert_eq!(kept.len(), 1, "사전 조건");

    // 그리고 재생성이 실패한다.
    let failing = fixture.generator_talking_to(StubServer::refusing());
    failing
        .start(fixture.recording_id.as_str(), NoteType::Meeting)
        .expect("재생성 시작은 접수된다");
    assert_eq!(wait_for_finish(&failing).state, "failed");

    assert_eq!(
        fixture.notes(),
        kept,
        "이미 있던 노트가 한 글자도 바뀌지 않는다 — UPDATE도 DELETE도 아니다"
    );
    assert_eq!(
        fixture.recording().ai_status,
        ProcessingStatus::Failed,
        "마지막 시도가 실패했다는 사실은 상태로 남는다 (§13)"
    );
    assert_eq!(fixture.audio_bytes(), AUDIO_BYTES);
}
