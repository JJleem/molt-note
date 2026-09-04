//! **긴 문서 하나가 순서대로 · 무손실로 나뉜다** (`docs/ADR-0009-notion-and-export.md` §6 ·
//! `phase-prompt/05` P-1).
//!
//! 판정 기준은 상수의 크기가 아니다 (§6.4). 이 파일이 답하는 질문은 넷이고, 전부 같은 성질을
//! 다른 입력으로 묻는다.
//!
//! ```text
//! 1. 예산보다 작은 문서는 나뉘지 않는다                                   (a)
//! 2. 예산을 넘는 문서는 여러 chunk가 되고, 이어 붙이면 원문이다           (b)
//! 3. ★ 1시간 분량 transcript 규모도 그렇다 ★                             (c)
//! 4. 한 chunk에 담을 수 없는 단위는 잘리지 않고 보이는 실패가 된다        (d)
//! ```
//!
//! 재조립 동등성은 **원문 전체와 바이트 단위로** 비교한다 — 앞뒤 몇 글자나 길이만 보면
//! 가운데가 사라진 것을 놓친다.
//!
//! **여기서 쓰는 문서는 §11 렌더러가 실제로 만든 것이다.** 손으로 지은 문자열이 아니라
//! [`export::markdown::render`]의 산출물을 그대로 나누므로, 이 검증은 제품이 실제로 보내는
//! 그 문자열에 대한 검증이다 (ADR-0009 §14 — 두 문이 같은 산출물을 쓴다).
//!
//! 네트워크도 저장소도 파일도 Tauri 런타임도 필요하지 않다. 통합 테스트이므로 crate의 공개
//! API만 쓴다.

use std::path::Path;

use molt_note_lib::domain::{
    ProcessingStatus, Recording, RecordingId, Transcript, TranscriptId, TranscriptSegment,
};
use molt_note_lib::export::{self, markdown::ExportDocument};
use molt_note_lib::notion::chunk::{
    split_markdown, AtomKind, CHUNK_MAX_BLOCK_UNITS, CHUNK_MAX_BYTES,
};

const CREATED_AT: &str = "2026-09-01T10:00:00.000Z";

/// 1시간 = 3,600,000ms. (c)의 입력이 실제로 이 길이의 녹음이다.
const ONE_HOUR_MS: i64 = 3_600_000;

/// 1시간 녹음의 segment 하나가 덮는 시간. 3초는 사람이 한 문장을 말하는 정도다.
const SEGMENT_MS: i64 = 3_000;

// --- 입력 ----------------------------------------------------------------------------

fn recording(duration_ms: i64) -> Recording {
    Recording {
        id: RecordingId::new("rec-chunking"),
        title: "3DGS 스터디 04".to_string(),
        created_at: CREATED_AT.to_string(),
        updated_at: CREATED_AT.to_string(),
        duration_ms,
        audio_path: "recordings/rec-chunking.wav".to_string(),
        audio_format: "wav".to_string(),
        microphone: Some("가짜 마이크".to_string()),
        current_transcript_id: None,
        transcription_status: ProcessingStatus::Done,
        ai_status: ProcessingStatus::None,
        notion_status: ProcessingStatus::None,
    }
}

fn transcript(segments: Vec<TranscriptSegment>) -> Transcript {
    Transcript {
        id: TranscriptId::new("tr-chunking"),
        recording_id: RecordingId::new("rec-chunking"),
        language: Some("ko".to_string()),
        raw_text: segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        segments,
        created_at: CREATED_AT.to_string(),
        engine: "stub".to_string(),
        model: "ggml-base.bin".to_string(),
    }
}

fn segments(count: i64) -> Vec<TranscriptSegment> {
    (0..count)
        .map(|index| TranscriptSegment {
            start_ms: index * SEGMENT_MS,
            end_ms: (index + 1) * SEGMENT_MS,
            text: format!(
                "{index}번 구간입니다. 발표자는 3D Gaussian Splatting의 학습 파이프라인과 \
                 렌더링 품질 지표를 설명했고, 다음 주까지 실험 결과를 정리해 공유하기로 했습니다.",
            ),
        })
        .collect()
}

/// §11 렌더러가 만든 실제 문서. AI Note는 없다 — Transcript만으로도 유효한 문서다 (INV-8).
fn rendered(duration_ms: i64, segment_count: i64) -> String {
    let recording = recording(duration_ms);
    let transcript = transcript(segments(segment_count));

    export::markdown::render(&ExportDocument {
        recording: &recording,
        transcript: &transcript,
        note: None,
    })
}

/// chunk 하나의 block unit 수 — ADR-0009 §6.2가 정한 대용값과 같은 방식으로 센다.
///
/// **모듈 안의 계산을 다시 부르지 않고 여기서 따로 센다.** 같은 함수로 재면 그 함수가 틀렸을 때
/// 검사도 함께 틀린다.
fn block_units(chunk: &str) -> usize {
    let mut units = 0;
    let mut inside_fence = false;

    for line in chunk.lines() {
        let fence = line.trim_start().starts_with("```");

        if fence {
            if !inside_fence {
                units += 1; // 펜스 블록은 통째로 1이다
            }
            inside_fence = !inside_fence;
            continue;
        }
        if !inside_fence && !line.trim().is_empty() {
            units += 1;
        }
    }

    units
}

fn assert_within_budgets(chunks: &[&str]) {
    for chunk in chunks {
        assert!(!chunk.is_empty(), "빈 chunk를 보내지 않는다");
        assert!(
            chunk.len() <= CHUNK_MAX_BYTES,
            "chunk가 바이트 예산을 넘는다: {} > {CHUNK_MAX_BYTES}",
            chunk.len()
        );
        assert!(
            block_units(chunk) <= CHUNK_MAX_BLOCK_UNITS,
            "chunk가 block unit 예산을 넘는다: {} > {CHUNK_MAX_BLOCK_UNITS}",
            block_units(chunk)
        );
    }
}

// --- (a) 예산보다 작은 문서 ------------------------------------------------------------

#[test]
fn a_document_under_the_budget_is_sent_as_one_piece() {
    let markdown = rendered(3_151_000, 20);

    assert!(
        markdown.len() < CHUNK_MAX_BYTES,
        "사전 조건: 예산 안의 문서여야 한다"
    );

    let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");

    assert_eq!(chunks.len(), 1, "나뉠 이유가 없는 문서가 나뉘었다");
    assert_eq!(chunks[0], markdown);
    assert_eq!(chunks.concat(), markdown);
}

// --- (b) 여러 chunk로 나뉘는 문서 -------------------------------------------------------

#[test]
fn a_document_over_the_budget_becomes_several_chunks_that_rejoin_into_the_original() {
    let markdown = rendered(600_000, 400);
    let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");

    assert!(chunks.len() > 1, "나뉘어야 할 문서가 하나로 남았다");
    assert_eq!(chunks.concat(), markdown, "재조립이 원문과 다르다");
    assert_within_budgets(&chunks);

    // 순서가 뒤집히면 재조립은 깨지지만, 순서 자체도 따로 고정한다 — 첫 chunk가 문서의
    // 시작이고 마지막 chunk가 문서의 끝이다.
    assert!(chunks[0].starts_with("# 3DGS 스터디 04\n"));
    assert!(chunks[chunks.len() - 1].ends_with("공유하기로 했습니다.\n"));
}

// --- (c) 1시간 분량 transcript 규모 -----------------------------------------------------

#[test]
fn an_hour_long_transcript_rejoins_byte_for_byte() {
    let segment_count = ONE_HOUR_MS / SEGMENT_MS; // 1,200 segment
    let markdown = rendered(ONE_HOUR_MS, segment_count);

    // 이 입력이 실제로 여러 요청이 되는 규모인지 먼저 고정한다. 한 chunk에 들어가는 문서로는
    // 이 테스트가 아무것도 검사하지 못한다.
    assert!(
        markdown.len() > 3 * CHUNK_MAX_BYTES,
        "1시간 규모라기에 너무 작다: {} 바이트",
        markdown.len()
    );

    let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");

    assert!(chunks.len() > 4, "chunk가 {}개뿐이다", chunks.len());
    assert_eq!(chunks.concat(), markdown, "재조립이 원문과 다르다");
    assert_within_budgets(&chunks);

    // 어떤 chunk도 줄 가운데서 끝나지 않는다 (§6.3 — 나누는 자리는 언제나 줄 경계 위다).
    for chunk in &chunks[..chunks.len() - 1] {
        assert!(chunk.ends_with('\n'), "chunk가 줄 가운데서 끝났다");
    }

    // 전사의 어느 한 줄도 사라지지 않았다. 재조립 동등성이 이미 그것을 말하지만, 이 문서에서
    // 무엇이 유실될 수 있는지를 이름으로 적어 둔다.
    let sent: usize = chunks
        .iter()
        .map(|chunk| chunk.matches("번 구간입니다.").count())
        .sum();
    assert_eq!(sent as i64, segment_count, "segment가 유실됐다");
}

// --- (d) 원자 단위가 예산을 넘는 경우 ---------------------------------------------------

#[test]
fn a_line_that_cannot_fit_is_refused_instead_of_being_cut_silently() {
    // 예산보다 긴 한 줄. 이 앱의 렌더러는 이런 줄을 만들지 않지만 (ADR-0008 §7.4가 노트
    // 항목을 2,000자로 제한하고 transcript segment는 초 단위 발화다), **만났을 때 무엇을
    // 하는지가 정해져 있어야 한다.**
    let long_line = "가".repeat(CHUNK_MAX_BYTES);
    let markdown = format!("# 제목\n\n{long_line}\n");

    let error = split_markdown(&markdown).expect_err("자르지 않고 멈춘다");

    assert_eq!(error.kind, AtomKind::Line);
    assert_eq!(error.line, 3, "몇 번째 줄인지 말한다");
    assert_eq!(error.budget, CHUNK_MAX_BYTES);
    assert!(error.bytes > CHUNK_MAX_BYTES);

    // 부분 결과가 성공으로 새어 나가지 않는다 — 실패는 실패다.
    assert!(split_markdown(&markdown).is_err());
}

#[test]
fn nothing_is_truncated_when_an_atom_does_not_fit() {
    let long_line = "나".repeat(CHUNK_MAX_BYTES);
    let markdown = format!("# 제목\n\n앞 문단\n\n{long_line}\n\n뒤 문단\n");

    // 앞부분만 담긴 chunk 목록을 돌려주는 경로가 없다는 것이 요점이다. `Result`가 그것을
    // 타입으로 말하지만, 그 사실을 여기서 이름으로 고정한다.
    match split_markdown(&markdown) {
        Ok(chunks) => panic!("자를 수 없는 문서를 {}개 chunk로 돌려줬다", chunks.len()),
        Err(error) => assert_eq!(error.line, 5),
    }
}

#[test]
fn a_code_block_that_cannot_fit_is_refused_too() {
    // 펜스 안에서는 나누지 않기로 했으므로 (§6.3-3), 펜스 블록 하나도 원자 단위다.
    let mut fence = String::from("```text\n");
    while fence.len() <= CHUNK_MAX_BYTES {
        fence.push_str("이 줄은 코드 블록 안에 있다\n");
    }
    fence.push_str("```\n");

    let markdown = format!("# 제목\n\n{fence}");
    let error = split_markdown(&markdown).expect_err("펜스는 나누지 않는다");

    assert_eq!(error.kind, AtomKind::CodeBlock);
    assert_eq!(error.line, 3, "여는 펜스의 줄 번호를 말한다");
}

#[test]
fn the_failure_does_not_carry_the_document_into_its_message() {
    let secret = "이 문장은 문서 안에만 있어야 한다";
    let markdown = format!("# 제목\n\n{secret}{}\n", "가".repeat(CHUNK_MAX_BYTES));

    let error = split_markdown(&markdown).expect_err("자르지 않고 멈춘다");
    let shown = error.to_string();

    assert!(!shown.contains(secret), "실패 문장에 문서 내용이 있다: {shown}");
}

// --- 재조립 동등성은 문서 모양과 무관하다 (ADR-0009 §6.3) --------------------------------

/// 결정론적 의사난수 (xorshift64*). **외부 crate를 들이지 않는다** — 같은 seed는 언제나 같은
/// 문서를 만들고, 그래서 이 테스트가 어느 날은 통과하고 어느 날은 실패하는 일이 없다.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

/// 임의의 모양과 길이를 갖는 markdown 문서 하나.
///
/// 렌더러가 실제로 만드는 모양(문단 · 제목 · 코드 펜스 · 빈 줄)에 더해, 경계에서 틀리기 쉬운
/// 것들을 일부러 섞는다 — 빈 줄이 연달아 오는 자리 · 펜스 안의 빈 줄 · 여러 바이트 문자 ·
/// **마지막 개행이 없는 문서**.
fn arbitrary_document(rng: &mut Rng) -> String {
    let mut markdown = String::new();

    for _ in 0..1 + rng.below(200) {
        match rng.below(10) {
            0 => markdown.push('\n'),
            1 => markdown.push_str("## 섹션 제목\n"),
            2 => {
                markdown.push_str("```text\n");
                for index in 0..rng.below(20) {
                    // 펜스 안의 빈 줄은 문단 경계처럼 보이지만 경계가 아니다 (§6.3-3).
                    markdown.push_str(&format!("{index} 펜스 안의 줄\n\n"));
                }
                markdown.push_str("```\n");
            }
            _ => {
                // '가'는 3바이트다 — 바이트 예산과 문자 수가 다르다는 사실을 입력에 넣는다.
                for _ in 0..1 + rng.below(800) {
                    markdown.push('가');
                }
                markdown.push('\n');
            }
        }
    }

    if rng.below(4) == 0 {
        while markdown.ends_with('\n') {
            markdown.pop();
        }
    }

    markdown
}

#[test]
fn any_document_rejoins_into_exactly_what_went_in() {
    let mut rng = Rng(0x5eed_2026_0904);

    for case in 0..64 {
        let markdown = arbitrary_document(&mut rng);

        let chunks = split_markdown(&markdown)
            .unwrap_or_else(|error| panic!("{case}번째 문서에서 나누지 못했다: {error}"));

        // ★ 이 한 줄이 이 파일 전체의 성질이다 — 원문 전체와 바이트 단위로 같다.
        assert_eq!(chunks.concat(), markdown, "{case}번째 문서의 재조립이 다르다");
        assert_within_budgets(&chunks);

        for chunk in &chunks[..chunks.len().saturating_sub(1)] {
            assert!(
                chunk.ends_with('\n'),
                "{case}번째 문서에서 chunk가 줄 가운데서 끝났다"
            );
        }
    }
}

#[test]
fn an_atom_over_the_budget_is_the_only_reason_to_refuse() {
    // 위 생성기에 예산을 넘는 줄 하나를 섞는다. 실패는 **그 줄 때문에만** 일어나고,
    // 실패했을 때 chunk 목록이 대신 나오는 경로는 없다.
    let mut rng = Rng(0xfeed_2026_0904);

    for case in 0..16 {
        let mut markdown = arbitrary_document(&mut rng);
        let before = markdown.lines().count();
        markdown.push_str(&format!("\n\n{}\n", "나".repeat(CHUNK_MAX_BYTES)));

        let error = match split_markdown(&markdown) {
            Ok(chunks) => panic!("{case}번째 문서를 {}개 chunk로 돌려줬다", chunks.len()),
            Err(error) => error,
        };

        assert!(
            error.bytes > CHUNK_MAX_BYTES,
            "{case}번째 문서: 예산을 넘지 않는 단위를 거절했다"
        );
        assert_eq!(error.budget, CHUNK_MAX_BYTES);
        assert_eq!(error.kind, AtomKind::Line);
        assert!(
            error.line > before,
            "{case}번째 문서: 긴 줄이 아닌 자리를 가리킨다"
        );
    }
}

// --- 이 모듈이 무엇을 알지 않는가 --------------------------------------------------------

#[test]
fn the_chunking_module_knows_nothing_about_the_network_storage_or_files() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/notion/chunk.rs"),
    )
    .expect("소스 파일을 읽는다");

    // 네트워크 · 파일 시스템 · 저장소 · 시계를 부르는 자리가 하나도 없다 (Task 요구).
    for outside in [
        "ureq", "std::net", "TcpStream", "std::fs", "File::", "rusqlite", "Connection",
        "SystemTime", "Instant",
    ] {
        assert!(
            !source.contains(outside),
            "순수해야 할 모듈이 바깥 세계를 안다: {outside}"
        );
    }
}
