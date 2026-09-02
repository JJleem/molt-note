# TASK-002 — Acceptance Criteria 대응표 (2026-09-02)

Run: `RUN-20260902T033241Z-TASK-002`

각 AC가 **무엇으로 판정되는지**와 그 판정 수단이 어디에 있는지를 적는다.
이것은 완료 선언이 아니다 — 판정은 Gate 와 Verifier 가 한다.

| AC | 판정 수단 | 위치 |
| --- | --- | --- |
| AC1 lint | Gate `lint` (exit 0) | `.loop/evidence/TASK-002/gate-run-20260902.log` |
| AC2 test | Gate `test` (exit 0, 22 web + 14 rust) | 같은 파일 |
| AC3 (a)(b)(c) | cargo 테스트 3개 | `src-tauri/src/db/mod.rs` |
| AC4 ADR | 문서 + 로컬 산출물 | `docs/ADR-0001-local-persistence.md` · `crate-resolution.txt` |
| AC5 파괴 경로 없음 | 코드 스캔 + 테스트 | `src-tauri/src/db/migrations.rs` |

---

## AC3 — 세 시나리오가 실제 assertion 인가

모든 테스트는 `std::env::temp_dir()` 아래에 만든 빈 디렉터리에서만 동작하며
(`TempDir`, `src-tauri/src/db/mod.rs:110-134`), Drop 에서 지운다.
실제 사용자 앱 데이터 디렉터리를 만들거나 읽지 않는다.

### (a) 빈 디렉터리에서 DB 생성 + 스키마 버전 최신

`creates_the_database_and_reaches_the_latest_schema_version_in_an_empty_directory`
(`src-tauri/src/db/mod.rs:156`)

```
assert!(!path.exists())                                  // 사전 조건: 파일이 아직 없다
let connection = open(&path)
assert!(path.is_file())                                  // 파일이 생겼다
assert_eq!(current_version(&connection), latest_version())
assert_eq!(ledger(&connection).len(), MIGRATIONS.len())
```

### (b) 연결을 닫았다가 같은 경로로 다시 열면 기존 행이 남아 있다

`rows_survive_closing_and_reopening_the_same_path` (`src-tauri/src/db/mod.rs:192`)

**같은 연결에서 SELECT 만 하는 것이 아니다.** 두 개의 별도 `Connection` 을 쓴다:

```
let first = open(&path)                  // 연결 1
first.execute_batch("CREATE TABLE persistence_probe ...; INSERT ... VALUES (1, ...)")
close(first)                             // ← Connection::close() 를 실제로 호출한다
                                         //    실패하면 테스트가 실패한다 (mod.rs:137-142)

let second = open(&path)                 // 연결 2 — 같은 경로를 새로 연다
let note: String = second.query_row("SELECT note FROM persistence_probe WHERE id = 1", ...)
assert_eq!(note, "재시작 후에도 남아야 한다")
assert_eq!(ledger(&second), applied_before)
```

`close()` 헬퍼는 `Connection::close()` 의 오류를 `.expect()` 로 승격시키므로,
연결이 닫히지 않은 채 테스트가 통과하는 경로가 없다.

### (c) migration 을 두 번 실행해도 데이터가 유실되지 않는다

`running_migrations_again_keeps_existing_data_and_history` (`src-tauri/src/db/mod.rs:224`)

```
first  = open(&path)   → migration 1회차 (신규 DB)
        데이터 1행 삽입, 적용 기록·버전 스냅샷
close(first)
second = open(&path)   → migration 2회차 (open 이 apply_pending 을 호출)
apply_pending(&mut second) → 3회차를 명시적으로 한 번 더

assert!(applied_now.is_empty())            // 재적용이 일어나지 않았다
assert_eq!(current_version(&second), version_before)
assert_eq!(ledger(&second), applied_before) // applied_at 시각까지 동일 = 기록을 다시 쓰지 않았다
assert_eq!(rows, 1)                         // 데이터가 그대로다
```

실행 횟수는 2회가 아니라 3회다 (open × 2 + 명시적 호출 1).

### 범위 밖이지만 같은 불변을 지키는 테스트

- `refuses_a_database_written_by_a_newer_schema_without_touching_it` (`mod.rs:267`) —
  DB 버전이 코드보다 높으면 `AheadOfCode` 로 거부하고, **거부 후에도 기존 행이 남아 있음**을 확인한다.
- `foreign_key_enforcement_is_enabled_on_the_connection` (`mod.rs:303`)
- `open_derives_the_database_path_from_the_app_data_directory` (`mod.rs:178`) —
  DB 가 `AppDataDirectory::database_path()` 경로에 만들어지는지 확인한다 (Task request 의 경로 요구사항).

---

## AC5 — 데이터를 지우는 경로가 코드에 없는가

저장소 전체 스캔:

```
$ grep -rniE "drop table|drop index|delete from|truncate|remove_file|remove_dir" src-tauri/src/

src-tauri/src/platform/app_data_dir.rs:127:  let _ = std::fs::remove_dir_all(&self.0);
src-tauri/src/db/mod.rs:132:                 let _ = std::fs::remove_dir_all(&self.0);
src-tauri/src/db/migrations.rs:144:          for forbidden in ["DROP TABLE", "DROP INDEX", "DELETE FROM", "TRUNCATE"] {
```

세 건 모두 파괴 경로가 아니다:

- 앞의 두 건은 **`#[cfg(test)]` 안의 `TempDir`/`TempRoot::drop`** 이다. 테스트가 자기 임시
  디렉터리를 치우는 코드이며 제품 경로에 포함되지 않는다.
- 세 번째는 `no_migration_destroys_existing_data` 테스트의 **금지어 목록**이다 — 파괴 문장을
  실행하는 것이 아니라, migration SQL 에 그런 문장이 들어오는 것을 막는 가드다.

migration 실행기(`migrations::apply_pending` · `migrations::apply`)에는 `DROP` / `DELETE` /
파일 삭제 후 재생성이 없다. 버전이 코드보다 높은 DB 를 만났을 때조차 **초기화하지 않고
`AheadOfCode` 오류로 멈춘다** (`migrations.rs:59-66`) — INV-4 의 정신에 따라 다운그레이드로
데이터를 망가뜨리는 대신 열기를 거부한다.

---

## AC4 — 외부 사실 재확인 시도 (2026-09-02)

`WebFetch` 로 `https://crates.io/api/v1/crates/rusqlite` 를 열어 ADR §4 의 UNVERIFIED 두 항목
(0.41 이상 계열의 존재 여부 · docs.rs 의 `bundled` 공식 설명)을 해소하려 했으나,
**도구 권한이 거부되어 호출이 실행되지 않았다.**

따라서 두 항목은 UNVERIFIED 로 남겼고, 추측으로 채우지 않았다.
ADR §4 에 시도 사실과 거부 사실을 명시했다.

Cargo.lock 재확인 (VERIFIED · local):

```
$ grep -n -A3 'name = "rusqlite"|name = "libsqlite3-sys"' src-tauri/Cargo.lock
2018:name = "libsqlite3-sys"
2019-version = "0.38.2"
2883:name = "rusqlite"
2884-version = "0.40.2"
```

번들된 SQLite (VERIFIED · local):

```
$ grep -o 'SQLITE_VERSION[^;]*' src-tauri/target/debug/build/libsqlite3-sys-*/out/bindgen.rs
SQLITE_VERSION: &::core::ffi::CStr = c"3.53.2"
SQLITE_VERSION_NUMBER: i32 = 3053002
```

ADR 의 서술과 Cargo.lock 의 실제 해석 버전이 일치한다.
