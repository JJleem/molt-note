//! 녹음 session의 상태 기계 (PRODUCT-SPEC §6 · §18 · Phase 2B 요구사항 8).
//!
//! ```text
//! idle ──start──▶ recording ──pause──▶ paused ──resume──▶ recording ──stop──▶ stopped
//!                     └────────────────── stop ──────────────────────────────────┘
//!                                          paused ──stop──▶ stopped
//! ```
//!
//! **이 모듈에는 장치도, 시계도, 파일도 없다.** 경과 시간은 호출자가 밀리초 **값**으로
//! 넣어 준다 ([`RecordingSession::start`] 등의 `at_ms`). 그래서 전이표 전체와 pause를
//! 제외한 duration 누적이 마이크 없이, 그리고 시간을 실제로 흘려보내지 않고 검증된다 (§18).
//! 진짜 장치를 여는 일은 [`super::capture`] 쪽에 있고, 그 둘이 갈라져 있는 것이 이 파일의
//! 목적이다 — 요구사항 8이 요구하는 분리가 이것이다.
//!
//! 잘못된 전이는 **panic이 아니라 [`Failure`] 값으로 거절한다** (§13). 사용자가 정지된
//! 녹음을 한 번 더 정지시키는 것은 버그가 아니라 제품이 설명해야 할 상태다.
//!
//! 사람이 읽는 길이 문자열은 [`crate::domain::format_duration_ms`]를 그대로 쓴다.
//! 그 규칙이 사는 곳은 여전히 한 곳뿐이다 (`tests/screen-boundary.test.ts`).

use std::fmt;

use crate::domain::{format_duration_ms, Failure, FailureKind};

/// 녹음 session이 있을 수 있는 네 상태.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SessionState {
    /// 아직 시작하지 않았다.
    #[default]
    Idle,
    /// 녹음 중이다. 이 상태에서만 경과 시간이 자란다.
    Recording,
    /// 일시정지 상태다. 경과 시간은 멈춰 있다.
    Paused,
    /// 끝났다. 같은 session을 다시 시작하지 않는다.
    Stopped,
}

impl SessionState {
    /// 네 상태 전부. 전이표를 빠짐없이 검사할 때 쓴다.
    pub const ALL: [Self; 4] = [Self::Idle, Self::Recording, Self::Paused, Self::Stopped];

    /// 안정적인 문자열 표현.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Recording => "recording",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
        }
    }
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 정지한 session이 남기는 값.
///
/// 길이를 밀리초와 사람이 읽는 문자열로 함께 담는다 — [`crate::domain::RecordingView`]와
/// 같은 이유다. 화면이 같은 계산을 다시 하지 않게 한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    /// pause 구간을 뺀 녹음 길이(밀리초).
    pub duration_ms: i64,
    /// [`format_duration_ms`]가 만든 표시용 길이(예: `"52:31"`).
    pub duration_label: String,
}

/// 녹음 session 하나.
///
/// 상태와 경과 시간만 들고 있다. 어떤 장치로 녹음하는지도, 파일이 어디에 쓰이는지도 알지 않는다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingSession {
    state: SessionState,
    /// 이미 끝난 녹음 구간들의 합(밀리초). pause와 stop 시점에 더해진다.
    accumulated_ms: i64,
    /// 지금 자라고 있는 구간이 시작된 시각. [`SessionState::Recording`]일 때만 값이 있다.
    segment_started_at_ms: Option<i64>,
}

impl Default for RecordingSession {
    fn default() -> Self {
        Self::idle()
    }
}

impl RecordingSession {
    /// 아직 시작하지 않은 session.
    pub fn idle() -> Self {
        Self {
            state: SessionState::Idle,
            accumulated_ms: 0,
            segment_started_at_ms: None,
        }
    }

    /// 지금 상태.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// 녹음을 시작한다. [`SessionState::Idle`]에서만 할 수 있다.
    ///
    /// 이미 진행 중이거나 끝난 session을 다시 시작하지 않는다 — 그렇게 하면 앞선 구간의
    /// 경과 시간이 조용히 사라진다.
    pub fn start(&mut self, at_ms: i64) -> Result<(), Failure> {
        match self.state {
            SessionState::Idle => {
                self.state = SessionState::Recording;
                self.segment_started_at_ms = Some(at_ms);
                Ok(())
            }
            SessionState::Recording | SessionState::Paused => Err(self.rejected(
                "이미 녹음이 진행 중이다. 새로 시작하려면 먼저 정지해야 한다.",
                "start",
            )),
            SessionState::Stopped => Err(self.rejected(
                "이 녹음은 이미 끝났다. 새로 녹음하려면 새 session이 필요하다.",
                "start",
            )),
        }
    }

    /// 일시정지한다. 녹음 중일 때만 할 수 있다.
    ///
    /// 여기서 지금까지의 구간이 누적으로 확정된다. **이 시점 이후의 시간은 duration에
    /// 들어가지 않는다** — 그것이 pause의 의미다 (Phase 2B 요구사항 4).
    pub fn pause(&mut self, at_ms: i64) -> Result<(), Failure> {
        if self.state != SessionState::Recording {
            return Err(self.rejected("녹음 중이 아니어서 일시정지할 수 없다.", "pause"));
        }

        self.accumulated_ms = self.accumulated_ms.saturating_add(self.open_segment_ms(at_ms));
        self.segment_started_at_ms = None;
        self.state = SessionState::Paused;
        Ok(())
    }

    /// 다시 녹음한다. 일시정지 상태일 때만 할 수 있다.
    ///
    /// 누적된 시간은 그대로 두고 새 구간을 연다. pause와 resume 사이의 시간은 어디에도
    /// 더해지지 않는다.
    pub fn resume(&mut self, at_ms: i64) -> Result<(), Failure> {
        if self.state != SessionState::Paused {
            return Err(self.rejected("일시정지 상태가 아니어서 재개할 수 없다.", "resume"));
        }

        self.segment_started_at_ms = Some(at_ms);
        self.state = SessionState::Recording;
        Ok(())
    }

    /// 녹음을 끝낸다. 녹음 중이거나 일시정지 상태일 때 할 수 있다.
    ///
    /// 일시정지 상태에서 정지하는 것은 정상적인 사용이다 — 멈춰 둔 녹음을 그대로 끝낸다.
    pub fn stop(&mut self, at_ms: i64) -> Result<SessionSummary, Failure> {
        match self.state {
            SessionState::Recording => {
                self.accumulated_ms =
                    self.accumulated_ms.saturating_add(self.open_segment_ms(at_ms));
                self.segment_started_at_ms = None;
                self.state = SessionState::Stopped;
                Ok(self.summary())
            }
            SessionState::Paused => {
                self.state = SessionState::Stopped;
                Ok(self.summary())
            }
            SessionState::Idle => Err(self.rejected("시작하지 않은 녹음은 정지할 수 없다.", "stop")),
            SessionState::Stopped => Err(self.rejected("이 녹음은 이미 정지됐다.", "stop")),
        }
    }

    /// 지금까지의 녹음 길이(밀리초). **pause 구간은 빠져 있다.**
    ///
    /// `now_ms`는 녹음 중일 때만 쓰인다. 일시정지·정지 상태의 길이는 시간이 흘러도 변하지 않으므로
    /// 어떤 값을 넣어도 같은 답이다.
    pub fn elapsed_ms(&self, now_ms: i64) -> i64 {
        self.accumulated_ms.saturating_add(self.open_segment_ms(now_ms))
    }

    /// 지금까지의 녹음 길이를 사람이 읽는 문자열로.
    ///
    /// 규칙은 [`format_duration_ms`] 한 곳에만 있다. 화면도 여기서 온 문자열을 그대로 쓴다.
    pub fn elapsed_label(&self, now_ms: i64) -> String {
        format_duration_ms(self.elapsed_ms(now_ms))
    }

    /// 아직 닫히지 않은 구간의 길이. 열린 구간이 없으면 0이다.
    ///
    /// 들어온 시각이 구간 시작보다 이르면 0으로 본다 — 호출자의 시계가 뒤로 갔다고 해서
    /// 이미 녹음한 시간을 깎지 않는다.
    fn open_segment_ms(&self, now_ms: i64) -> i64 {
        match self.segment_started_at_ms {
            Some(started) => now_ms.saturating_sub(started).max(0),
            None => 0,
        }
    }

    /// 확정된 길이로 만든 보고 값.
    fn summary(&self) -> SessionSummary {
        SessionSummary {
            duration_ms: self.accumulated_ms,
            duration_label: format_duration_ms(self.accumulated_ms),
        }
    }

    /// 거절 하나. 상태는 바꾸지 않는다 — 거절은 아무 일도 일어나지 않았다는 뜻이다.
    ///
    /// 같은 상태에서 같은 요청을 다시 보내면 결과가 같으므로 재시도 가능한 실패가 아니다.
    fn rejected(&self, message: &str, action: &str) -> Failure {
        Failure::permanent(FailureKind::InvalidInput, message)
            .with_detail(format!("{action} rejected in state={}", self.state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// session에 보낼 수 있는 요청 전부. 전이표를 빠짐없이 도는 데 쓴다.
    #[derive(Debug, Clone, Copy)]
    enum Action {
        Start,
        Pause,
        Resume,
        Stop,
    }

    impl Action {
        const ALL: [Self; 4] = [Self::Start, Self::Pause, Self::Resume, Self::Stop];

        /// 요청 하나를 보낸다. 어떤 요청이든 panic 없이 결과를 값으로 돌려준다.
        fn apply(self, session: &mut RecordingSession, at_ms: i64) -> Result<(), Failure> {
            match self {
                Self::Start => session.start(at_ms),
                Self::Pause => session.pause(at_ms),
                Self::Resume => session.resume(at_ms),
                Self::Stop => session.stop(at_ms).map(|_| ()),
            }
        }
    }

    /// 실제 전이만 써서 해당 상태의 session을 만든다. 내부 필드를 손으로 채우지 않는다.
    fn session_in(state: SessionState) -> RecordingSession {
        let mut session = RecordingSession::idle();
        match state {
            SessionState::Idle => {}
            SessionState::Recording => session.start(0).expect("idle에서 시작할 수 있다"),
            SessionState::Paused => {
                session.start(0).expect("idle에서 시작할 수 있다");
                session.pause(1_000).expect("녹음 중에 멈출 수 있다");
            }
            SessionState::Stopped => {
                session.start(0).expect("idle에서 시작할 수 있다");
                session.stop(1_000).expect("녹음 중에 정지할 수 있다");
            }
        }
        assert_eq!(session.state(), state, "사전 조건: 원하는 상태를 만들었다");
        session
    }

    /// 전이표. 허용되는 다섯 칸이 전부이고, 나머지 열한 칸은 거절이다.
    fn is_allowed(state: SessionState, action: Action) -> bool {
        matches!(
            (state, action),
            (SessionState::Idle, Action::Start)
                | (SessionState::Recording, Action::Pause)
                | (SessionState::Recording, Action::Stop)
                | (SessionState::Paused, Action::Resume)
                | (SessionState::Paused, Action::Stop)
        )
    }

    #[test]
    fn the_whole_lifecycle_walks_idle_recording_paused_recording_stopped() {
        let mut session = RecordingSession::idle();
        assert_eq!(session.state(), SessionState::Idle);

        session.start(0).expect("시작할 수 있어야 한다");
        assert_eq!(session.state(), SessionState::Recording);

        session.pause(5_000).expect("일시정지할 수 있어야 한다");
        assert_eq!(session.state(), SessionState::Paused);

        session.resume(9_000).expect("재개할 수 있어야 한다");
        assert_eq!(session.state(), SessionState::Recording);

        let summary = session.stop(11_000).expect("정지할 수 있어야 한다");
        assert_eq!(session.state(), SessionState::Stopped);

        // 0~5초 녹음 + 5~9초 일시정지 + 9~11초 녹음 = 7초.
        assert_eq!(summary.duration_ms, 7_000);
        assert_eq!(summary.duration_label, "0:07");
    }

    #[test]
    fn every_cell_of_the_transition_table_is_covered_and_only_five_are_allowed() {
        let mut allowed = 0;

        for state in SessionState::ALL {
            for action in Action::ALL {
                let mut session = session_in(state);
                let result = action.apply(&mut session, 2_000);

                if is_allowed(state, action) {
                    allowed += 1;
                    assert!(
                        result.is_ok(),
                        "{state}에서 {action:?}는 허용되어야 한다: {result:?}"
                    );
                    assert_ne!(session.state(), state, "허용된 전이는 상태를 옮긴다");
                } else {
                    let failure = result.expect_err(&format!("{state}에서 {action:?}는 거절이다"));
                    assert_eq!(failure.kind, FailureKind::InvalidInput);
                    assert_eq!(session.state(), state, "거절은 상태를 바꾸지 않는다");
                }
            }
        }

        assert_eq!(allowed, 5, "허용되는 칸은 다섯 개뿐이다");
    }

    #[test]
    fn a_rejected_transition_is_a_failure_value_the_user_can_read() {
        // panic이 아니다 — 실패는 값으로 돌아오고, 사용자에게 보여줄 문장을 갖는다 (§13).
        let mut idle = session_in(SessionState::Idle);
        let failure = idle.stop(1_000).expect_err("시작하지 않은 녹음은 정지할 수 없다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.message.is_empty(), "보여줄 문장이 있어야 한다");
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다");
        assert!(!failure.retryable, "같은 상태에서 다시 보내도 결과가 같다");
        assert_eq!(
            failure.detail.as_deref(),
            Some("stop rejected in state=idle"),
            "어느 상태에서 무엇이 거절됐는지 남는다"
        );
    }

    #[test]
    fn each_wrong_transition_names_what_went_wrong() {
        // 네 가지 잘못된 전이가 서로 다른 문장을 준다 — 화면이 같은 말을 반복하지 않는다.
        let mut recording = session_in(SessionState::Recording);
        let already_recording = recording.start(1).expect_err("이미 녹음 중이다");

        let mut idle = session_in(SessionState::Idle);
        let not_recording = idle.pause(1).expect_err("녹음 중이 아니다");

        let mut running = session_in(SessionState::Recording);
        let not_paused = running.resume(1).expect_err("일시정지 상태가 아니다");

        let mut fresh = session_in(SessionState::Idle);
        let never_started = fresh.stop(1).expect_err("시작한 적이 없다");

        let mut finished = session_in(SessionState::Stopped);
        let already_stopped = finished.stop(1).expect_err("이미 정지됐다");

        let messages = [
            already_recording.message,
            not_recording.message,
            not_paused.message,
            never_started.message,
            already_stopped.message,
        ];
        let mut seen: Vec<&str> = Vec::new();
        for message in &messages {
            assert!(!seen.contains(&message.as_str()), "문장이 겹친다: {message}");
            seen.push(message.as_str());
        }
    }

    #[test]
    fn the_paused_span_is_not_counted_in_the_duration() {
        // 이 테스트가 이 모듈의 핵심이다. pause 구간이 duration에 들어가면 여기서 실패한다.
        let mut session = RecordingSession::idle();
        session.start(1_000).expect("시작");
        session.pause(4_000).expect("일시정지"); // 3초 녹음

        // 일시정지 상태에서는 시간이 아무리 흘러도 길이가 자라지 않는다.
        assert_eq!(session.elapsed_ms(4_000), 3_000);
        assert_eq!(session.elapsed_ms(100_000), 3_000, "멈춰 있는 동안은 자라지 않는다");

        session.resume(100_000).expect("재개"); // 96초를 멈춰 있었다
        let summary = session.stop(102_000).expect("정지"); // 2초 더 녹음

        assert_eq!(
            summary.duration_ms, 5_000,
            "3초 + 2초다. 벽시계로는 101초가 지났다"
        );
        assert_ne!(summary.duration_ms, 101_000, "일시정지 구간이 포함되면 안 된다");
    }

    #[test]
    fn many_pause_and_resume_cycles_keep_accumulating_only_the_recorded_spans() {
        let mut session = RecordingSession::idle();
        session.start(0).expect("시작");

        // 10초 녹음 · 1000초 정지, 를 세 번 반복한다.
        let mut clock = 0;
        for _ in 0..3 {
            clock += 10_000;
            session.pause(clock).expect("일시정지");
            clock += 1_000_000;
            session.resume(clock).expect("재개");
        }

        // 마지막 구간은 30초 녹음하고 끝낸다.
        clock += 30_000;
        let summary = session.stop(clock).expect("정지");

        assert_eq!(summary.duration_ms, 3 * 10_000 + 30_000);
        assert_eq!(summary.duration_label, "1:00");
        assert!(clock > 3_000_000, "벽시계로는 훨씬 오래 지났다: {clock}ms");
    }

    #[test]
    fn the_duration_grows_with_the_time_it_is_given_while_recording() {
        // 시계를 읽지 않는다 — 흐르는 시간은 전부 인자로 들어온다.
        let mut session = RecordingSession::idle();
        session.start(1_000).expect("시작");

        assert_eq!(session.elapsed_ms(1_000), 0);
        assert_eq!(session.elapsed_ms(2_500), 1_500);
        assert_eq!(session.elapsed_ms(61_000), 60_000);
    }

    #[test]
    fn a_stopped_session_reports_the_same_length_no_matter_what_time_is_given() {
        let mut session = RecordingSession::idle();
        session.start(0).expect("시작");
        let summary = session.stop(7_000).expect("정지");

        assert_eq!(session.elapsed_ms(7_000), summary.duration_ms);
        assert_eq!(session.elapsed_ms(999_999_999), summary.duration_ms);
    }

    #[test]
    fn an_idle_session_has_no_length_yet() {
        let session = RecordingSession::idle();

        assert_eq!(session.elapsed_ms(500_000), 0);
        assert_eq!(session.elapsed_label(500_000), "0:00");
        assert_eq!(session, RecordingSession::default());
    }

    #[test]
    fn the_human_readable_length_comes_from_the_one_place_that_rule_lives() {
        let mut session = RecordingSession::idle();
        session.start(0).expect("시작");

        assert_eq!(session.elapsed_label(3_151_000), format_duration_ms(3_151_000));
        assert_eq!(session.elapsed_label(3_151_000), "52:31");

        let summary = session.stop(3_661_000).expect("정지");
        assert_eq!(summary.duration_label, format_duration_ms(summary.duration_ms));
        assert_eq!(summary.duration_label, "1:01:01");
    }

    #[test]
    fn a_time_that_goes_backwards_never_shortens_the_recording() {
        // 호출자의 시계가 뒤로 가더라도 음수 길이는 만들지 않는다.
        let mut session = RecordingSession::idle();
        session.start(10_000).expect("시작");

        assert_eq!(session.elapsed_ms(9_000), 0);

        let summary = session.stop(9_000).expect("정지");
        assert_eq!(summary.duration_ms, 0);
        assert_eq!(summary.duration_label, "0:00");
    }

    #[test]
    fn extreme_time_values_do_not_panic() {
        // 있을 수 없는 값이 들어와도 녹음 세션이 앱을 멈추지 않는다.
        let mut session = RecordingSession::idle();
        session.start(i64::MIN).expect("시작");

        assert!(session.elapsed_ms(i64::MAX) > 0);

        session.pause(i64::MAX).expect("일시정지");
        session.resume(i64::MIN).expect("재개");
        let summary = session.stop(i64::MAX).expect("정지");

        assert_eq!(summary.duration_ms, i64::MAX, "포화되고 넘치지 않는다");
    }

    #[test]
    fn every_state_has_a_distinct_stable_string() {
        let mut seen = Vec::new();
        for state in SessionState::ALL {
            let text = state.as_str();
            assert!(!seen.contains(&text), "상태 문자열이 겹친다: {text}");
            seen.push(text);
        }
        assert_eq!(seen, ["idle", "recording", "paused", "stopped"]);
        assert_eq!(SessionState::default(), SessionState::Idle);
        assert_eq!(SessionState::Recording.to_string(), "recording");
    }
}
