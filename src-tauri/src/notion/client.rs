//! Notion과 실제로 말하는 자리 — 요청 조립 · 응답 해석 · **벤더 실패 → §13의 제품 상태**
//! (ADR-0009 §5 · §9 · PRODUCT-SPEC §13).
//!
//! ```text
//! check_connection()  GET   /v1/users/me                    ─→ 토큰이 유효한가
//! create_page()       POST  /v1/pages                       ─→ 만들어진 PageId
//! append_markdown()   PATCH /v1/pages/<page_id>/markdown    ─→ 끝에 이어 붙였다
//! ```
//!
//! **벤더 오류가 이 디렉터리 밖으로 나가지 않는다.** 나가는 것은 §13의 실패 다섯 중 하나와,
//! rate limit일 때 **얼마나 기다리면 되는지**([`RetryAfter`])뿐이다.
//!
//! ## 기다리라는 지시는 값이지 잠이 아니다
//!
//! `429`·`529`를 만나도 **이 파일은 자지 않는다.** 서버가 말한 대기 시간을 값으로 돌려줄
//! 뿐이며, 얼마나 기다릴지 · 몇 번까지 다시 보낼지 · 그 사이 상태를 어떻게 적을지는
//! 전송 순서를 소유한 쪽이 정한다 (ADR-0009 §9.2의 3·4·5번은 이 adapter의 결정이 아니다).
//! 그래서 이 경로는 시간을 쓰지 않고 검증된다.
//!
//! ## 어떤 실패도 token과 destination을 옮기지 않는다 (INV-7 · ADR-0009 §10.4)
//!
//! 이 파일이 만드는 어떤 [`Failure`]에도 token · 부모 페이지 id · page id · 서버가 되돌려준
//! 문장이 들어가지 않는다. `message`는 사용자가 읽을 문장이고, `detail`은 **상태 코드와 벤더의
//! 고정 오류 코드**다. 그 성질은 관례가 아니라 타입이 지탱한다 — [`TransportError`]와
//! [`BodyRejection`]에는 문자열 자리가 없고, [`PageId`]와 [`AuthorizationValue`]는 `Debug`로
//! 내용을 내지 않는다.

use std::fmt;
use std::sync::Arc;

use crate::domain::{Failure, FailureKind};
use crate::platform::secret_store::Secret;

use super::http::{HttpHeader, HttpRequest, HttpResponse, HttpTransport, TransportError};
use super::wire::{
    self, ApiErrorCode, AuthorizationValue, BodyRejection, ConnectedIdentity, PageId, API_BASE_URL,
    AUTHORIZATION_HEADER, CONTENT_TYPE_HEADER, JSON_CONTENT_TYPE, NOTION_VERSION,
    NOTION_VERSION_HEADER,
};

/// 이 adapter가 만들 수 있는 §13의 실패 전부.
///
/// **다섯으로 나눈 이유는 사용자가 할 수 있는 일이 전부 다르기 때문이다** — token을 다시
/// 넣는 것 · 부모 페이지를 integration에 공유하는 것 · 잠시 기다리는 것 · 다시 시도하는 것 ·
/// Notion에서 무엇이 만들어졌는지 확인하는 것. 하나로 뭉치면 화면이 그 다섯을 구분해 안내할 수
/// 없다 (전사 실패 넷 · AI 실패 다섯이 나뉜 것과 같은 규칙이다).
pub const NOTION_FAILURE_KINDS: [FailureKind; 5] = [
    FailureKind::NotionAuthFailed,
    FailureKind::NotionDestinationUnavailable,
    FailureKind::NotionRateLimited,
    FailureKind::NotionRequestFailed,
    FailureKind::NotionResponseUnusable,
];

/// 서버가 지시한 대기 — `429`·`529`에서만 만들어진다 (ADR-0009 §9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAfter {
    /// `Retry-After`를 **정수 초**로 읽었다 (PRODUCT-SPEC §14.9.1의 문서 인용).
    Seconds(u32),
    /// 속도를 늦추라고 했지만 얼마나인지는 말하지 않았다 — 헤더가 없거나 정수로 읽히지 않았다.
    ///
    /// **얼마나 기다릴지를 이 adapter가 지어내지 않는다.** 그 값은 전송 순서를 소유한 쪽이
    /// 자기 backoff로 정하며, 그 사실이 값에 그대로 남는다 (ADR-0009 §9.2-5).
    Unspecified,
}

impl RetryAfter {
    /// 서버가 말한 초. 말하지 않았으면 `None`이다.
    pub fn seconds(self) -> Option<u32> {
        match self {
            Self::Seconds(seconds) => Some(seconds),
            Self::Unspecified => None,
        }
    }
}

/// 이 adapter가 낸 실패 하나.
///
/// 안에 든 [`Failure`]는 **그대로 화면에 도달하는 §13의 값**이고, [`Self::wait`]는 rate limit일
/// 때만 있는 **다시 보내도 되는 시점**이다. 둘을 한 값에 담는 이유는 하나다 — `Failure`에
/// 대기 시간을 담을 자리를 만들면 그것은 이 벤더 하나 때문에 domain 계약이 넓어지는 일이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotionFailure {
    failure: Failure,
    wait: Option<RetryAfter>,
}

impl NotionFailure {
    fn new(failure: Failure) -> Self {
        Self {
            failure,
            wait: None,
        }
    }

    fn waiting(failure: Failure, wait: RetryAfter) -> Self {
        Self {
            failure,
            wait: Some(wait),
        }
    }

    /// 사용자에게 보일 실패.
    pub fn failure(&self) -> &Failure {
        &self.failure
    }

    /// 사용자에게 보일 실패를 꺼낸다.
    pub fn into_failure(self) -> Failure {
        self.failure
    }

    pub fn kind(&self) -> FailureKind {
        self.failure.kind
    }

    /// **언제 다시 보내면 되는가.** rate limit이 아니면 `None`이며, 그때 자동 재시도는 없다
    /// (ADR-0009 §9.3 — `429`·`529` 밖의 응답은 다시 보내도 같거나, 언제 풀리는지 알 수 없다).
    pub fn wait(&self) -> Option<RetryAfter> {
        self.wait
    }
}

impl fmt::Display for NotionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.failure, f)
    }
}

impl From<NotionFailure> for Failure {
    fn from(failure: NotionFailure) -> Self {
        failure.into_failure()
    }
}

/// Notion API에 REST로 직접 말하는 client.
///
/// 실제 왕복은 [`HttpTransport`] 뒤에 있다. 그래서 자동 검증은 double 하나로 이 파일의 전
/// 경로를 지나가며 **실제 Notion 워크스페이스를 요구하지도 오염시키지도 않는다**
/// (PRODUCT-SPEC §18 · `phase-prompt/05` Important Rules).
///
/// **연결 대상을 주입하지 않는다.** AI provider와 갈리는 지점이다 — 로컬 AI 서버의 주소는
/// 사용자가 정하지만 Notion의 주소는 벤더가 정한다. 그 값은 [`wire::API_BASE_URL`] 하나다.
pub struct NotionClient {
    transport: Arc<dyn HttpTransport>,
}

impl NotionClient {
    pub fn new(transport: Arc<dyn HttpTransport>) -> Self {
        Self { transport }
    }

    /// 토큰이 유효한지 — 그 토큰에 딸린 사용자를 물어본다 (§14.9.1의 연결 확인 수단).
    ///
    /// **"말할 수 있는가"의 답은 상태 코드가 한다.** 본문에서 읽는 것은 하나뿐이다 — 이 토큰이
    /// **어느 워크스페이스의 것인가** ([`wire::connected_identity`]). 그 값은 사용자가 자기
    /// 워크스페이스를 알아보기 위해 설정 화면에 그대로 보인다 (§5-D). 그 밖의 사용자 정보는
    /// 꺼내지 않는다 — 쓰지 않는 값을 꺼내 두면 그것이 로그와 화면으로 새어 나갈 자리가 된다.
    ///
    /// 본문을 읽지 못한 것은 **실패가 아니다.** 그때는 이름을 지어내지 않고 모르는 채로 남는다.
    pub fn check_connection(&self, token: &Secret) -> Result<ConnectedIdentity, NotionFailure> {
        let authorization = AuthorizationValue::new(token);
        let headers = headers_without_body(&authorization);
        let url = wire::url(API_BASE_URL, wire::USERS_ME_PATH);

        self.send(&HttpRequest::get(&url, &headers))
            .map(|response| wire::connected_identity(&response.body))
    }

    /// 부모 페이지 아래에 markdown 문서로 페이지 하나를 만든다 (§14.9.1 · ADR-0009 §5.1).
    ///
    /// 제목은 markdown 첫 `# h1`이 된다 — `properties`를 보내지 않기 때문이다 (§5.3).
    /// 성공하면 **만들어진 페이지의 식별자**를 돌려준다. 그 값을 곧바로 영속화하는 것은 부르는
    /// 쪽의 일이며, 그래야 부분 성공 뒤의 재시도가 중복 페이지를 만들지 않는다 (§8.4-3).
    pub fn create_page(
        &self,
        token: &Secret,
        parent_page_id: &str,
        markdown: &str,
    ) -> Result<PageId, NotionFailure> {
        let authorization = AuthorizationValue::new(token);
        let headers = headers_with_body(&authorization);
        let url = wire::url(API_BASE_URL, wire::PAGES_PATH);
        let body = wire::create_page_body(parent_page_id, markdown);

        let response = self.send(&HttpRequest::post_json(&url, &headers, &body))?;

        // 2xx라고 해서 페이지가 만들어졌다고 단정하지 않는다 — 요청하지 않은 `202`가 그 자리다
        // (ADR-0009 §7.3). 페이지 id를 읽지 못하면 그 사실을 그대로 실패로 남긴다.
        wire::created_page_id(&response.body).map_err(unreadable_body)
    }

    /// 이미 있는 페이지의 **끝에** markdown을 이어 붙인다 (§14.9.1 · ADR-0009 §5.1).
    ///
    /// 쓰는 것은 `insert_content` + `position: { type: "end" }` 하나다. 페이지의 기존 내용을
    /// 바꾸거나 지우는 경로는 이 adapter에 없다 (§8.3).
    pub fn append_markdown(
        &self,
        token: &Secret,
        page: &PageId,
        markdown: &str,
    ) -> Result<(), NotionFailure> {
        let authorization = AuthorizationValue::new(token);
        let headers = headers_with_body(&authorization);
        let url = wire::page_markdown_url(API_BASE_URL, page);
        let body = wire::append_markdown_body(markdown);

        self.send(&HttpRequest::patch_json(&url, &headers, &body))
            .map(|_| ())
    }

    /// 한 번의 왕복. 실패와 비2xx를 §13의 제품 상태로 옮기는 자리다.
    fn send(&self, request: &HttpRequest<'_>) -> Result<HttpResponse, NotionFailure> {
        let response = self.transport.send(request).map_err(transport_failure)?;

        if response.is_success() {
            Ok(response)
        } else {
            Err(status_failure(&response))
        }
    }
}

/// 본문이 없는 요청의 헤더 (ADR-0009 §5.2 — `Content-Type`은 본문이 있는 요청에만).
fn headers_without_body(authorization: &AuthorizationValue) -> [HttpHeader<'_>; 2] {
    [
        (AUTHORIZATION_HEADER, authorization.expose()),
        (NOTION_VERSION_HEADER, NOTION_VERSION),
    ]
}

/// 본문이 있는 요청의 헤더.
fn headers_with_body(authorization: &AuthorizationValue) -> [HttpHeader<'_>; 3] {
    [
        (AUTHORIZATION_HEADER, authorization.expose()),
        (NOTION_VERSION_HEADER, NOTION_VERSION),
        (CONTENT_TYPE_HEADER, JSON_CONTENT_TYPE),
    ]
}

/// 왕복 자체가 실패했다 → 요청 실패 (§13 · ADR-0009 §9.3의 마지막 줄).
///
/// 셋 다 같은 종류인 이유: **사용자가 할 수 있는 일이 같다** — 연결을 확인하고 다시 누르는
/// 것이다. 그 이상은 이 앱이 알 수 없다.
fn transport_failure(error: TransportError) -> NotionFailure {
    let failure = match error {
        TransportError::NotConnected => {
            request_failed_temporarily("Notion에 연결하지 못했다. 네트워크 연결을 확인한 뒤 다시 시도할 수 있다.")
        }
        TransportError::TimedOut => {
            request_failed_temporarily("Notion이 제때 응답하지 않았다. 다시 시도할 수 있다.")
        }
        TransportError::Incomplete => {
            request_failed_temporarily("Notion과의 요청을 끝내지 못했다. 다시 시도할 수 있다.")
        }
    };

    NotionFailure::new(failure.with_detail(error))
}

/// 응답은 왔지만 2xx가 아니다 → §13의 제품 상태 (ADR-0009 §9.3).
///
/// **판정은 벤더의 오류 코드를 먼저 본다.** 상태 코드만으로는 갈리지 않는 자리가 있기
/// 때문이다 — 같은 `404`라도 `object_not_found`는 "그 페이지에 닿을 수 없다"이고, 코드가 없는
/// `404`는 우리가 주소를 잘못 만들었다는 뜻일 수 있다. 코드가 없거나 모르는 코드면 상태 코드가
/// 판정한다 (`529`처럼 JSON 본문이 온다는 계약이 없는 응답이 그 자리다).
fn status_failure(response: &HttpResponse) -> NotionFailure {
    let code = ApiErrorCode::from_body(&response.body);
    let detail = match code {
        Some(code) => format!("HTTP {} {code}", response.status),
        None => format!("HTTP {}", response.status),
    };

    // rate limit은 **언제 다시 보내면 되는지**를 함께 답한다 (ADR-0009 §9.2).
    if matches!(code, Some(ApiErrorCode::RateLimited)) || matches!(response.status, 429 | 529) {
        let wait = match response.retry_after_seconds {
            Some(seconds) => RetryAfter::Seconds(seconds),
            None => RetryAfter::Unspecified,
        };
        return NotionFailure::waiting(
            rate_limited("Notion이 요청 속도를 제한하고 있다. 잠시 뒤에 다시 보낸다.")
                .with_detail(detail),
            wait,
        );
    }

    let failure = match (code, response.status) {
        // token의 문제다. 다시 보내도 같고, 설정에서 값을 바꿔야 풀린다.
        (Some(ApiErrorCode::Unauthorized), _) | (None, 401) => auth_failed(
            "Notion이 연결을 거절했다. 설정에서 integration token을 다시 입력해야 한다.",
        ),
        // 보낼 자리의 문제다. 부모 페이지가 없거나 integration에 공유되지 않았다.
        (Some(ApiErrorCode::RestrictedResource | ApiErrorCode::ObjectNotFound), _)
        | (None, 403 | 404) => destination_unavailable(
            "Notion에서 보낼 위치를 찾지 못했다. 설정한 부모 페이지를 integration에 공유했는지 확인해야 한다.",
        ),
        // 우리가 보낸 요청이 잘못됐다. 같은 요청은 같은 답이다.
        (Some(ApiErrorCode::ValidationError), _) => {
            request_rejected("Notion이 요청을 받아들이지 않았다.")
        }
        // 지금은 안 되지만 사용자가 다시 눌러 볼 값이 있다 (ADR-0009 §9.3 — 자동 재시도는 없다).
        (Some(ApiErrorCode::ConflictError), _) | (None, 409) => request_failed_temporarily(
            "Notion에서 같은 페이지에 대한 다른 작업과 겹쳤다. 다시 시도할 수 있다.",
        ),
        (
            Some(ApiErrorCode::InternalServerError | ApiErrorCode::ServiceUnavailable),
            _,
        ) => request_failed_temporarily("Notion이 요청을 처리하지 못했다. 다시 시도할 수 있다."),
        // 그 밖의 4xx — 다시 보내도 같다. 그 밖의 응답은 서버 쪽 사정이므로 다시 시도해 볼 값이 있다.
        _ if response.is_client_error() => request_rejected("Notion이 요청을 거절했다."),
        _ => request_failed_temporarily("Notion이 요청을 처리하지 못했다. 다시 시도할 수 있다."),
    };

    NotionFailure::new(failure.with_detail(detail))
}

/// 응답은 2xx인데 이 adapter가 기대한 모양이 아니다 → **결과를 모른다** (ADR-0009 §7.3 · §8.5).
///
/// **재시도 가능으로 표시하지 않는다.** 페이지가 만들어졌을 수도 있고, 그대로 다시 보내면
/// 사용자가 모르는 사이에 페이지가 둘이 된다. 사용자가 Notion을 확인하고 다시 고를 일이다 —
/// "모른다"를 "실패했다"로도 "성공했다"로도 바꿔 적지 않는다.
fn unreadable_body(rejection: BodyRejection) -> NotionFailure {
    NotionFailure::new(
        Failure::permanent(
            FailureKind::NotionResponseUnusable,
            "Notion이 응답했지만 만들어진 페이지를 확인할 수 없다. Notion에서 페이지가 만들어졌는지 확인해야 한다.",
        )
        .with_detail(rejection),
    )
}

/// token이 거절됐다 (§13 `authentication failure`).
fn auth_failed(message: &str) -> Failure {
    Failure::permanent(FailureKind::NotionAuthFailed, message)
}

/// 보낼 자리에 닿을 수 없다 (§13 `권한 없는 destination` · `phase-prompt/05` 요구 12).
fn destination_unavailable(message: &str) -> Failure {
    Failure::permanent(FailureKind::NotionDestinationUnavailable, message)
}

/// 지금은 보낼 수 없다 — 기다렸다가 같은 요청을 다시 보낸다 (§13 `rate limit`).
fn rate_limited(message: &str) -> Failure {
    Failure::retryable(FailureKind::NotionRateLimited, message)
}

/// 요청이 거절됐다. **같은 요청을 다시 보내도 같다.**
fn request_rejected(message: &str) -> Failure {
    Failure::permanent(FailureKind::NotionRequestFailed, message)
}

/// 요청이 실패했지만 다시 시도해 볼 값이 있다.
fn request_failed_temporarily(message: &str) -> Failure {
    Failure::retryable(FailureKind::NotionRequestFailed, message)
}

#[cfg(test)]
mod tests {
    use super::super::testing::{StubReply, StubServer, CREATED_PAGE_ID};
    use super::*;

    /// 테스트가 쓰는 값들. **하나도 실재하지 않는다** (ADR-0009 §10.5).
    const NOT_A_REAL_TOKEN: &str = "ntn-double-value-not-a-real-credential";
    const PARENT_PAGE_ID: &str = "parent-page-identifier";

    fn token() -> Secret {
        Secret::new(NOT_A_REAL_TOKEN)
    }

    fn client(server: StubServer) -> NotionClient {
        NotionClient::new(Arc::new(server))
    }

    fn created_page(server: StubServer) -> Result<PageId, NotionFailure> {
        client(server).create_page(&token(), PARENT_PAGE_ID, "# 제목\n본문")
    }

    #[test]
    fn creating_a_page_goes_to_the_documented_address_with_the_documented_headers() {
        let server = Arc::new(StubServer::ready());
        let client = NotionClient::new(server.clone());

        let page = client
            .create_page(&token(), PARENT_PAGE_ID, "# 제목\n본문")
            .expect("stub이 페이지를 만든다");
        assert_eq!(page.as_str(), CREATED_PAGE_ID);

        let requests = server.requests();
        assert_eq!(requests.len(), 1, "생성은 왕복 한 번이다");
        assert_eq!(requests[0].url, "https://api.notion.com/v1/pages");
        assert_eq!(requests[0].method.as_str(), "POST");
        assert_eq!(
            requests[0].header(AUTHORIZATION_HEADER).as_deref(),
            Some(format!("Bearer {NOT_A_REAL_TOKEN}").as_str())
        );
        assert_eq!(
            requests[0].header(NOTION_VERSION_HEADER).as_deref(),
            Some("2026-03-11")
        );
        assert_eq!(
            requests[0].header(CONTENT_TYPE_HEADER).as_deref(),
            Some("application/json")
        );
    }

    #[test]
    fn appending_patches_the_markdown_endpoint_of_that_one_page() {
        let server = Arc::new(StubServer::ready());
        let client = NotionClient::new(server.clone());
        let page = PageId::parse(CREATED_PAGE_ID).expect("모양이 맞다");

        client
            .append_markdown(&token(), &page, "## 부록")
            .expect("stub이 이어 붙인다");

        let requests = server.requests();
        assert_eq!(requests[0].method.as_str(), "PATCH");
        assert_eq!(
            requests[0].url,
            format!("https://api.notion.com/v1/pages/{CREATED_PAGE_ID}/markdown")
        );
    }

    #[test]
    fn the_connection_check_asks_who_the_token_belongs_to_and_sends_no_body() {
        let server = Arc::new(StubServer::ready());
        let client = NotionClient::new(server.clone());

        client.check_connection(&token()).expect("stub이 답한다");

        let requests = server.requests();
        assert_eq!(requests[0].url, "https://api.notion.com/v1/users/me");
        assert_eq!(requests[0].method.as_str(), "GET");
        assert_eq!(requests[0].body, None);
        // 본문이 없는 요청에는 `Content-Type`을 싣지 않는다 (ADR-0009 §5.2).
        assert_eq!(requests[0].header(CONTENT_TYPE_HEADER), None);
        assert!(requests[0].header(AUTHORIZATION_HEADER).is_some());
    }

    #[test]
    fn a_rejected_token_is_a_different_state_from_a_destination_we_cannot_reach() {
        let unauthorized = created_page(
            StubServer::ready().with_create_page(StubReply::error(401, ApiErrorCode::Unauthorized)),
        )
        .expect_err("거절된 token으로는 만들 수 없다");
        assert_eq!(unauthorized.kind(), FailureKind::NotionAuthFailed);
        assert!(!unauthorized.failure().retryable, "token을 고쳐야 풀린다");
        assert_eq!(unauthorized.wait(), None, "자동 재시도가 없다");

        for (status, code) in [
            (404, ApiErrorCode::ObjectNotFound),
            (403, ApiErrorCode::RestrictedResource),
        ] {
            let failure = created_page(
                StubServer::ready().with_create_page(StubReply::error(status, code)),
            )
            .expect_err("닿을 수 없는 자리에는 만들 수 없다");

            assert_eq!(failure.kind(), FailureKind::NotionDestinationUnavailable);
            assert!(!failure.failure().retryable, "공유 설정을 고쳐야 풀린다");
        }
    }

    #[test]
    fn a_rate_limited_response_answers_with_how_long_to_wait() {
        // ADR-0009 §9.2: 값을 임의로 줄이지 않고 그대로 돌려준다.
        for status in [429, 529] {
            let failure = created_page(
                StubServer::ready()
                    .with_create_page(StubReply::rate_limited(status, Some("30"))),
            )
            .expect_err("지금은 보낼 수 없다");

            assert_eq!(failure.kind(), FailureKind::NotionRateLimited);
            assert_eq!(failure.wait(), Some(RetryAfter::Seconds(30)));
            assert_eq!(failure.wait().and_then(RetryAfter::seconds), Some(30));
            assert!(failure.failure().retryable, "기다렸다가 다시 보낸다");
        }
    }

    #[test]
    fn a_rate_limit_without_a_readable_header_says_so_instead_of_inventing_a_number() {
        let failure =
            created_page(StubServer::ready().with_create_page(StubReply::rate_limited(429, None)))
                .expect_err("지금은 보낼 수 없다");

        assert_eq!(failure.wait(), Some(RetryAfter::Unspecified));
        assert_eq!(
            failure.wait().and_then(RetryAfter::seconds),
            None,
            "얼마나 기다릴지는 이 adapter가 정하지 않는다"
        );
    }

    #[test]
    fn only_a_rate_limit_carries_a_wait_instruction() {
        for reply in [
            StubReply::error(400, ApiErrorCode::ValidationError),
            StubReply::error(500, ApiErrorCode::InternalServerError),
            StubReply::Status(418),
            StubReply::Fail(TransportError::TimedOut),
        ] {
            let failure = created_page(StubServer::ready().with_create_page(reply))
                .expect_err("이 상황에서는 만들 수 없다");

            assert_eq!(
                failure.wait(),
                None,
                "rate limit이 아닌 실패가 대기 지시를 만들었다: {failure:?}"
            );
        }
    }

    #[test]
    fn the_documented_error_codes_land_on_the_states_the_user_can_act_on() {
        let cases: Vec<(ApiErrorCode, u16, FailureKind, bool)> = vec![
            (ApiErrorCode::ValidationError, 400, FailureKind::NotionRequestFailed, false),
            (ApiErrorCode::Unauthorized, 401, FailureKind::NotionAuthFailed, false),
            (
                ApiErrorCode::RestrictedResource,
                403,
                FailureKind::NotionDestinationUnavailable,
                false,
            ),
            (
                ApiErrorCode::ObjectNotFound,
                404,
                FailureKind::NotionDestinationUnavailable,
                false,
            ),
            (ApiErrorCode::RateLimited, 429, FailureKind::NotionRateLimited, true),
            (ApiErrorCode::ConflictError, 409, FailureKind::NotionRequestFailed, true),
            (
                ApiErrorCode::InternalServerError,
                500,
                FailureKind::NotionRequestFailed,
                true,
            ),
            (
                ApiErrorCode::ServiceUnavailable,
                503,
                FailureKind::NotionRequestFailed,
                true,
            ),
        ];

        for (code, status, expected, retryable) in cases {
            let failure =
                created_page(StubServer::ready().with_create_page(StubReply::error(status, code)))
                    .expect_err("비2xx는 실패다");

            assert_eq!(failure.kind(), expected, "{code}");
            assert_eq!(failure.failure().retryable, retryable, "{code}: 재시도 가치");
            assert!(
                failure.failure().source_data_safe,
                "{code}: 전송 실패는 local data를 건드리지 않는다 (INV-3)"
            );
            assert_eq!(
                failure.failure().detail.as_deref(),
                Some(format!("HTTP {status} {code}").as_str()),
                "{code}: 원인은 상태 코드와 벤더 코드뿐이다"
            );
        }
    }

    #[test]
    fn a_status_code_with_no_body_is_still_judged() {
        // `529`는 JSON 본문이 온다는 계약이 없다 (ADR-0009 §9.1).
        let overloaded =
            created_page(StubServer::ready().with_create_page(StubReply::Status(529)))
                .expect_err("과부하는 실패다");
        assert_eq!(overloaded.kind(), FailureKind::NotionRateLimited);
        assert_eq!(overloaded.wait(), Some(RetryAfter::Unspecified));

        let unauthorized =
            created_page(StubServer::ready().with_create_page(StubReply::Status(401)))
                .expect_err("401은 실패다");
        assert_eq!(unauthorized.kind(), FailureKind::NotionAuthFailed);

        let unknown = created_page(StubServer::ready().with_create_page(StubReply::Status(418)))
            .expect_err("모르는 4xx도 실패다");
        assert_eq!(unknown.kind(), FailureKind::NotionRequestFailed);
        assert!(!unknown.failure().retryable, "4xx는 같은 요청에 같은 답이다");
    }

    #[test]
    fn a_two_hundred_that_is_not_a_created_page_is_never_reported_as_success() {
        // 요청하지 않은 202가 그 자리다 (ADR-0009 §7.3). 페이지가 만들어졌을 수도 있으므로
        // 조용히 다시 보내지 않는다.
        for reply in [
            StubReply::Body(r#"{"object":"async_task","id":"task-id"}"#.to_owned()),
            StubReply::Body("{}".to_owned()),
            StubReply::Body("Notion is fine".to_owned()),
        ] {
            let failure = created_page(StubServer::ready().with_create_page(reply))
                .expect_err("페이지를 확인하지 못하면 성공이 아니다");

            assert_eq!(failure.kind(), FailureKind::NotionResponseUnusable);
            assert!(
                !failure.failure().retryable,
                "그대로 다시 보내면 페이지가 둘이 될 수 있다"
            );
            assert!(failure.failure().detail.is_some(), "무엇이 달랐는지가 남는다");
        }
    }

    #[test]
    fn a_transport_that_never_answered_is_a_retryable_request_failure() {
        for error in [
            TransportError::NotConnected,
            TransportError::TimedOut,
            TransportError::Incomplete,
        ] {
            let failure = created_page(StubServer::failing(error)).expect_err("답이 없다");

            assert_eq!(failure.kind(), FailureKind::NotionRequestFailed);
            assert!(failure.failure().retryable);
            assert_eq!(failure.failure().detail.as_deref(), Some(error.as_str()));
        }
    }

    #[test]
    fn every_failure_this_adapter_makes_is_one_of_the_five_declared_kinds() {
        let situations = vec![
            StubReply::error(401, ApiErrorCode::Unauthorized),
            StubReply::error(404, ApiErrorCode::ObjectNotFound),
            StubReply::rate_limited(429, Some("1")),
            StubReply::error(500, ApiErrorCode::InternalServerError),
            StubReply::Body("{}".to_owned()),
        ];

        let mut seen: Vec<FailureKind> = Vec::new();
        for reply in situations {
            let failure = created_page(StubServer::ready().with_create_page(reply))
                .expect_err("이 상황에서는 만들 수 없다");
            assert!(
                NOTION_FAILURE_KINDS.contains(&failure.kind()),
                "선언되지 않은 실패 종류가 나왔다: {:?}",
                failure.kind()
            );
            seen.push(failure.kind());
        }

        seen.sort_by_key(|kind| kind.as_str());
        seen.dedup();
        assert_eq!(
            seen.len(),
            NOTION_FAILURE_KINDS.len(),
            "다섯 상황이 다섯 종류로 갈리지 않는다: {seen:?}"
        );
    }
}
