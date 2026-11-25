//! 표가 있는 V2 Meeting 노트의 end-to-end 검증 — **실제 provider 호출도 네트워크도 없이**
//! Notion 전송이 쓰는 경로(`export::markdown::render` → `notion::chunk::split_markdown`)를
//! Transcript 포함 · 제외 두 선택지로 모두 통과시킨다 (`phase-prompt/05.12` Required Outcome 2 · 7).
//!
//! ```text
//! MeetingNoteV2 (표 포함, 주제 여러 개)
//!   → encode_content → decode_content          (저장된 노트를 읽는 것과 같은 값)
//!   → render(&ExportDocument, include_transcript)   (sync::run::document_of가 부르는 렌더러)
//!   → split_markdown                            (sync::run::send가 부르는 분할)
//! ```
//!
//! 확인하는 것:
//!
//! - 모든 표가 **한 chunk 안에 통째로** 있다 — 머리글 없는 `|` 줄 조각이 어느 chunk에도 없다.
//! - 어떤 글머리표 줄에도 표 문법(`- | a | b | |---|`)이 없다.
//! - 결과 어디에도 `[HH:MM:SS]`가 없다.
//! - 표가 없는 노트는 이전과 **글자 하나까지 같은** 문서를 만든다.

use molt_note_lib::ai::note::{
    decode_content, encode_content, parse_note, ActionItemV2, MeetingNoteV2, PreviousActionItems,
    StructuredNote, Topic, TopicGroup, TopicTable,
};
use molt_note_lib::domain::{
    NoteType, ProcessingStatus, Recording, RecordingId, Transcript, TranscriptId,
    TranscriptSegment,
};
use molt_note_lib::export::markdown::{render, ExportDocument};
use molt_note_lib::notion::chunk::{split_markdown, CHUNK_MAX_BLOCK_UNITS, CHUNK_MAX_BYTES};

// ── fixture ────────────────────────────────────────────────────────────────────

const TOPIC_COUNT: usize = 30;

/// `index`번째 주제의 비교표 — 머리글과 마지막 줄에 주제 번호가 들어가 문서 안에서 유일하다.
/// 셀 하나에는 `|`와 줄바꿈이 섞여 있다 (렌더러가 막아야 하는 입력).
fn comparison_table(index: usize) -> TopicTable {
    let mut rows: Vec<Vec<String>> = (0..12)
        .map(|row| {
            vec![
                format!("후보 {index}-{row}"),
                format!("의견: 비용과 일정 측면에서 {row}번째 안을 검토했다"),
                format!("{row}주"),
            ]
        })
        .collect();
    rows.push(vec![
        format!("마지막 후보 {index}"),
        "A | B 중 하나\n둘째 줄".to_owned(),
        "미정".to_owned(),
    ]);

    TopicTable {
        columns: vec![
            format!("비교 {index}"),
            "의견".to_owned(),
            "기간".to_owned(),
        ],
        rows,
    }
}

fn long_item(topic: usize, item: usize) -> String {
    format!(
        "{topic}번 주제의 {item}번째 논의 — 담당자가 현재 진행 상황과 남은 작업, 예상 위험과 \
         대응 방안을 설명했고 다음 회의 전까지 세부 일정을 다시 정리해 공유하기로 했다."
    )
}

/// 표가 있는 주제와 없는 주제가 섞인 큰 V2 노트. 문자열 항목 안에 표 문법이 들어온
/// 경우(모델이 지시를 어긴 경우)도 두 가지 넣는다.
fn note_with_tables(lead: usize) -> MeetingNoteV2 {
    let topics = (0..TOPIC_COUNT)
        .map(|index| {
            let mut first_items: Vec<String> = (0..4).map(|item| long_item(index, item)).collect();
            if index == 3 {
                // 여러 줄의 표 문법이 문자열 항목에 섞여 왔다 → 표 블록으로 꺼내져야 한다.
                first_items.push(
                    "후보 비교는 아래와 같다.\n| 항목 속 표 | 의견 |\n|---|:-:|\n| A안 | 좋음 |\n| 항목 속 표 끝 | 보통 |\n결론은 A안이다."
                        .to_owned(),
                );
            }
            if index == 5 {
                // 이미 한 줄로 접힌 표 문법 → 깨진 표 글머리표가 되면 안 된다.
                first_items.push("| 후보 | 의견 | |---|---| | A | 좋음 |".to_owned());
            }

            Topic {
                title: format!("{index}번 주제: 일정과 리소스 점검"),
                groups: vec![
                    TopicGroup {
                        heading: "논의".to_owned(),
                        items: first_items,
                        table: (index % 2 == 0).then(|| comparison_table(index)),
                    },
                    TopicGroup {
                        heading: "후속".to_owned(),
                        items: (4..7).map(|item| long_item(index, item)).collect(),
                        table: (index % 3 == 0).then(|| comparison_table(index + 1_000)),
                    },
                ],
            }
        })
        .collect();

    MeetingNoteV2 {
        meeting_title: "Q4 로드맵 정기 회의".to_owned(),
        meeting_kind: "정기 위클리".to_owned(),
        attendees: vec!["김철수".to_owned(), "이영희".to_owned()],
        reliability_notice: "녹음을 자동 전사 후 정리했습니다. 이름과 용어는 틀릴 수 있습니다."
            .to_owned(),
        // lead만큼 줄 수를 바꿔 chunk 경계가 문서 안에서 움직이게 한다.
        key_summary: (0..=lead)
            .map(|index| format!("**요약 {index}**: 일정이 합의됐다"))
            .collect(),
        previous_action_items: PreviousActionItems {
            done: vec!["RFP 초안 완료".to_owned()],
            changed: vec![],
            in_progress: vec!["성능 최적화 진행 중".to_owned()],
            no_progress: vec![],
            not_mentioned: vec![],
        },
        topics,
        action_items: vec![ActionItemV2 {
            task: "마이그레이션 가이드 작성".to_owned(),
            detail: "토큰 매핑 표 포함".to_owned(),
            owner: "이영희".to_owned(),
            due: "11/18 (월)".to_owned(),
            period: "이번 주".to_owned(),
        }],
        needs_confirmation: vec!["베타 시작일이 10월 15일인지 확인 필요".to_owned()],
    }
}

fn recording() -> Recording {
    Recording {
        id: RecordingId::new("rec-table"),
        title: "주간 회의 녹음".to_owned(),
        created_at: "2026-11-20T01:00:00.000Z".to_owned(),
        updated_at: "2026-11-20T02:00:00.000Z".to_owned(),
        duration_ms: 3_151_000,
        audio_path: "recordings/rec-table.wav".to_owned(),
        audio_format: "wav".to_owned(),
        microphone: None,
        current_transcript_id: Some(TranscriptId::new("tr-table")),
        transcription_status: ProcessingStatus::Done,
        ai_status: ProcessingStatus::Done,
        notion_status: ProcessingStatus::None,
        capture_warning: None,
    }
}

fn transcript() -> Transcript {
    Transcript {
        id: TranscriptId::new("tr-table"),
        recording_id: RecordingId::new("rec-table"),
        language: Some("ko".to_owned()),
        segments: vec![
            TranscriptSegment {
                start_ms: 3_000,
                end_ms: 6_000,
                text: "안녕하세요.".to_owned(),
            },
            TranscriptSegment {
                start_ms: 3_723_000,
                end_ms: 3_725_000,
                text: "다음 주에 봅시다.".to_owned(),
            },
        ],
        raw_text: "안녕하세요. 다음 주에 봅시다.".to_owned(),
        created_at: "2026-11-20T02:10:00.000Z".to_owned(),
        engine: "whisper-rs".to_owned(),
        model: "ggml-base".to_owned(),
        transcription_ms: None,
    }
}

/// 저장했다가 다시 읽은 노트 — Notion 전송은 저장된 노트를 읽어 렌더한다.
fn stored(note: MeetingNoteV2) -> StructuredNote {
    let note = StructuredNote::MeetingV2(note);
    let decoded = decode_content(&encode_content(&note), NoteType::Meeting)
        .expect("encode한 노트를 다시 읽을 수 있어야 한다");
    assert_eq!(decoded, note, "encode → decode가 값을 보존해야 한다");
    decoded
}

fn markdown_of(note: &StructuredNote, include_transcript: bool) -> String {
    let recording = recording();
    let transcript = transcript();
    render(
        &ExportDocument {
            recording: &recording,
            transcript: &transcript,
            note: Some(note),
        },
        include_transcript,
    )
}

// ── 판정 도구 ──────────────────────────────────────────────────────────────────

/// `|`로 시작하는 줄인가 — `notion::chunk`와 GFM이 표의 줄로 보는 모양이다.
fn is_pipe_line(line: &str) -> bool {
    line.trim_start().starts_with('|')
}

/// `| --- | :-: |` 같은 GFM 구분 행인가.
fn is_separator(line: &str) -> bool {
    let row = line.trim();
    row.starts_with('|')
        && row.contains('-')
        && row
            .chars()
            .all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
}

/// 연속된 `|` 줄 덩어리들 — 각 덩어리는 줄들을 `\n`으로 이은 문자열이다.
fn pipe_runs(text: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in text.lines() {
        if is_pipe_line(line) {
            current.push(line);
        } else if !current.is_empty() {
            runs.push(current.join("\n"));
            current.clear();
        }
    }
    if !current.is_empty() {
        runs.push(current.join("\n"));
    }
    runs
}

/// 덩어리 하나가 온전한 GFM 표인가 — 머리 행, 둘째 줄의 구분 행, 그리고 열 수가 같은 행들.
fn assert_whole_table(run: &str, context: &str) {
    let lines: Vec<&str> = run.lines().collect();
    assert!(
        lines.len() >= 2 && is_separator(lines[1]),
        "{context}: 머리글 · 구분 행이 없는 표 조각이 있다:\n{run}"
    );
    let cells = |line: &str| line.replace("\\|", "").matches('|').count();
    let width = cells(lines[0]);
    for line in &lines {
        assert_eq!(cells(line), width, "{context}: 열 수가 다른 행이 있다: {line}");
    }
}

/// 글머리표 줄의 내용이 표 문법처럼 보이는가 (`- | a | b | |---|`).
fn bullet_has_table_syntax(content: &str) -> bool {
    let compact: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    compact.starts_with('|') || compact.contains("|-") || compact.contains("|:-")
}

fn assert_no_table_syntax_in_bullets(text: &str, context: &str) {
    for line in text.lines() {
        if let Some(content) = line.trim_start().strip_prefix("- ") {
            assert!(
                !bullet_has_table_syntax(content),
                "{context}: 글머리표 안에 표 문법이 있다: {line}"
            );
        }
    }
}

/// `[HH:MM:SS]` 모양이 있는가.
fn has_bracketed_timestamp(text: &str) -> bool {
    text.as_bytes().windows(10).any(|w| {
        w[0] == b'['
            && w[3] == b':'
            && w[6] == b':'
            && w[9] == b']'
            && [1, 2, 4, 5, 7, 8].iter().all(|&i| w[i].is_ascii_digit())
    })
}

fn units(chunk: &str) -> usize {
    chunk.lines().filter(|line| !line.trim().is_empty()).count()
}

// ── 표가 있는 노트 ─────────────────────────────────────────────────────────────

#[test]
fn every_table_lands_whole_inside_one_chunk_on_the_notion_path() {
    let mut boundary_right_before_a_table = false;

    for include_transcript in [true, false] {
        for lead in 0..40 {
            let context = format!("include_transcript={include_transcript}, lead={lead}");
            let note = stored(note_with_tables(lead));
            let markdown = markdown_of(&note, include_transcript);

            assert_eq!(
                markdown.contains("\n## Transcript\n"),
                include_transcript,
                "{context}: Transcript 포함 선택이 반영되지 않았다"
            );

            // 문서 전체: 표는 전부 온전하고, 기대한 개수만큼 있다.
            //  - 주제 표: 짝수 주제 15개 + 3의 배수 주제 10개
            //  - 문자열 항목에서 꺼낸 표 1개 (3번 주제)
            let runs = pipe_runs(&markdown);
            let expected_tables = (0..TOPIC_COUNT).filter(|i| i % 2 == 0).count()
                + (0..TOPIC_COUNT).filter(|i| i % 3 == 0).count()
                + 1;
            assert_eq!(runs.len(), expected_tables, "{context}: 표 개수가 다르다");
            for run in &runs {
                assert_whole_table(run, &context);
            }
            assert!(
                markdown.contains("\n\n| 항목 속 표 | 의견 |\n| --- | --- |\n"),
                "{context}: 문자열 항목의 표가 표 블록으로 꺼내지지 않았다"
            );
            assert!(
                markdown.contains("A \\| B 중 하나 둘째 줄"),
                "{context}: 셀의 `|`·줄바꿈이 처리되지 않았다"
            );

            // 표는 글머리표 밖에 있고, 앞뒤가 빈 줄이다.
            let lines: Vec<&str> = markdown.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if is_pipe_line(line) {
                    let before = lines[i - 1];
                    assert!(
                        is_pipe_line(before) || before.is_empty(),
                        "{context}: 표 앞에 빈 줄이 없다: {before}"
                    );
                    if let Some(after) = lines.get(i + 1) {
                        assert!(
                            is_pipe_line(after) || after.is_empty(),
                            "{context}: 표 뒤에 빈 줄이 없다: {after}"
                        );
                    }
                }
            }

            assert_no_table_syntax_in_bullets(&markdown, &context);
            assert!(
                !has_bracketed_timestamp(&markdown),
                "{context}: [HH:MM:SS]가 결과에 있다"
            );

            // Notion 전송의 분할.
            let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");
            assert!(chunks.len() > 1, "{context}: 나뉘지 않았다 — 분할을 검사하지 못한다");
            assert_eq!(chunks.concat(), markdown, "{context}: 재조립이 원문과 다르다");

            for (index, chunk) in chunks.iter().enumerate() {
                let chunk_context = format!("{context}, chunk {index}");
                assert!(chunk.len() <= CHUNK_MAX_BYTES, "{chunk_context}: 바이트 예산 초과");
                assert!(
                    units(chunk) <= CHUNK_MAX_BLOCK_UNITS,
                    "{chunk_context}: block unit 예산 초과"
                );
                // chunk 안의 모든 `|` 덩어리가 온전한 표다 — 머리글 없는 조각이 없다.
                for run in pipe_runs(chunk) {
                    assert_whole_table(&run, &chunk_context);
                }
                assert_no_table_syntax_in_bullets(chunk, &chunk_context);
                assert!(
                    !has_bracketed_timestamp(chunk),
                    "{chunk_context}: [HH:MM:SS]가 있다"
                );
                boundary_right_before_a_table |= chunk.starts_with('|');
            }

            // 문서의 표 하나하나가 정확히 한 chunk에 통째로 있다.
            for run in &runs {
                let holders = chunks.iter().filter(|chunk| chunk.contains(run.as_str())).count();
                assert_eq!(holders, 1, "{context}: 표를 통째로 가진 chunk가 하나가 아니다:\n{run}");

                let first = run.lines().next().expect("머리 행");
                let last = run.lines().last().expect("마지막 행");
                for chunk in &chunks {
                    assert_eq!(
                        chunk.contains(first),
                        chunk.contains(last),
                        "{context}: 표가 두 chunk에 갈렸다:\n{run}"
                    );
                }
            }
        }
    }

    // chunk 경계가 실제로 표 바로 앞까지 온 경우가 있어야 이 테스트가 무언가를 검사한다.
    assert!(boundary_right_before_a_table, "표 바로 앞에서 나뉜 경우가 없다");
}

// ── 표가 없는 노트 ─────────────────────────────────────────────────────────────

fn note_without_tables() -> MeetingNoteV2 {
    MeetingNoteV2 {
        meeting_title: "주간 회의".to_owned(),
        meeting_kind: "정기 위클리".to_owned(),
        attendees: vec!["김철수".to_owned(), "이영희".to_owned()],
        reliability_notice: "자동 전사 후 정리했습니다.".to_owned(),
        key_summary: vec!["**출시 확정**: 9월 말".to_owned()],
        previous_action_items: PreviousActionItems {
            done: vec!["RFP 초안 완료".to_owned()],
            changed: vec![],
            in_progress: vec![],
            no_progress: vec![],
            not_mentioned: vec![],
        },
        topics: vec![Topic {
            title: "일정".to_owned(),
            groups: vec![
                TopicGroup {
                    heading: "확정".to_owned(),
                    items: vec!["9월 30일 출시".to_owned(), "베타는 10월".to_owned()],
                    table: None,
                },
                TopicGroup {
                    heading: String::new(),
                    items: vec!["리스크: 연휴".to_owned()],
                    table: None,
                },
            ],
        }],
        action_items: vec![ActionItemV2 {
            task: "가이드 작성".to_owned(),
            detail: String::new(),
            owner: "이영희".to_owned(),
            due: "11/18".to_owned(),
            period: "이번 주".to_owned(),
        }],
        needs_confirmation: vec!["금액 확인 필요".to_owned()],
    }
}

/// 표 필드가 생기기 전의 렌더링 — 기대 문자열 전체를 고정한다.
fn expected_without_tables(include_transcript: bool) -> String {
    let mut lines = vec![
        "# 주간 회의 녹음",
        "",
        "Date: 2026-11-20",
        "Duration: 52:31",
        "",
        "> 📌 자동 전사 후 정리했습니다.",
        "",
        "- **회의명**: 주간 회의",
        "- **유형**: 정기 위클리",
        "- **참석자**: 김철수, 이영희",
        "",
        "## 🔑 핵심 요약",
        "- **출시 확정**: 9월 말",
        "",
        "## 🔁 지난주 액션 아이템 진행 상황",
        "**완료**",
        "- RFP 초안 완료",
        "",
        "## 1. 일정",
        "**확정**",
        "- 9월 30일 출시",
        "- 베타는 10월",
        "- 리스크: 연휴",
        "",
        "## ✅ 액션 아이템",
        "### 이번 주",
        "- [ ]  **가이드 작성** — 이영희 · 📅 11/18",
        "",
        "## 🔍 확인 필요",
        "- 금액 확인 필요",
    ];
    if include_transcript {
        lines.extend([
            "",
            "## Transcript",
            "",
            "### 00:00:03",
            "안녕하세요.",
            "",
            "### 01:02:03",
            "다음 주에 봅시다.",
        ]);
    }
    let mut markdown = lines.join("\n");
    markdown.push('\n');
    markdown
}

#[test]
fn a_note_without_a_table_renders_and_splits_exactly_as_before() {
    let note = note_without_tables();

    // 표 필드가 없던 시절의 저장 형태 — JSON에 `table` 키가 없다.
    let legacy_json = serde_json::to_string(&note).expect("직렬화");
    assert!(!legacy_json.contains("\"table\""), "표가 없으면 table 키도 없어야 한다");
    let parsed = parse_note(NoteType::Meeting, &legacy_json)
        .expect("table 키가 없는 기존 v2 노트가 그대로 읽혀야 한다");
    assert_eq!(parsed, StructuredNote::MeetingV2(note.clone()));

    for include_transcript in [true, false] {
        let expected = expected_without_tables(include_transcript);

        for source in [&parsed, &stored(note.clone())] {
            let markdown = markdown_of(source, include_transcript);
            assert_eq!(markdown, expected, "include_transcript={include_transcript}");

            let chunks = split_markdown(&markdown).expect("예산 안의 문서다");
            assert_eq!(chunks, vec![expected.as_str()], "작은 문서는 한 chunk 그대로다");
            assert!(!has_bracketed_timestamp(&markdown));
            assert!(pipe_runs(&markdown).is_empty(), "표가 없는 노트에 `|` 줄이 생겼다");
        }
    }
}

#[test]
fn the_timestamp_check_itself_recognizes_the_pattern() {
    // 판정 도구가 실제로 `[HH:MM:SS]`를 잡는지 확인한다 — 못 잡으면 위 검사들이 공허하다.
    assert!(has_bracketed_timestamp("인용 [00:03:23]-[00:04:23] 구간"));
    assert!(!has_bracketed_timestamp("### 00:00:03\n안녕하세요."));
    assert!(bullet_has_table_syntax("| 후보 | 의견 | |---|---|"));
    assert!(!bullet_has_table_syntax("A | B 중 하나"));
}
