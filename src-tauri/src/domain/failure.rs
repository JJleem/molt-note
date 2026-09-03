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
/// **지금 실제로 만들어지는 것만 있다.** AI provider·Notion의 실패 종류는 §13에 이미 적혀
/// 있지만, 그 기능이 존재하는 Phase가 그때 함께 추가한다 — 만들지 않은 실패의 자리를 미리
/// 만들어 두지 않는다 (§20.6). 전사 실패 네 종류는 **전사가 실재하는 Phase 3에서** 그 규칙대로
/// 추가됐다 (`crate::transcription::engine`).
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
