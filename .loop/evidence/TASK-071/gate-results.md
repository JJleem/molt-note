# TASK-071 — Gate 결과와 판정 근거

Run: `RUN-20260906T155109Z-TASK-071` · 2026-09-07 (worker 시각 기준)

실행한 명령 (Runtime이 허용한 진입점 하나):

```text
node tools/loop-runtime/loopctl.mjs self-check build lint test
```

## 1. Gate 결과 (exit code)

```text
[build] npm run build   PASS  exit=0   1.1s
[lint]  npm run lint    PASS  exit=0   4.2s
[test]  npm run test    PASS  exit=0  42.6s
```

원문 로그: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(Runtime 소유 경로이며 다음 self-check가 덮어쓴다. 아래에 판정에 쓴 부분을 옮겨 적는다.)

`npm run lint` = `eslint . && cargo clippy --all-targets -- -D warnings`
`npm run test` = `vitest run && cargo test --manifest-path src-tauri/Cargo.toml`

## 2. test Gate 요약

```text
vitest:  Test Files  27 passed (27)
         Tests      526 passed (526)

cargo test: 전체 통과. 이 Task가 더한 자리:

  running 7 tests                              (src-tauri/tests/show_saved_file.rs)
  test asking_to_open_something_never_creates_a_directory_or_a_file ... ok
  test the_file_this_app_wrote_is_the_one_that_reaches_the_os_boundary ... ok
  test nothing_outside_the_exports_directory_ever_reaches_the_os_boundary ... ok
  test a_failure_from_the_os_boundary_arrives_as_a_failure_the_user_can_read ... ok
  test only_one_place_decides_what_may_be_opened ... ok
  test os_calls_and_platform_branching_live_only_inside_the_boundary ... ok
  test no_automated_test_stands_up_the_real_file_manager ... ok
  test result: ok. 7 passed; 0 failed

  commands::saved_file::tests::a_file_this_app_wrote_is_the_one_that_opens ... ok
  commands::saved_file::tests::a_path_outside_the_exports_directory_is_refused ... ok
  commands::saved_file::tests::walking_out_of_the_exports_directory_does_not_work ... ok
  commands::saved_file::tests::a_symlink_that_points_outside_is_refused_too ... ok
  commands::saved_file::tests::the_exports_directory_itself_is_not_a_thing_to_open ... ok
  commands::saved_file::tests::a_file_that_is_gone_says_so_without_creating_anything ... ok
  commands::saved_file::tests::asking_before_anything_was_exported_does_not_create_the_directory ... ok
  commands::saved_file::tests::asking_to_open_nothing_is_refused_before_anything_is_touched ... ok
  commands::saved_file::tests::a_directory_that_could_not_be_resolved_is_reported_instead_of_panicking ... ok

  platform::file_manager::tests::what_was_asked_for_is_what_reaches_the_boundary ... ok
  platform::file_manager::tests::a_failure_carries_no_path_and_no_os_message ... ok
  platform::file_manager::tests::both_messages_say_the_file_is_still_where_the_path_says ... ok
  platform::file_manager::tests::the_system_without_a_file_manager_says_so_instead_of_doing_nothing ... ok
  platform::file_manager::tests::a_failing_manager_never_records_a_request ... ok
  platform::file_manager::tests::the_boundary_is_object_safe_so_a_double_can_stand_where_the_real_one_does ... ok
```

**자동 테스트는 실제 파일 관리자를 한 번도 세우지 않는다.** 전 경로가
`platform::file_manager::testing::RecordedFileManager`(창을 띄우지 않는 double)로 지나가며,
그 사실 자체를 `no_automated_test_stands_up_the_real_file_manager`가 소스에서 확인한다
(`tests/secret_store.rs`가 Keychain에 대해 쓰는 것과 같은 규약).

## 3. Acceptance Criteria → 무엇이 판정했는가

| AC | 판정 |
| --- | --- |
| AC1 `npm run build` | build Gate PASS (exit=0) |
| AC2 `npm run lint` | lint Gate PASS (exit=0 · clippy `-D warnings` 포함) |
| AC3 `npm run test` + 허용 목록 주석 | test Gate PASS. `tests/ipc-boundary.test.ts`의 `허용된 목록과 정확히 같은 command만 등록되어 있다` · `export 표면은 파일 하나를 만드는 이름 둘뿐이다`가 통과하며, 무엇이 왜 늘었는지가 그 파일의 `REGISTERED_COMMANDS` 문서와 두 검사의 주석에 있다 |
| AC4 OS 호출이 `platform/`에만 · 대상이 exports 아래로 제한 | `src-tauri/tests/show_saved_file.rs`의 소스 검사 둘(`os_calls_and_platform_branching_live_only_inside_the_boundary` · `only_one_place_decides_what_may_be_opened`)과 행동 검사 둘(`nothing_outside_the_exports_directory_ever_reaches_the_os_boundary` 등) |
| AC5 순수 모듈의 값 · 컴포넌트는 그리기만 · 경로는 여전히 보인다 | `src/screens/savedFileView.test.ts` · `exportView.test.ts`의 '만들어진 파일이 놓인 자리를 여는 수단이 함께 있다' · `aiHandoffView.test.ts`의 두 검사 · `tests/screen-boundary.test.ts`의 '자리를 여는 규칙은 순수 모듈에 있고 OS를 알지 않는다' |

## 4. 이 Run이 확인하지 못한 것 (UNVERIFIED)

- **macOS `open -R`과 Windows `explorer /select,`를 실제로 실행해 보지 못했다.** 이 Run은
  Gate 명령만 실행할 수 있고, 자동 테스트는 실제 파일 관리자를 세우지 않는 것이 규약이다.
  두 이름과 플래그는 각 플랫폼의 관례를 따른 것이며, 그 사실을
  `src-tauri/src/platform/file_manager.rs`의 문서에 UNVERIFIED로 적어 두었다.
- 그래서 **확인하지 못한 것에 제품을 걸지 않았다**: 띄우지 못하면 §13의 정의된 실패
  (`FailureKind::Storage`)가 되고, 만들어진 파일의 전체 경로는 화면에 그대로 남는다.
  실제로 열리는지는 이 Phase의 Human Review 항목이다 (`phase-prompt/05.6`).
- 각 플랫폼이 어떤 종료 코드로 끝나는지도 확인하지 못했다. 그래서 판정은 **띄울 수
  있었는가**까지이며, 종료 코드를 실패 판정에 쓰지 않는다.
