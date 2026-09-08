//! Claude(Anthropic) Messages API adapter.
//!
//! **로컬 AI adapter와 같은 계약**을 이행하는 두 번째 구현이다 ([`crate::ai::NoteAiProvider`]).
//! 갈리는 것은 [`crate::ai::Locality`]가 `External`이라는 것과, 자격증명이 설정이 아니라
//! SecretStore에서 온다는 것뿐이다.
//!
//! HTTP 계약은 새로 만들지 않고 [`crate::net`]을 쓴다 — 헤더가 필요한 외부
//! HTTPS API라는 점이 같고, **같은 것을 두 번 정의하지 않는다.**

pub mod provider;
pub mod wire;

pub use provider::{AnthropicProvider, PROVIDER_ID, PROVIDER_NAME};
