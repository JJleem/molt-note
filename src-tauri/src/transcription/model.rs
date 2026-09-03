//! 모델 파일을 해석하는 **단 한 곳** (ADR-0007 §8.2 · PRODUCT-SPEC §3.1 · INV-10).
//!
//! ```text
//! 설정 값(파일명 또는 절대 경로) + 모델 디렉터리
//!         │
//!         ▼
//!    이 모듈  ←── 모델 파일이 어디 있는지를 아는 코드는 여기뿐이다.
//!         │        엔진 구현도, 실행 경계도, 화면도 경로를 짓지 않는다.
//!         ▼
//!    ModelFile (존재하고 · 파일이고 · 열 수 있다)
//! ```
//!
//! **모델 디렉터리 자체는 여기서 만들지 않는다.** 그 경로는 이미 있는 플랫폼 경계에서 온다
//! ([`crate::platform::app_data_dir::AppDataDirectory::models_dir`]). 그래서 이 모듈에는
//! OS별 경로 문자열도 `cfg(target_os)` 분기도 없다 — 이 Phase에서 플랫폼이 실제로 갈리는
//! 지점은 그 경계 하나이며, 여기에 두 번째 경계를 만들지 않는다 (ADR-0007 §11 · §20.6).
//!
//! **모델을 내려받지 않는다.** 자동 다운로드는 DEFERRED이며 §12의 privacy 경계에 네트워크
//! 경로를 여는 결정이라 별도로 내린다 (ADR-0007 §8.1). 이 모듈이 하는 일은 이미 있는 파일을
//! 찾는 것뿐이고, 없으면 그것은 오류 로그가 아니라 **제품 상태**다 (§13).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::Failure;

use super::engine::{model_missing, model_unusable};

/// 실제로 존재하고 열 수 있는 모델 파일 하나.
///
/// **이 타입은 [`resolve`]로만 만들어진다.** 아무 경로나 담을 수 있는 생성자를 두지 않는 것은
/// "모델 위치를 해석하는 자리는 하나"라는 규칙을 타입이 지키게 하기 위해서다 — 다른 곳에서
/// 경로를 지어 엔진에 넘길 방법이 없다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFile {
    path: PathBuf,
    id: String,
}

impl ModelFile {
    /// 모델 파일의 실제 경로.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Transcript에 남길 모델 식별자 (§7 · ADR-0007 §8.2.4) — **실제로 쓴 파일의 이름**이다.
    ///
    /// 절대 경로를 쓰지 않는 것은 사용자의 홈 디렉터리 경로가 저장되고 내보내지는 것을
    /// 피하기 위해서다. 어떤 모델로 만든 전사인지는 파일 이름이 말해 준다.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// 설정 값에서 실제 모델 파일 하나를 해석한다.
///
/// `configured`는 사용자가 설정에 저장한 값이다 (`Settings::transcription_model`). 그 값을
/// 읽어 여기까지 넘기는 자리는 [`crate::commands::Transcriber`]다.
///
/// ```text
/// None · 빈 문자열   아직 모델을 고르지 않았다        → TranscriptionModelMissing
/// "ggml-base.bin"    모델 디렉터리 안의 파일          → models_dir/ggml-base.bin
/// "/path/to/x.bin"   사용자가 고른 자리              → 그대로
/// ```
///
/// 상대 경로를 모델 디렉터리 기준으로 푸는 것은 기본 탐색 위치가 거기이기 때문이고
/// (ADR-0007 §8.2), 절대 경로를 그대로 쓰는 것은 **모델을 어디에 둘지는 사용자가 정하기**
/// 때문이다 — 수 GB짜리 파일을 앱이 정한 자리로 옮기라고 요구하지 않는다.
///
/// 실패는 §13의 두 가지로만 나뉜다: **없는 것**과 **쓸 수 없는 것**. 둘을 구분하는 이유는
/// 사용자가 할 일이 다르기 때문이다 (`super::engine`의 표).
pub fn resolve(models_dir: &Path, configured: Option<&str>) -> Result<ModelFile, Failure> {
    let Some(value) = configured.map(str::trim).filter(|value| !value.is_empty()) else {
        return Err(model_missing(
            "전사에 쓸 모델 파일이 아직 지정되지 않았다",
        )
        .with_detail(format!("models_dir={}", models_dir.display())));
    };

    let candidate = PathBuf::from(value);
    let path = if candidate.is_absolute() {
        candidate
    } else {
        models_dir.join(candidate)
    };

    let metadata = fs::metadata(&path).map_err(|error| missing_or_unusable(&path, error))?;
    if !metadata.is_file() {
        return Err(
            model_unusable(format!("모델 파일이 아니다: {}", path.display()))
                .with_detail("경로가 파일이 아니라 디렉터리이거나 그 밖의 것이다"),
        );
    }
    if metadata.len() == 0 {
        return Err(
            model_unusable(format!("모델 파일이 비어 있다: {}", path.display())).with_detail(
                "내려받다 중단됐거나 옮기다 만 파일일 수 있다 (bytes=0)",
            ),
        );
    }

    // **열어서 확인한다.** 권한 때문에 읽을 수 없는 모델은 "없는 모델"이 아니라 "쓸 수 없는
    // 모델"이며, 그 사실이 엔진 안쪽이 아니라 여기서 드러나야 사용자가 무엇을 고칠지 안다.
    // 파일을 여는 것뿐이고 내용을 읽지도 쓰지도 않는다.
    fs::File::open(&path).map_err(|error| missing_or_unusable(&path, error))?;

    let id = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
        .to_owned();

    Ok(ModelFile { path, id })
}

/// 파일 시스템 오류를 §13의 두 실패로 가른다.
///
/// 없는 것과 접근하지 못하는 것을 뭉치지 않는다 — 전자는 모델을 구해 오면 풀리고 후자는
/// 그렇지 않다.
fn missing_or_unusable(path: &Path, error: io::Error) -> Failure {
    if error.kind() == io::ErrorKind::NotFound {
        model_missing(format!("모델 파일이 그 자리에 없다: {}", path.display())).with_detail(error)
    } else {
        model_unusable(format!("모델 파일을 열지 못했다: {}", path.display())).with_detail(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FailureKind;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
    ///
    /// **모델 파일은 저장소에 커밋하지 않는다** (ADR-0007 §8.3 · `.gitignore`의 `*.bin` ·
    /// `*.gguf` · `/models/`). 이 테스트들이 쓰는 "모델"은 전부 여기서 만든 몇 바이트짜리
    /// 파일이며, 실제 whisper 모델도 whisper 엔진도 필요하지 않다.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let unique = format!(
                "molt-note-transcription-model-{}-{}-{}",
                label,
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// 모델 파일 자리에 둘 몇 바이트짜리 파일. 내용은 이 모듈이 보지 않는다.
    fn placeholder(directory: &Path, name: &str) -> PathBuf {
        let path = directory.join(name);
        fs::write(&path, b"not a real model").expect("사전 조건: 파일을 만든다");
        path
    }

    #[test]
    fn a_file_name_is_resolved_inside_the_models_directory() {
        let temp = TempDir::new("by-name");
        let expected = placeholder(temp.path(), "ggml-base.bin");

        let model = resolve(temp.path(), Some("ggml-base.bin")).expect("있는 모델을 찾아야 한다");

        assert_eq!(model.path(), expected);
        assert_eq!(model.id(), "ggml-base.bin", "Transcript에는 파일 이름이 남는다");
    }

    #[test]
    fn an_absolute_path_is_used_as_the_user_gave_it() {
        // 수 GB짜리 파일을 앱이 정한 자리로 옮기라고 요구하지 않는다.
        let models = TempDir::new("absolute-models");
        let elsewhere = TempDir::new("absolute-elsewhere");
        let outside = placeholder(elsewhere.path(), "ggml-medium.bin");

        let model = resolve(models.path(), Some(outside.to_str().expect("경로 문자열")))
            .expect("사용자가 고른 자리의 모델을 써야 한다");

        assert_eq!(model.path(), outside);
        assert_eq!(model.id(), "ggml-medium.bin");
    }

    #[test]
    fn surrounding_whitespace_in_the_setting_does_not_hide_the_model() {
        let temp = TempDir::new("trimmed");
        placeholder(temp.path(), "ggml-base.bin");

        let model = resolve(temp.path(), Some("  ggml-base.bin \n"))
            .expect("설정 값의 공백 때문에 모델을 못 찾으면 안 된다");

        assert_eq!(model.id(), "ggml-base.bin");
    }

    #[test]
    fn no_model_configured_is_a_product_state_not_a_silent_skip() {
        // 모델을 고르지 않은 상태는 "전사를 건너뛴다"가 아니라 §13의 정의된 실패다.
        let temp = TempDir::new("unset");

        for configured in [None, Some(""), Some("   ")] {
            let failure = resolve(temp.path(), configured).expect_err("정의된 실패여야 한다");

            assert_eq!(failure.kind, FailureKind::TranscriptionModelMissing);
            assert!(!failure.retryable, "모델을 먼저 둬야 한다");
            assert!(failure.source_data_safe);
        }
    }

    #[test]
    fn a_model_that_is_not_there_is_missing_rather_than_unusable() {
        let temp = TempDir::new("absent");

        let failure = resolve(temp.path(), Some("ggml-large-v3.bin")).expect_err("없는 파일이다");

        assert_eq!(failure.kind, FailureKind::TranscriptionModelMissing);
        assert!(
            failure.message.contains("ggml-large-v3.bin"),
            "어느 파일을 찾았는지 사용자가 알아야 한다: {}",
            failure.message
        );
    }

    #[test]
    fn a_directory_in_the_place_of_a_model_is_unusable_rather_than_missing() {
        let temp = TempDir::new("directory");
        fs::create_dir(temp.path().join("ggml-base.bin")).expect("사전 조건: 디렉터리를 만든다");

        let failure = resolve(temp.path(), Some("ggml-base.bin")).expect_err("파일이 아니다");

        assert_eq!(failure.kind, FailureKind::TranscriptionModelUnusable);
        assert!(!failure.retryable);
    }

    #[test]
    fn an_empty_file_is_unusable_rather_than_a_model() {
        // 내려받다 중단된 파일이 흔한 경우다. 엔진에 넘기면 훨씬 뒤에서 알 수 없는 오류가 난다.
        let temp = TempDir::new("empty");
        fs::write(temp.path().join("ggml-base.bin"), b"").expect("사전 조건: 빈 파일을 만든다");

        let failure = resolve(temp.path(), Some("ggml-base.bin")).expect_err("빈 파일이다");

        assert_eq!(failure.kind, FailureKind::TranscriptionModelUnusable);
    }

    #[test]
    fn resolving_a_model_does_not_create_or_change_anything() {
        // 모델 디렉터리를 만들지도, 파일을 손대지도 않는다 (INV-1 · INV-3).
        let temp = TempDir::new("read-only");
        let path = placeholder(temp.path(), "ggml-base.bin");
        let before = fs::read(&path).expect("사전 조건: 파일을 읽는다");
        let missing_dir = temp.path().join("not-created");

        let _ = resolve(&missing_dir, Some("ggml-base.bin"));
        resolve(temp.path(), Some("ggml-base.bin")).expect("있는 모델");

        assert!(!missing_dir.exists(), "없는 모델 디렉터리를 만들지 않는다");
        assert_eq!(fs::read(&path).expect("파일이 남아 있어야 한다"), before);
        assert_eq!(
            fs::read_dir(temp.path())
                .expect("디렉터리를 읽는다")
                .count(),
            1,
            "파생 파일을 만들지 않는다"
        );
    }
}
