# TASK-087 이 Run이 바꾼 것

| 파일 | 무엇을 |
| --- | --- |
| `src-tauri/src/transcription/chunking.rs` | **새 파일.** ADR-0007 §20의 세 규칙을 담은 순수 모듈 + 단위 테스트 30개 |
| `src-tauri/src/transcription/mod.rs` | 모듈 등록(`pub mod chunking;`) · 재수출 · 파일 지도 주석 3줄 |

`git status --porcelain -- src-tauri/src/transcription/` (이 Run 종료 시점):

```
 M src-tauri/src/transcription/engine.rs      ← 이 Run이 건드리지 않았다 (Run 시작 전부터 dirty)
 M src-tauri/src/transcription/mod.rs         ← 이 Run이 등록·재수출을 더했다
 M src-tauri/src/transcription/run.rs         ← 이 Run이 건드리지 않았다
 M src-tauri/src/transcription/testing.rs     ← 이 Run이 건드리지 않았다
 M src-tauri/src/transcription/whisper.rs     ← 이 Run이 건드리지 않았다
?? src-tauri/src/transcription/chunking.rs    ← 이 Run이 만들었다
?? src-tauri/src/transcription/collapse.rs    ← 이 Run이 건드리지 않았다 (앞선 Task의 산출물)
```

**`Cargo.toml`을 이 Run은 열지 않았다** — 새 crate 의존성이 없다. 새 모듈이 쓰는 것은
`std`(`std::ops::Range` · `Vec` · `String`)와 저장소 안의 `crate::domain` ·
`super::{collapse, parse}`뿐이다.

**엔진 결선은 하지 않았다** (다음 Task의 몫). `whisper.rs` · `run.rs` · `engine.rs`는
이 모듈을 아직 부르지 않는다.

## Acceptance Criteria가 어디서 판정되는가

| AC | 어디서 |
| --- | --- |
| P2-AC1 build | `gates.md` — exit 0 |
| P2-AC2 lint | `gates.md` — exit 0 (`cargo clippy --all-targets -- -D warnings` 포함) |
| P2-AC3 test | `gates.md` + `test-chunking.txt` — 555 passed 안에 새 모듈 테스트 30개 |
| P2-AC4 순수성 | `chunking.rs`의 `use` 선언 3줄(`std::ops::Range` · `crate::domain` · `super::{collapse,parse}`)과 테스트 `this_module_does_not_know_the_outside_world` — 원문에서 fs · process · rusqlite · whisper 라이브러리 · 네트워크 · 시계 needle 16개가 없음을 확인한다. 실제 모델도 오디오 파일도 없이 값만으로 돈다 |
| P2-AC5 경계값 | 아래 표 |
| P2-AC6 상수 한 자리 | 테스트 `the_three_values_live_in_exactly_one_place` — 정의가 원문에 정확히 1회씩이고, `run.rs` · `engine.rs` · `whisper.rs` · `parse.rs` · `collapse.rs` 어디에도 그 이름과 파생 숫자(`1_920_000` · `12_000`)가 없음을 확인한다 |

## request가 열거한 경계값 → 그것을 고정하는 테스트

| 경계값 | 테스트 |
| --- | --- |
| 샘플이 하나도 없을 때 | `no_samples_means_no_chunks` (청크 0개) |
| 청크 길이보다 짧을 때 | `audio_shorter_than_one_chunk_is_a_single_chunk_of_its_own_length` (1개 · 길이는 받은 그대로) |
| 정확히 청크 길이의 배수일 때 | `exactly_one_chunk_length_is_one_chunk` (L → 1개) · `an_exact_multiple_of_the_chunk_length_has_no_short_last_chunk` (3L → 3개) |
| 배수 + 1 프레임 (마지막 짧은 청크) | `one_frame_past_a_multiple_makes_a_short_last_chunk` (2L+1 → 3개 · 마지막 1프레임) |
| 오프셋이 실제로 더해져 두 번째 청크가 되감기지 않는다 | `the_second_chunk_does_not_rewind_to_the_start_of_the_timeline` (starts = 0 · 11,700 · 12,000 · 12,500) |
| 반복 차단 임계값 **바로 아래** | `three_consecutive_repeats_survive` (3회 전부 남는다) |
| 반복 차단 임계값 **바로 위** | `the_fourth_consecutive_repeat_is_blocked` (4회째만 지워지고 `removed_count = 1`) |
| 연속이 아닌 반복은 자르지 않는다 | `repeats_that_are_not_consecutive_are_not_blocked` (같은 문장 10회지만 떨어져 있다) · `a_different_sentence_restarts_the_count` |
| 텍스트가 없는 segment가 섞였을 때 | `segments_without_text_pass_through_the_shift_untouched` (`text: None`도 시각은 옮기고 버리지 않는다) · `segments_without_a_sentence_are_neither_counted_nor_blocked` · `many_blank_segments_in_a_row_are_all_kept` |

ADR-0007 §20.8이 함께 요구한 것들:

| | |
| --- | --- |
| 청크가 문장을 하나도 못 내도 다음 오프셋이 밀리지 않는다 | `a_chunk_that_produced_nothing_does_not_shift_the_chunks_after_it` |
| `offset_cs(k) = 12,000 × k` · 12,000이 프레임 L에서 나머지 없이 나온다 | `the_offset_of_chunk_k_is_twelve_thousand_centiseconds_times_k` · `twelve_thousand_centiseconds_comes_out_of_the_frame_count_without_remainder` |
| 겹침 0 — 프레임 하나도 두 번 들어가지 않는다 | `chunks_are_adjacent_and_no_frame_is_used_twice` |
| 공백만 다른 두 문장은 같은 문장이다 | `sentences_that_differ_only_in_whitespace_are_the_same_sentence` |
| language: 첫 값이 이긴다 · 아무도 보고하지 않으면 None · 설정 값이 들어갈 자리가 없다 | `the_first_reported_language_wins` · `a_language_nobody_reported_is_not_invented` |
| 넘침을 saturate하지 않는다 | `an_offset_that_cannot_be_represented_is_a_failure_not_a_saturated_value` · `a_shifted_timestamp_that_cannot_be_represented_is_a_failure` |

## ADR에 없어서 이 Run이 정한 것 (Verifier가 볼 자리)

| 자리 | 정한 것 | 왜 |
| --- | --- | --- |
| `sample_rate_hz == 0` | `FailureKind::InvalidInput` 실패 | 0으로는 프레임을 시각으로 말할 수 없다. 새 `FailureKind`를 만들지 않았다 (`parse.rs`의 선례) |
| 오프셋 · 시각의 `i64` 넘침 | 같은 실패 | §20.5의 *"넘침은 조용히 접지 않는다"* 를 그대로 따랐다 |
| 문장이 없는(공백뿐인) segment의 차단 | 세지도 지우지도 않고 **묶음을 끊는다** | `collapse`가 빈 문장을 분모에 넣지 않는 것과 같은 태도다. 있지도 않은 문장을 반복으로 세지 않는다 |
