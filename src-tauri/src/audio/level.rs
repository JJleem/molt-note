//! 녹음 입력 레벨을 값으로 만든다 — **그 계산과 판정이 사는 자리는 이 모듈 하나다**
//! (ADR-0003 §16.3 · §16.4).
//!
//! ```text
//! 파일에 쓰이는 i16 샘플 덩어리  (capture.rs의 drain — 실시간 콜백이 아니다 · §16.2)
//!         │
//!         ▼
//!   이 모듈  ←── dBFS 환산 · 판정 구간 · 사람이 읽는 문장이 전부 여기서 만들어진다
//!         │      장치도 파일도 스레드도 저장소도 알지 않는다
//!         ▼
//!   평균 RMS의 dBFS · 전체 피크의 dBFS · 판정 하나 · 짧은 문장
//! ```
//!
//! ## 왜 이 자리인가
//!
//! 2026-09-07의 실사용에서 사람이 51분을 녹음하고 전사를 4.3분 기다린 뒤에야 소리가 담기지
//! 않았다는 것을 알았다. 그 파일의 평균 RMS는 **-42.2 dBFS**였고, 9/4에 성공한 녹음은
//! **-25.8 dBFS**였다 (ADR-0003 §16.1). 화면에 입력 레벨을 보여 주는 요소가 없었다.
//!
//! **화면은 dBFS를 다시 계산하지도, 임계값을 다시 두지도 않는다** — `tests/screen-boundary.test.ts`가
//! *"길이 포맷은 Rust에만 있다"* 를 지키는 것과 같은 이유다. 규칙이 두 벌이 되면 조용히 갈라진다.
//!
//! ## 규칙 (ADR-0003 §16.4 — 값은 그 절이 확정했다)
//!
//! ```text
//! 풀스케일   32768   16-bit 정수 PCM의 기준. §16.1의 관측값이 이 기준으로 적혀 있다
//!
//! 평균 RMS   지금까지 파일에 쓰인 **전체** 샘플의 제곱평균제곱근 (누적값)
//! 전체 피크   지금까지 파일에 쓰인 샘플의 최대 절댓값 (누적값)
//!
//! 판정 (평균 RMS의 dBFS로만 한다 — 피크로 하지 않는다)
//!   >= -36 dBFS                →  쓸 만함
//!   >= -60 이고 < -36 dBFS      →  낮음
//!   <  -60 dBFS                →  소리 없음
//!   샘플이 하나도 없다           →  값 없음 (None — '소리 없음'이 아니다)
//! ```
//!
//! **판정을 피크로 하지 않는 이유가 관측에 있다.** 9/7 파일의 전체 피크는 -19.4 dBFS로
//! 무음이 아니었다 — 피크만 보면 *"소리가 들어오고 있다"* 고 말하게 된다. 두 파일을 가른 것은
//! 피크가 아니라 평균 RMS의 16.4 dB 차이였다 (§16.4).
//!
//! ## 이 모듈이 하지 않는 것 (ADR-0003 §16.5)
//!
//! | 하지 않는 것 | 어디가 하는가 |
//! | --- | --- |
//! | 샘플을 바꾸는 것 — 게인 조정 · 정규화 | **아무도 하지 않는다.** 들어온 샘플이 그대로 파일에 쓰인다 (§4.2.3) |
//! | 샘플을 어디서 받아 언제 갱신할지 정하는 것 | `capture.rs`의 `drain` (§16.2) |
//! | 값을 화면까지 나르는 것 | `commands`의 상태 payload — **오디오 샘플 자체는 실리지 않는다** (INV-6) |
//! | 낮다고 녹음을 막거나 멈추는 것 | 아무도 하지 않는다. 이 표시는 사람에게 알릴 뿐이다 (§16.5) |
//! | 임계값을 설정 항목으로 여는 것 | 근거가 두 관측뿐이다. 틀렸다면 고칠 자리는 아래 상수 둘이다 (§16.4) |
//!
//! **누적값이 지불하는 대가를 감추지 않는다** (§16.3). 사람이 물어야 하는 질문이
//! *"이 녹음이 쓸 수 있는 소리를 담고 있는가"* 이므로 누적을 골랐지만, 누적값은 녹음 도중
//! 마이크를 고쳤을 때 곧바로 올라오지 않는다 — 누적 피크는 아예 내려가지 않는다.
//! 최근 구간 미터가 필요한지는 이 Phase가 정하지 않는다.
//!
//! 장치도 파일도 시계도 없으므로 **마이크 없이 값으로 그대로 검증된다** (PRODUCT-SPEC §18).

use std::fmt;

/// dBFS 환산의 기준 — 16-bit 정수 PCM의 풀스케일 (ADR-0003 §16.1).
///
/// **코드가 이 기준을 다시 고르지 않는다.** 다른 기준(예: 32767)을 쓰면 §16.1의 관측값과
/// 앱이 보여 주는 값이 어긋난다. `i16::MIN`의 절댓값이 정확히 이 값이다.
pub const FULL_SCALE: f64 = 32_768.0;

/// 평균 RMS가 이 값 **이상**이면 쓸 만하다 (ADR-0003 §16.4).
///
/// 근거: 아래쪽에 -42.2 dBFS(2026-09-07 · 전사 붕괴), 위쪽에 -25.8 dBFS(2026-09-04 · 고유
/// 문장 94.0%)가 있다. 두 관측의 중간은 -34.0 dBFS이고, **-36은 그 중간에 가장 가까운
/// 6 dB 눈금값**이다(6 dB = 진폭 2배). 실패한 쪽에서 6.2 dB 위, 성공한 쪽에서 10.2 dB 아래다.
pub const USABLE_AT_OR_ABOVE_DBFS: f64 = -36.0;

/// 평균 RMS가 이 값 **미만**이면 소리가 없는 것으로 본다 (ADR-0003 §16.4).
///
/// 16-bit에서 RMS 약 33 counts다. **장치는 열렸는데 아무것도 들어오지 않는 상태**를 "낮음"과
/// 구분하기 위한 값이며, **실측에서 온 값이 아니다** — 두 관측 중 어느 쪽도 이 아래가 아니다.
/// 같은 6 dB 격자 위의 값을 골랐다.
pub const SILENT_BELOW_DBFS: f64 = -60.0;

/// 이 모듈이 적는 가장 낮은 dBFS — 16-bit가 구분할 수 있는 가장 작은 진폭(1 count)의 값이다.
///
/// `20 · log10(1 / 32768) = -90.3`. 진폭이 0이면 dBFS는 `-inf`이고, 1 count 아래는 이 포맷이
/// 애초에 구분하지 못한다. **그 둘을 `-inf`나 `NaN`으로 내보내지 않고 이 값으로 적는다** —
/// 사람이 읽는 문장에 그런 값이 새면 화면이 무엇을 보여 줄지 알 수 없게 된다 (§16.3).
/// 이 값에 닿은 것은 *잰 값*이 아니라 *바닥*이므로, 문장은 그때 **"이하"** 를 함께 적는다.
pub const FLOOR_DBFS: f64 = -90.3;

/// 레벨을 눈으로 볼 수 있게 그릴 때의 아래 끝 (2026-09-08).
///
/// [`FLOOR_DBFS`](-90.3)까지 그리면 사람이 실제로 구분해야 하는 구간이 막대 끝에 몰린다.
/// 구분해야 하는 것은 **소리 없음([`SILENT_BELOW_DBFS`])과 쓸 만함([`USABLE_AT_OR_ABOVE_DBFS`])
/// 사이**이므로, 그 아래로 조금 더 내려간 자리에서 시작한다.
pub const METER_FLOOR_DBFS: f64 = -70.0;

/// 막대의 위 끝. 0 dBFS는 풀스케일이다.
pub const METER_CEILING_DBFS: f64 = 0.0;

/// dBFS 하나를 막대 위의 비율(`0.0..=1.0`)로 옮긴다.
///
/// **화면이 이 계산을 하지 않는다.** 판정 구간을 아는 자리가 이 모듈 하나이므로
/// (§16.4 · INV-9), 그 구간을 길이로 옮기는 일도 여기 있어야 한다 — 화면이 dBFS를 알게
/// 되는 순간 임계값이 두 곳에 살게 된다.
pub fn meter_fill(dbfs: f64) -> f64 {
    let span = METER_CEILING_DBFS - METER_FLOOR_DBFS;
    ((dbfs - METER_FLOOR_DBFS) / span).clamp(0.0, 1.0)
}

/// 판정 하나 (ADR-0003 §16.4).
///
/// **"값 없음"은 이 열거에 없다.** 재지 않은 것은 판정이 아니라 [`Option::None`]이며,
/// 그래서 *모르는 것*이 *낮음*으로 조용히 승격되지 않는다 (§16.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelVerdict {
    /// 평균 RMS >= -36 dBFS.
    Usable,
    /// -60 dBFS <= 평균 RMS < -36 dBFS.
    Low,
    /// 평균 RMS < -60 dBFS.
    Silent,
}

impl LevelVerdict {
    /// 사람이 읽는 판정 이름. ADR-0003 §16.4의 표기 그대로다.
    pub fn label(self) -> &'static str {
        match self {
            Self::Usable => "쓸 만함",
            Self::Low => "낮음",
            Self::Silent => "소리 없음",
        }
    }
}

impl fmt::Display for LevelVerdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// 평균 RMS의 dBFS 하나로 판정한다 — **판정 구간이 있는 자리는 이 함수 하나다**.
///
/// 부르는 쪽은 [`InputLevel::reading`]이며, 넘기는 값은 **화면에 보이는 것과 같은
/// 소수 한 자리 값**이다. 그래야 `-36.0`이 보이는데 "낮음"이라고 말하는 일이 생기지 않는다.
pub fn verdict_for_average_dbfs(average_dbfs: f64) -> LevelVerdict {
    if average_dbfs >= USABLE_AT_OR_ABOVE_DBFS {
        LevelVerdict::Usable
    } else if average_dbfs >= SILENT_BELOW_DBFS {
        LevelVerdict::Low
    } else {
        LevelVerdict::Silent
    }
}

/// 지금까지 쌓인 것에서 만든 값 하나. **판정은 수치를 잃지 않는다** (ADR-0003 §16.3).
///
/// 화면이 이 값을 그대로 쓴다 — 다시 계산하지 않는다.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelReading {
    /// 지금까지 들어온 샘플의 개수. 0이면 이 값 자체가 만들어지지 않는다.
    pub sample_count: u64,
    /// 평균 RMS (counts). 반올림하지 않은 값이며, dBFS를 만든 근거로 함께 남긴다.
    pub average_rms: f64,
    /// 전체 피크 (counts, `0..=32768`). `i16::MIN`의 절댓값이 32768이다.
    pub peak_amplitude: i32,
    /// 평균 RMS의 dBFS. **소수 한 자리**이며 [`FLOOR_DBFS`] 아래로는 내려가지 않는다.
    pub average_dbfs: f64,
    /// 전체 피크의 dBFS. 소수 한 자리. **판정에는 쓰이지 않는다** — 사람이 함께 보는 값이다.
    pub peak_dbfs: f64,
    /// 판정 하나.
    pub verdict: LevelVerdict,
    /// 사람이 읽는 짧은 문장. **화면이 이 문장을 다시 만들지 않는다** (§16.3).
    pub message: String,
}

/// 파일에 쓰이는 샘플을 받아 레벨을 쌓는 자리 (ADR-0003 §16.2).
///
/// 덩어리를 어떻게 잘라 넣어도 같은 값이 나온다 — 제곱합과 개수를 정수로 쌓기 때문에
/// 덩어리 경계에서 부동소수점 오차가 갈라지지 않는다 (§16.3의 "누적값이다").
///
/// **읽고 쌓기만 한다.** 샘플을 돌려주지도, 바꾸지도, 어디에 쓰지도 않는다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputLevel {
    /// 샘플 제곱의 합. 정수로 쌓아 덩어리 경계와 무관하게 같은 값이 되게 한다.
    ///
    /// 한 샘플의 제곱은 최대 `32768^2`이므로 `u128`은 어떤 길이의 녹음에서도 넘치지 않는다.
    sum_of_squares: u128,
    /// 지금까지 받은 샘플의 개수.
    sample_count: u64,
    /// 지금까지 본 최대 절댓값.
    peak_amplitude: i32,
}

impl InputLevel {
    /// 아직 아무것도 재지 않은 상태. **0을 잰 것과 다르다** — [`Self::reading`]이 `None`이다.
    pub fn new() -> Self {
        Self::default()
    }

    /// 파일에 쓰이는 샘플 덩어리 하나를 쌓는다. 빈 덩어리는 아무것도 바꾸지 않는다.
    ///
    /// 인자는 **읽기 전용 슬라이스**다. 이 모듈은 샘플을 바꾸지 않는다 (§16.5).
    pub fn push(&mut self, samples: &[i16]) {
        for &sample in samples {
            // i16::MIN(-32768)의 절댓값은 i16에 담기지 않는다. i32로 옮긴 뒤 절댓값을 잡는다.
            let amplitude = i32::from(sample).abs();
            self.sum_of_squares += (amplitude as u128) * (amplitude as u128);
            self.peak_amplitude = self.peak_amplitude.max(amplitude);
        }
        self.sample_count += samples.len() as u64;
    }

    /// 지금까지 받은 샘플의 개수.
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// 지금까지 쌓인 것에서 값을 만든다.
    ///
    /// **샘플이 하나도 없으면 `None`이다** — 재지 않은 것을 0이라고 말하지 않는다 (§16.3).
    /// 그래서 0 나눗셈도, `-inf`도, `NaN`도 이 함수 밖으로 나가지 않는다.
    pub fn reading(&self) -> Option<LevelReading> {
        if self.sample_count == 0 {
            return None;
        }

        let mean_square = self.sum_of_squares as f64 / self.sample_count as f64;
        let average_rms = mean_square.sqrt();

        let average_dbfs = round_to_tenth(to_dbfs(average_rms));
        let peak_dbfs = round_to_tenth(to_dbfs(f64::from(self.peak_amplitude)));
        let verdict = verdict_for_average_dbfs(average_dbfs);

        Some(LevelReading {
            sample_count: self.sample_count,
            average_rms,
            peak_amplitude: self.peak_amplitude,
            average_dbfs,
            peak_dbfs,
            verdict,
            message: describe(verdict, average_dbfs),
        })
    }
}

/// 판정과 평균 dBFS로 사람이 읽는 짧은 문장을 만든다 — **이 문장이 만들어지는 자리도 여기다**.
fn describe(verdict: LevelVerdict, average_dbfs: f64) -> String {
    let average = dbfs_phrase(average_dbfs);
    match verdict {
        LevelVerdict::Usable => format!("입력 레벨이 쓸 만하다 (평균 {average})"),
        LevelVerdict::Low => {
            format!("입력 레벨이 낮다 — 마이크와 자리를 확인한다 (평균 {average})")
        }
        LevelVerdict::Silent => {
            format!("소리가 들어오지 않는다 — 마이크를 확인한다 (평균 {average})")
        }
    }
}

/// dBFS 하나를 문장에 넣을 수 있는 형태로 적는다. 자릿수는 소수 한 자리다 (§16.3).
///
/// 바닥값에 닿았다면 **"이하"** 를 붙인다 — 그 값은 잰 값이 아니라 이 포맷이 구분하지 못하는
/// 아래쪽 전부이며, 그것을 측정값처럼 적으면 사람에게 거짓말이 된다 ([`FLOOR_DBFS`]).
fn dbfs_phrase(dbfs: f64) -> String {
    if dbfs <= FLOOR_DBFS {
        format!("{FLOOR_DBFS:.1} dBFS 이하")
    } else {
        format!("{dbfs:.1} dBFS")
    }
}

/// 진폭(counts)을 dBFS로 옮긴다. **환산이 일어나는 자리는 이 함수 하나다.**
///
/// 진폭 0은 `-inf`가 되고 1 count 아래는 16-bit가 구분하지 못하므로, 둘 다 [`FLOOR_DBFS`]로
/// 적는다. 어떤 입력으로도 `NaN`이나 무한대를 돌려주지 않는다.
fn to_dbfs(amplitude: f64) -> f64 {
    if amplitude <= 0.0 {
        return FLOOR_DBFS;
    }
    (20.0 * (amplitude / FULL_SCALE).log10()).max(FLOOR_DBFS)
}

/// 소수 한 자리로 반올림한다 (ADR-0003 §16.3의 자릿수 규칙).
///
/// `-0.0`은 `0.0`으로 만든다 — 풀스케일 근처에서 `-0.0 dBFS`가 문장에 새는 것을 막는다.
fn round_to_tenth(value: f64) -> f64 {
    let rounded = (value * 10.0).round() / 10.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 같은 진폭이 번갈아 나오는 덩어리 — RMS가 정확히 `amplitude`인 신호다.
    fn alternating(amplitude: i16, count: usize) -> Vec<i16> {
        (0..count)
            .map(|index| {
                if index % 2 == 0 {
                    amplitude
                } else {
                    -amplitude
                }
            })
            .collect()
    }

    fn reading_of(samples: &[i16]) -> LevelReading {
        let mut level = InputLevel::new();
        level.push(samples);
        level.reading().expect("샘플이 있으면 값이 있다")
    }

    // (a) 전부 0인 무음 — 잰 값이지만 dBFS가 -inf가 되는 자리다.
    #[test]
    fn digital_silence_is_judged_silent_without_leaking_negative_infinity() {
        let reading = reading_of(&[0; 1_000]);

        assert_eq!(reading.verdict, LevelVerdict::Silent);
        assert_eq!(reading.average_rms, 0.0);
        assert_eq!(reading.peak_amplitude, 0);
        // -inf도 NaN도 아니고, 이 포맷의 바닥값이다.
        assert_eq!(reading.average_dbfs, FLOOR_DBFS);
        assert_eq!(reading.peak_dbfs, FLOOR_DBFS);
        assert!(reading.average_dbfs.is_finite(), "무한대가 나가지 않는다");
        assert!(!reading.average_dbfs.is_nan(), "NaN이 나가지 않는다");

        // 바닥값은 잰 값이 아니므로 문장이 그것을 측정값처럼 적지 않는다.
        assert_eq!(
            reading.message,
            "소리가 들어오지 않는다 — 마이크를 확인한다 (평균 -90.3 dBFS 이하)"
        );
        assert_no_broken_numbers(&reading);
    }

    // (b) 2026-09-07 수준 — 평균 RMS 약 256(약 -42 dBFS). 이 레벨의 녹음이 붕괴했다.
    #[test]
    fn the_2026_09_07_level_is_judged_low() {
        // 진폭 256이 번갈아 나오면 RMS는 정확히 256이다.
        // 20·log10(256 / 32768) = -42.1  (§16.1의 관측 255.7 → -42.2 바로 옆이다)
        let reading = reading_of(&alternating(256, 1_000));

        assert_eq!(reading.average_rms, 256.0);
        assert_eq!(reading.average_dbfs, -42.1);
        assert_eq!(reading.peak_amplitude, 256);
        assert_eq!(reading.peak_dbfs, -42.1);
        assert_eq!(
            reading.verdict,
            LevelVerdict::Low,
            "-36 dBFS 아래이고 -60 dBFS 위다"
        );
        assert_eq!(
            reading.message,
            "입력 레벨이 낮다 — 마이크와 자리를 확인한다 (평균 -42.1 dBFS)"
        );

        // 관측값 그 자체(-42.2)도 같은 판정을 받는다 (ADR-0003 §16.1).
        assert_eq!(verdict_for_average_dbfs(-42.2), LevelVerdict::Low);
        assert_no_broken_numbers(&reading);
    }

    // (c) 2026-09-04 수준 — 평균 RMS 약 1685(약 -26 dBFS). 이 레벨의 녹음이 성공했다.
    #[test]
    fn the_2026_09_04_level_is_judged_usable() {
        // 20·log10(1685 / 32768) = -25.8 — §16.1이 적은 값과 소수 첫째 자리까지 같다.
        let reading = reading_of(&alternating(1_685, 1_000));

        assert_eq!(reading.average_rms, 1_685.0);
        assert_eq!(reading.average_dbfs, -25.8);
        assert_eq!(reading.verdict, LevelVerdict::Usable);
        assert_eq!(
            reading.message,
            "입력 레벨이 쓸 만하다 (평균 -25.8 dBFS)"
        );

        // 관측값 그 자체(-25.8)도 같은 판정을 받는다.
        assert_eq!(verdict_for_average_dbfs(-25.8), LevelVerdict::Usable);
        assert_no_broken_numbers(&reading);
    }

    // (d) 풀스케일 — 0 dBFS. 부호가 붙은 0(-0.0)이 문장에 새지 않는다.
    #[test]
    fn a_full_scale_signal_reads_as_zero_dbfs_without_a_minus_sign() {
        let reading = reading_of(&alternating(i16::MAX, 1_000));

        assert_eq!(reading.average_rms, f64::from(i16::MAX));
        assert_eq!(reading.peak_amplitude, 32_767);
        assert_eq!(reading.average_dbfs, 0.0, "32767은 §16.1대로 0.0 dBFS다");
        assert_eq!(reading.peak_dbfs, 0.0);
        assert!(
            !reading.message.contains("-0.0"),
            "부호가 붙은 0을 사람에게 보이지 않는다: {}",
            reading.message
        );
        assert_eq!(reading.message, "입력 레벨이 쓸 만하다 (평균 0.0 dBFS)");
        assert_eq!(reading.verdict, LevelVerdict::Usable);
        assert_no_broken_numbers(&reading);
    }

    #[test]
    fn the_most_negative_sample_does_not_overflow_and_is_exactly_full_scale() {
        // i16::MIN의 절댓값(32768)은 i16에 담기지 않는다. 여기서 패닉하면 녹음 중에 앱이 죽는다.
        let reading = reading_of(&[i16::MIN; 8]);

        assert_eq!(reading.peak_amplitude, 32_768, "풀스케일의 기준값과 같다");
        assert_eq!(reading.average_dbfs, 0.0);
        assert_eq!(reading.peak_dbfs, 0.0);
        assert_no_broken_numbers(&reading);
    }

    // (e) 샘플이 하나도 없다 — 값 없음이지 0도 '소리 없음'도 아니다.
    #[test]
    fn no_samples_at_all_is_absent_rather_than_zero() {
        let empty = InputLevel::new();
        assert_eq!(empty.reading(), None, "재지 않은 것을 0이라고 말하지 않는다");
        assert_eq!(empty.sample_count(), 0);

        // 빈 덩어리를 넣어도 여전히 잰 것이 없다 — 0 나눗셈이 일어날 자리다.
        let mut pushed_nothing = InputLevel::new();
        pushed_nothing.push(&[]);
        pushed_nothing.push(&[]);
        assert_eq!(pushed_nothing.reading(), None);
        assert_eq!(pushed_nothing.sample_count(), 0);

        // 샘플 하나만 들어와도 그때부터는 값이 있다.
        let mut one = InputLevel::new();
        one.push(&[0]);
        assert!(one.reading().is_some(), "0도 잰 값이다 — 없음과 다르다");
    }

    // (f) 덩어리 경계가 값을 바꾸지 않는다 (ADR-0003 §16.3의 "누적값이다").
    #[test]
    fn the_result_does_not_depend_on_how_the_samples_were_chunked() {
        let samples: Vec<i16> = (0..1_001)
            .map(|index: i32| ((index * 37) % 5_000 - 2_500) as i16)
            .collect();

        let whole = reading_of(&samples);

        // 한 개씩 · 7개씩 · 앞뒤로 갈라서 — 어떻게 잘라도 같은 값이어야 한다.
        let mut one_by_one = InputLevel::new();
        for sample in &samples {
            one_by_one.push(std::slice::from_ref(sample));
        }

        let mut in_sevens = InputLevel::new();
        for chunk in samples.chunks(7) {
            in_sevens.push(chunk);
        }

        let mut lopsided = InputLevel::new();
        lopsided.push(&samples[..3]);
        lopsided.push(&[]);
        lopsided.push(&samples[3..]);

        for (name, level) in [
            ("한 개씩", one_by_one),
            ("7개씩", in_sevens),
            ("3개와 나머지", lopsided),
        ] {
            let reading = level.reading().expect("샘플이 있다");
            assert_eq!(reading, whole, "덩어리 경계가 값을 바꿨다: {name}");
        }
    }

    #[test]
    fn the_judgement_boundaries_sit_exactly_where_the_adr_put_them() {
        // -36은 '이상'이 쓸 만함이고, -60은 '이상'이 낮음이다 (ADR-0003 §16.4).
        assert_eq!(verdict_for_average_dbfs(-35.9), LevelVerdict::Usable);
        assert_eq!(
            verdict_for_average_dbfs(USABLE_AT_OR_ABOVE_DBFS),
            LevelVerdict::Usable,
            "-36.0은 경계 위가 아니라 쓸 만함 안이다"
        );
        assert_eq!(verdict_for_average_dbfs(-36.1), LevelVerdict::Low);

        assert_eq!(verdict_for_average_dbfs(-59.9), LevelVerdict::Low);
        assert_eq!(
            verdict_for_average_dbfs(SILENT_BELOW_DBFS),
            LevelVerdict::Low,
            "-60.0은 아직 낮음이다 — 소리 없음은 그 아래다"
        );
        assert_eq!(verdict_for_average_dbfs(-60.1), LevelVerdict::Silent);
        assert_eq!(verdict_for_average_dbfs(FLOOR_DBFS), LevelVerdict::Silent);
    }

    #[test]
    fn what_the_screen_shows_and_what_the_verdict_says_never_disagree() {
        // RMS 519 counts의 원래 값은 -36.0057 dBFS다 — 경계 **아래**이지만 소수 한 자리로는
        // -36.0으로 보인다. 판정을 보이는 값으로 하지 않으면 화면이 '-36.0 · 낮음'을 말하게 된다.
        let reading = reading_of(&alternating(519, 1_000));

        assert_eq!(reading.average_dbfs, -36.0);
        assert_eq!(reading.verdict, LevelVerdict::Usable);
        assert!(
            reading.message.contains("-36.0 dBFS"),
            "문장과 판정이 같은 값에서 나온다: {}",
            reading.message
        );
    }

    #[test]
    fn the_peak_is_reported_but_never_decides_the_verdict() {
        // 2026-09-07 파일의 모양이다 — 전체 피크는 -19.4 dBFS로 무음이 아니었지만,
        // 평균 RMS는 -42.2 dBFS였고 그 전사는 붕괴했다 (ADR-0003 §16.4).
        let mut samples = alternating(231, 999);
        samples.push(3_494);

        let reading = reading_of(&samples);

        assert_eq!(reading.peak_amplitude, 3_494);
        assert_eq!(reading.peak_dbfs, -19.4, "§16.1이 적은 9/7의 전체 피크다");
        assert_eq!(reading.average_dbfs, -42.1);
        assert_eq!(
            reading.verdict,
            LevelVerdict::Low,
            "피크가 -19.4여도 판정은 평균 RMS로 한다: {reading:?}"
        );
        // 피크가 풀스케일에 닿아도 평균이 낮으면 여전히 낮음이다 — 51분짜리 녹음에서
        // 문 닫히는 소리 하나가 판정을 뒤집지 않는다는 뜻이다.
        let mut with_one_loud_sample = alternating(231, 100_000);
        with_one_loud_sample.push(i16::MAX);

        let reading = reading_of(&with_one_loud_sample);

        assert_eq!(reading.peak_dbfs, 0.0, "피크는 풀스케일에 닿았다");
        assert_eq!(reading.average_dbfs, -42.2);
        assert_eq!(reading.verdict, LevelVerdict::Low);
    }

    #[test]
    fn every_dbfs_value_carries_exactly_one_decimal_place() {
        // 자릿수 규칙은 §16.1의 관측 표기와 같다 — 소수 한 자리다.
        for samples in [
            vec![0_i16; 4],
            alternating(1, 4),
            alternating(33, 4),
            alternating(256, 4),
            alternating(1_685, 4),
            alternating(i16::MAX, 4),
        ] {
            let reading = reading_of(&samples);
            for value in [reading.average_dbfs, reading.peak_dbfs] {
                let text = format!("{value:.1}");
                assert_eq!(
                    text.parse::<f64>().expect("숫자다"),
                    value,
                    "소수 한 자리를 넘는 값이 남아 있다: {value}"
                );
            }
            assert_no_broken_numbers(&reading);
        }
    }

    #[test]
    fn the_same_samples_always_give_the_same_reading() {
        // 순수하다 — 시계도 장치도 보지 않는다.
        let samples = alternating(1_234, 500);
        assert_eq!(reading_of(&samples), reading_of(&samples));
    }

    #[test]
    fn this_module_does_not_know_the_outside_world() {
        // 레벨 계산이 장치 · 파일 · 스레드 · 저장소를 알기 시작하면 마이크 없이 값으로
        // 검증할 수 없게 된다 (ADR-0003 §16.3). needle을 이어 붙여 만드는 것은 이 검사가
        // 자기 자신에 걸리지 않게 하기 위해서다 (collapse.rs의 같은 검사와 같은 방법이다).
        //
        // 장치 라이브러리 자체는 여기서 다시 세지 않는다 — 그것을 아는 파일이 둘뿐이라는
        // 규칙은 저장소 전체를 보는 tests/audio-boundary.test.ts가 이미 소유한다.
        let production = production_source();
        let forbidden = [
            ["use ", "std::fs"].concat(),
            ["std::", "process"].concat(),
            ["std::", "thread"].concat(),
            ["std::", "sync"].concat(),
            ["File", "::open"].concat(),
            ["rusqlite", "::"].concat(),
            ["hound", "::"].concat(),
            ["crate::", "db"].concat(),
            ["crate::", "platform"].concat(),
            ["super::", "capture"].concat(),
            ["super::", "system_capture"].concat(),
            ["Instant", "::now"].concat(),
            ["SystemTime", "::now"].concat(),
        ];

        for needle in forbidden {
            assert!(
                !production.contains(&needle),
                "레벨 모듈에 바깥 세계가 들어왔다: {needle}"
            );
        }

        // 이 파일이 실제로 읽혔는지 확인한다 — 빈 문자열이면 위 검사는 아무것도 막지 못한다.
        assert!(production.contains("USABLE_AT_OR_ABOVE_DBFS"));
    }

    #[test]
    fn this_module_reads_samples_and_never_changes_them() {
        // ADR-0003 §16.5 — 게인 조정도 정규화도 하지 않는다. 샘플을 바꿀 수 있는 모양이
        // 표면에 없다는 것을 소스로 고정한다.
        let production = production_source();
        let forbidden = [
            ["&mut ", "[i16]"].concat(),
            ["&mut ", "Vec<i16>"].concat(),
            ["fn ", "apply_gain"].concat(),
            ["fn ", "normalize"].concat(),
            ["fn ", "set_gain"].concat(),
            ["-> ", "Vec<i16>"].concat(),
        ];

        for needle in forbidden {
            assert!(
                !production.contains(&needle),
                "레벨 모듈이 샘플을 바꾸는 자리가 생겼다: {needle}"
            );
        }

        // 샘플을 받는 자리는 읽기 전용 슬라이스 하나뿐이다.
        assert_eq!(
            production.matches("samples: &[i16]").count(),
            1,
            "샘플이 들어오는 자리는 하나다"
        );
    }

    #[test]
    fn the_thresholds_live_in_exactly_one_place() {
        // 임계값이 두 자리에 있으면 한쪽만 고쳐지는 날이 온다 (ADR-0003 §16.3 · §16.4).
        //
        // 이 검사가 막는 것은 **규칙의 복제**이지 이 모듈을 부르는 일이 아니다.
        // 캡처 경로가 `InputLevel`을 쓰는 것은 복제가 아니다.
        let production = production_source();
        let definitions = [
            "pub const FULL_SCALE: f64 = 32_768.0;",
            "pub const USABLE_AT_OR_ABOVE_DBFS: f64 = -36.0;",
            "pub const SILENT_BELOW_DBFS: f64 = -60.0;",
            "pub const FLOOR_DBFS: f64 = -90.3;",
        ];
        for definition in definitions {
            assert_eq!(
                production.matches(definition).count(),
                1,
                "임계값은 상수 한 자리에만 있다: {definition}"
            );
        }

        // dBFS 환산의 나눗셈과 로그도 한 자리다.
        assert_eq!(
            production.matches("(amplitude / FULL_SCALE).log10()").count(),
            1,
            "dBFS 환산은 한 함수에서만 일어난다"
        );

        // 그리고 그 값들이 캡처 경로에 흩어져 있지 않다.
        for (name, source) in [
            ("capture.rs", include_str!("capture.rs")),
            ("system_capture.rs", include_str!("system_capture.rs")),
        ] {
            for literal in ["-36.0", "-60.0", "log10"] {
                assert!(
                    !source.contains(literal),
                    "{name}에 레벨 판정이 복제됐다: {literal}"
                );
            }
        }
    }

    /// 사람이 읽는 문장에 깨진 수치가 새지 않는다는 것 — 다섯 경우 모두에서 함께 본다.
    fn assert_no_broken_numbers(reading: &LevelReading) {
        for broken in ["inf", "NaN", "nan"] {
            assert!(
                !reading.message.contains(broken),
                "문장에 {broken}이 새어 나왔다: {}",
                reading.message
            );
        }
        assert!(reading.average_dbfs.is_finite());
        assert!(reading.peak_dbfs.is_finite());
        assert!(reading.average_rms.is_finite());
        assert!(
            reading.average_dbfs >= FLOOR_DBFS,
            "바닥값 아래로 내려가지 않는다: {}",
            reading.average_dbfs
        );
    }

    /// 이 파일에서 테스트를 뺀 부분. 검사가 테스트 코드 자신에 걸리지 않게 한다.
    fn production_source() -> &'static str {
        include_str!("level.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("테스트 앞의 코드가 있어야 한다")
    }
}
