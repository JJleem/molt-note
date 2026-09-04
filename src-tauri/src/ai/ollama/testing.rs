//! HTTP 경계의 **결정론적 test double** — 이 adapter의 자동 검증이 서버를 요구하지 않는
//! 이유다 (ADR-0008 §18 · `phase-prompt/04` 요구).
//!
//! ```text
//! StubServer::ready()                 목록에 모델이 있고, 요청된 mode의 표본 노트를 낸다
//! StubServer::refusing()              연결이 되지 않는다 (서버가 꺼져 있는 상황)
//! StubServer::failing(error)          지정한 transport 실패
//! .with_models(list)                  목록이 답할 모델들 (빈 목록이면 "모델 없음")
//! .with_generate(reply) / .with_tags(reply)   그 경로의 답을 바꾼다
//! .with_text_field(name)              생성 텍스트를 담을 필드 이름을 바꾼다 (아래를 볼 것)
//! ```
//!
//! **소켓을 열지 않는다.** [`crate::ai::ollama::http::HttpTransport`]를 값으로 이행할 뿐이며,
//! 이 파일에는 네트워크도 HTTP 클라이언트도 없다. 테스트가 실제 서버에 닿을 일도, 서버를
//! 띄울 일도 없다 — 연결 불가는 skip 조건이 아니라 §13의 정의된 실패이고, 그 실패 자체가
//! 검증 대상이다 (그 사실은 `tests/ollama_adapter.rs`가 소스에서 확인한다).
//!
//! ## 생성 텍스트를 담는 필드 이름을 stub이 정한다
//!
//! 그 이름은 **UNVERIFIED다** (ADR-0008 §14.3). 그래서 [`super::wire::generated_note`]는
//! 이름에 의존하지 않고, 이 double은 이름을 바꿔 가며 그 사실을 실제로 검증할 수 있게 한다
//! ([`StubServer::with_text_field`]). 여기 적힌 기본 이름은 **벤더가 그렇게 부른다는 주장이
//! 아니라 stub의 선택**이다.
//!
//! `#[cfg(test)]`가 아닌 것은 통합 테스트(`src-tauri/tests/`)가 별개 crate에서 이것을 쓰기
//! 때문이다 (`crate::transcription::testing`과 같은 이유다).

use std::sync::Mutex;

use serde_json::{json, Map, Value};

use crate::ai::testing::sample_note_text;
use crate::domain::NoteType;

use super::http::{HttpMethod, HttpRequest, HttpResponse, HttpTransport, TransportError};
use super::wire::{self, GENERATE_PATH, TAGS_PATH};

/// [`StubServer::ready`]가 목록에 담는 모델. 테스트가 "고른 모델"로도 쓴다.
pub const MODEL_IN_THE_LIST: &str = "stub-model-in-the-list";

/// stub의 생성 응답이 텍스트를 담는 필드 이름. **벤더의 이름이 아니라 stub의 선택이다.**
pub const STUB_TEXT_FIELD: &str = "stub-generated-text";

/// stub이 받은 요청 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubRequest {
    pub method: HttpMethod,
    pub url: String,
    pub body: Option<String>,
}

/// stub이 한 경로에 대해 낼 답.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StubReply {
    /// 들고 있는 모델 목록을 담은 200 응답.
    Models,
    /// **요청된 mode의** 유효한 표본 노트를 담은 200 응답.
    SampleNote,
    /// 모델이 이 텍스트를 냈다고 가정하는 200 응답.
    GeneratedText(String),
    /// 있는 그대로의 200 본문.
    Body(String),
    /// 본문 없는 비2xx 응답.
    Status(u16),
    /// 응답이 오지 않는다.
    Fail(TransportError),
}

/// 두 경로에 미리 정해 둔 답을 돌려주는 transport.
///
/// **결정론적이다** — 무작위도, 시간도, 파일도, 네트워크도 쓰지 않는다.
#[derive(Debug)]
pub struct StubServer {
    models: Vec<String>,
    tags: StubReply,
    generate: StubReply,
    text_field: String,
    requests: Mutex<Vec<StubRequest>>,
}

impl StubServer {
    /// 목록에 [`MODEL_IN_THE_LIST`]가 있고, 생성이 요청된 mode의 표본 노트를 내는 서버.
    pub fn ready() -> Self {
        Self {
            models: vec![MODEL_IN_THE_LIST.to_owned()],
            tags: StubReply::Models,
            generate: StubReply::SampleNote,
            text_field: STUB_TEXT_FIELD.to_owned(),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// 어느 경로에도 연결되지 않는 서버 — 서버가 실행 중이 아닐 때의 모습이다.
    pub fn refusing() -> Self {
        Self::failing(TransportError::NotConnected)
    }

    /// 어느 경로에서도 같은 transport 실패를 내는 서버.
    pub fn failing(error: TransportError) -> Self {
        Self {
            tags: StubReply::Fail(error),
            generate: StubReply::Fail(error),
            ..Self::ready()
        }
    }

    /// 목록이 답할 모델들. 빈 목록은 "응답했지만 모델이 없다"는 상태다.
    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.models = models;
        self
    }

    pub fn with_tags(mut self, reply: StubReply) -> Self {
        self.tags = reply;
        self
    }

    pub fn with_generate(mut self, reply: StubReply) -> Self {
        self.generate = reply;
        self
    }

    /// 생성 텍스트를 담을 필드 이름을 바꾼다 — adapter가 이름에 의존하지 않는지 확인할 때 쓴다.
    pub fn with_text_field(mut self, field: impl Into<String>) -> Self {
        self.text_field = field.into();
        self
    }

    /// 지금까지 받은 요청들.
    pub fn requests(&self) -> Vec<StubRequest> {
        self.requests.lock().expect("stub 요청 기록을 잠근다").clone()
    }

    /// 목록 응답 본문 — 확인된 필드 이름으로 짓는다 (ADR-0008 §14.2 항목 2).
    fn tags_body(&self) -> String {
        let models: Vec<Value> = self
            .models
            .iter()
            .map(|name| json!({ "name": name, "model": name }))
            .collect();

        json!({ "models": models }).to_string()
    }

    /// 생성 응답 본문. 텍스트가 아닌 문자열도 함께 담는다 — 그런 값이 노트로 오인되지 않는지
    /// 실제 경로에서 확인되어야 한다.
    fn generate_body(&self, text: &str) -> String {
        // 필드 이름이 값(설정 가능)이므로 `json!`의 리터럴 키로 적을 수 없다.
        let mut body = Map::new();
        body.insert("model".to_owned(), json!("stub-served-model"));
        body.insert(self.text_field.clone(), json!(text));
        body.insert("noise".to_owned(), json!("이것은 노트가 아니다"));

        Value::Object(body).to_string()
    }

    fn reply_for(&self, request: &HttpRequest<'_>) -> StubReply {
        if request.url.ends_with(TAGS_PATH) {
            self.tags.clone()
        } else if request.url.ends_with(GENERATE_PATH) {
            self.generate.clone()
        } else {
            panic!(
                "이 adapter가 부르는 경로는 둘뿐이다 ({TAGS_PATH} · {GENERATE_PATH}). 받은 요청: {}",
                request.url
            )
        }
    }

    /// 생성 요청이 어떤 mode를 요구했는지 — **요청에 실린 schema로 판정한다.**
    ///
    /// 그래서 이 double은 adapter가 그 mode의 schema를 실제로 실어 보냈을 때만 답할 수 있다.
    fn requested_mode(body: Option<&str>) -> NoteType {
        let body = body.expect("생성 요청에는 본문이 있어야 한다");
        let value: Value = serde_json::from_str(body).expect("생성 요청 본문은 JSON이어야 한다");
        let format = value.get("format").cloned().unwrap_or(Value::Null);

        NoteType::ALL
            .into_iter()
            .find(|mode| crate::ai::json_schema(*mode) == format)
            .expect("요청이 어느 mode의 출력 형태도 요구하지 않았다")
    }
}

impl HttpTransport for StubServer {
    fn send(&self, request: &HttpRequest<'_>) -> Result<HttpResponse, TransportError> {
        self.requests
            .lock()
            .expect("stub 요청 기록을 잠근다")
            .push(StubRequest {
                method: request.method,
                url: request.url.to_owned(),
                body: request.body.map(str::to_owned),
            });

        match self.reply_for(request) {
            StubReply::Models => Ok(HttpResponse::new(200, self.tags_body())),
            StubReply::SampleNote => {
                let mode = Self::requested_mode(request.body);
                Ok(HttpResponse::new(
                    200,
                    self.generate_body(&sample_note_text(mode)),
                ))
            }
            StubReply::GeneratedText(text) => {
                Ok(HttpResponse::new(200, self.generate_body(&text)))
            }
            StubReply::Body(body) => Ok(HttpResponse::new(200, body)),
            StubReply::Status(status) => Ok(HttpResponse::new(status, String::new())),
            StubReply::Fail(error) => Err(error),
        }
    }
}

/// 이 stub이 아는 경로가 adapter가 부르는 경로와 같은지 — 둘이 어긋나면 stub은 조용히
/// panic하지 않고 여기서 먼저 드러난다.
pub fn known_paths() -> [&'static str; 2] {
    [wire::TAGS_PATH, wire::GENERATE_PATH]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stub_answers_the_two_paths_this_adapter_uses() {
        let server = StubServer::ready();

        for path in known_paths() {
            let url = format!("http://configured-host.invalid:65535{path}");
            let body = crate::ai::ollama::wire::generate_body(
                MODEL_IN_THE_LIST,
                "프롬프트",
                NoteType::Meeting,
                16_384,
            );
            let request = HttpRequest::post_json(&url, &body);

            assert_eq!(server.send(&request).expect("답한다").status, 200);
        }
    }

    #[test]
    fn the_stub_reads_the_mode_from_the_schema_the_adapter_sent() {
        for mode in NoteType::ALL {
            let body = crate::ai::ollama::wire::generate_body(
                MODEL_IN_THE_LIST,
                "프롬프트",
                mode,
                16_384,
            );
            assert_eq!(StubServer::requested_mode(Some(&body)), mode);
        }
    }
}
