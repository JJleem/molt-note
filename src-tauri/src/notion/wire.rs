//! **벤더 지식이 사는 자리** — 엔드포인트 경로 · 헤더 이름 · 요청 필드 · 응답과 오류 코드
//! 해석 (ADR-0009 §5 · INV-9).
//!
//! 이 파일의 모든 문자열은 제품의 다른 어느 곳에도 없다. domain도 db도 화면도 아래 경로와
//! 이름을 모른다.
//!
//! **여기에는 네트워크가 없다.** 값에서 값을 만드는 함수뿐이므로 Notion 없이 전부 검증된다
//! (PRODUCT-SPEC §18).
//!
//! ## 근거 — 여기 적힌 값은 어디서 왔는가
//!
//! ADR-0009 §5 · §9와 그 출처인 PRODUCT-SPEC §14.9.1 · §14.9.2가 유일한 출처다.
//! **기억이나 추측으로 채운 값이 없다.**
//!
//! | 값 | 상태 | 근거 |
//! | --- | --- | --- |
//! | [`NOTION_VERSION`] = `2026-03-11` | VERIFIED | §14.9.2 (2026-09-04에 두 번째 재확인) |
//! | `Authorization: Bearer <secret>` | VERIFIED | §14.9.1 |
//! | [`USERS_ME_PATH`] (연결 확인) | VERIFIED | §14.9.1 |
//! | 그 응답이 **bot이면 `workspace_name`을 준다** ([`connected_identity`]) | VERIFIED | §14.9.1 |
//! | [`PAGES_PATH`]의 `parent.page_id` + **`markdown`** body param | VERIFIED | §14.9.1 |
//! | `PATCH /v1/pages/:page_id/markdown` · `insert_content` · `position:{type:"end"}` | VERIFIED | §14.9.1 |
//! | `markdown`과 `children`/`content`를 **같은 요청에 함께 쓸 수 없다** | VERIFIED | §14.9.1 |
//! | 줄바꿈은 실제 개행이며 JSON에서는 `\n` | VERIFIED | §14.9.1 |
//! | 실패 코드 여덟 개 ([`ApiErrorCode`]) | VERIFIED | §14.9.1 |
//! | `Retry-After`는 **정수 초** | VERIFIED | §14.9.1 (문서 인용) |
//! | markdown 엔드포인트 **전용** 본문 크기 상한 | **UNVERIFIED** | §14.9.1 — 값이 존재하는지조차 확인되지 않았다. **이 파일에 그런 상수가 없다** |
//!
//! ⚠️ **조사 날짜를 API 버전으로 쓰지 않는다** (§14.9.2 · `phase-prompt/05`). `2026-09-01` ·
//! `2026-09-04`는 조사한 날짜이고 `2026-07-28`은 MCP 프로토콜 버전이다. 헤더로 나가는 값은
//! [`NOTION_VERSION`] 하나뿐이며, 그것이 다른 값이 아님을 테스트가 확인한다.

use std::fmt;

use serde_json::{json, Value};

use crate::platform::secret_store::Secret;

/// Notion API의 주소. **설정값이 아니다** — 사용자가 고를 수 있는 값이 아니므로 주입하지 않는다
/// (연결 대상이 설정에서 오는 AI adapter와 갈리는 지점이다 · ADR-0009 §5.1).
pub const API_BASE_URL: &str = "https://api.notion.com";

/// 이 앱이 말하는 API 버전 (PRODUCT-SPEC §14.9.2).
///
/// **설정으로 노출하지 않는다.** 사용자가 고를 수 있는 값이 아니다 (ADR-0009 §5.2).
pub const NOTION_VERSION: &str = "2026-03-11";

pub const AUTHORIZATION_HEADER: &str = "Authorization";
pub const NOTION_VERSION_HEADER: &str = "Notion-Version";
pub const CONTENT_TYPE_HEADER: &str = "Content-Type";
pub const JSON_CONTENT_TYPE: &str = "application/json";

/// 응답이 대기 시간을 말하는 헤더 (ADR-0009 §9.1).
pub const RETRY_AFTER_HEADER: &str = "Retry-After";

/// 연결 확인 — 토큰에 딸린 사용자를 돌려준다 (§14.9.1).
pub const USERS_ME_PATH: &str = "/v1/users/me";

/// 페이지 생성 (§14.9.1 — `parent.page_id` + `markdown`).
pub const PAGES_PATH: &str = "/v1/pages";

/// 이어붙이기 경로의 끝. 전체 경로는 [`page_markdown_url`]이 만든다.
pub const MARKDOWN_PATH_SUFFIX: &str = "/markdown";

/// 주소 하나.
pub fn url(base_url: &str, path: &str) -> String {
    format!("{}{path}", base_url.trim_end_matches('/'))
}

/// 이어붙이기 주소 — `PATCH /v1/pages/<page_id>/markdown` (§14.9.1).
pub fn page_markdown_url(base_url: &str, page: &PageId) -> String {
    format!(
        "{}{PAGES_PATH}/{}{MARKDOWN_PATH_SUFFIX}",
        base_url.trim_end_matches('/'),
        page.as_str()
    )
}

/// Notion 페이지 하나를 가리키는 식별자.
///
/// **문자열이 아니라 타입인 이유는 두 가지다.**
///
/// 1. 이 값은 **주소의 일부가 된다** ([`page_markdown_url`]). 주소에 그대로 이어 붙일 수 없는
///    문자열(공백 · `/` · `?` · 개행)이 여기까지 오면 요청이 다른 곳으로 갈 수 있다. 그래서
///    [`PageId::parse`]를 지나지 않고는 만들 수 없다.
/// 2. 이 값은 **destination이다.** `Debug`가 내용을 내지 않으므로 `{:?}` 한 번으로 로그나
///    실패 문장에 옮겨 가지 않는다 (ADR-0009 §10.4 · INV-7).
#[derive(Clone, PartialEq, Eq)]
pub struct PageId(String);

impl PageId {
    /// 주소에 그대로 실을 수 있는 식별자만 받는다. **고쳐 읽지 않는다** — 모양이 다르면 거절한다.
    ///
    /// Notion의 page id는 UUID이거나 그 하이픈 없는 형태다. 여기서 UUID 문법을 강제하지 않는
    /// 이유는 그것이 **확인된 계약이 아니기 때문**이고, 대신 "주소에 안전한가"라는 이 앱이
    /// 실제로 필요로 하는 성질만 본다.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let safe = !value.is_empty()
            && value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-');

        safe.then(|| Self(value.to_owned()))
    }

    /// 저장하거나 주소에 싣기 위해 값을 꺼낸다.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// **내용을 내지 않는다** — destination은 실패 문장에도 로그에도 남지 않는다.
impl fmt::Debug for PageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PageId(<redacted>)")
    }
}

/// `Authorization` 헤더 값 하나 — `Bearer <secret>` (§14.9.1).
///
/// **이 저장소에서 token이 문자열로 조립되는 유일한 자리다.** [`Secret`]과 같은 성질을 갖는다:
/// `Debug`가 내용을 내지 않고, `Display`가 없으며, 직렬화되지 않는다. 값을 꺼내는 자리는
/// [`AuthorizationValue::expose`] 하나이고 그것이 향하는 곳은 요청 헤더 하나뿐이다
/// (ADR-0009 §10.4).
#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizationValue(String);

impl AuthorizationValue {
    pub fn new(token: &Secret) -> Self {
        Self(format!("Bearer {}", token.expose()))
    }

    /// 헤더에 실기 위해 값을 꺼낸다. **부르는 자리가 곧 token이 지나가는 자리다.**
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AuthorizationValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AuthorizationValue(<redacted>)")
    }
}

/// 페이지 생성 요청 본문 (§14.9.1의 인용 그대로).
///
/// ```json
/// { "parent": { "page_id": "…" }, "markdown": "# Meeting Notes\nDiscussed roadmap priorities." }
/// ```
///
/// - **`children`도 `content`도 만들지 않는다.** 같은 요청에 함께 쓸 수 없다는 것이 VERIFIED이고
///   (§14.9.1), 이 앱은 블록 JSON 경로를 아예 쓰지 않는다 (ADR-0009 §5.4).
/// - **`properties`를 보내지 않는다.** 제목은 markdown 첫 `# h1`이 된다 (§5.3) — `properties`와
///   `markdown`을 함께 보낼 수 있는지는 UNVERIFIED이며, 확인되지 않은 조합 위에 생성 경로를
///   세우지 않는다.
/// - **`allow_async`를 보내지 않는다** (§7).
/// - 줄바꿈은 실제 개행이며, JSON 직렬화가 그것을 `\n`으로 내보낸다 (§14.9.1).
pub fn create_page_body(parent_page_id: &str, markdown: &str) -> String {
    json!({
        "parent": { "page_id": parent_page_id },
        "markdown": markdown,
    })
    .to_string()
}

/// 이어붙이기 요청 본문 (§14.9.1의 인용 그대로).
///
/// ```json
/// { "type": "insert_content", "insert_content": { "content": "…", "position": { "type": "end" } } }
/// ```
///
/// **`replace_content`를 쓰지 않는다** — 그 요청 본문의 정확한 형태는 UNVERIFIED이고, 확인되지
/// 않은 형태 위에 파괴적 동작을 세우지 않는다 (ADR-0009 §8.3).
pub fn append_markdown_body(markdown: &str) -> String {
    json!({
        "type": "insert_content",
        "insert_content": {
            "content": markdown,
            "position": { "type": "end" },
        },
    })
    .to_string()
}

/// 응답 본문을 이 adapter가 기대한 모양으로 읽지 못했다.
///
/// **변형에 문자열 자리가 없다.** 실패 `detail`로 옮겨질 값이므로, 서버가 되돌려준 문장이
/// 섞일 수 있는 자리를 만들지 않는다 (ADR-0009 §10.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyRejection {
    /// 본문이 JSON 객체가 아니었다.
    NotJson,
    /// JSON은 읽었지만 페이지가 아닌 것이 돌아왔다 — 요청하지 않은 `202`의 `async_task`가
    /// 여기 속한다 (ADR-0009 §7.3).
    NotAPage,
    /// 페이지라고 했지만 이 앱이 쓸 수 있는 식별자가 없었다.
    NoPageId,
}

impl BodyRejection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotJson => "response body is not a JSON object",
            Self::NotAPage => "response is not a page",
            Self::NoPageId => "response carries no usable page id",
        }
    }
}

impl fmt::Display for BodyRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 생성 응답 → 만들어진 페이지의 식별자 (§14.9.1).
///
/// **`object`가 페이지라고 말하지 않으면 받지 않는다.** 요청하지 않은 `202`가 `async_task`를
/// 돌려주는 경우, 그 안의 식별자는 페이지가 아니라 작업의 것이다 — 그것을 page id로 저장하면
/// 다음 요청이 존재하지 않는 페이지로 나간다 (ADR-0009 §7.3이 "성공으로 간주하지 않는다"고
/// 정한 자리다).
pub fn created_page_id(body: &str) -> Result<PageId, BodyRejection> {
    let value: Value = serde_json::from_str(body).map_err(|_| BodyRejection::NotJson)?;
    let object = value.as_object().ok_or(BodyRejection::NotJson)?;

    if let Some(kind) = object.get("object").and_then(Value::as_str) {
        if kind != "page" {
            return Err(BodyRejection::NotAPage);
        }
    }

    object
        .get("id")
        .and_then(Value::as_str)
        .and_then(PageId::parse)
        .ok_or(BodyRejection::NoPageId)
}

/// 연결 확인이 말해 준 **누구에게 연결됐는가** (§14.9.1 · PRODUCT-SPEC §5-D).
///
/// **secret이 아니다.** 워크스페이스 이름은 사용자가 "이 token이 내 어느 워크스페이스의
/// 것인가"를 알아보기 위한 값이며 (§5-D의 connection test), token은 이 타입에 담길 자리가 없다.
///
/// `Option`인 이유는 하나다 — **Notion이 말해 주지 않으면 지어내지 않는다.** bot이 아닌 token은
/// `workspace_name`을 주지 않으며, 그때 화면은 이름 없이 "연결됐다"만 말한다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectedIdentity {
    /// 이 token이 딸린 워크스페이스의 이름 (§14.9.1 — bot이면 `workspace_name`을 준다).
    pub workspace_name: Option<String>,
}

/// 연결 확인 응답 → 누구에게 연결됐는가 (§14.9.1).
///
/// **읽지 못해도 실패하지 않는다.** "이 token으로 말할 수 있는가"의 답은 이미 상태 코드가 했고,
/// 이름은 그 위에 얹히는 부가 사실이다 ([`created_page_id`]와 갈리는 지점이다: 그쪽은 값이
/// 없으면 다음 요청을 보낼 곳이 사라진다).
pub fn connected_identity(body: &str) -> ConnectedIdentity {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return ConnectedIdentity::default();
    };

    ConnectedIdentity {
        workspace_name: readable_text(value.get("bot").and_then(|bot| bot.get("workspace_name"))),
    }
}

/// 사람이 읽을 수 있는 값 하나. 문자열이 아니거나 공백뿐이면 **말하지 않은 것과 같다.**
fn readable_text(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// Notion이 실패를 말하는 코드 (§14.9.1의 VERIFIED 목록 중 이 adapter가 구분해 다루는 것들).
///
/// **여기 없는 코드는 지어내지 않는다** — `invalid_json` · `invalid_request_url` ·
/// `invalid_request`도 문서에 있지만, 이 앱에게는 "우리가 보낸 요청이 잘못됐다"로 같은 답이
/// 나오므로 상태 코드 경로가 그대로 처리한다 (ADR-0009 §9.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiErrorCode {
    ValidationError,
    Unauthorized,
    RestrictedResource,
    ObjectNotFound,
    RateLimited,
    ConflictError,
    InternalServerError,
    ServiceUnavailable,
}

impl ApiErrorCode {
    /// 이 adapter가 아는 코드 전부. `detail`에 그대로 남는 **벤더의 고정 문자열**이며,
    /// 설정값도 token도 destination도 아니다.
    pub const ALL: [Self; 8] = [
        Self::ValidationError,
        Self::Unauthorized,
        Self::RestrictedResource,
        Self::ObjectNotFound,
        Self::RateLimited,
        Self::ConflictError,
        Self::InternalServerError,
        Self::ServiceUnavailable,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ValidationError => "validation_error",
            Self::Unauthorized => "unauthorized",
            Self::RestrictedResource => "restricted_resource",
            Self::ObjectNotFound => "object_not_found",
            Self::RateLimited => "rate_limited",
            Self::ConflictError => "conflict_error",
            Self::InternalServerError => "internal_server_error",
            Self::ServiceUnavailable => "service_unavailable",
        }
    }

    /// 오류 본문의 `code`를 읽는다. 본문이 없거나 모르는 코드면 `None`이다 — 그때 판정은
    /// 상태 코드가 한다 (`529`처럼 JSON 본문이 온다는 계약이 없는 응답이 그런 자리다).
    pub fn from_body(body: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(body).ok()?;
        let code = value.get("code")?.as_str()?;

        Self::ALL.into_iter().find(|known| known.as_str() == code)
    }
}

impl fmt::Display for ApiErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `Retry-After` 헤더 값 → **정수 초** (§14.9.1 문서 인용: *"The header value is an integer
/// number of seconds."*).
///
/// **HTTP-date 형식을 파싱하지 않는다.** 확인된 계약이 "정수 초"이므로 그 밖의 형식을
/// 지원한다고 가정하지 않는다 (ADR-0009 §9.2-5). 읽지 못하면 `None`이며, 그때 얼마나 기다릴지는
/// **앱이 정하는 일**이지 이 함수가 지어낼 값이 아니다.
pub fn retry_after_seconds(header_value: &str) -> Option<u32> {
    let value = header_value.trim();

    // `parse::<u32>`는 `+30`도 받아들인다. 계약은 정수 초이고, 우리가 아는 모양만 받는다.
    if value.is_empty() || !value.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }

    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트가 쓰는 값들. **하나도 실재하지 않는다** (ADR-0009 §10.5).
    const NOT_A_REAL_TOKEN: &str = "ntn-double-value-not-a-real-credential";
    const PARENT: &str = "parent-page-identifier";

    #[test]
    fn the_api_version_is_the_one_notion_documents_and_not_a_date_we_looked_it_up() {
        // PRODUCT-SPEC §14.9.2의 표 그대로다. 조사 날짜와 MCP 프로토콜 버전은 API 버전이 아니다.
        assert_eq!(NOTION_VERSION, "2026-03-11");

        for not_a_version in ["2026-09-01", "2026-09-04", "2026-07-28", "2025-09-03", "2022-06-28"] {
            assert_ne!(
                NOTION_VERSION, not_a_version,
                "조사 날짜나 MCP 버전이 API 버전으로 나갔다"
            );
        }
    }

    #[test]
    fn every_address_is_built_from_the_one_base_and_the_recorded_paths() {
        assert_eq!(
            url(API_BASE_URL, USERS_ME_PATH),
            "https://api.notion.com/v1/users/me"
        );
        assert_eq!(
            url(API_BASE_URL, PAGES_PATH),
            "https://api.notion.com/v1/pages"
        );
        assert_eq!(
            page_markdown_url(API_BASE_URL, &PageId::parse("created-page").expect("모양이 맞다")),
            "https://api.notion.com/v1/pages/created-page/markdown"
        );
        // 끝의 `/`가 경로를 겹치게 하지 않는다.
        assert_eq!(url("https://api.notion.com/", PAGES_PATH), "https://api.notion.com/v1/pages");
    }

    #[test]
    fn a_page_id_that_could_change_where_the_request_goes_is_refused() {
        assert!(PageId::parse("2f0c1a9b-1111-2222-3333-444455556666").is_some());
        assert!(PageId::parse("2f0c1a9b111122223333444455556666").is_some());
        // 사용자가 붙여 넣은 값의 앞뒤 공백(줄바꿈 포함)은 다듬는다 — 그것은 값의 일부가 아니다.
        assert_eq!(
            PageId::parse("  spaced-id \n").expect("앞뒤 공백은 다듬는다").as_str(),
            "spaced-id"
        );

        for unsafe_value in [
            "",
            "   ",
            "../v1/users/me",
            "id?query=1",
            "id with space",
            "id\nmore", // 값 **가운데**의 줄바꿈은 다듬어지지 않는다 — 거절한다
            "id/markdown",
        ] {
            assert!(
                PageId::parse(unsafe_value).is_none(),
                "주소를 바꿀 수 있는 값이 통과했다: {unsafe_value:?}"
            );
        }
    }

    #[test]
    fn neither_the_token_nor_the_destination_can_be_printed() {
        let authorization = AuthorizationValue::new(&Secret::new(NOT_A_REAL_TOKEN));
        assert_eq!(authorization.expose(), format!("Bearer {NOT_A_REAL_TOKEN}"));

        let rendered = format!("{authorization:?}");
        assert!(!rendered.contains(NOT_A_REAL_TOKEN), "{rendered}");

        let page = PageId::parse("destination-page-identifier").expect("모양이 맞다");
        let rendered = format!("{page:?}");
        assert!(!rendered.contains("destination-page-identifier"), "{rendered}");
    }

    #[test]
    fn creating_a_page_sends_the_parent_and_the_markdown_and_nothing_else() {
        let body = create_page_body(PARENT, "# 제목\n본문");
        let value: Value = serde_json::from_str(&body).expect("요청 본문은 JSON이다");

        assert_eq!(value["parent"]["page_id"], PARENT);
        assert_eq!(value["markdown"], "# 제목\n본문");

        // §14.9.1: `markdown`과 함께 쓸 수 없는 것들이 요청에 없다.
        let object = value.as_object().expect("객체다");
        assert_eq!(object.len(), 2, "요청에 다른 필드가 늘었다: {object:?}");
        for forbidden in ["children", "content", "properties", "allow_async"] {
            assert!(object.get(forbidden).is_none(), "{forbidden}가 함께 나갔다");
        }
    }

    #[test]
    fn a_newline_in_the_document_leaves_as_an_escaped_newline_and_arrives_as_a_real_one() {
        // §14.9.1: markdown은 실제 개행을 기대하며 JSON에서는 `\n`이다.
        let body = create_page_body(PARENT, "첫 줄\n둘째 줄");

        assert!(body.contains(r"\n"), "직렬화가 개행을 이스케이프하지 않았다: {body}");
        assert!(!body.contains('\n'), "본문에 날 개행이 들어갔다: {body}");

        let value: Value = serde_json::from_str(&body).expect("요청 본문은 JSON이다");
        assert_eq!(value["markdown"], "첫 줄\n둘째 줄", "받는 쪽에는 실제 개행이다");
    }

    #[test]
    fn appending_inserts_the_content_at_the_end_of_the_page() {
        let body = append_markdown_body("## 부록\n내용");
        let value: Value = serde_json::from_str(&body).expect("요청 본문은 JSON이다");

        assert_eq!(value["type"], "insert_content");
        assert_eq!(value["insert_content"]["content"], "## 부록\n내용");
        assert_eq!(value["insert_content"]["position"]["type"], "end");

        // 페이지를 바꿔치기하는 경로는 만들지 않는다 (ADR-0009 §8.3).
        assert!(!body.contains("replace_content"));
        assert!(!body.contains("update_content"));
    }

    #[test]
    fn the_created_page_comes_back_as_a_value() {
        let body = r#"{"object":"page","id":"created-page-id","url":"https://notion.so/x"}"#;

        assert_eq!(
            created_page_id(body).expect("페이지 id를 읽는다").as_str(),
            "created-page-id"
        );
    }

    #[test]
    fn a_response_that_is_not_a_created_page_is_never_read_as_one() {
        // 요청하지 않은 202 — 작업 식별자를 page id로 저장하면 다음 요청이 엉뚱한 곳으로 간다.
        assert_eq!(
            created_page_id(r#"{"object":"async_task","id":"task-id","status_url":"…"}"#),
            Err(BodyRejection::NotAPage)
        );
        assert_eq!(created_page_id("{}"), Err(BodyRejection::NoPageId));
        assert_eq!(created_page_id(r#"{"object":"page"}"#), Err(BodyRejection::NoPageId));
        assert_eq!(
            created_page_id(r#"{"object":"page","id":"has space"}"#),
            Err(BodyRejection::NoPageId)
        );
        assert_eq!(created_page_id("not json"), Err(BodyRejection::NotJson));
        assert_eq!(created_page_id("[]"), Err(BodyRejection::NotJson));
    }

    #[test]
    fn the_connection_check_reads_which_workspace_answered() {
        // §14.9.1 — bot token이면 `bot.workspace_name`이 온다. 화면은 이 값으로 "어느
        // 워크스페이스에 연결됐는가"를 말한다 (§5-D).
        let body = json!({
            "object": "user",
            "id": "bot-user-id",
            "type": "bot",
            "bot": { "workspace_name": "Ada의 워크스페이스" },
        })
        .to_string();

        assert_eq!(
            connected_identity(&body).workspace_name.as_deref(),
            Some("Ada의 워크스페이스")
        );
    }

    #[test]
    fn a_connection_check_that_names_no_workspace_invents_none() {
        // 말해 주지 않은 이름을 지어내면 사용자는 자기 것이 아닌 워크스페이스에 연결됐다고
        // 읽을 수 있다. 읽지 못한 본문도 같다 — **확인의 답은 이미 상태 코드가 했다.**
        for body in [
            r#"{"object":"user","id":"person-id","type":"person"}"#,
            r#"{"object":"user","bot":{"workspace_name":"   "}}"#,
            r#"{"object":"user","bot":{"workspace_name":42}}"#,
            "not json",
            "[]",
        ] {
            assert_eq!(
                connected_identity(body),
                ConnectedIdentity::default(),
                "이름을 지어냈다: {body}"
            );
        }
    }

    #[test]
    fn a_rejection_detail_is_a_fixed_string_with_no_room_for_what_the_server_said() {
        for rejection in [BodyRejection::NotJson, BodyRejection::NotAPage, BodyRejection::NoPageId] {
            let text = rejection.to_string();
            assert!(!text.is_empty());
            assert!(!text.contains("notion"), "detail에 주소가 섞일 자리가 없다: {text}");
        }
    }

    #[test]
    fn the_documented_error_codes_are_read_from_the_body() {
        for code in ApiErrorCode::ALL {
            let body = format!(
                r#"{{"object":"error","status":400,"code":"{}","message":"…"}}"#,
                code.as_str()
            );
            assert_eq!(ApiErrorCode::from_body(&body), Some(code));
        }
    }

    #[test]
    fn a_body_with_no_code_we_know_is_not_guessed_at() {
        assert_eq!(ApiErrorCode::from_body(""), None);
        assert_eq!(ApiErrorCode::from_body("<html>overloaded</html>"), None);
        assert_eq!(ApiErrorCode::from_body(r#"{"code":"invalid_json"}"#), None);
        assert_eq!(ApiErrorCode::from_body(r#"{"code":42}"#), None);
    }

    #[test]
    fn the_wait_is_read_as_a_whole_number_of_seconds_and_nothing_else() {
        // §14.9.1 문서 인용: "The header value is an integer number of seconds."
        assert_eq!(retry_after_seconds("30"), Some(30));
        assert_eq!(retry_after_seconds(" 30 "), Some(30));
        assert_eq!(retry_after_seconds("0"), Some(0));

        // HTTP-date는 확인된 계약이 아니다 — 지원한다고 가정하지 않는다.
        assert_eq!(retry_after_seconds("Wed, 21 Oct 2026 07:28:00 GMT"), None);
        assert_eq!(retry_after_seconds("1.5"), None);
        assert_eq!(retry_after_seconds("+30"), None);
        assert_eq!(retry_after_seconds("-5"), None);
        assert_eq!(retry_after_seconds(""), None);
        assert_eq!(retry_after_seconds("soon"), None);
        // u32를 넘는 값도 지어내지 않는다.
        assert_eq!(retry_after_seconds("99999999999999999999"), None);
    }
}
