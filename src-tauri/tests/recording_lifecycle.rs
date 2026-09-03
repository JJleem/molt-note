//! Record → Pause → Resume → Stop 전체 경로가 **마이크 없이, 시간을 흘려보내지 않고**
//! 동작하는지 본다 (Phase 2B 요구사항 4 · 8 · R-001 · §18).
//!
//! 이 파일에는 마이크도, 마이크 권한도, 흐르는 시간도 없다. 두 자리에만 대체 구현을 넣는다.
//!
//! ```text
//! SampleSource   샘플을 언제 보낼지 테스트가 정한다   (실제 장치 자리)
//! Clock          지금이 몇 시인지 테스트가 정한다     (흐르는 시간 자리)
//! ```
//!
//! 그 두 자리 바깥은 **제품 코드가 그대로 실행된다** — 상태 기계도, WAV writer도, 파일
//! 확정도, 크기 읽기도 진짜다. 그래서 여기서 확인하는 것은 흉내가 아니라 실제 파일이다.
//!
//! 이 파일의 핵심 주장 하나: **일시정지 구간에 들어온 샘플은 파일에 없다.** 크기가 아니라
//! 확정된 WAV를 다시 읽어 샘플 값을 직접 본다.
//!
//! 실제 장치에서만 알 수 있는 것(권한 프롬프트 · 녹음된 소리가 들리는가 · 실제 컨테이너와
//! 코덱)은 사람이 확인하며, 이 테스트가 그것을 대신 판정하지 않는다 (ADR-0003 §12).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use molt_note_lib::audio::{CaptureFormat, OpenCapture, SampleSink, SampleSource};
use molt_note_lib::commands::Recorder;
use molt_note_lib::domain::{format_duration_ms, Failure, FailureKind};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::clock::Clock;

/// 테스트가 고른 장치 키. 가짜 마이크는 어떤 키를 줘도 같은 값을 낸다.
const DEVICE_KEY: &str = "0:가짜 마이크";

/// 한 번에 보내는 샘플 수. 파일에 실제로 쓰였는지 보기에 충분한 크기면 된다.
const CHUNK: usize = 512;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 앱 데이터 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-lifecycle-test-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        Self(std::env::temp_dir().join(unique))
    }

    fn app_data_dir(&self) -> AppDataDirectory {
        AppDataDirectory::new(self.0.join("app-data"))
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 테스트가 시각을 정하는 시계.
///
/// 시간이 실제로 흐르지 않으므로 **1시간짜리 일시정지도 즉시 검증된다.**
#[derive(Clone, Default)]
struct TestClock {
    now_ms: Arc<AtomicI64>,
}

impl TestClock {
    fn advance(&self, ms: i64) {
        self.now_ms.fetch_add(ms, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now_ms(&self) -> i64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

/// 테스트가 "지금 말한다"고 정할 때만 샘플을 보내는 마이크.
///
/// 실제 마이크처럼 **일시정지 중에도 계속 보낼 수 있다.** 그 샘플이 파일에 도달하지 않는
/// 것이 제품 코드의 일이며, 그것을 확인하는 것이 이 파일의 목적이다.
#[derive(Clone)]
struct ControlledMicrophone {
    inner: Arc<Inner>,
}

struct Inner {
    label: String,
    format: CaptureFormat,
    /// 열려 있는 동안의 통로. 정지하면 실제 장치의 콜백처럼 사라진다.
    sink: Mutex<Option<SampleSink>>,
    /// 장치를 연 횟수. resume이 장치를 다시 열지 않는다는 것을 여기서 본다.
    opens: AtomicUsize,
}

impl ControlledMicrophone {
    fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                label: "가짜 마이크".to_string(),
                format: CaptureFormat::pcm_16bit(16_000, 1),
                sink: Mutex::new(None),
                opens: AtomicUsize::new(0),
            }),
        }
    }

    /// 같은 값의 샘플 한 덩어리를 지금 보낸다.
    ///
    /// 샘플과 일시정지 표시는 같은 통로를 지나므로, 이 호출이 끝난 뒤에 보낸 pause는
    /// **이 샘플 뒤에** 놓인다. 그래서 순서가 흔들리지 않는다.
    fn speak(&self, value: i16) {
        let sink = self.inner.sink.lock().expect("통로를 빌릴 수 있어야 한다");
        sink.as_ref()
            .expect("장치가 열려 있어야 한다")
            .send(vec![value; CHUNK])
            .expect("샘플을 보낼 수 있어야 한다");
    }

    fn opens(&self) -> usize {
        self.inner.opens.load(Ordering::SeqCst)
    }
}

impl SampleSource for ControlledMicrophone {
    fn open(&self, _device_key: &str, samples: SampleSink) -> Result<OpenCapture, Failure> {
        self.inner.opens.fetch_add(1, Ordering::SeqCst);
        *self.inner.sink.lock().expect("통로를 둘 수 있어야 한다") = Some(samples);

        let inner = Arc::clone(&self.inner);
        Ok(OpenCapture {
            device_label: self.inner.label.clone(),
            format: self.inner.format,
            stop: Box::new(move || {
                // 실제 장치도 정지하면 콜백이 사라지고 통로가 닫힌다.
                *inner.sink.lock().expect("통로를 놓을 수 있어야 한다") = None;
                Ok(())
            }),
        })
    }
}

/// 마이크 하나와 시계 하나를 가진 녹음기.
fn recorder_with(
    temp: &TempRoot,
) -> (Recorder, ControlledMicrophone, TestClock, AppDataDirectory) {
    let microphone = ControlledMicrophone::new();
    let clock = TestClock::default();
    let app_data_dir = temp.app_data_dir();
    let recorder = Recorder::with_clock(app_data_dir.clone(), microphone.clone(), clock.clone());
    (recorder, microphone, clock, app_data_dir)
}

/// 디렉터리 안의 `.wav` 파일 전부. 없으면 빈 목록이다.
fn wav_files(directory: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match std::fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("wav"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

/// 확정된 WAV 파일에 실제로 들어 있는 샘플 전부.
fn samples_in(path: &Path) -> Vec<i16> {
    hound::WavReader::open(path)
        .expect("확정된 파일은 다시 읽을 수 있어야 한다")
        .into_samples::<i16>()
        .map(|sample| sample.expect("샘플을 읽을 수 있어야 한다"))
        .collect()
}

#[test]
fn the_whole_path_runs_start_pause_resume_stop_and_finishes_one_file() {
    // 이 테스트가 Task의 요구 그 자체다 — 전체 경로가 마이크 없이 지나가고,
    // 일시정지 구간의 샘플이 파일에 없으며, 결과가 파일 하나로 확정된다.
    let temp = TempRoot::new("whole-path");
    let (recorder, microphone, clock, app_data_dir) = recorder_with(&temp);

    recorder.start(DEVICE_KEY).expect("녹음을 시작할 수 있어야 한다");
    microphone.speak(1_000);
    clock.advance(3_000);

    recorder.pause().expect("일시정지할 수 있어야 한다");
    microphone.speak(2_000); // 멈춰 있는 동안 들어온 소리다. 파일에 들어가면 안 된다.
    clock.advance(3_600_000); // 한 시간을 멈춰 둔다.

    recorder.resume().expect("재개할 수 있어야 한다");
    microphone.speak(3_000);
    clock.advance(2_000);

    let report = recorder.stop().expect("정지할 수 있어야 한다");

    // 1. 파일은 하나다. resume이 새 파일을 만들지 않았다.
    let written = wav_files(&app_data_dir.recordings_dir());
    assert_eq!(written.len(), 1, "결과는 파일 하나여야 한다: {written:?}");
    assert_eq!(PathBuf::from(&report.output_path), written[0]);
    assert_eq!(
        microphone.opens(),
        1,
        "일시정지는 장치를 닫지 않고, 재개는 장치를 다시 열지 않는다"
    );

    // 2. 일시정지 구간의 샘플이 파일에 없다. 크기가 아니라 내용을 본다.
    let samples = samples_in(&written[0]);
    assert_eq!(samples, [vec![1_000; CHUNK], vec![3_000; CHUNK]].concat());
    assert!(!samples.contains(&2_000), "일시정지 구간이 파일에 들어갔다");

    // 3. 길이에서도 일시정지 구간이 빠져 있다. 벽시계로는 한 시간이 지났다.
    assert_eq!(report.duration_ms, 5_000);
    assert_eq!(report.duration_label, "0:05");
    assert_eq!(report.duration_label, format_duration_ms(report.duration_ms));

    // 4. 보고된 크기는 확정된 파일에서 읽은 값이다.
    let on_disk = std::fs::metadata(&written[0])
        .expect("파일을 읽을 수 있어야 한다")
        .len();
    assert_eq!(report.byte_size, on_disk);
    assert!(report.byte_size >= (2 * CHUNK * 2) as u64, "{}", report.byte_size);
}

#[test]
fn the_status_answer_carries_the_state_the_elapsed_ms_and_a_label_made_by_rust() {
    // 화면이 길이를 계산하지 않게 하는 것이 이 값의 목적이다 (tests/screen-boundary.test.ts).
    let temp = TempRoot::new("status");
    let (recorder, microphone, clock, _) = recorder_with(&temp);

    // 아직 시작하지 않았다. 특별한 빈 값이 아니라 idle session의 답이 온다.
    let idle = recorder.status().expect("상태를 물어볼 수 있어야 한다");
    assert_eq!(idle.state, "idle");
    assert_eq!(idle.elapsed_ms, 0);
    assert_eq!(idle.elapsed_label, "0:00");

    recorder.start(DEVICE_KEY).expect("시작");
    clock.advance(7_000);

    let recording = recorder.status().expect("상태를 물어본다");
    assert_eq!(recording.state, "recording");
    assert_eq!(recording.elapsed_ms, 7_000);
    assert_eq!(recording.elapsed_label, "0:07");
    assert_eq!(recording.elapsed_label, format_duration_ms(7_000));

    // 멈춰 있는 동안에는 시간이 아무리 흘러도 경과 시간이 자라지 않는다.
    recorder.pause().expect("일시정지");
    clock.advance(3_600_000);

    let paused = recorder.status().expect("상태를 물어본다");
    assert_eq!(paused.state, "paused");
    assert_eq!(paused.elapsed_ms, 7_000, "멈춰 있는 동안은 자라지 않는다");
    assert_eq!(paused.elapsed_label, "0:07");

    recorder.resume().expect("재개");
    clock.advance(53_000);
    let resumed = recorder.status().expect("상태를 물어본다");
    assert_eq!(resumed.state, "recording");
    assert_eq!(resumed.elapsed_ms, 60_000);
    assert_eq!(resumed.elapsed_label, "1:00", "규칙은 Rust 한 곳에만 있다");

    microphone.speak(1_000);
    recorder.stop().expect("정지");

    // 끝난 session은 남지 않는다. 다음 녹음은 처음부터 시작한다.
    let after = recorder.status().expect("상태를 물어본다");
    assert_eq!(after.state, "idle");
    assert_eq!(after.elapsed_ms, 0);
    assert_eq!(after.elapsed_label, "0:00");
}

#[test]
fn a_recording_stopped_while_paused_keeps_exactly_what_was_recorded() {
    // 멈춰 둔 녹음을 그대로 끝내는 것은 정상적인 사용이다.
    let temp = TempRoot::new("stopped-while-paused");
    let (recorder, microphone, clock, app_data_dir) = recorder_with(&temp);

    recorder.start(DEVICE_KEY).expect("시작");
    microphone.speak(1_000);
    clock.advance(4_000);
    recorder.pause().expect("일시정지");
    microphone.speak(2_000);
    clock.advance(600_000);

    let report = recorder.stop().expect("일시정지 상태에서도 정지할 수 있어야 한다");

    assert_eq!(report.duration_ms, 4_000);
    assert_eq!(report.duration_label, "0:04");
    let written = wav_files(&app_data_dir.recordings_dir());
    assert_eq!(written.len(), 1);
    assert_eq!(samples_in(&written[0]), vec![1_000; CHUNK]);
    assert!(report.byte_size > 0, "멈춰 둔 녹음도 파일로 확정된다");
}

#[test]
fn a_request_that_does_not_fit_the_current_state_is_refused_without_touching_the_recording() {
    // 잘못된 요청 하나가 진행 중인 녹음을 망가뜨리지 않는다 (R-001 · §13).
    let temp = TempRoot::new("refused");
    let (recorder, microphone, clock, app_data_dir) = recorder_with(&temp);

    // 시작하지 않았는데 일시정지·재개·정지를 요청한다.
    for failure in [
        recorder.pause().expect_err("시작하지 않은 녹음은 멈출 수 없다"),
        recorder.resume().expect_err("시작하지 않은 녹음은 재개할 수 없다"),
        recorder.stop().expect_err("시작하지 않은 녹음은 정지할 수 없다"),
    ] {
        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.message.is_empty(), "보여줄 문장이 있어야 한다");
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다");
        assert!(!failure.retryable, "같은 상태에서 다시 보내도 결과가 같다");
    }
    assert!(
        wav_files(&app_data_dir.recordings_dir()).is_empty(),
        "거절된 요청은 파일을 만들지 않는다"
    );

    recorder.start(DEVICE_KEY).expect("시작");
    microphone.speak(1_000);
    clock.advance(1_000);

    // 녹음 중인데 재개를 요청한다. 거절이며 상태도 길이도 그대로다.
    let not_paused = recorder.resume().expect_err("일시정지 상태가 아니다");
    assert_eq!(not_paused.kind, FailureKind::InvalidInput);
    assert_eq!(recorder.status().expect("상태를 물어본다").state, "recording");

    // 일시정지 상태인데 한 번 더 일시정지를 요청한다. 마찬가지로 거절이다.
    recorder.pause().expect("일시정지");
    let already_paused = recorder.pause().expect_err("이미 일시정지 상태다");
    assert_eq!(already_paused.kind, FailureKind::InvalidInput);
    let paused = recorder.status().expect("상태를 물어본다");
    assert_eq!(paused.state, "paused");
    assert_eq!(paused.elapsed_ms, 1_000, "거절은 길이도 바꾸지 않는다");

    // 거절당한 뒤에도 녹음은 멀쩡히 이어지고 정상적으로 끝난다.
    recorder.resume().expect("재개");
    microphone.speak(3_000);
    clock.advance(2_000);
    let report = recorder.stop().expect("정지");

    assert_eq!(report.duration_ms, 3_000);
    let written = wav_files(&app_data_dir.recordings_dir());
    assert_eq!(written.len(), 1);
    assert_eq!(
        samples_in(&written[0]),
        [vec![1_000; CHUNK], vec![3_000; CHUNK]].concat()
    );
}

#[test]
fn the_next_recording_starts_from_a_new_session_and_a_new_file() {
    // 정지한 session을 다시 쓰지 않는다 — 두 번째 녹음은 처음부터 시작한다.
    let temp = TempRoot::new("next-session");
    let (recorder, microphone, clock, app_data_dir) = recorder_with(&temp);

    recorder.start(DEVICE_KEY).expect("첫 녹음 시작");
    microphone.speak(1_000);
    clock.advance(5_000);
    let first = recorder.stop().expect("첫 녹음 정지");

    recorder.start(DEVICE_KEY).expect("두 번째 녹음 시작");
    microphone.speak(2_000);
    clock.advance(1_000);
    let second = recorder.stop().expect("두 번째 녹음 정지");

    assert_eq!(first.duration_ms, 5_000);
    assert_eq!(second.duration_ms, 1_000, "앞선 녹음의 길이를 물려받지 않는다");
    assert_ne!(first.output_path, second.output_path);
    assert_eq!(wav_files(&app_data_dir.recordings_dir()).len(), 2);
    assert_eq!(samples_in(&PathBuf::from(&first.output_path)), vec![1_000; CHUNK]);
    assert_eq!(samples_in(&PathBuf::from(&second.output_path)), vec![2_000; CHUNK]);
}
