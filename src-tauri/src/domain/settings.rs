//! 앱 설정 (PRODUCT-SPEC §5 D).
//!
//! 이 Phase가 다루는 것은 §5 D의 항목 전부가 아니라 **recordings directory**와
//! **automatic 처리 ON/OFF 토글** 둘뿐이다. whisper model · AI provider · Notion
//! destination 같은 항목은 그 기능을 실제로 구현하는 Phase가 함께 추가한다 —
//! 값을 담을 자리만 미리 만들어 두지 않는다 (§20.6).
//!
//! **INV-7: 이 타입에는 secret이 없다.** API key · integration token · password 류 값을
//! 담는 필드를 두지 않으며, 저장소에도 그런 열이 없다. secret 보관은 이 Phase의 범위 밖이다
//! (`phase-prompt/01-application-foundation.md` Out of Scope).

/// 사용자가 바꿀 수 있는 앱 설정 값.
///
/// 저장된 값이 아직 없을 때 무엇을 쓸지는 [`Settings::DEFAULT`]가 정한다 —
/// 기본값 정책은 스키마의 `DEFAULT` 절이 아니라 **코드에 선언되어 있다.** 그래야
/// 정책이 바뀔 때 이미 만들어진 사용자 DB를 고치지 않아도 되고, "저장된 적 없음"과
/// "기본값과 같은 값을 저장했음"을 저장소가 구분할 수 있다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// 녹음 파일을 두는 디렉터리. `None`은 **아직 고르지 않았다**는 정상 상태다.
    ///
    /// 값이 없을 때 실제로 어느 위치를 쓸지는 녹음 파일을 실제로 만드는 쪽이 정한다.
    /// domain은 OS별 경로 규약을 알지 않는다 (INV-10) — 그래서 여기에 플랫폼 경로를
    /// 기본값으로 박아 넣지 않는다.
    pub recordings_directory: Option<String>,
    /// 녹음이 끝난 뒤 후처리를 자동으로 시작할지 여부.
    ///
    /// 어떤 후처리를 돌릴지는 그 기능이 존재하는 Phase가 정한다. 이 Phase는 토글 값의
    /// 영속화만 다루며, 이 값을 읽고 무언가를 실행하는 코드는 아직 없다.
    pub automatic_processing: bool,
    /// 녹음을 시작할 때 기본으로 고를 입력 장치의 **선택 키**.
    /// `None`은 **아직 고르지 않았다**는 정상 상태다.
    ///
    /// 보여 주는 이름이 아니라 키를 담는 이유는 이름이 같은 장치가 둘 있을 수 있어서다
    /// (`crate::audio::devices`). domain은 그 키가 어떻게 만들어지는지 알지 않는다 —
    /// 여기서는 **다시 알아볼 수 있는 값**일 뿐이다 (INV-10).
    ///
    /// 저장된 키가 다음 열거 목록에 없을 수 있다. 장치를 뽑아 두면 그렇게 되며, 그것은
    /// 이 값이 틀렸다는 뜻이 아니라 **지금 그 장치가 없다**는 뜻이다. 그 구분은 목록을
    /// 함께 아는 쪽이 하고, 조용히 다른 장치로 바꾸지 않는다.
    pub default_microphone: Option<String>,
}

impl Settings {
    /// 저장된 값이 없을 때 쓰는 기본값.
    ///
    /// - `recordings_directory`: 고르지 않은 상태(`None`). 사용자가 고르기 전에 임의의
    ///   디렉터리를 설정 값으로 굳혀 두지 않는다.
    /// - `automatic_processing`: **OFF**. 자동 실행은 사용자가 명시적으로 켜는 것이며,
    ///   기본으로 켜 두고 끄게 하지 않는다.
    /// - `default_microphone`: 고르지 않은 상태(`None`). 열거된 첫 장치를 기본값으로
    ///   굳혀 두지 않는다 — 사용자가 고른 적 없는 값이 고른 값처럼 보이면, 나중에 그
    ///   장치가 사라져도 무엇이 바뀐 것인지 말할 수 없게 된다.
    pub const DEFAULT: Self = Self {
        recordings_directory: None,
        automatic_processing: false,
        default_microphone: None,
    };
}

impl Default for Settings {
    fn default() -> Self {
        Self::DEFAULT
    }
}
