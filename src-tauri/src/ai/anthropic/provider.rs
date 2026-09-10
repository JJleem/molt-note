//! Anthropic Messages API로 노트를 만드는 adapter.
//!
//! ```text
//! NoteRequest ─→ ai::prompt ─→ wire::request_body ─→ │ HttpTransport │ ─→ 상태 + 본문
//!                                                     └───────────────┘
//!                             ai::parse_note ←─ wire::text_from_response ←─┘
//! ```
//!
//! **로컬 AI adapter와 같은 계약을 이행한다** ([`NoteAiProvider`]). 다른 것은 두 가지뿐이다.
//!
//! ```text
//! locality   External — 전사 텍스트가 기기 밖으로 나간다. 화면이 이 사실을 말한다 (INV-5)
//! 자격증명    SecretStore. 설정(평문 DB)에 두지 않는다 — Notion 토큰과 같은 판단이다
//! ```
//!
//! ## ⚠️ 오디오는 나가지 않는다
//!
//! 나가는 것은 **전사된 텍스트**뿐이다 (PRODUCT-SPEC §12 · INV-6). 오디오 파일도, 그 조각도
//! 이 경로를 지나지 않는다 — `NoteRequest`가 애초에 텍스트만 들고 있다.
//!
//! ## 키를 문장에 넣지 않는다
//!
//! 실패 message나 detail에 API 키가 섞이지 않는다. `TransportError`에 문자열이 없는 것과
//! 같은 이유이며(ADR-0009 §11.3의 태도), 여기서는 키를 만지는 자리를 [`Self::api_key`]
//! 하나로 가둬 그것을 지킨다.

use std::sync::Arc;

use crate::ai::note::ResponseRejection;
use crate::ai::prompt::ContextBudget;
use crate::ai::provider::{
    model_unavailable, not_configured, rejected_response, request_failed_temporarily,
    request_rejected, unreachable,
    Availability, Locality, NoteAiProvider, NoteGeneration, NoteRequest, ProviderDescriptor,
};
use crate::ai::prompt::build_prompt;
use crate::domain::Failure;
use crate::net::{HttpHeader, HttpRequest, HttpResponse, HttpTransport, TransportError};
use crate::platform::secret_store::{SecretKey, SecretStore};

use super::wire;

/// 설정의 `ai_provider`가 이 adapter를 고르는 값.
pub const PROVIDER_ID: &str = "anthropic";

/// 사람이 읽는 이름.
pub const PROVIDER_NAME: &str = "Claude (Anthropic)";

pub struct AnthropicProvider {
    /// 쓸 모델. 비어 있으면 [`wire::DEFAULT_MODEL`]이다.
    model: String,
    secrets: Arc<dyn SecretStore>,
    transport: Arc<dyn HttpTransport>,
}

impl AnthropicProvider {
    pub fn new(
        model: String,
        secrets: Arc<dyn SecretStore>,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            model,
            secrets,
            transport,
        }
    }

    /// 저장된 API 키.
    ///
    /// **키가 이 함수 밖으로 나가는 자리는 헤더 하나뿐이다.** 없으면 §13의 `provider 미설정`
    /// 이다 — 재시도 대상이 아니고, 사용자가 설정에서 키를 넣어야 풀린다.
    fn api_key(&self) -> Result<String, Failure> {
        match self.secrets.get(SecretKey::AnthropicApiKey) {
            Ok(Some(secret)) if !secret.expose().trim().is_empty() => {
                Ok(secret.expose().trim().to_owned())
            }
            Ok(_) => Err(not_configured("Claude API 키가 아직 저장되지 않았다")),
            Err(failure) => Err(failure),
        }
    }

    fn model_name(&self) -> &str {
        if self.model.trim().is_empty() {
            wire::DEFAULT_MODEL
        } else {
            self.model.trim()
        }
    }
}

impl NoteAiProvider for AnthropicProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: PROVIDER_ID.to_owned(),
            name: PROVIDER_NAME.to_owned(),
            // **전사 텍스트가 기기 밖으로 나간다.** 화면은 이 값을 읽어 사용자에게 알린다
            // (§12 · INV-5). 이것이 로컬 adapter와 갈리는 자리다.
            locality: Locality::External,
        }
    }

    /// **왕복하지 않는다.**
    ///
    /// 로컬 adapter는 서버에 설치된 모델 목록을 물어볼 수 있지만, 여기서 "쓸 수 있는가"를
    /// 물으려면 **돈이 나가는 요청**을 보내야 한다. 준비 여부를 확인하는 것만으로 과금하지
    /// 않는다 — 키가 있으면 준비된 것으로 보고, 키가 틀렸다는 사실은 실제 생성에서 드러난다.
    fn availability(&self) -> Availability {
        match self.api_key() {
            Ok(_) => Availability::Ready {
                models: vec![self.model_name().to_owned()],
            },
            Err(failure) => Availability::Unavailable(failure),
        }
    }


    /// **1M 컨텍스트다.** 2시간 회의의 전사가 약 30K 토큰이므로 통째로 들어간다.
    ///
    /// **[문서 근거 · 미실측]** 값의 출처는 `docs/PRODUCT-SPEC.md` §16.1이 적어 둔
    /// 2026-09-01 기준 모델 표다. 이 앱이 1M을 실제로 채워 본 적은 없다.
    fn context_budget(&self) -> ContextBudget {
        ContextBudget { context_tokens: 1_000_000 }
    }

    fn generate_note(&self, request: &NoteRequest<'_>) -> Result<NoteGeneration, Failure> {
        let key = self.api_key()?;
        let model = self.model_name().to_owned();

        // 프롬프트는 domain이 만든다 — 이 adapter는 그 문자열을 어느 필드에 싣는지만 안다.
        let prompt = build_prompt(request.mode, request.transcript_text());
        let body = wire::request_body(&prompt, &model);

        // **두 방식을 다 받는다** (2026-09-08). API 키와 OAuth access token은 서로 다른
        // 헤더로 가며, 어느 쪽인지는 값의 모양이 말한다 — 사용자에게 묻지 않는다.
        let bearer;
        let headers: Vec<HttpHeader<'_>> = match wire::credential_kind(&key) {
            wire::Credential::ApiKey => vec![
                ("x-api-key", key.as_str()),
                ("anthropic-version", wire::API_VERSION),
                ("content-type", "application/json"),
            ],
            wire::Credential::OauthToken => {
                bearer = format!("Bearer {key}");
                vec![
                    ("authorization", bearer.as_str()),
                    (wire::OAUTH_BETA_HEADER, wire::OAUTH_BETA_VALUE),
                    ("anthropic-version", wire::API_VERSION),
                    ("content-type", "application/json"),
                ]
            }
        };

        let response = self
            .transport
            .send(&HttpRequest::post_json(wire::ENDPOINT, &headers, &body))
            .map_err(transport_failure)?;

        if !response.is_success() {
            return Err(status_failure(&response));
        }

        let text = wire::text_from_response(&response.body)?;
        let note = crate::ai::parse_note(request.mode, &text)
            .map_err(|rejection: ResponseRejection| rejected_response(&rejection))?;

        Ok(NoteGeneration {
            note,
            // **응답이 말한 모델을 적는다.** provenance는 실제로 쓴 것이어야 한다.
            model: wire::model_from_response(&response.body, &model),
        })
    }
}

/// 왕복 자체가 실패했다.
fn transport_failure(error: TransportError) -> Failure {
    match error {
        TransportError::NotConnected => unreachable("Claude API에 연결하지 못했다"),
        TransportError::TimedOut => request_failed_temporarily("Claude API가 제때 응답하지 않았다"),
        TransportError::Incomplete => {
            request_failed_temporarily("Claude API와의 요청을 끝내지 못했다")
        }
    }
    .with_detail(error)
}

/// 응답은 왔지만 2xx가 아니다.
///
/// **네 가지를 나눈다** — 사용자가 할 일이 다르기 때문이다.
///
/// ```text
/// 401 · 403   키가 틀렸거나 권한이 없다   → 미설정. 다시 눌러도 같다
/// 404         모델 이름이 없다            → 모델 없음. 다른 모델을 골라야 한다
/// 429         한도에 걸렸다               → 재시도 가능. 잠시 뒤 같은 버튼이 성공한다
/// 그 밖의 4xx  우리가 보낸 요청이 잘못됐다  → 다시 보내도 같다
/// 5xx         provider 쪽 사정            → 재시도 가능
/// ```
///
/// **본문을 통째로 옮기지 않는다.** 오류 본문에 요청 내용이 섞여 오는 경우가 있고, 그
/// 요청에는 전사가 들어 있다. 그래서 [`wire::error_detail`]이 꺼내 주는 두 가지 —
/// 오류 종류와 한 문장 — 만 싣고, 그 문장도 길이에서 자른다.
///
/// **status 숫자만으로는 400을 진단할 수 없다.** 2026-09-09에 그 자리에서 막혔고,
/// 그래서 API가 말한 이유가 여기까지 오게 됐다.
fn status_failure(response: &HttpResponse) -> Failure {
    match response.status {
        401 | 403 => not_configured("Claude API 키가 받아들여지지 않았다"),
        404 => model_unavailable("고른 모델을 Claude API에서 찾을 수 없다"),
        429 => request_failed_temporarily("Claude API 사용 한도에 걸렸다"),
        // **새 FailureKind를 만들지 않는다** — 이미 있는 갈래로 충분하다.
        status if (400..500).contains(&status) => request_rejected("Claude API가 요청을 거절했다"),
        _ => request_failed_temporarily("Claude API가 요청을 처리하지 못했다"),
    }
    .with_detail(match wire::error_detail(&response.body) {
        Some(said) => format!("status={} · {said}", response.status),
        None => format!("status={}", response.status),
    })
}
