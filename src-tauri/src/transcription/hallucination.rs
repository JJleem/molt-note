//! 창 하나를 통째로 채운 **한 줄짜리 상투구**를 걸러낸다.
//!
//! ## 기전 — 2026-09-08에 관측했다
//!
//! whisper의 디코딩 창은 30초다 (`WHISPER_CHUNK_SIZE`). 그 창 안에 알아들을 말이 없으면
//! 디코더는 침묵을 내놓는 대신 **학습 데이터의 자막 상투구 한 줄을 창 전체에 붙인다.**
//!
//! ```text
//! 00:58:00  30.0초  '한글자막 by 한효정'
//! 00:58:30  30.0초  '한글자막 by 한효정'
//! 00:59:00  30.0초  '한글자막 by 한효정'
//! 01:00:00  30.0초  '이 시각 세계였습니다.'
//! ```
//!
//! **정확히 30초 간격이다.** 이것이 이 규칙의 근거다 — 문구가 아니라 **창을 채운 방식**이
//! 환각을 드러낸다.
//!
//! ## 왜 문구 목록이 아닌가
//!
//! 알려진 문구를 막는 방식은 두 가지로 실패한다.
//!
//! ```text
//! 오탐   '구독제로 가야 될 거 아니야.' (1.4초 · 실제 발화)가 "구독"에 걸린다
//! 누락   'GGG' · '-' · 'twohang' 같은 것은 목록에 없다. 그리고 목록은 언제나 뒤늦다
//! ```
//!
//! 둘 다 2026-09-08에 실제로 확인했다. 말의 **속도**로 판정하면 둘 다 일어나지 않는다 —
//! 어느 언어의 어떤 상투구든 창을 채우고 말이 없으면 걸린다.
//!
//! ## 왜 VAD로는 안 됐는가
//!
//! 같은 날 VAD(Silero)를 켜고 같은 파일을 돌렸으나 이 구간은 그대로 남았다. 그 10분은
//! **디지털 무음이 아니라 알아들을 수 없이 작은 소리**이고, VAD는 거기서 음성 활동을
//! 찾아낸다. VAD가 지우는 것은 침묵이지 웅얼거림이 아니다.

use super::parse::TranscriptSegment;

/// 이보다 짧은 segment는 보지 않는다.
///
/// **20초다.** 2026-09-08에 세 전사(9/4 두 번 · 9/8 한 번 · 합계 6,141 segment)로 재서
/// 고른 값이다.
///
/// ```text
/// 20초   잡힌 27개가 전부 환각이었다. 실제 발화는 하나도 걸리지 않았다
/// 15초   '그러면 일단 위클리를 미루고' (16.2초 · 실제 발화)가 걸린다
/// 10초   '아, 네.' · '공윤은 어떻게 해야지.' 까지 걸린다
/// ```
///
/// **[미검증]** 다른 녹음 · 다른 언어에서도 20초가 맞는지는 재본 적이 없다.
pub const MIN_SUSPECT_SECONDS: f64 = 20.0;

/// 이보다 느리게 말한 것은 말이 아니다 — **글자 수 ÷ 초**다.
///
/// 한글은 글자 하나가 한 음절이므로 이 값이 말의 속도와 거의 같다. 관측된 값:
///
/// ```text
/// 3.6 자/초   '- 아 일정 본인 일정이세요? …' (20.1초 · 73자)   실제 발화
/// 11.4 자/초  '구독제로 가야 될 거 아니야.' (1.4초 · 16자)      실제 발화
/// 0.37 자/초  '한글자막 by 한효정' (30초 · 11자)                환각
/// 0.14 자/초  'GGG' (22초 · 3자)                                환각
/// 0.04 자/초  '-' (25.7초 · 1자)                                환각
/// ```
///
/// **[미검증]** 라틴 문자는 음절당 글자 수가 달라서 같은 값이 맞지 않을 수 있다.
/// 이 저장소가 잰 것은 한국어뿐이다.
pub const MIN_CHARS_PER_SECOND: f64 = 1.0;

/// 걸러낸 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filtered {
    pub segments: Vec<TranscriptSegment>,
    /// 지운 개수. **값으로 남는다** — 무엇이 지워졌는지 나중에 셀 수 있어야 한다.
    pub removed_count: usize,
}

/// 창을 채운 상투구를 지운다.
///
/// **판정은 segment 하나씩 독립이다.** 앞뒤를 보지 않으므로 되풀이가 아니어도 잡히고,
/// 되풀이여도 말의 속도가 정상이면 그대로 남는다 — 연속 반복 차단(`chunking`)과 겹치지
/// 않고 서로를 대신하지도 않는다.
pub fn drop_windows_without_speech(segments: &[TranscriptSegment]) -> Filtered {
    let mut kept = Vec::with_capacity(segments.len());
    let mut removed_count = 0usize;

    for segment in segments {
        if is_window_without_speech(segment) {
            removed_count += 1;
        } else {
            kept.push(segment.clone());
        }
    }

    Filtered {
        segments: kept,
        removed_count,
    }
}

/// 이 segment가 **창을 채웠는데 말이 없는** 것인가.
pub fn is_window_without_speech(segment: &TranscriptSegment) -> bool {
    let seconds = (segment.end_ms - segment.start_ms) as f64 / 1_000.0;
    if seconds < MIN_SUSPECT_SECONDS {
        return false;
    }

    // 공백은 말이 아니다. 글자만 센다.
    let characters = segment
        .text
        .chars()
        .filter(|character| !character.is_whitespace())
        .count() as f64;

    characters / seconds < MIN_CHARS_PER_SECOND
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(start_ms: i64, end_ms: i64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.to_owned(),
        }
    }

    /// 2026-09-08에 실제로 나온 값들이다.
    #[test]
    fn the_boilerplate_that_filled_a_whole_window_is_dropped() {
        for (seconds, text) in [
            (30.0, "한글자막 by 한효정"),
            (30.0, "이 시각 세계였습니다."),
            (30.0, "다음 영상에서 만나요."),
            (22.0, "GGG"),
            (25.7, "-"),
        ] {
            let end = (seconds * 1_000.0) as i64;
            assert!(
                is_window_without_speech(&segment(0, end, text)),
                "{text:?} ({seconds}초)"
            );
        }
    }

    /// **같은 날 관측된 실제 발화다.** 하나라도 걸리면 사람이 말한 것을 지우게 된다.
    #[test]
    fn real_speech_is_never_dropped() {
        for (seconds, text) in [
            (20.1, "- 아 일정 본인 일정이세요? - 아니 아니요. 이게 어르신들 보니까 영상으로서"),
            (1.4, "구독제로 가야 될 거 아니야."),
            (16.2, "그러면 일단 위클리를 미루고"),
            (12.0, "아, 네."),
            (14.0, "공윤은 어떻게 해야지."),
            (0.5, "네"),
        ] {
            let end = (seconds * 1_000.0) as i64;
            assert!(
                !is_window_without_speech(&segment(0, end, text)),
                "실제 발화가 지워졌다: {text:?} ({seconds}초)"
            );
        }
    }

    /// 짧은 segment는 아예 보지 않는다 — 속도만으로 판정하면 짧은 대답이 죽는다.
    #[test]
    fn short_segments_are_never_judged_by_rate() {
        // 1자 / 5초 = 0.2 자/초. 속도만 보면 걸리지만 20초에 못 미치므로 남는다.
        assert!(!is_window_without_speech(&segment(0, 5_000, "네")));
    }

    #[test]
    fn the_threshold_is_a_boundary_not_a_range() {
        let end = (MIN_SUSPECT_SECONDS * 1_000.0) as i64;
        // 정확히 20초이고 20자면 1.0 자/초 — **미만**이 아니므로 남는다.
        assert!(!is_window_without_speech(&segment(0, end, &"가".repeat(20))));
        // 한 글자 적으면 0.95 자/초로 떨어진다.
        assert!(is_window_without_speech(&segment(0, end, &"가".repeat(19))));
    }

    #[test]
    fn dropping_keeps_the_order_and_counts_what_it_removed() {
        let segments = vec![
            segment(0, 3_000, "실제 발화입니다"),
            segment(3_000, 33_000, "한글자막 by 한효정"),
            segment(33_000, 36_000, "이어지는 실제 발화"),
        ];

        let filtered = drop_windows_without_speech(&segments);

        assert_eq!(filtered.removed_count, 1);
        assert_eq!(
            filtered
                .segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>(),
            vec!["실제 발화입니다", "이어지는 실제 발화"]
        );
    }

    #[test]
    fn nothing_to_drop_leaves_everything_and_counts_zero() {
        let segments = vec![segment(0, 3_000, "가나다라마바사")];
        let filtered = drop_windows_without_speech(&segments);

        assert_eq!(filtered.removed_count, 0);
        assert_eq!(filtered.segments, segments);
    }

    #[test]
    fn an_empty_list_is_handled() {
        assert_eq!(drop_windows_without_speech(&[]).segments, Vec::new());
    }

    /// 공백은 말이 아니다 — 띄어쓰기로 글자 수를 채워 규칙을 지나갈 수 없다.
    #[test]
    fn whitespace_does_not_count_as_speech() {
        let text = format!("가{}", " ".repeat(100));
        assert!(is_window_without_speech(&segment(0, 30_000, &text)));
    }
}
