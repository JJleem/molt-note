# TASK-050 · self-check 결과

실행: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
(Runtime 소유 진입점. 아래 결과는 참고용이며 완료 판정이 아니다 — Runtime이 Worker 종료 후
Gate를 독립적으로 다시 돌린다.)

| Gate | 명령 (`.loop/project.yaml`) | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | 0 | PASS |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | 0 | PASS |
| test | `npm run test` (`vitest run` + `cargo test`) | 0 | PASS |

세 Gate 모두 이 Task의 변경을 포함한 상태에서 통과했다 (P8-AC1 · P8-AC2 · P8-AC3).

## test Gate가 실제로 무엇을 돌렸는가

```text
vitest run     Test Files  18 passed (18)
               Tests      316 passed (316)
cargo test     lib unit    412 passed; 0 failed
               integration 22개 test binary 전부 ok; 0 failed
```

이 Task가 더한 테스트는 다음과 같이 실행됐다 (`cargo test` 출력에서 그대로 옮긴 것이다).

```text
# src-tauri/src/commands/notion.rs (unit)
test commands::notion::tests::a_stored_token_is_answered_as_a_fact_and_never_as_a_value ... ok
test commands::notion::tests::a_blank_token_is_refused_without_echoing_what_was_sent ... ok
test commands::notion::tests::a_pasted_token_keeps_its_value_but_not_the_whitespace_around_it ... ok
test commands::notion::tests::nothing_confirmed_is_the_answer_when_the_screen_says_nothing ... ok
test commands::notion::tests::an_unknown_confirmation_is_refused_instead_of_being_guessed ... ok

# src-tauri/tests/command_boundary.rs (integration · P8-AC3의 "command 경계 테스트")
test a_connection_test_without_a_stored_token_is_a_state_not_a_failure ... ok
test a_connection_test_separates_a_rejected_token_from_a_destination_it_cannot_reach ... ok
test a_stored_token_is_answered_as_a_fact_and_never_as_a_value ... ok
test a_blank_token_is_refused_without_echoing_what_was_sent ... ok
test a_second_send_while_one_is_running_is_refused_instead_of_disappearing ... ok
test a_recording_that_was_never_sent_has_no_stored_sync_and_that_is_not_a_failure ... ok
test a_partly_sent_recording_says_so_through_the_command_surface ... ok
```

IPC 경계 테스트(`tests/ipc-boundary.test.ts`)는 vitest의 18개 파일 중 하나로 함께 돌았다.
그 파일은 **등록된 command 집합과 정확히 같은 집합**을 요구하므로, 이 Task가 command 여섯을
등록하고 목록을 갱신하지 않았다면 test Gate가 실패한다 (P8-AC3 · P8-AC6).

## 실제 Notion · 실제 자격증명 저장소에 닿지 않았다 (§18)

새로 더한 테스트가 쓰는 것은 `notion::testing::StubServer`(소켓을 열지 않는 값 double)와
`platform::secret_store::testing::InMemorySecretStore`(메모리 안에서만 사는 저장소), 그리고
붙잡혀 있는 자격증명 저장소 double(`PausedSecretStore`, 테스트 파일 안에 있다)뿐이다.
`OsSecretStore`를 세우는 자리는 없으며, 그 사실은 기존 검사
(`src-tauri/tests/secret_store.rs::no_automated_test_stands_up_the_real_credential_store`)가
소스에서 다시 확인한다 — 그 테스트도 이번 실행에서 통과했다.
