//! 테스트가 쓰는 test double (§18 · `phase-prompt/03` 요구 9).
//!
//! **자동 검증은 실제 whisper 바이너리도 모델 파일도 요구하지 않는다.** Gate가 도는 기기에
//! 수 GB짜리 모델이 있는지, Metal이 쓸 수 있는지, 추론이 몇 초 걸리는지에 전사 경계의 검증이
//! 걸려 있으면 그 검증은 하드웨어를 검사하는 것이지 제품을 검사하는 것이 아니다.
//!
//! 그렇다고 **모델이 없을 때 테스트를 건너뛰지도 않는다.** 모델 없음은 skip 조건이 아니라
//! §13의 정의된 실패이며 ([`super::model::resolve`]), 그 실패 자체가 테스트 대상이다.
//!
//! `#[cfg(test)]`가 아닌 것은 통합 테스트(`src-tauri/tests/`)와 이후 Task(TASK-027 ·
//! TASK-028)의 orchestration 테스트가 별개 crate에서 이것을 쓰기 때문이다.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::domain::Failure;

use super::audio_input::TranscriptionInput;
use super::engine::{ensure_usable, TranscriptionEngine};
use super::model::ModelFile;
use super::parse::RawTranscription;

/// [`StubEngine`]이 받은 호출 하나. 무엇으로 무엇을 전사하라고 했는지가 남는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubCall {
    /// 넘어온 파생 입력의 샘플 프레임 수.
    pub frames: usize,
    /// 넘어온 모델 파일의 경로.
    pub model_path: PathBuf,
    /// 넘어온 모델 파일의 식별자.
    pub model_id: String,
}

/// 미리 정해 둔 결과를 돌려주는 엔진.
///
/// **계약을 우회하지 않는다** — 실제 엔진과 똑같이 [`ensure_usable`]을 통과한 값만 돌려준다.
/// double이 제품보다 관대하면 테스트는 제품이 아니라 double을 검증하게 된다.
///
/// ```text
/// StubEngine::returning(raw)          같은 결과를 몇 번이든 돌려준다
/// StubEngine::failing(failure)        같은 실패를 몇 번이든 돌려준다
/// StubEngine::responding_with(list)   순서대로 돌려주고, 다 쓰면 마지막 것을 되풀이한다
///                                     (재전사처럼 같은 엔진이 두 번 불리는 경우 · TASK-027)
/// ```
#[derive(Debug)]
pub struct StubEngine {
    responses: Mutex<Vec<Result<RawTranscription, Failure>>>,
    calls: Mutex<Vec<StubCall>>,
}

impl StubEngine {
    /// 언제나 같은 원시 출력을 내는 엔진.
    pub fn returning(raw: RawTranscription) -> Self {
        Self::responding_with(vec![Ok(raw)])
    }

    /// 언제나 같은 실패를 내는 엔진.
    pub fn failing(failure: Failure) -> Self {
        Self::responding_with(vec![Err(failure)])
    }

    /// 호출 순서대로 결과를 내는 엔진. 목록을 다 쓰면 마지막 결과를 되풀이한다.
    ///
    /// # Panics
    ///
    /// `responses`가 비어 있으면 panic한다 — 무엇을 돌려줄지 정하지 않은 double은 테스트를
    /// 조용히 통과시키는 것보다 즉시 실패하는 편이 낫다.
    pub fn responding_with(responses: Vec<Result<RawTranscription, Failure>>) -> Self {
        assert!(
            !responses.is_empty(),
            "StubEngine은 적어도 하나의 결과를 가져야 한다"
        );
        Self {
            responses: Mutex::new(responses),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// 지금까지 받은 호출들.
    pub fn calls(&self) -> Vec<StubCall> {
        self.locked_calls().clone()
    }

    /// 지금까지 받은 호출 수.
    pub fn call_count(&self) -> usize {
        self.locked_calls().len()
    }

    fn locked_calls(&self) -> std::sync::MutexGuard<'_, Vec<StubCall>> {
        self.calls.lock().expect("stub 호출 기록을 잠근다")
    }
}

impl TranscriptionEngine for StubEngine {
    fn engine_id(&self) -> String {
        // 실제 엔진의 식별자와 절대 겹치지 않는다. 이 값이 저장된 Transcript는 테스트가
        // 만든 것이다.
        "stub-transcription-engine".to_owned()
    }

    fn transcribe(
        &self,
        input: &TranscriptionInput,
        model: &ModelFile,
    ) -> Result<RawTranscription, Failure> {
        self.locked_calls().push(StubCall {
            frames: input.frames(),
            model_path: model.path().to_path_buf(),
            model_id: model.id().to_owned(),
        });

        let mut responses = self.responses.lock().expect("stub 결과 목록을 잠근다");
        let response = if responses.len() > 1 {
            responses.remove(0)
        } else {
            responses[0].clone()
        };

        response.and_then(ensure_usable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FailureKind;
    use crate::transcription::engine::{engine_failed, model_missing};
    use crate::transcription::parse::RawSegment;
    use std::path::Path;

    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 임시 디렉터리 하나. 실제 모델 파일이 아니라 몇 바이트짜리 자리표시자를 둔다.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let unique = format!(
                "molt-note-transcription-stub-{}-{}-{}",
                label,
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn model_in(directory: &Path) -> ModelFile {
        std::fs::write(directory.join("ggml-base.bin"), b"not a real model")
            .expect("사전 조건: 자리표시자 모델을 만든다");
        crate::transcription::model::resolve(directory, Some("ggml-base.bin"))
            .expect("해석할 수 있어야 한다")
    }

    fn input() -> TranscriptionInput {
        TranscriptionInput {
            samples: vec![0.0; 1_600],
            sample_rate_hz: crate::transcription::audio_input::TARGET_SAMPLE_RATE_HZ,
            channels: crate::transcription::audio_input::TARGET_CHANNELS,
            source_sample_rate_hz: crate::transcription::audio_input::TARGET_SAMPLE_RATE_HZ,
            source_channels: 1,
            resampled: false,
            downmixed: false,
        }
    }

    fn spoken(text: &str) -> RawTranscription {
        RawTranscription {
            language: Some("ko".to_owned()),
            segments: vec![RawSegment {
                start_centiseconds: 9_000,
                end_centiseconds: 9_150,
                text: Some(text.to_owned()),
            }],
        }
    }

    #[test]
    fn the_double_records_what_it_was_asked_to_transcribe() {
        let temp = TempDir::new("records");
        let model = model_in(temp.path());
        let engine = StubEngine::returning(spoken("들린 문장"));

        let output = engine
            .transcribe(&input(), &model)
            .expect("정상 출력을 돌려줘야 한다");

        assert_eq!(output, spoken("들린 문장"), "값을 그대로 옮긴다");
        assert_eq!(engine.call_count(), 1);
        assert_eq!(
            engine.calls()[0],
            StubCall {
                frames: 1_600,
                model_path: model.path().to_path_buf(),
                model_id: "ggml-base.bin".to_owned(),
            }
        );
    }

    #[test]
    fn the_double_repeats_its_last_answer_for_later_calls() {
        let temp = TempDir::new("repeat");
        let model = model_in(temp.path());
        let engine = StubEngine::responding_with(vec![
            Err(engine_failed("첫 시도가 죽었다")),
            Ok(spoken("두 번째 시도")),
        ]);

        let first = engine.transcribe(&input(), &model);
        let second = engine.transcribe(&input(), &model);
        let third = engine.transcribe(&input(), &model);

        assert_eq!(
            first.expect_err("첫 시도는 실패다").kind,
            FailureKind::TranscriptionEngineFailed
        );
        assert_eq!(second.expect("두 번째는 성공이다"), spoken("두 번째 시도"));
        assert_eq!(third.expect("마지막 결과가 되풀이된다"), spoken("두 번째 시도"));
        assert_eq!(engine.call_count(), 3);
    }

    #[test]
    fn the_double_can_stand_in_for_any_product_failure() {
        let temp = TempDir::new("failing");
        let model = model_in(temp.path());
        let engine = StubEngine::failing(model_missing("모델이 없다"));

        let failure = engine
            .transcribe(&input(), &model)
            .expect_err("실패를 돌려줘야 한다");

        assert_eq!(failure.kind, FailureKind::TranscriptionModelMissing);
    }

    #[test]
    fn the_double_obeys_the_same_output_contract_as_the_real_engine() {
        // double이 빈 출력을 성공으로 돌려줄 수 있으면, 제품이 막는 것을 테스트가 통과시킨다.
        let temp = TempDir::new("empty-output");
        let model = model_in(temp.path());
        let engine = StubEngine::returning(RawTranscription {
            language: Some("ko".to_owned()),
            segments: Vec::new(),
        });

        let failure = engine
            .transcribe(&input(), &model)
            .expect_err("빈 출력은 실패다");

        assert_eq!(failure.kind, FailureKind::TranscriptionOutputUnusable);
    }

    #[test]
    fn the_double_never_claims_to_be_the_real_engine() {
        let engine = StubEngine::returning(spoken("x"));

        assert_eq!(engine.engine_id(), "stub-transcription-engine");
        assert!(!engine.engine_id().contains("whisper"));
    }
}
