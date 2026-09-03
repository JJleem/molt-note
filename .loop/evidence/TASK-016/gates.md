# TASK-016 — Gate 실행 결과

실행: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
날짜: 2026-09-02
원본 로그: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(advisory — Runtime이 Worker 종료 후 독립적으로 다시 실행한다)

| Gate | command | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | 0 | PASS |
| lint | `npm run lint` (`eslint . && cargo clippy --all-targets -- -D warnings`) | 0 | PASS |
| test | `npm run test` (`vitest run && cargo test`) | 0 | PASS |

## test Gate 집계

```text
vitest:  Test Files  12 passed (12)
              Tests  101 passed (101)

cargo test (lib unit)            90 passed; 0 failed
  tests/capture.rs                9 passed; 0 failed
  tests/command_boundary.rs      12 passed; 0 failed
  tests/domain_invariants.rs      5 passed; 0 failed
  tests/domain_model.rs          16 passed; 0 failed
  tests/input_devices.rs          6 passed; 0 failed
  tests/recording_lifecycle.rs    5 passed; 0 failed   ← 이 Task가 추가
  tests/recording_repository.rs  13 passed; 0 failed
  tests/recording_session.rs      4 passed; 0 failed   ← 이 Task가 추가
  tests/settings_repository.rs    7 passed; 0 failed
```

## AC1을 판정하는 테스트

`src-tauri/tests/recording_lifecycle.rs` — 가짜 `SampleSource`와 값을 넣는 `Clock`으로
`Recorder`의 실제 경로를 그대로 지난다. **마이크도, 마이크 권한도, 흐르는 시간도 필요하지 않다.**

| 테스트 | 확인하는 것 |
| --- | --- |
| `the_whole_path_runs_start_pause_resume_stop_and_finishes_one_file` | start → pause → resume → stop 전체 경로. ① 파일이 **하나**로 확정된다 ② 일시정지 중 들어온 샘플(`2_000`)이 파일 **내용**에 없다 ③ 장치는 한 번만 열린다(pause가 닫지 않는다) ④ 길이 5,000ms — 벽시계로는 1시간이 지났다 ⑤ 보고된 byte 크기가 디스크에서 읽은 값과 같다 |
| `the_status_answer_carries_the_state_the_elapsed_ms_and_a_label_made_by_rust` | 상태 조회가 상태 · 경과 ms · **Rust가 만든 문자열**을 함께 돌려준다 |
| `a_recording_stopped_while_paused_keeps_exactly_what_was_recorded` | 멈춰 둔 채 정지해도 녹음된 것만 파일로 확정된다 |
| `a_request_that_does_not_fit_the_current_state_is_refused_without_touching_the_recording` | 잘못된 요청은 panic이 아니라 `Failure` 값이고 진행 중인 녹음을 건드리지 않는다 |
| `the_next_recording_starts_from_a_new_session_and_a_new_file` | 정지한 session은 남지 않고 다음 녹음은 새 파일로 간다 |

일시정지 구간 검증은 **크기가 아니라 내용을 본다** — `hound::WavReader`로 확정된 파일의
샘플을 전부 읽어 비교한다 (`samples_in`).

## 이번 Attempt에서 고친 것

Attempt 1이 남긴 상태에서 두 Gate가 실패했고, 둘 다 고쳤다.

1. `build` — `src/screens/captureSpikeView.test.ts`의 `CaptureReport` fixture에
   새 필드 `durationMs` · `durationLabel`이 없어 TS2739. fixture에 추가.
2. `lint` — `src-tauri/tests/recording_lifecycle.rs:311`의 `.err().expect(...)`가
   `clippy::err_expect`에 걸렸다. `.expect_err(...)`로 교체.
