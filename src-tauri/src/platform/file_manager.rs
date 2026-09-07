//! 파일이 **놓인 자리를 여는** 경계 — OS 파일 관리자 (PRODUCT-SPEC §3.1 · §13 · INV-10 ·
//! `phase-prompt/05.6` 성공 기준 2 · R-4).
//!
//! ```text
//! FileManager                     파일 하나가 놓인 자리를 연다. 그 이상은 하지 않는다
//! OsFileManager                   이 기기의 실제 파일 관리자 ★ 자동 테스트가 실행하지 않는다
//! FileManagerError                열지 못한 이유. 문자열도 경로도 담지 않는다
//! testing::RecordedFileManager    자동 테스트가 쓰는 유일한 구현
//! ```
//!
//! ## 왜 경계가 하나 더 필요했는가 (R-4)
//!
//! export는 이미 만들어진 파일의 **전체 경로를 화면에 그대로 보여 준다**
//! (`crate::commands::payload::ExportedFilePayload`). 그럼에도 2026-09-05의 첫 실사용에서
//! 사람이 그 파일에 도달하지 못했다 — macOS에서 `~/Library`는 Finder 기본 숨김이라 경로를
//! 알아도 걸어 들어갈 수 없기 때문이다. **"경로를 보여 주면 찾을 수 있다"는 가정이 이
//! 플랫폼에서 틀렸다.** 그래서 경로를 없애는 대신 **여는 수단을 하나 더한다.**
//!
//! ## OS를 부르는 코드는 이 파일 안에만 있다 (INV-10)
//!
//! `platform`의 다른 넷과 같은 종류의 자리다 — 바깥 세계에 닿는 곳이며, 그 바깥을 값으로
//! 바꿔 넣으면 나머지 코드가 실제 파일 관리자 없이 검증된다 (§18). 화면도 domain도 export
//! 실행 순서도 이 경계 뒤의 이름(`open` · `explorer` · `cfg(target_os)`)을 알지 않으며,
//! 그 사실은 `src-tauri/tests/show_saved_file.rs`가 소스에서 확인한다.
//!
//! ## 무엇을 여는지 이 경계는 고르지 않는다
//!
//! **여기에는 "어느 파일을 열어도 되는가"에 대한 판정이 없다.** 그 판정은 부르는 쪽
//! ([`crate::commands::SavedFiles`])이 자기 exports 디렉터리 아래인지 확인하는 자리에 있고,
//! 이 경계는 이미 확인된 경로 하나를 받아 연다. 두 곳에서 판정하면 한쪽이 느슨해졌을 때
//! 그것을 아무도 알아채지 못한다.
//!
//! ## Tauri 플러그인도, 새 capability 권한도 쓰지 않는다
//!
//! 이 경계가 쓰는 것은 표준 라이브러리의 프로세스 실행 하나뿐이다. 그래서
//! `src-tauri/capabilities/default.json`의 permissions는 `["core:default"]` 그대로이고
//! (`shell:` 권한이 붙지 않는다 — `src-tauri/tests/transcription_engine.rs`가 그 사실을 이미
//! 지키고 있다), `Cargo.toml`에도 새 의존성이 생기지 않았다. **필요하지 않은 권한을 미리 켜
//! 두지 않는다** (PRODUCT-SPEC §12 · §20.5). 권한이 실제로 필요해지는 구현으로 바뀌는 날, 그
//! 결정은 이 파일 하나를 고치는 일이며 그때 이유와 함께 켠다.
//!
//! ## 파일을 **여는** 것이 아니다
//!
//! 여는 것은 파일이 **놓인 자리**이며, 파일 자체를 연결된 앱으로 실행하지 않는다. 그래서 이
//! 경계는 사용자가 고른 문서를 임의의 앱에 넘기는 통로가 되지 않는다.
//!
//! ## 지원하지 않는 시스템에서 조용히 아무 일도 하지 않지 않는다 (§13)
//!
//! 열 수단이 없는 시스템에서는 [`FileManagerError::Unsupported`]가 되고, 그것은 §13의 세
//! 질문에 답하는 [`Failure`] 하나로 화면에 도착한다 — 버튼을 눌렀는데 아무 일도 일어나지
//! 않는 상태를 만들지 않는다 (`platform::secret_store`가 자격증명 저장소가 없는 시스템에서
//! 파일로 조용히 떨어뜨리지 않는 것과 같은 태도다).
//!
//! ## 이 Run이 확인한 것과 확인하지 못한 것
//!
//! **확인하지 못했다.** 이 Run은 Gate 명령만 실행할 수 있으므로 `open -R`도
//! `explorer /select,`도 실제로 실행해 보지 못했고, 두 이름과 플래그는 각 플랫폼의 관례를
//! 따른 것이다 (UNVERIFIED — `docs/ADR-0005-microphone-permission.md`가 TCC에 대해 한 것과
//! 같은 방식으로 적어 둔다). 그 대신 **확인하지 못한 것에 제품을 걸지 않는다**: 실행하지
//! 못하면 그 사실이 정의된 실패로 화면에 도착하고, 전체 경로는 그대로 남아 사용자가 직접
//! 찾아갈 수 있다 (`src/screens/savedFileView.ts`). 실제로 열리는지는 이 Phase의 Human
//! Review 항목이다.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::domain::{Failure, FailureKind};

/// 이 기기의 실제 파일 관리자 하나 — **앱이 지나는 경로다.**
///
/// 실제 구현을 **세우는 자리도 이 파일 하나로 묶는다** ([`crate::platform::secret_store`]의
/// `app_secret_store`와 같은 규칙이다). 바깥의 코드는 [`FileManager`]만 알고, 어느 구현이
/// 서는지는 여기서 정해진다 (INV-10).
///
/// ★ **자동 테스트는 이 함수를 부르지 않는다.** 부르면 검사가 도는 동안 창이 열린다.
/// 테스트가 쓰는 것은 [`testing::RecordedFileManager`] 하나다.
pub fn app_file_manager() -> Arc<dyn FileManager> {
    Arc::new(OsFileManager)
}

/// 파일 하나가 **놓인 자리**를 사람이 볼 수 있게 연다. **그 이상은 하지 않는다.**
///
/// `Send + Sync`인 것은 이 경계를 쓰는 command가 `async`이기 때문이다 —
/// [`crate::platform::secret_store::SecretStore`]와 같은 이유다.
///
/// **읽기도 쓰기도 없다.** 이 trait에는 파일을 만들거나 고치거나 지우는 이름이 없으며,
/// 그래서 이 경계를 통해 저장된 것이 달라지는 경로가 없다 (INV-3).
pub trait FileManager: Send + Sync {
    /// 이 파일이 놓인 자리를 연다. 경로는 부르는 쪽이 이미 확인한 것이다.
    fn show(&self, path: &Path) -> Result<(), Failure>;
}

/// 자리를 열지 못한 이유. **값을 담지 않는다.**
///
/// [`crate::platform::secret_store::SecretStoreError`]와 같은 설계다 — 변형만 있고 데이터가
/// 없으므로 OS 오류 문장이나 경로가 이 타입을 통해 밖으로 나갈 수 없다. 둘로 나눈 것은
/// **사용자가 할 수 있는 일이 다르기 때문**이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerError {
    /// 이 시스템에는 자리를 열어 줄 파일 관리자가 없다. 다시 눌러도 결과가 같다.
    Unsupported,
    /// 파일 관리자는 있는데 이번에 띄우지 못했다. 다시 눌러 볼 수 있다.
    Failed,
}

impl FileManagerError {
    /// 실패 `detail`에 남길 **기술적 원인**. 경로가 섞일 수 없는 고정 문자열이다.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => "no file manager on this system",
            Self::Failed => "could not open the file manager",
        }
    }

    /// 사용자에게 보여줄 수 있는 실패로 옮긴다 (§13).
    ///
    /// **OS 지식이 이 경계 밖으로 나가지 않는다** (INV-10) — 나가는 것은 §13의 세 질문에 대한
    /// 답뿐이다.
    ///
    /// 종류가 [`FailureKind::Storage`]인 것은 이것이 **이 기기의 로컬 파일을 다루지 못한
    /// 실패**이기 때문이다. 새 종류를 만들지 않는 이유는 §20.6과 같다 — 화면이 이 실패를 다른
    /// 실패와 구분해 안내해야 한다는 것이 실제로 드러나는 Task가 그때 나눈다. 지금 화면이 하는
    /// 일은 두 갈래 모두에서 같다: **경로는 그대로 있으니 직접 찾아갈 수 있다고 말한다**
    /// (`src/screens/savedFileView.ts`).
    pub fn failure(self) -> Failure {
        let failure = match self {
            // 이 시스템에는 열 수단이 없다. 다시 눌러도 같다.
            Self::Unsupported => Failure::permanent(FailureKind::Storage, UNSUPPORTED_MESSAGE),
            Self::Failed => Failure::retryable(FailureKind::Storage, FAILED_MESSAGE),
        };
        failure.with_detail(self.as_str())
    }
}

impl fmt::Display for FileManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<FileManagerError> for Failure {
    fn from(error: FileManagerError) -> Self {
        error.failure()
    }
}

/// 화면에 그대로 띄우는 문장들.
///
/// **둘 다 "그래도 파일은 그 자리에 있다"고 말한다.** 여는 수단은 경로의 대체가 아니라
/// 추가이므로, 그 수단이 없거나 실패한 상태에서도 사용자에게 남는 길이 있다.
const UNSUPPORTED_MESSAGE: &str =
    "이 시스템에서는 파일이 있는 자리를 대신 열 수 없다. 파일은 화면에 보이는 경로에 그대로 있다.";
const FAILED_MESSAGE: &str =
    "파일이 있는 자리를 열지 못했다. 파일은 화면에 보이는 경로에 그대로 있으며, 다시 시도할 수 있다.";

/// 이 기기의 **실제 파일 관리자**를 쓰는 구현.
///
/// ```text
/// macOS     Finder              (`open -R` — 파일을 열지 않고 그 자리를 열어 고른다)
/// Windows   File Explorer       (`explorer /select,` — 검증은 Phase 6)
/// 그 밖      FileManagerError::Unsupported. 조용히 아무 일도 하지 않지 않는다
/// ```
///
/// ★ **자동 테스트는 이 타입을 실행하지 않는다.** Gate는 컴파일만 한다
/// ([`crate::platform::secret_store::OsSecretStore`]와 같은 규약). 테스트가 쓰는 것은
/// [`testing::RecordedFileManager`] 하나이며, 그 사실은 `src-tauri/tests/show_saved_file.rs`가
/// 소스에서 확인한다.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsFileManager;

impl FileManager for OsFileManager {
    fn show(&self, path: &Path) -> Result<(), Failure> {
        os::show(path).map_err(FileManagerError::failure)
    }
}

/// **이 앱에서 `cfg(target_os)`로 파일 관리자를 가르는 유일한 자리** (INV-10).
///
/// 바깥의 어떤 코드도 어느 OS에서 도는지 묻지 않는다 — [`FileManager`] 하나만 안다.
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod os {
    use std::io::ErrorKind;
    use std::path::Path;
    use std::process::Command;

    use super::FileManagerError;

    pub(super) fn show(path: &Path) -> Result<(), FileManagerError> {
        let mut child = command_for(path).spawn().map_err(|error| match error.kind() {
            // 부를 프로그램 자체가 없다 — 다시 눌러도 같다.
            ErrorKind::NotFound => FileManagerError::Unsupported,
            _ => FileManagerError::Failed,
        })?;

        // 자식이 좀비로 남지 않게 거둔다. 둘 다 창을 띄운 뒤 곧바로 끝나는 프로그램이다.
        //
        // **끝 상태를 판정에 쓰지 않는다.** 각 플랫폼이 어떤 코드로 끝나는지 이 Run은 확인하지
        // 못했고 (UNVERIFIED), 확인하지 못한 값으로 "실패했다"고 말하면 실제로 열린 창을 두고
        // 사용자에게 거짓을 알리게 된다. 여기서 판정하는 것은 **띄울 수 있었는가**까지다.
        let _ = child.wait();
        Ok(())
    }

    /// macOS의 Finder에게 **파일이 있는 자리**를 열고 그 파일을 고른 상태로 보여 달라고 한다.
    ///
    /// `-R`(reveal)이 요점이다. 그것이 없으면 파일이 연결된 앱으로 **열리며**, 그것은 이
    /// 경계가 하기로 한 일이 아니다.
    #[cfg(target_os = "macos")]
    fn command_for(path: &Path) -> Command {
        let mut command = Command::new("open");
        command.arg("-R").arg(path);
        command
    }

    /// Windows의 File Explorer에서 같은 일을 한다 — 자리를 열고 그 파일을 고른다.
    ///
    /// 인자를 하나로 붙이는 것은 이 명령의 문법이 `/select,<path>`이기 때문이다.
    /// **실제 동작 확인은 Phase 6이 한다** (PRODUCT-SPEC §14.3의 다른 Windows 항목들과 같다).
    #[cfg(target_os = "windows")]
    fn command_for(path: &Path) -> Command {
        let mut command = Command::new("explorer");
        command.arg(format!("/select,{}", path.display()));
        command
    }
}

/// 파일 관리자가 없는 시스템. **조용히 아무 일도 하지 않지 않는다** (§13).
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod os {
    use std::path::Path;

    use super::FileManagerError;

    pub(super) fn show(_path: &Path) -> Result<(), FileManagerError> {
        Err(FileManagerError::Unsupported)
    }
}

/// 이 경계의 **결정론적 test double** — 자동 검증이 실제 파일 관리자를 열지 않는 이유다
/// (PRODUCT-SPEC §18).
///
/// **창을 하나도 띄우지 않는다.** 이 모듈에는 `process::Command`도 `cfg(target_os)`도 없으며,
/// 요청은 프로세스 메모리 안에만 남는다. 그래서 테스트를 몇 번을 돌려도 검사하는 사람의 화면에
/// 아무것도 열리지 않는다.
///
/// `#[cfg(test)]`가 아닌 것은 통합 테스트(`src-tauri/tests/`)가 별개 crate에서 이것을 쓰기
/// 때문이다 ([`crate::platform::secret_store::testing`]과 같은 이유다).
pub mod testing {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use crate::domain::Failure;

    use super::{FileManager, FileManagerError};

    /// 무엇을 열어 달라고 했는지만 기억하는 [`FileManager`].
    ///
    /// ```text
    /// RecordedFileManager::new()      전부 성공한다
    /// .failing(error)                 모든 요청이 그 실패를 낸다
    /// .shown()                        지금까지 열어 달라고 한 경로 (테스트가 관찰하는 자리)
    /// ```
    #[derive(Debug, Default)]
    pub struct RecordedFileManager {
        shown: Mutex<Vec<PathBuf>>,
        failure: Option<FileManagerError>,
    }

    impl RecordedFileManager {
        pub fn new() -> Self {
            Self::default()
        }

        /// 모든 요청이 지정한 실패를 내는 double. 실패 경로를 창 없이 지난다.
        pub fn failing(error: FileManagerError) -> Self {
            Self {
                failure: Some(error),
                ..Self::default()
            }
        }

        /// 지금까지 **열어 달라고 요청받은** 경로 전부.
        ///
        /// 거절된 요청은 여기 오지 않는다 — 그것이 "검증을 통과한 것만 열린다"를 테스트가
        /// 관찰하는 방식이다.
        pub fn shown(&self) -> Vec<PathBuf> {
            self.lock().clone()
        }

        pub fn is_empty(&self) -> bool {
            self.lock().is_empty()
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, Vec<PathBuf>> {
            self.shown.lock().expect("double의 잠금은 오염되지 않는다")
        }
    }

    impl FileManager for RecordedFileManager {
        fn show(&self, path: &Path) -> Result<(), Failure> {
            if let Some(error) = self.failure {
                return Err(error.failure());
            }
            self.lock().push(path.to_path_buf());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::RecordedFileManager;
    use super::*;

    #[test]
    fn what_was_asked_for_is_what_reaches_the_boundary() {
        let manager = RecordedFileManager::new();
        let path = Path::new("/tmp/molt-note/exports/2026-09-01-weekly.md");

        manager.show(path).expect("열 수 있어야 한다");

        assert_eq!(manager.shown(), vec![path.to_path_buf()]);
    }

    #[test]
    fn a_failure_carries_no_path_and_no_os_message() {
        // 이 경계 밖으로 나가는 문장은 전부 고정 문자열이다 — 담을 자리가 없다.
        for error in [FileManagerError::Unsupported, FileManagerError::Failed] {
            let failure = error.failure();

            assert_eq!(failure.kind, FailureKind::Storage);
            assert!(failure.source_data_safe, "여는 일은 아무것도 건드리지 않는다");
            assert_eq!(failure.detail.as_deref(), Some(error.as_str()));
            assert!(
                !format!("{failure:?}").contains("exports"),
                "경로가 실패에 실려 나간다"
            );
        }
    }

    #[test]
    fn both_messages_say_the_file_is_still_where_the_path_says() {
        // 여는 수단은 경로의 **대체가 아니라 추가다.** 그 수단이 없거나 실패해도 사용자에게
        // 남는 길이 있고, 그 사실이 문장 안에 있다.
        for error in [FileManagerError::Unsupported, FileManagerError::Failed] {
            let message = error.failure().message;
            assert!(message.contains("경로"), "{message}");
        }
    }

    #[test]
    fn the_system_without_a_file_manager_says_so_instead_of_doing_nothing() {
        assert!(!FileManagerError::Unsupported.failure().retryable);
        assert!(FileManagerError::Failed.failure().retryable);
    }

    #[test]
    fn a_failing_manager_never_records_a_request() {
        let manager = RecordedFileManager::failing(FileManagerError::Unsupported);

        let failure = manager
            .show(Path::new("/tmp/molt-note/exports/weekly.md"))
            .expect_err("실패해야 한다");

        assert_eq!(failure, FileManagerError::Unsupported.failure());
        assert!(manager.is_empty(), "실패한 요청은 아무것도 남기지 않는다");
    }

    #[test]
    fn the_boundary_is_object_safe_so_a_double_can_stand_where_the_real_one_does() {
        let manager: Box<dyn FileManager> = Box::new(RecordedFileManager::new());
        assert!(manager.show(Path::new("/tmp/molt-note/exports/a.md")).is_ok());
    }
}
