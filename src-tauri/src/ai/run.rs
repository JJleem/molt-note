//! Recording 하나에 대해 AI 노트를 만드는 **실행 순서**
//! (PRODUCT-SPEC §7 · §7.2 · §7.3 · `docs/ADR-0008-note-ai-provider.md` §7 · §8 · §9).
//!
//! 앞의 모듈들이 각자 한 조각씩 맡았고, 그것들을 잇는 자리는 여기 하나다.
//! [`crate::transcription::run`]이 전사에 대해 하는 역할과 같은 자리이며, 같은 규칙을 따른다.
//!
//! ```text
//! Recording 레코드 ─→ current Transcript ─→ prompt::prepare ─→ provider ─→ encode_content ─→ insert_ai_note
//!                        §7.2의 기본 입력      크기 판정 §8       계약뿐         §7.5 봉투        §7.3 provenance
//!        │
//!        └─ ai_status:  pending ─→ running ─→ done
//!                                        └─→ failed
//! ```
//!
//! ## 이 모듈은 어떤 벤더도 알지 않는다 (INV-9)
//!
//! [`NoteAiProvider`]를 **받아서** 쓴다. 그 값을 만드는 것 — 어떤 provider인지 고르고, 어디에
//! 연결할지 정하고, 고르지 않은 상태(`None`)를 [`FailureKind::AiProviderNotConfigured`]로
//! 옮기는 것 — 은 전부 부르는 쪽의 일이다 (ADR-0008 §4.3). 여기에는 엔드포인트도, 주소도,
//! 모델 이름도, 벤더 식별자도 없다.
//!
//! ## 재생성은 대체가 아니라 추가다 (ADR-0008 §9 · §7.1)
//!
//! 성공할 때마다 **`ai_notes` 행이 하나 늘어난다.** 이 모듈에 기존 노트를 고치거나 지우는
//! 경로는 없고, 만들 수도 없다 — 저장소가 내놓는 쓰기가 [`store::insert_ai_note`] 하나이기
//! 때문이다 (`crate::db::store`). 그래서 이전 노트의 `model` · `promptVersion` ·
//! `generatedAt`이 지워지지 않고, "프롬프트를 바꾼 뒤 전보다 나아졌는가"를 물을 수 있다.
//!
//! ## 실패는 아무것도 잃지 않는다 (INV-2 · INV-3 · §13)
//!
//! ```text
//! Transcript A · B          그대로 (읽기만 한다 — 이 모듈에 transcripts를 쓰는 코드가 없다)
//! transcript_segments       그대로
//! 원본 오디오 파일          그대로 (이 모듈은 파일시스템에 접근하지 않는다 · INV-6)
//! 이미 저장된 ai_notes 행   그대로 (UPDATE·DELETE 경로가 없다)
//! Recording 레코드          ai_status와 updated_at 말고는 그대로
//! ```
//!
//! 상태를 옮기는 자리는 [`store::update_recording_statuses`] 하나이며, 그 함수는 세 후처리
//! 상태와 `updated_at`만 만진다. 다른 후처리 상태(`transcription_status` · `notion_status`)는
//! **읽은 그대로 다시 쓴다** — AI가 남의 파이프라인 상태를 옮기지 않는다. `updated_at`이 함께
//! 갱신되는 것은 전사 경로의 규약을 그대로 따른 것이다.
//!
//! ## 입력이 아직 없는 것은 실패가 아니다 (§7.2 · INV-8)
//!
//! current Transcript가 없는 Recording에 대해서는 [`Outcome::NoTranscriptYet`]을 돌려준다.
//! 오류가 아니라 **아직 재료가 없다**는 상태이며, 그래서 이 경로는 `ai_status`를 `failed`로
//! 옮기지 않고 **아무것도 저장하지 않는다** — 사용자가 한 적 없는 실패를 남기지 않기 위해서다.
//!
//! ## 여기서 하지 않는 것
//!
//! 스레드를 만들지 않고, Tauri command를 열지 않으며, 어떤 provider를 쓸지 · 어떤 context
//! 예산을 쓸지 설정에서 읽지 **않는다** — 그 값들을 읽어 넘기는 것은 부르는 쪽의 일이다
//! (전사가 [`crate::transcription::run::ModelChoice`]를 받아서 넘기기만 하는 것과 같다).

use rusqlite::Connection;

use crate::db::store;
use crate::domain::{
    AiNote, AiNoteId, Failure, FailureKind, NoteType, ProcessingStatus, Recording, RecordingId,
    TranscriptId,
};

use super::note::encode_content;
use super::prompt::{self, ContextBudget, ContextOverflow};
use super::provider::{response_unusable, NoteAiProvider, NoteRequest, TranscriptText};

/// 노트 생성 한 번의 결과.
///
/// **실패가 아닌 두 가지 끝을 구분한다.** 노트가 만들어진 것과, 만들 재료가 아직 없는 것은
/// 사용자가 보는 상태가 다르고 할 수 있는 일도 다르다 (전사를 먼저 돌린다).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// 노트 하나가 **새로** 저장됐다. 이전 노트는 그대로 남는다 (ADR-0008 §9.2).
    Generated(AiNote),
    /// current Transcript가 없어 입력이 없다 (§7.2). **오류가 아니며, 아무것도 저장하지 않았다.**
    NoTranscriptYet,
}

impl Outcome {
    /// 이번에 저장된 노트. 입력이 없어 아무것도 만들지 않았으면 `None`이다.
    pub fn generated(&self) -> Option<&AiNote> {
        match self {
            Self::Generated(note) => Some(note),
            Self::NoTranscriptYet => None,
        }
    }
}

/// 입력이 이 요청의 context 예산을 넘었다 (ADR-0008 §8.2-3 · §13.1의 여섯 번째 실패).
///
/// **provider를 부르기 전에 만들어지는 실패다** — 그래서 만드는 자리가 adapter가 아니라 여기다
/// (`crate::ai::provider`의 다섯과 구분된다). 다시 시도해도 입력도 예산도 그대로면 결과가
/// 같으므로 재시도 대상이 아니다: 사용자가 context 크기를 키우거나 더 짧은 녹음을 골라야 한다.
///
/// `detail`에 남는 것은 추정 토큰 수뿐이다 — provider 설정값(주소 · 모델 이름)은 실패 문장에도
/// 원인에도 남지 않는다 (ADR-0008 §11.3).
pub fn input_too_large(overflow: &ContextOverflow) -> Failure {
    Failure::permanent(
        FailureKind::AiInputTooLarge,
        "전사가 한 번에 보낼 수 있는 크기를 넘어 노트를 만들지 않았다",
    )
    .with_detail(overflow)
}

/// Recording 하나에 대해 그 mode의 AI 노트를 만들고 영속화한다.
///
/// 기본 입력은 **`current_transcript_id`가 가리키는 Transcript**다 (§7.2). 그 Transcript의
/// 식별자가 그대로 `ai_notes.transcript_id`에 남으므로, 서로 다른 Transcript version에서 나온
/// 노트는 그 열로 구분된다 (§7.3).
///
/// 상태는 실제로 세 번(또는 두 번 + 실패) 저장된다 — `pending` · `running` 다음에 `done` 또는
/// `failed`다. 중간 상태를 건너뛰지 않는 이유는 전사와 같다: 그것이 화면이 읽는 값이며, 생성이
/// 도는 동안 목록과 상세가 `running`을 볼 수 있어야 한다.
///
/// 실패해도 Transcript · segment · 오디오 파일 · 이미 저장된 노트는 그대로다 (INV-2 · INV-3).
/// 바뀌는 것은 `ai_status`와 `updated_at`뿐이며, 그 둘이 바뀌는 것이 §13이 요구하는 "실패가
/// 사용자에게 보인다"의 저장 형태다.
pub fn generate(
    connection: &Connection,
    recording_id: &RecordingId,
    mode: NoteType,
    provider: &dyn NoteAiProvider,
    budget: ContextBudget,
) -> Result<Outcome, Failure> {
    // 상태를 쓸 대상이 실재하는지부터 본다. 없는 Recording에 대해서는 아무것도 쓰지 않는다.
    let recording = store::load_recording(connection, recording_id)?
        .ok_or_else(|| unknown_recording(recording_id))?;

    // 입력이 없는 것은 실패가 아니다 (§7.2 · INV-8). **상태를 옮기기 전에** 판정하므로 이
    // 경로는 `ai_status`도 `updated_at`도 건드리지 않는다 — 시도한 적 없는 일이 실패로 남지
    // 않는다.
    let Some(transcript_id) = recording.current_transcript_id.clone() else {
        return Ok(Outcome::NoTranscriptYet);
    };

    // 접수(`pending`)와 실행 시작(`running`)을 둘 다 남긴다. 이 두 번의 쓰기가 실패하면 그대로
    // 나간다 — 상태를 남기지 못한 채 생성을 시작하면 화면이 진행 중인 일을 볼 수 없다.
    mark(connection, &recording, ProcessingStatus::Pending)?;
    mark(connection, &recording, ProcessingStatus::Running)?;

    match attempt(connection, &recording, &transcript_id, mode, provider, budget) {
        Ok(note) => Ok(Outcome::Generated(note)),
        Err(failure) => Err(record_failure(connection, &recording, failure)),
    }
}

/// 생성 한 번. 여기서 나오는 모든 실패는 호출자가 `failed`로 기록한다.
fn attempt(
    connection: &Connection,
    recording: &Recording,
    transcript_id: &TranscriptId,
    mode: NoteType,
    provider: &dyn NoteAiProvider,
    budget: ContextBudget,
) -> Result<AiNote, Failure> {
    // **읽기뿐이다.** Transcript는 immutable이고 (§7.1 · INV-2), 저장소에는 애초에 그것을
    // 고치는 API가 없다.
    let transcript = store::load_transcript(connection, transcript_id)?
        .ok_or_else(|| dangling_transcript(transcript_id))?;

    // 보내기 전에 크기를 판정한다 (ADR-0008 §8.2-3). 넘치면 **요청을 보내지 않고** 자르지도
    // 않는다. 여기서 만들어진 프롬프트 문자열 자체는 쓰이지 않는다 — 실제 요청 본문을 만드는
    // 것은 adapter이며 (INV-9), 이 자리가 필요한 것은 크기 판정과 `promptVersion` 둘이다.
    // 둘을 같은 함수에서 얻으므로 저장되는 버전은 **크기를 잰 그 프롬프트의 버전**이다.
    let prepared = prompt::prepare(mode, &transcript.raw_text, budget)
        .map_err(|overflow| input_too_large(&overflow))?;

    // 계약에 실리는 것은 전사 **텍스트**뿐이다 — 오디오를 가리킬 자리가 타입에 없다 (INV-6).
    let generation = provider.generate_note(&NoteRequest {
        mode,
        transcript: TranscriptText::new(&transcript.raw_text),
        context_budget: budget,
    })?;

    if generation.note.mode() != mode {
        // 계약을 지키는 provider는 여기 오지 않는다 (`ai::testing::assert_generation_contract`).
        // 그래도 확인하는 것은 저장된 노트가 immutable하게 남기 때문이다 — `note_type` 열과
        // 봉투의 `mode`가 어긋난 행은 읽을 때마다 `Storage` 실패가 되고 (ADR-0008 §7.5), 고칠
        // 경로가 없다. 저장 직전이 그것을 막을 수 있는 마지막 자리다.
        return Err(response_unusable("AI가 요청과 다른 종류의 노트를 만들었다").with_detail(
            format!("requested={mode} · generated={}", generation.note.mode()),
        ));
    }

    let note = AiNote {
        id: AiNoteId::new(store::new_id(connection)?),
        // provenance는 §7.3이 요구하는 것을 전부 채운다. 어느 값도 추정이 아니다 —
        // `transcript_id`는 실제로 입력에 쓴 Transcript, `model`은 provider가 답한 모델,
        // `prompt_version`은 크기를 잰 그 프롬프트의 버전이다.
        recording_id: recording.id.clone(),
        transcript_id: transcript.id.clone(),
        note_type: mode,
        content: encode_content(&generation.note),
        provider: provider.descriptor().id,
        model: generation.model,
        prompt_version: prepared.prompt_version.to_owned(),
        generated_at: store::now(connection)?,
    };

    // 순서가 규칙이다: 노트를 남긴 다음에 `done`. 화면이 `done`을 보는 시점에는 그 노트가 이미
    // 저장돼 있다.
    store::insert_ai_note(connection, &note)?;
    mark(connection, recording, ProcessingStatus::Done)?;

    Ok(note)
}

/// AI 상태 하나를 저장한다. 다른 후처리 상태는 읽은 값을 그대로 다시 쓴다.
fn mark(
    connection: &Connection,
    recording: &Recording,
    status: ProcessingStatus,
) -> Result<(), Failure> {
    let now = store::now(connection)?;
    store::update_recording_statuses(
        connection,
        &recording.id,
        recording.transcription_status,
        status,
        recording.notion_status,
        &now,
    )?;
    Ok(())
}

/// 실패를 `failed`로 남기고 그 실패를 그대로 돌려준다.
///
/// **원래 원인을 다른 것으로 바꾸지 않는다.** 상태를 남기는 데까지 실패하면 그 사실을 detail에
/// 덧붙일 뿐이다 — 사용자가 읽을 문장은 여전히 노트 생성이 왜 실패했는지다 (§13).
fn record_failure(connection: &Connection, recording: &Recording, failure: Failure) -> Failure {
    match mark(connection, recording, ProcessingStatus::Failed) {
        Ok(()) => failure,
        Err(storage) => {
            let detail = match failure.detail.as_deref() {
                Some(existing) => format!("{existing} · 실패 상태도 저장하지 못했다: {storage}"),
                None => format!("실패 상태도 저장하지 못했다: {storage}"),
            };
            failure.with_detail(detail)
        }
    }
}

/// 그런 Recording이 없다. 상태를 쓸 대상 자체가 없으므로 아무것도 저장하지 않았다.
fn unknown_recording(id: &RecordingId) -> Failure {
    Failure::permanent(FailureKind::InvalidInput, "노트를 만들 녹음을 찾을 수 없다")
        .with_detail(format!("recordingId={id}"))
}

/// current가 가리키는 Transcript 행이 없다.
///
/// 스키마의 복합 FK가 막는 상태이므로 정상적으로는 일어나지 않는다. 일어났다면 저장소가
/// 어긋난 것이지 AI가 실패한 것이 아니다 — 그래서 AI 실패로 옮기지 않는다. **추측해서 다른
/// Transcript를 고르지 않는다.**
fn dangling_transcript(id: &TranscriptId) -> Failure {
    Failure::permanent(
        FailureKind::Storage,
        "노트의 입력이 될 전사를 저장소에서 읽지 못했다",
    )
    .with_detail(format!("transcriptId={id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_input_that_does_not_fit_is_a_failure_of_its_own_kind() {
        // 이 상황에서 사용자가 할 수 있는 일은 다른 다섯 AI 실패와 다르다 (ADR-0008 §13.1).
        let overflow = ContextOverflow {
            estimated_input_tokens: 40_000,
            available_input_tokens: 13_824,
            context_tokens: 16_384,
        };

        let failure = input_too_large(&overflow);

        assert_eq!(failure.kind, FailureKind::AiInputTooLarge);
        assert!(!failure.retryable, "입력도 예산도 그대로면 결과도 그대로다");
        assert!(
            failure.source_data_safe,
            "요청을 보내지도 않았다 — 원본은 그대로다 (INV-3)"
        );
        assert!(!failure.message.trim().is_empty(), "화면에 띄울 문장이 있다");
        assert_eq!(
            failure.detail.as_deref(),
            Some(overflow.to_string().as_str()),
            "얼마나 넘쳤는지가 남는다"
        );
    }

    #[test]
    fn the_overflow_failure_is_not_one_the_provider_can_make() {
        // 만드는 자리가 adapter가 아니라는 것이 목록으로도 남는다 (ADR-0008 §13.1).
        assert!(
            !crate::ai::provider::AI_PROVIDER_FAILURE_KINDS.contains(&FailureKind::AiInputTooLarge)
        );
    }

    #[test]
    fn a_missing_recording_says_which_one_without_touching_anything() {
        let failure = unknown_recording(&RecordingId::new("rec-없음"));

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.retryable);
        assert_eq!(failure.detail.as_deref(), Some("recordingId=rec-없음"));
    }

    #[test]
    fn a_dangling_current_transcript_is_a_storage_problem_not_an_ai_one() {
        let failure = dangling_transcript(&TranscriptId::new("tr-없음"));

        assert_eq!(failure.kind, FailureKind::Storage);
        assert_eq!(failure.detail.as_deref(), Some("transcriptId=tr-없음"));
    }

    #[test]
    fn an_outcome_without_input_carries_no_note() {
        assert!(Outcome::NoTranscriptYet.generated().is_none());
    }
}
