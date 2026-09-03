# TASK-029 — 새로 추가·갱신된 테스트의 실제 실행 출력

`npm run test`(test Gate) 실행 중 `cargo test`가 낸 출력에서 이 Task와 직접 관련된 두
binary를 그대로 옮긴다. 전체 출력은 `.loop-local/self-check/gates/test/stdout.log`에 있었고,
그 파일은 다음 self-check 실행이 덮어쓴다.

## `src-tauri/tests/automatic_transcription.rs` (이 Task가 만든 파일 · AC6 · AC7)

```
running 7 tests
test a_missing_model_is_reported_as_a_failure_and_the_toggle_is_left_as_the_user_set_it ... ok
test a_stop_starts_a_transcription_only_when_automatic_transcription_is_on ... ok
test a_failed_stop_does_not_start_a_transcription ... ok
test a_stop_starts_nothing_when_automatic_transcription_is_off ... ok
test the_default_settings_do_not_start_a_transcription_after_a_stop ... ok
test automatic_processing_is_a_different_toggle_and_does_not_start_a_transcription ... ok
test a_manual_transcription_works_while_automatic_transcription_is_off ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
```

## `src-tauri/tests/settings_repository.rs` (AC4 · AC5 · AC7)

```
running 15 tests
test a_database_written_before_the_default_microphone_existed_keeps_its_values ... ok
test a_database_written_before_the_transcription_settings_existed_keeps_its_values ... ok
test the_settings_api_does_not_accept_or_store_secrets ... ok
test defaults_are_returned_when_nothing_has_been_saved ... ok
test reading_defaults_does_not_write_a_row ... ok
test saved_values_survive_closing_and_reopening_the_database ... ok
test a_chosen_transcription_model_survives_closing_and_reopening_the_database ... ok
test a_toggle_turned_off_is_remembered_and_not_mistaken_for_an_unsaved_value ... ok
test a_default_microphone_that_is_no_longer_plugged_in_is_still_the_saved_choice ... ok
test a_model_file_that_is_no_longer_there_is_still_the_saved_choice ... ok
test clearing_the_default_microphone_is_remembered_as_not_chosen ... ok
test the_settings_schema_has_no_secret_columns ... ok
test saving_settings_does_not_disturb_other_stored_data ... ok
test turning_automatic_transcription_on_does_not_touch_the_other_settings ... ok
test the_two_automatic_toggles_are_stored_and_restored_independently ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

## 나머지

`db::migrations::tests::released_migrations_keep_their_version_and_name`과
`db::migrations::tests::a_new_settings_column_is_added_by_a_new_migration_instead_of_editing_an_old_one`
(AC5)은 unit 테스트 binary(172개)에 있고 둘 다 `ok`였다.
`command_boundary.rs`(15) · `stop_persistence.rs`(6) · `transcription_background.rs`(10)도
전부 통과했다 — 요약은 `gates.md`에 있다.

화면 쪽(`src/screens/settingsView.test.ts`)은 vitest가 돌렸다: `14 files / 164 tests passed`.
vitest는 개별 이름을 출력하지 않으므로 여기 옮길 줄이 없다. 실패했다면 Gate가 exit 0을
내지 않았다.
