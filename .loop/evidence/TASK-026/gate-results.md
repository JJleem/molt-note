# TASK-026 — Gate 실행 결과 (Worker self-check · 참고용)

```text
Date:     2026-09-03
Task:     TASK-026 — 실제 whisper를 실행하는 경계 (trait · whisper-rs 구현 · test double)
Command:  node tools/loop-runtime/loopctl.mjs self-check build lint test
```

> 이 결과는 참고용이다. 완료 판정은 Runtime과 Verifier가 Worker 종료 후 Gate를 독립적으로
> 다시 돌려서 내린다 (KERNEL §3 · §5).

## 최종 실행

| Gate | command | exit | 시간 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | **0 · PASS** | 0.9s |
| lint | `npm run lint` (`eslint . && cargo clippy --all-targets -- -D warnings`) | **0 · PASS** | 2.5s |
| test | `npm run test` (`vitest run && cargo test`) | **0 · PASS** | 10.6s |

```text
Self-check: all gates passed
```

> 위는 **최종 소스**에 대한 실행이다. 그 직전, 같은 소스에서 문서 주석만 다른 상태로 돌린
> 전체 실행도 세 Gate 전부 PASS였다 (build 0.9s · lint 1.0s · test 33.3s — test 시간 차이는
> 새 통합 테스트 target을 처음 링크한 몫이다). 아래 테스트 수치는 그 실행의 출력이며,
> 최종 실행에서도 동일하게 통과했다.

- vitest: `Test Files 14 passed (14)` · `Tests 154 passed (154)`
- cargo test lib target: `test result: ok. 169 passed; 0 failed; 0 ignored`
  (이 Task 전 lib target은 146 tests였다 — 아래 23개가 늘어난 몫이다.)
- 새 통합 테스트 target `tests/transcription_engine.rs`:
  `test result: ok. 10 passed; 0 failed; 0 ignored`
- 기존 통합 테스트 target 전부 그대로 통과했다 (실패 0).

## 중간 실행 — 무엇을 고쳤는지 감추지 않는다

| # | Gate | 결과 | 원인 |
| --- | --- | --- | --- |
| 1 | build | PASS 0.9s | — |
| 2 | lint | **FAIL exit=101 · 27.7s** | `whisper-rs` 0.16의 segment API가 이 Task가 처음 쓴 이름과 달랐다 (E0599 ×4). **의존성 해석과 `whisper-rs-sys` 빌드 자체는 이 실행에서 성공했다** |
| 3 | lint | FAIL exit=101 · 2.2s | 실제 시그니처를 컴파일러에게 물어보기 위해 일부러 넣은 probe (`let _: () = …`). 그 출력이 `whisper-rs-api-verification.md`의 근거다 |
| 4 | lint | PASS 2.2s | probe 제거 · 확인된 실제 API로 구현 |
| 5 | build · lint · test | **전부 PASS** | 최종 |

**테스트를 지우거나 skip해서 통과시킨 것은 없다.** 2·3번 실패는 전부 제품 코드의 컴파일
오류였고, 고친 것도 제품 코드다.

## 이 Task가 추가한 테스트 33개 (전부 통과)

`transcription::engine::tests::` (6)

```text
output_with_text_passes_through_untouched
an_engine_that_produced_nothing_is_a_distinct_product_failure
segments_without_usable_text_are_the_same_as_no_output
one_usable_segment_is_enough
the_four_transcription_failures_never_collapse_into_one
only_the_engine_failure_says_retrying_is_worth_it
```

`transcription::model::tests::` (8)

```text
a_file_name_is_resolved_inside_the_models_directory
an_absolute_path_is_used_as_the_user_gave_it
surrounding_whitespace_in_the_setting_does_not_hide_the_model
no_model_configured_is_a_product_state_not_a_silent_skip
a_model_that_is_not_there_is_missing_rather_than_unusable
a_directory_in_the_place_of_a_model_is_unusable_rather_than_missing
an_empty_file_is_unusable_rather_than_a_model
resolving_a_model_does_not_create_or_change_anything
```

`transcription::testing::tests::` (5)

```text
the_double_records_what_it_was_asked_to_transcribe
the_double_repeats_its_last_answer_for_later_calls
the_double_can_stand_in_for_any_product_failure
the_double_obeys_the_same_output_contract_as_the_real_engine
the_double_never_claims_to_be_the_real_engine
```

`transcription::whisper::tests::` (2 — **엔진을 실행하지 않는다**)

```text
the_engine_identifies_itself_for_transcript_provenance
the_thread_count_is_always_usable
```

`platform::app_data_dir::tests::` (2)

```text
the_models_directory_is_derived_from_the_same_root_and_is_not_the_recordings_directory
ensuring_the_models_directory_keeps_the_model_that_is_already_there
```

`tests/transcription_engine.rs` (10 · 통합)

```text
a_normal_run_returns_the_engine_output_untouched
a_missing_model_is_a_defined_failure_rather_than_a_skipped_test
an_unusable_model_is_told_apart_from_a_missing_one
an_engine_that_fails_is_reported_as_a_retryable_product_failure
output_that_cannot_be_read_as_a_transcript_is_its_own_failure
the_four_transcription_failures_reach_the_screen_as_four_different_kinds
the_transcription_boundary_has_no_way_to_send_audio_anywhere
the_engine_runs_in_this_process_and_spawns_nothing
no_platform_branch_was_pre_paid_for_windows
model_files_and_large_binaries_stay_out_of_the_repository
```

**이 테스트들은 whisper 모델 없이 돌았다.** 이 기기에는 whisper 모델 파일이 배치되어 있지
않고(`~/Library/Application Support/…/models/`를 만들지도 않았다), 테스트가 쓰는 "모델"은
임시 디렉터리에 만든 16바이트짜리 자리표시자다. 실제 엔진(`transcription::whisper`)은
**컴파일되지만 어떤 테스트도 그것을 호출하지 않는다.**

## 변경한 파일

```text
src-tauri/Cargo.toml                          whisper-rs = "0.16" 추가 (ADR-0007 §2)
src-tauri/Cargo.lock                          cargo가 해석한 값 (whisper-rs 0.16.0 · sys 0.15.0)
src-tauri/src/transcription/engine.rs         신규 — 실행 계약(trait) · 네 실패 · 출력 계약
src-tauri/src/transcription/model.rs          신규 — 모델 파일을 해석하는 단 한 곳
src-tauri/src/transcription/whisper.rs        신규 — whisper-rs 실제 구현
src-tauri/src/transcription/testing.rs        신규 — test double (StubEngine)
src-tauri/src/transcription/mod.rs            새 모듈 선언 · 재수출 · 모듈 지도 갱신
src-tauri/src/domain/failure.rs               전사 실패 4종 추가 (§13이 예고한 자리)
src-tauri/src/platform/app_data_dir.rs        models_dir · ensure_models_dir (INV-10)
src-tauri/tests/transcription_engine.rs       신규 — 통합 테스트 10개
src/ipc/failure.ts                            FailureKind union에 전사 실패 4종 (Rust와 1:1)
```

`.gitignore` · `tauri.conf.json` · `capabilities/default.json`은 **바꾸지 않았다** —
이유는 `ac-map.md`의 AC6 · AC7에 적었다. Task 범위 밖의 파일도 건드리지 않았고,
`.loop/` 아래는 이 Evidence 디렉터리에만 썼다.
