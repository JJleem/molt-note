//! Recording 하나를 로컬 Markdown 파일로 내보내는 command의 소유자
//! (PRODUCT-SPEC §11 · `docs/ADR-0009-notion-and-export.md` §4 · `phase-prompt/05` 요구 A-1~3).
//!
//! ```text
//! export_markdown ─→ Exporter ─→ exports 디렉터리 준비 ─→ export::run::export ─→ 쓰인 경로
//!                       │              (INV-10)              (읽기 + 파일 하나 쓰기)
//!                       └─ 앱 데이터 디렉터리를 얻지 못한 실패도 값으로 들고 있다 (§13)
//! ```
//!
//! ## 시작·상태 조회 규약을 쓰지 않는다
//!
//! 전사와 노트 생성은 오래 걸리므로 배경 스레드에서 돌고 화면이 상태를 물어본다
//! ([`super::Transcriber`] · [`super::NoteGenerator`]). export는 그 규약을 쓰지 **않는다** —
//! 기다릴 모델도 서버도 없고, 하는 일은 저장소 조회 몇 번과 파일 하나 쓰기다. 오래 도는 일에만
//! 쓰는 규약을 짧은 일에 붙이면 화면이 결과를 얻기까지 한 번 더 물어봐야 한다.
//!
//! 대신 command는 `async`다 — Tauri는 `async`가 아닌 command를 main thread에서 실행하므로,
//! 디스크가 느릴 때 그 시간만큼 창이 멈추는 것을 피한다 ([`super::ai_provider_status`]와 같은
//! 이유다).
//!
//! ## AI가 없어도 내보낸다 (INV-8)
//!
//! 이 경계에는 provider도, AI 설정도, `ai::provider`에 대한 참조도 없다. **AI Note가 하나도
//! 없다는 이유로 거절할 수단 자체가 없다** — 노트는 [`crate::export::run`]이 있으면 넣고 없으면
//! 넣지 않는 선택 입력이다.
//!
//! ## 무엇도 지우거나 고치지 않는다 (INV-3 · INV-6)
//!
//! export는 **읽고 새 파일 하나를 더하는 일**이다. 이 모듈에도, 그 아래 실행 순서에도 저장소에
//! 쓰는 코드가 없고, 오디오 파일은 복사하지도 읽지도 않는다. 실패해도 마찬가지다 — 실패는
//! §13의 세 질문에 답하는 [`Failure`] 하나로 돌아갈 뿐, 원본에 흔적을 남기지 않는다.

use tauri::Manager;

use crate::db;
use crate::domain::{Failure, FailureKind, RecordingId};
use crate::export;
use crate::platform::app_data_dir::AppDataDirectory;

use super::payload::ExportedFilePayload;

/// 앱이 들고 있는 **Markdown export 실행자**.
///
/// [`super::Storage`]처럼 **앱 데이터 디렉터리를 얻지 못한 실패도 값으로 들고 있다.** 그 실패는
/// 내보내려 할 때 사용자에게 그대로 전달된다 — 앱 시작을 막지 않는다 (§13).
///
/// 저장소 연결은 들고 있지 않다. 내보낼 때마다 새로 열며 ([`Self::export`]), 그래서 export가
/// 도는 동안 목록 조회 같은 다른 command가 앱의 연결을 기다리며 멈추지 않는다 (노트 생성이
/// 연결을 새로 여는 것과 같은 이유다).
pub struct Exporter {
    app_data_dir: Result<AppDataDirectory, Failure>,
}

impl Exporter {
    /// Tauri가 결정한 앱 데이터 디렉터리 아래로 내보낸다 (INV-10).
    pub fn open_for<R, M>(manager: &M) -> Self
    where
        R: tauri::Runtime,
        M: Manager<R>,
    {
        Self {
            app_data_dir: AppDataDirectory::from_manager(manager).map_err(Into::into),
        }
    }

    /// 주어진 디렉터리 아래로 내보낸다.
    ///
    /// **자동 검증이 지나는 자리다** — 임시 루트를 주면 제품 경로 그대로 돌면서도 사용자의 실제
    /// export 디렉터리를 건드리지 않는다 (§18).
    pub fn in_directory(app_data_dir: AppDataDirectory) -> Self {
        Self {
            app_data_dir: Ok(app_data_dir),
        }
    }

    /// Recording 하나를 Markdown 파일로 내보내고 **쓰인 파일**을 돌려준다.
    ///
    /// 순서는 셋이다 — 쓸 자리를 준비하고, 저장소를 열고, 읽어서 파일 하나를 쓴다. 자리를 먼저
    /// 준비하는 이유는 [`Recorder::start`](super::Recorder::start)가 장치를 열기 전에 녹음
    /// 디렉터리를 준비하는 것과 같다: 쓸 자리가 없다는 사실은 무엇을 읽기 전에 알 수 있고,
    /// 그 실패에는 사용자가 할 수 있는 일이 따로 있다 (§13).
    ///
    /// 같은 이름의 파일이 이미 있으면 **덮어쓰지 않고 번호를 붙인다** (ADR-0009 §4.3). 그래서
    /// 돌려주는 값에는 실제로 쓰인 이름이 함께 들어 있다 — 화면이 이름을 다시 짐작하지 않는다.
    pub fn export(&self, recording_id: &str) -> Result<ExportedFilePayload, Failure> {
        let app_data_dir = self.app_data_dir.as_ref().map_err(Clone::clone)?;

        let recording_id = recording_id.trim();
        if recording_id.is_empty() {
            return Err(Failure::permanent(
                FailureKind::InvalidInput,
                "내보낼 녹음을 고르지 않았다.",
            ));
        }

        let directory = app_data_dir.ensure_exports_dir()?;
        let connection = db::open_in(app_data_dir)?;
        let written = export::run::export(&connection, &directory, &RecordingId::new(recording_id))?;

        Ok(ExportedFilePayload::new(recording_id, written))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_that_could_not_be_resolved_is_reported_instead_of_panicking() {
        // 앱 시작 시점의 실패가 여기까지 값으로 실려 온다 (§13). 그 상태에서도 앱은 떠 있다.
        let exporter = Exporter {
            app_data_dir: Err(Failure::retryable(
                FailureKind::Storage,
                "앱 데이터 디렉터리 경로를 결정하지 못했다",
            )),
        };

        let failure = exporter.export("rec-1").expect_err("자리를 모르면 쓸 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (INV-3)");
    }

    #[test]
    fn asking_to_export_nothing_is_refused_before_the_storage_is_opened() {
        let temp = std::env::temp_dir().join(format!(
            "molt-note-exporter-empty-id-{}",
            std::process::id()
        ));
        let exporter = Exporter::in_directory(AppDataDirectory::new(&temp));

        let failure = exporter.export("   ").expect_err("고른 녹음이 없다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.retryable, "같은 값을 다시 보내도 같다");
        assert!(failure.source_data_safe);
        assert!(
            !temp.exists(),
            "고르지 않은 요청 때문에 디렉터리를 만들지 않는다"
        );
    }
}
