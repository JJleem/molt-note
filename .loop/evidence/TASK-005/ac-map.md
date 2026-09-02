# TASK-005 — Acceptance Criteria ↔ 검증 대응

실행 방법: `node tools/loop-runtime/loopctl.mjs self-check lint test` (2026-09-02).
결과는 참고용이다 — 완료 판정은 Runtime과 Verifier가 한다.

| AC | 판정 수단 | 결과 | 근거 |
| --- | --- | --- | --- |
| AC1 `npm run lint` | Gate `lint` (`eslint . && cargo clippy --all-targets -- -D warnings`) | PASS exit=0 | `gate-lint.log` |
| AC2 `npm run test` | Gate `test` (`vitest run && cargo test`) | PASS exit=0 | `gate-test.log` |
| AC3 기본값 · 재오픈 후 값 유지 | `src-tauri/tests/settings_repository.rs` | 7 tests passed | 아래 §2 |
| AC4 secret 부재 (INV-7) | 같은 파일의 INV-7 테스트 2개 + 스키마 | 통과 | 아래 §3 |

## 1. 실행 요약

```text
lint: PASS  exit=0  2.3s
test: PASS  exit=0  6.6s
```

`gate-test.log`의 실행 목록에 새 테스트 바이너리가 있다:

```text
Running tests/settings_repository.rs (src-tauri/target/debug/deps/settings_repository-98659e658b3deb79)
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 2. AC3 — 실제 assertion 위치

(a) **값이 없을 때 기본값이 반환된다**

- `defaults_are_returned_when_nothing_has_been_saved`
  - `assert_eq!(loaded, Settings::DEFAULT, …)`
  - `assert_eq!(loaded.recordings_directory, None, …)`
  - `assert!(!loaded.automatic_processing, …)`
- `reading_defaults_does_not_write_a_row`
  - `assert_eq!(rows(&connection), 0, …)` — 기본값을 돌려주는 것이 저장 행위가 아니다.

기본값 정책은 `src-tauri/src/domain/settings.rs`의 `Settings::DEFAULT`에 코드로 선언돼 있다
(스키마에 `DEFAULT` 절을 두지 않았다). 이유는 `docs/ADR-0001-local-persistence.md` §5.5.

(b) **저장한 값이 연결을 닫고 다시 연 뒤에도 그대로 읽힌다**

- `saved_values_survive_closing_and_reopening_the_database`
  - 저장 → `close(first)`(연결을 실제로 닫고, 닫히지 않으면 실패시킨다) → 같은 DB 파일을 새 연결로 다시 연다.
  - `assert_eq!(loaded.recordings_directory, Some("/Users/tester/Molt Note/Recordings".to_string()), …)`
  - `assert!(loaded.automatic_processing, …)`
  - `assert_eq!(loaded, saved, …)`
- `a_toggle_turned_off_is_remembered_and_not_mistaken_for_an_unsaved_value`
  - 켰다가 끈 뒤 재오픈: `assert!(!loaded.automatic_processing, …)` — 끈 값이 '저장한 적 없음'으로 오해되지 않는다.
  - `assert_eq!(rows(&second), 1, …)` — 설정은 한 행이다.
- `saving_settings_does_not_disturb_other_stored_data`
  - 설정 저장이 같은 DB의 다른 데이터를 건드리지 않는다.

테스트는 시스템 임시 디렉터리의 DB 파일만 쓴다 (`TempDir`, Drop에서 삭제).
실제 사용자 앱 데이터 디렉터리를 만들거나 오염시키지 않는다.

## 3. AC4 — secret이 없다 (INV-7)

- `the_settings_schema_has_no_secret_columns`
  - 실행 중인 DB에 `SELECT name FROM pragma_table_info('settings')`를 질의해
    열 목록이 정확히 `[id, recordings_directory, automatic_processing]` 인지 확인한다.
  - 각 열 이름에 `api_key · apikey · token · password · secret · credential` 이 없는지 확인한다.
- `the_settings_api_does_not_accept_or_store_secrets`
  - `src/db/settings.rs` · `src/domain/settings.rs` 소스를 `include_str!`로 읽어
    **코드 줄**(주석 제외)에 위 단어가 없는지 확인한다.
    주석을 제외하는 이유는 그 주석들이 INV-7 자체를 설명하기 때문이다.

이 Task는 secret을 입력받는 UI도, 저장하는 API도, 담는 열도 만들지 않았다.
whisper 모델 선택 · AI provider 설정 · Notion 토큰 같은 기능도 구현하지 않았다.

## 4. 범위 밖으로 나가지 않은 것

- Tauri command를 새로 등록하지 않았다 — 설정 읽기/쓰기 API는 Rust 경계 안에 있다
  (`molt_note_lib::db::settings`). 기존 Recording 저장소도 아직 command로 노출돼 있지 않아
  이 Task만 프론트엔드 경계를 앞당겨 열지 않았다.
- automatic 토글은 **하나**다. §5 D가 나열하는 transcription / AI / Notion 각각의 토글을
  미리 만들지 않았다 (§20.6 — 추상화를 선입금하지 않는다).
- `git commit`을 하지 않았다.
