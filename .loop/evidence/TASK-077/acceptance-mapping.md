# TASK-077 — Acceptance Criteria가 무엇으로 판정되는가

## AC-1 · AC-2 · AC-3 (Gate)

`gate-results.md` — build · lint · test 세 Gate가 exit 0으로 통과했다.
(Worker 로컬 실행은 참고용이며 Runtime이 독립적으로 다시 돌린다.)

### AC-3의 (a)~(e) 다섯 경우가 실제로 어디에 있는가

Task가 요구한 다섯 경우는 전부 `src-tauri/src/transcription/collapse.rs`의
`mod tests`에 있고, 전부 실행되어 통과했다 (`collapse-unit-tests.txt`).

| Task가 요구한 경우 | 테스트 이름 | 고정하는 값 |
| --- | --- | --- |
| **(a)** segment 103개 · 고유 2개 · 한 문장 99% → 붕괴 | `the_2026_09_07_transcription_is_collapsed` | 2026-09-07 실측 그대로다 — `한글자막 by 한효정` 102회 + `감사합니다.` 1회. 판정은 `Collapsed { unique_ratio_too_low: true, one_sentence_dominates: true }` |
| **(b)** 고유 94% → 통과 | `a_transcription_with_ninety_four_percent_unique_sentences_passes` | n=100 · u=94 · r=7 → `Usable` |
| **(c)** 최소 개수 미만의 짧은 전사에서 같은 말이 반복돼도 → 통과 | `a_short_transcription_is_not_called_collapsed_even_when_every_sentence_repeats` | 19문장 전부 같은 문장(고유 5.3% · 최다 100%)인데도 `Usable`. 개수가 충분했다면 두 조건에 모두 걸렸을 값이다 |
| **(d)** 빈 열 → 쓸 수 없음 | `an_empty_sequence_is_unusable` · `a_sequence_of_blank_segments_is_unusable_too` | `Verdict::Empty` · `is_usable() == false`. 공백뿐인 segment 3개도 같은 판정이며, 그때 `segment_count = 3`이고 `sentence_count = 0`이다 |
| **(e)** 판정 값이 수치를 잃지 않는다 | `the_verdict_carries_the_numbers_that_made_it` | `segment_count 103` · `sentence_count 103` · `unique_count 2` · `top_repeat_count 102` · `top_sentence = "한글자막 by 한효정"` · `unique_ratio() ≈ 1.9%` · `top_repeat_share() ≈ 99.0%` |

경계와 규칙을 함께 못 박는 테스트 (요구된 다섯 개 위에 더한 것):

| 테스트 | 고정하는 것 |
| --- | --- |
| `the_unique_ratio_threshold_sits_exactly_where_the_adr_put_it` | n=20 · u=4 → `u/n = 0.20` 정확히 → 붕괴(**이하**). 최다 반복은 25%로 다른 조건에 걸리지 않는다 |
| `the_top_repeat_threshold_sits_exactly_where_the_adr_put_it` | n=20 · r=10 → `r/n = 0.50` 정확히 → 붕괴(**이상**). 고유 비율 55%는 다른 조건에 걸리지 않는다 |
| `just_past_both_thresholds_is_usable` | u/n=60% · r/n=45% → `Usable` |
| `the_minimum_count_is_the_first_count_that_gets_judged` | 같은 문장만 19개 → `Usable`, 20개 → 붕괴 |
| `sentences_that_differ_only_in_whitespace_are_the_same_sentence` · `case_and_punctuation_are_not_touched` | 공백 정규화 규칙(내부 연속 공백 축약 · 앞뒤 제거)과, **대소문자·문장부호는 건드리지 않는다**는 것 |
| `blank_segments_do_not_enter_the_denominator` | 빈 segment 30개를 섞어도 비율이 변하지 않는다 |
| `the_same_input_always_gives_the_same_top_sentence` | 같은 입력 → 같은 판정·같은 수치. 동률이면 사전 순으로 앞선 문장 |

## AC-4 (Verifier) — 모듈이 순수하다

읽을 자리: `src-tauri/src/transcription/collapse.rs`의 `use` 구문과 본문.

```rust
use std::collections::BTreeMap;
use super::parse::TranscriptSegment;
```

**이 둘이 전부다.** 파일시스템 · 데이터베이스(rusqlite) · 네트워크 · 시계 · 엔진 타입에 대한
의존이 없다. 입력은 이미 정규화된 segment 열 하나이고, 출력은 값 하나다.

기계가 지키는 검사 두 개가 같은 파일에 있다:

- `this_module_does_not_know_the_outside_world` — 이 파일의 **테스트를 뺀 부분**을
  `include_str!`로 읽어 `use std::fs` · `std::process` · `Command::new` · `rusqlite::` ·
  `whisper_rs` · `crate::db` · `crate::platform` · `super::engine` · `super::run` ·
  `Instant::now` · `SystemTime::now`가 없음을 확인한다. (parse.rs의 같은 검사와 같은 방법이다.)
- `the_thresholds_live_in_exactly_one_place` — 세 상수의 정의가 이 파일에 **정확히 한 번씩**
  있고, 비율 나눗셈도 한 함수에만 있으며, `run.rs` · `engine.rs` · `whisper.rs`에
  임계값 리터럴(`0.20` · `0.50`)이 **하나도 없음**을 확인한다.

복제 여부의 현재 상태: `run.rs` · `engine.rs` · `whisper.rs` · `src/screens/**` 어디에도
임계값도 비율 계산도 없다. 이 Run은 그 파일들을 건드리지 않았다 (`changed-files.txt`).

> 이 검사가 막는 것은 **규칙의 복제**이지 이 모듈을 부르는 일이 아니다. 저장 직전에서
> `assess(...)`를 부르고 그 수치를 실패 문장에 쓰는 다음 Task는 이 검사에 걸리지 않는다.

## AC-5 (Verifier) — ADR-0007 §18.2와 값이 일치하는가

| ADR-0007 §18.2가 적은 것 | 코드 |
| --- | --- |
| 문장 = 앞뒤 공백 제거 + 내부 연속 공백 한 칸 축약, 대소문자·문장부호 유지, 빈 문자열은 세지 않음 | `sentence_key()` (`split_whitespace().join(" ")`) · `assess()`가 빈 결과를 건너뛴다 |
| `n` = 빈 문장을 뺀 총 개수 | `CollapseMetrics::sentence_count` |
| `u` = 서로 다른 문장 수 → `u/n` | `unique_count` · `unique_ratio()` |
| `r` = 최다 출현 횟수 → `r/n` | `top_repeat_count` · `top_repeat_share()` |
| `n == 0` → 쓸 수 없다 (빈 결과) | `CollapseVerdict::Empty` |
| `n < 20` → 붕괴로 판정하지 않는다 | `MINIMUM_SENTENCES_TO_JUDGE = 20` → `Usable` |
| `n >= 20` 이고 `u/n <= 0.20` → 붕괴 | `COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW = 0.20` · `<=` |
| `n >= 20` 이고 `r/n >= 0.50` → 붕괴 | `COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE = 0.50` · `>=` |
| 두 조건은 OR이며, 어느 쪽이 필요한지는 [미검증] | `Collapsed { unique_ratio_too_low, one_sentence_dominates }` — 걸린 조건을 값으로 남긴다 |
| 판정은 수치를 잃지 않는다 (§18.3) | `CollapseAssessment`가 판정과 `CollapseMetrics`를 함께 들고 다닌다. 최다 반복 **문장 자체**도 남는다 |

**§18이 정했지만 이 Task가 하지 않은 것** (다른 Task의 몫이며, 여기서 하지 않은 것이 맞다):

- §18.4 `FailureKind::TranscriptionOutputUnusable`로 만드는 것 — 이 모듈은 `Failure`를 만들지
  않는다. 이미 있는 `engine::output_unusable`을 부르는 자리는 저장 직전이다.
- §18.5 저장을 막고 `transcription_status = failed`로 옮기는 것 — `run.rs`가 할 일이다.
- 화면 표현 — Task 범위 밖이다.
