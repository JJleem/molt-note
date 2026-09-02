//! 입력 장치 열거가 command 경계까지 도달하는지 본다 (ADR-0003 · PRODUCT-SPEC §6.1 · §13).
//!
//! **이 파일은 실제 마이크도 마이크 권한도 요구하지 않는다.** 하드웨어에 묻는 부분이
//! [`InputDeviceSource`] 뒤에 있으므로, 여기서는 그 자리에 값을 돌려주는 구현을 넣는다 (§18).
//! 실제 장치에서만 알 수 있는 것(권한 프롬프트 · 어떤 장치가 실제로 열리는가)은
//! 사람이 확인하며, 이 테스트가 그것을 대신 판정하지 않는다 (ADR-0003 §12).
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use molt_note_lib::audio::{InputDeviceSource, ObservedInputDevice};
use molt_note_lib::commands::AudioDevices;
use molt_note_lib::domain::{Failure, FailureKind};

/// 정해진 목록을 그대로 돌려주는 경계 구현.
struct FixedDevices(Vec<ObservedInputDevice>);

impl InputDeviceSource for FixedDevices {
    fn observe(&self) -> Result<Vec<ObservedInputDevice>, Failure> {
        Ok(self.0.clone())
    }
}

/// 목록 자체를 얻지 못하는 경계 구현. 실제 원인은 장치 상태나 권한이다.
struct UnavailableDevices;

impl InputDeviceSource for UnavailableDevices {
    fn observe(&self) -> Result<Vec<ObservedInputDevice>, Failure> {
        Err(
            Failure::retryable(FailureKind::AudioDevice, "입력 장치 목록을 읽지 못했다.")
                .with_detail("host unavailable"),
        )
    }
}

#[test]
fn a_machine_without_any_input_device_gets_an_empty_list_not_a_failure() {
    // Gate가 도는 환경에 마이크가 없을 수 있다. 그것은 제품 결함이 아니다.
    let devices = AudioDevices::with_source(FixedDevices(Vec::new()));

    assert!(devices
        .list()
        .expect("장치가 없는 것은 오류가 아니다")
        .is_empty());
}

#[test]
fn the_listed_devices_carry_a_key_a_label_and_the_default_marking() {
    let devices = AudioDevices::with_source(FixedDevices(vec![
        ObservedInputDevice::named("USB Microphone"),
        ObservedInputDevice::default_named("MacBook Pro Microphone"),
        ObservedInputDevice::named("USB Microphone"),
    ]));

    let listed = devices.list().expect("목록을 얻을 수 있어야 한다");

    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].label, "MacBook Pro Microphone", "기본 장치가 먼저다");
    assert!(listed[0].is_default);
    assert!(listed.iter().all(|device| !device.key.is_empty()));
    assert!(listed.iter().all(|device| !device.label.trim().is_empty()));

    let mut keys: Vec<&str> = listed.iter().map(|device| device.key.as_str()).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), listed.len(), "이름이 같아도 키는 겹치지 않는다");
}

#[test]
fn asking_twice_gives_the_same_keys_back() {
    // 화면이 고른 키가 다음 조회에서 사라지면 선택을 이어갈 수 없다.
    let observed = vec![
        ObservedInputDevice::default_named("MacBook Pro Microphone"),
        ObservedInputDevice::named(""),
    ];
    let devices = AudioDevices::with_source(FixedDevices(observed));

    assert_eq!(
        devices.list().expect("첫 조회"),
        devices.list().expect("두 번째 조회")
    );
}

#[test]
fn a_device_without_a_name_is_still_listed_with_something_to_choose() {
    let devices = AudioDevices::with_source(FixedDevices(vec![ObservedInputDevice::named("")]));

    let listed = devices.list().expect("이름이 없어도 목록에 남는다");

    assert_eq!(listed.len(), 1);
    assert!(!listed[0].label.trim().is_empty());
}

#[test]
fn a_failure_to_enumerate_reaches_the_user_as_the_shared_failure_contract() {
    let devices = AudioDevices::with_source(UnavailableDevices);

    let failure = devices.list().expect_err("목록을 얻지 못한 것은 실패다");

    assert_eq!(failure.kind, FailureKind::AudioDevice, "저장소 실패와 구분된다");
    assert!(!failure.message.is_empty(), "보여줄 문장이 있어야 한다");
    assert!(failure.source_data_safe, "장치를 읽는 것은 저장된 데이터를 건드리지 않는다");
    assert!(failure.retryable, "장치가 다시 연결되면 성공할 수 있다");
    assert_eq!(failure.detail.as_deref(), Some("host unavailable"));
}

#[test]
fn the_failure_serializes_into_the_shape_the_frontend_reads() {
    let devices = AudioDevices::with_source(UnavailableDevices);
    let failure = devices.list().expect_err("실패해야 한다");

    let json = serde_json::to_value(&failure).expect("직렬화할 수 있어야 한다");

    // `src/ipc/failure.ts`의 FailureKind union에 같은 문자열이 있어야 한다.
    assert_eq!(json["kind"], "audioDevice");
    assert_eq!(json["retryable"], true);
}
