//! **녹음이 도는 동안 전사를 함께 돌리는 실행자** (2026-09-14).
//!
//! ```text
//! Recorder::start ─→ LiveTranscriber::begin ─→ 배경 스레드
//!                                                 └ live_run::advance 를 되풀이
//! Recorder::stop  ─→ LiveTranscriber::finish ─→ 남은 꼬리까지 전사하고 결과를 돌려준다
//! ```
//!
//! ## 녹음이 절대 위험해지지 않는다
//!
//! **이 모듈의 어떤 실패도 녹음을 멈추지 않는다.** 모델을 못 올려도, 파일을 못 읽어도,
//! 전사가 실패해도 녹음은 계속된다 — 실시간 전사는 **덤이고 녹음이 본체다.**
//!
//! 그래서 [`Self::begin`]은 실패를 돌려주지 않는다. 시작하지 못했다는 사실은 상태로
//! 남고, 화면은 "지금은 받아 적지 않는다"고 말할 뿐이다.
//!
//! 읽기만 한다는 것도 같은 약속의 일부다 — 원본 오디오는 읽기 전용이며(INV-1) 이 경로는
//! 파일을 열어 읽을 뿐 쓰는 쪽을 기다리게 하지 않는다 (`transcription::growing_wav`).
//!
//! ## 왜 스레드 하나인가
//!
//! 창은 순서대로 이어져야 한다 — 앞 창의 꼬리가 다음 창의 문맥이고, `done_frames`가
//! 하나뿐이다. 여럿이 나눠 돌면 그 둘이 깨진다. **[실측 2026-09-14] 전사는 약 10배속**
//! 이므로 하나로도 밀리지 않는다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::domain::Failure;
use crate::transcription::engine::{LanguageChoice, TranscriptionEngine};
use crate::transcription::growing_wav::GrowingWav;
use crate::transcription::live_run::{self, LiveProgress, Step};
use crate::transcription::model::ModelFile;
use crate::transcription::parse::TranscriptSegment;

/// 전사할 것이 없을 때 다음에 물어볼 때까지 쉬는 시간.
///
/// **짧을 이유가 없다.** 창 하나가 30초이므로 그보다 촘촘히 물어봐야 답은 같다.
/// 길면 녹음이 끝난 뒤 마지막 창이 늦게 잡힌다 — 그 사이를 고른 값이다.
const IDLE_PAUSE: Duration = Duration::from_secs(2);

/// 실패한 뒤 다시 시도하기까지 쉬는 시간.
///
/// **실패해도 바로 포기하지 않는다.** 파일이 아직 안 열렸거나 디스크가 잠깐 바쁜 것일 수
/// 있다. 되풀이가 로그를 채울 걱정은 [`MAX_CONSECUTIVE_FAILURES`]가 막으므로, 이 값은
/// **정말 안 되는 경우를 빨리 알려 주는 쪽**으로 잡는다 — 둘을 곱한 만큼이 포기까지
/// 걸리는 시간이고, 그동안 화면은 "받아 적는 중"이라고 말하고 있다.
const RETRY_PAUSE: Duration = Duration::from_secs(2);

/// 연달아 이만큼 실패하면 **실시간 전사만** 그만둔다. 녹음은 계속된다.
const MAX_CONSECUTIVE_FAILURES: usize = 5;

/// 지금 실시간 전사가 어떤 상태인가.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveState {
    /// 돌고 있지 않다. **정상 상태다** — 녹음 중이 아니거나 이 기능을 쓰지 않는다.
    Idle,
    /// 녹음과 함께 돌고 있다.
    Running,
    /// 녹음은 끝났고 **남은 구간을 마저 전사하는 중이다** (2026-09-14).
    ///
    /// 정지는 이미 성공했다 — 파일도 레코드도 저장됐다. 이 단계는 배경에서 돌며 화면을
    /// 붙잡지 않는다.
    Finishing,
    /// 시작하지 못했거나 도중에 그만뒀다. **녹음과는 무관하다.**
    GaveUp(Failure),
}

/// 실시간 전사가 남긴 것 전부.
///
/// **결과와 provenance가 같은 자리에서 나온다** — 무엇으로 받아 적었는지는 나중에 따로
/// 물어볼 수 없다. `finish`가 끝나면 모델도 언어도 놓이기 때문이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveOutcome {
    pub progress: LiveProgress,
    /// 이 전사를 만든 엔진 (§7 · ADR-0007 §8.2.4).
    pub engine_id: String,
    /// 실제로 쓴 모델 파일의 이름. 설정 값이 아니다.
    pub model_id: String,
}

/// 실시간 전사가 쓰는 값 전부. 스레드와 바깥이 함께 본다.
#[derive(Debug)]
struct Shared {
    progress: Mutex<LiveProgress>,
    state: Mutex<LiveState>,
    /// 그만 돌라는 신호. **스레드는 이것만 본다.**
    stop: AtomicBool,
}

/// 앱이 들고 있는 실시간 전사 실행자.
///
/// [`Transcriber`](super::Transcriber)와 같은 모양이다 — 실제 엔진은 trait 뒤에 있고,
/// 테스트는 자기 구현을 넣어 **실제 모델 없이** 이 경로를 그대로 지난다 (§18).
pub struct LiveTranscriber {
    /// 모델이 놓인 자리. 얻지 못했다면 그 실패를 들고 있다 — [`super::Storage`]와 같다.
    app_data_dir: Result<crate::platform::app_data_dir::AppDataDirectory, Failure>,
    engine: Arc<dyn TranscriptionEngine>,
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
    /// 지금 따라 읽는 녹음 파일. 마무리할 때 같은 파일을 본다.
    source: Mutex<Option<Source>>,
}

#[derive(Debug, Clone)]
struct Source {
    path: PathBuf,
    model: ModelFile,
    language: LanguageChoice,
}

impl LiveTranscriber {
    /// 앱이 쓰는 실행자. **모델을 물고 있는 엔진**을 쓴다 — 30초마다 다시 올릴 수 없다
    /// (`transcription::whisper::WhisperEngine::holding_the_model`).
    pub fn open_for<R, M>(manager: &M) -> Self
    where
        R: tauri::Runtime,
        M: tauri::Manager<R>,
    {
        Self {
            app_data_dir: crate::platform::app_data_dir::AppDataDirectory::from_manager(manager)
                .map_err(Into::into),
            ..Self::with_engine(crate::transcription::whisper::WhisperEngine::holding_the_model())
        }
    }

    /// 주어진 디렉터리에서 모델을 찾는 실행자. **테스트가 쓰는 자리다.**
    pub fn with_app_data_dir(
        app_data_dir: crate::platform::app_data_dir::AppDataDirectory,
        engine: impl TranscriptionEngine + 'static,
    ) -> Self {
        Self {
            app_data_dir: Ok(app_data_dir),
            ..Self::with_engine(engine)
        }
    }

    pub fn with_engine(engine: impl TranscriptionEngine + 'static) -> Self {
        Self {
            app_data_dir: Err(Failure::permanent(
                crate::domain::FailureKind::Storage,
                "이 실행자에는 모델 디렉터리가 주어지지 않았다.",
            )),
            engine: Arc::new(engine),
            shared: Arc::new(Shared {
                progress: Mutex::new(LiveProgress::default()),
                state: Mutex::new(LiveState::Idle),
                stop: AtomicBool::new(false),
            }),
            worker: Mutex::new(None),
            source: Mutex::new(None),
        }
    }

    /// 녹음이 시작됐다. 따라 적기 시작한다.
    ///
    /// **실패를 돌려주지 않는다** (모듈 문서). 시작하지 못하면 [`LiveState::GaveUp`]으로
    /// 남고 녹음은 그대로 간다.
    pub fn begin(&self, path: &Path, model: ModelFile, language: Option<&str>) {
        self.reset();

        let language = LanguageChoice::from_setting(language);
        let source = Source {
            path: path.to_path_buf(),
            model,
            language,
        };
        if let Ok(mut held) = self.source.lock() {
            *held = Some(source.clone());
        }

        self.set_state(LiveState::Running);

        let engine = Arc::clone(&self.engine);
        let shared = Arc::clone(&self.shared);
        let handle = thread::spawn(move || follow(&source, engine.as_ref(), &shared));

        if let Ok(mut worker) = self.worker.lock() {
            *worker = Some(handle);
        }
    }

    /// 설정을 읽어 시작한다. **여기가 실패해도 녹음은 계속된다** (모듈 문서).
    ///
    /// 모델 해석 규칙을 새로 만들지 않는다 — 배치 전사가 쓰는
    /// [`model::resolve`](crate::transcription::model::resolve) 그대로다.
    pub fn begin_from_settings(
        &self,
        path: &Path,
        configured_model: Option<&str>,
        language: Option<&str>,
    ) {
        let models_dir = match self.app_data_dir.as_ref() {
            Ok(app_data_dir) => app_data_dir.models_dir(),
            Err(failure) => {
                self.reset();
                self.set_state(LiveState::GaveUp(failure.clone()));
                return;
            }
        };

        match crate::transcription::model::resolve(&models_dir, configured_model) {
            Ok(model) => self.begin(path, model, language),
            // 모델이 없거나 읽을 수 없다. **녹음을 막지 않는다** — 화면이 그 사실을 말한다.
            Err(failure) => {
                self.reset();
                self.set_state(LiveState::GaveUp(failure));
            }
        }
    }

    /// 지금까지 받아 적은 것. **화면이 묻는 자리다.**
    pub fn snapshot(&self) -> Vec<TranscriptSegment> {
        self.shared
            .progress
            .lock()
            .map(|progress| progress.segments.clone())
            .unwrap_or_default()
    }

    /// 이 녹음을 따라 적고 있었는가. **정지가 어느 길로 갈지 정하는 값이다.**
    ///
    /// 받아 적던 것이 있으면 그것이 최종본이 되고, 없으면 지금까지처럼 자동 전사가
    /// 판단한다 — 둘 다 하면 같은 오디오를 두 번 전사하게 된다.
    pub fn was_following(&self) -> bool {
        matches!(self.state(), LiveState::Running)
    }

    pub fn state(&self) -> LiveState {
        self.shared
            .state
            .lock()
            .map(|state| state.clone())
            .unwrap_or(LiveState::Idle)
    }

    /// 녹음이 끝났다. **배경에서** 남은 구간을 마저 전사하고 저장한다 (2026-09-14).
    ///
    /// ## 왜 배경인가
    ///
    /// 2026-09-14에 이것을 정지 경로에서 곧바로 돌렸다가 **UI가 7분 멈췄다.** 실시간
    /// 전사가 밀려 있으면 남은 구간이 녹음 전체일 수 있고, 그것을 명령 스레드에서
    /// 돌리면 창이 통째로 굳는다.
    ///
    /// **정지는 이미 성공했다.** 파일도 레코드도 저장된 뒤에 불린다 — 여기서 무슨 일이
    /// 일어나든 그 사실은 되돌아가지 않는다 (R-002).
    pub fn finish_in_background(self: &Arc<Self>, recording_id: String) {
        if !matches!(self.state(), LiveState::Running | LiveState::GaveUp(_)) {
            return;
        }
        self.set_state(LiveState::Finishing);

        let live = Arc::clone(self);
        thread::spawn(move || {
            let outcome = live.finish();
            live.save(&recording_id, outcome);
        });
    }

    /// 마무리한 결과를 저장한다. **실패해도 아무것도 잃지 않는다** — 이미 저장된 녹음은
    /// 그대로이고, 사용자는 전사 탭에서 다시 전사할 수 있다 (INV-8).
    fn save(&self, recording_id: &str, outcome: Option<LiveOutcome>) {
        let Some(outcome) = outcome else { return };
        if outcome.progress.segments.is_empty() {
            return;
        }
        let Ok(app_data_dir) = self.app_data_dir.as_ref() else { return };
        let Ok(mut connection) = crate::db::open_in(app_data_dir) else { return };

        let _ = crate::transcription::run::save_live(
            &mut connection,
            &crate::domain::RecordingId::new(recording_id),
            outcome.progress.language,
            outcome.progress.segments,
            outcome.engine_id,
            outcome.model_id,
            0,
        );
    }

    /// 남은 꼬리까지 전사하고 결과를 돌려준다.
    ///
    /// **명령 스레드에서 부르지 않는다.** 실시간 전사가 밀려 있으면 남은 구간이 녹음
    /// 전체일 수 있고, 그것을 여기서 기다리면 창이 굳는다 — 2026-09-14에 7분 굳었다.
    /// 앱이 쓰는 자리는 [`Self::finish_in_background`]다.
    ///
    /// 돌고 있지 않았으면 `None`이다 — 실패가 아니다.
    pub fn finish(&self) -> Option<LiveOutcome> {
        self.shared.stop.store(true, Ordering::Release);

        // 스레드가 창 하나를 붙잡고 있을 수 있다. **기다린다** — 끝내지 않고 결과를 읽으면
        // 마지막 창이 빠지거나 `done_frames`가 어긋난다.
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(handle) = worker.take() {
                let _ = handle.join();
            }
        }

        let source = self.source.lock().ok()?.take()?;
        let mut progress = self.shared.progress.lock().ok()?.clone();

        // 마지막 꼬리는 여기서 전사한다 — 스레드가 아니라. 그래야 "정지했는데 아직
        // 몇 초가 비어 있다"가 생기지 않는다.
        match GrowingWav::open(&source.path) {
            Ok(wav) => {
                if let Err(failure) = live_run::finish(
                    &wav,
                    self.engine.as_ref(),
                    &source.model,
                    &source.language,
                    &mut progress,
                ) {
                    // 꼬리를 못 읽어도 **앞서 받아 적은 것은 그대로 돌려준다.**
                    self.set_state(LiveState::GaveUp(failure));
                }
            }
            Err(failure) => self.set_state(LiveState::GaveUp(failure)),
        }

        if matches!(self.state(), LiveState::Running) {
            self.set_state(LiveState::Idle);
        }
        Some(LiveOutcome {
            progress,
            engine_id: self.engine.engine_id(),
            model_id: source.model.id().to_owned(),
        })
    }

    fn reset(&self) {
        self.shared.stop.store(false, Ordering::Release);
        if let Ok(mut progress) = self.shared.progress.lock() {
            *progress = LiveProgress::default();
        }
    }

    fn set_state(&self, state: LiveState) {
        if let Ok(mut held) = self.shared.state.lock() {
            *held = state;
        }
    }
}

/// 배경 스레드가 하는 일 전부.
///
/// **여기서 나오는 어떤 실패도 녹음을 건드리지 않는다.** 연달아 실패하면 실시간 전사만
/// 그만두고, 그 사실이 상태로 남는다.
fn follow(source: &Source, engine: &dyn TranscriptionEngine, shared: &Shared) {
    let wav = match GrowingWav::open(&source.path) {
        Ok(wav) => wav,
        Err(failure) => {
            give_up(shared, failure);
            return;
        }
    };

    let mut failures = 0_usize;

    while !shared.stop.load(Ordering::Acquire) {
        // **자물쇠를 쥔 채 전사하지 않는다.** 전사는 수 초가 걸리고, 그동안 화면이
        // 지금까지 받아 적은 것을 읽지 못하면 화면이 멎은 것처럼 보인다.
        let mut working = match shared.progress.lock() {
            Ok(progress) => progress.clone(),
            Err(_) => return,
        };

        match live_run::advance(&wav, engine, &source.model, &source.language, &mut working) {
            Ok(Step::Advanced) => {
                failures = 0;
                if let Ok(mut progress) = shared.progress.lock() {
                    *progress = working;
                }
                // 바로 다음 창을 본다 — 밀려 있다면 따라잡아야 한다.
                continue;
            }
            Ok(Step::NotYet) => {
                failures = 0;
                sleep_unless_stopped(shared, IDLE_PAUSE);
            }
            Err(failure) => {
                failures += 1;
                if failures >= MAX_CONSECUTIVE_FAILURES {
                    give_up(shared, failure);
                    return;
                }
                sleep_unless_stopped(shared, RETRY_PAUSE);
            }
        }
    }
}

/// 쉬되, 그만하라는 신호가 오면 바로 깬다.
fn sleep_unless_stopped(shared: &Shared, total: Duration) {
    const TICK: Duration = Duration::from_millis(200);
    let mut slept = Duration::ZERO;
    while slept < total {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        thread::sleep(TICK);
        slept += TICK;
    }
}

fn give_up(shared: &Shared, failure: Failure) {
    if let Ok(mut state) = shared.state.lock() {
        *state = LiveState::GaveUp(failure);
    }
}
