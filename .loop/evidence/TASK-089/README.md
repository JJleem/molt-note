# TASK-089 Evidence

Phase 5.8(전사 청크 분할)의 불변을 **이미 만들어진 경로에 대한 전용 테스트 두 벌**에 고정했다.
**제품 소스는 하나도 바꾸지 않았다** — 추가된 것은 테스트 파일 둘뿐이다 (AC-6).

## 변경된 파일

```text
tests/transcription-chunking-boundary.test.ts      (신규 · 26 테스트 · 소스 수준 불변)
src-tauri/tests/transcription_chunking.rs          (신규 ·  7 테스트 · 행동 불변)
```

`git status --porcelain`은 `git-status.txt`에 있다. 이 Task가 더한 것은 위 둘과
`.loop/evidence/TASK-089/` 아래의 Evidence뿐이다.

## Gate 결과 (AC-1 · AC-2 · AC-3)

`node tools/loop-runtime/loopctl.mjs self-check build lint test` (2026-09-07)

```text
build: PASS  exit=0  1.1s     npm run build   (tsc && vite build)
lint:  PASS  exit=0  1.6s     npm run lint    (eslint . && cargo clippy --all-targets -- -D warnings)
test:  PASS  exit=0  6.9s     npm run test    (vitest run && cargo test)
```

전문은 `gates.txt`에 있다.

**새 테스트 두 벌이 그 안에서 실제로 돈다** (AC-1):

```text
vitest   Test Files  30 passed (30)        ← 이전 29 + 이 Task의 1
         Tests      612 passed (612)       ← 이전 586 + 이 Task의 26

cargo    Running tests/transcription_chunking.rs
         running 7 tests
         test ch1_chunk_local_times_rewind_which_is_exactly_what_the_offset_undoes ... ok
         test ch4_a_run_of_repeats_that_crosses_a_chunk_boundary_is_still_one_run ... ok
         test ch4_repeats_that_are_not_consecutive_survive_the_blocking ... ok
         test ch2_a_collapsed_result_is_blocked_even_when_it_arrived_in_chunks ... ok
         test ch2_blocking_first_would_have_hidden_this_collapse_so_the_product_judges_first ... ok
         test ch1_a_result_that_arrived_in_chunks_is_stored_as_one_transcript_that_never_rewinds ... ok
         test ch3_a_failed_chunked_run_leaves_the_audio_and_the_current_transcript_alone ... ok
         test result: ok. 7 passed; 0 failed; 0 ignored
```

원본 로그: `.loop-local/self-check/gates/{build,lint,test}/`.

## 모델도 오디오도 네트워크도 요구하지 않는다 · skip 경로가 없다 (AC-4)

| 무엇 | 어떻게 대신하는가 |
| --- | --- |
| 실제 whisper 추론 | `transcription::testing::StubEngine` — 계약이 같은 test double (PRODUCT-SPEC §18) |
| 모델 파일 | 임시 디렉터리의 16바이트짜리 자리표시자 (`ggml-base.bin`) |
| 실제 오디오 | 0.1초짜리 16 kHz mono PCM16 WAV를 테스트가 직접 만든다 |
| 청크 구간 · 오프셋 | `chunking::plan`에게 물어본다 — 값을 두 번째 자리에 적지 않는다 |
| 네트워크 | 없다. 두 파일 어디에도 HTTP·소켓·외부 프로세스 호출이 없다 |

- **`#[ignore]` · `return` 조기 종료 · 환경 변수 분기 · `it.skip` / `describe.skip`이 하나도
  없다.** 두 파일 전체에서 `skip`이라는 낱말이 등장하지 않는다.
- 모델 없음은 skip 조건이 아니라 §13의 정의된 실패이며(`transcription/model.rs`), 이 파일들은
  모델이 **있는** 자리표시자 상태로만 돈다.
- cargo 출력의 `0 ignored`가 그 사실의 제3자 관찰이다.

## 어떤 불변이 어디서 판정되는가 (AC-5)

### `tests/transcription-chunking-boundary.test.ts` — 소스 수준 (26)

| request가 열거한 불변 | `describe` 블록 | 테스트 수 |
| --- | --- | --- |
| 규칙이 순수 모듈 **한 파일에만** 있고 `whisper.rs` · `run.rs` · `src/screens/`에 복제되지 않았다 · 청크 길이와 반복 차단 임계값이 각각 한 자리 (AC-5 c) | `(a) 청킹 규칙이 한 자리에만 있다` | 6 |
| 그 순수 모듈이 fs · db · 네트워크 · `whisper_rs`를 알지 않는다 | `(b) 청킹 모듈이 바깥 세계를 모른다` | 4 |
| 센티초 → 밀리초 계수가 여전히 `parse.rs` 한 자리뿐이다 | `(c) 단위 변환의 자리가 늘지 않았다` | 3 |
| `run.rs`의 저장 직전 붕괴 판정 호출이 그대로 있다 (AC-5 b) | `(d) 저장 직전의 붕괴 판정이 그대로 있다` | 3 |
| `Cargo.toml`에 새 의존성이 늘지 않았다 (AC-5 d) | `(e) 새 의존성이 늘지 않았다` | 3 |
| 벤더 고유 청킹 개념이 `domain` · `payload.rs` · `types.ts`에 새로 생기지 않았다 · INV-9 (AC-5 d) | `(f) 새 벤더 고유 청킹 개념이 생기지 않았다` | 5 |

나머지 1개는 `검사가 실제로 원문을 읽었다` — 목록이 비어서 통과하는 일을 막는 자리다.

### `src-tauri/tests/transcription_chunking.rs` — 행동 (7)

| request가 열거한 불변 | 테스트 |
| --- | --- |
| 청크가 여럿인 결과가 하나로 합쳐져 `parse::normalize`를 통과하면 timestamp가 **뒤로 가지 않는다** (AC-5 a) | `ch1_chunk_local_times_rewind_which_is_exactly_what_the_offset_undoes` · `ch1_a_result_that_arrived_in_chunks_is_stored_as_one_transcript_that_never_rewinds` |
| 청킹이 들어와도 붕괴한 결과는 저장 직전에 막히고 `TranscriptionOutputUnusable`로 실패하며 Transcript가 늘지 않는다 (AC-5 b) | `ch2_a_collapsed_result_is_blocked_even_when_it_arrived_in_chunks` · `ch2_blocking_first_would_have_hidden_this_collapse_so_the_product_judges_first` |
| 실패해도 원본 오디오 경로와 기존 current Transcript가 그대로다 (INV-1 · INV-2 · INV-3) | `ch3_a_failed_chunked_run_leaves_the_audio_and_the_current_transcript_alone` |
| 반복 차단이 정상적인(비연속) 반복을 지우지 않는다 | `ch4_repeats_that_are_not_consecutive_survive_the_blocking` · `ch4_a_run_of_repeats_that_crosses_a_chunk_boundary_is_still_one_run` |

`ch1`의 첫 테스트는 **이 검사가 무엇을 막는지**를 값으로 먼저 보인다 — 오프셋 없이 이어 붙인
시각 열(`0,450,900,0,450,900,…`)이 실제로 되감긴다는 것을 단언한 뒤, 제품 경로를 지난 열이
되감기지 않는다는 것을 본다. `ch2`의 둘째 테스트는 ADR-0007 §20.6.2의 *"차단을 판정보다 먼저
두면 붕괴한 전사가 통과한다"*가 이 fixture에서 실제로 참임을 값으로 보이고(차단 전 130문장 ·
고유 1 → 붕괴 / 차단 후 3문장 → 붕괴 아님), 그럼에도 제품이 같은 값을 막는다는 것을 확인한다.

## 검사가 빈 단언이 아니라는 근거 — falsification

`falsification.md`에 전문. 제품 파일을 고치지 않기 위해 규칙을 위반하는 **임시 파일 둘**을 넣고
Gate를 돌린 뒤 지웠다. 세 검사가 실제로 실패했다.

## 이 Task가 판정하지 않는 것

- **고유 문장 비율 94% · 한국어로 읽히는가 · 소요 시간**은 `phase-prompt/05.8`의 Human Review
  항목이며 사람이 앱으로 72분 오디오를 전사해 판정한다. 자동 Gate는 그것을 하지 않는다.
- **실제 오디오 버퍼가 청크 구간으로 잘리는 자리**(`WhisperEngine::transcribe` 안)는 모델
  파일을 요구하므로 실행되지 않는다. 통합 테스트가 보는 것은 그 뒤 — 청크 결과가 합쳐진
  다음의 경로 전부다.

## 관찰 — 제품 경로에 대한 사실 하나 (이 Task의 범위 밖)

`chunking::block_consecutive_repeats`를 **부르는 제품 코드가 저장소에 없다**
(`src-tauri/src/transcription/mod.rs`의 재수출과 모듈 자신의 단위 테스트가 전부다. 2026-09-07
기준). ADR-0007 §20.6.2가 정한 자리는 `run.rs`의 붕괴 판정 **뒤 · 저장 앞**이다.

이 Task는 제품 소스를 바꾸지 않으므로 결선하지 않았고, 그것을 요구하는 검사도 쓰지 않았다 —
request가 열거한 불변에 그 항목이 없다. 통합 테스트가 고정한 것은 그 규칙이 **값에 대해 무엇을
하는가**이며, 그 사실은 `src-tauri/tests/transcription_chunking.rs`의 머리말에도 적혀 있다.
운영자와 Verifier가 판단할 몫으로 여기 남긴다.
