# TASK-012 — Gate 실행 결과 (self-check, advisory)

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
실행 시각: 2026-09-02 (worker attempt 2)

| Gate  | command         | exit | 결과 |
|-------|-----------------|------|------|
| build | `npm run build` | 0    | PASS |
| lint  | `npm run lint`  | 0    | PASS |
| test  | `npm run test`  | 0    | PASS |

원본 출력: `.loop-local/self-check/gates/{lint,test}/{stdout,stderr}.log`
(self-check가 덮어쓰므로 Runtime이 Gate를 다시 돌린 결과가 판정 기준이다.)

## test 게이트 내역

frontend (`vitest run`):

```
Test Files  10 passed (10)
     Tests  70 passed (70)
```

backend (`cargo test --manifest-path src-tauri/Cargo.toml`) — 전 바이너리 ok, 실패 0:

```
running 71 tests   test result: ok. 71 passed; 0 failed   (lib unit tests)
running  9 tests   test result: ok.  9 passed; 0 failed   (tests/capture.rs)
running 12 tests   test result: ok. 12 passed; 0 failed
running  5 tests   test result: ok.  5 passed; 0 failed
running 16 tests   test result: ok. 16 passed; 0 failed
running  6 tests   test result: ok.  6 passed; 0 failed
running 13 tests   test result: ok. 13 passed; 0 failed
running  7 tests   test result: ok.  7 passed; 0 failed
```

## Attempt 1이 남긴 상태와 이번 attempt가 고친 것

attempt 1은 timeout으로 죽으면서 **컴파일되지 않는 코드**를 남겼다.
`npm run lint` / `npm run test`가 22개 오류로 실패했고, 이번 attempt는 그것을 고쳤다.

1. `cpal` 0.18 API 불일치 (`src-tauri/src/audio/system_capture.rs`)
   - `config.sample_rate()`는 `u32`다 — `.0`이 없다 (E0610).
   - `build_input_stream`은 `StreamConfig`를 **값으로** 받는다 — `&`가 아니다 (E0308 ×2).
   - `cpal::StreamError`라는 경로가 없다 (E0425). 오류 콜백의 인자 타입을 이름으로 적지 않고
     `impl Display`를 받는 `note_stream_error`로 바꿨다 — 타입 이름은 `cpal`이 정한다.
   - `SupportedStreamConfig`는 `Copy`다 — `.clone()`은 clippy `-D warnings`에서 오류다.
2. non-ASCII byte string literal (`b"녹음"`)은 Rust에서 오류다 (18건).
   `src-tauri/src/audio/capture.rs` · `src-tauri/src/platform/app_data_dir.rs`의 테스트에서
   `b"..."` → `"..."` / `fs::read` → `fs::read_to_string`으로 바꿨다.

테스트를 지우거나 skip해서 통과시킨 것은 없다. 테스트 수는 attempt 1이 의도한 그대로다.
