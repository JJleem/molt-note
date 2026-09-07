//! Recording의 날짜와 제목에서 **결정론적이고 안전한 파일 이름 하나**를 만드는 순수 함수
//! (PRODUCT-SPEC §11 · ADR-0009 §4.2 · `phase-prompt/05` 요구 A-2).
//!
//! ```text
//! created_at "2026-09-01T10:00:00.000Z" ─┐
//!                                        ├─→ "2026-09-01-3dgs-study-04.md"
//! title      "3DGS Study #04" ───────────┘
//! ```
//!
//! **파일시스템에 묻지 않는다.** 여기에는 경로도, 디렉터리도, `std::fs`도 없다 — 만들어진
//! 이름을 실제 자리에 놓는 일과 같은 이름이 이미 있을 때 번호를 붙이는 일(ADR-0009 §4.3)은
//! 이 모듈 밖의 몫이다. 그래서 적대적인 제목도 실제 파일 없이 전부 검증된다 (§18).
//!
//! 규칙은 ADR-0009 §4.2가 정한 순서 그대로다.
//!
//! ```text
//! 1. 유니코드 문자/숫자는 남긴다 — **한글도 남긴다**
//! 2. 그 밖의 전부(공백 · 문장부호 · / \ : * ? " < > | · 제어문자 · 개행 · 이모지)는 `-`
//! 3. 연속된 `-`는 하나로, 앞뒤의 `-`는 없앤다
//! 4. **ASCII 대문자만** 소문자로 — 유니코드 케이스 폴딩은 하지 않는다
//! 5. 80 바이트를 넘으면 UTF-8 문자 경계에서 자른다
//! 6. 결과가 비면 `untitled`
//! 7. Windows 예약 이름과 정확히 같으면 `-file`을 덧붙인다
//! ```
//!
//! 4번이 ASCII로 제한된 이유는 결정론이다 — 유니코드 케이스 폴딩은 언어별 규칙이 개입해서
//! 같은 제목이 로캘에 따라 다른 파일 이름을 만든다. 7번을 macOS 전용 Phase에서 미리 지키는
//! 이유도 하나다: **파일 이름은 만들어지고 나면 남는다.** 나중에 규칙을 바꾸면 이미 만들어진
//! 파일들과 규칙이 갈린다 (INV-10 · §3.1).

use super::iso_date;

/// 슬러그가 가질 수 있는 바이트 수의 상한.
///
/// **이것은 [A] 이 앱이 고른 값이다** — 파일시스템이 말한 한도가 아니다. 흔한 한도(대개 255
/// 바이트)에 `<date>-`(11 바이트) · `.md` · 충돌 접미사가 함께 들어갈 자리를 남긴다
/// (ADR-0009 §4.2).
pub const MAX_SLUG_BYTES: usize = 80;

/// 제목에서 남는 것이 없을 때 쓰는 슬러그 (ADR-0009 §4.2의 6).
pub const UNTITLED_SLUG: &str = "untitled";

/// `created_at`이 예상한 모양이 아닐 때 쓰는 날짜 (ADR-0009 §4.2).
///
/// **날짜를 지어내지 않는다.** 오늘 날짜를 채우려면 시계가 필요하고, 시계가 들어오는 순간
/// 같은 입력이 같은 이름을 낸다는 성질이 사라진다.
pub const UNKNOWN_DATE: &str = "unknown-date";

/// 만들어지는 파일의 확장자.
pub const MARKDOWN_EXTENSION: &str = "md";

/// 조각을 잇는 문자. 날짜와 슬러그 사이도, 슬러그 안의 자리도 이것 하나다.
const SEPARATOR: char = '-';

/// Windows 예약 장치 이름 (ADR-0009 §4.2의 7).
///
/// 끝나는 `.`과 공백은 이미 2·3에서 사라지므로 여기서 다시 다루지 않는다.
const WINDOWS_RESERVED_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// 예약 이름과 부딪혔을 때 덧붙이는 꼬리.
const RESERVED_NAME_SUFFIX: &str = "-file";

/// Recording 하나가 가질 파일 이름 — `2026-09-01-3dgs-study-04.md`.
///
/// 날짜는 `created_at`의 앞 10글자를 그대로 잘라 쓰고, 예상과 다른 모양이면
/// [`UNKNOWN_DATE`]다 ([`super::iso_date`]). 제목은 [`slug`]가 정규화한다.
///
/// 같은 입력은 언제나 같은 이름을 낸다. 시계도, 난수도, 파일시스템도 보지 않으므로
/// 이 함수가 돌려준 이름이 이미 쓰이고 있는지는 **여기서 알 수 없고, 알려고 하지도 않는다.**
pub fn export_file_name(created_at: &str, title: &str) -> String {
    let date = iso_date(created_at).unwrap_or(UNKNOWN_DATE);

    format!("{date}{SEPARATOR}{}.{MARKDOWN_EXTENSION}", slug(title))
}

/// AI-ready 문서의 이름에만 들어가는 **고정 표식** (ADR-0010 §5.6).
///
/// 같은 Recording의 Markdown export와 이름이 같으면 서로 다른 두 문서가 `-2` 접미사로만
/// 구분되고, 그 상태에서는 파일 목록만 보고 무엇이 무엇인지 알 수 없다.
pub const AI_REQUEST_MARKER: &str = "-ai-request";

/// 같은 Recording의 **AI-ready 문서**가 가질 파일 이름 —
/// `2026-09-01-3dgs-study-04-ai-request.md` (ADR-0010 §5.6).
///
/// **두 번째 이름 규칙을 만들지 않는다.** 날짜 · 슬러그 · `unknown-date` · Windows 예약 이름 ·
/// 80바이트 상한은 전부 [`export_file_name`] 한 자리에 그대로 있고, 여기서 더하는 것은 확장자
/// 앞의 [`AI_REQUEST_MARKER`] 하나뿐이다. 표식 11바이트는 ADR-0009 §4.2가 남겨 둔 여유 안에
/// 들어간다 [A].
///
/// 같은 이름이 이미 쓰이고 있는지는 여기서 알 수 없다 — 그 판단은
/// [`super::file::write_new`]의 몫이며, 덮어쓰지 않고 번호를 붙인다 (ADR-0009 §4.3).
pub fn ai_request_file_name(created_at: &str, title: &str) -> String {
    with_marker(&export_file_name(created_at, title), AI_REQUEST_MARKER)
}

/// 나뉜 조각 하나의 자리를 이름에 적는 꼴 — `-part-2-of-4` (`phase-prompt/05.6` 성공 기준 4).
///
/// **조각이 하나뿐이면 이 표식은 붙지 않는다** ([`ai_request_portion_file_name`]) — 나뉘지
/// 않은 문서의 이름이 이 Phase 때문에 달라지지 않는다.
pub const PORTION_MARKER: &str = "-part-";

/// 조각의 자리에서 `2`와 `4`를 잇는 낱말.
const PORTION_OF: &str = "-of-";

/// 나뉜 **AI-ready 문서 한 조각**이 가질 파일 이름 —
/// `2026-09-01-3dgs-study-04-ai-request-part-2-of-4.md`.
///
/// **세 번째 이름 규칙을 만들지 않는다.** 날짜 · 슬러그 · 예약 이름 · 80바이트 상한은 여전히
/// [`export_file_name`] 한 자리에 있고, 여기서 더하는 것은 확장자 앞의 표식 하나뿐이다.
///
/// 표식이 필요한 이유는 **덮어쓰지 않는 것만으로는 부족하기 때문이다.** 같은 이름으로 네 조각을
/// 쓰면 [`super::file::write_new`]가 `-2` · `-3` · `-4`를 붙여 파일은 지키지만 (ADR-0009 §4.3),
/// 그 번호는 **조각의 자리가 아니라 충돌의 순서**다 — 앞서 한 번 내보낸 적이 있으면 둘이
/// 어긋나고, 사용자는 파일 목록만 보고 어느 파일이 몇 번째 조각인지 알 수 없다.
///
/// 표식 길이는 조각이 아무리 많아도 스무 바이트 남짓이며, ADR-0009 §4.2가 남겨 둔 여유 안에
/// 들어간다 [A].
pub fn ai_request_portion_file_name(
    created_at: &str,
    title: &str,
    portion: usize,
    portion_count: usize,
) -> String {
    let name = ai_request_file_name(created_at, title);

    if portion_count <= 1 {
        return name;
    }

    with_marker(
        &name,
        &format!("{PORTION_MARKER}{portion}{PORTION_OF}{portion_count}"),
    )
}

/// 확장자 **앞에** 표식 하나를 끼워 넣는다.
fn with_marker(name: &str, marker: &str) -> String {
    let extension = format!(".{MARKDOWN_EXTENSION}");

    match name.strip_suffix(&extension) {
        Some(stem) => format!("{stem}{marker}{extension}"),
        // [`export_file_name`]은 언제나 확장자를 붙이므로 여기 오지 않는다. 그래도 이름을
        // 만들지 못하는 대신 표식을 붙인 이름을 낸다 — 파일 이름 하나 때문에 앱이 멈추지 않는다.
        None => format!("{name}{marker}"),
    }
}

/// 제목을 파일 이름에 쓸 수 있는 조각으로 정규화한다 (ADR-0009 §4.2의 슬러그 규칙).
///
/// 결과는 **언제나 비어 있지 않고**, 유니코드 문자/숫자와 `-`만으로 이루어진다. 그래서
/// `/` · `\` · `:` · 개행 · 제어문자 · `..` 같은 경로 탈출 시도가 남을 자리가 없다 —
/// 걸러 내는 것이 아니라 **남길 것만 남기기 때문이다.**
pub fn slug(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    // 구분자는 다음 글자가 실제로 남을 때만 붙는다. 그래서 연속된 `-`도, 앞뒤의 `-`도
    // 애초에 생기지 않는다 (규칙 3).
    let mut separator_pending = false;

    for character in title.chars() {
        if character.is_alphanumeric() {
            if separator_pending && !slug.is_empty() {
                slug.push(SEPARATOR);
            }
            separator_pending = false;
            // ASCII 대문자만 내린다 (규칙 4). 한글·숫자·그 밖의 문자는 그대로 남는다.
            slug.push(character.to_ascii_lowercase());
        } else {
            separator_pending = true;
        }
    }

    // 자른 자리가 `-` 바로 뒤일 수 있다. 그 하나만 다시 떨어진다 (규칙 5 뒤의 3).
    let truncated = truncate_at_char_boundary(&slug, MAX_SLUG_BYTES).trim_end_matches(SEPARATOR);

    if truncated.is_empty() {
        return UNTITLED_SLUG.to_owned();
    }
    if WINDOWS_RESERVED_NAMES.contains(&truncated) {
        return format!("{truncated}{RESERVED_NAME_SUFFIX}");
    }

    truncated.to_owned()
}

/// `limit` 바이트를 넘지 않도록 자른다. **문자 가운데를 자르지 않는다.**
///
/// 한 글자가 여러 바이트인 제목(한글이 기본이다)에서 바이트로만 자르면 유효하지 않은
/// UTF-8이 되거나 panic한다.
fn truncate_at_char_boundary(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }

    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 어떤 제목이 와도 이름이 지켜야 하는 것들 — 경로가 되지 않고, 숨은 파일이 되지 않고,
    /// 확장자가 남는다.
    fn assert_is_a_safe_file_name(name: &str) {
        for forbidden in ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0', '\n', '\r'] {
            assert!(
                !name.contains(forbidden),
                "이름에 {forbidden:?}가 남았다: {name}"
            );
        }
        assert!(!name.contains(".."), "경로를 거슬러 올라갈 수 있다: {name}");
        assert!(!name.starts_with('.'), "숨은 파일이 된다: {name}");
        assert!(!name.starts_with(SEPARATOR), "이름이 `-`로 시작한다: {name}");
        assert!(name.ends_with(".md"), "확장자가 없다: {name}");
        assert!(
            !name.chars().any(char::is_control),
            "제어문자가 남았다: {name:?}"
        );
        assert_eq!(name.trim(), name, "앞뒤에 공백이 있다: {name:?}");
    }

    // ── ADR-0009 §4.2의 예시 세 줄 ───────────────────────────────────────────────

    #[test]
    fn the_three_examples_of_the_adr_come_out_exactly_as_written() {
        assert_eq!(
            export_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04"),
            "2026-09-01-3dgs-study-04.md"
        );
        assert_eq!(
            export_file_name("2026-09-01T10:00:00.000Z", "회의: 로드맵 / Q4 🎯"),
            "2026-09-01-회의-로드맵-q4.md"
        );
        assert_eq!(
            export_file_name("2026-09-01T10:00:00.000Z", "///"),
            "2026-09-01-untitled.md"
        );
    }

    // ── AI-ready 문서의 이름 (ADR-0010 §5.6) ─────────────────────────────────────

    #[test]
    fn the_ai_request_name_is_the_export_name_with_one_marker_before_the_extension() {
        assert_eq!(
            ai_request_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04"),
            "2026-09-01-3dgs-study-04-ai-request.md"
        );
        // 같은 Recording의 두 문서가 **이름에서** 구분된다 — `-2` 접미사에 기대지 않는다.
        assert_ne!(
            ai_request_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04"),
            export_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04")
        );
    }

    #[test]
    fn the_ai_request_name_inherits_every_rule_of_the_export_name() {
        for (created_at, title) in [
            ("2026-09-01T10:00:00.000Z", "회의: 로드맵 / Q4 🎯"),
            ("2026-09-01T10:00:00.000Z", "///"),
            ("어제", "con"),
            ("", "../../etc/passwd"),
            ("2026-09-01", &"가".repeat(200)),
        ] {
            let name = ai_request_file_name(created_at, title);
            let exported = export_file_name(created_at, title);

            assert_is_a_safe_file_name(&name);
            assert_eq!(
                name,
                format!(
                    "{}{AI_REQUEST_MARKER}.{MARKDOWN_EXTENSION}",
                    exported
                        .strip_suffix(".md")
                        .expect("export 이름은 언제나 .md로 끝난다")
                ),
                "표식 하나만 더한다: {title}"
            );
        }
    }

    #[test]
    fn the_same_recording_always_gets_the_same_ai_request_name() {
        // 시계도 난수도 파일시스템도 보지 않는다 — export 이름과 같은 성질이다.
        assert_eq!(
            ai_request_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04"),
            ai_request_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04")
        );
    }

    // ── 나뉜 조각의 이름 (`phase-prompt/05.6` 성공 기준 4) ───────────────────────

    #[test]
    fn one_whole_document_keeps_the_name_it_already_had() {
        // 나뉘지 않은 문서의 이름이 이 Phase 때문에 달라지지 않는다.
        for portion_count in [0, 1] {
            assert_eq!(
                ai_request_portion_file_name(
                    "2026-09-01T10:00:00.000Z",
                    "3DGS Study #04",
                    1,
                    portion_count
                ),
                ai_request_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04")
            );
        }
    }

    #[test]
    fn a_split_document_says_which_portion_each_file_holds() {
        let names: Vec<String> = (1..=4)
            .map(|portion| {
                ai_request_portion_file_name(
                    "2026-09-01T10:00:00.000Z",
                    "3DGS Study #04",
                    portion,
                    4,
                )
            })
            .collect();

        assert_eq!(
            names,
            [
                "2026-09-01-3dgs-study-04-ai-request-part-1-of-4.md",
                "2026-09-01-3dgs-study-04-ai-request-part-2-of-4.md",
                "2026-09-01-3dgs-study-04-ai-request-part-3-of-4.md",
                "2026-09-01-3dgs-study-04-ai-request-part-4-of-4.md",
            ]
        );

        // 네 이름이 전부 다르다 — 충돌 번호(`-2`)에 기대지 않고 이름 자체가 자리를 말한다.
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len());
    }

    #[test]
    fn a_portion_name_is_still_a_safe_file_name_and_still_marks_the_ai_request() {
        for (created_at, title) in [
            ("2026-09-01T10:00:00.000Z", "회의: 로드맵 / Q4 🎯"),
            ("어제", "con"),
            ("", "../../etc/passwd"),
            ("2026-09-01", &"가".repeat(200)),
        ] {
            let name = ai_request_portion_file_name(created_at, title, 12, 137);

            assert_is_a_safe_file_name(&name);
            assert!(name.contains(AI_REQUEST_MARKER), "AI 문서라는 표식이 없다");
            assert!(name.ends_with("-part-12-of-137.md"), "조각의 자리가 없다: {name}");
        }
    }

    #[test]
    fn the_same_portion_always_gets_the_same_name() {
        assert_eq!(
            ai_request_portion_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04", 2, 4),
            ai_request_portion_file_name("2026-09-01T10:00:00.000Z", "3DGS Study #04", 2, 4)
        );
    }

    // ── 적대적인 제목 ────────────────────────────────────────────────────────────

    #[test]
    fn a_title_that_tries_to_escape_the_directory_becomes_an_ordinary_name() {
        // `..`도 `/`도 남을 자리가 없다 — 남기는 것은 문자와 숫자뿐이다.
        assert_eq!(
            export_file_name("2026-09-01", "../../etc/passwd"),
            "2026-09-01-etc-passwd.md"
        );
        assert_eq!(export_file_name("2026-09-01", ".."), "2026-09-01-untitled.md");
        assert_eq!(
            export_file_name("2026-09-01", "..\\..\\Windows\\system32"),
            "2026-09-01-windows-system32.md"
        );
        assert_eq!(
            export_file_name("2026-09-01", "/etc/passwd"),
            "2026-09-01-etc-passwd.md"
        );
    }

    #[test]
    fn slashes_colons_backslashes_emoji_newlines_and_control_characters_all_fall_into_one_name() {
        let hostile = [
            "a/b",
            "a\\b",
            "a:b",
            "a*b",
            "a?b",
            "a\"b",
            "a<b",
            "a>b",
            "a|b",
            "a\nb",
            "a\r\nb",
            "a\tb",
            "a\u{0}b",
            "a\u{7}b",
            "a🎯b",
            "a\u{200b}b",
        ];

        for title in hostile {
            let name = export_file_name("2026-09-01", title);

            assert_is_a_safe_file_name(&name);
            assert_eq!(name, "2026-09-01-a-b.md", "title: {title:?}");
        }
    }

    #[test]
    fn a_title_with_nothing_to_keep_becomes_untitled() {
        for title in [
            "",
            " ",
            "   \t\r\n ",
            "///",
            "...",
            "..",
            ".",
            "🎯🎯🎯",
            "\u{0}\u{1}\u{2}",
            "-",
            "---",
            "#$%^&*()",
            "\u{301}\u{301}", // 결합문자만 남은 조각
        ] {
            let name = export_file_name("2026-09-01", title);

            assert_eq!(name, "2026-09-01-untitled.md", "title: {title:?}");
            assert_is_a_safe_file_name(&name);
        }
    }

    #[test]
    fn every_hostile_title_still_produces_one_safe_name() {
        // 위 표들이 값까지 고정하는 것과 별개로, 이 성질은 어떤 입력에도 성립해야 한다.
        let long_hangul = "가".repeat(500);
        let long_ascii = "a".repeat(500);

        for title in [
            "../../../../etc/shadow",
            "C:\\Windows\\System32\\drivers\\etc\\hosts",
            "제목\n\n두 번째 줄",
            "  .hidden  ",
            "....md",
            "🎯 회의: 로드맵 / Q4 🎯",
            "con",
            "\u{0}",
            long_hangul.as_str(),
            long_ascii.as_str(),
        ] {
            assert_is_a_safe_file_name(&export_file_name("2026-09-01T10:00:00Z", title));
            assert_is_a_safe_file_name(&export_file_name("", title));
        }
    }

    // ── 길이 ─────────────────────────────────────────────────────────────────────

    #[test]
    fn an_overlong_title_is_cut_at_a_character_boundary() {
        // 한 글자가 3바이트인 제목이 기본이다 — 바이트로만 자르면 깨진다.
        let slug = slug(&"가".repeat(300));

        assert!(slug.len() <= MAX_SLUG_BYTES, "{} 바이트다", slug.len());
        assert_eq!(slug.chars().count(), MAX_SLUG_BYTES / 3, "26글자가 들어간다");
        assert!(slug.chars().all(|character| character == '가'));

        let ascii = super::slug(&"a".repeat(300));
        assert_eq!(ascii.len(), MAX_SLUG_BYTES);
    }

    #[test]
    fn a_cut_never_leaves_a_dangling_separator() {
        // 80바이트 자리가 정확히 `-` 다음이 되도록 만든 제목이다.
        let title = format!("{} {}", "a".repeat(MAX_SLUG_BYTES - 1), "b".repeat(10));

        let slug = slug(&title);

        assert!(!slug.ends_with(SEPARATOR), "끝에 `-`가 남았다: {slug}");
        assert_eq!(slug, "a".repeat(MAX_SLUG_BYTES - 1));
    }

    // ── Windows 예약 이름 ────────────────────────────────────────────────────────

    #[test]
    fn a_windows_reserved_name_gets_a_suffix_even_on_this_platform() {
        // 파일 이름은 만들어지고 나면 남는다. 규칙을 지금 한 번만 정한다 (§3.1).
        assert_eq!(export_file_name("2026-09-01", "CON"), "2026-09-01-con-file.md");
        assert_eq!(export_file_name("2026-09-01", "nul"), "2026-09-01-nul-file.md");
        assert_eq!(export_file_name("2026-09-01", "com1"), "2026-09-01-com1-file.md");
        assert_eq!(export_file_name("2026-09-01", "LPT9"), "2026-09-01-lpt9-file.md");
        assert_eq!(export_file_name("2026-09-01", "aux."), "2026-09-01-aux-file.md");
    }

    #[test]
    fn a_name_that_merely_starts_with_a_reserved_word_is_left_alone() {
        assert_eq!(export_file_name("2026-09-01", "console"), "2026-09-01-console.md");
        assert_eq!(export_file_name("2026-09-01", "com10"), "2026-09-01-com10.md");
        assert_eq!(export_file_name("2026-09-01", "회의 con"), "2026-09-01-회의-con.md");
    }

    // ── 날짜 ─────────────────────────────────────────────────────────────────────

    #[test]
    fn the_date_is_the_stored_text_cut_not_a_computed_local_date() {
        assert_eq!(
            export_file_name("2026-09-01T23:59:59.999Z", "밤 녹음"),
            "2026-09-01-밤-녹음.md",
            "시간대를 계산하지 않는다 (ADR-0009 §4.2)"
        );
    }

    #[test]
    fn a_created_at_of_an_unexpected_shape_becomes_unknown_date() {
        for created_at in ["", "2026", "not-a-date", "2026/09/01", "가나다라마바사아자차"] {
            assert_eq!(
                export_file_name(created_at, "회의"),
                "unknown-date-회의.md",
                "created_at: {created_at:?}"
            );
        }
    }

    // ── 결정성 ───────────────────────────────────────────────────────────────────

    #[test]
    fn the_same_recording_always_gets_the_same_name() {
        // 시계도 난수도 파일시스템도 보지 않는다 (§18).
        let long_hangul = "가".repeat(300);

        for (created_at, title) in [
            ("2026-09-01T10:00:00.000Z", "3DGS Study #04"),
            ("2026-09-01", "회의: 로드맵 / Q4 🎯"),
            ("", "///"),
            ("2026-09-01", long_hangul.as_str()),
        ] {
            assert_eq!(
                export_file_name(created_at, title),
                export_file_name(created_at, title)
            );
        }
    }

    #[test]
    fn hangul_survives_because_this_products_titles_are_korean() {
        // 한글을 버리면 대부분의 제목이 빈 슬러그가 된다 (ADR-0009 §4.2의 1).
        assert_eq!(
            export_file_name("2026-09-01", "3DGS 스터디 04회차"),
            "2026-09-01-3dgs-스터디-04회차.md"
        );
    }

    #[test]
    fn unicode_case_folding_does_not_happen() {
        // 규칙 4는 ASCII 대문자뿐이다 — 유니코드 케이스 규칙이 개입하면 같은 제목이 로캘에
        // 따라 다른 이름을 만든다. 그래서 아래 두 글자는 **내려가지 않는다.**
        assert_eq!(slug("İSTANBUL"), "İstanbul");
        assert_eq!(slug("ÄÖÜ"), "ÄÖÜ");
    }
}
