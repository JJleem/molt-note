//! **벤더 지식이 사는 자리** — 엔드포인트 경로 · 요청 필드 이름 · 응답 해석 (INV-9).
//!
//! 이 파일의 모든 문자열은 제품의 다른 어느 곳에도 없다. domain도 db도 계약(`ai::provider`)도
//! 아래 경로와 이름을 모르며, 그래서 벤더가 바뀌면 흔들리는 것은 이 디렉터리 하나다
//! (ADR-0008 §16.1).
//!
//! **여기에는 네트워크가 없다.** 값에서 값을 만드는 함수뿐이므로 서버 없이 전부 검증된다
//! (ADR-0008 §4.3 · §18).
//!
//! ## 근거 — 여기 적힌 이름은 어디서 왔는가
//!
//! ADR-0008 §14.2의 재확인 표가 유일한 출처다. **기억이나 추측으로 채운 이름이 없다.**
//!
//! | 값 | 상태 | 확인 시점 | 근거 |
//! | --- | --- | --- | --- |
//! | [`TAGS_PATH`] · 응답의 `models[]`와 `name`/`model` | VERIFIED | 2026-09-01 | §14.2 항목 2 |
//! | [`GENERATE_PATH`] | VERIFIED | 2026-09-01 | §14.2 항목 3 |
//! | 요청 필드 `stream`(=`false`면 단일 JSON) | VERIFIED | 2026-09-01 | §14.2 항목 4 |
//! | 요청 필드 `format`(완전한 JSON Schema 객체를 받는다) | VERIFIED | 2026-09-01 | §14.2 항목 5 |
//! | 요청 필드 `options.num_ctx` | VERIFIED | 2026-09-01 | §14.2 항목 10 |
//! | 요청 필드 `model` · `prompt` | VERIFIED | 2026-09-01 | §14.5 · ADR-0008 §6.4의 본문 예시 |
//! | **생성 응답에서 본문 텍스트가 담기는 필드 이름** | **UNVERIFIED [E4]** | — | §14.3이 "어디에도 기록이 없다"고 적었다 |
//!
//! 이 Run도 그 마지막 항목을 확인하지 못했다 — `WebFetch`가 거부됐고 (증거:
//! `.loop/evidence/TASK-036/verification-log.md`), TASK-032가 겪은 것과 같은 제약이다.
//! **그래서 이 파일은 그 이름을 지어내지 않고, 그 이름에 의존하지 않는 경로를 택했다**
//! ([`generated_note`]). ADR-0008 §3.1이 §14.5의 세부값에 대해 내린 세 결정과 같은 방식이다 —
//! *틀렸을 때 무너지는 범위를 줄이는 쪽*을 고른다.

use serde_json::{json, Map, Value};

use crate::ai::note::{json_schema, parse_note, ResponseRejection, StructuredNote};
use crate::domain::NoteType;

/// 설치된 모델 목록 (ADR-0008 §6.4 — 가용성과 모델 목록을 이 한 번으로 답한다).
pub const TAGS_PATH: &str = "/api/tags";

/// 노트 생성 (ADR-0008 §6.4 — 대화 상태가 없는 한 번의 변환이므로 chat 쪽을 쓰지 않는다).
pub const GENERATE_PATH: &str = "/api/generate";

/// 목록 응답에서 모델들이 담기는 배열의 이름.
const MODELS_FIELD: &str = "models";

/// 목록의 각 항목이 자기 식별자를 담는 이름. 앞의 것이 없으면 뒤의 것을 쓴다 — 둘 다 §14.2
/// 항목 2가 기록한 이름이다.
const MODEL_IDENTIFIER_FIELDS: [&str; 2] = ["name", "model"];

/// 요청 하나가 보낼 주소.
///
/// **base URL은 인자로 들어온다.** 이 adapter는 host도 port도 자기 안에 갖지 않으며, 값이
/// 어디서 오는지도 알지 않는다 (설정 → 부르는 쪽 → 여기). 기본 주소를 아는 자리는
/// [`crate::domain::settings::DEFAULT_AI_BASE_URL`] 하나다.
pub fn endpoint(base_url: &str, path: &str) -> String {
    format!("{}{path}", base_url.trim_end_matches('/'))
}

/// 생성 요청 본문 (ADR-0008 §6.4의 모양 그대로).
///
/// - `stream`은 **언제나 `false`**다. 스트리밍 응답 UI는 이 Phase의 범위 밖이며(§15),
///   단일 JSON이어야 [`generated_note`]가 한 번에 읽는다.
/// - `format`에는 그 mode의 JSON Schema를 그대로 싣는다. **정확성을 여기에 걸지 않는다** —
///   서버가 무시하거나 거절해도 파싱 경로가 같은 답을 낸다 (§6.2).
/// - `num_ctx`를 **항상 명시한다.** 서버 기본값(4096)에 기대면 1시간 분량 transcript가
///   조용히 잘린다 (§8.2).
pub fn generate_body(model: &str, prompt: &str, mode: NoteType, context_tokens: u32) -> String {
    json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "format": json_schema(mode),
        "options": { "num_ctx": context_tokens },
    })
    .to_string()
}

/// 응답 본문을 이 adapter가 기대한 모양으로 읽지 못했다.
///
/// **변형에 문자열 자리가 없다.** 실패 `detail`로 옮겨질 값이므로, 설정값이나 응답 본문이
/// 섞일 수 있는 자리를 만들지 않는다 (ADR-0008 §11.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyRejection {
    /// 본문이 JSON 객체가 아니었다.
    NotJson,
    /// JSON은 읽었지만 기대한 자리에 목록이 없었다.
    Shape,
}

impl BodyRejection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotJson => "response body is not a JSON object",
            Self::Shape => "response body has no model list",
        }
    }
}

impl std::fmt::Display for BodyRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 목록 응답 → 설치된 모델 식별자들 (ADR-0008 §14.2 항목 2).
///
/// 식별자가 없거나 공백뿐인 항목은 **버린다.** 사용자가 고를 수 없는 항목을 목록에 남기면
/// 계약이 요구하는 "빈 이름이 없는 목록"이 깨진다 (`assert_availability_contract`).
pub fn installed_models(body: &str) -> Result<Vec<String>, BodyRejection> {
    let value: Value = serde_json::from_str(body).map_err(|_| BodyRejection::NotJson)?;
    let entries = value
        .get(MODELS_FIELD)
        .and_then(Value::as_array)
        .ok_or(BodyRejection::Shape)?;

    Ok(entries
        .iter()
        .filter_map(model_identifier)
        .filter(|name| !name.is_empty())
        .collect())
}

/// 목록 항목 하나가 말하는 자기 식별자.
fn model_identifier(entry: &Value) -> Option<String> {
    MODEL_IDENTIFIER_FIELDS
        .iter()
        .find_map(|field| entry.get(field).and_then(Value::as_str))
        .map(|name| name.trim().to_owned())
}

/// 생성 응답 본문 → 그 mode의 노트 (ADR-0008 §6.3의 1~4단계).
///
/// ## 왜 필드 이름으로 꺼내지 않는가
///
/// §6.3의 1단계는 "본문을 JSON으로 읽고 **생성 결과 필드**를 꺼낸다"이지만, **그 필드의 이름은
/// 이 저장소 어디에도 확인된 적이 없다** (ADR-0008 §14.3 — "§14.5에 명시되어 있지 않다 [E4]").
/// 이 Run도 확인에 실패했다. 그래서 이름을 지어내는 대신, 이름을 묻지 않는 방법을 쓴다.
///
/// ```text
/// 본문(JSON 객체)의 **최상위 문자열 값들**을 후보로 놓고
///   → 각 후보에 §6.3의 2~4단계(parse_note)를 그대로 적용하고
///   → 그 mode의 노트로 읽히는 첫 후보를 취한다
/// ```
///
/// 나머지 최상위 문자열(모델 이름 · 시각 · 종료 사유 같은 값)은 노트 schema를 만족할 수 없으므로
/// 후보에서 자연히 떨어진다 — 필수 필드가 있는 JSON 객체여야 통과하기 때문이다.
/// **느슨해진 것은 어느 이름에서 꺼내는가 뿐이고, 무엇을 노트로 받아들이는가는 조금도 느슨해지지
/// 않았다.** 판정은 제품의 [`parse_note`]가 그대로 한다.
///
/// 이름이 확인되면 이 자리를 그 이름 하나로 좁힌다. 그때 바뀌는 것은 이 함수 하나다.
pub fn generated_note(mode: NoteType, body: &str) -> Result<StructuredNote, ResponseRejection> {
    let value: Value = serde_json::from_str(body).map_err(|_| ResponseRejection::NotJson)?;
    let Some(object) = value.as_object() else {
        return Err(ResponseRejection::NotJson);
    };

    let mut seen_candidate = false;
    // "왜 노트가 아닌가"는 **가장 노트에 가까웠던 후보**의 이유로 남긴다. 모델 이름 같은 값이
    // 내는 `NotJson`을 그대로 올리면 진짜 원인(필드 누락 등)이 가려진다.
    let mut closest: Option<ResponseRejection> = None;

    for candidate in top_level_strings(object) {
        seen_candidate = true;
        match parse_note(mode, candidate) {
            Ok(note) => return Ok(note),
            Err(rejection) => {
                let uninformative = matches!(
                    rejection,
                    ResponseRejection::Empty | ResponseRejection::NotJson
                );
                if !uninformative && closest.is_none() {
                    closest = Some(rejection);
                }
            }
        }
    }

    Err(closest.unwrap_or(if seen_candidate {
        ResponseRejection::NotJson
    } else {
        ResponseRejection::Empty
    }))
}

/// 본문 객체의 최상위 문자열 값들.
fn top_level_strings(object: &Map<String, Value>) -> impl Iterator<Item = &str> {
    object.values().filter_map(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::note::MAX_ITEMS;
    use crate::ai::testing::{sample_note, sample_note_text};

    #[test]
    fn an_endpoint_is_the_injected_address_plus_this_adapters_path() {
        // 주소는 주입된다 — 이 파일에 host도 port도 없다.
        assert_eq!(
            endpoint("http://configured-host.invalid:65535", TAGS_PATH),
            "http://configured-host.invalid:65535/api/tags"
        );
        // 사용자가 끝에 `/`를 붙여 저장했어도 경로가 겹치지 않는다.
        assert_eq!(
            endpoint("http://configured-host.invalid:65535/", GENERATE_PATH),
            "http://configured-host.invalid:65535/api/generate"
        );
    }

    #[test]
    fn a_generation_request_always_turns_streaming_off_and_states_the_context_size() {
        let body = generate_body("configured-model", "프롬프트", NoteType::Meeting, 8_192);
        let value: Value = serde_json::from_str(&body).expect("요청 본문은 JSON이다");

        assert_eq!(value["model"], "configured-model");
        assert_eq!(value["prompt"], "프롬프트");
        assert_eq!(value["stream"], false, "스트리밍 응답은 이 Phase의 범위 밖이다");
        assert_eq!(
            value["options"]["num_ctx"], 8_192,
            "서버 기본값(4096)에 기대지 않는다 (ADR-0008 §8.2)"
        );
        // 요구하는 출력 형태는 domain이 만든 schema 그대로다 — adapter가 다시 적지 않는다.
        assert_eq!(value["format"], json_schema(NoteType::Meeting));
    }

    #[test]
    fn each_mode_asks_for_its_own_output_shape() {
        for mode in NoteType::ALL {
            let body = generate_body("configured-model", "프롬프트", mode, 16_384);
            let value: Value = serde_json::from_str(&body).expect("요청 본문은 JSON이다");
            assert_eq!(value["format"], json_schema(mode));
        }
    }

    #[test]
    fn the_model_list_is_read_from_the_recorded_field_names() {
        let body = r#"{"models":[
            {"name":"first-model","model":"first-model","size":1},
            {"model":"second-model"}
        ]}"#;

        assert_eq!(
            installed_models(body).expect("목록을 읽는다"),
            vec!["first-model".to_owned(), "second-model".to_owned()]
        );
    }

    #[test]
    fn a_server_with_no_models_is_an_empty_list_not_a_rejection() {
        // "응답했지만 모델이 없다"는 정의된 제품 상태다 — 본문을 못 읽은 것과 구분한다 (§13).
        assert_eq!(installed_models(r#"{"models":[]}"#).expect("빈 목록"), Vec::<String>::new());
        // 이름이 없거나 공백뿐인 항목은 고를 수 없으므로 목록에 남기지 않는다.
        assert_eq!(
            installed_models(r#"{"models":[{"size":1},{"name":"   "}]}"#).expect("빈 목록"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_body_that_is_not_the_expected_shape_is_rejected_without_guessing() {
        assert_eq!(installed_models("Ollama is running"), Err(BodyRejection::NotJson));
        assert_eq!(installed_models("{}"), Err(BodyRejection::Shape));
        assert_eq!(installed_models(r#"{"models":{}}"#), Err(BodyRejection::Shape));
    }

    #[test]
    fn a_rejection_detail_is_a_fixed_string_with_no_room_for_a_configured_value() {
        for rejection in [BodyRejection::NotJson, BodyRejection::Shape] {
            let text = rejection.to_string();
            assert!(!text.contains("http"), "detail에 주소가 섞일 자리가 없다: {text}");
        }
    }

    #[test]
    fn the_note_is_recovered_whatever_the_vendor_calls_the_field_that_holds_it() {
        // 생성 응답의 텍스트 필드 이름은 UNVERIFIED다. 그 이름이 무엇으로 밝혀지더라도
        // 이 경로는 같은 노트를 낸다 — 그것이 이름을 지어내지 않은 대가이자 이점이다.
        for field in ["response", "text", "output", "완전히-다른-이름"] {
            let body = json!({
                "model": "served-model",
                field: sample_note_text(NoteType::Meeting),
                "noise": "이것은 노트가 아니다",
            })
            .to_string();

            assert_eq!(
                generated_note(NoteType::Meeting, &body).expect("노트를 회수한다"),
                sample_note(NoteType::Meeting)
            );
        }
    }

    #[test]
    fn a_fenced_note_is_recovered_the_same_way_the_product_does_it() {
        // 회수 규칙은 이 파일이 다시 쓰지 않는다 — 제품의 parse_note가 그대로 한다.
        let fenced = format!("```json\n{}\n```", sample_note_text(NoteType::Summary));
        let body = json!({ "response": fenced }).to_string();

        assert_eq!(
            generated_note(NoteType::Summary, &body).expect("코드펜스는 벗겨진다"),
            sample_note(NoteType::Summary)
        );
    }

    #[test]
    fn other_strings_in_the_body_never_become_a_note() {
        // 모델 이름 · 시각 같은 값이 노트로 오인되지 않는다. 통과하려면 필수 필드를 갖춘
        // 그 mode의 JSON 객체여야 한다.
        let body = json!({
            "model": "served-model",
            "created": "2026-09-03T00:00:00Z",
            "reason": "stop",
        })
        .to_string();

        assert!(matches!(
            generated_note(NoteType::Meeting, &body),
            Err(ResponseRejection::NotJson)
        ));
    }

    #[test]
    fn a_note_of_another_mode_is_not_accepted_for_this_mode() {
        let body = json!({ "response": sample_note_text(NoteType::Study) }).to_string();

        // Study 노트에는 meeting의 필수 필드가 없다 — 모양이 다르다는 이유로 거절된다.
        assert!(matches!(
            generated_note(NoteType::Meeting, &body),
            Err(ResponseRejection::Shape { .. })
        ));
    }

    #[test]
    fn the_reason_kept_is_the_one_from_the_closest_candidate() {
        // 노트에 가장 가까웠던 후보의 이유가 남아야 "무엇이 달랐는가"를 말할 수 있다.
        let body = json!({
            "model": "served-model",
            "response": r#"{"overview":"","keyDiscussions":[],"decisions":[],"actionItems":[],"openQuestions":[]}"#,
        })
        .to_string();

        assert_eq!(
            generated_note(NoteType::Meeting, &body),
            Err(ResponseRejection::BlankRequiredText { field: "overview" })
        );
    }

    #[test]
    fn a_body_without_any_text_says_so() {
        assert_eq!(generated_note(NoteType::Meeting, "{}"), Err(ResponseRejection::Empty));
        assert_eq!(
            generated_note(NoteType::Meeting, "Ollama is running"),
            Err(ResponseRejection::NotJson)
        );
        assert_eq!(generated_note(NoteType::Meeting, "[]"), Err(ResponseRejection::NotJson));
    }

    #[test]
    fn the_size_limits_of_the_domain_still_apply_to_what_the_server_sent() {
        // adapter가 제품의 검증을 우회하지 않는다는 것을 상한 하나로 확인한다 (§7.4).
        let items = vec!["항목"; MAX_ITEMS + 1];
        let note = json!({
            "shortSummary": "요약",
            "keyPoints": items,
        });
        let body = json!({ "response": note.to_string() }).to_string();

        assert!(matches!(
            generated_note(NoteType::Summary, &body),
            Err(ResponseRejection::TooManyItems { .. })
        ));
    }

    #[test]
    fn a_recovered_note_is_the_mode_that_was_asked_for() {
        for mode in NoteType::ALL {
            let body = json!({ "response": sample_note_text(mode) }).to_string();
            let note: StructuredNote = generated_note(mode, &body).expect("노트를 회수한다");
            assert_eq!(note.mode(), mode);
        }
    }
}
