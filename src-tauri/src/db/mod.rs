//! 로컬 영속 저장소 (SQLite).
//!
//! persistence ownership은 Rust 경계 안에 있다 (PRODUCT-SPEC §14.7 ·
//! `docs/ADR-0001-local-persistence.md`). 프론트엔드는 임의의 SQL을 실행하지 않는다.
//!
//! DB 파일 위치는 이 모듈이 정하지 않는다. [`AppDataDirectory`]가 돌려주는 경로를 받을 뿐이므로
//! OS별 경로 지식은 여전히 `platform` 경계 안에만 있다 (INV-10).

pub mod migrations;
pub mod settings;
pub mod store;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::domain::{Failure, FailureKind};
use crate::platform::app_data_dir::AppDataDirectory;

/// 다른 연결이 쥔 잠금을 기다리는 시간. 이 시간이 지나면 저장소 실패로 드러난다.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// 앱 데이터 디렉터리의 DB 파일을 열고 스키마를 최신으로 만든다.
///
/// 디렉터리는 이미 존재해야 한다 ([`AppDataDirectory::ensure`]).
pub fn open_in(app_data_dir: &AppDataDirectory) -> Result<Connection, DatabaseError> {
    open(app_data_dir.database_path())
}

/// 주어진 경로의 DB를 열고(파일이 없으면 만들고) 아직 적용되지 않은 migration을 적용한다.
///
/// 이미 있는 DB를 열 때도 기존 데이터는 그대로 남는다 — 적용되지 않은 migration만 추가로 실행된다.
pub fn open(path: impl AsRef<Path>) -> Result<Connection, DatabaseError> {
    let path = path.as_ref();
    let mut connection = Connection::open(path).map_err(|source| DatabaseError::Open {
        path: path.to_path_buf(),
        source,
    })?;

    // 참조 무결성은 연결마다 켜야 한다 (SQLite 기본값이 off다).
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(DatabaseError::Sql)?;

    // 이 DB에는 이제 연결이 둘 있을 수 있다 — 앱이 들고 있는 것과, 전사가 도는 동안 쓰는 것이다
    // (`crate::commands::transcriber`). 두 번째 연결을 여는 이유는 전사가 앱의 연결을 붙들어
    // 다른 command를 멈추게 하지 않기 위해서이며, 그 대가로 **쓰기가 겹칠 수 있다.**
    //
    // 기본값에서는 그 순간 즉시 `SQLITE_BUSY`가 난다. 두 쓰기 모두 짧으므로 그것을 실패로
    // 만드는 대신 잠깐 기다린다 — 목록을 읽는 사이에 전사 상태가 저장됐다는 이유로 사용자에게
    // 저장소 실패를 보이지 않기 위해서다. 무한정 기다리지도 않는다: 정말 풀리지 않는 잠금은
    // 여전히 §13의 실패로 드러나야 한다.
    connection
        .busy_timeout(BUSY_TIMEOUT)
        .map_err(DatabaseError::Sql)?;

    migrations::apply_pending(&mut connection)?;
    Ok(connection)
}

/// 저장소를 열거나 스키마를 갱신하지 못한 경우.
#[derive(Debug)]
pub enum DatabaseError {
    /// DB 파일 자체를 열지 못했다.
    Open {
        path: PathBuf,
        source: rusqlite::Error,
    },
    /// 스키마 버전 조회 같은 기본 SQL이 실패했다.
    Sql(rusqlite::Error),
    /// 특정 migration을 적용하지 못했다. 해당 migration은 적용되지 않은 상태로 롤백된다.
    Migrate {
        version: i64,
        name: &'static str,
        source: rusqlite::Error,
    },
    /// migration 목록이 순서 규칙(0보다 크고 엄격히 증가)을 어겼다.
    InvalidMigrationOrder { previous: i64, found: i64 },
    /// DB가 이 코드가 아는 것보다 새로운 스키마다. 다운그레이드하지 않고 멈춘다.
    AheadOfCode { database: i64, latest: i64 },
    /// 저장된 값을 domain 타입으로 해석하지 못했다. 추측해서 대신 채우지 않는다.
    Decode {
        table: &'static str,
        column: &'static str,
        value: String,
    },
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open { path, .. } => {
                write!(f, "로컬 저장소를 열지 못했다: {}", path.display())
            }
            Self::Sql(_) => write!(f, "로컬 저장소 질의가 실패했다"),
            Self::Migrate { version, name, .. } => {
                write!(f, "migration {version}({name})을 적용하지 못했다")
            }
            Self::InvalidMigrationOrder { previous, found } => write!(
                f,
                "migration 순서가 어긋났다: {previous} 다음에 {found}가 선언됐다"
            ),
            Self::AheadOfCode { database, latest } => write!(
                f,
                "저장소 스키마 버전 {database}가 이 앱이 아는 최신 버전 {latest}보다 새롭다"
            ),
            Self::Decode {
                table,
                column,
                value,
            } => write!(f, "{table}.{column}에 해석할 수 없는 값이 있다: {value}"),
        }
    }
}

impl std::error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Open { source, .. } | Self::Sql(source) | Self::Migrate { source, .. } => {
                Some(source)
            }
            Self::InvalidMigrationOrder { .. }
            | Self::AheadOfCode { .. }
            | Self::Decode { .. } => None,
        }
    }
}

/// 저장소 오류를 사용자에게 보여줄 수 있는 domain 공통 실패로 옮긴다 (§13).
///
/// **어떤 변환도 실패를 삼키지 않는다.** 저장소 계층의 오류가 UI까지 가려면 반드시 이 곳을
/// 지나므로, 저장소 실패가 로그로만 남고 끝나는 경로가 없다.
///
/// 재시도 가능 여부는 오류마다 다르다 — 그 판단을 UI에 떠넘기지 않고 여기서 한다.
/// 파일을 열지 못했거나 질의·migration이 실패한 것은 조건(권한 · 디스크 여유 · 잠금)이 바뀌면
/// 성공할 수 있다. 실패한 migration은 트랜잭션째 되돌아가므로 다시 시도해도 안전하다.
/// 반대로 스키마가 앱보다 새롭거나, 저장된 값을 해석할 수 없거나, migration 목록 자체가
/// 어긋난 것은 **다시 시도해도 같은 결과다** — 앱이나 DB 자체가 달라져야 한다.
impl From<DatabaseError> for Failure {
    fn from(error: DatabaseError) -> Self {
        use std::error::Error as _;

        let retryable = matches!(
            error,
            DatabaseError::Open { .. } | DatabaseError::Sql(_) | DatabaseError::Migrate { .. }
        );

        // 사용자에게 보여줄 문장은 각 오류의 Display가 이미 갖고 있다. 여기서 다시 쓰지 않는다.
        let failure = if retryable {
            Failure::retryable(FailureKind::Storage, error.to_string())
        } else {
            Failure::permanent(FailureKind::Storage, error.to_string())
        };

        match error.source() {
            Some(source) => failure.with_detail(source),
            None => failure,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::migrations::{current_version, latest_version, MIGRATIONS};
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 시스템 임시 디렉터리 아래의 빈 디렉터리. Drop 시 지운다.
    ///
    /// 테스트는 실제 사용자 앱 데이터 디렉터리를 만들거나 오염시키지 않는다.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let unique = format!(
                "molt-note-db-test-{}-{}-{}",
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

    /// 연결을 실제로 닫는다. 닫히지 않으면 테스트를 실패시킨다.
    fn close(connection: Connection) {
        connection
            .close()
            .map_err(|(_, error)| error)
            .expect("연결이 닫혀야 한다");
    }

    fn ledger(connection: &Connection) -> Vec<(i64, String, String)> {
        let mut statement = connection
            .prepare("SELECT version, name, applied_at FROM schema_migrations ORDER BY version")
            .expect("적용 기록 테이블을 읽을 수 있어야 한다");
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("적용 기록을 질의할 수 있어야 한다");
        rows.collect::<Result<Vec<_>, _>>()
            .expect("적용 기록 행을 읽을 수 있어야 한다")
    }

    #[test]
    fn creates_the_database_and_reaches_the_latest_schema_version_in_an_empty_directory() {
        let dir = TempDir::new("fresh");
        let path = dir.database_path();
        assert!(!path.exists(), "사전 조건: DB 파일이 아직 없어야 한다");

        let connection = open(&path).expect("빈 디렉터리에서 DB를 만들 수 있어야 한다");

        assert!(path.is_file(), "DB 파일이 생성돼야 한다");
        assert_eq!(
            current_version(&connection).expect("스키마 버전을 읽을 수 있어야 한다"),
            latest_version(),
            "새 DB는 최신 스키마 버전이어야 한다"
        );
        assert_eq!(
            ledger(&connection).len(),
            MIGRATIONS.len(),
            "선언된 migration이 모두 적용 기록에 남아야 한다"
        );
        close(connection);
    }

    #[test]
    fn open_derives_the_database_path_from_the_app_data_directory() {
        let dir = TempDir::new("appdata");
        let app_data_dir = AppDataDirectory::new(&dir.0);

        let connection = open_in(&app_data_dir).expect("앱 데이터 디렉터리에서 열 수 있어야 한다");

        assert!(
            app_data_dir.database_path().is_file(),
            "DB는 AppDataDirectory가 돌려준 경로에 만들어져야 한다"
        );
        close(connection);
    }

    #[test]
    fn rows_survive_closing_and_reopening_the_same_path() {
        let dir = TempDir::new("reopen");
        let path = dir.database_path();

        let first = open(&path).expect("처음 열 수 있어야 한다");
        first
            .execute_batch(
                "CREATE TABLE persistence_probe (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
                 INSERT INTO persistence_probe (id, note) VALUES (1, '재시작 후에도 남아야 한다');",
            )
            .expect("행을 쓸 수 있어야 한다");
        let applied_before = ledger(&first);
        close(first);

        // 같은 경로를 새 연결로 다시 연다. 앞의 연결은 이미 닫혔다.
        let second = open(&path).expect("같은 경로를 다시 열 수 있어야 한다");

        let note: String = second
            .query_row("SELECT note FROM persistence_probe WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("이전 연결에서 쓴 행이 그대로 있어야 한다");
        assert_eq!(note, "재시작 후에도 남아야 한다");
        assert_eq!(
            ledger(&second),
            applied_before,
            "적용 기록도 그대로여야 한다"
        );
        close(second);
    }

    #[test]
    fn running_migrations_again_keeps_existing_data_and_history() {
        let dir = TempDir::new("twice");
        let path = dir.database_path();

        let first = open(&path).expect("처음 열 수 있어야 한다");
        first
            .execute_batch(
                "CREATE TABLE persistence_probe (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
                 INSERT INTO persistence_probe (id, note) VALUES (1, '두 번째 실행에도 남아야 한다');",
            )
            .expect("행을 쓸 수 있어야 한다");
        let applied_before = ledger(&first);
        let version_before = current_version(&first).expect("스키마 버전을 읽을 수 있어야 한다");
        close(first);

        // 두 번째 open이 migration 실행을 한 번 더 시도한다.
        let mut second = open(&path).expect("두 번째로 열 수 있어야 한다");
        let applied_now =
            migrations::apply_pending(&mut second).expect("세 번째 실행도 성공해야 한다");

        assert!(
            applied_now.is_empty(),
            "이미 적용된 migration은 다시 실행되지 않아야 한다: {applied_now:?}"
        );
        assert_eq!(
            current_version(&second).expect("스키마 버전을 읽을 수 있어야 한다"),
            version_before
        );
        assert_eq!(
            ledger(&second),
            applied_before,
            "적용 시각까지 그대로여야 한다 — 기록을 다시 쓰지 않는다"
        );
        let rows: i64 = second
            .query_row("SELECT count(*) FROM persistence_probe", [], |row| {
                row.get(0)
            })
            .expect("행 수를 셀 수 있어야 한다");
        assert_eq!(rows, 1, "migration을 다시 실행해도 데이터가 유실되지 않는다");
        close(second);
    }

    #[test]
    fn refuses_a_database_written_by_a_newer_schema_without_touching_it() {
        let dir = TempDir::new("newer");
        let path = dir.database_path();

        let first = open(&path).expect("처음 열 수 있어야 한다");
        first
            .execute_batch(
                "CREATE TABLE persistence_probe (id INTEGER PRIMARY KEY, note TEXT NOT NULL);
                 INSERT INTO persistence_probe (id, note) VALUES (1, '건드리지 않는다');",
            )
            .expect("행을 쓸 수 있어야 한다");
        let future = latest_version() + 100;
        first
            .pragma_update(None, "user_version", future)
            .expect("미래 버전을 흉내낼 수 있어야 한다");
        close(first);

        let error = open(&path).expect_err("더 새로운 스키마는 열지 않아야 한다");
        assert!(
            matches!(error, DatabaseError::AheadOfCode { database, latest }
                if database == future && latest == latest_version()),
            "예상과 다른 오류다: {error}"
        );

        // 데이터는 그대로 남아 있어야 한다 — 실패가 저장소를 지우지 않는다.
        let survivor = Connection::open(&path).expect("파일이 남아 있어야 한다");
        let note: String = survivor
            .query_row("SELECT note FROM persistence_probe WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("행이 그대로 있어야 한다");
        assert_eq!(note, "건드리지 않는다");
        close(survivor);
    }

    #[test]
    fn foreign_key_enforcement_is_enabled_on_the_connection() {
        let dir = TempDir::new("fk");
        let connection = open(dir.database_path()).expect("열 수 있어야 한다");

        let enabled: i64 = connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .expect("pragma를 읽을 수 있어야 한다");

        assert_eq!(enabled, 1, "연결마다 참조 무결성이 켜져 있어야 한다");
        close(connection);
    }

    #[test]
    fn a_connection_waits_for_another_connection_instead_of_failing_immediately() {
        // 전사가 도는 동안 이 DB에는 연결이 둘 있다. 겹친 쓰기가 즉시 실패하면 사용자는
        // 아무 잘못도 하지 않고 저장소 실패를 본다.
        let dir = TempDir::new("busy");
        let connection = open(dir.database_path()).expect("열 수 있어야 한다");

        let timeout_ms: i64 = connection
            .pragma_query_value(None, "busy_timeout", |row| row.get(0))
            .expect("pragma를 읽을 수 있어야 한다");

        assert_eq!(
            timeout_ms,
            BUSY_TIMEOUT.as_millis() as i64,
            "연결마다 대기 시간이 설정돼 있어야 한다"
        );
        assert!(timeout_ms > 0, "0이면 기다리지 않고 즉시 실패한다");
        close(connection);
    }

    // --- 오류 → 사용자에게 보여줄 실패 (§13) ---------------------------------------

    /// 변환에 쓸 임의의 하위 오류. 어떤 값이든 detail에 실려 나가는지만 본다.
    fn sqlite_error() -> rusqlite::Error {
        rusqlite::Error::QueryReturnedNoRows
    }

    #[test]
    fn a_failed_open_is_reported_as_a_storage_failure_worth_retrying() {
        let error = DatabaseError::Open {
            path: PathBuf::from("/nowhere/molt-note.db"),
            source: sqlite_error(),
        };
        let sentence = error.to_string();

        let failure = Failure::from(error);

        assert_eq!(failure.kind, FailureKind::Storage);
        assert_eq!(failure.message, sentence, "사용자 문장을 새로 지어내지 않는다");
        assert!(failure.retryable, "권한·디스크 조건이 바뀌면 열릴 수 있다");
        assert_eq!(
            failure.detail.as_deref(),
            Some(sqlite_error().to_string().as_str()),
            "기술적 원인은 detail로 함께 나간다"
        );
    }

    #[test]
    fn a_database_newer_than_the_app_is_reported_as_not_worth_retrying() {
        // 같은 앱으로 다시 시도해도 결과가 같다 — 사용자가 앱을 올려야 한다.
        let failure = Failure::from(DatabaseError::AheadOfCode {
            database: 9,
            latest: 3,
        });

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(!failure.retryable);
        assert_eq!(failure.detail, None, "하위 오류가 없으면 detail도 없다");
    }

    #[test]
    fn an_undecodable_stored_value_is_reported_as_not_worth_retrying() {
        let failure = Failure::from(DatabaseError::Decode {
            table: "recordings",
            column: "ai_status",
            value: "cancelled".to_string(),
        });

        assert!(!failure.retryable);
        assert!(
            failure.message.contains("cancelled"),
            "어떤 값이 문제였는지 사용자에게 보인다: {}",
            failure.message
        );
    }

    #[test]
    fn a_broken_migration_list_is_reported_as_not_worth_retrying() {
        let failure = Failure::from(DatabaseError::InvalidMigrationOrder {
            previous: 3,
            found: 2,
        });

        assert!(!failure.retryable, "코드가 고쳐져야 하는 상태다");
    }

    #[test]
    fn a_failed_migration_can_be_retried_because_it_rolled_back() {
        let failure = Failure::from(DatabaseError::Migrate {
            version: 3,
            name: "create_settings",
            source: sqlite_error(),
        });

        assert!(failure.retryable);
    }

    #[test]
    fn no_storage_failure_claims_the_stored_data_was_damaged() {
        // Phase 1의 저장소 실패는 어느 것도 원본을 훼손하지 않는다 (§13의 두 번째 질문).
        let errors = [
            DatabaseError::Open {
                path: PathBuf::from("/nowhere/molt-note.db"),
                source: sqlite_error(),
            },
            DatabaseError::Sql(sqlite_error()),
            DatabaseError::Migrate {
                version: 2,
                name: "create_domain_tables",
                source: sqlite_error(),
            },
            DatabaseError::InvalidMigrationOrder {
                previous: 2,
                found: 1,
            },
            DatabaseError::AheadOfCode {
                database: 9,
                latest: 3,
            },
            DatabaseError::Decode {
                table: "settings",
                column: "automatic_processing",
                value: "2".to_string(),
            },
        ];

        for error in errors {
            let described = error.to_string();
            let failure = Failure::from(error);
            assert!(
                failure.source_data_safe,
                "원본이 안전하다고 답할 수 있어야 한다: {described}"
            );
            assert!(
                !failure.message.is_empty(),
                "보여줄 문장이 비어 있으면 안 된다: {described}"
            );
        }
    }
}
