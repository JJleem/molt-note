# Gate 실행 결과 (self-check · advisory)

Run: RUN-20260902T081554Z-TASK-015 · Task: TASK-015 · 2026-09-02

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
(Runtime 소유 진입점. Worker가 임의로 만든 명령이 아니다. 판정은 Runtime이 다시 돌린다.)

```text
build: PASS  exit=0
lint:  PASS  exit=0
test:  PASS  exit=0
Self-check: all gates passed
```

원본 로그는 `.loop-local/self-check/gates/<gate>/{stdout,stderr}.log`에 있다
(self-check을 다시 돌리면 덮어써지므로 아래에 핵심 부분을 옮겨 적었다).

## AC3 — build (`npm run build` = `tsc && vite build`)

```text
> tsc && vite build
vite v7.3.6 building client environment for production...
✓ 44 modules transformed.
dist/index.html                   0.40 kB │ gzip:  0.27 kB
dist/assets/index-mYziqwUP.css    3.79 kB │ gzip:  1.22 kB
dist/assets/index-CjAzNTZn.js   206.85 kB │ gzip: 64.66 kB
✓ built in 262ms
```

## AC2 — lint (`npm run lint` = `eslint .` + clippy)

```text
> molt-note@0.1.0 lint:web
> eslint .

> molt-note@0.1.0 lint:rust
> cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
    Checking molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.07s
```

`-D warnings`가 붙은 채로 새 모듈(`src/audio/session.rs`)과 새 통합 테스트
(`tests/recording_session.rs` — `--all-targets`에 포함된다)를 검사했고 exit 0이다.

## AC1 — test (`npm run test` = `vitest run` + `cargo test`)

```text
vitest:  Test Files  11 passed (11)
              Tests  96 passed (96)

cargo test:
  unittests src/lib.rs          84 passed; 0 failed   (이 Task가 13개를 더했다)
  tests/capture.rs               9 passed; 0 failed
  tests/command_boundary.rs     12 passed; 0 failed
  tests/domain_invariants.rs     5 passed; 0 failed
  tests/domain_model.rs         16 passed; 0 failed
  tests/input_devices.rs         6 passed; 0 failed
  tests/recording_repository.rs 13 passed; 0 failed
  tests/recording_session.rs     4 passed; 0 failed   (이 Task가 추가한 파일)
  tests/settings_repository.rs   7 passed; 0 failed
```

### 이 Task가 추가한 테스트 (전부 마이크 없이 · 시간을 흘려보내지 않고 실행된다)

`src/audio/session.rs`의 단위 테스트 13개:

```text
the_whole_lifecycle_walks_idle_recording_paused_recording_stopped
every_cell_of_the_transition_table_is_covered_and_only_five_are_allowed
a_rejected_transition_is_a_failure_value_the_user_can_read
each_wrong_transition_names_what_went_wrong
the_paused_span_is_not_counted_in_the_duration
many_pause_and_resume_cycles_keep_accumulating_only_the_recorded_spans
the_duration_grows_with_the_time_it_is_given_while_recording
a_stopped_session_reports_the_same_length_no_matter_what_time_is_given
an_idle_session_has_no_length_yet
the_human_readable_length_comes_from_the_one_place_that_rule_lives
a_time_that_goes_backwards_never_shortens_the_recording
extreme_time_values_do_not_panic
every_state_has_a_distinct_stable_string
```

`tests/recording_session.rs`의 통합 테스트 4개:

```text
the_state_machine_module_reaches_no_hardware_no_clock_and_no_filesystem
a_full_recording_walks_idle_recording_paused_recording_stopped
the_paused_span_stays_out_of_the_duration_however_long_it_lasts
a_wrong_transition_comes_back_as_a_failure_the_screen_can_show
```
