//! Notion adapter: 확인된 API 계약대로 보내고, 벤더 실패를 §13의 제품 상태로 옮기며,
//! **Notion 없이 도는 검증** (ADR-0009 §5 · §9 · PRODUCT-SPEC §18 · `phase-prompt/05` 요구 14).
//!
//! ⚠️ **이 파일은 실제 Notion에 한 번도 요청하지 않는다.** 실제 왕복을 하는
//! `notion::network`는 여기서 한 번도 만들어지지 않고, 그 자리에는 계약이 같은 test double
//! (`notion::testing::StubServer`)이 선다. 그래서 어떤 워크스페이스도 오염되지 않고, 어떤
//! 자격증명도 필요하지 않다.
//!
//! **이 파일에는 실제 token이 없다.** 아래 상수들은 전부 지어낸 값이며, 그 값이 실패 문장이나
//! 로그에서 발견되면 INV-7이 깨진 것이다 — 그것을 확인하는 것이 이 파일의 검사 중 하나다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use molt_note_lib::domain::FailureKind;
use molt_note_lib::notion::client::NOTION_FAILURE_KINDS;
use molt_note_lib::notion::http::TransportError;
use molt_note_lib::notion::testing::{StubReply, StubRequest, StubServer, CREATED_PAGE_ID};
use molt_note_lib::notion::wire::ApiErrorCode;
use molt_note_lib::notion::{NotionClient, NotionFailure, PageId, RetryAfter, NOTION_VERSION};
use molt_note_lib::platform::secret_store::Secret;

/// 이 테스트가 "사용자가 설정한 값"으로 쓰는 것들. **하나도 실재하지 않는다.**
const NOT_A_REAL_TOKEN: &str = "ntn-test-double-value-not-a-real-credential";
const PARENT_PAGE_ID: &str = "parent-page-identifier-not-a-real-page";
const DOCUMENT: &str = "# 3DGS Study #04\n\n## Overview\n첫 문단.";

fn token() -> Secret {
    Secret::new(NOT_A_REAL_TOKEN)
}

fn client(server: StubServer) -> NotionClient {
    NotionClient::new(Arc::new(server))
}

/// 그 서버에 페이지를 만들려 했을 때의 실패.
fn create_failure(server: StubServer) -> NotionFailure {
    client(server)
        .create_page(&token(), PARENT_PAGE_ID, DOCUMENT)
        .expect_err("이 상황에서는 페이지를 만들 수 없다")
}

/// 세 경로가 보낸 요청 전부 — 연결 확인 · 생성 · 이어붙이기.
fn requests_of_every_call() -> Vec<StubRequest> {
    let server = Arc::new(StubServer::ready());
    let client = NotionClient::new(server.clone());

    client.check_connection(&token()).expect("stub이 답한다");
    let page = client
        .create_page(&token(), PARENT_PAGE_ID, DOCUMENT)
        .expect("stub이 페이지를 만든다");
    client
        .append_markdown(&token(), &page, "## Transcript\n### 00:00:03\n안녕하세요.")
        .expect("stub이 이어 붙인다");

    server.requests()
}

// ---------------------------------------------------------------------------
// 요청 조립 — ADR-0009 §5의 계약 그대로 (P5-AC4)
// ---------------------------------------------------------------------------

#[test]
fn every_request_carries_bearer_auth_and_the_documented_api_version() {
    let requests = requests_of_every_call();
    assert_eq!(requests.len(), 3, "세 경로가 각각 한 번씩 나간다");

    for request in &requests {
        assert_eq!(
            request.header("Authorization").as_deref(),
            Some(format!("Bearer {NOT_A_REAL_TOKEN}").as_str()),
            "{} {}: integration token은 Bearer로 실린다",
            request.method,
            request.url
        );
        assert_eq!(
            request.header("Notion-Version").as_deref(),
            Some("2026-03-11"),
            "{}: API 버전 헤더가 없거나 다르다",
            request.url
        );
        // 본문이 있는 요청에만 `Content-Type`을 싣는다 (ADR-0009 §5.2).
        assert_eq!(
            request.header("Content-Type").as_deref(),
            request.body.as_ref().map(|_| "application/json"),
            "{}: Content-Type이 본문 유무와 어긋난다",
            request.url
        );
    }
}

#[test]
fn the_api_version_is_the_documented_one_and_never_a_date_we_looked_it_up() {
    // PRODUCT-SPEC §14.9.2 · `phase-prompt/05` Important Rules.
    assert_eq!(NOTION_VERSION, "2026-03-11");

    for not_a_version in [
        "2026-09-01", // §14.9.1을 조사한 날짜
        "2026-09-04", // §14.9.2를 재확인한 날짜
        "2026-07-28", // Notion **MCP 프로토콜** 버전
        "2025-09-03",
        "2022-06-28",
    ] {
        assert_ne!(NOTION_VERSION, not_a_version);
    }

    for request in requests_of_every_call() {
        assert_eq!(request.header("Notion-Version").as_deref(), Some(NOTION_VERSION));
    }
}

#[test]
fn creating_a_page_sends_the_markdown_string_and_never_a_block_array() {
    let requests = requests_of_every_call();
    let create = &requests[1];

    assert_eq!(create.method.as_str(), "POST");
    assert_eq!(create.url, "https://api.notion.com/v1/pages");

    let body: serde_json::Value =
        serde_json::from_str(create.body.as_deref().expect("본문이 있다")).expect("본문은 JSON이다");

    assert_eq!(body["parent"]["page_id"], PARENT_PAGE_ID);
    assert_eq!(body["markdown"], DOCUMENT);

    // §14.9.1: `markdown`과 `children`/`content`는 같은 요청에 함께 쓸 수 없다.
    // 그리고 이 앱은 블록 JSON 경로를 아예 만들지 않는다 (ADR-0009 §5.4).
    let object = body.as_object().expect("객체다");
    for forbidden in ["children", "content", "blocks", "properties", "allow_async"] {
        assert!(object.get(forbidden).is_none(), "{forbidden}가 함께 나갔다: {body}");
    }
}

#[test]
fn a_newline_in_the_document_goes_out_escaped_and_arrives_as_a_real_newline() {
    // §14.9.1: markdown은 실제 개행을 기대하며, JSON에서는 `\n`이다.
    let requests = requests_of_every_call();
    let raw = requests[1].body.as_deref().expect("본문이 있다");

    assert!(raw.contains(r"\n"), "직렬화가 개행을 이스케이프하지 않았다");
    assert!(!raw.contains('\n'), "요청 본문에 날 개행이 들어갔다");

    let body: serde_json::Value = serde_json::from_str(raw).expect("본문은 JSON이다");
    assert_eq!(
        body["markdown"].as_str().expect("문자열이다"),
        DOCUMENT,
        "받는 쪽에는 원문 그대로 도착한다"
    );
}

#[test]
fn appending_asks_for_insertion_at_the_end_of_that_one_page() {
    let requests = requests_of_every_call();
    let append = &requests[2];

    assert_eq!(append.method.as_str(), "PATCH");
    assert_eq!(
        append.url,
        format!("https://api.notion.com/v1/pages/{CREATED_PAGE_ID}/markdown")
    );

    let body: serde_json::Value =
        serde_json::from_str(append.body.as_deref().expect("본문이 있다")).expect("본문은 JSON이다");

    assert_eq!(body["type"], "insert_content");
    assert_eq!(
        body["insert_content"]["content"],
        "## Transcript\n### 00:00:03\n안녕하세요."
    );
    assert_eq!(body["insert_content"]["position"]["type"], "end");

    // 페이지를 바꿔치기하거나 지우는 경로는 이 adapter에 없다 (ADR-0009 §8.3).
    let raw = append.body.as_deref().expect("본문이 있다");
    assert!(!raw.contains("replace_content"));
    assert!(!raw.contains("update_content"));
}

#[test]
fn the_connection_check_asks_who_the_token_belongs_to() {
    let requests = requests_of_every_call();
    let check = &requests[0];

    assert_eq!(check.method.as_str(), "GET");
    assert_eq!(check.url, "https://api.notion.com/v1/users/me");
    assert_eq!(check.body, None, "연결 확인에는 본문이 없다");
}

// ---------------------------------------------------------------------------
// `Retry-After` — 정수 초로 읽는다 (P5-AC5 · ADR-0009 §9)
// ---------------------------------------------------------------------------

#[test]
fn a_rate_limited_response_says_how_long_to_wait_in_whole_seconds() {
    // §14.9.1 문서 인용: "The header value is an integer number of seconds."
    // `429`와 `529` **둘 다** `Retry-After`를 준다 (`phase-prompt/05` P-5).
    for status in [429, 529] {
        for (header, expected) in [("30", 30_u32), (" 120 ", 120), ("0", 0)] {
            let failure = create_failure(
                StubServer::ready()
                    .with_create_page(StubReply::rate_limited(status, Some(header))),
            );

            assert_eq!(failure.kind(), FailureKind::NotionRateLimited, "HTTP {status}");
            assert_eq!(
                failure.wait(),
                Some(RetryAfter::Seconds(expected)),
                "HTTP {status}: `Retry-After: {header}`를 그대로 읽지 않았다"
            );
            assert!(failure.failure().retryable, "기다렸다가 다시 보낸다");
        }
    }
}

#[test]
fn a_wait_this_app_cannot_read_is_left_as_unspecified_instead_of_invented() {
    // HTTP-date는 확인된 계약이 아니다 (ADR-0009 §9.2-5). 얼마나 기다릴지는 전송 순서를
    // 소유한 쪽이 자기 backoff로 정하며, 그 사실이 값에 그대로 남는다.
    for header in [None, Some("Wed, 21 Oct 2026 07:28:00 GMT"), Some("1.5"), Some("soon")] {
        let failure = create_failure(
            StubServer::ready().with_create_page(StubReply::rate_limited(429, header)),
        );

        assert_eq!(failure.kind(), FailureKind::NotionRateLimited);
        assert_eq!(
            failure.wait(),
            Some(RetryAfter::Unspecified),
            "읽지 못한 헤더에서 숫자를 지어냈다: {header:?}"
        );
        assert_eq!(failure.wait().and_then(RetryAfter::seconds), None);
    }
}

#[test]
fn nothing_but_a_rate_limit_ever_asks_the_caller_to_wait() {
    // ADR-0009 §9.3: 자동 재시도가 있는 응답은 `429`와 `529`뿐이다.
    for reply in [
        StubReply::error(400, ApiErrorCode::ValidationError),
        StubReply::error(401, ApiErrorCode::Unauthorized),
        StubReply::error(404, ApiErrorCode::ObjectNotFound),
        StubReply::error(409, ApiErrorCode::ConflictError),
        StubReply::error(500, ApiErrorCode::InternalServerError),
        StubReply::error(503, ApiErrorCode::ServiceUnavailable),
        StubReply::Status(418),
        StubReply::Fail(TransportError::TimedOut),
    ] {
        let failure = create_failure(StubServer::ready().with_create_page(reply));
        assert_eq!(failure.wait(), None, "{failure:?}가 대기를 지시했다");
    }
}

// ---------------------------------------------------------------------------
// 벤더 실패 → §13의 제품 상태 (ADR-0009 §9.3)
// ---------------------------------------------------------------------------

#[test]
fn the_documented_error_codes_become_the_states_the_user_can_act_on() {
    let cases: Vec<(&str, NotionFailure, FailureKind, bool)> = vec![
        (
            "token이 거절됐다",
            create_failure(
                StubServer::ready()
                    .with_create_page(StubReply::error(401, ApiErrorCode::Unauthorized)),
            ),
            FailureKind::NotionAuthFailed,
            false,
        ),
        (
            "부모 페이지가 공유되지 않았다",
            create_failure(
                StubServer::ready()
                    .with_create_page(StubReply::error(403, ApiErrorCode::RestrictedResource)),
            ),
            FailureKind::NotionDestinationUnavailable,
            false,
        ),
        (
            "부모 페이지를 찾지 못했다",
            create_failure(
                StubServer::ready()
                    .with_create_page(StubReply::error(404, ApiErrorCode::ObjectNotFound)),
            ),
            FailureKind::NotionDestinationUnavailable,
            false,
        ),
        (
            "요청이 잘못됐다",
            create_failure(
                StubServer::ready()
                    .with_create_page(StubReply::error(400, ApiErrorCode::ValidationError)),
            ),
            FailureKind::NotionRequestFailed,
            false,
        ),
        (
            "속도 제한",
            create_failure(
                StubServer::ready().with_create_page(StubReply::rate_limited(429, Some("5"))),
            ),
            FailureKind::NotionRateLimited,
            true,
        ),
        (
            "서버 쪽 오류",
            create_failure(
                StubServer::ready()
                    .with_create_page(StubReply::error(500, ApiErrorCode::InternalServerError)),
            ),
            FailureKind::NotionRequestFailed,
            true,
        ),
        (
            "네트워크가 없다",
            create_failure(StubServer::refusing()),
            FailureKind::NotionRequestFailed,
            true,
        ),
        (
            "2xx인데 페이지를 확인할 수 없다",
            create_failure(
                StubServer::ready()
                    .with_create_page(StubReply::Body(r#"{"object":"async_task"}"#.to_owned())),
            ),
            FailureKind::NotionResponseUnusable,
            false,
        ),
    ];

    let mut kinds: Vec<FailureKind> = Vec::new();
    for (situation, failure, expected, retryable) in cases {
        assert_eq!(failure.kind(), expected, "{situation}");
        assert_eq!(failure.failure().retryable, retryable, "{situation}: 재시도 가치");
        assert!(
            failure.failure().source_data_safe,
            "{situation}: 전송 실패는 오디오도 전사도 노트도 건드리지 않는다 (INV-3)"
        );
        assert!(!failure.failure().message.trim().is_empty(), "{situation}");
        assert!(
            NOTION_FAILURE_KINDS.contains(&failure.kind()),
            "{situation}: 선언되지 않은 실패 종류가 나왔다"
        );
        kinds.push(failure.kind());
    }

    kinds.sort_by_key(|kind| kind.as_str());
    kinds.dedup();
    assert_eq!(
        kinds.len(),
        NOTION_FAILURE_KINDS.len(),
        "사용자가 할 수 있는 일이 다른 상황들이 뭉쳐 있다: {kinds:?}"
    );
}

#[test]
fn a_response_that_never_says_a_page_was_made_is_not_reported_as_success() {
    // 요청하지 않은 `202`가 그 자리다 (ADR-0009 §7.3) — 우리는 `allow_async`를 보내지 않으므로
    // 그 응답을 해석할 수 없다. 페이지가 만들어졌을 수 있으니 조용히 다시 보내지 않는다.
    let failure = create_failure(
        StubServer::ready().with_create_page(StubReply::Body(
            r#"{"object":"async_task","id":"task-id","status_url":"…"}"#.to_owned(),
        )),
    );

    assert_eq!(failure.kind(), FailureKind::NotionResponseUnusable);
    assert!(
        !failure.failure().retryable,
        "그대로 다시 보내면 페이지가 둘이 될 수 있다"
    );
}

#[test]
fn the_created_page_comes_back_as_a_value_the_caller_can_persist() {
    // 부분 성공 뒤의 재시도가 중복 페이지를 만들지 않으려면 이 값이 곧바로 남아야 한다
    // (ADR-0009 §8.4-3). 이 adapter의 몫은 그 값을 정확히 돌려주는 것까지다.
    let page = client(StubServer::ready())
        .create_page(&token(), PARENT_PAGE_ID, DOCUMENT)
        .expect("stub이 페이지를 만든다");

    assert_eq!(page.as_str(), CREATED_PAGE_ID);
    assert_eq!(PageId::parse(page.as_str()).as_ref(), Some(&page));
}

// ---------------------------------------------------------------------------
// token · destination이 새어 나가지 않는다 (P5-AC7 · INV-7 · ADR-0009 §10.4)
// ---------------------------------------------------------------------------

#[test]
fn no_failure_on_any_path_carries_the_token_or_the_destination() {
    // 경로마다 따로 확인한다 — 한 곳만 새어도 규칙은 깨진다. stub의 오류 본문은 우리가 보낸
    // 주소를 그대로 되돌려주므로(`Could not process the request for …`), 그 문장을 옮기는 순간
    // 이 검사가 잡는다.
    let situations: Vec<(&str, StubReply)> = vec![
        ("연결되지 않음", StubReply::Fail(TransportError::NotConnected)),
        ("타임아웃", StubReply::Fail(TransportError::TimedOut)),
        ("끝나지 않은 요청", StubReply::Fail(TransportError::Incomplete)),
        ("token 거절", StubReply::error(401, ApiErrorCode::Unauthorized)),
        ("공유되지 않음", StubReply::error(403, ApiErrorCode::RestrictedResource)),
        ("페이지 없음", StubReply::error(404, ApiErrorCode::ObjectNotFound)),
        ("요청 거절", StubReply::error(400, ApiErrorCode::ValidationError)),
        ("속도 제한", StubReply::rate_limited(429, Some("30"))),
        ("서버 오류", StubReply::error(503, ApiErrorCode::ServiceUnavailable)),
        ("본문 없는 상태 코드", StubReply::Status(500)),
        ("읽을 수 없는 2xx", StubReply::Body("{}".to_owned())),
    ];

    let page = PageId::parse(CREATED_PAGE_ID).expect("모양이 맞다");

    for (situation, reply) in situations {
        let server = StubServer::ready()
            .with_users_me(reply.clone())
            .with_create_page(reply.clone())
            .with_append(reply.clone());
        let client = client(server);

        let mut failures = vec![
            client
                .create_page(&token(), PARENT_PAGE_ID, DOCUMENT)
                .expect_err(situation),
        ];
        if let Err(failure) = client.check_connection(&token()) {
            failures.push(failure);
        }
        if let Err(failure) = client.append_markdown(&token(), &page, DOCUMENT) {
            failures.push(failure);
        }

        for failure in failures {
            assert_no_secret_value(&failure, situation);
        }
    }
}

/// 실패가 token이나 destination을 옮기지 않았는지 (ADR-0009 §10.4).
///
/// 사용자가 읽는 문장(`message`) · 기술적 원인(`detail`) · **그리고 `Debug` 출력**을 함께 본다.
/// 셋 중 하나라도 새면 그 값은 언젠가 로그나 화면에 도달한다.
fn assert_no_secret_value(failure: &NotionFailure, situation: &str) {
    let shown = format!(
        "{} {} {failure:?}",
        failure.failure().message,
        failure.failure().detail.clone().unwrap_or_default()
    );

    for secret in [
        NOT_A_REAL_TOKEN,
        PARENT_PAGE_ID,
        CREATED_PAGE_ID,
        "Bearer",
        "api.notion.com",
    ] {
        assert!(
            !shown.contains(secret),
            "{situation}: 실패가 값을 옮겼다 ({secret}): {shown}"
        );
    }
}

// ---------------------------------------------------------------------------
// 경계 — 벤더 지식이 이 디렉터리 밖으로 나가지 않는다 · 테스트가 Notion에 닿지 않는다 (P5-AC6)
// ---------------------------------------------------------------------------

/// adapter 디렉터리. 이 안에서만 Notion API 지식이 허용된다.
const ADAPTER_DIR: &str = "src/notion";

/// adapter를 마운트하는 자리. Rust에서 `pub mod notion;`은 피할 수 없고, **그것은 주소도
/// 헤더 이름도 파라미터 이름도 오류 코드도 아니다.**
const ADAPTER_MOUNT: &str = "src/lib.rs";

#[test]
fn no_source_outside_the_adapter_knows_the_notion_api() {
    // INV-9의 태도 그대로다 — 이 API가 바뀔 때 흔들리는 곳이 한 디렉터리로 남는다.
    let forbidden = [
        "api.notion.com",
        "Notion-Version",
        "2026-03-11",
        "/v1/pages",
        "/v1/users/me",
        "insert_content",
        "restricted_resource",
        "object_not_found",
        "Retry-After",
    ];

    for path in rust_sources() {
        let shown = path.display().to_string().replace('\\', "/");
        if shown.contains(ADAPTER_DIR) || shown.ends_with(ADAPTER_MOUNT) {
            continue;
        }

        let source = std::fs::read_to_string(&path).expect("소스 파일을 읽는다");
        for knowledge in forbidden {
            assert!(
                !source.contains(knowledge),
                "adapter 밖에 Notion API 지식이 있다: {shown} — {knowledge}"
            );
        }
    }
}

#[test]
fn only_the_network_file_of_this_adapter_can_open_a_socket() {
    // 이 adapter 안에서 소켓을 열 수 있는 파일이 하나인지 — 나머지는 값에서 값을 만든다.
    let mut users: Vec<String> = Vec::new();

    for path in adapter_sources() {
        let source = std::fs::read_to_string(&path).expect("소스 파일을 읽는다");
        if source.contains("ureq") || source.contains("std::net") || source.contains("TcpStream") {
            users.push(path.display().to_string().replace('\\', "/"));
        }
    }

    assert_eq!(users.len(), 1, "네트워크에 닿는 파일이 하나가 아니다: {users:?}");
    assert!(users[0].ends_with("notion/network.rs"), "{users:?}");
}

#[test]
fn the_test_double_is_pure_values_and_no_test_here_reaches_notion() {
    let double = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/notion/testing.rs"),
    )
    .expect("test double을 읽는다");

    for network in ["ureq", "std::net", "TcpStream", "TcpListener", "reqwest"] {
        assert!(
            !double.contains(network),
            "test double이 네트워크에 닿는다: {network}"
        );
    }

    // 이 파일이 실제 transport를 세우지 않는다는 것도 소스에서 확인한다. 이름을 조각으로
    // 적는 이유는 하나다 — 온전한 이름을 여기 적으면 이 검사가 자기 자신에 걸린다.
    let this_file = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/notion_adapter.rs"),
    )
    .expect("이 파일을 읽는다");
    let real_transport = format!("Ureq{}Transport", "Notion");

    assert!(
        !this_file.contains(&real_transport),
        "이 테스트가 실제 transport를 세운다"
    );
    assert!(
        this_file.contains("StubServer"),
        "이 테스트가 double을 쓴다는 사실이 소스에 없다"
    );
}

/// `src` 아래의 모든 `.rs` 파일.
fn rust_sources() -> Vec<PathBuf> {
    fn walk(directory: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(directory).expect("소스 디렉터리를 읽는다") {
            let path = entry.expect("디렉터리 항목을 읽는다").path();
            if path.is_dir() {
                walk(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut files);
    files
}

fn adapter_sources() -> Vec<PathBuf> {
    rust_sources()
        .into_iter()
        .filter(|path| {
            path.display()
                .to_string()
                .replace('\\', "/")
                .contains(ADAPTER_DIR)
        })
        .collect()
}
