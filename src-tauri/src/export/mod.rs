//! 기록을 앱 밖으로 꺼내는 경계 (PRODUCT-SPEC §11 · `docs/ADR-0009-notion-and-export.md`).
//!
//! ```text
//! markdown.rs   Recording + Transcript + (선택) Structured Note → 결정론적 Markdown 문자열
//! filename.rs   Recording의 created_at + title → 결정론적이고 안전한 파일 이름 하나
//! file.rs       (디렉터리 · 이름 · 문자열) → 실제로 쓰인 파일 하나 (덮어쓰지 않는다)
//! run.rs        저장소에서 읽어 위 셋을 잇는 실행 순서
//! ```
//!
//! **파일이 있는 자리는 [`file`] 하나다.** [`markdown`]과 [`filename`]은 값에서 값을 만들 뿐이라
//! `std::fs`도, 저장소도, 시계도, 네트워크도 쓰지 않는다 — 그래서 §11의 산출물과 파일 이름은
//! 실제 파일 하나 만들지 않고 전부 검증된다 (§18 · `phase-prompt/05` 요구 4). 그 둘을 저장소·
//! 파일시스템과 잇는 일은 [`run`]의 몫이며, **그래서 렌더링 규칙과 쓰기 정책이 서로를 모른다.**
//!
//! ```text
//! Recording · Transcript · StructuredNote ─→ markdown::render ─→ String ─┬─→ file::write_new
//!                                                                        └─→ Notion 본문 (경계 밖)
//! Recording.created_at · Recording.title ──→ filename::export_file_name ─→ "2026-09-01-….md"
//! ```
//!
//! **Notion으로 가는 경로는 여전히 이 경계 밖이다** — 같은 [`markdown::render`] 결과를 쓰기
//! 때문에 산출물은 하나로 유지된다 (ADR-0009 §14).
//!
//! **벤더가 없다** (INV-9). 렌더러가 소비하는 것은 §9.3의 provider 중립 [`StructuredNote`]
//! ([`crate::ai::note`])이며, 어떤 제공자의 원시 응답도 고유 필드도 여기에 닿지 않는다.
//!
//! **AI가 없어도 성립한다** (INV-8 · §17.1). Structured Note는 선택 입력이고, 없는 입력도
//! Transcript와 메타데이터만으로 읽을 수 있는 문서를 낸다. 없는 것의 빈 자리를 남기지 않는다.
//!
//! [`StructuredNote`]: crate::ai::note::StructuredNote

pub mod file;
pub mod filename;
pub mod markdown;
pub mod run;

pub use file::{write_new, WrittenFile};
pub use run::export;

pub use filename::{
    export_file_name, slug, MARKDOWN_EXTENSION, MAX_SLUG_BYTES, UNKNOWN_DATE, UNTITLED_SLUG,
};
pub use markdown::{
    format_timestamp_ms, render, ExportDocument, MEETING_SECTIONS, STUDY_SECTIONS,
    SUMMARY_SECTIONS, TRANSCRIPT_SECTION, UNTITLED_TITLE,
};

/// 저장된 시각 텍스트의 앞 10글자가 `YYYY-MM-DD` 모양이면 그것.
///
/// 저장소가 만든 UTC 텍스트를 **그대로 잘라 쓴다. 시간대를 계산하지 않는다** — 계산하는 순간
/// 같은 Recording이 기기 설정에 따라 다른 이름과 다른 `Date:` 줄을 갖게 되어 "결정론적"이라는
/// 성질 자체가 깨진다 (ADR-0009 §4.2). 그 대가로 현지 시각으로 늦은 밤에 녹음한 것은 날짜가
/// 하루 뒤일 수 있다.
///
/// 모양이 예상과 다르면 **날짜를 지어내지 않고** `None`이다. 무엇을 대신 쓸지는 부르는 쪽이
/// 정한다 — 파일 이름은 [`filename::UNKNOWN_DATE`], 문서의 `Date:` 줄은 받은 값 그대로다
/// ([`markdown`]).
///
/// 문자 경계가 아닌 자리에서 자르지 않는다 — `get`이 `None`을 돌려주므로 panic하지 않는다.
fn iso_date(created_at: &str) -> Option<&str> {
    let candidate = created_at.get(0..10)?;
    let shaped = candidate
        .as_bytes()
        .iter()
        .enumerate()
        .all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        });

    shaped.then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_utc_text_gives_its_first_ten_characters() {
        assert_eq!(iso_date("2026-09-01T10:00:00.000Z"), Some("2026-09-01"));
        assert_eq!(iso_date("2026-09-01"), Some("2026-09-01"));
    }

    #[test]
    fn a_shape_this_code_did_not_expect_is_not_guessed_at() {
        for created_at in [
            "",
            "2026",
            "2026-09-0",
            "2026/09/01",
            "20260901T1000",
            "not-a-date",
            "가나다라마바사아자차",
        ] {
            assert_eq!(iso_date(created_at), None, "created_at: {created_at}");
        }
    }

    #[test]
    fn a_multibyte_prefix_does_not_panic_on_a_character_boundary() {
        // `get`은 경계가 아니면 `None`이다 — 여기서 앱이 끝나지 않는다.
        assert_eq!(iso_date("가나다라"), None);
    }
}
