//! **레코드 없이 남은 녹음 파일을 찾아낸다** (2026-09-15).
//!
//! ## 왜 있는가
//!
//! 정지는 두 걸음이다 — 파일을 확정하고, 레코드를 저장한다. 그 사이에서 무슨 일이든
//! 생기면 **디스크에는 온전한 녹음이 있는데 앱은 그것을 모른다.** 사용자에게는 녹음이
//! 사라진 것으로 보인다.
//!
//! 2026-09-15에 실제로 그 일이 있었다. 15.7분짜리 회의 녹음이 확정까지 되고도 목록에
//! 없었고, 사람이 DB를 직접 고쳐 되살려야 했다. **앱에는 그것을 알아챌 방법도, 되살릴
//! 방법도 없었다.**
//!
//! 반대 방향(`레코드는 있는데 파일이 없다`)은 이미 `finalized::audio_is_present`가
//! 다룬다. 이 모듈은 그 짝이다.
//!
//! ## 이 모듈이 하는 일과 하지 않는 일
//!
//! **찾기만 한다.** 되살릴지, 무엇을 제목으로 붙일지, 언제 할지는 부르는 쪽이 정한다 —
//! 그래야 파일도 저장소도 없이 이 규칙을 그대로 지날 수 있다 (§18).
//!
//! **지우지 않는다.** 어떤 경우에도 파일을 건드리지 않는다 (INV-1).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 녹음 디렉터리에서 이 확장자만 본다. `capture::CONTAINER`가 정한 것과 같다.
const EXTENSION: &str = "wav";

/// 이보다 작은 파일은 녹음으로 보지 않는다.
///
/// 헤더만 있고 소리가 없는 파일이 남을 수 있다 — 시작하자마자 실패한 경우다. 그것을
/// 되살리면 **비어 있는 녹음**이 목록에 생기고, 사용자는 그것이 무엇인지 알 수 없다.
/// `finalized::MIN_FINALIZED_BYTES`와 같은 뜻의 문턱이다.
pub const MIN_BYTES: u64 = 1_024;

/// 레코드 없이 남은 녹음 파일 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orphan {
    pub path: PathBuf,
    /// 파일 크기(byte). 되살릴 값을 고르는 쪽이 본다.
    pub bytes: u64,
}

/// 디렉터리에 있는 `.wav` 중 **알려진 경로에 없는 것**을 찾는다.
///
/// 비교는 경로 문자열로 한다 — 레코드가 들고 있는 것이 그것이기 때문이다. 디렉터리를
/// 읽지 못하면 빈 목록이다: **찾지 못한 것은 실패가 아니다.** 시작을 막을 이유가 없다.
pub fn find(directory: &Path, known_paths: &HashSet<String>) -> Vec<Orphan> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut found: Vec<Orphan> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some(EXTENSION) {
                return None;
            }
            let bytes = entry.metadata().ok()?.len();
            if bytes < MIN_BYTES {
                return None;
            }
            if known_paths.contains(path.to_str()?) {
                return None;
            }
            Some(Orphan { path, bytes })
        })
        .collect();

    // 오래된 것부터. 되살린 뒤의 목록이 시간 순서와 어긋나지 않게 한다.
    found.sort_by(|left, right| left.path.cmp(&right.path));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct Dir(PathBuf);

    impl Dir {
        fn new() -> Self {
            let index = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("molt-orphan-{}-{index}", std::process::id()));
            std::fs::create_dir_all(&path).expect("만들 수 있어야 한다");
            Self(path)
        }

        fn file(&self, name: &str, bytes: usize) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, vec![0_u8; bytes]).expect("쓸 수 있어야 한다");
            path
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn known(paths: &[&PathBuf]) -> HashSet<String> {
        paths
            .iter()
            .map(|path| path.to_str().expect("경로").to_owned())
            .collect()
    }

    #[test]
    fn a_recording_with_no_record_is_found() {
        // 지키려는 것: **확정된 녹음이 조용히 사라지지 않는다.** 2026-09-15에 실제로
        // 15.7분짜리가 이렇게 남았고, 사람이 DB를 고쳐야 했다.
        let dir = Dir::new();
        let lost = dir.file("capture-1789437702.wav", 90_000);

        let found = find(&dir.0, &HashSet::new());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, lost);
        assert_eq!(found[0].bytes, 90_000);
    }

    #[test]
    fn a_recording_that_is_already_known_is_not_found_again() {
        // 되살리기를 두 번 하면 같은 녹음이 목록에 둘이 된다.
        let dir = Dir::new();
        let saved = dir.file("capture-1.wav", 90_000);

        assert!(find(&dir.0, &known(&[&saved])).is_empty());
    }

    #[test]
    fn only_the_unknown_ones_come_back() {
        let dir = Dir::new();
        let saved = dir.file("capture-1.wav", 90_000);
        let lost = dir.file("capture-2.wav", 50_000);

        let found = find(&dir.0, &known(&[&saved]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, lost);
    }

    #[test]
    fn a_file_with_only_a_header_is_left_alone() {
        // 소리가 없는 파일을 되살리면 **비어 있는 녹음**이 목록에 생기고, 사용자는
        // 그것이 무엇인지 알 수 없다.
        let dir = Dir::new();
        dir.file("capture-empty.wav", 44);

        assert!(find(&dir.0, &HashSet::new()).is_empty());
    }

    #[test]
    fn other_files_are_not_recordings() {
        let dir = Dir::new();
        dir.file("메모.txt", 90_000);
        dir.file("capture.wav.bak", 90_000);

        assert!(find(&dir.0, &HashSet::new()).is_empty());
    }

    #[test]
    fn a_directory_that_cannot_be_read_is_not_a_failure() {
        // **찾지 못한 것은 실패가 아니다.** 시작을 막을 이유가 없다.
        assert!(find(Path::new("/그런/자리는/없다"), &HashSet::new()).is_empty());
    }
}
