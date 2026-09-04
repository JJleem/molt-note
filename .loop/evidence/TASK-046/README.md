# TASK-046 — Evidence

Notion 설정을 secret과 비-secret으로 갈라 각각의 자리를 만들었다.

> **이 디렉터리에는 실제 자격증명이 하나도 없다** (ADR-0009 §10.5). 테스트가 쓰는 값은
> `molt-note-test-double-value-not-a-real-credential` 같은 double 문자열이며, 이 Run은
> 어떤 OS 자격증명 저장소에도 접근하지 않았다.

## 파일

| 파일 | 무엇인가 |
| --- | --- |
| `self-check.txt` | `loopctl self-check build lint test`의 출력 — 세 Gate 전부 exit 0 |
| `secret-store-tests.txt` | 이 Task가 더한 테스트들의 실행 결과 줄 (`cargo test` 출력에서 추출) |
| `keyring-verification.md` | ADR-0009 §10.3이 UNVERIFIED로 남긴 crate/feature 사실을 확인한 기록과, **확인하지 못한 것** |
| `task-046.diff` | 이 Task가 만진 제품 파일의 diff |
| `changed-files.txt` | `git diff --stat` + `git status --porcelain` |

⚠️ `changed-files.txt`와 `task-046.diff`에는 **이 Task 이전의 커밋되지 않은 작업**(TASK-045의
markdown export 등)이 섞여 있다. 이 Task가 실제로 만진 파일은 아래 목록이다.

```text
src-tauri/Cargo.toml                      keyring 한 줄
src-tauri/Cargo.lock                      그 해석 결과
src-tauri/src/platform/secret_store.rs    새 파일 — SecretStore 경계
src-tauri/src/platform/mod.rs             모듈 등록
src-tauri/src/db/migrations.rs            version 7을 목록 끝에 추가
src-tauri/src/db/settings.rs              열 하나를 저장/복원에 잇는다
src-tauri/src/domain/settings.rs          notion_parent_page_id
src-tauri/src/commands/payload.rs         SettingsPayload.notion_parent_page_id
src/ipc/types.ts                          Settings.notionParentPageId
src/screens/settingsView.ts               폼 왕복 (값이 조용히 지워지지 않게)
src-tauri/tests/secret_store.rs           새 파일 — 경계 검증
src-tauri/tests/settings_repository.rs    version 7 · destination 영속성
src-tauri/tests/command_boundary.rs       payload 왕복
src/screens/settingsView.test.ts          폼 왕복 · 지워지지 않음
src/screens/aiProviderSettings.test.ts    fixture
```

## Acceptance Criteria 대조

| AC | 무엇이 판정했는가 |
| --- | --- |
| **P4-AC1** build | `self-check.txt` — `build: PASS exit=0` |
| **P4-AC2** lint | `self-check.txt` — `lint: PASS exit=0` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) |
| **P4-AC3** test | `self-check.txt` — `test: PASS exit=0`. 기존 migration 불변 테스트 두 개(`no_migration_creates_a_place_to_put_a_secret` · `released_migrations_keep_their_version_and_name`)가 **고쳐지지 않은 채** 통과한다. 후자는 목록 끝에 `(7, "add_notion_settings")`가 붙은 것만 반영했다 |
| **P4-AC4** 경계 하나 | `platform_branching_and_the_credential_crate_live_only_inside_the_boundary` (`tests/secret_store.rs`) — `src/` 전체를 훑어 `keyring` · `security_framework` · `windows_sys`가 `platform/secret_store.rs` 밖에 하나도 없음을 확인하고, 그 파일 안에는 macOS · Windows · 그 밖(`Unsupported`) 세 자리가 다 있음을 확인한다 |
| **P4-AC5** 실제 저장소 미접근 | `no_automated_test_stands_up_the_real_credential_store` — `src/`와 `tests/` 전체에서 실제 구현 타입을 코드로 세우는 자리가 경계 파일 밖에 없음을 확인한다. `.loop-local/self-check/gates/test/stdout.log`에 `OsSecretStore`를 실행한 테스트가 하나도 없다 |
| **P4-AC6** secret 자리 없음 · 끝에 추가 | `the_settings_schema_has_no_secret_columns` · `the_settings_api_does_not_accept_or_store_secrets` · `no_migration_creates_a_place_to_put_a_secret` · `a_new_settings_column_is_added_by_a_new_migration_instead_of_editing_an_old_one` · `released_migrations_keep_their_version_and_name` · `a_database_written_before_the_notion_settings_existed_keeps_its_values`(version 6 DB가 열만 얻고 값이 그대로 남는다) |

## 이 Task가 **하지 않은** 것

- Notion integration 자격증명을 받거나 돌려주는 **Tauri command를 만들지 않았다.** ADR-0009
  §10.4의 경로는 그 화면이 실재하는 Task가 만든다. 지금 있는 것은 경계 하나뿐이다.
- **미래 Cloud AI 자격증명을 위한 일반화를 만들지 않았다** (ADR-0009 §10.6). `SecretKey`에
  변형이 하나뿐이고, `tests/secret_store.rs`의 `the_list_of_secrets_this_app_keeps_is_closed`가
  그 사실을 고정한다.
- **`notion_syncs`의 전송 상태(ADR-0009 §8.4의 version 8)를 만들지 않았다.** 이 Task가 요구받은
  것은 settings의 version 7 하나다.
- Settings 화면에 Notion destination 입력란을 만들지 않았다. 폼은 값을 **지나가게만** 한다 —
  그러지 않으면 다른 설정을 저장할 때마다 고른 값이 조용히 지워지기 때문이다.

## 알려진 한계 (확인하지 못한 것)

**macOS Keychain에 실제로 저장·조회·삭제가 되는지는 이 Run이 확인하지 않았다.** 자동 테스트가
실제 자격증명 저장소를 건드리지 않는다는 것이 ADR-0009 §10.2의 결정이며, 그 결정을 지키면
그 사실은 자동으로 확인되지 않는다. 이것은 Phase Goal의 Human Review 항목이다.
`keyring-verification.md` §3에 확인하지 못한 것을 전부 적었다.
