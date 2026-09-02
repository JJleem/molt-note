//! 녹음 길이를 사람이 읽는 형식으로 바꾸는 순수 함수 (PRODUCT-SPEC §5 · §11).
//!
//! 이 규칙이 사는 곳은 여기 한 곳이다. 조회 결과가 [`crate::domain::RecordingView`]로
//! 이미 만들어진 문자열을 함께 돌려주므로, UI가 같은 계산을 TypeScript에 다시 구현할
//! 이유가 없다.
//!
//! 파일시스템도 시계도 쓰지 않는다 — 입력이 같으면 출력이 항상 같다 (§18의 자동 검증 대상).

/// 1초의 밀리초.
const MILLIS_PER_SECOND: i64 = 1_000;
/// 1분의 초.
const SECONDS_PER_MINUTE: i64 = 60;
/// 1시간의 초.
const SECONDS_PER_HOUR: i64 = 3_600;

/// 밀리초 길이를 화면에 그대로 쓸 수 있는 문자열로 만든다.
///
/// ```text
/// 1시간 미만    m:ss       3_151_000ms → "52:31"
/// 1시간 이상    h:mm:ss    3_661_000ms → "1:01:01"
/// ```
///
/// 초 미만은 버린다(반올림하지 않는다) — `999ms`는 `"0:00"`이다.
/// 음수 길이는 존재할 수 없는 값이므로 `0`으로 본다. 부호를 붙여 표시하거나 추측해서
/// 다른 값으로 바꾸지 않는다.
pub fn format_duration_ms(duration_ms: i64) -> String {
    let total_seconds = duration_ms.max(0) / MILLIS_PER_SECOND;
    let seconds = total_seconds % SECONDS_PER_MINUTE;
    let minutes = (total_seconds / SECONDS_PER_MINUTE) % SECONDS_PER_MINUTE;
    let hours = total_seconds / SECONDS_PER_HOUR;

    if hours == 0 {
        format!("{minutes}:{seconds:02}")
    } else {
        format!("{hours}:{minutes:02}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fifty_two_minute_recording_reads_as_52_31() {
        // §5 A 화면의 예시 그대로다: 3151초 → 52:31.
        assert_eq!(format_duration_ms(3_151 * 1_000), "52:31");
    }

    #[test]
    fn a_zero_length_recording_reads_as_0_00() {
        assert_eq!(format_duration_ms(0), "0:00");
    }

    #[test]
    fn less_than_a_second_is_truncated_not_rounded_up() {
        assert_eq!(format_duration_ms(1), "0:00");
        assert_eq!(format_duration_ms(999), "0:00");
        assert_eq!(format_duration_ms(1_000), "0:01");
        assert_eq!(format_duration_ms(1_999), "0:01");
    }

    #[test]
    fn seconds_are_always_two_digits_below_a_minute() {
        assert_eq!(format_duration_ms(5_000), "0:05");
        assert_eq!(format_duration_ms(59_000), "0:59");
    }

    #[test]
    fn the_minute_boundary_carries_over() {
        assert_eq!(format_duration_ms(59_999), "0:59");
        assert_eq!(format_duration_ms(60_000), "1:00");
        assert_eq!(format_duration_ms(61_000), "1:01");
    }

    #[test]
    fn the_hour_boundary_switches_to_h_mm_ss() {
        // 59:59까지는 분:초이고, 정확히 1시간부터 시간이 붙는다.
        assert_eq!(format_duration_ms(3_599_000), "59:59");
        assert_eq!(format_duration_ms(3_599_999), "59:59");
        assert_eq!(format_duration_ms(3_600_000), "1:00:00");
        assert_eq!(format_duration_ms(3_601_000), "1:00:01");
    }

    #[test]
    fn recordings_longer_than_an_hour_keep_two_digit_minutes_and_seconds() {
        assert_eq!(format_duration_ms(3_661_000), "1:01:01");
        // 1시간 5분 9초 — 분이 한 자리여도 05로 적는다.
        assert_eq!(format_duration_ms((3_600 + 309) * 1_000), "1:05:09");
        // 10시간짜리 녹음도 자릿수 때문에 깨지지 않는다.
        assert_eq!(format_duration_ms(10 * 3_600 * 1_000), "10:00:00");
        assert_eq!(format_duration_ms(100 * 3_600 * 1_000 + 1_000), "100:00:01");
    }

    #[test]
    fn a_negative_length_is_treated_as_zero_rather_than_guessed() {
        assert_eq!(format_duration_ms(-1), "0:00");
        assert_eq!(format_duration_ms(-3_151_000), "0:00");
    }

    #[test]
    fn an_extreme_value_formats_without_overflowing() {
        // 존재할 수 없는 길이라도 패닉하지 않는다 — 조회 하나가 앱을 멈추지 않게 한다.
        let text = format_duration_ms(i64::MAX);
        assert_eq!(text.matches(':').count(), 2, "h:mm:ss 형태여야 한다: {text}");
    }

    #[test]
    fn the_same_input_always_produces_the_same_text() {
        // 순수 함수다 — 시계도 로케일도 보지 않는다.
        for ms in [0, 1, 999, 60_000, 3_151_000, 3_600_000] {
            assert_eq!(format_duration_ms(ms), format_duration_ms(ms));
        }
    }
}
