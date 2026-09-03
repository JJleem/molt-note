//! 진행 중인 **전사**를 소유하는 자리 (`phase-prompt/03` 요구 3).
//!
//! 전사 한 건은 1시간 분량 녹음이면 오래 걸린다. 그동안 사용자가 목록을 보거나 설정을 바꾸는
//! 일이 멈추면 안 되고, 화면을 떠났다 왔다는 이유로 진행 중인 전사가 사라져도 안 된다.
//! 두 요구는 같은 방식으로 풀린다 — **backend가 소유하고, 화면은 물어본다.**
//!
//! ```text
//! start_transcription ─→ Transcriber ─→ 배경 스레드 ──→ transcription::run::transcribe
//!                             │             (자물쇠를 쥐지 않고 도는 구간)
//! transcription_status ──→ 상태 한 값  ←────┘  끝날 때 결과를 여기에 남긴다
//! ```
//!
//! [`Recorder`]와 같은 규약이다 (`crate::commands` 모듈 문서 · R-001) — Tauri managed state가
//! 값을 들고, 화면 컴포넌트는 핸들을 갖지 않으며, 지금 상태는 command로 물어본다.
//!
//! ## 상태 조회가 전사를 기다리지 않는다
//!
//! 배경 스레드는 [`Transcriber::state`]의 자물쇠를 **시작할 때와 끝날 때만** 잡는다. 엔진이
//! 도는 동안에는 아무 자물쇠도 쥐고 있지 않으므로 [`Transcriber::status`]는 즉시 답한다.
//!
//! 저장소 연결도 같은 이유로 새로 연다 ([`transcribe_one`]). 앱이 들고 있는 연결
//! ([`super::Storage`])을 전사 내내 붙들면 목록 조회 같은 다른 command가 그 시간만큼 멈춘다 —
//! UI를 막지 않으려고 스레드를 만들어 놓고 저장소에서 다시 막는 셈이 된다.
//!
//! ## 한 번에 한 건이다
//!
//! 여러 Recording을 줄 세우는 큐는 이 Phase의 범위 밖이다 (PRODUCT-SPEC §16 DEFERRED ·
//! `phase-prompt/03`의 Out of Scope). 그래서 **대기열도, 스케줄러도, 우선순위도 없다** —
//! 전사 중에 들어온 시작 요청은 줄을 서지 않고 [`already_running`]으로 거절된다.
//! 조용히 무시하면 사용자는 자신이 누른 것이 접수됐는지 알 수 없다 (§13).

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use tauri::Manager;

use crate::db::{self, settings};
use crate::domain::{Failure, FailureKind, RecordingId};
use crate::platform::app_data_dir::AppDataDirectory;
use crate::transcription::{run, ModelChoice, TranscriptionEngine, WhisperEngine};

use super::payload::TranscriptionStatusPayload;

/// 전사 한 건이 있을 수 있는 상태. **네 가지가 전부다.**
///
/// 끝난 전사를 지우지 않고 남겨 두는 이유는 화면이 결과를 물어볼 수 있어야 하기 때문이다 —
/// 성공이면 어느 Transcript가 생겼는지, 실패면 무엇이 실패했는지. 다음 시작이 이 값을 덮는다.
enum TranscriptionState {
    /// 아직 아무 전사도 하지 않았다.
    Idle,
    /// 이 녹음을 지금 전사하고 있다.
    Running { recording_id: String },
    /// 이 녹음의 전사가 끝났고 Transcript가 하나 추가됐다 (§7.1 · §7.2).
    Done {
        recording_id: String,
        transcript_id: String,
    },
    /// 이 녹음의 전사가 실패했다. 원본과 Recording은 그대로다 (INV-1 · INV-3).
    Failed {
        recording_id: String,
        failure: Failure,
    },
}

/// 앱이 들고 있는 **전사 실행자**. 진행 중인 전사의 소유자다.
///
/// [`Recorder`]와 같은 방식으로 갈라져 있다 — 실제 추론은 [`TranscriptionEngine`] 뒤에 있고,
/// 테스트는 그 자리에 자신의 구현을 넣어 **실제 whisper도 모델도 없이** 이 경로를 그대로
/// 지난다 ([`Self::with_engine`] · §18).
///
/// [`super::Storage`]처럼 **앱 데이터 디렉터리를 얻지 못한 실패도 값으로 들고 있다.** 그 실패는
/// 전사를 시작하려 할 때 사용자에게 그대로 전달된다 — 앱 시작을 막지 않는다 (§13).
///
/// [`Recorder`]: super::Recorder
pub struct Transcriber {
    /// 실제로 추론하는 쪽. 배경 스레드와 공유하므로 [`Arc`]다.
    engine: Arc<dyn TranscriptionEngine>,
    /// DB와 모델 디렉터리가 파생되는 뿌리. 얻지 못했다면 그 실패를 들고 있다.
    app_data_dir: Result<AppDataDirectory, Failure>,
    /// 지금 상태 한 값. 배경 스레드와 공유한다.
    state: Arc<Mutex<TranscriptionState>>,
}

impl Transcriber {
    /// Tauri가 결정한 앱 데이터 디렉터리 아래에서 실제 엔진으로 전사한다 (INV-10 · ADR-0007).
    pub fn open_for<R, M>(manager: &M) -> Self
    where
        R: tauri::Runtime,
        M: Manager<R>,
    {
        Self {
            engine: Arc::new(WhisperEngine::new()),
            app_data_dir: AppDataDirectory::from_manager(manager).map_err(Into::into),
            state: Arc::new(Mutex::new(TranscriptionState::Idle)),
        }
    }

    /// 주어진 디렉터리에서 주어진 엔진으로 전사한다.
    ///
    /// **자동 검증이 지나는 자리다** — 계약이 같은 test double을 넣으면 실제 whisper 바이너리도
    /// 수 GB짜리 모델도 없이 이 경로 전체를 그대로 지날 수 있다 (§18 · `phase-prompt/03` 요구 9).
    pub fn with_engine(
        app_data_dir: AppDataDirectory,
        engine: impl TranscriptionEngine + 'static,
    ) -> Self {
        Self {
            engine: Arc::new(engine),
            app_data_dir: Ok(app_data_dir),
            state: Arc::new(Mutex::new(TranscriptionState::Idle)),
        }
    }

    /// Recording 하나의 전사를 시작하고, 그 결과 상태를 돌려준다.
    ///
    /// **돌아오는 것은 전사 결과가 아니라 접수 사실이다.** 실제 전사는 배경 스레드에서 돌고,
    /// 이 호출은 스레드를 만든 뒤 바로 끝난다 — 그래서 1시간짜리 녹음을 걸어도 이 command가
    /// IPC를 붙잡지 않는다.
    ///
    /// 이미 전사 중이면 **거절한다** ([`already_running`]). 같은 Recording이어도 마찬가지다 —
    /// 조용히 무시하면 사용자는 두 번째 요청이 접수됐는지 알 수 없고, 두 번 돌리면 같은 녹음에
    /// 대해 Transcript가 이유 없이 둘 생긴다.
    ///
    /// 그 녹음이 실재하는지는 여기서 묻지 않는다 — 저장소를 지나는 일은 전부 배경 스레드의
    /// 몫이며, 없는 녹음은 `failed` 상태와 §13의 실패로 드러난다
    /// ([`crate::transcription::run::transcribe`]).
    pub fn start(&self, recording_id: &str) -> Result<TranscriptionStatusPayload, Failure> {
        let app_data_dir = self.app_data_dir.as_ref().map_err(Clone::clone)?.clone();

        let recording_id = recording_id.trim();
        if recording_id.is_empty() {
            return Err(Failure::permanent(
                FailureKind::InvalidInput,
                "전사할 녹음을 고르지 않았다.",
            ));
        }

        let mut state = self.state()?;
        if let TranscriptionState::Running {
            recording_id: running,
        } = &*state
        {
            return Err(already_running(running, recording_id));
        }
        *state = TranscriptionState::Running {
            recording_id: recording_id.to_string(),
        };
        // 자물쇠를 놓고 나서 스레드를 만든다 — 시작하는 쪽이 자물쇠를 쥔 채로 남지 않게 한다.
        drop(state);

        self.spawn(recording_id.to_string(), app_data_dir);
        Ok(TranscriptionStatusPayload::running(recording_id))
    }

    /// 지금 전사가 어떤 상태인지.
    ///
    /// **엔진이 도는 동안에도 즉시 답한다.** 이 함수가 잡는 자물쇠를 배경 스레드는 시작할 때와
    /// 끝날 때만 잡기 때문이다 (모듈 문서).
    pub fn status(&self) -> Result<TranscriptionStatusPayload, Failure> {
        Ok(match &*self.state()? {
            TranscriptionState::Idle => TranscriptionStatusPayload::idle(),
            TranscriptionState::Running { recording_id } => {
                TranscriptionStatusPayload::running(recording_id)
            }
            TranscriptionState::Done {
                recording_id,
                transcript_id,
            } => TranscriptionStatusPayload::done(recording_id, transcript_id),
            TranscriptionState::Failed {
                recording_id,
                failure,
            } => TranscriptionStatusPayload::failed(recording_id, failure),
        })
    }

    /// 전사 한 건을 배경 스레드에서 돌린다.
    ///
    /// 스레드는 **결과를 상태 한 값에 남기는 것 말고는 아무것도 하지 않는다.** 화면에 직접
    /// 알리지 않으므로 창이 닫히거나 화면이 바뀌어도 이 스레드가 향할 곳을 잃지 않는다.
    fn spawn(&self, recording_id: String, app_data_dir: AppDataDirectory) {
        let engine = Arc::clone(&self.engine);
        let state = Arc::clone(&self.state);

        thread::spawn(move || {
            let outcome = transcribe_one(&app_data_dir, &recording_id, engine.as_ref());

            // 여기서 처음으로 자물쇠를 잡는다. 전사가 도는 동안에는 쥐고 있지 않았다.
            //
            // 잠그지 못하는 것은 상태를 읽는 쪽이 이미 죽은 채로 남았다는 뜻이며, 그 사실은
            // `status`가 자신의 실패로 알린다 — 여기서 다시 알릴 상대가 없다.
            if let Ok(mut state) = state.lock() {
                *state = match outcome {
                    Ok(completed) => TranscriptionState::Done {
                        recording_id,
                        transcript_id: completed.transcript.id.as_str().to_string(),
                    },
                    Err(failure) => TranscriptionState::Failed {
                        recording_id,
                        failure,
                    },
                };
            }
        });
    }

    /// 상태 한 값을 빌린다. 이전 호출이 쥔 채 죽었다면 그 사실을 그대로 알린다.
    fn state(&self) -> Result<MutexGuard<'_, TranscriptionState>, Failure> {
        self.state.lock().map_err(|_| {
            Failure::permanent(
                FailureKind::TranscriptionEngineFailed,
                "전사 상태를 더 이상 알 수 없다. 앱을 다시 시작해야 한다.",
            )
        })
    }
}

/// 배경 스레드가 하는 일 전부 — 저장소를 열고 전사 한 건을 돌린다.
///
/// **연결을 새로 연다.** 앱이 들고 있는 연결을 쓰면 전사가 도는 내내 그 자물쇠가 잡혀 있어
/// 목록 조회 같은 다른 command가 함께 멈춘다. 같은 DB 파일에 대한 두 번째 연결이므로
/// 쓰기가 겹칠 수 있고, 그때 즉시 실패하지 않고 기다리는 것은 [`crate::db::open`]이 정한다.
///
/// ## 어떤 모델을 쓸지는 **전사를 시작할 때 설정에서 읽는다** (ADR-0007 §8.2 · TASK-029)
///
/// 앱을 켤 때 한 번 읽어 들고 있지 않는 이유는 하나다 — 사용자가 설정에서 모델을 고른 뒤
/// 앱을 다시 시작해야 한다면, 고친 값이 반영되지 않는 구간이 생긴다.
///
/// 읽은 값을 여기서 파일 하나로 해석하지는 않는다. **경로를 짓는 자리는 여전히
/// [`crate::transcription::model`] 하나이며** (INV-10), 이 함수는 설정 값과 모델 디렉터리를
/// [`ModelChoice`]로 묶어 넘기기만 한다. 고른 모델이 없는 상태는 조용한 skip이 아니라
/// §13의 정의된 실패(`transcriptionModelMissing`)로 화면에 도달한다.
///
/// 전사의 순서와 영속화 규칙은 여기에 없다 — 그것을 아는 자리는
/// [`crate::transcription::run`] 하나다.
fn transcribe_one(
    app_data_dir: &AppDataDirectory,
    recording_id: &str,
    engine: &dyn TranscriptionEngine,
) -> Result<run::Completed, Failure> {
    let mut connection = db::open_in(app_data_dir)?;
    let models_dir = app_data_dir.models_dir();
    let configured = settings::load(&connection)?.transcription_model;

    run::transcribe(
        &mut connection,
        &RecordingId::new(recording_id),
        engine,
        ModelChoice {
            models_dir: &models_dir,
            configured: configured.as_deref(),
        },
    )
}

/// 이미 전사 중인데 또 시작하라는 요청이 왔다.
///
/// 두 경우를 구분한다 — **사용자가 할 수 있는 일이 다르기 때문이다.** 같은 녹음이면 이미
/// 하고 있는 일이라 다시 눌러도 달라지지 않고, 다른 녹음이면 지금 것이 끝난 뒤에 하면 된다.
/// 줄을 세우지 않는 이유는 여러 Recording 동시 전사 큐가 DEFERRED이기 때문이다 (§16).
fn already_running(running: &str, requested: &str) -> Failure {
    let failure = if running == requested {
        Failure::permanent(
            FailureKind::InvalidInput,
            "이미 이 녹음을 전사하고 있다.",
        )
    } else {
        Failure::retryable(
            FailureKind::InvalidInput,
            "다른 녹음을 전사하고 있다. 그것이 끝난 뒤에 시작할 수 있다.",
        )
    };

    failure.with_detail(format!("runningRecordingId={running}"))
}
