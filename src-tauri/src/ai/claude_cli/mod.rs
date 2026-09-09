//! 이 기기의 공식 CLI를 통해 노트를 만드는 provider (2026-09-09).
//!
//! `ai::anthropic`과 같은 서비스를 부르지만 **과금 주체가 다르다.** 그쪽은 API 크레딧,
//! 이쪽은 CLI가 알아서 한다. 그 차이가 실제로 드러난 날의 기록은 `provider.rs`에 있다.
//!
//! **이 앱은 자격증명을 만지지 않는다** — 그것이 이 경로의 존재 이유다.

pub mod provider;
pub mod runner;

pub use provider::{ClaudeCliProvider, DEFAULT_MODEL, PROGRAM, PROVIDER_ID, PROVIDER_NAME};
pub use runner::{CommandError, CommandOutput, CommandRequest, CommandRunner};

/// 실제로 프로세스를 띄우는 구현은 `platform/` 안에 있다 (INV-10).
pub use crate::platform::command_runner::SystemCommandRunner;
