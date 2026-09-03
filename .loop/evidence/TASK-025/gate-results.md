# TASK-025 — Gate 실행 결과 (Worker self-check · 참고용)

```text
Date:     2026-09-03
Task:     TASK-025 — whisper 원시 출력 → 도메인 값 정규화 모듈
Command:  node tools/loop-runtime/loopctl.mjs self-check build lint test
```

> 이 결과는 참고용이다. 완료 판정은 Runtime과 Verifier가 Worker 종료 후 Gate를 독립적으로
> 다시 돌려서 내린다 (KERNEL §3 · §5).

## 최종 실행 (정정 후)

| Gate | command | exit | 시간 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | **0 · PASS** | 0.9s |
| lint | `npm run lint` (`eslint . && cargo clippy --all-targets -- -D warnings`) | **0 · PASS** | 2.1s |
| test | `npm run test` (`vitest run && cargo test`) | **0 · PASS** | 9.8s |

```text
Self-check: all gates passed
```

- clippy는 `-D warnings`로 돌았고 crate를 실제로 다시 검사했다
  (`Checking molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)`).
- cargo test lib target: `test result: ok. 146 passed; 0 failed; 0 ignored`.
  (이 Task 전 lib target은 125 tests였다 — 아래 21개가 늘어난 몫이다.)

## 이 Task가 추가한 테스트 21개 (전부 통과)

`transcription::parse::tests::`

```text
one_minute_thirty_seconds_is_ninety_thousand_milliseconds
known_timestamps_map_to_literal_milliseconds
an_hour_long_recording_does_not_drift_at_the_end
a_normal_output_becomes_the_domain_value_without_complaint
raw_text_is_built_from_the_surviving_segments_by_one_rule
the_same_input_always_produces_the_same_output
inner_whitespace_survives_while_the_edges_are_trimmed
an_empty_output_is_an_empty_transcription_rather_than_a_panic
an_output_whose_segments_are_all_blank_yields_no_text_and_says_why
a_segment_whose_end_precedes_its_start_keeps_its_text_and_collapses_to_a_point
a_truncated_last_segment_does_not_take_the_rest_of_the_transcript_with_it
a_negative_timestamp_is_folded_to_the_start_of_the_recording
out_of_order_segments_keep_the_engines_order_and_are_reported
overlapping_segments_are_kept_intact_and_reported
a_dropped_segment_does_not_become_the_yardstick_for_the_next_one
a_timestamp_too_large_to_convert_is_a_defined_failure_rather_than_a_wrong_number
the_language_the_engine_reported_is_passed_through_untouched
a_missing_or_blank_language_becomes_none_rather_than_an_empty_string
no_hostile_output_makes_this_module_panic
this_module_never_runs_a_process_or_calls_the_whisper_library
the_conversion_factor_lives_in_exactly_one_place
```

**이 테스트들은 whisper 바이너리도 모델도 없이 돌았다.** 모델 파일은 이 기기의 어디에도
배치되지 않았고 `Cargo.toml`에 whisper 의존성이 아직 없다 (TASK-026의 몫이다).

## 변경한 파일

```text
src-tauri/src/transcription/parse.rs   신규 — 정규화 모듈 + 21개 테스트
src-tauri/src/transcription/mod.rs     parse 모듈 선언 · 재수출 · 모듈 지도 갱신
```

Task 범위 밖의 파일은 건드리지 않았다. `.loop/` 아래는 이 Evidence 디렉터리에만 썼다.
