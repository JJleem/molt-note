//! 로컬 Ollama adapter: 같은 계약 · 같은 실패 · **서버 없이 도는 검증** (ADR-0008 §4.3 · §18).
//!
//! **이 파일은 Ollama 프로세스도 네트워크도 요구하지 않는다.** 실제 왕복을 하는
//! `ai::ollama::network`는 여기서 한 번도 호출되지 않고, 그 자리에는 계약이 같은 test double
//! (`ai::ollama::testing::StubServer`)이 선다. 서버가 없는 상황은 **skip 조건이 아니라 검증
//! 대상이다** — 연결 불가는 §13의 정의된 실패이기 때문이다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use molt_note_lib::ai::ollama::http::TransportError;
use molt_note_lib::ai::ollama::testing::{StubReply, StubServer};
use molt_note_lib::ai::ollama::{OllamaProvider, PROVIDER_ID};
use molt_note_lib::ai::testing::{
    assert_note_ai_provider_contract, FakeNoteAiProvider, CONTRACT_TRANSCRIPT,
};
use molt_note_lib::ai::{Availability, NoteAiProvider, NoteRequest};
use molt_note_lib::domain::{Failure, FailureKind, NoteType};

/// 이 테스트가 "사용자가 설정한 값"으로 쓰는 것들.
///
/// **하나도 실재하지 않는다.** `.invalid`는 어떤 이름 해석도 성공하지 않도록 예약된 TLD이며,
/// 그래서 실수로 실제 연결이 일어날 수 없다. 값이 특이한 이유는 따로 있다 — 이 문자열이
/// 실패 message나 detail에서 발견되면 ADR-0008 §11.3이 깨진 것이다.
const CONFIGURED_HOST: &str = "configured-host-name.invalid";
const CONFIGURED_PORT: &str = "65535";
const CONFIGURED_MODEL: &str = "configured-model-identifier";

fn configured_base_url() -> String {
    format!("http://{CONFIGURED_HOST}:{CONFIGURED_PORT}")
}

/// 설정에서 온 값으로 연결되는 adapter 하나. **주소도 모델도 주입된다.**
fn adapter(server: StubServer) -> OllamaProvider {
    OllamaProvider::new(configured_base_url(), CONFIGURED_MODEL, Arc::new(server))
}

/// 고른 모델이 실제로 설치돼 있는 서버.
fn serving() -> StubServer {
    StubServer::ready().with_models(vec![CONFIGURED_MODEL.to_owned()])
}

// ---------------------------------------------------------------------------
// 계약 — fake와 **같은 묶음**을 통과한다 (ADR-0008 §4.3 · INV-9)
// ---------------------------------------------------------------------------

/// 같은 상황에 놓인 두 구현 — 결정론적 double과 실제 adapter.
type SameSituation = (&'static str, Box<dyn NoteAiProvider>, Box<dyn NoteAiProvider>);

#[test]
fn the_adapter_and_the_double_pass_the_very_same_contract_suite() {
    // 구현이 하나뿐인 추상화는 검증된 추상화가 아니다. 아래 네 상황에서 **같은 함수**가
    // 두 구현에 그대로 적용된다 — adapter 전용으로 느슨하게 다시 쓴 검사가 아니다.
    let pairs: Vec<SameSituation> = vec![
        (
            "정상 응답",
            Box::new(FakeNoteAiProvider::ready()),
            Box::new(adapter(serving())),
        ),
        (
            "연결 불가",
            Box::new(FakeNoteAiProvider::unreachable()),
            Box::new(adapter(StubServer::refusing())),
        ),
        (
            "모델 없음",
            Box::new(FakeNoteAiProvider::without_models()),
            Box::new(adapter(StubServer::ready().with_models(vec![]))),
        ),
        (
            "schema에 어긋난 응답",
            Box::new(FakeNoteAiProvider::generating_text(
                "죄송하지만 JSON으로 답할 수 없습니다",
            )),
            Box::new(adapter(serving().with_generate(StubReply::GeneratedText(
                "죄송하지만 JSON으로 답할 수 없습니다".to_owned(),
            )))),
        ),
    ];

    for (situation, double, real) in pairs {
        assert_note_ai_provider_contract(double.as_ref());
        assert_note_ai_provider_contract(real.as_ref());

        // 같은 상황에서 두 구현이 **같은 종류의 답**을 낸다 — 계약이 요구하는 것은 여기까지다.
        let request = NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT);
        let by_double = double.generate_note(&request);
        let by_real = real.generate_note(&request);

        match (by_double, by_real) {
            (Ok(one), Ok(other)) => assert_eq!(
                one.note.mode(),
                other.note.mode(),
                "{situation}: 요청한 mode의 노트를 낸다"
            ),
            (Err(one), Err(other)) => assert_eq!(
                one.kind, other.kind,
                "{situation}: 같은 상황이 같은 §13 실패가 된다"
            ),
            (one, other) => panic!("{situation}: 한쪽만 성공했다: {one:?} / {other:?}"),
        }
    }
}

#[test]
fn the_adapter_names_itself_for_provenance_and_says_the_data_stays_on_this_device() {
    let descriptor = adapter(serving()).descriptor();

    assert_eq!(descriptor.id, PROVIDER_ID, "ai_notes.provider에 그대로 남는다");
    assert!(descriptor.locality.is_local(), "사용자의 기기에서 도는 서버다 (INV-5)");
    // 화면이 그리는 이름에 연결 대상이 섞이지 않는다.
    assert!(!descriptor.name.contains(CONFIGURED_HOST));
    assert!(!descriptor.name.contains(CONFIGURED_MODEL));
}

#[test]
fn every_mode_comes_back_as_a_note_of_that_mode_with_the_model_recorded() {
    for mode in NoteType::ALL {
        let generation = adapter(serving())
            .generate_note(&NoteRequest::new(mode, CONTRACT_TRANSCRIPT))
            .expect("표본 노트를 낸다");

        assert_eq!(generation.note.mode(), mode);
        assert_eq!(
            generation.model, CONFIGURED_MODEL,
            "provenance는 이 요청이 지정한 모델이다"
        );
    }
}

#[test]
fn the_note_is_recovered_whatever_the_server_calls_the_field_that_holds_it() {
    // 생성 응답의 텍스트 필드 이름은 ADR-0008 §14.3이 UNVERIFIED로 남긴 값이고, 이 Run도
    // 확인하지 못했다. adapter는 그 이름을 지어내는 대신 **이름에 의존하지 않는다.**
    for field in ["response", "text", "output"] {
        let generation = adapter(serving().with_text_field(field))
            .generate_note(&NoteRequest::new(NoteType::Summary, CONTRACT_TRANSCRIPT))
            .expect("노트를 회수한다");

        assert_eq!(generation.note.mode(), NoteType::Summary);
    }
}

// ---------------------------------------------------------------------------
// 벤더 실패 → §13의 domain 공통 실패 (ADR-0008 §13.1 · §13.3)
// ---------------------------------------------------------------------------

#[test]
fn the_four_vendor_failures_become_four_different_domain_failures() {
    // ADR-0008 §13.1의 표 그대로다. 하나로 뭉치면 화면이 구분해 안내할 수 없다.
    let cases: Vec<(&str, Failure, FailureKind, bool)> = vec![
        (
            "연결 거부",
            generate_with(StubServer::refusing()),
            FailureKind::AiProviderUnreachable,
            true,
        ),
        (
            "비정상 상태 코드 (5xx)",
            generate_with(serving().with_generate(StubReply::Status(500))),
            FailureKind::AiRequestFailed,
            true,
        ),
        (
            "비정상 상태 코드 (4xx)",
            generate_with(serving().with_generate(StubReply::Status(404))),
            FailureKind::AiRequestFailed,
            false,
        ),
        (
            "요청한 모델이 목록에 없음",
            generate_with(StubServer::ready().with_models(vec!["another-model".to_owned()])),
            FailureKind::AiModelUnavailable,
            false,
        ),
        (
            "본문을 해석할 수 없음",
            generate_with(serving().with_tags(StubReply::Body("Ollama is running".to_owned()))),
            FailureKind::AiResponseUnusable,
            true,
        ),
        (
            "응답이 기대 schema와 다름",
            generate_with(serving().with_generate(StubReply::GeneratedText("{}".to_owned()))),
            FailureKind::AiResponseUnusable,
            true,
        ),
    ];

    let mut kinds: Vec<FailureKind> = Vec::new();
    for (situation, failure, expected, retryable) in cases {
        assert_eq!(failure.kind, expected, "{situation}");
        assert_eq!(failure.retryable, retryable, "{situation}: 재시도 가치");
        assert!(
            failure.source_data_safe,
            "{situation}: 이 경계는 오디오도 전사도 건드리지 않는다 (INV-3)"
        );
        assert!(!failure.message.trim().is_empty(), "{situation}");
        assert_no_configured_value(&failure, situation);
        kinds.push(failure.kind);
    }

    // 네 벤더 상황이 서로 다른 종류로 남았는지 — 사용자가 할 수 있는 일이 다르기 때문이다.
    kinds.sort_by_key(|kind| kind.as_str());
    kinds.dedup();
    assert_eq!(kinds.len(), 4, "네 실패가 뭉쳐 있다: {kinds:?}");
}

#[test]
fn a_timeout_is_separated_from_a_server_that_is_not_running() {
    // 둘 다 "답이 없다"이지만 사용자가 할 수 있는 일이 다르다 (§13.3).
    let timed_out = generate_with(StubServer::failing(TransportError::TimedOut));
    assert_eq!(timed_out.kind, FailureKind::AiRequestFailed);
    assert!(timed_out.retryable);

    let not_running = generate_with(StubServer::refusing());
    assert_eq!(not_running.kind, FailureKind::AiProviderUnreachable);
    assert_ne!(timed_out.kind, not_running.kind);
}

#[test]
fn availability_separates_a_silent_server_from_one_with_no_models() {
    let Availability::Unavailable(failure) = adapter(StubServer::refusing()).availability() else {
        panic!("응답하지 않는 서버는 Unavailable이다");
    };
    assert_eq!(failure.kind, FailureKind::AiProviderUnreachable);
    assert_no_configured_value(&failure, "availability");

    assert_eq!(
        adapter(StubServer::ready().with_models(vec![])).availability(),
        Availability::NoModels
    );
    assert_eq!(
        adapter(serving()).availability(),
        Availability::Ready {
            models: vec![CONFIGURED_MODEL.to_owned()]
        }
    );
}

#[test]
fn no_failure_on_any_path_carries_the_host_the_port_or_the_model_name() {
    // ADR-0008 §11.3. 경로마다 따로 확인한다 — 한 곳만 새어도 규칙은 깨진다.
    let servers = vec![
        ("연결 거부", StubServer::refusing()),
        ("타임아웃", StubServer::failing(TransportError::TimedOut)),
        ("끝나지 않은 요청", StubServer::failing(TransportError::Incomplete)),
        ("목록이 비2xx", StubServer::ready().with_tags(StubReply::Status(500))),
        (
            "목록 본문이 이상함",
            StubServer::ready().with_tags(StubReply::Body("{}".to_owned())),
        ),
        (
            "모델이 목록에 없음",
            StubServer::ready().with_models(vec!["another-model".to_owned()]),
        ),
        ("설치된 모델 없음", StubServer::ready().with_models(vec![])),
        ("생성이 비2xx", serving().with_generate(StubReply::Status(400))),
        (
            "생성 본문이 노트가 아님",
            serving().with_generate(StubReply::GeneratedText("산문 응답".to_owned())),
        ),
    ];

    for (situation, server) in servers {
        let provider = adapter(server);

        if let Availability::Unavailable(failure) = provider.availability() {
            assert_no_configured_value(&failure, situation);
        }
        let failure = provider
            .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
            .expect_err(situation);
        assert_no_configured_value(&failure, situation);
    }
}

/// 실패가 설정값을 옮기지 않았는지 (ADR-0008 §11.3).
fn assert_no_configured_value(failure: &Failure, situation: &str) {
    let shown = format!("{} {}", failure.message, failure.detail.clone().unwrap_or_default());
    let base_url = configured_base_url();

    for secret in [
        CONFIGURED_HOST,
        CONFIGURED_PORT,
        CONFIGURED_MODEL,
        base_url.as_str(),
    ] {
        assert!(
            !shown.contains(secret),
            "{situation}: 실패가 설정값을 옮겼다 ({secret}): {shown}"
        );
    }
}

/// 그 서버에 대해 생성을 시도했을 때의 실패.
fn generate_with(server: StubServer) -> Failure {
    adapter(server)
        .generate_note(&NoteRequest::new(NoteType::Meeting, CONTRACT_TRANSCRIPT))
        .expect_err("이 상황에서는 노트를 만들 수 없다")
}

// ---------------------------------------------------------------------------
// 경계 — 벤더 지식이 이 디렉터리 밖으로 나가지 않는다 (INV-9) · 테스트가 네트워크에 닿지 않는다
// ---------------------------------------------------------------------------

/// adapter 디렉터리. 이 안에서만 벤더 지식이 허용된다.
const ADAPTER_DIR: &str = "ai/ollama";

/// adapter를 마운트하는 자리. Rust에서 `pub mod ollama;`는 피할 수 없고, **그것은 타입도
/// URL도 파라미터 이름도 에러 코드도 아니다.** 이 파일에 그 이상이 들어오면 아래 검사가 잡는다.
const ADAPTER_MOUNT: &str = "ai/mod.rs";

#[test]
fn no_source_outside_the_adapter_knows_this_vendors_endpoints_or_parameters() {
    // AC6 · INV-9: domain · db · 계약 · P2/P3의 어느 모듈에도 벤더 지식이 없어야 한다.
    let forbidden = [
        "/api/tags",
        "/api/generate",
        "num_ctx",
        "api/chat",
        "OLLAMA_HOST",
    ];

    for path in rust_sources_outside_the_adapter() {
        let source = std::fs::read_to_string(&path).expect("소스 파일을 읽는다");
        let shown = path.display().to_string();

        for knowledge in forbidden {
            assert!(
                !source.contains(knowledge),
                "adapter 밖에 벤더 지식이 있다: {shown} — {knowledge}"
            );
        }

        // 벤더 이름 자체는 adapter를 마운트하는 한 줄에서만 허용된다.
        if !shown.replace('\\', "/").ends_with(ADAPTER_MOUNT) {
            assert!(
                !source.to_uppercase().contains("OLLAMA"),
                "adapter 밖에서 벤더를 이름으로 알고 있다: {shown} (INV-9)"
            );
        }
    }
}

#[test]
fn the_default_address_lives_in_the_settings_and_not_in_the_adapter() {
    // 연결 대상은 주입된다 — adapter 안에 주소가 없다는 것을 소스에서 확인한다.
    for path in rust_sources_in_the_adapter() {
        let source = std::fs::read_to_string(&path).expect("소스 파일을 읽는다");
        assert!(
            !source.contains("localhost") && !source.contains("127.0.0.1"),
            "adapter가 연결 대상을 자기 안에 갖고 있다: {}",
            path.display()
        );
    }

    // 기본 주소를 아는 자리는 설정 하나다 (ADR-0008 §11.1).
    assert_eq!(
        molt_note_lib::domain::settings::DEFAULT_AI_BASE_URL,
        "http://localhost:11434"
    );
}

#[test]
fn only_one_file_in_the_repository_can_open_a_socket() {
    // §18: 자동 검증이 실제 서버에 닿지 않는다는 것은 "그런 테스트를 안 썼다"가 아니라
    // **네트워크에 닿을 수 있는 코드가 어디에 있는지**로 말한다.
    let mut users: Vec<String> = Vec::new();

    for path in rust_sources() {
        let source = std::fs::read_to_string(&path).expect("소스 파일을 읽는다");
        if source.contains("ureq") || source.contains("std::net") || source.contains("TcpStream") {
            users.push(path.display().to_string().replace('\\', "/"));
        }
    }

    assert_eq!(users.len(), 1, "네트워크에 닿는 파일이 하나가 아니다: {users:?}");
    assert!(
        users[0].ends_with("ai/ollama/network.rs"),
        "네트워크에 닿는 파일이 예상 밖이다: {users:?}"
    );
}

#[test]
fn the_test_double_is_pure_values_and_the_tests_never_start_a_server() {
    let double = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ai/ollama/testing.rs"),
    )
    .expect("test double을 읽는다");

    // 이 목록의 문자열은 이 파일 자신에도 들어 있다. 그래서 **이 파일은 검사 대상이 아니다** —
    // `tests/domain_invariants.rs`가 금지 문자열을 다루는 방식과 같다. 이 파일이 실제로 무엇을
    // 쓰는지는 위의 모든 테스트가 보여 준다: 어느 것도 주소를 열지 않고 StubServer만 세운다.
    for network in ["ureq", "std::net", "TcpStream", "TcpListener", "127.0.0.1", "11434"] {
        assert!(
            !double.contains(network),
            "test double이 네트워크에 닿는다: {network}"
        );
    }
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

fn in_the_adapter(path: &Path) -> bool {
    path.display().to_string().replace('\\', "/").contains(ADAPTER_DIR)
}

fn rust_sources_outside_the_adapter() -> Vec<PathBuf> {
    rust_sources()
        .into_iter()
        .filter(|path| !in_the_adapter(path))
        .collect()
}

fn rust_sources_in_the_adapter() -> Vec<PathBuf> {
    rust_sources().into_iter().filter(|path| in_the_adapter(path)).collect()
}
