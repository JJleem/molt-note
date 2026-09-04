//! HTTP 경계의 **결정론적 test double** — 이 adapter의 자동 검증이 Notion을 요구하지 않는
//! 이유다 (PRODUCT-SPEC §18 · `phase-prompt/05` 요구 14).
//!
//! ```text
//! StubServer::ready()               세 경로가 모두 답한다 (페이지가 만들어진다)
//! StubServer::refusing()            연결되지 않는다
//! StubServer::failing(error)        지정한 transport 실패
//! .with_users_me(reply)             연결 확인이 답할 것
//! .with_create_page(reply)          페이지 생성이 답할 것
//! .with_append(reply)               이어붙이기가 답할 것
//! .requests()                       지금까지 받은 요청들 (헤더 · 본문 포함)
//! ```
//!
//! **소켓을 열지 않는다.** [`super::http::HttpTransport`]를 값으로 이행할 뿐이며, 이 파일에는
//! 네트워크도 HTTP 클라이언트도 없다. 그래서 **자동 테스트는 실제 Notion에 한 번도 요청하지
//! 않고, 어떤 워크스페이스도 오염시키지 않는다.** 그 사실은 `tests/notion_adapter.rs`가
//! 소스에서 확인한다 — 규칙을 적어 두는 것으로 끝내지 않는다.
//!
//! **여기에는 실제 자격증명이 없다.** 이 double은 어떤 token도 만들지 않으며, 테스트가 넘긴
//! 값을 그대로 되돌려 볼 수 있게 기록할 뿐이다 (그 기록이 실패로 새어 나가지 않는다는 것이
//! 검증 대상 중 하나다 · ADR-0009 §10.4).
//!
//! `#[cfg(test)]`가 아닌 것은 통합 테스트(`src-tauri/tests/`)가 별개 crate에서 이것을 쓰기
//! 때문이다 (`platform::secret_store::testing`과 같은 이유다).

use std::sync::Mutex;

use serde_json::json;

use super::http::{HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError};
use super::wire::{self, ApiErrorCode};

/// [`StubServer::ready`]가 만들어 주는 페이지의 식별자. **실재하지 않는다.**
pub const CREATED_PAGE_ID: &str = "stub-created-page-identifier";

/// stub이 받은 요청 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

impl StubRequest {
    /// 그 이름으로 실려 온 헤더 값. **테스트가 "무엇을 보냈는가"를 관찰하는 자리다.**
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers
            .iter()
            .find(|(sent, _)| sent == name)
            .map(|(_, value)| value.clone())
    }
}

/// stub이 한 경로에 대해 낼 답.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StubReply {
    /// `200` + 이 식별자를 가진 페이지.
    CreatedPage(String),
    /// `200` + 있는 그대로의 본문.
    Body(String),
    /// 이 상태 코드 + Notion이 실패를 말하는 본문 (`code` 포함).
    Error { status: u16, code: ApiErrorCode },
    /// 본문 없는 이 상태 코드 — `529`처럼 JSON이 온다는 계약이 없는 응답의 모습이다.
    Status(u16),
    /// rate limit — 상태 코드와 `Retry-After` **헤더 원문**.
    ///
    /// 헤더 값이 문자열인 것은 일부러다: 그래야 이 double을 지나는 테스트가 **실제 파싱
    /// 경로**([`wire::retry_after_seconds`])를 지난다. 값을 미리 숫자로 넣으면 파싱은 검증되지
    /// 않는다.
    RateLimited {
        status: u16,
        retry_after: Option<String>,
    },
    /// 응답이 오지 않는다.
    Fail(TransportError),
}

impl StubReply {
    /// Notion이 실패를 말하는 응답 하나.
    pub fn error(status: u16, code: ApiErrorCode) -> Self {
        Self::Error { status, code }
    }

    /// 속도를 늦추라는 응답 하나. `retry_after`는 헤더에 실릴 문자열 그대로다.
    pub fn rate_limited(status: u16, retry_after: Option<&str>) -> Self {
        Self::RateLimited {
            status,
            retry_after: retry_after.map(str::to_owned),
        }
    }
}

/// 이 adapter가 부르는 세 경로에 미리 정해 둔 답을 돌려주는 transport.
///
/// **결정론적이다** — 무작위도, 시간도, 파일도, 네트워크도 쓰지 않는다.
#[derive(Debug)]
pub struct StubServer {
    users_me: StubReply,
    create_page: StubReply,
    append: StubReply,
    requests: Mutex<Vec<StubRequest>>,
}

impl StubServer {
    /// 연결이 확인되고 페이지가 만들어지고 이어붙이기가 되는 서버.
    pub fn ready() -> Self {
        Self {
            users_me: StubReply::Body(
                json!({ "object": "user", "id": "stub-bot-user", "type": "bot" }).to_string(),
            ),
            create_page: StubReply::CreatedPage(CREATED_PAGE_ID.to_owned()),
            append: StubReply::Body(
                json!({ "object": "page", "id": CREATED_PAGE_ID }).to_string(),
            ),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// 어느 경로에도 연결되지 않는 서버 — 네트워크가 없을 때의 모습이다.
    pub fn refusing() -> Self {
        Self::failing(TransportError::NotConnected)
    }

    /// 어느 경로에서도 같은 transport 실패를 내는 서버.
    pub fn failing(error: TransportError) -> Self {
        Self {
            users_me: StubReply::Fail(error),
            create_page: StubReply::Fail(error),
            append: StubReply::Fail(error),
            ..Self::ready()
        }
    }

    pub fn with_users_me(mut self, reply: StubReply) -> Self {
        self.users_me = reply;
        self
    }

    pub fn with_create_page(mut self, reply: StubReply) -> Self {
        self.create_page = reply;
        self
    }

    pub fn with_append(mut self, reply: StubReply) -> Self {
        self.append = reply;
        self
    }

    /// 지금까지 받은 요청들.
    pub fn requests(&self) -> Vec<StubRequest> {
        self.requests.lock().expect("stub 요청 기록을 잠근다").clone()
    }

    fn reply_for(&self, request: &HttpRequest<'_>) -> StubReply {
        if request.url.ends_with(wire::USERS_ME_PATH) {
            self.users_me.clone()
        } else if request.url.ends_with(wire::MARKDOWN_PATH_SUFFIX) {
            self.append.clone()
        } else if request.url.ends_with(wire::PAGES_PATH) {
            self.create_page.clone()
        } else {
            panic!(
                "이 adapter가 부르는 경로는 셋뿐이다 ({:?}). 받은 요청: {}",
                known_paths(),
                request.url
            )
        }
    }

    /// Notion이 실패를 말하는 본문 (PRODUCT-SPEC §14.9.1의 모양).
    ///
    /// `message`에 요청에 실렸던 값이 그대로 되돌아오는 상황을 **일부러 만든다** — 그 문장이
    /// 실패 message나 detail로 옮겨 가지 않는다는 것이 검증 대상이기 때문이다 (INV-7).
    fn error_body(status: u16, code: ApiErrorCode, echoed: &str) -> String {
        json!({
            "object": "error",
            "status": status,
            "code": code.as_str(),
            "message": format!("Could not process the request for {echoed}."),
        })
        .to_string()
    }

    /// 이 요청이 되돌려 받을 만한 값 — 주소에 실린 것을 그대로 쓴다.
    fn echoed(request: &HttpRequest<'_>) -> String {
        request.url.to_owned()
    }
}

impl HttpTransport for StubServer {
    fn send(&self, request: &HttpRequest<'_>) -> Result<HttpResponse, TransportError> {
        self.requests
            .lock()
            .expect("stub 요청 기록을 잠근다")
            .push(StubRequest {
                method: request.method,
                url: request.url.to_owned(),
                headers: request
                    .headers
                    .iter()
                    .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                    .collect(),
                body: request.body.map(str::to_owned),
            });

        match self.reply_for(request) {
            StubReply::CreatedPage(id) => Ok(HttpResponse::new(
                200,
                json!({ "object": "page", "id": id, "url": "https://notion.invalid/p" })
                    .to_string(),
            )),
            StubReply::Body(body) => Ok(HttpResponse::new(200, body)),
            StubReply::Error { status, code } => Ok(HttpResponse::new(
                status,
                Self::error_body(status, code, &Self::echoed(request)),
            )),
            StubReply::Status(status) => Ok(HttpResponse::new(status, String::new())),
            StubReply::RateLimited {
                status,
                retry_after,
            } => {
                // 실제 transport와 같은 자리에서 같은 함수로 읽는다 — 헤더 원문이 정수 초로
                // 읽히지 않으면 값이 없다.
                let seconds = retry_after
                    .as_deref()
                    .and_then(wire::retry_after_seconds);
                let body = Self::error_body(status, ApiErrorCode::RateLimited, &Self::echoed(request));

                Ok(HttpResponse::new(status, body).with_retry_after(seconds))
            }
            StubReply::Fail(error) => Err(error),
        }
    }
}

/// 이 stub이 아는 경로가 adapter가 부르는 경로와 같은지 — 둘이 어긋나면 stub은 조용히
/// panic하지 않고 여기서 먼저 드러난다.
pub fn known_paths() -> [&'static str; 3] {
    [
        wire::USERS_ME_PATH,
        wire::PAGES_PATH,
        wire::MARKDOWN_PATH_SUFFIX,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notion::wire::PageId;

    #[test]
    fn the_stub_answers_the_three_paths_this_adapter_uses() {
        let server = StubServer::ready();
        let page = PageId::parse(CREATED_PAGE_ID).expect("모양이 맞다");
        let headers = [("Notion-Version", wire::NOTION_VERSION)];

        let addresses = [
            wire::url(wire::API_BASE_URL, wire::USERS_ME_PATH),
            wire::url(wire::API_BASE_URL, wire::PAGES_PATH),
            wire::page_markdown_url(wire::API_BASE_URL, &page),
        ];

        for url in addresses {
            let request = HttpRequest::post_json(&url, &headers, "{}");
            assert_eq!(server.send(&request).expect("답한다").status, 200);
        }

        assert_eq!(server.requests().len(), 3, "받은 요청을 전부 기록한다");
    }

    #[test]
    fn the_stub_reads_the_wait_header_with_the_very_function_the_transport_uses() {
        let server = StubServer::ready()
            .with_create_page(StubReply::rate_limited(429, Some("Wed, 21 Oct 2026 07:28:00 GMT")));
        let url = wire::url(wire::API_BASE_URL, wire::PAGES_PATH);
        let headers: [(&str, &str); 0] = [];

        let response = server
            .send(&HttpRequest::post_json(&url, &headers, "{}"))
            .expect("답한다");

        assert_eq!(response.status, 429);
        assert_eq!(
            response.retry_after_seconds, None,
            "HTTP-date는 정수 초가 아니다 — 지어내지 않는다"
        );
    }

    #[test]
    fn a_path_this_adapter_never_calls_is_not_quietly_answered() {
        let server = StubServer::ready();
        let headers: [(&str, &str); 0] = [];
        let request = HttpRequest::get("https://api.notion.com/v1/databases", &headers);

        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| server.send(&request)))
                .is_err(),
            "stub이 모르는 경로에 조용히 답했다"
        );
    }
}
