//! ADR-0007이 고른 실제 구현 — `whisper-rs`로 whisper.cpp를 **이 프로세스 안에서** 실행한다.
//!
//! ```text
//! 16 kHz mono f32 (메모리)        모델 파일 하나 (사용자가 둔다)
//!         └──────────┬──────────────────┘
//!                    ▼
//!        whisper-rs → 앱 바이너리에 링크된 whisper.cpp
//!                    ▼
//!        원시 segment (센티초 · 단위 변환 없이 그대로 넘긴다 → parse.rs)
//! ```
//!
//! **배포물에 들어가는 실행 파일이 없다** (ADR-0007 §2 · §4.2). sidecar를 택하지 않았으므로
//! 이 파일에는 프로세스 실행도, `tauri.conf.json`의 `bundle.externalBin`도, target triple
//! 파일명 규약(`-aarch64-apple-darwin`)도, shell 권한도 없다. **해석할 sidecar 경로가 없으니
//! `SidecarResolver`도 없다** (§7 · §20.6 — 쓰지 않는 추상화를 미리 만들지 않는다).
//! 이 Phase에서 플랫폼이 갈리는 지점은 모델 파일 위치 하나이며 그것은 [`super::model`]과
//! [`crate::platform::app_data_dir`]가 이미 처리한다. 그래서 이 파일에도 `cfg(target_os)`가
//! 없다.
//!
//! **오디오는 이 프로세스 밖으로 나가지 않는다** (§12 · INV-6). 라이브러리 호출 하나이며
//! 네트워크도, 자식 프로세스도, 임시 파일도 없다.
//!
//! ## 여기 쓰인 API는 문서가 아니라 컴파일러가 확인한 것이다
//!
//! `whisper-rs` 0.16.0 (`whisper-rs-sys` 0.15.0)에 대해 실제로 확인한 시그니처다.
//!
//! ```text
//! WhisperState::full_n_segments()          -> i32
//! WhisperState::get_segment(i32)           -> Option<WhisperSegment<'_>>
//! WhisperSegment::start_timestamp()        -> i64      ← 단위는 아래를 볼 것
//! WhisperSegment::end_timestamp()          -> i64
//! WhisperSegment::to_str()                 -> Result<&str, WhisperError>
//! WhisperState::full_lang_id_from_state()  -> i32
//! ```
//!
//! ⚠️ **타입이 `i64`라는 것과 그 값이 센티초라는 것은 다른 진술이다.** 단위는 실제 추론을
//! 한 번 돌려야 드러나며 이 Phase의 자동 검증은 그것을 하지 않는다 (PRODUCT-SPEC §14.4.3의
//! 운영자 smoke test). 그래서 이 파일은 값을 **그대로** 넘기고, 단위에 대한 가정은
//! [`super::parse`]의 계수 한 자리에만 있다 (ADR-0007 §10 · §14).
//!
//! ## 실제 whisper 없이 도는 자동 검증과의 관계
//!
//! 이 파일의 코드 경로는 **모델 파일이 있어야만** 의미가 있다. Gate는 모델을 두지 않으므로
//! 이 구현을 실행하지 않는다 — 자동 검증은 [`super::testing::StubEngine`]로 하고, 실제 추론은
//! 운영자의 smoke test가 한 번 수행한다 (PRODUCT-SPEC §14.4.3 · TASK-031).
//! **그래도 이 파일은 Gate가 컴파일한다** — 엔진이 `cargo build`의 산출물 안에 들어가는 것이
//! ADR-0007 §4.2가 B를 고른 이유이기 때문이다. 사람이 옮겨 둔 파일이 없어도 저장소가 엔진을
//! 재현한다.

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::domain::Failure;

use super::audio_input::TranscriptionInput;
use super::engine::{
    ensure_usable, engine_failed, model_unusable, output_unusable, TranscriptionEngine,
};
use super::model::ModelFile;
use super::parse::{RawSegment, RawTranscription};

/// `Cargo.toml`이 핀한 `whisper-rs` 버전.
///
/// Transcript의 `engine` provenance에 들어간다 (§7). **`Cargo.toml`과 함께 갱신한다** —
/// crate가 자신의 버전을 런타임에 노출하는 API는 이 Task에서 확인하지 못했고, 확인되지 않은
/// 것을 지어내지 않는다 (PRODUCT-SPEC §20.2).
const WHISPER_RS_VERSION: &str = "0.16";

/// 스레드 수를 정하지 못했을 때 쓰는 값. 어떤 기기에서도 도는 보수적인 기본값이다.
const FALLBACK_THREADS: i32 = 4;

/// 실제 whisper 엔진.
///
/// **모델을 미리 들고 있지 않는다.** 모델은 [`TranscriptionEngine::transcribe`]마다 열린다 —
/// 사용자가 설정에서 모델을 바꾸면 다음 전사부터 바로 그 모델이 쓰이고, 쓰지 않는 동안 수 GB를
/// 메모리에 붙들고 있지 않는다. 이 선택의 대가는 전사마다 드는 모델 적재 시간이다.
#[derive(Debug, Clone, Copy, Default)]
pub struct WhisperEngine {
    /// 추론에 쓸 스레드 수. `None`이면 기기에서 얻는다.
    threads: Option<i32>,
}

impl WhisperEngine {
    /// 기기에 맞춰 스레드 수를 정하는 엔진.
    pub fn new() -> Self {
        Self::default()
    }

    /// 스레드 수를 고정한 엔진. 1 미만은 1로 올린다.
    pub fn with_threads(threads: i32) -> Self {
        Self {
            threads: Some(threads.max(1)),
        }
    }

    fn threads(&self) -> i32 {
        self.threads.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|count| i32::try_from(count.get()).unwrap_or(FALLBACK_THREADS))
                .unwrap_or(FALLBACK_THREADS)
        })
    }
}

impl TranscriptionEngine for WhisperEngine {
    fn engine_id(&self) -> String {
        format!("whisper-rs/{WHISPER_RS_VERSION}")
    }

    fn transcribe(
        &self,
        input: &TranscriptionInput,
        model: &ModelFile,
    ) -> Result<RawTranscription, Failure> {
        let path = model.path().to_str().ok_or_else(|| {
            model_unusable(format!(
                "모델 파일 경로를 엔진에 넘길 수 없다: {}",
                model.path().display()
            ))
            .with_detail("경로가 UTF-8이 아니다")
        })?;

        // 모델 적재 실패는 **실행 실패와 구분한다** — 파일이 손상됐거나 이 엔진이 지원하지
        // 않는 모델이라는 뜻이고, 다시 시도해도 같다 (§13 `unsupported whisper model`).
        let context = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|error| {
                model_unusable(format!(
                    "이 모델 파일로는 전사할 수 없다: {}",
                    model.path().display()
                ))
                .with_detail(error)
            })?;

        let mut state = context
            .create_state()
            .map_err(|error| engine_failed("전사 엔진을 시작하지 못했다").with_detail(error))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(self.threads());
        // 번역이 아니라 전사다 — 들린 언어 그대로 받는다 (§7의 `language`).
        params.set_translate(false);
        // whisper.cpp의 진행 출력은 제품 로그가 아니다. 상태는 Recording.transcriptionStatus가
        // 말한다 (TASK-027 · TASK-028).
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // **입력은 이미 16 kHz mono f32다** (ADR-0007 §9 · audio_input.rs). 여기서 오디오를
        // 다시 만지지 않고, 파생 파일도 만들지 않는다 — 넘기는 것은 메모리 위의 슬라이스다.
        state
            .full(params, &input.samples)
            .map_err(|error| engine_failed("전사 도중 엔진이 실패했다").with_detail(error))?;

        let count = state.full_n_segments();
        let mut segments = Vec::with_capacity(count.max(0) as usize);
        for index in 0..count {
            // 엔진이 segment 개수를 말해 놓고 그 자리의 값을 주지 못하는 것은 **실행 실패가
            // 아니라 해석할 수 없는 출력**이다 — 엔진은 정상적으로 끝났다.
            let segment = state.get_segment(index).ok_or_else(|| {
                output_unusable("전사 결과를 읽지 못했다")
                    .with_detail(format!("segment {index}/{count}이(가) 없다"))
            })?;
            let text = segment.to_str().map_err(|error| {
                output_unusable("전사 결과를 읽지 못했다")
                    .with_detail(format!("segment {index}/{count}: {error}"))
            })?;

            // **원시 값을 그대로 옮긴다 — 계산하지 않는다.** 센티초 → 밀리초 변환은
            // parse.rs 한 곳에서만 일어난다 (ADR-0007 §10).
            segments.push(RawSegment {
                start_centiseconds: segment.start_timestamp(),
                end_centiseconds: segment.end_timestamp(),
                text: Some(text.to_owned()),
            });
        }

        // 엔진이 언어를 말하지 못하면 지어내지 않는다 — 알 수 없는 id는 `None`이 되고
        // §7의 `language`도 비어 있다. 코드를 해석하거나 바꾸지 않는다 (parse.rs와 같다).
        let language = whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_owned);

        ensure_usable(RawTranscription { language, segments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 이 파일의 테스트는 **엔진을 실행하지 않는다.** 모델도 없고 whisper도 부르지 않는다 —
    // 실행 경로의 검증은 test double이 하고 (`super::testing`), 실제 추론은 운영자의
    // smoke test가 한 번 한다 (PRODUCT-SPEC §14.4.3).

    #[test]
    fn the_engine_identifies_itself_for_transcript_provenance() {
        // 어떤 엔진으로 만든 전사인지 나중에 되짚을 수 있어야 한다 (§7).
        let engine_id = WhisperEngine::new().engine_id();

        assert!(engine_id.contains("whisper-rs"), "{engine_id}");
        assert!(engine_id.contains(WHISPER_RS_VERSION), "{engine_id}");
    }

    #[test]
    fn the_thread_count_is_always_usable() {
        assert!(WhisperEngine::new().threads() >= 1);
        assert_eq!(WhisperEngine::with_threads(2).threads(), 2);
        assert_eq!(
            WhisperEngine::with_threads(0).threads(),
            1,
            "0 스레드로는 아무것도 돌지 않는다"
        );
        assert_eq!(WhisperEngine::with_threads(-8).threads(), 1);
    }
}
