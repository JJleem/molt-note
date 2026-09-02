//! 실제 장치에 묻는 유일한 자리 (ADR-0003의 잠정 선택 · `cpal`).
//!
//! **`cpal`을 아는 코드는 이 파일뿐이다.** 나머지 코드는 [`InputDeviceSource`]만 안다.
//! 그래서 이 경로를 지나지 않고도 목록을 만드는 규칙 전부를 검증할 수 있고
//! (`super::devices`의 테스트), engine 선택이 뒤집혀도 바꿀 자리가 여기 하나다 —
//! ADR-0003은 아직 `PROVISIONAL`이며 이 파일은 그 사실 위에 있다.
//!
//! ```text
//! macOS    CoreAudio
//! Windows  WASAPI
//! ```
//!
//! 플랫폼 분기는 `cpal` 안에 있고 이 파일에는 `cfg(target_os)`가 없다 (INV-10).
//!
//! **자동 테스트는 이 파일의 코드를 실행하지 않는다.** 여기서부터는 실제 하드웨어와 OS 권한의
//! 영역이며, 열거가 macOS의 TCC 프롬프트를 유발하는지는 **UNVERIFIED**다 —
//! 사람이 번들된 앱에서 확인한다 (ADR-0003 §12).

use cpal::traits::{DeviceTrait, HostTrait};

use crate::audio::devices::{InputDeviceSource, ObservedInputDevice};
use crate::domain::{Failure, FailureKind};

/// 이 기기의 입력 장치를 `cpal`의 기본 host에 묻는다.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemInputDevices;

impl InputDeviceSource for SystemInputDevices {
    fn observe(&self) -> Result<Vec<ObservedInputDevice>, Failure> {
        let host = cpal::default_host();

        // 기본 장치는 이름으로만 대조한다. 이름을 읽지 못하면 표시하지 않을 뿐,
        // 그 사실이 열거 전체를 실패로 만들지는 않는다.
        let default_name = host.default_input_device().and_then(|device| name_of(&device));
        let mut default_marked = false;

        let devices = host.input_devices().map_err(|error| {
            Failure::retryable(FailureKind::AudioDevice, "입력 장치 목록을 읽지 못했다.")
                .with_detail(error)
        })?;

        Ok(devices
            .map(|device| {
                let name = name_of(&device).unwrap_or_default();
                let is_default = !default_marked && default_name.as_deref() == Some(name.as_str());
                default_marked |= is_default;
                ObservedInputDevice { name, is_default }
            })
            .collect())
    }
}

/// 장치가 알려주는 이름. 읽지 못하면 `None`이다.
///
/// **이름을 읽지 못하는 것은 열거 실패가 아니다.** 이름 없는 장치로 넘기고, 무엇을 보여줄지는
/// 목록을 만드는 쪽이 정한다 ([`super::devices::catalog`]).
///
/// `cpal`은 이름과 별개로 `DeviceId`도 제공하지만 여기서는 쓰지 않는다 — 그 값이 앱 실행
/// 사이에서도 같은지는 **UNVERIFIED**이고, 지금 필요한 것은 한 번의 열거 결과 안에서만
/// 유효한 키이기 때문이다. 장치를 실제로 여는 Task가 필요해지면 그때 다시 본다.
pub(super) fn name_of(device: &cpal::Device) -> Option<String> {
    device
        .description()
        .ok()
        .map(|description| description.name().to_string())
}
