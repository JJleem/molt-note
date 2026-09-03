//! frontend가 부를 수 있는 동작의 전부 (Tauri command 경계).
//!
//! **SQL은 이 경계를 넘지 않는다.** frontend는 질의를 보내지 않고 여기 선언된 동작만 부른다.
//! 저장소를 아는 코드는 [`crate::db`] 안에만 있고, 프론트엔드는 그 존재를 알지 않는다
//! (`docs/ADR-0001-local-persistence.md` · PRODUCT-SPEC §12).
//!
//! Phase 1의 표면은 두 가지였다 — **recording CRUD**와 **settings**.
//! 여기에 **입력 장치 열거**와 **녹음 session**(시작 · 일시정지 · 재개 · 정지 · 상태 조회),
//! 그리고 **레코드와 파일이 어긋난 상태의 감지**가 더해진다.
//! Phase 3은 **전사**(시작 · 상태 조회) 둘과 **저장된 Transcript 읽기**([`get_transcript`])를
//! 더한다 ([`Transcriber`]). 마지막 것은 읽기뿐이다 — Transcript는 immutable이므로
//! 고치거나 지우는 이름은 이 표면에 없다 (§7.1 · INV-2).
//! AI · Notion을 위한 command도 그 기능이 존재하는 Phase가 함께 추가한다.
//!
//! ## 정지의 성공은 "파일이 확정됐다"는 뜻이다 (R-002)
//!
//! [`finish_recording`]이 그 순서를 갖고 있다 — **파일을 확정하고 · 확인하고 · 레코드를
//! 저장한다.** 셋 중 하나라도 성립하지 않으면 정지는 사용자에게 보이는 실패이며, 그 실패는
//! **확정된 파일이 어디에 남아 있는지** 함께 말한다.
//!
//! **어느 실패 경로도 오디오 파일을 지우지 않는다** (INV-3 · INV-4 · R-004). 파일과 레코드가
//! 어긋난 두 상태를 각각 어떻게 다루는지는
//! `docs/ADR-0004-recording-session-lifecycle.md` §10~§13에 있다.
//!
//! 정지가 성공한 **뒤에** 전사가 자동으로 시작될 수 있다. 그것은 설정 값 하나가 정하며
//! (`automatic_transcription`), 꺼져 있으면 아무 전사도 시작되지 않는다
//! ([`start_automatic_transcription`]). 자동으로 시작하지 않는 것과 전사할 수 없는 것은
//! 다른 말이다 — 수동 전사는 이 설정과 무관하게 언제나 [`start_transcription`]으로 할 수 있다.
//!
//! ## 진행 중인 녹음을 소유하는 것은 여기다
//!
//! [`Recorder`]는 Tauri managed state로 앱이 들고 있다 (`crate::run`). 화면 컴포넌트가
//! session 핸들을 갖지 않으므로 **화면이 unmount되어도 녹음이 사라지지 않는다** (R-001).
//! 화면은 command로 시작·정지를 요청하고 [`Recorder::status`]로 지금 상태를 물어볼 뿐이다.
//! 이 결정과 근거는 `docs/ADR-0004-recording-session-lifecycle.md`에 있다.
//!
//! **진행 중인 전사도 같은 규약을 따른다** ([`Transcriber`] · `transcriber` 모듈 문서).
//! 다른 점은 하나다 — 전사는 오래 걸리므로 배경 스레드에서 돌며, 그동안 이 표면의 다른
//! command가 함께 멈추지 않는다.
//!
//! ## 저장소 초기화 실패는 앱을 죽이지 않는다
//!
//! 저장소는 앱 시작 시 한 번 연다. 열지 못하면 [`Storage`]는 **실패를 값으로 들고 있다가**
//! 모든 command 응답으로 돌려준다. 초기화 경로에 `unwrap` · `expect` · `panic!`이 없는 이유가
//! 이것이다 — 여기서 죽으면 사용자는 아무 설명도 받지 못하고, 실패는 콘솔에만 남는다 (§13).

pub mod payload;
pub mod transcriber;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use tauri::{Manager, State};

use crate::audio::capture::{self, ActiveCapture};
use crate::audio::{
    catalog, finalized, InputDeviceSource, RecordingSession, SampleSource, SystemInputDevices,
    SystemSampleSource,
};
use crate::db::{self, settings, store};
use crate::domain::{Failure, FailureKind, RecordingId, RecordingView, TranscriptId};
use crate::platform::app_data_dir::AppDataDirectory;
use crate::platform::clock::{Clock, MonotonicClock};
use crate::platform::microphone::{
    self, MicrophoneAccess, MicrophonePermission, SystemMicrophonePermission,
};

pub use payload::{
    CaptureReportPayload, InputDevicePayload, MissingAudioPayload, NewRecording, RecordingPayload,
    SessionStatusPayload, SettingsPayload, StoppedRecordingPayload, TranscriptPayload,
    TranscriptSegmentPayload, TranscriptionStatusPayload,
};
pub use transcriber::Transcriber;

/// 앱이 들고 있는 로컬 저장소.
///
/// 열려 있거나, **열지 못한 이유를 들고 있거나** 둘 중 하나다. 세 번째 상태는 없다.
pub struct Storage {
    state: StorageState,
}

enum StorageState {
    /// 저장소가 열려 있다. command는 이 연결 하나를 차례로 쓴다.
    Ready(Mutex<Connection>),
    /// 초기화가 실패했다. 이 실패는 command를 부를 때마다 사용자에게 그대로 전달된다.
    Unavailable(Failure),
}

impl Storage {
    /// 주어진 앱 데이터 디렉터리에서 저장소를 연다. **실패해도 panic하지 않는다.**
    ///
    /// Tauri 없이 부를 수 있으므로 실패 경로를 그대로 테스트할 수 있다.
    pub fn open(app_data_dir: &AppDataDirectory) -> Self {
        let state = match initialize(app_data_dir) {
            Ok(connection) => StorageState::Ready(Mutex::new(connection)),
            Err(failure) => StorageState::Unavailable(failure),
        };
        Self { state }
    }

    /// Tauri가 결정한 앱 데이터 디렉터리에서 저장소를 연다 (INV-10).
    ///
    /// 경로 자체를 얻지 못하는 것도 실패의 한 형태이며, 마찬가지로 값으로 보관한다.
    pub fn open_for<R, M>(manager: &M) -> Self
    where
        R: tauri::Runtime,
        M: Manager<R>,
    {
        match AppDataDirectory::from_manager(manager) {
            Ok(app_data_dir) => Self::open(&app_data_dir),
            Err(error) => Self {
                state: StorageState::Unavailable(error.into()),
            },
        }
    }

    /// 초기화가 실패했다면 그 실패. 정상이면 `None`이다.
    pub fn failure(&self) -> Option<&Failure> {
        match &self.state {
            StorageState::Ready(_) => None,
            StorageState::Unavailable(failure) => Some(failure),
        }
    }

    /// 저장된 녹음을 최근 것부터 돌려준다. 하나도 없으면 빈 목록이다 (오류가 아니다).
    pub fn list_recordings(&self) -> Result<Vec<RecordingPayload>, Failure> {
        let connection = self.connection()?;
        let views = store::list_recordings(&connection)?;
        Ok(views.into_iter().map(RecordingPayload::from).collect())
    }

    /// 녹음 하나를 돌려준다. 그런 id가 없으면 `None`이다 (오류가 아니다).
    pub fn recording(&self, id: &str) -> Result<Option<RecordingPayload>, Failure> {
        let connection = self.connection()?;
        let view = store::load_recording_view(&connection, &RecordingId::new(id))?;
        Ok(view.map(RecordingPayload::from))
    }

    /// Transcript 하나를 **segment까지** 돌려준다. 그런 id가 없으면 `None`이다 (오류가 아니다).
    ///
    /// 화면은 이것으로 `Recording.current_transcript_id`가 가리키는 Transcript를 읽는다
    /// (§7.2 · `phase-prompt/03` 요구 6). **읽기뿐이다** — 저장소가 내놓는 Transcript 쓰기
    /// 경로는 추가([`store::append_transcript`]) 하나이고 그것을 부르는 자리는
    /// [`crate::transcription::run`]뿐이므로, 이 command가 열려도 화면이 Transcript를
    /// 고치거나 지울 수 있게 되지는 않는다 (§7.1 · INV-2).
    pub fn transcript(&self, id: &str) -> Result<Option<TranscriptPayload>, Failure> {
        let connection = self.connection()?;
        let transcript = store::load_transcript(&connection, &TranscriptId::new(id))?;
        Ok(transcript.map(TranscriptPayload::from))
    }

    /// 녹음 하나를 새로 저장하고, 저장된 모습을 그대로 돌려준다.
    ///
    /// 식별자와 시각은 저장소가 만든다 — 프론트엔드가 정하지 않는다.
    pub fn create_recording(&self, recording: NewRecording) -> Result<RecordingPayload, Failure> {
        self.insert(|_| recording)
    }

    /// 방금 확정된 녹음 하나를 레코드로 남긴다 (Phase 2B 요구사항 5 · 6).
    ///
    /// **파일이 확인된 뒤에만 부른다.** 이 함수는 파일을 다시 확인하지 않는다 — 순서를 아는
    /// 자리는 [`finish_recording`] 하나이며, 그 순서가 정책이다
    /// (`docs/ADR-0004-recording-session-lifecycle.md` §11).
    ///
    /// 남는 값은 요구사항이 열거한 여섯 가지다 — title · duration(일시정지 제외) ·
    /// audioPath · audioFormat · microphone · createdAt. 저장 경로는 Phase 1의 저장소
    /// 그대로이며([`store::insert_recording`]), 여기에 별도의 쓰기 경로를 만들지 않는다.
    ///
    /// `title`이 비어 있으면 저장 시각에서 제목을 만든다 ([`title_for`]) — 제목이 빈 레코드는
    /// 저장되지 않으므로([`validated`]) 이름 없는 녹음이 조용히 유실되지 않게 한다.
    pub fn save_capture(
        &self,
        capture: &CaptureReportPayload,
        title: Option<&str>,
    ) -> Result<RecordingPayload, Failure> {
        let chosen = title.map(str::trim).filter(|title| !title.is_empty());

        self.insert(|created_at| NewRecording {
            title: match chosen {
                Some(title) => title.to_string(),
                None => title_for(created_at),
            },
            // 일시정지 구간을 뺀 길이다. 그것을 세는 곳은 상태 기계 한 곳뿐이다.
            duration_ms: capture.duration_ms,
            audio_path: capture.output_path.clone(),
            audio_format: capture::EXTENSION.to_string(),
            // 실제로 열린 장치의 이름이다 — 고른 이름이 아니다.
            microphone: Some(capture.device_label.clone()),
        })
    }

    /// **레코드는 있는데 오디오 파일이 없는** 녹음 전부 (Phase 2B 요구사항 6).
    ///
    /// 정지 경로는 이 상태를 만들지 않는다 — 레코드는 파일이 확인된 뒤에만 쓰인다. 그래도
    /// 파일은 앱 밖에서 옮겨지거나 지워질 수 있으므로, 그 사실을 **알 수 있는 수단**이 필요하다.
    ///
    /// **감지는 감지일 뿐이다.** 여기서 레코드를 지우거나 고치지 않고 파일을 새로 만들지도
    /// 않는다 (INV-3 · INV-4). 하나도 없으면 빈 목록이며 그것이 정상 상태다.
    pub fn missing_audio(&self) -> Result<Vec<MissingAudioPayload>, Failure> {
        Ok(self
            .list_recordings()?
            .iter()
            .filter(|recording| !finalized::audio_is_present(&recording.audio_path))
            .map(MissingAudioPayload::from)
            .collect())
    }

    /// 녹음 하나를 저장한다. 식별자와 시각을 만드는 자리가 여기 하나뿐이다.
    ///
    /// 저장할 값은 **저장 시각을 받아** 만들어진다 — 제목을 그 시각에서 짓는 경로가 있기
    /// 때문이다 ([`Self::save_capture`]).
    fn insert(
        &self,
        recording: impl FnOnce(&str) -> NewRecording,
    ) -> Result<RecordingPayload, Failure> {
        let connection = self.connection()?;
        let id = store::new_id(&connection)?;
        let timestamp = store::now(&connection)?;

        let recording = validated(recording(&timestamp))?.into_recording(id, timestamp);
        store::insert_recording(&connection, &recording)?;

        Ok(RecordingPayload::from(RecordingView::from(recording)))
    }

    /// 녹음 레코드 하나를 지운다. 지웠으면 `true`, 그런 id가 없었으면 `false`다.
    ///
    /// **사용자가 명시적으로 요청했을 때만 지나는 경로다** (INV-4). 지우는 것은 레코드이며,
    /// 오디오 파일은 건드리지 않는다 ([`store::delete_recording`]).
    pub fn delete_recording(&self, id: &str) -> Result<bool, Failure> {
        let connection = self.connection()?;
        Ok(store::delete_recording(&connection, &RecordingId::new(id))?)
    }

    /// 저장된 설정을 돌려준다. 저장된 적이 없으면 기본값이다 (오류가 아니다).
    pub fn settings(&self) -> Result<SettingsPayload, Failure> {
        let connection = self.connection()?;
        Ok(SettingsPayload::from(settings::load(&connection)?))
    }

    /// 설정을 저장하고 **저장된 결과를 다시 읽어** 돌려준다.
    ///
    /// 화면이 "무엇이 저장됐는가"를 추측하지 않게 하기 위해서다 — 정규화된 값이 있으면
    /// 그 값이 그대로 돌아온다.
    pub fn update_settings(&self, payload: SettingsPayload) -> Result<SettingsPayload, Failure> {
        let connection = self.connection()?;
        settings::save(&connection, &payload.into())?;
        Ok(SettingsPayload::from(settings::load(&connection)?))
    }

    /// 저장소 연결을 빌린다. 초기화에 실패했다면 그때의 실패를 그대로 돌려준다.
    fn connection(&self) -> Result<MutexGuard<'_, Connection>, Failure> {
        match &self.state {
            StorageState::Ready(connection) => connection.lock().map_err(|_| {
                // 이전 command가 연결을 쥔 채 죽었다. 남은 상태를 아는 척하지 않는다.
                Failure::permanent(
                    FailureKind::Storage,
                    "로컬 저장소를 더 이상 사용할 수 없다. 앱을 다시 시작해야 한다.",
                )
            }),
            StorageState::Unavailable(failure) => Err(failure.clone()),
        }
    }
}

/// 앱이 들고 있는 입력 장치 열거 경로.
///
/// 실제 장치를 묻는 부분은 [`InputDeviceSource`] 뒤에 있고, 앱은 시스템 구현을 쓴다
/// ([`Self::system`]). 테스트는 자신의 구현을 넣어 **마이크 없이** 이 경로를 그대로 지난다
/// ([`Self::with_source`]).
///
/// [`Storage`]와 달리 시작할 때 여는 것이 없다 — 목록은 물어볼 때마다 새로 얻는다.
/// 장치는 언제든 꽂히고 빠지므로 앱이 시작 시점의 목록을 들고 있으면 그것이 곧 거짓말이 된다.
pub struct AudioDevices {
    source: Box<dyn InputDeviceSource + Send + Sync>,
}

impl AudioDevices {
    /// 이 기기의 실제 입력 장치를 묻는다 (ADR-0003의 잠정 선택 경로).
    pub fn system() -> Self {
        Self::with_source(SystemInputDevices)
    }

    /// 주어진 경계 구현에 묻는다.
    pub fn with_source(source: impl InputDeviceSource + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }

    /// 고를 수 있는 입력 장치를 표시 순서로 돌려준다.
    ///
    /// 하나도 없으면 빈 목록이다 — **오류가 아니다.** 마이크를 뽑아 둔 상태는 정상 상태이며,
    /// 그것을 실패로 만들면 화면이 사용자에게 없는 문제를 알리게 된다.
    pub fn list(&self) -> Result<Vec<InputDevicePayload>, Failure> {
        let observed = self.source.observe()?;
        Ok(catalog(observed)
            .into_iter()
            .map(InputDevicePayload::from)
            .collect())
    }
}

/// 진행 중인 녹음 하나. **상태 기계와 열려 있는 캡처가 한 값으로 묶여 있다.**
///
/// 둘이 따로 놓이면 "session은 일시정지인데 파일에는 계속 쓰이는" 상태가 만들어질 수 있다.
/// 여기서는 그 둘을 함께만 얻을 수 있다.
struct ActiveRecording {
    /// 전이 규칙과 경과 시간을 아는 쪽 (`crate::audio::session`).
    session: RecordingSession,
    /// 열린 장치와 쓰는 중인 파일을 아는 쪽 (`crate::audio::capture`).
    capture: ActiveCapture,
}

/// 앱이 들고 있는 **녹음 session**. 진행 중인 녹음의 소유자다 (R-001).
///
/// [`AudioDevices`]와 같은 방식으로 갈라져 있다 — 실제 장치에서 샘플을 받는 부분은
/// [`SampleSource`] 뒤에, 흐르는 시간은 [`Clock`] 뒤에, 마이크 접근 권한은
/// [`MicrophonePermission`] 뒤에 있다. 테스트는 그 세 자리에 자신의 구현을 넣어 **마이크 없이,
/// 시간을 실제로 흘려보내지 않고, 실제 권한 상태와 무관하게** 이 경로를 그대로 지난다
/// ([`Self::with_clock`] · [`Self::with_microphone`] · §18).
///
/// [`Storage`]처럼 **앱 데이터 디렉터리를 얻지 못한 실패도 값으로 들고 있다.** 그 실패는
/// 녹음을 시작하려 할 때 사용자에게 그대로 전달된다 — 시작 시점에 죽지 않는다 (§13).
///
/// **한 번에 하나만 녹음한다.** 진행 중인 녹음이 있는지, 있다면 그 session이 어떤 상태인지가
/// 이 값의 상태 전부다. 정지한 session은 여기 남지 않는다 — 끝난 녹음을 들고 있을 이유가
/// 없고, 다음 녹음은 새 session으로 시작한다 ([`Self::stop`]).
pub struct Recorder {
    source: Box<dyn SampleSource + Send + Sync>,
    /// 경과 시간을 재는 자리. 상태 기계는 시각을 값으로만 받는다.
    clock: Box<dyn Clock>,
    /// 마이크 접근이 허용됐는지 묻는 자리 (`crate::platform::microphone` · INV-10).
    ///
    /// 여기 값을 바꿔 넣으면 **실제 권한 상태와 무관하게** 세 상태를 전부 지날 수 있다
    /// ([`Self::with_microphone`]).
    microphone: Box<dyn MicrophonePermission>,
    /// 출력 파일이 놓일 자리. 얻지 못했다면 그 실패를 들고 있다.
    app_data_dir: Result<AppDataDirectory, Failure>,
    active: Mutex<Option<ActiveRecording>>,
}

impl Recorder {
    /// Tauri가 결정한 앱 데이터 디렉터리에 이 기기의 실제 입력 장치로 녹음한다 (INV-10).
    pub fn open_for<R, M>(manager: &M) -> Self
    where
        R: tauri::Runtime,
        M: Manager<R>,
    {
        Self {
            source: Box::new(SystemSampleSource),
            clock: Box::new(MonotonicClock::new()),
            microphone: Box::new(SystemMicrophonePermission),
            app_data_dir: AppDataDirectory::from_manager(manager).map_err(Into::into),
            active: Mutex::new(None),
        }
    }

    /// 주어진 디렉터리에 주어진 경계 구현으로 녹음한다. 시간은 실제로 흐른다.
    pub fn with_source(
        app_data_dir: AppDataDirectory,
        source: impl SampleSource + Send + Sync + 'static,
    ) -> Self {
        Self::with_clock(app_data_dir, source, MonotonicClock::new())
    }

    /// 시계까지 주어진 것으로 바꾼다. **경과 시간을 확인하는 테스트가 쓰는 자리다.**
    pub fn with_clock(
        app_data_dir: AppDataDirectory,
        source: impl SampleSource + Send + Sync + 'static,
        clock: impl Clock + 'static,
    ) -> Self {
        Self {
            source: Box::new(source),
            clock: Box::new(clock),
            microphone: Box::new(SystemMicrophonePermission),
            app_data_dir: Ok(app_data_dir),
            active: Mutex::new(None),
        }
    }

    /// 마이크 접근 권한을 묻는 자리까지 주어진 것으로 바꾼다.
    ///
    /// **권한이 거부된 상태를 확인하는 테스트가 쓰는 자리다** — 실제 마이크 권한을 끄지 않고도
    /// 허용 · 거부 · 미결정 세 상태를 그대로 지날 수 있다 (§18).
    pub fn with_microphone(mut self, microphone: impl MicrophonePermission + 'static) -> Self {
        self.microphone = Box::new(microphone);
        self
    }

    /// 고른 장치를 열고 녹음을 시작한다.
    ///
    /// 이미 녹음 중이면 시작하지 않는다 — **진행 중인 녹음을 조용히 버리지 않는다.**
    ///
    /// ## 권한을 먼저 묻는다
    ///
    /// 마이크 접근이 **거부된 상태면 캡처를 시작하지 않는다.** 디렉터리도 파일도 만들지 않고
    /// 장치도 열지 않은 채, 사용자가 무엇을 해야 하는지 담은 실패로 끝난다
    /// (`crate::platform::microphone` · §13). 어디서 무엇을 허용해야 하는지는 여기가 아니라
    /// platform 경계가 안다 (INV-10).
    pub fn start(&self, device_key: &str) -> Result<(), Failure> {
        let app_data_dir = self.app_data_dir.as_ref().map_err(Clone::clone)?;
        let mut active = self.active()?;
        if active.is_some() {
            return Err(Failure::permanent(
                FailureKind::InvalidInput,
                "이미 녹음 중이다. 먼저 정지해야 한다.",
            ));
        }

        // 아직 결정되지 않은 상태라면 여기서 요청한다 — 요청은 platform 경계의 일이다.
        let access = self.microphone.request();
        if access == MicrophoneAccess::Denied {
            return Err(microphone::access_denied());
        }

        // 파일이 놓일 자리는 앱 데이터 디렉터리가 정한다. 이 함수는 경로를 만들지 않는다.
        let directory = app_data_dir.ensure_recordings_dir()?;
        let stem = capture::file_stem(unix_seconds());

        // 장치를 먼저 열고, 열린 뒤부터 시간을 잰다 — 장치를 여는 데 걸린 시간은 녹음이 아니다.
        //
        // 권한을 확정하지 못한 채 장치를 열지 못했다면, 그 실패는 권한 문제로 분류해 안내한다.
        // 그것이 항상 옳다는 보증은 없으며 그 한계는 UNVERIFIED다
        // (`docs/ADR-0005-microphone-permission.md` §4·§6).
        let capture = capture::start(self.source.as_ref(), device_key, &directory, &stem)
            .map_err(|failure| microphone::explain_open_failure(access, failure))?;

        let mut session = RecordingSession::idle();
        if let Err(failure) = session.start(self.clock.now_ms()) {
            // 새 session은 언제나 idle이므로 여기 오지 않는다. 그래도 열어 둔 장치를 두고
            // 나가지는 않는다 — 열린 것을 닫는 책임은 실패해도 사라지지 않는다.
            let _ = capture.stop();
            return Err(failure);
        }

        *active = Some(ActiveRecording { session, capture });
        Ok(())
    }

    /// 일시정지한다. **장치도 파일도 열려 있는 채로 둔다.**
    ///
    /// 이 시점 이후의 샘플은 파일에 도달하지 않고, 흐르는 시간도 길이에 더해지지 않는다
    /// (Phase 2B 요구사항 4).
    pub fn pause(&self) -> Result<(), Failure> {
        let now = self.clock.now_ms();
        let mut active = self.active()?;
        let recording = active.as_mut().ok_or_else(not_recording)?;

        // 전이가 먼저다. 거절되면 파일 쪽은 건드리지 않는다 — 거절은 아무 일도 일어나지
        // 않았다는 뜻이기 때문이다.
        recording.session.pause(now)?;
        recording.capture.pause()
    }

    /// 다시 녹음한다. **같은 파일에 이어 쓴다.**
    pub fn resume(&self) -> Result<(), Failure> {
        let now = self.clock.now_ms();
        let mut active = self.active()?;
        let recording = active.as_mut().ok_or_else(not_recording)?;

        recording.session.resume(now)?;
        recording.capture.resume()
    }

    /// 녹음을 끝내고 파일을 확정한 뒤, **그 파일이 정말 쓸 수 있는 녹음인지 확인한다** (R-002).
    ///
    /// 확정만으로는 성공이 아니다. 경로가 존재하고 · 크기가 유효 최소치를 넘고 · 포맷을 알고
    /// 있어야 한다 ([`finalized::verify`]). 확인에 실패하면 사용자에게 보이는 실패다.
    ///
    /// **어떤 실패에서도 파일을 지우지 않는다** (INV-4 · R-004). 그래서 모든 실패는 그 파일이
    /// 어디에 있는지 함께 말한다 ([`keeping_file`]) — 지우지 않는 파일은 찾을 수 있어야 한다.
    ///
    /// 성공이든 실패든 **이 호출로 녹음 하나가 끝난다.** 정지한 session은 남지 않으므로
    /// 다음 녹음은 새 session으로 시작한다. 실패한 녹음을 다시 정지할 수는 없다.
    ///
    /// 여기까지가 파일의 일이고, 레코드를 남기는 것은 [`finish_recording`]이다.
    pub fn stop(&self) -> Result<CaptureReportPayload, Failure> {
        let now = self.clock.now_ms();
        let mut active = self.active()?;

        // 길이를 먼저 확정한다. 전이가 거절되면 진행 중인 녹음을 그대로 둔다.
        let summary = match active.as_mut() {
            Some(recording) => recording.session.stop(now)?,
            None => return Err(not_recording()),
        };
        let capture = match active.take() {
            Some(recording) => recording.capture,
            None => return Err(not_recording()),
        };
        drop(active);

        // 여기서부터 실패하더라도 **이미 쓰인 파일은 남는다.** 그 자리를 잃지 않도록 먼저 적어 둔다.
        let output_path = capture.output_path().to_path_buf();
        let report = capture
            .stop()
            .map_err(|failure| keeping_file(failure, &output_path))?;

        finalized::verify(&report.output_path, report.format)
            .map_err(|failure| keeping_file(failure, &output_path))?;

        Ok(CaptureReportPayload::new(report, summary))
    }

    /// 지금 녹음이 어떤 상태이고 얼마나 진행됐는지.
    ///
    /// 진행 중인 녹음이 없으면 **아직 시작하지 않은 session의 답**을 그대로 돌려준다 —
    /// `idle`과 `0:00`이다. 여기에 특별한 빈 값을 따로 만들지 않는다.
    pub fn status(&self) -> Result<SessionStatusPayload, Failure> {
        let now = self.clock.now_ms();
        let active = self.active()?;
        let idle = RecordingSession::idle();
        let session = match active.as_ref() {
            Some(recording) => &recording.session,
            None => &idle,
        };

        Ok(SessionStatusPayload::new(
            session.state(),
            session.elapsed_ms(now),
            session.elapsed_label(now),
        ))
    }

    /// 진행 중인 녹음 자리를 빌린다. 이전 호출이 쥔 채 죽었다면 그 사실을 그대로 알린다.
    fn active(&self) -> Result<MutexGuard<'_, Option<ActiveRecording>>, Failure> {
        self.active.lock().map_err(|_| {
            Failure::permanent(
                FailureKind::AudioDevice,
                "녹음 상태를 더 이상 알 수 없다. 앱을 다시 시작해야 한다.",
            )
        })
    }
}

/// 녹음을 끝낸다 — **파일 확정 · 확인 · 레코드 영속화까지가 한 동작이다** (R-002).
///
/// ## 순서: 파일이 먼저, 레코드가 나중
///
/// ```text
/// 1. 파일을 확정한다            (Recorder::stop → capture)
/// 2. 확정된 파일을 확인한다      (Recorder::stop → finalized::verify)
/// 3. 레코드를 저장한다          (Storage::save_capture)
/// ```
///
/// 반대로 하지 않는 이유는 하나다 — **레코드는 존재가 확인된 파일만 가리켜야 한다.** 먼저
/// 행을 쓰면 확정에 실패한 순간 "레코드는 있는데 audio가 없다"가 정상 경로에서 만들어진다.
/// 이 순서에서는 그 상태가 정지 때문에 생기지 않는다.
///
/// ## 어긋난 상태와 보상 (`docs/ADR-0004-recording-session-lifecycle.md` §12)
///
/// 3에서 실패하면 **audio는 있는데 레코드가 없는** 상태가 남는다. 그때 하는 일은 하나다 —
/// **파일을 그대로 두고, 그 경로를 담은 실패를 사용자에게 보낸다.** 방금 녹음한 것을
/// "정리"라는 이름으로 지우는 보상은 하지 않는다 (INV-3 · INV-4 · R-004).
///
/// 반대 방향(레코드는 있는데 audio가 없다)은 이 경로가 만들지 않지만, 파일은 앱 밖에서
/// 옮겨지거나 지워질 수 있다. 그것을 아는 수단이 [`Storage::missing_audio`]이며,
/// 그 역시 지우거나 고치지 않고 **알리기만 한다.**
///
/// ## 자동 전사는 **성공한 뒤에만, 설정이 켜져 있을 때만** 시작된다 (`phase-prompt/03` 요구 4)
///
/// 그 판단은 [`start_automatic_transcription`]이 하며, 여기서는 순서만 정해진다 — 레코드가
/// 저장된 뒤다. 저장되지 않은 녹음을 전사하려 들지 않기 위해서다.
///
/// `title`은 사용자가 입력한 제목이다. 없거나 비어 있으면 저장 시각에서 만든다.
pub fn finish_recording(
    recorder: &Recorder,
    storage: &Storage,
    transcriber: &Transcriber,
    title: Option<&str>,
) -> Result<StoppedRecordingPayload, Failure> {
    let capture = recorder.stop()?;
    let output_path = PathBuf::from(&capture.output_path);

    let recording = storage
        .save_capture(&capture, title)
        .map_err(|failure| keeping_file(not_listed(failure), &output_path))?;

    start_automatic_transcription(storage, transcriber, &recording.id);

    Ok(StoppedRecordingPayload { recording, capture })
}

/// 방금 저장된 녹음의 전사를 **설정이 켜져 있을 때만** 시작한다 (ADR-0007 §8.2.3).
///
/// ```text
/// automatic_transcription = ON   → 전사를 건다 (배경 스레드에서 돈다)
/// automatic_transcription = OFF  → 아무것도 하지 않는다
/// ```
///
/// **수동 전사는 이 값과 무관하다** — [`start_transcription`] command는 설정을 보지 않는다.
/// 꺼 두는 것은 "자동으로 시작하지 않는다"는 뜻이지 "전사할 수 없다"는 뜻이 아니다.
///
/// ## 여기서 일어나는 어떤 일도 정지의 성공을 되돌리지 않는다
///
/// 이 함수는 실패를 돌려주지 않는다. 정지는 이미 성공했고 — 파일이 확정됐고, 확인됐고,
/// 레코드가 저장됐다 (R-002) — 전사를 걸지 못했다는 이유로 그 사실을 실패로 바꾸면
/// 사용자는 저장된 녹음을 잃은 것처럼 보게 된다.
///
/// 시작하지 못하는 경우는 둘이다. 설정을 읽지 못했다면 자동 전사를 **켜져 있다고 가정하지
/// 않는다** — 사용자가 켜지 않았을 수 있는 일을 추측으로 시작하지 않는다. 이미 다른 전사가
/// 돌고 있다면 [`Transcriber::start`]가 거절하며, 그때도 녹음은 저장돼 있으므로 사용자는
/// 앞의 전사가 끝난 뒤 수동으로 시작할 수 있다 (§16의 큐는 DEFERRED다).
///
/// **모델이 없어도 시작한다.** 그 상태를 이유로 토글을 뒤집거나 조용히 건너뛰지 않는다 —
/// 전사는 §13의 `transcriptionModelMissing`으로 실패하고, 그 실패가 곧 사용자가 보는
/// 제품 상태다 (ADR-0007 §8.2.3).
fn start_automatic_transcription(storage: &Storage, transcriber: &Transcriber, recording_id: &str) {
    let Ok(settings) = storage.settings() else {
        return;
    };
    if !settings.automatic_transcription {
        return;
    }

    let _ = transcriber.start(recording_id);
}

/// 파일은 남았는데 레코드를 남기지 못한 실패.
///
/// 저장소가 말한 원인은 그대로 두고 **무슨 일이 일어났는지를 앞에 붙인다** — 사용자가 읽어야
/// 하는 첫 문장은 "저장소 질의가 실패했다"가 아니라 "녹음은 파일로 남았지만 목록에는 없다"다.
///
/// `source_data_safe`를 바꾸지 않는다. 이 실패는 이미 저장된 것을 훼손하지 않았고,
/// 방금 녹음한 파일도 확인을 마친 채 그대로 있다 (§13).
fn not_listed(failure: Failure) -> Failure {
    Failure {
        message: format!("녹음은 파일로 저장됐지만 목록에 추가하지 못했다: {failure}"),
        ..failure
    }
}

/// 실패 하나에 **확정된 파일이 남아 있는 자리**를 덧붙인다.
///
/// 이 앱에는 녹음 파일을 지우는 자동 경로가 없다 (INV-4 · R-004). 그러므로 정지가 실패해도
/// 파일은 거기 있고, **사용자가 그것을 찾을 수 있어야 실패가 유실이 되지 않는다** (INV-1).
///
/// 이미 경로를 담고 있는 실패는 그대로 둔다 — 같은 사실을 두 번 적지 않는다.
fn keeping_file(failure: Failure, path: &Path) -> Failure {
    let path = path.display().to_string();
    if failure.message.contains(&path) {
        return failure;
    }

    Failure {
        message: format!("{failure} 녹음된 파일은 남아 있다: {path}"),
        ..failure
    }
}

/// 저장 시각에서 만드는 기본 제목.
///
/// 사용자가 제목을 입력하지 않아도 목록에서 서로 구분되어야 하고, 제목이 빈 레코드는 저장되지
/// 않는다 ([`validated`]) — 이름이 없다는 이유로 방금 녹음한 것을 잃지 않게 하는 자리다.
///
/// 저장된 시각 텍스트(`2026-09-02T10:00:00.000Z`)의 날짜와 분까지를 그대로 쓴다. 그 값은
/// 저장소가 만든 UTC 텍스트이며, **여기서 시간대를 계산하지 않는다** — 이 함수에는 시계도
/// 시간대 지식도 없다. 예상과 다른 모양이 오면 지어내지 않고 받은 값을 그대로 쓴다.
fn title_for(created_at: &str) -> String {
    match (created_at.get(0..10), created_at.get(11..16)) {
        (Some(date), Some(time)) => format!("녹음 {date} {time}"),
        _ => format!("녹음 {created_at}"),
    }
}

/// 진행 중인 녹음이 없는데 그것을 전제한 요청이 왔다.
///
/// 같은 요청을 다시 보내도 결과가 같으므로 재시도 가능한 실패가 아니다.
fn not_recording() -> Failure {
    Failure::permanent(FailureKind::InvalidInput, "녹음 중이 아니다.")
}

/// 1970년 이후 흐른 초. 출력 파일 이름의 뿌리가 된다.
///
/// 시계가 그보다 앞을 가리켜도 죽지 않는다 — 이름을 정하는 일이 앱을 멈추게 할 이유는 없다.
/// 그래서 이 값이 겹쳐도 파일은 덮어써지지 않는다 ([`capture::output_path`]).
fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// 저장소를 실제로 연다. 디렉터리 준비와 스키마 갱신이 모두 성공해야 한다.
///
/// 두 단계의 오류는 각자의 모듈이 domain 공통 실패로 옮긴다 (`?`가 그 변환을 부른다).
fn initialize(app_data_dir: &AppDataDirectory) -> Result<Connection, Failure> {
    app_data_dir.ensure()?;
    Ok(db::open_in(app_data_dir)?)
}

/// 저장할 수 없는 값을 저장소까지 보내지 않는다.
///
/// 여기서 걸러지는 것은 **사용자에게 보여줄 수 있는 문제**이며, 저장소 실패와 구분된다.
fn validated(recording: NewRecording) -> Result<NewRecording, Failure> {
    let invalid = |message: &str| Failure::permanent(FailureKind::InvalidInput, message);

    if recording.title.trim().is_empty() {
        return Err(invalid("녹음 제목이 비어 있다."));
    }
    if recording.duration_ms < 0 {
        return Err(invalid("녹음 길이가 음수다."));
    }
    if recording.audio_path.trim().is_empty() {
        return Err(invalid("녹음 파일 경로가 비어 있다."));
    }
    if recording.audio_format.trim().is_empty() {
        return Err(invalid("녹음 파일 형식이 비어 있다."));
    }
    Ok(recording)
}

// --- Tauri command 표면 -------------------------------------------------------------
//
// 아래 열다섯 개가 frontend가 부를 수 있는 전부다. 각 함수는 [`Storage`] · [`AudioDevices`] ·
// [`Recorder`] · [`Transcriber`]의 같은 이름 동작이나 [`finish_recording`]을 그대로 부른다 —
// 로직을 여기에 두지 않으므로, 실제 동작은 Tauri 없이 테스트할 수 있다.

#[tauri::command]
pub fn list_recordings(storage: State<'_, Storage>) -> Result<Vec<RecordingPayload>, Failure> {
    storage.list_recordings()
}

#[tauri::command]
pub fn get_recording(
    storage: State<'_, Storage>,
    recording_id: String,
) -> Result<Option<RecordingPayload>, Failure> {
    storage.recording(&recording_id)
}

/// Transcript 하나를 segment까지 읽는다. 그런 id가 없으면 `null`이다.
///
/// **읽기 전용 표면이다.** Transcript를 고치거나 지우는 command는 없고, 만들 수도 없다 —
/// 저장소의 Transcript 쓰기 경로가 추가 하나뿐이기 때문이다 (§7.1 · INV-2).
#[tauri::command]
pub fn get_transcript(
    storage: State<'_, Storage>,
    transcript_id: String,
) -> Result<Option<TranscriptPayload>, Failure> {
    storage.transcript(&transcript_id)
}

#[tauri::command]
pub fn create_recording(
    storage: State<'_, Storage>,
    recording: NewRecording,
) -> Result<RecordingPayload, Failure> {
    storage.create_recording(recording)
}

#[tauri::command]
pub fn delete_recording(storage: State<'_, Storage>, recording_id: String) -> Result<bool, Failure> {
    storage.delete_recording(&recording_id)
}

#[tauri::command]
pub fn get_settings(storage: State<'_, Storage>) -> Result<SettingsPayload, Failure> {
    storage.settings()
}

#[tauri::command]
pub fn update_settings(
    storage: State<'_, Storage>,
    settings: SettingsPayload,
) -> Result<SettingsPayload, Failure> {
    storage.update_settings(settings)
}

#[tauri::command]
pub fn list_input_devices(
    devices: State<'_, AudioDevices>,
) -> Result<Vec<InputDevicePayload>, Failure> {
    devices.list()
}

#[tauri::command]
pub fn start_capture(recorder: State<'_, Recorder>, device_key: String) -> Result<(), Failure> {
    recorder.start(&device_key)
}

#[tauri::command]
pub fn pause_capture(recorder: State<'_, Recorder>) -> Result<(), Failure> {
    recorder.pause()
}

#[tauri::command]
pub fn resume_capture(recorder: State<'_, Recorder>) -> Result<(), Failure> {
    recorder.resume()
}

/// 정지한다. **성공은 파일이 확정되고 확인되고 레코드로 저장됐다는 뜻이다** (R-002).
///
/// 저장된 뒤에 전사가 자동으로 시작될 수 있다 — 설정이 켜져 있을 때만이다
/// ([`start_automatic_transcription`]). 그 전사는 배경 스레드에서 돌므로 이 command를
/// 붙잡지 않는다.
#[tauri::command]
pub fn stop_capture(
    recorder: State<'_, Recorder>,
    storage: State<'_, Storage>,
    transcriber: State<'_, Transcriber>,
    title: Option<String>,
) -> Result<StoppedRecordingPayload, Failure> {
    finish_recording(&recorder, &storage, &transcriber, title.as_deref())
}

/// 레코드는 있는데 오디오 파일이 없는 녹음을 알린다. **아무것도 지우거나 고치지 않는다.**
#[tauri::command]
pub fn list_missing_audio(
    storage: State<'_, Storage>,
) -> Result<Vec<MissingAudioPayload>, Failure> {
    storage.missing_audio()
}

#[tauri::command]
pub fn capture_status(recorder: State<'_, Recorder>) -> Result<SessionStatusPayload, Failure> {
    recorder.status()
}

/// Recording 하나의 전사를 시작한다. **돌아오는 것은 접수 사실이지 전사 결과가 아니다.**
///
/// 실제 전사는 배경 스레드에서 돌므로 이 호출은 바로 끝난다 — 1시간짜리 녹음을 걸어도 이
/// command가 IPC를 붙잡지 않는다. 결과는 [`transcription_status`]로 물어본다.
///
/// 이미 전사 중이면 거절한다 — 조용히 무시하지 않는다 ([`Transcriber::start`]).
#[tauri::command]
pub fn start_transcription(
    transcriber: State<'_, Transcriber>,
    recording_id: String,
) -> Result<TranscriptionStatusPayload, Failure> {
    transcriber.start(&recording_id)
}

/// 지금 전사가 어떤 상태인지 묻는다. **전사가 도는 동안에도 즉시 답한다.**
#[tauri::command]
pub fn transcription_status(
    transcriber: State<'_, Transcriber>,
) -> Result<TranscriptionStatusPayload, Failure> {
    transcriber.status()
}
