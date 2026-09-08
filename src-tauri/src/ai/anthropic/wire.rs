//! Anthropic Messages API의 **선 위 형태** — 무엇을 보내고 무엇을 읽는가.
//!
//! ```text
//! POST https://api.anthropic.com/v1/messages
//!   x-api-key: <키>
//!   anthropic-version: 2023-06-01
//!   content-type: application/json
//!
//!   { "model": ..., "max_tokens": ..., "messages": [{ "role": "user", "content": ... }] }
//!
//! ← { "content": [{ "type": "text", "text": ... }], "stop_reason": ..., "usage": {...} }
//! ```
//!
//! **이 모듈은 네트워크를 모른다.** 문자열을 만들고 문자열을 읽을 뿐이며, 그래서 왕복 없이
//! 값으로 검증된다 — 앞선 두 adapter의 `wire`가 세운 선례와 같은 자리다.
//!
//! ## 확인한 근거
//!
//! 엔드포인트 · 헤더 이름 · `anthropic-version` 값 · 응답의 `content[]` 모양은 Anthropic이
//! 배포하는 raw HTTP 예제에서 그대로 옮겼다 (`claude-api` skill · `curl/examples.md`).
//! **추측으로 적은 필드가 없다.**

use crate::ai::provider::response_unusable;
use crate::domain::{Failure, NoteType};

/// 이 adapter가 부르는 유일한 엔드포인트.
pub const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

/// `anthropic-version` 헤더 값.
///
/// **날짜형 API 버전이다.** 이 값을 올리면 응답 형태가 달라질 수 있으므로 확인 없이 바꾸지
/// 않는다.
pub const API_VERSION: &str = "2023-06-01";

/// 쓸 모델.
///
/// **1M 컨텍스트다** — 2시간 회의의 전사가 약 30K 토큰이므로 통째로 들어간다. 그래서 이
/// adapter는 전사를 자르지 않는다.
pub const DEFAULT_MODEL: &str = "claude-opus-5";

/// 한 번에 받을 최대 출력 토큰.
///
/// 비스트리밍 요청의 권장값이다 — 이보다 크게 잡으면 HTTP 타임아웃에 걸릴 수 있다.
/// 회의록 하나는 이 안에 충분히 들어간다.
pub const MAX_TOKENS: u32 = 16_000;

/// 보낼 요청 본문을 만든다.
///
/// **프롬프트를 여기서 만들지 않는다** — 무엇을 물을지는 `ai::prompt`가 정하고, 이 모듈은
/// 그 문자열을 규격에 맞는 JSON에 담을 뿐이다. 두 모듈이 같은 것을 두 번 정하지 않는다.
pub fn request_body(prompt: &str, model: &str) -> String {
    let model = if model.trim().is_empty() {
        DEFAULT_MODEL
    } else {
        model.trim()
    };

    format!(
        r#"{{"model":{},"max_tokens":{},"messages":[{{"role":"user","content":{}}}]}}"#,
        json_string(model),
        MAX_TOKENS,
        json_string(prompt)
    )
}

/// 응답 본문에서 **모델이 쓴 텍스트**만 꺼낸다.
///
/// `content`는 블록의 배열이고 `type`이 여러 가지일 수 있다 (`text` · `thinking` 등).
/// **`text` 블록만 이어 붙인다** — 다른 블록을 텍스트로 착각해 노트에 섞으면 JSON 파싱이
/// 깨진다.
pub fn text_from_response(body: &str) -> Result<String, Failure> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|error| {
        rejected("AI provider가 JSON이 아닌 응답을 보냈다", error.to_string())
    })?;

    // 오류 응답은 2xx가 아니므로 provider가 먼저 걸러낸다. 여기 온 것이 오류 모양이면
    // 그것을 텍스트로 오해하지 않고 실패로 만든다.
    if value.get("type").and_then(|kind| kind.as_str()) == Some("error") {
        let detail = value
            .get("error")
            .and_then(|error| error.get("type"))
            .and_then(|kind| kind.as_str())
            .unwrap_or("unknown")
            .to_owned();
        return Err(rejected(
            "AI provider가 요청을 거절했다",
            detail,
        ));
    }

    let blocks = value
        .get("content")
        .and_then(|content| content.as_array())
        .ok_or_else(|| {
            rejected(
                "AI provider의 응답에 내용이 없다",
                "content[] missing".to_owned(),
            )
        })?;

    let mut text = String::new();
    for block in blocks {
        if block.get("type").and_then(|kind| kind.as_str()) != Some("text") {
            continue;
        }
        if let Some(chunk) = block.get("text").and_then(|value| value.as_str()) {
            text.push_str(chunk);
        }
    }

    if text.trim().is_empty() {
        // 안전 판정으로 멈춘 경우가 여기로 온다 (`stop_reason: "refusal"`).
        let stop = value
            .get("stop_reason")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_owned();
        return Err(rejected(
            "AI provider가 노트를 만들지 못했다",
            format!("no text block (stop_reason={stop})"),
        ));
    }

    Ok(text)
}

/// 실제로 응답한 모델 이름. 없으면 요청한 이름을 그대로 쓴다.
///
/// **provenance는 실제로 쓴 것을 적는다** — 전사가 `model.rs`의 파일명을 적는 것과 같다.
pub fn model_from_response(body: &str, requested: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("model")
                .and_then(|model| model.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| requested.to_owned())
}

/// 이 mode가 요구하는 노트 형태 — 프롬프트를 고르는 자리는 `ai::prompt`이며, 여기서는
/// 그 선택을 바꾸지 않는다. 규격상 필요한 것이 없어 이 함수는 값을 그대로 통과시킨다.
pub const fn mode_is_supported(_mode: NoteType) -> bool {
    true
}

/// 문자열 하나를 JSON 문자열 리터럴로.
///
/// **직접 이스케이프하지 않는다.** 전사에는 따옴표 · 줄바꿈 · 이모지가 들어 있고, 손으로 쓴
/// 이스케이프는 그중 하나에서 반드시 깨진다.
fn json_string(value: &str) -> String {
    serde_json::Value::String(value.to_owned()).to_string()
}

/// 응답을 노트로 쓸 수 없다 — §13의 공통 실패 하나로 옮긴다.
///
/// **재시도 가능이다.** 같은 요청이 다음번에 규격에 맞는 답을 낼 수 있다
/// (`response_unusable`의 판단 그대로).
fn rejected(message: &str, detail: String) -> Failure {
    response_unusable(message).with_detail(detail)
}

/// OAuth access token을 쓸 때 함께 보내야 하는 beta 헤더.
///
/// **API 키와 OAuth token은 다른 헤더로 간다.** 키는 `x-api-key`, token은
/// `Authorization: Bearer`이며 token 쪽은 이 헤더가 함께 있어야 받아들여진다.
pub const OAUTH_BETA_HEADER: &str = "anthropic-beta";
pub const OAUTH_BETA_VALUE: &str = "oauth-2025-04-20";

/// 저장된 자격증명이 어느 방식인가.
///
/// 사용자에게 "어느 종류를 넣는지" 묻지 않는다 — **값의 모양이 이미 말하고 있다.**
/// Anthropic API 키는 `sk-ant-` 로 시작하고, OAuth access token은 그렇지 않다.
///
/// **[미검증]** 이 접두사가 앞으로도 유지되는지는 이 저장소가 보장할 수 없다. 다만 틀려도
/// 잃는 것이 크지 않다 — 잘못 고르면 401이 오고, 그 갈래는 이미 §13의 실패로 있다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Credential {
    /// `x-api-key` 로 보낸다.
    ApiKey,
    /// `Authorization: Bearer` + beta 헤더로 보낸다.
    OauthToken,
}

pub fn credential_kind(secret: &str) -> Credential {
    if secret.trim_start().starts_with("sk-ant-") {
        Credential::ApiKey
    } else {
        Credential::OauthToken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_carries_the_prompt_and_the_model() {
        let body = request_body("회의록을 만들어 줘", "claude-opus-5");
        let value: serde_json::Value = serde_json::from_str(&body).expect("JSON이어야 한다");

        assert_eq!(value["model"], "claude-opus-5");
        assert_eq!(value["messages"][0]["role"], "user");
        assert_eq!(value["messages"][0]["content"], "회의록을 만들어 줘");
        assert!(value["max_tokens"].as_u64().unwrap() > 0);
    }

    /// 전사에는 따옴표도 줄바꿈도 들어 있다. **손으로 이스케이프하면 여기서 깨진다.**
    #[test]
    fn a_transcript_with_quotes_and_newlines_stays_valid_json() {
        let nasty = "그가 \"이건 아니죠\"라고 했다.\n다음 줄\t탭\\백슬래시 😀";
        let body = request_body(nasty, "claude-opus-5");
        let value: serde_json::Value = serde_json::from_str(&body).expect("JSON이어야 한다");

        assert_eq!(value["messages"][0]["content"], nasty);
    }

    #[test]
    fn an_empty_model_falls_back_to_the_default() {
        let body = request_body("x", "   ");
        let value: serde_json::Value = serde_json::from_str(&body).expect("JSON");

        assert_eq!(value["model"], DEFAULT_MODEL);
    }

    #[test]
    fn only_text_blocks_become_the_answer() {
        // thinking 블록이 섞여 와도 노트에 들어가지 않는다.
        let body = r#"{"content":[
            {"type":"thinking","thinking":"속으로 생각한 것"},
            {"type":"text","text":"{\"overview\":\"회의\"}"}
        ],"stop_reason":"end_turn"}"#;

        assert_eq!(text_from_response(body).unwrap(), "{\"overview\":\"회의\"}");
    }

    #[test]
    fn several_text_blocks_are_joined_in_order() {
        let body = r#"{"content":[
            {"type":"text","text":"앞"},
            {"type":"text","text":"뒤"}
        ]}"#;

        assert_eq!(text_from_response(body).unwrap(), "앞뒤");
    }

    #[test]
    fn an_error_shaped_response_is_a_failure_not_an_answer() {
        let body = r#"{"type":"error","error":{"type":"invalid_request_error","message":"bad"}}"#;

        let failure = text_from_response(body).expect_err("실패여야 한다");
        assert!(failure.detail.as_deref().unwrap().contains("invalid_request_error"));
    }

    /// 안전 판정으로 멈춘 응답. **빈 문자열을 노트로 넘기지 않는다.**
    #[test]
    fn a_refusal_with_no_text_is_a_failure() {
        let body = r#"{"content":[],"stop_reason":"refusal"}"#;

        let failure = text_from_response(body).expect_err("실패여야 한다");
        assert!(failure.detail.as_deref().unwrap().contains("refusal"));
    }

    #[test]
    fn a_body_that_is_not_json_is_a_failure() {
        assert!(text_from_response("<html>502</html>").is_err());
    }

    #[test]
    fn the_reported_model_is_what_the_response_says() {
        let body = r#"{"content":[{"type":"text","text":"x"}],"model":"claude-opus-5"}"#;
        assert_eq!(model_from_response(body, "요청한-것"), "claude-opus-5");
    }

    #[test]
    fn a_response_without_a_model_falls_back_to_what_was_requested() {
        let body = r#"{"content":[{"type":"text","text":"x"}]}"#;
        assert_eq!(model_from_response(body, "요청한-것"), "요청한-것");
    }

    /// 확인 없이 바꾸면 응답 형태가 달라질 수 있는 값들. 바뀌면 이 테스트가 알린다.
    #[test]
    fn an_api_key_and_an_oauth_token_are_told_apart_by_shape() {
        // 사용자에게 종류를 묻지 않는다 — 값의 모양이 이미 말하고 있다.
        assert_eq!(credential_kind("sk-ant-api03-abc"), Credential::ApiKey);
        assert_eq!(credential_kind("  sk-ant-oat01-abc"), Credential::ApiKey);
        assert_eq!(credential_kind("eyJhbGciOi..."), Credential::OauthToken);
        assert_eq!(credential_kind(""), Credential::OauthToken);
    }

    #[test]
    fn the_wire_constants_are_the_documented_ones() {
        assert_eq!(ENDPOINT, "https://api.anthropic.com/v1/messages");
        assert_eq!(API_VERSION, "2023-06-01");
        assert_eq!(DEFAULT_MODEL, "claude-opus-5");
    }
}
