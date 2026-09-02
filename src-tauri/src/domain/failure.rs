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
/// **지금 실제로 만들어지는 것만 있다.** 전사·AI provider·Notion의 실패 종류는 §13에 이미
/// 적혀 있지만, 그 기능이 존재하는 Phase가 그때 함께 추가한다 — 만들지 않은 실패의 자리를
/// 미리 만들어 두지 않는다 (§20.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureKind {
    /// 로컬 저장소를 열거나 읽고 쓰지 못했다. 저장소 초기화 실패도 여기 속한다.
    Storage,
    /// 들어온 값이 규칙에 맞지 않아 아무것도 하지 않았다.
    InvalidInput,
}

impl FailureKind {
    /// 직렬화·전송에 쓰는 안정적인 문자열 표현.
    ///
    /// 이 문자열은 frontend의 `FailureKind`(`src/ipc/failure.ts`)와 1:1이다.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::InvalidInput => "invalidInput",
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
    fn every_kind_has_a_distinct_stable_string() {
        assert_eq!(FailureKind::Storage.as_str(), "storage");
        assert_eq!(FailureKind::InvalidInput.as_str(), "invalidInput");
        assert_ne!(FailureKind::Storage.as_str(), FailureKind::InvalidInput.as_str());
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
