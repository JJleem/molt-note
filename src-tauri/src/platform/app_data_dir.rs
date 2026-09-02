//! 앱 데이터 디렉터리 결정 (PRODUCT-SPEC §3.1의 `AppDataDirectory` 경계).
//!
//! 실제 플랫폼 위치는 Tauri v2의 `PathResolver`가 결정한다 (§14.2 · VERIFIED).
//! 그러므로 이 모듈에도 OS별 경로 문자열이나 `cfg(target_os)` 분기가 없다 —
//! 플랫폼 차이를 아는 유일한 주체는 Tauri이고, 이 모듈은 그 호출을 한 곳에 가둔다.
//!
//! 나머지 코드는 "앱 데이터 디렉터리"와 거기서 파생된 경로만 안다 (INV-10).

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::domain::{Failure, FailureKind};

/// 로컬 영속 저장소 파일 이름. 루트가 정해지면 DB 경로는 결정론적으로 따라온다.
const DATABASE_FILE_NAME: &str = "molt-note.db";

/// 녹음 파일을 두는 하위 디렉터리 이름. DB와 같은 방식으로 루트에서 파생된다.
const RECORDINGS_DIR_NAME: &str = "recordings";

/// 앱이 자신의 데이터를 두는 디렉터리.
///
/// 임의의 base path로 생성할 수 있으므로 테스트는 임시 디렉터리를 주입해
/// 실제 사용자 디렉터리를 건드리지 않고 검증할 수 있다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppDataDirectory {
    root: PathBuf,
}

impl AppDataDirectory {
    /// 주어진 경로를 루트로 삼는다. 파일시스템에는 접근하지 않는다.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Tauri가 결정한 플랫폼별 앱 데이터 디렉터리를 루트로 삼는다.
    ///
    /// `PathResolver::app_data_dir()` 호출은 이 함수 밖으로 새지 않는다.
    pub fn from_manager<R, M>(manager: &M) -> Result<Self, AppDataDirError>
    where
        R: tauri::Runtime,
        M: Manager<R>,
    {
        let root = manager
            .path()
            .app_data_dir()
            .map_err(AppDataDirError::Resolve)?;
        Ok(Self::new(root))
    }

    /// 디렉터리 루트.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 디렉터리가 없으면 만든다. 이미 있으면 아무 일도 하지 않고 성공한다.
    pub fn ensure(&self) -> Result<(), AppDataDirError> {
        create(&self.root)
    }

    /// 로컬 DB 파일 경로. 같은 루트면 항상 같은 값이다.
    pub fn database_path(&self) -> PathBuf {
        self.root.join(DATABASE_FILE_NAME)
    }

    /// 녹음 파일을 두는 디렉터리. 같은 루트면 항상 같은 값이다.
    ///
    /// **오디오 파일의 위치를 정하는 자리는 여기 하나다.** 캡처 코드는 이 값을 받아 쓸 뿐
    /// 플랫폼 경로를 스스로 만들지 않는다 (INV-10 · ADR-0003 §5.11).
    pub fn recordings_dir(&self) -> PathBuf {
        self.root.join(RECORDINGS_DIR_NAME)
    }

    /// 녹음 디렉터리가 없으면 만들고 그 경로를 돌려준다.
    ///
    /// [`Self::ensure`]와 같이 **이미 있는 것을 지우거나 비우지 않는다.** 이전에 만들어 둔
    /// 녹음 파일은 그대로 남는다 (INV-1 · INV-4).
    pub fn ensure_recordings_dir(&self) -> Result<PathBuf, AppDataDirError> {
        let directory = self.recordings_dir();
        create(&directory)?;
        Ok(directory)
    }
}

/// 디렉터리 하나를 만든다. 실패하면 어느 경로가 문제인지 함께 남긴다.
fn create(path: &Path) -> Result<(), AppDataDirError> {
    std::fs::create_dir_all(path).map_err(|source| AppDataDirError::Create {
        path: path.to_path_buf(),
        source,
    })
}

/// 앱 데이터 디렉터리를 준비하지 못한 경우.
#[derive(Debug)]
pub enum AppDataDirError {
    /// 플랫폼 경로 자체를 얻지 못했다.
    Resolve(tauri::Error),
    /// 경로는 얻었지만 디렉터리를 만들지 못했다.
    Create { path: PathBuf, source: io::Error },
}

impl fmt::Display for AppDataDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolve(_) => write!(f, "앱 데이터 디렉터리 경로를 결정하지 못했다"),
            Self::Create { path, .. } => {
                write!(f, "앱 데이터 디렉터리를 만들지 못했다: {}", path.display())
            }
        }
    }
}

impl std::error::Error for AppDataDirError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Resolve(source) => Some(source),
            Self::Create { source, .. } => Some(source),
        }
    }
}

/// 디렉터리 준비 실패를 사용자에게 보여줄 수 있는 domain 공통 실패로 옮긴다 (§13).
///
/// 앱 데이터 디렉터리가 없으면 저장소도 없다. 사용자 입장에서 이것은 "로컬 저장소를 준비하지
/// 못했다"는 한 가지 사실이므로 [`FailureKind::Storage`]로 보고한다.
///
/// 두 실패 모두 재시도 가능으로 본다 — 경로 결정 실패도, 디렉터리 생성 실패도 권한이나
/// 디스크 상태처럼 **바뀔 수 있는 조건** 때문에 일어난다.
impl From<AppDataDirError> for Failure {
    fn from(error: AppDataDirError) -> Self {
        use std::error::Error as _;

        let failure = Failure::retryable(FailureKind::Storage, error.to_string());
        match error.source() {
            Some(source) => failure.with_detail(source),
            None => failure,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 시스템 임시 디렉터리 아래의 고유 경로. Drop 시 지운다.
    ///
    /// 경로는 `std::env::temp_dir()`에서만 나오므로 테스트가 사용자 홈 디렉터리나
    /// 실제 앱 데이터 위치를 만들거나 오염시키는 일이 없다.
    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let unique = format!(
                "molt-note-test-{}-{}-{}",
                label,
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            );
            Self(std::env::temp_dir().join(unique))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn temp_root_stays_inside_the_system_temp_directory() {
        let temp = TempRoot::new("sandbox");
        assert!(
            temp.path().starts_with(std::env::temp_dir()),
            "테스트는 시스템 임시 디렉터리 밖에 쓰지 않는다: {}",
            temp.path().display()
        );
    }

    #[test]
    fn ensure_creates_the_directory_when_it_is_missing() {
        let temp = TempRoot::new("missing");
        let root = temp.path().join("app-data");
        assert!(!root.exists(), "사전 조건: 디렉터리가 아직 없어야 한다");

        let dir = AppDataDirectory::new(&root);
        dir.ensure().expect("디렉터리를 만들 수 있어야 한다");

        assert!(root.is_dir(), "ensure 후에는 디렉터리가 존재해야 한다");
    }

    #[test]
    fn ensure_succeeds_when_the_directory_already_exists() {
        let temp = TempRoot::new("existing");
        let root = temp.path().join("app-data");
        std::fs::create_dir_all(&root).expect("사전 조건: 디렉터리를 미리 만든다");
        let marker = root.join("marker.txt");
        std::fs::write(&marker, b"keep me").expect("사전 조건: 기존 내용을 둔다");

        let dir = AppDataDirectory::new(&root);
        dir.ensure().expect("이미 있어도 성공해야 한다");
        dir.ensure().expect("반복 호출해도 성공해야 한다");

        assert!(root.is_dir());
        assert_eq!(
            std::fs::read(&marker).expect("기존 파일이 남아 있어야 한다"),
            b"keep me",
            "ensure는 기존 내용을 지우지 않는다"
        );
    }

    #[test]
    fn database_path_is_derived_deterministically_from_the_root() {
        let temp = TempRoot::new("derived");
        let root = temp.path().join("app-data");

        let dir = AppDataDirectory::new(&root);
        let first = dir.database_path();
        let second = AppDataDirectory::new(&root).database_path();

        assert_eq!(first, second, "같은 루트면 같은 경로가 나와야 한다");
        assert_eq!(first, root.join(DATABASE_FILE_NAME));
        assert_eq!(first.parent(), Some(root.as_path()));
    }

    #[test]
    fn the_recordings_directory_is_derived_from_the_same_root_as_everything_else() {
        let temp = TempRoot::new("recordings-derived");
        let root = temp.path().join("app-data");

        let dir = AppDataDirectory::new(&root);

        assert_eq!(dir.recordings_dir(), root.join(RECORDINGS_DIR_NAME));
        assert_eq!(
            dir.recordings_dir(),
            AppDataDirectory::new(&root).recordings_dir(),
            "같은 루트면 같은 경로가 나와야 한다"
        );
        assert_eq!(dir.recordings_dir().parent(), Some(root.as_path()));
        assert_ne!(dir.recordings_dir(), dir.database_path());
    }

    #[test]
    fn ensuring_the_recordings_directory_keeps_what_is_already_there() {
        let temp = TempRoot::new("recordings-kept");
        let dir = AppDataDirectory::new(temp.path().join("app-data"));

        let created = dir
            .ensure_recordings_dir()
            .expect("녹음 디렉터리를 만들 수 있어야 한다");
        let existing = created.join("earlier.wav");
        std::fs::write(&existing, "이전 녹음").expect("사전 조건: 이전 녹음을 둔다");

        let again = dir
            .ensure_recordings_dir()
            .expect("두 번째 호출도 성공해야 한다");

        assert_eq!(again, created);
        assert_eq!(
            std::fs::read_to_string(&existing).expect("이전 녹음이 남아 있어야 한다"),
            "이전 녹음",
            "녹음 디렉터리를 준비하는 것이 기존 녹음을 지우지 않는다"
        );
    }

    #[test]
    fn a_recordings_directory_that_cannot_be_created_becomes_a_readable_failure() {
        // 디렉터리가 있어야 할 자리에 파일이 있다.
        let temp = TempRoot::new("recordings-blocked");
        let root = temp.path().join("app-data");
        std::fs::create_dir_all(&root).expect("사전 조건: 루트를 만든다");
        std::fs::write(root.join(RECORDINGS_DIR_NAME), "디렉터리가 아니다")
            .expect("사전 조건: 파일을 둔다");

        let error = AppDataDirectory::new(&root)
            .ensure_recordings_dir()
            .expect_err("파일 위에 디렉터리를 만들 수는 없다");
        let failure = Failure::from(error);

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(failure.message.contains(RECORDINGS_DIR_NAME));
        assert!(failure.source_data_safe, "아무것도 쓰지 못했다");
        assert!(failure.retryable);
    }

    #[test]
    fn database_path_differs_when_the_root_differs() {
        let temp = TempRoot::new("distinct");

        let a = AppDataDirectory::new(temp.path().join("a")).database_path();
        let b = AppDataDirectory::new(temp.path().join("b")).database_path();

        assert_ne!(a, b);
    }

    #[test]
    fn a_directory_that_cannot_be_created_becomes_a_failure_the_user_can_read() {
        // 실제 실패를 만든다: 디렉터리가 있어야 할 자리에 파일이 있다.
        let temp = TempRoot::new("blocked");
        std::fs::create_dir_all(temp.path()).expect("사전 조건: 부모 디렉터리를 만든다");
        let occupied = temp.path().join("app-data");
        std::fs::write(&occupied, "디렉터리가 아니다").expect("사전 조건: 파일을 둔다");

        let error = AppDataDirectory::new(&occupied)
            .ensure()
            .expect_err("파일 위에 디렉터리를 만들 수는 없다");
        let sentence = error.to_string();

        let failure = Failure::from(error);

        assert_eq!(failure.kind, FailureKind::Storage);
        assert_eq!(failure.message, sentence, "문장을 새로 지어내지 않는다");
        assert!(
            failure.message.contains(&occupied.display().to_string()),
            "어느 경로가 문제인지 보인다: {}",
            failure.message
        );
        assert!(failure.retryable, "조건이 바뀌면 성공할 수 있다");
        assert!(failure.source_data_safe);
        assert!(
            failure.detail.is_some(),
            "io 오류가 기술적 원인으로 함께 실린다"
        );
    }
}
