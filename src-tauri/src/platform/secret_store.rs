//! secret이 놓이는 자리 — **OS 자격증명 저장소 경계** (ADR-0009 §10 · INV-7 · INV-10).
//!
//! ```text
//! SecretKey                       앱이 보관하는 secret의 닫힌 목록 (문자열 키를 받지 않는다)
//! Secret                          값. Debug도 Display도 내용을 내지 않고 직렬화되지 않는다
//! SecretStore                     저장 · 조회 · 삭제. 테스트가 대체할 수 있는 자리
//! OsSecretStore                   이 기기의 OS 자격증명 저장소 ★ 자동 테스트가 실행하지 않는다
//! testing::InMemorySecretStore    자동 테스트가 쓰는 유일한 구현
//! ```
//!
//! `platform`의 다른 셋과 같은 종류의 자리다 — **바깥 세계를 읽는 곳**이며, 그 바깥을 값으로
//! 바꿔 넣으면 나머지 코드가 실제 Keychain 없이 검증된다 (PRODUCT-SPEC §18).
//!
//! ## 자동 테스트는 실제 자격증명 저장소를 건드리지 않는다 (ADR-0009 §10.2)
//!
//! AI adapter가 실제로 소켓을 여는 파일과 같은 규약이다: 실제 바깥에 닿는 구현은 이 파일 안에
//! 있지만 **Gate는 그것을 컴파일만 하고 실행하지 않으며**, 테스트는 언제나
//! [`testing::InMemorySecretStore`]를 쓴다. 그 사실은 `tests/secret_store.rs`가 소스에서
//! 확인한다 — 규칙을 적어 두는 것으로 끝내지 않는다.
//!
//! ## 값이 새어 나갈 통로를 규칙이 아니라 타입으로 막는다
//!
//! [`SecretStoreError`]에는 문자열이 없다. AI adapter의 HTTP 경계가 쓰는 `TransportError`와 같은
//! 이유이며 여기서는 더 직접적이다 — 자격증명 라이브러리의 오류 문장에는 서비스/계정 이름이
//! 그대로 들어 있는 경우가 흔하고, 그것이 [`Failure::message`]·[`Failure::detail`]·로그로
//! 옮겨 가면 ADR-0009 §10.4를 어긴다. **옮길 수 있는 문자열이 애초에 없으면 새어 나갈 수도
//! 없다.** 그래서 이 경계 밖으로 나가는 문장은 [`SecretStoreError::as_str`]의 고정 문자열뿐이며,
//! 그 목록 어디에도 secret도 계정 이름도 없다.
//!
//! ## 여기까지만 만든다 (ADR-0009 §10.6)
//!
//! Cloud AI provider의 자격증명 · token 자동 갱신 · 여러 workspace · 키 회전은 만들지 않는다.
//! [`SecretKey`]에 변형이 하나뿐인 것이 그 사실을 코드로 적은 것이다 — **언젠가 필요할지
//! 모른다는 이유로 자리를 파 두지 않는다** (PRODUCT-SPEC §20.5 · ADR-0008 §11.3).

use std::fmt;
use std::sync::Arc;

use crate::domain::{Failure, FailureKind};

/// 이 기기의 실제 자격증명 저장소 하나 — **앱이 지나는 경로다** (ADR-0009 §10.2).
///
/// 실제 구현을 **세우는 자리도 이 파일 하나로 묶는다.** 바깥의 코드는 [`SecretStore`]만 알고,
/// 어느 구현이 서는지는 여기서 정해진다 (`crate::ai::provider_for`가 벤더 선택을 한 자리에
/// 묶는 것과 같은 규칙이며, INV-10의 태도 그대로다).
///
/// ★ **자동 테스트는 이 함수를 부르지 않는다.** 부르면 사용자의 Keychain에 항목이 생긴다.
/// 테스트가 쓰는 것은 [`testing::InMemorySecretStore`] 하나이며, 그 사실은
/// `tests/secret_store.rs`가 소스에서 확인한다 — 이 함수가 생겼다고 그 검사가 약해지지 않는다.
pub fn app_secret_store() -> Arc<dyn SecretStore> {
    Arc::new(OsSecretStore)
}

/// 자격증명 저장소에서 이 앱의 항목을 묶는 이름.
///
/// **결정론적으로 정한다** (ADR-0009 §10.2) — 사용자가 자기 Keychain에서 이 항목을 직접 찾아
/// 지울 수 있어야 하기 때문이다. 기기마다 다른 값을 만들면 그것이 불가능해진다.
pub const SECRET_SERVICE: &str = "molt-note";

/// 앱이 보관해야 하는 secret의 **닫힌 목록** (ADR-0009 §10.1).
///
/// 임의의 문자열 키를 받지 않는다. 문자열을 받는 순간 이 경계는 "아무거나 담는 자격증명
/// 저장소"가 되고, 그때부터 무엇이 저장돼 있는지 코드를 읽어서는 알 수 없게 된다.
/// 필요해지면 **변형을 하나 더한다.**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKey {
    /// Notion integration이 요구하는 자격증명 (ADR-0009 §5.2의 `Authorization` 헤더).
    NotionIntegrationToken,
}

impl SecretKey {
    /// 자격증명 저장소에서 이 항목을 찾는 계정 이름. [`SECRET_SERVICE`]와 짝이다.
    ///
    /// **이것은 값이 아니라 자리의 이름이다.** 저장된 값은 여기 없다.
    pub const fn account(self) -> &'static str {
        match self {
            Self::NotionIntegrationToken => "notion-integration-token",
        }
    }
}

/// 보관되는 secret 값 하나.
///
/// `String`이 아니라 별도 타입인 이유는 하나다 — `String`이면 로그 · 실패 문장 · IPC 응답
/// 어디로든 갈 수 있다. 이 타입은 그 통로를 **타입으로** 막는다.
///
/// - [`fmt::Debug`]를 손으로 이행해 내용을 내지 않는다. `derive`였다면 `{:?}` 한 번에 샌다.
/// - [`fmt::Display`]가 **없다.** `{}`로 찍히는 일이 문법적으로 불가능하다.
/// - `serde` 파생이 **없다.** 직렬화될 수 있으면 언젠가 직렬화된다 (ADR-0009 §10.1).
///
/// 값을 꺼내는 자리는 [`Secret::expose`] 하나이며, 그 이름이 "지금 secret을 꺼내고 있다"는
/// 사실을 호출부에 남긴다.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 값을 꺼낸다. **부르는 자리가 곧 secret이 지나가는 자리다.**
    ///
    /// 지금 이 값이 향하는 곳은 Notion 요청의 `Authorization` 헤더 하나뿐이다
    /// (ADR-0009 §10.4). 그 밖의 어디로도 옮기지 않는다.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// **내용을 내지 않는다.** 길이도 앞글자도 내지 않는다 — 그런 것도 값에 대한 정보다.
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

/// 자격증명 저장소가 요청한 일을 하지 못한 이유. **값을 담지 않는다.**
///
/// AI adapter의 HTTP 경계가 쓰는 `TransportError`와 같은 설계다 (ADR-0009 §10.4) — 변형만 있고
/// 데이터가 없으므로, 저장된 secret도 계정 이름도 OS 오류 문장도 이 타입을 통해 밖으로 나갈 수
/// 없다. 넷으로 나눈 것은 **사용자가 할 수 있는 일이 다르기 때문**이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretStoreError {
    /// 이 기기의 OS에 이 앱이 쓸 수 있는 자격증명 저장소가 없다.
    ///
    /// **파일로 대신 떨어뜨리지 않는다** (ADR-0009 §10.2). "어딘가에는 저장된다"가 되는 순간
    /// INV-7이 조용히 깨진다 — 저장할 수 없으면 저장할 수 없다고 말한다.
    Unsupported,
    /// 저장소를 열거나 잠금을 풀지 못했다. 사용자가 접근을 허용하면 풀릴 수 있다.
    NotAccessible,
    /// 저장소는 열렸지만 요청한 일을 마치지 못했다.
    OperationFailed,
    /// 저장된 값이 이 앱이 넣은 모양이 아니다 — 다른 도구가 같은 자리에 다른 것을 넣었을 수
    /// 있다. **추측해서 고쳐 읽지 않는다.**
    StoredValueUnusable,
}

impl SecretStoreError {
    /// 실패 `detail`에 남길 **기술적 원인**. secret도 계정 이름도 섞일 수 없는 고정 문자열이다.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => "no credential store on this system",
            Self::NotAccessible => "credential store not accessible",
            Self::OperationFailed => "credential store operation failed",
            Self::StoredValueUnusable => "stored credential is not readable",
        }
    }

    /// 사용자에게 보여줄 수 있는 실패로 옮긴다 (§13 · ADR-0009 §10.1).
    ///
    /// **OS 지식이 이 경계 밖으로 나가지 않는다** (INV-10). 나가는 것은 §13의 세 질문에 대한
    /// 답뿐이며, `detail`은 [`Self::as_str`]의 고정 문자열이다.
    ///
    /// 종류가 [`FailureKind::Storage`]인 것은 이것이 **로컬 저장소를 읽고 쓰지 못한 실패**이기
    /// 때문이다. 새 종류를 만들지 않는 이유는 §20.6과 같다 — 화면이 이 실패를 다른 실패와
    /// 구분해 안내해야 한다는 것이 실제로 드러나는 Task가 그때 나눈다.
    pub fn failure(self) -> Failure {
        let failure = match self {
            // 다시 눌러도 결과가 같다. 이 기기에서는 저장할 수 있는 자리가 없다.
            Self::Unsupported => Failure::permanent(FailureKind::Storage, UNSUPPORTED_MESSAGE),
            Self::NotAccessible => Failure::retryable(FailureKind::Storage, NOT_ACCESSIBLE_MESSAGE),
            Self::OperationFailed => Failure::retryable(FailureKind::Storage, FAILED_MESSAGE),
            // 값이 이상한 것은 다시 시도해서 달라지지 않는다. 사용자가 다시 넣어야 한다.
            Self::StoredValueUnusable => Failure::permanent(FailureKind::Storage, UNUSABLE_MESSAGE),
        };
        failure.with_detail(self.as_str())
    }
}

impl fmt::Display for SecretStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<SecretStoreError> for Failure {
    fn from(error: SecretStoreError) -> Self {
        error.failure()
    }
}

/// 화면에 그대로 띄우는 문장들. **어느 것에도 secret도 계정 이름도 없다.**
const UNSUPPORTED_MESSAGE: &str =
    "이 시스템에는 Molt Note가 쓸 수 있는 보안 저장소가 없다. 이 기기에서는 Notion 연결을 저장할 수 없다.";
const NOT_ACCESSIBLE_MESSAGE: &str =
    "시스템 보안 저장소에 접근하지 못했다. 접근을 허용한 뒤 다시 시도해야 한다.";
const FAILED_MESSAGE: &str = "시스템 보안 저장소에서 요청한 작업을 마치지 못했다. 다시 시도할 수 있다.";
const UNUSABLE_MESSAGE: &str =
    "시스템 보안 저장소에 저장된 값을 읽을 수 없다. 값을 지우고 다시 입력해야 한다.";

/// secret 하나를 저장 · 조회 · 삭제한다. **그 이상은 하지 않는다.**
///
/// `Send + Sync`인 것은 전송이 UI 밖 스레드에서 돌기 때문이다 —
/// [`crate::transcription::TranscriptionEngine`] · [`crate::ai::NoteAiProvider`]와 같다.
///
/// [`Self::get`]이 `Option`인 것은 **아직 저장한 적 없음이 오류가 아니기 때문**이다.
/// Notion을 설정하지 않은 것은 정상 상태다 (INV-8).
pub trait SecretStore: Send + Sync {
    /// 저장된 값을 읽는다. 저장한 적이 없으면 `Ok(None)`이다.
    fn get(&self, key: SecretKey) -> Result<Option<Secret>, Failure>;

    /// 값을 저장한다. 이미 있으면 대체한다.
    fn set(&self, key: SecretKey, secret: &Secret) -> Result<(), Failure>;

    /// 값을 지운다. **없던 것을 지우는 것은 실패가 아니다** — 부르고 난 뒤 없다는 사실이
    /// 같으면 같은 결과다.
    fn delete(&self, key: SecretKey) -> Result<(), Failure>;
}

/// 이 기기의 **실제 OS 자격증명 저장소**를 쓰는 구현 (ADR-0009 §10.2).
///
/// ```text
/// macOS     Keychain            (keyring `apple-native` → security-framework)
/// Windows   Credential Manager  (keyring `windows-native` → windows-sys) — 검증은 Phase 6
/// 그 밖      SecretStoreError::Unsupported. 파일로 조용히 떨어뜨리지 않는다
/// ```
///
/// ★ **자동 테스트는 이 타입을 실행하지 않는다.** Gate는 컴파일만 한다
/// (AI adapter가 소켓을 여는 파일과 같은 규약 · ADR-0009 §10.2). 테스트가 쓰는 것은
/// [`testing::InMemorySecretStore`] 하나다.
///
/// Windows를 위한 **구현 경계는 여기 안에 있다** — 같은 타입 · 같은 trait이며, 다른 것은 이
/// 파일 안의 `cfg` 분기 하나뿐이다. 그래서 Phase 6은 새 타입도 새 호출부도 만들지 않는다
/// (INV-10 · PRODUCT-SPEC §3.1). 그 동작이 실제로 확인된 것은 아니며, 그것이 Phase 6의 일이다.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsSecretStore;

impl SecretStore for OsSecretStore {
    fn get(&self, key: SecretKey) -> Result<Option<Secret>, Failure> {
        os::get(key).map_err(SecretStoreError::failure)
    }

    fn set(&self, key: SecretKey, secret: &Secret) -> Result<(), Failure> {
        os::set(key, secret).map_err(SecretStoreError::failure)
    }

    fn delete(&self, key: SecretKey) -> Result<(), Failure> {
        os::delete(key).map_err(SecretStoreError::failure)
    }
}

/// **이 앱에서 `cfg(target_os)`로 자격증명 저장소를 가르는 유일한 자리** (INV-10).
///
/// 바깥의 어떤 코드도 어느 OS에서 도는지 묻지 않는다 — [`SecretStore`] 하나만 안다.
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod os {
    use keyring::{Entry, Error};

    use super::{Secret, SecretKey, SecretStoreError, SECRET_SERVICE};

    /// 이 항목을 가리키는 자리 하나. **값은 여기 없다.**
    fn entry(key: SecretKey) -> Result<Entry, SecretStoreError> {
        Entry::new(SECRET_SERVICE, key.account()).map_err(translate)
    }

    pub(super) fn get(key: SecretKey) -> Result<Option<Secret>, SecretStoreError> {
        match entry(key)?.get_password() {
            Ok(value) => Ok(Some(Secret::new(value))),
            // 저장한 적이 없는 것은 실패가 아니다 (INV-8).
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(translate(error)),
        }
    }

    pub(super) fn set(key: SecretKey, secret: &Secret) -> Result<(), SecretStoreError> {
        entry(key)?.set_password(secret.expose()).map_err(translate)
    }

    pub(super) fn delete(key: SecretKey) -> Result<(), SecretStoreError> {
        match entry(key)?.delete_credential() {
            Ok(()) => Ok(()),
            // 없던 것을 지운 결과와 지운 결과는 같다 — 없다는 사실이 같으면 같은 결과다.
            Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(translate(error)),
        }
    }

    /// 라이브러리 오류를 **값을 담지 않는** 이 경계의 실패로 옮긴다.
    ///
    /// `error`를 문자열로 만들지 않는다. 여기서 `to_string()`을 한 번 부르는 것이
    /// ADR-0009 §10.4가 막으려는 유출 경로 그 자체다 — 서비스/계정 이름이 그 문장에 들어 있다.
    fn translate(error: Error) -> SecretStoreError {
        match error {
            // 저장소는 있는데 열지 못했다 — 잠겨 있거나 접근이 거부됐다.
            Error::NoStorageAccess(_) => SecretStoreError::NotAccessible,
            // 저장된 값을 이 앱이 넣은 모양으로 읽지 못했다.
            Error::BadEncoding(_) | Error::Ambiguous(_) => SecretStoreError::StoredValueUnusable,
            // NoEntry는 호출부가 이미 정상 상태로 처리했다. 나머지는 전부 "하지 못했다"이며,
            // 그 이상 나누지 않는다 — 사용자가 할 수 있는 일이 같다.
            _ => SecretStoreError::OperationFailed,
        }
    }
}

/// 자격증명 저장소가 없는 시스템. **파일로 대신 저장하지 않는다** (ADR-0009 §10.2).
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod os {
    use super::{Secret, SecretKey, SecretStoreError};

    pub(super) fn get(_key: SecretKey) -> Result<Option<Secret>, SecretStoreError> {
        Err(SecretStoreError::Unsupported)
    }

    pub(super) fn set(_key: SecretKey, _secret: &Secret) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::Unsupported)
    }

    pub(super) fn delete(_key: SecretKey) -> Result<(), SecretStoreError> {
        Err(SecretStoreError::Unsupported)
    }
}

/// 이 경계의 **결정론적 test double** — 자동 검증이 실제 Keychain을 요구하지 않는 이유다
/// (ADR-0009 §10.2 · PRODUCT-SPEC §18).
///
/// **자격증명 저장소를 열지 않는다.** 이 모듈에는 `keyring`도 `cfg(target_os)`도 없으며,
/// 값은 프로세스 메모리 안에서만 산다. 그래서 테스트를 몇 번을 돌려도 사용자의 Keychain에는
/// 아무 항목도 생기지 않고, 남지도 않는다.
///
/// `#[cfg(test)]`가 아닌 것은 통합 테스트(`src-tauri/tests/`)가 별개 crate에서 이것을 쓰기
/// 때문이다 ([`crate::transcription::testing`]과 같은 이유다).
pub mod testing {
    use std::sync::Mutex;

    use crate::domain::Failure;

    use super::{Secret, SecretKey, SecretStore, SecretStoreError};

    /// 메모리 안에서만 사는 [`SecretStore`].
    ///
    /// ```text
    /// InMemorySecretStore::new()          비어 있다 (아직 아무것도 저장하지 않았다)
    /// .failing(error)                     모든 호출이 그 실패를 낸다
    /// .stored(key)                        지금 담고 있는 값 (테스트가 관찰하는 자리)
    /// ```
    #[derive(Debug, Default)]
    pub struct InMemorySecretStore {
        entries: Mutex<Vec<(SecretKey, Secret)>>,
        failure: Option<SecretStoreError>,
    }

    impl InMemorySecretStore {
        pub fn new() -> Self {
            Self::default()
        }

        /// 모든 호출이 지정한 실패를 내는 double. 실패 경로를 Keychain 없이 지난다.
        pub fn failing(error: SecretStoreError) -> Self {
            Self {
                failure: Some(error),
                ..Self::default()
            }
        }

        /// 지금 담고 있는 값. **테스트가 "무엇이 저장됐는가"를 관찰하는 자리다.**
        pub fn stored(&self, key: SecretKey) -> Option<Secret> {
            self.lock()
                .iter()
                .find(|(stored, _)| *stored == key)
                .map(|(_, secret)| secret.clone())
        }

        /// 지금 담고 있는 항목 수. 지우기가 실제로 지웠는지 판정할 때 쓴다.
        pub fn len(&self) -> usize {
            self.lock().len()
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, Vec<(SecretKey, Secret)>> {
            self.entries.lock().expect("double의 잠금은 오염되지 않는다")
        }

        fn check(&self) -> Result<(), Failure> {
            match self.failure {
                Some(error) => Err(error.failure()),
                None => Ok(()),
            }
        }
    }

    impl SecretStore for InMemorySecretStore {
        fn get(&self, key: SecretKey) -> Result<Option<Secret>, Failure> {
            self.check()?;
            Ok(self.stored(key))
        }

        fn set(&self, key: SecretKey, secret: &Secret) -> Result<(), Failure> {
            self.check()?;
            let mut entries = self.lock();
            entries.retain(|(stored, _)| *stored != key);
            entries.push((key, secret.clone()));
            Ok(())
        }

        fn delete(&self, key: SecretKey) -> Result<(), Failure> {
            self.check()?;
            // 없던 것을 지우는 것도 성공이다 — 실제 구현과 같은 규약이다.
            self.lock().retain(|(stored, _)| *stored != key);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::InMemorySecretStore;
    use super::*;

    /// 테스트가 쓰는 값. **실제 자격증명이 아니다** — 이 저장소에 진짜 값을 적지 않는다
    /// (ADR-0009 §10.5).
    const NOT_A_REAL_VALUE: &str = "molt-note-test-double-value-not-a-real-credential";

    #[test]
    fn a_value_that_was_stored_comes_back_and_a_value_that_was_not_is_not_an_error() {
        let store = InMemorySecretStore::new();

        // 저장한 적이 없는 것은 실패가 아니라 '아직 없음'이다 (INV-8).
        assert_eq!(
            store
                .get(SecretKey::NotionIntegrationToken)
                .expect("아직 없는 것을 묻는 것은 실패가 아니다"),
            None
        );

        store
            .set(
                SecretKey::NotionIntegrationToken,
                &Secret::new(NOT_A_REAL_VALUE),
            )
            .expect("저장할 수 있어야 한다");

        let found = store
            .get(SecretKey::NotionIntegrationToken)
            .expect("읽을 수 있어야 한다")
            .expect("방금 저장한 값이 있어야 한다");
        assert_eq!(found.expose(), NOT_A_REAL_VALUE);
    }

    #[test]
    fn storing_again_replaces_instead_of_piling_up() {
        let store = InMemorySecretStore::new();

        for value in ["first-double-value", NOT_A_REAL_VALUE] {
            store
                .set(SecretKey::NotionIntegrationToken, &Secret::new(value))
                .expect("저장할 수 있어야 한다");
        }

        assert_eq!(store.len(), 1, "같은 자리에 하나만 남는다");
        assert_eq!(
            store
                .stored(SecretKey::NotionIntegrationToken)
                .expect("값이 있어야 한다")
                .expose(),
            NOT_A_REAL_VALUE
        );
    }

    #[test]
    fn deleting_leaves_nothing_and_deleting_nothing_is_not_a_failure() {
        let store = InMemorySecretStore::new();

        store
            .delete(SecretKey::NotionIntegrationToken)
            .expect("없던 것을 지우는 것은 실패가 아니다");

        store
            .set(
                SecretKey::NotionIntegrationToken,
                &Secret::new(NOT_A_REAL_VALUE),
            )
            .expect("저장할 수 있어야 한다");
        store
            .delete(SecretKey::NotionIntegrationToken)
            .expect("지울 수 있어야 한다");

        assert!(store.is_empty(), "지운 뒤에는 담고 있는 것이 없다");
        assert_eq!(store.get(SecretKey::NotionIntegrationToken).unwrap(), None);
    }

    #[test]
    fn the_value_never_appears_in_debug_output() {
        // ADR-0009 §10.4: `{:?}` 한 번으로 새는 경로를 타입이 막는다.
        let secret = Secret::new(NOT_A_REAL_VALUE);

        let rendered = format!("{secret:?}");
        assert!(!rendered.contains(NOT_A_REAL_VALUE), "{rendered}");
        assert_eq!(rendered, "Secret(<redacted>)");

        // double 전체를 찍어도 마찬가지다 — 담고 있는 것이 `Secret`이기 때문이다.
        let store = InMemorySecretStore::new();
        store
            .set(SecretKey::NotionIntegrationToken, &secret)
            .expect("저장할 수 있어야 한다");
        let rendered = format!("{store:?}");
        assert!(!rendered.contains(NOT_A_REAL_VALUE), "{rendered}");
    }

    #[test]
    fn a_failure_carries_no_value_and_no_account_name() {
        // 이 경계 밖으로 나가는 문장은 전부 고정 문자열이다 (ADR-0009 §10.4).
        for error in [
            SecretStoreError::Unsupported,
            SecretStoreError::NotAccessible,
            SecretStoreError::OperationFailed,
            SecretStoreError::StoredValueUnusable,
        ] {
            let failure = error.failure();

            assert_eq!(failure.kind, FailureKind::Storage);
            assert!(failure.source_data_safe, "secret 실패는 녹음을 건드리지 않는다");
            assert_eq!(failure.detail.as_deref(), Some(error.as_str()));

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
    fn the_two_permanent_failures_say_retrying_will_not_help() {
        // 사용자가 할 수 있는 일이 다르다 — 저장할 자리가 없는 것과 값이 이상한 것은
        // 다시 눌러서 풀리지 않는다.
        assert!(!SecretStoreError::Unsupported.failure().retryable);
        assert!(!SecretStoreError::StoredValueUnusable.failure().retryable);
        assert!(SecretStoreError::NotAccessible.failure().retryable);
        assert!(SecretStoreError::OperationFailed.failure().retryable);
    }

    #[test]
    fn a_failing_store_fails_every_operation_without_touching_anything() {
        let store = InMemorySecretStore::failing(SecretStoreError::NotAccessible);
        let expected = SecretStoreError::NotAccessible.failure();

        assert_eq!(
            store.get(SecretKey::NotionIntegrationToken).unwrap_err(),
            expected
        );
        assert_eq!(
            store
                .set(
                    SecretKey::NotionIntegrationToken,
                    &Secret::new(NOT_A_REAL_VALUE)
                )
                .unwrap_err(),
            expected
        );
        assert_eq!(
            store.delete(SecretKey::NotionIntegrationToken).unwrap_err(),
            expected
        );
        assert!(store.is_empty(), "실패한 저장이 값을 남기지 않는다");
    }

    #[test]
    fn the_item_is_findable_by_a_person_in_their_own_credential_store() {
        // 사용자가 자기 Keychain에서 이 항목을 찾아 지울 수 있어야 한다 (ADR-0009 §10.2).
        assert_eq!(SECRET_SERVICE, "molt-note");
        assert_eq!(
            SecretKey::NotionIntegrationToken.account(),
            "notion-integration-token"
        );
    }

    #[test]
    fn the_boundary_is_object_safe_so_a_double_can_stand_where_the_real_one_does() {
        // 호출부가 구현을 이름으로 고르지 않는다는 사실 그 자체를 고정한다.
        let store: Box<dyn SecretStore> = Box::new(InMemorySecretStore::new());
        assert_eq!(store.get(SecretKey::NotionIntegrationToken).unwrap(), None);
    }
}
