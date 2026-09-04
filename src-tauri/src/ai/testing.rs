//! 계약 준수 검사 묶음과, 그것을 통과하는 **결정론적 test double** (ADR-0008 §4.3 · §18).
//!
//! ```text
//! assert_note_ai_provider_contract(&impl)   구현을 인자로 받는다 — fake도, 이후의 실제
//!                                          adapter도 같은 묶음을 그대로 통과한다
//! FakeNoteAiProvider                       그 계약의 test double. 정상 응답 · schema에 어긋난
//!                                          응답 · 연결 불가 · 모델 없음을 결정론적으로 낸다
//! ```
//!
//! **자동 검증은 실제 AI 서버도 모델 파일도 요구하지 않는다** (§18). Gate가 도는 기기에 어떤
//! 모델이 설치돼 있는지, 서버가 떠 있는지에 계약의 검증이 걸려 있으면 그 검증은 환경을
//! 검사하는 것이지 제품을 검사하는 것이 아니다. 그렇다고 서버가 없을 때 테스트를 건너뛰지도
//! 않는다 — 연결 불가는 skip 조건이 아니라 §13의 정의된 실패이며, 그 실패 자체가 검증 대상이다.
//!
//! **구현이 하나뿐인 추상화는 검증된 추상화가 아니다** (ADR-0008 §4.3). 이 double이 계약의 두
//! 번째 구현이며, 그래서 실제 adapter가 생기기 전에도 경계가 지켜지는지 확인할 수 있다.
//!
//! ## 이것은 제품의 선택지가 아니다
//!
//! [`FakeNoteAiProvider`]는 **사용자에게 제공되는 provider가 아니다.** 제품 경로는 이 double을
//! 만들지 않으며, 그 사실을 [`tests::product_code_never_constructs_the_fake_provider`]가 소스에서
//! 확인한다. 식별자도 실제 벤더와 겹치지 않는 [`FAKE_PROVIDER_ID`]이므로, 이 값이 저장된
//! `ai_notes` 행은 테스트가 만든 것이다.
//!
//! `#[cfg(test)]`가 아닌 것은 통합 테스트(`src-tauri/tests/`)와 이후 Task의 orchestration
//! 테스트가 **별개 crate에서** 이것을 쓰기 때문이다 (`crate::transcription::testing`과 같은
//! 이유다).

use std::sync::Mutex;

use crate::domain::{Failure, NoteType};

use super::note::{parse_note, MeetingNote, StructuredNote, StudyNote, SummaryNote};
use super::provider::{
    model_unavailable, rejected_response, unreachable, Availability, Locality, NoteAiProvider,
    NoteGeneration, NoteRequest, ProviderDescriptor, AI_PROVIDER_FAILURE_KINDS,
};

/// double의 provider 식별자. **실제 벤더 식별자와 절대 겹치지 않는다.**
pub const FAKE_PROVIDER_ID: &str = "fake-note-ai-provider";

/// double이 자기가 썼다고 말하는 모델. 실제 모델 이름이 아니다.
pub const FAKE_MODEL_ID: &str = "fake-note-model";

/// 계약 준수 검사가 쓰는 전사 텍스트. 짧고 고정돼 있다 — 실제 전사 결과가 아니다.
pub const CONTRACT_TRANSCRIPT: &str =
    "오늘 회의에서는 다음 분기 일정과 남은 과제를 이야기했다. 결론은 다음 주에 다시 정리하기로 했다.";

/// [`FakeNoteAiProvider`]가 받은 호출 하나.
///
/// **여기에 오디오가 없는 것은 double의 선택이 아니다** — 계약이 오디오를 옮길 수 없으므로
/// 기록할 것도 없다 (INV-6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeCall {
    pub mode: NoteType,
    /// 넘어온 전사 텍스트 그대로.
    pub transcript: String,
    /// 요청에 실린 context 크기.
    pub context_tokens: u32,
}

/// double이 낼 수 있는 응답 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeResponse {
    /// 요청된 mode의 **유효한** 표본 노트를 낸다 ([`sample_note`]).
    SampleNote,
    /// 모델이 냈다고 가정할 **원시 텍스트**를 그대로 낸다.
    ///
    /// schema에 어긋난 텍스트를 주면 실제 adapter와 똑같은 실패가 난다 — double은 제품의
    /// 방어 경로([`parse_note`])를 우회하지 않는다.
    RawText(String),
    /// 이미 §13의 공통 실패로 번역된 상황 (연결 불가 · 모델 없음 등).
    Rejected(Failure),
}

/// 미리 정해 둔 응답을 돌려주는 provider.
///
/// ```text
/// FakeNoteAiProvider::ready()                 요청된 mode의 유효한 표본 노트를 낸다
/// FakeNoteAiProvider::generating_text(text)   그 텍스트를 모델 출력으로 삼는다 (schema 위반 검증)
/// FakeNoteAiProvider::unreachable()           서버가 응답하지 않는 상황
/// FakeNoteAiProvider::without_models()        응답하지만 쓸 모델이 하나도 없는 상황
/// FakeNoteAiProvider::failing(failure)        언제나 같은 실패
/// FakeNoteAiProvider::responding_with(list)   순서대로 내고, 다 쓰면 마지막을 되풀이한다
/// ```
///
/// **결정론적이다.** 같은 요청은 언제나 같은 답을 낸다 — 무작위도, 시간도, 파일도, 네트워크도
/// 쓰지 않는다.
#[derive(Debug)]
pub struct FakeNoteAiProvider {
    name: String,
    locality: Locality,
    availability: Availability,
    model: String,
    responses: Mutex<Vec<FakeResponse>>,
    calls: Mutex<Vec<FakeCall>>,
}

impl FakeNoteAiProvider {
    /// 요청된 mode의 유효한 표본 노트를 내는 provider.
    pub fn ready() -> Self {
        Self::responding_with(vec![FakeResponse::SampleNote])
    }

    /// 모델이 이 텍스트를 냈다고 가정하는 provider.
    ///
    /// 정상 JSON도, 코드펜스에 싸인 JSON도, JSON이 아닌 산문도 줄 수 있다 — 무엇을 주든
    /// 제품과 같은 파싱·검증을 통과한다.
    pub fn generating_text(text: impl Into<String>) -> Self {
        Self::responding_with(vec![FakeResponse::RawText(text.into())])
    }

    /// 서버가 응답하지 않는 provider. 가용성도 생성도 같은 실패를 낸다.
    pub fn unreachable() -> Self {
        let failure = || unreachable("AI 서버가 응답하지 않는다");
        Self {
            availability: Availability::Unavailable(failure()),
            ..Self::responding_with(vec![FakeResponse::Rejected(failure())])
        }
    }

    /// 응답하지만 쓸 수 있는 모델이 하나도 없는 provider.
    pub fn without_models() -> Self {
        Self {
            availability: Availability::NoModels,
            ..Self::responding_with(vec![FakeResponse::Rejected(model_unavailable(
                "쓸 수 있는 모델이 없다",
            ))])
        }
    }

    /// 언제나 같은 실패를 내는 provider.
    pub fn failing(failure: Failure) -> Self {
        Self::responding_with(vec![FakeResponse::Rejected(failure)])
    }

    /// 호출 순서대로 응답하는 provider. 목록을 다 쓰면 마지막 응답을 되풀이한다.
    ///
    /// # Panics
    ///
    /// `responses`가 비어 있으면 panic한다 — 무엇을 돌려줄지 정하지 않은 double은 테스트를
    /// 조용히 통과시키는 것보다 즉시 실패하는 편이 낫다.
    pub fn responding_with(responses: Vec<FakeResponse>) -> Self {
        assert!(
            !responses.is_empty(),
            "FakeNoteAiProvider는 적어도 하나의 응답을 가져야 한다"
        );
        Self {
            name: "테스트용 가짜 provider".to_owned(),
            locality: Locality::Local,
            availability: Availability::Ready {
                models: vec![FAKE_MODEL_ID.to_owned()],
            },
            model: FAKE_MODEL_ID.to_owned(),
            responses: Mutex::new(responses),
            calls: Mutex::new(Vec::new()),
        }
    }

    /// 외부로 나가는 provider인 것처럼 행동하게 한다 (§12 · INV-5의 표시를 검증할 때 쓴다).
    pub fn with_locality(mut self, locality: Locality) -> Self {
        self.locality = locality;
        self
    }

    /// 가용성이 돌려줄 모델 목록을 바꾼다.
    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.availability = Availability::Ready { models };
        self
    }

    /// 생성 결과에 provenance로 남길 모델 이름을 바꾼다.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// 지금까지 받은 호출들.
    pub fn calls(&self) -> Vec<FakeCall> {
        self.locked_calls().clone()
    }

    /// 지금까지 받은 호출 수.
    pub fn call_count(&self) -> usize {
        self.locked_calls().len()
    }

    fn locked_calls(&self) -> std::sync::MutexGuard<'_, Vec<FakeCall>> {
        self.calls.lock().expect("fake 호출 기록을 잠근다")
    }

    fn next_response(&self) -> FakeResponse {
        let mut responses = self.responses.lock().expect("fake 응답 목록을 잠근다");
        if responses.len() > 1 {
            responses.remove(0)
        } else {
            responses[0].clone()
        }
    }
}

impl NoteAiProvider for FakeNoteAiProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: FAKE_PROVIDER_ID.to_owned(),
            name: self.name.clone(),
            locality: self.locality,
        }
    }

    fn availability(&self) -> Availability {
        self.availability.clone()
    }

    fn generate_note(&self, request: &NoteRequest<'_>) -> Result<NoteGeneration, Failure> {
        self.locked_calls().push(FakeCall {
            mode: request.mode,
            transcript: request.transcript_text().to_owned(),
            context_tokens: request.context_budget.context_tokens,
        });

        let text = match self.next_response() {
            FakeResponse::SampleNote => sample_note_text(request.mode),
            FakeResponse::RawText(text) => text,
            FakeResponse::Rejected(failure) => return Err(failure),
        };

        // **계약을 우회하지 않는다** — 실제 adapter와 똑같이 파싱·검증을 통과한 값만 돌려준다.
        // double이 제품보다 관대하면 테스트는 제품이 아니라 double을 검증하게 된다.
        let note = parse_note(request.mode, &text).map_err(|rejection| rejected_response(&rejection))?;

        Ok(NoteGeneration {
            note,
            model: self.model.clone(),
        })
    }
}

/// 그 mode의 유효한 표본 노트. 고정 값이며 실제 생성 결과가 아니다.
pub fn sample_note(mode: NoteType) -> StructuredNote {
    match mode {
        NoteType::Meeting => StructuredNote::Meeting(MeetingNote {
            overview: "다음 분기 일정과 남은 과제를 정리한 회의".to_owned(),
            key_discussions: vec!["일정 조정".to_owned(), "남은 과제".to_owned()],
            decisions: vec!["다음 주에 다시 정리한다".to_owned()],
            action_items: vec!["일정 초안 작성".to_owned()],
            open_questions: vec!["담당자는 누구인가".to_owned()],
        }),
        NoteType::Study => StructuredNote::Study(StudyNote {
            overview: "회의에서 다룬 개념 정리".to_owned(),
            key_concepts: vec!["분기 계획".to_owned()],
            important_details: vec!["일정은 다음 주에 확정된다".to_owned()],
            questions: vec!["무엇을 먼저 해야 하는가".to_owned()],
            things_to_study: vec!["일정 관리 방법".to_owned()],
            references_mentioned: vec![],
        }),
        NoteType::Summary => StructuredNote::Summary(SummaryNote {
            short_summary: "다음 분기 일정을 다음 주에 확정하기로 했다".to_owned(),
            key_points: vec!["일정 조정".to_owned(), "남은 과제 확인".to_owned()],
        }),
    }
}

/// [`sample_note`]를 **모델이 냈을 법한 텍스트**로 옮긴 것 — 봉투 없이 노트 객체만 담긴 JSON이다.
///
/// 저장 형태(`ai_notes.content`의 봉투)와 다르다. provider가 돌려주는 것은 저장 형태가 아니라
/// 생성 결과이며, 봉투를 씌우는 것은 저장하는 쪽의 일이다.
pub fn sample_note_text(mode: NoteType) -> String {
    let encoded = match sample_note(mode) {
        StructuredNote::Meeting(note) => serde_json::to_string(&note),
        StructuredNote::Study(note) => serde_json::to_string(&note),
        StructuredNote::Summary(note) => serde_json::to_string(&note),
    };

    encoded.expect("표본 노트는 String과 Vec<String>뿐이므로 직렬화에 실패할 값이 없다")
}

/// **어떤 [`NoteAiProvider`] 구현에도 적용할 수 있는** 계약 준수 검사 묶음.
///
/// 구현을 인자로 받고, 그 구현이 무엇을 돌려주든 **계약이 요구하는 성질만** 확인한다.
/// 특정 double을 겨냥한 단언이 없으므로 실제 adapter가 그대로 이 묶음을 통과해야 한다.
///
/// 여기서 확인하는 것은 셋이다.
///
/// ```text
/// descriptor()     자기 자신을 말할 수 있는가 — 식별자 · 이름 · 로컬/외부 (INV-5)
/// availability()   지금 쓸 수 있는지 답할 수 있는가, 그 답이 정합적인가 (INV-8)
/// generate_note()  요청한 mode의 노트를 내거나, §13의 공통 실패로만 거절하는가 (INV-3 · INV-9)
/// ```
///
/// # Panics
///
/// 계약이 깨진 자리에서 panic한다. 테스트에서 부르도록 만든 함수다.
pub fn assert_note_ai_provider_contract(provider: &dyn NoteAiProvider) {
    assert_descriptor_contract(provider);
    assert_availability_contract(provider);
    assert_generation_contract(provider);
}

/// provider가 자기 자신을 말하는 방식이 계약을 지키는가.
pub fn assert_descriptor_contract(provider: &dyn NoteAiProvider) {
    let descriptor = provider.descriptor();

    assert!(
        !descriptor.id.trim().is_empty(),
        "provider 식별자가 비어 있다 — 이 값은 ai_notes.provider에 그대로 남는 provenance다"
    );
    assert_eq!(
        descriptor.id,
        descriptor.id.trim(),
        "provider 식별자에 앞뒤 공백이 있다: {:?}",
        descriptor.id
    );
    assert!(
        !descriptor.name.trim().is_empty(),
        "사람이 읽을 이름이 비어 있다 — 화면이 그릴 것이 없다"
    );

    // 화면은 이 값을 읽어 사용자에게 알린다 (§12 · INV-5). 그래서 물어볼 때마다 같아야 한다.
    assert_eq!(
        provider.descriptor(),
        descriptor,
        "provider가 자기 자신을 말하는 답이 호출마다 달라진다"
    );
}

/// 가용성 답이 정합적인가 (INV-8).
pub fn assert_availability_contract(provider: &dyn NoteAiProvider) {
    match provider.availability() {
        Availability::Ready { models } => {
            assert!(
                !models.is_empty(),
                "쓸 수 있다고 답했는데 모델 목록이 비어 있다 — 그 상태는 NoModels다"
            );
            for model in &models {
                assert!(
                    !model.trim().is_empty(),
                    "모델 목록에 빈 이름이 있다 — 사용자가 고를 수 없는 항목이다"
                );
            }
        }
        Availability::NoModels => {}
        Availability::Unavailable(failure) => {
            assert_ai_provider_failure(&failure, "availability");
        }
    }
}

/// 생성이 계약을 지키는가 — **세 mode 전부**에 대해 확인한다.
pub fn assert_generation_contract(provider: &dyn NoteAiProvider) {
    for mode in NoteType::ALL {
        let request = NoteRequest::new(mode, CONTRACT_TRANSCRIPT);

        match provider.generate_note(&request) {
            Ok(generation) => {
                assert_eq!(
                    generation.note.mode(),
                    mode,
                    "요청한 mode와 다른 노트를 돌려줬다"
                );
                assert!(
                    !generation.model.trim().is_empty(),
                    "무엇이 만들었는지가 비어 있다 — provenance는 추정이 아니라 기록이어야 한다"
                );
            }
            Err(failure) => assert_ai_provider_failure(&failure, mode.as_str()),
        }
    }
}

/// 이 경계에서 나온 실패가 §13의 공통 실패인가 (INV-3 · INV-9).
fn assert_ai_provider_failure(failure: &Failure, context: &str) {
    assert!(
        AI_PROVIDER_FAILURE_KINDS.contains(&failure.kind),
        "{context}: AI provider 경계 밖의 실패로 거절했다: {}",
        failure.kind
    );
    assert!(
        !failure.message.trim().is_empty(),
        "{context}: 사용자에게 보여줄 문장이 없다 (§13)"
    );
    assert!(
        failure.source_data_safe,
        "{context}: AI 실패가 원본을 훼손했다고 말한다 — 이 경계는 오디오도 전사도 쓰지 않는다 (INV-3)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::note::{encode_content, ResponseRejection};
    use crate::ai::provider::{request_rejected, TranscriptText};
    use crate::domain::FailureKind;

    #[test]
    fn the_fake_passes_the_shared_contract_in_every_state() {
        // 같은 묶음이 네 상태를 전부 통과한다 — 이후의 실제 adapter도 이것을 그대로 통과한다.
        assert_note_ai_provider_contract(&FakeNoteAiProvider::ready());
        assert_note_ai_provider_contract(&FakeNoteAiProvider::unreachable());
        assert_note_ai_provider_contract(&FakeNoteAiProvider::without_models());
        assert_note_ai_provider_contract(&FakeNoteAiProvider::generating_text(
            "죄송하지만 JSON으로 답할 수 없습니다",
        ));
    }

    #[test]
    fn the_contract_suite_is_not_written_against_the_fake() {
        // 계약 묶음은 구현을 인자로 받는다. 여기서 만든 즉석 구현도 계약만 지키면 통과한다.
        struct AnotherProvider;

        impl NoteAiProvider for AnotherProvider {
            fn descriptor(&self) -> ProviderDescriptor {
                ProviderDescriptor {
                    id: "another-provider".to_owned(),
                    name: "다른 구현".to_owned(),
                    locality: Locality::External,
                }
            }

            fn availability(&self) -> Availability {
                Availability::NoModels
            }

            fn generate_note(&self, _request: &NoteRequest<'_>) -> Result<NoteGeneration, Failure> {
                Err(model_unavailable("쓸 수 있는 모델이 없다"))
            }
        }

        assert_note_ai_provider_contract(&AnotherProvider);
    }

    #[test]
    fn the_fake_gives_the_same_answer_to_the_same_request() {
        let first = FakeNoteAiProvider::ready();
        let second = FakeNoteAiProvider::ready();
        let request = NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT);

        let one = first.generate_note(&request).expect("표본 노트를 낸다");
        let again = first.generate_note(&request).expect("두 번째도 같다");
        let other = second.generate_note(&request).expect("다른 인스턴스도 같다");

        assert_eq!(one, again);
        assert_eq!(one, other);
        assert_eq!(one.model, FAKE_MODEL_ID);
    }

    #[test]
    fn every_mode_gets_a_note_of_that_mode() {
        let provider = FakeNoteAiProvider::ready();

        for mode in NoteType::ALL {
            let generation = provider
                .generate_note(&NoteRequest::new(mode, CONTRACT_TRANSCRIPT))
                .expect("표본 노트를 낸다");

            assert_eq!(generation.note.mode(), mode);
            // 돌려준 노트는 그대로 저장 형태로 옮길 수 있다 — 반쯤 채워진 값이 아니다.
            assert!(encode_content(&generation.note).contains(mode.as_str()));
        }
    }

    #[test]
    fn a_response_that_breaks_the_schema_becomes_the_response_failure() {
        for broken in [
            "죄송하지만 JSON으로 답할 수 없습니다",
            "{}",
            r#"{"overview":"   ","keyDiscussions":[],"decisions":[],"actionItems":[],"openQuestions":[]}"#,
        ] {
            let provider = FakeNoteAiProvider::generating_text(broken);

            let failure = provider
                .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
                .expect_err("기대 schema와 다른 응답은 실패다");

            assert_eq!(failure.kind, FailureKind::AiResponseUnusable);
            assert!(failure.retryable, "생성은 결정론적이지 않다");
            assert!(failure.source_data_safe, "원본은 건드리지 않았다 (INV-3)");
            assert!(failure.detail.is_some(), "무엇이 달랐는지가 남는다");
        }
    }

    #[test]
    fn the_double_obeys_the_same_parsing_contract_as_a_real_adapter() {
        // double이 임의의 텍스트를 성공으로 돌려줄 수 있으면, 제품이 막는 것을 테스트가
        // 통과시킨다. 코드펜스에 싸인 정상 JSON은 제품과 똑같이 회수된다.
        let fenced = format!("```json\n{}\n```", sample_note_text(NoteType::Summary));
        let provider = FakeNoteAiProvider::generating_text(fenced);

        let generation = provider
            .generate_note(&NoteRequest::new(NoteType::Summary, CONTRACT_TRANSCRIPT))
            .expect("회수할 수 있는 응답이다");

        assert_eq!(generation.note, sample_note(NoteType::Summary));
    }

    #[test]
    fn the_unreachable_fake_says_the_same_thing_twice() {
        let provider = FakeNoteAiProvider::unreachable();

        let Availability::Unavailable(reported) = provider.availability() else {
            panic!("응답하지 않는 provider는 Unavailable이다");
        };
        assert_eq!(reported.kind, FailureKind::AiProviderUnreachable);
        assert!(reported.retryable, "서버를 켜고 다시 시도할 수 있다");

        let failure = provider
            .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
            .expect_err("연결되지 않으면 생성도 실패한다");
        assert_eq!(failure.kind, FailureKind::AiProviderUnreachable);
    }

    #[test]
    fn the_fake_without_models_is_a_different_state_from_being_unreachable() {
        let provider = FakeNoteAiProvider::without_models();

        assert_eq!(provider.availability(), Availability::NoModels);
        assert_eq!(
            provider
                .generate_note(&NoteRequest::new(NoteType::Study, CONTRACT_TRANSCRIPT))
                .expect_err("모델이 없으면 생성할 수 없다")
                .kind,
            FailureKind::AiModelUnavailable
        );
    }

    #[test]
    fn the_fake_repeats_its_last_answer_for_later_calls() {
        let provider = FakeNoteAiProvider::responding_with(vec![
            FakeResponse::Rejected(request_rejected("첫 요청이 거절됐다")),
            FakeResponse::SampleNote,
        ]);
        let request = NoteRequest::new(NoteType::Summary, CONTRACT_TRANSCRIPT);

        let first = provider.generate_note(&request);
        let second = provider.generate_note(&request);
        let third = provider.generate_note(&request);

        assert_eq!(
            first.expect_err("첫 시도는 실패다").kind,
            FailureKind::AiRequestFailed
        );
        assert!(second.is_ok());
        assert_eq!(third.expect("마지막 응답이 되풀이된다").note, sample_note(NoteType::Summary));
        assert_eq!(provider.call_count(), 3);
    }

    #[test]
    fn the_fake_records_what_it_was_asked_to_generate_and_nothing_more() {
        let provider = FakeNoteAiProvider::ready();
        let request = NoteRequest::new(NoteType::Study, "말해진 것");

        provider.generate_note(&request).expect("표본 노트를 낸다");

        // 기록에 오디오가 없는 것은 double의 선택이 아니다 — 계약이 오디오를 옮기지 않는다 (INV-6).
        assert_eq!(
            provider.calls(),
            vec![FakeCall {
                mode: NoteType::Study,
                transcript: "말해진 것".to_owned(),
                context_tokens: crate::ai::prompt::ContextBudget::DEFAULT.context_tokens,
            }]
        );
    }

    #[test]
    fn the_fake_can_stand_in_for_any_boundary_failure() {
        let provider = FakeNoteAiProvider::failing(rejected_response(&ResponseRejection::NotJson));

        assert_eq!(
            provider
                .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
                .expect_err("실패를 돌려준다")
                .kind,
            FailureKind::AiResponseUnusable
        );
    }

    #[test]
    fn the_fake_can_speak_for_an_external_provider_too() {
        // §12 · INV-5의 표시를 화면이 그리는지 검증하려면 double이 두 값을 다 낼 수 있어야 한다.
        let external = FakeNoteAiProvider::ready().with_locality(Locality::External);

        assert_eq!(external.descriptor().locality, Locality::External);
        assert!(FakeNoteAiProvider::ready().descriptor().locality.is_local());
        assert_note_ai_provider_contract(&external);
    }

    #[test]
    fn the_fake_never_claims_to_be_a_real_provider() {
        let descriptor = FakeNoteAiProvider::ready().descriptor();

        // 이 식별자가 저장된 ai_notes 행은 테스트가 만든 것이다.
        assert_eq!(descriptor.id, FAKE_PROVIDER_ID);
        assert!(descriptor.id.starts_with("fake-"));
        assert_eq!(
            FakeNoteAiProvider::ready()
                .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
                .expect("표본 노트를 낸다")
                .model,
            FAKE_MODEL_ID
        );
    }

    #[test]
    fn product_code_never_constructs_the_fake_provider() {
        // fake는 사용자가 고를 수 있는 provider가 아니다. 제품 경로가 이것을 만들기 시작하면
        // 그 사실이 소스에 남으므로, 여기서 그것을 막는다 — 관례가 아니라 테스트로.
        let mut offenders: Vec<String> = Vec::new();
        let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

        for path in rust_sources(&source_root) {
            if path.ends_with("ai/testing.rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("소스 파일을 읽는다");
            // 테스트 모듈 안에서 쓰는 것은 제품 경로가 아니다.
            let test_boundary = source.find("#[cfg(test)]").unwrap_or(usize::MAX);

            if source
                .match_indices("FakeNoteAiProvider::")
                .any(|(at, _)| at < test_boundary)
            {
                offenders.push(path.display().to_string());
            }
        }

        assert!(
            offenders.is_empty(),
            "제품 경로가 test double을 provider로 만든다: {offenders:?}"
        );
    }

    /// `src` 아래의 모든 `.rs` 파일. 외부 crate 없이 훑는다.
    fn rust_sources(directory: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        let entries = std::fs::read_dir(directory).expect("소스 디렉터리를 읽는다");

        for entry in entries {
            let path = entry.expect("디렉터리 항목을 읽는다").path();
            if path.is_dir() {
                files.extend(rust_sources(&path));
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }

        files
    }

    #[test]
    fn the_request_the_fake_receives_can_only_carry_text() {
        // INV-6을 double 쪽에서도 확인한다 — 넘길 수 있는 것이 텍스트 하나뿐이다.
        let request = NoteRequest {
            mode: NoteType::Meeting,
            transcript: TranscriptText::new("말해진 것"),
            context_budget: crate::ai::prompt::ContextBudget::DEFAULT,
        };

        assert_eq!(request.transcript_text(), "말해진 것");
    }
}
