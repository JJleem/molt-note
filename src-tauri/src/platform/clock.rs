//! 흐르는 시간을 읽는 경계 (PRODUCT-SPEC §18).
//!
//! 녹음 session의 상태 기계는 시각을 **값으로** 받는다 — 그 안에는 시계가 없다
//! (`crate::audio::session`). 그 값을 실제로 만들어 주는 자리가 여기다.
//!
//! 경계를 하나 더 만드는 이유는 하나다. 시계가 코드 안에 박혀 있으면 "1시간 녹음의 경과
//! 시간"을 확인하는 방법이 **실제로 1시간을 기다리는 것뿐**이 된다. 이 trait 자리에 값을
//! 넣으면 같은 확인이 즉시 끝난다.
//!
//! ## 벽시계가 아니다
//!
//! [`MonotonicClock`]은 [`std::time::Instant`]를 쓴다. 사용자가 시스템 시각을 바꾸거나
//! 서머타임이 바뀌어도 진행 중인 녹음의 길이가 흔들리지 않는다. 그래서 이 값은 **차이에만
//! 의미가 있고** 어떤 날짜도 가리키지 않는다 — 저장할 시각(`createdAt`)은 여기서 오지 않는다.

use std::time::Instant;

/// 어떤 기준점에서 흐른 밀리초를 알려주는 것.
///
/// 값이 뒤로 가지 않는 것이 유일한 요구사항이다. 뒤로 가더라도 녹음 길이가 음수가 되지는
/// 않는다 — 그 방어는 상태 기계 쪽에 있다 (`RecordingSession::elapsed_ms`).
pub trait Clock: Send + Sync {
    /// 기준점에서 흐른 밀리초.
    fn now_ms(&self) -> i64;
}

/// 뒤로 가지 않는 시계. 앱이 이것을 쓴다.
#[derive(Debug, Clone)]
pub struct MonotonicClock {
    /// 이 시계가 만들어진 순간. 모든 값이 여기서부터 잰 것이다.
    origin: Instant,
}

impl MonotonicClock {
    /// 지금을 기준점으로 삼는 시계 하나.
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MonotonicClock {
    /// 기준점에서 흐른 밀리초.
    ///
    /// `i64`에 담기지 않을 만큼 오래 켜져 있었다면 포화시킨다 — 시계를 읽는 일이 앱을
    /// 멈추게 할 이유는 없다. (그 값은 2억 9천만 년쯤이다.)
    fn now_ms(&self) -> i64 {
        i64::try_from(self.origin.elapsed().as_millis()).unwrap_or(i64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_clock_starts_at_its_own_origin() {
        let clock = MonotonicClock::new();

        // 방금 만든 시계는 0에 가깝다. 절대 시각이 아니라 기준점에서 잰 값이기 때문이다.
        let now = clock.now_ms();
        assert!(now >= 0, "음수가 나올 수 없다: {now}");
        assert!(now < 60_000, "기준점은 이 시계가 만들어진 순간이다: {now}");
    }

    #[test]
    fn the_clock_never_goes_backwards() {
        let clock = MonotonicClock::new();

        let mut previous = clock.now_ms();
        for _ in 0..100 {
            let now = clock.now_ms();
            assert!(now >= previous, "{now} < {previous}");
            previous = now;
        }
    }

    #[test]
    fn two_clocks_measure_from_their_own_origins() {
        // 시계는 날짜를 가리키지 않는다 — 각자의 기준점에서 잰 값을 준다.
        let first = MonotonicClock::new();
        let second = MonotonicClock::new();

        assert!(first.now_ms() >= second.now_ms());
    }
}
