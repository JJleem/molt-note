//! 진행 중인 **AI 노트 생성**을 소유하는 자리와, 고른 provider의 상태를 묻는 자리.
//!
//! 노트 한 건은 1시간 분량 전사면 오래 걸린다 — 로컬 모델에서 분 단위가 될 수 있으므로 이
//! 경계는 생성에 시간 제한을 두지 않는다 (ADR-0008 §12.2). 그동안 사용자가 목록을 보거나 설정을
//! 바꾸는 일이 멈추면 안 되고, 화면을 떠났다 왔다는 이유로 진행 중인 생성이 사라져도 안 된다.
//!
//! **그 두 요구는 전사에서 이미 풀렸고, 여기서 같은 방식을 쓴다** ([`super::Transcriber`] ·
//! `transcriber` 모듈 문서 · R-001). 새로운 실행 방식을 만들지 않는다.
//!
//! ```text
//! start_ai_note ─→ NoteGenerator ─→ 배경 스레드 ──→ ai::run::generate
//!                        │             (자물쇠를 쥐지 않고 도는 구간)
//! ai_note_status ──→ 상태 한 값  ←────┘  끝날 때 결과를 여기에 남긴다
//! ```
//!
//! ## 상태 조회가 생성을 기다리지 않는다
//!
//! 배경 스레드는 [`NoteGenerator::state`]의 자물쇠를 **시작할 때와 끝날 때만** 잡는다.
//! provider가 도는 동안에는 아무 자물쇠도 쥐고 있지 않으므로 [`NoteGenerator::status`]는 즉시
//! 답한다. 저장소 연결도 전사와 같은 이유로 새로 연다 ([`generate_one`]) — 앱이 들고 있는
//! 연결([`super::Storage`])을 생성 내내 붙들면 목록 조회 같은 다른 command가 그 시간만큼 멈춘다.
//!
//! ## provider가 없는 것은 이 경계의 실패가 아니다 (INV-8 · §13)
//!
//! [`NoteGenerator::provider_status`]는 [`Result`]를 돌려주지 않는다. 고르지 않은 것도, 서버가
//! 응답하지 않는 것도 [`AiProviderStatusPayload`]의 상태 하나이며, 그것이 INV-8을 타입으로 적는
//! 방법이다.
//!
//! [`NoteGenerator::start`]도 provider를 이유로 거절하지 않는다. **provider를 고르는 일 자체가
//! 배경 스레드 안에서 일어나기 때문이다** — 그래서 "provider를 고르지 않았다"는 §13의 실패는
//! command의 `Err`가 아니라 `failed` 상태에 실려 화면에 도달한다 (ADR-0008 §13.2). 사용자가 그
//! 상태에서 굳이 생성을 눌렀을 때의 답이며, 조용히 무시되지 않는다 (§13).
//!
//! ## 한 번에 한 건이다
//!
//! 여러 Recording을 줄 세우는 큐는 이 Phase의 범위 밖이다 (PRODUCT-SPEC §16 DEFERRED ·
//! `phase-prompt/04`의 Out of Scope). 그래서 대기열도 스케줄러도 없다 — 생성 중에 들어온 시작
//! 요청은 줄을 서지 않고 [`already_running`]으로 거절된다. 전사와 같은 규칙이다.

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use tauri::Manager;

use crate::ai::prompt::ContextBudget;
use crate::ai::provider::{not_configured, NoteAiProvider};
use crate::ai::provider_for;
use crate::ai::run::{self, Outcome};
use crate::db::{self, settings};
use crate::domain::{Failure, FailureKind, NoteType, RecordingId, Settings};
use crate::platform::app_data_dir::AppDataDirectory;

use super::payload::{AiNoteStatusPayload, AiProviderStatusPayload};

/// 노트 생성 한 건이 있을 수 있는 상태. **다섯 가지가 전부다.**
///
/// 끝난 생성을 지우지 않고 남겨 두는 이유는 화면이 결과를 물어볼 수 있어야 하기 때문이다.
/// 다음 시작이 이 값을 덮는다.
enum NoteGenerationState {
    /// 아직 아무 노트도 만들지 않았다.
    Idle,
    /// 이 녹음의 이 mode 노트를 지금 만들고 있다.
    Running {
        recording_id: String,
        mode: NoteType,
    },
    /// 노트 하나가 **새로** 저장됐다. 이전 노트는 그대로 남는다 (ADR-0008 §9.2).
    Done {
        recording_id: String,
        mode: NoteType,
        ai_note_id: String,
    },
    /// 입력이 될 current Transcript가 없었다 (§7.2). **실패가 아니며 아무것도 저장하지 않았다.**
    NoTranscript {
        recording_id: String,
        mode: NoteType,
    },
    /// 생성이 실패했다. 오디오 · Transcript · 이미 저장된 노트는 그대로다 (INV-2 · INV-3).
    Failed {
        recording_id: String,
        mode: NoteType,
        failure: Failure,
    },
}

/// 노트를 만들 provider를 어디서 얻는가.
///
/// [`super::Transcriber`]가 엔진을 값으로 들고 있는 자리와 같다 — 다른 점은 **제품 경로가
/// provider를 미리 만들어 두지 않는다**는 것이다. 사용자가 설정을 바꾼 뒤 앱을 다시 시작해야
/// 한다면 고친 값이 반영되지 않는 구간이 생기므로, 실제 provider는 물어볼 때마다 지금 설정에서
/// 만들어진다 ([`provider_for`] · 전사가 모델을 시작할 때 읽는 것과 같은 이유다).
#[derive(Clone)]
enum Providers {
    /// 설정이 고른 provider. **앱이 지나는 경로다.**
    Configured,
    /// 고정된 provider 하나. **자동 검증이 지나는 자리다** — 계약이 같은 test double을 넣으면
    /// 실제 AI 서버도 모델도 없이 이 경로 전체를 그대로 지난다 (§18).
    Fixed(Arc<dyn NoteAiProvider>),
}

impl Providers {
    /// 상태를 물어볼 provider. 고르지 않았으면 `None`이며 **그것은 실패가 아니다** (INV-8).
    fn to_ask(&self, settings: &Settings) -> Option<Arc<dyn NoteAiProvider>> {
        match self {
            Self::Configured => provider_for(settings),
            Self::Fixed(provider) => Some(Arc::clone(provider)),
        }
    }

    /// 노트를 만들 provider. 여기서 나오는 실패는 §13의 `provider 미설정`이며, **배경 스레드
    /// 안에서 만들어져 상태값으로 화면에 도달한다** (모듈 문서).
    fn to_generate_with(&self, settings: &Settings) -> Result<Arc<dyn NoteAiProvider>, Failure> {
        match self {
            Self::Configured => {
                let provider = provider_for(settings).ok_or_else(|| {
                    not_configured("노트를 만들 AI provider를 아직 고르지 않았다.")
                })?;

                // 모델을 고르지 않은 채로는 시작하지 않는다. 설치된 모델 중 아무 것이나 골라
                // 대신 쓰지 않는다 — 사용자가 고른 적 없는 모델이 provenance에 남으면 그것은
                // 기록이 아니라 추정이다 (§7.3 · ADR-0008 §11.1).
                if settings.ai_model.is_none() {
                    return Err(not_configured("노트를 만들 모델을 아직 고르지 않았다."));
                }

                Ok(provider)
            }
            Self::Fixed(provider) => Ok(Arc::clone(provider)),
        }
    }
}

/// 앱이 들고 있는 **노트 생성 실행자**. 진행 중인 생성의 소유자다.
///
/// [`super::Transcriber`]와 같은 방식으로 갈라져 있다 — 실제 생성은 [`NoteAiProvider`] 뒤에
/// 있고, 테스트는 그 자리에 자신의 구현을 넣어 **실제 AI 서버도 모델도 없이** 이 경로를 그대로
/// 지난다 ([`Self::with_provider`] · §18).
///
/// [`super::Storage`]처럼 **앱 데이터 디렉터리를 얻지 못한 실패도 값으로 들고 있다.** 그 실패는
/// 생성을 시작하려 할 때 사용자에게 그대로 전달된다 — 앱 시작을 막지 않는다 (§13).
pub struct NoteGenerator {
    /// provider를 어디서 얻는가. 배경 스레드와 공유한다.
    providers: Providers,
    /// DB가 파생되는 뿌리. 얻지 못했다면 그 실패를 들고 있다.
    app_data_dir: Result<AppDataDirectory, Failure>,
    /// 지금 상태 한 값. 배경 스레드와 공유한다.
    state: Arc<Mutex<NoteGenerationState>>,
}

impl NoteGenerator {
    /// Tauri가 결정한 앱 데이터 디렉터리 아래에서 **설정이 고른** provider로 노트를 만든다
    /// (INV-10 · ADR-0008 §11.1).
    pub fn open_for<R, M>(manager: &M) -> Self
    where
        R: tauri::Runtime,
        M: Manager<R>,
    {
        Self::with_providers(
            AppDataDirectory::from_manager(manager).map_err(Into::into),
            Providers::Configured,
        )
    }

    /// 주어진 디렉터리에서 **설정이 고른** provider로 노트를 만든다.
    ///
    /// provider를 고르지 않은 상태를 그대로 지나는 자리다 — INV-8의 자동 검증이 여기로 온다.
    pub fn configured_in(app_data_dir: AppDataDirectory) -> Self {
        Self::with_providers(Ok(app_data_dir), Providers::Configured)
    }

    /// 주어진 디렉터리에서 **주어진 provider 하나로** 노트를 만든다.
    ///
    /// **자동 검증이 지나는 자리다** — 계약이 같은 test double을 넣으면 실제 AI 서버도 모델도
    /// 없이 이 경로 전체를 그대로 지날 수 있다 (§18 · `phase-prompt/04` 요구 16).
    pub fn with_provider(
        app_data_dir: AppDataDirectory,
        provider: impl NoteAiProvider + 'static,
    ) -> Self {
        Self::with_providers(Ok(app_data_dir), Providers::Fixed(Arc::new(provider)))
    }

    fn with_providers(app_data_dir: Result<AppDataDirectory, Failure>, providers: Providers) -> Self {
        Self {
            providers,
            app_data_dir,
            state: Arc::new(Mutex::new(NoteGenerationState::Idle)),
        }
    }

    /// 고른 provider가 지금 어떤 상태인지 — 무엇을 골랐는지 · 쓸 수 있는지 · 어떤 모델이
    /// 있는지 · **로컬인지 외부인지** (§12 · INV-5 · INV-8).
    ///
    /// **[`Result`]가 아니다.** provider를 고르지 않은 것도, 서버가 응답하지 않는 것도 실패가
    /// 아니라 상태값이다 — 이 함수에는 그것을 실패로 돌려줄 채널 자체가 없다.
    ///
    /// 설정 값을 인자로 받는 이유는 하나다 — 이 함수가 도는 동안 앱의 저장소 연결이 잡혀 있으면
    /// 안 된다. 응답하지 않는 서버를 기다리는 시간만큼 목록 조회가 함께 멈추기 때문이다.
    pub fn provider_status(&self, settings: &Settings) -> AiProviderStatusPayload {
        match self.providers.to_ask(settings) {
            None => AiProviderStatusPayload::not_configured(),
            Some(provider) => {
                AiProviderStatusPayload::describing(provider.descriptor(), provider.availability())
            }
        }
    }

    /// Recording 하나의 노트 생성을 시작하고, 그 결과 상태를 돌려준다.
    ///
    /// **돌아오는 것은 노트가 아니라 접수 사실이다.** 실제 생성은 배경 스레드에서 돌고, 이
    /// 호출은 스레드를 만든 뒤 바로 끝난다 — 로컬 모델이 몇 분을 써도 이 command가 IPC를
    /// 붙잡지 않는다 (전사와 같은 규약).
    ///
    /// 이미 생성 중이면 **거절한다** ([`already_running`]). 같은 녹음·같은 mode여도
    /// 마찬가지다 — 조용히 무시하면 사용자는 두 번째 요청이 접수됐는지 알 수 없고, 두 번 돌리면
    /// 같은 입력에 대해 노트가 이유 없이 둘 생긴다 (재생성은 추가이므로 지워지지도 않는다).
    ///
    /// **provider를 고르지 않았다는 이유로는 거절하지 않는다** (INV-8 · 모듈 문서). 그 판정은
    /// 배경 스레드에서 일어나 `failed` 상태로 도달한다.
    ///
    /// 그 녹음이 실재하는지, 입력이 될 Transcript가 있는지도 여기서 묻지 않는다 — 저장소를
    /// 지나는 일은 전부 배경 스레드의 몫이다 ([`crate::ai::run::generate`]).
    pub fn start(&self, recording_id: &str, mode: NoteType) -> Result<AiNoteStatusPayload, Failure> {
        let app_data_dir = self.app_data_dir.as_ref().map_err(Clone::clone)?.clone();

        let recording_id = recording_id.trim();
        if recording_id.is_empty() {
            return Err(Failure::permanent(
                FailureKind::InvalidInput,
                "노트를 만들 녹음을 고르지 않았다.",
            ));
        }

        let mut state = self.state()?;
        if let NoteGenerationState::Running {
            recording_id: running,
            mode: running_mode,
        } = &*state
        {
            return Err(already_running(running, *running_mode, recording_id));
        }
        *state = NoteGenerationState::Running {
            recording_id: recording_id.to_string(),
            mode,
        };
        // 자물쇠를 놓고 나서 스레드를 만든다 — 시작하는 쪽이 자물쇠를 쥔 채로 남지 않게 한다.
        drop(state);

        self.spawn(recording_id.to_string(), mode, app_data_dir);
        Ok(AiNoteStatusPayload::running(recording_id, mode))
    }

    /// 지금 노트 생성이 어떤 상태인지.
    ///
    /// **provider가 도는 동안에도 즉시 답한다.** 이 함수가 잡는 자물쇠를 배경 스레드는 시작할
    /// 때와 끝날 때만 잡기 때문이다 (모듈 문서).
    pub fn status(&self) -> Result<AiNoteStatusPayload, Failure> {
        Ok(match &*self.state()? {
            NoteGenerationState::Idle => AiNoteStatusPayload::idle(),
            NoteGenerationState::Running { recording_id, mode } => {
                AiNoteStatusPayload::running(recording_id, *mode)
            }
            NoteGenerationState::Done {
                recording_id,
                mode,
                ai_note_id,
            } => AiNoteStatusPayload::done(recording_id, *mode, ai_note_id),
            NoteGenerationState::NoTranscript { recording_id, mode } => {
                AiNoteStatusPayload::no_transcript(recording_id, *mode)
            }
            NoteGenerationState::Failed {
                recording_id,
                mode,
                failure,
            } => AiNoteStatusPayload::failed(recording_id, *mode, failure.clone()),
        })
    }

    /// 노트 한 건을 배경 스레드에서 돌린다.
    ///
    /// 스레드는 **결과를 상태 한 값에 남기는 것 말고는 아무것도 하지 않는다.** 화면에 직접
    /// 알리지 않으므로 창이 닫히거나 화면이 바뀌어도 이 스레드가 향할 곳을 잃지 않는다.
    fn spawn(&self, recording_id: String, mode: NoteType, app_data_dir: AppDataDirectory) {
        let providers = self.providers.clone();
        let state = Arc::clone(&self.state);

        thread::spawn(move || {
            let outcome = generate_one(&app_data_dir, &recording_id, mode, &providers);

            // 여기서 처음으로 자물쇠를 잡는다. 생성이 도는 동안에는 쥐고 있지 않았다.
            //
            // 잠그지 못하는 것은 상태를 읽는 쪽이 이미 죽은 채로 남았다는 뜻이며, 그 사실은
            // `status`가 자신의 실패로 알린다 — 여기서 다시 알릴 상대가 없다.
            if let Ok(mut state) = state.lock() {
                *state = match outcome {
                    Ok(Outcome::Generated(note)) => NoteGenerationState::Done {
                        recording_id,
                        mode,
                        ai_note_id: note.id.as_str().to_string(),
                    },
                    Ok(Outcome::NoTranscriptYet) => NoteGenerationState::NoTranscript {
                        recording_id,
                        mode,
                    },
                    Err(failure) => NoteGenerationState::Failed {
                        recording_id,
                        mode,
                        failure,
                    },
                };
            }
        });
    }

    /// 상태 한 값을 빌린다. 이전 호출이 쥔 채 죽었다면 그 사실을 그대로 알린다.
    fn state(&self) -> Result<MutexGuard<'_, NoteGenerationState>, Failure> {
        self.state.lock().map_err(|_| {
            Failure::permanent(
                FailureKind::Storage,
                "노트 생성 상태를 더 이상 알 수 없다. 앱을 다시 시작해야 한다.",
            )
        })
    }
}

/// 배경 스레드가 하는 일 전부 — 저장소를 열고, 설정에서 provider를 고르고, 노트 한 건을 만든다.
///
/// **연결을 새로 연다.** 앱이 들고 있는 연결을 쓰면 생성이 도는 내내 그 자물쇠가 잡혀 있어
/// 목록 조회 같은 다른 command가 함께 멈춘다 ([`super::transcriber`]와 같은 이유이며, 그쪽보다
/// 더 오래 잡힐 수 있다).
///
/// ## 어떤 provider를 쓸지는 **생성을 시작할 때 설정에서 읽는다** (ADR-0008 §11.1)
///
/// 앱을 켤 때 한 번 읽어 들고 있지 않는 이유는 전사와 같다 — 사용자가 설정에서 provider나
/// 모델을 고친 뒤 앱을 다시 시작해야 한다면, 고친 값이 반영되지 않는 구간이 생긴다.
///
/// 여기서 주소를 짓거나 모델을 찾아보지 않는다. 설정 값을 provider 하나로 옮기는 자리는
/// [`provider_for`] 하나이며 (INV-9), 생성의 순서와 영속화 규칙을 아는 자리는
/// [`crate::ai::run`] 하나다.
fn generate_one(
    app_data_dir: &AppDataDirectory,
    recording_id: &str,
    mode: NoteType,
    providers: &Providers,
) -> Result<Outcome, Failure> {
    let connection = db::open_in(app_data_dir)?;
    let configured = settings::load(&connection)?;
    let provider = providers.to_generate_with(&configured)?;

    run::generate(
        &connection,
        &RecordingId::new(recording_id),
        mode,
        provider.as_ref(),
        // 설정에 context 크기 항목이 아직 없으므로 시작값을 쓴다 (ADR-0008 §8.4). 이 값을
        // 여기서 새로 정하지 않는다 — 아는 자리는 `ai::prompt` 하나다.
        ContextBudget::DEFAULT,
    )
}

/// 화면이 보낸 mode 문자열을 domain의 [`NoteType`]으로 읽는다 (§9.5).
///
/// **모르는 값을 추측해서 고르지 않는다.** 세 mode 중 하나가 아니면 §13의 정의된 실패이며,
/// 사용자가 고른 적 없는 종류의 노트가 만들어지지 않는다.
pub(super) fn parse_mode(mode: &str) -> Result<NoteType, Failure> {
    NoteType::parse(mode.trim()).ok_or_else(|| {
        Failure::permanent(FailureKind::InvalidInput, "만들 수 없는 종류의 노트다.")
            .with_detail(format!("mode={mode}"))
    })
}

/// 이미 노트를 만들고 있는데 또 시작하라는 요청이 왔다.
///
/// 두 경우를 구분한다 — **사용자가 할 수 있는 일이 다르기 때문이다.** 같은 녹음이면 이미 하고
/// 있는 일이고, 다른 녹음이면 지금 것이 끝난 뒤에 하면 된다. 줄을 세우지 않는 이유는 여러
/// Recording 일괄 처리 큐가 DEFERRED이기 때문이다 (§16).
///
/// 어느 mode를 돌리고 있는지는 `detail`에 남는다 — 같은 녹음이어도 사용자가 방금 누른 것과
/// 다른 종류일 수 있다.
fn already_running(running: &str, running_mode: NoteType, requested: &str) -> Failure {
    let failure = if running == requested {
        Failure::permanent(
            FailureKind::InvalidInput,
            "이미 이 녹음의 노트를 만들고 있다.",
        )
    } else {
        Failure::retryable(
            FailureKind::InvalidInput,
            "다른 녹음의 노트를 만들고 있다. 그것이 끝난 뒤에 시작할 수 있다.",
        )
    };

    failure.with_detail(format!(
        "runningRecordingId={running} · runningMode={running_mode}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_note_modes_are_the_only_ones_that_can_be_requested() {
        for mode in NoteType::ALL {
            assert_eq!(parse_mode(mode.as_str()).expect("고를 수 있는 mode다"), mode);
        }
        // 앞뒤 공백은 mode를 다른 것으로 만들지 않는다.
        assert_eq!(parse_mode("  summary  ").expect("공백은 벗긴다"), NoteType::Summary);
    }

    #[test]
    fn an_unknown_mode_is_refused_instead_of_being_guessed() {
        let failure = parse_mode("diary").expect_err("모르는 mode는 만들 수 없다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.retryable, "같은 값을 다시 보내도 같다");
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다");
        assert_eq!(failure.detail.as_deref(), Some("mode=diary"));
    }

    #[test]
    fn nothing_is_chosen_when_settings_chose_nothing() {
        // INV-8: 고르지 않은 상태에서 물어볼 provider가 없다는 것은 실패가 아니라 `None`이다.
        assert!(Providers::Configured.to_ask(&Settings::DEFAULT).is_none());
    }

    #[test]
    fn generating_without_a_chosen_provider_is_the_section_13_failure_not_a_panic() {
        let Err(failure) = Providers::Configured.to_generate_with(&Settings::DEFAULT) else {
            panic!("고른 provider가 없으면 노트를 만들 수 없다");
        };

        assert_eq!(failure.kind, FailureKind::AiProviderNotConfigured);
        assert!(!failure.retryable, "설정에서 골라야 풀린다");
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다");
    }

    // **고른 provider가 있는 경우는 여기서 검사하지 않는다.** 그러려면 벤더 식별자를 이름으로
    // 써야 하고, `src/` 아래에서 그것이 허용되는 자리는 adapter를 마운트하는 `ai/mod.rs`
    // 하나이기 때문이다 (INV-9 — adapter 테스트가 그것을 소스에서 확인한다). 그 경로는 통합
    // 테스트가 지난다 (`tests/ai_note_commands.rs`).

    #[test]
    fn a_refused_start_says_which_one_is_running_without_naming_settings_values() {
        let failure = already_running("rec-a", NoteType::Meeting, "rec-b");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.retryable, "지금 것이 끝난 뒤에는 시작할 수 있다");
        assert_eq!(
            failure.detail.as_deref(),
            Some("runningRecordingId=rec-a · runningMode=meeting")
        );

        let same = already_running("rec-a", NoteType::Study, "rec-a");
        assert!(!same.retryable, "이미 하고 있는 일이다");
    }
}
