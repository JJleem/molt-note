//! §11의 Markdown 산출물을 만드는 **순수 렌더러** (PRODUCT-SPEC §11 · §9.5 · ADR-0009 §5.4).
//!
//! ```text
//! Recording  (제목 · created_at · duration_ms)  ─┐
//! Transcript (segments · raw_text) ─────────────┼─→ render ─→ String
//! StructuredNote (선택) ────────────────────────┘
//! ```
//!
//! **파일도 저장소도 시계도 네트워크도 없다.** 여기 있는 것은 값에서 문자열을 만드는 함수뿐이며,
//! 같은 입력은 언제나 같은 문자열을 낸다 (§18 · `phase-prompt/05` 요구 A-4). 만들어진 문자열이
//! 파일이 되는 자리도, Notion 본문이 되는 자리도 이 모듈 밖이다 — 그래서 **산출물이 하나다**
//! (ADR-0009 §14).
//!
//! ## AI Note는 선택이다 (INV-8 · §17.1)
//!
//! [`ExportDocument::note`]가 `None`이어도 유효한 문서가 나온다 — 제목 · `Date:` · `Duration:` ·
//! `## Transcript`만으로 읽을 수 있고, **없는 AI 섹션의 빈 껍데기를 남기지 않는다.** 비어 있는
//! 제목만 있는 섹션은 "AI가 실패했다"처럼 보이지만 실제로는 AI를 쓰지 않은 정상 상태다.
//! 같은 이유로 **AI Note가 있어도 내용이 없는 섹션은 적지 않는다** — 배열이 비는 것은 정상이고
//! (ADR-0008 §7.3), 빈 제목은 그 사실을 말해 주지 않는다.
//!
//! ## 벤더를 알지 않는다 (INV-9)
//!
//! 이 렌더러가 소비하는 것은 §9.3의 provider 중립 [`StructuredNote`] 하나다. 제공자의 원시
//! 응답(JSON 텍스트 · 고유 필드 · 상태 코드)은 여기에 닿지 않는다 — 그것을 값으로 바꾸는 일은
//! 이미 [`crate::ai::note::parse_note`]에서 끝났고, 그래서 provider가 바뀌어도 이 파일은
//! 바뀌지 않는다.
//!
//! ## 섹션 제목의 출처는 §9.5 하나다
//!
//! `## Overview` · `## Key Discussions` … 는 §9.5 표의 **출력 섹션 이름 그대로**이며,
//! [`MEETING_SECTIONS`] · [`STUDY_SECTIONS`] · [`SUMMARY_SECTIONS`]가 그 이름을 값으로 들고
//! 있다. 노트 타입의 필드 이름도 같은 표에서 왔으므로 (ADR-0008 §7.1) 제목과 내용이 어긋날
//! 자리가 없다.

use crate::ai::note::StructuredNote;
use crate::domain::{format_duration_ms, Recording, Transcript};

use super::iso_date;

/// 제목이 비어 있을 때 `# ` 자리에 쓰는 값.
///
/// 저장된 Recording의 제목은 비어 있을 수 없지만(`commands::validated`), 문서의 첫 `# `는
/// **비어 있으면 안 된다** — Notion은 첫 h1을 페이지 제목으로 삼고(ADR-0009 §5.3), 빈 제목의
/// 문서는 목록에서 서로 구분되지 않는다.
pub const UNTITLED_TITLE: &str = "제목 없음";

/// `created_at`이 예상한 모양이 아니고 그마저 비어 있을 때 `Date:` 줄에 쓰는 값.
pub const UNKNOWN_DATE: &str = super::filename::UNKNOWN_DATE;

/// Meeting 노트의 섹션 제목 — §9.5의 출력 섹션 순서 그대로다.
pub const MEETING_SECTIONS: [&str; 5] = [
    "Overview",
    "Key Discussions",
    "Decisions",
    "Action Items",
    "Open Questions",
];

/// Study 노트의 섹션 제목 — §9.5의 출력 섹션 순서 그대로다.
pub const STUDY_SECTIONS: [&str; 6] = [
    "Overview",
    "Key Concepts",
    "Important Details",
    "Questions",
    "Things to Study",
    "References Mentioned",
];

/// Summary 노트의 섹션 제목 — §9.5의 출력 섹션 순서 그대로다.
pub const SUMMARY_SECTIONS: [&str; 2] = ["Short Summary", "Key Points"];

/// Transcript 섹션의 제목 (§11).
pub const TRANSCRIPT_SECTION: &str = "Transcript";

/// 한 문서로 만들 입력 — Recording 하나와 그 Transcript, 그리고 **있으면** AI Note.
///
/// 어느 Transcript version인지, 어느 AI Note인지 고르는 것은 부르는 쪽의 일이다 (§7.2 · §7.3).
/// 여기서는 저장소를 보지 않으므로 고를 수도 없다.
#[derive(Debug, Clone, Copy)]
pub struct ExportDocument<'a> {
    pub recording: &'a Recording,
    pub transcript: &'a Transcript,
    /// **`None`은 정상 상태다** (INV-8). AI를 쓰지 않은 Recording도 유효한 문서가 된다.
    pub note: Option<&'a StructuredNote>,
}

/// §11의 구조를 가진 Markdown 문서 하나를 만든다.
///
/// ```text
/// # <제목>
///
/// Date: 2026-09-01
/// Duration: 52:31
///
/// ## Overview                 ← AI Note가 있을 때만. 내용이 없는 섹션은 적지 않는다
/// …
///
/// ## Transcript
///
/// ### 00:00:03
/// …
/// ```
///
/// 문서는 언제나 개행 하나로 끝나고, 블록 사이에는 빈 줄이 정확히 하나 있다. 같은 입력은
/// 언제나 같은 문자열을 낸다 — 시계도 로캘도 해시맵 순회도 없고, segment는 **받은 순서
/// 그대로** 적는다.
pub fn render(document: &ExportDocument<'_>) -> String {
    let recording = document.recording;

    // 블록 하나가 문단 하나다. 마지막에 빈 줄 하나로 잇는다 — 이렇게 하면 "언제 빈 줄을
    // 넣는가"를 각 자리가 따로 판단하지 않아도 되고, 빈 섹션이 생길 여지도 없다.
    let mut blocks = vec![
        format!("# {}", heading_text(&recording.title)),
        format!(
            "Date: {}\nDuration: {}",
            date_text(&recording.created_at),
            format_duration_ms(recording.duration_ms)
        ),
    ];

    if let Some(note) = document.note {
        for (title, body) in note_sections(note) {
            // 내용이 없는 섹션은 제목도 남기지 않는다 (INV-8 · ADR-0008 §7.3).
            if let Some(rendered) = render_body(body) {
                blocks.push(format!("## {title}\n{rendered}"));
            }
        }
    }

    blocks.extend(transcript_blocks(document.transcript));

    let mut markdown = blocks.join("\n\n");
    markdown.push('\n');
    markdown
}

/// 녹음 시작 기준 오프셋을 `### ` 다음에 오는 타임스탬프로 적는다 — `00:00:03` (§11).
///
/// [`format_duration_ms`]와 다른 형식이다. 그쪽은 화면에 길이를 보여 주는 값(`52:31`)이고,
/// 이쪽은 **문서 안에서 정렬되어 읽히는 자리 표시**라 시·분·초가 언제나 두 자리다.
///
/// 존재할 수 없는 음수 오프셋은 `0`으로 본다. 100시간이 넘으면 시간 자리가 세 자리가 된다 —
/// 자르지 않는다.
pub fn format_timestamp_ms(offset_ms: i64) -> String {
    let total_seconds = offset_ms.max(0) / 1_000;

    format!(
        "{:02}:{:02}:{:02}",
        total_seconds / 3_600,
        (total_seconds / 60) % 60,
        total_seconds % 60
    )
}

/// 섹션 하나의 내용 — §9.3이 정한 두 가지 타입뿐이다 (`string`과 `string[]`).
#[derive(Debug, Clone, Copy)]
enum SectionBody<'a> {
    Text(&'a str),
    List(&'a [String]),
}

/// 노트 하나가 만드는 섹션들 — 제목은 §9.5의 출력 섹션 이름이고, 순서도 그 표의 순서다.
fn note_sections(note: &StructuredNote) -> Vec<(&'static str, SectionBody<'_>)> {
    match note {
        StructuredNote::Meeting(note) => vec![
            ("Overview", SectionBody::Text(&note.overview)),
            ("Key Discussions", SectionBody::List(&note.key_discussions)),
            ("Decisions", SectionBody::List(&note.decisions)),
            ("Action Items", SectionBody::List(&note.action_items)),
            ("Open Questions", SectionBody::List(&note.open_questions)),
        ],
        StructuredNote::Study(note) => vec![
            ("Overview", SectionBody::Text(&note.overview)),
            ("Key Concepts", SectionBody::List(&note.key_concepts)),
            ("Important Details", SectionBody::List(&note.important_details)),
            ("Questions", SectionBody::List(&note.questions)),
            ("Things to Study", SectionBody::List(&note.things_to_study)),
            (
                "References Mentioned",
                SectionBody::List(&note.references_mentioned),
            ),
        ],
        StructuredNote::Summary(note) => vec![
            ("Short Summary", SectionBody::Text(&note.short_summary)),
            ("Key Points", SectionBody::List(&note.key_points)),
        ],
    }
}

/// 섹션의 내용을 적는다. 적을 것이 없으면 `None`이고, 그러면 제목도 나가지 않는다.
fn render_body(body: SectionBody<'_>) -> Option<String> {
    match body {
        SectionBody::Text(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        }
        SectionBody::List(items) => {
            // 목록 항목은 한 줄이어야 `- ` 하나가 항목 하나로 남는다. 항목 안의 개행은
            // 공백으로 접는다 — 내용을 버리지 않으면서 목록 구조를 지키는 방법이다.
            let lines: Vec<String> = items
                .iter()
                .map(|item| single_line(item))
                .filter(|item| !item.is_empty())
                .map(|item| format!("- {item}"))
                .collect();

            (!lines.is_empty()).then(|| lines.join("\n"))
        }
    }
}

/// Transcript가 만드는 블록들 — `## Transcript`와 타임스탬프 소제목들 (§11).
///
/// segment가 있으면 그것을 순서대로 적고, 없으면 `raw_text`를 한 문단으로 적는다.
/// **둘 다 없으면 섹션 자체를 만들지 않는다** — 빈 제목은 "여기 무언가 실패했다"처럼
/// 보이지만 실제로는 적을 것이 없을 뿐이다 (INV-8과 같은 이유).
fn transcript_blocks(transcript: &Transcript) -> Vec<String> {
    let segments: Vec<String> = transcript
        .segments
        .iter()
        .filter(|segment| !segment.text.trim().is_empty())
        .map(|segment| {
            format!(
                "### {}\n{}",
                format_timestamp_ms(segment.start_ms),
                segment.text.trim()
            )
        })
        .collect();

    if !segments.is_empty() {
        let mut blocks = vec![format!("## {TRANSCRIPT_SECTION}")];
        blocks.extend(segments);
        return blocks;
    }

    let raw_text = transcript.raw_text.trim();
    if raw_text.is_empty() {
        return Vec::new();
    }

    vec![format!("## {TRANSCRIPT_SECTION}\n{raw_text}")]
}

/// 문서의 첫 `# ` 줄에 쓸 제목.
///
/// **한 줄로 만든다.** 제목 안의 개행이 그대로 나가면 `# ` 다음 줄부터는 제목이 아니게 되고,
/// Notion은 첫 h1을 페이지 제목으로 삼으므로(ADR-0009 §5.3) 그 경계가 문서마다 달라진다.
fn heading_text(title: &str) -> String {
    let collapsed = single_line(title);

    if collapsed.is_empty() {
        return UNTITLED_TITLE.to_owned();
    }
    collapsed
}

/// `Date:` 줄에 쓸 값.
///
/// 저장된 UTC 텍스트의 앞 10글자를 그대로 쓴다 ([`super::iso_date`]). 모양이 다르면
/// **지어내지 않고 받은 값을 그대로** 적고(`commands::title_for`와 같은 태도), 그마저 비어
/// 있으면 [`UNKNOWN_DATE`]다.
fn date_text(created_at: &str) -> String {
    if let Some(date) = iso_date(created_at) {
        return date.to_owned();
    }

    let collapsed = single_line(created_at);
    if collapsed.is_empty() {
        return UNKNOWN_DATE.to_owned();
    }
    collapsed
}

/// 공백 덩어리(개행 포함)를 공백 하나로 접고 앞뒤를 잘라 **한 줄**로 만든다.
fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ai::note::{MeetingNote, StudyNote, SummaryNote};
    use crate::domain::{ProcessingStatus, RecordingId, TranscriptId, TranscriptSegment};

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

    fn study_note() -> StructuredNote {
        StructuredNote::Study(StudyNote {
            overview: "3DGS 스터디 4회차.".to_owned(),
            key_concepts: vec!["Gaussian splatting".to_owned(), "래스터화".to_owned()],
            important_details: vec![],
            questions: vec!["왜 point cloud로 시작하나?".to_owned()],
            things_to_study: vec![],
            references_mentioned: vec![],
        })
    }

    fn meeting_note() -> StructuredNote {
        StructuredNote::Meeting(MeetingNote {
            overview: "분기 계획 회의.".to_owned(),
            key_discussions: vec!["일정".to_owned()],
            decisions: vec!["9월 출시".to_owned()],
            action_items: vec!["초안 작성".to_owned()],
            open_questions: vec![],
        })
    }

    fn summary_note() -> StructuredNote {
        StructuredNote::Summary(SummaryNote {
            short_summary: "짧은 요약.".to_owned(),
            key_points: vec!["요점 하나".to_owned()],
        })
    }

    fn document<'a>(
        recording: &'a Recording,
        transcript: &'a Transcript,
        note: Option<&'a StructuredNote>,
    ) -> ExportDocument<'a> {
        ExportDocument {
            recording,
            transcript,
            note,
        }
    }

    // ── §11의 구조 ───────────────────────────────────────────────────────────────

    #[test]
    fn a_recording_with_an_ai_note_renders_exactly_the_document_of_section_11() {
        // 기대 문자열 전체를 고정한다 — 구조가 조금이라도 움직이면 여기서 걸린다.
        let recording = recording();
        let transcript = transcript();
        let note = study_note();

        let markdown = render(&document(&recording, &transcript, Some(&note)));

        assert_eq!(
            markdown,
            [
                "# 3DGS Study #04",
                "",
                "Date: 2026-09-01",
                "Duration: 52:31",
                "",
                "## Overview",
                "3DGS 스터디 4회차.",
                "",
                "## Key Concepts",
                "- Gaussian splatting",
                "- 래스터화",
                "",
                "## Questions",
                "- 왜 point cloud로 시작하나?",
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

    // ── INV-8: AI Note가 없는 입력 ───────────────────────────────────────────────

    #[test]
    fn a_recording_without_an_ai_note_is_still_a_valid_document() {
        // §17.1의 core 성공 기준이다 — AI를 설정하지 않은 사용자도 기록을 꺼낼 수 있다.
        let recording = recording();
        let transcript = transcript();

        let markdown = render(&document(&recording, &transcript, None));

        assert_eq!(
            markdown,
            [
                "# 3DGS Study #04",
                "",
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
    fn a_document_without_an_ai_note_leaves_no_empty_ai_section_behind() {
        let recording = recording();
        let transcript = transcript();

        let markdown = render(&document(&recording, &transcript, None));

        // 메타데이터와 Transcript는 그대로 있다.
        assert!(markdown.starts_with("# 3DGS Study #04\n"));
        assert!(markdown.contains("\nDate: 2026-09-01\n"));
        assert!(markdown.contains("\nDuration: 52:31\n"));
        assert!(markdown.contains("\n## Transcript\n"));
        assert!(markdown.contains("\n### 00:00:03\n"));

        // AI 섹션의 껍데기는 하나도 없다 — 열세 개 이름 전부.
        for section in MEETING_SECTIONS
            .iter()
            .chain(STUDY_SECTIONS.iter())
            .chain(SUMMARY_SECTIONS.iter())
        {
            assert!(
                !markdown.contains(&format!("## {section}")),
                "쓰지 않은 AI 섹션이 남았다: {section}"
            );
        }
        // 제목만 있고 내용이 없는 줄도 없다.
        assert!(!markdown.contains("##\n"), "빈 제목이 남았다");
        assert!(!markdown.contains("\n\n\n"), "빈 블록이 남았다");
    }

    // ── §9.5의 섹션 이름 ─────────────────────────────────────────────────────────

    #[test]
    fn the_section_titles_are_the_output_section_names_of_section_9_5() {
        for (note, expected) in [
            (meeting_note(), MEETING_SECTIONS.as_slice()),
            (study_note(), STUDY_SECTIONS.as_slice()),
            (summary_note(), SUMMARY_SECTIONS.as_slice()),
        ] {
            let titles: Vec<&str> = note_sections(&note)
                .into_iter()
                .map(|(title, _)| title)
                .collect();

            assert_eq!(titles, expected, "{}의 섹션이 §9.5와 다르다", note.mode());
        }

        // §9.5의 13개 출력 섹션이 13개 제목에 1:1로 대응한다.
        assert_eq!(
            MEETING_SECTIONS.len() + STUDY_SECTIONS.len() + SUMMARY_SECTIONS.len(),
            13
        );
    }

    #[test]
    fn every_mode_renders_its_own_sections_and_no_others() {
        let recording = recording();
        let transcript = transcript();

        for (note, expected) in [
            (meeting_note(), MEETING_SECTIONS.as_slice()),
            (study_note(), STUDY_SECTIONS.as_slice()),
            (summary_note(), SUMMARY_SECTIONS.as_slice()),
        ] {
            let markdown = render(&document(&recording, &transcript, Some(&note)));
            let rendered: Vec<&str> = markdown
                .lines()
                .filter_map(|line| line.strip_prefix("## "))
                .filter(|title| *title != TRANSCRIPT_SECTION)
                .collect();

            // 내용이 있는 섹션만 나온다 — 그리고 그 전부가 이 mode의 이름이다.
            assert!(!rendered.is_empty(), "{}에 섹션이 하나도 없다", note.mode());
            for title in &rendered {
                assert!(
                    expected.contains(title),
                    "{}에 없는 섹션이 나왔다: {title}",
                    note.mode()
                );
            }
            // 순서는 §9.5의 순서를 따른다.
            let order: Vec<&str> = expected
                .iter()
                .copied()
                .filter(|title| rendered.contains(title))
                .collect();
            assert_eq!(rendered, order, "{}의 섹션 순서가 §9.5와 다르다", note.mode());
        }
    }

    #[test]
    fn the_renderer_consumes_the_provider_neutral_note_of_section_9_3() {
        // 입력은 §9.3의 값 타입 하나다 — 제공자의 원시 응답도 고유 필드도 필요하지 않다
        // (INV-9). 어느 mode든 같은 함수 하나가 문서를 만든다.
        let recording = recording();
        let transcript = transcript();

        for note in [meeting_note(), study_note(), summary_note()] {
            let markdown = render(&document(&recording, &transcript, Some(&note)));

            assert!(markdown.starts_with("# 3DGS Study #04\n"));
            assert!(markdown.contains("\n## Transcript\n"));
            // 어느 mode인지는 값이 스스로 말한다 — 제공자에게 묻지 않는다.
            assert!(markdown.contains(&format!("## {}", note_sections(&note)[0].0)));
        }
    }

    // ── 빈 섹션 · 빈 항목 ────────────────────────────────────────────────────────

    #[test]
    fn an_empty_list_does_not_become_an_empty_heading() {
        // 배열이 비는 것은 정상이다 (ADR-0008 §7.3). 빈 제목은 그 사실을 말해 주지 않는다.
        let recording = recording();
        let transcript = transcript();
        let note = StructuredNote::Meeting(MeetingNote {
            overview: "짧은 회의.".to_owned(),
            key_discussions: vec![],
            decisions: vec![],
            action_items: vec![],
            open_questions: vec![],
        });

        let markdown = render(&document(&recording, &transcript, Some(&note)));

        assert!(markdown.contains("## Overview\n짧은 회의."));
        assert!(!markdown.contains("## Decisions"));
        assert!(!markdown.contains("## Action Items"));
        assert!(!markdown.contains("\n\n\n"));
    }

    #[test]
    fn a_list_item_stays_one_line() {
        let recording = recording();
        let transcript = transcript();
        let note = StructuredNote::Summary(SummaryNote {
            short_summary: "요약.".to_owned(),
            key_points: vec!["첫\n줄과  둘째 줄".to_owned(), "   ".to_owned()],
        });

        let markdown = render(&document(&recording, &transcript, Some(&note)));

        assert!(markdown.contains("## Key Points\n- 첫 줄과 둘째 줄\n"));
        // 빈 항목은 `- `만 남기지 않고 사라진다.
        assert!(!markdown.contains("- \n"));
    }

    // ── Transcript ───────────────────────────────────────────────────────────────

    #[test]
    fn a_transcript_without_segments_falls_back_to_its_raw_text() {
        let recording = recording();
        let transcript = Transcript {
            segments: vec![],
            ..transcript()
        };

        let markdown = render(&document(&recording, &transcript, None));

        assert!(markdown.ends_with(
            "## Transcript\n안녕하세요. 오늘은 3DGS를 봅니다. 먼저 splat 표현부터 보겠습니다.\n"
        ));
    }

    #[test]
    fn a_transcript_with_nothing_in_it_leaves_no_empty_section() {
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

        let markdown = render(&document(&recording, &transcript, None));

        assert_eq!(
            markdown,
            "# 3DGS Study #04\n\nDate: 2026-09-01\nDuration: 52:31\n"
        );
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

        let markdown = render(&document(&recording, &transcript, None));

        // 정렬하지 않는다 — 순서를 만드는 것은 전사이고, 여기서 바꾸면 원본과 문서가 달라진다.
        let later = markdown.find("나중 것").expect("첫 segment가 있어야 한다");
        let earlier = markdown.find("먼저 것").expect("둘째 segment가 있어야 한다");
        assert!(later < earlier);
    }

    #[test]
    fn a_timestamp_is_always_two_digit_hours_minutes_and_seconds() {
        assert_eq!(format_timestamp_ms(3_000), "00:00:03");
        assert_eq!(format_timestamp_ms(0), "00:00:00");
        assert_eq!(format_timestamp_ms(999), "00:00:00");
        assert_eq!(format_timestamp_ms(61_000), "00:01:01");
        assert_eq!(format_timestamp_ms(3_151_000), "00:52:31");
        assert_eq!(format_timestamp_ms(3_661_000), "01:01:01");
        // 존재할 수 없는 값도 문서 하나 때문에 앱을 멈추지 않는다.
        assert_eq!(format_timestamp_ms(-1), "00:00:00");
        assert_eq!(format_timestamp_ms(360_000_000), "100:00:00");
        assert_eq!(format_timestamp_ms(i64::MAX).matches(':').count(), 2);
    }

    // ── 메타데이터 ───────────────────────────────────────────────────────────────

    #[test]
    fn the_date_line_is_the_stored_text_cut_not_a_computed_local_date() {
        let recording = Recording {
            created_at: "2026-09-01T23:59:59.999Z".to_owned(),
            ..recording()
        };
        let transcript = transcript();

        let markdown = render(&document(&recording, &transcript, None));

        assert!(markdown.contains("\nDate: 2026-09-01\n"));
    }

    #[test]
    fn a_created_at_of_an_unexpected_shape_is_written_as_it_came() {
        let transcript = transcript();

        for (created_at, expected) in [
            ("2026/09/01 10:00", "2026/09/01 10:00"),
            ("어제", "어제"),
            ("  ", UNKNOWN_DATE),
            ("", UNKNOWN_DATE),
        ] {
            let recording = Recording {
                created_at: created_at.to_owned(),
                ..recording()
            };

            let markdown = render(&document(&recording, &transcript, None));

            assert!(
                markdown.contains(&format!("\nDate: {expected}\n")),
                "created_at {created_at:?}가 {expected:?}로 나오지 않았다"
            );
        }
    }

    #[test]
    fn the_title_is_one_line_and_never_empty() {
        let transcript = transcript();

        for (title, expected) in [
            ("3DGS Study #04", "3DGS Study #04"),
            ("회의:\n로드맵", "회의: 로드맵"),
            ("  앞뒤 공백  ", "앞뒤 공백"),
            ("", UNTITLED_TITLE),
            ("   \n ", UNTITLED_TITLE),
        ] {
            let recording = Recording {
                title: title.to_owned(),
                ..recording()
            };

            let markdown = render(&document(&recording, &transcript, None));

            assert!(
                markdown.starts_with(&format!("# {expected}\n")),
                "title {title:?}가 {expected:?}로 나오지 않았다"
            );
        }
    }

    #[test]
    fn the_duration_is_the_one_the_rest_of_the_app_shows() {
        let recording = recording();
        let transcript = transcript();

        let markdown = render(&document(&recording, &transcript, None));

        assert!(markdown.contains(&format!(
            "\nDuration: {}\n",
            format_duration_ms(recording.duration_ms)
        )));
    }

    // ── 결정성 ───────────────────────────────────────────────────────────────────

    #[test]
    fn the_same_input_always_renders_the_same_string() {
        // 두 번 렌더해서 비교한다 — 시계도 난수도 해시맵 순회도 없다 (§18).
        let recording = recording();
        let transcript = transcript();

        for note in [None, Some(meeting_note()), Some(study_note()), Some(summary_note())] {
            let document = document(&recording, &transcript, note.as_ref());

            assert_eq!(render(&document), render(&document));
        }
    }

    #[test]
    fn the_document_ends_with_exactly_one_newline() {
        let recording = recording();
        let transcript = transcript();
        let note = meeting_note();

        for note in [None, Some(&note)] {
            let markdown = render(&document(&recording, &transcript, note));

            assert!(markdown.ends_with('\n'));
            assert!(!markdown.ends_with("\n\n"));
        }
    }
}
