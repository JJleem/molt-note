//! **벤더를 모르는 HTTP 경계** — 요청 하나를 보내고 상태 코드와 본문을 받는다.
//!
//! ```text
//! HttpRequest ─→ │ HttpTransport │ ─→ HttpResponse (상태 + 본문 + Retry-After)
//!                └───────────────┘ ─→ TransportError (닿지 못했다 · 늦었다 · 읽지 못했다)
//! ```
//!
//! ## 왜 여기 있는가
//!
//! 이 계약은 처음에 한 adapter 안에서 만들어졌고, 그 파일의 첫 줄이 이미
//! *"이 계약은 그 벤더를 모른다"* 고 적고 있었다. 2026-09-08에 **두 번째 사용자**가
//! 생기면서 그 사실이 자리로도 참이 됐다 — 헤더가 필요한 외부 HTTPS API라는 점이 같고,
//! **같은 것을 두 번 정의하지 않는다.**
//!
//! 앞의 adapter는 이 모듈을 재수출한다. 그래서 그쪽 코드는 한 줄도 바뀌지 않았다.
//!
//! **로컬 AI adapter는 여기 오지 않았다.** 그쪽 HTTP 계약에는 헤더가 없고(로컬 서버라
//! 자격증명이 없다), 옮길 이유가 생기기 전에 옮기지 않는다.
//!
//! (이 파일은 벤더 이름을 하나도 부르지 않는다 — INV-9. adapter 경계 테스트가 그것을
//! 검사하며, 이 괄호 안의 문장조차 이름을 쓸 수 없다.)

pub mod http;
pub mod ureq;

pub use http::{
    HttpHeader, HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError,
    RETRY_AFTER_HEADER,
};
pub use ureq::UreqTransport;
