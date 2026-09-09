//! 플랫폼 지식이 갇혀 있는 경계 (PRODUCT-SPEC §3.1 · INV-10).
//!
//! OS별 경로 규약이나 `cfg(target_os)` 분기는 이 모듈 안에만 존재한다.
//! 나머지 코드는 여기서 노출되는 개념(예: 앱 데이터 디렉터리 · 흐르는 시간 · 마이크 접근
//! 권한)만 안다.
//!
//! 여기 있는 것은 전부 **바깥 세계를 읽는 자리**다. 그래서 그 바깥을 값으로 바꿔 넣으면
//! 나머지 코드가 하드웨어도 시계도 실제 디렉터리도 실제 권한도 없이 검증된다 (§18).
//!
//! ```text
//! app_data_dir.rs   앱 데이터가 놓이는 자리   (Tauri가 플랫폼 차이를 안다)
//! clock.rs          흐르는 시간
//! file_manager.rs   그 자리를 여는 수단        (OS 파일 관리자를 아는 유일한 자리)
//! microphone.rs     마이크 접근 권한          (macOS 설정 경로를 아는 유일한 자리)
//! secret_store.rs   secret이 놓이는 자리      (OS 자격증명 저장소를 아는 유일한 자리)
//! ```
//!
//! `app_data_dir.rs`와 `file_manager.rs`는 한 쌍이다 — 앞의 것이 **파일이 어디에 놓이는가**를
//! 답하고, 뒤의 것이 **그 자리를 사람이 어떻게 여는가**를 답한다. 경로를 글자로 보여 주는
//! 것만으로는 부족하다는 것이 2026-09-05의 실사용에서 드러났기 때문이다
//! (`phase-prompt/05.6` R-4).

pub mod app_data_dir;
pub mod clock;
pub mod command_runner;
pub mod file_manager;
pub mod microphone;
pub mod secret_store;
#[cfg(target_os = "macos")]
pub mod system_audio;
