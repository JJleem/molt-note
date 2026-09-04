//! Recording 하나를 Markdown 파일로 내보내는 **실행 순서**
//! (PRODUCT-SPEC §11 · `docs/ADR-0009-notion-and-export.md` §4 · `phase-prompt/05` 요구 A-1~3).
//!
//! 앞의 두 모듈이 각자 순수 함수 하나씩을 맡았고, 그것을 저장소·파일시스템과 잇는 자리는
//! 여기 하나다. [`crate::ai::run`]이 노트 생성에 대해 하는 역할과 같은 자리이며, 같은 규칙을
//! 따른다.
//!
//! ```text
//! Recording 레코드 ─→ current Transcript ─→ (있으면) 최신 AI Note ─→ markdown::render ─→ 문자열
//!      §7.2의 기본 입력                        §7.3 · 없으면 None                             │
//!                                                                                             ▼
//!                              filename::export_file_name ─→ file::write_new ─→ 쓰인 파일 하나
//! ```
//!
//! ## AI가 없어도 성립한다 (INV-8 · §17.1)
//!
//! **AI provider를 고르지 않았다는 이유로도, AI Note가 하나도 없다는 이유로도 거절하지
//! 않는다.** 이 모듈에는 provider도 설정도 들어오지 않으며 (`crate::ai::provider`를 쓰지
//! 않는다), 노트는 있으면 넣고 없으면 `None`인 선택 입력이다. 그 입력으로도 렌더러는 제목 ·
//! `Date:` · `Duration:` · `## Transcript`를 갖춘 유효한 문서를 낸다 ([`super::markdown`]).
//!
//! ## 읽기만 한다 (INV-3 · INV-6)
//!
//! ```text
//! recordings · transcripts · transcript_segments · ai_notes   전부 그대로 (읽기 질의뿐이다)
//! 원본 오디오 파일                                            그대로 (경로조차 읽지 않는다)
//! 이미 export된 파일                                          그대로 (덮어쓰지 않는다 · §4.3)
//! ```
//!
//! 이 모듈에는 저장소에 쓰는 코드가 **없다** — `store`에서 부르는 것이 전부 `load_*`·`list_*`
//! 이므로, 어떤 실패 경로도 recording · transcript · ai_note를 고치거나 지울 수 없다. export가
//! 실패했다는 사실을 레코드에 남기지도 않는다: `notion_status`는 Notion의 것이고, Markdown
//! export를 위한 상태 열은 §7에 없다 — 없는 자리를 여기서 만들지 않는다 (§20.6).
//!
//! **오디오는 복사하지도 읽지도 않는다** (INV-6). 이 모듈이 파일시스템에 손대는 자리는
//! [`super::file::write_new`] 하나이며, 그 함수가 받는 것은 Markdown 문자열뿐이다.

use std::path::Path;

use rusqlite::Connection;

use crate::ai::note::{decode_content, StructuredNote};
use crate::db::store;
use crate::domain::{Failure, FailureKind, RecordingId, TranscriptId};

use super::file::{self, WrittenFile};
use super::filename::export_file_name;
use super::markdown::{render, ExportDocument};

/// Recording 하나를 주어진 디렉터리에 Markdown 파일 하나로 쓴다.
///
/// 돌려주는 것은 **실제로 쓰인 파일**이다 — 같은 이름이 이미 있었으면 번호가 붙으므로
/// (ADR-0009 §4.3), 부르는 쪽이 이름을 다시 짐작하지 않게 한다.
///
/// 디렉터리는 여기서 만들지 않는다. 자리를 정하고 준비하는 일은
/// [`crate::platform::app_data_dir::AppDataDirectory::ensure_exports_dir`]의 몫이며 (INV-10),
/// 그 실패도 그쪽이 §13의 실패로 옮긴다.
pub fn export(
    connection: &Connection,
    directory: &Path,
    recording_id: &RecordingId,
) -> Result<WrittenFile, Failure> {
    // 내보낼 대상이 실재하는지부터 본다. 없는 Recording에 대해서는 파일을 만들지 않는다.
    let recording =
        store::load_recording(connection, recording_id)?.ok_or_else(|| unknown_recording(recording_id))?;

    // 기본 입력은 `current_transcript_id`가 가리키는 Transcript다 (§7.2). **다른 version을
    // 추측해서 고르지 않는다.**
    let Some(transcript_id) = recording.current_transcript_id.clone() else {
        return Err(nothing_to_export(recording_id));
    };
    let transcript = store::load_transcript(connection, &transcript_id)?
        .ok_or_else(|| dangling_transcript(&transcript_id))?;

    // **없는 것이 정상이다** (INV-8). 여기서 provider를 묻지 않고, 노트를 만들지도 않는다.
    let note = latest_note(connection, &transcript.id)?;

    let markdown = render(&ExportDocument {
        recording: &recording,
        transcript: &transcript,
        note: note.as_ref(),
    });
    let file_name = export_file_name(&recording.created_at, &recording.title);

    file::write_new(directory, &file_name, &markdown)
}

/// 그 Transcript에서 만들어진 노트 중 **마지막 것**. 하나도 없으면 `None`이다.
///
/// 저장소가 주는 순서는 `generated_at` · `id`이므로 마지막이 가장 최근에 만들어진 노트다
/// (`store::list_ai_notes_for_transcript`). **mode로 가려내지 않는다** — 사용자가 마지막으로
/// 만든 것이 무엇이든 그것이 지금 화면에서 보고 있을 노트이며, 이 자리가 "meeting이 study보다
/// 낫다" 같은 판단을 대신 내리지 않는다.
///
/// 봉투를 읽지 못하면 실패다. 그 실패는 AI 실패가 아니라 **저장소 실패이며** (ADR-0008 §7.5),
/// 읽을 수 없는 노트를 빈 값으로 채워 정상인 척하지 않는다 — 그러면 사용자는 노트가 빠진
/// 문서를 완전한 것으로 오해한다.
///
/// Notion으로 가는 실행 순서도 이 함수를 쓴다 (`crate::sync::run`). **"어느 노트를 넣는가"의
/// 규칙이 두 벌이 되면 같은 Recording의 로컬 파일과 Notion 페이지가 서로 다른 노트를 담을 수
/// 있다** — 산출물이 하나라는 것(ADR-0009 §14)은 렌더러만이 아니라 그 입력을 고르는 규칙까지
/// 하나라는 뜻이다.
pub(crate) fn latest_note(
    connection: &Connection,
    transcript_id: &TranscriptId,
) -> Result<Option<StructuredNote>, Failure> {
    let mut notes = store::list_ai_notes_for_transcript(connection, transcript_id)?;

    match notes.pop() {
        None => Ok(None),
        Some(note) => Ok(Some(decode_content(&note.content, note.note_type)?)),
    }
}

/// 그런 Recording이 없다. **아무 파일도 만들지 않았다.**
fn unknown_recording(id: &RecordingId) -> Failure {
    Failure::permanent(FailureKind::InvalidInput, "내보낼 녹음을 찾을 수 없다.")
        .with_detail(format!("recordingId={id}"))
}

/// 아직 current Transcript가 없다 (§7.2).
///
/// **AI Note가 없는 것과는 다른 상황이다.** 노트는 선택 입력이지만 (INV-8) Transcript는 문서의
/// 본문이며, 그것이 없으면 제목과 길이만 담긴 파일이 남는다. 그런 파일은 "export가 잘못됐다"처럼
/// 보이면서 사용자의 export 디렉터리에 실제로 쌓인다 — **빈 산출물을 만드는 대신 무엇이 필요한지
/// 말한다** (§13).
///
/// 전사를 먼저 돌리면 풀리므로 재시도 가능한 실패다. 여기서 전사를 **대신 시작하지 않는다** —
/// 사용자가 요청한 것은 export이고, 하지 않은 일을 대신 하는 경로를 만들지 않는다.
fn nothing_to_export(id: &RecordingId) -> Failure {
    Failure::retryable(
        FailureKind::InvalidInput,
        "아직 전사가 없어 내보낼 내용이 없다. 전사를 먼저 끝내야 한다.",
    )
    .with_detail(format!("recordingId={id}"))
}

/// current가 가리키는 Transcript 행이 없다.
///
/// 스키마의 복합 FK가 막는 상태이므로 정상적으로는 일어나지 않는다. 일어났다면 저장소가
/// 어긋난 것이다 — **추측해서 다른 Transcript를 고르지 않는다** ([`crate::ai::run`]과 같은 규칙).
fn dangling_transcript(id: &TranscriptId) -> Failure {
    Failure::permanent(
        FailureKind::Storage,
        "내보낼 전사를 저장소에서 읽지 못했다.",
    )
    .with_detail(format!("transcriptId={id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recording_that_is_not_there_is_told_apart_from_a_storage_problem() {
        // 사용자가 할 수 있는 일이 다르다 — 이쪽은 다른 녹음을 고르는 것이고, 저장소 실패는
        // 앱을 다시 시작하는 것이다 (§13).
        let failure = unknown_recording(&RecordingId::new("rec-1"));

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.retryable, "같은 id로 다시 해도 결과가 같다");
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (INV-3)");
        assert_eq!(failure.detail.as_deref(), Some("recordingId=rec-1"));
    }

    #[test]
    fn having_no_transcript_yet_says_what_to_do_and_leaves_everything_alone() {
        let failure = nothing_to_export(&RecordingId::new("rec-1"));

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.retryable, "전사가 끝나면 성공할 수 있다");
        assert!(failure.source_data_safe);
        assert!(!failure.message.trim().is_empty(), "화면에 띄울 문장이 있다");
    }

    #[test]
    fn a_transcript_row_that_cannot_be_read_is_a_storage_failure_not_an_export_bug() {
        let failure = dangling_transcript(&TranscriptId::new("tr-1"));

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(!failure.retryable);
        assert!(failure.source_data_safe);
        assert_eq!(failure.detail.as_deref(), Some("transcriptId=tr-1"));
    }
}
