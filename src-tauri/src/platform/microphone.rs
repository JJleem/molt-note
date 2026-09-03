//! 마이크 접근 권한 경계 (PRODUCT-SPEC §3.1의 `PlatformPermissions` 자리 · §13 · INV-10).
//!
//! **"어디서 무엇을 켜야 하는가"를 아는 유일한 자리다.** domain은 마이크에 접근할 수 없다는
//! 사실만 알고([`FailureKind::MicrophonePermission`]), 시스템 설정의 경로도 macOS라는 단어도
//! 알지 않는다. 그 문장은 전부 이 파일 안에서 만들어진다.
//!
//! ```text
//! MicrophoneAccess       허용 · 거부 · 아직 결정되지 않음   (값이다. OS를 모른다)
//! MicrophonePermission   그 값을 어디서 얻는가              (테스트가 대체할 수 있는 자리)
//! access_denied()        거부를 사용자가 읽을 문장으로      (macOS 경로를 아는 곳)
//! ```
//!
//! 경계로 만든 이유는 [`crate::platform::clock`]과 같다. 권한 판정이 코드 안에 박혀 있으면
//! "권한이 거부된 상태에서 녹음을 시작하지 않는다"를 확인하는 방법이 **실제로 마이크 권한을
//! 꺼 보는 것뿐**이 된다. 이 trait 자리에 값을 넣으면 같은 확인이 마이크 없이 끝난다 (§18).
//!
//! ## 이 파일이 확인한 것과 확인하지 못한 것
//!
//! `docs/ADR-0005-microphone-permission.md`에 VERIFIED / UNVERIFIED로 나누어 적었다.
//! 요약하면 이렇다 — **이 Run에서 신뢰할 수 있는 TCC 판정 수단을 확인하지 못했다.** 그래서
//! [`SystemMicrophonePermission`]은 macOS에서 상태를 안다고 주장하지 않고
//! [`MicrophoneAccess::Undetermined`]를 돌려주며, 장치를 열지 못한 경우를 권한 문제로
//! **분류해 안내한다** ([`explain_open_failure`]). 그 분류가 항상 옳다는 보증은 없으며,
//! 그것이 이 경계의 알려진 한계다.
//!
//! Windows의 마이크 privacy 토글 동작은 여기서 다루지 않는다 (PRODUCT-SPEC §14.3 · Phase 6).

use crate::domain::{Failure, FailureKind};

/// 마이크 접근이 지금 어떤 상태인가.
///
/// 세 가지뿐이다. **"모른다"를 "허용"으로도 "거부"로도 접지 않는다** — 그 둘은 사용자가 할 일이
/// 서로 다르고, 모르는 상태에서 둘 중 하나를 고르면 앱이 사용자에게 거짓말을 하게 된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicrophoneAccess {
    /// 접근이 허용돼 있다. 녹음을 시작할 수 있다.
    Granted,
    /// 접근이 거부돼 있다. **이 상태에서는 캡처를 시작하지 않는다.**
    Denied,
    /// 아직 결정되지 않았다 — 사용자가 아직 답하지 않았거나, 우리가 판정할 수단이 없다.
    Undetermined,
}

/// 마이크 접근 권한을 묻고 요청하는 자리.
///
/// 앱은 [`SystemMicrophonePermission`]을 쓰고, 테스트는 자신의 구현을 넣어 세 상태를 전부
/// 마이크 없이 지난다.
pub trait MicrophonePermission: Send + Sync {
    /// 지금 상태를 묻는다. **사용자에게 아무것도 띄우지 않는다.**
    fn status(&self) -> MicrophoneAccess;

    /// 접근을 요청한다. 아직 결정되지 않은 상태라면 OS가 사용자에게 물을 수 있다.
    ///
    /// 돌려주는 값은 요청 **뒤**의 상태다.
    fn request(&self) -> MicrophoneAccess;
}

/// 이 기기의 실제 권한 상태를 묻는 구현.
///
/// **지금은 상태를 안다고 주장하지 않는다.** macOS에서 권한을 직접 조회하려면 TCC
/// (`AVCaptureDevice.authorizationStatus(for:)`)를 부를 수 있어야 하는데, 이 Run에서는 그
/// 호출을 제공하는 crate의 현재 버전과 지원 범위를 확인하지 못했다. 확인하지 못한 것을 확인한
/// 것처럼 다루지 않기 위해 [`MicrophoneAccess::Undetermined`]를 돌려준다
/// (`docs/ADR-0005-microphone-permission.md` §4).
///
/// macOS가 아닌 곳에서는 이 경계가 **아무 주장도 하지 않는다** — 권한을 이유로 녹음을 막지
/// 않는다. Windows의 권한 동작은 Phase 6이 다룬다 (PRODUCT-SPEC §14.3).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemMicrophonePermission;

impl MicrophonePermission for SystemMicrophonePermission {
    fn status(&self) -> MicrophoneAccess {
        if cfg!(target_os = "macos") {
            MicrophoneAccess::Undetermined
        } else {
            MicrophoneAccess::Granted
        }
    }

    fn request(&self) -> MicrophoneAccess {
        // 요청을 따로 보내지 않는다. macOS에서 TCC 프롬프트를 실제로 띄우는 것은 장치를 여는
        // 순간이며(UNVERIFIED · ADR-0005 §4), 그 자리는 `crate::audio::system_capture`다.
        self.status()
    }
}

/// 접근이 거부된 상태를 **사용자가 읽을 수 있는 실패**로 옮긴다 (§13).
///
/// 다시 시도할 수 없는 실패다 — 같은 동작을 그대로 다시 눌러도 결과가 같고, 사용자가 시스템
/// 설정에서 접근을 허용해야 풀린다.
pub fn access_denied() -> Failure {
    Failure::permanent(FailureKind::MicrophonePermission, DENIED_MESSAGE)
}

/// 장치를 열지 못한 실패를, **권한을 확정할 수 없었다면** 권한 문제로 분류해 안내한다.
///
/// 신뢰할 수 있는 판정 수단이 없는 동안의 폴백이며 그 한계는 UNVERIFIED다
/// (`docs/ADR-0005-microphone-permission.md` §4·§6). 두 가지를 지킨다.
///
/// - 권한이 **허용된 것으로 확인된** 상태에서 난 실패는 건드리지 않는다. 그때 장치를 열지
///   못한 것은 권한 문제가 아니다.
/// - 장치를 여는 것과 무관한 실패(예: 파일을 만들지 못했다)도 건드리지 않는다.
///   그래서 이 분류는 **녹음 초기화 실패를 권한 실패로 바꿔 놓지 않는다.**
///
/// 원래 실패의 기술적 원인은 [`Failure::detail`]에 그대로 남는다 — 사용자가 실제 원인을
/// 옮겨 적을 수 있어야 하기 때문이다.
pub fn explain_open_failure(access: MicrophoneAccess, failure: Failure) -> Failure {
    if access != MicrophoneAccess::Undetermined || failure.kind != FailureKind::AudioDevice {
        return failure;
    }

    let detail = failure
        .detail
        .clone()
        .map(|detail| format!("{} ({detail})", failure.message))
        .unwrap_or(failure.message);

    Failure::retryable(FailureKind::MicrophonePermission, UNDETERMINED_MESSAGE).with_detail(detail)
}

/// 접근이 거부돼 있을 때 화면에 그대로 띄우는 문장.
///
/// **경로를 문장 안에 적는다.** "권한이 없다"만으로는 사용자가 어디를 열어야 하는지 모른다.
const DENIED_MESSAGE: &str = if cfg!(target_os = "macos") {
    "마이크에 접근할 수 없다. 시스템 설정 › 개인정보 보호 및 보안 › 마이크에서 Molt Note의 접근을 허용한 뒤 다시 녹음을 시작해야 한다."
} else {
    "마이크에 접근할 수 없다. 시스템의 마이크 개인정보 설정에서 Molt Note의 접근을 허용한 뒤 다시 녹음을 시작해야 한다."
};

/// 권한을 확정하지 못한 채 장치를 열지 못했을 때의 문장.
///
/// **단정하지 않는다.** 이 경로는 권한 문제인지 장치 문제인지 구분하지 못하므로, 문장도
/// 구분한 척하지 않는다 — 그러면서도 사용자가 확인할 자리는 알려 준다.
const UNDETERMINED_MESSAGE: &str = if cfg!(target_os = "macos") {
    "마이크를 열지 못했다. 마이크 접근이 아직 허용되지 않았을 수 있으니 시스템 설정 › 개인정보 보호 및 보안 › 마이크에서 Molt Note의 접근을 확인해야 한다."
} else {
    "마이크를 열지 못했다. 마이크 접근이 아직 허용되지 않았을 수 있으니 시스템의 마이크 개인정보 설정에서 Molt Note의 접근을 확인해야 한다."
};

#[cfg(test)]
mod tests {
    use super::*;

    /// 테스트가 상태를 정하는 권한 경계.
    struct FixedAccess(MicrophoneAccess);

    impl MicrophonePermission for FixedAccess {
        fn status(&self) -> MicrophoneAccess {
            self.0
        }

        fn request(&self) -> MicrophoneAccess {
            self.0
        }
    }

    #[test]
    fn the_three_states_stay_three_states() {
        for access in [
            MicrophoneAccess::Granted,
            MicrophoneAccess::Denied,
            MicrophoneAccess::Undetermined,
        ] {
            let source = FixedAccess(access);
            assert_eq!(source.status(), access);
            assert_eq!(source.request(), access, "요청 뒤의 상태를 돌려준다");
        }

        assert_ne!(MicrophoneAccess::Denied, MicrophoneAccess::Undetermined);
    }

    #[test]
    fn a_denied_state_tells_the_user_where_to_go() {
        let failure = access_denied();

        assert_eq!(failure.kind, FailureKind::MicrophonePermission);
        assert!(!failure.retryable, "설정을 바꾸지 않으면 결과가 같다");
        assert!(failure.source_data_safe, "권한 실패는 저장된 것을 건드리지 않는다");

        // 사용자가 읽고 행동할 수 있는 문장이다 — 어디를 열어야 하는지가 들어 있다.
        assert!(failure.message.contains("마이크"), "{}", failure.message);
        assert!(
            failure.message.contains("허용"),
            "무엇을 해야 하는지 없다: {}",
            failure.message
        );
        if cfg!(target_os = "macos") {
            assert!(
                failure.message.contains("시스템 설정")
                    && failure.message.contains("개인정보 보호 및 보안")
                    && failure.message.contains("마이크"),
                "macOS 설정 경로가 없다: {}",
                failure.message
            );
        }
    }

    #[test]
    fn a_device_failure_becomes_a_permission_hint_only_while_the_state_is_unknown() {
        let opening = Failure::retryable(FailureKind::AudioDevice, "고른 입력 장치를 열지 못했다.")
            .with_detail("device unavailable");

        let explained =
            explain_open_failure(MicrophoneAccess::Undetermined, opening.clone());

        assert_eq!(explained.kind, FailureKind::MicrophonePermission);
        assert_ne!(explained.message, opening.message, "안내 문장으로 바뀐다");
        let detail = explained.detail.expect("기술적 원인이 남아야 한다");
        assert!(detail.contains("device unavailable"), "{detail}");
        assert!(detail.contains("고른 입력 장치를 열지 못했다."), "{detail}");
    }

    #[test]
    fn a_device_failure_stays_a_device_failure_when_access_is_known_to_be_granted() {
        // 권한이 있는데도 장치를 열지 못했다면 그것은 권한 문제가 아니다 — 사용자가 할 일이 다르다.
        let opening = Failure::retryable(FailureKind::AudioDevice, "고른 입력 장치를 열지 못했다.");

        let explained = explain_open_failure(MicrophoneAccess::Granted, opening.clone());

        assert_eq!(explained, opening, "그대로 둔다");
    }

    #[test]
    fn a_recording_setup_failure_is_never_relabelled_as_a_permission_problem() {
        // 파일을 만들지 못한 것은 녹음 초기화 실패다. 권한 안내로 바꾸면 사용자는 없는 문제를
        // 고치려 하게 된다 (§13).
        let storage = Failure::retryable(FailureKind::Storage, "녹음 파일을 만들지 못했다.");

        let explained = explain_open_failure(MicrophoneAccess::Undetermined, storage.clone());

        assert_eq!(explained, storage);
        assert_ne!(explained.kind, FailureKind::MicrophonePermission);
    }

    #[test]
    fn the_system_boundary_does_not_claim_a_state_it_cannot_observe() {
        // 확인하지 못한 것을 확인한 것처럼 다루지 않는다 (ADR-0005 §4).
        let expected = if cfg!(target_os = "macos") {
            MicrophoneAccess::Undetermined
        } else {
            MicrophoneAccess::Granted
        };

        assert_eq!(SystemMicrophonePermission.status(), expected);
        assert_eq!(SystemMicrophonePermission.request(), expected);
    }
}
