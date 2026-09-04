//! 만들어진 Markdown 문자열을 **실제 파일 하나로 떨어뜨리는 자리** (ADR-0009 §4.3).
//!
//! ```text
//! (디렉터리 · 파일 이름 · 문자열) ─→ write_new ─→ 쓰인 파일 하나 (WrittenFile)
//! ```
//!
//! 이 모듈이 아는 것은 그것뿐이다 — 저장소도, Recording도, 렌더링 규칙도 여기 없다.
//! 무엇을 쓸지 정하는 자리는 [`super::run`]이고, 이 파일은 **어디에 어떻게 놓는가**만 안다.
//!
//! ## 덮어쓰지 않는다 (ADR-0009 §4.3)
//!
//! ```text
//! 2026-09-01-3dgs-study-04.md        없으면 이것
//! 2026-09-01-3dgs-study-04-2.md      있으면 이것
//! 2026-09-01-3dgs-study-04-3.md      …
//! 1000번째까지 자리가 없으면 → FailureKind::Storage 로 보이는 실패
//! ```
//!
//! export한 파일은 **사용자의 문서다.** Obsidian에서 손댔을 수도 있고 다른 곳으로 옮기는
//! 중일 수도 있다. 덮어쓰면 그것을 되돌릴 방법이 없다 (INV-4의 태도). 규칙과 상한(1000)은
//! [`crate::audio::capture::output_path`]와 같다 — 사용자가 두 종류의 규칙을 배우지 않아도
//! 되게 하기 위해서다.
//!
//! **자리를 찾는 것과 쓰는 것 사이에는 경합이 있다.** 확인한 뒤 쓰기 전에 다른 프로세스가
//! 같은 이름을 만들 수 있으므로, 여기서는 `exists()`로 묻지 않고 **`create_new`(이미 있으면
//! 실패)로 만들고 그 실패를 다음 번호로 넘어가는 신호로 쓴다.** 그래서 "확인했을 때는 없었다"는
//! 이유로 남의 파일을 여는 경로가 없다.
//!
//! ## 지우는 코드가 없다 (INV-3 · INV-4)
//!
//! 이 모듈에는 `remove_file`도 `remove_dir`도 없다. 쓰다가 실패해 **일부만 쓰인 파일이 남아도
//! 지우지 않고, 그 경로를 실패 문장에 담아 보낸다** — 앱이 만든 것이라는 이유로 파일을 지우기
//! 시작하면 "무엇이 지워질 수 있는가"의 답이 늘어난다. 어느 실패 경로도 recording · transcript ·
//! ai_note를 건드리지 않는다: 이 모듈은 저장소를 알지 못한다.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::domain::{Failure, FailureKind};

/// 같은 이름을 몇 번까지 비켜 볼 것인가 (ADR-0009 §4.3).
///
/// [`crate::audio::capture`]의 상한과 같은 값이며, 여기까지 전부 차 있으면 이름을 만들지 못한
/// 것이다 — 조용히 덮어쓰는 대신 실패로 끝난다.
const MAX_NAME_ATTEMPTS: u32 = 1_000;

/// 이름을 비켜 갈 때 번호 앞에 붙는 구분자.
const SEPARATOR: char = '-';

/// 실제로 쓰인 파일 하나.
///
/// **요청한 이름과 다를 수 있다.** 같은 이름이 이미 있었으면 번호가 붙기 때문이며, 그래서
/// 부르는 쪽이 이름을 다시 짐작하지 않도록 쓰인 이름을 함께 돌려준다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenFile {
    /// 사용자에게 보여줄 전체 경로 (ADR-0009 §4.1 — 파일을 찾지 못하는 상태로 두지 않는다).
    pub path: PathBuf,
    /// 그 파일의 이름. `2026-09-01-3dgs-study-04-2.md`처럼 번호가 붙어 있을 수 있다.
    pub name: String,
}

/// 주어진 디렉터리에 **기존 파일을 건드리지 않고** 새 파일 하나를 쓴다.
///
/// 이름이 이미 쓰이고 있으면 번호를 붙여 비켜 간다 ([`MAX_NAME_ATTEMPTS`]번까지). 디렉터리는
/// 여기서 만들지 않는다 — 자리를 준비하는 일은 [`crate::platform::app_data_dir`]의 몫이고,
/// 그 실패도 그쪽이 §13의 실패로 옮긴다 (INV-10).
pub fn write_new(directory: &Path, file_name: &str, contents: &str) -> Result<WrittenFile, Failure> {
    write_new_within(directory, file_name, contents, MAX_NAME_ATTEMPTS)
}

/// 몇 번까지 비켜 볼지를 인자로 받는 실제 구현.
///
/// 상한을 값으로 받는 이유는 하나다 — **자리를 찾지 못하는 경로를 파일 1000개 없이 검증하기
/// 위해서다.** 제품이 쓰는 값은 [`write_new`]가 정한다.
fn write_new_within(
    directory: &Path,
    file_name: &str,
    contents: &str,
    attempts: u32,
) -> Result<WrittenFile, Failure> {
    for attempt in 0..attempts {
        let name = numbered(file_name, attempt);
        let candidate = directory.join(&name);

        match create_new(&candidate) {
            Ok(mut file) => {
                // 여기부터는 **이 호출이 만든 파일**이다. 쓰지 못했다면 그 사실을 알리되,
                // 일부만 쓰인 파일을 지우지 않는다 (모듈 문서).
                return match file.write_all(contents.as_bytes()).and_then(|()| file.flush()) {
                    Ok(()) => Ok(WrittenFile {
                        path: candidate,
                        name,
                    }),
                    Err(error) => Err(not_written(&candidate, error)),
                };
            }
            // 이미 있는 파일은 **열리지도 않았다.** 그것이 이 신호의 값어치다 — 덮어쓰지 않았음이
            // 확인된 채로 다음 번호로 넘어간다.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(not_created(&candidate, error)),
        }
    }

    Err(no_free_name(directory, file_name))
}

/// `attempt`번째 후보 이름. 0번째는 요청한 이름 그대로다.
///
/// 번호는 **확장자 앞**에 붙는다 — `2026-09-01-study.md-2`가 되면 Markdown 파일이 아니게 되고,
/// 외부 도구에서 그대로 열 수 있어야 한다는 §11의 요구가 깨진다.
fn numbered(file_name: &str, attempt: u32) -> String {
    if attempt == 0 {
        return file_name.to_owned();
    }

    // 첫 비켜남이 `-2`다 (ADR-0009 §4.3). `-1`은 원본과 같은 것을 가리키는 것처럼 읽힌다.
    let number = attempt + 1;
    match split_extension(file_name) {
        Some((stem, extension)) => format!("{stem}{SEPARATOR}{number}.{extension}"),
        None => format!("{file_name}{SEPARATOR}{number}"),
    }
}

/// 마지막 `.` 앞뒤로 나눈다. 앞이 비어 있으면(`.hidden`) 확장자로 보지 않는다.
fn split_extension(file_name: &str) -> Option<(&str, &str)> {
    let (stem, extension) = file_name.rsplit_once('.')?;

    (!stem.is_empty()).then_some((stem, extension))
}

/// 그 자리에 아직 아무것도 없을 때만 파일을 만든다.
///
/// **이 앱이 파일을 여는 방식 중 유일하게 `create_new`인 자리다** — 기존 파일을 여는 순간
/// 사용자의 문서를 덮어쓸 수 있기 때문이다.
fn create_new(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// 파일을 만들지도 못했다 — 디렉터리가 없거나 권한이 없는 경우다.
///
/// **아무것도 쓰지 않았다.** 조건이 바뀌면 성공할 수 있으므로 재시도 가능한 실패다 (§13).
fn not_created(path: &Path, error: io::Error) -> Failure {
    Failure::retryable(
        FailureKind::Storage,
        format!("Markdown 파일을 만들지 못했다: {}", path.display()),
    )
    .with_detail(error)
}

/// 파일은 만들었지만 내용을 끝까지 쓰지 못했다 (디스크가 찼거나 장치가 사라졌다).
///
/// **그 파일은 지우지 않고 그대로 둔다** (모듈 문서). 그래서 실패 문장이 어디에 남았는지
/// 말한다 — 지우지 않는 파일은 찾을 수 있어야 한다.
///
/// `source_data_safe`는 그대로 참이다. 이 실패가 만들지 못한 것은 **새 산출물**이며,
/// recording · transcript · ai_note는 이 경로가 애초에 닿지 못하는 자리에 있다 (INV-3).
fn not_written(path: &Path, error: io::Error) -> Failure {
    Failure::retryable(
        FailureKind::Storage,
        format!(
            "Markdown 파일을 끝까지 쓰지 못했다. 쓰다 만 파일이 남아 있다: {}",
            path.display()
        ),
    )
    .with_detail(error)
}

/// 비켜 갈 이름이 다 찼다. **덮어쓰는 대신 보이는 실패로 끝난다** (ADR-0009 §4.3).
fn no_free_name(directory: &Path, file_name: &str) -> Failure {
    Failure::permanent(
        FailureKind::Storage,
        "Markdown 파일을 둘 이름을 찾지 못했다. 같은 이름의 파일이 이미 너무 많다.",
    )
    .with_detail(format!("directory={} · name={file_name}", directory.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 시스템 임시 디렉터리 아래의 고유 경로. Drop 시 지운다.
    ///
    /// 경로는 `std::env::temp_dir()`에서만 나오므로 **사용자의 실제 export 디렉터리를 건드리는
    /// 자동 검증이 없다.**
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let unique = format!(
                "molt-note-export-file-{}-{}-{}",
                label,
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_requested_name_is_used_when_nothing_is_there() {
        let temp = TempDir::new("fresh");

        let written = write_new(temp.path(), "2026-09-01-study.md", "# 제목\n")
            .expect("빈 디렉터리에는 그대로 쓸 수 있어야 한다");

        assert_eq!(written.name, "2026-09-01-study.md");
        assert_eq!(written.path, temp.path().join("2026-09-01-study.md"));
        assert_eq!(
            std::fs::read_to_string(&written.path).expect("쓴 파일을 읽을 수 있어야 한다"),
            "# 제목\n",
            "받은 문자열이 그대로 파일이 된다 — 여기서 내용을 고치지 않는다"
        );
    }

    #[test]
    fn a_name_that_is_taken_is_stepped_over_instead_of_overwritten() {
        // ADR-0009 §4.3의 결정 그 자체다. 이미 있는 파일은 **열리지도 않는다.**
        let temp = TempDir::new("collision");
        let taken = temp.path().join("2026-09-01-study.md");
        std::fs::write(&taken, "사용자가 손댄 문서").expect("사전 조건: 같은 이름을 둔다");

        let second = write_new(temp.path(), "2026-09-01-study.md", "두 번째")
            .expect("이름이 겹쳐도 export는 성공한다");
        let third = write_new(temp.path(), "2026-09-01-study.md", "세 번째")
            .expect("세 번째도 성공한다");

        assert_eq!(second.name, "2026-09-01-study-2.md");
        assert_eq!(third.name, "2026-09-01-study-3.md");
        assert_eq!(
            std::fs::read_to_string(&taken).expect("기존 파일이 남아 있어야 한다"),
            "사용자가 손댄 문서",
            "기존 파일은 조용히 덮어써지지 않는다"
        );
        assert_eq!(
            std::fs::read_to_string(&second.path).expect("두 번째 파일을 읽는다"),
            "두 번째"
        );
        assert_eq!(
            std::fs::read_to_string(&third.path).expect("세 번째 파일을 읽는다"),
            "세 번째"
        );
    }

    #[test]
    fn the_number_goes_before_the_extension() {
        assert_eq!(numbered("2026-09-01-study.md", 0), "2026-09-01-study.md");
        assert_eq!(numbered("2026-09-01-study.md", 1), "2026-09-01-study-2.md");
        assert_eq!(numbered("2026-09-01-study.md", 9), "2026-09-01-study-10.md");
        // 확장자가 없어도 이름을 망가뜨리지 않는다.
        assert_eq!(numbered("study", 1), "study-2");
        // 숨은 파일처럼 보이는 이름의 앞부분을 확장자로 오해하지 않는다.
        assert_eq!(numbered(".md", 1), ".md-2");
    }

    #[test]
    fn running_out_of_names_is_a_visible_failure_rather_than_an_overwrite() {
        // 자리를 못 찾으면 **덮어쓰지 않고 실패한다** (ADR-0009 §4.3). 상한을 값으로 받는
        // 구현이라 파일 1000개 없이 이 경로를 지난다.
        let temp = TempDir::new("exhausted");
        for name in ["a.md", "a-2.md", "a-3.md"] {
            std::fs::write(temp.path().join(name), "이미 있다").expect("사전 조건");
        }

        let failure = write_new_within(temp.path(), "a.md", "새 내용", 3)
            .expect_err("세 자리가 다 찼으면 이름을 만들지 못한다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(!failure.retryable, "같은 상태로 다시 해도 결과가 같다");
        assert!(failure.source_data_safe, "아무것도 쓰지 못했다 (INV-3)");
        for name in ["a.md", "a-2.md", "a-3.md"] {
            assert_eq!(
                std::fs::read_to_string(temp.path().join(name)).expect("남아 있어야 한다"),
                "이미 있다",
                "실패해도 기존 파일은 그대로다"
            );
        }
    }

    #[test]
    fn a_directory_that_is_not_there_becomes_a_readable_failure() {
        // 쓰기 실패는 §13의 세 질문에 답하는 domain Failure다 — panic도, 조용한 성공도 아니다.
        let temp = TempDir::new("missing-directory");
        let missing = temp.path().join("없는-디렉터리");

        let failure = write_new(&missing, "2026-09-01-study.md", "내용")
            .expect_err("없는 디렉터리에는 쓸 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(failure.retryable, "자리가 생기면 성공할 수 있다");
        assert!(failure.source_data_safe, "원본은 이 경로가 닿지도 않는다");
        assert!(
            failure.message.contains("2026-09-01-study.md"),
            "어느 파일을 쓰려 했는지 보인다: {}",
            failure.message
        );
        assert!(failure.detail.is_some(), "io 오류가 기술적 원인으로 실린다");
        assert!(!missing.exists(), "실패한 경로에 디렉터리를 만들어 두지 않는다");
    }

    #[test]
    fn the_written_path_is_the_full_path_the_user_can_be_shown() {
        // 사용자가 파일을 찾지 못하는 상태로 두지 않는다 (ADR-0009 §4.1).
        let temp = TempDir::new("full-path");

        let written = write_new(temp.path(), "2026-09-01-study.md", "내용").expect("쓸 수 있어야 한다");

        assert!(written.path.is_absolute() || written.path.starts_with(temp.path()));
        assert_eq!(written.path.parent(), Some(temp.path()));
        assert_eq!(
            written.path.file_name().and_then(|name| name.to_str()),
            Some(written.name.as_str()),
            "돌려준 이름과 실제 파일 이름이 같다"
        );
    }
}
