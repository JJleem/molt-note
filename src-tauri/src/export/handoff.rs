//! 사람이 자기 AI 채팅으로 가져가는 산출물 셋의 **실행 순서**
//! (`docs/ADR-0010-manual-ai-handoff.md` §5 · §8 · Phase 5.5 Required Outcome A).
//!
//! [`super::ai_request`]가 값에서 문자열을 만드는 순수 모듈이고, 그것을 저장소·파일시스템과
//! 잇는 자리는 여기 하나다 — [`super::run`]이 Markdown export에 대해 하는 역할과 같은 자리이며,
//! 같은 규칙을 따른다.
//!
//! ```text
//! Recording ─→ current Transcript ─┬─→ AiRequest::manual_prompt      ─→ 문자열
//!   §7.2의 입력 하나               ├─→ AiRequest::transcript_text    ─→ 문자열
//!                                  └─→ AiRequest::ai_ready_document  ─→ file::write_new ─→ 파일 하나
//! ```
//!
//! ## current가 가리키는 Transcript만 읽는다 (§7.2 · MH-5)
//!
//! 고르는 규칙은 [`super::run::current_input`] **한 자리**에 있고 이 모듈은 그것을 부른다.
//! 실패했거나 대체된 옛 version을 고르는 경로가 여기에도, 이 모듈을 부르는 command 경계에도
//! 없다 — 경계가 `transcriptId`를 받지 않으므로 **wire에 고를 수단 자체가 없다**
//! (ADR-0010 §8.2).
//!
//! ## provider도 설정도 들어오지 않는다 (MH-1 · MH-2 · INV-8)
//!
//! 이 모듈은 `crate::ai::provider`도, AI 설정도, 어떤 벤더 이름도 알지 않는다. 그래서
//! **provider를 하나도 고르지 않았다는 이유로 거절할 수단 자체가 없다** — 읽는 것은
//! `crate::ai::prompt`의 상수뿐이고 (`super::ai_request`), 그것은 어디에도 연결하지 않는다.
//!
//! ## 저장소에 쓰지 않는다 (INV-3 · MH-7)
//!
//! ```text
//! recordings · transcripts · transcript_segments · ai_notes   전부 그대로 (읽기 질의뿐이다)
//! 원본 오디오 파일                                            그대로 (경로조차 읽지 않는다)
//! 이미 export된 파일                                          그대로 (덮어쓰지 않는다 · §4.3)
//! ```
//!
//! 이 모듈이 파일시스템에 손대는 자리는 [`super::file::write_new`] 하나이며, 그 함수가 받는
//! 것은 Markdown 문자열뿐이다. 복사 경로 둘은 파일에도 닿지 않는다 — 문자열을 돌려줄 뿐이고,
//! clipboard에 쓰는 일은 이 경계 밖(프론트엔드의 platform 모듈)에서 일어난다 (ADR-0010 §7).
//!
//! ## 네트워크가 없다 (MH-3)
//!
//! 나가는 행위의 주체는 사람이다. 이 모듈에는 HTTP 클라이언트도 주소도 없다 — 여기까지가
//! 앱이 하는 일이고, 그다음은 사용자가 자기 채팅에 붙여 넣거나 파일을 첨부하는 것이다.

use std::path::Path;

use rusqlite::Connection;

use crate::domain::{Failure, FailureKind, NoteType, Recording, RecordingId, Transcript};

use super::ai_request::AiRequest;
use super::file::{self, WrittenFile};
use super::filename::ai_request_file_name;
use super::markdown::{transcript_body, TranscriptBody};
use super::run::current_input;

/// 고른 mode에 맞는 **Manual 프롬프트**를 만든다 — transcript를 포함한다 (ADR-0010 §6.2).
///
/// 돌려주는 것은 사람이 자기 AI 채팅에 그대로 붙여 넣는 문자열 하나다. **어디에도 보내지
/// 않고 아무것도 저장하지 않는다** (MH-3 · MH-7).
pub fn ai_prompt(
    connection: &Connection,
    recording_id: &RecordingId,
    mode: NoteType,
) -> Result<String, Failure> {
    let (recording, transcript) = request_input(connection, recording_id)?;

    Ok(AiRequest::new(mode, &recording, &transcript).manual_prompt())
}

/// 사람이 그대로 붙여 넣을 수 있는 **Transcript 텍스트**를 만든다 (ADR-0010 §5.4).
///
/// mode를 받지 않는다 — 같은 전사에서 언제나 같은 문자열이 나오며, 무엇을 요청할지는 이
/// 산출물이 정하지 않는다.
pub fn transcript_text(
    connection: &Connection,
    recording_id: &RecordingId,
) -> Result<String, Failure> {
    let (recording, transcript) = request_input(connection, recording_id)?;

    // mode는 이 산출물에 나타나지 않는다 (`ai_request`의 단위 테스트가 그것을 고정한다).
    Ok(AiRequest::new(NoteType::Meeting, &recording, &transcript).transcript_text())
}

/// **AI-ready 문서 하나**를 주어진 디렉터리에 파일로 쓴다 (ADR-0010 §5.2 · §5.6).
///
/// 돌려주는 것은 **실제로 쓰인 파일**이다 — 같은 이름이 이미 있었으면 번호가 붙으므로
/// (ADR-0009 §4.3), 부르는 쪽이 이름을 다시 짐작하지 않게 한다.
///
/// 두 번째 export 시스템을 만들지 않는다 — 디렉터리를 정하는 자리
/// ([`crate::platform::app_data_dir::AppDataDirectory::ensure_exports_dir`])도, 덮어쓰지 않고
/// 쓰는 자리([`file::write_new`])도 Phase 5가 만든 그 자리 그대로다. 이 경로가 더하는 것은
/// 파일 이름의 표식 하나뿐이다 ([`ai_request_file_name`]).
pub fn export_ai_request(
    connection: &Connection,
    directory: &Path,
    recording_id: &RecordingId,
    mode: NoteType,
) -> Result<WrittenFile, Failure> {
    let (recording, transcript) = request_input(connection, recording_id)?;

    let document = AiRequest::new(mode, &recording, &transcript).ai_ready_document();
    let file_name = ai_request_file_name(&recording.created_at, &recording.title);

    file::write_new(directory, &file_name, &document)
}

/// 세 산출물이 공유하는 입력 — Recording 하나와 **current가 가리키는 Transcript** (§7.2).
///
/// 적을 것이 하나도 없는 전사는 여기서 거절된다 (ADR-0010 §5.5). 순수 렌더러는 받은 값을
/// 그대로 렌더하지만, **빈 요청을 만드는 것은 사용자가 원한 일이 아니다** — 지시만 있고 본문이
/// 없는 프롬프트를 채팅에 붙여 넣게 두거나, 내용 없는 파일을 export 디렉터리에 쌓아 두는 대신
/// 무엇이 필요한지 말한다 (§13).
fn request_input(
    connection: &Connection,
    recording_id: &RecordingId,
) -> Result<(Recording, Transcript), Failure> {
    let (recording, transcript) = current_input(
        connection,
        recording_id,
        unknown_recording,
        nothing_to_hand_off,
    )?;

    if matches!(transcript_body(&transcript), TranscriptBody::Empty) {
        return Err(nothing_to_hand_off(recording_id));
    }

    Ok((recording, transcript))
}

/// 그런 Recording이 없다. **아무것도 만들지 않았고 아무것도 건드리지 않았다.**
fn unknown_recording(id: &RecordingId) -> Failure {
    Failure::permanent(
        FailureKind::InvalidInput,
        "AI에게 넘길 녹음을 찾을 수 없다.",
    )
    .with_detail(format!("recordingId={id}"))
}

/// 아직 current Transcript가 없거나, 있어도 적을 내용이 없다 (§7.2 · ADR-0010 §5.5).
///
/// **AI Provider와는 아무 상관이 없는 실패다** (MH-1 · MH-2) — 이 경로는 provider를 보지
/// 않으며, 여기서 막는 것은 "AI에게 줄 본문이 아직 없다"는 사실 하나다.
///
/// 전사를 먼저 돌리면 풀리므로 재시도 가능한 실패다. 여기서 전사를 **대신 시작하지 않는다** —
/// 사용자가 요청한 것은 복사나 export이고, 하지 않은 일을 대신 하는 경로를 만들지 않는다
/// ([`super::run`]과 같은 규칙).
fn nothing_to_hand_off(id: &RecordingId) -> Failure {
    Failure::retryable(
        FailureKind::InvalidInput,
        "아직 전사 내용이 없어 AI에게 줄 것이 없다. 전사를 먼저 끝내야 한다.",
    )
    .with_detail(format!("recordingId={id}"))
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
    fn having_nothing_to_hand_off_says_what_to_do_and_leaves_everything_alone() {
        let failure = nothing_to_hand_off(&RecordingId::new("rec-1"));

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.retryable, "전사가 끝나면 성공할 수 있다");
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (MH-7)");
        assert!(!failure.message.trim().is_empty(), "화면에 띄울 문장이 있다");
        // provider를 이유로 거절하는 문장이 아니다 (MH-1 · MH-2).
        assert!(!failure.message.contains("provider"));
        assert!(!failure.message.contains("AI Provider"));
    }
}
