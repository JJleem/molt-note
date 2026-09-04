//! AI 노트 provider **계약** (PRODUCT-SPEC §9.2 · `docs/ADR-0008-note-ai-provider.md` §4 · §13).
//!
//! 이 모듈에는 **벤더가 없다.** 엔드포인트도, 파라미터 이름도, 상태 코드 해석도, 특정
//! 제공자의 이름도 없다. 있는 것은 "무엇을 할 수 있어야 하는가"뿐이며, 그것을 실제로
//! 이행하는 구현은 이 모듈 밖에 있다 (INV-9).
//!
//! ```text
//!                     ┌──────────────────────────────┐
//! mode + transcript ─→ │  NoteAiProvider (trait)      │ ─→ StructuredNote + 실제 쓰인 모델
//! 텍스트               └──────────────────────────────┘        └→ §13의 공통 실패로만 거절한다
//!                        │                        │
//!                        │                        └ testing::FakeNoteAiProvider
//!                        └ 실제 adapter (Phase 4 후반)  결정론적 test double
//! ```
//!
//! **trait이 있는 이유는 플랫폼이 아니라 벤더 교체와 테스트다** (ADR-0008 §1 · §4.3).
//! 벤더는 바뀌고, 바뀔 때 흔들리는 것이 adapter 하나여야 한다. 그리고 구현이 하나뿐인
//! 추상화는 검증된 추상화가 아니므로, 이 계약은 실제 adapter와 결정론적 double **둘이**
//! 통과한다 — 그 둘이 같은 묶음을 통과하는지 확인하는 것이
//! [`super::testing::assert_note_ai_provider_contract`]다.
//!
//! ## 오디오는 이 계약에 실릴 수 없다 (§12 · INV-6)
//!
//! [`NoteRequest`]에는 오디오 **경로도 바이트도 파일 핸들도 담을 자리가 없다.** transcript는
//! [`TranscriptText`]로만 들어오고 그 안에 있는 것은 텍스트 하나다. adapter는 오디오를 보내고
//! 싶어도 보낼 것을 받지 못한다 — INV-6을 "그런 코드를 안 짰다"가 아니라 **타입**으로 말한다.
//! `&Transcript`를 그대로 넘기지 않는 이유도 같다 (ADR-0008 §4.2 표 6행): 계약이 domain
//! 레코드에 묶이면 그 레코드가 자라는 순간 경계가 함께 자란다.
//!
//! ## 다섯 가지 실패는 서로 구분된다 (§13 · ADR-0008 §13.1)
//!
//! | 실패 | [`FailureKind`] | 누가 만드는가 | 재시도 |
//! | --- | --- | --- | --- |
//! | provider 미설정 | `AiProviderNotConfigured` | 서비스 계층 ([`not_configured`]) | ✗ 설정에서 골라야 한다 |
//! | 연결 불가 | `AiProviderUnreachable` | adapter ([`unreachable`]) | ✓ 서버를 켜고 다시 누르면 된다 |
//! | 모델 없음 | `AiModelUnavailable` | adapter ([`model_unavailable`]) | ✗ 모델을 받아야 한다 |
//! | 요청 실패 | `AiRequestFailed` | adapter ([`request_failed_temporarily`] · [`request_rejected`]) | 원인에 따라 다르다 |
//! | 응답 schema 불일치 | `AiResponseUnusable` | adapter의 파싱·검증 ([`rejected_response`]) | ✓ 생성은 결정론적이지 않다 |
//!
//! **다섯 다 `source_data_safe`를 내리지 않는다.** 이 경계는 오디오도 `transcripts`도
//! 건드리지 않는다 — 읽는 것은 메모리 위의 텍스트뿐이고, 실패했을 때 바뀌는 것은 Recording의
//! AI 상태 하나다 (INV-1 · INV-2 · INV-3).
//!
//! **"provider 미설정"은 [`Availability`]의 상태가 아니다** (ADR-0008 §4.3). 그것은 provider
//! 객체가 아예 없는 상태(`Option<Arc<dyn NoteAiProvider>>` = `None`)로 표현한다 — 없는 것에게
//! 물어보지 않는 것이 INV-8을 타입으로 적는 방법이다.

use crate::domain::{Failure, FailureKind, NoteType};

use super::note::{ResponseRejection, StructuredNote};
use super::prompt::ContextBudget;

/// provider가 만드는 §13의 실패 다섯 (ADR-0008 §13.1).
///
/// 계약 준수 검사가 "이 경계에서 나온 실패인가"를 판정하는 근거다. §13.4의 `rate limit` ·
/// `인증 실패`는 여기 없다 — 그 실패를 내는 provider가 아직 없고, 만들지 않은 실패의 자리를
/// 미리 만들어 두지 않는다 (§20.6).
pub const AI_PROVIDER_FAILURE_KINDS: [FailureKind; 5] = [
    FailureKind::AiProviderNotConfigured,
    FailureKind::AiProviderUnreachable,
    FailureKind::AiModelUnavailable,
    FailureKind::AiRequestFailed,
    FailureKind::AiResponseUnusable,
];

/// 전송되는 데이터가 기기 밖으로 나가는가 (§12 · INV-5).
///
/// **화면은 이 값을 읽어 사용자에게 알린다.** 어디로 나가는지가 아니라 "나가는가"가 사용자가
/// 알아야 하는 것이며, 그 답을 provider가 스스로 말한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locality {
    /// 데이터가 사용자의 기기 안에 머문다.
    Local,
    /// 데이터가 기기 밖으로 나간다.
    External,
}

impl Locality {
    /// 직렬화·표시에 쓰는 안정적인 문자열 표현.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::External => "external",
        }
    }

    /// 데이터가 기기 안에 머무는가.
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }
}

impl std::fmt::Display for Locality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// provider가 자기 자신에 대해 말하는 것.
///
/// `id`는 `ai_notes.provider`에 그대로 남는 **벤더 중립 자유 식별자**다 (INV-9) — domain은 이
/// 값을 알려진 목록과 대조하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDescriptor {
    /// provenance에 그대로 저장되는 식별자.
    pub id: String,
    /// 사람이 읽는 이름.
    pub name: String,
    /// 이 provider를 쓰면 데이터가 기기 밖으로 나가는가 (§12 · INV-5).
    pub locality: Locality,
}

/// provider가 지금 쓸 수 있는 상태인가 (§9.2의 `isAvailable` 자리 · INV-8).
///
/// **boolean이 아닌 이유**는 §13이 서로 다른 제품 상태로 구분하라고 요구했기 때문이다
/// (ADR-0008 §4.2 표 2행). 응답하지 않는 것과 응답했지만 모델이 없는 것은 사용자가 할 수 있는
/// 일이 다르다 — 서버를 켜는 것과 모델을 받는 것. 화면이 그 둘을 구분해 안내하려면 계약이
/// 먼저 구분해야 한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// 응답했고 쓸 수 있는 모델이 하나 이상 있다.
    Ready {
        /// 지금 고를 수 있는 모델 식별자들.
        models: Vec<String>,
    },
    /// 응답했지만 쓸 수 있는 모델이 하나도 없다 (§13 `모델 없음`).
    NoModels,
    /// 지금 쓸 수 없다. **이미 §13의 공통 실패로 번역된 값이다** — 벤더 오류가 이 계약을 타고
    /// 밖으로 나가지 않는다 (INV-9).
    Unavailable(Failure),
}

impl Availability {
    /// 지금 고를 수 있는 모델들. 쓸 수 없는 상태에서는 빈 목록이다.
    pub fn models(&self) -> &[String] {
        match self {
            Self::Ready { models } => models,
            Self::NoModels | Self::Unavailable(_) => &[],
        }
    }

    /// 지금 생성을 시도해 볼 수 있는가.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }
}

/// Transcript에서 **텍스트만** 뽑은 값 (INV-6).
///
/// 경로도, id도, 오디오도 담지 않는다. 이 타입에 담을 수 있는 것은 문자열 하나뿐이며 그것이
/// 이 계약이 오디오를 옮길 수 없다는 것의 근거다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscriptText<'a> {
    /// 사람이 읽을 수 있는 전사 텍스트.
    pub text: &'a str,
}

impl<'a> TranscriptText<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }
}

/// provider에게 넘기는 생성 요청 하나.
///
/// **오디오를 가리킬 수 있는 필드가 없다** (INV-6). 필드는 셋뿐이며, 어느 것도 파일도 바이트도
/// 핸들도 담지 못한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteRequest<'a> {
    /// 어떤 노트를 만들 것인가 — domain의 `meeting` · `study` · `summary` 그대로다.
    pub mode: NoteType,
    /// 무엇을 재료로 쓸 것인가 — **텍스트뿐이다.**
    pub transcript: TranscriptText<'a>,
    /// 이 요청이 쓸 context 크기. 서버 기본값에 기대지 않는다 (ADR-0008 §8.2).
    pub context_budget: ContextBudget,
}

impl<'a> NoteRequest<'a> {
    /// 기본 context 예산으로 요청 하나를 만든다.
    pub fn new(mode: NoteType, transcript: &'a str) -> Self {
        Self {
            mode,
            transcript: TranscriptText::new(transcript),
            context_budget: ContextBudget::DEFAULT,
        }
    }

    /// 설정에서 온 context 예산을 쓴다.
    pub fn with_context_budget(self, context_budget: ContextBudget) -> Self {
        Self {
            context_budget,
            ..self
        }
    }

    /// 재료가 되는 전사 텍스트.
    pub fn transcript_text(&self) -> &'a str {
        self.transcript.text
    }
}

/// provider가 돌려주는 것.
///
/// `model`이 함께 오는 이유는 provenance다 (ADR-0008 §4.2 표 4행) — **실제로 어떤 모델이
/// 답했는지는 provider만 안다.** 호출자가 설정값을 그대로 적어 두면 그것은 기록이 아니라
/// 추정이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteGeneration {
    pub note: StructuredNote,
    /// 실제로 이 노트를 만든 모델. 그대로 `ai_notes.model`에 남는다.
    pub model: String,
}

/// transcript 텍스트 하나를 structured note 하나로 바꾸는 계약.
///
/// `Send + Sync`인 것은 생성이 UI를 막지 않고 다른 스레드에서 도는 자리에 놓이기 때문이다
/// (전사 경계와 같은 이유 · `crate::transcription::engine`). 여기서 스레드를 만들지는 않는다.
///
/// **비동기가 아니다** (ADR-0008 §4.3). 이 저장소에는 async runtime이 없고, 오래 걸리는 일은
/// 이미 평범한 스레드로 UI 밖에서 돈다. 호출 하나 때문에 런타임을 들이지 않는다.
pub trait NoteAiProvider: Send + Sync {
    /// 이 provider가 자기 자신에 대해 말하는 것 — 식별자 · 사람이 읽는 이름 · 로컬/외부 구분.
    fn descriptor(&self) -> ProviderDescriptor;

    /// 지금 쓸 수 있는 상태인가, 그리고 쓸 수 있는 모델은 무엇인가 (INV-8).
    fn availability(&self) -> Availability;

    /// 준비된 transcript 텍스트로 structured note 하나를 만든다.
    ///
    /// 거절은 언제나 §13의 공통 실패여야 한다 — 벤더 오류를 그대로 흘려보내면 INV-9가 깨진다.
    fn generate_note(&self, request: &NoteRequest<'_>) -> Result<NoteGeneration, Failure>;
}

/// 쓸 provider가 아직 골라지지 않았다 (§13 `provider 미설정`).
///
/// **adapter가 아니라 서비스 계층이 만든다** — 없는 provider에게 물어볼 수는 없기 때문이다
/// (ADR-0008 §4.3). 재시도 대상이 아니다: 같은 상태로 다시 눌러도 같고, 사용자가 설정에서
/// provider를 골라야 풀린다.
pub fn not_configured(message: impl Into<String>) -> Failure {
    Failure::permanent(FailureKind::AiProviderNotConfigured, message)
}

/// 고른 provider에 닿지 못했다 (§13 `provider 연결 불가`).
///
/// **재시도할 수 있다.** 사용자가 로컬 AI 서버를 켜고 같은 버튼을 다시 누르면 성공한다 —
/// 앱이 고칠 것은 없고 재시도가 의미를 갖는다.
///
/// 설정된 host/port를 `message`나 `detail`에 넣지 않는다 (ADR-0008 §11.3).
pub fn unreachable(message: impl Into<String>) -> Failure {
    Failure::retryable(FailureKind::AiProviderUnreachable, message)
}

/// provider는 응답했지만 쓸 수 있는 모델이 없다 (§13 `모델 없음`).
///
/// 연결 불가와 구분한다 — 서버는 살아 있다. 모델을 받아 오거나 다른 모델을 고르기 전에는
/// 다시 시도해도 같다 (`TranscriptionModelMissing`이 `permanent`인 것과 같은 판단이다).
pub fn model_unavailable(message: impl Into<String>) -> Failure {
    Failure::permanent(FailureKind::AiModelUnavailable, message)
}

/// 요청이 실패했고 **원인이 일시적이다** (ADR-0008 §13.3 — 타임아웃 · 5xx).
pub fn request_failed_temporarily(message: impl Into<String>) -> Failure {
    Failure::retryable(FailureKind::AiRequestFailed, message)
}

/// 요청 자체가 거절됐다 (ADR-0008 §13.3 — 4xx).
///
/// **우리가 보낸 요청이 잘못됐다는 뜻이며, 같은 요청을 다시 보내도 같다.** 그래서 같은
/// 종류(`AiRequestFailed`)이지만 재시도 값이 다르다 — 사용자가 알아야 하는 것은 "다시 눌러
/// 볼 가치가 있는가"이고, 그 답이 원인에 따라 갈린다.
pub fn request_rejected(message: impl Into<String>) -> Failure {
    Failure::permanent(FailureKind::AiRequestFailed, message)
}

/// 응답은 받았지만 기대한 노트 schema로 읽을 수 없다 (§13 `응답 schema 불일치`).
///
/// **재시도할 수 있다 — 여기서 전사와 갈린다.** `TranscriptionOutputUnusable`은 "같은 입력은
/// 같은 출력을 낸다"는 이유로 `permanent`지만, **생성은 결정론적이지 않다.** 같은 프롬프트가
/// 다음 번에는 읽을 수 있는 JSON을 낼 수 있고, 그것이 로컬 소형 모델에서 실제로 일어나는
/// 일이다 (ADR-0008 §13.2).
pub fn response_unusable(message: impl Into<String>) -> Failure {
    Failure::retryable(FailureKind::AiResponseUnusable, message)
}

/// 응답 검증이 거절한 이유를 §13의 공통 실패 **하나**로 옮긴다 (ADR-0008 §6.3 · §13.1).
///
/// [`ResponseRejection`]의 변형은 "무엇이 달랐는가"를 남기기 위한 것이지 서로 다른 제품
/// 상태가 아니다. 사용자가 할 수 있는 일은 어느 변형에서나 같으므로 실패는 하나로 모이고,
/// 구체적인 원인은 `detail`에 남는다.
pub fn rejected_response(rejection: &ResponseRejection) -> Failure {
    response_unusable("AI가 만든 노트를 읽을 수 없다").with_detail(rejection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generation_request_has_nowhere_to_put_audio() {
        // INV-6: 이 구조 분해는 **필드를 전부 나열한다.** 오디오 경로·바이트·핸들을 담는
        // 필드가 하나라도 생기면 이 테스트가 컴파일되지 않는다.
        let NoteRequest {
            mode,
            transcript,
            context_budget,
        } = NoteRequest::new(NoteType::Meeting, "말해진 것");

        assert_eq!(mode, NoteType::Meeting);
        assert_eq!(context_budget, ContextBudget::DEFAULT);

        // transcript가 담을 수 있는 것도 텍스트 하나뿐이다.
        let TranscriptText { text } = transcript;
        assert_eq!(text, "말해진 것");
    }

    #[test]
    fn a_request_carries_the_budget_the_caller_chose() {
        let request = NoteRequest::new(NoteType::Study, "긴 전사")
            .with_context_budget(ContextBudget { context_tokens: 8_192 });

        assert_eq!(request.context_budget.context_tokens, 8_192);
        assert_eq!(request.transcript_text(), "긴 전사");
        assert_eq!(request.mode, NoteType::Study);
    }

    #[test]
    fn the_five_ai_failures_never_collapse_into_one() {
        // §13: 사용자가 할 수 있는 일이 다섯 다 다르다. 종류가 겹치면 화면이 구분해 안내할 수 없다.
        let failures = [
            not_configured("provider를 고르지 않았다"),
            unreachable("AI 서버가 응답하지 않는다"),
            model_unavailable("쓸 수 있는 모델이 없다"),
            request_failed_temporarily("요청이 실패했다"),
            response_unusable("응답을 읽을 수 없다"),
        ];

        let mut seen: Vec<&str> = Vec::new();
        for failure in &failures {
            let kind = failure.kind.as_str();
            assert!(!seen.contains(&kind), "실패 종류가 겹친다: {kind}");
            seen.push(kind);

            // 어떤 AI 실패도 원본을 훼손하지 않는다 (INV-3). 이 경계는 오디오도 전사도 쓰지 않는다.
            assert!(failure.source_data_safe, "{kind}");
        }

        assert_eq!(
            seen.len(),
            AI_PROVIDER_FAILURE_KINDS.len(),
            "계약이 만드는 실패와 목록이 어긋난다"
        );
    }

    #[test]
    fn retrying_is_worth_it_only_where_the_situation_can_change() {
        // ADR-0008 §13.2의 매핑 표 그대로다.
        assert!(!not_configured("").retryable, "설정에서 골라야 풀린다");
        assert!(unreachable("").retryable, "서버를 켜고 다시 누르면 된다");
        assert!(!model_unavailable("").retryable, "모델을 받아야 풀린다");
        assert!(request_failed_temporarily("").retryable, "타임아웃 · 5xx");
        assert!(!request_rejected("").retryable, "4xx — 같은 요청은 같은 답이다");
        assert!(
            response_unusable("").retryable,
            "생성은 결정론적이지 않다 — 전사와 갈리는 지점이다"
        );
    }

    #[test]
    fn both_request_outcomes_are_the_same_kind_of_failure() {
        // 재시도 값은 갈리지만 사용자가 보는 실패의 종류는 하나다 (§13.3).
        assert_eq!(
            request_failed_temporarily("").kind,
            FailureKind::AiRequestFailed
        );
        assert_eq!(request_rejected("").kind, FailureKind::AiRequestFailed);
    }

    #[test]
    fn every_rejected_response_becomes_one_visible_failure_with_its_reason_kept() {
        for rejection in [
            ResponseRejection::Empty,
            ResponseRejection::NotJson,
            ResponseRejection::Shape {
                detail: "missing field `overview`".to_owned(),
            },
            ResponseRejection::BlankRequiredText { field: "overview" },
        ] {
            let failure = rejected_response(&rejection);

            assert_eq!(failure.kind, FailureKind::AiResponseUnusable);
            assert!(failure.retryable);
            assert!(failure.source_data_safe);
            assert_eq!(
                failure.detail.as_deref(),
                Some(rejection.to_string().as_str()),
                "무엇이 달랐는지는 사라지지 않는다"
            );
        }
    }

    #[test]
    fn locality_is_a_definite_answer_the_screen_can_show() {
        // INV-5: 사용자가 알아야 하는 것은 "데이터가 나가는가"이고, 그 답은 둘 중 하나다.
        assert!(Locality::Local.is_local());
        assert!(!Locality::External.is_local());
        assert_eq!(Locality::Local.as_str(), "local");
        assert_eq!(Locality::External.as_str(), "external");
        assert_ne!(Locality::Local.as_str(), Locality::External.as_str());
    }

    #[test]
    fn availability_separates_no_models_from_not_answering() {
        let ready = Availability::Ready {
            models: vec!["some-model".to_owned()],
        };
        assert!(ready.is_ready());
        assert_eq!(ready.models(), ["some-model".to_owned()].as_slice());

        // 응답했지만 모델이 없는 것과, 응답하지 않는 것은 다른 제품 상태다.
        assert!(!Availability::NoModels.is_ready());
        assert!(Availability::NoModels.models().is_empty());

        let unavailable = Availability::Unavailable(unreachable("응답이 없다"));
        assert!(!unavailable.is_ready());
        assert!(unavailable.models().is_empty());
        assert_ne!(unavailable, Availability::NoModels);
    }
}
