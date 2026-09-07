//! 전사가 붕괴했는지 판정한다 — **그 규칙이 사는 자리는 이 모듈 하나다**
//! (ADR-0007 §18.2 · §18.3).
//!
//! ```text
//! 정규화된 segment 열 (parse.rs가 낸 것 — 단위 변환이 끝난 값)
//!         │
//!         ▼
//!   이 모듈  ←── 임계값과 비율 계산은 여기서만 일어난다
//!         │      파일시스템 · 데이터베이스 · 네트워크 · 시계 · 엔진을 알지 않는다
//!         ▼
//!   판정 하나 + 그 판정을 만든 수치 (n · u · u/n · r · r/n · 최다 반복 문장)
//! ```
//!
//! ## 왜 이 판정이 필요한가
//!
//! 2026-09-07의 실사용에서 51분 회의의 전사가 **고유 문장 2개**(한 문장이 99.0%)로 나왔고,
//! 제품은 그것을 `done`으로 저장했다. 저장 직전 검사가 물었던 것이 `segments.is_empty()`
//! 하나였기 때문이다 — **개수만 보고 내용을 보지 않았다** (ADR-0007 §18.1).
//! 이 모듈은 그 자리에 더할 두 번째 질문을 값으로 만든다.
//!
//! ## 규칙 (ADR-0007 §18.2 — 값은 그 절이 확정했다)
//!
//! ```text
//! 문장   segment의 텍스트에서 앞뒤 공백을 걷고, 내부의 연속 공백을 하나로 줄인 문자열.
//!        대소문자와 문장부호는 건드리지 않는다. 결과가 빈 문자열이면 문장으로 세지 않는다.
//!
//! n      빈 문장을 뺀 문장의 총 개수
//! u      서로 다른 문장의 개수          →  고유 비율        = u / n
//! r      가장 많이 나온 문장의 출현 횟수  →  최다 반복 점유율  = r / n
//!
//! 판정
//!   n == 0                        →  쓸 수 없다 (빈 결과)
//!   n <  20                       →  붕괴로 판정하지 않는다 (쓸 수 있다)
//!   n >= 20 이고  u/n <= 0.20      →  붕괴
//!   n >= 20 이고  r/n >= 0.50      →  붕괴
//!   그 밖                          →  쓸 수 있다
//! ```
//!
//! ## 이 모듈이 하지 않는 것
//!
//! | 하지 않는 것 | 어디가 하는가 |
//! | --- | --- |
//! | 저장을 막는 것 · 상태를 `failed`로 옮기는 것 | `run.rs`의 저장 직전 자리 (ADR-0007 §18.5) |
//! | [`crate::domain::Failure`]를 만드는 것 | 부르는 쪽이 이미 있는 `output_unusable`을 쓴다 (§18.4) |
//! | 붕괴한 텍스트를 고치는 것 · 반복을 걷어 내는 것 | 아무도 하지 않는다 — Transcript는 immutable이다 (INV-2) |
//! | 붕괴의 **원인**을 말하는 것 | 같은 판정이 낮은 입력 레벨 · 잘못된 언어 · 부족한 모델 어디서도 나온다 (§18.6) |
//! | 임계값을 설정 항목으로 여는 것 | 근거가 세 관측뿐이다. 틀렸다면 고칠 자리는 아래 상수 셋이다 (§18.6) |
//!
//! **판정은 수치를 잃지 않는다** (§18.3). 사람이 읽는 실패 문장이 *무엇이 얼마나 반복됐는가*를
//! 말할 수 있어야 하고, 그 수치가 남지 않으면 다음 사람이 같은 붕괴를 처음부터 다시 재야 한다.
//! 화면은 판정을 다시 하지 않고 여기서 나온 수치를 쓴다.
//!
//! 프로세스 실행도 라이브러리 호출도 없으므로 **실제 whisper도 모델도 없이 테스트된다**
//! (PRODUCT-SPEC §18). 붕괴한 segment 열은 값으로 그대로 만들어 낼 수 있다.

use std::collections::BTreeMap;

use super::parse::TranscriptSegment;

/// 붕괴 판정이 시작되는 최소 문장 수. 이 개수 **미만**은 판정하지 않는다 (ADR-0007 §18.2).
///
/// 근거: 관측된 두 붕괴(103 · 1,711)는 이 값의 5배 이상이다. 20개 미만에서는 `u/n`의 눈금이
/// 5%p보다 굵어지고, `r/n >= 0.50`이 "같은 말이 10번 나왔다"만으로 성립한다 — 짧고 반복적인
/// 대화가 실제로 그럴 수 있다. **짧은 녹음을 붕괴라고 부르지 않는다.**
pub const MINIMUM_SENTENCES_TO_JUDGE: usize = 20;

/// 고유 비율(`u / n`)이 이 값 **이하**면 붕괴다 (ADR-0007 §18.2).
///
/// 근거: 아래쪽에 1.9%(2026-09-07) · 3.4%(2026-09-05)가, 위쪽에 94.0%(2026-09-05)가 있다.
/// 가장 가까운 붕괴의 약 5.9배 위, 유일한 정상 관측의 약 4.7배 아래다.
pub const COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW: f64 = 0.20;

/// 최다 반복 문장의 점유율(`r / n`)이 이 값 **이상**이면 붕괴다 (ADR-0007 §18.2).
///
/// 근거: 아래쪽에 99.0%(2026-09-07) · 62.1%(2026-09-05)가, 위쪽에 0.6%(2026-09-05)가 있다.
/// **세 값 중 가장 약한 임계값이다** — 가장 가까운 붕괴(62.1%)까지 12.1%p뿐이며,
/// ADR-0007 §18.2가 그 사실을 감추지 않고 적었다.
pub const COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE: f64 = 0.50;

/// 판정을 만든 수치. **판정과 함께 다닌다** (ADR-0007 §18.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollapseMetrics {
    /// 받은 segment의 총 개수. 빈 문장이 섞여 있으면 [`Self::sentence_count`]보다 크다.
    pub segment_count: usize,
    /// `n` — 빈 문장을 뺀 문장의 총 개수.
    pub sentence_count: usize,
    /// `u` — 서로 다른 문장의 개수.
    pub unique_count: usize,
    /// `r` — 가장 많이 나온 문장의 출현 횟수. 문장이 하나도 없으면 `0`이다.
    pub top_repeat_count: usize,
    /// 가장 많이 나온 문장 (공백 정규화가 끝난 형태). 문장이 하나도 없으면 `None`이다.
    ///
    /// 사람이 읽는 실패 문장이 *무엇이* 반복됐는지 말할 수 있게 하는 값이다 —
    /// 2026-09-07의 `한글자막 by 한효정`이 그 자리에 들어간다.
    /// 같은 횟수가 여럿이면 사전 순으로 앞선 문장이다 (판정을 재현할 수 있게 고정한다).
    pub top_sentence: Option<String>,
}

impl CollapseMetrics {
    /// 고유 비율 `u / n`. 문장이 하나도 없으면 `0.0`이다.
    pub fn unique_ratio(&self) -> f64 {
        ratio(self.unique_count, self.sentence_count)
    }

    /// 최다 반복 점유율 `r / n`. 문장이 하나도 없으면 `0.0`이다.
    pub fn top_repeat_share(&self) -> f64 {
        ratio(self.top_repeat_count, self.sentence_count)
    }
}

/// 판정 하나.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollapseVerdict {
    /// 쓸 수 없다 — 문장이 하나도 없다 (`n == 0`).
    ///
    /// 붕괴와 구분해 남긴다. `run.rs`의 빈 결과 검사가 이미 이 경우를 막고 있고,
    /// 붕괴 판정은 그 검사를 대체하지 않고 더해진다 (ADR-0007 §18.5).
    Empty,
    /// 쓸 수 있다.
    Usable,
    /// 붕괴했다. 두 조건 중 **적어도 하나**가 참이다 (ADR-0007 §18.2의 OR).
    ///
    /// 어느 쪽에 걸렸는지를 남기는 이유: 관측된 두 붕괴는 **둘 다 두 조건에 함께** 걸렸고,
    /// 어느 한쪽만으로 충분한지를 그 관측들은 가르지 못했다 (§18.2 [미검증]).
    /// 걸린 조건이 값으로 남아야 다음 관측이 그 질문에 답할 수 있다.
    Collapsed {
        /// `u / n <= 0.20` — 문장의 가짓수 자체가 없다.
        unique_ratio_too_low: bool,
        /// `r / n >= 0.50` — 한 문장이 전사를 지배한다.
        one_sentence_dominates: bool,
    },
}

/// 판정과 그 판정을 만든 수치. **둘은 떨어지지 않는다** (ADR-0007 §18.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollapseAssessment {
    pub verdict: CollapseVerdict,
    pub metrics: CollapseMetrics,
}

impl CollapseAssessment {
    /// 전사 결과로 쓸 수 있는가. 빈 결과는 **쓸 수 없다**.
    pub fn is_usable(&self) -> bool {
        matches!(self.verdict, CollapseVerdict::Usable)
    }

    /// 붕괴로 판정됐는가. 빈 결과는 붕괴가 **아니다** — 그것은 이미 있는 판정이다.
    pub fn is_collapsed(&self) -> bool {
        matches!(self.verdict, CollapseVerdict::Collapsed { .. })
    }
}

/// 두 문장을 같다고 볼 규칙 — **이 규칙도 이 모듈이 정한다** (ADR-0007 §18.2).
///
/// 앞뒤 공백을 걷고 내부의 연속 공백을 한 칸으로 줄인다. 대소문자와 문장부호는 건드리지 않는다
/// — 엔진이 낸 문장을 해석하지 않고, 공백의 차이만 없앤다. 결과가 빈 문자열이면 부르는 쪽이
/// 그 segment를 문장으로 세지 않는다.
pub fn sentence_key(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// 정규화된 segment 열을 보고 **쓸 수 있는가 / 붕괴했는가**를 판정한다.
///
/// 입력은 값 하나뿐이다 — 파일도 데이터베이스도 엔진도 시계도 보지 않으므로 같은 입력은
/// 언제나 같은 판정을 낸다. 어떤 입력으로도 panic하지 않는다.
pub fn assess(segments: &[TranscriptSegment]) -> CollapseAssessment {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut sentence_count: usize = 0;

    for segment in segments {
        let sentence = sentence_key(&segment.text);
        if sentence.is_empty() {
            // 내용이 없는 구간이다. 있지도 않은 문장을 분모에 넣으면 비율이 흐려진다.
            continue;
        }
        sentence_count += 1;
        *counts.entry(sentence).or_insert(0) += 1;
    }

    let unique_count = counts.len();

    // BTreeMap은 사전 순으로 돈다. 같은 횟수가 여럿일 때 먼저 만난 것을 잡으므로,
    // 최다 반복 문장은 사전 순으로 앞선 것으로 **고정된다** — 실행마다 달라지지 않는다.
    let mut top_repeat_count: usize = 0;
    let mut top_sentence: Option<String> = None;
    for (sentence, count) in counts {
        if count > top_repeat_count {
            top_repeat_count = count;
            top_sentence = Some(sentence);
        }
    }

    let metrics = CollapseMetrics {
        segment_count: segments.len(),
        sentence_count,
        unique_count,
        top_repeat_count,
        top_sentence,
    };

    let verdict = if metrics.sentence_count == 0 {
        CollapseVerdict::Empty
    } else if metrics.sentence_count < MINIMUM_SENTENCES_TO_JUDGE {
        // 짧은 전사에서는 같은 말이 반복돼도 붕괴라고 부르지 않는다 (ADR-0007 §18.2).
        CollapseVerdict::Usable
    } else {
        let unique_ratio_too_low = metrics.unique_ratio() <= COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW;
        let one_sentence_dominates =
            metrics.top_repeat_share() >= COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE;

        if unique_ratio_too_low || one_sentence_dominates {
            CollapseVerdict::Collapsed {
                unique_ratio_too_low,
                one_sentence_dominates,
            }
        } else {
            CollapseVerdict::Usable
        }
    };

    CollapseAssessment { verdict, metrics }
}

/// 비율 하나. 분모가 0이면 `0.0`이다 — 그 경우의 판정은 비율을 보지 않는다.
fn ratio(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    part as f64 / whole as f64
}

/// ⑤ **한 segment **안에서** 되풀이되는 말을 줄인다** (2026-09-07 실사용 수정).
///
/// # 왜 segment 단위 차단으로는 안 되는가
///
/// §20.6.1의 차단은 **이어진 segment들**이 같은 문장일 때 동작한다. 그런데 2026-09-07
/// 실행이 만든 붕괴는 그 모양이 아니었다 — **segment 하나의 텍스트 안에서** 같은 말이
/// 되풀이됐다.
///
/// ```text
/// "엉덩이 엉덩이 엉덩이 … 엉덩이"           한 segment · 13회
/// "그의 정리를 지키고, 그는 그의 정리를 지키고, …"   한 segment · 20회
/// "아는게 아는게 아는게 … 아는게"           한 segment · 70회
/// ```
///
/// 묶음이 성립하지 않으니 segment 단위 차단은 이것을 하나도 잡지 못한다.
///
/// # 규칙
///
/// ```text
/// 단위     공백으로 나눈 어절. 문장부호와 대소문자는 건드리지 않는다 (sentence_key와 같은 태도)
/// 구       1 ~ MAX_PHRASE_WORDS 어절짜리 **이어붙은** 되풀이를 본다
/// 한도     `max_consecutive_repeats` 번까지 남기고 그다음부터 버린다
/// ///        **값을 여기서 정하지 않는다** — 부르는 쪽이 §20.6.1의 상수를 넘긴다
/// 공백     원래 어절 사이 공백은 한 칸으로 정규화된다 (sentence_key의 규칙과 같다)
/// ```
///
/// **짧은 구를 먼저 본다.** 긴 것부터 보면 "엉덩이"가 13번 이어진 것을 *세 어절짜리 구가
/// 4번 이어진 것*으로 읽고 9개를 남긴다 — 한 어절짜리 되풀이로 봐야 3개가 남는다. 그리고
/// 짧은 쪽이 성립하지 않을 때만 긴 구를 보므로, "그는 그의 정리를 지키고,"처럼 어절 하나로는
/// 되풀이가 아닌 것도 네 어절 묶음에서 잡힌다.
///
/// 되풀이가 없으면 **공백 정규화 말고는 아무것도 바꾸지 않는다.**
pub fn collapse_repeated_phrases(text: &str, max_consecutive_repeats: usize) -> String {
    /// 되풀이로 보는 구의 최대 길이(어절). 이보다 긴 되풀이는 이 함수가 다루지 않는다.
    const MAX_PHRASE_WORDS: usize = 8;

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_consecutive_repeats {
        return words.join(" ");
    }

    let mut kept: Vec<&str> = Vec::with_capacity(words.len());
    let mut position = 0usize;

    while position < words.len() {
        let mut collapsed = false;

        // 짧은 구부터 본다 — 긴 것부터 보면 한 어절짜리 되풀이를 구의 되풀이로 오인한다.
        for phrase_len in 1..=MAX_PHRASE_WORDS {
            if position + phrase_len > words.len() {
                continue;
            }
            let phrase = &words[position..position + phrase_len];

            // 이 자리에서 같은 구가 몇 번 이어지는가.
            let mut runs = 1usize;
            while position + (runs + 1) * phrase_len <= words.len()
                && &words[position + runs * phrase_len..position + (runs + 1) * phrase_len]
                    == phrase
            {
                runs += 1;
            }

            if runs > max_consecutive_repeats {
                for _ in 0..max_consecutive_repeats {
                    kept.extend_from_slice(phrase);
                }
                position += runs * phrase_len;
                collapsed = true;
                break;
            }
        }

        if !collapsed {
            kept.push(words[position]);
            position += 1;
        }
    }

    kept.join(" ")
}


#[cfg(test)]
mod tests {
    use super::*;

    /// 텍스트 열을 정규화된 segment 열로 만든다. 시각은 2026-09-07의 관측처럼 30초 간격이다 —
    /// **판정은 시각을 보지 않으므로** 값 자체는 판정에 영향을 주지 않는다.
    fn segments(texts: &[&str]) -> Vec<TranscriptSegment> {
        texts
            .iter()
            .enumerate()
            .map(|(index, text)| {
                let start_ms = index as i64 * 30_000;
                TranscriptSegment {
                    start_ms,
                    end_ms: start_ms + 29_980,
                    text: (*text).to_owned(),
                }
            })
            .collect()
    }

    /// 같은 문장이 `times`번 나오는 열.
    fn repeated(text: &str, times: usize) -> Vec<&str> {
        vec![text; times]
    }

    /// 서로 다른 문장 `count`개.
    fn distinct(count: usize) -> Vec<String> {
        (0..count).map(|index| format!("문장 {index}")).collect()
    }

    fn borrow(texts: &[String]) -> Vec<&str> {
        texts.iter().map(String::as_str).collect()
    }

    // (a) 2026-09-07의 실제 관측 — segment 103개 중 고유 2개, 한 문장이 99.0%.
    #[test]
    fn the_2026_09_07_transcription_is_collapsed() {
        // ADR-0007 §18.1의 세 번째 줄이다. 102회 `한글자막 by 한효정` + 1회 `감사합니다.`
        let mut texts = repeated("한글자막 by 한효정", 102);
        texts.push("감사합니다.");

        let assessment = assess(&segments(&texts));

        assert!(
            assessment.is_collapsed(),
            "고유 1.9% · 최다 99.0%는 붕괴다: {assessment:?}"
        );
        assert_eq!(
            assessment.verdict,
            CollapseVerdict::Collapsed {
                unique_ratio_too_low: true,
                one_sentence_dominates: true,
            },
            "관측된 이 붕괴는 두 조건에 함께 걸린다 (ADR-0007 §18.2)"
        );
        assert!(!assessment.is_usable(), "붕괴한 전사를 쓸 수 있다고 하지 않는다");
    }

    // (b) 2026-09-05의 실제 관측 — 고유 94%는 사람이 쓸 수 있다고 판정한 전사다.
    #[test]
    fn a_transcription_with_ninety_four_percent_unique_sentences_passes() {
        // ADR-0007 §18.1의 첫 줄과 같은 비율이다 — 100문장 중 94개가 서로 다르고,
        // 가장 많이 나온 문장이 7회(7.0%)다.
        let unique = distinct(94);
        let mut texts = borrow(&unique);
        texts.extend(repeated("문장 0", 6));

        let assessment = assess(&segments(&texts));

        assert_eq!(assessment.metrics.sentence_count, 100);
        assert_eq!(assessment.metrics.unique_count, 94);
        assert_eq!(assessment.metrics.top_repeat_count, 7);
        assert_eq!(
            assessment.verdict,
            CollapseVerdict::Usable,
            "고유 94.0% · 최다 7.0%는 붕괴가 아니다: {:?}",
            assessment.metrics
        );
    }

    // (c) 짧은 전사는 같은 말이 반복돼도 붕괴가 아니다.
    #[test]
    fn a_short_transcription_is_not_called_collapsed_even_when_every_sentence_repeats() {
        // 19개 전부가 같은 문장이다 — 고유 5.3% · 최다 100%로, 개수가 충분했다면
        // 두 조건에 모두 걸렸을 값이다.
        let texts = repeated("네", MINIMUM_SENTENCES_TO_JUDGE - 1);
        assert_eq!(texts.len(), 19);

        let assessment = assess(&segments(&texts));

        assert_eq!(
            assessment.verdict,
            CollapseVerdict::Usable,
            "최소 개수 미만은 판정하지 않는다 (ADR-0007 §18.2)"
        );
        // 판정하지 않았을 뿐, 수치는 그대로 남는다.
        assert_eq!(assessment.metrics.sentence_count, 19);
        assert_eq!(assessment.metrics.unique_count, 1);
        assert_eq!(assessment.metrics.top_repeat_count, 19);
    }

    // (d) 빈 열은 쓸 수 없다.
    #[test]
    fn an_empty_sequence_is_unusable() {
        let assessment = assess(&[]);

        assert_eq!(assessment.verdict, CollapseVerdict::Empty);
        assert!(!assessment.is_usable(), "빈 결과는 쓸 수 없다");
        assert!(!assessment.is_collapsed(), "빈 결과는 붕괴와 다른 판정이다");
        assert_eq!(assessment.metrics.sentence_count, 0);
        assert_eq!(assessment.metrics.unique_count, 0);
        assert_eq!(assessment.metrics.top_repeat_count, 0);
        assert_eq!(assessment.metrics.top_sentence, None);
        // 분모가 0일 때 비율을 지어내지 않는다.
        assert!(assessment.metrics.unique_ratio().abs() < f64::EPSILON);
        assert!(assessment.metrics.top_repeat_share().abs() < f64::EPSILON);
    }

    // (d) 문장이 하나도 없는 segment 열도 같은 판정이다 — 개수는 있지만 내용이 없다.
    #[test]
    fn a_sequence_of_blank_segments_is_unusable_too() {
        let assessment = assess(&segments(&["", "   ", "\t\n"]));

        assert_eq!(assessment.verdict, CollapseVerdict::Empty);
        assert_eq!(assessment.metrics.segment_count, 3, "받은 segment는 3개다");
        assert_eq!(assessment.metrics.sentence_count, 0, "문장은 하나도 없다");
    }

    // (e) 판정은 수치를 잃지 않는다 (ADR-0007 §18.3).
    #[test]
    fn the_verdict_carries_the_numbers_that_made_it() {
        let mut texts = repeated("한글자막 by 한효정", 102);
        texts.push("감사합니다.");

        let assessment = assess(&segments(&texts));
        let metrics = &assessment.metrics;

        // 사람이 §18.1의 표를 손으로 다시 세지 않아도 되는 값들이다.
        assert_eq!(metrics.segment_count, 103, "총 segment 수");
        assert_eq!(metrics.sentence_count, 103, "n");
        assert_eq!(metrics.unique_count, 2, "u");
        assert_eq!(metrics.top_repeat_count, 102, "r");
        assert_eq!(
            metrics.top_sentence.as_deref(),
            Some("한글자막 by 한효정"),
            "무엇이 반복됐는지가 남는다"
        );

        // 비율은 §18.1이 적은 값과 같다 — 1.9% · 99.0%.
        let unique_percent = metrics.unique_ratio() * 100.0;
        let top_percent = metrics.top_repeat_share() * 100.0;
        assert!(
            (unique_percent - 1.9).abs() < 0.05,
            "고유 비율은 1.9%다: {unique_percent}"
        );
        assert!(
            (top_percent - 99.0).abs() < 0.05,
            "최다 반복 점유율은 99.0%다: {top_percent}"
        );
    }

    #[test]
    fn the_unique_ratio_threshold_sits_exactly_where_the_adr_put_it() {
        // n = 20 · u = 4 → u/n = 0.20 정확히. '이하'이므로 붕괴다.
        // 최다 반복은 5회(25%)로 다른 조건에는 걸리지 않는다 — 두 조건이 독립임을 보인다.
        let texts: Vec<&str> = ["가", "나", "다", "라"]
            .iter()
            .flat_map(|text| repeated(text, 5))
            .collect();
        assert_eq!(texts.len(), 20);

        let assessment = assess(&segments(&texts));

        assert_eq!(
            assessment.verdict,
            CollapseVerdict::Collapsed {
                unique_ratio_too_low: true,
                one_sentence_dominates: false,
            },
            "u/n = 0.20은 경계 위가 아니라 경계 안이다: {:?}",
            assessment.metrics
        );
    }

    #[test]
    fn the_top_repeat_threshold_sits_exactly_where_the_adr_put_it() {
        // n = 20 · r = 10 → r/n = 0.50 정확히. '이상'이므로 붕괴다.
        // 고유 비율은 11/20 = 55%로 다른 조건에는 걸리지 않는다.
        let unique = distinct(10);
        let mut texts = repeated("같은 말", 10);
        texts.extend(borrow(&unique));
        assert_eq!(texts.len(), 20);

        let assessment = assess(&segments(&texts));

        assert_eq!(
            assessment.verdict,
            CollapseVerdict::Collapsed {
                unique_ratio_too_low: false,
                one_sentence_dominates: true,
            },
            "r/n = 0.50은 경계 위가 아니라 경계 안이다: {:?}",
            assessment.metrics
        );
    }

    #[test]
    fn just_past_both_thresholds_is_usable() {
        // n = 20 · u = 12 (60%) · r = 9 (45%) — 두 조건 어느 쪽에도 걸리지 않는다.
        let unique = distinct(11);
        let mut texts = repeated("같은 말", 9);
        texts.extend(borrow(&unique));
        assert_eq!(texts.len(), 20);

        let assessment = assess(&segments(&texts));

        assert_eq!(assessment.metrics.unique_count, 12);
        assert_eq!(assessment.metrics.top_repeat_count, 9);
        assert_eq!(assessment.verdict, CollapseVerdict::Usable);
    }

    #[test]
    fn the_minimum_count_is_the_first_count_that_gets_judged() {
        // 같은 문장만 있는 열을 19개 → 20개로 늘리면 판정이 바뀐다. 그 자리가 경계다.
        let below = assess(&segments(&repeated("네", MINIMUM_SENTENCES_TO_JUDGE - 1)));
        let at = assess(&segments(&repeated("네", MINIMUM_SENTENCES_TO_JUDGE)));

        assert_eq!(below.verdict, CollapseVerdict::Usable, "19개는 판정하지 않는다");
        assert!(at.is_collapsed(), "20개부터 판정한다: {:?}", at.metrics);
    }

    #[test]
    fn sentences_that_differ_only_in_whitespace_are_the_same_sentence() {
        // 공백 정규화 규칙이 없으면 아래는 고유 4개가 되어 붕괴를 빠져나간다.
        let texts = [
            "한글자막 by 한효정",
            "  한글자막 by 한효정  ",
            "한글자막  by  한효정",
            "한글자막\tby\n한효정",
        ];
        let padded: Vec<&str> = texts
            .iter()
            .cycle()
            .take(MINIMUM_SENTENCES_TO_JUDGE)
            .copied()
            .collect();

        let assessment = assess(&segments(&padded));

        assert_eq!(assessment.metrics.unique_count, 1, "네 표기는 같은 문장이다");
        assert_eq!(
            assessment.metrics.top_sentence.as_deref(),
            Some("한글자막 by 한효정"),
            "정규화된 형태로 남는다"
        );
        assert!(assessment.is_collapsed());
    }

    #[test]
    fn case_and_punctuation_are_not_touched() {
        // 규칙이 지우는 것은 공백의 차이뿐이다. 문장을 해석하지 않는다.
        assert_eq!(sentence_key(" Hello,  world! "), "Hello, world!");
        assert_ne!(sentence_key("hello"), sentence_key("Hello"));
        assert_ne!(sentence_key("네"), sentence_key("네."));
    }

    #[test]
    fn blank_segments_do_not_enter_the_denominator() {
        // 빈 문장이 분모에 들어가면 고유 비율이 실제보다 낮아 보인다.
        let sentences = distinct(20);
        let mut texts = borrow(&sentences);
        let unique = texts.clone();
        texts.extend(repeated("   ", 30));

        let assessment = assess(&segments(&texts));
        let only_sentences = assess(&segments(&unique));

        assert_eq!(assessment.metrics.segment_count, 50);
        assert_eq!(assessment.metrics.sentence_count, 20);
        assert_eq!(
            assessment.metrics.unique_ratio(),
            only_sentences.metrics.unique_ratio(),
            "빈 segment는 비율을 바꾸지 않는다"
        );
    }

    #[test]
    fn the_same_input_always_gives_the_same_top_sentence() {
        // 같은 횟수가 둘일 때도 판정과 수치가 흔들리지 않아야 한다 (재현 가능한 실패 문장).
        let mut texts = repeated("가나다", 10);
        texts.extend(repeated("라마바", 10));

        let first = assess(&segments(&texts));
        let second = assess(&segments(&texts));

        assert_eq!(first, second);
        assert_eq!(
            first.metrics.top_sentence.as_deref(),
            Some("가나다"),
            "동률이면 사전 순으로 앞선 문장이다"
        );
    }

    #[test]
    fn this_module_does_not_know_the_outside_world() {
        // 판정이 파일 · 데이터베이스 · 네트워크 · 시계 · 엔진을 알기 시작하면
        // 실제 whisper 없이 값으로 검증할 수 없게 된다 (ADR-0007 §18.3).
        // needle을 이어 붙여 만드는 것은 이 검사가 자기 자신에 걸리지 않게 하기 위해서다
        // (parse.rs의 같은 검사와 같은 방법이다).
        let production = production_source();
        let forbidden = [
            ["use ", "std::fs"].concat(),
            ["std::", "process"].concat(),
            ["Command", "::new"].concat(),
            ["rusqlite", "::"].concat(),
            ["whisper", "_rs"].concat(),
            ["crate::", "db"].concat(),
            ["crate::", "platform"].concat(),
            ["super::", "engine"].concat(),
            ["super::", "run"].concat(),
            ["Instant", "::now"].concat(),
            ["SystemTime", "::now"].concat(),
        ];

        for needle in forbidden {
            assert!(
                !production.contains(&needle),
                "판정 모듈에 바깥 세계가 들어왔다: {needle}"
            );
        }

        // 이 파일이 실제로 읽혔는지 확인한다 — 빈 문자열이면 위 검사는 아무것도 막지 못한다.
        assert!(production.contains("MINIMUM_SENTENCES_TO_JUDGE"));
    }

    #[test]
    fn the_thresholds_live_in_exactly_one_place() {
        // 임계값이 두 자리에 있으면 한쪽만 고쳐지는 날이 온다 (ADR-0007 §18.3).
        //
        // 이 검사가 막는 것은 **규칙의 복제**이지 이 모듈을 부르는 일이 아니다.
        // 저장 직전에서 `assess(...)`를 부르고 그 수치를 실패 문장에 쓰는 것은 복제가 아니다.
        let production = production_source();
        let definitions = [
            "pub const MINIMUM_SENTENCES_TO_JUDGE: usize = 20;",
            "pub const COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW: f64 = 0.20;",
            "pub const COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE: f64 = 0.50;",
        ];
        for definition in definitions {
            assert_eq!(
                production.matches(definition).count(),
                1,
                "임계값은 상수 한 자리에만 있다: {definition}"
            );
        }

        // 비율을 만드는 나눗셈도 한 자리다.
        assert_eq!(
            production.matches("part as f64 / whole as f64").count(),
            1,
            "비율 계산은 한 함수에서만 일어난다"
        );

        // 그리고 그 값들이 실행 경로와 엔진 쪽에 흩어져 있지 않다.
        for (name, source) in [
            ("run.rs", include_str!("run.rs")),
            ("engine.rs", include_str!("engine.rs")),
            ("whisper.rs", include_str!("whisper.rs")),
        ] {
            for literal in ["0.20", "0.50"] {
                assert!(
                    !source.contains(literal),
                    "{name}에 판정 임계값이 복제됐다: {literal}"
                );
            }
        }
    }

    /// 이 파일에서 테스트를 뺀 부분. 검사가 테스트 코드 자신에 걸리지 않게 한다.
    fn production_source() -> &'static str {
        include_str!("collapse.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("테스트 앞의 코드가 있어야 한다")
    }
}

#[cfg(test)]
mod intra_segment_repeat_tests {
    use super::*;
    use crate::transcription::MAX_CONSECUTIVE_REPEATS;


    #[test]
    fn the_2026_09_07_single_word_collapse_is_shortened() {
        // 실측 그대로 — 한 segment 안에서 "엉덩이"가 13회였다.
        let text = "엉덩이 ".repeat(13);
        let collapsed = collapse_repeated_phrases(text.trim(), MAX_CONSECUTIVE_REPEATS);
        assert_eq!(collapsed, "엉덩이 엉덩이 엉덩이");
    }

    #[test]
    fn the_2026_09_07_phrase_collapse_is_shortened_too() {
        // 어절 하나로 보면 되풀이가 아니다. 네 어절 묶음으로 봐야 잡힌다.
        let text = "그는 그의 정리를 지키고, ".repeat(20);
        let collapsed = collapse_repeated_phrases(text.trim(), MAX_CONSECUTIVE_REPEATS);
        assert_eq!(
            collapsed,
            "그는 그의 정리를 지키고, 그는 그의 정리를 지키고, 그는 그의 정리를 지키고,"
        );
    }

    #[test]
    fn the_limit_is_the_same_value_as_segment_level_blocking() {
        // 임계값을 두 번 정의하지 않는다 — §20.6.1과 같은 상수다.
        let kept = collapse_repeated_phrases("네 ".repeat(MAX_CONSECUTIVE_REPEATS).trim(), MAX_CONSECUTIVE_REPEATS);
        assert_eq!(kept.split_whitespace().count(), MAX_CONSECUTIVE_REPEATS);

        let blocked = collapse_repeated_phrases("네 ".repeat(MAX_CONSECUTIVE_REPEATS + 1).trim(), MAX_CONSECUTIVE_REPEATS);
        assert_eq!(blocked.split_whitespace().count(), MAX_CONSECUTIVE_REPEATS);
    }

    #[test]
    fn normal_speech_that_merely_repeats_a_little_survives() {
        // 실제 대화는 같은 말을 두세 번 한다. 그것은 붕괴가 아니다.
        let text = "아니 아니 그게 아니고 진짜 진짜 그랬다니까";
        assert_eq!(collapse_repeated_phrases(text, MAX_CONSECUTIVE_REPEATS), text);
    }

    #[test]
    fn a_repeat_that_returns_later_is_not_a_run() {
        // 떨어져서 다시 나오는 것은 연속이 아니다 (§20.6.1과 같은 규칙).
        let text = "네 그렇죠 네 그렇죠 네";
        assert_eq!(collapse_repeated_phrases(text, MAX_CONSECUTIVE_REPEATS), text);
    }

    #[test]
    fn only_whitespace_is_normalized_when_there_is_no_repeat() {
        assert_eq!(collapse_repeated_phrases("  가   나  다 ", MAX_CONSECUTIVE_REPEATS), "가 나 다");
    }

    #[test]
    fn short_input_is_returned_as_is() {
        assert_eq!(collapse_repeated_phrases("네 네", MAX_CONSECUTIVE_REPEATS), "네 네");
        assert_eq!(collapse_repeated_phrases("", MAX_CONSECUTIVE_REPEATS), "");
    }
}
