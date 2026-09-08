//! 무음 구간을 디코더에 **주지 않는다** — VAD 모델 파일을 찾고, 없으면 없는 대로 간다.
//!
//! ## 왜 있는가 — 2026-09-08에 두 번 관측된 것
//!
//! whisper는 **말이 없는 구간에서 학습 데이터의 자막 상투구를 뱉는다.**
//!
//! ```text
//! 9/4 회의 (72분)   00:57~01:08의 10분      "한글자막 by 한효정" · "이 시각 세계였습니다"
//!                   그 10분은 나머지보다 10.3 dB 조용했다 (30초 창 20개 전부)
//! 9/8 회의 (124분)  회의가 끝난 뒤 조용한 구간  "수리 : 응!" · "다음 영상에서 만나요."
//! ```
//!
//! **같은 패턴이 다른 녹음 · 다른 화자 · 다른 날에 재현됐다.** 말이 있는 구간은 멀쩡하다.
//!
//! ## 왜 임계값이 아니라 VAD인가
//!
//! whisper.cpp에도 무음 판정이 있지만 조건이 **AND**다.
//!
//! ```c
//! is_no_speech = (no_speech_prob > no_speech_thold) && (avg_logprobs < logprob_thold)
//! ```
//!
//! 자막 상투구는 **모델이 외운 문장이라 confidence가 높다.** `avg_logprobs`가 임계값 아래로
//! 내려가지 않으면 이 조건에 걸리지 않는다. 그래서 임계값만 만지는 것은 미덥지 않다.
//!
//! VAD는 다르다. 무음을 **샘플 버퍼에서 실제로 들어낸 뒤** 디코딩하므로
//! (vendored `whisper.cpp:6615-6790`) 디코더가 그 구간을 볼 일이 없다. 뱉을 자리가 없다.
//!
//! ## 시간축은 깨지지 않는다
//!
//! VAD가 압축한 시간축은 `vad_mapping_table`로 **원래 시각으로 되돌려진 뒤**
//! `whisper_full_get_segment_t0/t1`이 나온다 (vendored `whisper.cpp:7912-7964`).
//! 따라서 ADR-0007 §20.5의 "원시 단위(센티초)에서 청크 오프셋을 더한다"가 그대로 유효하다.
//!
//! ## 이 모듈이 하지 않는 것
//!
//! **모델 파일을 내려받지 않는다.** 네트워크를 모른다. 있는지 보고, 있으면 경로를 준다.
//! **없는 것은 실패가 아니다** — VAD 없이 전사하는 것은 2026-09-08까지의 정상 동작이며,
//! 그 경로는 지금과 한 글자도 다르지 않다.

use std::fs;
use std::path::{Path, PathBuf};

/// 이 앱이 찾는 VAD 모델 파일 이름.
///
/// whisper.cpp가 배포하는 이름 그대로다 (`models/download-vad-model.sh`가 만드는 파일명 ·
/// `https://huggingface.co/ggml-org/whisper-vad`). **이 앱이 지어낸 이름이 아니다.**
///
/// v6.2.0도 배포되지만 이 저장소가 쓰는 `whisper-rs-sys 0.15.0`의 vendored whisper.cpp가
/// 그것을 읽는지 **확인한 적이 없다.** 확인된 것만 기본으로 둔다.
pub const VAD_MODEL_FILE: &str = "ggml-silero-v5.1.2.bin";

/// VAD를 쓸 수 있는가, 쓴다면 어느 파일인가.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VadChoice {
    /// 모델 파일이 있다. 이 경로로 VAD를 켠다.
    Enabled(PathBuf),
    /// 모델 파일이 없다. **정상 상태다** — VAD 없이 전사한다.
    Disabled,
}

impl VadChoice {
    /// 켤 경로. 끄기로 했으면 `None`이다.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Enabled(path) => Some(path.as_path()),
            Self::Disabled => None,
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled(_))
    }
}

/// 모델 디렉터리에서 VAD 모델을 찾는다.
///
/// **실패를 만들지 않는다.** 없는 것도, 읽을 수 없는 것도, 비어 있는 것도 전부
/// [`VadChoice::Disabled`]다 — VAD는 전사의 **선택적 보강**이지 전제가 아니며,
/// 이것 때문에 전사가 실패하면 2026-09-08 이전보다 나빠진다.
///
/// 비어 있는 파일까지 거르는 이유는 [`super::model::resolve`]와 같다: 내려받다 중단된
/// 파일이 있을 수 있고, 그것을 `enable_vad`에 넘기면 whisper.cpp 안쪽에서 실패한다.
pub fn find(models_dir: &Path) -> VadChoice {
    let path = models_dir.join(VAD_MODEL_FILE);

    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() && metadata.len() > 0 => VadChoice::Enabled(path),
        _ => VadChoice::Disabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "molt-note-vad-{}-{}",
                label,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("시계")
                    .as_nanos()
            ));
            fs::create_dir_all(&path).expect("임시 디렉터리");
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

    #[test]
    fn a_model_file_that_is_there_is_used() {
        let dir = TempDir::new("present");
        let path = dir.path().join(VAD_MODEL_FILE);
        fs::write(&path, b"not really a model, but not empty").expect("쓰기");

        let choice = find(dir.path());

        assert!(choice.is_enabled());
        assert_eq!(choice.path(), Some(path.as_path()));
    }

    #[test]
    fn no_model_file_is_a_normal_state_not_a_failure() {
        let dir = TempDir::new("absent");

        // 함수가 Result가 아니라는 것 자체가 이 규칙이다 — 여기서 전사가 멈추지 않는다.
        assert_eq!(find(dir.path()), VadChoice::Disabled);
        assert_eq!(find(dir.path()).path(), None);
    }

    #[test]
    fn an_empty_file_is_not_used() {
        // 내려받다 중단된 파일. `enable_vad`에 넘기면 whisper.cpp 안쪽에서 실패한다.
        let dir = TempDir::new("empty");
        fs::write(dir.path().join(VAD_MODEL_FILE), b"").expect("쓰기");

        assert_eq!(find(dir.path()), VadChoice::Disabled);
    }

    #[test]
    fn a_directory_with_the_model_name_is_not_used() {
        let dir = TempDir::new("directory");
        fs::create_dir_all(dir.path().join(VAD_MODEL_FILE)).expect("디렉터리");

        assert_eq!(find(dir.path()), VadChoice::Disabled);
    }

    #[test]
    fn a_models_directory_that_does_not_exist_is_not_a_failure() {
        let dir = TempDir::new("gone");
        let missing = dir.path().join("이-디렉터리는-없다");

        assert_eq!(find(&missing), VadChoice::Disabled);
    }

    /// 파일 이름은 whisper.cpp가 배포하는 그대로여야 한다 — 이 앱이 지어낸 이름이면
    /// 사용자가 받은 파일과 앱이 찾는 파일이 어긋난다.
    #[test]
    fn the_model_file_name_is_the_one_whisper_cpp_ships() {
        assert_eq!(VAD_MODEL_FILE, "ggml-silero-v5.1.2.bin");
    }
}
