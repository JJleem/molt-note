# TASK-057 — Gate 실행 결과 (Worker self-check · 2026-09-04)

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
(Runtime 소유 진입점. `.loop/project.yaml`의 gate 명령만 실행한다.)

```text
[build] npm run build   → build: PASS  exit=0  1.1s
        tsc && vite build

[lint]  npm run lint    → lint:  PASS  exit=0  5.1s
        eslint . && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

[test]  npm run test    → test:  PASS  exit=0  36.5s
        vitest run && cargo test --manifest-path src-tauri/Cargo.toml

Self-check: all gates passed
```

원본 출력: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(Runtime 소유 디렉터리이며, 이 파일은 그 결과를 옮겨 적은 것이다. **advisory이지 gate 판정이
아니다** — Runtime이 Worker 종료 후 Gate를 독립적으로 다시 돌린다.)

## 총계

```text
vitest        Test Files 21 passed (21) · Tests 384 passed (384)     (stdout.log 13–14행)
cargo lib     test result: ok. 436 passed; 0 failed                  (461행)
              (TASK-056 종료 시점 430 → 이 Task가 더한 lib 단위 테스트 6개)
```

## 이 Task가 더한 통합 테스트 10개 (전부 `ok`)

`src-tauri/tests/manual_ai_handoff.rs` — `.loop-local/self-check/gates/test/stdout.log` 637–648행.

```text
running 10 tests
test nothing_in_the_manual_handoff_boundary_can_reach_a_provider_a_network_or_a_write ... ok
test the_three_commands_take_a_recording_id_and_never_a_transcript_id ... ok
test the_written_document_is_exactly_what_the_pure_renderer_makes ... ok
test the_ai_request_lands_in_the_same_exports_directory_under_its_own_name ... ok
test a_second_ai_request_gets_a_number_instead_of_overwriting_the_first ... ok
test asking_for_a_recording_that_is_not_there_leaves_the_others_alone ... ok
test a_recording_without_a_current_transcript_is_refused_and_nothing_changes ... ok
test only_the_transcript_that_current_points_at_reaches_any_of_the_three_outputs ... ok
test a_transcript_with_nothing_in_it_is_refused_instead_of_making_an_empty_request ... ok
test all_three_outputs_are_produced_without_any_ai_provider_configured ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## 이 Task가 더한 lib 단위 테스트 6개 (전부 `ok`)

```text
test export::filename::tests::the_ai_request_name_is_the_export_name_with_one_marker_before_the_extension ... ok
test export::filename::tests::the_ai_request_name_inherits_every_rule_of_the_export_name ... ok
test export::filename::tests::the_same_recording_always_gets_the_same_ai_request_name ... ok
test export::handoff::tests::a_recording_that_is_not_there_is_told_apart_from_a_storage_problem ... ok
test export::handoff::tests::having_nothing_to_hand_off_says_what_to_do_and_leaves_everything_alone ... ok
test commands::export::tests::the_ai_request_path_answers_the_same_two_failures_the_same_way ... ok
```

## 고치지 않은 채로 통과한 기존 검사 (MH-8 · AC-4)

```text
tests/markdown_export.rs                12 passed   §11의 export 산출물과 실패 경로 (한 줄도 고치지 않았다)
tests/notion_and_export_invariants.rs   13 passed   INV-3 · INV-6 · INV-7 · INV-9
tests/ai_note_commands.rs                       "  Connected Provider 경로 (MH-8)
tests/screen-boundary.test.ts · src/screens/**  vitest 384개 전부 — 화면 쪽 파일은 이 Task가 건드리지 않았다
src-tauri/src/ai/**                     diff 없음   프롬프트 상수도 provider 추상화도 그대로다 (MH-8)
```

`git diff --stat -- src-tauri/src/ai`는 출력이 없다.
