//! 이 adapter 안에서 **실제 네트워크에 닿는 유일한 경계**의 계약
//! (ADR-0009 §5.5 · PRODUCT-SPEC §18).
//!
//! ```text
//! HttpRequest  ─→ │ HttpTransport │ ─→ 상태 코드 + 본문 + (있으면) Retry-After 초
//!                 └───────────────┘ ─→ TransportError (닿지 못했다 · 늦었다 · 읽지 못했다)
//! ```
//!
//! **이 계약은 Notion을 모른다.** 엔드포인트도 파라미터 이름도 오류 코드도 여기 없다 —
//! 그것은 [`super::wire`]의 몫이다. 여기 있는 것은 "요청을 보내고 응답을 받는다"뿐이며,
//! 그래서 double 하나로 adapter의 전 경로가 검증된다 (§18).
//!
//! ## AI adapter에 있는 같은 이름과 왜 따로 있는가
//!
//! ADR-0009 §5.5는 이 Phase가 HTTP 경계를 "넓히되 최소로 넓힌다"고 적었다 — `PATCH` · 요청
//! 헤더 · 응답의 `Retry-After` 셋이다. 그 셋을 **여기서** 더한 이유는 하나다: 지금 있는 같은
//! 모양의 경계는 **AI 벤더 adapter 디렉터리 안에** 있다. Notion adapter가 그것을 쓰면 두 벤더
//! adapter가 서로를 알게 되고, 한쪽 벤더 사정으로 넓힌 타입이 다른 쪽의 계약이 된다 (INV-9가
//! 막으려는 것이 정확히 그 결합이다). 그래서 **넓힌 셋은 이 디렉터리 안에만 있고, Phase 4의
//! 파일은 이 Task에서 한 줄도 바뀌지 않는다.**
//!
//! ## 이 타입들은 값을 담고도 값을 내지 않는다
//!
//! 요청 헤더에는 **token이 실리고**(`Authorization`), URL에는 **destination이 실린다**
//! (`/v1/pages/<page_id>/markdown`). 그래서 [`HttpRequest`]와 [`HttpResponse`]는 `Debug`를
//! 손으로 이행해 **값을 내지 않는다** — `{:?}` 한 번으로 token이나 destination이 로그·실패
//! 문장에 옮겨 가는 통로를 규칙이 아니라 타입으로 막는다 (ADR-0009 §10.4 · INV-7).
//!
//! [`TransportError`]에 문자열이 없는 것도 같은 설계다 (AI adapter의 HTTP 경계와 같은 이유) —
//! HTTP 라이브러리의 오류 문장에는 요청한 URL이 그대로 들어 있는 경우가 흔하다. 옮길 수 있는
//! 문자열이 애초에 없으면 새어 나갈 수도 없다.

use std::fmt;

/// 이 adapter가 쓰는 메서드 셋 (ADR-0009 §5.1).
///
/// `Patch`가 있는 것은 이어붙이기가 `PATCH /v1/pages/:page_id/markdown`이기 때문이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Patch,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Patch => "PATCH",
        }
    }

    /// 본문을 실어 보내는 메서드인가.
    pub fn carries_body(&self) -> bool {
        matches!(self, Self::Post | Self::Patch)
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 요청 헤더 하나 — 이름과 값.
///
/// **값에는 token이 들어 있을 수 있다.** 그래서 이 쌍은 [`HttpRequest`]의 `Debug`를 지나지
/// 않으며, 값을 읽는 자리는 transport 구현 하나뿐이다.
pub type HttpHeader<'a> = (&'a str, &'a str);

/// 한 번의 왕복에 필요한 전부.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HttpRequest<'a> {
    pub method: HttpMethod,
    /// 보낼 주소 전체. 만드는 자리는 [`super::wire`] 하나다.
    pub url: &'a str,
    /// 보낼 헤더들. 이 adapter는 세 개를 쓴다 (ADR-0009 §5.2).
    pub headers: &'a [HttpHeader<'a>],
    /// 요청 본문(JSON). `GET`에는 없다.
    pub body: Option<&'a str>,
}

impl<'a> HttpRequest<'a> {
    pub fn get(url: &'a str, headers: &'a [HttpHeader<'a>]) -> Self {
        Self {
            method: HttpMethod::Get,
            url,
            headers,
            body: None,
        }
    }

    pub fn post_json(url: &'a str, headers: &'a [HttpHeader<'a>], body: &'a str) -> Self {
        Self {
            method: HttpMethod::Post,
            url,
            headers,
            body: Some(body),
        }
    }

    pub fn patch_json(url: &'a str, headers: &'a [HttpHeader<'a>], body: &'a str) -> Self {
        Self {
            method: HttpMethod::Patch,
            url,
            headers,
            body: Some(body),
        }
    }
}

/// **요청의 어떤 값도 내지 않는다** — 헤더 값에는 token이, URL에는 destination이, 본문에는
/// 사용자의 문서가 들어 있다 (ADR-0009 §10.4 · §5.5). 나오는 것은 메서드와 헤더 **이름**뿐이다.
impl fmt::Debug for HttpRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.headers.iter().map(|(name, _)| *name).collect();
        write!(
            f,
            "HttpRequest {{ method: {}, url: <redacted>, headers: {names:?}, body: {} }}",
            self.method,
            match self.body {
                Some(body) => format!("<{} bytes>", body.len()),
                None => "none".to_owned(),
            }
        )
    }
}

/// 응답이 돌아왔다는 사실 그 자체. **해석은 하지 않는다.**
///
/// 비2xx도 여기로 온다 — "응답이 왔다"와 "그 응답이 성공이다"는 다른 진술이고, 후자를
/// 판정하는 것은 [`super::client`]다.
#[derive(Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    /// `Retry-After` 헤더를 **정수 초로 읽은 값** (ADR-0009 §5.5 · §9.1).
    ///
    /// **헤더 원문이 아니다.** 응답 헤더 전체를 노출하지 않는 이유는 [`TransportError`]에
    /// 문자열을 두지 않은 것과 같다 — 필요한 것 하나만 값으로 꺼내면 나머지가 새어 나갈 통로가
    /// 아예 없다. 읽지 못했거나 헤더가 없으면 `None`이다.
    pub retry_after_seconds: Option<u32>,
}

impl HttpResponse {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            retry_after_seconds: None,
        }
    }

    /// 서버가 지시한 대기 시간을 함께 담은 응답.
    pub fn with_retry_after(self, seconds: Option<u32>) -> Self {
        Self {
            retry_after_seconds: seconds,
            ..self
        }
    }

    /// 2xx인가.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 4xx인가 — **같은 요청을 다시 보내도 같다**는 뜻이다.
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status)
    }
}

/// **본문을 내지 않는다.** Notion의 오류 본문에는 우리가 보낸 destination이 그대로 들어 있는
/// 경우가 있다 (`Could not find page with ID: …`). 그것이 `{:?}` 한 번으로 로그에 남지 않게 한다.
impl fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HttpResponse {{ status: {}, body: <{} bytes>, retry_after_seconds: {:?} }}",
            self.status,
            self.body.len(),
            self.retry_after_seconds
        )
    }
}

/// 응답을 받지 못했다. **세 가지를 구분하는 이유는 §13이 서로 다른 제품 상태를 요구하기
/// 때문이다.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// 연결 자체가 되지 않았다 — 네트워크가 없거나 이름을 찾지 못했다.
    NotConnected,
    /// 연결은 됐지만 제때 답이 오지 않았다.
    TimedOut,
    /// 그 밖의 이유로 요청을 마치지 못했다 — 본문을 끝까지 읽지 못한 경우가 여기 속한다.
    Incomplete,
}

impl TransportError {
    /// 실패 `detail`에 남길 **기술적 원인**. token도 destination도 섞일 수 없는 고정 문자열이다.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotConnected => "connection not established",
            Self::TimedOut => "request timed out",
            Self::Incomplete => "request did not complete",
        }
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 요청 하나를 보내고 응답을 받는다. **그 이상은 하지 않는다.**
///
/// `Send + Sync`인 것은 전송이 UI를 막지 않는 스레드에서 돌기 때문이다 — 이 저장소의 다른
/// 경계들과 같다 (`TranscriptionEngine` · `NoteAiProvider` · `SecretStore`).
pub trait HttpTransport: Send + Sync {
    fn send(&self, request: &HttpRequest<'_>) -> Result<HttpResponse, TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트가 쓰는 값. **실제 자격증명이 아니다** (ADR-0009 §10.5).
    const NOT_A_REAL_TOKEN: &str = "ntn-double-value-not-a-real-credential";
    const DESTINATION: &str = "destination-page-identifier";

    #[test]
    fn a_transport_error_can_carry_no_value_at_all() {
        // 이 타입에 문자열 자리가 생기면 URL이 실패 detail로 새어 나갈 수 있다.
        // 변형을 전부 나열하는 이 match는 데이터를 가진 변형이 생기면 컴파일되지 않는다.
        for error in [
            TransportError::NotConnected,
            TransportError::TimedOut,
            TransportError::Incomplete,
        ] {
            let text = match error {
                TransportError::NotConnected => "connection not established",
                TransportError::TimedOut => "request timed out",
                TransportError::Incomplete => "request did not complete",
            };
            assert_eq!(error.to_string(), text);
        }
    }

    #[test]
    fn printing_a_request_shows_neither_the_token_nor_the_destination() {
        // ADR-0009 §10.4 · INV-7: `{:?}` 한 번으로 새는 경로를 타입이 막는다.
        let url = format!("https://api.notion.invalid/v1/pages/{DESTINATION}/markdown");
        let authorization = format!("Bearer {NOT_A_REAL_TOKEN}");
        let headers = [
            ("Authorization", authorization.as_str()),
            ("Notion-Version", "2026-03-11"),
        ];
        let request = HttpRequest::patch_json(&url, &headers, "# 사용자의 문서");

        let rendered = format!("{request:?}");

        assert!(!rendered.contains(NOT_A_REAL_TOKEN), "{rendered}");
        assert!(!rendered.contains(DESTINATION), "{rendered}");
        assert!(!rendered.contains("사용자의 문서"), "{rendered}");
        // 무엇을 보냈는지는 여전히 말한다 — 값 없이.
        assert!(rendered.contains("PATCH"), "{rendered}");
        assert!(rendered.contains("Authorization"), "{rendered}");
    }

    #[test]
    fn printing_a_response_does_not_show_what_the_server_echoed_back() {
        // Notion의 오류 본문에는 우리가 보낸 destination이 그대로 들어 있을 수 있다.
        let response = HttpResponse::new(
            404,
            format!(r#"{{"code":"object_not_found","message":"Could not find page {DESTINATION}"}}"#),
        );

        let rendered = format!("{response:?}");

        assert!(!rendered.contains(DESTINATION), "{rendered}");
        assert!(rendered.contains("404"), "{rendered}");
    }

    #[test]
    fn the_boundary_reports_status_codes_instead_of_judging_them() {
        assert!(HttpResponse::new(200, "{}").is_success());
        assert!(!HttpResponse::new(404, "").is_success());
        assert!(HttpResponse::new(404, "").is_client_error());
        assert!(!HttpResponse::new(529, "").is_client_error());
        assert!(!HttpResponse::new(500, "").is_success());
    }

    #[test]
    fn a_request_carries_a_body_only_where_one_is_sent() {
        let headers: [HttpHeader<'_>; 0] = [];

        let get = HttpRequest::get("https://api.notion.invalid/v1/users/me", &headers);
        assert_eq!(get.method, HttpMethod::Get);
        assert_eq!(get.body, None);
        assert!(!get.method.carries_body());

        let post = HttpRequest::post_json("https://api.notion.invalid/v1/pages", &headers, "{}");
        assert_eq!(post.method, HttpMethod::Post);
        assert_eq!(post.body, Some("{}"));
        assert!(post.method.carries_body());

        let patch = HttpRequest::patch_json("https://api.notion.invalid/x", &headers, "{}");
        assert_eq!(patch.method, HttpMethod::Patch);
        assert_eq!(patch.method.as_str(), "PATCH");
        assert!(patch.method.carries_body());
    }

    #[test]
    fn a_wait_instruction_is_a_number_and_not_a_header_line() {
        let response = HttpResponse::new(429, "{}").with_retry_after(Some(30));

        assert_eq!(response.retry_after_seconds, Some(30));
        assert_eq!(
            HttpResponse::new(429, "{}").retry_after_seconds,
            None,
            "헤더가 없으면 값이 없다 — 앱이 지어내지 않는다"
        );
    }
}
