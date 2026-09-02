//! §7 데이터 모델이 요구하는 domain 규칙을 저장소가 실제로 지키는지 검증한다.
//!
//! 통합 테스트이므로 **crate의 공개 API만** 쓴다. 여기서 할 수 없는 일은 제품 코드에서도
//! 할 수 없다는 뜻이다 — Transcript를 갱신하는 경로가 없다는 것이 그렇게 드러난다
//! (§7.1 · INV-2).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::db::{self, store, DatabaseError};
use molt_note_lib::domain::{
    AiNote, AiNoteId, NoteType, NotionSync, ProcessingStatus, Recording, RecordingId, Transcript,
    TranscriptId, TranscriptSegment,
};
use rusqlite::Connection;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 빈 디렉터리. Drop 시 지운다.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-domain-test-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
        Self(path)
    }

    fn database_path(&self) -> PathBuf {
        self.0.join("molt-note.db")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 스키마가 최신인 빈 DB를 연다.
fn open(dir: &TempDir) -> Connection {
    db::open(dir.database_path()).expect("임시 DB를 열 수 있어야 한다")
}

fn recording(id: &str) -> Recording {
    Recording {
        id: RecordingId::new(id),
        title: format!("녹음 {id}"),
        created_at: "2026-09-02T10:00:00Z".to_string(),
        updated_at: "2026-09-02T10:00:00Z".to_string(),
        duration_ms: 93_000,
        audio_path: format!("recordings/{id}.wav"),
        audio_format: "wav".to_string(),
        microphone: Some("MacBook Pro Microphone".to_string()),
        // 녹음 직후에는 아직 성공한 Transcript가 없다 — 정상 상태다 (§7.2).
        current_transcript_id: None,
        transcription_status: ProcessingStatus::None,
        ai_status: ProcessingStatus::None,
        notion_status: ProcessingStatus::None,
    }
}

fn transcript(id: &str, recording_id: &str, text: &str) -> Transcript {
    Transcript {
        id: TranscriptId::new(id),
        recording_id: RecordingId::new(recording_id),
        language: Some("ko".to_string()),
        segments: vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 3_000,
                text: format!("{text} 앞부분"),
            },
            TranscriptSegment {
                start_ms: 3_000,
                end_ms: 7_500,
                text: format!("{text} 뒷부분"),
            },
        ],
        raw_text: text.to_string(),
        created_at: "2026-09-02T10:05:00Z".to_string(),
        engine: "whisper.cpp".to_string(),
        model: "base".to_string(),
    }
}

fn ai_note(id: &str, recording_id: &str, transcript_id: &str) -> AiNote {
    AiNote {
        id: AiNoteId::new(id),
        recording_id: RecordingId::new(recording_id),
        transcript_id: TranscriptId::new(transcript_id),
        note_type: NoteType::Study,
        content: r#"{"overview":"요약","keyConcepts":[]}"#.to_string(),
        provider: "local-runtime".to_string(),
        model: "some-model:8b".to_string(),
        prompt_version: "study-v1".to_string(),
        generated_at: "2026-09-02T11:00:00Z".to_string(),
    }
}

/// 테이블 하나의 열 이름을 선언 순서대로 읽는다.
fn columns_of(connection: &Connection, table: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("스키마를 읽을 수 있어야 한다");
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .expect("스키마를 질의할 수 있어야 한다");
    rows.collect::<Result<Vec<_>, _>>()
        .expect("열 이름을 읽을 수 있어야 한다")
}

// --- §7 스키마 형태 -----------------------------------------------------------

#[test]
fn the_four_concepts_live_in_four_separate_tables_with_the_fields_section_7_lists() {
    let dir = TempDir::new("schema");
    let connection = open(&dir);

    assert_eq!(
        columns_of(&connection, "recordings"),
        [
            "id",
            "title",
            "created_at",
            "updated_at",
            "duration_ms",
            "audio_path",
            "audio_format",
            "microphone",
            "current_transcript_id",
            "transcription_status",
            "ai_status",
            "notion_status",
        ]
    );
    assert_eq!(
        columns_of(&connection, "transcripts"),
        [
            "id",
            "recording_id",
            "language",
            "raw_text",
            "created_at",
            "engine",
            "model",
        ]
    );
    // segments[]는 Transcript의 자식 테이블이다 (ADR-0001 §5.4).
    assert_eq!(
        columns_of(&connection, "transcript_segments"),
        ["transcript_id", "ordinal", "start_ms", "end_ms", "text"]
    );
    assert_eq!(
        columns_of(&connection, "ai_notes"),
        [
            "id",
            "recording_id",
            "transcript_id",
            "note_type",
            "content",
            "provider",
            "model",
            "prompt_version",
            "generated_at",
        ]
    );
    assert_eq!(
        columns_of(&connection, "notion_syncs"),
        ["recording_id", "page_id", "synced_at", "status", "error"]
    );

    // Transcript와 AINote는 서로 다른 테이블이다 — 하나가 다른 하나를 덮어쓸 자리가 없다 (INV-2).
    assert_ne!(
        columns_of(&connection, "transcripts"),
        columns_of(&connection, "ai_notes")
    );
}

// --- §7.2 current Transcript · 상태 필드 -------------------------------------

#[test]
fn a_recording_round_trips_with_no_current_transcript() {
    let dir = TempDir::new("recording");
    let connection = open(&dir);
    let saved = recording("rec-1");

    store::insert_recording(&connection, &saved).expect("Recording을 저장할 수 있어야 한다");
    let loaded = store::load_recording(&connection, &saved.id)
        .expect("읽을 수 있어야 한다")
        .expect("방금 저장한 Recording이 있어야 한다");

    assert_eq!(loaded, saved, "저장한 그대로 복원돼야 한다");
    assert_eq!(
        loaded.current_transcript_id, None,
        "current가 없는 상태도 정상 상태다 (§7.2)"
    );
}

#[test]
fn current_transcript_can_point_at_a_successful_transcript_and_be_cleared_again() {
    let dir = TempDir::new("current");
    let mut connection = open(&dir);
    let recording = recording("rec-current");
    store::insert_recording(&connection, &recording).expect("Recording을 저장할 수 있어야 한다");

    // 저장 직후: 값 없음 상태가 그대로 복원된다.
    let before = store::load_recording(&connection, &recording.id)
        .expect("읽을 수 있어야 한다")
        .expect("Recording이 있어야 한다");
    assert_eq!(before.current_transcript_id, None);

    let successful = transcript("tr-current", "rec-current", "성공한 전사");
    store::append_transcript(&mut connection, &successful).expect("Transcript를 추가할 수 있어야 한다");
    store::set_current_transcript(
        &connection,
        &recording.id,
        Some(&successful.id),
        "2026-09-02T10:06:00Z",
    )
    .expect("current를 지정할 수 있어야 한다");

    let pointing = store::load_recording(&connection, &recording.id)
        .expect("읽을 수 있어야 한다")
        .expect("Recording이 있어야 한다");
    assert_eq!(
        pointing.current_transcript_id,
        Some(successful.id.clone()),
        "성공한 Transcript를 가리킬 수 있어야 한다 (§7.2)"
    );

    // 다시 '값 없음'으로 되돌리는 것도 표현할 수 있어야 한다.
    store::set_current_transcript(&connection, &recording.id, None, "2026-09-02T10:07:00Z")
        .expect("current를 비울 수 있어야 한다");
    let cleared = store::load_recording(&connection, &recording.id)
        .expect("읽을 수 있어야 한다")
        .expect("Recording이 있어야 한다");
    assert_eq!(cleared.current_transcript_id, None);

    // current를 오가는 동안 Transcript 자체는 그대로다 (INV-2).
    let survivor = store::load_transcript(&connection, &successful.id)
        .expect("읽을 수 있어야 한다")
        .expect("Transcript가 있어야 한다");
    assert_eq!(survivor, successful);
}

#[test]
fn all_five_processing_statuses_are_stored_and_restored_distinctly() {
    let dir = TempDir::new("statuses");
    let connection = open(&dir);

    for (index, status) in ProcessingStatus::ALL.into_iter().enumerate() {
        let mut saved = recording(&format!("rec-status-{index}"));
        saved.transcription_status = status;
        // ai_status·notion_status도 같은 다섯 값을 구분해야 하므로 한 칸씩 밀어서 넣는다.
        saved.ai_status = ProcessingStatus::ALL[(index + 1) % ProcessingStatus::ALL.len()];
        saved.notion_status = ProcessingStatus::ALL[(index + 2) % ProcessingStatus::ALL.len()];

        store::insert_recording(&connection, &saved).expect("Recording을 저장할 수 있어야 한다");
        let loaded = store::load_recording(&connection, &saved.id)
            .expect("읽을 수 있어야 한다")
            .expect("Recording이 있어야 한다");

        assert_eq!(loaded.transcription_status, saved.transcription_status);
        assert_eq!(loaded.ai_status, saved.ai_status);
        assert_eq!(loaded.notion_status, saved.notion_status);
    }

    // 다섯 값이 실제로 서로 다른 문자열로 저장됐는지 확인한다 — 하나로 뭉개지면 구분이 아니다.
    let mut stored: Vec<String> = {
        let mut statement = connection
            .prepare("SELECT DISTINCT transcription_status FROM recordings ORDER BY 1")
            .expect("질의를 준비할 수 있어야 한다");
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("질의할 수 있어야 한다");
        rows.collect::<Result<Vec<_>, _>>()
            .expect("행을 읽을 수 있어야 한다")
    };
    stored.sort();
    assert_eq!(
        stored,
        ["done", "failed", "none", "pending", "running"],
        "다섯 상태가 서로 구분돼 저장돼야 한다"
    );
}

#[test]
fn status_changes_move_between_the_five_states_without_touching_the_transcript() {
    let dir = TempDir::new("transitions");
    let mut connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-t"))
        .expect("Recording을 저장할 수 있어야 한다");
    let source = transcript("tr-t", "rec-t", "상태가 변해도 남는 전사");
    store::append_transcript(&mut connection, &source).expect("Transcript를 추가할 수 있어야 한다");

    for (index, status) in ProcessingStatus::ALL.into_iter().enumerate() {
        store::update_recording_statuses(
            &connection,
            &RecordingId::new("rec-t"),
            status,
            status,
            status,
            &format!("2026-09-02T1{index}:00:00Z"),
        )
        .expect("상태를 갱신할 수 있어야 한다");

        let loaded = store::load_recording(&connection, &RecordingId::new("rec-t"))
            .expect("읽을 수 있어야 한다")
            .expect("Recording이 있어야 한다");
        assert_eq!(loaded.transcription_status, status);
        assert_eq!(loaded.ai_status, status);
        assert_eq!(loaded.notion_status, status);
    }

    // 전사·AI·Notion이 실패로 끝나도 Transcript는 훼손되지 않는다 (INV-3 · INV-2).
    let after = store::load_transcript(&connection, &source.id)
        .expect("읽을 수 있어야 한다")
        .expect("Transcript가 남아 있어야 한다");
    assert_eq!(after, source);
}

#[test]
fn storage_refuses_a_status_value_outside_the_five_defined_states() {
    let dir = TempDir::new("bad-status");
    let connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-check"))
        .expect("Recording을 저장할 수 있어야 한다");

    let error = connection
        .execute(
            "UPDATE recordings SET transcription_status = 'cancelled' WHERE id = 'rec-check'",
            [],
        )
        .expect_err("정의되지 않은 상태는 저장되지 않아야 한다");

    assert!(
        error.to_string().to_uppercase().contains("CHECK"),
        "CHECK 제약이 막아야 한다: {error}"
    );
}

// --- §7.1 Transcript는 immutable · versioned ---------------------------------

#[test]
fn appending_a_second_transcript_leaves_the_first_one_untouched() {
    let dir = TempDir::new("versioned");
    let mut connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-v"))
        .expect("Recording을 저장할 수 있어야 한다");

    let first = transcript("tr-1", "rec-v", "첫 번째 전사");
    store::append_transcript(&mut connection, &first).expect("첫 Transcript를 추가할 수 있어야 한다");

    // 재전사 = 기존 것을 고치는 것이 아니라 새 Transcript를 추가하는 행위다 (§7.1).
    let mut second = transcript("tr-2", "rec-v", "두 번째 전사");
    second.created_at = "2026-09-02T12:00:00Z".to_string();
    second.model = "medium".to_string();
    store::append_transcript(&mut connection, &second).expect("둘째 Transcript를 추가할 수 있어야 한다");

    let first_after = store::load_transcript(&connection, &TranscriptId::new("tr-1"))
        .expect("읽을 수 있어야 한다")
        .expect("첫 Transcript가 그대로 있어야 한다");
    assert_eq!(
        first_after, first,
        "둘째를 저장해도 첫 Transcript의 id와 내용이 변하지 않아야 한다 (§7.1)"
    );
    assert_eq!(first_after.id, TranscriptId::new("tr-1"));
    assert_eq!(first_after.raw_text, "첫 번째 전사");
    assert_eq!(first_after.segments, first.segments);
    assert_eq!(first_after.model, "base", "첫 Transcript의 model도 그대로다");

    let all = store::list_transcripts(&connection, &RecordingId::new("rec-v"))
        .expect("목록을 읽을 수 있어야 한다");
    assert_eq!(all, vec![first, second], "둘 다 남아 있어야 한다 (1:N)");
}

#[test]
fn reusing_a_transcript_id_fails_and_the_original_row_survives() {
    let dir = TempDir::new("no-overwrite");
    let mut connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-dup"))
        .expect("Recording을 저장할 수 있어야 한다");

    let original = transcript("tr-dup", "rec-dup", "원본 전사");
    store::append_transcript(&mut connection, &original).expect("추가할 수 있어야 한다");

    let mut impostor = transcript("tr-dup", "rec-dup", "덮어쓰기를 노린 전사");
    impostor.engine = "other-engine".to_string();
    let error = store::append_transcript(&mut connection, &impostor)
        .expect_err("같은 id로 다시 쓰는 것은 실패해야 한다");
    assert!(
        matches!(&error, DatabaseError::Sql(source)
            if source.to_string().to_uppercase().contains("UNIQUE")),
        "identity 충돌로 실패해야 한다: {error}"
    );

    let after = store::load_transcript(&connection, &original.id)
        .expect("읽을 수 있어야 한다")
        .expect("원본이 남아 있어야 한다");
    assert_eq!(after, original, "실패한 재작성이 원본을 바꾸지 않아야 한다");

    let all = store::list_transcripts(&connection, &RecordingId::new("rec-dup"))
        .expect("목록을 읽을 수 있어야 한다");
    assert_eq!(all.len(), 1, "실패한 시도가 행을 남기지 않아야 한다");
}

// --- §7.3 AI Note provenance -------------------------------------------------

#[test]
fn writing_an_ai_note_does_not_overwrite_the_transcript_it_came_from() {
    let dir = TempDir::new("inv2");
    let mut connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-note"))
        .expect("Recording을 저장할 수 있어야 한다");
    let source = transcript("tr-note", "rec-note", "AI가 읽을 원본 전사");
    store::append_transcript(&mut connection, &source).expect("Transcript를 추가할 수 있어야 한다");

    let note = ai_note("note-1", "rec-note", "tr-note");
    store::insert_ai_note(&connection, &note).expect("AI Note를 저장할 수 있어야 한다");
    // 재생성도 Transcript를 건드리지 않는다 (§9.6 · INV-2).
    let mut regenerated = ai_note("note-2", "rec-note", "tr-note");
    regenerated.content = r#"{"overview":"다시 만든 요약","keyConcepts":[]}"#.to_string();
    regenerated.generated_at = "2026-09-02T12:30:00Z".to_string();
    store::insert_ai_note(&connection, &regenerated).expect("재생성 노트를 저장할 수 있어야 한다");

    let transcript_after = store::load_transcript(&connection, &source.id)
        .expect("읽을 수 있어야 한다")
        .expect("Transcript가 남아 있어야 한다");
    assert_eq!(
        transcript_after, source,
        "AI Note를 써도 Transcript는 그대로여야 한다 (INV-2)"
    );

    // 서로 다른 레코드다 — 노트를 두 개 써도 Transcript는 하나뿐이고 내용이 섞이지 않는다.
    let notes = store::list_ai_notes_for_transcript(&connection, &source.id)
        .expect("노트를 읽을 수 있어야 한다");
    assert_eq!(notes.len(), 2, "AI Note는 Transcript와 별개의 레코드다");
    assert_ne!(notes[0].content, transcript_after.raw_text);
    assert_ne!(notes[1].content, transcript_after.raw_text);
    assert_eq!(
        store::list_transcripts(&connection, &RecordingId::new("rec-note"))
            .expect("목록을 읽을 수 있어야 한다")
            .len(),
        1
    );
}

#[test]
fn ai_notes_are_traced_back_to_the_transcript_version_they_came_from() {
    let dir = TempDir::new("provenance");
    let mut connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-prov"))
        .expect("Recording을 저장할 수 있어야 한다");

    let first = transcript("tr-a", "rec-prov", "Transcript A");
    let mut second = transcript("tr-b", "rec-prov", "Transcript B");
    second.created_at = "2026-09-02T13:00:00Z".to_string();
    store::append_transcript(&mut connection, &first).expect("A를 추가할 수 있어야 한다");
    store::append_transcript(&mut connection, &second).expect("B를 추가할 수 있어야 한다");

    let mut from_a = ai_note("note-a1", "rec-prov", "tr-a");
    from_a.provider = "provider-one".to_string();
    from_a.model = "model-one".to_string();
    from_a.prompt_version = "study-v1".to_string();
    from_a.generated_at = "2026-09-02T13:10:00Z".to_string();

    let mut from_b = ai_note("note-b1", "rec-prov", "tr-b");
    from_b.note_type = NoteType::Meeting;
    from_b.provider = "provider-two".to_string();
    from_b.model = "model-two".to_string();
    from_b.prompt_version = "meeting-v3".to_string();
    from_b.generated_at = "2026-09-02T13:20:00Z".to_string();

    store::insert_ai_note(&connection, &from_a).expect("A의 노트를 저장할 수 있어야 한다");
    store::insert_ai_note(&connection, &from_b).expect("B의 노트를 저장할 수 있어야 한다");

    // recordingId는 같지만 transcriptId로 출처가 갈린다 (§7.3).
    assert_eq!(from_a.recording_id, from_b.recording_id);
    let notes_of_a = store::list_ai_notes_for_transcript(&connection, &first.id)
        .expect("A의 노트를 읽을 수 있어야 한다");
    let notes_of_b = store::list_ai_notes_for_transcript(&connection, &second.id)
        .expect("B의 노트를 읽을 수 있어야 한다");
    assert_eq!(notes_of_a, vec![from_a.clone()]);
    assert_eq!(notes_of_b, vec![from_b.clone()]);

    // provenance 다섯 항목이 그대로 복원된다 (§7.3 · §9.6).
    let restored = store::load_ai_note(&connection, &from_b.id)
        .expect("읽을 수 있어야 한다")
        .expect("노트가 있어야 한다");
    assert_eq!(restored.transcript_id, TranscriptId::new("tr-b"));
    assert_eq!(restored.provider, "provider-two");
    assert_eq!(restored.model, "model-two");
    assert_eq!(restored.prompt_version, "meeting-v3");
    assert_eq!(restored.generated_at, "2026-09-02T13:20:00Z");
    assert_eq!(restored, from_b);
}

#[test]
fn an_ai_note_provider_is_an_opaque_identifier_the_domain_does_not_interpret() {
    let dir = TempDir::new("provider");
    let mut connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-any"))
        .expect("Recording을 저장할 수 있어야 한다");
    let source = transcript("tr-any", "rec-any", "전사");
    store::append_transcript(&mut connection, &source).expect("Transcript를 추가할 수 있어야 한다");

    // 저장소도 domain도 provider 목록을 갖지 않는다 (INV-9). 처음 보는 식별자도 그대로 저장된다.
    let identifiers = ["local-runtime", "some-vendor-x", "self-hosted/gateway-7", "미지정"];
    for (index, identifier) in identifiers.iter().enumerate() {
        let mut note = ai_note(&format!("note-{index}"), "rec-any", "tr-any");
        note.provider = (*identifier).to_string();
        store::insert_ai_note(&connection, &note).expect("어떤 provider 식별자든 저장돼야 한다");

        let restored = store::load_ai_note(&connection, &note.id)
            .expect("읽을 수 있어야 한다")
            .expect("노트가 있어야 한다");
        assert_eq!(restored.provider, *identifier);
    }
}

// --- 참조 무결성 -------------------------------------------------------------

#[test]
fn an_ai_note_cannot_claim_a_transcript_that_belongs_to_another_recording() {
    let dir = TempDir::new("cross-note");
    let mut connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-x"))
        .expect("Recording x를 저장할 수 있어야 한다");
    store::insert_recording(&connection, &recording("rec-y"))
        .expect("Recording y를 저장할 수 있어야 한다");
    store::append_transcript(&mut connection, &transcript("tr-x", "rec-x", "x의 전사"))
        .expect("Transcript를 추가할 수 있어야 한다");

    // recordingId는 y인데 transcriptId는 x의 것이다 — provenance가 어긋난다.
    let mismatched = ai_note("note-bad", "rec-y", "tr-x");

    let error = store::insert_ai_note(&connection, &mismatched)
        .expect_err("어긋난 provenance는 저장되지 않아야 한다");
    assert!(
        matches!(&error, DatabaseError::Sql(source)
            if source.to_string().to_uppercase().contains("FOREIGN KEY")),
        "(transcript_id, recording_id) 복합 FK가 막아야 한다: {error}"
    );
    assert_eq!(
        store::load_ai_note(&connection, &mismatched.id).expect("읽을 수 있어야 한다"),
        None,
        "거부된 노트는 저장되지 않아야 한다"
    );
}

#[test]
fn current_transcript_cannot_point_at_another_recordings_transcript() {
    let dir = TempDir::new("cross-current");
    let mut connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-p"))
        .expect("Recording p를 저장할 수 있어야 한다");
    store::insert_recording(&connection, &recording("rec-q"))
        .expect("Recording q를 저장할 수 있어야 한다");
    store::append_transcript(&mut connection, &transcript("tr-q", "rec-q", "q의 전사"))
        .expect("Transcript를 추가할 수 있어야 한다");

    let error = store::set_current_transcript(
        &connection,
        &RecordingId::new("rec-p"),
        Some(&TranscriptId::new("tr-q")),
        "2026-09-02T14:00:00Z",
    )
    .expect_err("남의 Transcript를 current로 지정할 수 없어야 한다");
    assert!(
        matches!(&error, DatabaseError::Sql(source)
            if source.to_string().to_uppercase().contains("FOREIGN KEY")),
        "(current_transcript_id, id) 복합 FK가 막아야 한다: {error}"
    );

    // 거부된 지정이 Recording을 바꾸지 않았다.
    assert_eq!(
        store::load_recording(&connection, &RecordingId::new("rec-p"))
            .expect("읽을 수 있어야 한다")
            .expect("Recording이 있어야 한다")
            .current_transcript_id,
        None
    );
}

// --- NotionSync ---------------------------------------------------------------

#[test]
fn notion_sync_round_trips_including_the_failure_shape() {
    let dir = TempDir::new("notion");
    let connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-n"))
        .expect("Recording을 저장할 수 있어야 한다");

    let failed = NotionSync {
        recording_id: RecordingId::new("rec-n"),
        page_id: None,
        synced_at: None,
        status: ProcessingStatus::Failed,
        error: Some("네트워크에 연결할 수 없다".to_string()),
    };
    store::save_notion_sync(&connection, &failed).expect("실패 기록을 저장할 수 있어야 한다");
    assert_eq!(
        store::load_notion_sync(&connection, &failed.recording_id)
            .expect("읽을 수 있어야 한다")
            .expect("기록이 있어야 한다"),
        failed
    );

    let succeeded = NotionSync {
        recording_id: RecordingId::new("rec-n"),
        page_id: Some("page-123".to_string()),
        synced_at: Some("2026-09-02T15:00:00Z".to_string()),
        status: ProcessingStatus::Done,
        error: None,
    };
    store::save_notion_sync(&connection, &succeeded).expect("성공 기록을 저장할 수 있어야 한다");
    assert_eq!(
        store::load_notion_sync(&connection, &succeeded.recording_id)
            .expect("읽을 수 있어야 한다")
            .expect("기록이 있어야 한다"),
        succeeded
    );

    let never_tried = store::load_notion_sync(&connection, &RecordingId::new("rec-unknown"))
        .expect("읽을 수 있어야 한다");
    assert_eq!(never_tried, None, "시도한 적 없으면 기록이 없다");
}

// --- 재시작 후에도 남는다 -------------------------------------------------------

#[test]
fn domain_rows_survive_closing_and_reopening_the_database() {
    let dir = TempDir::new("reopen");
    let path: PathBuf = dir.database_path();

    let saved_transcript = {
        let mut connection = db::open(&path).expect("처음 열 수 있어야 한다");
        store::insert_recording(&connection, &recording("rec-r"))
            .expect("Recording을 저장할 수 있어야 한다");
        let saved = transcript("tr-r", "rec-r", "재시작 후에도 남아야 한다");
        store::append_transcript(&mut connection, &saved).expect("Transcript를 추가할 수 있어야 한다");
        store::insert_ai_note(&connection, &ai_note("note-r", "rec-r", "tr-r"))
            .expect("AI Note를 저장할 수 있어야 한다");
        store::set_current_transcript(&connection, &RecordingId::new("rec-r"), Some(&saved.id), "2026-09-02T16:00:00Z")
            .expect("current를 지정할 수 있어야 한다");
        connection
            .close()
            .map_err(|(_, error)| error)
            .expect("연결이 닫혀야 한다");
        saved
    };

    let reopened = db::open(&path).expect("다시 열 수 있어야 한다");
    assert_eq!(
        store::load_transcript(&reopened, &saved_transcript.id)
            .expect("읽을 수 있어야 한다")
            .expect("Transcript가 남아 있어야 한다"),
        saved_transcript
    );
    let restored_recording = store::load_recording(&reopened, &RecordingId::new("rec-r"))
        .expect("읽을 수 있어야 한다")
        .expect("Recording이 남아 있어야 한다");
    assert_eq!(
        restored_recording.current_transcript_id,
        Some(saved_transcript.id)
    );
    assert_eq!(
        store::load_ai_note(&reopened, &AiNoteId::new("note-r"))
            .expect("읽을 수 있어야 한다")
            .map(|note| note.transcript_id),
        Some(TranscriptId::new("tr-r"))
    );
}

#[test]
fn tests_never_write_outside_the_system_temp_directory() {
    let dir = TempDir::new("sandbox");
    assert!(
        Path::new(&dir.0).starts_with(std::env::temp_dir()),
        "테스트는 실제 사용자 앱 데이터 디렉터리를 건드리지 않는다"
    );
}
