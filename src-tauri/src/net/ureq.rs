//! [`HttpTransport`]의 실제 구현 — **이 adapter에서 소켓을 여는 유일한 파일이다.**
//!
//! Phase 4가 고른 경로를 그대로 따른다 (ADR-0008 §12.2): SDK wrapper를 쓰지 않고 REST를 직접
//! 부르며, 클라이언트는 동기(blocking) `ureq`다. 이 저장소의 모든 경계가 동기이고, 오래 걸리는
//! 일은 평범한 스레드로 UI 밖에서 돈다 — 호출 하나 때문에 async runtime을 들이지 않는다.
//! `@notionhq/client`(JS SDK)를 쓰지 않는 이유도 같다: 외부로 나가는 호출은 Rust backend에서
//! 나간다 (ADR-0009 §1).
//!
//! **자동 검증은 이 파일을 실행하지 않는다** (PRODUCT-SPEC §18). 테스트가 쓰는 것은
//! [`super::testing::StubServer`]이며, 여기 있는 코드는 Gate가 **컴파일**한다.
//! AI adapter가 소켓을 여는 파일 · [`crate::platform::secret_store::OsSecretStore`]와 같은
//! 자리다.
//!
//! ## HTTPS는 켜야 하는 것이다 — 여기서 가정하지 않는다
//!
//! Notion은 HTTPS다. `ureq`의 TLS 경로는 `src-tauri/Cargo.toml`에서 **이름으로 켠 feature**가
//! 만들며(ADR-0009 §11 · P-4), 켠 feature 이름과 확인한 버전, 그리고 `Cargo.lock`이 실제로
//! 무엇을 들여왔는지는 그 파일의 주석과 `.loop/evidence/TASK-047/ureq-tls-verification.md`에
//! 있다. **인증서 검증을 끄는 구성은 이 파일에 없다** — 그런 선택지를 두지 않는다.
//!
//! ## 요청 헤더에는 token이 실린다
//!
//! 그 값은 [`HttpRequest`]가 들고 오며 여기서는 그대로 헤더에 얹기만 한다. 이 파일은 값을
//! 기록하지도, 문장으로 만들지도 않는다 (ADR-0009 §10.4).

use std::io::ErrorKind;
use std::time::Duration;

use super::http::{HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError};
use super::wire::{self, RETRY_AFTER_HEADER};

/// 서지 않는 연결을 오래 붙들고 있지 않기 위한 값.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// `ureq`로 왕복 하나를 수행하는 transport.
#[derive(Debug)]
pub struct UreqNotionTransport {
    agent: ureq::Agent,
}

impl UreqNotionTransport {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            // 비2xx를 오류가 아니라 응답으로 받는다. 상태 코드를 §13의 어느 실패로 옮길지는
            // [`super::client`]가 판정하며, 그 판정이 라이브러리의 기본값에 좌우되지 않게 한다.
            // `429`·`529`의 `Retry-After`를 읽으려면 응답이 응답으로 와야 한다.
            .http_status_as_error(false)
            .build();

        Self {
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

impl Default for UreqNotionTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport for UreqNotionTransport {
    fn send(&self, request: &HttpRequest<'_>) -> Result<HttpResponse, TransportError> {
        // 메서드마다 builder의 타입이 다르므로(본문이 있는 것과 없는 것) 세 갈래가 그대로 있다.
        let outcome = match request.method {
            HttpMethod::Get => {
                let mut builder = self.agent.get(request.url);
                for (name, value) in request.headers {
                    builder = builder.header(*name, *value);
                }
                builder.call()
            }
            HttpMethod::Post => {
                let mut builder = self.agent.post(request.url);
                for (name, value) in request.headers {
                    builder = builder.header(*name, *value);
                }
                builder.send(request.body.unwrap_or_default())
            }
            HttpMethod::Patch => {
                let mut builder = self.agent.patch(request.url);
                for (name, value) in request.headers {
                    builder = builder.header(*name, *value);
                }
                builder.send(request.body.unwrap_or_default())
            }
        };

        let response = outcome.map_err(classify)?;

        // **헤더 중 이것 하나만 꺼낸다** (ADR-0009 §5.5). 정수 초로 읽는 규칙은 이 파일이 다시
        // 쓰지 않고 [`wire::retry_after_seconds`]가 그대로 한다 — 그래서 그 규칙이 네트워크
        // 없이 검증된다.
        let retry_after = response
            .headers()
            .get(RETRY_AFTER_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(wire::retry_after_seconds);

        let status = response.status().as_u16();
        let body = response
            .into_body()
            .read_to_string()
            .map_err(|_| TransportError::Incomplete)?;

        Ok(HttpResponse::new(status, body).with_retry_after(retry_after))
    }
}

/// 라이브러리 오류를 [`TransportError`]의 세 가지 중 하나로 옮긴다.
///
/// **오류 문장을 옮기지 않는다.** `ureq`의 오류 표현에는 요청한 URL이 들어 있을 수 있고, 그
/// URL에는 destination(page id)이 실려 있다 (ADR-0009 §10.4). 여기서 남는 것은 변형뿐이다.
///
/// 분류하지 못한 오류는 [`TransportError::Incomplete`]로 간다 — 확인되지 않은 원인을 "연결되지
/// 않았다"로 몰지 않는다.
fn classify(error: ureq::Error) -> TransportError {
    match error {
        ureq::Error::Timeout(_) => TransportError::TimedOut,
        ureq::Error::ConnectionFailed => TransportError::NotConnected,
        ureq::Error::Io(io) => match io.kind() {
            ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::NotConnected => TransportError::NotConnected,
            ErrorKind::TimedOut => TransportError::TimedOut,
            _ => TransportError::Incomplete,
        },
        _ => TransportError::Incomplete,
    }
}
