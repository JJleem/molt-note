//! 긴 markdown 문서 하나를 요청 하나에 담을 수 있는 크기로 나누는 **순수 모듈**
//! (`docs/ADR-0009-notion-and-export.md` §6 · `phase-prompt/05` P-1).
//!
//! ```text
//! markdown 문자열 ─→ split_markdown ─→ [chunk 1, chunk 2, …]   (원본의 연속된 조각들)
//!                                   └─→ Err(OversizedAtom)      (나눌 수 없는 단위를 만났다)
//! ```
//!
//! ## 판정 기준은 상수의 크기가 아니다 (ADR-0009 §6.4)
//!
//! ```text
//! 전체 문서가 · 순서대로 · 무손실로 나뉜다.
//! ```
//!
//! 그래서 이 모듈이 지키는 성질은 **재조립 동등성** 하나로 적힌다.
//!
//! ```text
//! chunks.concat() == 원본 markdown          (바이트 단위로 같다)
//! ```
//!
//! 나누기는 **문자열을 자르는 것이지 바꾸는 것이 아니다.** 돌려주는 chunk는 전부 입력의
//! 부분 슬라이스이며, 경계에서 개행을 먹지도 더하지도 않는다 — 그래서 위 성질이 테스트로
//! 확인되는 것을 넘어 **타입으로 거의 강제된다.**
//!
//! ## 어떤 경우에도 조용히 자르지 않는다
//!
//! 한 chunk에 담을 수 없는 **원자 단위**(예산보다 긴 한 줄 · 예산보다 긴 코드 펜스 블록)를
//! 만나면 그 자리에서 [`OversizedAtom`]으로 **멈춘다.** 줄 가운데를 자르면 강조나 링크가
//! 갈려 보낸 문서가 원본과 다른 문서가 되고, 그것은 무손실이 아니다 (§6.3-4). 부분 결과를
//! 성공처럼 돌려주는 경로는 여기 없다.
//!
//! ## 이 모듈이 알지 않는 것
//!
//! 네트워크 · 저장소 · 파일 시스템 · 시계를 알지 않는다. 주소도 헤더도 요청 형태도 재시도
//! 횟수도 없다 — 여기 있는 것은 문자열에서 문자열 조각을 만드는 함수뿐이고, 그것을 실제로
//! 보내는 순서는 이 모듈 밖에 있다 (ADR-0009 §8 · §9). 같은 입력은 언제나 같은 결과를 낸다.

/// markdown 한 chunk의 UTF-8 바이트 상한.
///
/// ⚠️ **이 값은 이 앱이 고른 값이다 (ADR-0009 §3의 [A]). 확인된 Notion API 한도가 아니다.**
/// 이 숫자를 "API가 이렇게 정했다"로 읽으면 안 된다 — 줄이는 것도 늘리는 것도 이 앱의
/// 판단이며, 상수 한 줄을 고치는 일이다.
///
/// **무엇에서 나왔는가**
///
/// ```text
/// VERIFIED   요청당 500KB overall (일반 요청 한도)
/// 60,000 × 6 = 360,000 B  <  500,000 B      본문이 전부 제어문자여서 JSON 이스케이프가
///                                           문자당 6배로 팽창해도 그 아래에 있다
/// ```
///
/// 남는 자리는 URL · 헤더 · JSON 봉투 · **서버가 우리와 다르게 셀 가능성**에 둔다.
///
/// **무엇에서 나오지 않았는가**
///
/// - markdown 엔드포인트 **전용** 본문 상한은 UNVERIFIED다 — 그런 값이 따로 있는지조차
///   확인되지 않았다 (ADR-0009 §6.1). 웹에서 보이는 750KB 같은 값은 primary source에서
///   확인된 적이 없으므로 이 상수의 근거가 아니고, 이 코드는 그것을 API 사실로 적지 않는다.
/// - 옛 2000자 rich text 규칙과 100블록 배치 규칙에서 유도하지 **않았다.** 그것은 블록 JSON
///   경로(`children`/`content`)의 규칙이고, 이 앱은 그 경로를 쓰지 않는다
///   (`phase-prompt/05` P-1 · ADR-0009 §5.4).
pub const CHUNK_MAX_BYTES: usize = 60_000;

/// markdown 한 chunk의 block unit 상한.
///
/// ⚠️ **이 값도 이 앱이 고른 값이다 ([A]). 확인된 API 한도가 아니다.**
///
/// 앱은 markdown 한 문서가 Notion에서 정확히 몇 block element가 되는지 **알 수 없다** —
/// 표 한 줄이나 중첩 리스트가 몇 개로 펼쳐지는지는 확인되지 않았다. 그래서 세는 방법을
/// 보수적인 대용값으로 정한다 (ADR-0009 §6.2).
///
/// ```text
/// block unit = chunk 안의 "비어 있지 않은 줄" 수. 코드 펜스 블록은 통째로 1로 센다.
/// 300 × 3.3 ≈ 1000                          VERIFIED 요청당 1000 block elements
/// ```
///
/// 한 줄이 여러 block element로 펼쳐져도 3.3배까지는 버틴다. 그 배수가 충분한지는 확인된
/// 사실이 아니며, 그래서 **판정 기준을 이 상수에 두지 않는다** (§6.4).
pub const CHUNK_MAX_BLOCK_UNITS: usize = 300;

/// 나눌 수 없는 단위가 무엇이었는가.
///
/// 둘을 구분하는 이유는 사용자가 할 수 있는 일이 다르기 때문이다 — 한 줄이 너무 긴 것과
/// 코드 블록 하나가 통째로 너무 큰 것은 문서에서 찾아가는 자리가 다르다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomKind {
    /// 줄 하나. 줄 가운데는 자르지 않는다 (ADR-0009 §6.3-4).
    Line,
    /// 코드 펜스 블록 하나. 펜스 안에서는 절대 나누지 않는다 (§6.3-3).
    CodeBlock,
}

/// 한 chunk에 담을 수 없는 단위를 만났다 — **아무것도 자르지 않고 멈췄다는 뜻이다.**
///
/// 이 값은 `Failure`가 아니다. §13의 제품 상태로 옮기는 것은 **전송 순서를 소유한 쪽**의
/// 일이며 ([`crate::ai::prompt::ContextOverflow`]가 [`crate::ai::run`]에서 옮겨지는 것과
/// 같은 규약이다), 그래서 이 모듈은 만들지 않은 실패의 자리를 미리 만들지 않는다.
///
/// **문서 내용을 담지 않는다.** 어디인지(줄 번호)와 얼마나 큰지(바이트)만 있으면 사용자가
/// 그 자리를 찾을 수 있고, 내용 조각이 실패 문장·로그로 새어 나갈 통로가 아예 없다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OversizedAtom {
    /// 그 단위가 시작하는 줄 번호 (1부터 센다).
    pub line: usize,
    /// 그 단위의 UTF-8 바이트 수.
    pub bytes: usize,
    /// 넘어선 예산 — [`CHUNK_MAX_BYTES`]다.
    pub budget: usize,
    /// 줄이었는가, 코드 블록이었는가.
    pub kind: AtomKind,
}

impl std::fmt::Display for OversizedAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let what = match self.kind {
            AtomKind::Line => "줄",
            AtomKind::CodeBlock => "코드 블록",
        };

        write!(
            f,
            "{}번째 줄에서 시작하는 {}이 {}바이트로 한 chunk 예산 {}바이트를 넘는다 — 자르지 않았다",
            self.line, what, self.bytes, self.budget
        )
    }
}

/// markdown 문서 하나를 예산 아래의 chunk들로 **순서대로 · 무손실로** 나눈다.
///
/// 돌려주는 조각들은 입력의 연속된 부분 슬라이스이며, 이어 붙이면 입력과 정확히 같다.
///
/// ```
/// use molt_note_lib::notion::chunk::split_markdown;
///
/// let markdown = "# 제목\n\nDate: 2026-09-01\n";
/// let chunks = split_markdown(markdown).expect("예산 안의 문서다");
///
/// assert_eq!(chunks.concat(), markdown);
/// ```
///
/// **나누는 자리는 언제나 줄 경계 위에 있다** (ADR-0009 §6.3).
///
/// ```text
/// 1. 빈 줄(문단 경계)에서 나눈다.                        ← 기본
/// 2. 한 문단이 혼자 예산을 넘으면 그 안의 줄 경계에서 나눈다.
/// 3. 코드 펜스 블록 안에서는 절대 나누지 않는다.
/// 4. 한 줄(또는 코드 블록 하나)이 혼자 예산을 넘으면 → Err. 자르지 않는다.
/// ```
///
/// # Errors
///
/// 나눌 수 없는 단위를 만나면 [`OversizedAtom`]이다. **부분 결과를 돌려주지 않는다** —
/// 일부만 보내고 나머지를 버리는 것은 조용한 유실이며, 그것을 성공으로 부르지 않는다.
///
/// 빈 문서는 chunk가 하나도 없다(`Ok(vec![])`). 보낼 것이 없다는 사실을 chunk 0개로
/// 말하며, 빈 요청을 만들어 내지 않는다.
pub fn split_markdown(markdown: &str) -> Result<Vec<&str>, OversizedAtom> {
    let atoms = atoms(markdown);
    let mut packer = Packer::new(markdown);

    for (first, last) in paragraph_blocks(&atoms) {
        let block = &atoms[first..last];
        let units = block.iter().map(|atom| atom.block_units).sum();

        // 문단 하나가 통째로 들어가면 그것이 §6.3-1의 기본 경계다.
        if packer.push(block[0].start, block[block.len() - 1].end, units) {
            continue;
        }

        // 들어가지 않으면 그 문단 안의 줄 경계로 내려간다 (§6.3-2). 코드 펜스 블록은 여기서도
        // 단위 하나이므로 갈리지 않는다 (§6.3-3).
        for atom in block {
            if !packer.push(atom.start, atom.end, atom.block_units) {
                return Err(OversizedAtom {
                    line: atom.line,
                    bytes: atom.end - atom.start,
                    budget: CHUNK_MAX_BYTES,
                    kind: atom.kind,
                });
            }
        }
    }

    Ok(packer.finish())
}

/// 이 모듈이 더 이상 나누지 않는 단위 — 줄 하나, 또는 코드 펜스 블록 하나.
#[derive(Debug, Clone, Copy)]
struct Atom {
    start: usize,
    end: usize,
    /// 1부터 세는 시작 줄 번호. 실패를 사람이 찾아갈 수 있게 하는 값이다.
    line: usize,
    /// 빈 줄만으로 이루어졌는가 — 문단 경계를 찾는 데 쓴다.
    blank: bool,
    block_units: usize,
    kind: AtomKind,
}

/// 예산 안에서 단위들을 순서대로 담는 자리.
///
/// 담는 것은 언제나 **바로 앞에서 끝난 자리부터 이어지는 범위**이며, 그래서 만들어진
/// chunk들은 원본을 빈틈없이 덮는다.
struct Packer<'a> {
    markdown: &'a str,
    chunks: Vec<&'a str>,
    open: Option<OpenChunk>,
}

#[derive(Debug, Clone, Copy)]
struct OpenChunk {
    start: usize,
    end: usize,
    units: usize,
}

impl<'a> Packer<'a> {
    fn new(markdown: &'a str) -> Self {
        Self {
            markdown,
            chunks: Vec::new(),
            open: None,
        }
    }

    /// 단위 하나를 담는다. 지금 chunk에 들어가지 않으면 그것을 닫고 새 chunk에서 다시 본다.
    ///
    /// 혼자서도 예산을 넘어 **어떤 chunk에도 담기지 않으면** `false`다. 그때 이 함수는
    /// 아무것도 자르지 않았고, 부르는 쪽이 그 사실을 실패로 만든다.
    fn push(&mut self, start: usize, end: usize, units: usize) -> bool {
        let bytes = end - start;

        if let Some(open) = self.open.as_mut() {
            debug_assert_eq!(open.end, start, "chunk가 원본의 연속된 범위가 아니다");

            if open.end - open.start + bytes <= CHUNK_MAX_BYTES
                && open.units + units <= CHUNK_MAX_BLOCK_UNITS
            {
                open.end = end;
                open.units += units;
                return true;
            }

            self.close();
        }

        if bytes > CHUNK_MAX_BYTES || units > CHUNK_MAX_BLOCK_UNITS {
            return false;
        }

        self.open = Some(OpenChunk { start, end, units });
        true
    }

    fn close(&mut self) {
        if let Some(open) = self.open.take() {
            self.chunks.push(&self.markdown[open.start..open.end]);
        }
    }

    fn finish(mut self) -> Vec<&'a str> {
        self.close();
        self.chunks
    }
}

/// 문서를 단위들로 나눈다. 이어 붙이면 입력과 같다.
fn atoms(markdown: &str) -> Vec<Atom> {
    let lines = line_ranges(markdown);
    let mut atoms = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let (start, end) = lines[index];
        let line = &markdown[start..end];
        let number = index + 1;

        if is_fence(line) {
            // 여는 펜스부터 닫는 펜스까지가 단위 하나다. 닫히지 않은 채 문서가 끝나면
            // 거기까지가 단위다 — 펜스 안이라고 추측한 자리를 나누느니 통째로 든다.
            let mut cursor = index + 1;
            while cursor < lines.len() {
                let inside = &markdown[lines[cursor].0..lines[cursor].1];
                cursor += 1;
                if is_fence(inside) {
                    break;
                }
            }

            atoms.push(Atom {
                start,
                end: lines[cursor - 1].1,
                line: number,
                blank: false,
                block_units: 1,
                kind: AtomKind::CodeBlock,
            });
            index = cursor;
            continue;
        }

        let blank = line.trim().is_empty();
        atoms.push(Atom {
            start,
            end,
            line: number,
            blank,
            block_units: usize::from(!blank),
            kind: AtomKind::Line,
        });
        index += 1;
    }

    atoms
}

/// 단위들을 문단으로 묶는다 — 결과는 `atoms`의 `[first, last)` 구간들이다.
///
/// 문단은 **빈 줄 다음에 내용이 다시 시작하는 자리**에서 끝난다. 그래서 빈 줄은 앞 문단에
/// 붙고, 새 chunk가 빈 줄로 시작하지 않는다.
fn paragraph_blocks(atoms: &[Atom]) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let mut first = 0;

    for (index, atom) in atoms.iter().enumerate() {
        let last = index + 1 == atoms.len();
        let boundary = atom.blank && !last && !atoms[index + 1].blank;

        if last || boundary {
            blocks.push((first, index + 1));
            first = index + 1;
        }
    }

    blocks
}

/// 각 줄의 범위. 줄에는 **자신의 개행이 포함된다** — 그래야 이어 붙였을 때 원본이 된다.
fn line_ranges(markdown: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;

    for (index, byte) in markdown.bytes().enumerate() {
        if byte == b'\n' {
            ranges.push((start, index + 1));
            start = index + 1;
        }
    }

    if start < markdown.len() {
        ranges.push((start, markdown.len()));
    }

    ranges
}

/// 코드 펜스 줄인가 — 백틱 세 개로 시작하는 줄이다 (ADR-0009 §6.3-3).
///
/// 여는 펜스와 닫는 펜스를 구분하지 않는다. 구분하려면 정보 문자열 규칙을 흉내 내야 하고,
/// 틀리면 **펜스 안에서 나누는** 쪽으로 틀린다. 여기서는 보수적인 쪽으로 틀린다 — 잘못 묶은
/// 단위는 chunk가 하나 커질 뿐이지만, 잘못 나눈 펜스는 문서를 바꾼다.
fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 문단 `blocks`를 §11 렌더러와 같은 방식으로 잇는다 — 빈 줄 하나, 끝에 개행 하나.
    fn document(blocks: &[String]) -> String {
        let mut markdown = blocks.join("\n\n");
        markdown.push('\n');
        markdown
    }

    /// 1시간 분량 transcript 규모의 문서 — 3초에 한 segment로 1,200개다.
    fn hour_long_transcript() -> String {
        let mut blocks = vec![
            "# 3DGS 스터디 04".to_owned(),
            "Date: 2026-09-01\nDuration: 59:57".to_owned(),
            "## Transcript".to_owned(),
        ];

        for index in 0..1_200 {
            let seconds = index * 3;
            blocks.push(format!(
                "### {:02}:{:02}:{:02}\n발표자는 {index}번째 구간에서 3D Gaussian Splatting의 \
                 학습 파이프라인과 렌더링 품질 지표를 설명했고, 다음 주까지 실험 결과를 정리해 \
                 공유하기로 했다.",
                seconds / 3_600,
                (seconds / 60) % 60,
                seconds % 60
            ));
        }

        document(&blocks)
    }

    fn block_units(chunk: &str) -> usize {
        atoms(chunk).iter().map(|atom| atom.block_units).sum()
    }

    /// 모든 chunk가 두 예산 안에 있고 비어 있지 않다.
    fn assert_within_budgets(chunks: &[&str]) {
        for chunk in chunks {
            assert!(!chunk.is_empty(), "빈 chunk가 있다");
            assert!(
                chunk.len() <= CHUNK_MAX_BYTES,
                "{} 바이트짜리 chunk가 있다",
                chunk.len()
            );
            assert!(
                block_units(chunk) <= CHUNK_MAX_BLOCK_UNITS,
                "block unit {}짜리 chunk가 있다",
                block_units(chunk)
            );
        }
    }

    #[test]
    fn an_empty_document_becomes_no_chunk_at_all() {
        let chunks = split_markdown("").expect("빈 문서는 실패가 아니다");

        assert!(chunks.is_empty());
        assert_eq!(chunks.concat(), "");
    }

    #[test]
    fn a_document_under_the_budget_is_not_split() {
        let markdown = document(&[
            "# 3DGS 스터디 04".to_owned(),
            "Date: 2026-09-01\nDuration: 52:31".to_owned(),
            "## Transcript".to_owned(),
            "### 00:00:03\n오늘은 splatting의 학습 파이프라인을 봅니다.".to_owned(),
        ]);

        let chunks = split_markdown(&markdown).expect("예산 안의 문서다");

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], markdown);
        assert_eq!(chunks.concat(), markdown);
    }

    #[test]
    fn a_document_over_the_budget_is_split_and_rejoins_into_the_original() {
        let blocks: Vec<String> = (0..1_000)
            .map(|index| format!("### 구간 {index}\n이 문단은 예산을 넘기기 위한 내용이다."))
            .collect();
        let markdown = document(&blocks);

        let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");

        assert!(chunks.len() > 1, "나뉘지 않았다");
        assert_eq!(chunks.concat(), markdown);
        assert_within_budgets(&chunks);
    }

    #[test]
    fn an_hour_long_transcript_rejoins_byte_for_byte() {
        let markdown = hour_long_transcript();

        // 이 입력이 실제로 여러 요청이 되는 규모인지 먼저 고정한다 — 한 chunk에 들어가는
        // 문서로는 이 테스트가 아무것도 검사하지 못한다.
        assert!(
            markdown.len() > 3 * CHUNK_MAX_BYTES,
            "1시간 규모라기에 너무 작다: {} 바이트",
            markdown.len()
        );

        let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");

        assert!(chunks.len() > 4, "chunk가 {}개뿐이다", chunks.len());
        assert_eq!(chunks.concat(), markdown, "재조립이 원문과 다르다");
        assert_within_budgets(&chunks);
    }

    #[test]
    fn a_line_longer_than_the_budget_is_refused_instead_of_cut() {
        let long_line = "가".repeat(CHUNK_MAX_BYTES); // 한 글자가 3바이트다
        let markdown = document(&["# 제목".to_owned(), long_line]);

        let error = split_markdown(&markdown).expect_err("자르지 않고 멈춘다");

        assert_eq!(error.kind, AtomKind::Line);
        assert_eq!(error.line, 3, "긴 줄은 세 번째 줄이다");
        assert_eq!(error.budget, CHUNK_MAX_BYTES);
        assert!(error.bytes > CHUNK_MAX_BYTES);
    }

    #[test]
    fn a_code_block_longer_than_the_budget_is_refused_instead_of_cut() {
        let mut fence = String::from("```text\n");
        for index in 0..20_000 {
            fence.push_str(&format!("{index} 이 줄은 코드 블록 안에 있다\n"));
        }
        fence.push_str("```");

        let markdown = document(&["# 제목".to_owned(), fence]);
        let error = split_markdown(&markdown).expect_err("펜스는 나누지 않는다");

        assert_eq!(error.kind, AtomKind::CodeBlock);
        assert_eq!(error.line, 3, "여는 펜스는 세 번째 줄이다");
        assert!(error.bytes > CHUNK_MAX_BYTES);
    }

    #[test]
    fn the_failure_says_where_and_how_big_without_quoting_the_document() {
        let secret = "이 문장은 문서 안에만 있어야 한다";
        let long_line = format!("{secret}{}", "가".repeat(CHUNK_MAX_BYTES));
        let markdown = document(&["# 제목".to_owned(), long_line]);

        let error = split_markdown(&markdown).expect_err("자르지 않고 멈춘다");
        let shown = error.to_string();

        assert!(!shown.contains(secret), "실패 문장에 문서 내용이 있다: {shown}");
        assert!(shown.contains("3번째 줄"), "{shown}");
        assert!(shown.contains(&CHUNK_MAX_BYTES.to_string()), "{shown}");
    }

    #[test]
    fn a_code_fence_is_never_split() {
        let mut fence = String::from("```text\n");
        for index in 0..400 {
            // 펜스 안에는 빈 줄도 있다 — 문단 경계처럼 보이는 자리에서도 나뉘지 않는다.
            fence.push_str(&format!("{index} 펜스 안의 줄\n\n"));
        }
        fence.push_str("```");

        let mut blocks: Vec<String> = (0..400)
            .map(|index| format!("### 구간 {index}\n펜스 앞의 문단이다."))
            .collect();
        blocks.push(fence.clone());
        blocks.extend((0..400).map(|index| format!("### 뒷 구간 {index}\n펜스 뒤의 문단이다.")));

        let markdown = document(&blocks);
        let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");

        assert_eq!(chunks.concat(), markdown);
        assert_within_budgets(&chunks);

        let holders = chunks
            .iter()
            .filter(|chunk| chunk.contains(fence.as_str()))
            .count();
        assert_eq!(holders, 1, "펜스 블록이 통째로 있는 chunk가 하나가 아니다");

        for chunk in &chunks {
            // 이 문서의 펜스는 하나뿐이다 — 어떤 chunk도 여는 펜스만, 닫는 펜스만 갖지 않는다.
            let fences = chunk.lines().filter(|line| is_fence(line)).count();
            assert!(fences == 0 || fences == 2, "펜스가 갈린 chunk가 있다: {fences}");
        }
    }

    #[test]
    fn a_code_fence_counts_as_one_block_unit() {
        let mut fence = String::from("```text\n");
        for index in 0..CHUNK_MAX_BLOCK_UNITS * 3 {
            fence.push_str(&format!("{index} 펜스 안의 줄\n"));
        }
        fence.push_str("```");

        let markdown = document(&["# 제목".to_owned(), fence]);
        let chunks = split_markdown(&markdown).expect("바이트 예산 안의 문서다");

        // 줄 수로 세면 예산을 한참 넘지만 펜스는 통째로 1이다 (ADR-0009 §6.2).
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks.concat(), markdown);
    }

    #[test]
    fn a_split_lands_on_a_paragraph_boundary_when_it_can() {
        let blocks: Vec<String> = (0..1_000)
            .map(|index| format!("### 구간 {index}\n문단 경계에서 나뉘어야 한다."))
            .collect();
        let markdown = document(&blocks);

        let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");

        assert!(chunks.len() > 1);
        for chunk in &chunks[..chunks.len() - 1] {
            assert!(chunk.ends_with("\n\n"), "문단 경계가 아닌 자리에서 나뉘었다");
        }
        for chunk in &chunks[1..] {
            assert!(chunk.starts_with("### "), "chunk가 문단 가운데서 시작한다");
        }
    }

    #[test]
    fn a_paragraph_bigger_than_the_budget_is_split_at_its_line_boundaries() {
        // 빈 줄이 하나도 없는 문단 하나 — 나눌 자리가 줄 경계뿐이다 (§6.3-2).
        let mut paragraph = String::new();
        for index in 0..CHUNK_MAX_BLOCK_UNITS * 4 {
            paragraph.push_str(&format!("- 항목 {index}\n"));
        }

        let chunks = split_markdown(&paragraph).expect("줄 경계가 있다");

        assert!(chunks.len() > 1);
        assert_eq!(chunks.concat(), paragraph);
        assert_within_budgets(&chunks);

        for chunk in &chunks {
            assert!(chunk.ends_with('\n'), "줄 가운데서 나뉘었다");
            assert!(chunk.starts_with("- 항목 "), "줄 가운데서 시작한다");
        }
    }

    #[test]
    fn a_document_without_a_trailing_newline_keeps_its_last_byte() {
        let markdown = "# 제목\n\n마지막 줄에는 개행이 없다";
        let chunks = split_markdown(markdown).expect("예산 안의 문서다");

        assert_eq!(chunks.concat(), markdown);
    }

    #[test]
    fn every_chunk_boundary_sits_on_a_line_boundary() {
        let markdown = hour_long_transcript();
        let chunks = split_markdown(&markdown).expect("나눌 수 있는 문서다");

        for chunk in &chunks[..chunks.len() - 1] {
            assert!(chunk.ends_with('\n'), "chunk가 줄 가운데서 끝난다");
        }
    }

    #[test]
    fn the_byte_budget_stays_under_the_verified_general_request_limit() {
        // VERIFIED된 것은 "요청당 500KB overall" 하나다. 이 앱이 고른 60,000이 그 아래에
        // 있다는 사실을 여기서 고정한다 — 상수를 키우려는 다음 사람이 이 계산을 먼저 본다.
        const VERIFIED_REQUEST_LIMIT_BYTES: usize = 500_000;
        const WORST_CASE_JSON_ESCAPE: usize = 6;

        // 컴파일 시점에 확인한다 — 예산을 키우려는 다음 사람은 테스트를 돌리기 전에 막힌다.
        const {
            assert!(CHUNK_MAX_BYTES * WORST_CASE_JSON_ESCAPE < VERIFIED_REQUEST_LIMIT_BYTES);
        }
    }

    #[test]
    fn atoms_cover_the_whole_document_in_order() {
        let markdown = "# 제목\n\n```text\n펜스\n\n안\n```\n\n마지막";
        let atoms = atoms(markdown);

        let mut offset = 0;
        for atom in &atoms {
            assert_eq!(atom.start, offset, "단위 사이에 빈틈이 있다");
            offset = atom.end;
        }
        assert_eq!(offset, markdown.len(), "문서 끝까지 덮지 않았다");
    }
}
