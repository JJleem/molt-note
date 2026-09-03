# TASK-027 self-check (advisory — Runtime이 Gate를 독립적으로 다시 돌린다)

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
실행 시각: 2026-09-03 (worker run RUN-20260903T033017Z-TASK-027)

| Gate | command | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | 0 | PASS (0.9s) |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | 0 | PASS (2.5s) |
| test | `npm run test` (`vitest run` + `cargo test`) | 0 | PASS (14.1s) |

원본 로그(재실행 가능): `.loop-local/self-check/gates/<gate>/{stdout,stderr}.log`

## build (stdout 발췌)

```text
> molt-note@0.1.0 build
> tsc && vite build

vite v7.3.6 building client environment for production...
✓ 46 modules transformed.
dist/index.html                   0.40 kB │ gzip:  0.26 kB
dist/assets/index-BYKmgwJd.css    4.18 kB │ gzip:  1.30 kB
dist/assets/index-BueuPG8k.js   212.63 kB │ gzip: 66.17 kB
✓ built in 281ms
```

## lint (stdout 발췌)

```text
> npm run lint:web && npm run lint:rust

> molt-note@0.1.0 lint:web
> eslint .

> molt-note@0.1.0 lint:rust
> cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

```text
    Checking molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.34s
```

경고 없음 — `-D warnings`이므로 경고가 하나라도 있으면 exit≠0이다.

## test — 이 Task가 추가한 `tests/transcription_run.rs`

```text
     Running tests/transcription_run.rs (src-tauri/target/debug/deps/transcription_run-59021384ec06d020)

running 13 tests
test a_failed_run_leaves_the_original_audio_file_and_the_recording_record_intact ... ok
test nothing_in_the_orchestration_can_remove_or_rewrite_the_source ... ok
test an_unknown_recording_changes_nothing_at_all ... ok
test a_failed_run_stores_pending_then_running_then_failed ... ok
test a_successful_run_appends_one_transcript_and_makes_it_current ... ok
test a_recording_whose_audio_file_is_gone_fails_without_touching_the_record ... ok
test a_missing_model_reaches_the_user_together_with_the_failed_status ... ok
test a_second_successful_run_adds_a_transcript_instead_of_updating_the_first ... ok
test a_failed_re_transcription_leaves_the_previous_transcript_as_current ... ok
test a_successful_run_stores_pending_then_running_then_done ... ok
test a_failed_run_can_be_retried_and_then_succeed ... ok
test the_stored_transcript_carries_every_field_section_7_requires ... ok
test the_timestamps_are_not_off_by_a_factor_of_ten_or_a_hundred ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

기존 test 대상(`tests/transcription_engine.rs` 10건 포함)도 전부 통과했고 실패·무시된 테스트는 없다.

**이 테스트는 실제 whisper 바이너리도 모델 파일도 요구하지 않는다** — 엔진 자리에는
`transcription::testing::StubEngine`이 서고, "모델"은 임시 디렉터리에 만든 16바이트짜리
자리표시자이며, 오디오는 테스트가 `hound`로 만든 0.1초 16 kHz mono PCM16 WAV다
(`phase-prompt/03` 요구 8 · 9).
