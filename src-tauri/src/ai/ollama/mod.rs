//! 로컬 Ollama adapter — **이 앱에서 이 벤더를 아는 유일한 자리다** (INV-9 · ADR-0008 §16.1).
//!
//! ```text
//! http.rs      실제 프로세스·네트워크에 닿는 얇은 경계(trait) — 요청 → 상태 코드 + 본문
//! network.rs   그 경계의 실제 구현 — **이 저장소에서 소켓을 여는 유일한 파일**
//! testing.rs   그 경계의 결정론적 test double — 자동 검증은 이것만 쓴다 (§18)
//! wire.rs      벤더 지식 — 엔드포인트 경로 · 요청 필드 이름 · 응답 해석 (순수 함수)
//! provider.rs  NoteAiProvider 이행 · 벤더 실패 → §13의 domain 공통 실패
//! ```
//!
//! ```text
//! NoteRequest ─→ provider ─→ wire::generate_body ─→ HttpTransport ─→ (사용자의 로컬 서버)
//!                    │                                    │
//!                    │                  테스트에서는 testing::StubServer가 이 자리에 선다
//!                    └─→ wire::generated_note ─→ ai::note::parse_note ─→ StructuredNote
//! ```
//!
//! ## 이 디렉터리 밖으로 나가지 않는 것
//!
//! 엔드포인트 경로 · 요청 파라미터 이름 · 응답 필드 해석 · 상태 코드 해석 · 벤더 식별자.
//! 밖으로 나가는 것은 [`crate::ai::provider`]의 계약과 §13의 공통 실패뿐이며, 그래서 벤더가
//! 바뀔 때 흔들리는 것은 이 디렉터리 하나다 (ADR-0008 §1).
//!
//! **연결 대상(host/port)과 모델은 여기서 정하지 않는다.** 둘 다 [`OllamaProvider::new`]의
//! 인자이고, 값은 P4의 설정에서 온다 — 이 디렉터리에 주소가 하나도 없다는 것이 그 사실의
//! 표현이다 (ADR-0008 §11.1).
//!
//! ## 자동 검증이 실제 서버를 요구하지 않는다 (§18)
//!
//! HTTP 왕복이 [`http::HttpTransport`] 하나 뒤에 있으므로, double을 세우면 요청 생성부터 실패
//! 매핑까지 **전 경로**가 서버 없이 지나간다. 서버가 없을 때 테스트를 건너뛰지도 않는다 —
//! 연결 불가는 skip 조건이 아니라 §13의 정의된 실패이며 그 자체가 검증 대상이다.

pub mod http;
pub mod network;
pub mod provider;
pub mod testing;
pub mod wire;

pub use http::{HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError};
pub use network::UreqTransport;
pub use provider::{OllamaProvider, PROVIDER_ID};

// **test double은 여기서 다시 내보내지 않는다** — `crate::ai::testing`이 같은 규칙을 따르는
// 이유와 같다. 쓰는 쪽은 `ollama::testing`을 이름 그대로 부른다.
