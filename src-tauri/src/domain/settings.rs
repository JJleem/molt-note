//! 앱 설정 (PRODUCT-SPEC §5 D).
//!
//! 여기 있는 항목은 §5 D의 전부가 아니다. 각 항목은 **그 기능을 실제로 구현하는 Phase가
//! 함께 추가한다** — 값을 담을 자리만 미리 만들어 두지 않는다 (§20.6). Phase 1이
//! recordings directory · automatic 처리 토글 · default microphone을 두었고, Phase 3이
//! 전사 두 값(**자동 전사 토글**과 **모델 선택**)을 더하며, Phase 4가 AI provider 세 값을
//! 더한다 (`docs/ADR-0008-note-ai-provider.md` §11.1). Notion destination은 여전히 없다.
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
    /// 어떤 후처리를 돌릴지는 그 기능이 존재하는 Phase가 정한다. 이 값을 읽고 무언가를
    /// 실행하는 코드는 아직 없다.
    ///
    /// **[`Self::automatic_transcription`]과 다른 값이다.** 하나의 boolean에 두 의미를 겹치지
    /// 않는다 — 한쪽을 켰다고 다른 쪽이 켜지지 않으며, 둘은 따로 저장되고 따로 복원된다.
    pub automatic_processing: bool,
    /// 녹음을 정지해 저장한 직후에 **전사를 자동으로 시작할지** 여부
    /// (`phase-prompt/03` 요구 4 · ADR-0007 §8.2.3).
    ///
    /// 이 값을 읽고 실제로 전사를 거는 자리는 `crate::commands::finish_recording` 하나다.
    /// 꺼져 있으면 정지 뒤에 아무 전사도 시작되지 않지만, **수동 전사는 이 값과 무관하게
    /// 언제나 가능하다** — 이것은 자동 시작에 대한 값이지 전사 기능의 스위치가 아니다.
    ///
    /// 모델이 없어서 전사가 실패하더라도 **앱이 이 값을 뒤집지 않는다.** 사용자가 켠 것은
    /// 켜진 채로 남고, 실행이 불가능하다는 사실은 별도의 상태로 표현된다 (ADR-0007 §8.2.3).
    pub automatic_transcription: bool,
    /// 전사에 쓸 모델 파일의 **이름 또는 경로**. `None`은 **아직 고르지 않았다**는 정상 상태다.
    ///
    /// domain은 이 값이 어느 파일을 가리키는지 알지 않는다 (INV-10). 모델 디렉터리 안의
    /// 파일명인지 절대 경로인지를 해석하는 자리는 `crate::transcription::model` 하나다
    /// (ADR-0007 §8.2).
    ///
    /// `default_microphone`과 같은 성질을 갖는다 — 저장된 값이 가리키는 파일이 지금 그 자리에
    /// 없을 수 있고, 그것은 **이 값이 틀렸다는 뜻이 아니라 지금 그 모델이 없다**는 뜻이다.
    /// 그 사실 때문에 저장된 선택을 조용히 지우거나 바꾸지 않는다.
    pub transcription_model: Option<String>,
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
    /// 노트를 만들 때 쓸 AI provider의 **식별자**. `None`은 **아직 고르지 않았다**는
    /// 정상 상태다 (INV-8 · ADR-0008 §11.1).
    ///
    /// 벤더 중립 자유 식별자이며, domain은 이 값을 알려진 목록과 대조하지 않는다 (INV-9) —
    /// `AiNote::provider`가 이미 같은 규칙을 따른다. 이 값이 어떤 서버를 어떻게 부르는지는
    /// 그 provider의 adapter만 안다.
    ///
    /// **고르지 않은 것은 오류가 아니다.** 그 상태에서 녹음 · 전사 · 열람은 전부 그대로
    /// 동작하고, 없는 provider에게 물어보는 대신 "AI 기능이 비활성"이라는 담담한 상태가 된다.
    pub ai_provider: Option<String>,
    /// AI provider에 연결할 **주소**. `None`은 아직 고르지 않았다는 정상 상태이며, 그때
    /// 실제로 쓰는 값은 [`Settings::ai_base_url_or_default`]가 답한다.
    ///
    /// **secret이 아니다** — 어디에 연결하는지일 뿐이며, 그래서 INV-7과 충돌하지 않는다.
    /// 다만 이 값은 실패 message · detail · 로그에 남지 않는다 (ADR-0008 §11.3).
    ///
    /// `transcription_model`과 같은 성질을 갖는다 — 이 주소의 서버가 지금 응답하는지는
    /// 이 값만으로 알 수 없고, 응답하지 않는다고 해서 저장된 값을 지우거나 바꾸지 않는다.
    pub ai_base_url: Option<String>,
    /// 노트를 만들 때 쓸 **모델 식별자**. `None`은 아직 고르지 않았다는 정상 상태다.
    ///
    /// 고른 모델이 지금 그 서버에 설치돼 있는지는 여기서 묻지 않는다. 모델은 지워질 수 있고,
    /// 그것은 **이 값이 틀렸다는 뜻이 아니라 지금 그 모델이 없다**는 뜻이다 — 그 구분은
    /// 서버에게 물어본 쪽이 하며, 앱이 조용히 다른 모델로 바꾸지 않는다.
    pub ai_model: Option<String>,
}

/// AI provider에 연결할 주소를 아직 고르지 않았을 때 쓰는 값.
///
/// **이 값은 저장소에도, adapter에도, 화면에도 복사되지 않는다 — 여기 한 곳에만 있다.**
/// 같은 주소가 여러 곳에 적히면 한 곳을 고쳤을 때 나머지가 조용히 달라진다.
///
/// 로컬 AI 서버가 기본으로 듣는 주소이며, PRODUCT-SPEC §14.5가 primary source에서 확인해
/// 기록하고 ADR-0008 §14가 그대로 옮긴 값이다 (VERIFIED · 2026-09-01). 사용자가 서버를 다른
/// 곳에 띄웠다면 [`Settings::ai_base_url`]로 그 값을 대신 쓴다 — 그래서 이것은 **강제된 주소가
/// 아니라 고르지 않았을 때의 출발점**이다.
pub const DEFAULT_AI_BASE_URL: &str = "http://localhost:11434";

impl Settings {
    /// 저장된 값이 없을 때 쓰는 기본값.
    ///
    /// - `recordings_directory`: 고르지 않은 상태(`None`). 사용자가 고르기 전에 임의의
    ///   디렉터리를 설정 값으로 굳혀 두지 않는다.
    /// - `automatic_processing`: **OFF**. 자동 실행은 사용자가 명시적으로 켜는 것이며,
    ///   기본으로 켜 두고 끄게 하지 않는다.
    /// - `automatic_transcription`: **OFF** (V1). 같은 이유이며 하나가 더 있다 — 전사는
    ///   기기의 자원을 오래 쓰는 일이고, 모델을 고르기 전에는 시작할 수도 없다. 자동으로
    ///   켜 두면 사용자가 켠 적 없는 실패를 녹음마다 보게 된다.
    /// - `transcription_model`: 고르지 않은 상태(`None`). 모델 디렉터리에서 아무 파일이나
    ///   찾아 기본값으로 굳히지 않는다 — 어떤 모델로 전사할지는 사용자가 정한다
    ///   (ADR-0007 §8.1).
    /// - `default_microphone`: 고르지 않은 상태(`None`). 열거된 첫 장치를 기본값으로
    ///   굳혀 두지 않는다 — 사용자가 고른 적 없는 값이 고른 값처럼 보이면, 나중에 그
    ///   장치가 사라져도 무엇이 바뀐 것인지 말할 수 없게 된다.
    /// - `ai_provider`: 고르지 않은 상태(`None`). **어떤 provider도 기본으로 굳혀 두지
    ///   않는다** — 노트를 만드는 것은 사용자가 시작하는 일이고, 고르지 않은 상태가
    ///   그 자체로 정상이다 (INV-8 · ADR-0008 §11.1).
    /// - `ai_base_url`: 고르지 않은 상태(`None`). 그때 실제로 쓰는 주소는
    ///   [`DEFAULT_AI_BASE_URL`]이지만, 그것을 **저장된 값으로 써 넣지 않는다** — 그래야
    ///   "고른 적 없음"과 "마침 같은 주소를 고름"이 계속 구분되고, 나중에 기본 주소가
    ///   달라져도 고른 적 없는 사용자의 DB를 고치지 않아도 된다.
    /// - `ai_model`: 고르지 않은 상태(`None`). 설치된 모델 중 아무 것이나 골라 두지
    ///   않는다 — `transcription_model`과 같은 이유다.
    pub const DEFAULT: Self = Self {
        recordings_directory: None,
        automatic_processing: false,
        automatic_transcription: false,
        transcription_model: None,
        default_microphone: None,
        ai_provider: None,
        ai_base_url: None,
        ai_model: None,
    };

    /// 실제로 연결할 주소. 고른 값이 없으면 [`DEFAULT_AI_BASE_URL`]이다.
    ///
    /// **기본 주소를 아는 자리는 여기 하나다.** adapter는 이 값을 받아서 쓰고 자기 안에
    /// 주소를 갖지 않으며, 그래서 사용자가 서버를 옮겼을 때 바뀌는 곳도 한 곳이다.
    pub fn ai_base_url_or_default(&self) -> &str {
        self.ai_base_url.as_deref().unwrap_or(DEFAULT_AI_BASE_URL)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::DEFAULT
    }
}
