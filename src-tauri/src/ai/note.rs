//! 세 mode의 structured note — 타입 · 저장 형태 · 요구할 schema · **기대와 다른 응답에 대한 방어**
//! (ADR-0008 §6.3 · §7).
//!
//! ## mode를 세는 타입은 하나다
//!
//! ADR-0008 §4.2(표 5행)가 확정했다 — **같은 세 값을 세는 타입을 두 개 두지 않는다.**
//! 그래서 여기서 말하는 "note mode"는 이미 저장·복원까지 왕복하는 [`NoteType`]이며
//! (`crate::domain`), 이 모듈은 그것을 다시 정의하지 않고 그대로 쓴다.
//!
//! ## 필드 이름은 §9.5의 출력 섹션 이름이다
//!
//! ADR-0008 §7.1의 규칙 하나를 그대로 옮겼다 — 필드 이름은 §9.5 표의 출력 섹션 이름을
//! camelCase로 옮긴 것이다. 렌더러(UI · Markdown · 외부 내보내기)가 만들 제목이 곧 그
//! 섹션 이름이므로, 이름이 섹션과 1:1이면 매핑 표를 따로 들지 않아도 된다.
//! **§9.5의 13개 출력 섹션이 13개 필드에 1:1로 대응한다.**
//!
//! 타입은 두 가지뿐이다 — `string`과 `string[]`. 중첩 객체를 쓰지 않는다 (ADR-0008 §7.3).
//!
//! ## 응답은 신뢰하지 않는 텍스트다
//!
//! 구조화 출력을 요구하는 수단([`json_schema`])을 쓰더라도 **정확성은 그것에 걸지 않는다**
//! (ADR-0008 §6.1). 기대와 다른 응답은 예외가 아니라 **기본 경로**이며, [`parse_note`]가
//! 그 경로다. 어떤 입력에서도 `panic`하지 않고, 회수할 수 있으면 회수하고, 아니면
//! [`ResponseRejection`]("응답이 기대 schema와 다름")으로 끝난다.
//!
//! ```text
//! 생성된 텍스트
//!   1. 그대로 JSON이면 그것을 쓴다
//!   2. 아니면 **딱 한 번** 회수한다 — 코드펜스를 벗기고 바깥쪽 균형 잡힌 {...} 하나를 취한다
//!      그 이상은 하지 않는다: 필드 추측 · 정규식 짜맞추기 · 재요청은 없다
//!   3. mode별 struct로 역직렬화 — **모르는 필드는 무시한다**, 필수 필드가 없으면 거절
//!   4. domain 규칙 검증 (§7.4) — 필수 문자열이 비면 거절, 빈 배열 원소는 버린다,
//!      크기 상한을 넘으면 **잘라내지 않고** 거절
//! ```
//!
//! [`ResponseRejection`]을 §13의 domain 공통 실패로 옮기는 것은 이 모듈의 일이 아니다
//! (ADR-0008 §13.1) — 여기에는 화면도, 실패 문자열 계약도, 제공자도 없다.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::domain::{Failure, FailureKind, NoteType};

/// `ai_notes.content` 봉투의 형식 번호 (ADR-0008 §7.5).
///
/// **저장된 노트는 schema보다 오래 산다.** `ai_notes`에는 UPDATE 경로가 없으므로 옛 노트를
/// 새 형태로 고쳐 쓸 수 없고, 고쳐 쓰는 것은 provenance를 지우는 일이다. 읽는 쪽이 형태를
/// 판정할 값이 봉투 안에 있어야 한다.
pub const CONTENT_SCHEMA_VERSION: u32 = 1;

/// 배열 하나가 가질 수 있는 원소 수의 상한 (ADR-0008 §7.4).
pub const MAX_ITEMS: usize = 200;

/// 배열 원소 하나의 길이 상한(문자 수) (ADR-0008 §7.4).
///
/// 외부 내보내기의 텍스트 한도와 같은 값이다 — 나중에 다시 자르지 않아도 되도록 맞췄다.
pub const MAX_ITEM_CHARS: usize = 2_000;

/// `overview` · `shortSummary`의 길이 상한(문자 수) (ADR-0008 §7.4).
pub const MAX_OVERVIEW_CHARS: usize = 4_000;

/// Meeting 노트의 필드 이름 — §9.5의 출력 섹션 순서 그대로다.
///
/// 이 배열은 [`json_schema`]의 `required`와 직렬화 키 순서 **양쪽의 근거**이며, 테스트가
/// 셋이 어긋나지 않는 것을 고정한다.
pub const MEETING_FIELDS: [&str; 5] = [
    "overview",
    "keyDiscussions",
    "decisions",
    "actionItems",
    "openQuestions",
];

/// Study 노트의 필드 이름 — §9.5의 출력 섹션 순서 그대로다.
pub const STUDY_FIELDS: [&str; 6] = [
    "overview",
    "keyConcepts",
    "importantDetails",
    "questions",
    "thingsToStudy",
    "referencesMentioned",
];

/// Summary 노트의 필드 이름 — §9.5의 출력 섹션 순서 그대로다.
pub const SUMMARY_FIELDS: [&str; 2] = ["shortSummary", "keyPoints"];

/// Meeting 노트 (§9.5: Overview · Key Discussions · Decisions · Action Items · Open Questions).
///
/// **키는 언제나 있고, 배열은 비어도 된다.** "결정된 것이 없었다"는 실제 결과이며, 배열을
/// 채우도록 강요하면 모델이 없는 결정을 지어낸다 (ADR-0008 §7.3).
///
/// `actionItems`가 `{text, owner, dueDate}`가 아니라 `string[]`인 이유도 같은 절에 있다 —
/// 담당자·기한 추출 능력은 UNVERIFIED이며, 확인되지 않은 능력 위에 스키마를 세우지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingNote {
    /// 필수. trim 후 비어 있으면 무효다 — 요약이 없는 요약 노트는 노트가 아니다.
    pub overview: String,
    pub key_discussions: Vec<String>,
    pub decisions: Vec<String>,
    pub action_items: Vec<String>,
    pub open_questions: Vec<String>,
}

/// Study 노트
/// (§9.5: Overview · Key Concepts · Important Details · Questions · Things to Study · References Mentioned).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyNote {
    /// 필수. trim 후 비어 있으면 무효다.
    pub overview: String,
    pub key_concepts: Vec<String>,
    pub important_details: Vec<String>,
    pub questions: Vec<String>,
    pub things_to_study: Vec<String>,
    /// **"언급된" 참고자료다.** 이름의 제한이 남아 있어야 없는 참고문헌을 지어내지 않는다
    /// (ADR-0008 §7.1).
    pub references_mentioned: Vec<String>,
}

/// Summary 노트 (§9.5: Short Summary · Key Points).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryNote {
    /// 필수. trim 후 비어 있으면 무효다.
    pub short_summary: String,
    pub key_points: Vec<String>,
}

/// 세 mode 중 하나의 노트.
///
/// **모드마다 다른 struct다** — 하나의 옵셔널 필드 뭉치로 합치면 "Meeting에 `thingsToStudy`가
/// 있는" 상태가 타입 수준에서 가능해진다 (ADR-0008 §7.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuredNote {
    Meeting(MeetingNote),
    Study(StudyNote),
    Summary(SummaryNote),
}

impl StructuredNote {
    /// 이 노트가 어느 mode의 것인가.
    pub fn mode(&self) -> NoteType {
        match self {
            Self::Meeting(_) => NoteType::Meeting,
            Self::Study(_) => NoteType::Study,
            Self::Summary(_) => NoteType::Summary,
        }
    }
}

/// 응답이 **기대 schema와 다르다**는 것 하나를 뜻하는 값.
///
/// 변형은 "무엇이 달랐는가"를 남기기 위한 것이지 서로 다른 제품 상태가 아니다 — 이 값이
/// 도달하는 곳은 §13의 실패 **하나**이며, 그 번역은 계약을 만드는 쪽이 한다 (ADR-0008 §13.1).
/// 그래서 여기에는 사용자에게 보여줄 문장도, frontend와의 문자열 계약도 없다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseRejection {
    /// 본문이 비어 있거나 공백뿐이었다.
    Empty,
    /// JSON 객체를 찾지 못했다 — 산문이거나, 코드펜스를 벗겨도 균형 잡힌 `{...}`가 없었다.
    NotJson,
    /// JSON은 읽었지만 이 mode의 노트로 해석할 수 없었다 (필드 누락 · 타입 불일치 등).
    Shape {
        /// 기술적 원인. 사용자에게 그대로 보여줄 문장이 아니다.
        detail: String,
    },
    /// 필수 문자열이 trim 후 비어 있었다.
    BlankRequiredText {
        /// 어느 필드인가. §9.5의 섹션 이름과 같다.
        field: &'static str,
    },
    /// 배열 원소가 상한([`MAX_ITEMS`])을 넘었다. **잘라내지 않는다.**
    TooManyItems { field: &'static str, count: usize },
    /// 문자열이 상한을 넘었다. **잘라내지 않는다** — 조용히 자른 노트는 완전해 보이지만
    /// 완전하지 않다 (ADR-0008 §7.4).
    TextTooLong {
        field: &'static str,
        chars: usize,
        limit: usize,
    },
}

impl std::fmt::Display for ResponseRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => f.write_str("응답 본문이 비어 있다"),
            Self::NotJson => f.write_str("응답 본문에서 JSON 객체를 찾지 못했다"),
            Self::Shape { detail } => write!(f, "응답을 이 모드의 노트로 읽을 수 없다: {detail}"),
            Self::BlankRequiredText { field } => write!(f, "필수 항목이 비어 있다: {field}"),
            Self::TooManyItems { field, count } => {
                write!(f, "{field}의 항목이 너무 많다: {count} > {MAX_ITEMS}")
            }
            Self::TextTooLong {
                field,
                chars,
                limit,
            } => write!(f, "{field}의 길이가 상한을 넘었다: {chars} > {limit}"),
        }
    }
}

/// provider에게 **요구할** 출력 형태를 JSON Schema로 적은 것 (ADR-0008 §6.1).
///
/// 이것은 schema 그 자체이며 **전송 형식이 아니다.** 어느 제공자의 파라미터 이름에 어떻게
/// 실리는지는 이 모듈이 알지 않는다 — 그 지식은 adapter 하나에만 있다 (INV-9).
///
/// 이 값에 정확성을 걸지 않는다. 서버가 무시하거나 거절해도 [`parse_note`]가 같은 답을 낸다
/// (ADR-0008 §6.2).
pub fn json_schema(mode: NoteType) -> Value {
    let text = || json!({ "type": "string" });
    let list = || json!({ "type": "array", "items": { "type": "string" } });

    let (properties, required): (Value, &[&str]) = match mode {
        NoteType::Meeting => (
            json!({
                "overview": text(),
                "keyDiscussions": list(),
                "decisions": list(),
                "actionItems": list(),
                "openQuestions": list(),
            }),
            &MEETING_FIELDS,
        ),
        NoteType::Study => (
            json!({
                "overview": text(),
                "keyConcepts": list(),
                "importantDetails": list(),
                "questions": list(),
                "thingsToStudy": list(),
                "referencesMentioned": list(),
            }),
            &STUDY_FIELDS,
        ),
        NoteType::Summary => (
            json!({
                "shortSummary": text(),
                "keyPoints": list(),
            }),
            &SUMMARY_FIELDS,
        ),
    };

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

/// 생성된 텍스트를 그 mode의 노트로 읽는다 — **이 모듈의 기본 경로다.**
///
/// 응답 본문의 봉투에서 생성 텍스트를 꺼내는 일(ADR-0008 §6.3의 1단계)은 제공자마다 다르므로
/// adapter가 하고, 이 함수는 그 **텍스트**를 받는다. 여기서부터는 제공자가 누구든 같다.
///
/// 어떤 입력에서도 `panic`하지 않는다. 응답 본문에 대해 `unwrap`/`expect`를 쓰지 않는다.
pub fn parse_note(mode: NoteType, generated_text: &str) -> Result<StructuredNote, ResponseRejection> {
    let value = extract_json_object(generated_text)?;

    match mode {
        NoteType::Meeting => {
            let raw: MeetingNote = from_value(value)?;
            Ok(StructuredNote::Meeting(MeetingNote {
                overview: clean_required_text("overview", &raw.overview, MAX_OVERVIEW_CHARS)?,
                key_discussions: clean_items("keyDiscussions", raw.key_discussions)?,
                decisions: clean_items("decisions", raw.decisions)?,
                action_items: clean_items("actionItems", raw.action_items)?,
                open_questions: clean_items("openQuestions", raw.open_questions)?,
            }))
        }
        NoteType::Study => {
            let raw: StudyNote = from_value(value)?;
            Ok(StructuredNote::Study(StudyNote {
                overview: clean_required_text("overview", &raw.overview, MAX_OVERVIEW_CHARS)?,
                key_concepts: clean_items("keyConcepts", raw.key_concepts)?,
                important_details: clean_items("importantDetails", raw.important_details)?,
                questions: clean_items("questions", raw.questions)?,
                things_to_study: clean_items("thingsToStudy", raw.things_to_study)?,
                references_mentioned: clean_items("referencesMentioned", raw.references_mentioned)?,
            }))
        }
        NoteType::Summary => {
            let raw: SummaryNote = from_value(value)?;
            Ok(StructuredNote::Summary(SummaryNote {
                short_summary: clean_required_text(
                    "shortSummary",
                    &raw.short_summary,
                    MAX_OVERVIEW_CHARS,
                )?,
                key_points: clean_items("keyPoints", raw.key_points)?,
            }))
        }
    }
}

/// `ai_notes.content`에 담을 문자열 — `{schemaVersion, mode, note}` 봉투의 compact JSON
/// **한 줄**이다 (ADR-0008 §7.5).
///
/// 키 순서는 §7.2 표의 순서로 고정된다(선언 순서). 같은 노트는 언제나 같은 문자열이 되므로
/// 테스트가 문자열을 그대로 비교할 수 있다.
pub fn encode_content(note: &StructuredNote) -> String {
    // 봉투는 `serde_json::Value`를 거치지 않고 직접 직렬화한다 — Value를 거치면 키 순서가
    // 선언 순서가 아니게 될 수 있고, 그러면 §7.5가 요구한 "같은 노트는 같은 문자열"이
    // 조용히 깨진다.
    let mode = note.mode().as_str();
    let encoded = match note {
        StructuredNote::Meeting(inner) => serde_json::to_string(&Envelope {
            schema_version: CONTENT_SCHEMA_VERSION,
            mode,
            note: inner,
        }),
        StructuredNote::Study(inner) => serde_json::to_string(&Envelope {
            schema_version: CONTENT_SCHEMA_VERSION,
            mode,
            note: inner,
        }),
        StructuredNote::Summary(inner) => serde_json::to_string(&Envelope {
            schema_version: CONTENT_SCHEMA_VERSION,
            mode,
            note: inner,
        }),
    };

    // 여기서 직렬화가 실패할 수 있는 값은 없다 — 필드는 `String`과 `Vec<String>`뿐이며,
    // 이 값들은 사용자 입력이 아니라 이미 검증을 통과한 domain 값이다. 그래도 `unwrap`으로
    // 앱을 끝내지 않는다: 실패하면 스스로를 설명하는 봉투를 남긴다.
    encoded.unwrap_or_else(|_| {
        format!(r#"{{"schemaVersion":{CONTENT_SCHEMA_VERSION},"mode":"{mode}","note":null}}"#)
    })
}

/// 저장된 `ai_notes.content`를 다시 노트로 읽는다.
///
/// `note_type`은 **그 행의 `note_type` 열**이다. 봉투 안의 `mode`와 열이 다르면 어느 한쪽을
/// 추측해 고르지 않고 실패한다 (ADR-0008 §7.5).
///
/// **읽기 실패는 AI 실패가 아니다** — provider와 무관하므로 §13의 AI 실패로 옮기지 않고
/// [`FailureKind::Storage`]로 말한다. 행은 건드리지 않는다: 지우지도, 다시 쓰지도 않는다.
pub fn decode_content(content: &str, note_type: NoteType) -> Result<StructuredNote, Failure> {
    let envelope: OwnedEnvelope = serde_json::from_str(content).map_err(|error| {
        storage_failure("저장된 AI 노트를 읽을 수 없다").with_detail(error)
    })?;

    if envelope.schema_version != CONTENT_SCHEMA_VERSION {
        return Err(storage_failure("이 앱보다 새 버전이 만든 AI 노트다").with_detail(format!(
            "schemaVersion {} (이 앱이 아는 값: {CONTENT_SCHEMA_VERSION})",
            envelope.schema_version
        )));
    }

    let stored_mode = NoteType::parse(&envelope.mode).ok_or_else(|| {
        storage_failure("저장된 AI 노트의 종류를 알 수 없다")
            .with_detail(format!("mode {:?}", envelope.mode))
    })?;
    if stored_mode != note_type {
        return Err(
            storage_failure("저장된 AI 노트의 종류가 기록과 다르다").with_detail(format!(
                "note_type {note_type}, content mode {stored_mode}"
            )),
        );
    }

    let note = match note_type {
        NoteType::Meeting => serde_json::from_value(envelope.note).map(StructuredNote::Meeting),
        NoteType::Study => serde_json::from_value(envelope.note).map(StructuredNote::Study),
        NoteType::Summary => serde_json::from_value(envelope.note).map(StructuredNote::Summary),
    };

    note.map_err(|error| storage_failure("저장된 AI 노트의 형태가 기대와 다르다").with_detail(error))
}

/// 직렬화 전용 봉투. `note`를 빌려 쓰므로 필드 순서가 선언 순서 그대로 나간다.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a, T: Serialize> {
    schema_version: u32,
    mode: &'a str,
    note: &'a T,
}

/// 역직렬화 전용 봉투. 형태 판정은 `note`를 읽기 **전에** 끝난다.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnedEnvelope {
    schema_version: u32,
    mode: String,
    note: Value,
}

fn storage_failure(message: &str) -> Failure {
    // 다시 읽어도 같은 결과다 — 사용자가 재시도로 풀 수 있는 실패가 아니다.
    Failure::permanent(FailureKind::Storage, message)
}

/// 생성된 텍스트에서 JSON 객체 하나를 얻는다 (ADR-0008 §6.3의 2단계).
///
/// 회수는 **딱 한 번**이다 — 코드펜스를 벗기고 바깥쪽 균형 잡힌 `{...}` 하나를 취한다.
/// 필드를 추측하거나 정규식으로 짜맞추지 않는다.
fn extract_json_object(generated_text: &str) -> Result<Value, ResponseRejection> {
    let trimmed = generated_text.trim();
    if trimmed.is_empty() {
        return Err(ResponseRejection::Empty);
    }

    if let Ok(value @ Value::Object(_)) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }

    let unfenced = strip_code_fence(trimmed);
    let candidate = outermost_braced_slice(unfenced).ok_or(ResponseRejection::NotJson)?;
    match serde_json::from_str::<Value>(candidate) {
        Ok(value @ Value::Object(_)) => Ok(value),
        // 객체가 아닌 JSON(배열 · 숫자 · 문자열)은 이 mode의 노트가 될 수 없다.
        Ok(_) | Err(_) => Err(ResponseRejection::NotJson),
    }
}

/// ```` ```json … ``` ```` 같은 코드펜스를 한 겹 벗긴다. 펜스가 없으면 그대로 돌려준다.
fn strip_code_fence(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("```") else {
        return text;
    };
    // 여는 펜스의 나머지(언어 태그)를 줄 단위로 버린다. 줄바꿈이 없으면 벗길 것이 없다.
    let Some((_language_tag, body)) = rest.split_once('\n') else {
        return text;
    };
    match body.rfind("```") {
        Some(end) => body[..end].trim(),
        // 닫는 펜스가 없다 — 잘린 응답이다. 남은 본문에서 계속 찾는다.
        None => body.trim(),
    }
}

/// 첫 `{`에서 시작해 짝이 맞는 `}`까지의 조각. 문자열 리터럴 안의 중괄호는 세지 않는다.
fn outermost_braced_slice(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, character) in text[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..start + offset + character.len_utf8()]);
                }
            }
            _ => {}
        }
    }

    // 닫히지 않았다 — 짜맞추지 않는다.
    None
}

/// 모르는 필드는 무시하고, 없는 필드와 다른 타입은 거절한다 (ADR-0008 §6.3의 3단계).
fn from_value<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, ResponseRejection> {
    serde_json::from_value(value).map_err(|error| ResponseRejection::Shape {
        detail: error.to_string(),
    })
}

/// 필수 문자열의 검증 (ADR-0008 §7.4). 비면 무효이고, 상한을 넘으면 **자르지 않고** 무효다.
fn clean_required_text(
    field: &'static str,
    value: &str,
    limit: usize,
) -> Result<String, ResponseRejection> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ResponseRejection::BlankRequiredText { field });
    }
    let chars = trimmed.chars().count();
    if chars > limit {
        return Err(ResponseRejection::TextTooLong {
            field,
            chars,
            limit,
        });
    }
    Ok(trimmed.to_owned())
}

/// 배열의 검증 (ADR-0008 §7.4). 빈 원소는 **버리고**(실패가 아니다), 상한을 넘으면 무효다.
fn clean_items(
    field: &'static str,
    values: Vec<String>,
) -> Result<Vec<String>, ResponseRejection> {
    let mut cleaned = Vec::with_capacity(values.len());
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        let chars = trimmed.chars().count();
        if chars > MAX_ITEM_CHARS {
            return Err(ResponseRejection::TextTooLong {
                field,
                chars,
                limit: MAX_ITEM_CHARS,
            });
        }
        cleaned.push(trimmed.to_owned());
    }

    if cleaned.len() > MAX_ITEMS {
        return Err(ResponseRejection::TooManyItems {
            field,
            count: cleaned.len(),
        });
    }
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meeting() -> StructuredNote {
        StructuredNote::Meeting(MeetingNote {
            overview: "분기 계획 회의".to_owned(),
            key_discussions: vec!["일정".to_owned(), "예산".to_owned()],
            decisions: vec!["9월 출시".to_owned()],
            action_items: vec!["초안 작성".to_owned()],
            open_questions: vec![],
        })
    }

    fn study() -> StructuredNote {
        StructuredNote::Study(StudyNote {
            overview: "분산 시스템 강의".to_owned(),
            key_concepts: vec!["합의".to_owned()],
            important_details: vec![],
            questions: vec!["왜 과반인가?".to_owned()],
            things_to_study: vec!["Raft".to_owned()],
            references_mentioned: vec![],
        })
    }

    fn summary() -> StructuredNote {
        StructuredNote::Summary(SummaryNote {
            short_summary: "짧은 요약".to_owned(),
            key_points: vec!["요점 하나".to_owned()],
        })
    }

    // ── §7.2의 schema와 §9.5의 출력 섹션 ──────────────────────────────────────────

    #[test]
    fn the_three_modes_carry_every_output_section_of_section_9_5() {
        // §9.5의 13개 출력 섹션이 13개 필드에 1:1로 대응한다 (ADR-0008 §7.2).
        assert_eq!(MEETING_FIELDS.len() + STUDY_FIELDS.len() + SUMMARY_FIELDS.len(), 13);

        let encoded = serde_json::to_value(MeetingNote {
            overview: String::new(),
            key_discussions: vec![],
            decisions: vec![],
            action_items: vec![],
            open_questions: vec![],
        })
        .expect("직렬화할 수 있어야 한다");
        for field in MEETING_FIELDS {
            assert!(encoded.get(field).is_some(), "meeting에 {field}가 없다");
        }

        let encoded = serde_json::to_value(StudyNote {
            overview: String::new(),
            key_concepts: vec![],
            important_details: vec![],
            questions: vec![],
            things_to_study: vec![],
            references_mentioned: vec![],
        })
        .expect("직렬화할 수 있어야 한다");
        for field in STUDY_FIELDS {
            assert!(encoded.get(field).is_some(), "study에 {field}가 없다");
        }

        let encoded = serde_json::to_value(SummaryNote {
            short_summary: String::new(),
            key_points: vec![],
        })
        .expect("직렬화할 수 있어야 한다");
        for field in SUMMARY_FIELDS {
            assert!(encoded.get(field).is_some(), "summary에 {field}가 없다");
        }
    }

    #[test]
    fn the_field_names_are_the_section_names_of_section_9_5() {
        // §7.1이 §9.3 예시에서 바꾼 세 이름. 이것이 틀리면 렌더러의 제목이 섹션과 어긋난다.
        assert!(MEETING_FIELDS.contains(&"keyDiscussions"));
        assert!(MEETING_FIELDS.contains(&"openQuestions"));
        assert!(STUDY_FIELDS.contains(&"referencesMentioned"));
        assert!(!MEETING_FIELDS.contains(&"keyPoints"), "keyPoints는 summary의 이름이다");
    }

    #[test]
    fn the_schema_asks_for_exactly_the_fields_of_the_type() {
        for (mode, fields) in [
            (NoteType::Meeting, MEETING_FIELDS.as_slice()),
            (NoteType::Study, STUDY_FIELDS.as_slice()),
            (NoteType::Summary, SUMMARY_FIELDS.as_slice()),
        ] {
            let schema = json_schema(mode);
            assert_eq!(schema["type"], "object");

            let required: Vec<&str> = schema["required"]
                .as_array()
                .expect("required는 배열이다")
                .iter()
                .map(|value| value.as_str().expect("이름은 문자열이다"))
                .collect();
            assert_eq!(required, fields, "{mode}의 required가 타입과 다르다");

            let properties = schema["properties"].as_object().expect("properties는 객체다");
            assert_eq!(properties.len(), fields.len());
            for field in fields {
                let declared = &properties[*field];
                let kind = declared["type"].as_str().expect("type이 있어야 한다");
                assert!(
                    kind == "string" || kind == "array",
                    "{field}: 타입은 string과 string[] 뿐이다 (§7.2)"
                );
                if kind == "array" {
                    assert_eq!(declared["items"]["type"], "string");
                }
            }
        }
    }

    // ── §7.5 content 봉투 ────────────────────────────────────────────────────────

    #[test]
    fn content_round_trips_through_storage_unchanged() {
        for note in [meeting(), study(), summary()] {
            let content = encode_content(&note);
            let restored = decode_content(&content, note.mode()).expect("다시 읽을 수 있어야 한다");

            assert_eq!(restored, note, "저장했다 읽으면 그대로 돌아온다");
            assert_eq!(encode_content(&restored), content, "다시 담아도 같은 문자열이다");
        }
    }

    #[test]
    fn the_stored_envelope_is_one_compact_line_in_the_declared_order() {
        // ADR-0008 §7.5의 예시 그대로다. 키 순서가 바뀌면 이 비교가 깨진다.
        let content = encode_content(&StructuredNote::Meeting(MeetingNote {
            overview: "...".to_owned(),
            key_discussions: vec![],
            decisions: vec![],
            action_items: vec![],
            open_questions: vec![],
        }));

        assert_eq!(
            content,
            r#"{"schemaVersion":1,"mode":"meeting","note":{"overview":"...","keyDiscussions":[],"decisions":[],"actionItems":[],"openQuestions":[]}}"#
        );
        assert!(!content.contains('\n'), "한 줄이다");
    }

    #[test]
    fn a_note_from_a_newer_schema_version_is_not_guessed_at() {
        let content = r#"{"schemaVersion":2,"mode":"summary","note":{"shortSummary":"x","keyPoints":[]}}"#;

        let failure = decode_content(content, NoteType::Summary).expect_err("읽을 수 없어야 한다");

        assert_eq!(failure.kind, FailureKind::Storage, "AI 실패가 아니다");
        assert!(!failure.retryable);
        assert!(failure.source_data_safe, "행을 건드리지 않았다");
    }

    #[test]
    fn a_mode_that_disagrees_with_the_column_is_a_storage_failure() {
        let content = encode_content(&summary());

        let failure = decode_content(&content, NoteType::Meeting).expect_err("읽을 수 없어야 한다");

        assert_eq!(failure.kind, FailureKind::Storage);
    }

    #[test]
    fn unreadable_content_never_panics() {
        for content in [
            "",
            "   ",
            "not json",
            "{}",
            r#"{"schemaVersion":1}"#,
            r#"{"schemaVersion":"1","mode":"meeting","note":{}}"#,
            r#"{"schemaVersion":1,"mode":"lecture","note":{}}"#,
            r#"{"schemaVersion":1,"mode":"meeting","note":{"overview":1}}"#,
            r#"{"schemaVersion":1,"mode":"meeting","note":null}"#,
        ] {
            let failure =
                decode_content(content, NoteType::Meeting).expect_err("읽을 수 없는 content다");
            assert_eq!(failure.kind, FailureKind::Storage, "content: {content}");
        }
    }

    // ── §6.3 방어 경로 ───────────────────────────────────────────────────────────

    #[test]
    fn a_well_formed_response_becomes_a_note() {
        let body = r#"{"overview":" 분기 계획 ","keyDiscussions":["일정"],"decisions":[],"actionItems":[],"openQuestions":[]}"#;

        let note = parse_note(NoteType::Meeting, body).expect("읽을 수 있어야 한다");

        let StructuredNote::Meeting(meeting) = note else {
            panic!("meeting 노트여야 한다");
        };
        assert_eq!(meeting.overview, "분기 계획", "앞뒤 공백은 잘라낸다");
        assert_eq!(meeting.key_discussions, ["일정"]);
    }

    #[test]
    fn a_missing_field_is_rejected_rather_than_filled_in() {
        // (1) 필드 누락
        let body = r#"{"overview":"x","keyDiscussions":[],"decisions":[],"actionItems":[]}"#;

        let rejection = parse_note(NoteType::Meeting, body).expect_err("거절해야 한다");

        assert!(matches!(rejection, ResponseRejection::Shape { .. }));
        assert!(rejection.to_string().contains("openQuestions"));
    }

    #[test]
    fn a_field_of_the_wrong_type_is_rejected() {
        // (2) 타입 불일치 — 문자열 자리에 숫자
        let body = r#"{"overview":42,"keyDiscussions":[],"decisions":[],"actionItems":[],"openQuestions":[]}"#;

        assert!(matches!(
            parse_note(NoteType::Meeting, body),
            Err(ResponseRejection::Shape { .. })
        ));
    }

    #[test]
    fn a_string_where_an_array_belongs_is_rejected() {
        // (3) 배열 자리에 문자열 — 한 줄짜리 답을 배열로 고쳐 읽지 않는다
        let body = r#"{"overview":"x","keyDiscussions":"일정 이야기","decisions":[],"actionItems":[],"openQuestions":[]}"#;

        assert!(matches!(
            parse_note(NoteType::Meeting, body),
            Err(ResponseRejection::Shape { .. })
        ));
    }

    #[test]
    fn unknown_extra_fields_do_not_break_the_note() {
        // (4) 알 수 없는 추가 필드 — 모델이 덧붙인 필드 때문에 실패하지 않는다
        let body = r#"{"overview":"x","keyDiscussions":[],"decisions":[],"actionItems":[],"openQuestions":[],"confidence":0.9,"notes":{"a":[1,2]}}"#;

        let note = parse_note(NoteType::Meeting, body).expect("무시하고 읽어야 한다");

        assert_eq!(note.mode(), NoteType::Meeting);
    }

    #[test]
    fn prose_that_is_not_json_is_rejected() {
        // (5) JSON이 아닌 산문
        for body in [
            "죄송합니다. 이 전사를 요약할 수 없습니다.",
            "Here is your note!",
            "overview: 분기 계획\nkeyDiscussions: 일정",
        ] {
            assert_eq!(
                parse_note(NoteType::Meeting, body),
                Err(ResponseRejection::NotJson),
                "body: {body}"
            );
        }
    }

    #[test]
    fn json_wrapped_in_a_code_fence_is_recovered() {
        // (6) 코드펜스로 감싼 JSON — 한 번만 회수한다
        let fenced = "여기 있습니다:\n```json\n{\"shortSummary\":\"짧은 요약\",\"keyPoints\":[\"하나\"]}\n```\n도움이 되었길 바랍니다.";

        let note = parse_note(NoteType::Summary, fenced).expect("회수할 수 있어야 한다");

        assert_eq!(
            note,
            StructuredNote::Summary(SummaryNote {
                short_summary: "짧은 요약".to_owned(),
                key_points: vec!["하나".to_owned()],
            })
        );
    }

    #[test]
    fn an_empty_body_is_rejected() {
        // (7) 빈 본문
        for body in ["", " ", "\n\n", "\t  \r\n"] {
            assert_eq!(
                parse_note(NoteType::Summary, body),
                Err(ResponseRejection::Empty)
            );
        }
    }

    #[test]
    fn a_blank_required_text_is_not_a_usable_note() {
        // (8) 요약이 없는 요약 노트는 노트가 아니다 (§7.3)
        let body = r#"{"shortSummary":"   ","keyPoints":["하나"]}"#;

        assert_eq!(
            parse_note(NoteType::Summary, body),
            Err(ResponseRejection::BlankRequiredText {
                field: "shortSummary"
            })
        );
    }

    #[test]
    fn blank_array_items_are_dropped_but_an_empty_array_is_normal() {
        // (9) 빈 원소는 버린다. 배열이 비는 것은 정상이다.
        let body = r#"{"shortSummary":"x","keyPoints":["하나","  ","","둘"]}"#;

        let note = parse_note(NoteType::Summary, body).expect("읽을 수 있어야 한다");

        assert_eq!(
            note,
            StructuredNote::Summary(SummaryNote {
                short_summary: "x".to_owned(),
                key_points: vec!["하나".to_owned(), "둘".to_owned()],
            })
        );
    }

    #[test]
    fn oversized_output_is_rejected_rather_than_silently_truncated() {
        // (10) 크기 상한 — 조용히 자른 노트는 완전해 보이지만 완전하지 않다 (§7.4)
        let long_item = "가".repeat(MAX_ITEM_CHARS + 1);
        let body = format!(
            r#"{{"shortSummary":"x","keyPoints":[{}]}}"#,
            serde_json::to_string(&long_item).expect("직렬화할 수 있어야 한다")
        );
        assert_eq!(
            parse_note(NoteType::Summary, &body),
            Err(ResponseRejection::TextTooLong {
                field: "keyPoints",
                chars: MAX_ITEM_CHARS + 1,
                limit: MAX_ITEM_CHARS,
            })
        );

        let items: Vec<String> = (0..=MAX_ITEMS).map(|index| index.to_string()).collect();
        let body = format!(
            r#"{{"shortSummary":"x","keyPoints":{}}}"#,
            serde_json::to_string(&items).expect("직렬화할 수 있어야 한다")
        );
        assert_eq!(
            parse_note(NoteType::Summary, &body),
            Err(ResponseRejection::TooManyItems {
                field: "keyPoints",
                count: MAX_ITEMS + 1,
            })
        );

        let long_overview = "나".repeat(MAX_OVERVIEW_CHARS + 1);
        let body = format!(
            r#"{{"shortSummary":{},"keyPoints":[]}}"#,
            serde_json::to_string(&long_overview).expect("직렬화할 수 있어야 한다")
        );
        assert!(matches!(
            parse_note(NoteType::Summary, &body),
            Err(ResponseRejection::TextTooLong { .. })
        ));
    }

    #[test]
    fn no_response_body_can_end_the_app() {
        // 위 열 가지 외에도, 어떤 본문이 와도 값으로 끝난다 — panic도 unwrap도 없다.
        let deep_nesting = format!("{}{}", "{\"a\":".repeat(300), "1".to_owned() + &"}".repeat(300));
        let bodies = [
            "{".to_owned(),
            "}".to_owned(),
            "[]".to_owned(),
            "[{\"overview\":\"x\"}]".to_owned(),
            "null".to_owned(),
            "0".to_owned(),
            "\"just a string\"".to_owned(),
            "{\"overview\":".to_owned(),
            "```json\n{\"overview\":\"x\"".to_owned(),
            "```\n```".to_owned(),
            "```json```".to_owned(),
            "{\"overview\":\"{ 중괄호가 든 문장 }\"}".to_owned(),
            "{\"overview\":\"백슬래시 \\\" 안의 }\"}".to_owned(),
            "\u{0}\u{1}".to_owned(),
            "가나다".to_owned(),
            "{\"overview\":\"x\",\"keyDiscussions\":null,\"decisions\":[],\"actionItems\":[],\"openQuestions\":[]}".to_owned(),
            deep_nesting,
        ];

        for body in bodies {
            for mode in NoteType::ALL {
                // 결과가 무엇이든 좋다. 돌아오기만 하면 된다.
                let _ = parse_note(mode, &body);
            }
        }
    }

    #[test]
    fn recovery_happens_once_and_does_not_guess_fields() {
        // 두 개의 객체가 이어져 있으면 **바깥쪽 균형 잡힌 하나**만 취한다. 합치지 않는다.
        let body = "설명 {\"shortSummary\":\"첫째\",\"keyPoints\":[]} 그리고 {\"keyPoints\":[\"둘째\"]}";

        let note = parse_note(NoteType::Summary, body).expect("첫 객체를 읽는다");

        assert_eq!(
            note,
            StructuredNote::Summary(SummaryNote {
                short_summary: "첫째".to_owned(),
                key_points: vec![],
            })
        );
    }

    #[test]
    fn a_parsed_note_can_be_stored_and_read_back() {
        // 방어 경로를 통과한 값이 그대로 저장 형태로 이어진다.
        let body = "```\n{\"overview\":\"강의\",\"keyConcepts\":[\"합의\"],\"importantDetails\":[],\"questions\":[],\"thingsToStudy\":[],\"referencesMentioned\":[]}\n```";

        let note = parse_note(NoteType::Study, body).expect("읽을 수 있어야 한다");
        let content = encode_content(&note);

        assert_eq!(
            decode_content(&content, NoteType::Study).expect("다시 읽을 수 있어야 한다"),
            note
        );
    }
}
