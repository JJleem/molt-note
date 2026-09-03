//! 실제 whisper를 실행하는 경계 (PRODUCT-SPEC §3.1의 `TranscriptionRunner` 자리 · §13).
//!
//! 이 모듈에는 **계약만** 있다. 계약을 실제로 이행하는 것은 두 구현이다.
//!
//! ```text
//!                     ┌──────────────────────────────┐
//! 파생 입력 ─────────→ │  TranscriptionEngine (trait) │ ─→ 원시 출력(단위 변환 전)
//! (audio_input.rs)     └──────────────────────────────┘        └→ parse.rs가 정규화한다
//!                        │                        │
//!                        │                        └ testing::StubEngine
//!                        └ whisper::WhisperEngine    실제 whisper 없이 테스트가 쓴다
//!                          ADR-0007이 고른 실제 구현
//! ```
//!
//! **trait이 있는 이유는 플랫폼이 아니라 테스트다** (ADR-0007 §11). 실제 whisper 바이너리와
//! 모델 없이 검증하기 위한 두 번째 구현이 지금 실재한다 (§18). "언젠가 Windows에서 다를
//! 것"은 근거가 아니며, 이 Phase에서 플랫폼이 실제로 갈리는 지점은 **모델 파일 위치** 하나다.
//! 그것은 이미 있는 경계가 처리한다 ([`crate::platform::app_data_dir::AppDataDirectory::models_dir`]
//! · INV-10). 그래서 이 모듈에도 [`super::model`]에도 `cfg(target_os)` 분기가 없다.
//!
//! ## 오디오는 이 경계 밖으로 나가지 않는다 (§12 · INV-6)
//!
//! 엔진은 **같은 프로세스 안의 라이브러리**다 (ADR-0007 §2 — `whisper-rs`). 오디오도 전사
//! 결과도 네트워크로 보내지 않으며, 이 경계에는 그런 경로를 만들 수 있는 의존성 자체가 없다.
//! 원격 전사 API는 이 제품의 선택지가 아니다.
//!
//! ## 네 가지 실패는 서로 구분된다 (§13)
//!
//! | 실패 | [`FailureKind`] | 어디서 나는가 | 재시도 |
//! | --- | --- | --- | --- |
//! | 모델 파일이 없다 | `TranscriptionModelMissing` | [`super::model::resolve`] | ✗ 모델을 먼저 둬야 한다 |
//! | 모델을 읽을 수 없거나 지원하지 않는다 | `TranscriptionModelUnusable` | [`super::model::resolve`] · 엔진의 모델 적재 | ✗ 다른 모델을 골라야 한다 |
//! | 엔진이 실행되지 못했거나 비정상 종료했다 | `TranscriptionEngineFailed` | 엔진 실행 | ✓ |
//! | 출력이 없거나 해석할 수 없다 | `TranscriptionOutputUnusable` | [`ensure_usable`] · 출력 읽기 | ✗ 같은 입력은 같은 출력을 낸다 |
//!
//! **넷 다 `source_data_safe`를 내리지 않는다.** 이 경계는 원본 오디오도 Recording 레코드도
//! 건드리지 않는다 — 읽는 것은 메모리 위의 파생 입력과 모델 파일뿐이다 (INV-1 · INV-3).

use crate::domain::{Failure, FailureKind};

use super::audio_input::TranscriptionInput;
use super::model::ModelFile;
use super::parse::RawTranscription;

/// 전사 엔진 하나를 실행하는 계약.
///
/// 구현은 **값을 옮기기만 한다 — 계산하지 않는다.** timestamp 단위 변환은 이 경계가 아니라
/// [`super::parse`] 한 곳에서만 일어나므로, 여기서 돌려주는 것은 엔진이 낸 원시 값 그대로다
/// (ADR-0007 §10).
///
/// `Send + Sync`인 것은 전사가 UI를 막지 않고 다른 스레드에서 도는 자리에 놓이기 때문이다
/// (TASK-028). 여기서 스레드를 만들지는 않는다 — 그 결정은 이 경계의 몫이 아니다.
pub trait TranscriptionEngine: Send + Sync {
    /// Transcript에 남길 엔진 provenance (§7 · ADR-0007 §8.2.4).
    ///
    /// 무엇으로 만든 전사인지는 나중에 되짚을 수 있어야 한다 — 모델과 엔진이 바뀌면 같은
    /// 오디오에서 다른 Transcript가 나온다.
    fn engine_id(&self) -> String;

    /// 파생 입력 하나를 모델 하나로 전사한다.
    ///
    /// 입력은 이미 16 kHz mono f32다 ([`TranscriptionInput`]) — 이 경계는 오디오를 다시
    /// 변환하지 않는다. 모델은 이미 해석이 끝난 파일이다 ([`ModelFile`]) — 이 경계는 경로를
    /// 다시 짓지 않는다.
    fn transcribe(
        &self,
        input: &TranscriptionInput,
        model: &ModelFile,
    ) -> Result<RawTranscription, Failure>;
}

/// 엔진 구현이 공통으로 지키는 **출력 계약**.
///
/// 실제 엔진도 test double도 돌려주기 전에 이것을 통과한다. double이 계약을 우회하면
/// 테스트가 검증하는 것은 제품이 아니라 double이 된다.
///
/// 규칙은 하나다 — **텍스트를 가진 segment가 하나도 없으면 그것은 전사 결과가 아니다.**
/// 무음 녹음도 여기 포함된다. 조용히 빈 Transcript를 만들지 않는 것은 Transcript가 immutable
/// 하기 때문이다 (INV-2) — 한 번 저장하면 그 빈 결과가 영구히 남고, 사용자는 전사가 됐다고
/// 믿는다. 대신 §13의 실패로 돌려주면 원본은 그대로이고 다시 시도할 수 있다.
///
/// 그 밖의 이상(음수 시각 · 뒤집힌 구간 · 겹침)은 여기서 판정하지 않는다. 그것은
/// [`super::parse`]가 anomaly로 남기며 텍스트는 살린다.
pub fn ensure_usable(raw: RawTranscription) -> Result<RawTranscription, Failure> {
    let has_text = raw
        .segments
        .iter()
        .any(|segment| segment.text.as_deref().is_some_and(|t| !t.trim().is_empty()));

    if has_text {
        Ok(raw)
    } else {
        Err(output_unusable("전사 결과가 비어 있다").with_detail(format!(
            "segments={} 중 텍스트를 가진 것이 없다",
            raw.segments.len()
        )))
    }
}

/// 모델 파일이 있어야 할 자리에 없다 (§13 `모델 파일 없음`).
///
/// **재시도 대상이 아니다.** 같은 상태로 다시 실행하면 같은 결과다 — 사용자가 모델을 먼저
/// 구해 둬야 한다. 그 사실을 `retryable: false`로 말하는 것이 §13의 세 질문 중 하나에 대한
/// 답이다.
pub fn model_missing(message: impl Into<String>) -> Failure {
    Failure::permanent(FailureKind::TranscriptionModelMissing, message)
}

/// 모델 파일은 있지만 읽을 수 없거나 엔진이 지원하지 않는다 (§13 `unsupported whisper model`).
///
/// 없는 것과 구분한다 — 사용자가 할 일이 다르다. 하나는 모델을 구해 오는 것이고 다른 하나는
/// 다른 모델을 고르는 것이다.
pub fn model_unusable(message: impl Into<String>) -> Failure {
    Failure::permanent(FailureKind::TranscriptionModelUnusable, message)
}

/// 엔진이 실행되지 못했거나 비정상 종료했다 (§13 `transcription process failure`).
///
/// **재시도할 수 있다.** 메모리 · 자원 · 일시적 조건 때문에 일어날 수 있고, 원본 오디오가
/// 그대로 남아 있으므로 같은 입력을 다시 만들 수 있다 (INV-1).
pub fn engine_failed(message: impl Into<String>) -> Failure {
    Failure::retryable(FailureKind::TranscriptionEngineFailed, message)
}

/// 엔진은 끝났지만 출력이 없거나 그 출력을 전사 결과로 해석할 수 없다.
///
/// 실행 실패와 구분한다 — 엔진은 정상적으로 끝났다. 같은 오디오와 같은 모델은 같은 출력을
/// 내므로 그대로 다시 시도할 이유가 없다. 사용자가 할 수 있는 일은 다른 모델을 고르거나 다른
/// 녹음을 쓰는 것이다.
pub fn output_unusable(message: impl Into<String>) -> Failure {
    Failure::permanent(FailureKind::TranscriptionOutputUnusable, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::parse::RawSegment;

    fn segment(text: Option<&str>) -> RawSegment {
        RawSegment {
            start_centiseconds: 0,
            end_centiseconds: 100,
            text: text.map(str::to_owned),
        }
    }

    #[test]
    fn output_with_text_passes_through_untouched() {
        // 경계는 값을 옮기기만 한다 — 단위도 텍스트도 손대지 않는다 (ADR-0007 §10).
        let raw = RawTranscription {
            language: Some("ko".to_owned()),
            segments: vec![RawSegment {
                start_centiseconds: 9_000,
                end_centiseconds: 9_150,
                text: Some(" 그러면 이번에는 ".to_owned()),
            }],
        };

        let passed = ensure_usable(raw.clone()).expect("텍스트가 있으면 통과해야 한다");

        assert_eq!(passed, raw, "출력 계약이 값을 바꾸지 않는다");
    }

    #[test]
    fn an_engine_that_produced_nothing_is_a_distinct_product_failure() {
        let failure = ensure_usable(RawTranscription {
            language: None,
            segments: Vec::new(),
        })
        .expect_err("빈 출력은 전사 결과가 아니다");

        assert_eq!(failure.kind, FailureKind::TranscriptionOutputUnusable);
        assert!(!failure.retryable, "같은 입력은 같은 출력을 낸다");
        assert!(failure.source_data_safe, "원본은 건드리지 않았다 (INV-3)");
    }

    #[test]
    fn segments_without_usable_text_are_the_same_as_no_output() {
        // 엔진이 segment를 냈지만 전부 비었거나 공백뿐이면 남는 전사가 없다. 그것을 빈
        // Transcript로 저장하면 되돌릴 수 없다 (INV-2).
        for empty in [
            vec![segment(None)],
            vec![segment(Some(""))],
            vec![segment(Some("   \n\t "))],
            vec![segment(None), segment(Some(" "))],
        ] {
            let failure = ensure_usable(RawTranscription {
                language: Some("ko".to_owned()),
                segments: empty,
            })
            .expect_err("텍스트가 없는 출력은 실패다");

            assert_eq!(failure.kind, FailureKind::TranscriptionOutputUnusable);
        }
    }

    #[test]
    fn one_usable_segment_is_enough() {
        let raw = RawTranscription {
            language: None,
            segments: vec![segment(None), segment(Some("들린 문장")), segment(Some(""))],
        };

        assert!(
            ensure_usable(raw).is_ok(),
            "일부가 비었다는 이유로 나머지를 버리지 않는다 — 그 판단은 parse의 몫이다"
        );
    }

    #[test]
    fn the_four_transcription_failures_never_collapse_into_one() {
        // §13: 사용자가 할 수 있는 일이 넷 다 다르다. 종류가 겹치면 화면이 구분해 안내할 수 없다.
        let failures = [
            model_missing("모델이 없다"),
            model_unusable("모델을 읽을 수 없다"),
            engine_failed("엔진이 죽었다"),
            output_unusable("출력이 없다"),
        ];

        let mut seen: Vec<&str> = Vec::new();
        for failure in &failures {
            let kind = failure.kind.as_str();
            assert!(!seen.contains(&kind), "실패 종류가 겹친다: {kind}");
            seen.push(kind);

            // 어떤 전사 실패도 원본을 훼손하지 않는다 (INV-1 · INV-3).
            assert!(failure.source_data_safe, "{kind}");
        }
    }

    #[test]
    fn only_the_engine_failure_says_retrying_is_worth_it() {
        assert!(!model_missing("").retryable);
        assert!(!model_unusable("").retryable);
        assert!(engine_failed("").retryable);
        assert!(!output_unusable("").retryable);
    }
}
