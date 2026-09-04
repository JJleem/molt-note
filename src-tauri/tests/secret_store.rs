//! secret이 놓이는 자리 — **경계는 하나이고, 자동 테스트는 실제 자격증명 저장소를 건드리지
//! 않는다** (ADR-0009 §10 · INV-7 · INV-10 · PRODUCT-SPEC §18).
//!
//! "그렇게 안 했다"는 검증이 아니다. 이 파일은 같은 사실을 **서로 다른 네 각도**에서 못박으며,
//! 넷 중 어느 하나만 깨져도 실패한다.
//!
//! ```text
//! 1. 행동   double 하나로 저장 · 조회 · 삭제 · 실패의 전 경로가 지나간다 — Keychain 없이
//! 2. 타입   secret을 담는 값이 Debug로도 직렬화로도 밖으로 나가지 않는다
//! 3. 소스   `keyring`과 `cfg(target_os)`가 platform/secret_store.rs 밖에 하나도 없다
//! 4. 소스   어떤 테스트도 실제 저장소 구현(`OsSecretStore`)을 세우지 않는다
//! ```
//!
//! 3·4번이 소스 검사인 이유는 AI adapter가 소켓을 여는 파일과 같다 — **실제 바깥에 닿는 구현은
//! 실행하지 않는 것이 규약이고, 규약은 실행이 아니라 관찰로 확인한다.** 이 파일이 사용자의
//! Keychain에 항목을 하나라도 만들면 그 규약이 이미 깨진 것이다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::path::{Path, PathBuf};

use molt_note_lib::domain::FailureKind;
use molt_note_lib::platform::secret_store::testing::InMemorySecretStore;
use molt_note_lib::platform::secret_store::{
    Secret, SecretKey, SecretStore, SecretStoreError, SECRET_SERVICE,
};

/// 테스트가 쓰는 값. **실제 자격증명이 아니다** — 이 저장소에 진짜 값을 적지 않는다
/// (ADR-0009 §10.5 · `phase-prompt/05` Important Rules).
const NOT_A_REAL_VALUE: &str = "molt-note-integration-test-value-not-a-real-credential";

/// 이 경계가 사는 유일한 파일. 3번 각도의 대상이다.
const BOUNDARY: &str = "src/platform/secret_store.rs";

// --- 1. 행동: 전 경로가 Keychain 없이 지나간다 -------------------------------------

#[test]
fn the_whole_path_runs_against_a_double_without_any_credential_store() {
    let store: Box<dyn SecretStore> = Box::new(InMemorySecretStore::new());
    let key = SecretKey::NotionIntegrationToken;

    // 아직 저장한 적이 없다 — 실패가 아니라 '아직 없음'이다 (INV-8).
    assert_eq!(store.get(key).expect("묻는 것 자체는 실패가 아니다"), None);

    store
        .set(key, &Secret::new(NOT_A_REAL_VALUE))
        .expect("저장할 수 있어야 한다");
    assert_eq!(
        store
            .get(key)
            .expect("읽을 수 있어야 한다")
            .expect("방금 저장한 값이 있다")
            .expose(),
        NOT_A_REAL_VALUE
    );

    store.delete(key).expect("지울 수 있어야 한다");
    assert_eq!(
        store.get(key).expect("지운 뒤에도 묻는 것은 실패가 아니다"),
        None,
        "지운 뒤에는 '아직 없음'으로 돌아간다"
    );
}

#[test]
fn a_store_failure_arrives_as_a_failure_the_user_can_read() {
    // 실패 경로도 Keychain 없이 지난다 — 저장소를 잠가 볼 필요가 없다 (§18).
    let store = InMemorySecretStore::failing(SecretStoreError::NotAccessible);

    let failure = store
        .get(SecretKey::NotionIntegrationToken)
        .expect_err("실패해야 한다");

    assert_eq!(failure.kind, FailureKind::Storage);
    assert!(!failure.message.is_empty(), "화면에 띄울 문장이 있어야 한다");
    assert!(failure.source_data_safe, "secret 실패는 녹음을 건드리지 않는다");
    assert!(failure.retryable, "접근을 허용하면 풀릴 수 있다");
}

// --- 2. 타입: 값이 밖으로 나가는 통로가 없다 ---------------------------------------

#[test]
fn the_stored_value_never_leaves_through_a_message_a_detail_or_a_debug_line() {
    // ADR-0009 §10.4 — token은 실패 message · detail · 로그 어디에도 남지 않는다.
    let store = InMemorySecretStore::new();
    store
        .set(
            SecretKey::NotionIntegrationToken,
            &Secret::new(NOT_A_REAL_VALUE),
        )
        .expect("저장할 수 있어야 한다");

    // 값을 담은 double을 통째로 찍어도 값이 나오지 않는다.
    let rendered = format!("{store:?}");
    assert!(!rendered.contains(NOT_A_REAL_VALUE), "{rendered}");

    // 이 경계가 만드는 모든 실패를 통째로 찍어도 값도 계정 이름도 없다. 담을 자리 자체가
    // 없기 때문이다 — 규칙이 아니라 타입이 막는다.
    for error in [
        SecretStoreError::Unsupported,
        SecretStoreError::NotAccessible,
        SecretStoreError::OperationFailed,
        SecretStoreError::StoredValueUnusable,
    ] {
        let failure = InMemorySecretStore::failing(error)
            .get(SecretKey::NotionIntegrationToken)
            .expect_err("실패해야 한다");
        let rendered = format!("{failure:?}");

        assert!(!rendered.contains(NOT_A_REAL_VALUE), "{rendered}");
        assert!(
            !rendered.contains(SecretKey::NotionIntegrationToken.account()),
            "계정 이름이 실패에 실려 나간다: {rendered}"
        );
        assert!(!rendered.contains(SECRET_SERVICE), "{rendered}");
    }
}

#[test]
fn the_error_type_has_no_room_for_a_value() {
    // AI adapter의 HTTP 경계가 쓰는 `TransportError`와 같은 설계다 — 변형을 전부 나열하는 이 match는
    // **데이터를 가진 변형이 생기면 컴파일되지 않는다.**
    for error in [
        SecretStoreError::Unsupported,
        SecretStoreError::NotAccessible,
        SecretStoreError::OperationFailed,
        SecretStoreError::StoredValueUnusable,
    ] {
        let text = match error {
            SecretStoreError::Unsupported => "no credential store on this system",
            SecretStoreError::NotAccessible => "credential store not accessible",
            SecretStoreError::OperationFailed => "credential store operation failed",
            SecretStoreError::StoredValueUnusable => "stored credential is not readable",
        };
        assert_eq!(error.as_str(), text);
        assert_eq!(
            error.failure().detail.as_deref(),
            Some(text),
            "detail에 나가는 것은 이 고정 문자열뿐이다"
        );
    }
}

#[test]
fn the_list_of_secrets_this_app_keeps_is_closed() {
    // ADR-0009 §10.1 · §10.6 — 임의의 문자열 키를 받지 않는다. 변형을 전부 나열하는 이
    // match는 자리가 하나 늘어나면 컴파일되지 않으므로, "언젠가 쓸지 모른다"는 이유로
    // Cloud AI 자격증명 자리가 조용히 생기지 않는다.
    let key = SecretKey::NotionIntegrationToken;
    let account = match key {
        SecretKey::NotionIntegrationToken => "notion-integration-token",
    };

    // 사용자가 자기 자격증명 저장소에서 이 항목을 찾아 지울 수 있어야 한다 (§10.2).
    assert_eq!(key.account(), account);
    assert_eq!(SECRET_SERVICE, "molt-note");
}

// --- 3·4. 소스: 경계가 새지 않고, 테스트가 실제 저장소를 세우지 않는다 --------------

#[test]
fn platform_branching_and_the_credential_crate_live_only_inside_the_boundary() {
    // INV-10 · PRODUCT-SPEC §3.1 — 자격증명 저장소를 가르는 `cfg(target_os)`도, 그것을 다루는
    // crate도 `platform/secret_store.rs` 밖에 있으면 안 된다. 밖으로 새는 순간 Windows 구현은
    // 파일 하나가 아니라 여러 자리를 고치는 일이 된다.
    for path in rust_sources(Path::new("src")) {
        let relative = relative(&path);
        if relative == BOUNDARY {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("소스를 읽을 수 있어야 한다");

        assert!(
            !source.contains("keyring"),
            "{relative}이 자격증명 crate를 직접 쓴다 (INV-10)"
        );
        assert!(
            !source.contains("security_framework") && !source.contains("windows_sys"),
            "{relative}이 플랫폼 자격증명 API를 직접 쓴다 (INV-10)"
        );
    }

    // 그리고 그 파일 안에는 실제로 세 자리가 다 있다 — macOS · Windows · 그 밖.
    let boundary = std::fs::read_to_string(Path::new("src/platform/secret_store.rs"))
        .expect("경계 파일을 읽을 수 있어야 한다");
    assert!(boundary.contains("keyring"), "실제 구현이 있어야 한다");
    assert!(
        boundary.contains("target_os = \"macos\"") && boundary.contains("target_os = \"windows\""),
        "macOS 구현과 Windows 구현 경계가 이 파일 안에 있어야 한다"
    );
    assert!(
        boundary.contains("SecretStoreError::Unsupported"),
        "자격증명 저장소가 없는 시스템은 조용히 파일로 떨어지지 않고 실패해야 한다 (§10.2)"
    );
}

#[test]
fn no_automated_test_stands_up_the_real_credential_store() {
    // ADR-0009 §10.2 — `OsSecretStore`를 실행하면 사용자의 Keychain에 항목이 생긴다.
    // 자동 테스트가 쓰는 구현은 메모리 double 하나뿐이며, 그 사실을 여기서 관찰한다
    // (실제로 소켓을 여는 파일을 테스트가 실행하지 않는 것과 같은 규약이다).
    // **이름을 리터럴로 적지 않는다** — 이 파일 자신도 검사 대상이기 때문이다.
    let real_store = ["Os", "SecretStore"].concat();

    let mut checked = 0;
    for path in rust_sources(Path::new("tests"))
        .into_iter()
        .chain(rust_sources(Path::new("src")))
    {
        let relative = relative(&path);
        // 경계 파일 자신은 그 타입을 선언하고 이행한다 — 그것이 이 경계의 존재 이유다.
        if relative == BOUNDARY {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("소스를 읽을 수 있어야 한다");

        for (number, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            // 주석은 이 규약을 설명하기 위해 그 이름을 쓴다. 검사 대상은 코드다.
            if trimmed.starts_with("//") {
                continue;
            }
            assert!(
                !trimmed.contains(&real_store),
                "{relative}:{}이 실제 자격증명 저장소를 세운다 — 자동 테스트는 메모리 double만 쓴다",
                number + 1
            );
        }
        checked += 1;
    }
    assert!(checked > 1, "검사한 파일이 있어야 한다");
}

/// 디렉터리 아래의 `.rs` 파일 전부.
fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory).expect("디렉터리를 읽을 수 있어야 한다");
        for entry in entries {
            let path = entry.expect("항목을 읽을 수 있어야 한다").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|kind| kind == "rs") {
                found.push(path);
            }
        }
    }

    assert!(!found.is_empty(), "{}에 소스가 있어야 한다", root.display());
    found
}

/// 실패 문장에 쓰는, 저장소 기준의 경로 표현.
fn relative(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
