# TASK-025 — 단위 오류 mutation 검사 (AC5의 실제 증거)

AC5는 "×10 · ×100 단위 오류를 **반드시** 잡는" 테스트를 요구하고, "변환식을 그대로 되풀이해
계산하는 테스트는 이 조건을 만족하지 않는다"고 못박는다.

**테스트가 리터럴 기대값을 단언한다는 주장만으로는 증거가 아니다.** 그래서 계수를 실제로
어긋나게 바꿔 보고, 테스트가 죽는 것을 확인했다. 아래는 그 실행 결과다.

절차:

```text
1. src-tauri/src/transcription/parse.rs 의 MILLISECONDS_PER_CENTISECOND 를 10 → 100 으로 바꾼다
2. node tools/loop-runtime/loopctl.mjs self-check test
3. 10 → 1 로 바꾸고 다시 돌린다
4. 10 으로 되돌리고 build · lint · test 를 다시 돌려 초록을 확인한다   ← 저장소에 남은 상태
```

## 1. ×100 어긋남 (계수 10 → 100)

```text
test result: FAILED. 136 passed; 10 failed; 0 ignored
```

죽은 테스트 10개 — 값 단언이 그대로 어긋난 수를 잡았다.

```text
---- one_minute_thirty_seconds_is_ninety_thousand_milliseconds stdout ----
assertion `left == right` failed: 1분 30초는 90000 밀리초다
  left: 900000
 right: 90000

---- an_hour_long_recording_does_not_drift_at_the_end stdout ----
assertion `left == right` failed
  left: 36000000
 right: 3600000

---- known_timestamps_map_to_literal_milliseconds stdout ----
assertion `left == right` failed: 1센티초는 10밀리초여야 한다
  left: 100
 right: 10

---- a_normal_output_becomes_the_domain_value_without_complaint stdout ----
assertion `left == right` failed
  left: [TranscriptSegment { start_ms: 0, end_ms: 25000, text: "안녕하세요" }, ...]
 right: [TranscriptSegment { start_ms: 0, end_ms: 2500, text: "안녕하세요" }, ...]

---- a_segment_whose_end_precedes_its_start_keeps_its_text_and_collapses_to_a_point ----
  left: 100000   right: 10000
---- a_negative_timestamp_is_folded_to_the_start_of_the_recording ----
  left: 10000    right: 1000
---- a_truncated_last_segment_does_not_take_the_rest_of_the_transcript_with_it ----
  left: 48000    right: 4800
---- out_of_order_segments_keep_the_engines_order_and_are_reported ----
  left: 100000   right: 10000
---- overlapping_segments_are_kept_intact_and_reported ----
  left: 50000    right: 5000
---- the_conversion_factor_lives_in_exactly_one_place ----
assertion `left == right` failed: 계수는 상수 한 자리에만 있다
  left: 0        right: 1
```

전체 목록:

```text
failures:
    transcription::parse::tests::a_negative_timestamp_is_folded_to_the_start_of_the_recording
    transcription::parse::tests::a_normal_output_becomes_the_domain_value_without_complaint
    transcription::parse::tests::a_segment_whose_end_precedes_its_start_keeps_its_text_and_collapses_to_a_point
    transcription::parse::tests::a_truncated_last_segment_does_not_take_the_rest_of_the_transcript_with_it
    transcription::parse::tests::an_hour_long_recording_does_not_drift_at_the_end
    transcription::parse::tests::known_timestamps_map_to_literal_milliseconds
    transcription::parse::tests::one_minute_thirty_seconds_is_ninety_thousand_milliseconds
    transcription::parse::tests::out_of_order_segments_keep_the_engines_order_and_are_reported
    transcription::parse::tests::overlapping_segments_are_kept_intact_and_reported
    transcription::parse::tests::the_conversion_factor_lives_in_exactly_one_place
```

## 2. 변환을 잊음 (계수 10 → 1 · 센티초를 밀리초라 부르는 ×10 어긋남)

```text
test result: FAILED. 135 passed; 11 failed; 0 ignored
```

```text
failures:
    transcription::parse::tests::a_negative_timestamp_is_folded_to_the_start_of_the_recording
    transcription::parse::tests::a_normal_output_becomes_the_domain_value_without_complaint
    transcription::parse::tests::a_segment_whose_end_precedes_its_start_keeps_its_text_and_collapses_to_a_point
    transcription::parse::tests::a_timestamp_too_large_to_convert_is_a_defined_failure_rather_than_a_wrong_number
    transcription::parse::tests::a_truncated_last_segment_does_not_take_the_rest_of_the_transcript_with_it
    transcription::parse::tests::an_hour_long_recording_does_not_drift_at_the_end
    transcription::parse::tests::known_timestamps_map_to_literal_milliseconds
    transcription::parse::tests::one_minute_thirty_seconds_is_ninety_thousand_milliseconds
    transcription::parse::tests::out_of_order_segments_keep_the_engines_order_and_are_reported
    transcription::parse::tests::overlapping_segments_are_kept_intact_and_reported
    transcription::parse::tests::the_conversion_factor_lives_in_exactly_one_place
```

(11번째는 `a_timestamp_too_large_to_convert...`다 — 계수가 1이면 `i64::MAX`가 넘치지 않아
정의된 실패가 나오지 않는다. overflow 경로도 계수에 실제로 매여 있다는 뜻이다.)

## 3. 되돌린 뒤

```text
build: PASS  exit=0   lint: PASS  exit=0   test: PASS  exit=0
Self-check: all gates passed
```

저장소에 남은 값은 **`const MILLISECONDS_PER_CENTISECOND: i64 = 10;`** 이며, 그 근거는
ADR-0007 §10 · §4.1이 기록한 `whisper-rs`의 단위(센티초)와 저장 스키마의 단위(밀리초)다.

## 이 검사가 보여주지 못하는 것

**`whisper-rs`가 실제로 센티초를 준다는 사실은 이 Task가 확인하지 않았다.** ADR-0007 §14는
그것을 *"[E2] 기록은 있으나 실측은 UNVERIFIED"* 로 남겼고, crate를 실제로 추가하는 TASK-026이
빌드와 실제 값으로 확인한다. 이 검사가 증명한 것은 **계수가 어긋나면 테스트가 반드시 죽는다**는
것이지, **계수가 맞다**는 것이 아니다. 두 문장은 다르다.
