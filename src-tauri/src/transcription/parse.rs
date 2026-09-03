//! 엔진의 원시 출력을 도메인 값으로 바꾼다 — **단위 변환이 일어나는 단 한 자리다**
//! (ADR-0007 §10 · `phase-prompt/03` 요구 4).
//!
//! ```text
//! whisper-rs 원시 segment (센티초)
//!         │
//!         ▼
//!   이 모듈  ←── 단위 변환은 여기서만 일어난다
//!         │      프로세스 실행도 라이브러리 호출도 없다 → whisper 바이너리·모델 없이 테스트된다
//!         ▼
//!   language · segments[{ start_ms, end_ms, text }] · raw_text
//!         │
//!         ▼      실행 경계(TASK-026)는 원시 값을 그대로 넘기고,
//!   영속성 · 화면   영속성(TASK-027)은 이미 밀리초인 값을 저장하며,
//!                 화면(TASK-030)은 밀리초 → HH:MM:SS 표시만 한다. 아무도 단위를 다시 만지지 않는다.
//! ```
//!
//! ## 입력이 문자열이 아니라 타입인 이유
//!
//! ADR-0007 §2.1이 고른 통합 방식은 **`whisper-rs`**다. sidecar(`whisper-cli`)를 골랐다면
//! 이 모듈의 입력은 *JSON 출력 문자열*이었겠지만, 그 후보는 §12.1에서 탈락했다. 라이브러리는
//! 문자열이 아니라 값을 준다. 그래서 입력은 [`RawTranscription`] — **라이브러리가 준 값을 단위
//! 변환 전 그대로 담은 타입**이다. 파싱할 텍스트가 없으므로 이 모듈에 JSON 파서도 없다.
//!
//! 그 대신 JSON 경로가 가졌을 실패("필드 누락" · "타입이 다른 값")는 타입 경계에서 이렇게 남는다.
//!
//! ```text
//! 필드 누락        language: Option<String> · RawSegment::text: Option<String>
//!                 엔진이 값을 주지 못한 경우를 실행 경계가 지어내지 않고 그대로 넘긴다
//! 타입이 다른 값    타입이 이미 고정되어 있으므로 이 경계에 도달할 수 없다.
//!                 남는 것은 **범위**의 문제(음수 · 뒤집힌 구간 · 넘치는 값)이며 아래가 전부 다룬다
//! ```
//!
//! ## 원시 출력이 기대와 다를 때 — 무엇을 하고 무엇을 하지 않는가
//!
//! 두 가지를 지킨다. **사람이 말한 텍스트는 버리지 않는다. 모르는 시각은 지어내지 않는다.**
//!
//! | 원시 입력 | 처리 | 왜 |
//! | --- | --- | --- |
//! | segment가 하나도 없다 | `Ok` — `segments: []` · `raw_text: ""` | 무음 녹음에서 정상적으로 나온다. 이것이 제품 실패인지는 이 모듈이 정하지 않는다 (TASK-027) |
//! | `text`가 `None` | 그 segment를 버리고 [`AnomalyKind::TextMissing`] | 엔진이 주지 못한 문장을 지어내지 않는다 |
//! | `text`가 비었거나 공백뿐 | 그 segment를 버리고 [`AnomalyKind::BlankText`] | 내용이 없는 구간이다. 남겨 두면 화면에 빈 줄이 쌓인다 |
//! | `text` 앞뒤 공백 | 잘라낸다 | 엔진은 선행 공백을 붙여 낸다. 안쪽 공백은 건드리지 않는다 |
//! | 음수 `start` | `0`으로 접고 [`AnomalyKind::NegativeStart`] | 녹음은 0에서 시작한다. 음수 시각은 가리킬 자리가 없다 |
//! | `end < start` (잘린 마지막 segment 포함) | `end = start`로 접고 [`AnomalyKind::EndBeforeStart`] | **끝 시각을 추정하지 않는다.** 길이 0은 "시작 자리만 안다"는 뜻이고, 텍스트는 그대로 살아남는다 |
//! | 앞 segment보다 이른 시작 | 그대로 두고 [`AnomalyKind::OutOfOrder`] | 순서를 바꾸면 문장이 뒤섞인다. 저장 스키마의 `ordinal`은 **엔진이 낸 순서**다 |
//! | 앞 segment와 겹치는 구간 | 그대로 두고 [`AnomalyKind::Overlap`] | 겹침은 엔진의 chunk 경계에서 나온다. 잘라내면 없던 경계를 만든다 |
//! | 센티초 × 10이 `i64`를 넘친다 | [`Failure`] | 표현할 수 없는 값이다. 조용히 saturate하면 틀린 시각이 **영구히** 저장된다 (INV-2) |
//!
//! **정정은 조용히 일어나지 않는다.** 버리거나 접은 자리는 전부 [`Transcription::anomalies`]에
//! 원시 인덱스와 함께 남는다. 정상 출력에서 이 목록은 비어 있다.
//!
//! 실패는 [`FailureKind::InvalidInput`]이다 — **새 [`FailureKind`]를 만들지 않는다.** 그 enum은
//! `src/ipc/failure.ts`의 union과 1:1이며, 화면을 다루지 않는 이 Task가 계약을 넓히면 frontend가
//! 모르는 종류가 조용히 생긴다 (`audio_input`과 같은 판단이다). `source_data_safe`는 참으로
//! 남는다 — 이 모듈은 파일도 데이터베이스도 건드리지 않는다 (INV-3).

use crate::domain::{Failure, FailureKind};

/// 원시 timestamp를 밀리초로 옮기는 계수. **코드 전체에서 이 한 자리에만 있다** (ADR-0007 §10).
///
/// 근거: **`whisper-rs`의 segment timestamp 단위는 센티초(1/100초)다**
/// (ADR-0007 §10 표 · §4.1 "timestamp 단위 = 센티초" · [E2] 2026-09-03). 저장 스키마가 쓰는
/// 단위는 밀리초이며(`transcript_segments.start_ms` · `end_ms` — ADR-0007 §10 [E1]),
/// 1 센티초 = 10 밀리초다. 그래서 계수는 **10**이다.
///
/// ⚠️ ADR-0007 §14는 이 단위를 **[E2] 기록은 있으나 실측은 UNVERIFIED**로 남겼고,
/// crate를 실제로 추가하는 TASK-026이 실제 값으로 확인한다. 확인 결과가 다르면 **ADR과 이 상수를
/// 함께** 갱신한다 (TASK-031). 추측으로 다른 값을 넣지 않는다 — 근거는 위 두 줄이 전부다.
const MILLISECONDS_PER_CENTISECOND: i64 = 10;

/// 엔진이 낸 segment 하나를 **단위 변환 전 값 그대로** 담은 것.
///
/// 이 타입을 채우는 것은 실행 경계(TASK-026)이고, 그 경계는 값을 옮기기만 한다 —
/// 계산하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSegment {
    /// 엔진이 준 시작 timestamp. **센티초다** ([`MILLISECONDS_PER_CENTISECOND`]).
    pub start_centiseconds: i64,
    /// 엔진이 준 끝 timestamp. **센티초다.**
    pub end_centiseconds: i64,
    /// segment의 텍스트.
    ///
    /// `None`은 **엔진이 이 segment의 텍스트를 주지 못했다**는 뜻이다. 실행 경계는 그때
    /// 문자열을 지어내지 않고 없음을 그대로 넘긴다.
    pub text: Option<String>,
}

/// 엔진이 낸 전사 출력 전체를 단위 변환 전 값 그대로 담은 것.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTranscription {
    /// 엔진이 보고한 언어. 보고하지 않았으면 `None`이다.
    pub language: Option<String>,
    /// 엔진이 낸 순서 그대로의 segment들.
    pub segments: Vec<RawSegment>,
}

/// 정규화가 끝난 segment 하나. 시간 단위는 **밀리초**다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    /// 앞뒤 공백을 잘라낸 텍스트. 비어 있지 않다 — 빈 segment는 살아남지 못한다.
    pub text: String,
}

/// 원시 출력이 기대와 달랐던 자리의 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnomalyKind {
    /// 엔진이 텍스트를 주지 못했다. 그 segment를 버렸다.
    TextMissing,
    /// 텍스트가 비었거나 공백뿐이었다. 그 segment를 버렸다.
    BlankText,
    /// 시작 시각이 음수였다. `0`으로 접었다.
    NegativeStart,
    /// 끝이 시작보다 앞이었다. `end = start`로 접었다. 잘린 마지막 segment가 여기 속한다.
    EndBeforeStart,
    /// 앞 segment보다 이른 시각에서 시작했다. **순서를 바꾸지 않았다.**
    OutOfOrder,
    /// 앞 segment가 끝나기 전에 시작했다. **잘라내지 않았다.**
    Overlap,
}

/// 원시 출력이 기대와 달랐던 자리 하나.
///
/// 이 목록이 있기 때문에 위 표의 처리가 **조용한 흡수가 아니다** — 무엇을 버렸고 무엇을
/// 접었는지가 값으로 남는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anomaly {
    /// 원시 출력에서의 인덱스. 버려진 segment도 이 번호로 가리킬 수 있다.
    pub raw_index: usize,
    pub kind: AnomalyKind,
}

/// 정규화가 끝난 전사 출력. 여기서부터는 어디서도 단위를 다시 만지지 않는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcription {
    /// 엔진이 보고한 언어. 앞뒤 공백을 잘라냈고, 빈 문자열은 `None`이 된다.
    ///
    /// **코드를 해석하거나 바꾸지 않는다** — 엔진이 보고한 값 그대로다.
    pub language: Option<String>,
    /// 엔진이 낸 순서 그대로의 segment들. 시간 단위는 밀리초다.
    pub segments: Vec<TranscriptSegment>,
    /// 살아남은 segment의 텍스트를 이어 붙인 전문.
    ///
    /// 규칙은 하나다 — **각 텍스트를 앞뒤로 다듬어 한 칸 공백으로 잇는다.** 줄바꿈을 넣지
    /// 않는 것은 segment 경계가 문장 경계가 아니라 **엔진의 chunk 경계**이기 때문이다.
    /// 줄을 나누면 원본에 없던 구조를 주장하게 된다.
    pub raw_text: String,
    /// 원시 출력이 기대와 달랐던 자리들. 정상 출력에서는 비어 있다.
    pub anomalies: Vec<Anomaly>,
}

/// 원시 출력을 도메인 값으로 바꾼다.
///
/// **이 함수가 하는 단위 변환은 [`MILLISECONDS_PER_CENTISECOND`] 하나뿐이다.** 원시 출력이
/// 기대와 다를 때의 처리는 이 모듈 문서의 표에 전부 적혀 있고, 그 처리는
/// [`Transcription::anomalies`]에 남는다.
///
/// 실패는 **표현할 수 없는 시간 값** 하나뿐이다. 어떤 입력으로도 panic하지 않는다.
pub fn normalize(raw: RawTranscription) -> Result<Transcription, Failure> {
    let mut segments: Vec<TranscriptSegment> = Vec::with_capacity(raw.segments.len());
    let mut anomalies: Vec<Anomaly> = Vec::new();

    for (raw_index, segment) in raw.segments.into_iter().enumerate() {
        // 텍스트부터 본다. 내용이 없는 segment의 시간은 계산할 필요가 없다.
        let Some(text) = segment.text else {
            anomalies.push(Anomaly {
                raw_index,
                kind: AnomalyKind::TextMissing,
            });
            continue;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            anomalies.push(Anomaly {
                raw_index,
                kind: AnomalyKind::BlankText,
            });
            continue;
        }
        let text = trimmed.to_owned();

        // **단위 변환. 코드 전체에서 이 두 줄뿐이다.**
        let mut start_ms = to_milliseconds(segment.start_centiseconds, raw_index, "start")?;
        let mut end_ms = to_milliseconds(segment.end_centiseconds, raw_index, "end")?;

        if start_ms < 0 {
            anomalies.push(Anomaly {
                raw_index,
                kind: AnomalyKind::NegativeStart,
            });
            start_ms = 0;
        }
        if end_ms < start_ms {
            // 끝을 추정하지 않는다. 길이 0은 "시작 자리만 안다"는 뜻이다.
            anomalies.push(Anomaly {
                raw_index,
                kind: AnomalyKind::EndBeforeStart,
            });
            end_ms = start_ms;
        }

        // 순서와 겹침은 **살아남은 직전 segment**를 기준으로 본다. 버려진 빈 segment가
        // 기준이 되면 있지도 않은 겹침을 보고하게 된다.
        if let Some(previous) = segments.last() {
            if start_ms < previous.start_ms {
                anomalies.push(Anomaly {
                    raw_index,
                    kind: AnomalyKind::OutOfOrder,
                });
            } else if start_ms < previous.end_ms {
                anomalies.push(Anomaly {
                    raw_index,
                    kind: AnomalyKind::Overlap,
                });
            }
        }

        segments.push(TranscriptSegment {
            start_ms,
            end_ms,
            text,
        });
    }

    let raw_text = join_text(&segments);

    Ok(Transcription {
        language: normalize_language(raw.language),
        segments,
        raw_text,
        anomalies,
    })
}

/// 센티초를 밀리초로. **정규화 경계의 유일한 계산이다** (ADR-0007 §10).
///
/// 넘치면 값을 접지 않고 실패로 나간다 — saturate한 시각은 조용히 틀린 채로 영구히 저장된다.
fn to_milliseconds(centiseconds: i64, raw_index: usize, field: &str) -> Result<i64, Failure> {
    centiseconds
        .checked_mul(MILLISECONDS_PER_CENTISECOND)
        .ok_or_else(|| {
            Failure::permanent(
                FailureKind::InvalidInput,
                "전사 결과의 시간 값이 다룰 수 있는 범위를 넘었다",
            )
            .with_detail(format!(
                "segment[{raw_index}].{field}={centiseconds}cs × {MILLISECONDS_PER_CENTISECOND} overflows i64"
            ))
        })
}

/// 살아남은 segment의 텍스트를 한 칸 공백으로 잇는다 ([`Transcription::raw_text`]).
///
/// 버려진 segment는 여기에 도달하지 않으므로 빈 자리가 이중 공백을 만들지 않는다.
fn join_text(segments: &[TranscriptSegment]) -> String {
    let mut raw_text = String::new();
    for segment in segments {
        if !raw_text.is_empty() {
            raw_text.push(' ');
        }
        raw_text.push_str(&segment.text);
    }
    raw_text
}

/// 언어 코드를 다듬는다. 값을 해석하지도 바꾸지도 않는다 — 공백만 잘라낸다.
fn normalize_language(language: Option<String>) -> Option<String> {
    let language = language?;
    let trimmed = language.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() == language.len() {
        return Some(language);
    }
    Some(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 원시 segment 하나. 텍스트가 있는 정상적인 경우다.
    fn raw(start_centiseconds: i64, end_centiseconds: i64, text: &str) -> RawSegment {
        RawSegment {
            start_centiseconds,
            end_centiseconds,
            text: Some(text.to_owned()),
        }
    }

    /// 언어를 보고하지 않은 원시 출력.
    fn output(segments: Vec<RawSegment>) -> RawTranscription {
        RawTranscription {
            language: None,
            segments,
        }
    }

    #[test]
    fn one_minute_thirty_seconds_is_ninety_thousand_milliseconds() {
        // **이 테스트는 변환식을 되풀이하지 않는다.** 아래 숫자는 손으로 계산해 박아 넣은
        // 값이며, 코드가 × 1 · × 100 · ÷ 10 어느 쪽으로 어긋나도 반드시 깨진다.
        //
        //   1분 30초 = 90초 = 9000 센티초 = 90000 밀리초
        //
        // ADR-0007 §10이 요구하는 바로 그 값이다.
        let normalized = normalize(output(vec![raw(9_000, 9_250, "여기까지가 1분 30초 지점이다")]))
            .expect("정상 출력은 정규화된다");

        let segment = &normalized.segments[0];
        assert_eq!(segment.start_ms, 90_000, "1분 30초는 90000 밀리초다");
        assert_ne!(segment.start_ms, 9_000, "센티초를 그대로 두면 ×10 어긋난다");
        assert_ne!(segment.start_ms, 900_000, "×100은 100배 어긋난 값이다");
        assert_eq!(segment.end_ms, 92_500, "1분 32.5초");
    }

    #[test]
    fn known_timestamps_map_to_literal_milliseconds() {
        // 자릿수를 옮겨 가며 못 박는다. 손으로 계산한 값만 적었다.
        let cases: [(i64, i64); 6] = [
            (0, 0),               // 시작
            (1, 10),              // 1 센티초 = 10 밀리초
            (50, 500),            // 0.5초
            (100, 1_000),         // 1초
            (12_345, 123_450),    // 123.45초
            (360_000, 3_600_000), // 1시간
        ];

        for (centiseconds, expected_ms) in cases {
            let normalized = normalize(output(vec![raw(centiseconds, centiseconds, "값")]))
                .expect("정규화된다");

            assert_eq!(
                normalized.segments[0].start_ms, expected_ms,
                "{centiseconds}센티초는 {expected_ms}밀리초여야 한다"
            );
        }
    }

    #[test]
    fn an_hour_long_recording_does_not_drift_at_the_end() {
        // 1시간 지점에서 ×10 어긋나면 6분이나 10시간이 된다. 둘 다 아니다.
        let normalized = normalize(output(vec![raw(360_000, 360_150, "한 시간 지점")]))
            .expect("정규화된다");

        assert_eq!(normalized.segments[0].start_ms, 3_600_000);
        assert_ne!(normalized.segments[0].start_ms, 360_000);
        assert_ne!(normalized.segments[0].start_ms, 36_000_000);
        assert_eq!(normalized.segments[0].end_ms, 3_601_500);
    }

    #[test]
    fn a_normal_output_becomes_the_domain_value_without_complaint() {
        let normalized = normalize(RawTranscription {
            language: Some("ko".to_owned()),
            segments: vec![
                raw(0, 250, " 안녕하세요"),
                raw(250, 480, " 오늘 회의를 시작하겠습니다"),
            ],
        })
        .expect("정상 출력은 정규화된다");

        assert_eq!(normalized.language.as_deref(), Some("ko"));
        assert_eq!(
            normalized.segments,
            vec![
                TranscriptSegment {
                    start_ms: 0,
                    end_ms: 2_500,
                    text: "안녕하세요".to_owned(),
                },
                TranscriptSegment {
                    start_ms: 2_500,
                    end_ms: 4_800,
                    text: "오늘 회의를 시작하겠습니다".to_owned(),
                },
            ]
        );
        assert_eq!(normalized.raw_text, "안녕하세요 오늘 회의를 시작하겠습니다");
        assert!(
            normalized.anomalies.is_empty(),
            "정상 출력에서는 정정할 것이 없다: {:?}",
            normalized.anomalies
        );
    }

    #[test]
    fn raw_text_is_built_from_the_surviving_segments_by_one_rule() {
        // 규칙: 각 텍스트를 다듬어 **한 칸 공백**으로 잇는다. 버려진 segment는 자리를 남기지
        // 않는다 — 이중 공백도, 앞뒤 공백도 생기지 않는다.
        let normalized = normalize(output(vec![
            raw(0, 100, "  첫 문장  "),
            raw(100, 200, "   "),  // 공백뿐 — 버려진다
            raw(200, 300, "둘째 문장"),
            RawSegment {
                start_centiseconds: 300,
                end_centiseconds: 400,
                text: None, // 없음 — 버려진다
            },
            raw(400, 500, " 셋째 문장"),
        ]))
        .expect("정규화된다");

        assert_eq!(normalized.raw_text, "첫 문장 둘째 문장 셋째 문장");
        assert!(!normalized.raw_text.contains("  "), "이중 공백이 없다");
        assert_eq!(normalized.raw_text.trim(), normalized.raw_text);
        assert_eq!(normalized.segments.len(), 3, "빈 segment는 살아남지 않는다");
    }

    #[test]
    fn the_same_input_always_produces_the_same_output() {
        let input = || {
            RawTranscription {
                language: Some("ko".to_owned()),
                segments: vec![
                    raw(0, 100, " 하나"),
                    raw(100, 100, ""),
                    raw(90, 80, "뒤집힌 구간"),
                    raw(500, 600, "둘"),
                ],
            }
        };

        let first = normalize(input()).expect("정규화된다");
        let second = normalize(input()).expect("정규화된다");

        assert_eq!(first, second, "같은 입력은 언제나 같은 값을 만든다");
        assert_eq!(first.raw_text, second.raw_text);
        assert_eq!(first.anomalies, second.anomalies);
    }

    #[test]
    fn inner_whitespace_survives_while_the_edges_are_trimmed() {
        let normalized =
            normalize(output(vec![raw(0, 100, "  두 낱말  사이 ")])).expect("정규화된다");

        assert_eq!(normalized.segments[0].text, "두 낱말  사이");
    }

    #[test]
    fn an_empty_output_is_an_empty_transcription_rather_than_a_panic() {
        // 무음 녹음에서 정상적으로 나오는 모양이다. 이것이 제품 실패인지는 이 모듈이 정하지
        // 않는다 — 정의된 값으로 돌려주고 판단은 호출자에게 남긴다.
        let normalized = normalize(output(Vec::new())).expect("빈 출력도 정규화된다");

        assert!(normalized.segments.is_empty());
        assert_eq!(normalized.raw_text, "");
        assert_eq!(normalized.language, None);
        assert!(normalized.anomalies.is_empty(), "버린 것이 없으면 남길 것도 없다");
    }

    #[test]
    fn an_output_whose_segments_are_all_blank_yields_no_text_and_says_why() {
        let normalized = normalize(output(vec![
            raw(0, 100, ""),
            raw(100, 200, "   \t\n "),
            RawSegment {
                start_centiseconds: 200,
                end_centiseconds: 300,
                text: None,
            },
        ]))
        .expect("전부 빈 출력도 정규화된다");

        assert!(normalized.segments.is_empty());
        assert_eq!(normalized.raw_text, "");
        assert_eq!(
            normalized.anomalies,
            vec![
                Anomaly {
                    raw_index: 0,
                    kind: AnomalyKind::BlankText
                },
                Anomaly {
                    raw_index: 1,
                    kind: AnomalyKind::BlankText
                },
                Anomaly {
                    raw_index: 2,
                    kind: AnomalyKind::TextMissing
                },
            ],
            "무엇을 왜 버렸는지가 남는다 — 조용히 사라지지 않는다"
        );
    }

    #[test]
    fn a_segment_whose_end_precedes_its_start_keeps_its_text_and_collapses_to_a_point() {
        let normalized = normalize(output(vec![raw(1_000, 200, "여기서 한 말")]))
            .expect("뒤집힌 구간도 정규화된다");

        let segment = &normalized.segments[0];
        assert_eq!(segment.start_ms, 10_000, "시작 시각은 그대로 변환된다");
        assert_eq!(segment.end_ms, 10_000, "끝을 추정하지 않고 시작에 맞춘다");
        assert_eq!(segment.text, "여기서 한 말", "텍스트는 버리지 않는다");
        assert_eq!(
            normalized.anomalies,
            vec![Anomaly {
                raw_index: 0,
                kind: AnomalyKind::EndBeforeStart
            }]
        );
    }

    #[test]
    fn a_truncated_last_segment_does_not_take_the_rest_of_the_transcript_with_it() {
        // 엔진이 마지막 segment의 끝을 채우지 못한 모양(끝이 0으로 남았다). 앞의 정상 segment는
        // 그대로 살아남아야 한다 — 한 자리가 잘렸다고 한 시간짜리 전사를 통째로 버리지 않는다.
        let normalized = normalize(output(vec![
            raw(0, 250, "온전한 첫 문장"),
            raw(250, 480, "온전한 둘째 문장"),
            raw(480, 0, "잘린 마지막 문장"),
        ]))
        .expect("잘린 마지막 segment가 있어도 정규화된다");

        assert_eq!(normalized.segments.len(), 3);
        assert_eq!(normalized.segments[1].end_ms, 4_800, "앞 문장은 그대로다");
        assert_eq!(normalized.segments[2].start_ms, 4_800);
        assert_eq!(normalized.segments[2].end_ms, 4_800, "끝을 지어내지 않는다");
        assert_eq!(normalized.segments[2].text, "잘린 마지막 문장");
        assert_eq!(
            normalized.raw_text,
            "온전한 첫 문장 온전한 둘째 문장 잘린 마지막 문장"
        );
        assert_eq!(
            normalized.anomalies,
            vec![Anomaly {
                raw_index: 2,
                kind: AnomalyKind::EndBeforeStart
            }]
        );
    }

    #[test]
    fn a_negative_timestamp_is_folded_to_the_start_of_the_recording() {
        let normalized = normalize(output(vec![
            raw(-5, 100, "음수에서 시작했다고 주장하는 문장"),
            raw(-30, -20, "양쪽 다 음수다"),
        ]))
        .expect("음수 timestamp에도 정규화된다");

        assert_eq!(normalized.segments[0].start_ms, 0, "녹음은 0에서 시작한다");
        assert_eq!(normalized.segments[0].end_ms, 1_000);
        assert_eq!(normalized.segments[1].start_ms, 0);
        assert_eq!(normalized.segments[1].end_ms, 0, "음수 끝도 0 아래로 내려가지 않는다");
        assert!(normalized.segments.iter().all(|segment| segment.start_ms >= 0
            && segment.end_ms >= segment.start_ms));
        assert!(normalized
            .anomalies
            .iter()
            .any(|anomaly| anomaly.kind == AnomalyKind::NegativeStart));
    }

    #[test]
    fn out_of_order_segments_keep_the_engines_order_and_are_reported() {
        // 순서를 바꾸면 문장이 뒤섞인다. `ordinal`은 엔진이 낸 순서이고, 이 모듈은 그것을
        // 재배열하지 않는다.
        let normalized = normalize(output(vec![
            raw(1_000, 1_100, "나중 자리의 문장"),
            raw(100, 200, "앞 자리의 문장"),
        ]))
        .expect("역순 출력도 정규화된다");

        assert_eq!(normalized.segments[0].text, "나중 자리의 문장");
        assert_eq!(normalized.segments[0].start_ms, 10_000);
        assert_eq!(normalized.segments[1].text, "앞 자리의 문장");
        assert_eq!(normalized.segments[1].start_ms, 1_000);
        assert_eq!(
            normalized.raw_text, "나중 자리의 문장 앞 자리의 문장",
            "전문도 엔진의 순서를 따른다"
        );
        assert_eq!(
            normalized.anomalies,
            vec![Anomaly {
                raw_index: 1,
                kind: AnomalyKind::OutOfOrder
            }]
        );
    }

    #[test]
    fn overlapping_segments_are_kept_intact_and_reported() {
        // 겹침은 엔진의 chunk 경계에서 나온다. 잘라내면 원본에 없던 경계를 만든다.
        let normalized = normalize(output(vec![
            raw(0, 500, "앞 chunk의 끝자락"),
            raw(450, 900, "뒤 chunk의 첫머리"),
        ]))
        .expect("겹치는 출력도 정규화된다");

        assert_eq!(normalized.segments[0].end_ms, 5_000);
        assert_eq!(normalized.segments[1].start_ms, 4_500, "겹침을 잘라내지 않는다");
        assert_eq!(normalized.segments[1].end_ms, 9_000);
        assert_eq!(
            normalized.anomalies,
            vec![Anomaly {
                raw_index: 1,
                kind: AnomalyKind::Overlap
            }]
        );
    }

    #[test]
    fn a_dropped_segment_does_not_become_the_yardstick_for_the_next_one() {
        // 버려진 빈 segment가 기준이 되면 있지도 않은 겹침을 보고하게 된다.
        let normalized = normalize(output(vec![
            raw(0, 100, "첫 문장"),
            raw(5_000, 6_000, "   "), // 버려진다. 이 시각은 기준이 되면 안 된다
            raw(100, 200, "둘째 문장"),
        ]))
        .expect("정규화된다");

        assert_eq!(normalized.segments.len(), 2);
        assert_eq!(
            normalized.anomalies,
            vec![Anomaly {
                raw_index: 1,
                kind: AnomalyKind::BlankText
            }],
            "버려진 segment 때문에 없던 역순/겹침이 보고되지 않는다"
        );
    }

    #[test]
    fn a_timestamp_too_large_to_convert_is_a_defined_failure_rather_than_a_wrong_number() {
        // saturate하면 조용히 틀린 시각이 영구히 저장된다 (INV-2). 값을 접지 않고 실패한다.
        let failure = normalize(output(vec![raw(i64::MAX, i64::MAX, "넘치는 값")]))
            .expect_err("표현할 수 없는 시간 값은 실패다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.source_data_safe, "이 모듈은 아무것도 건드리지 않는다 (INV-3)");
        assert!(!failure.retryable, "같은 값을 다시 넣어도 결과는 같다");
        let detail = failure.detail.expect("어느 자리의 어떤 값인지가 남는다");
        assert!(detail.contains("segment[0].start"), "{detail}");
    }

    #[test]
    fn the_language_the_engine_reported_is_passed_through_untouched() {
        let reported = normalize(RawTranscription {
            language: Some("ko".to_owned()),
            segments: Vec::new(),
        })
        .expect("정규화된다");
        assert_eq!(reported.language.as_deref(), Some("ko"));

        let padded = normalize(RawTranscription {
            language: Some("  en  ".to_owned()),
            segments: Vec::new(),
        })
        .expect("정규화된다");
        assert_eq!(padded.language.as_deref(), Some("en"), "공백만 잘라낸다");
    }

    #[test]
    fn a_missing_or_blank_language_becomes_none_rather_than_an_empty_string() {
        for reported in [None, Some(String::new()), Some("   ".to_owned())] {
            let normalized = normalize(RawTranscription {
                language: reported.clone(),
                segments: Vec::new(),
            })
            .expect("정규화된다");

            assert_eq!(
                normalized.language, None,
                "{reported:?}는 '언어를 모른다'와 같다"
            );
        }
    }

    #[test]
    fn no_hostile_output_makes_this_module_panic() {
        // AC6의 요구는 "정의된 처리"이지 특정 결과가 아니다. 한자리에서 확인한다 —
        // 어떤 입력도 이 모듈을 무너뜨리지 못한다.
        let cases: [(&str, RawTranscription); 8] = [
            ("빈 출력", output(Vec::new())),
            (
                "언어만 있고 segment가 없다",
                RawTranscription {
                    language: Some("ko".to_owned()),
                    segments: Vec::new(),
                },
            ),
            ("텍스트가 전부 없음", output(vec![
                RawSegment { start_centiseconds: 0, end_centiseconds: 0, text: None },
            ])),
            ("공백뿐인 텍스트", output(vec![raw(0, 0, "\u{00a0} \t\r\n")])),
            ("음수 timestamp", output(vec![raw(i64::MIN, i64::MIN, "아래로 넘친다")])),
            ("뒤집힌 구간", output(vec![raw(900, 100, "뒤집혔다")])),
            (
                "역순 + 겹침 + 잘림이 한꺼번에",
                output(vec![
                    raw(1_000, 2_000, "가"),
                    raw(500, 1_500, "나"),
                    raw(1_400, 0, "다"),
                ]),
            ),
            ("넘치는 값", output(vec![raw(i64::MAX, 0, "넘친다")])),
        ];

        for (label, input) in cases {
            match normalize(input) {
                Ok(normalized) => {
                    // 성공했다면 결과는 언제나 성립하는 값이어야 한다.
                    for segment in &normalized.segments {
                        assert!(segment.start_ms >= 0, "{label}: 음수 시각이 남았다");
                        assert!(
                            segment.end_ms >= segment.start_ms,
                            "{label}: 끝이 시작보다 앞이다"
                        );
                        assert!(!segment.text.is_empty(), "{label}: 빈 텍스트가 남았다");
                    }
                }
                Err(failure) => {
                    assert!(failure.source_data_safe, "{label}: 원본은 언제나 안전하다");
                }
            }
        }
    }

    #[test]
    fn this_module_never_runs_a_process_or_calls_the_whisper_library() {
        // ADR-0007 §10 · `phase-prompt/03` §18: 이 모듈은 whisper 바이너리도 모델도 없이
        // 테스트된다. 그 성질은 주석이 아니라 **여기 없는 코드**가 만든다 — 실행 경계가
        // 나중에(TASK-026) 이 파일로 흘러들지 않도록 소스 자체를 검사한다.
        //
        // 찾는 문자열은 조각을 이어 붙여 만든다. 그대로 적으면 이 테스트가 자기 자신을
        // 발견해 버린다.
        let source = include_str!("parse.rs");
        let forbidden = [
            ["std::", "process"].concat(),
            ["Command", "::new"].concat(),
            ["whisper", "_rs"].concat(),
            ["use ", "std::fs"].concat(),
        ];

        for needle in forbidden {
            assert!(
                !source.contains(&needle),
                "정규화 모듈에 실행/외부 호출이 들어왔다: {needle}"
            );
        }

        // 이 파일이 실제로 읽혔는지 확인한다 — 빈 문자열이면 위 검사는 아무것도 막지 못한다.
        assert!(source.contains("MILLISECONDS_PER_CENTISECOND"));
    }

    #[test]
    fn the_conversion_factor_lives_in_exactly_one_place() {
        // 계수가 두 자리에 생기면 한쪽만 고쳐질 수 있다 (ADR-0007 §10의 정규화 경계).
        // 상수 정의 한 줄과, 변환이 일어나는 `to_milliseconds` 한 줄뿐이다.
        let source = include_str!("parse.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("테스트 앞의 코드가 있어야 한다");

        assert_eq!(
            production.matches("checked_mul").count(),
            1,
            "단위 변환은 한 자리에서만 일어난다"
        );
        assert_eq!(
            production
                .matches("const MILLISECONDS_PER_CENTISECOND: i64 = 10;")
                .count(),
            1,
            "계수는 상수 한 자리에만 있다"
        );
    }
}
