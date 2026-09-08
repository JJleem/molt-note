//! 낮게 녹음된 오디오를 **전사 입력에서만** 키운다 — 원본은 손대지 않는다.
//!
//! ## 왜 있는가 — 2026-09-08에 실측했다
//!
//! ```text
//! 9/7 회의 (51분)   평균 -42.2 dBFS · 증폭 없음   고유 문장  1.9%   전부 자막 상투구
//! 9/8 회의 (124분)  평균 -36.8 dBFS · +14 dB      고유 문장 88.9%   사람이 읽을 수 있다
//! 9/4 회의 (72분)   평균 -25.8 dBFS · 증폭 없음   고유 문장 94.7%
//! ```
//!
//! 낮은 레벨로 녹음된 회의는 그대로 돌리면 무너진다. **9/8은 증폭한 사본으로 살아났다.**
//! 그때까지 그 증폭은 사람이 저장소 밖에서 손으로 했다 — 이 모듈이 그 일을 제품 안으로
//! 옮긴다.
//!
//! **[미검증]** 같은 파일의 증폭하지 않은 판을 돌려 보지 않았으므로, 위 88.9%를 **증폭의
//! 효과라고 단정할 수 없다.** 9/7과의 비교가 근거의 전부다.
//!
//! ## 왜 이것이 no-op이 아닌가
//!
//! whisper.cpp의 mel 정규화(vendored `whisper.cpp:3228-3245`)는 최댓값 기준으로 **바닥만
//! 클램프**하고, 최종 스케일 `(mel + 4.0) / 4.0`은 **절대값**이다. 균일 이득 G dB는
//! 스펙트로그램 전체를 `G / 40`만큼 밀어 올린다 — 최댓값으로 나누는 정규화였다면 아무 일도
//! 일어나지 않았을 것이다.
//!
//! **[미검증]** 그 이동이 전사를 좋게 하는 *기전*인지는 확인되지 않았다. 관측된 것은 결과뿐이다.
//!
//! ## 무엇을 하지 않는가
//!
//! **원본 파일을 고치지 않는다** (INV-1). 이 모듈이 만지는 것은 `audio_input`이 메모리에
//! 만든 파생 버퍼이며, 그것은 재생성 가능하고 앱이 죽으면 그냥 사라진다. 녹음 파일에
//! 쓰이는 샘플은 지금도 앞으로도 들어온 그대로다 (ADR-0003 §16.5).
//!
//! **녹음 중에는 아무 일도 하지 않는다.** 마이크 게인도 만지지 않는다 — 이 모듈은 이미
//! 저장된 파일을 전사하려고 읽어 들인 뒤에만 돈다.
//!
//! **줄이지 않는다.** 이미 충분한 녹음은 건드리지 않는다. 고칠 대상은 *낮은* 입력이다.
//!
//! **소리가 없는 것은 키우지 않는다.** 무음을 목표 레벨까지 끌어올리면 잡음만 60 dB 커지고,
//! 그것은 whisper가 자막 상투구를 뱉기 가장 좋은 조건이다 — 고치려던 문제를 **만들게 된다.**

use crate::audio::level::{FULL_SCALE, SILENT_BELOW_DBFS};

/// 증폭이 목표로 삼는 평균 RMS.
///
/// **2026-09-08에 이 파이프라인으로 실제로 검증된 값이다** — +14 dB로 올린 124분 회의가
/// 평균 -23.0 dBFS였고 고유 문장 88.9%가 나왔다. 30초 창 248개 중 위험 구간(-32 dBFS 아래)에
/// 남은 것은 1개뿐이었다.
///
/// 9/4의 -25.8 dBFS(고유 94.7%)도 잘 되는 값이지만, 그 파일은 창 20개가 위험 구간에 있었고
/// **그 20개에서 환각이 났다.** 그래서 그보다 조금 위를 목표로 둔다.
pub const TARGET_DBFS: f64 = -23.0;

/// 한 번에 올릴 수 있는 최대 이득.
///
/// 목표까지 필요한 값이 이보다 크면 여기서 멈춘다. 아주 낮은 녹음을 목표까지 끌어올리려면
/// 잡음도 같은 만큼 커지는데, **증폭은 SNR을 좋게 하지 않는다** — 어느 지점부터는 잡음을
/// 키우는 일밖에 되지 않는다. 20 dB는 진폭 10배다.
///
/// **[미검증]** 이 상한이 옳은 값인지는 재본 적이 없다. 실측된 것은 +14 dB 하나뿐이며,
/// 그 값이 이 상한 아래에 있다.
pub const MAX_GAIN_DB: f64 = 20.0;

/// 이보다 작은 이득은 걸지 않는다.
///
/// 1 dB 아래는 스펙트로그램을 `1/40` 미만 밀 뿐이라 얻는 것이 없고, 그 때문에 수억 개의
/// 샘플을 다시 쓰는 것은 시간만 쓴다. 목표에 이미 닿아 있는 녹음이 부동소수점 오차 때문에
/// "조금 모자란" 것으로 읽히는 경우도 여기서 걸러진다.
pub const MIN_GAIN_DB: f64 = 1.0;

/// 증폭이 실제로 무엇을 했는가. **결정은 값으로 남는다** — 나중에 비교할 수 있어야 한다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Applied {
    /// 올리기 전 평균 RMS.
    pub before_dbfs: f64,
    /// 올린 뒤 평균 RMS. 올리지 않았으면 [`Self::before_dbfs`]와 같다.
    pub after_dbfs: f64,
    /// 실제로 곱한 이득(dB). 올리지 않았으면 `0.0`이다.
    pub gain_db: f64,
    /// `[-1.0, 1.0]`을 넘어 잘린 샘플 수.
    pub clipped: usize,
}

impl Applied {
    /// 실제로 올렸는가.
    pub fn changed(&self) -> bool {
        self.gain_db > 0.0
    }
}

/// 전사 입력 버퍼를 목표 레벨까지 올린다. **제자리에서 바꾼다.**
///
/// 아무것도 하지 않는 경우가 셋이다.
///
/// ```text
/// 비어 있다            잴 것이 없다
/// 이미 목표 이상이다    고칠 대상이 아니다 — 줄이지 않는다
/// 소리가 없다          키우면 잡음만 커진다 (SILENT_BELOW_DBFS · ADR-0003 §16.4)
/// ```
pub fn normalize(samples: &mut [f32]) -> Applied {
    let before_dbfs = average_dbfs(samples);

    let unchanged = Applied {
        before_dbfs,
        after_dbfs: before_dbfs,
        gain_db: 0.0,
        clipped: 0,
    };

    if samples.is_empty() {
        return unchanged;
    }

    // 소리가 없다. **키우면 잡음만 커진다** — 무음을 목표까지 끌어올린 버퍼는 whisper가
    // 자막 상투구를 뱉기 가장 좋은 조건이며, 그것이 고치려던 문제 자체다.
    if before_dbfs < SILENT_BELOW_DBFS {
        return unchanged;
    }

    // 이미 충분하다. **줄이지 않는다** — 고칠 대상은 낮은 입력이다.
    if before_dbfs >= TARGET_DBFS {
        return unchanged;
    }

    let gain_db = (TARGET_DBFS - before_dbfs).min(MAX_GAIN_DB);
    if gain_db < MIN_GAIN_DB {
        return unchanged;
    }

    let factor = 10.0_f64.powf(gain_db / 20.0) as f32;
    let mut clipped = 0usize;
    for sample in samples.iter_mut() {
        let scaled = *sample * factor;
        // **자른다.** whisper가 받는 것은 `[-1.0, 1.0]`이며, 넘긴 값을 그대로 넘기면
        // 그 범위를 전제한 계산이 무엇을 하는지 이 저장소가 알지 못한다.
        // 2026-09-08의 +14 dB 실측에서 잘린 것은 3.5억 중 41,970개(0.0118%)였다.
        *sample = if scaled > 1.0 {
            clipped += 1;
            1.0
        } else if scaled < -1.0 {
            clipped += 1;
            -1.0
        } else {
            scaled
        };
    }

    Applied {
        before_dbfs,
        after_dbfs: average_dbfs(samples),
        gain_db,
        clipped,
    }
}

/// `[-1.0, 1.0]` 버퍼의 평균 RMS를 dBFS로.
///
/// **환산 기준은 `audio::level`의 것을 그대로 쓴다** (`FULL_SCALE` · ADR-0003 §16.1) —
/// 여기서 기준을 다시 고르면 앱이 보여 주는 레벨과 이 모듈이 보는 레벨이 어긋난다.
/// 그래서 `f32` 진폭을 counts로 되돌린 뒤 잰다.
fn average_dbfs(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return crate::audio::level::FLOOR_DBFS;
    }

    let sum: f64 = samples
        .iter()
        .map(|sample| {
            let counts = f64::from(*sample) * FULL_SCALE;
            counts * counts
        })
        .sum();
    let rms = (sum / samples.len() as f64).sqrt();

    if rms <= 0.0 {
        return crate::audio::level::FLOOR_DBFS;
    }
    (20.0 * (rms / FULL_SCALE).log10()).max(crate::audio::level::FLOOR_DBFS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 주어진 dBFS의 평균 RMS를 갖는 톤. 진폭이 일정하므로 RMS = 진폭이다.
    fn tone_at(dbfs: f64, len: usize) -> Vec<f32> {
        let amplitude = 10.0_f64.powf(dbfs / 20.0) as f32;
        (0..len)
            .map(|index| if index % 2 == 0 { amplitude } else { -amplitude })
            .collect()
    }

    #[test]
    fn a_quiet_recording_is_lifted_to_the_target() {
        let mut samples = tone_at(-40.0, 1_000);

        let applied = normalize(&mut samples);

        assert!(applied.changed());
        assert!((applied.before_dbfs - -40.0).abs() < 0.1, "{applied:?}");
        assert!((applied.after_dbfs - TARGET_DBFS).abs() < 0.1, "{applied:?}");
        assert_eq!(applied.clipped, 0);
    }

    #[test]
    fn a_recording_that_is_already_loud_enough_is_not_touched() {
        // 줄이지 않는다 — 고칠 대상은 낮은 입력이다.
        let mut samples = tone_at(-12.0, 1_000);
        let before = samples.clone();

        let applied = normalize(&mut samples);

        assert!(!applied.changed());
        assert_eq!(applied.gain_db, 0.0);
        assert_eq!(samples, before, "버퍼가 바뀌지 않아야 한다");
    }

    #[test]
    fn exactly_at_the_target_is_not_touched() {
        let mut samples = tone_at(TARGET_DBFS, 1_000);
        let before = samples.clone();

        assert!(!normalize(&mut samples).changed());
        assert_eq!(samples, before);
    }

    /// 목표에 거의 닿은 녹음을 위해 수억 개의 샘플을 다시 쓰지 않는다.
    #[test]
    fn a_gain_too_small_to_matter_is_not_applied() {
        let mut samples = tone_at(TARGET_DBFS - (MIN_GAIN_DB / 2.0), 1_000);
        let before = samples.clone();

        let applied = normalize(&mut samples);

        assert!(!applied.changed(), "{applied:?}");
        assert_eq!(samples, before);
    }

    /// **이것이 이 모듈에서 가장 중요한 검사다.** 무음을 목표까지 끌어올리면 잡음만 커지고,
    /// 그 조건이 whisper가 자막 상투구를 뱉는 바로 그 조건이다 — 고치려던 문제를 만들게 된다.
    #[test]
    fn silence_is_never_amplified() {
        let mut samples = tone_at(SILENT_BELOW_DBFS - 6.0, 1_000);
        let before = samples.clone();

        let applied = normalize(&mut samples);

        assert!(!applied.changed(), "{applied:?}");
        assert_eq!(samples, before);
    }

    #[test]
    fn a_buffer_of_pure_zeroes_is_not_amplified_and_does_not_produce_nan() {
        let mut samples = vec![0.0_f32; 1_000];

        let applied = normalize(&mut samples);

        assert!(!applied.changed());
        assert!(applied.before_dbfs.is_finite(), "{applied:?}");
        assert!(samples.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn an_empty_buffer_is_handled() {
        let mut samples: Vec<f32> = Vec::new();
        let applied = normalize(&mut samples);

        assert!(!applied.changed());
        assert!(applied.before_dbfs.is_finite());
    }

    #[test]
    fn the_gain_never_exceeds_the_ceiling() {
        // 목표까지 30 dB가 필요한 아주 낮은 녹음. 상한에서 멈춘다.
        let mut samples = tone_at(-53.0, 1_000);

        let applied = normalize(&mut samples);

        assert!((applied.gain_db - MAX_GAIN_DB).abs() < 1e-9, "{applied:?}");
        assert!(applied.after_dbfs < TARGET_DBFS, "목표에 못 미쳐도 된다");
    }

    #[test]
    fn everything_stays_inside_the_range_whisper_expects() {
        // 피크가 풀스케일에 가까운데 평균은 낮은 버퍼 — 증폭하면 반드시 잘린다.
        let mut samples = tone_at(-40.0, 1_000);
        samples[0] = 0.99;
        samples[1] = -0.99;

        let applied = normalize(&mut samples);

        assert!(applied.clipped > 0, "{applied:?}");
        assert!(
            samples.iter().all(|sample| (-1.0..=1.0).contains(sample)),
            "whisper가 받는 값은 [-1.0, 1.0] 안에 있어야 한다"
        );
    }

    /// 환산 기준을 이 모듈이 다시 고르지 않는다 (ADR-0003 §16.1).
    #[test]
    fn the_full_scale_reference_is_the_one_the_level_module_defines() {
        let mut samples = vec![1.0_f32; 8];
        let applied = normalize(&mut samples);

        // 진폭 1.0 == FULL_SCALE counts == 0.0 dBFS.
        assert!(applied.before_dbfs.abs() < 0.05, "{applied:?}");
    }
}
