# TASK-025 — Acceptance Criteria가 무엇으로 판정되는가

| AC | 판정 수단 | 어디서 |
| --- | --- | --- |
| AC1 build | Gate | `gate-results.md` — exit 0 |
| AC2 lint | Gate | `gate-results.md` — `cargo clippy --all-targets -- -D warnings` exit 0 |
| AC3 test | Gate | `gate-results.md` — lib target 146 passed / 0 failed |
| AC4 정규화가 한 곳에서만 · 근거가 ADR-0007에 연결 | 아래 §1 | `parse.rs` + `the_conversion_factor_lives_in_exactly_one_place` |
| AC5 리터럴 기대값 · ×10 · ×100을 반드시 잡는다 | 아래 §2 | `mutation-check.md` (실제 mutation 실행 결과) |
| AC6 경계 · 잘못된 출력 방어, panic 없음 | 아래 §3 | 테스트 15개 |
| AC7 프로세스 실행 · 라이브러리 호출 없음 | 아래 §4 | `this_module_never_runs_a_process_or_calls_the_whisper_library` |

---

## §1 — AC4 · 단위 변환은 한 자리에만 있다

**계수:** `src-tauri/src/transcription/parse.rs`

```rust
const MILLISECONDS_PER_CENTISECOND: i64 = 10;
```

- **비공개 상수다.** `pub`이 아니므로 다른 모듈이 가져다 쓸 수 없다 — "한 자리"가 문서상의
  약속이 아니라 가시성으로 강제된다. `mod.rs`의 재수출 목록에도 없다.
- **곱셈은 `to_milliseconds()` 한 함수에만 있다.** `normalize()`가 segment마다 그 함수를
  start · end에 부르는 것이 전부다.
- 이것을 테스트가 지킨다 — `the_conversion_factor_lives_in_exactly_one_place`는 프로덕션
  구간(`#[cfg(test)]` 앞)에서 `checked_mul`이 정확히 1회, 상수 정의가 정확히 1회 나오는지
  센다. 계수가 두 자리로 늘면 한쪽만 고쳐지는 사고가 나므로 그것 자체를 막는다.

**근거의 연결 (ADR-0007):**

```text
ADR-0007 §10 표 · §4.1     whisper-rs의 segment timestamp 단위 = 센티초 (1/100초) [E2 · 2026-09-03]
ADR-0007 §10 표 [E1]        저장 스키마 transcript_segments.start_ms · end_ms = 밀리초
                            (src-tauri/src/db/migrations.rs migration 2에서 직접 확인)
                            → 1 센티초 = 10 밀리초 → 계수 10
```

상수의 doc comment가 이 두 줄을 그대로 인용하고, ADR-0007 §14가 이 단위를 **실측
UNVERIFIED**로 남겼다는 사실과 **TASK-026이 확인하고 다르면 ADR과 상수를 함께 갱신한다**는
것까지 같은 자리에 적었다. 추측으로 정규화하지 않았다.

**경계 밖에서는 단위를 만지지 않는다** (ADR-0007 §10):

```text
실행 경계 (TASK-026)  원시 값을 그대로 RawSegment에 담는다 — 계산하지 않는다
영속성   (TASK-027)  이미 밀리초인 값을 저장한다
화면     (TASK-030)  밀리초 → HH:MM:SS 표시 변환. 단위 변환이 아니다
```

---

## §2 — AC5 · 테스트가 계산을 되풀이하지 않는다

기대값은 전부 **손으로 계산해 박아 넣은 리터럴**이다. 프로덕션 계수를 참조하는 테스트는
하나도 없다 (`MILLISECONDS_PER_CENTISECOND`를 곱하는 테스트가 없다).

```text
1분 30초 = 90초 = 9000 센티초 → start_ms == 90_000
                                 assert_ne!(9_000)    ← 변환을 잊은 값
                                 assert_ne!(900_000)  ← ×100 어긋난 값
1시간     = 360000 센티초      → 3_600_000  (≠ 360_000 · ≠ 36_000_000)
1 센티초                        → 10
100 센티초                      → 1_000
12345 센티초                    → 123_450
```

**그리고 그것이 실제로 죽는지 확인했다** — 계수를 100으로, 다시 1로 바꿔 Gate를 돌렸다.
각각 10개 · 11개 테스트가 실패했다. 실행 출력은 `mutation-check.md`에 있다.

---

## §3 — AC6 · 경계와 잘못된 출력

| 상황 | 처리 | 테스트 |
| --- | --- | --- |
| 빈 출력 (segment 0개) | `Ok` · `segments: []` · `raw_text: ""` | `an_empty_output_is_an_empty_transcription_rather_than_a_panic` |
| 전부 빈 segment | 전부 버리고 anomaly로 남긴다 | `an_output_whose_segments_are_all_blank_yields_no_text_and_says_why` |
| 빈 문자열 · 공백만 있는 text | 그 segment를 버린다 (`BlankText`) | `raw_text_is_built_from_the_surviving_segments_by_one_rule` |
| 필드 누락 (`text: None`) | 그 segment를 버린다 (`TextMissing`) | 같은 테스트 · `an_output_whose_segments_...` |
| 필드 누락 (`language: None` · 빈 문자열) | `None`이 된다 | `a_missing_or_blank_language_becomes_none_rather_than_an_empty_string` |
| `start > end` | `end = start` (길이 0) · 텍스트 보존 · `EndBeforeStart` | `a_segment_whose_end_precedes_its_start_keeps_its_text_and_collapses_to_a_point` |
| 마지막 segment가 잘림 (`end`가 0으로 남음) | 같은 처리 · **앞 segment는 전부 살아남는다** | `a_truncated_last_segment_does_not_take_the_rest_of_the_transcript_with_it` |
| 음수 timestamp | `0`으로 접는다 · `NegativeStart` | `a_negative_timestamp_is_folded_to_the_start_of_the_recording` |
| 순서가 뒤바뀐 segment | **재배열하지 않는다** · `OutOfOrder` | `out_of_order_segments_keep_the_engines_order_and_are_reported` |
| 겹치는 구간 | **잘라내지 않는다** · `Overlap` | `overlapping_segments_are_kept_intact_and_reported` |
| 비정상 timestamp (`i64::MAX` · `i64::MIN`) | 정의된 [`Failure`] — saturate하지 않는다 | `a_timestamp_too_large_to_convert_is_a_defined_failure_rather_than_a_wrong_number` |
| 위의 것들이 한꺼번에 | 어떤 조합에서도 panic하지 않고 불변식이 성립한다 | `no_hostile_output_makes_this_module_panic` |

**panic이 없는 이유는 조심해서가 아니라 코드에 panic 경로가 없기 때문이다.**

- 인덱싱(`segments[i]`)이 프로덕션 코드에 없다 — 순회는 전부 이터레이터다.
- 산술은 `checked_mul` 하나뿐이다. `i64 * 10`을 그냥 쓰면 debug 빌드에서 overflow panic이
  나는데(cargo test가 debug다), 그 경로를 `Result`로 바꿨다.
- `unwrap` · `expect` · `panic!` · 슬라이싱이 프로덕션 구간에 없다.

**"깨진 JSON"은 이 경로에 존재하지 않는다.** ADR-0007 §2.1이 고른 통합 방식은 `whisper-rs`
(라이브러리)이고, sidecar가 아니다. 입력은 JSON 문자열이 아니라 타입이므로 파싱할 텍스트도,
JSON 파서도 없다. Task 서술이 "sidecar를 택했다면 ... whisper-rs를 택했다면 ..."으로 두 경우를
나눠 적은 그대로다. JSON 경로가 가졌을 실패는 타입 경계에서 이렇게 남았다:

```text
필드 누락      language: Option<String> · RawSegment::text: Option<String>  → 위 표에서 테스트됨
타입이 다른 값  타입이 고정되어 이 경계에 도달할 수 없다.
              남는 것은 범위 문제(음수 · 뒤집힘 · 넘침)이며 위 표가 전부 다룬다
```

**rawText 구성 규칙도 결정론적으로 테스트된다** — 살아남은 텍스트를 다듬어 한 칸 공백으로
잇는다. 버려진 segment가 이중 공백이나 앞뒤 공백을 남기지 않는 것,
같은 입력이 언제나 같은 값을 만드는 것(`the_same_input_always_produces_the_same_output`),
안쪽 공백은 보존되는 것을 각각 값으로 못 박았다.

**정정은 조용하지 않다.** 버리거나 접은 자리는 전부 `Transcription::anomalies`에 원시 인덱스와
함께 남고, 정상 출력에서는 그 목록이 비어 있다는 것까지 테스트한다
(`a_normal_output_becomes_the_domain_value_without_complaint`).

---

## §4 — AC7 · 프로세스도 라이브러리도 부르지 않는다

`parse.rs`의 `use` 문은 하나다.

```rust
use crate::domain::{Failure, FailureKind};
```

`std::process` · `Command` · `whisper_rs` · 파일 I/O가 전부 없다. `Cargo.toml`에는 아직 whisper
의존성 자체가 없다 (TASK-026의 몫이다). 그래서 이 모듈의 테스트 21개는 **whisper 바이너리도
모델 파일도 없이** 돌았고, 실제로 그 상태에서 통과했다 (`gate-results.md`).

이 성질은 주석이 아니라 테스트가 지킨다 —
`this_module_never_runs_a_process_or_calls_the_whisper_library`가 소스 자체를 읽어 금지된
호출 형태가 들어왔는지 검사한다. 나중에 실행 경계(TASK-026)를 쓰는 사람이 편의상 이 파일에
라이브러리 호출을 들여오면 Gate가 막는다. (검사 문자열은 조각을 이어 붙여 만든다 — 그대로
적으면 테스트가 자기 자신을 발견한다.)
