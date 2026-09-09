//! 이 기기에 설치·로그인된 공식 CLI를 불러 노트를 만드는 provider.
//!
//! ```text
//! Molt Note ──프롬프트(stdin)──▶ claude -p --model … ──▶ 노트 JSON(stdout)
//!                                     │
//!                                     └ 자기 인증을 자기가 한다
//! ```
//!
//! ## 이 provider는 자격증명을 만지지 않는다
//!
//! **그것이 이 provider의 존재 이유다.** API 키를 저장하지도, 읽지도, 보내지도 않는다.
//! 로그인은 CLI가 자기 방식으로 이미 해 두었고, 이 앱은 그 사실을 확인하지도 못한다 —
//! 확인할 필요가 없기 때문이다.
//!
//! 그래서 [`SecretKey`](crate::platform::secret_store::SecretKey)에 새 항목이 생기지 않는다.
//! 설정 화면의 API 키 칸도 이 provider와 무관하다.
//!
//! ## 왜 HTTP adapter와 따로 있는가
//!
//! `ai::anthropic`은 같은 서비스를 부르지만 **과금 주체가 다르다** — 그쪽은 API 크레딧이고
//! 이쪽은 CLI가 알아서 하는 것이다. 2026-09-09에 그 차이가 `status=400 · 크레딧 부족`으로
//! 드러났고, 그래서 두 경로가 나란히 있다. 사용자가 어느 쪽인지 고른다.
//!
//! ## 전사는 기기 밖으로 나간다
//!
//! CLI를 거치더라도 전사 텍스트는 결국 외부로 간다. [`Locality::External`]이며, 화면은 그
//! 사실을 사용자에게 그대로 알린다 (§12 · INV-5). **로컬 provider가 아니다.**

use std::sync::Arc;
use std::time::Duration;

use super::runner::{CommandError, CommandOutput, CommandRequest, CommandRunner};
use crate::ai::provider::{
    not_configured, rejected_response, request_failed_temporarily, unreachable, Availability,
    Locality, NoteAiProvider, NoteGeneration, NoteRequest, ProviderDescriptor,
};
use crate::ai::ResponseRejection;
use crate::domain::Failure;

/// 설정에 저장되는 식별자.
pub const PROVIDER_ID: &str = "claude-cli";

/// 사람이 읽는 이름. **무엇으로 과금되는지가 이름에 들어간다** — 그것이 다른 경로와
/// 갈리는 유일한 이유이기 때문이다.
pub const PROVIDER_NAME: &str = "Claude Code (구독)";

/// 부를 프로그램. **경로를 적지 않는다** — PATH가 찾는다.
pub const PROGRAM: &str = "claude";

/// 모델을 고르지 않았을 때 쓸 값.
pub const DEFAULT_MODEL: &str = "claude-sonnet-5";

/// 노트 한 건을 기다리는 시간.
///
/// 2시간 회의도 이 안에 끝나야 한다. 넉넉하게 잡는 이유는 **끝나지 않는 것보다 늦는 것이
/// 낫기 때문**이다 — 생성은 이미 배경 스레드에서 돌고 UI를 막지 않는다.
pub const GENERATE_TIMEOUT: Duration = Duration::from_secs(600);

/// 설치 여부만 물어볼 때의 시간.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

pub struct ClaudeCliProvider {
    model: String,
    runner: Arc<dyn CommandRunner>,
}

impl ClaudeCliProvider {
    pub fn new(model: String, runner: Arc<dyn CommandRunner>) -> Self {
        Self { model, runner }
    }

    fn model_name(&self) -> &str {
        if self.model.trim().is_empty() {
            DEFAULT_MODEL
        } else {
            self.model.trim()
        }
    }
}

impl NoteAiProvider for ClaudeCliProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: PROVIDER_ID.to_owned(),
            name: PROVIDER_NAME.to_owned(),
            // CLI를 거치더라도 전사는 외부로 간다. 이 값을 로컬로 적으면 화면이 거짓말을 한다.
            locality: Locality::External,
        }
    }

    /// **설치되어 있는지만 묻는다.**
    ///
    /// 로그인 상태까지 확인하려면 실제 프롬프트를 한 번 보내야 하고, 그것은 사용량을 쓴다.
    /// 준비 여부를 확인하는 것만으로 사용량을 쓰지 않는다 — `ai::anthropic`이 과금 요청을
    /// 보내지 않는 것과 같은 규칙이다. 로그인이 안 되어 있다는 사실은 실제 생성에서 드러난다.
    fn availability(&self) -> Availability {
        let request = CommandRequest {
            program: PROGRAM,
            arguments: vec!["--version".to_owned()],
            stdin: "",
            timeout: PROBE_TIMEOUT,
        };

        match self.runner.run(&request) {
            Ok(output) if output.succeeded() => Availability::Ready {
                models: vec![self.model_name().to_owned()],
            },
            Ok(output) => Availability::Unavailable(command_failed(&output)),
            Err(error) => Availability::Unavailable(run_failure(error)),
        }
    }

    fn generate_note(&self, request: &NoteRequest<'_>) -> Result<NoteGeneration, Failure> {
        let model = self.model_name().to_owned();

        // 프롬프트는 domain이 만든다 — 이 adapter는 그것을 어디로 넣는지만 안다.
        let prompt = crate::ai::build_prompt(request.mode, request.transcript_text());

        let command = CommandRequest {
            program: PROGRAM,
            // **전사를 인자로 넘기지 않는다.** stdin으로 간다 — 인자는 길이 한계가 있고,
            // 프로세스 목록에 전사가 그대로 드러난다.
            arguments: vec!["-p".to_owned(), "--model".to_owned(), model.clone()],
            stdin: &prompt,
            timeout: GENERATE_TIMEOUT,
        };

        let output = self.runner.run(&command).map_err(run_failure)?;
        if !output.succeeded() {
            return Err(command_failed(&output));
        }

        // CLI는 프롬프트가 금했는데도 코드펜스를 씌워 보낼 때가 있다 (2026-09-09 관측).
        // 벗기는 일은 `ai::parse_note`가 이미 한다 — 여기서 또 하지 않는다.
        let note = crate::ai::parse_note(request.mode, &output.stdout)
            .map_err(|rejection: ResponseRejection| rejected_response(&rejection))?;

        Ok(NoteGeneration {
            note,
            // **응답이 어느 모델인지 말하지 않는다.** CLI는 텍스트만 준다. 그래서 우리가
            // 요청한 값을 적는다 — 지어내는 것이 아니라 실제로 요청한 값이다.
            model,
        })
    }
}

/// 명령을 돌리지도 못했다.
fn run_failure(error: CommandError) -> Failure {
    match error {
        CommandError::NotFound => not_configured(
            "Claude Code CLI를 찾지 못했다. 설치한 뒤 다시 고른다.",
        ),
        CommandError::TimedOut => {
            request_failed_temporarily("Claude Code CLI가 제때 끝나지 않았다")
        }
        CommandError::Failed(_) => unreachable("Claude Code CLI를 실행하지 못했다"),
    }
    .with_detail(error.to_string())
}

/// 돌긴 했는데 0이 아닌 상태로 끝났다.
///
/// **stderr를 통째로 옮기지 않는다.** 거기에 우리가 보낸 프롬프트가 섞여 올 수 있고, 그
/// 프롬프트에는 전사가 들어 있다 — `ai::anthropic`이 오류 본문에 대해 지키는 규칙과 같다.
fn command_failed(output: &CommandOutput) -> Failure {
    let said = first_line(&output.stderr);
    let detail = match (output.status, said) {
        (Some(status), Some(said)) => format!("exit={status} · {said}"),
        (Some(status), None) => format!("exit={status}"),
        (None, Some(said)) => format!("exit=신호로 종료됨 · {said}"),
        (None, None) => "exit=신호로 종료됨".to_owned(),
    };

    unreachable("Claude Code CLI가 노트를 만들지 못했다. 로그인되어 있는지 확인한다.")
        .with_detail(detail)
}

/// stderr의 **첫 줄만** 가져온다. 길면 자른다.
fn first_line(stderr: &str) -> Option<String> {
    const MAX_CHARS: usize = 200;

    let line = stderr.lines().map(str::trim).find(|line| !line.is_empty())?;
    if line.chars().count() <= MAX_CHARS {
        return Some(line.to_owned());
    }
    Some(line.chars().take(MAX_CHARS).collect::<String>() + "…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::testing::CONTRACT_TRANSCRIPT;
    use crate::domain::{FailureKind, NoteType};
    use std::sync::Mutex;

    /// 명령을 돌리는 척하고 **무엇으로 불렸는지 기록한다.**
    struct FakeRunner {
        answer: Result<CommandOutput, CommandError>,
        seen: Mutex<Vec<(Vec<String>, String)>>,
    }

    impl FakeRunner {
        fn saying(stdout: &str) -> Self {
            Self::finishing(0, stdout, "")
        }

        fn finishing(status: i32, stdout: &str, stderr: &str) -> Self {
            Self {
                answer: Ok(CommandOutput {
                    status: Some(status),
                    stdout: stdout.to_owned(),
                    stderr: stderr.to_owned(),
                }),
                seen: Mutex::new(Vec::new()),
            }
        }

        fn failing(error: CommandError) -> Self {
            Self {
                answer: Err(error),
                seen: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<(Vec<String>, String)> {
            self.seen.lock().expect("기록을 읽을 수 있어야 한다").clone()
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, request: &CommandRequest<'_>) -> Result<CommandOutput, CommandError> {
            self.seen
                .lock()
                .expect("기록할 수 있어야 한다")
                .push((request.arguments.clone(), request.stdin.to_owned()));
            self.answer.clone()
        }
    }

    const NOTE_JSON: &str = r#"{"overview":"개요","keyDiscussions":["하나"],"decisions":[],"actionItems":[],"openQuestions":[]}"#;

    fn generate(runner: Arc<FakeRunner>, model: &str) -> Result<NoteGeneration, Failure> {
        let provider = ClaudeCliProvider::new(model.to_owned(), runner);
        provider.generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
    }

    #[test]
    fn the_transcript_goes_through_standard_input_and_never_through_an_argument() {
        // 지키려는 것: **전사가 프로세스 목록에 드러나지 않는다.** 인자로 넘기면 `ps`에
        // 그대로 보이고, 길이 한계에도 걸린다.
        let runner = Arc::new(FakeRunner::saying(NOTE_JSON));
        generate(Arc::clone(&runner), "claude-sonnet-5").expect("노트가 나와야 한다");

        let calls = runner.calls();
        assert_eq!(calls.len(), 1, "한 번만 부른다");
        let (arguments, stdin) = &calls[0];

        assert!(stdin.contains("회의"), "전사가 stdin으로 가야 한다");
        for argument in arguments {
            assert!(
                !argument.contains("회의"),
                "전사가 인자에 실리면 안 된다: {argument}",
            );
        }
        assert_eq!(
            arguments,
            &vec![
                "-p".to_owned(),
                "--model".to_owned(),
                "claude-sonnet-5".to_owned()
            ],
        );
    }

    #[test]
    fn the_model_that_was_asked_for_is_the_one_recorded() {
        // CLI는 어느 모델이었는지 말해 주지 않는다. 그래서 **요청한 값**을 적는다 —
        // 지어낸 값이 provenance에 남으면 그것은 기록이 아니라 추정이다.
        let runner = Arc::new(FakeRunner::saying(NOTE_JSON));
        let made = generate(runner, "claude-sonnet-5").expect("노트가 나와야 한다");
        assert_eq!(made.model, "claude-sonnet-5");
    }

    #[test]
    fn no_chosen_model_falls_back_to_the_declared_default() {
        let runner = Arc::new(FakeRunner::saying(NOTE_JSON));
        let made = generate(Arc::clone(&runner), "   ").expect("노트가 나와야 한다");
        assert_eq!(made.model, DEFAULT_MODEL);
        assert!(runner.calls()[0].0.contains(&DEFAULT_MODEL.to_owned()));
    }

    #[test]
    fn a_fenced_answer_is_still_a_note() {
        // **2026-09-09 관측**: 프롬프트가 코드펜스를 금했는데도 CLI가 씌워 보냈다.
        // 벗기는 일은 `parse_note`가 한다 — 여기서 또 하지 않는다는 것을 못박는다.
        let fenced = format!("```json\n{NOTE_JSON}\n```");
        let runner = Arc::new(FakeRunner::saying(&fenced));
        generate(runner, "claude-sonnet-5").expect("펜스가 있어도 노트가 나와야 한다");
    }

    #[test]
    fn a_missing_cli_says_what_to_install_and_is_not_retryable() {
        let runner = Arc::new(FakeRunner::failing(CommandError::NotFound));
        let failure = generate(runner, "").expect_err("실패해야 한다");

        assert_eq!(failure.kind, FailureKind::AiProviderNotConfigured);
        assert!(
            failure.message.contains("설치"),
            "무엇을 해야 하는지 말해야 한다: {}",
            failure.message,
        );
    }

    #[test]
    fn a_cli_that_exits_nonzero_says_to_check_the_login_and_can_be_retried() {
        let runner = Arc::new(FakeRunner::finishing(1, "", "Invalid API key · Please run /login"));
        let failure = generate(runner, "").expect_err("실패해야 한다");

        assert_eq!(failure.kind, FailureKind::AiProviderUnreachable);
        assert!(failure.message.contains("로그인"), "{}", failure.message);
    }

    #[test]
    fn a_failure_detail_is_bounded_so_a_long_transcript_cannot_ride_along() {
        // 지키려는 것: **stderr에 우리가 보낸 프롬프트가 섞여 와도 전사가 통째로 실려
        // 나가지 않는다.** `ai::anthropic`이 오류 본문에 대해 지키는 규칙과 같다.
        //
        // **이것은 상한이지 부재 보장이 아니다.** 짧은 전사라면 200자 안에 들어와 detail에
        // 남을 수 있다. 그 한계는 `first_line`에 적혀 있다.
        let long = CONTRACT_TRANSCRIPT.repeat(40);
        let runner = Arc::new(FakeRunner::finishing(2, "", &format!("could not send: {long}")));
        let failure = generate(runner, "").expect_err("실패해야 한다");

        let detail = failure.detail.clone().unwrap_or_default();
        assert!(
            detail.chars().count() <= 220,
            "상한을 지켜야 한다: {}자",
            detail.chars().count(),
        );
        assert!(detail.ends_with('…'), "잘렸다는 표시가 있어야 한다: {detail}");
        assert!(
            detail.chars().count() < long.chars().count(),
            "stderr 전체가 실려 나가면 안 된다",
        );
    }

    #[test]
    fn a_failure_detail_still_says_what_went_wrong() {
        // 상한이 있다고 해서 진단을 버리지 않는다 — 그것이 오늘 400을 못 읽게 만든 원인이었다.
        let runner = Arc::new(FakeRunner::finishing(1, "", "Invalid API key · Please run /login"));
        let failure = generate(runner, "").expect_err("실패해야 한다");

        let detail = failure.detail.clone().unwrap_or_default();
        assert!(detail.contains("exit=1"), "{detail}");
        assert!(detail.contains("/login"), "CLI가 한 말이 남아야 한다: {detail}");
    }

    #[test]
    fn asking_whether_it_is_ready_does_not_spend_the_subscription() {
        // 지키려는 것: **준비 여부를 묻는 것만으로 사용량을 쓰지 않는다.**
        // 로그인까지 확인하려면 실제 프롬프트를 보내야 하고, 그것은 사용량이 나간다.
        let runner = Arc::new(FakeRunner::saying("2.1.266 (Claude Code)"));
        let provider =
            ClaudeCliProvider::new("claude-sonnet-5".to_owned(), Arc::clone(&runner) as Arc<dyn CommandRunner>);

        let availability = provider.availability();
        assert!(matches!(availability, Availability::Ready { .. }));

        let calls = runner.calls();
        assert_eq!(calls[0].0, vec!["--version".to_owned()], "버전만 물어본다");
        assert!(calls[0].1.is_empty(), "프롬프트를 보내지 않는다");
    }

    #[test]
    fn the_transcript_leaves_the_device_even_through_the_cli() {
        // 지키려는 것: **화면이 거짓말하지 않는다** (§12 · INV-5). CLI를 거치더라도
        // 전사는 외부로 간다. 로컬로 적으면 사용자가 그 사실을 모른 채 보내게 된다.
        let runner = Arc::new(FakeRunner::saying(NOTE_JSON));
        let provider = ClaudeCliProvider::new(String::new(), runner);
        assert_eq!(provider.descriptor().locality, Locality::External);
    }
}
