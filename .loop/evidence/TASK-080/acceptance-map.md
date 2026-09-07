# TASK-080 — Acceptance Criterion이 무엇으로 판정되는가

## AC-1 · AC-2 · AC-3 (gate)

`gates.md` 참조 — build · lint · test 셋 다 exit 0이다.

AC-3이 요구한 다섯 경우는 `src-tauri/src/audio/level.rs`의 `#[cfg(test)] mod tests`에 있다.
각 경우에 대응하는 테스트 이름은 아래와 같다.

| AC-3의 경우 | 테스트 | 무엇을 값으로 고정하는가 |
| --- | --- | --- |
| 무음 (전부 0) | `digital_silence_is_judged_silent_without_leaking_negative_infinity` | 판정 `소리 없음` · dBFS `-90.3`(바닥값) · **`-inf`도 `NaN`도 문장에 새지 않는다** · 문장은 바닥값을 측정값처럼 적지 않고 "이하"를 붙인다 |
| -42 dBFS 수준 (평균 RMS 약 256) | `the_2026_09_07_level_is_judged_low` | RMS 256.0 → `-42.1` · 판정 `낮음` · 문장 전문. 관측값 그 자체(`-42.2`)도 같은 판정임을 함께 고정 |
| -26 dBFS 수준 (평균 RMS 약 1685) | `the_2026_09_04_level_is_judged_usable` | RMS 1685.0 → `-25.8` (§16.1의 표기와 소수 첫째 자리까지 같다) · 판정 `쓸 만함` · 문장 전문 |
| 풀스케일 | `a_full_scale_signal_reads_as_zero_dbfs_without_a_minus_sign` · `the_most_negative_sample_does_not_overflow_and_is_exactly_full_scale` | `32767` → `0.0` dBFS이고 **`-0.0`이 문장에 새지 않는다** · `i16::MIN`의 절댓값(32768)에서 넘치지 않는다 |
| 빈 입력 | `no_samples_at_all_is_absent_rather_than_zero` | `reading()`이 `None`이다 — 0도 `소리 없음`도 아니다. 빈 덩어리를 여러 번 넣어도 마찬가지이며, 0 나눗셈이 일어나는 자리가 아예 없다 |

Task가 함께 요구한 나머지 두 가지:

| 요구 | 테스트 |
| --- | --- |
| 누적이 덩어리 경계에 따라 달라지지 않는다 | `the_result_does_not_depend_on_how_the_samples_were_chunked` — 같은 1,001 샘플을 한 개씩 · 7개씩 · 3개와 나머지로 나눠 넣고 `LevelReading` 전체가 같은지 본다 (제곱합을 정수로 쌓아 부동소수점 오차가 갈라지지 않는다) |
| dBFS 자릿수 | `every_dbfs_value_carries_exactly_one_decimal_place` — 여섯 신호에 대해 소수 한 자리를 넘는 값이 남지 않는 것을 본다 |

## AC-4 (verifier) — 모듈이 순수하다

소스로 고정한 것:

- `this_module_does_not_know_the_outside_world` — 이 파일에서 테스트를 뺀 부분에
  `use std::fs` · `std::process` · `std::thread` · `std::sync` · `File::open` · `rusqlite::` ·
  `hound::` · `crate::db` · `crate::platform` · `super::capture` · `super::system_capture` ·
  `Instant::now` · `SystemTime::now`가 하나도 없다.
- `this_module_reads_samples_and_never_changes_them` — `&mut [i16]` · `&mut Vec<i16>` ·
  `-> Vec<i16>` · `fn apply_gain` · `fn normalize` · `fn set_gain`이 없고, 샘플이 들어오는
  자리는 읽기 전용 슬라이스 `samples: &[i16]` **하나뿐**이다.
- 장치 라이브러리를 아는 파일이 둘뿐이라는 규칙은 저장소 전체를 보는
  `tests/audio-boundary.test.ts`가 소유하며, `level.rs`는 그 목록에 들어가지 않는다
  (그 검사가 실제로 이 파일을 한 번 잡았고, 원인을 없앴다 — `gates.md`).

모듈 전체의 `use`는 `std::fmt` 하나다.

## AC-5 (verifier) — 판정 구간과 문장이 ADR-0003 §16과 일치한다

| ADR-0003 §16 | 코드 |
| --- | --- |
| §16.1 — dBFS 환산 기준은 풀스케일 **32768** | `pub const FULL_SCALE: f64 = 32_768.0;` |
| §16.4 — 쓸 만함: 평균 RMS >= **-36 dBFS** | `pub const USABLE_AT_OR_ABOVE_DBFS: f64 = -36.0;` |
| §16.4 — 소리 없음: 평균 RMS < **-60 dBFS** | `pub const SILENT_BELOW_DBFS: f64 = -60.0;` |
| §16.4 — 판정 이름 `쓸 만함` · `낮음` · `소리 없음` | `LevelVerdict::label()`이 그 세 문자열을 돌려준다 |
| §16.3 — 값 없음은 0이 아니다 (null) | `InputLevel::reading()`이 `Option<LevelReading>`이고, 샘플이 0개면 `None`이다 |
| §16.3 — 누적값이다 | `InputLevel`이 제곱합과 개수를 정수로 쌓는다 |
| §16.3 — 자릿수는 소수 한 자리 | `round_to_tenth` 하나를 지난다 |
| §16.3 — 계산 · 판정 · 문장이 한 자리에서만 | `the_thresholds_live_in_exactly_one_place` — 상수 정의가 각각 한 번뿐이고, dBFS 환산 나눗셈/로그도 한 자리이며, `capture.rs`와 `system_capture.rs`에 `-36.0` · `-60.0` · `log10`이 없다 |
| §16.4 — 판정은 피크가 아니라 평균 RMS로 한다 | `verdict_for_average_dbfs`는 평균만 받는다. `the_peak_is_reported_but_never_decides_the_verdict`가 9/7의 모양(피크 -19.4 dBFS · 평균 -42.x dBFS → `낮음`)을 고정한다 |
| §16.5 — 게인 조정도 정규화도 하지 않는다 | AC-4의 소스 검사 |

관측값이 그대로 테스트에 들어간 자리:

```text
9/7 평균  -42.2 dBFS  →  verdict_for_average_dbfs(-42.2) == Low
9/4 평균  -25.8 dBFS  →  RMS 1685에서 계산된 값이 -25.8이고 판정은 Usable
9/7 피크  -19.4 dBFS  →  peak_amplitude 3494에서 계산된 값이 -19.4
풀스케일   0.0 dBFS   →  32767에서 계산된 값이 0.0 (-0.0이 아니다)
```

## 이 Task가 하지 않은 것 (범위 밖)

- 캡처 경로(`capture.rs`의 `drain`)에 레벨 갱신을 붙이지 않았다 — TASK-081의 범위다.
- 상태 payload · `src/ipc/types.ts` · 화면을 건드리지 않았다 — TASK-082의 범위다.
- ADR-0003을 고치지 않았다. 이 모듈은 §16이 확정한 값을 소비할 뿐이다.
