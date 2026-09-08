//! 3채널로 들어온 것을 **스테레오 두 채널로** 놓는 규칙 (PRODUCT-SPEC §22).
//!
//! ```text
//! 들어오는 것   [마이크] [시스템 L] [시스템 R]      aggregate device의 입력 3채널
//! 나가는 것     [L = 마이크] [R = 시스템]          파일에 쓰이는 인터리브 스테레오
//! ```
//!
//! ## 왜 이 자리가 순수한가
//!
//! 이 규칙은 오디오 콜백 안에서 돈다 — 실시간 스레드다. 거기서는 무엇이 잘못돼도 알아채기
//! 어렵고, 잘못되면 녹음이 통째로 망가진다. **그래서 규칙만 떼어 값으로 검증한다.**
//! Core Audio도 파일도 스레드도 모르는 함수 하나이며, `collapse` · `chunking`이 세운
//! 선례와 같은 자리다.
//!
//! ## 시스템 오디오를 왜 모노로 접는가
//!
//! tap은 스테레오로 온다. 그러나 저장 형태는 **L = 나 · R = 상대**이므로 상대 쪽에 쓸 수
//! 있는 칸이 하나다. 두 칸을 평균한다 — 한쪽만 고르면 그쪽에만 담긴 소리를 잃는다
//! (회의 앱이 화자를 좌우로 배치하는 경우가 있다).

/// 마이크가 놓이는 자리.
pub const MIC_CHANNEL: usize = 0;

/// 파일에 쓰이는 채널 수. **스테레오다** (§22).
pub const OUTPUT_CHANNELS: u16 = 2;

/// `f32` 하나를 `i16`으로. 범위를 넘는 값은 끝에 붙인다.
///
/// **`i16::MIN`의 절댓값이 32768이므로 그 값으로 환산한다** — `audio/level.rs`의
/// `FULL_SCALE`과 같은 기준이며, 여기서 다른 기준을 고르면 레벨 표시와 파일이 어긋난다.
fn to_i16(sample: f32) -> i16 {
    let scaled = sample * 32_768.0;
    if scaled >= 32_767.0 {
        i16::MAX
    } else if scaled <= -32_768.0 {
        i16::MIN
    } else {
        scaled as i16
    }
}

/// 인터리브된 입력 한 덩어리를 스테레오로 접는다.
///
/// `input`은 `frames × channels` 크기의 인터리브 버퍼이고, `channels`는 그 장치가 실제로
/// 내는 채널 수다 (**[A✓ 2026-09-08] 마이크 1 + tap 2 = 3**).
///
/// 나오는 것은 `frames × 2`이며 `[L, R, L, R, …]` 순서다.
///
/// ## 채널이 예상과 다르면
///
/// 장치 구성은 우리가 만들지만 그것이 언제나 3채널이라는 보장은 없다 — 마이크가 스테레오일
/// 수도 있고, tap이 모노로 올 수도 있다. **모르는 배치에서 소리를 잃지 않는 쪽으로
/// 접는다**: 마이크는 첫 채널, 시스템은 **나머지 전부의 평균**이다.
///
/// 채널이 하나뿐이면 시스템 오디오가 없는 것이므로 그 자리를 무음으로 둔다 — **마이크를
/// 양쪽에 복사하지 않는다.** 복사하면 "상대가 말했다"로 읽히는 소리가 생긴다.
pub fn fold_to_stereo(input: &[f32], channels: usize) -> Vec<i16> {
    if channels == 0 || input.is_empty() {
        return Vec::new();
    }

    let frames = input.len() / channels;
    let mut out = Vec::with_capacity(frames * usize::from(OUTPUT_CHANNELS));

    for frame in 0..frames {
        let base = frame * channels;
        let mic = input[base + MIC_CHANNEL];

        let system = if channels > 1 {
            let rest = &input[base + 1..base + channels];
            rest.iter().sum::<f32>() / rest.len() as f32
        } else {
            // 시스템 채널이 없다. **마이크를 복사하지 않는다.**
            0.0
        };

        out.push(to_i16(mic));
        out.push(to_i16(system));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_microphone_goes_left_and_the_system_goes_right() {
        // 한 프레임: 마이크 0.5 · 시스템 L 0.25 · 시스템 R 0.25
        let out = fold_to_stereo(&[0.5, 0.25, 0.25], 3);

        assert_eq!(out.len(), 2);
        assert_eq!(out[0], to_i16(0.5), "왼쪽은 마이크다");
        assert_eq!(out[1], to_i16(0.25), "오른쪽은 시스템이다");
    }

    /// 회의 앱이 화자를 좌우로 배치하면 한쪽만 골랐을 때 그 사람을 잃는다.
    #[test]
    fn the_two_system_channels_are_averaged_not_picked() {
        // 시스템 L에만 소리가 있다.
        let out = fold_to_stereo(&[0.0, 1.0, 0.0], 3);

        assert!(out[1] > 0, "한쪽에만 있는 소리를 잃으면 안 된다");
        assert_eq!(out[1], to_i16(0.5), "평균이다");
    }

    #[test]
    fn frames_are_kept_in_order_and_counted() {
        let two_frames = [0.1, 0.0, 0.0, 0.2, 0.0, 0.0];
        let out = fold_to_stereo(&two_frames, 3);

        assert_eq!(out.len(), 4);
        assert_eq!(out[0], to_i16(0.1));
        assert_eq!(out[2], to_i16(0.2));
    }

    /// **마이크를 양쪽에 복사하지 않는다.** 복사하면 "상대가 말했다"로 읽히는 소리가 생기고,
    /// §22.3의 귀속 규칙이 전부 틀리게 된다.
    #[test]
    fn a_single_channel_leaves_the_system_side_silent() {
        let out = fold_to_stereo(&[0.7, 0.7], 1);

        assert_eq!(out[0], to_i16(0.7));
        assert_eq!(out[1], 0, "마이크가 오른쪽에 복사되면 안 된다");
    }

    #[test]
    fn a_stereo_microphone_still_finds_the_system_channels() {
        // 배치가 달라도 소리를 잃지 않는다 — 첫 채널이 마이크, 나머지가 시스템이다.
        let out = fold_to_stereo(&[0.4, 0.2, 0.2, 0.2], 4);

        assert_eq!(out.len(), 2);
        assert_eq!(out[0], to_i16(0.4));
        assert_eq!(out[1], to_i16(0.2));
    }

    #[test]
    fn values_outside_the_range_are_clamped_not_wrapped() {
        // 감싸면 큰 소리가 반대 부호로 튀어 **딸깍 소리**가 된다.
        let out = fold_to_stereo(&[2.0, -2.0, -2.0], 3);

        assert_eq!(out[0], i16::MAX);
        assert_eq!(out[1], i16::MIN);
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(fold_to_stereo(&[], 3).is_empty());
        assert!(fold_to_stereo(&[0.1, 0.2], 0).is_empty());
    }

    /// 환산 기준이 `audio::level`과 같아야 한다 — 다르면 레벨 표시와 파일이 어긋난다.
    #[test]
    fn the_full_scale_reference_matches_the_level_module() {
        assert_eq!(to_i16(1.0), i16::MAX);
        assert_eq!(to_i16(-1.0), i16::MIN);
        assert_eq!(to_i16(0.0), 0);
    }
}
