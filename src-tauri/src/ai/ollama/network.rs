//! [`HttpTransport`]의 실제 구현 — **이 저장소에서 소켓을 여는 유일한 파일이다.**
//!
//! ADR-0008 §12.2가 고른 경로다: wrapper crate(`ollama-rs`)를 쓰지 않고 REST를 직접 부르며,
//! 클라이언트는 동기(blocking) `ureq`다. 이 저장소의 모든 경계가 동기이고, 오래 걸리는 일은
//! 평범한 스레드로 UI 밖에서 돈다 — 호출 하나 때문에 async runtime을 들이지 않는다 (§20.5).
//!
//! **자동 검증은 이 파일을 실행하지 않는다** (§18). 테스트가 쓰는 것은
//! [`super::testing::StubServer`]이며, 여기 있는 코드는 Gate가 **컴파일**한다.
//! `transcription::whisper`가 모델 없이 컴파일만 되는 것과 같은 자리다.
//!
//! ## 여기 쓰인 API는 문서가 아니라 컴파일러가 확인한 것이다
//!
//! `ureq` 3.4.0에 대해 실제로 컴파일된 시그니처다 (`Cargo.lock`이 버전과 checksum을 고정한다).
//! ADR-0008 §12.3이 남긴 확인 항목 중 이 Run이 답한 것과 답하지 못한 것은
//! `.loop/evidence/TASK-036/verification-log.md`에 적었다.

use std::io::ErrorKind;
use std::time::Duration;

use super::http::{HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError};

/// 연결이 서지 않는 서버를 오래 붙들고 있지 않기 위한 값.
///
/// **생성 자체에는 시간 제한을 두지 않는다** — 로컬 모델의 생성은 분 단위가 될 수 있고
/// (1시간 분량 transcript는 Phase 4 Human Review 항목이다), 앱이 임의로 끊으면 사용자는
/// 무엇이 실패했는지 알 수 없는 실패를 보게 된다.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// `ureq`로 왕복 하나를 수행하는 transport.
#[derive(Debug)]
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl UreqTransport {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            // 비2xx를 오류가 아니라 응답으로 받는다. 상태 코드를 §13의 어느 실패로 옮길지는
            // [`super::provider`]가 판정하며, 그 판정이 라이브러리의 기본값에 좌우되지 않게 한다.
            .http_status_as_error(false)
            .build();

        Self {
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransport for UreqTransport {
    fn send(&self, request: &HttpRequest<'_>) -> Result<HttpResponse, TransportError> {
        let outcome = match request.method {
            HttpMethod::Get => self.agent.get(request.url).call(),
            HttpMethod::Post => self
                .agent
                .post(request.url)
                .content_type("application/json")
                .send(request.body.unwrap_or_default()),
        };

        let response = outcome.map_err(classify)?;
        let status = response.status().as_u16();
        let body = response
            .into_body()
            .read_to_string()
            .map_err(|_| TransportError::Incomplete)?;

        Ok(HttpResponse::new(status, body))
    }
}

/// 라이브러리 오류를 [`TransportError`]의 세 가지 중 하나로 옮긴다.
///
/// **오류 문장을 옮기지 않는다.** `ureq`의 오류 표현에는 요청한 URL이 들어 있을 수 있고,
/// 그 URL은 사용자가 설정한 host/port다 (ADR-0008 §11.3). 여기서 남는 것은 변형뿐이다.
///
/// 분류하지 못한 오류는 [`TransportError::Incomplete`]로 간다 — "연결되지 않았다"는 사용자에게
/// **서버를 켜라**고 말하는 답이므로, 확인되지 않은 원인을 그쪽으로 몰지 않는다.
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
