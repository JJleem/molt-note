//! 이미 만들어진 파일이 **놓인 자리를 여는** command의 소유자
//! (`phase-prompt/05.6` 성공 기준 2 · R-4 · PRODUCT-SPEC §12 · §13 · INV-3 · INV-10).
//!
//! ```text
//! show_saved_file ──→ SavedFiles ─→ ① 이 앱의 exports 디렉터리 아래인가  ← 여기서만 허용된다
//!                                  ─→ ② 그 자리에 실제로 있는 파일인가
//!                                  ─→ ③ platform::file_manager (OS를 부르는 유일한 자리)
//! ```
//!
//! ## webview가 임의 경로를 열 수 없다
//!
//! 이 command는 화면이 보낸 경로를 **그대로 넘기지 않는다.** 넘기기 전에 두 가지를 확인하며,
//! 통과하지 못한 요청은 거절된다:
//!
//! ```text
//! ① 그 경로를 정규화한 결과가 이 앱의 `exports/` 아래인가   (`..`도 symlink도 여기서 풀린다)
//! ② 디렉터리가 아니라 파일 하나인가
//! ```
//!
//! **여는 자리는 한 곳뿐이다** ([`SavedFiles::show`]) — 그 함수 밖에는
//! [`crate::platform::file_manager::FileManager`]를 부르는 코드가 없으므로, 검증을 지나지 않고
//! 여는 경로가 만들어질 자리가 없다. 이것은 asset protocol을 녹음 디렉터리 하나로만 여는 것과
//! 같은 태도다 (`crate::run` · `docs/ADR-0006-audio-playback.md`) — **주소를 만드는 쪽이 아니라
//! 여는 쪽이 범위를 정한다.**
//!
//! 정규화를 쓰는 이유는 문자열 비교로는 부족하기 때문이다. `exports/../../.ssh/id_rsa`는
//! 글자로 보면 `exports/`로 시작하지만 가리키는 자리는 그 밖이며,
//! [`std::fs::canonicalize`]는 `..`과 symlink를 실제로 풀어 그 사실을 드러낸다.
//!
//! ## 무엇도 만들지 · 고치지 · 지우지 않는다 (INV-3 · INV-4)
//!
//! 이 모듈에는 저장소에 쓰는 코드도, 파일을 쓰거나 지우는 코드도 없다. **디렉터리를 만들지도
//! 않는다** — [`AppDataDirectory::ensure_exports_dir`]가 아니라
//! [`AppDataDirectory::exports_dir`]를 쓰므로, 한 번도 내보낸 적 없는 사용자에게 이 경로가 빈
//! 디렉터리를 만들어 두지 않는다. 하는 일은 **이미 있는 것을 사람에게 보여 주는 것**뿐이다.
//!
//! ## 왜 export 표면이 아닌가
//!
//! 이름도 자리도 [`super::Exporter`]와 나눠 두었다. 저쪽은 **파일을 만드는** 일이고 이쪽은
//! **만들어진 것을 여는** 일이며, 둘을 한 이름에 묶으면 "export 표면은 파일 하나를 만드는 이름
//! 둘뿐이다"라는 사실(`tests/ipc-boundary.test.ts`)이 흐려진다.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::Manager;

use crate::domain::{Failure, FailureKind};
use crate::platform::app_data_dir::AppDataDirectory;
use crate::platform::file_manager::{self, FileManager};

/// 앱이 들고 있는 **저장된 파일 열람자**.
///
/// [`super::Storage`] · [`super::Exporter`]처럼 **앱 데이터 디렉터리를 얻지 못한 실패도 값으로
/// 들고 있다.** 그 실패는 열려고 할 때 사용자에게 그대로 전달된다 — 앱 시작을 막지 않는다 (§13).
pub struct SavedFiles {
    app_data_dir: Result<AppDataDirectory, Failure>,
    file_manager: Arc<dyn FileManager>,
}

impl SavedFiles {
    /// Tauri가 결정한 앱 데이터 디렉터리와 이 기기의 실제 파일 관리자를 쓴다 (INV-10).
    pub fn open_for<R, M>(manager: &M) -> Self
    where
        R: tauri::Runtime,
        M: Manager<R>,
    {
        Self {
            app_data_dir: AppDataDirectory::from_manager(manager).map_err(Into::into),
            file_manager: file_manager::app_file_manager(),
        }
    }

    /// 주어진 디렉터리와 주어진 파일 관리자로 판정한다.
    ///
    /// **자동 검증이 지나는 자리다** — 임시 루트와 double을 주면 제품 경로 그대로 돌면서도
    /// 창이 하나도 열리지 않는다 (§18 · [`super::Exporter::in_directory`]와 같은 규약).
    pub fn in_directory(app_data_dir: AppDataDirectory, file_manager: Arc<dyn FileManager>) -> Self {
        Self {
            app_data_dir: Ok(app_data_dir),
            file_manager,
        }
    }

    /// 이 앱이 만든 파일 하나가 **놓인 자리**를 연다.
    ///
    /// **이 앱의 `exports/` 아래에 실제로 있는 파일일 때만 연다.** 그 밖의 경로는 무엇이든
    /// 거절되며, 거절은 §13의 실패로 화면에 도착한다 — 조용히 아무 일도 하지 않지 않는다.
    ///
    /// 순서에 이유가 있다. **먼저 자리를 확정하고 그다음에 요청을 확정한다** — 그래야 어떤
    /// 입력이 와도 판정의 기준이 이 앱의 디렉터리 하나로 고정된다.
    pub fn show(&self, path: &str) -> Result<(), Failure> {
        let app_data_dir = self.app_data_dir.as_ref().map_err(Clone::clone)?;

        let requested = path.trim();
        if requested.is_empty() {
            return Err(Failure::permanent(
                FailureKind::InvalidInput,
                "열어 볼 파일을 고르지 않았다.",
            ));
        }

        // 이 앱이 파일을 두는 자리. **만들지 않는다** — 한 번도 내보낸 적이 없으면 열 것도 없다.
        let exports_dir = resolved(&app_data_dir.exports_dir()).ok_or_else(nothing_saved_yet)?;

        // `..`도 symlink도 여기서 풀린다. 풀지 못했다는 것은 그 자리에 없다는 뜻이다.
        let file = resolved(Path::new(requested))
            .ok_or_else(|| missing_or_foreign(requested, &app_data_dir.exports_dir()))?;

        // **여기가 허용을 결정하는 단 한 곳이다.** 아래 한 줄을 지나지 않고 OS에 닿는 경로는
        // 이 모듈에도 이 crate에도 없다.
        if !file.starts_with(&exports_dir) || !file.is_file() {
            return Err(not_ours());
        }

        self.file_manager.show(&file)
    }
}

/// 경로를 실제 자리로 정규화한다. 그 자리에 없으면 `None`이다.
///
/// 실패 이유를 나누지 않는 이유는 이 경계가 그것으로 할 일이 없기 때문이다 — 없는 것과 읽을 수
/// 없는 것 모두 **열 수 없다**는 하나의 사실이며, 그 문장은 부르는 쪽이 만든다.
fn resolved(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// 아직 이 앱이 만든 파일이 하나도 없다. **실패지만 사용자가 잘못한 것은 아니다.**
fn nothing_saved_yet() -> Failure {
    Failure::permanent(
        FailureKind::InvalidInput,
        "아직 이 앱이 만든 파일이 없다. 먼저 내보낸 뒤에 그 자리를 열 수 있다.",
    )
}

/// 요청한 경로가 이 앱의 것이 아니거나 그 자리에 없다.
///
/// 두 문장을 나누는 이유는 **사용자가 할 일이 다르기 때문**이다 — 앞의 것은 파일이 옮겨졌거나
/// 지워졌다는 뜻이고(이 앱이 지운 것이 아니다 · INV-3), 뒤의 것은 이 앱이 열어 줄 대상이
/// 아니라는 뜻이다.
///
/// **판정에는 쓰지 않는다.** 문장을 고르기 위한 글자 비교일 뿐이며, 실제로 열어도 되는지는
/// 정규화된 경로 하나로만 판정된다 ([`SavedFiles::show`]).
fn missing_or_foreign(requested: &str, exports_dir: &Path) -> Failure {
    let requested = Path::new(requested);
    let looks_like_ours = requested.starts_with(exports_dir)
        && !requested
            .components()
            .any(|component| component == std::path::Component::ParentDir);

    if looks_like_ours {
        return Failure::permanent(
            FailureKind::InvalidInput,
            "그 파일이 더 이상 그 자리에 없다. 옮겨졌거나 지워졌을 수 있다.",
        );
    }

    not_ours()
}

/// 이 앱이 만든 파일이 아니다. **거절이 기본값이다.**
fn not_ours() -> Failure {
    Failure::permanent(
        FailureKind::InvalidInput,
        "이 앱이 만든 파일만 자리를 열 수 있다.",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::platform::file_manager::testing::RecordedFileManager;

    use super::*;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "molt-note-saved-file-{}-{}-{}",
                label,
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
            Self(path)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// 임시 루트 · double · 그 위에서 도는 열람자.
    struct Fixture {
        _root: TempRoot,
        app_data_dir: AppDataDirectory,
        manager: Arc<RecordedFileManager>,
        saved_files: SavedFiles,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let root = TempRoot::new(label);
            let app_data_dir = AppDataDirectory::new(root.0.join("app-data"));
            let manager = Arc::new(RecordedFileManager::new());

            Self {
                saved_files: SavedFiles::in_directory(app_data_dir.clone(), manager.clone()),
                app_data_dir,
                manager,
                _root: root,
            }
        }

        /// 내보낸 파일 하나를 만든다 — export가 하는 것과 같은 자리다.
        fn exported(&self, name: &str) -> PathBuf {
            let directory = self
                .app_data_dir
                .ensure_exports_dir()
                .expect("사전 조건: export 디렉터리를 만든다");
            let path = directory.join(name);
            std::fs::write(&path, "# 내보낸 문서").expect("사전 조건: 파일을 둔다");
            path
        }
    }

    #[test]
    fn a_file_this_app_wrote_is_the_one_that_opens() {
        let fixture = Fixture::new("ours");
        let file = fixture.exported("2026-09-01-weekly.md");

        fixture
            .saved_files
            .show(&file.display().to_string())
            .expect("이 앱이 만든 파일은 열 수 있어야 한다");

        assert_eq!(
            fixture.manager.shown(),
            vec![std::fs::canonicalize(&file).expect("정규화할 수 있어야 한다")],
            "경계에 도착하는 것은 정규화된 경로 하나다"
        );
    }

    #[test]
    fn a_path_outside_the_exports_directory_is_refused() {
        let fixture = Fixture::new("outside");
        fixture.exported("2026-09-01-weekly.md");

        // 앱 데이터 루트 안이지만 exports가 아니다 — 저장소 파일이 그 예다.
        let elsewhere = fixture.app_data_dir.database_path();
        std::fs::write(&elsewhere, "not for the user").expect("사전 조건: 파일을 둔다");

        let failure = fixture
            .saved_files
            .show(&elsewhere.display().to_string())
            .expect_err("거절되어야 한다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.retryable, "같은 경로를 다시 보내도 같다");
        assert!(fixture.manager.is_empty(), "OS에 닿지 않았다");
    }

    #[test]
    fn walking_out_of_the_exports_directory_does_not_work() {
        // 글자로만 보면 `exports/`로 시작한다. 정규화하면 그 밖이다.
        let fixture = Fixture::new("traversal");
        fixture.exported("2026-09-01-weekly.md");
        let outside = fixture.app_data_dir.root().join("escaped.md");
        std::fs::write(&outside, "밖의 파일").expect("사전 조건: 밖에 파일을 둔다");

        let sneaky = fixture
            .app_data_dir
            .exports_dir()
            .join("..")
            .join("escaped.md");

        let failure = fixture
            .saved_files
            .show(&sneaky.display().to_string())
            .expect_err("거절되어야 한다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(fixture.manager.is_empty(), "OS에 닿지 않았다");
        assert!(outside.exists(), "거절이 파일을 건드리지 않는다 (INV-3)");
    }

    #[test]
    fn a_symlink_that_points_outside_is_refused_too() {
        // symlink는 글자에 드러나지 않는다. 정규화가 그것을 푼다.
        #[cfg(unix)]
        {
            let fixture = Fixture::new("symlink");
            fixture.exported("2026-09-01-weekly.md");
            let outside = fixture.app_data_dir.root().join("secret.md");
            std::fs::write(&outside, "밖의 파일").expect("사전 조건: 밖에 파일을 둔다");
            let link = fixture.app_data_dir.exports_dir().join("looks-ours.md");
            std::os::unix::fs::symlink(&outside, &link).expect("사전 조건: symlink를 만든다");

            let failure = fixture
                .saved_files
                .show(&link.display().to_string())
                .expect_err("거절되어야 한다");

            assert_eq!(failure.kind, FailureKind::InvalidInput);
            assert!(fixture.manager.is_empty(), "OS에 닿지 않았다");
        }
    }

    #[test]
    fn the_exports_directory_itself_is_not_a_thing_to_open() {
        let fixture = Fixture::new("directory");
        fixture.exported("2026-09-01-weekly.md");

        let failure = fixture
            .saved_files
            .show(&fixture.app_data_dir.exports_dir().display().to_string())
            .expect_err("디렉터리는 이 이름이 여는 것이 아니다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(fixture.manager.is_empty());
    }

    #[test]
    fn a_file_that_is_gone_says_so_without_creating_anything() {
        let fixture = Fixture::new("gone");
        let file = fixture.exported("2026-09-01-weekly.md");
        std::fs::remove_file(&file).expect("사전 조건: 사용자가 지운 상태를 만든다");

        let failure = fixture
            .saved_files
            .show(&file.display().to_string())
            .expect_err("없는 파일은 열 수 없다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.source_data_safe, "이 경로는 아무것도 지우지 않았다");
        assert!(!file.exists(), "거절이 파일을 만들어 내지 않는다");
        assert!(fixture.manager.is_empty());
    }

    #[test]
    fn asking_before_anything_was_exported_does_not_create_the_directory() {
        let fixture = Fixture::new("nothing-yet");

        let failure = fixture
            .saved_files
            .show("/tmp/whatever.md")
            .expect_err("열 것이 없다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(
            !fixture.app_data_dir.exports_dir().exists(),
            "열어 보려는 요청이 빈 디렉터리를 만들지 않는다"
        );
    }

    #[test]
    fn asking_to_open_nothing_is_refused_before_anything_is_touched() {
        let fixture = Fixture::new("empty-path");

        let failure = fixture.saved_files.show("   ").expect_err("고른 파일이 없다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.retryable);
        assert!(fixture.manager.is_empty());
    }

    #[test]
    fn a_directory_that_could_not_be_resolved_is_reported_instead_of_panicking() {
        // 앱 시작 시점의 실패가 여기까지 값으로 실려 온다 (§13). 그 상태에서도 앱은 떠 있다.
        let saved_files = SavedFiles {
            app_data_dir: Err(Failure::retryable(
                FailureKind::Storage,
                "앱 데이터 디렉터리 경로를 결정하지 못했다",
            )),
            file_manager: Arc::new(RecordedFileManager::new()),
        };

        let failure = saved_files
            .show("/tmp/whatever.md")
            .expect_err("자리를 모르면 판정할 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (INV-3)");
    }
}
