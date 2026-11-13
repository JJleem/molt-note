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
//! ## Transcript 본문의 모양은 둘이지만 규칙은 하나다 ([`TranscriptShape`])
//!
//! §11의 export 파일은 segment 하나를 `### HH:MM:SS` 제목 + 본문으로 적고, AI Handoff 경로는
//! 같은 segment를 `HH:MM:SS 문장` 한 줄로 적는다. **어느 segment를 적는가 · 어떤 순서로 적는가 ·
//! 비어 있으면 어떻게 하는가 · segment가 하나도 없을 때 무엇을 적는가는 두 모양이 공유한다** —
//! 그 규칙이 [`transcript_body`] 하나에만 있기 때문이다. 모양을 고르는 것은 부르는 쪽이며,
//! 규칙을 복제하는 자리는 이 저장소에 없다 (ADR-0010 §5.4).
//!
//! ## 섹션 제목의 출처는 §9.5 하나다
//!
//! `## Overview` · `## Key Discussions` … 는 §9.5 표의 **출력 섹션 이름 그대로**이며,
//! [`MEETING_SECTIONS`] · [`STUDY_SECTIONS`] · [`SUMMARY_SECTIONS`]가 그 이름을 값으로 들고
//! 있다. 노트 타입의 필드 이름도 같은 표에서 왔으므로 (ADR-0008 §7.1) 제목과 내용이 어긋날
//! 자리가 없다.

use crate::ai::note::{ActionItemV2, MeetingNoteV2, PreviousActionItems, StructuredNote, Topic};
use crate::domain::{format_duration_ms, Recording, Transcript, TranscriptSegment};

use super::iso_date;

/// 제목이 비어 있을 때 `# ` 자리에 쓰는 값.
///
/// 저장된 Recording의 제목은 비어 있을 수 없지만(`commands::validated`), 문서의 첫 `# `는
/// **비어 있으면 안 된다** — Notion은 첫 h1을 페이지 제목으로 삼고(ADR-0009 §5.3), 빈 제목의
/// 문서는 목록에서 서로 구분되지 않는다.
pub const UNTITLED_TITLE: &str = "제목 없음";

/// `created_at`이 예상한 모양이 아니고 그마저 비어 있을 때 `Date:` 줄에 쓰는 값.
pub const UNKNOWN_DATE: &str = super::filename::UNKNOWN_DATE;

/// Meeting V1 노트의 섹션 제목 — 옛 노트 렌더링에 쓰인다.
pub const MEETING_V1_SECTIONS: [&str; 5] = [
    "Overview",
    "Key Discussions",
    "Decisions",
    "Action Items",
    "Open Questions",
];

/// Meeting V2 노트의 섹션 제목 — `docs/MEETING-NOTE-FORMAT.md` 구조다.
pub const MEETING_SECTIONS: [&str; 9] = [
    "Meeting Title",
    "Meeting Kind",
    "Attendees",
    "Reliability Notice",
    "Key Summary",
    "Previous Action Items",
    "Topics",
    "Action Items",
    "Needs Confirmation",
];

/// Meeting V2가 렌더링하는 `##` 섹션 제목 — 머리말은 인라인, Topics는 동적이므로 여기 없다.
pub const MEETING_V2_SECTIONS: [&str; 4] = [
    "🔑 핵심 요약",
    "🔁 지난주 액션 아이템 진행 상황",
    "✅ 액션 아이템",
    "🔍 확인 필요",
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
/// ## Transcript               ← include_transcript이 false이면 이 섹션이 빠진다
///
/// ### 00:00:03
/// …
/// ```
///
/// 문서는 언제나 개행 하나로 끝나고, 블록 사이에는 빈 줄이 정확히 하나 있다. 같은 입력은
/// 언제나 같은 문자열을 낸다 — 시계도 로캘도 해시맵 순회도 없고, segment는 **받은 순서
/// 그대로** 적는다.
///
/// `include_transcript`가 `false`이면 `## Transcript` 섹션 전체를 생략한다. AI 노트가 있는
/// Meeting 녹음에서 사용자가 Transcript를 빼고 내보내거나 Notion으로 보낼 수 있게 하기
/// 위해서다. AI 노트가 없으면 부르는 쪽이 `true`를 넘겨야 한다 (INV-8).
pub fn render(document: &ExportDocument<'_>, include_transcript: bool) -> String {
    let recording = document.recording;

    // 블록 하나가 문단 하나다. 마지막에 빈 줄 하나로 잇는다 — 이렇게 하면 "언제 빈 줄을
    // 넣는가"를 각 자리가 따로 판단하지 않아도 되고, 빈 섹션이 생길 여지도 없다.
    let mut blocks = vec![
        format!("# {}", heading_text(&recording.title)),
        metadata_block(recording),
    ];

    if let Some(note) = document.note {
        match note {
            StructuredNote::MeetingV2(v2) => blocks.extend(meeting_v2_blocks(v2)),
            _ => {
                for (title, body) in note_sections(note) {
                    // 내용이 없는 섹션은 제목도 남기지 않는다 (INV-8 · ADR-0008 §7.3).
                    if let Some(rendered) = render_body(body) {
                        blocks.push(format!("## {title}\n{rendered}"));
                    }
                }
            }
        }
    }

    if include_transcript {
        // §11의 파일 형식이다 — 이 문서의 모양은 [`TranscriptShape::Sectioned`] 하나뿐이며,
        // AI 경로가 고른 압축 모양은 여기에 닿지 않는다.
        blocks.extend(transcript_blocks(document.transcript, TranscriptShape::Sectioned));
    }

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
        // render()는 MeetingV2를 meeting_v2_blocks()로 직접 렌더한다. 이 경로는
        // note_sections를 직접 호출하는 곳이 있을 때의 안전 경로다.
        StructuredNote::MeetingV2(note) => vec![
            ("Overview", SectionBody::Text(&note.meeting_title)),
            ("Key Summary", SectionBody::List(&note.key_summary)),
            ("Needs Confirmation", SectionBody::List(&note.needs_confirmation)),
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

// ── Meeting V2 전용 렌더링 ─────────────────────────────────────────────────────
//
// V2의 구조(Topics · 기간별 액션 아이템 · PreviousActionItems의 다섯 분류)는
// `SectionBody::Text | List` 둘로 표현되지 않는다. 그래서 `note_sections` +
// `render_body`를 거치지 않고, 블록을 직접 만들어 `render`에 넘긴다.

/// MeetingV2 노트의 블록들 — `docs/MEETING-NOTE-FORMAT.md` 형식.
fn meeting_v2_blocks(note: &MeetingNoteV2) -> Vec<String> {
    let mut blocks = Vec::new();

    // 머리말: 📌 안내 + 메타데이터 ─────────────────────────────────────────
    let mut preamble_parts: Vec<String> = Vec::new();
    let notice = single_line(&note.reliability_notice);
    if !notice.is_empty() {
        preamble_parts.push(format!("> 📌 {notice}"));
    }

    let mut meta_lines: Vec<String> = Vec::new();
    let title = single_line(&note.meeting_title);
    if !title.is_empty() {
        meta_lines.push(format!("- **회의명**: {title}"));
    }
    let kind = single_line(&note.meeting_kind);
    if !kind.is_empty() {
        meta_lines.push(format!("- **유형**: {kind}"));
    }
    let attendees: Vec<String> = note
        .attendees
        .iter()
        .map(|a| single_line(a))
        .filter(|a| !a.is_empty())
        .collect();
    if !attendees.is_empty() {
        meta_lines.push(format!("- **참석자**: {}", attendees.join(", ")));
    }
    if !meta_lines.is_empty() {
        preamble_parts.push(meta_lines.join("\n"));
    }
    if !preamble_parts.is_empty() {
        blocks.push(preamble_parts.join("\n\n"));
    }

    // 🔑 핵심 요약 ────────────────────────────────────────────────────────
    if let Some(body) = render_bullet_list(&note.key_summary) {
        blocks.push(format!("## 🔑 핵심 요약\n{body}"));
    }

    // 🔁 지난주 액션 아이템 진행 상황 (전부 비면 생략) ──────────────────────
    if let Some(body) = render_previous_action_items(&note.previous_action_items) {
        blocks.push(format!("## 🔁 지난주 액션 아이템 진행 상황\n{body}"));
    }

    // 주제별 ## 섹션 ──────────────────────────────────────────────────────
    for (i, topic) in note.topics.iter().enumerate() {
        if let Some(section) = render_topic(i + 1, topic) {
            blocks.push(section);
        }
    }

    // ✅ 액션 아이템 (기간별 그룹) ─────────────────────────────────────────
    if let Some(section) = render_action_items_v2(&note.action_items) {
        blocks.push(section);
    }

    // 🔍 확인 필요 ────────────────────────────────────────────────────────
    if let Some(body) = render_bullet_list(&note.needs_confirmation) {
        blocks.push(format!("## 🔍 확인 필요\n{body}"));
    }

    blocks
}

/// 글머리표 목록을 렌더한다. 빈 항목은 버리고, 전부 비면 `None`.
fn render_bullet_list(items: &[String]) -> Option<String> {
    let lines: Vec<String> = items
        .iter()
        .map(|item| single_line(item))
        .filter(|item| !item.is_empty())
        .map(|item| format!("- {item}"))
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// PreviousActionItems의 다섯 분류를 렌더한다. 전부 비면 `None` (AC-5-3).
fn render_previous_action_items(pai: &PreviousActionItems) -> Option<String> {
    let categories: [(&str, &[String]); 5] = [
        ("완료", &pai.done),
        ("일정·방향 변경", &pai.changed),
        ("진행 중", &pai.in_progress),
        ("진전 없음", &pai.no_progress),
        ("회의에서 언급 없음", &pai.not_mentioned),
    ];

    let mut parts: Vec<String> = Vec::new();
    for (label, items) in &categories {
        let lines: Vec<String> = items
            .iter()
            .map(|item| single_line(item))
            .filter(|item| !item.is_empty())
            .map(|item| format!("- {item}"))
            .collect();
        if !lines.is_empty() {
            parts.push(format!("**{label}**\n{}", lines.join("\n")));
        }
    }

    (!parts.is_empty()).then(|| parts.join("\n"))
}

/// 주제 하나를 `## N. <title>` 섹션으로 렌더한다.
fn render_topic(number: usize, topic: &Topic) -> Option<String> {
    let title = single_line(&topic.title);
    if title.is_empty() {
        return None;
    }

    let mut section = format!("## {number}. {title}");
    for group in &topic.groups {
        let heading = single_line(&group.heading);
        let items: Vec<String> = group
            .items
            .iter()
            .map(|item| single_line(item))
            .filter(|item| !item.is_empty())
            .collect();

        if !heading.is_empty() {
            section.push_str(&format!("\n**{heading}**"));
        }
        for item in &items {
            section.push_str(&format!("\n- {item}"));
        }
    }

    Some(section)
}

/// 액션 아이템을 기간(`period`)별로 묶어 렌더한다.
fn render_action_items_v2(items: &[ActionItemV2]) -> Option<String> {
    if items.is_empty() {
        return None;
    }

    // 안정적인 순서 — 먼저 나온 period가 먼저.
    let mut groups: Vec<(String, Vec<&ActionItemV2>)> = Vec::new();
    for item in items {
        let period = {
            let p = single_line(&item.period);
            if p.is_empty() {
                "기타".to_owned()
            } else {
                p
            }
        };
        if let Some(group) = groups.iter_mut().find(|(p, _)| *p == period) {
            group.1.push(item);
        } else {
            groups.push((period, vec![item]));
        }
    }

    let mut section = "## ✅ 액션 아이템".to_owned();
    for (period, group_items) in &groups {
        section.push_str(&format!("\n### {period}"));
        for item in group_items {
            let task = single_line(&item.task);
            let detail = single_line(&item.detail);
            let owner = single_line(&item.owner);
            let due = single_line(&item.due);

            let mut line = format!("\n- [ ]  **{task}**");
            if !detail.is_empty() {
                line.push_str(&format!(" ({detail})"));
            }
            line.push_str(&format!(" — {owner} · 📅 {due}"));
            section.push_str(&line);
        }
    }

    Some(section)
}

/// segment 하나를 어떤 모양으로 적는가 — **규칙이 아니라 모양이다.**
///
/// ```text
/// Sectioned   ### 00:00:03            §11의 export 파일 · Notion 본문
///             안녕하세요.
///
/// Compact     00:00:03 안녕하세요.     AI Handoff의 세 산출물 (ADR-0010 §5)
/// ```
///
/// **왜 두 번째 모양이 있는가** (`phase-prompt/05.6` R-5). 72분 녹음의 §11 export는 99 KB ·
/// 5,139줄이었고 그중 1,711줄이 `### HH:MM:SS` 제목이었다. 제목 한 줄과 그것을 앞뒤로 감싸는
/// 빈 줄은 사람이 읽는 파일에서는 구조지만, 채팅 창에 붙여 넣는 문자열에서는 **내용 없이
/// 늘어나는 크기**다.
///
/// **압축해도 잃는 것이 없다.** segment 하나가 여전히 자기 timestamp를 갖고 (`00:00:03 문장`),
/// 순서도 개수도 그대로이며, 문장은 한 글자도 지워지지 않는다 — 사라지는 것은 `### ` 네 글자와
/// 줄바꿈뿐이다. 그래서 이것은 요약이 아니라 **같은 내용의 다른 모양**이다.
///
/// **§11의 파일 형식은 이 모양을 모른다.** [`render`]는 언제나 [`TranscriptShape::Sectioned`]로
/// 적으며, 그 사실이 `markdown_export` 테스트의 기대 문자열로 고정돼 있다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TranscriptShape {
    /// §11의 모양 — segment 하나가 `### HH:MM:SS` 제목과 그 아래 본문이다.
    Sectioned,
    /// AI Handoff가 고르는 모양 — segment 하나가 `HH:MM:SS 문장` 한 줄이다.
    Compact,
}

impl TranscriptShape {
    /// segment 하나가 만드는 문자열.
    ///
    /// 압축 모양의 문장은 **한 줄로 접는다** ([`single_line`]) — 한 segment가 한 줄이어야
    /// "이 줄의 timestamp는 이 문장의 것"이 모양만으로 읽히고, 나중에 조각으로 나눌 때
    /// 줄 경계가 곧 문장 경계가 된다 (`super::portion`). 목록 항목을 한 줄로 접는 것과 같은
    /// 이유이며, 접는 것은 공백뿐이라 글자는 하나도 잃지 않는다.
    fn segment_block(self, segment: &TranscriptSegment) -> String {
        let timestamp = format_timestamp_ms(segment.start_ms);

        match self {
            Self::Sectioned => format!("### {timestamp}\n{}", segment.text.trim()),
            Self::Compact => format!("{timestamp} {}", single_line(&segment.text)),
        }
    }

    /// segment 블록들을 잇는 문자열.
    ///
    /// §11에서 블록 하나는 문단 하나이므로 빈 줄로 갈린다. 압축 모양에서 블록 하나는 줄
    /// 하나이므로 개행 하나로 잇는다 — 여기서 빈 줄을 넣으면 압축한 만큼이 도로 늘어난다.
    fn separator(self) -> &'static str {
        match self {
            Self::Sectioned => "\n\n",
            Self::Compact => "\n",
        }
    }
}

/// Transcript 본문이 만드는 것 — **제목을 붙이지 않은 값**이다 (ADR-0010 §5.4).
///
/// 제목까지 필요한 자리(§11의 문서 · Copy Transcript)와 본문만 필요한 자리(프롬프트의
/// `{{transcript}}`)가 **같은 규칙을 쓰되 붙이는 쪽이 붙이도록** 이 값으로 갈라진다 —
/// 완성된 문자열을 잘라 쓰면 규칙이 두 벌이 된다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TranscriptBody<'a> {
    /// 각 블록이 segment 하나다. 받은 순서 그대로이며 정렬하지 않는다.
    Segments {
        blocks: Vec<String>,
        /// 블록을 만든 모양 — 블록 사이를 어떻게 잇는지가 여기서 나온다.
        shape: TranscriptShape,
    },
    /// segment가 하나도 남지 않았을 때의 `raw_text` 한 문단.
    ///
    /// **모양이 없다.** 지어낼 timestamp가 없으므로 압축할 것도 없고, 그래서 두 경로가
    /// 여기서는 글자 하나까지 같은 문자열을 만든다.
    Raw(&'a str),
    /// 적을 것이 없다.
    Empty,
}

impl TranscriptBody<'_> {
    /// 본문을 한 문자열로 잇는다 — 제목은 붙이지 않는다.
    ///
    /// 적을 것이 없으면 **빈 문자열이다.** 제목만 남은 본문을 만들지 않는다.
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Segments { blocks, shape } => blocks.join(shape.separator()),
            Self::Raw(raw_text) => (*raw_text).to_owned(),
            Self::Empty => String::new(),
        }
    }
}

/// Transcript 본문을 만드는 **유일한 규칙** (§11 · ADR-0010 §5.4).
///
/// segment가 있으면 그것을 순서대로 적고, 없으면 `raw_text`를 한 문단으로 적는다.
/// **둘 다 없으면 아무것도 만들지 않는다** — 빈 제목은 "여기 무언가 실패했다"처럼
/// 보이지만 실제로는 적을 것이 없을 뿐이다 (INV-8과 같은 이유).
///
/// `shape`는 **무엇을 적는가가 아니라 어떻게 적는가**만 고른다 ([`TranscriptShape`]). 어느
/// segment가 살아남는지도, 그 순서도, segment가 없을 때의 대체도 모양과 무관하게 같다.
pub(crate) fn transcript_body(
    transcript: &Transcript,
    shape: TranscriptShape,
) -> TranscriptBody<'_> {
    let blocks: Vec<String> = transcript
        .segments
        .iter()
        .filter(|segment| !segment.text.trim().is_empty())
        .map(|segment| shape.segment_block(segment))
        .collect();

    if !blocks.is_empty() {
        return TranscriptBody::Segments { blocks, shape };
    }

    let raw_text = transcript.raw_text.trim();
    if raw_text.is_empty() {
        return TranscriptBody::Empty;
    }

    TranscriptBody::Raw(raw_text)
}

/// Transcript가 만드는 블록들 — `## Transcript`와 [`transcript_body`]가 만든 본문 (§11).
///
/// 블록 하나는 문서를 조립하는 쪽에서 **문단 하나**가 된다(빈 줄로 갈린다). 그래서 §11의
/// 모양에서는 segment마다 블록이 하나씩이고, 압축 모양에서는 **본문 전체가 블록 하나**다 —
/// 압축한 줄들 사이에 빈 줄이 다시 끼어들면 압축한 이유가 사라진다.
///
/// 적을 것이 없으면 **섹션 자체를 만들지 않는다.**
pub(crate) fn transcript_blocks(transcript: &Transcript, shape: TranscriptShape) -> Vec<String> {
    match transcript_body(transcript, shape) {
        TranscriptBody::Segments {
            blocks,
            shape: TranscriptShape::Sectioned,
        } => {
            let mut section = vec![format!("## {TRANSCRIPT_SECTION}")];
            section.extend(blocks);
            section
        }
        TranscriptBody::Segments {
            blocks,
            shape: shape @ TranscriptShape::Compact,
        } => vec![format!(
            "## {TRANSCRIPT_SECTION}\n{}",
            blocks.join(shape.separator())
        )],
        TranscriptBody::Raw(raw_text) => vec![format!("## {TRANSCRIPT_SECTION}\n{raw_text}")],
        TranscriptBody::Empty => Vec::new(),
    }
}

/// Recording의 메타데이터 블록 — `Date:` · `Duration:` 두 줄 (§11).
///
/// **읽는 필드는 `created_at` · `duration_ms` 둘뿐이다.** 오디오 경로도 형식도 여기에 닿지
/// 않는다 (INV-6). AI-ready 문서의 `## Recording`도 이 함수가 만든 두 줄을 그대로 쓴다
/// (ADR-0010 §5.2) — 두 벌이 되면 같은 녹음이 문서마다 다른 날짜를 갖게 된다.
pub(crate) fn metadata_block(recording: &Recording) -> String {
    format!(
        "Date: {}\nDuration: {}",
        date_text(&recording.created_at),
        format_duration_ms(recording.duration_ms)
    )
}

/// 문서의 첫 `# ` 줄에 쓸 제목.
///
/// **한 줄로 만든다.** 제목 안의 개행이 그대로 나가면 `# ` 다음 줄부터는 제목이 아니게 되고,
/// Notion은 첫 h1을 페이지 제목으로 삼으므로(ADR-0009 §5.3) 그 경계가 문서마다 달라진다.
pub(crate) fn heading_text(title: &str) -> String {
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

    use crate::ai::note::{
        ActionItemV2, MeetingNote, MeetingNoteV2, PreviousActionItems, StudyNote, SummaryNote,
        Topic, TopicGroup,
    };
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
            transcription_ms: None,
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

    fn meeting_v2_note() -> StructuredNote {
        StructuredNote::MeetingV2(MeetingNoteV2 {
            meeting_title: "분기 계획 회의".to_owned(),
            meeting_kind: "정기 위클리 (일정 확정)".to_owned(),
            attendees: vec!["김철수".to_owned(), "이영희".to_owned()],
            reliability_notice: "1시간 53분 녹음을 자동 전사 후 정리했습니다.".to_owned(),
            key_summary: vec!["**9월 출시 확정**: 일정이 최종 합의됨".to_owned()],
            previous_action_items: PreviousActionItems {
                done: vec!["RFP 초안 완료".to_owned()],
                changed: vec![],
                in_progress: vec![],
                no_progress: vec![],
                not_mentioned: vec![],
            },
            topics: vec![Topic {
                title: "일정 확정".to_owned(),
                groups: vec![TopicGroup {
                    heading: "".to_owned(),
                    items: vec!["9월 출시로 합의".to_owned()],
                }],
            }],
            action_items: vec![ActionItemV2 {
                task: "초안 작성".to_owned(),
                detail: "RFP 기반".to_owned(),
                owner: "김철수".to_owned(),
                due: "11/20".to_owned(),
                period: "이번 주".to_owned(),
            }],
            needs_confirmation: vec![],
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

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

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

        let markdown = render(&document(&recording, &transcript, None), true);

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

        let markdown = render(&document(&recording, &transcript, None), true);

        // 메타데이터와 Transcript는 그대로 있다.
        assert!(markdown.starts_with("# 3DGS Study #04\n"));
        assert!(markdown.contains("\nDate: 2026-09-01\n"));
        assert!(markdown.contains("\nDuration: 52:31\n"));
        assert!(markdown.contains("\n## Transcript\n"));
        assert!(markdown.contains("\n### 00:00:03\n"));

        // AI 섹션의 껍데기는 하나도 없다.
        for section in MEETING_SECTIONS
            .iter()
            .chain(MEETING_V2_SECTIONS.iter())
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
            (meeting_note(), MEETING_V1_SECTIONS.as_slice()),
            (study_note(), STUDY_SECTIONS.as_slice()),
            (summary_note(), SUMMARY_SECTIONS.as_slice()),
        ] {
            let titles: Vec<&str> = note_sections(&note)
                .into_iter()
                .map(|(title, _)| title)
                .collect();

            assert_eq!(titles, expected, "{}의 섹션이 §9.5와 다르다", note.mode());
        }

        // V1 meeting(5) + V2 meeting(9) + study(6) + summary(2) 제목이 각각 있다.
        assert_eq!(MEETING_V1_SECTIONS.len(), 5);
        assert_eq!(MEETING_SECTIONS.len(), 9);
    }

    #[test]
    fn every_mode_renders_its_own_sections_and_no_others() {
        let recording = recording();
        let transcript = transcript();

        for (note, expected) in [
            (meeting_note(), MEETING_V1_SECTIONS.as_slice()),
            (study_note(), STUDY_SECTIONS.as_slice()),
            (summary_note(), SUMMARY_SECTIONS.as_slice()),
        ] {
            let markdown = render(&document(&recording, &transcript, Some(&note)), true);
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
            let markdown = render(&document(&recording, &transcript, Some(&note)), true);

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

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

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

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

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

        let markdown = render(&document(&recording, &transcript, None), true);

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

        let markdown = render(&document(&recording, &transcript, None), true);

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

        let markdown = render(&document(&recording, &transcript, None), true);

        // 정렬하지 않는다 — 순서를 만드는 것은 전사이고, 여기서 바꾸면 원본과 문서가 달라진다.
        let later = markdown.find("나중 것").expect("첫 segment가 있어야 한다");
        let earlier = markdown.find("먼저 것").expect("둘째 segment가 있어야 한다");
        assert!(later < earlier);
    }

    // ── 압축 모양 (`phase-prompt/05.6` R-5 · ADR-0010 §5.4) ──────────────────────

    #[test]
    fn the_compact_shape_writes_one_line_per_segment_with_its_own_timestamp() {
        let transcript = transcript();

        let body = transcript_body(&transcript, TranscriptShape::Compact);

        assert_eq!(
            body.text(),
            [
                "00:00:03 안녕하세요. 오늘은 3DGS를 봅니다.",
                "00:00:06 먼저 splat 표현부터 보겠습니다.",
            ]
            .join("\n")
        );
        // 제목 줄이 사라졌다 — 그것이 압축한 것의 전부다.
        assert!(!body.text().contains("### "));
        // 줄 수가 segment 수와 같다 — timestamp 하나에 문장 하나다.
        assert_eq!(body.text().lines().count(), transcript.segments.len());
    }

    #[test]
    fn the_compact_shape_keeps_every_timestamp_and_every_word_of_the_transcript() {
        // 압축은 요약이 아니다 — segment의 timestamp도, 그 문장의 낱말도 하나도 빠지지 않는다.
        let transcript = Transcript {
            segments: vec![
                TranscriptSegment {
                    start_ms: 3_000,
                    end_ms: 6_500,
                    text: "여러 줄로\n나뉜  문장입니다.".to_owned(),
                },
                TranscriptSegment {
                    start_ms: 3_661_000,
                    end_ms: 3_665_000,
                    text: "한 시간이 넘은 자리의 문장.".to_owned(),
                },
            ],
            ..transcript()
        };

        let compact = transcript_body(&transcript, TranscriptShape::Compact).text();

        for segment in &transcript.segments {
            let timestamp = format_timestamp_ms(segment.start_ms);
            assert!(compact.contains(&timestamp), "{timestamp}가 사라졌다");

            for word in segment.text.split_whitespace() {
                assert!(compact.contains(word), "{word}가 사라졌다");
            }
        }
        // segment 안의 개행은 공백으로 접힌다 — 한 segment가 한 줄이다.
        assert_eq!(compact.lines().count(), 2);
        assert!(compact.starts_with("00:00:03 여러 줄로 나뉜 문장입니다.\n"));
        assert!(compact.ends_with("01:01:01 한 시간이 넘은 자리의 문장."));
    }

    #[test]
    fn the_compact_shape_is_smaller_than_the_section_11_shape_for_the_same_transcript() {
        // R-5가 잰 것은 제목 1,711개다. 같은 전사에서 압축 모양이 실제로 더 작다는 것을
        // 여기서 고정한다 — 작아지지 않으면 이 모양이 있을 이유가 없다.
        let segments: Vec<TranscriptSegment> = (0..200)
            .map(|index| TranscriptSegment {
                start_ms: index * 3_000,
                end_ms: index * 3_000 + 3_000,
                text: format!("{index}번째 구간의 문장입니다."),
            })
            .collect();
        let transcript = Transcript {
            segments,
            ..transcript()
        };

        let sectioned = transcript_body(&transcript, TranscriptShape::Sectioned).text();
        let compact = transcript_body(&transcript, TranscriptShape::Compact).text();

        assert!(
            compact.len() < sectioned.len(),
            "압축 모양이 {}바이트로 §11 모양 {}바이트보다 작지 않다",
            compact.len(),
            sectioned.len()
        );
        // 줄 수도 함께 줄어든다 — 제목 줄과 빈 줄이 사라진 만큼이다.
        assert_eq!(compact.lines().count(), 200);
        assert_eq!(sectioned.lines().count(), 200 * 3 - 1);
    }

    #[test]
    fn both_shapes_choose_the_same_segments_and_fall_back_the_same_way() {
        // 모양이 고르는 것은 **어떻게 적는가**뿐이다 — 무엇을 적는가는 규칙 하나가 정한다.
        let transcript = Transcript {
            segments: vec![
                TranscriptSegment {
                    start_ms: 1_000,
                    end_ms: 2_000,
                    text: "   ".to_owned(),
                },
                TranscriptSegment {
                    start_ms: 2_000,
                    end_ms: 3_000,
                    text: "남는 문장".to_owned(),
                },
            ],
            ..transcript()
        };

        for shape in [TranscriptShape::Sectioned, TranscriptShape::Compact] {
            // 빈 segment는 어느 모양에서도 남지 않는다.
            let TranscriptBody::Segments { blocks, .. } = transcript_body(&transcript, shape)
            else {
                panic!("{shape:?}: segment 본문이어야 한다");
            };
            assert_eq!(blocks.len(), 1, "{shape:?}: 고른 segment가 다르다");

            // segment가 하나도 없으면 둘 다 raw_text로 내려가고, 그 문자열은 같다.
            let without_segments = Transcript {
                segments: vec![],
                ..transcript.clone()
            };
            assert_eq!(
                transcript_body(&without_segments, shape).text(),
                without_segments.raw_text
            );

            // 적을 것이 하나도 없으면 둘 다 아무것도 만들지 않는다.
            let nothing = Transcript {
                segments: vec![],
                raw_text: "  \n ".to_owned(),
                ..transcript.clone()
            };
            assert_eq!(transcript_body(&nothing, shape), TranscriptBody::Empty);
            assert_eq!(transcript_blocks(&nothing, shape), Vec::<String>::new());
        }
    }

    #[test]
    fn the_compact_transcript_section_is_one_block_and_the_section_11_one_is_not() {
        // 조립하는 쪽은 블록 사이에 빈 줄을 넣는다 — 압축 본문이 여러 블록이면 압축한 만큼이
        // 도로 늘어난다.
        let transcript = transcript();

        let compact = transcript_blocks(&transcript, TranscriptShape::Compact);
        assert_eq!(compact.len(), 1);
        assert_eq!(
            compact[0],
            "## Transcript\n00:00:03 안녕하세요. 오늘은 3DGS를 봅니다.\n00:00:06 먼저 splat 표현부터 보겠습니다."
        );

        let sectioned = transcript_blocks(&transcript, TranscriptShape::Sectioned);
        assert_eq!(sectioned.len(), 1 + transcript.segments.len());
        assert_eq!(sectioned[0], "## Transcript");
    }

    #[test]
    fn the_section_11_document_is_never_written_in_the_compact_shape() {
        // §11의 파일 형식은 이 Phase가 바꾸지 않는다 — 문서에는 여전히 `### ` 제목이 있다.
        let recording = recording();
        let transcript = transcript();

        let markdown = render(&document(&recording, &transcript, None), true);

        assert!(markdown.contains("\n### 00:00:03\n안녕하세요. 오늘은 3DGS를 봅니다.\n"));
        assert!(
            !markdown.contains("00:00:03 안녕하세요"),
            "§11 문서에 압축 모양이 새어 들어갔다"
        );
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

        let markdown = render(&document(&recording, &transcript, None), true);

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

            let markdown = render(&document(&recording, &transcript, None), true);

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

            let markdown = render(&document(&recording, &transcript, None), true);

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

        let markdown = render(&document(&recording, &transcript, None), true);

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

        for note in [
            None,
            Some(meeting_note()),
            Some(meeting_v2_note()),
            Some(study_note()),
            Some(summary_note()),
        ] {
            let document = document(&recording, &transcript, note.as_ref());

            assert_eq!(render(&document, true), render(&document, true));
        }
    }

    #[test]
    fn the_document_ends_with_exactly_one_newline() {
        let recording = recording();
        let transcript = transcript();
        let note = meeting_note();
        let v2 = meeting_v2_note();

        for note in [None, Some(&note), Some(&v2)] {
            let markdown = render(&document(&recording, &transcript, note), true);

            assert!(markdown.ends_with('\n'));
            assert!(!markdown.ends_with("\n\n"));
        }
    }

    // ── Meeting V2 렌더링 (AC-5-1 · AC-5-2 · AC-5-3) ──────────────────────────

    #[test]
    fn a_v2_meeting_renders_the_reference_format_sections_in_order() {
        let recording = recording();
        let transcript = transcript();
        let note = meeting_v2_note();

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

        assert_eq!(
            markdown,
            [
                "# 3DGS Study #04",
                "",
                "Date: 2026-09-01",
                "Duration: 52:31",
                "",
                "> 📌 1시간 53분 녹음을 자동 전사 후 정리했습니다.",
                "",
                "- **회의명**: 분기 계획 회의",
                "- **유형**: 정기 위클리 (일정 확정)",
                "- **참석자**: 김철수, 이영희",
                "",
                "## 🔑 핵심 요약",
                "- **9월 출시 확정**: 일정이 최종 합의됨",
                "",
                "## 🔁 지난주 액션 아이템 진행 상황",
                "**완료**",
                "- RFP 초안 완료",
                "",
                "## 1. 일정 확정",
                "- 9월 출시로 합의",
                "",
                "## ✅ 액션 아이템",
                "### 이번 주",
                "- [ ]  **초안 작성** (RFP 기반) — 김철수 · 📅 11/20",
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
    fn v1_meeting_renders_exactly_like_before() {
        let recording = recording();
        let transcript = transcript();
        let note = meeting_note();

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

        // V1 섹션이 그대로 나온다.
        assert!(markdown.contains("## Overview\n분기 계획 회의."));
        assert!(markdown.contains("## Key Discussions\n- 일정"));
        assert!(markdown.contains("## Decisions\n- 9월 출시"));
        assert!(markdown.contains("## Action Items\n- 초안 작성"));
        // V2 이모지가 새어 들어가지 않는다.
        assert!(!markdown.contains("🔑"));
        assert!(!markdown.contains("🔁"));
        assert!(!markdown.contains("✅"));
        assert!(!markdown.contains("📌"));
    }

    #[test]
    fn previous_action_items_section_is_omitted_when_all_categories_are_empty() {
        let recording = recording();
        let transcript = transcript();
        let note = StructuredNote::MeetingV2(MeetingNoteV2 {
            meeting_title: "회의".to_owned(),
            meeting_kind: "위클리".to_owned(),
            attendees: vec![],
            reliability_notice: "녹음 기반.".to_owned(),
            key_summary: vec!["요약".to_owned()],
            previous_action_items: PreviousActionItems {
                done: vec![],
                changed: vec![],
                in_progress: vec![],
                no_progress: vec![],
                not_mentioned: vec![],
            },
            topics: vec![],
            action_items: vec![],
            needs_confirmation: vec![],
        });

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

        assert!(!markdown.contains("🔁"), "빈 지난 액션 아이템 섹션이 남았다");
        assert!(markdown.contains("## 🔑 핵심 요약"), "핵심 요약은 있어야 한다");
    }

    #[test]
    fn v2_needs_confirmation_section_is_omitted_when_empty() {
        let recording = recording();
        let transcript = transcript();
        let note = meeting_v2_note(); // needs_confirmation is empty

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

        assert!(!markdown.contains("🔍"), "빈 확인 필요 섹션이 남았다");
    }

    #[test]
    fn v2_action_items_are_grouped_by_period() {
        let recording = recording();
        let transcript = transcript();
        let note = StructuredNote::MeetingV2(MeetingNoteV2 {
            meeting_title: "회의".to_owned(),
            meeting_kind: "위클리".to_owned(),
            attendees: vec![],
            reliability_notice: "녹음.".to_owned(),
            key_summary: vec!["요약".to_owned()],
            previous_action_items: PreviousActionItems {
                done: vec![], changed: vec![], in_progress: vec![],
                no_progress: vec![], not_mentioned: vec![],
            },
            topics: vec![],
            action_items: vec![
                ActionItemV2 {
                    task: "A".to_owned(), detail: "".to_owned(),
                    owner: "김".to_owned(), due: "11/20".to_owned(),
                    period: "이번 주".to_owned(),
                },
                ActionItemV2 {
                    task: "B".to_owned(), detail: "설명".to_owned(),
                    owner: "이".to_owned(), due: "12/01".to_owned(),
                    period: "다음 주".to_owned(),
                },
                ActionItemV2 {
                    task: "C".to_owned(), detail: "".to_owned(),
                    owner: "박".to_owned(), due: "11/22".to_owned(),
                    period: "이번 주".to_owned(),
                },
            ],
            needs_confirmation: vec![],
        });

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

        // 이번 주 그룹이 먼저 오고, 다음 주 그룹이 뒤에 온다.
        let this_week = markdown.find("### 이번 주").expect("이번 주 그룹");
        let next_week = markdown.find("### 다음 주").expect("다음 주 그룹");
        assert!(this_week < next_week);

        // A와 C가 같은 그룹에, B가 다음 그룹에.
        assert!(markdown.contains("**A** — 김"));
        assert!(markdown.contains("**C** — 박"));
        assert!(markdown.contains("**B** (설명) — 이"));
    }

    #[test]
    fn v2_topics_render_numbered_sections_with_bold_subheadings() {
        let recording = recording();
        let transcript = transcript();
        let note = StructuredNote::MeetingV2(MeetingNoteV2 {
            meeting_title: "회의".to_owned(),
            meeting_kind: "위클리".to_owned(),
            attendees: vec![],
            reliability_notice: "녹음.".to_owned(),
            key_summary: vec!["요약".to_owned()],
            previous_action_items: PreviousActionItems {
                done: vec![], changed: vec![], in_progress: vec![],
                no_progress: vec![], not_mentioned: vec![],
            },
            topics: vec![
                Topic {
                    title: "일정".to_owned(),
                    groups: vec![
                        TopicGroup {
                            heading: "결정 사항".to_owned(),
                            items: vec!["9월 출시".to_owned()],
                        },
                        TopicGroup {
                            heading: "".to_owned(),
                            items: vec!["추가 논의 필요".to_owned()],
                        },
                    ],
                },
                Topic {
                    title: "예산".to_owned(),
                    groups: vec![TopicGroup {
                        heading: "".to_owned(),
                        items: vec!["500만원 배정".to_owned()],
                    }],
                },
            ],
            action_items: vec![],
            needs_confirmation: vec![],
        });

        let markdown = render(&document(&recording, &transcript, Some(&note)), true);

        assert!(markdown.contains("## 1. 일정\n**결정 사항**\n- 9월 출시\n- 추가 논의 필요"));
        assert!(markdown.contains("## 2. 예산\n- 500만원 배정"));
    }

    // ── include_transcript (AC-6-1 · AC-6-2) ──────────────────────────────────

    #[test]
    fn include_transcript_false_omits_the_transcript_section() {
        // AC-6-1: include_transcript=false로 render하면 출력에 ## Transcript가 없다.
        let recording = recording();
        let transcript = transcript();
        let note = study_note();

        let markdown = render(&document(&recording, &transcript, Some(&note)), false);

        assert!(!markdown.contains("## Transcript"), "Transcript 섹션이 남았다");
        assert!(!markdown.contains("### 00:00:03"), "segment가 남았다");
        // AI 섹션은 그대로 있다.
        assert!(markdown.contains("## Overview\n3DGS 스터디 4회차."));
        assert!(markdown.contains("## Key Concepts"));
        // 메타데이터도 그대로다.
        assert!(markdown.starts_with("# 3DGS Study #04\n"));
        assert!(markdown.contains("Date: 2026-09-01"));
        assert!(markdown.contains("Duration: 52:31"));
        // 문서는 개행 하나로 끝난다.
        assert!(markdown.ends_with('\n'));
        assert!(!markdown.ends_with("\n\n"));
    }

    #[test]
    fn include_transcript_true_keeps_transcript_as_before() {
        // AC-6-2: include_transcript=true로 render하면 기존과 동일하게 ## Transcript가 있다.
        let recording = recording();
        let transcript = transcript();
        let note = study_note();

        let with = render(&document(&recording, &transcript, Some(&note)), true);
        let without = render(&document(&recording, &transcript, Some(&note)), false);

        assert!(with.contains("## Transcript"));
        assert!(with.contains("### 00:00:03"));
        assert!(!without.contains("## Transcript"));
        // true일 때의 문서 모양은 기존 §11 기대와 같다.
        assert!(with.contains("\n## Transcript\n\n### 00:00:03\n"));
    }

    #[test]
    fn include_transcript_false_without_ai_note_still_omits_transcript() {
        // 렌더러 자체는 AI 노트 유무를 판단하지 않는다. 부르는 쪽이 정한다.
        let recording = recording();
        let transcript = transcript();

        let markdown = render(&document(&recording, &transcript, None), false);

        assert!(!markdown.contains("## Transcript"));
        assert_eq!(
            markdown,
            "# 3DGS Study #04\n\nDate: 2026-09-01\nDuration: 52:31\n"
        );
    }

    #[test]
    fn include_transcript_false_still_produces_a_valid_document_ending_with_one_newline() {
        let recording = recording();
        let transcript = transcript();

        for note in [None, Some(meeting_note()), Some(meeting_v2_note()), Some(study_note()), Some(summary_note())] {
            let markdown = render(&document(&recording, &transcript, note.as_ref()), false);

            assert!(markdown.ends_with('\n'), "개행 하나로 끝나야 한다");
            assert!(!markdown.ends_with("\n\n"), "빈 줄로 끝나면 안 된다");
            assert!(!markdown.contains("## Transcript"), "Transcript가 남았다");
        }
    }
}
