//! adapter 안에서 **실제 네트워크에 닿는 유일한 경계**의 계약 (ADR-0008 §4.3 · §18).
//!
//! ```text
//! HttpRequest  ─→ │ HttpTransport │ ─→ 상태 코드 + 본문
//!                 └───────────────┘ ─→ TransportError (닿지 못했다 · 늦었다 · 읽지 못했다)
//! ```
//!
//! **이 계약은 Ollama를 모른다.** 엔드포인트도 파라미터 이름도 응답 해석도 여기 없다 —
//! 그것은 [`super::wire`]의 몫이다. 여기 있는 것은 "요청을 보내고 상태 코드와 본문을
//! 받는다"뿐이며, 그래서 double 하나로 adapter의 전 경로가 검증된다 (§18). 전사 경계가
//! `engine`(실행)과 `parse`(순수 값)로 나뉜 것과 같은 이유다.
//!
//! ## [`TransportError`]에는 문자열이 없다
//!
//! 변형만 있고 데이터가 없는 것은 실수가 아니라 **설계다.** HTTP 라이브러리의 오류 문장에는
//! 요청한 URL이 그대로 들어 있는 경우가 흔하고, 그 URL은 사용자가 설정한 host/port다.
//! 그것이 실패 message나 detail로 새어 나가면 ADR-0008 §11.3을 어긴다. 옮길 수 있는 문자열이
//! 애초에 없으면 새어 나갈 수도 없다 — 규칙이 아니라 타입으로 막는다.

/// 보낼 요청 하나. 이 adapter가 쓰는 것은 두 가지뿐이다 (ADR-0008 §6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 한 번의 왕복에 필요한 전부.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpRequest<'a> {
    pub method: HttpMethod,
    /// 보낼 주소 전체. **여기 담긴 host/port는 설정에서 온 값이다** — 만드는 자리는
    /// [`super::wire::endpoint`] 하나이며, 이 값이 실패 문장으로 옮겨 가지 않는다.
    pub url: &'a str,
    /// 요청 본문(JSON). `GET`에는 없다.
    pub body: Option<&'a str>,
}

impl<'a> HttpRequest<'a> {
    pub fn get(url: &'a str) -> Self {
        Self {
            method: HttpMethod::Get,
            url,
            body: None,
        }
    }

    pub fn post_json(url: &'a str, body: &'a str) -> Self {
        Self {
            method: HttpMethod::Post,
            url,
            body: Some(body),
        }
    }
}

/// 응답이 돌아왔다는 사실 그 자체. **해석은 하지 않는다.**
///
/// 비2xx도 여기로 온다 — "응답이 왔다"와 "그 응답이 성공이다"는 다른 진술이고, 후자를
/// 판정하는 것은 [`super::provider`]다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

impl HttpResponse {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
        }
    }

    /// 2xx인가.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 4xx인가 — **같은 요청을 다시 보내도 같다**는 뜻이다 (ADR-0008 §13.3).
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status)
    }
}

/// 응답을 받지 못했다. **세 가지를 구분하는 이유는 §13이 서로 다른 제품 상태를 요구하기
/// 때문이다** (ADR-0008 §13.1 · §13.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// 연결 자체가 되지 않았다 (connection refused 등) — 서버가 실행 중이 아닐 때의 모습이다
    /// (PRODUCT-SPEC §14.5 "미실행 시 TCP 연결 거부"). `AiProviderUnreachable`로 간다.
    NotConnected,
    /// 연결은 됐지만 제때 답이 오지 않았다. `AiRequestFailed`(재시도 가능)로 간다.
    TimedOut,
    /// 그 밖의 이유로 요청을 마치지 못했다 — 본문을 끝까지 읽지 못한 경우가 여기 속한다.
    Incomplete,
}

impl TransportError {
    /// 실패 `detail`에 남길 **기술적 원인**. 설정값이 섞일 수 없는 고정 문자열이다.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotConnected => "connection not established",
            Self::TimedOut => "request timed out",
            Self::Incomplete => "request did not complete",
        }
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 요청 하나를 보내고 상태 코드와 본문을 받는다. **그 이상은 하지 않는다.**
///
/// `Send + Sync`인 것은 provider가 그렇기 때문이다 ([`crate::ai::NoteAiProvider`]) — 생성은
/// UI를 막지 않는 스레드에서 돈다. 여기서 스레드를 만들지는 않는다.
pub trait HttpTransport: Send + Sync {
    fn send(&self, request: &HttpRequest<'_>) -> Result<HttpResponse, TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transport_error_can_carry_no_configured_value() {
        // ADR-0008 §11.3: 이 타입에 문자열 자리가 생기면 URL이 실패 detail로 새어 나갈 수 있다.
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
    fn the_boundary_reports_status_codes_instead_of_judging_them() {
        assert!(HttpResponse::new(200, "{}").is_success());
        assert!(!HttpResponse::new(404, "").is_success());
        assert!(HttpResponse::new(404, "").is_client_error());
        assert!(!HttpResponse::new(500, "").is_client_error());
        assert!(!HttpResponse::new(500, "").is_success());
    }

    #[test]
    fn a_request_carries_a_body_only_where_one_is_sent() {
        let get = HttpRequest::get("http://example.invalid/path");
        assert_eq!(get.method, HttpMethod::Get);
        assert_eq!(get.body, None);

        let post = HttpRequest::post_json("http://example.invalid/path", "{}");
        assert_eq!(post.method, HttpMethod::Post);
        assert_eq!(post.body, Some("{}"));
    }
}
