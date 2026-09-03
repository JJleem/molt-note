# TASK-028 — Gate 실행 결과

실행 방법: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
(Worker의 self-check이며 완료 판정이 아니다. Runtime이 독립적으로 다시 실행한다.)

실행 시각: 2026-09-03 · 원시 로그: `.loop-local/self-check/gates/<gate>/{stdout,stderr}.log`
(그 디렉터리는 Runtime 소유이며 실행할 때마다 덮어써진다. 아래는 마지막 실행의 요약이다.)

| Gate | command | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | 0 | PASS |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | 0 | PASS |
| test | `npm run test` (`vitest run` + `cargo test`) | 0 | PASS |

`build`가 실제로 산출물을 냈다 (`.loop-local/self-check/gates/build/stdout.log`):

```text
vite v7.3.6 building client environment for production...
✓ 46 modules transformed.
dist/assets/index-BueuPG8k.js   212.63 kB │ gzip: 66.17 kB
✓ built in 273ms
```

## test Gate 안에서 실제로 돈 것

```text
vitest    Test Files  14 passed (14)
          Tests      155 passed (155)
cargo     17개 테스트 바이너리 · `test result: ok` 17회 · FAILED 0회 · ignored 0
```

이 Task가 추가한 통합 테스트 바이너리
(`src-tauri/target/debug/deps/transcription_background-b994186d3a20b929`) —
**10개 전부 통과** (stdout.log 350~364행):

```text
running 10 tests
test an_empty_recording_id_is_refused_before_anything_starts ... ok
test a_failed_transcription_arrives_as_the_failure_the_engine_reported ... ok
test a_transcription_for_an_unknown_recording_fails_without_touching_anything ... ok
test starting_the_same_recording_twice_is_refused_instead_of_being_ignored ... ok
test running_and_finished_are_two_different_answers ... ok
test a_transcription_without_a_chosen_model_reports_the_model_missing_failure ... ok
test a_status_query_answers_while_the_engine_is_still_transcribing ... ok
test other_commands_answer_while_a_transcription_is_running ... ok
test a_finished_transcription_can_be_started_again ... ok
test a_second_recording_does_not_queue_up_behind_the_running_one ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
```

`skip`도 `ignored`도 없다. 실제 whisper 바이너리도 모델 파일도 쓰지 않는다 — 엔진 자리에는
계약이 같은 test double이 서고, "모델"은 임시 디렉터리의 몇 바이트짜리 자리표시자다
(§18 · `phase-prompt/03` 요구 8 · 9).

각 AC가 어느 테스트로 판정되는지는 `acceptance-map.md`에 있다.
