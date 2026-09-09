//! 명령 하나를 돌리고 그 결과를 값으로 받는 경계.
//!
//! ```text
//! CommandRunner  ← 계약만 여기 있다
//!      ↑
//!      ├ platform::command_runner::SystemCommandRunner   진짜로 프로세스를 띄운다
//!      └ 테스트의 구현                                     실제 프로세스도 로그인도 필요 없다 (§18)
//! ```
//!
//! **실행하는 코드는 여기 없다.** 프로세스를 띄우는 것은 OS 호출이고, 그것은 `platform/`
//! 안에만 있어야 한다 (INV-10). 이 파일이 아는 것은 "명령 하나를 돌리면 결과가 나온다"는
//! 모양뿐이다.
//!
//! `net::HttpTransport`가 HTTP에 대해 하는 일을 프로세스에 대해 한다. **이 경계가 없으면
//! provider를 테스트할 때마다 실제 `claude`가 돌고 구독 사용량이 나간다.**

use std::time::Duration;

/// 돌릴 명령 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest<'a> {
    /// 실행할 프로그램. **경로를 여기서 만들지 않는다** — PATH가 찾는다.
    pub program: &'a str,
    pub arguments: Vec<String>,
    /// 표준 입력으로 넣을 것. 프롬프트가 여기로 간다 — **인자로 넣지 않는다.**
    ///
    /// 인자로 넘기면 길이 한계에 걸리고, 프로세스 목록에 전사가 그대로 보인다.
    pub stdin: &'a str,
    pub timeout: Duration,
}

/// 명령이 끝나고 남긴 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn succeeded(&self) -> bool {
        self.status == Some(0)
    }
}

/// 명령을 돌리지도 못한 경우. **답이 틀린 것과 다르다.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// 그런 프로그램이 없다.
    NotFound,
    /// 시간 안에 끝나지 않았다.
    TimedOut,
    /// 시작했으나 끝을 보지 못했다.
    Failed(String),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found"),
            Self::TimedOut => write!(f, "timed out"),
            Self::Failed(reason) => write!(f, "failed: {reason}"),
        }
    }
}

pub trait CommandRunner: Send + Sync {
    fn run(&self, request: &CommandRequest<'_>) -> Result<CommandOutput, CommandError>;
}
