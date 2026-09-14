//! 로컬 전사 경계 (PRODUCT-SPEC §3.1의 `TranscriptionRunner` 자리 · Phase 3).
//!
//! 통합 방식은 `docs/ADR-0007-transcription-engine.md`가 정했다 — 엔진은 `whisper-rs`로
//! **앱 바이너리 안에 링크되며**, 저장소 밖에서 오는 것은 모델 파일 하나뿐이다.
//! 사용자는 whisper.cpp도 Homebrew도 CMake도 ffmpeg도 설치하지 않는다 (ADR-0007 §7).
//!
//! ```text
//! audio_input.rs   원본 녹음 → 16 kHz mono f32 (하드웨어도 모델도 없이 테스트된다)
//! model.rs         설정 값 + 모델 디렉터리 → 실제 모델 파일 하나
//!                  **모델 위치를 아는 코드는 여기뿐이다** (ADR-0007 §8.2 · INV-10)
//! engine.rs        실행 계약(trait) · §13의 네 가지 전사 실패 · 출력 계약
//! whisper.rs       그 계약의 실제 구현 — whisper-rs 호출 (ADR-0007 §2)
//! testing.rs       그 계약의 test double — 실제 whisper 없이 검증한다 (§18)
//! parse.rs         엔진의 원시 출력 → language · segments{start_ms,end_ms,text} · raw_text
//!                  **단위 변환이 일어나는 단 한 자리다** (ADR-0007 §10)
//! collapse.rs      정규화된 segment 열 → 쓸 수 있다 / 붕괴했다 + 그 판정을 만든 수치
//!                  **붕괴 판정 규칙이 사는 단 한 자리다** (ADR-0007 §18.2 · §18.3)
//! chunking.rs      어디서 자르는가 · 어떻게 되돌려 잇는가 · 무엇을 차단하는가
//!                  **청크 길이(120초) · 겹침(0) · 연속 반복 임계값(3)이 사는 단 한 자리다**
//!                  (ADR-0007 §20.8). 오디오도 엔진도 모르는 순수 규칙이다
//! run.rs           위의 조각들을 잇는 실행 순서 — 상태 전이와 Transcript 영속화 (§7 · §7.2)
//! ```
//!
//! ```text
//! recording.wav ─→ audio_input ─→ TranscriptionInput ─┐
//!                                                     │
//! 설정 값(모델) ─→ model ────────→ ModelFile ─────────┼─→ engine ─→ RawTranscription ─→ parse
//!                                                     │   (센티초)              (밀리초)
//! 설정 값(언어) ────────────────→ LanguageChoice ─────┘                                  │
//!                                                                                       │
//!                                          run ─→ Transcript 추가 · current 갱신 ←──────┘
//! ```
//!
//! **언어도 모델처럼 값으로 경계에 도달한다** (ADR-0007 §17.1.4). 설정을 읽는 자리는
//! [`crate::commands::Transcriber`] 하나이며, 엔진 구현은 그 값을 받아 쓸 뿐 스스로 정하지
//! 않는다 — 고르지 않음은 영어가 아니라 자동 감지다 (§17.1.4-1).
//!
//! **오디오도 전사 결과도 이 경계 밖으로 나가지 않는다** (§12 · INV-6). 엔진은 같은 프로세스
//! 안의 라이브러리이며, 이 모듈 전체에 네트워크 호출도 자식 프로세스 실행도 없다.
//!
//! **이 경계는 스레드를 만들지 않는다.** 전사를 UI와 IPC를 막지 않고 돌리는 일과 그것을
//! 화면에 여는 command는 [`crate::commands::Transcriber`]의 몫이다 — 여기 있는 것은 전부
//! 부르는 쪽의 스레드에서 그대로 도는 동기 코드이며, 그래서 실제 whisper 없이 값으로
//! 검증된다 (§18).

pub mod audio_input;
pub mod chunking;
pub mod collapse;
pub mod engine;
pub mod gain;
pub mod hallucination;
pub mod growing_wav;
pub mod live;
pub mod live_run;
pub mod model;
pub mod parse;
pub mod progress;
pub mod run;
pub mod testing;
pub mod vad;
pub mod whisper;

pub use audio_input::{
    load, TranscriptionInput, SOURCE_BITS_PER_SAMPLE, TARGET_CHANNELS, TARGET_SAMPLE_RATE_HZ,
};
pub use chunking::{
    block_consecutive_repeats, merge, plan, shift, AudioChunk, ChunkTranscription, RepeatBlocking,
    CHUNK_OVERLAP_FRAMES, CHUNK_SECONDS, MAX_CONSECUTIVE_REPEATS,
};
pub use collapse::{
    collapse_repeated_phrases,
    assess, CollapseAssessment, CollapseMetrics, CollapseVerdict,
    COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE, COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW,
    MINIMUM_SENTENCES_TO_JUDGE,
};
pub use engine::{
    engine_failed, ensure_usable, model_missing, model_unusable, output_unusable, LanguageChoice,
    TranscriptionEngine,
};
pub use model::ModelFile;
pub use progress::{PartialLine, Progress, Snapshot as ProgressSnapshot};
pub use parse::{
    milliseconds_from_centiseconds,
    normalize, Anomaly, AnomalyKind, RawSegment, RawTranscription, TranscriptSegment, Transcription,
};
pub use run::{transcribe, Completed, ModelChoice};
pub use testing::{StubCall, StubEngine};
pub use whisper::WhisperEngine;
