//! 녹음 session의 상태 기계가 **하드웨어도 시계도 없이** 동작하는지 본다
//! (PRODUCT-SPEC §6 · §18 · Phase 2B 요구사항 4 · 8).
//!
//! 이 파일에는 마이크도, 임시 디렉터리도, 흐르는 시간도 없다. 시각은 전부 숫자로 들어간다 —
//! 그래서 1시간짜리 녹음의 duration 누적도 즉시 검증된다.
//!
//! 두 가지를 본다.
//!
//! ```text
//! 1. crate 공개 API로 지나가는 전이와 duration 누적 (행동)
//! 2. 모듈 소스에 하드웨어·시계·파일시스템 의존이 들어오지 못하게 하는 검사 (구조)
//! ```
//!
//! 2번은 `domain_invariants.rs`와 같은 방식이다. 나중에 누군가 이 모듈 안에서 시계를 직접
//! 읽으면 그 순간 상태 기계는 값만으로 검증할 수 없게 되고, 여기서 실패한다.

use molt_note_lib::audio::{RecordingSession, SessionState};
use molt_note_lib::domain::{format_duration_ms, FailureKind};

/// 구조 검사 대상. 이 테스트 파일 자신은 대상이 아니다 —
/// 그래서 금지 문자열을 여기 그대로 적어도 자기 자신에 걸리지 않는다.
const SESSION_SOURCE: &str = include_str!("../src/audio/session.rs");

/// 공백을 하나로 줄이고 대문자로 맞춘다. 줄바꿈이나 들여쓰기로 검사를 피해 갈 수 없게 한다.
fn normalized(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase()
}

#[test]
fn the_state_machine_module_reaches_no_hardware_no_clock_and_no_filesystem() {
    // 요구사항 8: 상태 기계와 실제 캡처 장치가 분리되어 있어야 한다.
    let forbidden = [
        "CPAL",           // 실제 오디오 장치
        "HOUND",          // WAV 파일 쓰기
        "STD::TIME",      // 시계
        "INSTANT",        // 시계
        "SYSTEMTIME",     // 시계
        "NOW()",          // 시계를 읽는 흔한 이름
        "STD::FS",        // 파일시스템
        "FS::",           // 파일시스템
        "PATHBUF",        // 파일 경로
        "STD::THREAD",    // 캡처 스레드
        "SYNCSENDER",     // 캡처 통로
        "TAURI",          // 앱 프레임워크
    ];

    let text = normalized(SESSION_SOURCE);
    for token in forbidden {
        assert!(
            !text.contains(token),
            "상태 기계 모듈이 {token}에 닿아 있다. 시각은 값으로 들어와야 한다"
        );
    }
}

#[test]
fn a_full_recording_walks_idle_recording_paused_recording_stopped() {
    let mut session = RecordingSession::idle();
    assert_eq!(session.state(), SessionState::Idle);

    session.start(0).expect("idle에서 시작할 수 있다");
    assert_eq!(session.state(), SessionState::Recording);

    session.pause(600_000).expect("녹음 중에 일시정지할 수 있다");
    assert_eq!(session.state(), SessionState::Paused);

    session.resume(1_200_000).expect("일시정지 상태에서 재개할 수 있다");
    assert_eq!(session.state(), SessionState::Recording);

    let summary = session.stop(1_800_000).expect("녹음 중에 정지할 수 있다");
    assert_eq!(session.state(), SessionState::Stopped);

    // 10분 녹음 · 10분 일시정지 · 10분 녹음. 벽시계로는 30분이지만 녹음은 20분이다.
    assert_eq!(summary.duration_ms, 1_200_000);
    assert_eq!(summary.duration_label, "20:00");
    assert_eq!(summary.duration_label, format_duration_ms(summary.duration_ms));
}

#[test]
fn the_paused_span_stays_out_of_the_duration_however_long_it_lasts() {
    let mut session = RecordingSession::idle();
    session.start(0).expect("시작");
    session.pause(1_000).expect("일시정지");

    // 한 시간을 멈춰 둔다. 그동안 길이는 1초 그대로다.
    for elapsed in [1_000, 60_000, 3_600_000] {
        assert_eq!(session.elapsed_ms(elapsed), 1_000);
        assert_eq!(session.elapsed_label(elapsed), "0:01");
    }

    let summary = session.stop(3_600_000).expect("일시정지 상태에서 정지할 수 있다");
    assert_eq!(summary.duration_ms, 1_000, "멈춰 있던 한 시간은 녹음이 아니다");
}

#[test]
fn a_wrong_transition_comes_back_as_a_failure_the_screen_can_show() {
    // panic이 아니라 값이다 — 진행 중인 녹음이 잘못된 요청 하나로 사라지지 않는다 (R-001 · §13).
    let mut session = RecordingSession::idle();
    session.start(0).expect("시작");
    session.pause(5_000).expect("일시정지");

    let failure = session.pause(6_000).expect_err("이미 일시정지 상태다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(!failure.message.is_empty(), "보여줄 문장이 있어야 한다");
    assert!(failure.source_data_safe);
    assert_eq!(session.state(), SessionState::Paused, "거절은 상태를 바꾸지 않는다");
    assert_eq!(session.elapsed_ms(9_999), 5_000, "거절은 길이도 바꾸지 않는다");

    // 거절당한 뒤에도 session은 멀쩡히 이어진다.
    session.resume(6_000).expect("재개할 수 있다");
    assert_eq!(session.stop(7_000).expect("정지").duration_ms, 6_000);
}
