# TASK-034 self-check (advisory)

```text
명령: node tools/loop-runtime/loopctl.mjs self-check build lint test
일시: 2026-09-03
```

| Gate | 명령 | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` (tsc && vite build) | 0 | PASS (0.9s) |
| lint | `npm run lint` (eslint . && cargo clippy --all-targets -- -D warnings) | 0 | PASS (3.3s) |
| test | `npm run test` (vitest run && cargo test) | 0 | PASS (17.5s) |

원본 로그: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(Runtime 소유 경로 · advisory. 완료 판정은 Runtime이 Gate를 다시 돌려 내린다.)

## 이 Task가 더한 테스트가 실제로 돌았다

`cargo test` 총 391개 중 새로 더한 것들(발췌, 전부 `ok`):

```text
ai::provider::tests::the_generation_request_has_nowhere_to_put_audio ... ok
ai::provider::tests::a_request_carries_the_budget_the_caller_chose ... ok
ai::provider::tests::the_five_ai_failures_never_collapse_into_one ... ok
ai::provider::tests::retrying_is_worth_it_only_where_the_situation_can_change ... ok
ai::provider::tests::both_request_outcomes_are_the_same_kind_of_failure ... ok
ai::provider::tests::every_rejected_response_becomes_one_visible_failure_with_its_reason_kept ... ok
ai::provider::tests::locality_is_a_definite_answer_the_screen_can_show ... ok
ai::provider::tests::availability_separates_no_models_from_not_answering ... ok
ai::testing::tests::the_fake_passes_the_shared_contract_in_every_state ... ok
ai::testing::tests::the_contract_suite_is_not_written_against_the_fake ... ok
ai::testing::tests::the_fake_gives_the_same_answer_to_the_same_request ... ok
ai::testing::tests::every_mode_gets_a_note_of_that_mode ... ok
ai::testing::tests::a_response_that_breaks_the_schema_becomes_the_response_failure ... ok
ai::testing::tests::the_double_obeys_the_same_parsing_contract_as_a_real_adapter ... ok
ai::testing::tests::the_unreachable_fake_says_the_same_thing_twice ... ok
ai::testing::tests::the_fake_without_models_is_a_different_state_from_being_unreachable ... ok
ai::testing::tests::the_fake_repeats_its_last_answer_for_later_calls ... ok
ai::testing::tests::the_fake_records_what_it_was_asked_to_generate_and_nothing_more ... ok
ai::testing::tests::the_fake_can_stand_in_for_any_boundary_failure ... ok
ai::testing::tests::the_fake_can_speak_for_an_external_provider_too ... ok
ai::testing::tests::the_fake_never_claims_to_be_a_real_provider ... ok
ai::testing::tests::product_code_never_constructs_the_fake_provider ... ok
ai::testing::tests::the_request_the_fake_receives_can_only_carry_text ... ok
```

frontend 쪽 1:1 검사는 기존 `tests/ipc-boundary.test.ts`가 한다 — 그 테스트는
`src-tauri/src/domain/failure.rs`의 `as_str` 문자열을 소스에서 읽어 `src/ipc/failure.ts`의
union에 전부 있는지 확인하며, 이번 다섯 문자열을 포함한 채로 통과했다 (test Gate exit 0).
