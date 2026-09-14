//! **녹음이 도는 동안 전사가 함께 돌고, 그 어떤 실패도 녹음을 건드리지 않는다** (2026-09-14).
//!
//! ```text
//! LiveTranscriber.begin ─→ 배경 스레드 ─→ live_run::advance ─→ TranscriptionEngine
//!   (실행자 경계)                            (진행 규칙)          ↑ 여기만 double
//! ```
//!
//! **대체 구현이 들어가는 자리는 엔진 하나뿐이다.** 실행자도, 배경 스레드도, 창을 고르는
//! 규칙도, 녹음 중인 파일을 읽는 경로도 제품 코드 그대로다 — 그래서 여기서 확인되는 것은
//! "double이 흉내 냈다"가 아니라 **실제 경로가 무엇을 남기는가**다.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use molt_note_lib::commands::live_transcriber::{LiveState, LiveTranscriber};
use molt_note_lib::domain::{Failure, FailureKind};
use molt_note_lib::transcription::audio_input::TranscriptionInput;
use molt_note_lib::transcription::engine::{LanguageChoice, TranscriptionEngine};
use molt_note_lib::transcription::model::{self, ModelFile};
use molt_note_lib::transcription::parse::{RawSegment, RawTranscription};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique(name: &str) -> PathBuf {
    let index = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("molt-live-{name}-{}-{index}", std::process::id()))
}

/// 자리표시자 모델. **엔진 double은 이 파일을 열지 않는다.**
fn model() -> ModelFile {
    let dir = unique("model");
    std::fs::create_dir_all(&dir).expect("만들 수 있어야 한다");
    std::fs::write(dir.join("ggml-base.bin"), b"not a real model").expect("쓸 수 있어야 한다");
    model::resolve(&dir, Some("ggml-base.bin")).expect("해석할 수 있어야 한다")
}

/// 실제로 쓰이는 중인 녹음 파일.
struct Recording {
    path: PathBuf,
    file: std::fs::File,
    rate: u32,
}

impl Recording {
    fn start(name: &str) -> Self {
        let path = unique(name).with_extension("wav");
        let rate = 16_000_u32;
        let mut file = std::fs::File::create(&path).expect("만들 수 있어야 한다");

        let (channels, bits) = (1_u16, 16_u16);
        let block_align = channels * bits / 8;
        let mut header = Vec::new();
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&0_u32.to_le_bytes());
        header.extend_from_slice(b"WAVE");
        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16_u32.to_le_bytes());
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&channels.to_le_bytes());
        header.extend_from_slice(&rate.to_le_bytes());
        header.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
        header.extend_from_slice(&block_align.to_le_bytes());
        header.extend_from_slice(&bits.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&0_u32.to_le_bytes());
        file.write_all(&header).expect("헤더를 쓸 수 있어야 한다");

        Self { path, file, rate }
    }

    fn record(&mut self, seconds: usize) {
        let frames = seconds * self.rate as usize;
        let mut bytes = Vec::with_capacity(frames * 2);
        for frame in 0..frames {
            let value = ((frame.wrapping_mul(2_654_435_761) >> 9) & 0x3fff) as i16 - 0x2000;
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        self.file.write_all(&bytes).expect("쓸 수 있어야 한다");
        self.file.flush().expect("내보낼 수 있어야 한다");
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// 부를 때마다 문장 하나를 내는 엔진.
struct Speaking {
    calls: Mutex<usize>,
}

impl Speaking {
    fn new() -> Self {
        Self { calls: Mutex::new(0) }
    }
}

impl TranscriptionEngine for Speaking {
    fn engine_id(&self) -> String {
        "speaking".to_owned()
    }
    fn transcribe(
        &self,
        _input: &TranscriptionInput,
        _model: &ModelFile,
        _language: &LanguageChoice,
    ) -> Result<RawTranscription, Failure> {
        let index = {
            let mut calls = self.calls.lock().expect("셀 수 있어야 한다");
            *calls += 1;
            *calls
        };
        Ok(RawTranscription {
            language: Some("ko".to_owned()),
            segments: vec![RawSegment {
                start_centiseconds: 0,
                end_centiseconds: 100,
                text: Some(format!("들린 말 {index}")),
            }],
        })
    }
}

/// 언제나 실패하는 엔진 — 실시간 전사가 포기하는 경로를 지난다.
struct Broken;

impl TranscriptionEngine for Broken {
    fn engine_id(&self) -> String {
        "broken".to_owned()
    }
    fn transcribe(
        &self,
        _input: &TranscriptionInput,
        _model: &ModelFile,
        _language: &LanguageChoice,
    ) -> Result<RawTranscription, Failure> {
        Err(Failure::permanent(
            FailureKind::TranscriptionEngineFailed,
            "이 엔진은 언제나 실패한다",
        ))
    }
}

/// 조건이 될 때까지 기다린다. 안 되면 실패한다 — **영원히 기다리지 않는다.**
fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("{what}: 시간 안에 일어나지 않았다");
}

#[test]
fn sentences_appear_while_the_recording_is_still_going() {
    // 이 기능의 목적 그 자체다 — **정지하기 전에** 받아 적은 것이 보여야 한다.
    let mut recording = Recording::start("live");
    let live = LiveTranscriber::with_engine(Speaking::new());

    live.begin(recording.path(), model(), Some("ko"));
    assert_eq!(live.state(), LiveState::Running);
    assert!(live.snapshot().is_empty(), "아직 아무 소리도 없다");

    recording.record(32);
    wait_until("첫 문장이 보인다", || !live.snapshot().is_empty());

    recording.record(30);
    wait_until("둘째 문장이 보인다", || live.snapshot().len() >= 2);

    let finished = live.finish().expect("결과가 있어야 한다");
    assert!(finished.progress.segments.len() >= 2);
}

#[test]
fn every_sentence_sits_on_the_recording_timeline() {
    // 창마다 0초부터 매겨진 시각을 옮기지 않으면 전부 0초 근처에 겹친다.
    let mut recording = Recording::start("timeline");
    let live = LiveTranscriber::with_engine(Speaking::new());

    live.begin(recording.path(), model(), Some("ko"));
    recording.record(62);
    wait_until("두 창이 전사된다", || live.snapshot().len() >= 2);

    let finished = live.finish().expect("결과가 있어야 한다");
    assert_eq!(finished.progress.segments[0].start_ms, 0);
    assert_eq!(finished.progress.segments[1].start_ms, 30_000);
}

#[test]
fn stopping_picks_up_the_tail_that_never_filled_a_window() {
    // 지키려는 것: **마지막 몇 초가 사라지지 않는다.**
    let mut recording = Recording::start("tail");
    let live = LiveTranscriber::with_engine(Speaking::new());

    live.begin(recording.path(), model(), Some("ko"));
    recording.record(32);
    wait_until("첫 창이 전사된다", || !live.snapshot().is_empty());

    // 창을 채우지 못하는 꼬리.
    recording.record(5);
    let finished = live.finish().expect("결과가 있어야 한다");

    assert_eq!(finished.progress.segments.len(), 2, "꼬리가 전사돼야 한다");
    assert_eq!(finished.progress.done_frames, 37 * 16_000, "끝까지 나아가야 한다");
}

#[test]
fn an_engine_that_always_fails_gives_up_without_touching_the_recording() {
    // **이 모듈의 어떤 실패도 녹음을 멈추지 않는다.** 실시간 전사만 그만둔다.
    let mut recording = Recording::start("broken");
    let live = LiveTranscriber::with_engine(Broken);

    live.begin(recording.path(), model(), Some("ko"));
    recording.record(40);

    wait_until("포기한다", || matches!(live.state(), LiveState::GaveUp(_)));

    // 녹음은 계속된다 — 파일이 그대로 자란다.
    let before = std::fs::metadata(recording.path()).expect("읽기").len();
    recording.record(10);
    let after = std::fs::metadata(recording.path()).expect("읽기").len();
    assert!(after > before, "녹음이 멈추면 안 된다");

    // 포기한 뒤에도 정지는 결과를 돌려준다 — 받아 적은 것이 없을 뿐이다.
    let finished = live.finish().expect("결과가 있어야 한다");
    assert!(finished.progress.segments.is_empty());
}

#[test]
fn a_missing_file_gives_up_instead_of_killing_the_app() {
    let live = LiveTranscriber::with_engine(Speaking::new());
    let nowhere = unique("nowhere").with_extension("wav");

    live.begin(&nowhere, model(), Some("ko"));
    wait_until("포기한다", || matches!(live.state(), LiveState::GaveUp(_)));

    assert!(live.snapshot().is_empty());
}

#[test]
fn finishing_without_ever_starting_is_not_a_failure() {
    let live = LiveTranscriber::with_engine(Speaking::new());
    assert_eq!(live.state(), LiveState::Idle);
    assert!(live.finish().is_none(), "돌지 않았으면 결과가 없다");
}

#[test]
fn a_second_recording_does_not_inherit_the_first_one() {
    // 지키려는 것: **앞 녹음의 문장이 다음 녹음에 섞이지 않는다.**
    let mut first = Recording::start("first");
    let live = LiveTranscriber::with_engine(Speaking::new());

    live.begin(first.path(), model(), Some("ko"));
    first.record(32);
    wait_until("첫 녹음이 전사된다", || !live.snapshot().is_empty());
    live.finish().expect("결과가 있어야 한다");

    let second = Recording::start("second");
    live.begin(second.path(), model(), Some("ko"));
    assert!(live.snapshot().is_empty(), "새 녹음은 빈 채로 시작해야 한다");
    live.finish();
}
