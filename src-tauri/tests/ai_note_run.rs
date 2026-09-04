//! AI 노트 orchestration: provenance · 입력 선택 · 재생성 이력 · 실패가 잃는 것이 없음
//! (PRODUCT-SPEC §7.1 · §7.2 · §7.3 · §13 · INV-2 · INV-3 ·
//! `docs/ADR-0008-note-ai-provider.md` §9).
//!
//! **실제 Ollama도 실제 Whisper 추론 결과도 요구하지 않는다** (A-TRANS-001 · ADR-0008 §18).
//! provider 자리에는 계약이 같은 test double이 서고(`ai::testing::FakeNoteAiProvider`),
//! Transcript는 손으로 쓴 fixture이며, "오디오"는 몇 바이트짜리 파일이다 — AI 경로는 그 파일을
//! 열지 않으므로 내용이 무엇이든 상관없고, **바이트가 그대로인지**만 확인 대상이다.
//!
//! 여기서 판정하는 것은 **영속성 규칙과 불변식**이다 — 노트 품질이 아니다 (그것은 Phase 4의
//! Human Review 항목이며 이 테스트가 대신 판정하지 않는다).
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::ai::prompt::{self, ContextBudget};
use molt_note_lib::ai::run::{self, Outcome};
use molt_note_lib::ai::testing::{
    sample_note, FakeNoteAiProvider, FakeResponse, FAKE_MODEL_ID, FAKE_PROVIDER_ID,
};
use molt_note_lib::ai::{encode_content, NoteAiProvider};
use molt_note_lib::db::{self, store};
use molt_note_lib::domain::{
    AiNote, Failure, FailureKind, NoteType, ProcessingStatus, Recording, RecordingId, Transcript,
    TranscriptId, TranscriptSegment,
};
use rusqlite::Connection;

/// 실패 경로가 원본을 지우거나 고치는 코드를 갖고 있지 않다는 것은 소스에서도 확인한다
/// (INV-2 · INV-3 · INV-6).
const RUN_SOURCE: &str = include_str!("../src/ai/run.rs");

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-ai-note-run-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// "오디오"로 둘 바이트. AI 경로는 이 파일을 열지 않으므로 내용은 아무것이나 좋다 —
/// 확인하는 것은 **그대로 남아 있는가**뿐이다 (INV-1 · INV-6).
const AUDIO_BYTES: &[u8] = b"molt-note fixture audio bytes";

/// 노트 한 건을 만드는 데 필요한 자리 전부 — DB · 녹음 레코드 · 오디오 파일 · Transcript 둘.
struct Fixture {
    _dir: TempDir,
    connection: Connection,
    recording_id: RecordingId,
    audio_path: PathBuf,
    /// 먼저 만들어진 Transcript. 처음에는 current가 아니다.
    first: TranscriptId,
    /// 나중에 만들어진 Transcript. 처음의 current다 (§7.2).
    second: TranscriptId,
}

impl Fixture {
    /// Transcript **둘**을 가진 Recording. current는 나중 것이다.
    fn new(label: &str) -> Self {
        let mut fixture = Self::without_transcripts(label);
        let (first, second) = (fixture.first.clone(), fixture.second.clone());

        fixture.append_transcript(&first, "2026-09-03T10:01:00.000Z", "첫 전사");
        fixture.append_transcript(&second, "2026-09-03T10:02:00.000Z", "두 번째 전사");
        fixture.set_current(Some(&second));

        fixture
    }

    /// Transcript가 하나도 없는 Recording — current도 없다 (§7.2의 정상 상태).
    fn without_transcripts(label: &str) -> Self {
        let dir = TempDir::new(label);
        let audio_path = dir.join("recording.wav");
        fs::write(&audio_path, AUDIO_BYTES).expect("사전 조건: 오디오 파일을 만든다");

        let connection = db::open(dir.join("molt-note.db")).expect("임시 DB를 열 수 있어야 한다");
        let recording_id = RecordingId::new("rec-ai-note");
        store::insert_recording(
            &connection,
            &Recording {
                id: recording_id.clone(),
                title: "3DGS Study #04".to_owned(),
                created_at: "2026-09-03T10:00:00.000Z".to_owned(),
                updated_at: "2026-09-03T10:00:00.000Z".to_owned(),
                duration_ms: 3_151_000,
                audio_path: audio_path.to_str().expect("경로 문자열").to_owned(),
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

        Self {
            _dir: dir,
            connection,
            recording_id,
            audio_path,
            first: TranscriptId::new("tr-first"),
            second: TranscriptId::new("tr-second"),
        }
    }

    /// fixture Transcript 하나를 추가한다. **실제 전사 결과가 아니다** (A-TRANS-001).
    fn append_transcript(&mut self, id: &TranscriptId, created_at: &str, text: &str) {
        store::append_transcript(
            &mut self.connection,
            &Transcript {
                id: id.clone(),
                recording_id: self.recording_id.clone(),
                language: Some("ko".to_owned()),
                segments: vec![
                    TranscriptSegment {
                        start_ms: 0,
                        end_ms: 1_000,
                        text: format!("{text} 첫 문장"),
                    },
                    TranscriptSegment {
                        start_ms: 1_000,
                        end_ms: 2_000,
                        text: format!("{text} 두 번째 문장"),
                    },
                ],
                raw_text: format!("{text} 첫 문장 {text} 두 번째 문장"),
                created_at: created_at.to_owned(),
                engine: "fixture-engine".to_owned(),
                model: "fixture-model".to_owned(),
            },
        )
        .expect("사전 조건: Transcript를 추가한다");
    }

    fn set_current(&self, transcript: Option<&TranscriptId>) {
        store::set_current_transcript(
            &self.connection,
            &self.recording_id,
            transcript,
            "2026-09-03T10:03:00.000Z",
        )
        .expect("사전 조건: current Transcript를 지정한다");
    }

    fn generate(
        &self,
        mode: NoteType,
        provider: &dyn NoteAiProvider,
    ) -> Result<Outcome, Failure> {
        self.generate_within(mode, provider, ContextBudget::DEFAULT)
    }

    fn generate_within(
        &self,
        mode: NoteType,
        provider: &dyn NoteAiProvider,
        budget: ContextBudget,
    ) -> Result<Outcome, Failure> {
        run::generate(
            &self.connection,
            &self.recording_id,
            mode,
            provider,
            budget,
        )
    }

    fn recording(&self) -> Recording {
        store::load_recording(&self.connection, &self.recording_id)
            .expect("녹음을 읽을 수 있어야 한다")
            .expect("녹음이 남아 있어야 한다")
    }

    fn notes_for(&self, transcript: &TranscriptId) -> Vec<AiNote> {
        store::list_ai_notes_for_transcript(&self.connection, transcript)
            .expect("AI Note 목록을 읽을 수 있어야 한다")
    }

    fn all_notes(&self) -> Vec<AiNote> {
        let mut notes = self.notes_for(&self.first);
        notes.extend(self.notes_for(&self.second));
        notes
    }

    fn audio_bytes(&self) -> Vec<u8> {
        fs::read(&self.audio_path).expect("오디오 파일을 읽을 수 있어야 한다")
    }

    /// `transcripts`와 `transcript_segments`의 **모든 행 · 모든 열**을 한 문자열로 뜬다.
    ///
    /// domain 타입으로 비교하면 저장소가 복원하지 않는 열이 바뀌어도 알 수 없다. 여기서는
    /// 테이블을 그대로 읽으므로 **바이트 단위의 동일성**을 말할 수 있다 (INV-2).
    fn transcript_tables(&self) -> String {
        let mut dump = String::new();

        let mut transcripts = self
            .connection
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

        let mut segments = self
            .connection
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

    /// 저장된 `ai_status`를 **저장된 순서대로** 기록하기 시작한다.
    ///
    /// 기록하는 것은 제품 코드가 아니라 DB의 trigger다 — 무엇이 실제로 행에 쓰였는지를 제3자가
    /// 관찰한 결과이며, 그래서 orchestration에 테스트 전용 통로를 내지 않는다.
    fn record_status_writes(&self) {
        self.connection
            .execute_batch(
                "CREATE TABLE status_log (seq INTEGER PRIMARY KEY, status TEXT NOT NULL);
                 CREATE TRIGGER log_ai_status
                 AFTER UPDATE OF ai_status ON recordings
                 BEGIN
                     INSERT INTO status_log (status) VALUES (NEW.ai_status);
                 END;",
            )
            .expect("사전 조건: 상태 기록용 trigger를 만든다");
    }

    fn stored_statuses(&self) -> Vec<String> {
        let mut statement = self
            .connection
            .prepare("SELECT status FROM status_log ORDER BY seq")
            .expect("기록을 읽을 수 있어야 한다");
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("기록을 질의할 수 있어야 한다");
        rows.collect::<Result<Vec<_>, _>>()
            .expect("기록 행을 읽을 수 있어야 한다")
    }
}

/// 저장된 노트 하나. 돌려받은 값이 아니라 **행을 다시 읽은 것**이다.
fn stored(fixture: &Fixture, note: &AiNote) -> AiNote {
    store::load_ai_note(&fixture.connection, &note.id)
        .expect("AI Note를 읽을 수 있어야 한다")
        .expect("그 AI Note가 남아 있어야 한다")
}

fn generated(outcome: Outcome) -> AiNote {
    match outcome {
        Outcome::Generated(note) => note,
        Outcome::NoTranscriptYet => panic!("입력이 있는 Recording이어야 한다"),
    }
}

// --- 성공 경로와 provenance (§7 · §7.3) -------------------------------------------------

#[test]
fn a_successful_run_stores_one_note_with_every_provenance_value() {
    let fixture = Fixture::new("provenance");
    let provider = FakeNoteAiProvider::ready();

    let note = generated(
        fixture
            .generate(NoteType::Meeting, &provider)
            .expect("노트 생성이 성공해야 한다"),
    );

    // 돌려준 값이 아니라 저장된 행으로 확인한다.
    let saved = stored(&fixture, &note);
    assert_eq!(saved, note, "돌려준 것이 저장된 것이다");

    assert_eq!(saved.recording_id, fixture.recording_id, "recordingId");
    assert_eq!(
        saved.transcript_id, fixture.second,
        "transcriptId — 실제로 입력에 쓴 Transcript다 (§7.3)"
    );
    assert_eq!(saved.note_type, NoteType::Meeting, "type");
    assert_eq!(saved.provider, FAKE_PROVIDER_ID, "provider");
    assert_eq!(saved.model, FAKE_MODEL_ID, "model — provider가 답한 값이다");
    assert_eq!(
        saved.prompt_version,
        prompt::prompt_version(NoteType::Meeting),
        "promptVersion"
    );
    assert!(
        saved.generated_at.starts_with("20") && saved.generated_at.ends_with('Z'),
        "generatedAt이 ISO-8601 UTC 텍스트여야 한다: {}",
        saved.generated_at
    );
    assert_eq!(
        saved.content,
        encode_content(&sample_note(NoteType::Meeting)),
        "content는 §7.5의 봉투를 씌운 structured note다"
    );

    assert_eq!(fixture.recording().ai_status, ProcessingStatus::Done);
}

#[test]
fn the_stored_prompt_version_is_the_version_of_the_prompt_that_was_used() {
    // 상수를 그대로 옮겨 적기만 하면 프롬프트를 고쳐도 저장된 값이 거짓이 되는 것을 잡을 수
    // 없다. 그래서 **지금의 프롬프트 원문과 schema로부터 다시 계산한 값**과 비교한다
    // (ADR-0008 §10.2 — `computed_prompt_version`이 그 계산이다).
    for mode in NoteType::ALL {
        let fixture = Fixture::new("prompt-version");
        let provider = FakeNoteAiProvider::ready();

        let note = generated(fixture.generate(mode, &provider).expect("생성이 성공해야 한다"));

        assert_eq!(
            stored(&fixture, &note).prompt_version,
            prompt::computed_prompt_version(mode),
            "{mode}: 저장된 promptVersion이 지금의 프롬프트·schema에서 나온 값과 다르다"
        );
        assert!(
            note.prompt_version.contains(mode.as_str()),
            "{mode}: 모드가 값에 있어야 한다"
        );
    }
}

#[test]
fn each_mode_is_stored_as_its_own_note() {
    let fixture = Fixture::new("modes");
    let provider = FakeNoteAiProvider::ready();

    for mode in NoteType::ALL {
        let note = generated(fixture.generate(mode, &provider).expect("생성이 성공해야 한다"));
        assert_eq!(stored(&fixture, &note).note_type, mode);
    }

    let notes = fixture.notes_for(&fixture.second);
    assert_eq!(notes.len(), 3, "세 노트가 함께 남는다");
    let mut types: Vec<&str> = notes.iter().map(|note| note.note_type.as_str()).collect();
    types.sort_unstable();
    assert_eq!(types, ["meeting", "study", "summary"]);
}

#[test]
fn only_the_transcript_text_is_handed_to_the_provider() {
    // INV-6: 넘어간 것이 무엇인지 double의 기록으로 확인한다. 오디오 경로도 바이트도 없다 —
    // 계약에 그것을 담을 자리가 없기 때문이다.
    let fixture = Fixture::new("input");
    let provider = FakeNoteAiProvider::ready();

    fixture
        .generate(NoteType::Summary, &provider)
        .expect("생성이 성공해야 한다");

    let calls = provider.calls();
    assert_eq!(calls.len(), 1, "한 번의 생성은 한 번의 호출이다");
    assert_eq!(calls[0].mode, NoteType::Summary);
    assert_eq!(
        calls[0].transcript, "두 번째 전사 첫 문장 두 번째 전사 두 번째 문장",
        "current Transcript의 텍스트가 그대로 간다 (§7.2)"
    );
    assert_eq!(
        calls[0].context_tokens,
        ContextBudget::DEFAULT.context_tokens,
        "요청은 언제나 context 크기를 싣는다 (ADR-0008 §8.2)"
    );
}

// --- 기본 입력은 current가 가리키는 Transcript다 (§7.2 · §7.3) ---------------------------

#[test]
fn notes_from_different_transcripts_are_told_apart_by_transcript_id() {
    let fixture = Fixture::new("two-transcripts");
    let provider = FakeNoteAiProvider::ready();

    // current는 나중 Transcript다.
    let from_second = generated(
        fixture
            .generate(NoteType::Meeting, &provider)
            .expect("생성이 성공해야 한다"),
    );

    // current를 먼저 만든 Transcript로 옮기면 **기본 입력도 함께 옮겨간다.**
    fixture.set_current(Some(&fixture.first));
    let from_first = generated(
        fixture
            .generate(NoteType::Meeting, &provider)
            .expect("생성이 성공해야 한다"),
    );

    assert_eq!(from_second.transcript_id, fixture.second);
    assert_eq!(from_first.transcript_id, fixture.first);
    assert_ne!(
        from_first.transcript_id, from_second.transcript_id,
        "어떤 Transcript version에서 나왔는지가 열로 구분된다 (§7.3)"
    );

    // 저장된 행도 같은 말을 한다 — 각 Transcript의 목록에 자기 노트만 있다.
    let first_notes = fixture.notes_for(&fixture.first);
    let second_notes = fixture.notes_for(&fixture.second);
    assert_eq!(first_notes.len(), 1);
    assert_eq!(second_notes.len(), 1);
    assert_eq!(first_notes[0].id, from_first.id);
    assert_eq!(second_notes[0].id, from_second.id);

    // 넘어간 입력 텍스트도 서로 다르다 — id만 다르게 적고 같은 것을 보낸 것이 아니다.
    let calls = provider.calls();
    assert_ne!(calls[0].transcript, calls[1].transcript);
    assert!(calls[1].transcript.starts_with("첫 전사"));
}

#[test]
fn a_recording_without_a_current_transcript_is_not_an_error() {
    // §7.2: 아직 입력이 없는 것은 오류가 아니라 상태다. 그래서 **아무것도 쓰지 않는다.**
    let fixture = Fixture::without_transcripts("no-input");
    let before = fixture.recording();
    let provider = FakeNoteAiProvider::ready();

    let outcome = fixture
        .generate(NoteType::Meeting, &provider)
        .expect("실패가 아니다");

    assert_eq!(outcome, Outcome::NoTranscriptYet);
    assert!(outcome.generated().is_none());
    assert_eq!(provider.call_count(), 0, "provider를 부르지도 않았다");

    let after = fixture.recording();
    assert_eq!(
        after.ai_status,
        ProcessingStatus::None,
        "시도한 적 없는 일이 failed로 남지 않는다"
    );
    assert_eq!(
        after.updated_at, before.updated_at,
        "레코드에 아무 쓰기도 일어나지 않았다"
    );
    assert_eq!(after, before);
    assert!(fixture.all_notes().is_empty());
}

// --- Transcript는 한 바이트도 바뀌지 않는다 (§7.1 · INV-2) -------------------------------

#[test]
fn generating_a_note_changes_no_byte_of_any_transcript() {
    let fixture = Fixture::new("transcripts-intact");
    let before = fixture.transcript_tables();
    let provider = FakeNoteAiProvider::ready();

    for mode in NoteType::ALL {
        fixture.generate(mode, &provider).expect("생성이 성공해야 한다");
    }
    // current를 옮겨 다른 Transcript에서도 만들어 본다 — Transcript가 여럿인 Recording이다.
    fixture.set_current(Some(&fixture.first));
    fixture
        .generate(NoteType::Meeting, &provider)
        .expect("생성이 성공해야 한다");

    assert_eq!(
        fixture.transcript_tables(),
        before,
        "transcripts와 transcript_segments의 모든 행이 그대로다 (INV-2)"
    );
    assert_eq!(fixture.all_notes().len(), 4, "늘어난 것은 노트뿐이다");
    assert_eq!(fixture.audio_bytes(), AUDIO_BYTES, "오디오도 그대로다 (INV-1)");
}

#[test]
fn the_orchestration_reads_transcripts_and_never_writes_them() {
    // 위 테스트가 지나간 경로만 안전한 것이 아니라, 그런 코드가 아예 없다.
    //
    // 주석은 빼고 본다 — 규칙을 문장으로 적는 것("UPDATE 경로가 없다")과 그 규칙을 어기는
    // 코드는 다르다. 검사 대상은 실행되는 줄이다.
    let code = RUN_SOURCE
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "append_transcript",
        "set_current_transcript",
        "insert_recording",
        "delete_recording",
        "fs::",
        "File::",
        "UPDATE",
        "DELETE",
        "INSERT",
    ] {
        assert!(
            !code.contains(forbidden),
            "AI orchestration에 {forbidden}이(가) 있으면 안 된다 (INV-2 · INV-3 · INV-6)"
        );
    }

    // 저장소에 닿는 것은 아래 넷뿐이며 전부 읽기이거나 노트 추가다.
    for expected in [
        "store::load_recording",
        "store::load_transcript",
        "store::insert_ai_note",
        "store::update_recording_statuses",
    ] {
        assert!(code.contains(expected), "{expected}을(를) 찾지 못했다");
    }
}

// --- 재생성은 추가다 (ADR-0008 §9) -------------------------------------------------------

#[test]
fn regenerating_appends_a_new_note_instead_of_replacing_the_old_one() {
    let fixture = Fixture::new("regenerate");
    let provider = FakeNoteAiProvider::ready();
    let transcripts_before = fixture.transcript_tables();

    let first = generated(
        fixture
            .generate(NoteType::Meeting, &provider)
            .expect("첫 생성이 성공해야 한다"),
    );
    let first_row = stored(&fixture, &first);

    let second = generated(
        fixture
            .generate(NoteType::Meeting, &provider)
            .expect("재생성이 성공해야 한다"),
    );

    assert_ne!(first.id, second.id, "새 identity다");

    let notes = fixture.notes_for(&fixture.second);
    assert_eq!(notes.len(), 2, "두 노트가 함께 남는다 (ADR-0008 §9.2)");
    assert_eq!(
        stored(&fixture, &first),
        first_row,
        "이전 노트는 한 글자도 바뀌지 않는다 — UPDATE도 DELETE도 아니다"
    );
    let ids: Vec<&str> = notes.iter().map(|note| note.id.as_str()).collect();
    assert!(ids.contains(&first.id.as_str()), "이전 것이 남는다");
    assert!(ids.contains(&second.id.as_str()), "새 것이 추가된다");

    assert_eq!(
        fixture.transcript_tables(),
        transcripts_before,
        "재생성이 Transcript를 훼손하지 않는다 (INV-2)"
    );
    assert_eq!(fixture.audio_bytes(), AUDIO_BYTES, "오디오도 그대로다 (INV-1)");
    assert_eq!(fixture.recording().ai_status, ProcessingStatus::Done);
}

#[test]
fn regenerating_keeps_the_provenance_of_the_earlier_note() {
    // 이력을 남기는 이유 자체가 이것이다 — 덮어쓰면 이전 model·promptVersion·generatedAt이
    // 사라지고 "전보다 나아졌는가"를 물을 수 없다 (ADR-0008 §9.3).
    let fixture = Fixture::new("provenance-history");
    let first_provider = FakeNoteAiProvider::ready().with_model("이전-모델");
    let second_provider = FakeNoteAiProvider::ready().with_model("새-모델");

    let first = generated(
        fixture
            .generate(NoteType::Study, &first_provider)
            .expect("첫 생성이 성공해야 한다"),
    );
    let second = generated(
        fixture
            .generate(NoteType::Study, &second_provider)
            .expect("재생성이 성공해야 한다"),
    );

    assert_eq!(stored(&fixture, &first).model, "이전-모델");
    assert_eq!(stored(&fixture, &second).model, "새-모델");
    assert_eq!(fixture.notes_for(&fixture.second).len(), 2);
}

// --- 실패가 잃는 것이 없다 (INV-2 · INV-3 · §13) ------------------------------------------

/// provider 실패 세 종류. 사용자가 할 수 있는 일이 셋 다 다르다 (§13 · ADR-0008 §13.1).
fn failing_providers() -> Vec<(&'static str, FakeNoteAiProvider, FailureKind, bool)> {
    vec![
        (
            "연결 불가",
            FakeNoteAiProvider::unreachable(),
            FailureKind::AiProviderUnreachable,
            true,
        ),
        (
            "모델 없음",
            FakeNoteAiProvider::without_models(),
            FailureKind::AiModelUnavailable,
            false,
        ),
        (
            "schema 불일치",
            FakeNoteAiProvider::generating_text("죄송하지만 JSON으로 답할 수 없습니다"),
            FailureKind::AiResponseUnusable,
            true,
        ),
    ]
}

#[test]
fn every_provider_failure_leaves_the_source_data_intact_and_stays_visible() {
    for (label, provider, expected_kind, retryable) in failing_providers() {
        let fixture = Fixture::new("failure");
        let before = fixture.recording();
        let transcripts_before = fixture.transcript_tables();

        let failure = match fixture.generate(NoteType::Meeting, &provider) {
            Ok(outcome) => panic!("{label}: 생성은 실패해야 한다 — {outcome:?}"),
            Err(failure) => failure,
        };

        assert_eq!(failure.kind, expected_kind, "{label}");
        assert_eq!(failure.retryable, retryable, "{label}: 다시 시도할 가치가 있는가");
        assert!(failure.source_data_safe, "{label}: 원본은 안전하다 (INV-3)");
        assert!(
            !failure.message.trim().is_empty(),
            "{label}: 그대로 화면에 띄울 수 있는 문장이어야 한다 (§13)"
        );

        // 실패가 **상태로 남는다.** console 로그로만 처리되면 화면이 그릴 것이 없다.
        let after = fixture.recording();
        assert_eq!(after.ai_status, ProcessingStatus::Failed, "{label}");

        // 바뀐 것은 AI 상태와 updated_at뿐이다 (§7 · INV-3).
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
            "{label}: Transcript와 segment가 그대로다 (INV-2)"
        );
        assert_eq!(fixture.audio_bytes(), AUDIO_BYTES, "{label}: 오디오도 그대로다");
        assert!(
            fixture.all_notes().is_empty(),
            "{label}: 반쯤 채워진 노트를 남기지 않는다"
        );
    }
}

#[test]
fn a_failed_run_can_be_retried_and_then_succeed() {
    // 실패는 막다른 길이 아니다 (§13). 같은 Recording에 같은 요청을 다시 할 수 있다.
    let fixture = Fixture::new("retry");
    let provider = FakeNoteAiProvider::responding_with(vec![
        FakeResponse::Rejected(molt_note_lib::ai::unreachable("AI 서버가 응답하지 않는다")),
        FakeResponse::SampleNote,
    ]);

    let failure = fixture
        .generate(NoteType::Meeting, &provider)
        .expect_err("첫 시도는 실패한다");
    assert!(failure.retryable, "서버를 켜고 다시 누르면 된다");
    assert_eq!(fixture.recording().ai_status, ProcessingStatus::Failed);

    let note = generated(
        fixture
            .generate(NoteType::Meeting, &provider)
            .expect("같은 Recording을 다시 시도할 수 있다"),
    );

    assert_eq!(fixture.recording().ai_status, ProcessingStatus::Done);
    assert_eq!(fixture.notes_for(&fixture.second), vec![stored(&fixture, &note)]);
}

#[test]
fn an_input_that_does_not_fit_is_never_sent_and_is_never_truncated() {
    // ADR-0008 §8.2: 청킹도 절단도 하지 않는다. 넘치면 **요청 자체를 보내지 않고** 사용자가
    // 그 사실을 본다.
    let fixture = Fixture::new("too-large");
    let provider = FakeNoteAiProvider::ready();
    let budget = ContextBudget { context_tokens: 1 };

    let failure = fixture
        .generate_within(NoteType::Meeting, &provider, budget)
        .expect_err("예산을 넘으면 실패한다");

    assert_eq!(failure.kind, FailureKind::AiInputTooLarge);
    assert!(!failure.retryable, "예산을 키우거나 더 짧은 녹음을 골라야 한다");
    assert!(failure.source_data_safe);
    assert_eq!(provider.call_count(), 0, "요청을 보내지 않았다");
    assert!(fixture.all_notes().is_empty(), "잘라서 만든 노트가 남지 않는다");
    assert_eq!(
        fixture.recording().ai_status,
        ProcessingStatus::Failed,
        "그래도 실패는 상태로 보인다 (§13)"
    );
}

#[test]
fn a_missing_recording_is_rejected_before_anything_is_written() {
    let fixture = Fixture::new("unknown");
    let provider = FakeNoteAiProvider::ready();

    let failure = run::generate(
        &fixture.connection,
        &RecordingId::new("rec-없는-것"),
        NoteType::Meeting,
        &provider,
        ContextBudget::DEFAULT,
    )
    .expect_err("없는 녹음에는 노트를 만들 수 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert_eq!(provider.call_count(), 0);
    assert_eq!(
        fixture.recording().ai_status,
        ProcessingStatus::None,
        "다른 Recording의 상태를 건드리지 않는다"
    );
}

// --- 상태 전이 (§7 · §13) ------------------------------------------------------------------

#[test]
fn a_successful_run_stores_pending_then_running_then_done() {
    let fixture = Fixture::new("statuses-done");
    fixture.record_status_writes();
    let provider = FakeNoteAiProvider::ready();

    fixture
        .generate(NoteType::Meeting, &provider)
        .expect("생성이 성공해야 한다");

    assert_eq!(
        fixture.stored_statuses(),
        ["pending", "running", "done"],
        "중간 상태를 건너뛰지 않는다 — 화면이 읽는 값이다"
    );
}

#[test]
fn a_failed_run_stores_pending_then_running_then_failed() {
    let fixture = Fixture::new("statuses-failed");
    fixture.record_status_writes();
    let provider = FakeNoteAiProvider::unreachable();

    fixture
        .generate(NoteType::Meeting, &provider)
        .expect_err("생성은 실패한다");

    assert_eq!(fixture.stored_statuses(), ["pending", "running", "failed"]);
}

#[test]
fn a_recording_without_input_writes_no_status_at_all() {
    let fixture = Fixture::without_transcripts("statuses-no-input");
    fixture.record_status_writes();
    let provider = FakeNoteAiProvider::ready();

    fixture
        .generate(NoteType::Meeting, &provider)
        .expect("실패가 아니다");

    assert!(
        fixture.stored_statuses().is_empty(),
        "아직 입력이 없는 것은 시도가 아니다 — 상태를 옮기지 않는다"
    );
}
