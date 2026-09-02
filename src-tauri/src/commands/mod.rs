//! frontend가 부를 수 있는 동작의 전부 (Tauri command 경계).
//!
//! **SQL은 이 경계를 넘지 않는다.** frontend는 질의를 보내지 않고 여기 선언된 동작만 부른다.
//! 저장소를 아는 코드는 [`crate::db`] 안에만 있고, 프론트엔드는 그 존재를 알지 않는다
//! (`docs/ADR-0001-local-persistence.md` · PRODUCT-SPEC §12).
//!
//! Phase 1의 표면은 두 가지뿐이다 — **recording CRUD**와 **settings**.
//! 녹음 · 전사 · AI · Notion을 위한 command는 그 기능이 존재하는 Phase가 함께 추가한다.
//!
//! ## 저장소 초기화 실패는 앱을 죽이지 않는다
//!
//! 저장소는 앱 시작 시 한 번 연다. 열지 못하면 [`Storage`]는 **실패를 값으로 들고 있다가**
//! 모든 command 응답으로 돌려준다. 초기화 경로에 `unwrap` · `expect` · `panic!`이 없는 이유가
//! 이것이다 — 여기서 죽으면 사용자는 아무 설명도 받지 못하고, 실패는 콘솔에만 남는다 (§13).

pub mod payload;

use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use tauri::{Manager, State};

use crate::db::{self, settings, store};
use crate::domain::{Failure, FailureKind, RecordingId, RecordingView};
use crate::platform::app_data_dir::AppDataDirectory;

pub use payload::{NewRecording, RecordingPayload, SettingsPayload};

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

    /// 녹음 하나를 새로 저장하고, 저장된 모습을 그대로 돌려준다.
    ///
    /// 식별자와 시각은 저장소가 만든다 — 프론트엔드가 정하지 않는다.
    pub fn create_recording(&self, recording: NewRecording) -> Result<RecordingPayload, Failure> {
        let recording = validated(recording)?;

        let connection = self.connection()?;
        let id = store::new_id(&connection)?;
        let timestamp = store::now(&connection)?;
        let recording = recording.into_recording(id, timestamp);
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
// 아래 여섯 개가 frontend가 부를 수 있는 전부다. 각 함수는 [`Storage`]의 같은 이름 동작을
// 그대로 부른다 — 로직을 여기에 두지 않으므로, 실제 동작은 Tauri 없이 테스트할 수 있다.

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
