//! mode별 프롬프트 · `promptVersion` · transcript를 프롬프트 입력으로 준비하는 자리
//! (ADR-0008 §8 · §10).
//!
//! ## 프롬프트가 바뀌면 `promptVersion`이 **반드시** 바뀐다
//!
//! "바꾸는 것을 잊지 말자"는 정책이 아니라 **잊을 수 없게 만드는 것**이 §10의 결정이다.
//!
//! ```text
//! 프롬프트를 한 글자 고친다
//!   → 계산된 hash8이 달라진다
//!   → 아래 선언된 상수와 다르다
//!   → test Gate가 깨진다
//!   → 선언값을 고치기 전에는 통과할 수 없다
//! ```
//!
//! 해시 대상은 **프롬프트 원문 ‖ 그 모드로 요구하는 JSON Schema ‖ content schemaVersion**이다.
//! 같은 문장이라도 요구한 출력 형태가 달라지면 나온 노트가 달라지기 때문이다 — 노트를 만든 것은
//! 프롬프트 혼자가 아니다 (ADR-0008 §10.2-3).
//!
//! 해시 함수는 이 파일 안에 직접 있다(FNV-1a 64bit). `std::hash::DefaultHasher`를 쓰지 않는다 —
//! 그 값은 Rust 버전 간 안정성이 보장되지 않으므로, 툴체인을 올렸다는 이유로 **이미 저장된
//! provenance와 계산값이 어긋날 수 있다.** 영속되는 값은 재현 가능해야 한다 (§10.2-5).
//!
//! ## context 전략 — 청킹도 절단도 하지 않는다
//!
//! ADR-0008 §8.2가 셋 중에서 고른 것은 **"context 크기를 명시하고, 보내기 전에 미리 판정한다"**다.
//!
//! - **절단하지 않는다.** 잘린 transcript로 만든 노트는 완전해 보이지만 완전하지 않다.
//! - **청킹하지 않는다** (DEFERRED). 부분 결과 병합의 품질은 이 Phase에서 UNVERIFIED인 것에
//!   달려 있고, 검증되지 않은 것 위에 두 번째 미검증 층을 쌓지 않는다.
//! - 대신 [`prepare`]가 **보내기 전에** 크기를 보수적으로 추정해 예산과 비교하고, 넘치면
//!   [`ContextOverflow`]로 돌려준다. 사용자가 그 사실을 본다.
//!
//! 이 모듈은 그 예산 값을 **어디에 어떤 이름으로 실어 보내는지 알지 않는다** — 그것은 adapter
//! 하나의 지식이다 (INV-9).

use crate::domain::NoteType;

use super::note::{json_schema, CONTENT_SCHEMA_VERSION};

/// 사람이 올리는 프롬프트 세트 버전 (ADR-0008 §10.2). 첫 값은 1이다.
pub const PROMPT_SET_VERSION: u32 = 1;

/// 프롬프트 안에서 transcript 텍스트가 들어가는 **단 한 자리**.
///
/// 실행 중에 조립되는 부분은 이것뿐이다 — 프롬프트는 정적 상수이며, 커스터마이즈 UI는
/// DEFERRED다 (ADR-0008 §10.2-1 · §15).
pub const TRANSCRIPT_PLACEHOLDER: &str = "{{transcript}}";

/// Meeting 프롬프트. 요구하는 필드는 §7.2의 Meeting schema와 같다.
pub const MEETING_PROMPT: &str = "\
You turn a meeting transcript into a structured note.

Return exactly one JSON object and nothing else. No prose before or after it, no code fence,
no explanation.

The object has exactly these keys:
- \"overview\": string. A short paragraph describing what the meeting was about.
- \"keyDiscussions\": array of strings. The subjects that were actually discussed.
- \"decisions\": array of strings. Decisions that were actually made.
- \"actionItems\": array of strings. Work that someone agreed to do. Keep the wording of the
  transcript, including who does it and by when if that was said.
- \"openQuestions\": array of strings. Questions raised but left unresolved.

Rules:
- Use only what the transcript says. Do not invent decisions, owners, dates or facts.
- Every key must be present. If a section has nothing, use an empty array.
- One item is one line of plain text. Do not nest objects or arrays inside an item.
- Write the note in the main language of the transcript.

Transcript:
{{transcript}}
";

/// Study 프롬프트. 요구하는 필드는 §7.2의 Study schema와 같다.
pub const STUDY_PROMPT: &str = "\
You turn a lecture or study session transcript into a structured note.

Return exactly one JSON object and nothing else. No prose before or after it, no code fence,
no explanation.

The object has exactly these keys:
- \"overview\": string. A short paragraph describing what was taught or studied.
- \"keyConcepts\": array of strings. The concepts the material is built on.
- \"importantDetails\": array of strings. Details worth remembering.
- \"questions\": array of strings. Questions that help check understanding of this material.
- \"thingsToStudy\": array of strings. What to study next, based on what was said.
- \"referencesMentioned\": array of strings. Books, papers, tools or links that were actually
  mentioned. Do not add references that were not mentioned.

Rules:
- Use only what the transcript says. Do not invent facts, sources or numbers.
- Every key must be present. If a section has nothing, use an empty array.
- One item is one line of plain text. Do not nest objects or arrays inside an item.
- Write the note in the main language of the transcript.

Transcript:
{{transcript}}
";

/// Summary 프롬프트. 요구하는 필드는 §7.2의 Summary schema와 같다.
pub const SUMMARY_PROMPT: &str = "\
You turn a transcript into a short structured summary.

Return exactly one JSON object and nothing else. No prose before or after it, no code fence,
no explanation.

The object has exactly these keys:
- \"shortSummary\": string. A few sentences covering the whole transcript.
- \"keyPoints\": array of strings. The points a reader should take away.

Rules:
- Use only what the transcript says. Do not invent facts.
- Every key must be present. If there are no key points, use an empty array.
- One item is one line of plain text. Do not nest objects or arrays inside an item.
- Write the summary in the main language of the transcript.

Transcript:
{{transcript}}
";

/// Meeting 프롬프트의 `promptVersion`.
///
/// **손으로 선언한 값이다.** 프롬프트나 schema를 고치면 [`computed_prompt_version`]이 다른 값을
/// 내고 `prompt_version_is_bound_to_the_prompt_text` 테스트가 깨진다. 그때 이 값을 함께 고친다.
pub const PROMPT_VERSION_MEETING: &str = "v1.meeting.5c6b8a90";

/// Study 프롬프트의 `promptVersion`. [`PROMPT_VERSION_MEETING`]과 같은 규칙이다.
pub const PROMPT_VERSION_STUDY: &str = "v1.study.2cfad9a0";

/// Summary 프롬프트의 `promptVersion`. [`PROMPT_VERSION_MEETING`]과 같은 규칙이다.
pub const PROMPT_VERSION_SUMMARY: &str = "v1.summary.beca7d6c";

/// 프롬프트 크기로 예약해 두는 토큰 수 (ADR-0008 §8.5).
pub const PROMPT_RESERVE_TOKENS: u32 = 1_024;

/// 출력이 차지할 자리로 예약해 두는 토큰 수 (ADR-0008 §8.5).
pub const OUTPUT_RESERVE_TOKENS: u32 = 1_536;

/// 추정에 쓰는 문자/토큰 비율 (ADR-0008 §8.5).
///
/// **tokenizer 사실이 아니라 안전한 과대추정이다** — 앱은 모델의 tokenizer를 갖고 있지 않다.
/// 과대추정이어야 "들어간다"가 "거의 확실히 들어간다"를 뜻한다. 과소추정하면 이 가드는 존재
/// 이유를 잃는다. 이 값을 노트 품질이나 비용 계산에 쓰지 않는다.
pub const CHARS_PER_ESTIMATED_TOKEN: usize = 2;

/// 한 번의 생성이 쓸 수 있는 context 크기.
///
/// **서버 기본값에 기대지 않는다** (ADR-0008 §8.2-1). 사용자가 서버 쪽 설정을 만졌는지 앱은 알 수
/// 없고, 알 수 없는 값 위에서 결과가 달라지면 재현할 수 없다. 그래서 값은 언제나 이 타입으로
/// 함께 다니며, adapter는 그것을 요청에 명시한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextBudget {
    /// 이 요청이 쓸 context 크기(토큰).
    pub context_tokens: u32,
}

impl ContextBudget {
    /// 설정에 저장된 값이 아직 없을 때 쓰는 시작값 (ADR-0008 §8.4).
    ///
    /// **검증된 용량 수치가 아니라 시작값이다.** 어떤 기기에서 어느 모델이 이 값을 감당하는지는
    /// 확인되지 않았고(UNVERIFIED), 상한은 사용자의 RAM/VRAM에 달려 있어 앱이 알 수 없다.
    /// 그래서 값을 설정으로 노출하고 기본값을 하나 고른다.
    pub const DEFAULT: Self = Self {
        context_tokens: 16_384,
    };

    /// transcript가 쓸 수 있는 몫. 프롬프트 지시문과 출력의 예약분을 뺀 나머지다.
    pub fn input_tokens(&self) -> u32 {
        self.context_tokens
            .saturating_sub(PROMPT_RESERVE_TOKENS)
            .saturating_sub(OUTPUT_RESERVE_TOKENS)
    }
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// 보낼 준비가 끝난 프롬프트 하나.
///
/// **여기에는 제공자가 없다.** 무엇에게 어떤 이름의 파라미터로 실어 보낼지는 adapter가 안다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPrompt {
    pub mode: NoteType,
    /// transcript가 끼워진 최종 프롬프트 텍스트.
    pub prompt: String,
    /// 이 프롬프트를 만든 상수의 버전. 그대로 `ai_notes.prompt_version`에 남는다.
    pub prompt_version: &'static str,
    /// 이 요청에 명시할 context 크기.
    pub context_tokens: u32,
    /// transcript에 대한 보수적 추정치. 판정의 근거이지 정확한 토큰 수가 아니다.
    pub estimated_input_tokens: u32,
}

/// 입력이 context 예산을 넘었다 (ADR-0008 §8.2-3 · §13.1의 여섯 번째 실패).
///
/// **요청을 보내지 않았다는 뜻이다.** 잘라서 보내지 않는다 — 사용자가 예산을 키우거나, 더 큰
/// context의 모델을 고르거나, 더 짧은 녹음을 고를 수 있다. 이 값을 §13의 domain 공통 실패로
/// 옮기는 것은 계약을 만드는 쪽의 일이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextOverflow {
    pub estimated_input_tokens: u32,
    pub available_input_tokens: u32,
    pub context_tokens: u32,
}

impl std::fmt::Display for ContextOverflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "입력 추정 {} 토큰이 사용 가능한 {} 토큰(context {})을 넘는다",
            self.estimated_input_tokens, self.available_input_tokens, self.context_tokens
        )
    }
}

/// 그 mode의 프롬프트 원문. transcript가 아직 끼워지지 않은 상태다.
pub fn prompt_template(mode: NoteType) -> &'static str {
    match mode {
        NoteType::Meeting => MEETING_PROMPT,
        NoteType::Study => STUDY_PROMPT,
        NoteType::Summary => SUMMARY_PROMPT,
    }
}

/// 그 mode의 `promptVersion` — **선언된 값**이다.
///
/// 계산값을 그대로 돌려주지 않는 이유가 이 설계의 핵심이다. 계산값을 쓰면 프롬프트를 고칠 때
/// 값이 조용히 따라 바뀌고, 아무도 그 변화를 보지 못한다. 선언값을 쓰면 **테스트가 먼저 깨진다.**
pub fn prompt_version(mode: NoteType) -> &'static str {
    match mode {
        NoteType::Meeting => PROMPT_VERSION_MEETING,
        NoteType::Study => PROMPT_VERSION_STUDY,
        NoteType::Summary => PROMPT_VERSION_SUMMARY,
    }
}

/// 지금 상수들로부터 계산한 `promptVersion`.
///
/// [`prompt_version`]과 다르면 프롬프트나 schema가 바뀌었는데 선언값이 따라오지 않은 것이다.
/// 그 상태는 테스트가 통과시키지 않는다.
pub fn computed_prompt_version(mode: NoteType) -> String {
    version_of(mode, prompt_template(mode), &schema_material(mode))
}

/// transcript를 끼운 프롬프트 텍스트.
pub fn build_prompt(mode: NoteType, transcript: &str) -> String {
    prompt_template(mode).replace(TRANSCRIPT_PLACEHOLDER, transcript)
}

/// transcript 텍스트의 크기를 **보수적으로** 추정한다 (ADR-0008 §8.5).
///
/// 문자 수를 [`CHARS_PER_ESTIMATED_TOKEN`]으로 나누어 올림한다. 표현할 수 없을 만큼 긴
/// 입력에서는 `u32::MAX`로 접는다 — 과대추정 쪽이므로 가드는 그대로 작동한다.
pub fn estimate_tokens(text: &str) -> u32 {
    let characters = text.chars().count();
    u32::try_from(characters.div_ceil(CHARS_PER_ESTIMATED_TOKEN)).unwrap_or(u32::MAX)
}

/// transcript 텍스트를 프롬프트 입력으로 준비한다 — **보내기 전에 크기를 판정하는 자리다.**
///
/// 넘치면 프롬프트를 만들지 않고 [`ContextOverflow`]를 돌려준다. 자르지 않는다.
pub fn prepare(
    mode: NoteType,
    transcript: &str,
    budget: ContextBudget,
) -> Result<PreparedPrompt, ContextOverflow> {
    let estimated_input_tokens = estimate_tokens(transcript);
    let available_input_tokens = budget.input_tokens();

    if estimated_input_tokens > available_input_tokens {
        return Err(ContextOverflow {
            estimated_input_tokens,
            available_input_tokens,
            context_tokens: budget.context_tokens,
        });
    }

    Ok(PreparedPrompt {
        mode,
        prompt: build_prompt(mode, transcript),
        prompt_version: prompt_version(mode),
        context_tokens: budget.context_tokens,
        estimated_input_tokens,
    })
}

/// 해시 대상 중 schema 쪽 재료. 같은 schema는 언제나 같은 문자열이 된다.
fn schema_material(mode: NoteType) -> String {
    json_schema(mode).to_string()
}

/// `v<SET>.<mode>.<hash8>` (ADR-0008 §10.2-2).
///
/// 재료를 인자로 받는 이유는 **테스트가 프롬프트를 바꿔 보고 값이 따라 바뀌는지 확인할 수 있게
/// 하기 위해서다.** 제품 경로는 [`computed_prompt_version`] 하나뿐이다.
fn version_of(mode: NoteType, template: &str, schema: &str) -> String {
    // 구분자(unit separator)를 넣어 두 재료의 경계가 섞이지 않게 한다.
    let mut material = String::with_capacity(template.len() + schema.len() + 16);
    material.push_str(template);
    material.push('\u{1f}');
    material.push_str(schema);
    material.push('\u{1f}');
    material.push_str(&CONTENT_SCHEMA_VERSION.to_string());

    let digest = fnv1a64(material.as_bytes());
    format!(
        "v{PROMPT_SET_VERSION}.{mode}.{:08x}",
        (digest >> 32) as u32
    )
}

/// FNV-1a 64bit. 값이 Rust 버전이나 플랫폼에 따라 달라지지 않는다 (ADR-0008 §10.2-5).
fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_version_is_bound_to_the_prompt_text() {
        // ⚠️ 이 테스트가 깨졌다면 프롬프트나 schema를 고친 것이다. 그것은 정상이며, 해야 할 일은
        // **선언된 상수를 계산값으로 고치는 것**이다 (ADR-0008 §10.2-4). 계산 쪽을 고치거나
        // 이 테스트를 무르게 만들면 값이 조용히 어긋나고, 저장된 provenance가 거짓이 된다.
        // 세 mode를 한 번에 본다 — 첫 mode에서 멈추면 나머지 두 값을 고치려고 테스트를 다시
        // 돌려야 한다.
        let drifted: Vec<String> = NoteType::ALL
            .into_iter()
            .filter(|mode| prompt_version(*mode) != computed_prompt_version(*mode))
            .map(|mode| format!("{mode}: 선언 {} ≠ 계산 {}", prompt_version(mode), computed_prompt_version(mode)))
            .collect();

        assert!(
            drifted.is_empty(),
            "프롬프트/schema가 바뀌었는데 선언된 promptVersion이 그대로다 — 선언값을 계산값으로 고친다:\n{}",
            drifted.join("\n")
        );
    }

    #[test]
    fn changing_one_character_of_the_prompt_changes_the_version() {
        for mode in NoteType::ALL {
            let template = prompt_template(mode);
            let schema = schema_material(mode);
            let declared = version_of(mode, template, &schema);

            let edited = format!("{template} ");
            assert_ne!(
                version_of(mode, &edited, &schema),
                declared,
                "{mode}: 프롬프트가 바뀌었는데 버전이 같다"
            );

            let reworded = template.replacen("Return exactly one JSON object", "Return JSON", 1);
            assert_ne!(reworded, template, "치환 대상이 프롬프트에 있어야 한다");
            assert_ne!(version_of(mode, &reworded, &schema), declared);
        }
    }

    #[test]
    fn changing_the_required_schema_changes_the_version() {
        // 노트를 만든 것은 프롬프트 혼자가 아니다 (ADR-0008 §10.2-3).
        for mode in NoteType::ALL {
            let template = prompt_template(mode);
            let schema = schema_material(mode);

            assert_ne!(
                version_of(mode, template, &schema.replace("string", "number")),
                version_of(mode, template, &schema),
                "{mode}: 요구하는 schema가 바뀌었는데 버전이 같다"
            );
        }
    }

    #[test]
    fn each_mode_has_its_own_version_value() {
        let versions: Vec<&str> = NoteType::ALL.into_iter().map(prompt_version).collect();

        for (index, version) in versions.iter().enumerate() {
            assert!(
                version.starts_with(&format!("v{PROMPT_SET_VERSION}.")),
                "세트 버전이 값에 있어야 한다: {version}"
            );
            assert!(
                version.contains(NoteType::ALL[index].as_str()),
                "mode가 값에 있어야 한다: {version}"
            );
            assert!(
                !versions[..index].contains(version),
                "모드끼리 값이 겹친다: {version}"
            );
        }
    }

    #[test]
    fn the_hash_function_does_not_depend_on_the_toolchain() {
        // 알려진 FNV-1a 64bit 값. 이 값이 달라지면 이미 저장된 promptVersion과 계산값이
        // 어긋난다 (ADR-0008 §10.2-5).
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn every_prompt_has_exactly_one_place_for_the_transcript() {
        for mode in NoteType::ALL {
            let template = prompt_template(mode);
            assert_eq!(
                template.matches(TRANSCRIPT_PLACEHOLDER).count(),
                1,
                "{mode}: transcript 자리는 하나다"
            );
        }
    }

    #[test]
    fn the_prompt_asks_for_exactly_the_fields_of_the_schema() {
        // 프롬프트가 요구하는 키와 schema가 요구하는 키가 어긋나면, 모델은 둘 중 하나를 어긴다.
        for (mode, fields) in [
            (NoteType::Meeting, crate::ai::note::MEETING_FIELDS.as_slice()),
            (NoteType::Study, crate::ai::note::STUDY_FIELDS.as_slice()),
            (NoteType::Summary, crate::ai::note::SUMMARY_FIELDS.as_slice()),
        ] {
            let template = prompt_template(mode);
            for field in fields {
                assert!(
                    template.contains(&format!("\"{field}\"")),
                    "{mode}: 프롬프트가 {field}를 요구하지 않는다"
                );
            }
        }
    }

    #[test]
    fn building_a_prompt_puts_the_transcript_in_and_changes_nothing_else() {
        let prompt = build_prompt(NoteType::Meeting, "안녕하세요. 회의를 시작하겠습니다.");

        assert!(prompt.contains("안녕하세요. 회의를 시작하겠습니다."));
        assert!(!prompt.contains(TRANSCRIPT_PLACEHOLDER), "자리표시자가 남지 않는다");
        assert!(prompt.starts_with("You turn a meeting transcript"));
    }

    #[test]
    fn the_estimate_is_a_conservative_over_estimate() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1, "올림한다");
        assert_eq!(estimate_tokens("ab"), 1);
        assert_eq!(estimate_tokens("abc"), 2);
        // 문자 수로 센다 — 바이트 수가 아니다. 한글 한 글자는 UTF-8에서 3바이트다.
        assert_eq!(estimate_tokens("가나"), 1);
    }

    #[test]
    fn the_budget_keeps_room_for_the_instructions_and_the_output() {
        let budget = ContextBudget::DEFAULT;

        assert_eq!(budget.context_tokens, 16_384);
        assert_eq!(
            budget.input_tokens(),
            16_384 - PROMPT_RESERVE_TOKENS - OUTPUT_RESERVE_TOKENS
        );
        // 예약분보다 작은 예산에서도 밑돌지 않는다.
        assert_eq!(ContextBudget { context_tokens: 1 }.input_tokens(), 0);
    }

    #[test]
    fn a_transcript_that_fits_is_prepared_with_its_version_and_context_size() {
        let prepared = prepare(NoteType::Study, "짧은 전사", ContextBudget::DEFAULT)
            .expect("들어가야 한다");

        assert_eq!(prepared.mode, NoteType::Study);
        assert_eq!(prepared.prompt_version, PROMPT_VERSION_STUDY);
        assert_eq!(prepared.context_tokens, 16_384);
        assert_eq!(prepared.estimated_input_tokens, estimate_tokens("짧은 전사"));
        assert!(prepared.prompt.contains("짧은 전사"));
    }

    #[test]
    fn a_transcript_that_does_not_fit_is_not_truncated_and_not_sent() {
        let budget = ContextBudget {
            context_tokens: PROMPT_RESERVE_TOKENS + OUTPUT_RESERVE_TOKENS + 10,
        };
        let transcript = "가".repeat(1_000);

        let overflow = prepare(NoteType::Meeting, &transcript, budget)
            .expect_err("넘쳐야 한다");

        assert_eq!(overflow.available_input_tokens, 10);
        assert_eq!(overflow.estimated_input_tokens, 500);
        assert_eq!(overflow.context_tokens, budget.context_tokens);
        // 값 하나로 사용자가 무엇을 할 수 있는지가 드러난다 — 예산을 키우거나 더 짧은 녹음을 고른다.
        assert!(overflow.to_string().contains("500"));
    }

    #[test]
    fn the_boundary_of_the_budget_is_inclusive() {
        let budget = ContextBudget {
            context_tokens: PROMPT_RESERVE_TOKENS + OUTPUT_RESERVE_TOKENS + 5,
        };

        assert!(prepare(NoteType::Summary, &"a".repeat(10), budget).is_ok(), "정확히 예산만큼은 들어간다");
        assert!(prepare(NoteType::Summary, &"a".repeat(11), budget).is_err());
    }
}
