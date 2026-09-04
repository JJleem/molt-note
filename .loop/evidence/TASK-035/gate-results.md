# TASK-035 — Gate 실행 결과 (self-check, advisory)

실행: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
작업 트리: 이 Run의 변경이 전부 적용된 상태.

```text
build: PASS  exit=0     npm run build   (tsc && vite build)
lint:  PASS  exit=0     npm run lint    (eslint . && cargo clippy --all-targets -- -D warnings)
test:  PASS  exit=0     npm run test    (vitest run && cargo test)
```

원본 출력: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(Runtime 소유 경로이며 Worker가 만든 것이 아니다. Runtime이 Worker 종료 후 Gate를 독립적으로
다시 돌리므로 위 결과는 참고용이다.)

## test Gate가 실제로 돌린 것

```text
vitest run                     15 files / 202 tests passed
cargo test  (unit)            231 passed
cargo test  (integration)     모든 파일 통과 — 아래는 이 Task와 직접 관련된 것
```

`src-tauri/tests/settings_repository.rs` — 21 passed. 이 Run이 더한 것:

```text
not_having_chosen_a_provider_is_the_normal_starting_state            ok   (AC6)
the_default_connection_target_is_declared_in_exactly_one_place       ok
a_chosen_provider_survives_closing_and_reopening_the_database        ok   (AC6)
a_model_that_the_server_no_longer_has_is_still_the_saved_choice      ok
clearing_the_chosen_provider_is_remembered_as_not_chosen             ok
a_database_written_before_the_ai_settings_existed_keeps_its_values   ok   (AC4)
```

이 Run이 값을 더해 강화한 기존 테스트:

```text
defaults_are_returned_when_nothing_has_been_saved                    ok   (AC6)
saved_values_survive_closing_and_reopening_the_database              ok   (AC6)
the_settings_schema_has_no_secret_columns                            ok   (AC5)
the_settings_api_does_not_accept_or_store_secrets                    ok   (AC5)
```

`src-tauri/src/db/migrations.rs` 단위 테스트:

```text
released_migrations_keep_their_version_and_name                             ok   (AC4)
a_new_settings_column_is_added_by_a_new_migration_instead_of_editing_an_old_one  ok   (AC4)
no_migration_destroys_existing_data                                         ok   (AC4)
no_migration_creates_a_place_to_put_a_secret                                ok   (AC5, 이 Run이 추가)
```

`src-tauri/tests/command_boundary.rs` — 17 passed. `updated_settings_are_stored_and_read_back`가
payload 왕복에 세 값을 포함하도록 강화됐다 (AC7).

`src/screens/settingsView.test.ts` — 편집하지 않는 AI 설정이 저장 왕복에서 사라지지 않는지
보는 테스트가 추가됐다.
