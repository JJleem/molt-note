# TASK-088 self-check 결과 (advisory — Runtime이 Gate를 독립적으로 다시 돌린다)

실행: `node tools/loop-runtime/loopctl.mjs self-check <gate>` (2026-09-07)

| Gate | 명령 | exit | 소요 | 결과 |
| --- | --- | --- | --- | --- |
| build | `npm run build` (tsc && vite build) | 0 | 1.3s | PASS |
| lint | `npm run lint` (eslint . && cargo clippy --all-targets -- -D warnings) | 0 | 7.2s | PASS |
| test | `npm run test` (vitest run && cargo test) | 0 | 48.8s | PASS |

lint의 clippy가 실제로 이 crate를 다시 검사했다 (자체 로그: `Checking molt-note v0.1.0 … Finished dev profile in 5.32s`).
`whisper-rs` 0.16에서 **하나의 context로 청크마다 `create_state()` → `full()`** 을 부르는 코드가
`-D warnings` 아래에서 컴파일된다는 것을 그 실행이 확인한다 (P3-AC1 · P3-AC2).

cargo test: 전 테스트 바이너리에서 `0 failed`. 전사 관련 통합 테스트
(`transcription_engine` · `transcription_run` · `transcription_language` · `transcription_background` ·
`transcription_and_reach_invariants` · `automatic_transcription` · `core_pipeline_without_ai`)가 모두 통과했다 (P3-AC3).
전체 출력은 `test-stdout.log` · `test-stderr.log`에 있다.

## 이 Task가 바꾼 것

`src-tauri/src/transcription/whisper.rs` 한 파일 (`whisper.rs.diff` · `diff-stat.txt`).

- `WhisperEngine::transcribe` **안쪽**만 바뀌었다. `TranscriptionEngine` trait의 시그니처도
  `RawTranscription` · `RawSegment`의 모양도 그대로이고, `testing::StubEngine`은 손대지 않았다.
- 모델 context는 지금처럼 전사 한 건에 한 번 연다. `chunking::plan`이 낸 청크마다
  **새 state**를 만들고 그 청크의 샘플 슬라이스만 `full()`에 넘긴다.
- `FullParams`는 청크마다 새로 만들되 내용은 그대로다 — 스레드 수 · `set_translate(false)` ·
  언어 선택의 두 갈래(`set_language(Some(code))`+`set_detect_language(false)` /
  `set_language(None)`+`set_detect_language(true)`) · 출력 억제 넷이 **모든 청크에** 적용된다.
  디코딩 파라미터를 새로 더하거나 바꾸지 않았다.
- 청크 분할 규칙 · 오프셋 덧셈 · 청크 결과 잇기 · 합쳐진 `language` 결정은 전부
  `chunking::plan` / `chunking::merge` 호출이며 그 규칙을 이 파일에 복제하지 않았다.
  센티초 → 밀리초 변환은 여전히 `parse.rs`에만 있다.
- 실패는 기존 네 종류 안에서만 난다: state 생성 실패 · `full()` 실패 → `TranscriptionEngineFailed`,
  segment를 읽지 못함 → `TranscriptionOutputUnusable`, 시각 넘침 → `chunking`이 낸 실패를 그대로
  넘긴다. **새 `FailureKind`를 만들지 않았다.** 어느 청크였는지는 detail에 `chunk k/n`으로 남는다.
- `run.rs`의 저장 직전 붕괴 판정 · `parse.rs`의 단위 변환 · `collapse.rs`의 임계값은 건드리지 않았다.
