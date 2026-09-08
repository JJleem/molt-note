//! ADR-0007이 고른 실제 구현 — `whisper-rs`로 whisper.cpp를 **이 프로세스 안에서** 실행한다.
//!
//! ```text
//! 16 kHz mono f32 (메모리)        모델 파일 하나 (사용자가 둔다)
//!         └──────────┬──────────────────┘
//!                    ▼
//!        모델 context 하나  ← 전사 한 건에 한 번 연다 (§20.3)
//!                    ▼
//!        청크마다:  새 state → whisper-rs → 앱 바이너리에 링크된 whisper.cpp
//!                    ▼           그 청크의 샘플 슬라이스 하나만 full()에 넘긴다
//!        청크 로컬 원시 segment (센티초 · 값을 그대로 옮긴다)
//!                    ▼
//!        chunking::merge → 전체 시간축의 RawTranscription 하나 → parse.rs
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
//! ## 오디오 전체를 한 번에 넘기지 않는다 — **청크 단위로 돈다** (ADR-0007 §20)
//!
//! ```text
//! 모델 context   전사 한 건에 한 번 연다. 청크마다 다시 열지 않는다 — 가중치는 변하지 않는다
//! state          **청크마다 새로 만든다** (§20.3). 반복 루프에 빠진 디코딩의 흔적이 남는 자리가
//!                여기이므로, 한 청크의 붕괴가 다음 청크로 이어지지 않게 끊는다
//! FullParams     청크마다 새로 만든다. 설정 내용은 아래 두 갈래와 출력 억제 넷 그대로다 —
//!                **디코딩 파라미터를 새로 튜닝하지 않는다** (§20.9)
//! 샘플           청크 구간으로 자른 슬라이스 하나만 full()에 넘긴다. 오디오를 복사해 들고
//!                있지 않으며, 파생 파일도 여전히 만들지 않는다
//! ```
//!
//! **그 규칙은 이 파일에 없다.** 어디서 자르는가([`chunking::plan`]) · 청크 로컬 시각을 전체
//! 시간축으로 어떻게 되돌리고 청크 결과를 어떻게 잇는가 · 청크들이 보고한 언어에서 어떻게
//! 하나를 정하는가([`chunking::merge`] · §20.7)는 전부 [`super::chunking`] 하나에 있고, 이
//! 파일은 그 함수를 **부르기만** 한다. 청크 길이도 겹침도 반복 임계값도 여기 숫자로 나타나지
//! 않는다 (§20.8) — 두 자리에 있으면 한쪽만 고쳐지는 날이 온다. 연속 반복 차단은 붕괴 판정
//! **뒤에** 오므로 이 파일이 아니라 저장 직전의 자리에서 돈다 (§20.6.2).
//!
//! **밖으로 나가는 계약은 지금과 같다** (§20.8). trait의 시그니처도 [`RawTranscription`]의
//! 모양도 바뀌지 않았고, 돌려주는 것은 여전히 [`ensure_usable`]을 통과한 전사 **하나**다 —
//! 청크는 이 함수 밖에서 보이지 않는다 (INV-9).
//!
//! ## 청크 하나가 실패하면 — **새 실패 종류를 만들지 않는다** (§13)
//!
//! ```text
//! state를 만들지 못했다 · full()이 실패했다   →  TranscriptionEngineFailed   재시도 가능
//! segment를 읽지 못했다                        →  TranscriptionOutputUnusable 엔진은 끝났다
//! 시각이 i64를 넘쳤다                          →  chunking이 낸 실패를 그대로 넘긴다
//!                                                (값을 접지 않는다 · §20.5)
//! ```
//!
//! 어느 쪽이든 **전사 전체가 실패한다.** 앞선 청크의 결과를 부분 전사로 저장하지 않는다 —
//! Transcript는 immutable하므로(INV-2) 중간까지만 있는 전사가 영구히 남는다. 대신 **몇 번째
//! 청크였는지가 `detail`에 `chunk k/n`으로 남는다.** 그것이 없으면 72분짜리 녹음에서 어디가
//! 무너졌는지 아무도 되짚지 못한다.
//!
//! ## 여기 쓰인 API는 문서가 아니라 컴파일러가 확인한 것이다
//!
//! `whisper-rs` 0.16.0 (`whisper-rs-sys` 0.15.0)에 대해 실제로 확인한 시그니처다.
//!
//! ```text
//! WhisperContext::create_state()           -> Result<WhisperState, WhisperError>
//!                                             ← 하나의 context에서 **청크마다** 부른다 (§20.3)
//! WhisperState::full_n_segments()          -> i32
//! WhisperState::get_segment(i32)           -> Option<WhisperSegment<'_>>
//! WhisperSegment::start_timestamp()        -> i64      ← 단위는 아래를 볼 것
//! WhisperSegment::end_timestamp()          -> i64
//! WhisperSegment::to_str()                 -> Result<&str, WhisperError>
//! WhisperState::full_lang_id_from_state()  -> i32
//! FullParams::set_language(Option<&str>)   ← 고른 언어를 지정한다 (None이면 지정하지 않는다)
//! FullParams::set_detect_language(bool)    ← 엔진에게 감지를 시킨다
//! ```
//!
//! 마지막 두 줄은 **이 Task가 컴파일러로 확인했다** (TASK-068). 문서를 읽고 옮긴 것이 아니라
//! Rust Gate가 실제로 이 호출을 컴파일한다 — 시그니처가 다르면 Gate가 실패한다.
//!
//! ## 무슨 언어로 들을지는 여기서 정하지 않는다 (ADR-0007 §17.1.4-4)
//!
//! 이 파일은 설정도 저장소도 읽지 않는다. 넘어오는 것은 [`LanguageChoice`] 하나이고, 이
//! 파일이 하는 일은 그 선택을 `FullParams` 호출로 옮기는 것뿐이다.
//!
//! ```text
//! Chosen("ko")  →  set_language(Some("ko")) · set_detect_language(false)
//! Detect        →  set_language(None)       · set_detect_language(true)
//! 어느 쪽이든    →  set_translate(false)      번역이 아니라 전사다 (§2)
//! ```
//!
//! **두 갈래 중 어느 쪽도 whisper.cpp의 기본값에 기대지 않는다.** 그 기본값은 자동 감지가
//! 아니라 `language = "en"` · `detect_language = false`이며 (§17.1.2), 아무도 그것을 바꾸지
//! 않은 채로 72분짜리 한국어 회의가 통째로 못 쓰게 됐다 (§17.1.1).
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

use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperVadParams,
};

use crate::domain::Failure;

use super::audio_input::TranscriptionInput;
use super::chunking::{self, ChunkTranscription};
use super::progress::{PartialLine, Progress};
use super::engine::{
    ensure_usable, engine_failed, model_unusable, output_unusable, LanguageChoice,
    TranscriptionEngine,
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
#[derive(Debug, Clone, Default)]
pub struct WhisperEngine {
    /// 추론에 쓸 스레드 수. `None`이면 기기에서 얻는다.
    threads: Option<i32>,
    /// 조각이 끝날 때마다 나온 문장을 놓아 두는 자리. 없으면 아무것도 보고하지 않는다.
    ///
    /// **이것은 저장 경로가 아니다** (`progress` 모듈 문서). 여기 놓인 값은 미리보기이며,
    /// 저장되는 Transcript는 여전히 `run`이 `parse` · `collapse`를 지나 만든다.
    progress: Option<Progress>,
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
            progress: None,
        }
    }

    /// 진행 상황을 보고하는 엔진.
    ///
    /// **trait 시그니처를 바꾸지 않는다** (ADR-0007 §20.8). 조각을 아는 것은 이 구현뿐이므로
    /// 보고할 것이 있는 것도 이 구현뿐이다 — [`TranscriptionEngine`]에 인자를 더하면 조각을
    /// 모르는 구현까지 그것을 알아야 한다.
    pub fn reporting_to(mut self, progress: Progress) -> Self {
        self.progress = Some(progress);
        self
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
        language: &LanguageChoice,
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

        // **VAD 모델을 모델 파일 옆에서 찾는다.** `model::resolve`가 상대 이름을 모델
        // 디렉터리 안에서 풀므로 보통은 그 디렉터리이고, 사용자가 절대 경로로 모델을 다른
        // 자리에 뒀다면 VAD 모델도 그 옆에 있는 것이 규칙이다 — 모델을 함께 둔다.
        //
        // **없으면 없는 대로 간다.** 그때의 실행은 2026-09-08 이전과 한 글자도 다르지 않다.
        let vad = model
            .path()
            .parent()
            .map(super::vad::find)
            .unwrap_or(super::vad::VadChoice::Disabled);

        // 경로가 UTF-8이 아니면 VAD를 켜지 않는다. **여기서 실패를 만들지 않는다** — VAD는
        // 보강이지 전제가 아니며, 이것 때문에 전사가 멈추면 전보다 나빠진다.
        let vad_path: Option<String> = vad
            .path()
            .and_then(|path| path.to_str())
            .map(str::to_owned);

        // ① **어디서 자르는가는 여기서 정하지 않는다** (ADR-0007 §20.8). 넘기는 것은 프레임
        // 수와 샘플레이트뿐이고, 돌아오는 것은 구간과 오프셋이다 — 값도 규칙도 chunking에 있다.
        // 전체가 청크 하나에 들어가면 목록은 하나이며 지금까지와 완전히 같은 실행이 된다.
        let chunks = chunking::plan(input.frames(), input.sample_rate_hz)?;
        let chunk_count = chunks.len();

        // 지난 전사의 미리보기가 남아 있지 않게 비우고, 전체 크기를 알린다. **프레임 수다** —
        // 청크 개수를 밖으로 내보내지 않는다 (§20.8 · INV-9).
        if let Some(progress) = &self.progress {
            progress.begin(input.frames());
        }
        let mut transcribed: Vec<ChunkTranscription> = Vec::with_capacity(chunk_count);

        for (index, chunk) in chunks.iter().enumerate() {
            // **청크마다 새 state다** (§20.3). 모델 context는 위에서 한 번 열었고 여기서 다시
            // 열지 않는다 — 끊는 것은 디코딩 중에 값이 변하는 쪽이고, 변하지 않는 가중치를
            // 청크 수만큼 다시 읽는 비용은 얻는 것 없이 붙는다.
            let mut state = context.create_state().map_err(|error| {
                engine_failed("전사 엔진을 시작하지 못했다")
                    .with_detail(format!("chunk {index}/{chunk_count}: {error}"))
            })?;

            // 파라미터도 청크마다 새로 만든다. **내용은 하나도 잃지 않는다** — 스레드 수,
            // 번역이 아니라는 것, 언어 선택의 두 갈래, 출력 억제 넷이 **모든 청크에** 그대로
            // 적용된다. 청크를 나눴다는 이유로 디코딩 파라미터를 새로 튜닝하지 않는다 (§20.9).
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_n_threads(self.threads());
            // 번역이 아니라 전사다 — 들린 언어 그대로 받는다 (§7의 `language`).
            params.set_translate(false);

            // **언어 파라미터를 건드리지 않는 경로는 없다** (ADR-0007 §17.1.4-1). 넘어온 선택이
            // 무엇이든 두 함수를 다 부르며, 정하는 것은 이 파일이 아니라 부르는 쪽이다.
            match language.chosen() {
                // 사용자가 고른 언어로 듣는다. 감지는 함께 끈다 — 고른 값이 있는데 엔진이
                // 스스로 다른 언어로 판단하면 그 선택은 없는 것과 같다.
                Some(code) => {
                    params.set_language(Some(code));
                    params.set_detect_language(false);
                }
                // 고르지 않음은 영어가 아니라 감지다. 지정할 언어가 없다는 것을 그대로 넘기고
                // (`None`), 감지를 켠다 — whisper.cpp의 기본값 `"en"`이 조용히 쓰이지 않게 하는
                // 것이 이 두 줄의 전부다 (§17.1.1 · §17.1.2).
                None => {
                    params.set_language(None);
                    params.set_detect_language(true);
                }
            }

            // whisper.cpp의 진행 출력은 제품 로그가 아니다. 상태는 Recording.transcriptionStatus가
            // 말한다 (TASK-027 · TASK-028).
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);

            // **무음을 디코더에 주지 않는다** (`super::vad` 문서 · 2026-09-08의 두 관측).
            //
            // ⚠️ **순서가 규칙이다.** `enable_vad(true)`는 경로가 설정돼 있지 않으면
            // **panic한다** (`whisper_params.rs:823`). 경로를 먼저 넣고, 넣은 경우에만 켠다.
            if let Some(path) = &vad_path {
                params.set_vad_model_path(Some(path));
                params.enable_vad(true);

                // 기본값에서 **두 가지만** 안전한 쪽으로 옮긴다. 나머지는 whisper.cpp가 정한
                // 값 그대로다 — 재본 적 없는 값을 이 앱이 고르지 않는다.
                //
                // 이 Phase의 가장 큰 위험은 **무음 억제가 실제 발화를 함께 지우는 것**이다.
                // 아래 둘은 그 위험을 줄이는 방향(오디오를 더 남기는 쪽)으로만 움직인다.
                let mut vad_params = WhisperVadParams::default();
                // 침묵이 이만큼은 이어져야 발화가 끝난 것으로 본다 (기본 100ms).
                // 말 사이의 짧은 숨이 발화를 끊지 않게 한다 — 끊길수록 잘려 나갈 위험이 커진다.
                vad_params.set_min_silence_duration(500);
                // 발화 앞뒤로 이만큼 남긴다 (기본 30ms). 말의 첫 소리와 끝 소리가 잘리지 않게
                // 한다. 문제였던 것은 **몇 분짜리** 무음이지 0.2초가 아니다.
                vad_params.set_speech_pad(200);
                params.set_vad_params(vad_params);
            }

            // **입력은 이미 16 kHz mono f32다** (ADR-0007 §9 · audio_input.rs). 여기서 오디오를
            // 다시 만지지 않고, 파생 파일도 만들지 않는다 — 넘기는 것은 메모리 위의 슬라이스,
            // 그중에서도 이 청크의 구간 하나다 (§20.8 — 자르는 일은 부르는 쪽의 몫이다).
            state
                .full(params, &input.samples[chunk.range()])
                .map_err(|error| {
                    engine_failed("전사 도중 엔진이 실패했다")
                        .with_detail(format!("chunk {index}/{chunk_count}: {error}"))
                })?;

            let count = state.full_n_segments();
            // **바이트로 먼저 모은다** (§20.9). whisper.cpp의 segment 경계는 토큰 경계이지
            // 글자 경계가 아니므로, 한글처럼 여러 바이트인 글자는 두 segment에 걸쳐 잘린다.
            // 여기서 `to_str()`을 부르면 그 한 글자 때문에 전사 전체가 실패한다 —
            // 2026-09-07 실행이 `chunk 32/37`에서 그렇게 죽었다.
            let mut raw_bytes: Vec<Vec<u8>> = Vec::with_capacity(count.max(0) as usize);
            let mut segments: Vec<RawSegment> = Vec::with_capacity(count.max(0) as usize);
            for position in 0..count {
                // 엔진이 segment 개수를 말해 놓고 그 자리의 값을 주지 못하는 것은 **실행 실패가
                // 아니라 해석할 수 없는 출력**이다 — 엔진은 정상적으로 끝났다.
                let segment = state.get_segment(position).ok_or_else(|| {
                    output_unusable("전사 결과를 읽지 못했다").with_detail(format!(
                        "chunk {index}/{chunk_count}: segment {position}/{count}이(가) 없다"
                    ))
                })?;
                // 바이트를 못 얻는 것은 null 포인터뿐이다 — 그것은 진짜 해석 불가다.
                let bytes = segment.to_bytes().map_err(|error| {
                    output_unusable("전사 결과를 읽지 못했다").with_detail(format!(
                        "chunk {index}/{chunk_count}: segment {position}/{count}: {error}"
                    ))
                })?;

                // **원시 값을 그대로 옮긴다 — 계산하지 않는다.** 시각은 이 청크 안에서의 값이며,
                // 전체 시간축으로 되돌리는 덧셈은 아래 `merge`가 한다 (§20.5). 센티초 → 밀리초
                // 변환은 그다음의 parse.rs 한 곳에서만 일어난다 (ADR-0007 §10).
                // 텍스트는 아직 넣지 않는다 — 잘린 글자를 이어 붙이는 일이 아래에 있다.
                segments.push(RawSegment {
                    start_centiseconds: segment.start_timestamp(),
                    end_centiseconds: segment.end_timestamp(),
                    text: None,
                });
                raw_bytes.push(bytes.to_vec());
            }

            // 잘린 글자를 다음 segment 앞으로 넘기며 디코딩한다. **규칙은 이 파일에 없다** —
            // `chunking::decode_segment_texts` 한 곳이다 (§20.9). 시각은 건드리지 않는다.
            for (segment, text) in segments
                .iter_mut()
                .zip(chunking::decode_segment_texts(&raw_bytes))
            {
                segment.text = Some(text);
            }

            // **엔진이 이 청크에 대해 보고한 값이다.** 위에서 넘겨받은 선택을 여기에 베껴 넣지
            // 않는다 — 감지를 켰다는 것이 "언제나 값이 있다"는 뜻이 되지 않으며 (§17.1.4-3),
            // 엔진이 언어를 말하지 못하면 알 수 없는 id는 `None`이 된다. 코드를 해석하거나
            // 바꾸지 않는다 (parse.rs와 같다 · §16.2). 여러 청크의 값에서 **하나를 정하는 규칙은
            // 이 파일이 아니라 `merge`에 있다** (§20.7).
            let language =
                whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_owned);

            // **조각 하나가 끝났다 — 지금 보고한다** (`progress` 모듈). 저장 경로가 아니므로
            // 여기서 실패해도 전사는 계속된다. 시각은 전체 시간축의 **원시 센티초**이며,
            // 오프셋을 더하는 규칙은 `chunking`의 것을 그대로 쓴다 (§20.5 · §10).
            if let Some(progress) = &self.progress {
                let lines = segments.iter().filter_map(|segment| {
                    let text = segment.text.as_deref()?.trim();
                    (!text.is_empty()).then(|| PartialLine {
                        start_centiseconds: segment.start_centiseconds
                            + chunk.offset_centiseconds,
                        text: text.to_owned(),
                    })
                });
                progress.advance(lines.collect::<Vec<_>>(), chunk.end_frame());
            }

            transcribed.push(ChunkTranscription {
                offset_centiseconds: chunk.offset_centiseconds,
                output: RawTranscription { language, segments },
            });
        }

        // ② **이어 붙이는 규칙도 여기 없다** — 오프셋을 더해 전체 시간축으로 되돌리는 일과
        // 청크들의 language에서 하나를 정하는 일은 `merge`가 한다 (§20.5 · §20.7). 그리고
        // 합쳐진 결과 하나가 지금까지와 똑같이 출력 계약을 통과한 뒤 나간다.
        ensure_usable(chunking::merge(transcribed)?)
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
