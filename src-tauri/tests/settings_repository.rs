//! 설정 영속성: 기본값 정책 · 재시작 후 값 유지 · secret 부재(INV-7).
//!
//! 이 테스트는 **임시 디렉터리의 DB 파일 하나**만 있으면 돌아간다. 실제 사용자 앱 데이터
//! 디렉터리도, UI도, 하드웨어도 필요하지 않다 (§18).
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::db::{self, settings};
use molt_note_lib::domain::Settings;
use rusqlite::Connection;

/// secret을 담는 자리가 생기지 않았는지 소스에서도 확인하기 위한 대상 (INV-7).
const SETTINGS_STORE_SOURCE: &str = include_str!("../src/db/settings.rs");
const SETTINGS_DOMAIN_SOURCE: &str = include_str!("../src/domain/settings.rs");

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 빈 디렉터리. Drop 시 지운다.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-settings-test-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
        Self(path)
    }

    fn database_path(&self) -> PathBuf {
        self.0.join("molt-note.db")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 스키마가 최신인 DB를 연다.
fn open(dir: &TempDir) -> Connection {
    db::open(dir.database_path()).expect("임시 DB를 열 수 있어야 한다")
}

/// 설정 테이블의 행 수. "저장된 적 있는가"와 "한 행뿐인가"를 판정할 때 쓴다.
fn rows(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT count(*) FROM settings", [], |row| row.get(0))
        .expect("행 수를 셀 수 있어야 한다")
}

/// 연결을 실제로 닫는다. 닫히지 않으면 테스트를 실패시킨다 — "앱 종료"를 흉내내는 지점이다.
fn close(connection: Connection) {
    connection
        .close()
        .map_err(|(_, error)| error)
        .expect("연결이 닫혀야 한다");
}

// --- 기본값 정책 -------------------------------------------------------------------

#[test]
fn defaults_are_returned_when_nothing_has_been_saved() {
    let dir = TempDir::new("defaults");
    let connection = open(&dir);

    let loaded = settings::load(&connection).expect("설정을 읽을 수 있어야 한다");

    assert_eq!(
        loaded,
        Settings::DEFAULT,
        "저장된 값이 없으면 코드에 선언된 기본값이 나와야 한다"
    );
    assert_eq!(
        loaded.recordings_directory, None,
        "recordings directory의 기본값은 '아직 고르지 않음'이다"
    );
    assert!(
        !loaded.automatic_processing,
        "automatic 처리의 기본값은 OFF다 — 켜는 것은 사용자의 명시적 선택이다"
    );
    assert_eq!(
        loaded.default_microphone, None,
        "default microphone의 기본값은 '아직 고르지 않음'이다 — 첫 장치를 대신 골라 두지 않는다"
    );
    close(connection);
}

#[test]
fn reading_defaults_does_not_write_a_row() {
    let dir = TempDir::new("no-write");
    let connection = open(&dir);

    settings::load(&connection).expect("설정을 읽을 수 있어야 한다");

    assert_eq!(
        rows(&connection),
        0,
        "기본값을 돌려주는 것이 값을 저장하는 행위가 되어서는 안 된다"
    );
    close(connection);
}

// --- 재시작 후에도 남는다 -----------------------------------------------------------

#[test]
fn saved_values_survive_closing_and_reopening_the_database() {
    let dir = TempDir::new("reopen");
    let saved = Settings {
        recordings_directory: Some("/Users/tester/Molt Note/Recordings".to_string()),
        automatic_processing: true,
        default_microphone: Some("0:Studio Mic".to_string()),
    };

    let first = open(&dir);
    settings::save(&first, &saved).expect("설정을 저장할 수 있어야 한다");
    close(first);

    // 같은 DB 파일을 새 연결로 다시 연다 — 앱을 껐다 켜는 것에 해당한다.
    let second = open(&dir);
    let loaded = settings::load(&second).expect("다시 열어도 설정을 읽을 수 있어야 한다");

    assert_eq!(
        loaded.recordings_directory,
        Some("/Users/tester/Molt Note/Recordings".to_string()),
        "recordings directory가 재시작 후에도 그대로여야 한다"
    );
    assert!(
        loaded.automatic_processing,
        "automatic 토글이 재시작 후에도 켜져 있어야 한다"
    );
    assert_eq!(
        loaded.default_microphone,
        Some("0:Studio Mic".to_string()),
        "고른 default microphone이 재시작 후에도 그대로여야 한다"
    );
    assert_eq!(loaded, saved, "저장한 설정과 읽은 설정이 같아야 한다");
    close(second);
}

#[test]
fn a_default_microphone_that_is_no_longer_plugged_in_is_still_the_saved_choice() {
    // 저장소는 장치를 알지 않는다. 장치를 뽑아 둔 채 앱을 껐다 켜도 **저장된 선택은 그대로**이며,
    // 읽는 김에 지우거나 다른 값으로 바꾸지 않는다. 지금 그 장치가 있는지를 말하는 것은
    // 목록을 아는 쪽의 일이다 (`src/screens/defaultMicrophone.ts`).
    let dir = TempDir::new("missing-microphone");

    let first = open(&dir);
    settings::save(
        &first,
        &Settings {
            recordings_directory: None,
            automatic_processing: false,
            default_microphone: Some("0:USB Microphone".to_string()),
        },
    )
    .expect("설정을 저장할 수 있어야 한다");
    close(first);

    let second = open(&dir);
    let loaded = settings::load(&second).expect("다시 열어도 설정을 읽을 수 있어야 한다");

    assert_eq!(
        loaded.default_microphone,
        Some("0:USB Microphone".to_string()),
        "장치가 없다는 이유로 저장된 선택이 사라지거나 바뀌면 안 된다"
    );
    close(second);
}

#[test]
fn clearing_the_default_microphone_is_remembered_as_not_chosen() {
    // '고르지 않음'으로 되돌리는 것도 사용자의 선택이다. 이전 값이 남아 부활하면 안 된다.
    let dir = TempDir::new("clear-microphone");

    let first = open(&dir);
    settings::save(
        &first,
        &Settings {
            recordings_directory: None,
            automatic_processing: false,
            default_microphone: Some("0:Studio Mic".to_string()),
        },
    )
    .expect("먼저 고른 값을 저장할 수 있어야 한다");
    settings::save(&first, &Settings::DEFAULT).expect("다시 비운 값을 저장할 수 있어야 한다");
    close(first);

    let second = open(&dir);
    let loaded = settings::load(&second).expect("다시 열어도 설정을 읽을 수 있어야 한다");

    assert_eq!(loaded.default_microphone, None);
    assert_eq!(rows(&second), 1, "설정은 여전히 한 행이다");
    close(second);
}

#[test]
fn a_database_written_before_the_default_microphone_existed_keeps_its_values() {
    // version 3까지만 적용된 DB에 값이 이미 있는 상황이다. 새 migration이 그 행을 지우거나
    // 다시 만들지 않고, 없던 열은 '아직 고르지 않음'으로 시작한다.
    let dir = TempDir::new("older-schema");

    let older = Connection::open(dir.database_path()).expect("빈 DB를 만들 수 있어야 한다");
    older
        .execute_batch(
            "CREATE TABLE schema_migrations (
                 version    INTEGER PRIMARY KEY,
                 name       TEXT NOT NULL,
                 applied_at TEXT NOT NULL
             );
             INSERT INTO schema_migrations (version, name, applied_at)
             VALUES (3, 'create_settings', datetime('now'));
             CREATE TABLE settings (
                 id                   INTEGER PRIMARY KEY CHECK (id = 1),
                 recordings_directory TEXT,
                 automatic_processing INTEGER NOT NULL
                     CHECK (automatic_processing IN (0, 1))
             );
             INSERT INTO settings (id, recordings_directory, automatic_processing)
             VALUES (1, '/tmp/older-schema', 1);
             PRAGMA user_version = 3;",
        )
        .expect("사전 조건: default microphone이 없던 스키마를 만든다");
    close(older);

    // 여기서 새 migration이 적용된다.
    let upgraded = open(&dir);
    let loaded = settings::load(&upgraded).expect("올린 뒤에도 설정을 읽을 수 있어야 한다");

    assert_eq!(
        loaded.recordings_directory,
        Some("/tmp/older-schema".to_string()),
        "이미 저장돼 있던 값이 그대로여야 한다"
    );
    assert!(loaded.automatic_processing, "토글도 그대로여야 한다");
    assert_eq!(
        loaded.default_microphone, None,
        "새로 생긴 열은 '아직 고르지 않음'으로 시작한다"
    );
    assert_eq!(rows(&upgraded), 1, "행이 늘거나 다시 만들어지지 않았다");
    close(upgraded);
}

#[test]
fn a_toggle_turned_off_is_remembered_and_not_mistaken_for_an_unsaved_value() {
    let dir = TempDir::new("off");

    let first = open(&dir);
    settings::save(
        &first,
        &Settings {
            recordings_directory: Some("/tmp/molt-note-recordings".to_string()),
            automatic_processing: true,
            default_microphone: None,
        },
    )
    .expect("먼저 켠 상태를 저장할 수 있어야 한다");
    settings::save(
        &first,
        &Settings {
            recordings_directory: Some("/tmp/molt-note-recordings".to_string()),
            automatic_processing: false,
            default_microphone: None,
        },
    )
    .expect("다시 끈 상태를 저장할 수 있어야 한다");
    close(first);

    let second = open(&dir);
    let loaded = settings::load(&second).expect("다시 열어도 설정을 읽을 수 있어야 한다");

    assert!(
        !loaded.automatic_processing,
        "끈 상태가 재시작 후에도 유지돼야 한다"
    );
    assert_eq!(
        loaded.recordings_directory,
        Some("/tmp/molt-note-recordings".to_string()),
        "토글만 바꿨을 때 다른 값이 함께 지워지면 안 된다"
    );
    assert_eq!(
        rows(&second),
        1,
        "설정은 한 행이다 — 저장할 때마다 행이 늘어나면 안 된다"
    );
    close(second);
}

#[test]
fn saving_settings_does_not_disturb_other_stored_data() {
    let dir = TempDir::new("isolation");

    let first = open(&dir);
    first
        .execute_batch(
            "CREATE TABLE persistence_probe (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
             INSERT INTO persistence_probe (id, note) VALUES (1, '설정 저장이 건드리지 않는다');",
        )
        .expect("사전 조건: 다른 데이터를 둔다");
    settings::save(
        &first,
        &Settings {
            recordings_directory: Some("/tmp/elsewhere".to_string()),
            automatic_processing: true,
            default_microphone: Some("0:Desk Mic".to_string()),
        },
    )
    .expect("설정을 저장할 수 있어야 한다");
    close(first);

    let second = open(&dir);
    let note: String = second
        .query_row("SELECT note FROM persistence_probe WHERE id = 1", [], |row| {
            row.get(0)
        })
        .expect("다른 데이터가 그대로 있어야 한다");

    assert_eq!(note, "설정 저장이 건드리지 않는다");
    close(second);
}

// --- INV-7: secret은 다루지 않는다 ---------------------------------------------------

/// secret을 뜻하는 이름들. 열 이름·API 표면 어디에도 나오면 안 된다.
const SECRET_WORDS: [&str; 6] = ["api_key", "apikey", "token", "password", "secret", "credential"];

#[test]
fn the_settings_schema_has_no_secret_columns() {
    let dir = TempDir::new("inv7-schema");
    let connection = open(&dir);

    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info('settings')")
        .expect("스키마를 읽을 수 있어야 한다");
    let columns: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("열 목록을 질의할 수 있어야 한다")
        .collect::<Result<Vec<_>, _>>()
        .expect("열 이름을 읽을 수 있어야 한다");

    assert_eq!(
        columns,
        vec![
            "id".to_string(),
            "recordings_directory".to_string(),
            "automatic_processing".to_string(),
            "default_microphone".to_string()
        ],
        "설정 테이블에는 이 Phase가 다루는 값만 있어야 한다"
    );
    for column in &columns {
        let lowered = column.to_lowercase();
        for word in SECRET_WORDS {
            assert!(
                !lowered.contains(word),
                "설정 스키마에 secret 성격의 열이 있다 (INV-7): {column}"
            );
        }
    }

    drop(statement);
    close(connection);
}

#[test]
fn the_settings_api_does_not_accept_or_store_secrets() {
    for (name, source) in [
        ("db/settings.rs", SETTINGS_STORE_SOURCE),
        ("domain/settings.rs", SETTINGS_DOMAIN_SOURCE),
    ] {
        for line in source.lines() {
            let trimmed = line.trim();
            // 주석은 INV-7을 설명하기 위해 이 단어들을 쓴다. 검사 대상은 코드다.
            if trimmed.starts_with("//") || trimmed.starts_with("--") {
                continue;
            }
            let lowered = trimmed.to_lowercase();
            for word in SECRET_WORDS {
                assert!(
                    !lowered.contains(word),
                    "{name}의 코드에 secret 성격의 이름이 있다 (INV-7): {trimmed}"
                );
            }
        }
    }
}
