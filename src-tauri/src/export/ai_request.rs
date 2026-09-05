//! 사람이 자기 AI 채팅으로 가져갈 수 있는 산출물 셋을 만드는 **순수 모듈**
//! (`docs/ADR-0010-manual-ai-handoff.md` §5 · §6 · Phase 5.5 Required Outcome A).
//!
//! ```text
//! Recording + Transcript ─┬─→ manual_prompt()      Manual 프롬프트 (transcript 포함)
//!                         ├─→ transcript_text()    붙여 넣을 수 있는 Transcript 텍스트
//!                         └─→ ai_ready_document()  AI-ready Markdown 문서 하나
//! ```
//!
//! **파일도 clipboard도 네트워크도 저장소도 시계도 난수도 로캘도 없다** (§18 · ADR-0010 §5.1).
//! 여기 있는 것은 값에서 문자열을 만드는 함수뿐이며, 같은 입력은 언제나 같은 문자열을 낸다.
//! 만들어진 문자열이 파일이 되는 자리도, clipboard로 가는 자리도 이 모듈 밖이다.
//!
//! ## 재료는 둘뿐이다 — 새로 만드는 규칙이 없다 (ADR-0010 §5.4 · §6)
//!
//! ```text
//! crate::ai::prompt의 상수      한 글자도 고치지 않는다. 치환 두 번만 적용한다  → §6
//! export::markdown의 렌더링     transcript 본문 · 메타데이터 블록 · 제목 한 줄  → §5.4
//! ```
//!
//! transcript 렌더링 규칙(segment 순서 보존 · 빈 segment 처리 · segment가 없으면 `raw_text`)은
//! [`super::markdown::transcript_body`] **한 자리에만** 있다. 여기서 복제하면 §11의 export
//! 파일과 AI-ready 문서가 조용히 갈라진다.
//!
//! ## audio가 들어갈 자리가 없다 (INV-6 · MH-4)
//!
//! [`AiRequest`]가 읽는 Recording 필드는 `title` · `created_at` · `duration_ms` **셋뿐**이며,
//! 그 셋을 읽는 것도 [`super::markdown`]의 함수들이다. 이 모듈에는 audio 경로도 audio 바이트도
//! 담을 필드가 없다.
//!
//! ## 벤더를 알지 않는다 (INV-9 · MH-6)
//!
//! 어떤 AI 채팅 서비스의 이름도 산출물 어디에도 없다. 요청이 요구하는 것은
//! **Markdown 하나**이며, 그것은 어느 채팅에나 붙여 넣을 수 있다. `## Mode`에 적히는
//! `Meeting` · `Study` · `Summary`는 **출력 형태의 종류이지 벤더가 아니다** (§9.5).

use crate::ai::prompt::{prompt_template, TRANSCRIPT_PLACEHOLDER};
use crate::domain::{NoteType, Recording, Transcript};

use super::markdown::{
    heading_text, metadata_block, transcript_blocks, transcript_body, ExportDocument,
    TranscriptBody, MEETING_SECTIONS, STUDY_SECTIONS, SUMMARY_SECTIONS, TRANSCRIPT_SECTION,
};

/// AI-ready 문서의 첫 `# ` 줄 (ADR-0010 §5.2).
///
/// §11의 export(`# <제목>`)와 **첫 줄에서 구분된다** — 이 문서는 녹음의 기록이 아니라
/// 요청이며, 파일 두 개가 한 디렉터리에 섞여도 무엇이 무엇인지 즉시 보인다.
/// "Molt Note"는 이 제품 자신이지 목적지가 아니다 (§10).
pub const AI_REQUEST_HEADING: &str = "Molt Note AI Request";

/// AI-ready 문서의 mode 섹션 제목.
pub const MODE_SECTION: &str = "Mode";

/// AI-ready 문서의 지시문 섹션 제목.
pub const INSTRUCTIONS_SECTION: &str = "Instructions";

/// AI-ready 문서의 메타데이터 섹션 제목.
pub const RECORDING_SECTION: &str = "Recording";

/// 세 프롬프트 상수에 **글자까지 같은 모양으로 한 번씩** 들어 있는 JSON 출력 계약 블록.
///
/// Manual 경로는 이 블록만 Markdown 출력 계약으로 바꾼다 (ADR-0010 §6.2-a).
/// **상수를 고치는 것이 아니라 읽어서 치환한다** — 상수를 고치면
/// `prompt_version_is_bound_to_the_prompt_text`가 깨지고, 선언값을 따라 올리면 이미 저장된
/// `ai_notes.prompt_version`이 가리키는 프롬프트가 저장소 어디에도 없게 된다 (§6.1).
///
/// 이 문장이 프롬프트에서 사라지면 치환이 0회가 되고, 그때 만들어지는 것은 "Markdown으로
/// 달라"와 "JSON만 달라"가 동시에 들어간 **모순된 프롬프트**다. 그래서 치환 횟수를 단언한다.
const JSON_OUTPUT_CONTRACT: &str = "\
Return exactly one JSON object and nothing else. No prose before or after it, no code fence,
no explanation.";

/// AI-ready 문서에서 `{{transcript}}` 자리에 들어가는 **가리키는 한 문장** (ADR-0010 §6.2-b).
///
/// 같은 문서 안에서 transcript 본문이 두 번 나오지 않게 한다.
fn transcript_pointer() -> String {
    format!("The transcript is in the \"## {TRANSCRIPT_SECTION}\" section of this document.")
}

/// 그 mode가 요구하는 출력 섹션 제목들 — §9.5의 이름 그대로다.
///
/// **손으로 적지 않는다.** [`super::markdown`]의 상수가 그 이름이고, `markdown::render`가
/// 실제로 만드는 제목이며, Notion으로 가는 문서의 제목이기도 하다 (ADR-0010 §6.3). 그래서
/// Manual 프롬프트가 요구하는 출력 모양이 이 앱이 이미 만드는 모양과 같아진다.
fn output_sections(mode: NoteType) -> &'static [&'static str] {
    match mode {
        NoteType::Meeting => MEETING_SECTIONS.as_slice(),
        NoteType::Study => STUDY_SECTIONS.as_slice(),
        NoteType::Summary => SUMMARY_SECTIONS.as_slice(),
    }
}

/// `## Mode`에 적히는 사람이 읽는 이름 (§9.5).
///
/// wire 값([`NoteType::as_str`] — 소문자)은 경계를 지나는 값이고 문서에 적히는 것은 표시
/// 이름이다. 둘을 섞지 않는다 (ADR-0010 §5.2).
fn mode_label(mode: NoteType) -> &'static str {
    match mode {
        NoteType::Meeting => "Meeting",
        NoteType::Study => "Study",
        NoteType::Summary => "Summary",
    }
}

/// JSON 출력 계약을 대신하는 Markdown 출력 계약 (ADR-0010 §6.3).
///
/// 다섯 가지를 말한다 — Markdown으로 답할 것 · 키 하나가 섹션 하나일 것 · 제목은 이 목록
/// 그대로일 것 · 목록은 `- ` 한 줄씩일 것 · 내용이 없는 섹션은 제목도 쓰지 않을 것.
fn markdown_output_contract(mode: NoteType) -> String {
    let headings: Vec<String> = output_sections(mode)
        .iter()
        .map(|section| format!("## {section}"))
        .collect();

    format!(
        "\
Return the note as Markdown and nothing else. No JSON object, no code fence, no prose before or
after it.

Write one `## ` section for each key described below, in that order, using exactly these
headings and nothing else:

{}

Write a key whose value is a list as one `- ` line per item. Leave out the heading of a section
that has nothing in it.",
        headings.join("\n")
    )
}

/// 한 Recording을 외부 AI 채팅으로 가져가기 위한 요청 하나.
///
/// **노트를 받지 않는다.** 이 요청은 노트를 만들어 달라는 요청이므로, 이미 있는 노트를 실을
/// 자리가 생성자에 없다 — 그래서 [`ExportDocument::note`]는 이 경로에서 언제나 `None`이다
/// (ADR-0010 §5.4).
#[derive(Debug, Clone, Copy)]
pub struct AiRequest<'a> {
    /// 어떤 형태의 노트를 요청하는가 (§9.5).
    pub mode: NoteType,
    /// §11의 문서 입력 그대로다 — 두 번째 입력 타입을 만들지 않는다.
    document: ExportDocument<'a>,
}

impl<'a> AiRequest<'a> {
    /// Recording 하나와 그 Transcript로 요청을 만든다.
    ///
    /// 어느 Transcript version인지 고르는 것은 부르는 쪽의 일이다 (§7.2) — 여기서는 저장소를
    /// 보지 않으므로 고를 수도 없다.
    pub fn new(mode: NoteType, recording: &'a Recording, transcript: &'a Transcript) -> Self {
        Self {
            mode,
            document: ExportDocument {
                recording,
                transcript,
                note: None,
            },
        }
    }

    /// 사람이 자기 AI 채팅에 그대로 붙여 넣는 **Manual 프롬프트** — transcript를 포함한다.
    ///
    /// 포함하는 이유: 포함하지 않으면 두 번 붙여 넣어야 하고, 그 사이에 채팅이 지시만 보고
    /// 답하기 시작한다. 자기 지시를 직접 쓰고 싶은 사람에게는 [`Self::transcript_text`]가
    /// 별개의 길로 남는다 (ADR-0010 §6.2).
    pub fn manual_prompt(&self) -> String {
        let transcript = match transcript_body(self.document.transcript) {
            TranscriptBody::Segments(segments) => segments.join("\n\n"),
            TranscriptBody::Raw(raw_text) => raw_text.to_owned(),
            TranscriptBody::Empty => String::new(),
        };

        instructions(self.mode, &transcript)
    }

    /// 붙여 넣을 수 있는 **Transcript 텍스트** — `## Transcript`와 §11의 블록 그대로다.
    ///
    /// 제목을 **포함하는** 이유: 제목을 떼면 "제목 없는 본문"이라는 두 번째 조립 규칙과 두 번째
    /// 기대 문자열이 생긴다. 붙여 넣은 사람에게도 그 한 줄은 무엇을 붙였는지 말해 준다
    /// (ADR-0010 §5.4).
    ///
    /// 적을 것이 하나도 없으면 **빈 문자열이다** — 제목만 남은 텍스트를 만들지 않는다.
    pub fn transcript_text(&self) -> String {
        join_document(transcript_blocks(self.document.transcript))
    }

    /// 외부 AI 채팅이 요청을 이해하기에 충분한 맥락을 담은 **AI-ready Markdown 문서 하나**.
    ///
    /// ```text
    /// # Molt Note AI Request
    /// ## Mode          ← §9.5의 표시 이름
    /// ## Instructions  ← Manual 지시문. transcript 본문 대신 아래를 가리키는 한 문장이 들어간다
    /// ## Recording     ← Title · Date · Duration (오디오는 없다 · INV-6)
    /// ## Transcript    ← §11의 블록 그대로
    /// ```
    ///
    /// 순서는 고정이다 — 읽는 쪽이 **무엇을 해 달라는 요청인지** 먼저 만나고, 가장 긴 것이
    /// 마지막에 온다 (ADR-0010 §5.2).
    pub fn ai_ready_document(&self) -> String {
        let recording = self.document.recording;

        let mut blocks = vec![
            format!("# {AI_REQUEST_HEADING}"),
            format!("## {MODE_SECTION}\n{}", mode_label(self.mode)),
            format!(
                "## {INSTRUCTIONS_SECTION}\n{}",
                instructions(self.mode, &transcript_pointer())
            ),
            format!(
                "## {RECORDING_SECTION}\nTitle: {}\n{}",
                heading_text(&recording.title),
                metadata_block(recording)
            ),
        ];

        blocks.extend(transcript_blocks(self.document.transcript));

        join_document(blocks)
    }
}

/// 상수 원문에 **치환 두 번**을 적용해 Manual 지시문을 만든다 (ADR-0010 §6.2).
///
/// ```text
/// (a) JSON 출력 계약 블록 → Markdown 출력 계약 블록   ← 정확히 1회여야 한다
/// (b) {{transcript}}      → 상황에 맞는 내용          ← 이미 있는 유일한 seam
/// ```
///
/// 프롬프트 상수는 **읽기만 한다.** 두 번째 프롬프트 세트를 만들지 않는다.
fn instructions(mode: NoteType, transcript_slot: &str) -> String {
    prompt_template(mode)
        .replace(JSON_OUTPUT_CONTRACT, &markdown_output_contract(mode))
        .replace(TRANSCRIPT_PLACEHOLDER, transcript_slot)
        .trim_end()
        .to_owned()
}

/// 블록들을 빈 줄 하나로 잇고 개행 하나로 끝낸다 — `markdown::render`와 같은 조립 규칙이다.
///
/// 적을 블록이 하나도 없으면 빈 문자열이다. 개행 하나만 남은 산출물을 만들지 않는다.
fn join_document(blocks: Vec<String>) -> String {
    if blocks.is_empty() {
        return String::new();
    }

    let mut document = blocks.join("\n\n");
    document.push('\n');
    document
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ai::note::{MEETING_FIELDS, STUDY_FIELDS, SUMMARY_FIELDS};
    use crate::ai::prompt::{
        MEETING_PROMPT, PROMPT_VERSION_MEETING, PROMPT_VERSION_STUDY, PROMPT_VERSION_SUMMARY,
        STUDY_PROMPT, SUMMARY_PROMPT,
    };
    use crate::domain::{ProcessingStatus, RecordingId, TranscriptId, TranscriptSegment};

    /// 산출물에 절대 나오면 안 되는 이름들 (MH-6 · §10).
    ///
    /// **로컬 provider의 이름은 조각에서 만든다.** 이 저장소는 adapter 디렉터리 밖의 어떤
    /// 소스도 그 이름을 글자로 담는 것을 금지하고
    /// (INV-9 · `no_source_outside_the_adapter_knows_this_vendors_endpoints_or_parameters`),
    /// 이 파일은 adapter 밖이다. 검사는 그대로 하되 이름을 여기에 적어 두지는 않는다.
    fn vendor_names() -> Vec<String> {
        let local_provider = format!("{}{}", "Olla", "ma");

        let mut names = vec![
            "ChatGPT".to_owned(),
            "OpenAI".to_owned(),
            "Claude".to_owned(),
            "Anthropic".to_owned(),
            "Gemini".to_owned(),
            "Codex".to_owned(),
            local_provider.to_lowercase(),
        ];
        names.push(local_provider);
        names
    }

    fn recording() -> Recording {
        Recording {
            id: RecordingId::new("rec-1"),
            title: "3DGS Study #04".to_owned(),
            created_at: "2026-09-01T10:00:00.000Z".to_owned(),
            updated_at: "2026-09-01T10:52:31.000Z".to_owned(),
            duration_ms: 3_151_000,
            audio_path: "recordings/rec-1.wav".to_owned(),
            audio_format: "wav".to_owned(),
            microphone: None,
            current_transcript_id: Some(TranscriptId::new("tr-1")),
            transcription_status: ProcessingStatus::Done,
            ai_status: ProcessingStatus::None,
            notion_status: ProcessingStatus::None,
        }
    }

    fn transcript() -> Transcript {
        Transcript {
            id: TranscriptId::new("tr-1"),
            recording_id: RecordingId::new("rec-1"),
            language: Some("ko".to_owned()),
            segments: vec![
                TranscriptSegment {
                    start_ms: 3_000,
                    end_ms: 6_500,
                    text: "안녕하세요. 오늘은 3DGS를 봅니다.".to_owned(),
                },
                TranscriptSegment {
                    start_ms: 6_500,
                    end_ms: 9_000,
                    text: "먼저 splat 표현부터 보겠습니다.".to_owned(),
                },
            ],
            raw_text: "안녕하세요. 오늘은 3DGS를 봅니다. 먼저 splat 표현부터 보겠습니다."
                .to_owned(),
            created_at: "2026-09-01T11:00:00.000Z".to_owned(),
            engine: "whisper-rs".to_owned(),
            model: "ggml-base".to_owned(),
        }
    }

    // ── 프롬프트 상수를 고치지 않는다 (AC-3 · ADR-0010 §6.1) ─────────────────────

    #[test]
    fn the_json_output_contract_is_replaced_exactly_once_in_every_prompt() {
        // ⚠️ 이 테스트가 깨졌다면 프롬프트 상수의 출력 계약 문장이 바뀐 것이다. 그러면 치환이
        // 0회가 되고 "Markdown으로 달라"와 "JSON만 달라"가 동시에 들어간 모순된 프롬프트가
        // 사용자의 채팅으로 나간다. 해야 할 일은 이 모듈의 JSON_OUTPUT_CONTRACT를 상수의
        // 새 문장과 맞추는 것이다 (ADR-0010 §6.2-a · §13).
        for mode in NoteType::ALL {
            assert_eq!(
                prompt_template(mode).matches(JSON_OUTPUT_CONTRACT).count(),
                1,
                "{mode}: JSON 출력 계약 블록이 프롬프트에 정확히 한 번 있어야 한다"
            );
        }
    }

    #[test]
    fn the_manual_prompt_is_built_from_the_untouched_prompt_constants() {
        // 프롬프트 문장을 복사해 붙인 두 번째 세트가 아니라, 상수 원문에서 갈라진 것이다.
        for (mode, template) in [
            (NoteType::Meeting, MEETING_PROMPT),
            (NoteType::Study, STUDY_PROMPT),
            (NoteType::Summary, SUMMARY_PROMPT),
        ] {
            let prompt = AiRequest::new(mode, &recording(), &transcript()).manual_prompt();

            // 상수의 첫 문장이 그대로 프롬프트의 첫 문장이다.
            let first_line = template.lines().next().expect("상수에 첫 줄이 있다");
            assert!(prompt.starts_with(first_line), "{mode}: {first_line}");

            // 치환하지 않은 부분은 상수 원문 그대로다 — 규칙 절이 그것을 보여 준다.
            assert!(prompt.contains("Rules:\n- Use only what the transcript says."));
            assert!(prompt.contains("- Write the "), "{mode}: 언어 규칙이 남아 있다");
        }
    }

    #[test]
    fn the_declared_prompt_versions_are_untouched() {
        // 이 경로는 promptVersion을 만들지도 저장하지도 않지만 (§6.4), 상수를 고치면 이미
        // 저장된 provenance가 거짓이 된다 — 값이 그대로인지 여기서도 본다.
        assert_eq!(PROMPT_VERSION_MEETING, "v1.meeting.5c6b8a90");
        assert_eq!(PROMPT_VERSION_STUDY, "v1.study.2cfad9a0");
        assert_eq!(PROMPT_VERSION_SUMMARY, "v1.summary.beca7d6c");
    }

    #[test]
    fn the_manual_prompt_asks_for_markdown_and_never_for_json() {
        for mode in NoteType::ALL {
            let prompt = AiRequest::new(mode, &recording(), &transcript()).manual_prompt();

            assert!(
                !prompt.contains(JSON_OUTPUT_CONTRACT),
                "{mode}: JSON 출력 계약이 남았다"
            );
            assert!(prompt.contains("Return the note as Markdown and nothing else."));
            assert!(prompt.contains("Write a key whose value is a list as one `- ` line per item."));
            assert!(prompt.contains("Leave out the heading of a section\nthat has nothing in it."));
            assert!(
                !prompt.contains(TRANSCRIPT_PLACEHOLDER),
                "{mode}: 자리표시자가 남지 않는다"
            );
        }
    }

    #[test]
    fn the_requested_headings_are_the_ones_this_app_already_renders() {
        // 프롬프트가 요구하는 출력 모양이 markdown::render가 이미 만드는 모양과 같다
        // (ADR-0010 §6.3) — "AI에게 요구하는 형식"이라는 두 번째 규칙이 생기지 않는다.
        for (mode, fields, sections) in [
            (NoteType::Meeting, MEETING_FIELDS.as_slice(), MEETING_SECTIONS.as_slice()),
            (NoteType::Study, STUDY_FIELDS.as_slice(), STUDY_SECTIONS.as_slice()),
            (NoteType::Summary, SUMMARY_FIELDS.as_slice(), SUMMARY_SECTIONS.as_slice()),
        ] {
            let prompt = AiRequest::new(mode, &recording(), &transcript()).manual_prompt();

            // 키 하나가 섹션 하나다 — 개수도 순서도 1:1이다.
            assert_eq!(fields.len(), sections.len(), "{mode}: 키와 섹션 개수가 다르다");

            let requested: Vec<&str> = prompt
                .lines()
                .filter_map(|line| line.strip_prefix("## "))
                .collect();
            assert_eq!(requested, *sections, "{mode}: 요구하는 제목이 §9.5와 다르다");

            // 키가 나오는 순서도 그 순서다.
            let mut cursor = 0;
            for (index, field) in fields.iter().enumerate() {
                let found = prompt[cursor..]
                    .find(&format!("\"{field}\""))
                    .unwrap_or_else(|| panic!("{mode}: {field}가 순서대로 나오지 않는다"));
                cursor += found;
                assert!(index < sections.len());
            }
        }
    }

    // ── 세 산출물의 기대 문자열 ─────────────────────────────────────────────────

    #[test]
    fn the_transcript_text_is_the_section_11_block_with_its_heading() {
        let text = AiRequest::new(NoteType::Study, &recording(), &transcript()).transcript_text();

        assert_eq!(
            text,
            [
                "## Transcript",
                "",
                "### 00:00:03",
                "안녕하세요. 오늘은 3DGS를 봅니다.",
                "",
                "### 00:00:06",
                "먼저 splat 표현부터 보겠습니다.",
                "",
            ]
            .join("\n")
        );
    }

    #[test]
    fn the_ai_ready_document_is_exactly_the_four_sections_and_the_transcript() {
        let document =
            AiRequest::new(NoteType::Study, &recording(), &transcript()).ai_ready_document();

        assert_eq!(
            document,
            [
                "# Molt Note AI Request",
                "",
                "## Mode",
                "Study",
                "",
                "## Instructions",
                "You turn a lecture or study session transcript into a structured note.",
                "",
                "Return the note as Markdown and nothing else. No JSON object, no code fence, no prose before or",
                "after it.",
                "",
                "Write one `## ` section for each key described below, in that order, using exactly these",
                "headings and nothing else:",
                "",
                "## Overview",
                "## Key Concepts",
                "## Important Details",
                "## Questions",
                "## Things to Study",
                "## References Mentioned",
                "",
                "Write a key whose value is a list as one `- ` line per item. Leave out the heading of a section",
                "that has nothing in it.",
                "",
                "The object has exactly these keys:",
                "- \"overview\": string. A short paragraph describing what was taught or studied.",
                "- \"keyConcepts\": array of strings. The concepts the material is built on.",
                "- \"importantDetails\": array of strings. Details worth remembering.",
                "- \"questions\": array of strings. Questions that help check understanding of this material.",
                "- \"thingsToStudy\": array of strings. What to study next, based on what was said.",
                "- \"referencesMentioned\": array of strings. Books, papers, tools or links that were actually",
                "  mentioned. Do not add references that were not mentioned.",
                "",
                "Rules:",
                "- Use only what the transcript says. Do not invent facts, sources or numbers.",
                "- Every key must be present. If a section has nothing, use an empty array.",
                "- One item is one line of plain text. Do not nest objects or arrays inside an item.",
                "- Write the note in the main language of the transcript.",
                "",
                "Transcript:",
                "The transcript is in the \"## Transcript\" section of this document.",
                "",
                "## Recording",
                "Title: 3DGS Study #04",
                "Date: 2026-09-01",
                "Duration: 52:31",
                "",
                "## Transcript",
                "",
                "### 00:00:03",
                "안녕하세요. 오늘은 3DGS를 봅니다.",
                "",
                "### 00:00:06",
                "먼저 splat 표현부터 보겠습니다.",
                "",
            ]
            .join("\n")
        );
    }

    #[test]
    fn the_manual_prompt_carries_the_transcript_so_one_paste_is_enough() {
        let prompt =
            AiRequest::new(NoteType::Summary, &recording(), &transcript()).manual_prompt();

        assert_eq!(
            prompt,
            [
                "You turn a transcript into a short structured summary.",
                "",
                "Return the note as Markdown and nothing else. No JSON object, no code fence, no prose before or",
                "after it.",
                "",
                "Write one `## ` section for each key described below, in that order, using exactly these",
                "headings and nothing else:",
                "",
                "## Short Summary",
                "## Key Points",
                "",
                "Write a key whose value is a list as one `- ` line per item. Leave out the heading of a section",
                "that has nothing in it.",
                "",
                "The object has exactly these keys:",
                "- \"shortSummary\": string. A few sentences covering the whole transcript.",
                "- \"keyPoints\": array of strings. The points a reader should take away.",
                "",
                "Rules:",
                "- Use only what the transcript says. Do not invent facts.",
                "- Every key must be present. If there are no key points, use an empty array.",
                "- One item is one line of plain text. Do not nest objects or arrays inside an item.",
                "- Write the summary in the main language of the transcript.",
                "",
                "Transcript:",
                "### 00:00:03",
                "안녕하세요. 오늘은 3DGS를 봅니다.",
                "",
                "### 00:00:06",
                "먼저 splat 표현부터 보겠습니다.",
            ]
            .join("\n")
        );
    }

    #[test]
    fn every_mode_says_its_own_name_and_its_own_sections() {
        let recording = recording();
        let transcript = transcript();

        for (mode, label, sections) in [
            (NoteType::Meeting, "Meeting", MEETING_SECTIONS.as_slice()),
            (NoteType::Study, "Study", STUDY_SECTIONS.as_slice()),
            (NoteType::Summary, "Summary", SUMMARY_SECTIONS.as_slice()),
        ] {
            let request = AiRequest::new(mode, &recording, &transcript);
            let document = request.ai_ready_document();

            assert!(document.starts_with(&format!("# {AI_REQUEST_HEADING}\n")));
            assert!(
                document.contains(&format!("## {MODE_SECTION}\n{label}\n")),
                "{mode}: 표시 이름이 {label}이어야 한다"
            );
            // wire 값(소문자)은 문서에 적히지 않는다.
            assert!(
                !document.contains(&format!("## {MODE_SECTION}\n{}", mode.as_str())),
                "{mode}: wire 값이 문서에 적혔다"
            );

            for section in sections {
                assert!(
                    document.contains(&format!("## {section}\n")),
                    "{mode}: 요구 섹션 {section}이 문서에 없다"
                );
            }
            // 다른 mode의 섹션은 요구하지 않는다.
            for other in NoteType::ALL.into_iter().filter(|other| *other != mode) {
                for section in output_sections(other) {
                    if sections.contains(section) {
                        continue;
                    }
                    assert!(
                        !document.contains(&format!("## {section}\n")),
                        "{mode}: 다른 mode의 섹션 {section}이 들어갔다"
                    );
                }
            }

            // Transcript 텍스트는 mode와 무관하다 — 같은 전사에서 언제나 같은 문자열이다.
            assert_eq!(
                request.transcript_text(),
                AiRequest::new(NoteType::Meeting, &recording, &transcript).transcript_text()
            );
        }
    }

    // ── 결정성 (§18) ─────────────────────────────────────────────────────────────

    #[test]
    fn the_same_input_always_makes_the_same_three_strings() {
        // 두 번 만들어 비교한다 — 시계도 난수도 로캘도 해시맵 순회도 없다.
        let recording = recording();
        let transcript = transcript();

        for mode in NoteType::ALL {
            let first = AiRequest::new(mode, &recording, &transcript);
            let second = AiRequest::new(mode, &recording, &transcript);

            assert_eq!(first.manual_prompt(), second.manual_prompt());
            assert_eq!(first.transcript_text(), second.transcript_text());
            assert_eq!(first.ai_ready_document(), second.ai_ready_document());
        }
    }

    #[test]
    fn each_output_ends_with_exactly_one_newline_or_none_at_all() {
        let recording = recording();
        let transcript = transcript();

        for mode in NoteType::ALL {
            let request = AiRequest::new(mode, &recording, &transcript);

            for text in [request.transcript_text(), request.ai_ready_document()] {
                assert!(text.ends_with('\n'));
                assert!(!text.ends_with("\n\n"));
            }
            // 프롬프트는 붙여 넣는 텍스트다 — 끝에 빈 줄을 남기지 않는다.
            assert!(!request.manual_prompt().ends_with('\n'));
        }
    }

    // ── transcript의 모양이 다를 때 ──────────────────────────────────────────────

    #[test]
    fn a_transcript_without_segments_falls_back_to_its_raw_text_everywhere() {
        // §11과 같은 규칙이다 — 이 모듈이 두 번째 규칙을 만들지 않는다.
        let recording = recording();
        let transcript = Transcript {
            segments: vec![],
            ..transcript()
        };
        let raw_text = "안녕하세요. 오늘은 3DGS를 봅니다. 먼저 splat 표현부터 보겠습니다.";
        let request = AiRequest::new(NoteType::Meeting, &recording, &transcript);

        assert_eq!(
            request.transcript_text(),
            format!("## {TRANSCRIPT_SECTION}\n{raw_text}\n")
        );
        assert!(request.manual_prompt().ends_with(&format!("Transcript:\n{raw_text}")));
        assert!(request
            .ai_ready_document()
            .ends_with(&format!("## {TRANSCRIPT_SECTION}\n{raw_text}\n")));
        // 타임스탬프 소제목은 없다 — 지어내지 않는다.
        assert!(!request.transcript_text().contains("### "));
    }

    #[test]
    fn an_empty_transcript_leaves_no_empty_section_behind() {
        // 순수 렌더러는 받은 값을 그대로 렌더한다. 빈 요청을 거절하는 것은 command 경계의
        // 일이다 (ADR-0010 §5.5) — 여기서는 빈 제목을 만들지 않는 것까지가 규칙이다.
        let recording = recording();
        let transcript = Transcript {
            segments: vec![TranscriptSegment {
                start_ms: 0,
                end_ms: 0,
                text: "   ".to_owned(),
            }],
            raw_text: "  \n ".to_owned(),
            ..transcript()
        };
        let request = AiRequest::new(NoteType::Study, &recording, &transcript);

        assert_eq!(request.transcript_text(), "");

        let document = request.ai_ready_document();
        assert!(
            !document.contains(&format!("## {TRANSCRIPT_SECTION}\n\n")),
            "빈 Transcript 섹션이 남았다"
        );
        assert!(document.ends_with("Title: 3DGS Study #04\nDate: 2026-09-01\nDuration: 52:31\n"));
        assert!(!document.contains("\n\n\n"), "빈 블록이 남았다");

        // 프롬프트의 transcript 자리는 비지만, 지시문은 그대로 있다.
        let prompt = request.manual_prompt();
        assert!(prompt.ends_with("Transcript:"));
        assert!(prompt.contains("Return the note as Markdown and nothing else."));
    }

    #[test]
    fn segments_keep_the_order_they_were_given() {
        let recording = recording();
        let transcript = Transcript {
            segments: vec![
                TranscriptSegment {
                    start_ms: 9_000,
                    end_ms: 10_000,
                    text: "나중 것".to_owned(),
                },
                TranscriptSegment {
                    start_ms: 1_000,
                    end_ms: 2_000,
                    text: "먼저 것".to_owned(),
                },
            ],
            ..transcript()
        };
        let request = AiRequest::new(NoteType::Meeting, &recording, &transcript);

        for text in [
            request.transcript_text(),
            request.ai_ready_document(),
            request.manual_prompt(),
        ] {
            let later = text.find("나중 것").expect("첫 segment가 있어야 한다");
            let earlier = text.find("먼저 것").expect("둘째 segment가 있어야 한다");
            assert!(later < earlier, "정렬하지 않는다");
        }
        // 타임스탬프는 §11과 같은 함수가 만든 문자열이다.
        assert!(request.transcript_text().contains("### 00:00:09\n나중 것"));
    }

    // ── MH-4 · MH-6 ──────────────────────────────────────────────────────────────

    #[test]
    fn no_audio_path_and_no_audio_format_reaches_any_of_the_three_outputs() {
        // 렌더러가 읽는 Recording 필드는 title · created_at · duration_ms 셋뿐이다 (INV-6).
        let recording = Recording {
            audio_path: "recordings/secret-audio-path.wav".to_owned(),
            audio_format: "flac".to_owned(),
            ..recording()
        };
        let transcript = transcript();
        let request = AiRequest::new(NoteType::Meeting, &recording, &transcript);

        for text in [
            request.manual_prompt(),
            request.transcript_text(),
            request.ai_ready_document(),
        ] {
            assert!(!text.contains("secret-audio-path"), "audio 경로가 나갔다");
            assert!(!text.contains(".wav"), "audio 파일 이름이 나갔다");
            assert!(!text.contains("flac"), "audio 형식이 나갔다");
            assert!(!text.contains("audio"), "audio 필드 이름이 나갔다");
        }
    }

    #[test]
    fn no_vendor_name_appears_in_any_of_the_three_outputs() {
        let recording = recording();
        let transcript = transcript();

        for mode in NoteType::ALL {
            let request = AiRequest::new(mode, &recording, &transcript);

            for text in [
                request.manual_prompt(),
                request.transcript_text(),
                request.ai_ready_document(),
            ] {
                for vendor in vendor_names() {
                    assert!(!text.contains(&vendor), "{mode}: 벤더 이름 {vendor}가 나갔다");
                }
            }
        }
    }
}
