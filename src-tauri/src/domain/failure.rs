//! 사용자에게 보여줄 수 있는 도메인 공통 실패 타입 (PRODUCT-SPEC §13).
//!
//! §13은 실패를 버그가 아니라 **정상적인 제품 상태**로 다루며, 각 실패에 대해 사용자가
//! 세 가지를 알 수 있어야 한다고 못박는다:
//!
//! ```text
//! 무엇이 실패했는가 · 원본 데이터는 안전한가 · 다시 시도할 수 있는가
//! ```
//!
//! 이 타입의 필드가 그 세 질문에 대한 답이다. 그래서 실패를 로그로 흘려보내고 끝낼 수 없다 —
//! command가 오류를 돌려주려면 이 값을 채워야 하고, 채운 값은 그대로 화면에 도달한다.
//!
//! **이 타입은 직렬화된다.** domain의 다른 타입과 달리 [`Failure`]는 UI와의 계약 그 자체이며,
//! 저장소·플랫폼 같은 adapter가 자신의 오류를 여기로 옮긴다. 어떤 계층의 실패든 UI에는
//! 하나의 모양으로 도착한다 (§13의 "provider별 에러 의미는 adapter가 domain 공통 실패 타입으로
//! 변환한다"와 같은 방향이다).

use std::fmt;

use serde::Serialize;

/// 실패가 어느 종류인지.
///
/// **지금 실제로 만들어지는 것만 있다** — 만들지 않은 실패의 자리를 미리 만들어 두지
/// 않는다 (§20.6). 전사 실패 네 종류는 **전사가 실재하는 Phase 3에서** 그 규칙대로 추가됐고
/// (`crate::transcription::engine`), AI provider 실패 다섯 종류는 **provider 계약이 실재하는
/// Phase 4에서** 같은 규칙으로 추가됐다 (`crate::ai::provider`). 여섯 번째인
/// [`Self::AiInputTooLarge`]는 그보다 늦다 — **요청 전에 크기를 판정하는 실행 순서가 실재하는
/// 자리에서** 추가됐다 (`crate::ai::run`). Notion 실패 다섯 종류도 같은 규칙으로 **Notion과
/// 실제로 말하는 adapter가 실재하는 Phase 5에서** 추가됐다 (`crate::notion::client`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureKind {
    /// 로컬 저장소를 열거나 읽고 쓰지 못했다. 저장소 초기화 실패도 여기 속한다.
    Storage,
    /// 들어온 값이 규칙에 맞지 않아 아무것도 하지 않았다.
    InvalidInput,
    /// 오디오 입력 장치를 다루지 못했다. 지금은 장치 목록을 읽지 못한 경우가 여기 속한다.
    ///
    /// 저장소 실패와 구분한다 — 사용자가 할 수 있는 일이 다르다. 저장소는 앱을 다시 시작하는
    /// 문제이고, 이쪽은 장치 연결의 문제다.
    AudioDevice,
    /// 마이크에 접근할 수 없다 (§13의 `microphone permission denied`).
    ///
    /// **장치 실패와도, 녹음 초기화 실패와도 구분한다.** 사용자가 할 수 있는 일이 다르기
    /// 때문이다 — 이 실패는 다른 장치를 고르거나 앱을 다시 시작해서 풀리지 않고, 시스템의
    /// 권한 설정에서 접근을 허용해야 풀린다.
    ///
    /// **어디서 무엇을 켜야 하는지는 domain이 알지 않는다.** 그 문장은 platform 경계가
    /// 만들어 넣는다 (`crate::platform::microphone_permission` · INV-10).
    MicrophonePermission,
    /// 전사에 쓸 모델 파일이 그 자리에 없다 (§13 `모델 파일 없음`).
    ///
    /// **아래 네 전사 실패는 서로 다른 종류로 남는다.** 사용자가 할 수 있는 일이 전부 다르기
    /// 때문이다 — 모델을 구해 오는 것 · 다른 모델을 고르는 것 · 다시 시도하는 것 · 다른 녹음을
    /// 쓰는 것. 하나로 뭉치면 화면이 그 넷을 구분해 안내할 수 없다.
    TranscriptionModelMissing,
    /// 모델 파일은 그 자리에 있지만 읽을 수 없거나 엔진이 지원하지 않는다
    /// (§13 `unsupported whisper model`).
    TranscriptionModelUnusable,
    /// 엔진이 실행되지 못했거나 비정상 종료했다 (§13 `transcription process failure`).
    TranscriptionEngineFailed,
    /// 엔진은 끝났지만 출력이 없거나 그 출력을 전사 결과로 해석할 수 없다.
    TranscriptionOutputUnusable,
    /// 쓸 AI provider가 아직 골라지지 않았다 (§13 `provider 미설정` ·
    /// `docs/ADR-0008-note-ai-provider.md` §13.1).
    ///
    /// **아래 다섯 AI provider 실패도 서로 다른 종류로 남는다.** 전사 실패 넷과 같은 이유다 —
    /// 사용자가 할 수 있는 일이 전부 다르다: provider를 고르는 것 · 로컬 AI 서버를 켜는 것 ·
    /// 모델을 받아 오는 것 · 다시 시도하는 것 · 다른 요청을 만드는 것.
    ///
    /// **이것은 오류로 표시할 상태가 아니다.** provider가 없는 것은 정상 상태이며 (INV-8),
    /// 화면은 경고가 아니라 "AI 기능이 비활성"이라는 담담한 상태를 보인다. 이 실패 값은
    /// 그 상태에서 굳이 생성을 요청했을 때의 답이다.
    AiProviderNotConfigured,
    /// 고른 provider에 닿지 못했다 (§13 `provider 연결 불가` — 예: 로컬 AI 서버가 실행 중이 아니다).
    AiProviderUnreachable,
    /// provider는 응답했지만 쓸 수 있는 모델이 없다 (§13 `모델 없음`).
    AiModelUnavailable,
    /// provider에게 보낸 요청이 실패했다 (§13 `요청 실패` — 비정상 상태 코드 · 타임아웃 ·
    /// 본문을 읽지 못함).
    AiRequestFailed,
    /// 응답은 받았지만 기대한 노트 schema로 읽을 수 없다 (§13 `응답 schema 불일치`).
    AiResponseUnusable,
    /// 보낼 입력이 한 번의 생성이 쓸 수 있는 context 예산을 넘는다
    /// (`docs/ADR-0008-note-ai-provider.md` §8.2 · §13.1의 여섯 번째).
    ///
    /// **요청을 보내지 않았다는 뜻이다.** 잘라서 보내면 완전해 보이지만 완전하지 않은 노트가
    /// 남고, 사용자는 뒷부분이 통째로 빠진 것을 알 수 없다 — 조용한 절단 대신 보이는 상태로
    /// 둔다. 다른 다섯과 종류를 나누는 이유도 같다: 이 상황에서 사용자가 할 수 있는 일은
    /// **설정에서 context 크기를 키우거나, 더 큰 context의 모델을 고르거나, 더 짧은 녹음을
    /// 고르는 것**이며 다른 어느 실패와도 다르다.
    ///
    /// **provider가 만드는 실패가 아니다.** 만드는 쪽은 요청 전에 크기를 판정하는 실행 순서이며
    /// (`crate::ai::run`), 그래서 `crate::ai::provider::AI_PROVIDER_FAILURE_KINDS`에 없다.
    AiInputTooLarge,
    /// Notion이 이 앱의 자격증명을 받아들이지 않았다 (§13 `authentication failure` ·
    /// `docs/ADR-0009-notion-and-export.md` §9.3).
    ///
    /// **아래 다섯 Notion 실패도 서로 다른 종류로 남는다.** 전사 실패 넷 · AI 실패 여섯과 같은
    /// 이유다 — 사용자가 할 수 있는 일이 전부 다르다: token을 다시 넣는 것 · 부모 페이지를
    /// integration에 공유하는 것 · 잠시 기다리는 것 · 다시 시도하는 것 · Notion에서 무엇이
    /// 만들어졌는지 확인하는 것.
    ///
    /// **다섯뿐인 것도 같은 규칙이다** (§20.6). Notion 설정이 없는 상태 · 전송이 진행 중인
    /// 상태처럼 아직 만들어지지 않은 실패의 자리를 미리 만들지 않았다 — 다섯 전부
    /// `crate::notion::client`가 실제로 만드는 것이며, 그 사실을 `NOTION_FAILURE_KINDS`와
    /// 그 테스트가 고정한다.
    NotionAuthFailed,
    /// 보내려는 자리에 닿을 수 없다 — 부모 페이지가 없거나 integration에 공유되지 않았다
    /// (§13 `sync failure` 중 `권한 없는 destination` · `phase-prompt/05` 요구 12).
    ///
    /// 인증 실패와 나누는 이유는 사용자가 할 일이 다르기 때문이다. token은 멀쩡한데 페이지를
    /// 공유하지 않은 상황이 이 제품에서 가장 흔한 첫 실패이며, "token을 다시 넣어라"는 안내는
    /// 그 사용자를 엉뚱한 곳으로 보낸다.
    NotionDestinationUnavailable,
    /// Notion이 요청 속도를 제한하고 있다 (§13 `rate limit` · ADR-0009 §9).
    ///
    /// **다시 보내면 되는 실패다.** 언제 보내면 되는지는 `crate::notion::RetryAfter`가 값으로
    /// 함께 나온다 — 그 값을 `Failure`에 담지 않는 이유는 벤더 하나 때문에 domain 계약이
    /// 넓어지지 않게 하기 위해서다.
    NotionRateLimited,
    /// Notion에 보낸 요청이 실패했다 (§13 `sync failure` — 연결 불가 · 타임아웃 · 거절된 요청 ·
    /// 서버 쪽 오류).
    ///
    /// 재시도 가치는 원인에 따라 갈리지만 사용자가 보는 종류는 하나다 — 어느 쪽이든 할 수 있는
    /// 일은 "다시 시도한다"뿐이다.
    NotionRequestFailed,
    /// Notion이 응답했지만 만들어진 페이지를 확인할 수 없다 (ADR-0009 §7.3 · §8.5의 '결과를 모름').
    ///
    /// **"모른다"를 "실패했다"로도 "성공했다"로도 바꿔 적지 않는다.** 페이지가 만들어졌을 수
    /// 있으므로 그대로 다시 보내면 사용자가 모르는 사이에 페이지가 둘이 된다. 그래서 이 실패는
    /// 재시도 가능으로 표시되지 않고, 사용자가 Notion을 확인한 뒤 다시 고르는 자리로 남는다.
    NotionResponseUnusable,
}

impl FailureKind {
    /// 직렬화·전송에 쓰는 안정적인 문자열 표현.
    ///
    /// 이 문자열은 frontend의 `FailureKind`(`src/ipc/failure.ts`)와 1:1이다.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::InvalidInput => "invalidInput",
            Self::AudioDevice => "audioDevice",
            Self::MicrophonePermission => "microphonePermission",
            Self::TranscriptionModelMissing => "transcriptionModelMissing",
            Self::TranscriptionModelUnusable => "transcriptionModelUnusable",
            Self::TranscriptionEngineFailed => "transcriptionEngineFailed",
            Self::TranscriptionOutputUnusable => "transcriptionOutputUnusable",
            Self::AiProviderNotConfigured => "aiProviderNotConfigured",
            Self::AiProviderUnreachable => "aiProviderUnreachable",
            Self::AiModelUnavailable => "aiModelUnavailable",
            Self::AiRequestFailed => "aiRequestFailed",
            Self::AiResponseUnusable => "aiResponseUnusable",
            Self::AiInputTooLarge => "aiInputTooLarge",
            Self::NotionAuthFailed => "notionAuthFailed",
            Self::NotionDestinationUnavailable => "notionDestinationUnavailable",
            Self::NotionRateLimited => "notionRateLimited",
            Self::NotionRequestFailed => "notionRequestFailed",
            Self::NotionResponseUnusable => "notionResponseUnusable",
        }
    }
}

impl fmt::Display for FailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 사용자에게 보여줄 수 있는 실패 하나.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub kind: FailureKind,
    /// **무엇이 실패했는가** — 그대로 화면에 띄울 수 있는 한 문장.
    ///
    /// 기술적 원인이 아니라 사용자가 읽을 문장이다. 원인은 [`Self::detail`]에 둔다.
    pub message: String,
    /// 원인의 기술적 표현. 없을 수 있다.
    ///
    /// 화면에 반드시 띄워야 하는 값은 아니다 — 사용자가 스스로 판단하거나 옮겨 적을 때 쓴다.
    pub detail: Option<String>,
    /// **원본 데이터는 안전한가** — 이 실패가 이미 저장된 녹음·전사를 훼손했는가.
    ///
    /// Phase 1에서 발생 가능한 실패는 전부 안전하다: 조회 실패는 아무것도 바꾸지 않고,
    /// migration은 트랜잭션 안에서 되돌아가며, 쓰기 실패는 행을 남기지 않는다.
    /// 원본을 훼손할 수 있는 실패가 생기는 Phase에서 이 값이 `false`가 되는 경로가 생긴다.
    pub source_data_safe: bool,
    /// **다시 시도할 수 있는가** — 같은 동작을 그대로 다시 실행해 볼 가치가 있는가.
    ///
    /// `false`는 "재시도해도 같은 결과다"라는 뜻이며, 사용자가 무언가를 고쳐야 한다.
    pub retryable: bool,
}

impl Failure {
    /// 다시 시도해도 결과가 같은 실패. 사용자가 입력이나 환경을 고쳐야 한다.
    pub fn permanent(kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            detail: None,
            source_data_safe: true,
            retryable: false,
        }
    }

    /// 같은 동작을 다시 시도해 볼 수 있는 실패.
    pub fn retryable(kind: FailureKind, message: impl Into<String>) -> Self {
        Self {
            retryable: true,
            ..Self::permanent(kind, message)
        }
    }

    /// 기술적 원인을 덧붙인다. 사용자 문장([`Self::message`])은 바꾸지 않는다.
    pub fn with_detail(self, detail: impl fmt::Display) -> Self {
        Self {
            detail: Some(detail.to_string()),
            ..self
        }
    }

    /// 이 실패가 **원본 데이터를 온전한 상태로 남기지 못했다**고 표시한다 ([`Self::source_data_safe`]).
    ///
    /// 기본값은 "안전하다"이며, 그 값을 실패마다 다시 주장하지 않는다. 원본을 훼손할 수 있는
    /// 실패가 실제로 생기는 경로에서만 이 표시를 붙인다 — 지금은 **녹음 파일을 확정하지 못한
    /// 경우**가 그것이다. 그때 사용자는 방금 녹음한 것을 신뢰할 수 없다.
    pub fn with_source_data_at_risk(self) -> Self {
        Self {
            source_data_safe: false,
            ..self
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_answers_the_three_questions_of_section_13() {
        let failure = Failure::retryable(FailureKind::Storage, "로컬 저장소를 열지 못했다");

        assert_eq!(failure.message, "로컬 저장소를 열지 못했다", "무엇이 실패했는가");
        assert!(failure.source_data_safe, "원본 데이터는 안전한가");
        assert!(failure.retryable, "다시 시도할 수 있는가");
    }

    #[test]
    fn a_permanent_failure_says_retrying_will_not_help() {
        let failure = Failure::permanent(FailureKind::InvalidInput, "녹음 제목이 비어 있다");

        assert!(!failure.retryable);
        assert_eq!(failure.kind, FailureKind::InvalidInput);
    }

    #[test]
    fn adding_a_detail_does_not_change_the_sentence_shown_to_the_user() {
        let failure = Failure::permanent(FailureKind::Storage, "저장소 질의가 실패했다")
            .with_detail("no such table: recordings");

        assert_eq!(failure.message, "저장소 질의가 실패했다");
        assert_eq!(failure.detail.as_deref(), Some("no such table: recordings"));
    }

    #[test]
    fn a_failure_can_say_the_source_data_is_not_intact() {
        // 기본값은 "안전하다"이고, 그렇지 않은 경로에서만 표시가 붙는다.
        let safe = Failure::permanent(FailureKind::Storage, "녹음 파일을 확정하지 못했다");
        assert!(safe.source_data_safe);

        let at_risk = safe.clone().with_source_data_at_risk();

        assert!(!at_risk.source_data_safe);
        assert_eq!(at_risk.message, safe.message, "문장을 바꾸지 않는다");
        assert_eq!(at_risk.retryable, safe.retryable);
        assert_eq!(
            serde_json::to_value(&at_risk).expect("직렬화할 수 있어야 한다")["sourceDataSafe"],
            false,
            "화면이 읽는 필드에 그대로 도달한다"
        );
    }

    #[test]
    fn every_kind_has_a_distinct_stable_string() {
        assert_eq!(FailureKind::Storage.as_str(), "storage");
        assert_eq!(FailureKind::InvalidInput.as_str(), "invalidInput");
        assert_eq!(FailureKind::AudioDevice.as_str(), "audioDevice");
        assert_eq!(
            FailureKind::MicrophonePermission.as_str(),
            "microphonePermission"
        );
        assert_eq!(
            FailureKind::TranscriptionModelMissing.as_str(),
            "transcriptionModelMissing"
        );
        assert_eq!(
            FailureKind::TranscriptionModelUnusable.as_str(),
            "transcriptionModelUnusable"
        );
        assert_eq!(
            FailureKind::TranscriptionEngineFailed.as_str(),
            "transcriptionEngineFailed"
        );
        assert_eq!(
            FailureKind::TranscriptionOutputUnusable.as_str(),
            "transcriptionOutputUnusable"
        );
        assert_eq!(
            FailureKind::AiProviderNotConfigured.as_str(),
            "aiProviderNotConfigured"
        );
        assert_eq!(
            FailureKind::AiProviderUnreachable.as_str(),
            "aiProviderUnreachable"
        );
        assert_eq!(FailureKind::AiModelUnavailable.as_str(), "aiModelUnavailable");
        assert_eq!(FailureKind::AiRequestFailed.as_str(), "aiRequestFailed");
        assert_eq!(FailureKind::AiResponseUnusable.as_str(), "aiResponseUnusable");
        assert_eq!(FailureKind::AiInputTooLarge.as_str(), "aiInputTooLarge");
        assert_eq!(FailureKind::NotionAuthFailed.as_str(), "notionAuthFailed");
        assert_eq!(
            FailureKind::NotionDestinationUnavailable.as_str(),
            "notionDestinationUnavailable"
        );
        assert_eq!(FailureKind::NotionRateLimited.as_str(), "notionRateLimited");
        assert_eq!(FailureKind::NotionRequestFailed.as_str(), "notionRequestFailed");
        assert_eq!(
            FailureKind::NotionResponseUnusable.as_str(),
            "notionResponseUnusable"
        );

        // 이 문자열들은 `src/ipc/failure.ts`의 union과 1:1이다. 겹치면 화면이 두 실패를
        // 구분하지 못한다.
        let kinds = [
            FailureKind::Storage,
            FailureKind::InvalidInput,
            FailureKind::AudioDevice,
            FailureKind::MicrophonePermission,
            FailureKind::TranscriptionModelMissing,
            FailureKind::TranscriptionModelUnusable,
            FailureKind::TranscriptionEngineFailed,
            FailureKind::TranscriptionOutputUnusable,
            FailureKind::AiProviderNotConfigured,
            FailureKind::AiProviderUnreachable,
            FailureKind::AiModelUnavailable,
            FailureKind::AiRequestFailed,
            FailureKind::AiResponseUnusable,
            FailureKind::AiInputTooLarge,
            FailureKind::NotionAuthFailed,
            FailureKind::NotionDestinationUnavailable,
            FailureKind::NotionRateLimited,
            FailureKind::NotionRequestFailed,
            FailureKind::NotionResponseUnusable,
        ];
        let mut seen = Vec::new();
        for kind in kinds {
            let text = kind.as_str();
            assert!(!seen.contains(&text), "실패 종류 문자열이 겹친다: {text}");
            seen.push(text);
        }
    }

    #[test]
    fn the_serialized_shape_is_the_contract_the_frontend_reads() {
        // 필드 이름이 바뀌면 src/ipc/failure.ts의 타입이 조용히 어긋난다.
        let failure = Failure::retryable(FailureKind::Storage, "열지 못했다").with_detail("io error");

        let json = serde_json::to_value(&failure).expect("직렬화할 수 있어야 한다");

        assert_eq!(json["kind"], "storage");
        assert_eq!(json["message"], "열지 못했다");
        assert_eq!(json["detail"], "io error");
        assert_eq!(json["sourceDataSafe"], true);
        assert_eq!(json["retryable"], true);
    }

    #[test]
    fn a_missing_detail_serializes_as_null_rather_than_disappearing() {
        let json = serde_json::to_value(Failure::permanent(FailureKind::Storage, "실패"))
            .expect("직렬화할 수 있어야 한다");

        assert!(json.get("detail").is_some(), "detail 키 자체는 항상 있어야 한다");
        assert!(json["detail"].is_null());
    }
}
