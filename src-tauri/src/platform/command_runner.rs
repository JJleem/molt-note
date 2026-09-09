//! 명령 하나를 실제로 돌리는 자리 (INV-10).
//!
//! **이 저장소에서 프로세스를 띄우는 곳은 여기와 `file_manager.rs` 둘뿐이다.**
//! 계약은 `ai::claude_cli::runner`에 있고, 이 파일은 그 계약을 OS로 실행할 뿐이다.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;

use crate::ai::claude_cli::runner::{CommandError, CommandOutput, CommandRequest, CommandRunner};


#[derive(Debug, Default, Clone, Copy)]
pub struct SystemCommandRunner;

impl SystemCommandRunner {
    pub const fn new() -> Self {
        Self
    }
}

impl CommandRunner for SystemCommandRunner {
    fn run(&self, request: &CommandRequest<'_>) -> Result<CommandOutput, CommandError> {
        let mut child = Command::new(request.program)
            .args(&request.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => CommandError::NotFound,
                _ => CommandError::Failed(error.to_string()),
            })?;

        // **stdin을 먼저 닫는다.** 닫지 않으면 자식이 입력이 끝나기를 기다리고, 우리는 자식이
        // 끝나기를 기다린다 — 둘 다 영원히 기다린다.
        {
            let mut stdin = child.stdin.take().ok_or_else(|| {
                CommandError::Failed("표준 입력을 열지 못했다".to_owned())
            })?;
            stdin
                .write_all(request.stdin.as_bytes())
                .map_err(|error| CommandError::Failed(error.to_string()))?;
        }

        // 기다리는 일은 다른 스레드가 한다. **이 스레드는 시간 제한을 지킬 수 있어야 한다** —
        // `wait_with_output`은 시간 제한을 모른다.
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(child.wait_with_output());
        });

        match receiver.recv_timeout(request.timeout) {
            Ok(Ok(output)) => Ok(CommandOutput {
                status: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }),
            Ok(Err(error)) => Err(CommandError::Failed(error.to_string())),
            // 기다리기를 그만둘 뿐 자식을 죽이지는 않는다. 죽이려면 `child`를 여기서 들고
            // 있어야 하는데, 그러면 위의 스레드와 소유권이 부딪힌다. **[미검증]** 시간 제한에
            // 걸린 뒤 남은 프로세스가 실제로 어떻게 되는지 아직 관측하지 않았다.
            Err(mpsc::RecvTimeoutError::Timeout) => Err(CommandError::TimedOut),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(CommandError::Failed("기다리던 쪽이 사라졌다".to_owned()))
            }
        }
    }
}
