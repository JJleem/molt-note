//! **Recording 하나가 실제 Markdown 파일이 된다** (PRODUCT-SPEC §11 ·
//! `docs/ADR-0009-notion-and-export.md` §4 · `phase-prompt/05` 요구 A-1~3).
//!
//! 순수 렌더러(`export::markdown` · `export::filename`)는 자기 파일 안에서 값으로 검증된다.
//! 여기서 판정하는 것은 **그 문자열이 파일이 되는 자리**이며, 그 자리에만 있는 네 가지 질문에
//! 답한다.
//!
//! ```text
//! 1. 파일이 실제로 만들어지는가 · 이름과 내용이 §11 · ADR-0009와 같은가        (AC4)
//! 2. AI Note가 하나도 없는 녹음도 내보내지는가                                  (AC5 · INV-8)
//! 3. 실패가 domain Failure이고, 실패 뒤에도 저장된 것이 그대로인가              (AC6 · INV-3)
//! 4. 같은 이름이 이미 있을 때 조용히 덮어쓰지 않는가                            (AC7 · §4.3)
//! ```
//!
//! **전부 임시 디렉터리에서 돈다.** 경로는 `std::env::temp_dir()`에서만 나오므로 사용자의 실제
//! export 디렉터리에는 아무것도 쓰이지 않는다 (§18 · Task 요구).
//!
//! AI 서버도, 실제 whisper도, 마이크도, Tauri 런타임도 필요하지 않다 — 저장소에 값을 직접 넣고
//! 제품 경로([`Exporter`])를 그대로 지난다. 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::ai::note::{encode_content, MeetingNote, StructuredNote, SummaryNote};
use molt_note_lib::commands::{ExportedFilePayload, Exporter, Storage};
use molt_note_lib::db::{self, store};
use molt_note_lib::domain::{
    AiNote, AiNoteId, Failure, FailureKind, NoteType, ProcessingStatus, Recording, RecordingId,
    Transcript, TranscriptId, TranscriptSegment,
};
use molt_note_lib::export;
use molt_note_lib::platform::app_data_dir::AppDataDirectory;

/// 이 파일이 쓰는 녹음 시각. 파일 이름의 날짜가 여기서 나온다 (ADR-0009 §4.2).
const CREATED_AT: &str = "2026-09-01T10:00:00.000Z";

/// 3151초 = `52:31`. §5 A 화면과 §11 예시가 쓰는 그 값이다.
const DURATION_MS: i64 = 3_151_000;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 자리 ----------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-markdown-export-{}-{}-{}",
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

/// 저장소 하나와 그 위에서 도는 export 실행자.
struct Fixture {
    /// Drop 시 임시 디렉터리를 지운다.
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    exporter: Exporter,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.path().join("app-data"));

        // 스키마를 만드는 것은 앱과 같은 경로다. 이 값 자체는 이후에 쓰지 않는다.
        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

        Self {
            exporter: Exporter::in_directory(app_data_dir.clone()),
            app_data_dir,
            _root: root,
        }
    }

    fn connection(&self) -> rusqlite::Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    fn exports_dir(&self) -> PathBuf {
        self.app_data_dir.exports_dir()
    }

    /// 녹음 하나를 저장한다. 오디오 파일은 만들지 않는다 — export는 그것을 읽지 않는다 (INV-6).
    fn save_recording(&self, id: &str, title: &str) -> RecordingId {
        let recording = Recording {
            id: RecordingId::new(id),
            title: title.to_string(),
            created_at: CREATED_AT.to_string(),
            updated_at: CREATED_AT.to_string(),
            duration_ms: DURATION_MS,
            audio_path: format!("recordings/{id}.wav"),
            audio_format: "wav".to_string(),
            microphone: Some("가짜 마이크".to_string()),
            current_transcript_id: None,
            transcription_status: ProcessingStatus::Done,
            ai_status: ProcessingStatus::None,
            notion_status: ProcessingStatus::None,
        };
        store::insert_recording(&self.connection(), &recording).expect("사전 조건: 녹음을 저장한다");

        recording.id
    }

    /// Transcript 하나를 추가하고 current로 지정한다 (§7.2).
    fn save_transcript(&self, recording_id: &RecordingId, id: &str) -> TranscriptId {
        let transcript = Transcript {
            id: TranscriptId::new(id),
            recording_id: recording_id.clone(),
            language: Some("ko".to_string()),
            segments: vec![
                TranscriptSegment {
                    start_ms: 3_000,
                    end_ms: 6_000,
                    text: "첫 문장".to_string(),
                },
                TranscriptSegment {
                    start_ms: 65_000,
                    end_ms: 70_000,
                    text: "둘째 문장".to_string(),
                },
            ],
            raw_text: "첫 문장 둘째 문장".to_string(),
            created_at: CREATED_AT.to_string(),
            engine: "stub".to_string(),
            model: "ggml-base.bin".to_string(),
        };

        let mut connection = self.connection();
        store::append_transcript(&mut connection, &transcript).expect("사전 조건: 전사를 저장한다");
        store::set_current_transcript(
            &connection,
            recording_id,
            Some(&transcript.id),
            CREATED_AT,
        )
        .expect("사전 조건: current를 지정한다");

        transcript.id
    }

    /// AI 노트 하나를 저장한다. `generated_at`으로 순서가 정해진다 (§7.3).
    fn save_note(
        &self,
        recording_id: &RecordingId,
        transcript_id: &TranscriptId,
        id: &str,
        generated_at: &str,
        note: StructuredNote,
    ) {
        let stored = AiNote {
            id: AiNoteId::new(id),
            recording_id: recording_id.clone(),
            transcript_id: transcript_id.clone(),
            note_type: note.mode(),
            content: encode_content(&note),
            provider: "stub".to_string(),
            model: "stub-model".to_string(),
            prompt_version: "v1".to_string(),
            generated_at: generated_at.to_string(),
        };
        store::insert_ai_note(&self.connection(), &stored).expect("사전 조건: 노트를 저장한다");
    }

    fn export(&self, recording_id: &RecordingId) -> Result<ExportedFilePayload, Failure> {
        self.exporter.export(recording_id.as_str())
    }

    /// 저장된 것 전부를 한 값으로 찍는다. **실패 전후를 그대로 비교하기 위한 것이다** (INV-3).
    fn snapshot(&self, recording_id: &RecordingId) -> Snapshot {
        let connection = self.connection();
        let recording = store::load_recording(&connection, recording_id)
            .expect("녹음을 읽을 수 있어야 한다")
            .expect("녹음이 있어야 한다");
        let transcripts = store::list_transcripts(&connection, recording_id)
            .expect("전사를 읽을 수 있어야 한다");
        let notes = transcripts
            .iter()
            .flat_map(|transcript| {
                store::list_ai_notes_for_transcript(&connection, &transcript.id)
                    .expect("노트를 읽을 수 있어야 한다")
            })
            .collect();

        Snapshot {
            recording,
            transcripts,
            notes,
        }
    }
}

/// recording · transcript · ai_note를 한 번에 비교하기 위한 값 (INV-3).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    recording: Recording,
    transcripts: Vec<Transcript>,
    notes: Vec<AiNote>,
}

/// 이 파일이 쓰는 Summary 노트 하나.
fn summary_note() -> StructuredNote {
    StructuredNote::Summary(SummaryNote {
        short_summary: "세 줄 요약".to_string(),
        key_points: vec!["첫 번째".to_string(), "두 번째".to_string()],
    })
}

/// 내보낸 파일을 읽는다.
fn written(exported: &ExportedFilePayload) -> String {
    std::fs::read_to_string(&exported.path).expect("내보낸 파일을 읽을 수 있어야 한다")
}

// --- 1. 파일이 만들어지고, 이름과 내용이 §11 · ADR-0009와 같다 (AC4) ---------------------

#[test]
fn a_recording_becomes_a_markdown_file_whose_name_and_body_follow_the_spec() {
    let fixture = Fixture::new("one-recording");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let transcript_id = fixture.save_transcript(&recording_id, "tr-1");
    fixture.save_note(
        &recording_id,
        &transcript_id,
        "note-1",
        "2026-09-01T11:00:00.000Z",
        summary_note(),
    );

    let exported = fixture.export(&recording_id).expect("export가 성공해야 한다");

    // 이름 — `<created_at의 날짜>-<제목 슬러그>.md` (ADR-0009 §4.2).
    assert_eq!(exported.file_name, "2026-09-01-3dgs-study-04.md");
    assert_eq!(exported.recording_id, "rec-1");
    assert_eq!(
        exported.path,
        fixture
            .exports_dir()
            .join("2026-09-01-3dgs-study-04.md")
            .display()
            .to_string(),
        "앱 데이터 루트 아래 exports/에 놓인다 (ADR-0009 §4.1)"
    );
    assert!(
        Path::new(&exported.path).is_file(),
        "돌려준 경로에 파일이 실제로 있어야 한다"
    );

    // 내용 — §11의 구조 그대로다. 문자열 전체를 고정한다.
    assert_eq!(
        written(&exported),
        "# 3DGS Study #04\n\
         \n\
         Date: 2026-09-01\n\
         Duration: 52:31\n\
         \n\
         ## Short Summary\n\
         세 줄 요약\n\
         \n\
         ## Key Points\n\
         - 첫 번째\n\
         - 두 번째\n\
         \n\
         ## Transcript\n\
         \n\
         ### 00:00:03\n\
         첫 문장\n\
         \n\
         ### 00:01:05\n\
         둘째 문장\n"
    );
}

#[test]
fn a_title_full_of_dangerous_characters_still_lands_in_the_exports_directory() {
    // 슬러그 규칙은 순수 함수 쪽에서 값으로 검증된다. 여기서 보는 것은 **그 이름이 실제
    // 파일시스템에서도 경로를 벗어나지 않는가**다 (ADR-0009 §4.2 · `phase-prompt/05` 요구 A-2).
    let fixture = Fixture::new("dangerous-title");
    let recording_id = fixture.save_recording("rec-1", "회의: 로드맵 / Q4 🎯\n../../etc");
    fixture.save_transcript(&recording_id, "tr-1");

    let exported = fixture.export(&recording_id).expect("export가 성공해야 한다");
    let path = PathBuf::from(&exported.path);

    assert_eq!(
        path.parent(),
        Some(fixture.exports_dir().as_path()),
        "이름이 디렉터리를 벗어나지 않는다: {}",
        exported.path
    );
    assert!(exported.file_name.starts_with("2026-09-01-"));
    assert!(exported.file_name.ends_with(".md"));
    assert!(!exported.file_name.contains(".."));
    assert!(path.is_file());
}

#[test]
fn the_most_recently_generated_note_is_the_one_that_ends_up_in_the_document() {
    // "(있으면) 최신 AI Note"를 무엇으로 정하는지 고정한다 — 저장소 순서의 마지막이며,
    // mode로 가려내지 않는다 (`export::run::latest_note`).
    let fixture = Fixture::new("latest-note");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let transcript_id = fixture.save_transcript(&recording_id, "tr-1");
    fixture.save_note(
        &recording_id,
        &transcript_id,
        "note-old",
        "2026-09-01T11:00:00.000Z",
        StructuredNote::Meeting(MeetingNote {
            overview: "먼저 만든 노트".to_string(),
            key_discussions: Vec::new(),
            decisions: Vec::new(),
            action_items: Vec::new(),
            open_questions: Vec::new(),
        }),
    );
    fixture.save_note(
        &recording_id,
        &transcript_id,
        "note-new",
        "2026-09-01T12:00:00.000Z",
        summary_note(),
    );

    let document = written(&fixture.export(&recording_id).expect("export가 성공해야 한다"));

    assert!(document.contains("## Short Summary"), "나중 노트가 들어간다");
    assert!(
        !document.contains("먼저 만든 노트"),
        "이전 노트가 함께 붙지 않는다: {document}"
    );
}

// --- 2. AI Note가 없어도 내보내진다 (AC5 · INV-8) ----------------------------------------

#[test]
fn a_recording_with_no_ai_note_at_all_is_exported_as_a_valid_document() {
    // §17.1의 core 성공 기준이다 — 선택적 편의가 아니다. AI 설정도 노트도 하나도 없다.
    let fixture = Fixture::new("no-ai-note");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let transcript_id = fixture.save_transcript(&recording_id, "tr-1");
    assert!(
        store::list_ai_notes_for_transcript(&fixture.connection(), &transcript_id)
            .expect("노트 목록을 읽는다")
            .is_empty(),
        "사전 조건: 노트가 하나도 없다"
    );

    let exported = fixture.export(&recording_id).expect("AI 노트가 없어도 성공한다 (INV-8)");
    let document = written(&exported);

    assert_eq!(exported.file_name, "2026-09-01-3dgs-study-04.md");
    assert_eq!(
        document,
        "# 3DGS Study #04\n\
         \n\
         Date: 2026-09-01\n\
         Duration: 52:31\n\
         \n\
         ## Transcript\n\
         \n\
         ### 00:00:03\n\
         첫 문장\n\
         \n\
         ### 00:01:05\n\
         둘째 문장\n",
        "Transcript와 메타데이터만으로 읽을 수 있는 문서가 된다"
    );
    // 없는 AI 섹션의 빈 껍데기를 남기지 않는다 — 빈 제목은 "AI가 실패했다"처럼 보인다.
    for heading in ["## Overview", "## Short Summary", "## Key Points", "## Decisions"] {
        assert!(!document.contains(heading), "{heading}이 남아 있다: {document}");
    }
}

#[test]
fn exporting_reads_no_audio_and_copies_none() {
    // INV-6: 오디오는 복사하지도 읽지도 않는다. export 디렉터리에 남는 것은 `.md` 하나뿐이다.
    let fixture = Fixture::new("no-audio");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1");

    let recordings_dir = fixture
        .app_data_dir
        .ensure_recordings_dir()
        .expect("사전 조건: 녹음 디렉터리");
    let audio = recordings_dir.join("rec-1.wav");
    std::fs::write(&audio, b"pretend audio").expect("사전 조건: 오디오 파일을 둔다");

    fixture.export(&recording_id).expect("export가 성공해야 한다");

    assert_eq!(
        std::fs::read(&audio).expect("오디오가 남아 있어야 한다"),
        b"pretend audio",
        "오디오 파일은 그대로다 (INV-1 · INV-6)"
    );
    let exported: Vec<String> = std::fs::read_dir(fixture.exports_dir())
        .expect("export 디렉터리를 읽는다")
        .map(|entry| entry.expect("항목").file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(exported.len(), 1, "만들어진 것은 문서 하나뿐이다: {exported:?}");
    assert!(exported[0].ends_with(".md"), "오디오가 복사되지 않았다: {exported:?}");
}

// --- 3. 실패는 domain Failure이고 원본을 훼손하지 않는다 (AC6 · INV-3) ---------------------

#[test]
fn a_directory_that_cannot_be_created_fails_visibly_and_changes_nothing() {
    // export 디렉터리가 있어야 할 자리에 파일이 있다. 가장 이른 실패 지점이다.
    let fixture = Fixture::new("blocked-directory");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let transcript_id = fixture.save_transcript(&recording_id, "tr-1");
    fixture.save_note(
        &recording_id,
        &transcript_id,
        "note-1",
        "2026-09-01T11:00:00.000Z",
        summary_note(),
    );
    std::fs::write(fixture.exports_dir(), "디렉터리가 아니다").expect("사전 조건: 파일을 둔다");
    let before = fixture.snapshot(&recording_id);

    let failure = fixture
        .export(&recording_id)
        .expect_err("자리를 만들지 못하면 내보낼 수 없다");

    assert_eq!(failure.kind, FailureKind::Storage);
    assert!(!failure.message.trim().is_empty(), "화면에 띄울 문장이 있다");
    assert!(failure.source_data_safe, "원본은 안전하다 (INV-3)");
    assert!(failure.retryable, "자리를 비우면 성공할 수 있다");
    assert_eq!(
        fixture.snapshot(&recording_id),
        before,
        "실패 뒤에도 recording · transcript · ai_note가 그대로다 (INV-3)"
    );
}

#[test]
fn a_write_that_fails_is_a_domain_failure_that_leaves_the_stored_data_untouched() {
    // 자리는 있는데 **쓰지 못하는** 경우다 — 실행 순서를 없는 디렉터리로 직접 부른다.
    let fixture = Fixture::new("write-failure");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let transcript_id = fixture.save_transcript(&recording_id, "tr-1");
    fixture.save_note(
        &recording_id,
        &transcript_id,
        "note-1",
        "2026-09-01T11:00:00.000Z",
        summary_note(),
    );
    let before = fixture.snapshot(&recording_id);
    let missing = fixture.exports_dir().join("없는-자리");

    let failure = export::run::export(&fixture.connection(), &missing, &recording_id)
        .expect_err("쓸 수 없는 자리에는 만들지 못한다");

    assert_eq!(failure.kind, FailureKind::Storage);
    assert!(failure.source_data_safe, "원본은 안전하다 (INV-3)");
    assert!(failure.detail.is_some(), "기술적 원인이 함께 실린다");
    assert!(!missing.exists(), "실패한 자리에 무언가 만들어 두지 않는다");
    assert_eq!(
        fixture.snapshot(&recording_id),
        before,
        "실패 뒤에도 recording · transcript · ai_note가 그대로다"
    );
}

#[test]
fn a_recording_that_is_not_there_is_refused_without_creating_a_file() {
    let fixture = Fixture::new("unknown-recording");

    let failure = fixture
        .exporter
        .export("없는-녹음")
        .expect_err("없는 녹음은 내보낼 수 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(failure.source_data_safe);
    assert_eq!(
        std::fs::read_dir(fixture.exports_dir())
            .expect("디렉터리를 읽는다")
            .count(),
        0,
        "실패한 요청이 파일을 남기지 않는다"
    );
}

#[test]
fn a_recording_without_a_transcript_is_refused_instead_of_leaving_an_empty_document() {
    // 제목과 길이만 담긴 파일은 "export가 잘못됐다"처럼 보이면서 사용자의 디렉터리에 쌓인다.
    // 대신 무엇이 필요한지 말한다 (§13). **AI Note가 없는 것과는 다른 상황이다** (INV-8).
    let fixture = Fixture::new("no-transcript");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let before = fixture.snapshot(&recording_id);

    let failure = fixture
        .export(&recording_id)
        .expect_err("내보낼 내용이 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(failure.retryable, "전사가 끝나면 성공할 수 있다");
    assert!(failure.source_data_safe);
    assert_eq!(fixture.snapshot(&recording_id), before, "저장된 것은 그대로다");
    assert_eq!(
        std::fs::read_dir(fixture.exports_dir())
            .expect("디렉터리를 읽는다")
            .count(),
        0,
        "빈 문서를 남기지 않는다"
    );
}

// --- 4. 같은 이름이 이미 있을 때 (AC7 · ADR-0009 §4.3) ------------------------------------

#[test]
fn exporting_the_same_recording_again_never_overwrites_the_earlier_file() {
    // export한 파일은 **사용자의 문서다.** 손댔을 수도 있고 옮기는 중일 수도 있다.
    let fixture = Fixture::new("collision");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1");

    let first = fixture.export(&recording_id).expect("첫 export");
    // 사용자가 그 파일을 Obsidian에서 고쳤다.
    std::fs::write(&first.path, "# 사용자가 고친 문서\n").expect("사전 조건: 사용자가 손댄다");

    let second = fixture.export(&recording_id).expect("두 번째 export도 성공한다");
    let third = fixture.export(&recording_id).expect("세 번째 export도 성공한다");

    assert_eq!(first.file_name, "2026-09-01-3dgs-study-04.md");
    assert_eq!(second.file_name, "2026-09-01-3dgs-study-04-2.md");
    assert_eq!(third.file_name, "2026-09-01-3dgs-study-04-3.md");
    assert_eq!(
        std::fs::read_to_string(&first.path).expect("첫 파일이 남아 있어야 한다"),
        "# 사용자가 고친 문서\n",
        "기존 파일을 조용히 덮어쓰지 않는다 (ADR-0009 §4.3)"
    );
    assert!(written(&second).starts_with("# 3DGS Study #04"));
    assert_eq!(
        written(&second),
        written(&third),
        "같은 입력은 같은 문서를 만든다 — 이름만 다르다"
    );
    assert_eq!(
        std::fs::read_dir(fixture.exports_dir())
            .expect("디렉터리를 읽는다")
            .count(),
        3,
        "세 파일이 함께 남는다"
    );
}

#[test]
fn two_recordings_with_the_same_title_and_date_do_not_collapse_into_one_file() {
    // 이름이 결정론적이라는 것은 **다른 녹음이 같은 이름을 가질 수 있다**는 뜻이기도 하다.
    // 그때도 앞의 것이 사라지지 않는다.
    let fixture = Fixture::new("same-name");
    let first_id = fixture.save_recording("rec-1", "주간 회의");
    fixture.save_transcript(&first_id, "tr-1");
    let second_id = fixture.save_recording("rec-2", "주간 회의");
    fixture.save_transcript(&second_id, "tr-2");

    let first = fixture.export(&first_id).expect("첫 녹음을 내보낸다");
    let second = fixture.export(&second_id).expect("둘째 녹음도 내보낸다");

    assert_eq!(first.file_name, "2026-09-01-주간-회의.md");
    assert_eq!(second.file_name, "2026-09-01-주간-회의-2.md");
    assert_eq!(first.recording_id, "rec-1");
    assert_eq!(second.recording_id, "rec-2");
    assert!(Path::new(&first.path).is_file() && Path::new(&second.path).is_file());
}

#[test]
fn the_note_type_of_the_stored_row_is_the_one_the_document_uses() {
    // 봉투와 열이 어긋난 행은 저장 시점에 막힌다 (ADR-0008 §7.5). export는 그 규약을 그대로
    // 따르며, 노트 종류에 따라 §9.5의 섹션 이름이 달라진다.
    let fixture = Fixture::new("note-type");
    let recording_id = fixture.save_recording("rec-1", "주간 회의");
    let transcript_id = fixture.save_transcript(&recording_id, "tr-1");
    fixture.save_note(
        &recording_id,
        &transcript_id,
        "note-1",
        "2026-09-01T11:00:00.000Z",
        StructuredNote::Meeting(MeetingNote {
            overview: "이번 주 진행 상황".to_string(),
            key_discussions: vec!["일정".to_string()],
            decisions: Vec::new(),
            action_items: vec!["문서 정리".to_string()],
            open_questions: Vec::new(),
        }),
    );
    assert_eq!(
        store::list_ai_notes_for_transcript(&fixture.connection(), &transcript_id)
            .expect("노트를 읽는다")[0]
            .note_type,
        NoteType::Meeting,
        "사전 조건: meeting 노트다"
    );

    let document = written(&fixture.export(&recording_id).expect("export가 성공해야 한다"));

    assert!(document.contains("## Overview\n이번 주 진행 상황"));
    assert!(document.contains("## Key Discussions\n- 일정"));
    assert!(document.contains("## Action Items\n- 문서 정리"));
    // 내용이 없는 섹션은 제목도 남기지 않는다 — 빈 제목은 실패처럼 보인다.
    assert!(!document.contains("## Decisions"), "빈 섹션: {document}");
    assert!(!document.contains("## Open Questions"), "빈 섹션: {document}");
}
