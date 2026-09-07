# TASK-081 self-check (advisory — Runtime이 Gate를 독립적으로 다시 돌린다)

명령: `node tools/loop-runtime/loopctl.mjs self-check <gate>` (Runtime 소유 진입점)
실행 일자: 2026-09-07

| Gate | 명령 | exit | 시간 | 결과 |
| --- | --- | --- | --- | --- |
| build | `npm run build` (tsc && vite build) | 0 | 1.2s | PASS |
| lint | `npm run lint` (eslint . && cargo clippy --all-targets -- -D warnings) | 0 | 6.4s | PASS |
| test | `npm run test` (vitest run && cargo test) | 0 | 46.3s | PASS |

원본 출력: `.loop-local/self-check/gates/test/stdout.log` (Runtime 소유 · 이 Task가 쓰지 않았다).
그 로그에서 이 Task가 더한 검사만 추린 것이 `test-new-cases.log`다.

## AC-3이 요구한 세 가지를 어떤 검사가 판정하는가

| AC-3의 요구 | 검사 | 파일 |
| --- | --- | --- |
| 낮은 레벨 / 정상 레벨 구분 | `the_status_answer_tells_a_low_input_level_apart_from_a_usable_one` | `src-tauri/tests/recording_lifecycle.rs` |
| 일시정지 중 미갱신 | `the_input_level_does_not_move_while_the_recording_is_paused` | 〃 |
| 녹음 전 값 없음 | `there_is_no_input_level_before_a_recording_and_none_again_after_it` | 〃 |
| (INV-6) payload에 오디오가 없다 | `the_status_that_reaches_the_screen_carries_numbers_and_a_sentence_but_no_audio` | 〃 |
| 재는 자리가 파일에 쓰는 자리와 같다 | `the_level_sees_exactly_the_samples_that_reached_the_file` | `src-tauri/src/audio/capture.rs` |
| 쓰인 것이 없으면 값 없음 | `a_capture_that_wrote_nothing_has_no_level_rather_than_zero` | 〃 |

네 통합 검사는 **마이크도 마이크 권한도 흐르는 시간도 쓰지 않는다** — 이미 있던 가짜
`SampleSource`(`ControlledMicrophone`)와 `TestClock` 자리에만 값을 넣는다 (PRODUCT-SPEC §18).
