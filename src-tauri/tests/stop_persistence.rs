//! **정지의 성공은 "파일이 확정됐다"는 뜻이다** (PRODUCT-SPEC §6의 R-002 · Phase 2B 요구사항 5·6).
//!
//! 이 파일이 판정하는 것은 네 조건과 두 어긋난 상태, 그리고 하나의 불변식이다.
//!
//! ```text
//! 1. 파일 경로가 실제로 존재한다
//! 2. 파일 크기가 유효 최소치를 넘는다
//! 3. 포맷을 알고 있다
//! 4. Recording 메타데이터가 영속화됐다 (title · duration · audioPath · audioFormat ·
//!    microphone · createdAt — Phase 1의 저장소를 지나서)
//!
//! audio는 있는데 레코드가 없다   → 파일을 그대로 두고, 그 경로를 담은 실패를 보낸다
//! 레코드는 있는데 audio가 없다   → 감지해서 알린다. 지우지도 고치지도 않는다
//!
//! INV-3 · INV-4 · R-004 — 어떤 실패도 이미 녹음된 audio를 지우지 않는다
//! ```
//!
//! 마이크도 흐르는 시간도 없다. `SampleSource`와 `Clock` 두 자리에만 대체 구현을 넣고,
//! 그 바깥은 제품 코드가 그대로 실행된다 — 파일도 저장소도 진짜다 (§18).
//!
//! 정책 자체는 `docs/ADR-0004-recording-session-lifecycle.md` §10~§13에 적혀 있다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use molt_note_lib::audio::{
    CaptureFormat, OpenCapture, SampleSink, SampleSource, MIN_FINALIZED_BYTES,
};
use molt_note_lib::commands::{
    finish_recording, Recorder, Storage, StoppedRecordingPayload, Transcriber,
};
use molt_note_lib::domain::Failure;
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::clock::Clock;
use molt_note_lib::transcription::{engine_failed, StubEngine};

/// 가짜 마이크는 어떤 키를 줘도 같은 값을 낸다.
const DEVICE_KEY: &str = "0:가짜 마이크";

/// 열린 장치가 알려주는 이름. 레코드의 `microphone`이 이 값이어야 한다.
const DEVICE_LABEL: &str = "가짜 마이크";

/// 한 번에 보내는 샘플 수.
const CHUNK: usize = 512;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-stop-test-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 테스트가 시각을 정하는 시계.
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
#[derive(Clone)]
struct ControlledMicrophone {
    sink: Arc<Mutex<Option<SampleSink>>>,
}

impl ControlledMicrophone {
    fn new() -> Self {
        Self {
            sink: Arc::new(Mutex::new(None)),
        }
    }

    fn speak(&self, value: i16) {
        let sink = self.sink.lock().expect("통로를 빌릴 수 있어야 한다");
        sink.as_ref()
            .expect("장치가 열려 있어야 한다")
            .send(vec![value; CHUNK])
            .expect("샘플을 보낼 수 있어야 한다");
    }
}

impl SampleSource for ControlledMicrophone {
    fn open(&self, _device_key: &str, samples: SampleSink) -> Result<OpenCapture, Failure> {
        *self.sink.lock().expect("통로를 둘 수 있어야 한다") = Some(samples);

        let sink = Arc::clone(&self.sink);
        Ok(OpenCapture {
            device_label: DEVICE_LABEL.to_string(),
            format: CaptureFormat::pcm_16bit(16_000, 1),
            stop: Box::new(move || {
                *sink.lock().expect("통로를 놓을 수 있어야 한다") = None;
                Ok(())
            }),
        })
    }
}

/// 정상적으로 열린 저장소와, 같은 자리에 녹음하는 녹음기, 그리고 전사 실행자.
///
/// 전사 실행자가 여기 있는 이유는 **정지가 그것을 지나기 때문이다** — 자동 전사가 켜져
/// 있으면 저장 직후에 전사가 걸린다. 이 파일의 테스트는 그 설정을 켜지 않으므로(기본값 OFF)
/// 아무 전사도 시작되지 않으며, 그 자체를 판정하는 것은
/// `src-tauri/tests/automatic_transcription.rs`다.
fn recorder_and_storage(
    temp: &TempRoot,
) -> (
    Recorder,
    Storage,
    Transcriber,
    ControlledMicrophone,
    TestClock,
) {
    let app_data_dir = AppDataDirectory::new(temp.path().join("app-data"));
    let storage = Storage::open(&app_data_dir);
    assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

    let microphone = ControlledMicrophone::new();
    let clock = TestClock::default();
    let recorder = Recorder::with_clock(app_data_dir.clone(), microphone.clone(), clock.clone());
    (
        recorder,
        storage,
        idle_transcriber(app_data_dir),
        microphone,
        clock,
    )
}

/// 불릴 일이 없는 전사 실행자.
///
/// 자동 전사가 꺼져 있으므로 엔진까지 가지 않는다. 그래도 실패를 내는 double을 두는 이유는
/// **불렸다면 그 사실이 조용히 지나가지 않게** 하기 위해서다 — 성공을 내는 double을 두면
/// 걸리지 말았어야 할 전사가 성공한 채 남는다.
fn idle_transcriber(app_data_dir: AppDataDirectory) -> Transcriber {
    Transcriber::with_engine(
        app_data_dir,
        StubEngine::failing(engine_failed("이 파일의 정지는 전사를 걸지 않는다")),
    )
}

/// 소리가 들어 있는 녹음 하나를 끝까지 마친다.
fn record_something(
    recorder: &Recorder,
    storage: &Storage,
    transcriber: &Transcriber,
    microphone: &ControlledMicrophone,
    clock: &TestClock,
) -> StoppedRecordingPayload {
    recorder.start(DEVICE_KEY).expect("녹음을 시작할 수 있어야 한다");
    microphone.speak(1_000);
    clock.advance(5_000);
    finish_recording(recorder, storage, transcriber, None).expect("정지가 성공해야 한다")
}

/// 확정된 WAV 파일에 실제로 들어 있는 샘플 전부.
fn samples_in(path: &Path) -> Vec<i16> {
    hound::WavReader::open(path)
        .expect("확정된 파일은 다시 읽을 수 있어야 한다")
        .into_samples::<i16>()
        .map(|sample| sample.expect("샘플을 읽을 수 있어야 한다"))
        .collect()
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

// --- 정지가 성공한다는 것 (R-002) ----------------------------------------------------

#[test]
fn a_successful_stop_means_the_file_is_confirmed_and_the_record_is_saved() {
    // 이 테스트가 R-002 그 자체다. 네 조건이 여기서 한 번에 판정된다.
    let temp = TempRoot::new("confirmed");
    let (recorder, storage, transcriber, microphone, clock) = recorder_and_storage(&temp);

    recorder.start(DEVICE_KEY).expect("녹음을 시작할 수 있어야 한다");
    microphone.speak(1_000);
    clock.advance(3_000);
    recorder.pause().expect("일시정지할 수 있어야 한다");
    microphone.speak(2_000); // 멈춰 있는 동안의 소리다.
    clock.advance(3_600_000); // 한 시간을 멈춰 둔다.
    recorder.resume().expect("재개할 수 있어야 한다");
    microphone.speak(3_000);
    clock.advance(2_000);

    let stopped =
        finish_recording(&recorder, &storage, &transcriber, None).expect("정지가 성공해야 한다");
    let audio = PathBuf::from(&stopped.recording.audio_path);

    // 1. 파일 경로가 실제로 존재한다.
    assert!(audio.is_file(), "레코드가 가리키는 파일이 있어야 한다: {audio:?}");
    assert_eq!(
        stopped.recording.audio_path, stopped.capture.output_path,
        "레코드는 방금 확정된 그 파일을 가리킨다"
    );

    // 2. 파일 크기가 유효 최소치를 넘는다. 보고된 크기는 파일시스템에서 읽은 값이다.
    let on_disk = std::fs::metadata(&audio).expect("파일을 읽을 수 있어야 한다").len();
    assert_eq!(stopped.capture.byte_size, on_disk);
    assert!(on_disk > MIN_FINALIZED_BYTES, "{on_disk} byte");

    // 3. 포맷을 알고 있다. 확정된 파일을 다시 읽어 확인한 것이다.
    assert_eq!(stopped.recording.audio_format, "wav");
    assert_eq!(stopped.capture.container, "WAV");
    assert_eq!(stopped.capture.sample_rate_hz, 16_000);
    assert_eq!(samples_in(&audio), [vec![1_000; CHUNK], vec![3_000; CHUNK]].concat());

    // 4. 메타데이터가 Phase 1의 저장소에 남았다 — 여섯 값이 전부.
    assert!(!stopped.recording.title.trim().is_empty(), "제목이 있다");
    assert_eq!(stopped.recording.duration_ms, 5_000, "일시정지 구간이 빠져 있다");
    assert_eq!(stopped.recording.duration_label, "0:05");
    assert_eq!(stopped.recording.microphone.as_deref(), Some(DEVICE_LABEL));
    assert!(!stopped.recording.created_at.is_empty(), "저장 시각이 있다");

    // 그리고 그것은 이 앱을 다시 물어봐도 나오는 값이다 — 응답만 그럴듯한 것이 아니다.
    let listed = storage.list_recordings().expect("목록을 읽을 수 있어야 한다");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], stopped.recording);
    let loaded = storage
        .recording(&stopped.recording.id)
        .expect("단건 조회가 성공해야 한다")
        .expect("저장된 녹음이 있어야 한다");
    assert_eq!(loaded, stopped.recording);
}

#[test]
fn a_stop_that_captured_no_sound_is_a_failure_the_user_sees() {
    // 파일은 만들어졌지만 소리가 없다. "resolve됐다"를 성공으로 삼지 않는다.
    let temp = TempRoot::new("silent");
    let (recorder, storage, transcriber, _microphone, clock) = recorder_and_storage(&temp);
    let recordings_dir = AppDataDirectory::new(temp.path().join("app-data")).recordings_dir();

    recorder.start(DEVICE_KEY).expect("녹음을 시작할 수 있어야 한다");
    clock.advance(1_000);
    let failure = finish_recording(&recorder, &storage, &transcriber, None)
        .expect_err("빈 녹음은 성공이 아니다");

    let written = wav_files(&recordings_dir);
    assert_eq!(written.len(), 1, "만들어진 파일은 그대로 있다: {written:?}");
    assert!(
        failure.message.contains(&written[0].display().to_string()),
        "어느 파일인지 사용자에게 보인다: {}",
        failure.message
    );
    assert!(!failure.source_data_safe, "방금 녹음한 것을 신뢰할 수 없다");
    assert!(
        storage.list_recordings().expect("목록을 읽는다").is_empty(),
        "확인되지 않은 파일을 가리키는 레코드를 만들지 않는다"
    );
}

#[test]
fn a_title_the_user_typed_is_saved_and_a_blank_one_becomes_one_made_from_the_saved_time() {
    let temp = TempRoot::new("title");
    let (recorder, storage, transcriber, microphone, clock) = recorder_and_storage(&temp);

    recorder.start(DEVICE_KEY).expect("시작");
    microphone.speak(1_000);
    clock.advance(1_000);
    let named = finish_recording(&recorder, &storage, &transcriber, Some("  3DGS Study #04  "))
        .expect("정지가 성공해야 한다");

    recorder.start(DEVICE_KEY).expect("두 번째 시작");
    microphone.speak(2_000);
    clock.advance(1_000);
    let unnamed = finish_recording(&recorder, &storage, &transcriber, Some("   "))
        .expect("정지가 성공해야 한다");

    assert_eq!(named.recording.title, "3DGS Study #04", "입력한 제목이 저장된다");
    assert!(
        unnamed.recording.title.contains(&unnamed.recording.created_at[0..10]),
        "제목 없는 녹음도 저장 시각으로 구분된다: {}",
        unnamed.recording.title
    );
    assert_ne!(named.recording.id, unnamed.recording.id);
}

// --- 어긋난 상태 1: audio는 있는데 레코드가 없다 -------------------------------------

#[test]
fn a_recording_that_cannot_be_saved_keeps_its_audio_and_tells_the_user_where_it_is() {
    // 파일은 확정됐는데 저장소가 죽어 있다. 이때 파일을 "되돌리는" 보상은 하지 않는다.
    let temp = TempRoot::new("unlisted");
    let app_data_dir = AppDataDirectory::new(temp.path().join("audio"));
    let recordings_dir = app_data_dir.recordings_dir();

    // 저장소만 열리지 않게 만든다 — 디렉터리가 있어야 할 자리에 파일이 있다.
    let blocked = temp.path().join("blocked");
    std::fs::write(&blocked, "디렉터리가 아니다").expect("사전 조건: 파일을 둔다");
    let storage = Storage::open(&AppDataDirectory::new(&blocked));
    assert!(storage.failure().is_some(), "사전 조건: 저장소가 열리지 않는다");

    let microphone = ControlledMicrophone::new();
    let clock = TestClock::default();
    let recorder = Recorder::with_clock(app_data_dir.clone(), microphone.clone(), clock.clone());
    let transcriber = idle_transcriber(app_data_dir);

    recorder.start(DEVICE_KEY).expect("녹음을 시작할 수 있어야 한다");
    microphone.speak(1_000);
    clock.advance(4_000);
    let failure = finish_recording(&recorder, &storage, &transcriber, None)
        .expect_err("레코드를 남기지 못했으면 정지는 성공이 아니다");

    // 파일은 남아 있고, 내용도 그대로다.
    let written = wav_files(&recordings_dir);
    assert_eq!(written.len(), 1, "확정된 파일이 지워졌다: {written:?}");
    assert_eq!(samples_in(&written[0]), vec![1_000; CHUNK], "녹음된 소리가 그대로다");

    // 그리고 사용자는 그 파일이 어디 있는지 알 수 있다.
    assert!(
        failure.message.contains(&written[0].display().to_string()),
        "실패가 파일의 자리를 말해야 한다: {}",
        failure.message
    );
    assert!(
        failure.message.contains("목록"),
        "무슨 일이 일어났는지 먼저 말한다: {}",
        failure.message
    );
    assert!(
        failure.source_data_safe,
        "저장에 실패했을 뿐 녹음된 것은 온전하다"
    );
}

// --- 어긋난 상태 2: 레코드는 있는데 audio가 없다 -------------------------------------

#[test]
fn a_record_whose_audio_is_gone_is_detected_and_nothing_is_removed_or_repaired() {
    let temp = TempRoot::new("missing-audio");
    let (recorder, storage, transcriber, microphone, clock) = recorder_and_storage(&temp);

    let stopped = record_something(&recorder, &storage, &transcriber, &microphone, &clock);
    assert!(
        storage.missing_audio().expect("감지를 부를 수 있어야 한다").is_empty(),
        "방금 확정한 녹음은 어긋난 상태가 아니다"
    );

    // 파일이 앱 밖에서 사라진다. 앱 안에는 이렇게 만드는 경로가 없다 (INV-4).
    std::fs::remove_file(&stopped.recording.audio_path).expect("사전 조건: 파일을 치운다");

    let missing = storage.missing_audio().expect("감지를 부를 수 있어야 한다");
    assert_eq!(missing.len(), 1, "레코드는 있는데 파일이 없는 상태가 보인다");
    assert_eq!(missing[0].recording_id, stopped.recording.id);
    assert_eq!(missing[0].audio_path, stopped.recording.audio_path);
    assert_eq!(missing[0].title, stopped.recording.title);
    assert_eq!(missing[0].created_at, stopped.recording.created_at);

    // 감지는 감지일 뿐이다 — 레코드를 지우지도, 파일을 만들지도 않는다.
    let listed = storage.list_recordings().expect("목록을 읽을 수 있어야 한다");
    assert_eq!(listed.len(), 1, "감지가 레코드를 지웠다");
    assert_eq!(listed[0], stopped.recording, "감지가 레코드를 고쳤다");
    assert!(
        !Path::new(&stopped.recording.audio_path).exists(),
        "감지가 없는 파일을 만들어 냈다"
    );
    assert_eq!(
        storage.missing_audio().expect("다시 부를 수 있어야 한다").len(),
        1,
        "몇 번을 물어도 답이 같다"
    );
}

// --- 어떤 실패도 audio를 지우지 않는다 (INV-3 · INV-4 · R-004) -----------------------

#[test]
fn no_failure_on_any_path_removes_audio_that_was_already_written() {
    let temp = TempRoot::new("never-deletes");
    let (recorder, storage, transcriber, microphone, clock) = recorder_and_storage(&temp);
    let recordings_dir = AppDataDirectory::new(temp.path().join("app-data")).recordings_dir();

    // 지켜야 할 녹음 하나를 먼저 만들어 둔다.
    let kept = record_something(&recorder, &storage, &transcriber, &microphone, &clock);
    let kept_samples = samples_in(Path::new(&kept.recording.audio_path));

    // 1. 상태에 맞지 않는 요청들 — 거절이며 파일을 건드리지 않는다.
    recorder.pause().expect_err("녹음 중이 아니다");
    recorder.resume().expect_err("일시정지 상태가 아니다");
    finish_recording(&recorder, &storage, &transcriber, None).expect_err("정지할 녹음이 없다");

    // 2. 소리 없는 녹음 — 확인에 실패하지만 만들어진 파일은 남는다.
    recorder.start(DEVICE_KEY).expect("시작");
    clock.advance(1_000);
    finish_recording(&recorder, &storage, &transcriber, None).expect_err("빈 녹음은 성공이 아니다");

    // 3. 이미 녹음 중인데 또 시작 — 거절이며 진행 중인 녹음도 파일도 그대로다.
    recorder.start(DEVICE_KEY).expect("시작");
    microphone.speak(7_000);
    clock.advance(1_000);
    recorder.start(DEVICE_KEY).expect_err("이미 녹음 중이다");
    let second =
        finish_recording(&recorder, &storage, &transcriber, None).expect("정지가 성공해야 한다");

    // 앞선 녹음은 파일도 레코드도 그대로다.
    assert_eq!(
        samples_in(Path::new(&kept.recording.audio_path)),
        kept_samples,
        "앞선 녹음의 소리가 바뀌었다"
    );
    assert_eq!(wav_files(&recordings_dir).len(), 3, "만들어진 파일은 하나도 지워지지 않는다");
    let listed = storage.list_recordings().expect("목록을 읽을 수 있어야 한다");
    assert_eq!(listed.len(), 2, "확인된 녹음만 레코드가 된다");
    assert!(listed.iter().any(|recording| recording.id == kept.recording.id));
    assert!(listed.iter().any(|recording| recording.id == second.recording.id));
    assert!(
        storage.missing_audio().expect("감지를 부를 수 있어야 한다").is_empty(),
        "레코드가 가리키는 파일은 전부 제자리에 있다"
    );
}
