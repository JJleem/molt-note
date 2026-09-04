//! [`crate::ai::NoteAiProvider`]의 첫 실제 구현 — 사용자가 실행 중인 로컬 AI 서버.
//!
//! ```text
//! availability()    GET  /api/tags      ─→ Ready{models} · NoModels · Unavailable(공통 실패)
//! generate_note()   GET  /api/tags      ─→ 고른 모델이 목록에 있는가 (§13.3)
//!                   POST /api/generate  ─→ 본문 → parse_note → StructuredNote
//! ```
//!
//! **벤더 오류가 이 파일 밖으로 나가지 않는다** (INV-9). 나가는 것은 §13의 공통 실패
//! 다섯 중 하나뿐이며, 그 번역이 [`Self::availability`]와 [`Self::generate_note`]의 마지막 일이다.
//!
//! ## 연결 대상은 주입된다
//!
//! `base_url`도 `model`도 **생성자 인자**다. 이 파일에 host도 port도 기본 주소도 없다 —
//! 기본값을 아는 자리는 [`crate::domain::settings::DEFAULT_AI_BASE_URL`] 하나이고, 그 값을
//! 읽어 넘기는 것은 부르는 쪽의 일이다 (ADR-0008 §11.1).
//!
//! ## 설정값은 실패 문장에 남지 않는다 (ADR-0008 §11.3)
//!
//! 이 파일이 만드는 어떤 [`Failure`]에도 host · port · 모델 이름이 들어가지 않는다.
//! `message`는 사용자가 읽을 문장이고, `detail`은 **고정 문자열이거나 상태 코드**다.
//! 그 성질은 관례가 아니라 타입이 지탱한다: [`TransportError`]에는 문자열 자리가 없고
//! ([`super::http`]), [`BodyRejection`]도 마찬가지다 ([`super::wire`]).

use std::sync::Arc;

use crate::ai::note::ResponseRejection;
use crate::ai::prompt::build_prompt;
use crate::ai::provider::{
    model_unavailable, rejected_response, request_failed_temporarily, request_rejected,
    response_unusable, unreachable, Availability, Locality, NoteAiProvider, NoteGeneration,
    NoteRequest, ProviderDescriptor,
};
use crate::domain::Failure;

use super::http::{HttpRequest, HttpResponse, HttpTransport, TransportError};
use super::wire::{self, BodyRejection};

/// `ai_notes.provider`에 그대로 남는 식별자 (ADR-0008 §4.1).
///
/// domain은 이 값을 알려진 목록과 대조하지 않는다 — 자유 문자열 식별자다 (INV-9).
pub const PROVIDER_ID: &str = "ollama";

/// 화면이 그리는 이름. **설정값이 아니다** — 어디에 연결하는지는 여기 담기지 않는다.
const PROVIDER_NAME: &str = "Ollama (로컬)";

/// 로컬 AI 서버에 REST로 직접 말하는 provider.
///
/// 실제 왕복은 [`HttpTransport`] 뒤에 있다. 그래서 자동 검증은 double 하나로 이 파일의 전
/// 경로를 지나가며, **서버도 모델도 요구하지 않는다** (ADR-0008 §18).
pub struct OllamaProvider {
    /// 연결 대상. 설정에서 온 값이며 이 파일이 기본값을 갖지 않는다.
    base_url: String,
    /// 사용자가 고른 모델 식별자.
    model: String,
    transport: Arc<dyn HttpTransport>,
}

impl OllamaProvider {
    /// 연결 대상과 모델을 받아 provider 하나를 만든다.
    ///
    /// 값의 유효성을 여기서 묻지 않는다 — 서버가 응답하는지도, 그 모델이 설치돼 있는지도
    /// 물어봐야 알 수 있고, 그 답은 [`Self::availability`]가 낸다 (INV-8).
    pub fn new(
        base_url: impl Into<String>,
        model: impl Into<String>,
        transport: Arc<dyn HttpTransport>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            transport,
        }
    }

    /// 설치된 모델 목록을 서버에게 묻는다 (ADR-0008 §6.4).
    ///
    /// 헬스 체크를 따로 부르지 않는다 — 이 한 번으로 "응답하는가"와 "모델이 있는가"가 함께
    /// 답해진다.
    fn installed_models(&self) -> Result<Vec<String>, Failure> {
        let url = wire::endpoint(&self.base_url, wire::TAGS_PATH);
        let response = self.send(&HttpRequest::get(&url))?;

        wire::installed_models(&response.body).map_err(unreadable_body)
    }

    /// 한 번의 왕복. 실패와 비2xx를 §13의 공통 실패로 옮기는 자리다.
    fn send(&self, request: &HttpRequest<'_>) -> Result<HttpResponse, Failure> {
        let response = self.transport.send(request).map_err(transport_failure)?;

        if response.is_success() {
            Ok(response)
        } else {
            Err(status_failure(&response))
        }
    }
}

impl NoteAiProvider for OllamaProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: PROVIDER_ID.to_owned(),
            name: PROVIDER_NAME.to_owned(),
            // 사용자의 기기에서 도는 서버다. 전사 텍스트는 이 기기를 떠나지 않는다
            // (§12 · INV-5). 이 값이 `External`이 되는 것은 다른 adapter의 일이다.
            locality: Locality::Local,
        }
    }

    fn availability(&self) -> Availability {
        match self.installed_models() {
            Ok(models) if models.is_empty() => Availability::NoModels,
            Ok(models) => Availability::Ready { models },
            Err(failure) => Availability::Unavailable(failure),
        }
    }

    fn generate_note(&self, request: &NoteRequest<'_>) -> Result<NoteGeneration, Failure> {
        // **모델 존재 여부를 생성 응답의 상태 코드로 판정하지 않는다** (ADR-0008 §13.3).
        // 모델이 없을 때 서버가 무엇을 돌려주는지는 UNVERIFIED이고, 그것에 의존하지 않는
        // 판정이 목록 하나로 끝난다. 왕복이 한 번 늘지만 분류가 문서에 걸려 있지 않게 된다.
        let installed = self.installed_models()?;
        if !installed.iter().any(|model| model == &self.model) {
            // **고른 모델 이름을 문장에 넣지 않는다** (§11.3). 사용자는 자기가 무엇을 골랐는지
            // 설정 화면에서 본다.
            return Err(model_unavailable(if installed.is_empty() {
                "AI 서버에 설치된 모델이 없다"
            } else {
                "고른 모델이 AI 서버에 없다"
            }));
        }

        // 프롬프트는 domain이 만든다 — 이 adapter는 그 문자열을 어느 필드에 싣는지만 안다.
        //
        // **보내기 전 크기 판정(`prompt::prepare`)은 여기서 하지 않는다.** 그 실패는 아직
        // 만들어지지 않았고(§13.1의 여섯 번째), 만들지 않은 실패의 자리를 adapter가 임의의
        // 종류로 대신 채우면 사용자는 "설정에서 값을 키우면 된다"를 알 수 없게 된다
        // (ADR-0008 §13.1 · `crate::ai` 모듈 주석 · §20.6).
        let prompt = build_prompt(request.mode, request.transcript_text());
        let body = wire::generate_body(
            &self.model,
            &prompt,
            request.mode,
            request.context_budget.context_tokens,
        );

        let url = wire::endpoint(&self.base_url, wire::GENERATE_PATH);
        let response = self.send(&HttpRequest::post_json(&url, &body))?;

        let note = wire::generated_note(request.mode, &response.body)
            .map_err(|rejection: ResponseRejection| rejected_response(&rejection))?;

        Ok(NoteGeneration {
            note,
            // 응답이 자기가 쓴 모델을 어느 이름으로 말하는지는 UNVERIFIED다 (§14.3). 확인되지
            // 않은 이름에서 읽어 오는 대신 **이 요청이 지정한 모델**을 남긴다 — 서버는 지정된
            // 모델로 답했고, 지정한 것은 이 adapter다. 추정이 아니라 우리가 아는 사실이다.
            model: self.model.clone(),
        })
    }
}

/// 왕복 자체가 실패했다 → 연결 불가 · 요청 실패 (ADR-0008 §13.1 · §13.3).
fn transport_failure(error: TransportError) -> Failure {
    match error {
        // 서버가 실행 중이 아닐 때의 모습이다. **재시도가 의미를 갖는다** — 사용자가 서버를
        // 켜고 같은 버튼을 다시 누르면 성공한다.
        TransportError::NotConnected => unreachable("로컬 AI 서버에 연결하지 못했다"),
        // 타임아웃은 §13.3이 재시도 가능으로 정한 자리다.
        TransportError::TimedOut => request_failed_temporarily("AI 서버가 제때 응답하지 않았다"),
        TransportError::Incomplete => request_failed_temporarily("AI 서버와의 요청을 끝내지 못했다"),
    }
    .with_detail(error)
}

/// 응답은 왔지만 2xx가 아니다 → 요청 실패 (ADR-0008 §13.3).
///
/// 4xx는 **우리가 보낸 요청이 잘못됐다**는 뜻이므로 같은 요청을 다시 보내도 같다. 그 밖의
/// 비2xx(5xx 등)는 서버 쪽 사정이므로 다시 시도해 볼 값이 있다.
fn status_failure(response: &HttpResponse) -> Failure {
    let failure = if response.is_client_error() {
        request_rejected("AI 서버가 요청을 거절했다")
    } else {
        request_failed_temporarily("AI 서버가 요청을 처리하지 못했다")
    };

    // **본문을 detail에 담지 않는다.** 무엇이 들어 있을지 모르고, 그 안에 설정값이 그대로
    // 들어 있을 수 있다 (§11.3). 상태 코드는 설정값이 아니다.
    failure.with_detail(format!("HTTP {}", response.status))
}

/// 응답 본문을 이 adapter가 기대한 모양으로 읽지 못했다 → 응답 schema 불일치.
fn unreadable_body(rejection: BodyRejection) -> Failure {
    response_unusable("AI 서버의 응답을 읽을 수 없다").with_detail(rejection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::testing::{sample_note_text, CONTRACT_TRANSCRIPT};
    use crate::domain::{FailureKind, NoteType};

    use super::super::http::HttpMethod;
    use super::super::testing::{StubReply, StubServer, MODEL_IN_THE_LIST};

    /// 테스트가 쓰는 연결 대상. **실제 주소가 아니다** — `.invalid`는 어떤 이름 해석도
    /// 성공하지 않도록 예약된 TLD이며, 그래서 이 테스트는 소켓을 열 수 없다.
    const CONFIGURED_BASE_URL: &str = "http://configured-host.invalid:65535";

    fn provider(server: StubServer) -> OllamaProvider {
        OllamaProvider::new(CONFIGURED_BASE_URL, MODEL_IN_THE_LIST, Arc::new(server))
    }

    #[test]
    fn the_provider_says_it_keeps_the_data_on_this_device() {
        let descriptor = provider(StubServer::ready()).descriptor();

        assert_eq!(descriptor.id, PROVIDER_ID);
        assert!(descriptor.locality.is_local(), "로컬 서버다 (§12 · INV-5)");
        assert!(!descriptor.name.trim().is_empty());
        // 이름에 연결 대상이 섞이지 않는다.
        assert!(!descriptor.name.contains("configured-host"));
    }

    #[test]
    fn availability_asks_the_running_server_rather_than_the_documentation() {
        let server = StubServer::ready().with_models(vec!["first".into(), "second".into()]);
        let provider = provider(server);

        assert_eq!(
            provider.availability(),
            Availability::Ready {
                models: vec!["first".to_owned(), "second".to_owned()]
            }
        );
    }

    #[test]
    fn the_request_goes_to_the_injected_address() {
        let server = Arc::new(StubServer::ready());
        let provider = OllamaProvider::new(CONFIGURED_BASE_URL, MODEL_IN_THE_LIST, server.clone());

        provider.availability();

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, format!("{CONFIGURED_BASE_URL}/api/tags"));
        assert_eq!(requests[0].method, HttpMethod::Get);
    }

    #[test]
    fn generation_sends_the_prompt_the_domain_built_with_the_context_size_stated() {
        let server = Arc::new(StubServer::ready());
        let provider = OllamaProvider::new(CONFIGURED_BASE_URL, MODEL_IN_THE_LIST, server.clone());
        let request = NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT);

        provider.generate_note(&request).expect("표본 노트를 낸다");

        let requests = server.requests();
        assert_eq!(requests.len(), 2, "목록으로 모델을 먼저 판정한다 (§13.3)");
        assert_eq!(requests[1].url, format!("{CONFIGURED_BASE_URL}/api/generate"));

        let body: serde_json::Value =
            serde_json::from_str(requests[1].body.as_deref().expect("본문이 있다"))
                .expect("본문은 JSON이다");
        assert_eq!(body["model"], MODEL_IN_THE_LIST);
        assert_eq!(body["stream"], false);
        assert_eq!(
            body["options"]["num_ctx"],
            crate::ai::prompt::ContextBudget::DEFAULT.context_tokens
        );
        assert!(
            body["prompt"]
                .as_str()
                .expect("프롬프트는 문자열이다")
                .contains(CONTRACT_TRANSCRIPT),
            "전사 텍스트가 프롬프트에 실린다"
        );
    }

    #[test]
    fn a_generated_note_records_the_model_that_was_asked_for() {
        let generation = provider(StubServer::ready())
            .generate_note(&NoteRequest::new(NoteType::Study, CONTRACT_TRANSCRIPT))
            .expect("표본 노트를 낸다");

        assert_eq!(generation.note.mode(), NoteType::Study);
        assert_eq!(generation.model, MODEL_IN_THE_LIST);
    }

    #[test]
    fn a_refused_connection_is_the_unreachable_failure() {
        let provider = provider(StubServer::refusing());

        let Availability::Unavailable(reported) = provider.availability() else {
            panic!("연결되지 않으면 Unavailable이다");
        };
        assert_eq!(reported.kind, FailureKind::AiProviderUnreachable);
        assert!(reported.retryable, "서버를 켜고 다시 시도할 수 있다");

        let failure = provider
            .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
            .expect_err("연결되지 않으면 생성도 실패한다");
        assert_eq!(failure.kind, FailureKind::AiProviderUnreachable);
    }

    #[test]
    fn a_non_success_status_is_the_request_failure_and_its_retryability_follows_the_cause() {
        // 4xx — 같은 요청을 다시 보내도 같다.
        let rejected = provider(StubServer::ready().with_generate(StubReply::Status(400)))
            .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
            .expect_err("비2xx는 실패다");
        assert_eq!(rejected.kind, FailureKind::AiRequestFailed);
        assert!(!rejected.retryable);
        assert_eq!(rejected.detail.as_deref(), Some("HTTP 400"));

        // 5xx — 서버 쪽 사정이므로 다시 시도해 볼 값이 있다.
        let temporary = provider(StubServer::ready().with_generate(StubReply::Status(503)))
            .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
            .expect_err("비2xx는 실패다");
        assert_eq!(temporary.kind, FailureKind::AiRequestFailed);
        assert!(temporary.retryable);
    }

    #[test]
    fn a_timeout_is_a_retryable_request_failure_not_an_unreachable_server() {
        let failure = provider(StubServer::failing(TransportError::TimedOut))
            .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
            .expect_err("타임아웃은 실패다");

        assert_eq!(failure.kind, FailureKind::AiRequestFailed);
        assert!(failure.retryable, "§13.3: 타임아웃은 재시도 가능이다");
    }

    #[test]
    fn a_model_that_is_not_installed_is_a_different_failure_from_a_silent_server() {
        let server = StubServer::ready().with_models(vec!["another-model".into()]);
        let failure = provider(server)
            .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
            .expect_err("고른 모델이 없으면 생성할 수 없다");

        assert_eq!(failure.kind, FailureKind::AiModelUnavailable);
        assert!(!failure.retryable, "모델을 받아야 풀린다");
    }

    #[test]
    fn a_server_with_no_models_answers_but_cannot_generate() {
        let server = StubServer::ready().with_models(vec![]);
        let provider = provider(server);

        assert_eq!(provider.availability(), Availability::NoModels);
        assert_eq!(
            provider
                .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
                .expect_err("모델이 없으면 생성할 수 없다")
                .kind,
            FailureKind::AiModelUnavailable
        );
    }

    #[test]
    fn a_response_that_is_not_the_expected_note_is_the_schema_failure() {
        for text in [
            "죄송하지만 JSON으로 답할 수 없습니다",
            "{}",
            r#"{"overview":"   ","keyDiscussions":[],"decisions":[],"actionItems":[],"openQuestions":[]}"#,
        ] {
            let server = StubServer::ready().with_generate(StubReply::GeneratedText(text.into()));
            let failure = provider(server)
                .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
                .expect_err("기대 schema와 다른 응답은 실패다");

            assert_eq!(failure.kind, FailureKind::AiResponseUnusable);
            assert!(failure.retryable, "생성은 결정론적이지 않다");
            assert!(failure.source_data_safe, "원본은 건드리지 않았다 (INV-3)");
            assert!(failure.detail.is_some(), "무엇이 달랐는지가 남는다");
        }
    }

    #[test]
    fn a_body_that_is_not_the_expected_shape_is_also_the_schema_failure() {
        let server = StubServer::ready().with_tags(StubReply::Body("Ollama is running".into()));
        let failure = provider(server)
            .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
            .expect_err("목록을 읽지 못하면 생성할 수 없다");

        assert_eq!(failure.kind, FailureKind::AiResponseUnusable);
    }

    #[test]
    fn a_valid_note_survives_the_whole_path_for_every_mode() {
        for mode in NoteType::ALL {
            let server = StubServer::ready();
            let generation = provider(server)
                .generate_note(&NoteRequest::new(mode, CONTRACT_TRANSCRIPT))
                .expect("표본 노트를 낸다");

            assert_eq!(generation.note.mode(), mode);
            assert_eq!(
                crate::ai::encode_content(&generation.note),
                crate::ai::encode_content(
                    &crate::ai::parse_note(mode, &sample_note_text(mode)).expect("표본은 유효하다")
                )
            );
        }
    }
}
