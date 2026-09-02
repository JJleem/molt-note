//! 오디오 입력 경계 (PRODUCT-SPEC §3.1의 `RecordingBackend` 자리).
//!
//! ADR-0003이 **잠정 선택**한 native 오디오 경로가 여기에 있다. 그 문서는 아직
//! `PROVISIONAL`이며, 사람이 실제 장치에서 확인해야 할 항목이 남아 있다 (ADR-0003 §12).
//! 그러므로 이 모듈은 그 결정을 확정된 것처럼 다루지 않는다 — 확정 전에 필요한 만큼만 있다.
//!
//! 지금 있는 것은 **입력 장치 열거**와 **start / stop 한 쌍의 캡처**다.
//! pause/resume · 재시작 영속성 · 재생 · Recording 레코드 생성은 없다 (Phase 2B).
//!
//! ```text
//! devices.rs         목록을 다듬는 규칙        (하드웨어 없이 테스트된다)
//! capture.rs         파일을 만들고 확정하는 규칙 (하드웨어 없이 테스트된다)
//! system_devices.rs  실제 장치에 묻는 부분      (cpal을 아는 두 자리 중 하나)
//! system_capture.rs  실제 장치를 여는 부분      (cpal을 아는 두 자리 중 하나)
//! ```

pub mod capture;
pub mod devices;
pub mod system_capture;
pub mod system_devices;

pub use capture::{
    ActiveCapture, CaptureFormat, CaptureReport, OpenCapture, SampleSource, CONTAINER,
};
pub use devices::{catalog, InputDevice, InputDeviceSource, ObservedInputDevice};
pub use system_capture::SystemSampleSource;
pub use system_devices::SystemInputDevices;
