//! 사람이 자기 AI 채팅으로 가져갈 문자열의 **크기와 나눔**을 값으로 만드는 순수 모듈
//! (`phase-prompt/05.6` 성공 기준 4 · R-5).
//!
//! ```text
//! 문자열 ─┬─→ measure ─→ TextSize            얼마나 큰가
//!         └─→ split   ─→ [조각 1/3, 2/3, 3/3]  예산을 넘으면 순서대로 나눈 조각들
//! ```
//!
//! ## 왜 있는가
//!
//! 72분 녹음의 Markdown 산출물은 **99 KB · 5,139줄**이었다 (`phase-prompt/05.6` R-5의 실측).
//! 사람은 그것을 채팅 창에 붙여 넣으려다 실패했고, **앱은 그 사실을 말해 주지 않았다.**
//! 여기 있는 두 값이 그 침묵을 없앤다 — 화면은 [`TextSize`]로 크기를 말하고, [`Portion`]으로
//! "지금 가져가는 것이 셋 중 첫 번째"라고 말할 수 있다.
//!
//! ## 지키는 성질 하나 — 재조립 동등성
//!
//! ```text
//! split(text)의 조각들을 이어 붙이면 text다   (바이트 단위로 같다)
//! ```
//!
//! 나누기는 **문자열을 자르는 것이지 바꾸는 것이 아니다.** 돌려주는 조각은 전부 입력의 연속된
//! 부분 슬라이스이며, 경계에서 공백을 먹지도 더하지도 않는다. 조각이 자기 자리를 말하는 방법도
//! **값**([`Portion::index`] · [`Portion::total`])이지 문자열에 적어 넣는 표식이 아니다 —
//! 표식을 본문에 섞으면 이어 붙인 결과가 원본이 아니게 되고, 무엇을 어떻게 보여 줄지는
//! 화면의 일이다.
//!
//! ## 조각 경계는 문장 한가운데에 놓이지 않는다
//!
//! 나누는 자리는 **가장 큰 단위부터** 찾는다. 문단 경계로 충분하면 줄 경계까지 내려가지
//! 않는다.
//!
//! ```text
//! 1. 빈 줄(문단 경계)에서 나눈다.                          ← 기본
//! 2. 한 문단이 혼자 예산을 넘으면 그 안의 줄 경계에서.
//! 3. 한 줄이 혼자 예산을 넘으면 그 안의 문장 경계에서.      ← 여기까지가 "문장을 자르지 않는다"
//! 4. 한 문장이 혼자 예산을 넘으면 낱말 경계에서.
//! 5. 낱말 하나가 혼자 예산을 넘으면 글자 경계에서.          ← 마지막 수단
//! ```
//!
//! **4와 5에서도 한 글자도 잃지 않는다.** 예산보다 긴 문장 하나를 만났을 때 이 모듈이 할 수
//! 있는 일은 두 가지뿐이다 — 내용을 버리거나, 문장 안에서 나누거나. 버리지 않는 쪽을 고른다.
//! 실패를 돌려주지 않는 것도 같은 이유다: 사람이 요청한 것은 "가져가기"이며, 크기 때문에
//! 아무것도 주지 않는 것은 그 요청에 대한 답이 아니다.
//!
//! ## 이 모듈이 알지 않는 것
//!
//! 파일 시스템 · 저장소 · 네트워크 · clipboard · 시계 · 로캘을 알지 않는다. 여기 있는 것은
//! 문자열에서 문자열 조각을 만드는 함수뿐이고, 같은 입력은 언제나 같은 결과를 낸다 (§18).

/// 조각 하나의 UTF-8 바이트 예산.
///
/// ⚠️ **이 값은 이 앱이 고른 값이다. 어떤 AI 채팅 서비스의 확인된 한도가 아니다.**
///
/// **무엇에서 나왔는가**
///
/// ```text
/// 관측된 사실   72분 녹음의 Markdown 산출물이 99 KB · 5,139줄   (phase-prompt/05.6 R-5)
/// 이 앱의 선택  40,000 B — 한글로 약 13,000자. 위 문서가 서너 조각이 되는 크기다
/// ```
///
/// **무엇에서 나오지 않았는가**
///
/// - 어떤 AI 채팅이 한 번에 받는 텍스트의 한도는 **UNVERIFIED다.** 서비스마다 다르고, 이
///   저장소가 확인한 primary source가 없다. 그런 숫자를 여기 적으면 그것은 벤더 지식이 되고,
///   이 제품은 산출물에도 코드에도 벤더를 담지 않는다 (INV-9 · MH-6).
/// - [`crate::notion::chunk::CHUNK_MAX_BYTES`]에서 가져오지 **않았다.** 그 값은 VERIFIED된
///   Notion 요청 한도(요청당 500KB)에서 유도한 **벤더 제약의 표현**이고, 이 값은 **사람이 한
///   번에 붙여 넣고 확인할 수 있는 크기**에 대한 이 앱의 판단이다. 두 숫자는 서로를 근거로
///   삼지 않으며, 한쪽을 고쳐도 다른 쪽은 따라 움직이지 않는다.
///
/// 줄이는 것도 늘리는 것도 이 앱의 판단이며, 상수 한 줄을 고치는 일이다.
pub const PORTION_MAX_BYTES: usize = 40_000;

/// 문자열 하나가 얼마나 큰가.
///
/// **이 값이 없으면 화면은 사용자에게 크기를 말할 수 없다.** 세 가지를 함께 두는 이유는 사람이
/// 크기를 재는 방법이 하나가 아니기 때문이다 — 바이트는 예산과 견주는 값이고, 글자 수는 사람이
/// 채팅 창에서 감각하는 값이며, 줄 수는 전사에서 segment 수에 가깝다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSize {
    /// UTF-8 바이트 수. 예산과 견주는 값이다.
    pub bytes: usize,
    /// 글자(Unicode scalar value) 수. 한글 한 글자는 3바이트이므로 바이트와 크게 다르다.
    pub chars: usize,
    /// 줄 수. 마지막 줄이 개행으로 끝나도 한 줄로 세지 않는다 (`str::lines`와 같다).
    pub lines: usize,
}

impl TextSize {
    /// 한 조각에 들어가는가 — [`split`]이 나누지 않고 통째로 돌려주는가와 같은 판정이다.
    pub fn fits_in_one_portion(&self) -> bool {
        self.bytes <= PORTION_MAX_BYTES
    }
}

/// 순서대로 나뉜 조각 하나 — **자기 자리를 스스로 말한다.**
///
/// 조각은 언제나 원본의 연속된 부분이며, `index` 순서대로 이어 붙이면 원본이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Portion<'a> {
    /// 몇 번째 조각인가. **1부터 센다** — 사람에게 보이는 값이기 때문이다.
    pub index: usize,
    /// 조각이 전부 몇 개인가.
    pub total: usize,
    /// 이 조각의 내용. 원본의 부분 슬라이스이며 여기서 만들어진 글자는 하나도 없다.
    pub text: &'a str,
}

impl Portion<'_> {
    /// 이 조각의 크기.
    pub fn size(&self) -> TextSize {
        measure(self.text)
    }

    /// 나뉘지 않았는가 — 이 조각 하나가 원본 전부인가.
    pub fn is_whole(&self) -> bool {
        self.total == 1
    }
}

/// 문자열 하나의 크기를 잰다. **아무것도 만들지 않고 아무것도 자르지 않는다.**
///
/// ```
/// use molt_note_lib::export::portion::measure;
///
/// let size = measure("00:00:03 안녕하세요.\n");
///
/// assert_eq!(size.chars, 16);
/// assert_eq!(size.lines, 1);
/// assert!(size.fits_in_one_portion());
/// ```
pub fn measure(text: &str) -> TextSize {
    TextSize {
        bytes: text.len(),
        chars: text.chars().count(),
        lines: text.lines().count(),
    }
}

/// 문자열 하나를 예산 아래의 조각들로 **순서대로 · 무손실로** 나눈다.
///
/// ```
/// use molt_note_lib::export::portion::split;
///
/// let text = "## Transcript\n00:00:03 안녕하세요.\n";
/// let portions = split(text);
///
/// assert_eq!(portions.len(), 1);
/// assert_eq!(portions[0].index, 1);
/// assert_eq!(portions[0].total, 1);
/// assert!(portions[0].is_whole());
/// assert_eq!(
///     portions.iter().map(|portion| portion.text).collect::<String>(),
///     text
/// );
/// ```
///
/// 예산 안의 문자열은 **나뉘지 않는다** — 조각 하나가 원본 그대로다. 예산을 넘으면 위 모듈
/// 문서의 순서(문단 → 줄 → 문장 → 낱말 → 글자)로 자리를 찾아 나눈다.
///
/// 빈 문자열은 조각이 하나도 없다. 가져갈 것이 없다는 사실을 조각 0개로 말하며, 빈 조각을
/// 만들어 내지 않는다.
pub fn split(text: &str) -> Vec<Portion<'_>> {
    let mut packer = Packer::new(text);

    for (start, end) in paragraphs(text) {
        pack(&mut packer, text, start, end, Unit::Paragraph);
    }

    let pieces = packer.finish();
    let total = pieces.len();

    pieces
        .into_iter()
        .enumerate()
        .map(|(offset, text)| Portion {
            index: offset + 1,
            total,
            text,
        })
        .collect()
}

/// 더 나눌 수 있는 단위의 크기 — 위에서 아래로 갈수록 잘다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
    Paragraph,
    Line,
    Sentence,
    Word,
}

/// 단위 하나를 담는다. 혼자서 예산을 넘으면 **한 단계 더 잘게 나눠 다시 담는다.**
///
/// 자르는 자리를 이렇게 찾기 때문에 "가능한 한 큰 단위를 지킨다"가 규칙 하나로 표현된다 —
/// 문단이 들어가면 줄을 보지 않고, 줄이 들어가면 문장을 보지 않는다.
fn pack(packer: &mut Packer<'_>, text: &str, start: usize, end: usize, unit: Unit) {
    if packer.push(start, end) {
        return;
    }

    let (pieces, finer) = match unit {
        Unit::Paragraph => (lines(text, start, end), Unit::Line),
        Unit::Line => (sentences(text, start, end), Unit::Sentence),
        Unit::Sentence => (words(text, start, end), Unit::Word),
        // 낱말 하나가 예산을 넘는다 — 더 잘 나눌 자리가 없다. 글자 경계에서만 자른다.
        // 문자 경계를 넘어 자르지 않으므로 깨진 글자가 만들어지지 않는다.
        Unit::Word => {
            for (from, to) in characters(text, start, end) {
                packer.push(from, to);
            }
            return;
        }
    };

    for (from, to) in pieces {
        pack(packer, text, from, to, finer);
    }
}

/// 예산 안에서 단위들을 순서대로 담는 자리.
///
/// 담는 것은 언제나 **바로 앞에서 끝난 자리부터 이어지는 범위**이며, 그래서 만들어진 조각들은
/// 원본을 빈틈없이 덮는다.
struct Packer<'a> {
    text: &'a str,
    pieces: Vec<&'a str>,
    open: Option<(usize, usize)>,
}

impl<'a> Packer<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            pieces: Vec::new(),
            open: None,
        }
    }

    /// 단위 하나를 담는다. 지금 조각에 들어가지 않으면 그것을 닫고 새 조각에서 다시 본다.
    ///
    /// 혼자서도 예산을 넘어 **어떤 조각에도 담기지 않으면** `false`다. 그때 이 함수는
    /// 아무것도 자르지 않았고 **열려 있던 조각도 닫지 않았다** — 부르는 쪽이 더 잘게 나눠
    /// 다시 담으면 그 조각이 이어서 채워진다.
    fn push(&mut self, start: usize, end: usize) -> bool {
        let bytes = end - start;

        if bytes > PORTION_MAX_BYTES {
            return false;
        }

        if let Some((open_start, open_end)) = self.open {
            debug_assert_eq!(open_end, start, "조각이 원본의 연속된 범위가 아니다");

            if end - open_start <= PORTION_MAX_BYTES {
                self.open = Some((open_start, end));
                return true;
            }

            self.close();
        }

        self.open = Some((start, end));
        true
    }

    fn close(&mut self) {
        if let Some((start, end)) = self.open.take() {
            if end > start {
                self.pieces.push(&self.text[start..end]);
            }
        }
    }

    fn finish(mut self) -> Vec<&'a str> {
        self.close();
        self.pieces
    }
}

/// 문단들 — 빈 줄에서 끝난다. **빈 줄은 앞 문단에 붙는다**(새 조각이 빈 줄로 시작하지 않는다).
fn paragraphs(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut cursor = 0;

    while cursor < bytes.len() {
        if bytes[cursor] == b'\n' && bytes.get(cursor + 1) == Some(&b'\n') {
            let mut cut = cursor + 1;
            while bytes.get(cut) == Some(&b'\n') {
                cut += 1;
            }

            ranges.push((start, cut));
            start = cut;
            cursor = cut;
            continue;
        }

        cursor += 1;
    }

    if start < text.len() {
        ranges.push((start, text.len()));
    }

    ranges
}

/// 줄들 — 각 줄에는 **자신의 개행이 포함된다.** 그래야 이어 붙였을 때 원본이 된다.
fn lines(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut open = start;

    for (offset, byte) in text.as_bytes()[start..end].iter().enumerate() {
        if *byte == b'\n' {
            ranges.push((open, start + offset + 1));
            open = start + offset + 1;
        }
    }

    if open < end {
        ranges.push((open, end));
    }

    ranges
}

/// 문장을 끝내는 글자들.
///
/// 종결 부호 **하나만으로는 문장의 끝이 아니다** — 뒤에 공백이나 범위의 끝이 와야 한다.
/// 그래서 `1.5`의 마침표나 파일 이름 안의 점에서 나뉘지 않는다.
const SENTENCE_ENDS: [char; 7] = ['.', '!', '?', '…', '。', '！', '？'];

/// 문장들 — 종결 부호와 그 뒤의 공백까지가 한 문장이다.
///
/// 뒤따르는 공백을 앞 문장에 붙이는 이유는 [`paragraphs`]가 빈 줄을 앞에 붙이는 것과 같다:
/// 다음 조각이 공백으로 시작하지 않게 한다.
fn sentences(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut open = start;
    let mut cursor = start;

    while cursor < end {
        let letter = next_char(text, cursor, end);
        let after = cursor + letter.len_utf8();

        if SENTENCE_ENDS.contains(&letter) {
            let mut cut = after;
            while cut < end {
                let following = next_char(text, cut, end);
                if !following.is_whitespace() {
                    break;
                }
                cut += following.len_utf8();
            }

            if cut > after || after == end {
                ranges.push((open, cut));
                open = cut;
                cursor = cut;
                continue;
            }
        }

        cursor = after;
    }

    if open < end {
        ranges.push((open, end));
    }

    ranges
}

/// 낱말들 — 공백은 앞 낱말에 붙는다.
fn words(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut open = start;
    let mut cursor = start;
    let mut after_space = false;

    while cursor < end {
        let letter = next_char(text, cursor, end);

        if letter.is_whitespace() {
            after_space = true;
        } else if after_space {
            ranges.push((open, cursor));
            open = cursor;
            after_space = false;
        }

        cursor += letter.len_utf8();
    }

    if open < end {
        ranges.push((open, end));
    }

    ranges
}

/// 글자들 — 한 글자가 한 단위다. **마지막 수단이며, 여기서도 글자는 쪼개지지 않는다.**
fn characters(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut cursor = start;

    while cursor < end {
        let letter = next_char(text, cursor, end);
        ranges.push((cursor, cursor + letter.len_utf8()));
        cursor += letter.len_utf8();
    }

    ranges
}

/// `cursor`에서 시작하는 글자 하나. 범위 안에 글자가 있을 때만 부른다.
fn next_char(text: &str, cursor: usize, end: usize) -> char {
    text[cursor..end]
        .chars()
        .next()
        .expect("범위 안에 글자가 남아 있다")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 조각들을 이어 붙인 문자열.
    fn rejoined(portions: &[Portion<'_>]) -> String {
        portions.iter().map(|portion| portion.text).collect()
    }

    /// 조각들이 지켜야 하는 성질 전부 — 무손실 · 순서 · 예산 · 자기 자리.
    fn assert_portions_are_sound(portions: &[Portion<'_>], text: &str) {
        assert_eq!(rejoined(portions), text, "이어 붙인 결과가 원본과 다르다");

        for (offset, portion) in portions.iter().enumerate() {
            assert!(!portion.text.is_empty(), "빈 조각이 있다");
            assert!(
                portion.text.len() <= PORTION_MAX_BYTES,
                "{}바이트짜리 조각이 있다",
                portion.text.len()
            );
            assert_eq!(portion.index, offset + 1, "조각이 자기 순번을 모른다");
            assert_eq!(portion.total, portions.len(), "조각이 전체 개수를 모른다");
        }
    }

    /// AI Handoff의 압축 모양과 같은 형태의 transcript — segment 하나가 한 줄이다.
    fn compact_transcript(segments: usize) -> String {
        let mut text = String::from("## Transcript\n");

        for index in 0..segments {
            let seconds = index * 3;
            text.push_str(&format!(
                "{:02}:{:02}:{:02} 발표자는 {index}번째 구간에서 3D Gaussian Splatting의 학습 \
                 파이프라인과 렌더링 품질 지표를 설명했고, 다음 주까지 실험 결과를 정리해 \
                 공유하기로 했다.\n",
                seconds / 3_600,
                (seconds / 60) % 60,
                seconds % 60
            ));
        }

        text
    }

    /// 줄 하나가 정확히 40바이트인 문단 — 예산의 경계를 글자 수로 맞추기 위한 입력이다.
    fn ascii_lines(count: usize) -> String {
        let mut text = String::new();
        for index in 0..count {
            text.push_str(&format!("{index:0>39}\n"));
        }
        text
    }

    // ── 크기를 값으로 낸다 ───────────────────────────────────────────────────────

    #[test]
    fn measuring_says_how_big_the_text_is_in_bytes_characters_and_lines() {
        let size = measure("00:00:03 안녕하세요.\n00:00:06 반갑습니다.\n");

        // 한글은 한 글자가 3바이트다 — 바이트만 보여 주면 사람은 크기를 감각할 수 없다.
        assert_eq!(size.chars, 32);
        assert_eq!(size.bytes, "00:00:03 안녕하세요.\n00:00:06 반갑습니다.\n".len());
        assert!(size.bytes > size.chars);
        assert_eq!(size.lines, 2);
    }

    #[test]
    fn an_empty_text_has_no_size_and_no_portion_at_all() {
        assert_eq!(
            measure(""),
            TextSize {
                bytes: 0,
                chars: 0,
                lines: 0
            }
        );
        assert!(measure("").fits_in_one_portion());
        assert!(split("").is_empty(), "빈 조각을 만들어 내지 않는다");
    }

    #[test]
    fn the_size_says_whether_it_fits_in_one_portion_and_the_split_agrees() {
        for text in [
            String::from("작은 문서"),
            ascii_lines(PORTION_MAX_BYTES / 40),
            ascii_lines(PORTION_MAX_BYTES / 40 + 1),
            compact_transcript(1_711),
        ] {
            let size = measure(&text);
            let portions = split(&text);

            assert_eq!(
                size.fits_in_one_portion(),
                portions.len() == 1,
                "크기가 말하는 것과 실제 나눔이 다르다: {} 바이트 · {} 조각",
                size.bytes,
                portions.len()
            );
        }
    }

    // ── 나눔 — 무손실 · 순서 · 자기 자리 ─────────────────────────────────────────

    #[test]
    fn a_text_under_the_budget_is_one_whole_portion() {
        let text = "## Transcript\n00:00:03 안녕하세요. 오늘은 3DGS를 봅니다.\n";

        let portions = split(text);

        assert_eq!(portions.len(), 1);
        assert_eq!(portions[0].text, text, "나누지 않은 조각은 원본 그대로다");
        assert_eq!(portions[0].index, 1);
        assert_eq!(portions[0].total, 1);
        assert!(portions[0].is_whole());
        assert_eq!(portions[0].size(), measure(text));
    }

    #[test]
    fn a_text_over_the_budget_is_split_and_rejoins_into_the_original() {
        let text = compact_transcript(1_711);

        // 이 입력이 실제로 여러 조각이 되는 규모인지 먼저 고정한다 — 한 조각에 들어가는
        // 문서로는 이 테스트가 아무것도 검사하지 못한다.
        assert!(
            text.len() > 2 * PORTION_MAX_BYTES,
            "72분 규모라기에 너무 작다: {} 바이트",
            text.len()
        );

        let portions = split(&text);

        assert!(portions.len() > 2, "조각이 {}개뿐이다", portions.len());
        assert_portions_are_sound(&portions, &text);
        assert!(portions.iter().all(|portion| !portion.is_whole()));
    }

    #[test]
    fn every_portion_says_which_one_it_is_and_how_many_there_are() {
        let text = compact_transcript(1_711);

        let portions = split(&text);

        let places: Vec<(usize, usize)> = portions
            .iter()
            .map(|portion| (portion.index, portion.total))
            .collect();
        let expected: Vec<(usize, usize)> = (1..=portions.len())
            .map(|index| (index, portions.len()))
            .collect();

        assert_eq!(places, expected);
        // 자리는 값이지 본문에 적힌 표식이 아니다 — 그래서 이어 붙이면 원본이 된다.
        assert_eq!(rejoined(&portions), text);
    }

    #[test]
    fn splitting_the_same_text_twice_gives_exactly_the_same_portions() {
        // 시계도 난수도 해시맵 순회도 없다 (§18).
        for text in [
            String::new(),
            String::from("짧은 문서"),
            compact_transcript(1_711),
            ascii_lines(3_000),
        ] {
            assert_eq!(split(&text), split(&text));
        }
    }

    #[test]
    fn a_text_without_a_trailing_newline_keeps_its_last_byte() {
        let text = format!("{}마지막 줄에는 개행이 없다", ascii_lines(2_000));

        let portions = split(&text);

        assert!(portions.len() > 1, "나뉘지 않았다");
        assert_portions_are_sound(&portions, &text);
        assert!(portions[portions.len() - 1]
            .text
            .ends_with("마지막 줄에는 개행이 없다"));
    }

    // ── 경계값 ───────────────────────────────────────────────────────────────────

    #[test]
    fn a_text_of_exactly_the_budget_is_not_split_and_one_byte_more_is() {
        let exact = ascii_lines(PORTION_MAX_BYTES / 40);
        assert_eq!(exact.len(), PORTION_MAX_BYTES, "사전 조건: 예산과 같은 크기");

        let portions = split(&exact);
        assert_eq!(portions.len(), 1, "예산과 같은 크기는 나누지 않는다");
        assert_eq!(portions[0].text, exact);

        // 줄 하나(40바이트)를 더하면 예산을 넘는다 — 그때 처음으로 나뉜다.
        let over = ascii_lines(PORTION_MAX_BYTES / 40 + 1);
        let portions = split(&over);

        assert_eq!(portions.len(), 2);
        assert_eq!(portions[0].text.len(), PORTION_MAX_BYTES);
        assert_eq!(portions[1].text.len(), 40);
        assert_portions_are_sound(&portions, &over);
    }

    #[test]
    fn one_character_of_the_budget_is_a_portion_of_its_own() {
        // 예산을 한 글자 넘긴 것도 두 조각이다 — "거의 맞으니 잘라 버린다"가 없다.
        let text = format!("{}가", ascii_lines(PORTION_MAX_BYTES / 40));

        let portions = split(&text);

        assert_eq!(portions.len(), 2);
        assert_eq!(portions[1].text, "가");
        assert_portions_are_sound(&portions, &text);
    }

    // ── 경계가 놓이는 자리 ───────────────────────────────────────────────────────

    #[test]
    fn a_split_lands_on_a_paragraph_boundary_when_it_can() {
        let mut text = String::new();
        for index in 0..2_000 {
            text.push_str(&format!("### 구간 {index}\n문단 경계에서 나뉘어야 한다.\n\n"));
        }

        let portions = split(&text);

        assert!(portions.len() > 2);
        assert_portions_are_sound(&portions, &text);

        for portion in &portions[..portions.len() - 1] {
            assert!(
                portion.text.ends_with("\n\n"),
                "문단 경계가 아닌 자리에서 나뉘었다"
            );
        }
        for portion in &portions[1..] {
            assert!(
                portion.text.starts_with("### "),
                "조각이 문단 가운데서 시작한다"
            );
        }
    }

    #[test]
    fn a_paragraph_bigger_than_the_budget_is_split_at_its_line_boundaries() {
        // 빈 줄이 하나도 없는 문단 하나 — AI Handoff의 압축 모양이 바로 이 모양이다.
        let text = compact_transcript(1_711);

        let portions = split(&text);

        assert!(portions.len() > 2);
        assert_portions_are_sound(&portions, &text);

        for portion in &portions[..portions.len() - 1] {
            assert!(portion.text.ends_with('\n'), "줄 가운데서 나뉘었다");
        }
        for portion in &portions[1..] {
            // 압축 모양에서 줄의 첫 글자는 timestamp의 첫 숫자다.
            let first = portion.text.chars().next().expect("빈 조각이 아니다");
            assert!(first.is_ascii_digit(), "조각이 줄 가운데서 시작한다: {first}");
        }
    }

    #[test]
    fn a_line_bigger_than_the_budget_is_split_between_sentences() {
        // 줄바꿈이 하나도 없는 한 줄 — 나눌 자리가 문장 경계뿐이다. segment가 없어
        // `raw_text` 한 문단으로 내려간 전사가 이 모양이다.
        let sentence = "이 문장은 문장 경계에서만 나뉘어야 한다. ";
        let text = sentence.repeat(2_000);

        let portions = split(&text);

        assert!(portions.len() > 2);
        assert_portions_are_sound(&portions, &text);

        for portion in &portions[..portions.len() - 1] {
            assert!(
                portion.text.ends_with(". "),
                "문장 한가운데서 잘렸다: …{}",
                portion.text.chars().rev().take(10).collect::<String>()
            );
        }
        for portion in &portions[1..] {
            assert!(portion.text.starts_with("이 문장은"), "문장 가운데서 시작한다");
        }
    }

    #[test]
    fn a_number_with_a_dot_in_it_is_not_a_sentence_boundary() {
        // `1.5`의 마침표에서 나누면 그것은 문장 경계가 아니다.
        let ranges = sentences("값은 1.5다. 다음 문장.", 0, "값은 1.5다. 다음 문장.".len());

        let cut: Vec<&str> = ranges
            .iter()
            .map(|(start, end)| &"값은 1.5다. 다음 문장."[*start..*end])
            .collect();
        assert_eq!(cut, ["값은 1.5다. ", "다음 문장."]);
    }

    // ── 마지막 수단 — 그래도 한 글자도 잃지 않는다 ───────────────────────────────

    #[test]
    fn a_sentence_longer_than_the_budget_is_split_without_losing_a_character() {
        // 문장 하나가 예산보다 길면 문장 안에서 나눌 수밖에 없다. **버리는 쪽을 고르지
        // 않는다** — 사람이 요청한 것은 가져가기이며, 잘린 문서를 성공이라 부르지 않는다.
        let text = format!("{} 끝.", "말이 계속 이어진다 ".repeat(4_000));

        let portions = split(&text);

        assert!(portions.len() > 1);
        assert_portions_are_sound(&portions, &text);
        // 글자 수의 합이 원본과 같다 — 경계에서 먹힌 글자가 없다.
        assert_eq!(
            portions
                .iter()
                .map(|portion| portion.size().chars)
                .sum::<usize>(),
            measure(&text).chars
        );
    }

    #[test]
    fn a_word_longer_than_the_budget_is_split_only_at_character_boundaries() {
        // 공백도 문장 부호도 없는 한 덩어리 — 마지막 수단인 글자 경계까지 내려간다.
        // 한글 한 글자는 3바이트이므로, 바이트 자리로 자르면 글자가 깨진다.
        let text = "가".repeat(PORTION_MAX_BYTES);
        assert!(text.len() > PORTION_MAX_BYTES, "사전 조건: 예산을 넘는다");

        let portions = split(&text);

        assert!(portions.len() > 1);
        assert_portions_are_sound(&portions, &text);

        for portion in &portions {
            // 슬라이스가 문자 경계에 있으므로 조각은 언제나 온전한 `가`들로만 이루어진다.
            assert_eq!(portion.text.len() % 3, 0, "글자가 쪼개졌다");
            assert!(portion.text.chars().all(|letter| letter == '가'));
        }
        assert_eq!(
            portions
                .iter()
                .map(|portion| portion.size().chars)
                .sum::<usize>(),
            PORTION_MAX_BYTES
        );
    }

    // ── 예산의 출처 ──────────────────────────────────────────────────────────────

    #[test]
    fn the_budget_is_this_apps_choice_and_not_the_notion_api_constraint() {
        // ⚠️ 이 테스트가 걸렸다면 두 예산 중 하나를 다른 하나에 맞춘 것이다. 두 값은 서로
        // 다른 것에서 나왔다 — 하나는 VERIFIED된 Notion 요청 한도에서 유도한 벤더 제약의
        // 표현이고(`notion::chunk`), 다른 하나는 사람이 한 번에 붙여 넣는 크기에 대한 이
        // 앱의 판단이다. 같은 숫자로 만들려면 먼저 그 근거가 하나로 합쳐져야 한다.
        assert_ne!(
            PORTION_MAX_BYTES,
            crate::notion::chunk::CHUNK_MAX_BYTES,
            "AI Handoff의 예산이 Notion API 제약의 값과 같아졌다"
        );

        // 이 모듈은 Notion의 두 예산 어느 쪽도 읽지 않는다 — 값이 바뀌어도 여기는 움직이지
        // 않는다는 사실을 소스로 못박는다.
        let source = include_str!("portion.rs");
        let product_code = source
            .split("#[cfg(test)]")
            .next()
            .expect("제품 코드가 있다");
        for forbidden in ["CHUNK_MAX_BYTES", "CHUNK_MAX_BLOCK_UNITS", "split_markdown"] {
            assert!(
                !product_code
                    .lines()
                    .filter(|line| !line.trim_start().starts_with("//"))
                    .any(|line| line.contains(forbidden)),
                "제품 코드가 Notion의 {forbidden}에 기대고 있다"
            );
        }
    }
}
