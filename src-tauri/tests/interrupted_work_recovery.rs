//! **앱이 죽어서 남은 "진행 중" 표시에서 사용자가 빠져나올 수 있다** (2026-09-10).
//!
//! ## 무엇이 있었나
//!
//! 후처리 상태 셋(`transcription_status` · `ai_status` · `notion_status`)은 **DB에 남는
//! 사실**이다. 그래서 앱을 다시 켜도 그대로이고, 화면은 그 값을 읽어 "진행 중"이라 말하며
//! 시작 버튼을 감춘다 (`src/screens/transcriptView.ts`).
//!
//! 그 설계는 앱이 정상적으로 끝난다는 것을 전제했다. 2026-09-09에 앱이 죽으면서 1시간
//! 24분짜리 녹음이 `running`인 채로 남았고, **껐다 켜도 풀리지 않았다.** 실제로 도는 것은
//! 아무것도 없는데 화면은 영원히 진행 중이었고, 사용자에게는 빠져나올 수단이 없었다.
//! 사람이 DB를 직접 고쳐야 했다.
//!
//! ## 여기서 판정하는 것
//!
//! ```text
//! 1. 앱을 다시 열면 끝나지 못한 표시가 남지 않는다
//! 2. 그 정리가 **다른 것을 건드리지 않는다** — 오디오 · Transcript · 노트 · 끝난 상태
//! 3. 정리된 상태에서 **다시 시도할 수 있다**
//! ```

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::commands::Storage;
use molt_note_lib::db;
use molt_note_lib::db::store;
use molt_note_lib::domain::{ProcessingStatus, Recording, RecordingId};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "molt-note-interrupted-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("임시 디렉터리를 만들 수 있어야 한다");
        Self { path }
    }

    fn app_data_dir(&self) -> AppDataDirectory {
        AppDataDirectory::new(self.path.clone())
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// 녹음 하나를 저장소에 직접 넣는다 — command 경계를 지나지 않아도 되는 일이다.
fn save(app_data_dir: &AppDataDirectory, title: &str) -> Recording {
    let connection = db::open_in(app_data_dir).expect("저장소를 열 수 있어야 한다");
    let id = store::new_id(&connection).expect("식별자를 받을 수 있어야 한다");
    let now = store::now(&connection).expect("시각을 얻을 수 있어야 한다");

    let recording = Recording {
        id: RecordingId::new(id),
        title: title.to_owned(),
        created_at: now.clone(),
        updated_at: now,
        duration_ms: 5_095_685,
        audio_path: "/somewhere/capture.wav".to_owned(),
        audio_format: "wav".to_owned(),
        microphone: Some("가짜 마이크".to_owned()),
        current_transcript_id: None,
        transcription_status: ProcessingStatus::None,
        ai_status: ProcessingStatus::None,
        notion_status: ProcessingStatus::None,
    };
    store::insert_recording(&connection, &recording).expect("저장할 수 있어야 한다");
    recording
}

/// 상태 셋을 직접 박아 넣는다 — **앱이 죽은 뒤의 DB를 그대로 재현한다.**
fn leave_behind(
    app_data_dir: &AppDataDirectory,
    id: &RecordingId,
    transcription: ProcessingStatus,
    ai: ProcessingStatus,
    notion: ProcessingStatus,
) {
    let connection = db::open_in(app_data_dir).expect("저장소를 열 수 있어야 한다");
    let now = store::now(&connection).expect("시각을 얻을 수 있어야 한다");
    store::update_recording_statuses(&connection, id, transcription, ai, notion, &now)
        .expect("상태를 박아 넣을 수 있어야 한다");
}

fn status_of(app_data_dir: &AppDataDirectory, id: &str) -> (String, String, String) {
    let connection = db::open_in(app_data_dir).expect("저장소를 열 수 있어야 한다");
    connection
        .query_row(
            "SELECT transcription_status, ai_status, notion_status FROM recordings WHERE id = ?1",
            rusqlite::params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("상태를 읽을 수 있어야 한다")
}

#[test]
fn work_left_running_by_a_dead_app_does_not_survive_the_next_start() {
    // 지키려는 것: **앱이 막 시작한 시점에는 도는 작업이 있을 수 없다.** 진행 중 표시가
    // 남아 있다면 그것은 예외 없이 지난 실행이 남긴 것이다.
    let temp = TempRoot::new("running");
    let app_data_dir = temp.app_data_dir();

    let saved = save(&app_data_dir, "죽은 앱이 남긴 녹음");
    let id = saved.id.clone();

    leave_behind(
        &app_data_dir,
        &id,
        ProcessingStatus::Running,
        ProcessingStatus::Running,
        ProcessingStatus::Running,
    );
    assert_eq!(
        status_of(&app_data_dir, saved.id.as_str()).0,
        "running",
        "재현이 되어야 한다",
    );

    // 앱을 다시 연다. 이것이 사용자가 하는 일 전부다.
    let _reopened = Storage::open(&app_data_dir);

    let (transcription, ai, notion) = status_of(&app_data_dir, saved.id.as_str());
    for (name, status) in [
        ("전사", &transcription),
        ("AI", &ai),
        ("Notion", &notion),
    ] {
        assert_eq!(
            status, "failed",
            "{name} 상태가 진행 중으로 남았다 — 사용자가 빠져나올 수 없다",
        );
    }
}

#[test]
fn queued_work_that_never_started_is_cleared_too() {
    // `pending`은 큐에 들어갔다가 시작되지 못한 것이다. 되살릴 주체가 이 앱에는 없다.
    let temp = TempRoot::new("pending");
    let app_data_dir = temp.app_data_dir();

    let saved = save(&app_data_dir, "큐에만 있던 녹음");

    leave_behind(
        &app_data_dir,
        &saved.id,
        ProcessingStatus::Pending,
        ProcessingStatus::None,
        ProcessingStatus::None,
    );

    let _reopened = Storage::open(&app_data_dir);

    assert_eq!(status_of(&app_data_dir, saved.id.as_str()).0, "failed");
}

#[test]
fn finished_work_is_left_exactly_as_it_was() {
    // 지키려는 것: **정리가 다른 것을 건드리지 않는다.** 끝난 것과 시작하지 않은 것은
    // 지난 실행의 흔적이 아니라 사실이다.
    let temp = TempRoot::new("finished");
    let app_data_dir = temp.app_data_dir();

    let saved = save(&app_data_dir, "이미 끝난 녹음");

    leave_behind(
        &app_data_dir,
        &saved.id,
        ProcessingStatus::Done,
        ProcessingStatus::None,
        ProcessingStatus::Failed,
    );

    let _reopened = Storage::open(&app_data_dir);

    assert_eq!(
        status_of(&app_data_dir, saved.id.as_str()),
        ("done".to_owned(), "none".to_owned(), "failed".to_owned()),
        "끝난 것 · 시작 안 한 것 · 실패한 것이 그대로 남아야 한다",
    );
}

#[test]
fn reopening_a_clean_store_changes_nothing_and_does_not_fail() {
    // 정리할 것이 없을 때 조용해야 한다 — 앱을 열 때마다 도는 자리다.
    let temp = TempRoot::new("clean");
    let app_data_dir = temp.app_data_dir();

    let saved = save(&app_data_dir, "갓 저장한 녹음");

    let before = status_of(&app_data_dir, saved.id.as_str());
    for _ in 0..3 {
        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "다시 열 수 있어야 한다");
    }
    assert_eq!(status_of(&app_data_dir, saved.id.as_str()), before);
}

#[test]
fn the_recording_itself_is_untouched() {
    // INV-1 · INV-2 — 바뀌는 것은 상태 표시 세 칸뿐이다.
    let temp = TempRoot::new("untouched");
    let app_data_dir = temp.app_data_dir();

    let saved = save(&app_data_dir, "건드리면 안 되는 녹음");

    leave_behind(
        &app_data_dir,
        &saved.id,
        ProcessingStatus::Running,
        ProcessingStatus::None,
        ProcessingStatus::None,
    );

    let after = Storage::open(&app_data_dir)
        .recording(saved.id.as_str())
        .expect("읽을 수 있어야 한다")
        .expect("그 녹음이 있어야 한다");

    assert_eq!(after.title, saved.title);
    assert_eq!(after.duration_ms, saved.duration_ms);
    assert_eq!(after.audio_path, saved.audio_path);
    assert!(after.current_transcript_id.is_none());
}
