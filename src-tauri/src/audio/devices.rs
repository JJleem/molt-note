//! 입력 장치 열거 (PRODUCT-SPEC §6.1의 `microphone enumeration` · §18).
//!
//! 이 모듈은 둘로 갈라져 있고, **그 경계가 이 파일의 목적이다.**
//!
//! ```text
//! InputDeviceSource   실제 장치가 있어야 답할 수 있는 부분
//! catalog()           값만 있으면 답할 수 있는 부분
//! ```
//!
//! 목록을 화면이 쓸 수 있는 모양으로 만드는 규칙(빈 이름 · 중복 이름 · 선택 키 · 정렬)은
//! 전부 아래쪽에 있다. 그래서 **마이크도 마이크 권한도 없는 환경에서 그대로 검증된다** —
//! Gate가 도는 환경에 장치가 없을 수 있고, 그때 빨개지는 것은 제품 결함이 아니다 (§18).
//!
//! **장치가 하나도 없는 것은 오류가 아니다.** 마이크를 뽑아 둔 상태는 정상 상태이며,
//! 빈 목록으로 그대로 전달된다 — 저장된 녹음이 하나도 없는 것을 오류로 보지 않는 것과 같다.

use std::collections::HashMap;

use crate::domain::Failure;

/// 이름을 읽을 수 없는 장치에 붙이는 표시용 이름.
///
/// 이름이 없다는 사실을 숨기지 않는다. 빈 문자열을 그대로 화면에 보내면 사용자에게는
/// **고를 수 없는 항목**이 되기 때문이다.
pub const UNNAMED_INPUT_DEVICE_LABEL: &str = "이름 없는 입력 장치";

/// 열거된 장치 하나에 대해 **하드웨어 쪽이 알려준 것 그대로**.
///
/// 다듬지 않은 값이다 — 이름이 비어 있을 수도, 다른 장치와 같을 수도 있다.
/// 그 처리는 [`catalog`]가 한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedInputDevice {
    /// 장치가 알려준 이름. 읽지 못했으면 빈 문자열이다.
    pub name: String,
    /// 시스템 기본 입력 장치인가.
    pub is_default: bool,
}

impl ObservedInputDevice {
    /// 기본 장치가 아닌 장치 하나.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_default: false,
        }
    }

    /// 시스템 기본 입력 장치로 표시된 장치 하나.
    pub fn default_named(name: impl Into<String>) -> Self {
        Self {
            is_default: true,
            ..Self::named(name)
        }
    }
}

/// 화면이 그대로 그릴 수 있는 입력 장치 하나.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputDevice {
    /// 장치를 고를 때 쓰는 키. **같은 열거 결과면 언제나 같은 값이다.**
    ///
    /// 불투명한 값이며 화면에 보여주는 용도가 아니다. 이름이 같은 장치가 둘 있어도
    /// 서로 다른 키를 받는다 — 이름만으로는 둘을 구분할 수 없기 때문이다.
    ///
    /// **한 번의 열거 결과 안에서 유효한 키다.** 장치를 꽂거나 뽑은 뒤의 목록에서도 같은
    /// 물리 장치가 같은 키를 받는지는 확인되지 않았다 (UNVERIFIED).
    pub key: String,
    /// 사람이 읽는 이름. 비어 있지 않다.
    pub label: String,
    /// 시스템 기본 입력 장치인가.
    pub is_default: bool,
}

/// 장치 목록을 하드웨어에 묻는 경계.
///
/// 실제 구현은 [`crate::audio::system_devices`] 하나뿐이고, 테스트는 자신의 구현을 넣는다 —
/// **자동 테스트가 실제 마이크나 마이크 권한의 존재를 전제하지 않는 이유가 이 trait이다** (§18).
pub trait InputDeviceSource {
    /// 열거된 입력 장치를 **열거 순서 그대로** 돌려준다.
    ///
    /// 하나도 없으면 빈 목록이다 (오류가 아니다). 목록 자체를 얻지 못한 것만 실패다.
    fn observe(&self) -> Result<Vec<ObservedInputDevice>, Failure>;
}

/// 관찰된 장치 목록을 화면이 쓸 수 있는 목록으로 만든다.
///
/// 하는 일은 셋이다 — **선택 키를 붙이고 · 보여줄 이름을 정하고 · 표시 순서로 정렬한다.**
/// 세 가지 모두 입력만으로 결정되므로 같은 입력이면 언제나 같은 결과가 나온다.
pub fn catalog(observed: impl IntoIterator<Item = ObservedInputDevice>) -> Vec<InputDevice> {
    let observed: Vec<ObservedInputDevice> = observed.into_iter().collect();
    let names: Vec<String> = observed
        .iter()
        .map(|observation| observation.name.trim().to_string())
        .collect();

    let mut devices: Vec<InputDevice> = ordinals(&names)
        .into_iter()
        .zip(&names)
        .zip(&observed)
        .map(|((ordinal, name), observation)| InputDevice {
            key: selection_key(name, ordinal),
            label: label_for(name, ordinal),
            is_default: observation.is_default,
        })
        .collect();

    // 표시 순서: 기본 장치가 먼저, 그다음은 이름 순이다. 열거 순서는 플랫폼이 정하고
    // 호출마다 같다는 보장이 없으므로, 화면에 보이는 순서는 여기서 결정한다.
    devices.sort_by(|left, right| {
        right
            .is_default
            .cmp(&left.is_default)
            .then_with(|| left.label.cmp(&right.label))
            .then_with(|| left.key.cmp(&right.key))
    });
    devices
}

/// 열거 순서 그대로의 선택 키. **[`catalog`]가 붙이는 키와 같은 규칙이다.**
///
/// 고른 장치를 다시 찾아야 하는 쪽(실제 장치를 여는 경계)이 이 함수를 쓴다.
/// 키를 만드는 규칙이 두 곳에 생기면 **사용자가 고른 장치와 실제로 열리는 장치가
/// 조용히 어긋난다** — 그래서 규칙은 이 파일 하나에만 있다.
///
/// 이름은 열거 순서 그대로 주어야 한다. 정렬된 목록을 주면 다른 키가 나온다.
pub fn selection_keys<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let names: Vec<String> = names.into_iter().map(|name| name.trim().to_string()).collect();
    ordinals(&names)
        .into_iter()
        .zip(&names)
        .map(|(ordinal, name)| selection_key(name, ordinal))
        .collect()
}

/// 각 이름이 **같은 이름 중 몇 번째로 나왔는가.** 열거 순서 그대로다.
///
/// 키와 표시 이름이 모두 이 값으로 갈린다.
fn ordinals(names: &[String]) -> Vec<usize> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    names
        .iter()
        .map(|name| {
            let ordinal = seen.entry(name.as_str()).or_insert(0);
            let current = *ordinal;
            *ordinal += 1;
            current
        })
        .collect()
}

/// 선택 키. 순번을 앞에 두므로 이름이 무엇이든 두 장치가 같은 키를 받지 않는다.
fn selection_key(name: &str, ordinal: usize) -> String {
    format!("{ordinal}:{name}")
}

/// 보여줄 이름. 이름이 없으면 대체 이름을 쓰고, 같은 이름이 반복되면 몇 번째인지 붙인다.
fn label_for(name: &str, ordinal: usize) -> String {
    let base = if name.is_empty() {
        UNNAMED_INPUT_DEVICE_LABEL
    } else {
        name
    };
    if ordinal == 0 {
        base.to_string()
    } else {
        format!("{base} ({})", ordinal + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트가 쓰는 장치 목록. 실제 하드웨어를 부르는 코드는 이 파일에 없다.
    fn catalog_of(names: &[&str]) -> Vec<InputDevice> {
        catalog(names.iter().map(|name| ObservedInputDevice::named(*name)))
    }

    #[test]
    fn an_empty_list_is_a_normal_answer_not_a_failure() {
        // 마이크를 뽑아 둔 상태는 정상 상태다. 빈 목록이 그대로 나온다.
        assert_eq!(catalog(Vec::new()), Vec::new());
    }

    #[test]
    fn devices_with_the_same_name_are_told_apart() {
        // 같은 모델의 USB 마이크 두 개가 같은 이름을 낼 수 있다.
        let devices = catalog_of(&["USB Microphone", "USB Microphone"]);

        assert_eq!(devices.len(), 2);
        assert_ne!(devices[0].key, devices[1].key, "선택 키가 겹치면 안 된다");
        assert_ne!(
            devices[0].label, devices[1].label,
            "사용자가 둘을 구분할 수 있어야 한다"
        );
        assert_eq!(devices[0].label, "USB Microphone");
        assert_eq!(devices[1].label, "USB Microphone (2)");
    }

    #[test]
    fn a_device_without_a_name_still_gets_a_label_and_a_key() {
        let devices = catalog_of(&["", "   "]);

        assert_eq!(devices[0].label, UNNAMED_INPUT_DEVICE_LABEL);
        assert_eq!(
            devices[1].label,
            format!("{UNNAMED_INPUT_DEVICE_LABEL} (2)"),
            "이름이 없는 장치도 둘 이상일 수 있다"
        );
        for device in &devices {
            assert!(!device.label.trim().is_empty(), "고를 수 없는 항목을 만들지 않는다");
            assert!(!device.key.is_empty());
        }
        assert_ne!(devices[0].key, devices[1].key);
    }

    #[test]
    fn surrounding_whitespace_does_not_create_a_second_device_name() {
        let devices = catalog_of(&["MacBook Pro Microphone", "  MacBook Pro Microphone  "]);

        assert_eq!(devices[0].label, "MacBook Pro Microphone");
        assert_eq!(devices[1].label, "MacBook Pro Microphone (2)");
    }

    #[test]
    fn the_same_observation_always_produces_the_same_keys() {
        // 화면이 고른 키를 다음 호출에서도 찾을 수 있어야 한다.
        let names = ["Studio Mic", "USB Microphone", "USB Microphone", ""];

        let first = catalog_of(&names);
        let second = catalog_of(&names);

        assert_eq!(first, second);
        let keys: Vec<&str> = first.iter().map(|device| device.key.as_str()).collect();
        assert_eq!(keys.len(), 4);
        let mut unique = keys.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), keys.len(), "키는 목록 안에서 유일하다");
    }

    #[test]
    fn a_key_belongs_to_the_name_it_was_made_from() {
        // 이름이 달라지면 키도 달라진다 — 다른 장치를 같은 키로 부르지 않는다.
        let one = catalog_of(&["Studio Mic"]);
        let other = catalog_of(&["Desk Mic"]);

        assert_ne!(one[0].key, other[0].key);
    }

    #[test]
    fn the_default_device_is_marked_and_shown_first() {
        let devices = catalog([
            ObservedInputDevice::named("USB Microphone"),
            ObservedInputDevice::default_named("MacBook Pro Microphone"),
            ObservedInputDevice::named("Aggregate Device"),
        ]);

        assert_eq!(devices[0].label, "MacBook Pro Microphone");
        assert!(devices[0].is_default);
        assert_eq!(
            devices.iter().filter(|device| device.is_default).count(),
            1,
            "관찰된 기본 장치 표시를 늘리거나 지어내지 않는다"
        );
    }

    #[test]
    fn a_list_without_a_default_device_is_still_a_list() {
        // 기본 장치가 없다고 알려 올 수도 있다. 그때 아무 장치나 기본으로 만들지 않는다.
        let devices = catalog_of(&["Desk Mic", "Studio Mic"]);

        assert!(devices.iter().all(|device| !device.is_default));
        assert_eq!(devices.len(), 2);
    }

    #[test]
    fn the_keys_used_to_reopen_a_device_are_the_keys_the_list_handed_out() {
        // 두 규칙이 갈라지면 사용자가 고른 장치와 실제로 열리는 장치가 어긋난다.
        let names = ["USB Microphone", "", "USB Microphone", "  Studio Mic  "];

        let keys = selection_keys(names);
        let listed = catalog(names.iter().map(|name| ObservedInputDevice::named(*name)));

        let mut from_catalog: Vec<String> = listed.iter().map(|device| device.key.clone()).collect();
        let mut from_selection = keys.clone();
        from_catalog.sort();
        from_selection.sort();
        assert_eq!(from_catalog, from_selection);

        // 열거 순서 그대로여야 한다 — n번째 이름의 키가 n번째 자리에 있다.
        assert_eq!(keys[0], "0:USB Microphone");
        assert_eq!(keys[2], "1:USB Microphone");
        assert_eq!(keys[3], "0:Studio Mic", "이름 주변 공백은 키를 바꾸지 않는다");
    }

    #[test]
    fn the_display_order_does_not_depend_on_the_enumeration_order() {
        let ascending = catalog_of(&["Aggregate Device", "Desk Mic", "Studio Mic"]);
        let descending = catalog_of(&["Studio Mic", "Desk Mic", "Aggregate Device"]);

        let labels = |devices: &[InputDevice]| -> Vec<String> {
            devices.iter().map(|device| device.label.clone()).collect()
        };
        assert_eq!(labels(&ascending), labels(&descending));
        assert_eq!(
            labels(&ascending),
            ["Aggregate Device", "Desk Mic", "Studio Mic"]
        );
    }
}
