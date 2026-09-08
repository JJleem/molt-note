//! 녹음이 도는 동안 **어디까지 전사할 것인가**를 정한다 (PRODUCT-SPEC §15.1).
//!
//! ## 이 모듈이 정하는 것 하나
//!
//! 녹음 중인 파일은 계속 자란다. 어느 시점에 어느 구간을 전사할지 정하는 규칙이 필요하고,
//! **그 규칙이 이 모듈의 전부다.** 파일도 엔진도 스레드도 시계도 모른다 — 들어오는 것은
//! "지금까지 몇 프레임이 쓰였는가"와 "어디까지 전사했는가", 나가는 것은 구간 하나다.
//! `chunking`이 배치 전사에 대해 하는 일과 같은 자리다 (ADR-0007 §20.8).
//!
//! ## 왜 이런 규칙인가
//!
//! ```text
//! 최소 창    너무 짧으면 문맥이 없어 문장이 토막 난다. 짧은 창의 전사는 배치보다 나쁘다
//! 꼬리 여유   파일은 지금도 쓰이는 중이다. 맨 끝은 프레임 하나가 덜 쓰였을 수 있다
//! 겹침 없음   같은 구간을 두 번 전사하면 같은 문장이 두 번 화면에 쌓인다
//! ```
//!
//! ## 이 모듈이 하지 않는 것
//!
//! **저장하지 않는다.** 여기서 나온 전사는 **미리보기다.** 저장되는 Transcript는 여전히
//! 녹음이 끝난 뒤 배치 경로가 만든다 (`run::transcribe`) — 그 경로는 전체 오디오를 보고,
//! 청크 분할·붕괴 판정·반복 차단을 전부 지난다. 실시간은 그것을 대신하지 않는다.
//!
//! **녹음을 건드리지 않는다.** 읽기만 한다 (INV-1). 녹음 경로는 이 모듈이 있든 없든
//! 한 글자도 다르지 않게 돈다 — 2026-09-08에 화면이 죽고도 녹음이 2시간 4분을 버틴
//! 그 독립성을 이 기능이 깨뜨리지 않는다.
//!
//! **오디오를 기기 밖으로 보내지 않는다** (§15.1이 명시한 조건 · INV-6).

/// 실시간 전사가 한 번에 다루는 **최소** 길이.
///
/// 이보다 짧게 자르면 문맥이 모자라 문장이 토막 난다. 운영자가 §15.1에서 *"딜레이가 있어도
/// 상관없다"* 고 명시했으므로, 지연을 줄이려고 이 값을 낮추지 않는다.
///
/// **[미검증]** 30초가 옳은 값인지는 재본 적이 없다. 배치 경로가 쓰는 120초(§20.2)보다
/// 짧게 둔 것은 실시간의 목적이 "기다리지 않는 것"이기 때문이며, whisper의 창이 30초라는
/// 사실과도 맞는다 (`WHISPER_CHUNK_SIZE`).
pub const MIN_WINDOW_SECONDS: usize = 30;

/// 파일 끝에서 이만큼은 읽지 않는다.
///
/// 녹음은 지금도 쓰이는 중이다. 맨 끝 프레임은 절반만 쓰였을 수 있고, 그것을 읽으면
/// 마지막 샘플 하나가 튄다. **읽는 쪽이 물러선다** — 쓰는 쪽에 맞춰 달라고 하지 않는다.
pub const TAIL_MARGIN_SECONDS: usize = 1;

/// 다음에 전사할 구간. 프레임 단위이며 `[start, end)`다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub start_frame: usize,
    pub end_frame: usize,
}

impl Window {
    pub fn len(&self) -> usize {
        self.end_frame.saturating_sub(self.start_frame)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 이 구간이 전체 시간축에서 어디서 시작하는가 — 밀리초.
    ///
    /// 실시간 전사도 배치와 같은 시간축을 쓴다. 그래야 화면에 쌓인 문장의 시각이
    /// 녹음의 시각과 같은 뜻을 갖는다.
    pub fn offset_ms(&self, sample_rate_hz: u32) -> i64 {
        if sample_rate_hz == 0 {
            return 0;
        }
        (self.start_frame as i64) * 1_000 / i64::from(sample_rate_hz)
    }
}

/// 지금 전사할 구간이 있는가.
///
/// `None`이면 **아직 아니다** — 실패가 아니라 정상 상태다. 다음에 다시 물으면 된다.
///
/// ```text
/// written_frames    지금까지 파일에 쓰인 프레임 수
/// done_frames       이미 전사한 데까지 (다음 구간은 여기서 시작한다)
/// ```
pub fn next_window(
    written_frames: usize,
    done_frames: usize,
    sample_rate_hz: u32,
) -> Option<Window> {
    if sample_rate_hz == 0 {
        return None;
    }

    let margin = TAIL_MARGIN_SECONDS * sample_rate_hz as usize;
    let minimum = MIN_WINDOW_SECONDS * sample_rate_hz as usize;

    // 쓰이는 중인 꼬리는 읽지 않는다.
    let readable = written_frames.saturating_sub(margin);
    if readable <= done_frames {
        return None;
    }

    let available = readable - done_frames;
    if available < minimum {
        return None;
    }

    // **남은 것을 통째로 가져가지 않는다.** 창 하나씩 나아간다 — 그래야 한 번의 전사가
    // 길어지지 않고, 화면이 고르게 채워진다.
    Some(Window {
        start_frame: done_frames,
        end_frame: done_frames + minimum,
    })
}

/// 녹음이 끝났다. **남은 것을 마저 전사할 구간**이 있는가.
///
/// 이때는 최소 길이를 요구하지 않는다 — 더 기다려도 더 오지 않기 때문이다. 꼬리 여유도
/// 두지 않는다: 쓰기가 끝났으므로 마지막 프레임까지 완전하다.
pub fn final_window(written_frames: usize, done_frames: usize) -> Option<Window> {
    if written_frames <= done_frames {
        return None;
    }
    Some(Window {
        start_frame: done_frames,
        end_frame: written_frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 16_000;
    const SECOND: usize = 16_000;

    #[test]
    fn nothing_to_do_at_the_very_start() {
        assert_eq!(next_window(0, 0, SR), None);
    }

    #[test]
    fn a_window_shorter_than_the_minimum_waits() {
        // 20초 쓰였다. 최소 30초에 못 미친다.
        assert_eq!(next_window(20 * SECOND, 0, SR), None);
    }

    #[test]
    fn the_first_full_window_starts_at_zero() {
        // 40초 쓰였다. 꼬리 1초를 빼면 39초가 읽을 수 있고, 그중 30초를 가져간다.
        let window = next_window(40 * SECOND, 0, SR).expect("창이 있어야 한다");

        assert_eq!(window.start_frame, 0);
        assert_eq!(window.end_frame, MIN_WINDOW_SECONDS * SECOND);
    }

    #[test]
    fn the_next_window_starts_where_the_last_one_ended() {
        let first = next_window(80 * SECOND, 0, SR).expect("첫 창");
        let second = next_window(80 * SECOND, first.end_frame, SR).expect("둘째 창");

        assert_eq!(second.start_frame, first.end_frame);
        // 겹치지 않는다 — 같은 구간을 두 번 전사하면 문장이 두 번 쌓인다.
        assert!(second.start_frame >= first.end_frame);
    }

    #[test]
    fn windows_advance_one_at_a_time_even_when_much_is_available() {
        // 10분이 쌓여 있어도 한 번에 30초만 가져간다.
        let window = next_window(600 * SECOND, 0, SR).expect("창");
        assert_eq!(window.len(), MIN_WINDOW_SECONDS * SECOND);
    }

    /// **쓰이는 중인 꼬리는 읽지 않는다.** 이 여유가 없으면 절반만 쓰인 프레임을 읽는다.
    #[test]
    fn the_tail_being_written_is_left_alone() {
        // 정확히 30초 + 꼬리 여유만큼만 쓰였다면 아직 30초를 다 읽을 수 없다.
        let written = (MIN_WINDOW_SECONDS + TAIL_MARGIN_SECONDS) * SECOND - 1;
        assert_eq!(next_window(written, 0, SR), None);

        // 1프레임만 더 쓰이면 된다.
        let written = (MIN_WINDOW_SECONDS + TAIL_MARGIN_SECONDS) * SECOND;
        assert!(next_window(written, 0, SR).is_some());
    }

    #[test]
    fn a_zero_sample_rate_asks_for_nothing_instead_of_dividing_by_zero() {
        assert_eq!(next_window(600 * SECOND, 0, 0), None);
    }

    #[test]
    fn done_beyond_written_is_not_a_panic() {
        // 있을 수 없지만, 있어도 죽지 않는다.
        assert_eq!(next_window(SECOND, 10 * SECOND, SR), None);
    }

    #[test]
    fn the_offset_is_on_the_same_timeline_as_the_recording() {
        let window = Window {
            start_frame: 90 * SECOND,
            end_frame: 120 * SECOND,
        };
        assert_eq!(window.offset_ms(SR), 90_000);
    }

    // --- 녹음이 끝난 뒤 -----------------------------------------------------------------

    #[test]
    fn the_final_window_takes_whatever_is_left_however_short() {
        // 3초만 남았다. 최소 길이를 요구하지 않는다 — 더 기다려도 더 오지 않는다.
        let window = final_window(63 * SECOND, 60 * SECOND).expect("마지막 창");

        assert_eq!(window.start_frame, 60 * SECOND);
        assert_eq!(window.end_frame, 63 * SECOND);
    }

    #[test]
    fn the_final_window_reads_to_the_very_last_frame() {
        // 쓰기가 끝났으므로 꼬리 여유를 두지 않는다.
        let window = final_window(100 * SECOND, 0).expect("마지막 창");
        assert_eq!(window.end_frame, 100 * SECOND);
    }

    #[test]
    fn nothing_is_left_when_everything_was_transcribed() {
        assert_eq!(final_window(60 * SECOND, 60 * SECOND), None);
    }
}
