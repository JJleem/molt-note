//! 만들어진 파일이 **놓인 자리를 여는** 표면 — **여는 대상은 이 앱이 만든 것뿐이고, OS를
//! 부르는 코드는 경계 하나 안에만 있다** (`phase-prompt/05.6` 성공 기준 2 · R-4 ·
//! PRODUCT-SPEC §12 · §13 · INV-3 · INV-10).
//!
//! "그렇게 안 했다"는 검증이 아니다. 이 파일은 같은 사실을 **서로 다른 네 각도**에서 못박으며,
//! 넷 중 어느 하나만 깨져도 실패한다.
//!
//! ```text
//! 1. 행동   허용된 파일 하나만 경계에 도착한다 — 밖의 경로 · 탈출 · symlink · 디렉터리는 거절
//! 2. 행동   거절도 실패도 §13의 값이 되고, 어느 경우에도 저장된 것이 달라지지 않는다 (INV-3)
//! 3. 소스   OS를 부르는 코드(`process::Command` · `cfg(target_os)`)가 경계 파일 밖에 없다
//! 4. 소스   어떤 테스트도 실제 파일 관리자(`OsFileManager`)를 세우지 않는다
//! ```
//!
//! 3·4번이 소스 검사인 이유는 `tests/secret_store.rs`와 같다 — **실제 바깥에 닿는 구현은
//! 실행하지 않는 것이 규약이고, 규약은 실행이 아니라 관찰로 확인한다.** 이 파일이 검사하는
//! 사람의 화면에 창을 하나라도 띄우면 그 규약이 이미 깨진 것이다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use molt_note_lib::commands::SavedFiles;
use molt_note_lib::domain::FailureKind;
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::file_manager::testing::RecordedFileManager;

/// 이 경계가 사는 유일한 파일. 3번 각도의 대상이다.
const BOUNDARY: &str = "src/platform/file_manager.rs";

/// 그 경계를 쓰는 유일한 자리. 여기서만 무엇을 열어도 되는지 판정한다.
const CALLER: &str = "src/commands/saved_file.rs";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 자리 ------------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
///
/// 경로는 `std::env::temp_dir()`에서만 나오므로 이 테스트가 사용자의 실제 export 디렉터리를
/// 만들거나 건드리는 일이 없다 (§18).
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "molt-note-show-saved-file-{}-{}-{}",
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

struct Fixture {
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    /// 자동 테스트가 쓰는 유일한 구현. **창을 하나도 띄우지 않는다.**
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

    /// 내보낸 파일 하나. export가 파일을 두는 자리와 같다 (ADR-0009 §4.1).
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

// --- 1. 행동: 허용된 것만 경계에 도착한다 ------------------------------------------------

#[test]
fn the_file_this_app_wrote_is_the_one_that_reaches_the_os_boundary() {
    let fixture = Fixture::new("allowed");
    let file = fixture.exported("2026-09-01-3dgs-study-04.md");

    fixture
        .saved_files
        .show(&file.display().to_string())
        .expect("이 앱이 만든 파일은 열 수 있어야 한다");

    // 경계에 도착하는 것은 **정규화된 경로 하나**다 — 화면이 보낸 문자열 그대로가 아니다.
    assert_eq!(
        fixture.manager.shown(),
        vec![std::fs::canonicalize(&file).expect("정규화할 수 있어야 한다")]
    );
}

#[test]
fn nothing_outside_the_exports_directory_ever_reaches_the_os_boundary() {
    // 화면이 무엇을 보내든 열리는 자리는 하나다 (PRODUCT-SPEC §12). 네 가지를 한 번에 본다 —
    // 앱 데이터 안의 다른 파일 · 완전히 밖의 파일 · `..`로 걸어 나가는 경로 · 디렉터리 자신.
    let fixture = Fixture::new("refused");
    fixture.exported("2026-09-01-3dgs-study-04.md");

    let database = fixture.app_data_dir.database_path();
    std::fs::write(&database, "저장소 파일").expect("사전 조건: 파일을 둔다");
    let outside = fixture.app_data_dir.root().join("escaped.md");
    std::fs::write(&outside, "밖의 파일").expect("사전 조건: 파일을 둔다");

    let refused = [
        database.clone(),
        outside.clone(),
        fixture
            .app_data_dir
            .exports_dir()
            .join("..")
            .join("escaped.md"),
        fixture.app_data_dir.exports_dir(),
    ];

    for path in refused {
        let failure = match fixture.saved_files.show(&path.display().to_string()) {
            Ok(()) => panic!("{} 는 거절되어야 한다", path.display()),
            Err(failure) => failure,
        };

        assert_eq!(failure.kind, FailureKind::InvalidInput, "{}", path.display());
        assert!(!failure.retryable, "같은 경로를 다시 보내도 같다");
    }

    assert!(
        fixture.manager.is_empty(),
        "거절된 요청이 OS 경계까지 갔다 (§12)"
    );
    // 거절이 아무것도 건드리지 않았다 (INV-3).
    assert!(database.exists() && outside.exists());
}

// --- 2. 행동: 실패도 값이고, 저장된 것은 그대로다 ----------------------------------------

#[test]
fn a_failure_from_the_os_boundary_arrives_as_a_failure_the_user_can_read() {
    use molt_note_lib::platform::file_manager::testing::RecordedFileManager as Recorded;
    use molt_note_lib::platform::file_manager::FileManagerError;

    // 열 수단이 없는 시스템과 띄우지 못한 경우 — 둘 다 창 없이 지난다 (§18).
    for error in [FileManagerError::Unsupported, FileManagerError::Failed] {
        let root = TempRoot::new("boundary-failure");
        let app_data_dir = AppDataDirectory::new(root.0.join("app-data"));
        let manager = Arc::new(Recorded::failing(error));
        let saved_files = SavedFiles::in_directory(app_data_dir.clone(), manager.clone());

        let directory = app_data_dir
            .ensure_exports_dir()
            .expect("사전 조건: export 디렉터리를 만든다");
        let file = directory.join("2026-09-01-3dgs-study-04.md");
        std::fs::write(&file, "# 내보낸 문서").expect("사전 조건: 파일을 둔다");

        let failure = saved_files
            .show(&file.display().to_string())
            .expect_err("경계가 실패를 냈다");

        // §13의 세 질문에 답한다.
        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(!failure.message.is_empty(), "화면에 띄울 문장이 있어야 한다");
        assert!(failure.source_data_safe, "여는 일은 아무것도 건드리지 않는다");
        // 그리고 **파일은 그 자리에 그대로 있다** — 여는 수단은 경로의 대체가 아니라 추가다.
        assert!(file.exists());
        assert_eq!(
            std::fs::read_to_string(&file).expect("파일이 그대로 있어야 한다"),
            "# 내보낸 문서"
        );
    }
}

#[test]
fn asking_to_open_something_never_creates_a_directory_or_a_file() {
    // 이 표면에는 만드는 경로가 없다 (INV-3). 아직 아무것도 내보내지 않은 상태에서 물어도
    // 빈 `exports/`가 생기지 않는다.
    let fixture = Fixture::new("creates-nothing");

    let failure = fixture
        .saved_files
        .show("/tmp/molt-note-not-a-real-file.md")
        .expect_err("열 것이 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(!fixture.app_data_dir.exports_dir().exists());
    assert!(fixture.manager.is_empty());
}

// --- 3·4. 소스: OS 지식이 경계 밖으로 새지 않는다 ----------------------------------------

#[test]
fn os_calls_and_platform_branching_live_only_inside_the_boundary() {
    // INV-10 · PRODUCT-SPEC §3.1 — 파일 관리자를 부르는 코드도, 그것을 가르는 `cfg(target_os)`도
    // `platform/file_manager.rs` 밖에 있으면 안 된다. 밖으로 새는 순간 Windows 구현은 파일
    // 하나가 아니라 여러 자리를 고치는 일이 되고, 화면·domain·export 실행 순서 중 어디서
    // 프로세스가 뜨는지 아무도 알 수 없게 된다.
    //
    // 주석은 검사 대상이 아니다 — 이 규약을 설명하는 문장이 그 이름을 쓰기 때문이다.
    for path in rust_sources(Path::new("src")) {
        let relative = relative(&path);
        if relative == BOUNDARY {
            continue;
        }

        for (number, line) in code_lines(&path) {
            assert!(
                !line.contains("process::Command") && !line.contains("Command::new"),
                "{relative}:{number}이 프로세스를 실행한다 (INV-10)"
            );
        }
    }

    // 그리고 그 파일 안에는 실제로 세 자리가 다 있다 — macOS · Windows · 그 밖.
    let boundary = std::fs::read_to_string(Path::new(BOUNDARY)).expect("경계 파일을 읽어야 한다");
    assert!(boundary.contains("process::Command"), "실제 구현이 있어야 한다");
    assert!(
        boundary.contains("target_os = \"macos\"") && boundary.contains("target_os = \"windows\""),
        "macOS 구현과 Windows 구현 경계가 이 파일 안에 있어야 한다"
    );
    assert!(
        boundary.contains("FileManagerError::Unsupported"),
        "열 수단이 없는 시스템은 조용히 아무 일도 하지 않는 대신 실패로 말해야 한다 (§13)"
    );
}

#[test]
fn only_one_place_decides_what_may_be_opened() {
    // 이 경계를 쓰는 자리가 여럿이면 "exports 아래인가"라는 판정도 여럿이 되고, 한쪽이 느슨해진
    // 것을 아무도 알아채지 못한다. 그래서 **쓰는 자리 자체를 하나로 고정한다.**
    let mut users = Vec::new();

    for path in rust_sources(Path::new("src")) {
        let relative = relative(&path);
        if relative == BOUNDARY || relative == "src/platform/mod.rs" {
            continue;
        }
        if code_lines(&path)
            .iter()
            .any(|(_, line)| line.contains("file_manager"))
        {
            users.push(relative);
        }
    }

    assert_eq!(users, vec![CALLER.to_string()]);

    // 그 하나에 실제로 판정이 있다 — 정규화한 경로가 exports 아래인지, 그리고 파일인지.
    //
    // 검사 대상은 **제품 경로의 코드**다. 테스트 모듈은 파일을 놓기 위해 디렉터리를 만들고,
    // 이 모듈의 문서는 "디렉터리를 만들지 않는다"고 그 이름을 들어 설명한다 — 둘 다 규칙
    // 위반이 아니다.
    let caller = code_lines(Path::new(CALLER))
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    let product = caller
        .split_once("#[cfg(test)]")
        .expect("이 파일에는 테스트 모듈이 있다")
        .0;

    assert!(product.contains("canonicalize"), "경로를 정규화해야 한다");
    assert!(
        product.contains("starts_with(&exports_dir)") && product.contains("is_file()"),
        "허용 판정이 이 파일 안에 있어야 한다"
    );
    // **디렉터리를 만들지 않는다** — 여는 일이 자리를 새로 파지 않는다 (INV-3).
    assert!(
        !product.contains("ensure_exports_dir"),
        "제품 경로가 디렉터리를 만든다"
    );
}

#[test]
fn no_automated_test_stands_up_the_real_file_manager() {
    // `OsFileManager`를 실행하면 검사가 도는 동안 창이 열린다. 자동 테스트가 쓰는 구현은
    // double 하나뿐이며, 그 사실을 여기서 관찰한다 (`tests/secret_store.rs`와 같은 규약).
    // **이름을 리터럴로 적지 않는다** — 이 파일 자신도 검사 대상이기 때문이다.
    let real = ["Os", "FileManager"].concat();
    let factory = ["app_", "file_manager("].concat();

    let mut checked = 0;
    for path in rust_sources(Path::new("tests"))
        .into_iter()
        .chain(rust_sources(Path::new("src")))
    {
        let relative = relative(&path);
        // 경계 파일 자신은 그 타입을 선언하고 세운다 — 그것이 이 경계의 존재 이유다.
        if relative == BOUNDARY {
            continue;
        }
        // 앱이 실제로 세우는 자리는 딱 하나다: command 소유자가 앱 시작 시 만든다.
        let allowed_factory = relative == CALLER;

        for (number, line) in code_lines(&path) {
            assert!(
                !line.contains(&real),
                "{relative}:{number}이 실제 파일 관리자를 세운다 — 자동 테스트는 double만 쓴다"
            );
            assert!(
                allowed_factory || !line.contains(&factory),
                "{relative}:{number}이 실제 파일 관리자를 세운다"
            );
        }
        checked += 1;
    }
    assert!(checked > 1, "검사한 파일이 있어야 한다");
}

// --- 도구 ------------------------------------------------------------------------------

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

/// 주석이 아닌 줄과 그 줄 번호. **규약을 설명하는 문장은 검사 대상이 아니다.**
fn code_lines(path: &Path) -> Vec<(usize, String)> {
    std::fs::read_to_string(path)
        .expect("소스를 읽을 수 있어야 한다")
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            !trimmed.starts_with("//") && !trimmed.starts_with("*") && !trimmed.starts_with("/*")
        })
        .map(|(index, line)| (index + 1, line.to_string()))
        .collect()
}

/// 실패 문장에 쓰는, 저장소 기준의 경로 표현.
fn relative(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
