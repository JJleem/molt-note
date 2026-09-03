//! 오디오 입력 경계 (PRODUCT-SPEC §3.1의 `RecordingBackend` 자리).
//!
//! ADR-0003이 **잠정 선택**한 native 오디오 경로가 여기에 있다. 그 문서는 아직
//! `PROVISIONAL`이며, 사람이 실제 장치에서 확인해야 할 항목이 남아 있다 (ADR-0003 §12).
//! 그러므로 이 모듈은 그 결정을 확정된 것처럼 다루지 않는다 — 확정 전에 필요한 만큼만 있다.
//!
//! 지금 있는 것은 **입력 장치 열거**와 **start · pause · resume · stop 네 가지 캡처 동작**,
//! 녹음 session의 상태 기계, 그리고 **확정된 파일의 확인**이다. [`session`]의 전이 규칙과
//! [`capture`]의 실제 파일 쓰기를 엮고 확인을 마친 녹음을 레코드로 남기는 자리는
//! [`crate::commands`]이며, 진행 중인 session은 거기(Tauri managed state)가 소유한다 —
//! 화면이 아니다 (R-001 · `docs/ADR-0004-recording-session-lifecycle.md`).
//! 재시작 영속성과 재생은 아직 없다.
//!
//! ```text
//! devices.rs         목록을 다듬는 규칙        (하드웨어 없이 테스트된다)
//! capture.rs         파일을 만들고 확정하는 규칙 (하드웨어 없이 테스트된다)
//! finalized.rs       확정된 파일을 확인하는 규칙 (하드웨어 없이 테스트된다)
//! session.rs         녹음 session의 상태 기계   (하드웨어도 시계도 없이 테스트된다)
//! system_devices.rs  실제 장치에 묻는 부분      (cpal을 아는 두 자리 중 하나)
//! system_capture.rs  실제 장치를 여는 부분      (cpal을 아는 두 자리 중 하나)
//! ```

pub mod capture;
pub mod devices;
pub mod finalized;
pub mod session;
pub mod system_capture;
pub mod system_devices;

pub use capture::{
    ActiveCapture, CaptureFormat, CaptureReport, OpenCapture, SampleSink, SampleSource, SinkError,
    CONTAINER,
};
pub use finalized::{audio_is_present, VerifiedAudio, MIN_FINALIZED_BYTES};
pub use devices::{catalog, InputDevice, InputDeviceSource, ObservedInputDevice};
pub use session::{RecordingSession, SessionState, SessionSummary};
pub use system_capture::SystemSampleSource;
pub use system_devices::SystemInputDevices;
