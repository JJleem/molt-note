//! 소스 자체에 대한 구조적 검사.
//!
//! 행동 테스트(`domain_model.rs`)는 "지금 있는 코드가 규칙을 지키는가"를 본다.
//! 이 파일은 "규칙을 어기는 코드가 **들어오지 못하게**" 한다 — 나중에 누군가
//! Transcript를 갱신하는 함수나 벤더 이름을 domain에 추가하면 여기서 실패한다.
//!
//! 검사 대상 소스는 아래 세 파일이고, 이 테스트 파일 자신은 대상이 아니다
//! (그래서 금지 문자열을 여기 그대로 적어도 자기 자신에 걸리지 않는다).

const DOMAIN_SOURCE: &str = include_str!("../src/domain/mod.rs");
const STORE_SOURCE: &str = include_str!("../src/db/store.rs");
const MIGRATIONS_SOURCE: &str = include_str!("../src/db/migrations.rs");

/// 공백을 하나로 줄이고 대문자로 맞춘다. 줄바꿈이나 들여쓰기로 검사를 피해 갈 수 없게 한다.
fn normalized(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase()
}

#[test]
fn no_code_path_updates_or_deletes_an_existing_transcript_row() {
    // §7.1 · INV-2: Transcript는 immutable이다. 재전사는 새 행을 추가하는 행위이지
    // 기존 행을 고치는 행위가 아니다.
    let forbidden = [
        "UPDATE TRANSCRIPTS",
        "UPDATE TRANSCRIPT_SEGMENTS",
        "DELETE FROM TRANSCRIPTS",
        "DELETE FROM TRANSCRIPT_SEGMENTS",
        "REPLACE INTO TRANSCRIPTS",
        "REPLACE INTO TRANSCRIPT_SEGMENTS",
        "DROP TABLE TRANSCRIPTS",
    ];

    for (label, source) in [
        ("domain/mod.rs", DOMAIN_SOURCE),
        ("db/store.rs", STORE_SOURCE),
        ("db/migrations.rs", MIGRATIONS_SOURCE),
    ] {
        let text = normalized(source);
        for statement in forbidden {
            assert!(
                !text.contains(statement),
                "{label}에 Transcript를 변경하는 문장이 있다: {statement}"
            );
        }
    }
}

#[test]
fn the_store_exposes_no_api_for_changing_a_stored_transcript() {
    // SQL만 막으면 충분하지 않다 — 공개 API 이름에도 갱신 경로가 없어야 한다.
    // `set_current_transcript`는 Recording을 바꾸는 것이므로 여기서 걸리지 않는다.
    for line in STORE_SOURCE.lines() {
        let Some(rest) = line.trim().strip_prefix("pub fn ") else {
            continue;
        };
        let name = rest.split('(').next().unwrap_or_default();
        let changes_a_transcript = name.contains("transcript")
            && ["update", "replace", "overwrite", "delete", "edit"]
                .iter()
                .any(|verb| name.contains(verb));
        assert!(
            !changes_a_transcript,
            "Transcript를 갱신하는 것으로 보이는 공개 API가 있다: {name}"
        );
    }

    // 추가 경로는 하나뿐이어야 한다.
    assert!(
        STORE_SOURCE.contains("pub fn append_transcript"),
        "Transcript를 추가하는 API가 있어야 한다"
    );
}

#[test]
fn the_domain_does_not_know_any_ai_vendor() {
    // INV-9: core/domain은 특정 AI 벤더 타입에 의존하지 않는다. 벤더 지식은 adapter 안에만 있다.
    let vendors = [
        "CLAUDE",
        "ANTHROPIC",
        "GEMINI",
        "OPENAI",
        "GPT-",
        "OLLAMA",
        "GROQ",
        "MISTRAL",
        "LLAMA",
    ];

    for (label, source) in [
        ("domain/mod.rs", DOMAIN_SOURCE),
        ("db/store.rs", STORE_SOURCE),
        ("db/migrations.rs", MIGRATIONS_SOURCE),
    ] {
        let text = source.to_uppercase();
        for vendor in vendors {
            assert!(
                !text.contains(vendor),
                "{label}에 AI 벤더 지식이 들어왔다: {vendor} (INV-9)"
            );
        }
    }
}

#[test]
fn the_ai_note_provider_is_a_free_form_identifier_not_a_vendor_enum() {
    assert!(
        DOMAIN_SOURCE.contains("pub provider: String"),
        "provider는 벤더 enum이 아니라 자유 문자열 식별자여야 한다 (INV-9)"
    );
    // 스키마도 provider 값을 특정 목록으로 제한하지 않는다.
    let schema = normalized(MIGRATIONS_SOURCE);
    assert!(
        !schema.contains("CHECK (PROVIDER IN"),
        "스키마가 provider 값을 목록으로 제한하고 있다 (INV-9)"
    );
}

#[test]
fn the_domain_module_does_not_depend_on_the_storage_layer() {
    // domain 타입이 rusqlite를 알면 §7 규칙이 저장 기술에 묶인다 (ADR-0001의 경계).
    assert!(
        !DOMAIN_SOURCE.contains("rusqlite"),
        "domain이 저장 기술에 의존하고 있다"
    );
}
