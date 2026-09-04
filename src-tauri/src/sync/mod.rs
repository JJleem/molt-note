//! 기록 하나를 **Notion 페이지로 보내는 실행 순서** (PRODUCT-SPEC §10 ·
//! `docs/ADR-0009-notion-and-export.md` §8 · §9).
//!
//! ```text
//! pace.rs   얼마나 기다리고 몇 번까지 다시 보내는가 — 순수한 값과 얇은 대기 경계 하나
//! run.rs    저장소 · 렌더러 · adapter를 잇는 순서 하나. 무엇을 언제 영속화하는지가 여기 있다
//! ```
//!
//! ## 왜 `notion` 디렉터리 안이 아닌가
//!
//! [`crate::notion`]은 **Notion API를 아는 유일한 자리**이고, 거기까지만 만든다고 스스로
//! 적어 두었다 — 요청 하나를 보내고 그 답을 옮길 뿐, 몇 번 다시 보낼지도 · 무엇을 영속화할지도 ·
//! 언제 멈췄다 갈지도 그 디렉터리에 없다 (`notion` 모듈 문서 · ADR-0009 §8 · §9.2). 그 셋을
//! 소유하는 자리가 여기다. 그래서 이 모듈에는 주소도 헤더 이름도 오류 코드도 없다.
//!
//! ```text
//! Recording ─→ current Transcript ─→ (있으면) 최신 AI Note ─→ export::markdown::render
//!                                                                       │
//!                        notion::split_markdown ←───────────────────────┘
//!                                  │
//!             첫 chunk → notion::NotionClient::create_page  ─→ PageId ★ 즉시 영속화
//!             나머지   → …::append_markdown (순서대로)      ─→ sent_chunks 하나씩
//!                                  │
//!                        NotionSync (§7) · recordings.notion_status
//! ```
//!
//! ## 산출물이 하나다 (ADR-0009 §14)
//!
//! 보내는 문자열은 로컬 Markdown export가 쓰는 것과 **같은 렌더러의 같은 결과**다
//! ([`crate::export::markdown::render`]). 두 번째 렌더러도, "Notion용 변형"도 없다 — 그래서
//! 렌더러의 테스트가 곧 Notion 본문의 테스트다.
//!
//! ## AI가 없어도 보낸다 (INV-8)
//!
//! AI Note는 있으면 넣고 없으면 넣지 않는 선택 입력이다. 이 모듈에는 provider도 AI 설정도
//! 들어오지 않으므로 **노트가 없다는 이유로 거절할 수단 자체가 없다** ([`crate::export::run`]과
//! 같은 자리에서 같은 규칙으로 고른다).
//!
//! ## 오디오는 어떤 형태로도 나가지 않는다 (INV-6)
//!
//! 이 모듈이 보내는 것은 [`crate::export::markdown::render`]가 만든 문자열 하나뿐이고, 그
//! 렌더러는 오디오 경로도 바이트도 문서에 넣지 않는다. 여기에는 파일을 읽는 코드가 **없다** —
//! `std::fs`가 등장하지 않으므로 오디오를 실을 방법이 문법적으로 없다.
//!
//! ## 실패는 아무것도 잃지 않는다 (INV-3)
//!
//! ```text
//! recordings           notion_status와 updated_at 말고는 그대로
//! transcripts · segments · ai_notes   그대로 (읽기 질의뿐이다)
//! 원본 오디오 파일     그대로 (읽지 않는다)
//! 이미 만들어진 Notion 페이지         그대로 (지우거나 바꾸는 경로가 adapter에 없다)
//! ```
//!
//! 저장소에 쓰는 자리는 [`crate::db::store::save_notion_sync`]와
//! [`crate::db::store::update_recording_statuses`] 둘뿐이며, 그 둘은 Notion 전송 상태와 세
//! 후처리 상태 말고는 만지지 않는다.

pub mod pace;
pub mod run;

pub use pace::{SleepingWaiter, Waiter};
pub use run::{send, Confirmation, Destination, Sent};
