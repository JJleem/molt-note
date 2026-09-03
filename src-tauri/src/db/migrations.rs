//! 스키마 migration 목록과 실행기.
//!
//! 스키마 버전은 `PRAGMA user_version`이 결정한다. `schema_migrations` 테이블은
//! "무엇이 언제 적용됐는가"를 사람이 읽기 위한 기록이며 버전의 근거는 아니다.
//!
//! **적용된 migration은 다시 실행되지 않고, 이미 있는 데이터를 지우지 않는다.**
//! 스키마가 바뀔 때 저장소를 비우고 다시 만드는 경로는 만들지 않는다 (INV-4의 정신).

use rusqlite::Connection;

use super::DatabaseError;

/// 하나의 스키마 변경.
///
/// `version`은 적용 순서이자 적용 여부의 판단 기준이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// 적용 순서대로 선언된 migration 목록.
///
/// 새 migration은 **항상 목록 끝에 추가한다.** 이미 적용된 적이 있는 migration의
/// `version`이나 `sql`은 고치지 않는다 — 이미 그 스키마로 저장된 사용자 DB가 존재할 수 있고,
/// 되돌리기 위해 저장소를 지우는 것은 선택지가 아니다.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create_schema_migrations",
        sql: "CREATE TABLE IF NOT EXISTS schema_migrations (
              version    INTEGER PRIMARY KEY,
              name       TEXT NOT NULL,
              applied_at TEXT NOT NULL
          );",
    },
    // PRODUCT-SPEC §7의 데이터 모델. 네 개념은 네 개의 서로 다른 테이블이다 —
    // Transcript와 AINote가 겹치지 않는 것이 INV-2의 스키마 수준 표현이다.
    // (`segments[]`만 자식 테이블로 나뉜다. 이유는 docs/ADR-0001-local-persistence.md §5.4.)
    Migration {
        version: 2,
        name: "create_domain_tables",
        sql: "CREATE TABLE IF NOT EXISTS recordings (
              id                    TEXT    PRIMARY KEY,
              title                 TEXT    NOT NULL,
              created_at            TEXT    NOT NULL,
              updated_at            TEXT    NOT NULL,
              duration_ms           INTEGER NOT NULL,
              audio_path            TEXT    NOT NULL,
              audio_format          TEXT    NOT NULL,
              microphone            TEXT,
              -- §7.2: 현재 사용 중인 성공한 Transcript. NULL(값 없음)도 정상 상태다.
              -- 복합 FK는 이 Recording에 속한 Transcript만 가리킬 수 있게 한다.
              -- SQLite는 FK 열 중 하나라도 NULL이면 제약을 만족한 것으로 본다.
              current_transcript_id TEXT,
              transcription_status  TEXT    NOT NULL
                  CHECK (transcription_status IN ('none','pending','running','done','failed')),
              ai_status             TEXT    NOT NULL
                  CHECK (ai_status IN ('none','pending','running','done','failed')),
              notion_status         TEXT    NOT NULL
                  CHECK (notion_status IN ('none','pending','running','done','failed')),
              FOREIGN KEY (current_transcript_id, id)
                  REFERENCES transcripts (id, recording_id)
          );

          -- §7.1: Recording 1:N Transcript. immutable · versioned —
          -- 재전사는 이 테이블의 기존 행을 UPDATE하지 않고 새 행을 추가한다.
          CREATE TABLE IF NOT EXISTS transcripts (
              id           TEXT PRIMARY KEY,
              recording_id TEXT NOT NULL REFERENCES recordings (id),
              language     TEXT,
              raw_text     TEXT NOT NULL,
              created_at   TEXT NOT NULL,
              engine       TEXT NOT NULL,
              model        TEXT NOT NULL,
              -- recordings의 복합 FK가 참조할 수 있도록 (id, recording_id)에 UNIQUE를 준다.
              UNIQUE (id, recording_id)
          );

          CREATE TABLE IF NOT EXISTS transcript_segments (
              transcript_id TEXT    NOT NULL REFERENCES transcripts (id),
              ordinal       INTEGER NOT NULL,
              start_ms      INTEGER NOT NULL,
              end_ms        INTEGER NOT NULL,
              text          TEXT    NOT NULL,
              PRIMARY KEY (transcript_id, ordinal)
          );

          -- §7.1: Transcript 1:N AINote. Transcript와 별개의 테이블이다 (INV-2).
          -- §7.3: transcript_id는 provenance의 일부다 — 같은 Recording에 Transcript가
          -- 여럿일 때 어떤 version에서 나온 노트인지 구분한다.
          CREATE TABLE IF NOT EXISTS ai_notes (
              id             TEXT PRIMARY KEY,
              recording_id   TEXT NOT NULL REFERENCES recordings (id),
              transcript_id  TEXT NOT NULL REFERENCES transcripts (id),
              note_type      TEXT NOT NULL CHECK (note_type IN ('meeting','study','summary')),
              -- §9.3의 provider 중립 structured note. 최종 schema는 Phase 4가 확정하므로
              -- 스키마는 내용을 해석하지 않는 텍스트로 둔다.
              content        TEXT NOT NULL,
              -- INV-9: 벤더 중립 자유 식별자다. 허용 값 목록을 두지 않는다.
              provider       TEXT NOT NULL,
              model          TEXT NOT NULL,
              prompt_version TEXT NOT NULL,
              generated_at   TEXT NOT NULL,
              -- 노트의 transcript_id와 recording_id가 서로 어긋나지 않게 한다.
              FOREIGN KEY (transcript_id, recording_id)
                  REFERENCES transcripts (id, recording_id)
          );

          CREATE TABLE IF NOT EXISTS notion_syncs (
              recording_id TEXT PRIMARY KEY REFERENCES recordings (id),
              page_id      TEXT,
              synced_at    TEXT,
              status       TEXT NOT NULL
                  CHECK (status IN ('none','pending','running','done','failed')),
              error        TEXT
          );

          CREATE INDEX IF NOT EXISTS idx_transcripts_recording
              ON transcripts (recording_id, created_at);
          CREATE INDEX IF NOT EXISTS idx_ai_notes_transcript
              ON ai_notes (transcript_id, generated_at);
          CREATE INDEX IF NOT EXISTS idx_ai_notes_recording
              ON ai_notes (recording_id);",
    },
    // PRODUCT-SPEC §5 D의 설정 값. 한 행짜리 테이블이다 (`id`는 1만 허용).
    //
    // 열에 DEFAULT 절을 두지 않는다 — 기본값 정책은 `domain::settings::Settings::DEFAULT`가
    // 갖는다. 그래야 정책을 바꿀 때 이미 만들어진 사용자 DB를 고치지 않아도 되고,
    // "저장된 적 없음"(행이 없음)과 "기본값과 같은 값을 저장함"이 구분된다.
    //
    // INV-7: secret 열이 없다. API key · integration token · password를 담는 자리를
    // 만들지 않는다 — 저장하지 않는 값은 담을 곳도 없어야 한다.
    Migration {
        version: 3,
        name: "create_settings",
        sql: "CREATE TABLE IF NOT EXISTS settings (
              id                   INTEGER PRIMARY KEY CHECK (id = 1),
              -- NULL은 '아직 고르지 않았다'는 정상 상태다.
              recordings_directory TEXT,
              automatic_processing INTEGER NOT NULL
                  CHECK (automatic_processing IN (0, 1))
          );",
    },
    // PRODUCT-SPEC §5 D의 default microphone. 위의 `create_settings`를 고치지 않고 **열을 더한다** —
    // version 3까지 적용된 사용자 DB가 이미 있을 수 있고, 그 행의 값은 그대로 남아야 한다.
    // 이미 있는 행의 새 열은 NULL, 즉 '아직 고르지 않음'으로 시작한다.
    //
    // 담는 값은 장치가 보여 주는 **이름이 아니라 선택 키**다 (`crate::audio::devices`).
    // 이름이 같은 장치가 둘 있을 수 있어서 이름으로는 어느 것인지 말할 수 없다.
    //
    // 저장된 키가 다음 열거 목록에 없을 수 있다 — 장치를 뽑으면 그렇게 된다. 그것은
    // 스키마가 막을 일이 아니라 **해석하는 쪽이 구분해서 말해야 하는 상태**이며,
    // 조용히 다른 장치로 바꾸지 않는다 (`src/screens/defaultMicrophone.ts`).
    //
    // INV-7: 여기서도 secret 열은 만들지 않는다.
    Migration {
        version: 4,
        name: "add_default_microphone_to_settings",
        sql: "ALTER TABLE settings ADD COLUMN default_microphone TEXT;",
    },
];

/// 코드가 알고 있는 최신 스키마 버전. migration이 없으면 0이다.
pub fn latest_version() -> i64 {
    MIGRATIONS.last().map_or(0, |migration| migration.version)
}

/// DB에 현재 적용된 스키마 버전.
pub fn current_version(connection: &Connection) -> Result<i64, DatabaseError> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(DatabaseError::Sql)
}

/// 아직 적용되지 않은 migration만 순서대로 적용하고, 적용한 버전 목록을 돌려준다.
///
/// 같은 DB에 몇 번을 호출해도 결과는 같다 — 두 번째 호출부터는 적용할 것이 없으므로
/// 빈 목록을 돌려주고 DB를 건드리지 않는다.
pub fn apply_pending(connection: &mut Connection) -> Result<Vec<i64>, DatabaseError> {
    ensure_versions_are_strictly_increasing()?;

    let database_version = current_version(connection)?;
    let latest = latest_version();
    if database_version > latest {
        // 더 새로운 버전의 앱이 만든 DB다. 아는 척하고 쓰면 데이터를 망가뜨릴 수 있으므로
        // 열지 않고 멈춘다. 되돌리기 위해 스키마를 지우지 않는다.
        return Err(DatabaseError::AheadOfCode {
            database: database_version,
            latest,
        });
    }

    let mut applied = Vec::new();
    for migration in MIGRATIONS {
        if migration.version <= database_version {
            continue;
        }
        apply(connection, migration)?;
        applied.push(migration.version);
    }
    Ok(applied)
}

/// migration 하나를 트랜잭션 안에서 적용한다.
///
/// 스키마 변경 · 적용 기록 · 버전 갱신이 한 트랜잭션이므로, 중간에 실패하면
/// DB는 이전 버전 상태 그대로 남는다 (부분 적용된 스키마가 생기지 않는다).
fn apply(connection: &mut Connection, migration: &Migration) -> Result<(), DatabaseError> {
    let failed = |source| DatabaseError::Migrate {
        version: migration.version,
        name: migration.name,
        source,
    };

    let transaction = connection.transaction().map_err(failed)?;
    transaction.execute_batch(migration.sql).map_err(failed)?;
    transaction
        .execute(
            "INSERT INTO schema_migrations (version, name, applied_at)
             VALUES (?1, ?2, datetime('now'))",
            (migration.version, migration.name),
        )
        .map_err(failed)?;
    transaction
        .pragma_update(None, "user_version", migration.version)
        .map_err(failed)?;
    transaction.commit().map_err(failed)
}

/// 목록이 0보다 큰 버전으로 시작해 엄격히 증가하는지 확인한다.
///
/// 어긋난 목록은 "적용 여부를 버전으로 판단한다"는 전제를 깨뜨려 migration을 건너뛰게 만든다.
fn ensure_versions_are_strictly_increasing() -> Result<(), DatabaseError> {
    let mut previous = 0;
    for migration in MIGRATIONS {
        if migration.version <= previous {
            return Err(DatabaseError::InvalidMigrationOrder {
                previous,
                found: migration.version,
            });
        }
        previous = migration.version;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_versions_are_strictly_increasing() {
        ensure_versions_are_strictly_increasing().expect("선언된 목록은 순서를 지켜야 한다");
    }

    #[test]
    fn latest_version_is_the_last_declared_migration() {
        assert_eq!(
            latest_version(),
            MIGRATIONS.last().expect("migration이 최소 하나 있어야 한다").version
        );
    }

    #[test]
    fn no_migration_destroys_existing_data() {
        // 스키마 변경 경로가 사용자 데이터를 지우는 수단을 갖지 않는다 (INV-4의 정신).
        for migration in MIGRATIONS {
            let sql = migration.sql.to_uppercase();
            for forbidden in ["DROP TABLE", "DROP INDEX", "DELETE FROM", "TRUNCATE"] {
                assert!(
                    !sql.contains(forbidden),
                    "migration {} ({})이 데이터를 지우는 문장을 포함한다: {forbidden}",
                    migration.version,
                    migration.name
                );
            }
        }
    }
}
