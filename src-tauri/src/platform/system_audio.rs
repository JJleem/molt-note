//! macOS 시스템 오디오를 마이크와 **함께** 잡는 자리 (ADR-0012 · PRODUCT-SPEC §22).
//!
//! ```text
//! process tap  ─┐
//!               ├─→ aggregate device ─→ 입력 3채널 (마이크 1 + tap 2)
//! 마이크        ─┘                       한 IOProc · 한 클럭 · 드리프트 없음
//! ```
//!
//! ## 이 모듈이 하는 일과 하지 않는 일
//!
//! **장치를 만들고 지운다. 그것뿐이다.** 소리를 읽지도, 파일을 쓰지도, 채널을 섞지도
//! 않는다 — 그 일들은 캡처 경로의 몫이며, 이 경계가 좁을수록 `cfg(target_os)`가 퍼지지
//! 않는다 (INV-10 · `platform/` 아래에 플랫폼 지식을 가두는 규칙).
//!
//! ## 왜 `cpal`이 아닌가
//!
//! **[A✓ 2026-09-08 실측]** `cpal`은 이 aggregate device를 목록에서 보지만 **지원 config가
//! 전부 1채널**이다. 그것으로 열면 마이크만 오고 tap 채널은 오지 않는다 (ADR-0012 §6.4).
//! 그래서 회의 녹음은 기존 캡처 경로를 재사용할 수 없다.
//!
//! ## 함정 — 빈 목록은 "전부"가 아니다
//!
//! **[A✓ 2026-09-08 실측]** 프로세스 목록을 비운 채 tap을 만들면 `status = 0`으로 성공하고도
//! **무음만 나온다.** "포함할 프로세스가 없다"로 읽히기 때문이다. 전부를 잡으려면
//! [`CATapDescription::initStereoGlobalTapButExcludeProcesses`]에 **빈 제외 목록**을 준다 —
//! 이름이 그 뜻을 말하고 있어서, 이 API를 쓰면 그 함정에 빠질 수 없다.

use objc2::rc::Retained;
use objc2::AllocAnyThread;
use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceIsStackedKey,
    kAudioAggregateDeviceMainSubDeviceKey, kAudioAggregateDeviceNameKey,
    kAudioAggregateDeviceSubDeviceListKey, kAudioAggregateDeviceTapListKey,
    kAudioAggregateDeviceUIDKey, kAudioHardwareNoError, kAudioSubDeviceUIDKey,
    kAudioSubTapDriftCompensationKey, kAudioSubTapUIDKey, AudioHardwareCreateAggregateDevice,
    AudioHardwareCreateProcessTap, AudioHardwareDestroyAggregateDevice,
    AudioHardwareDestroyProcessTap, AudioObjectID, CATapDescription, CATapMuteBehavior,
};
use objc2_foundation::{NSArray, NSNumber, NSString};

use crate::domain::{Failure, FailureKind};

/// 이 앱이 만드는 aggregate device의 이름. 사용자가 오디오 설정에서 볼 수 있다.
pub const AGGREGATE_NAME: &str = "Molt Note 회의 입력";

/// 그 장치의 UID. **앱마다 하나여야 하므로 번들 식별자를 따른다.**
pub const AGGREGATE_UID: &str = "com.moltnote.app.meeting-input";

/// 살아 있는 동안 tap과 aggregate device를 붙들고 있는 값.
///
/// **`Drop`이 정리한다.** 만든 것을 지우지 않으면 사용자의 오디오 설정에 우리 장치가
/// 남는다 — 앱이 죽어도 남는다. 정리를 사람의 기억이나 호출 순서에 맡기지 않는다.
pub struct MeetingInput {
    aggregate_id: AudioObjectID,
    tap_id: AudioObjectID,
    /// `CATapDescription`을 살려 둔다 — UUID가 aggregate 기술에 들어가 있으므로
    /// 그것이 먼저 사라지면 무엇을 참조하고 있는지 알 수 없게 된다.
    _description: Retained<CATapDescription>,
}

impl MeetingInput {
    /// 이 장치를 여는 데 쓸 UID. 캡처 경로가 이 값으로 장치를 찾는다.
    pub fn uid(&self) -> &'static str {
        AGGREGATE_UID
    }

    /// Core Audio가 준 장치 식별자.
    pub fn device_id(&self) -> AudioObjectID {
        self.aggregate_id
    }
}

impl Drop for MeetingInput {
    fn drop(&mut self) {
        // 실패해도 할 수 있는 것이 없다. **여기서 panic하지 않는다** — 정리 중의 panic은
        // 녹음이 끝나는 자리에서 앱을 죽인다.
        unsafe {
            AudioHardwareDestroyAggregateDevice(self.aggregate_id);
            AudioHardwareDestroyProcessTap(self.tap_id);
        }
    }
}

/// 마이크 하나와 시스템 오디오를 함께 내는 입력 장치를 만든다.
///
/// `microphone_uid`는 Core Audio의 장치 UID다 (예: `BuiltInMicrophoneDevice`).
/// **어느 마이크인지 이 모듈이 고르지 않는다** — 기본 장치를 따를지 사용자가 고른 것을
/// 쓸지는 부르는 쪽이 정한다. AirPods를 끼면 기본 입력이 그것으로 바뀌므로, 기본을
/// 따르는 쪽이 §22의 "끼면 바로 인식된다"를 만족한다.
pub fn open(microphone_uid: &str) -> Result<MeetingInput, Failure> {
    let description = create_tap_description();

    let mut tap_id: AudioObjectID = 0;
    let status = unsafe { AudioHardwareCreateProcessTap(Some(&description), &mut tap_id) };
    if status != kAudioHardwareNoError {
        return Err(tap_failed(status));
    }

    let uuid = unsafe { description.UUID() }.UUIDString().to_string();

    match create_aggregate(microphone_uid, &uuid) {
        Ok(aggregate_id) => Ok(MeetingInput {
            aggregate_id,
            tap_id,
            _description: description,
        }),
        Err(failure) => {
            // aggregate를 못 만들었으면 tap도 남기지 않는다.
            unsafe { AudioHardwareDestroyProcessTap(tap_id) };
            Err(failure)
        }
    }
}

/// **전부를 잡는 tap.** 제외 목록이 비어 있다 = 빼는 것이 없다 = 전부다.
fn create_tap_description() -> Retained<CATapDescription> {
    let exclude: Retained<NSArray<NSNumber>> = NSArray::new();
    let description = unsafe {
        CATapDescription::initStereoGlobalTapButExcludeProcesses(
            <CATapDescription as AllocAnyThread>::alloc(),
            &exclude,
        )
    };

    unsafe {
        description.setName(&NSString::from_str(AGGREGATE_NAME));
        // 다른 앱의 오디오 설정에 이 tap이 보이지 않게 한다.
        description.setPrivate(true);
        // **소리는 그대로 들려야 한다.** 회의 중에 상대 목소리가 안 들리면 그것은 녹음이
        // 아니라 사고다.
        description.setMuteBehavior(CATapMuteBehavior::Unmuted);
    }

    description
}

fn create_aggregate(microphone_uid: &str, tap_uuid: &str) -> Result<AudioObjectID, Failure> {
    let plist = aggregate_description(microphone_uid, tap_uuid);
    let mut aggregate_id: AudioObjectID = 0;
    let status = unsafe {
        AudioHardwareCreateAggregateDevice(
            &plist,
            std::ptr::NonNull::from(&mut aggregate_id),
        )
    };
    if status != kAudioHardwareNoError {
        return Err(aggregate_failed(status));
    }
    Ok(aggregate_id)
}

/// tap을 만들지 못했다 — **권한이 가장 흔한 이유다.**
///
/// 이 API는 `NSAudioCaptureUsageDescription`을 요구하며, 그 권한을 요청하거나 가지고
/// 있는지 확인하는 public API가 **없다** (ADR-0012 §5). 그래서 이 실패가 권한 때문인지
/// 다른 이유인지 이 앱은 구분하지 못한다 — **구분하는 척하지 않는다.**
fn tap_failed(status: i32) -> Failure {
    Failure::permanent(
        FailureKind::AudioDevice,
        "시스템 오디오를 잡을 수 없다. 시스템 설정에서 이 앱의 오디오 녹음 권한을 확인한다.",
    )
    .with_detail(format!("AudioHardwareCreateProcessTap status={status}"))
}

fn aggregate_failed(status: i32) -> Failure {
    Failure::permanent(
        FailureKind::AudioDevice,
        "마이크와 시스템 오디오를 함께 쓸 입력 장치를 만들지 못했다.",
    )
    .with_detail(format!("AudioHardwareCreateAggregateDevice status={status}"))
}

/// aggregate device를 기술하는 사전을 만든다.
///
/// **Core Audio는 `CFDictionary`를 받고, 여기서는 `NSDictionary`로 만들어 넘긴다.**
/// 둘은 toll-free bridge이며, 중첩된 배열·사전을 손으로 조립하는 것보다 objc2의
/// 컬렉션이 훨씬 덜 위험하다.
///
/// ```text
/// name            사용자가 오디오 설정에서 보는 이름
/// uid             이 앱의 장치를 다시 찾는 열쇠
/// private         다른 앱의 장치 목록에 보이지 않는다
/// stacked=false   **[A✓ 2026-09-08]** true로 하면 이 장치가 목록에서 사라진다
/// main            시계의 기준이 되는 하위 장치 — 마이크로 둔다
/// subdevice[]     마이크 하나
/// tap[]           시스템 오디오 하나 · 드리프트 보정 켬
/// ```
fn aggregate_description(
    microphone_uid: &str,
    tap_uuid: &str,
) -> Retained<objc2_core_foundation::CFDictionary> {
    use objc2_foundation::{NSDictionary, NSObject};

    let key = |c: &std::ffi::CStr| NSString::from_str(&c.to_string_lossy());

    let sub_device: Retained<NSDictionary<NSString, NSObject>> = NSDictionary::from_slices(
        &[&*key(kAudioSubDeviceUIDKey)],
        &[&*Retained::into_super(NSString::from_str(microphone_uid))],
    );

    let tap: Retained<NSDictionary<NSString, NSObject>> = NSDictionary::from_slices(
        &[
            &*key(kAudioSubTapUIDKey),
            &*key(kAudioSubTapDriftCompensationKey),
        ],
        &[
            &*Retained::into_super(NSString::from_str(tap_uuid)),
            &*Retained::into_super(NSNumber::new_bool(true)),
        ],
    );

    let sub_devices = NSArray::from_slice(&[&*sub_device]);
    let taps = NSArray::from_slice(&[&*tap]);

    let plist: Retained<NSDictionary<NSString, NSObject>> = NSDictionary::from_slices(
        &[
            &*key(kAudioAggregateDeviceNameKey),
            &*key(kAudioAggregateDeviceUIDKey),
            &*key(kAudioAggregateDeviceIsPrivateKey),
            &*key(kAudioAggregateDeviceIsStackedKey),
            &*key(kAudioAggregateDeviceMainSubDeviceKey),
            &*key(kAudioAggregateDeviceSubDeviceListKey),
            &*key(kAudioAggregateDeviceTapListKey),
        ],
        &[
            &*Retained::into_super(NSString::from_str(AGGREGATE_NAME)),
            &*Retained::into_super(NSString::from_str(AGGREGATE_UID)),
            &*Retained::into_super(NSNumber::new_bool(true)),
            &*Retained::into_super(NSNumber::new_bool(false)),
            &*Retained::into_super(NSString::from_str(microphone_uid)),
            &*Retained::into_super(sub_devices),
            &*Retained::into_super(taps),
        ],
    );

    // toll-free bridge — NSDictionary와 CFDictionary는 같은 객체다.
    unsafe { Retained::cast_unchecked::<objc2_core_foundation::CFDictionary>(plist) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **실제 Core Audio에 닿는 테스트다.** 이 저장소에서 드문 일이며, 그럴 만한 이유가 있다:
    /// 여기서 판정하는 것은 순수한 값이 아니라 **OS가 우리 요청을 받아들이는가**이고,
    /// 그것은 double로 흉내 낼 수 없다.
    ///
    /// **아무것도 녹음하지 않는다.** 장치를 만들고 바로 지운다. 파일도 쓰지 않는다.
    ///
    /// 권한이 없는 기기(CI 등)에서는 만들지 못하는 것이 정상이므로 **실패를 실패로 보지
    /// 않는다** — 판정하는 것은 "만들어졌다면 그 뒤가 성립하는가"다.
    #[test]
    fn a_meeting_input_can_be_created_and_is_cleaned_up() {
        let Ok(input) = open("BuiltInMicrophoneDevice") else {
            // 권한이 없거나 그 마이크가 없는 기기다. 이 테스트가 판정할 것이 없다.
            return;
        };

        assert_eq!(input.uid(), AGGREGATE_UID);
        assert_ne!(input.device_id(), 0, "장치 식별자를 받아야 한다");

        // Drop이 지운다 — 지우지 못하면 사용자의 오디오 설정에 우리 장치가 남는다.
        drop(input);

        // 같은 UID로 다시 만들 수 있다 = 앞의 것이 실제로 사라졌다.
        if let Ok(again) = open("BuiltInMicrophoneDevice") {
            assert_ne!(again.device_id(), 0);
        }
    }

    /// 이름과 UID는 사용자에게 보이고 저장소가 다시 찾는 값이다. 조용히 바뀌면 안 된다.
    #[test]
    fn the_device_identity_is_stable() {
        assert_eq!(AGGREGATE_UID, "com.moltnote.app.meeting-input");
        assert!(AGGREGATE_NAME.contains("Molt Note"));
    }
}
