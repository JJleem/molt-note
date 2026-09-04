//! AI 노트 경계 (PRODUCT-SPEC §9 · `docs/ADR-0008-note-ai-provider.md`).
//!
//! ```text
//! note.rs      세 mode의 structured note 타입 · ai_notes.content 봉투 · 요구할 JSON Schema ·
//!              **응답이 기대와 다를 때의 방어 경로** (ADR-0008 §6.3 · §7)
//! prompt.rs    mode별 프롬프트 상수 · promptVersion(§10) · transcript를 프롬프트 입력으로
//!              준비하는 자리와 context 예산 판정 (§8)
//! provider.rs  **벤더 중립 계약**(trait) · §13의 AI provider 실패 다섯 (ADR-0008 §4 · §13)
//! run.rs       Recording 하나에 대한 **실행 순서** — 입력 선택 · 크기 판정 · 저장 · 상태 전이
//!              (§7.2 · §7.3 · ADR-0008 §9)
//! testing.rs   그 계약의 재사용 가능한 준수 검사와 결정론적 test double (§18)
//! ollama/      그 계약의 실제 구현 하나 — **벤더 지식은 이 디렉터리 안에서만 산다** (INV-9)
//! ```
//!
//! **이 파일들에는 벤더가 없다.** 네트워크도 파일도 오디오도 프로세스도 없고, 특정 제공자의
//! 엔드포인트·파라미터 이름·에러 코드도 없다 (INV-6 · INV-9). 실제 provider 구현(adapter)은
//! 이 모듈 **아래의 별도 자리**에 오며, 벤더 지식은 그 안에서만 산다 — 지금 있는 것은
//! [`ollama`] 하나이고, 그 디렉터리 이름 말고는 어떤 벤더 지식도 이 파일에 없다.
//!
//! **adapter를 이름으로 아는 파일은 이 파일 하나다** — `pub mod ollama;`는 피할 수 없고,
//! 어떤 adapter가 있는지 아는 자리가 하나여야 나머지가 벤더를 모를 수 있다. 그래서 설정에
//! 저장된 식별자를 실제 구현으로 옮기는 함수도 여기 있다 ([`provider_for`]). 그 함수가 하는
//! 일은 **adapter가 스스로 말하는 식별자와 대조하고 설정 값을 넘기는 것**뿐이며, 주소도 요청
//! 형태도 상태 코드 해석도 여전히 adapter 안에 있다 (`tests/ollama_adapter.rs`가 그것을 소스에서
//! 확인한다).
//!
//! ```text
//! transcript 텍스트 ─→ prompt::prepare ─→ 프롬프트 + promptVersion ─┐
//!                                                                  ├─→ provider::NoteAiProvider
//! note::json_schema ───────────────────────────────────────────────┘        (adapter가 이행한다)
//!                                                                          │
//!         ai_notes.content ←── note::encode_content ←── StructuredNote ←── note::parse_note
//! ```
//!
//! [`note`]와 [`prompt`]는 값에서 값을 만드는 함수뿐이라 어떤 서버도 띄우지 않고 검증된다
//! (ADR-0008 §18 · §4.3 "adapter 내부도 둘로 나눈다"). [`note::ResponseRejection`]을 §13의
//! 공통 실패로 옮기는 것은 계약을 만드는 쪽의 일이며 ([`provider::rejected_response`]),
//! [`prompt::ContextOverflow`]를 옮기는 것은 **요청 전 크기를 판정하는 실행 순서**의 일이다
//! ([`run::input_too_large`]) — 만들지 않은 실패의 자리를 미리 만들어 두지 않는다 (§20.6).
//!
//! [`run`]은 이 조각들을 잇고 저장소에 닿는 유일한 자리다. 그 위(스레드 · command · 화면)와
//! 그 아래(벤더)는 서로를 알지 않는다.

pub mod note;
pub mod ollama;
pub mod prompt;
pub mod provider;
pub mod run;
pub mod testing;

pub use note::{
    decode_content, encode_content, json_schema, parse_note, MeetingNote, ResponseRejection,
    StructuredNote, StudyNote, SummaryNote, CONTENT_SCHEMA_VERSION,
};
pub use prompt::{
    build_prompt, estimate_tokens, prepare, prompt_template, prompt_version, ContextBudget,
    ContextOverflow, PreparedPrompt,
};
pub use provider::{
    model_unavailable, not_configured, rejected_response, request_failed_temporarily,
    request_rejected, response_unusable, unreachable, Availability, Locality, NoteAiProvider,
    NoteGeneration, NoteRequest, ProviderDescriptor, TranscriptText, AI_PROVIDER_FAILURE_KINDS,
};
pub use run::{generate, input_too_large, Outcome};

// **test double은 여기서 다시 내보내지 않는다.** 계약 준수 검사와 double을 쓰는 쪽은
// `crate::ai::testing`을 이름 그대로 부른다 — 제품 경로에서 `ai::` 한 겹으로 닿는 provider
// 목록에 double이 섞이지 않게 하기 위해서다 (ADR-0008 §4.3).

use std::sync::Arc;

use crate::domain::Settings;

/// 설정이 고른 provider. 고르지 않았거나 이 앱이 모르는 식별자면 `None`이다.
///
/// ```text
/// Settings { ai_provider · ai_base_url · ai_model } ─→ provider_for ─→ Arc<dyn NoteAiProvider>
///                                                          │
///                          고르지 않았거나 만들 수 없는 식별자면 ──→ None (INV-8)
/// ```
///
/// **`None`은 오류가 아니다.** provider를 고르지 않은 것은 정상 상태이며, 없는 것에게
/// 물어보지 않는 것이 INV-8을 타입으로 적는 방법이다 ([`provider`] 모듈 문서). §13의
/// `provider 미설정` 실패를 만드는 것은 이 값을 받아 쓰는 **서비스 계층**이다 (ADR-0008 §4.3) —
/// 여기서는 아직 아무것도 실패하지 않았다.
///
/// **연결 대상과 모델은 여기서 정하지 않고 설정에서 온다** (ADR-0008 §11.1). 주소를 고르지
/// 않았을 때 무엇을 쓰는지 아는 자리도 여기가 아니라 [`Settings::ai_base_url_or_default`] 하나다.
///
/// 모델을 아직 고르지 않았어도 provider는 만들어진다 — **설치된 모델 목록을 물어보려면
/// provider가 먼저 있어야 하고**, 사용자는 그 목록에서 모델을 고른다 (`phase-prompt/04` 요구 8).
/// 고른 모델이 없는 채로 생성을 시작하지 않게 막는 것은 이 함수가 아니라 생성을 시작하는 쪽이다
/// (`crate::commands::notes`).
///
/// 값이 실제로 쓸 수 있는지도 여기서 묻지 않는다 — 서버가 응답하는지도, 그 모델이 설치돼
/// 있는지도 물어봐야 알 수 있고, 그 답은 [`NoteAiProvider::availability`]가 낸다 (INV-8).
pub fn provider_for(settings: &Settings) -> Option<Arc<dyn NoteAiProvider>> {
    let chosen = settings.ai_provider.as_deref()?;

    // 알아볼 수 없는 식별자를 다른 provider로 바꿔 고르지 않는다 — 사용자가 고른 적 없는
    // provider에게 전사를 보내는 일이 추측으로 일어나서는 안 된다 (§12 · INV-9).
    if chosen != ollama::PROVIDER_ID {
        return None;
    }

    Some(Arc::new(ollama::OllamaProvider::new(
        settings.ai_base_url_or_default(),
        settings.ai_model.clone().unwrap_or_default(),
        Arc::new(ollama::UreqTransport::new()),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_built_when_no_provider_was_chosen() {
        // INV-8: 고르지 않은 것은 정상 상태다. 아무 provider나 기본으로 세우지 않는다.
        assert!(provider_for(&Settings::DEFAULT).is_none());
    }

    #[test]
    fn an_identifier_this_app_cannot_build_is_not_swapped_for_another_one() {
        let settings = Settings {
            ai_provider: Some("어떤-다른-provider".to_owned()),
            ..Settings::DEFAULT
        };

        assert!(provider_for(&settings).is_none());
    }

    #[test]
    fn a_provider_is_built_even_before_a_model_has_been_chosen() {
        // 모델 목록을 물어보려면 provider가 먼저 있어야 한다 (요구 8).
        let settings = Settings {
            ai_provider: Some(ollama::PROVIDER_ID.to_owned()),
            ai_model: None,
            ..Settings::DEFAULT
        };

        let provider = provider_for(&settings).expect("고른 provider를 만들 수 있어야 한다");
        let descriptor = provider.descriptor();

        // 식별자는 그대로 `ai_notes.provider`에 남고 (§7.3), 로컬/외부 구분은 provider가
        // 스스로 말한다 (INV-5).
        assert_eq!(descriptor.id, ollama::PROVIDER_ID);
        assert!(!descriptor.name.trim().is_empty(), "화면에 그릴 이름이 있다");
        assert!(descriptor.locality.is_local());
    }
}

