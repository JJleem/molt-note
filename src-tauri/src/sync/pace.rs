//! **얼마나 기다리고 몇 번까지 다시 보내는가** (ADR-0009 §9.2).
//!
//! adapter는 서버가 지시한 대기를 [`RetryAfter`] 값으로 옮겨 줄 뿐 자지 않는다
//! (`crate::notion::client` 모듈 문서). 그 값을 실제 시간으로 바꾸는 정책이 여기 있다.
//!
//! ```text
//! 서버가 초를 말했다        → 그 값을 그대로 쓴다. 임의로 줄이지 않는다      (§9.2-1)
//! 말하지 않았다             → 이 앱의 backoff — 1초 · 2초 · 4초 [A]           (§9.2-5)
//! 한 chunk의 자동 재시도    → 최대 3회 [A]                                     (§9.2-3)
//! 한 번의 대기              → 최대 120초 [A]. 넘으면 기다리지 않고 멈춘다      (§9.2-4)
//! 요청 사이                 → 최소 350ms [A]                                   (§9.2-6)
//! ```
//!
//! `[A]`는 **앱이 고른 값**이라는 표시다 — 확인된 API 계약이 아니다. 확인된 것은 대기 지시가
//! 정수 초라는 것과 연결당 평균 초당 3회라는 두 가지뿐이며 (PRODUCT-SPEC §14.9.1), 나머지 넷은
//! 이 앱이 "몇 분씩 조용히 멈춰 있지 않는다"는 기준으로 정했다.
//!
//! ## 자는 자리가 하나다
//!
//! 실제로 시간을 쓰는 것은 [`SleepingWaiter`] 하나이고, 그 자리에 [`testing::RecordedWaits`]를
//! 넣으면 **한 밀리초도 자지 않고** 같은 경로가 지나간다. 그래서 rate limit 재시도가 실제로
//! 대기 지시를 존중하는지를 초 단위로 기다리지 않고 검증할 수 있다 (§18).

use std::time::Duration;

use crate::notion::RetryAfter;

/// 요청 사이의 최소 간격 [A] (§9.2-6).
///
/// VERIFIED된 "연결당 평균 초당 3회"에서 나온 값이며 (1/3초 ≈ 333ms), **그 자체가 API 한도는
/// 아니다.** chunk 전송은 순차이므로 왕복 시간이 대개 이보다 길고, 이 간격이 실제로 지연을
/// 더하는 경우는 드물다.
pub const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(350);

/// 한 chunk에 대한 자동 재시도 횟수의 상한 [A] (§9.2-3).
///
/// 그 뒤에는 멈추고 실패로 남긴다 — 사용자가 다시 누를 수 있고, 그 재시도는 '이어 보내기'다.
pub const MAX_RETRIES_PER_CHUNK: u32 = 3;

/// 한 번의 대기 상한 [A] (§9.2-4).
///
/// 서버가 이보다 긴 값을 주면 **기다리지 않고** 멈춘다. 앱이 몇 분씩 조용히 멈춰 있는 것은
/// 사용자에게 "멈춘 것"과 구분되지 않는다.
pub const MAX_WAIT: Duration = Duration::from_secs(120);

/// 서버가 얼마나 기다릴지 말하지 않았을 때 쓰는 대기 [A] (§9.2-5).
///
/// **정수 초가 아닌 형식은 해석하지 않는다.** 확인된 계약이 "정수 초"이므로 그 밖의 형식을
/// 지원한다고 가정하지 않으며, 읽지 못한 값을 짐작해 채우는 대신 이 값을 쓴다.
pub const BACKOFF: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

/// 이번에 기다릴 시간, 또는 기다리지 않기로 한 사실.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pause {
    /// 이만큼 기다린 뒤 같은 chunk를 다시 보낸다.
    For(Duration),
    /// 서버가 말한 대기가 [`MAX_WAIT`]를 넘는다. **기다리지 않고** 멈추며, 사용자에게
    /// 얼마 뒤에 다시 시도하면 되는지를 말한다.
    TooLong(Duration),
}

/// 지시받은 대기와 지금까지의 재시도 횟수에서 **이번에 기다릴 시간**을 정한다.
///
/// `retries`는 이미 끝난 재시도 수다 (처음 실패했을 때 `0`).
///
/// ```
/// use std::time::Duration;
/// use molt_note_lib::notion::RetryAfter;
/// use molt_note_lib::sync::pace::{pause, Pause};
///
/// // 서버가 말한 값은 그대로 쓴다.
/// assert_eq!(pause(RetryAfter::Seconds(30), 0), Pause::For(Duration::from_secs(30)));
/// // 말하지 않았으면 이 앱의 backoff다.
/// assert_eq!(pause(RetryAfter::Unspecified, 1), Pause::For(Duration::from_secs(2)));
/// ```
pub fn pause(wait: RetryAfter, retries: u32) -> Pause {
    match wait.seconds() {
        Some(seconds) => {
            let asked = Duration::from_secs(u64::from(seconds));
            if asked > MAX_WAIT {
                Pause::TooLong(asked)
            } else {
                Pause::For(asked)
            }
        }
        // 서버가 말하지 않은 대기는 이 앱이 정한다. backoff는 언제나 상한 안에 있으므로
        // 여기서 `TooLong`이 나올 수 없다.
        None => {
            let index = (retries as usize).min(BACKOFF.len() - 1);
            Pause::For(BACKOFF[index])
        }
    }
}

/// 흐르는 시간을 실제로 소비하는 얇은 경계.
///
/// [`crate::platform::clock::Clock`]과 같은 이유로 존재한다 — 시간이 코드 안에 박혀 있으면
/// "대기 지시를 존중하는가"를 확인하는 방법이 **실제로 그 시간을 기다리는 것뿐**이 된다.
pub trait Waiter: Send + Sync {
    /// 이만큼 기다린다.
    fn wait(&self, duration: Duration);
}

/// 실제로 자는 [`Waiter`]. 앱이 이것을 쓴다.
///
/// **배경 스레드에서만 돈다** (`crate::commands::notion`). 그래서 이 잠이 화면도, 다른
/// command도, 저장소 연결도 붙잡지 않는다.
#[derive(Debug, Default, Clone, Copy)]
pub struct SleepingWaiter;

impl Waiter for SleepingWaiter {
    fn wait(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// 자지 않는 [`Waiter`] — **자동 검증이 지나는 자리다** (§18).
///
/// `#[cfg(test)]`가 아닌 것은 통합 테스트(`src-tauri/tests/`)가 별개 crate에서 이것을 쓰기
/// 때문이다 (`crate::notion::testing`과 같은 이유다).
pub mod testing {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::Waiter;

    /// 기다리라는 요청을 **기록만 하고 자지 않는** double.
    ///
    /// 테스트는 [`Self::waits`]로 "서버가 말한 만큼 기다리려 했는가"를 관찰한다 — 실제로
    /// 기다리지 않으므로 rate limit 경로가 초 단위 시간을 쓰지 않고 검증된다.
    #[derive(Debug, Default)]
    pub struct RecordedWaits {
        waits: Mutex<Vec<Duration>>,
    }

    impl RecordedWaits {
        pub fn new() -> Self {
            Self::default()
        }

        /// 지금까지 요청받은 대기 전부. **관찰하는 자리다.**
        pub fn waits(&self) -> Vec<Duration> {
            self.lock().clone()
        }

        /// 요청받은 대기의 총합. 실제로 흐른 시간이 아니다.
        pub fn total(&self) -> Duration {
            self.lock().iter().sum()
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Duration>> {
            self.waits.lock().expect("double의 잠금은 오염되지 않는다")
        }
    }

    impl Waiter for RecordedWaits {
        fn wait(&self, duration: Duration) {
            self.lock().push(duration);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wait_the_server_asked_for_is_used_exactly_as_given() {
        // §9.2-1: 값을 임의로 줄이지 않는다.
        for seconds in [0, 1, 30, 120] {
            assert_eq!(
                pause(RetryAfter::Seconds(seconds), 0),
                Pause::For(Duration::from_secs(u64::from(seconds)))
            );
        }
    }

    #[test]
    fn a_wait_longer_than_the_ceiling_is_not_waited_out() {
        // §9.2-4: 앱이 몇 분씩 조용히 멈춰 있지 않는다. 얼마를 요구받았는지는 값에 남는다 —
        // 사용자에게 "얼마 뒤에 다시 시도하면 되는지"를 말해야 하기 때문이다.
        assert_eq!(
            pause(RetryAfter::Seconds(121), 0),
            Pause::TooLong(Duration::from_secs(121))
        );
        assert_eq!(
            pause(RetryAfter::Seconds(3_600), 2),
            Pause::TooLong(Duration::from_secs(3_600))
        );
        assert_eq!(
            pause(RetryAfter::Seconds(120), 0),
            Pause::For(MAX_WAIT),
            "상한 그 자체는 기다린다"
        );
    }

    #[test]
    fn a_wait_the_server_did_not_state_falls_back_to_this_apps_backoff() {
        // §9.2-5: 읽지 못한 값을 지어내지 않고 앱이 고른 값을 쓴다.
        assert_eq!(pause(RetryAfter::Unspecified, 0), Pause::For(BACKOFF[0]));
        assert_eq!(pause(RetryAfter::Unspecified, 1), Pause::For(BACKOFF[1]));
        assert_eq!(pause(RetryAfter::Unspecified, 2), Pause::For(BACKOFF[2]));
    }

    #[test]
    fn the_backoff_never_runs_off_the_end_of_its_table() {
        // 재시도 상한을 넘겨 불러도 마지막 값에 머문다 — 인덱스 계산이 앱을 끝내지 않는다.
        assert_eq!(
            pause(RetryAfter::Unspecified, u32::MAX),
            Pause::For(BACKOFF[BACKOFF.len() - 1])
        );
    }

    #[test]
    fn every_backoff_value_is_within_the_ceiling_this_app_chose() {
        for backoff in BACKOFF {
            assert!(backoff <= MAX_WAIT, "앱이 고른 대기가 자신의 상한을 넘는다");
        }
    }

    #[test]
    fn the_recording_waiter_does_not_actually_spend_time() {
        use testing::RecordedWaits;

        let waiter = RecordedWaits::new();
        let before = std::time::Instant::now();
        waiter.wait(Duration::from_secs(120));
        let spent = before.elapsed();

        assert_eq!(waiter.waits(), vec![Duration::from_secs(120)]);
        assert_eq!(waiter.total(), Duration::from_secs(120));
        assert!(spent < Duration::from_secs(1), "double이 실제로 잤다: {spent:?}");
    }
}
