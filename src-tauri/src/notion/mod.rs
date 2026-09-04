//! Notion adapter — **이 앱에서 Notion API를 아는 유일한 자리다**
//! (ADR-0009 §5 · INV-9의 태도를 그대로 따른다).
//!
//! ```text
//! http.rs      실제 네트워크에 닿는 얇은 경계(trait) — 요청 → 상태 코드 + 본문 + 대기 지시
//! network.rs   그 경계의 실제 구현 — 이 adapter에서 소켓을 여는 유일한 파일
//! testing.rs   그 경계의 결정론적 test double — 자동 검증은 이것만 쓴다 (§18)
//! wire.rs      벤더 지식 — 주소 · 헤더 이름 · 요청 필드 · 오류 코드 해석 (순수 함수)
//! client.rs    요청 조립 · 응답 해석 · 벤더 실패 → §13의 제품 상태
//! chunk.rs     ★ 요청을 보내지 않는다 ★ — 긴 markdown 하나를 요청 하나에 담기는 크기로
//!              순서대로 · 무손실로 나누는 순수 함수와 그 예산 (ADR-0009 §6)
//! ```
//!
//! ```text
//! markdown 문자열 ─→ client ─→ wire::create_page_body ─→ HttpTransport ─→ (Notion)
//!                       │                                     │
//!                       │            테스트에서는 testing::StubServer가 이 자리에 선다
//!                       └─→ wire::created_page_id ─→ PageId (부르는 쪽이 곧바로 영속화한다)
//! ```
//!
//! ## 이 디렉터리 밖으로 나가지 않는 것
//!
//! 주소 · 엔드포인트 경로 · 헤더 이름과 API 버전 · 요청 파라미터 이름 · 오류 코드 문자열.
//! 밖으로 나가는 것은 [`PageId`] · [`RetryAfter`] · §13의 공통 실패뿐이며, 그래서 Notion API가
//! 바뀔 때 흔들리는 것은 이 디렉터리 하나다.
//!
//! **여기까지만 만든다.** 요청을 보내는 다섯 파일은 요청 하나를 보내고 그 답을 옮길 뿐이다 —
//! 몇 번 다시 보낼지도, 무엇을 영속화할지도, 언제 멈췄다 갈지도 여기 없다. 그것은 전송 순서를
//! 소유한 쪽의 일이며 (ADR-0009 §8 · §9.2), 그래서 그 파일들에는 재시도 횟수도 `sleep`도 없다.
//!
//! **[`chunk`]는 그 다섯과 다른 자리에 있다.** 문서를 나누는 규칙과 그 예산은 요청 하나에
//! 무엇이 담기는가의 문제라서 이 디렉터리에 두지만 (§6), 그 모듈은 요청을 만들지도 보내지도
//! 않는다 — 문자열에서 문자열 조각을 만들 뿐이고, 나눈 것을 실제로 보내는 순서는 여전히
//! 이 디렉터리 밖이다.
//!
//! ## 자동 검증이 실제 Notion을 요구하지 않는다 (PRODUCT-SPEC §18)
//!
//! HTTP 왕복이 [`http::HttpTransport`] 하나 뒤에 있으므로, double을 세우면 요청 조립부터 실패
//! 매핑까지 **전 경로**가 네트워크 없이 지나간다. **어떤 자동 테스트도 실제 Notion에 요청하지
//! 않으며, 어떤 워크스페이스도 오염시키지 않는다** (`phase-prompt/05` Important Rules).
//! 그 사실은 `tests/notion_adapter.rs`가 소스에서 확인한다.

pub mod chunk;
pub mod client;
pub mod http;
pub mod network;
pub mod testing;
pub mod wire;

pub use chunk::{split_markdown, AtomKind, OversizedAtom, CHUNK_MAX_BLOCK_UNITS, CHUNK_MAX_BYTES};
pub use client::{NotionClient, NotionFailure, RetryAfter, NOTION_FAILURE_KINDS};
pub use http::{HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError};
pub use network::UreqNotionTransport;
pub use wire::{ApiErrorCode, ConnectedIdentity, PageId, NOTION_VERSION};

// **test double은 여기서 다시 내보내지 않는다** — AI adapter가 따르는 규칙과 같다.
// 쓰는 쪽은 `notion::testing`을 이름 그대로 부른다.
