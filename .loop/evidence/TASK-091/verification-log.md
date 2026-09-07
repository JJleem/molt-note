# TASK-091 — 문서에 적은 것을 저장소에서 어떻게 대조했는가

```text
Task:    TASK-091 (문서 전용 — 소스 · 설정 · 의존성 · 테스트를 바꾸지 않는다)
Phase:   5.8 — Transcription Chunking
바꾼 것:  docs/ADR-0007-transcription-engine.md   (§21 추가 + 머리말/읽기 안내에 덧붙임)
         docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md (부록 3 추가)
바꾸지 않은 것:
         src-tauri/** · Cargo.toml · Cargo.lock · package.json · tests/** · .loop/** (evidence 제외)
```

이 Run에서 허용된 명령은 `node tools/loop-runtime/loopctl.mjs self-check` 하나였다.
그러므로 아래 대조는 전부 **파일을 직접 읽어서** 했다 (`grep` · `git diff` 사용 불가).

## 1. 읽은 파일 (전부 [E1] — 이 Run이 직접 열었다)

| 파일 | 무엇을 확인했는가 |
| --- | --- |
| `src-tauri/src/transcription/chunking.rs` | 상수 셋(`CHUNK_SECONDS = 120` · `CHUNK_OVERLAP_FRAMES = 0` · `MAX_CONSECUTIVE_REPEATS = 3`) · `plan` · `shift` · `merge` · `block_consecutive_repeats` · `offset_centiseconds`(checked_mul) · `add_offset`(checked_add) · 단위 테스트 목록 |
| `src-tauri/src/transcription/whisper.rs` | `chunking::plan(input.frames(), input.sample_rate_hz)` · 청크 루프 안의 `context.create_state()` · `state.full(params, &input.samples[chunk.range()])` · `ensure_usable(chunking::merge(transcribed)?)` · `chunk k/n` detail · 모듈 문서가 적은 whisper-rs 0.16 시그니처 |
| `src-tauri/src/transcription/run.rs` | `collapse::assess` 호출이 저장 앞에 있다 · **`chunking`을 `use`하지 않는다** · `transcription.segments`를 그대로 `Transcript`에 담는다 |
| `src-tauri/src/transcription/engine.rs` | `TranscriptionEngine` trait 시그니처가 그대로다 · **trait 주석에 청킹 사실이 적혀 있지 않다** |
| `src-tauri/src/transcription/mod.rs` | `chunking`의 재수출 목록 (`block_consecutive_repeats` 포함) |
| `src-tauri/src/transcription/audio_input.rs` | `TranscriptionInput::sample_rate_hz` · `frames()` · `TARGET_SAMPLE_RATE_HZ` |
| `src-tauri/tests/transcription_chunking.rs` | CH-1 ~ CH-4가 무엇을 판정하는가 · 그 파일이 스스로 적은 한계 |
| `tests/transcription-chunking-boundary.test.ts` | (a)~(f) 여섯 가지 원문 검사 |
| `package.json` · `.loop/project.yaml` | Gate 명령의 실체 (`cargo clippy --all-targets` · `cargo test`가 실제로 들어 있다) |
| `phase-prompt/05.8-transcription-chunking.md` | 성공 기준 1 · P-2의 기준선(94.0% · 6.0분) · P-4의 두 파일 · Human Review 항목 |
| `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` | 2026-09-05 부록(결과 1 · 2) · [정정 2026-09-07 · TASK-090] · §부록2-3의 측정 방법 · §부록2-4의 `[미측정]` 선례 · §4 · §6.2 · §7.1의 절차 |

## 2. 문서에 적은 주장과 그 근거

| ADR-0007 §21의 주장 | 저장소에서 확인한 근거 |
| --- | --- |
| ①②(청크 분할 · 오프셋 · 병합)가 `chunking.rs`에 있고 `whisper.rs`가 부른다 | 두 파일의 실제 호출문 (위 표) |
| state는 청크마다 재생성 · 모델 context는 재사용 | `WhisperContext::new_with_params`가 루프 밖, `create_state()`가 루프 안 |
| 겹침 0이 산술에 실제로 들어간다 | `start_frame += frame_count - CHUNK_OVERLAP_FRAMES` |
| 단위 변환이 늘지 않았다 | `chunking.rs`에 밀리초가 없다 · 변환은 `parse.rs`의 `MILLISECONDS_PER_CENTISECOND` 한 자리 · `tests/transcription-chunking-boundary.test.ts` (c) |
| **③ 연속 반복 차단을 부르는 제품 코드가 없다** | ⑴ `run.rs`가 `chunking`을 `use`하지 않고 저장 직전에 `transcription.segments`를 그대로 담는다. ⑵ `whisper.rs`의 모듈 문서가 *"연속 반복 차단은 … 이 파일이 아니라 저장 직전의 자리에서 돈다"* 고 적는다(즉 엔진 안에서도 부르지 않는다). ⑶ `src-tauri/tests/transcription_chunking.rs`의 모듈 문서가 같은 사실을 명시한다 — *"2026-09-07 기준으로 저장소에서 `chunking::block_consecutive_repeats`를 부르는 제품 코드는 아직 없다(모듈과 재수출뿐이다)"* |
| `engine.rs`의 trait 주석이 §20.5의 요구대로 손질되지 않았다 | 그 주석에 청크 · 오프셋이라는 말이 없다. 대신 `whisper.rs` 모듈 문서가 그 사실을 적는다 |
| whisper-rs 0.16의 API가 §20의 서술과 다르지 않았다 | `whisper.rs`가 적은 시그니처 목록 + `lint`/`test` Gate가 그 호출을 실제로 컴파일한다(self-check exit 0) |
| 자동 검증이 판정하지 못하는 것 | 세 테스트 파일이 스스로 적은 한계 + Gate에 모델도 72분 오디오도 없다는 사실 |

## 3. 이 Task가 하지 않은 것

- **코드를 고치지 않았다.** ③이 연결되지 않았다는 사실은 **기록만** 했다 —
  호출 한 줄을 넣는 것은 제품 소스 변경이며 이 Task의 범위(문서 전용) 밖이다.
- **결과 칸을 추정으로 채우지 않았다.** 부록 3의 모든 결과 칸은 `[미측정]` · `[미기입]`이다
  (§부록2-4가 세운 선례 그대로).
- **기존 문서를 지우거나 다시 쓰지 않았다.** 두 문서 모두 **덧붙이는 형태**이며,
  TASK-090의 정정 블록 넷도 그대로 있다.
