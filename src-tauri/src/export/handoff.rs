//! 사람이 자기 AI 채팅으로 가져가는 산출물 셋의 **실행 순서**
//! (`docs/ADR-0010-manual-ai-handoff.md` §5 · §8 · Phase 5.5 Required Outcome A).
//!
//! [`super::ai_request`]가 값에서 문자열을 만드는 순수 모듈이고, 그것을 저장소·파일시스템과
//! 잇는 자리는 여기 하나다 — [`super::run`]이 Markdown export에 대해 하는 역할과 같은 자리이며,
//! 같은 규칙을 따른다.
//!
//! ```text
//! Recording ─→ current Transcript ─┬─→ AiRequest::manual_prompt      ─┬─→ 조각 하나 (문자열)
//!   §7.2의 입력 하나               ├─→ AiRequest::transcript_text    ─┤   + 전체 크기 · 자리
//!                                  └─→ AiRequest::ai_ready_document  ─┴─→ file::write_new ─→ 파일 하나
//!                                                                     ↑
//!                                                              super::portion (크기와 나눔)
//! ```
//!
//! ## 크기 때문에 조용히 실패하지 않는다 (`phase-prompt/05.6` 성공 기준 4 · R-5)
//!
//! 72분 녹음의 산출물은 99 KB였고, 사람은 그것을 채팅 창에 붙여 넣으려다 실패했다. 세 산출물
//! **전부** 여기서 [`super::portion`]을 지나므로, 이 모듈이 돌려주는 것은 언제나 셋을 함께
//! 들고 있다 — **얼마나 큰가**([`Measure::total`]) · **지금 것이 몇 번째인가**
//! ([`Measure::index`] · [`Measure::count`]) · **그 조각이 얼마나 큰가**([`Measure::size`]).
//!
//! **잘린 것을 온전한 것이라고 말하는 자리가 없다.** 조각 하나만 돌려줄 때에도 전체가 몇
//! 조각인지가 같은 값에 실려 있으므로, 부르는 쪽이 그것을 모른 채 "다 가져갔다"고 말할 수단이
//! 없다. 없는 조각을 달라는 요청은 빈 결과가 아니라 실패다 ([`no_such_portion`]).
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
//! **나뉜 문서를 내보내도 마찬가지다.** 조각마다 파일이 하나씩 생기지만 쓰는 자리는 여전히
//! [`super::file::write_new`] 하나이고, 그 함수는 이미 있는 파일을 덮어쓰지 않는다
//! (ADR-0009 §4.3). 조각의 자리는 파일 **이름**에도 적힌다
//! ([`super::filename::ai_request_portion_file_name`]) — 그러지 않으면 사용자가 파일 목록만
//! 보고 어느 파일이 몇 번째인지 알 수 없다.
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
use super::filename::ai_request_portion_file_name;
use super::markdown::{transcript_body, TranscriptBody, TranscriptShape};
use super::portion::{measure, split, TextSize};
use super::run::current_input;

/// 아무것도 고르지 않았을 때 가져가는 조각 — **첫 번째다.**
///
/// 조각 번호는 사람에게 보이는 값이므로 1부터 센다 ([`super::portion::Portion::index`]).
pub const FIRST_PORTION: usize = 1;

/// 산출물 하나가 얼마나 크고, 지금 가져가는 것이 그중 어디인가
/// (`phase-prompt/05.6` 성공 기준 4).
///
/// **세 산출물이 같은 값을 낸다.** 하나만 크기를 말하면 화면은 나머지 둘에 대해 사용자에게
/// 아무 말도 할 수 없고, 그때 크기 때문에 실패하는 일은 다시 조용해진다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measure {
    /// 산출물 **전체**의 크기. 조각 하나만 가져갈 때에도 이 값은 전체를 말한다.
    pub total: TextSize,
    /// 지금 가져가는 조각의 순번. **1부터 센다.**
    pub index: usize,
    /// 조각이 전부 몇 개인가. `1`이면 나뉘지 않았다.
    pub count: usize,
    /// 지금 가져가는 조각 하나의 크기.
    pub size: TextSize,
}

impl Measure {
    /// 나뉘지 않았는가 — 지금 가져가는 것이 전부인가.
    pub fn is_whole(&self) -> bool {
        self.count <= 1
    }

    /// 아직 가져가지 않은 조각이 몇 개 남았는가.
    pub fn remaining(&self) -> usize {
        self.count.saturating_sub(self.index)
    }
}

/// 사람이 지금 가져가는 텍스트 한 조각과, 그것이 무엇의 어디인가.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakenText {
    /// 이 조각의 텍스트. 조각이 하나뿐이면 산출물 전체다.
    pub text: String,
    pub measure: Measure,
}

/// 실제로 쓰인 파일 하나와, 그 파일이 산출물의 어디인가.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenPortion {
    pub file: WrittenFile,
    pub measure: Measure,
}

/// 고른 mode에 맞는 **Manual 프롬프트**를 만든다 — transcript를 포함한다 (ADR-0010 §6.2).
///
/// 돌려주는 것은 사람이 자기 AI 채팅에 그대로 붙여 넣는 텍스트 한 조각과 **그 조각이 전체의
/// 어디인가**다. **어디에도 보내지 않고 아무것도 저장하지 않는다** (MH-3 · MH-7).
///
/// `portion`이 `None`이면 첫 조각이다 — 예산 안의 프롬프트에서는 그것이 전체다.
pub fn ai_prompt(
    connection: &Connection,
    recording_id: &RecordingId,
    mode: NoteType,
    portion: Option<usize>,
) -> Result<TakenText, Failure> {
    let (recording, transcript) = request_input(connection, recording_id)?;

    take(
        &AiRequest::new(mode, &recording, &transcript).manual_prompt(),
        portion,
        recording_id,
    )
}

/// 사람이 그대로 붙여 넣을 수 있는 **Transcript 텍스트**를 만든다 (ADR-0010 §5.4).
///
/// mode를 받지 않는다 — 같은 전사에서 언제나 같은 문자열이 나오며, 무엇을 요청할지는 이
/// 산출물이 정하지 않는다. 크기와 나눔은 [`ai_prompt`]와 같은 규칙을 지난다.
pub fn transcript_text(
    connection: &Connection,
    recording_id: &RecordingId,
    portion: Option<usize>,
) -> Result<TakenText, Failure> {
    let (recording, transcript) = request_input(connection, recording_id)?;

    // mode는 이 산출물에 나타나지 않는다 (`ai_request`의 단위 테스트가 그것을 고정한다).
    take(
        &AiRequest::new(NoteType::Meeting, &recording, &transcript).transcript_text(),
        portion,
        recording_id,
    )
}

/// **AI-ready 문서의 조각 하나**를 주어진 디렉터리에 파일로 쓴다 (ADR-0010 §5.2 · §5.6).
///
/// 돌려주는 것은 **실제로 쓰인 파일**과 그것이 문서의 어디인가다 — 같은 이름이 이미 있었으면
/// 번호가 붙으므로 (ADR-0009 §4.3), 부르는 쪽이 이름을 다시 짐작하지 않게 한다.
///
/// 두 번째 export 시스템을 만들지 않는다 — 디렉터리를 정하는 자리
/// ([`crate::platform::app_data_dir::AppDataDirectory::ensure_exports_dir`])도, 덮어쓰지 않고
/// 쓰는 자리([`file::write_new`])도 Phase 5가 만든 그 자리 그대로다. 이 경로가 더하는 것은
/// 파일 이름의 표식이다 ([`ai_request_portion_file_name`]).
///
/// **한 번 부르면 파일 하나다.** 문서가 네 조각이면 네 번 불러 네 파일이 되며, 그중 어느 것도
/// 앞서 쓴 것을 덮어쓰지 않는다 — 나눔이 사용자의 문서를 지우는 경로가 되지 않게 한다.
pub fn export_ai_request(
    connection: &Connection,
    directory: &Path,
    recording_id: &RecordingId,
    mode: NoteType,
    portion: Option<usize>,
) -> Result<WrittenPortion, Failure> {
    let (recording, transcript) = request_input(connection, recording_id)?;

    let document = AiRequest::new(mode, &recording, &transcript).ai_ready_document();
    let taken = take(&document, portion, recording_id)?;
    let file_name = ai_request_portion_file_name(
        &recording.created_at,
        &recording.title,
        taken.measure.index,
        taken.measure.count,
    );

    Ok(WrittenPortion {
        file: file::write_new(directory, &file_name, &taken.text)?,
        measure: taken.measure,
    })
}

/// 산출물 하나에서 **가져갈 조각 하나**를 고른다 (`phase-prompt/05.6` 성공 기준 4).
///
/// 나누는 규칙은 여기 없다 — [`super::portion`] 한 자리에 있고 이 함수는 그것을 부른다.
/// 여기서 더하는 것은 **고르기**와, 고를 수 없는 요청에 대한 답이다.
///
/// 없는 조각을 달라는 요청은 **빈 결과가 아니라 실패다.** 조각 수는 산출물이 달라지면 함께
/// 달라지므로 (재전사 뒤에는 더 짧을 수 있다), 그 요청에 빈 텍스트로 답하면 사용자는 아무것도
/// 없는 것을 "다 가져갔다"고 읽게 된다.
fn take(
    text: &str,
    requested: Option<usize>,
    recording_id: &RecordingId,
) -> Result<TakenText, Failure> {
    let portions = split(text);
    // 빈 문자열은 조각이 하나도 없다. 그때 가져가는 것은 빈 텍스트 하나이며, 여기서 조각 수를
    // 0으로 말하면 "1번째 / 전체 0개"라는 읽을 수 없는 자리가 생긴다.
    let count = portions.len().max(1);
    let index = requested.unwrap_or(FIRST_PORTION);

    if index < FIRST_PORTION || index > count {
        return Err(no_such_portion(recording_id, index, count));
    }

    let piece = portions
        .get(index - FIRST_PORTION)
        .map(|portion| portion.text)
        .unwrap_or("");

    Ok(TakenText {
        text: piece.to_owned(),
        measure: Measure {
            total: measure(text),
            index,
            count,
            size: measure(piece),
        },
    })
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

    // 적을 것이 있는가는 모양과 무관하지만, 묻는 것은 **이 경로가 실제로 만드는 모양**이다 —
    // 모양마다 다른 판정이 생길 자리를 남기지 않는다.
    if matches!(
        transcript_body(&transcript, TranscriptShape::Compact),
        TranscriptBody::Empty
    ) {
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

/// 그런 조각이 없다 (`phase-prompt/05.6` 성공 기준 4).
///
/// **잘린 결과를 성공이라고 말하지 않기 위한 실패다.** 빈 텍스트를 돌려주면 사용자는 그것을
/// "가져갈 것이 더 없다"로 읽는다. 실제로 남은 조각이 몇 개인지는 detail에 실려 있으므로,
/// 이 실패를 받은 쪽은 다시 물어보지 않고도 무엇이 어긋났는지 알 수 있다.
///
/// **AI Provider와는 아무 상관이 없다** (MH-1 · MH-2). 같은 요청을 다시 보내도 결과가 같으므로
/// 재시도 가능한 실패가 아니다 — 사용자가 할 일은 몇 조각인지 다시 보고 고르는 것이다.
fn no_such_portion(id: &RecordingId, requested: usize, count: usize) -> Failure {
    Failure::permanent(
        FailureKind::InvalidInput,
        "그런 조각이 없다. 산출물이 몇 조각인지 다시 보고 골라야 한다.",
    )
    .with_detail(format!("recordingId={id} portion={requested}/{count}"))
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

    // ── 가져갈 조각 고르기 (`phase-prompt/05.6` 성공 기준 4) ─────────────────────

    /// 예산을 넉넉히 넘는 문자열 — 줄 하나가 40바이트다 (`portion`의 테스트와 같은 모양).
    fn over_the_budget() -> String {
        let mut text = String::new();
        for index in 0..(3 * super::super::portion::PORTION_MAX_BYTES / 40) {
            text.push_str(&format!("{index:0>39}\n"));
        }
        text
    }

    #[test]
    fn a_text_that_fits_comes_back_whole_and_says_so() {
        let text = "## Transcript\n00:00:03 안녕하세요.\n";

        let taken = take(text, None, &RecordingId::new("rec-1")).expect("가져갈 수 있다");

        assert_eq!(taken.text, text, "나뉘지 않은 조각은 원본 그대로다");
        assert_eq!(taken.measure.index, 1);
        assert_eq!(taken.measure.count, 1);
        assert!(taken.measure.is_whole());
        assert_eq!(taken.measure.remaining(), 0);
        // 전체의 크기와 조각의 크기가 같다 — 이것이 "다 가져갔다"의 값 수준 표현이다.
        assert_eq!(taken.measure.total, taken.measure.size);
    }

    #[test]
    fn a_text_over_the_budget_comes_in_order_and_rejoins_into_the_original() {
        let text = over_the_budget();
        let id = RecordingId::new("rec-1");

        let first = take(&text, None, &id).expect("첫 조각을 가져갈 수 있다");
        assert!(!first.measure.is_whole(), "나뉘어야 하는 크기다");
        assert_eq!(first.measure.index, 1);
        assert!(first.measure.remaining() > 0, "남은 것이 있다고 말해야 한다");
        // 조각 하나가 전체보다 작다는 사실이 값으로 있다 — 화면이 "다 가져갔다"고 말할 수 없다.
        assert!(first.measure.size.bytes < first.measure.total.bytes);
        assert_eq!(first.measure.total, measure(&text));

        let rejoined: String = (1..=first.measure.count)
            .map(|index| {
                let taken = take(&text, Some(index), &id).expect("조각을 가져갈 수 있다");
                assert_eq!(taken.measure.index, index);
                assert_eq!(taken.measure.count, first.measure.count);
                assert_eq!(taken.measure.total, first.measure.total);
                taken.text
            })
            .collect();

        assert_eq!(rejoined, text, "조각을 다 이어 붙이면 원본이다");
    }

    #[test]
    fn asking_for_a_portion_that_is_not_there_is_a_failure_and_not_an_empty_result() {
        // 빈 텍스트로 답하면 사용자는 그것을 "가져갈 것이 더 없다"로 읽는다 (성공 기준 4).
        let text = over_the_budget();
        let id = RecordingId::new("rec-1");
        let count = take(&text, None, &id).expect("첫 조각").measure.count;

        for requested in [0, count + 1, count + 100] {
            let failure = take(&text, Some(requested), &id).expect_err("없는 조각이다");

            assert_eq!(failure.kind, FailureKind::InvalidInput);
            assert!(!failure.retryable, "같은 요청을 다시 보내도 결과가 같다");
            assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (MH-7)");
            assert_eq!(
                failure.detail.as_deref(),
                Some(format!("recordingId=rec-1 portion={requested}/{count}").as_str())
            );
        }
    }

    #[test]
    fn an_empty_output_is_one_empty_portion_and_never_zero_portions() {
        // `split("")`은 조각이 하나도 없다. "1번째 / 전체 0개"라는 읽을 수 없는 자리를 만들지
        // 않는다 — 여기까지 오는 산출물은 없지만 (`request_input`이 먼저 거절한다), 그 판정이
        // 이 함수의 성질을 대신하게 두지 않는다.
        let taken = take("", None, &RecordingId::new("rec-1")).expect("가져갈 수 있다");

        assert_eq!(taken.text, "");
        assert_eq!(taken.measure.index, 1);
        assert_eq!(taken.measure.count, 1);
        assert!(taken.measure.is_whole());
    }
}
