# TASK-045 — self-check 결과 (advisory)

`node tools/loop-runtime/loopctl.mjs self-check build lint test` · 2026-09-04 ·
작업 트리 = 이 Task의 변경이 전부 반영된 상태.

> Runtime이 Worker 종료 뒤 Gate를 독립적으로 다시 돌린다. 아래는 참고용 기록이다.

```text
[build] npm run build      build: PASS  exit=0
[lint]  npm run lint       lint:  PASS  exit=0
[test]  npm run test       test:  PASS  exit=0
Self-check: all gates passed
```

원본 출력: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(Runtime 소유 디렉터리이며 다음 self-check가 덮어쓴다).

## build — `npm run build`

```text
> tsc && vite build      exit=0
```

## lint — `npm run lint`

```text
> eslint .                                                              exit=0
> cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
    Checking molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.38s   exit=0
```

경고 하나도 남기지 않았다 (`-D warnings`).

## test — `npm run test`

### web (vitest)

```text
Test Files  18 passed (18)
Tests       306 passed (306)
```

`tests/ipc-boundary.test.ts`가 등록된 command 목록을 정확히 같은 집합으로 요구하므로,
`export_markdown` 추가는 그 목록과 `src/ipc/commands.ts`의 호출 이름 양쪽에 함께 반영돼야
통과한다. 새로 더한 검사: **"Markdown export 표면은 파일 하나를 만드는 이름 하나뿐이다"**
(일괄 export가 생기면 여기서 먼저 드러난다).

### rust (cargo test)

lib 단위 테스트 326 passed · 통합 테스트 전부 passed · 실패 0.

이 Task가 더한 테스트 (`... ok`로 기록됨):

```text
tests/markdown_export.rs (12)
  a_recording_becomes_a_markdown_file_whose_name_and_body_follow_the_spec .......... ok   (AC4)
  a_title_full_of_dangerous_characters_still_lands_in_the_exports_directory ........ ok   (AC4)
  the_most_recently_generated_note_is_the_one_that_ends_up_in_the_document ......... ok
  a_recording_with_no_ai_note_at_all_is_exported_as_a_valid_document ............... ok   (AC5 · INV-8)
  exporting_reads_no_audio_and_copies_none ........................................ ok   (INV-6)
  a_directory_that_cannot_be_created_fails_visibly_and_changes_nothing ............. ok   (AC6 · INV-3)
  a_write_that_fails_is_a_domain_failure_that_leaves_the_stored_data_untouched ..... ok   (AC6 · INV-3)
  a_recording_that_is_not_there_is_refused_without_creating_a_file ................. ok   (AC6)
  a_recording_without_a_transcript_is_refused_instead_of_leaving_an_empty_document . ok   (AC6)
  exporting_the_same_recording_again_never_overwrites_the_earlier_file ............. ok   (AC7)
  two_recordings_with_the_same_title_and_date_do_not_collapse_into_one_file ........ ok   (AC7)
  the_note_type_of_the_stored_row_is_the_one_the_document_uses ..................... ok

src-tauri/src/export/file.rs (6)
  the_requested_name_is_used_when_nothing_is_there ................................. ok
  a_name_that_is_taken_is_stepped_over_instead_of_overwritten ..................... ok   (AC7)
  the_number_goes_before_the_extension ............................................ ok
  running_out_of_names_is_a_visible_failure_rather_than_an_overwrite .............. ok   (AC7)
  a_directory_that_is_not_there_becomes_a_readable_failure ....................... ok   (AC6)
  the_written_path_is_the_full_path_the_user_can_be_shown ........................ ok

src-tauri/src/export/run.rs (3)
  a_recording_that_is_not_there_is_told_apart_from_a_storage_problem .............. ok
  having_no_transcript_yet_says_what_to_do_and_leaves_everything_alone ............ ok
  a_transcript_row_that_cannot_be_read_is_a_storage_failure_not_an_export_bug ..... ok

src-tauri/src/commands/export.rs (2)
  a_directory_that_could_not_be_resolved_is_reported_instead_of_panicking ......... ok
  asking_to_export_nothing_is_refused_before_the_storage_is_opened ................ ok

src-tauri/src/platform/app_data_dir.rs (3)
  the_exports_directory_is_derived_from_the_same_root_as_everything_else .......... ok
  ensuring_the_exports_directory_keeps_the_documents_already_exported ............. ok
  an_exports_directory_that_cannot_be_created_becomes_a_readable_failure .......... ok
```

**모든 파일 쓰기 테스트는 `std::env::temp_dir()` 아래에서만 돈다.** 사용자의 실제 export
디렉터리(`<app data root>/exports`)를 만들거나 건드리는 테스트가 없다.
