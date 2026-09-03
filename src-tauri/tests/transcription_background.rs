//! 전사가 **UI와 IPC를 막지 않는다**는 계약 (`phase-prompt/03` 요구 3 · TASK-028).
//!
//! 여기서 판정하는 것은 전사 결과가 아니라 **동시성**이다. 그래서 엔진 자리에는 결과를 내기
//! 전에 멈추는 test double이 선다 ([`GatedEngine`]) — 테스트가 풀어 주기 전까지 전사 안에
//! 머무르므로, 그 사이에 다른 호출이 실제로 답하는지 확인할 수 있다. 엔진이 먼저 끝나 버리면
//! 그 확인은 "순서대로 불렀다"는 말밖에 되지 않는다.
//!
//! ```text
//! start_transcription ──→ 배경 스레드 ──→ GatedEngine (여기서 멈춘다)
//!                                              │
//!        이 구간에서: transcription_status 즉시 답한다
//!                     list_recordings   즉시 답한다
//!                     같은 녹음 재시작   거부된다
//!                                              │
//!                     controls.release() ──────┘ → done
//! ```
//!
//! **실제 whisper도 모델 파일도 요구하지 않는다** (§18 · `phase-prompt/03` 요구 8 · 9).
//! "모델"은 임시 디렉터리에 둔 몇 바이트짜리 파일이고, 오디오는 0.1초짜리 WAV다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다. Tauri 런타임도 창도 하드웨어도 없다.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use molt_note_lib::commands::{Storage, Transcriber, TranscriptionStatusPayload};
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{
    Failure, FailureKind, ProcessingStatus, Recording, RecordingId, Settings, Transcript,
};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::transcription::{
    engine_failed, ensure_usable, ModelFile, RawSegment, RawTranscription, TranscriptionEngine,
    TranscriptionInput,
};

/// 모델 디렉터리에 두는 자리표시자 파일의 이름. 실제 모델이 아니다.
const MODEL_FILE: &str = "ggml-base.bin";

/// 배경 스레드가 끝나기를 기다리는 한계. 넘으면 테스트가 실패한다 — 매달리지 않는다.
const FINISH_TIMEOUT: Duration = Duration::from_secs(30);

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 멈춰 있는 엔진 -------------------------------------------------------------------

/// 전사 도중에 **멈춰 서는** 엔진.
///
/// 실제 whisper가 오래 걸린다는 사실을 그대로 흉내 낸다 — 다만 걸리는 시간이 테스트의
/// 통제 아래 있다. [`Controls::wait_until_inside`]가 돌아온 시점에 이 엔진은 확실히
/// `transcribe` 안에 있고, [`Controls::release`] 전까지 거기서 나오지 않는다.
///
/// **계약을 우회하지 않는다** — 실제 엔진과 똑같이 [`ensure_usable`]을 통과한 값만 돌려준다.
#[derive(Clone)]
struct GatedEngine {
    inner: Arc<Gate>,
}

struct Gate {
    /// 전사에 들어갔다는 신호를 보내는 자리.
    entered: Mutex<Sender<()>>,
    /// 풀어 줄 때까지 여기서 기다린다.
    release: Mutex<Receiver<()>>,
    /// 무엇을 받아 몇 번 불렸는지 (frames · model id).
    calls: Mutex<Vec<(usize, String)>>,
    /// 풀려난 뒤에 돌려줄 답. 몇 번 불려도 같은 답이다.
    answer: Result<RawTranscription, Failure>,
}

/// 멈춰 있는 엔진을 조종하는 손잡이. 테스트 스레드가 들고 있는다.
struct Controls {
    entered: Receiver<()>,
    release: Sender<()>,
}

impl GatedEngine {
    fn new(answer: Result<RawTranscription, Failure>) -> (Self, Controls) {
        let (entered_tx, entered_rx) = channel();
        let (release_tx, release_rx) = channel();

        let engine = Self {
            inner: Arc::new(Gate {
                entered: Mutex::new(entered_tx),
                release: Mutex::new(release_rx),
                calls: Mutex::new(Vec::new()),
                answer,
            }),
        };
        let controls = Controls {
            entered: entered_rx,
            release: release_tx,
        };

        (engine, controls)
    }

    fn call_count(&self) -> usize {
        self.inner.calls.lock().expect("호출 기록을 잠근다").len()
    }

    fn calls(&self) -> Vec<(usize, String)> {
        self.inner.calls.lock().expect("호출 기록을 잠근다").clone()
    }
}

impl Controls {
    /// 엔진이 전사 안으로 들어갈 때까지 기다린다.
    ///
    /// **이 호출이 돌아온 뒤부터가 관찰 구간이다** — 엔진은 아직 결과를 내지 않았다.
    fn wait_until_inside(&self) {
        self.entered
            .recv_timeout(FINISH_TIMEOUT)
            .expect("엔진이 전사에 들어가야 한다");
    }

    /// 엔진을 풀어 준다. 들어가기 전에 미리 풀어 두면 멈추지 않고 지나간다.
    fn release(&self) {
        self.release.send(()).expect("엔진이 아직 살아 있어야 한다");
    }
}

impl TranscriptionEngine for GatedEngine {
    fn engine_id(&self) -> String {
        // 실제 엔진의 식별자와 겹치지 않는다. 이 값이 저장된 Transcript는 테스트가 만든 것이다.
        "gated-transcription-engine".to_owned()
    }

    fn transcribe(
        &self,
        input: &TranscriptionInput,
        model: &ModelFile,
    ) -> Result<RawTranscription, Failure> {
        self.inner
            .calls
            .lock()
            .expect("호출 기록을 잠근다")
            .push((input.frames(), model.id().to_owned()));

        self.inner
            .entered
            .lock()
            .expect("신호 통로를 잠근다")
            .send(())
            .expect("테스트가 신호를 받고 있어야 한다");
        self.inner
            .release
            .lock()
            .expect("신호 통로를 잠근다")
            .recv_timeout(FINISH_TIMEOUT)
            .expect("테스트가 엔진을 풀어 줘야 한다");

        self.inner.answer.clone().and_then(ensure_usable)
    }
}

// --- 자리 --------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-transcription-background-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
        Self(path)
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 전사 한 건을 걸 수 있는 앱 데이터 디렉터리 하나 — DB · 녹음 파일 · 자리표시자 모델.
struct Fixture {
    /// Drop 시 임시 디렉터리를 지운다.
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    recording_id: String,
    audio_path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.0.join("app-data"));
        app_data_dir.ensure().expect("사전 조건: 앱 데이터 디렉터리");

        let models_dir = app_data_dir
            .ensure_models_dir()
            .expect("사전 조건: 모델 디렉터리");
        fs::write(models_dir.join(MODEL_FILE), b"not a real model")
            .expect("사전 조건: 자리표시자 모델을 둔다");

        let recordings_dir = app_data_dir
            .ensure_recordings_dir()
            .expect("사전 조건: 녹음 디렉터리");
        let audio_path = recordings_dir.join("recording.wav");
        write_silence_wav(&audio_path);

        let fixture = Self {
            _root: root,
            app_data_dir,
            recording_id: "rec-background".to_owned(),
            audio_path,
        };
        fixture.insert_recording(&fixture.recording_id, "3DGS Study #04");
        fixture
    }

    /// 녹음 레코드 하나를 저장한다. 오디오는 fixture가 만든 WAV 하나를 함께 가리킨다.
    fn insert_recording(&self, id: &str, title: &str) {
        let connection = self.connection();
        store::insert_recording(
            &connection,
            &Recording {
                id: RecordingId::new(id),
                title: title.to_owned(),
                created_at: "2026-09-03T10:00:00.000Z".to_owned(),
                updated_at: "2026-09-03T10:00:00.000Z".to_owned(),
                duration_ms: 100,
                audio_path: self
                    .audio_path
                    .to_str()
                    .expect("경로 문자열")
                    .to_owned(),
                audio_format: "wav".to_owned(),
                microphone: Some("MacBook Pro Microphone".to_owned()),
                current_transcript_id: None,
                transcription_status: ProcessingStatus::None,
                ai_status: ProcessingStatus::None,
                notion_status: ProcessingStatus::None,
            },
        )
        .expect("사전 조건: 녹음 레코드를 저장한다");
    }

    /// 모델이 지정된 전사 실행자. 실제 앱이 지나는 경로와 같고 엔진만 다르다.
    ///
    /// **모델은 설정에서 온다** (TASK-029 · ADR-0007 §8.2) — 여기서 하는 일은 사용자가
    /// 설정 화면에서 고른 것과 같은 값을 저장해 두는 것뿐이고, 그 값을 읽어 쓰는 것은
    /// 전사 경로 자체다.
    fn transcriber(&self, engine: GatedEngine) -> Transcriber {
        settings::save(
            &self.connection(),
            &Settings {
                transcription_model: Some(MODEL_FILE.to_owned()),
                ..Settings::DEFAULT
            },
        )
        .expect("사전 조건: 쓸 모델을 설정에 저장한다");

        Transcriber::with_engine(self.app_data_dir.clone(), engine)
    }

    /// 앱이 들고 있는 것과 같은 저장소. **전사와 별개의 연결이다.**
    fn storage(&self) -> Storage {
        let storage = Storage::open(&self.app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");
        storage
    }

    fn connection(&self) -> rusqlite::Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    fn recording(&self, id: &str) -> Recording {
        store::load_recording(&self.connection(), &RecordingId::new(id))
            .expect("녹음을 읽을 수 있어야 한다")
            .expect("녹음이 남아 있어야 한다")
    }

    fn transcripts(&self, id: &str) -> Vec<Transcript> {
        store::list_transcripts(&self.connection(), &RecordingId::new(id))
            .expect("Transcript 목록을 읽을 수 있어야 한다")
    }
}

/// 0.1초짜리 16 kHz mono PCM16 WAV. 짧고 결정론적인 fixture다 (`phase-prompt/03` 요구 8).
fn write_silence_wav(path: &Path) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("사전 조건: WAV를 만든다");
    for index in 0..1_600_i32 {
        writer
            .write_sample((index % 128) as i16)
            .expect("사전 조건: 샘플을 쓴다");
    }
    writer.finalize().expect("사전 조건: WAV를 닫는다");
}

/// 엔진이 냈다고 가정하는 원시 출력. **센티초다** (ADR-0007 §10).
fn spoken() -> RawTranscription {
    RawTranscription {
        language: Some("ko".to_owned()),
        segments: vec![RawSegment {
            start_centiseconds: 13_400,
            end_centiseconds: 14_100,
            text: Some(" 그러면 이번에는 PLY 먼저 변환하고".to_owned()),
        }],
    }
}

/// 전사가 끝날 때까지 상태를 물어보며 기다린다. 끝난 상태를 돌려준다.
fn wait_for_finish(transcriber: &Transcriber) -> TranscriptionStatusPayload {
    let deadline = Instant::now() + FINISH_TIMEOUT;
    loop {
        let status = transcriber.status().expect("상태를 물어볼 수 있어야 한다");
        if status.state != "running" {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "전사가 끝나기를 기다리다 시간이 지났다"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

// --- 전사가 다른 호출을 붙잡지 않는다 --------------------------------------------------

#[test]
fn a_status_query_answers_while_the_engine_is_still_transcribing() {
    let fixture = Fixture::new("status-while-running");
    let (engine, controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = fixture.transcriber(engine.clone());

    let accepted = transcriber
        .start(&fixture.recording_id)
        .expect("전사를 시작할 수 있어야 한다");
    assert_eq!(accepted.state, "running", "시작은 접수 사실을 돌려준다");

    // 여기부터가 관찰 구간이다 — 엔진은 전사 안에 들어갔고 아직 나오지 않았다.
    controls.wait_until_inside();

    let started = Instant::now();
    for _ in 0..5 {
        let status = transcriber.status().expect("상태를 물어볼 수 있어야 한다");
        assert_eq!(status.state, "running");
        assert_eq!(
            status.recording_id.as_deref(),
            Some(fixture.recording_id.as_str())
        );
        assert_eq!(status.transcript_id, None, "아직 Transcript가 없다");
        assert_eq!(status.failure, None);
    }
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "상태 조회가 전사를 기다렸다: {elapsed:?}"
    );
    assert_eq!(
        engine.call_count(),
        1,
        "사전 조건: 엔진은 여전히 그 한 번의 전사 안에 있다"
    );
    assert_eq!(
        engine.calls()[0],
        (1_600, MODEL_FILE.to_owned()),
        "파생 입력과 해석된 모델이 실제로 엔진까지 간다"
    );

    controls.release();
    assert_eq!(wait_for_finish(&transcriber).state, "done");
}

#[test]
fn other_commands_answer_while_a_transcription_is_running() {
    // 전사가 앱의 저장소 연결을 붙들면, 스레드를 만들어 놓고 저장소에서 다시 막는 셈이 된다.
    let fixture = Fixture::new("other-commands");
    let (engine, controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = fixture.transcriber(engine);
    let storage = fixture.storage();

    transcriber
        .start(&fixture.recording_id)
        .expect("전사를 시작할 수 있어야 한다");
    controls.wait_until_inside();

    let started = Instant::now();
    let listed = storage.list_recordings().expect("목록을 읽을 수 있어야 한다");
    let settings = storage.settings().expect("설정을 읽을 수 있어야 한다");
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "다른 command가 전사를 기다렸다: {elapsed:?}"
    );
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].transcription_status, "running",
        "화면이 읽는 상태도 이미 저장돼 있다 (§7 · 요구 3)"
    );
    assert!(!settings.automatic_processing, "설정 조회도 정상적으로 답한다");

    controls.release();
    assert_eq!(wait_for_finish(&transcriber).state, "done");
}

// --- 중복 시작은 거부된다 --------------------------------------------------------------

#[test]
fn starting_the_same_recording_twice_is_refused_instead_of_being_ignored() {
    let fixture = Fixture::new("duplicate");
    let (engine, controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = fixture.transcriber(engine.clone());

    transcriber
        .start(&fixture.recording_id)
        .expect("첫 시작은 성공한다");
    controls.wait_until_inside();

    let failure = transcriber
        .start(&fixture.recording_id)
        .expect_err("같은 녹음을 다시 시작할 수 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(
        !failure.message.is_empty(),
        "그대로 화면에 띄울 수 있는 문장이어야 한다"
    );
    assert!(failure.source_data_safe, "아무것도 건드리지 않았다");
    assert_eq!(
        transcriber.status().expect("상태를 읽는다").state,
        "running",
        "거부가 진행 중인 전사를 흔들지 않는다"
    );

    controls.release();
    let finished = wait_for_finish(&transcriber);

    assert_eq!(finished.state, "done");
    assert_eq!(
        engine.call_count(),
        1,
        "거부된 요청이 전사를 한 번 더 돌리지 않는다"
    );
    assert_eq!(
        fixture.transcripts(&fixture.recording_id).len(),
        1,
        "Transcript도 하나뿐이다 — 중복 시작이 이유 없이 둘을 만들지 않는다"
    );
}

#[test]
fn a_second_recording_does_not_queue_up_behind_the_running_one() {
    // 여러 Recording 동시 전사 큐는 이 Phase의 범위 밖이다 (§16 DEFERRED). 줄을 세우는 대신
    // 거절하며, 거절된 요청은 앞의 전사가 끝났다고 해서 저절로 시작되지도 않는다.
    let fixture = Fixture::new("no-queue");
    fixture.insert_recording("rec-second", "두 번째 녹음");
    let (engine, controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = fixture.transcriber(engine.clone());

    transcriber
        .start(&fixture.recording_id)
        .expect("첫 시작은 성공한다");
    controls.wait_until_inside();

    let failure = transcriber
        .start("rec-second")
        .expect_err("전사 중에는 다른 녹음도 시작할 수 없다");
    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(failure.retryable, "지금 것이 끝난 뒤에는 시작할 수 있다");

    controls.release();
    let finished = wait_for_finish(&transcriber);

    assert_eq!(
        finished.recording_id.as_deref(),
        Some(fixture.recording_id.as_str()),
        "끝난 것은 첫 번째 전사다"
    );
    // 앞의 전사가 끝난 뒤에도 거절된 요청은 스스로 돌지 않는다.
    thread::sleep(Duration::from_millis(100));
    assert_eq!(engine.call_count(), 1, "돌아간 전사는 한 건뿐이다");
    assert_eq!(
        fixture.recording("rec-second").transcription_status,
        ProcessingStatus::None,
        "거절된 녹음의 상태는 손대지 않는다"
    );
    assert!(fixture.transcripts("rec-second").is_empty());
}

// --- 진행 중과 끝난 것은 다른 답이다 ---------------------------------------------------

#[test]
fn running_and_finished_are_two_different_answers() {
    let fixture = Fixture::new("running-vs-done");
    let (engine, controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = fixture.transcriber(engine);

    transcriber
        .start(&fixture.recording_id)
        .expect("전사를 시작할 수 있어야 한다");
    controls.wait_until_inside();
    let running = transcriber.status().expect("진행 중 상태를 읽는다");

    controls.release();
    let done = wait_for_finish(&transcriber);

    assert_eq!(running.state, "running");
    assert_eq!(done.state, "done");
    assert_eq!(running.transcript_id, None, "진행 중에는 결과가 없다");
    assert_eq!(running.failure, None);
    assert_eq!(done.failure, None);

    let transcript_id = done.transcript_id.expect("끝난 전사는 Transcript를 가리킨다");
    let recording = fixture.recording(&fixture.recording_id);
    assert_eq!(recording.transcription_status, ProcessingStatus::Done);
    assert_eq!(
        recording
            .current_transcript_id
            .as_ref()
            .map(|id| id.as_str()),
        Some(transcript_id.as_str()),
        "상태가 done인 시점에 current는 이미 그 Transcript다 (§7.2)"
    );
    let stored = fixture.transcripts(&fixture.recording_id);
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].id.as_str(), transcript_id);
    assert_eq!(
        stored[0].segments[0].start_ms, 134_000,
        "센티초 13400은 밀리초 134000이다 — 배경 스레드를 지나도 단위가 바뀌지 않는다"
    );
}

#[test]
fn a_finished_transcription_can_be_started_again() {
    // 거절은 **돌고 있는 동안에만**이다. 재전사는 새 Transcript를 추가한다 (INV-2 · §7.1).
    let fixture = Fixture::new("again");
    let (engine, controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = fixture.transcriber(engine.clone());

    controls.release();
    transcriber
        .start(&fixture.recording_id)
        .expect("첫 전사를 시작할 수 있어야 한다");
    assert_eq!(wait_for_finish(&transcriber).state, "done");

    controls.release();
    transcriber
        .start(&fixture.recording_id)
        .expect("끝난 뒤에는 다시 시작할 수 있어야 한다");
    let second = wait_for_finish(&transcriber);

    assert_eq!(second.state, "done");
    assert_eq!(engine.call_count(), 2);
    assert_eq!(
        fixture.transcripts(&fixture.recording_id).len(),
        2,
        "재전사는 기존 Transcript를 고치지 않고 새로 추가한다 (INV-2)"
    );
}

// --- 실패는 기존 Failure 계약으로 온다 (§13) --------------------------------------------

#[test]
fn a_failed_transcription_arrives_as_the_failure_the_engine_reported() {
    let fixture = Fixture::new("engine-failure");
    let (engine, controls) = GatedEngine::new(Err(engine_failed("전사 도중 엔진이 죽었다")));
    let transcriber = fixture.transcriber(engine);

    controls.release();
    transcriber
        .start(&fixture.recording_id)
        .expect("시작 자체는 성공한다 — 실패는 전사가 돌면서 드러난다");
    let finished = wait_for_finish(&transcriber);

    assert_eq!(finished.state, "failed");
    assert_eq!(finished.transcript_id, None);
    let failure = finished
        .failure
        .as_ref()
        .expect("실패가 상태에 실려 와야 한다");
    assert_eq!(failure.kind, FailureKind::TranscriptionEngineFailed);
    assert!(failure.retryable, "§13의 세 번째 질문에 답한다");
    assert!(failure.source_data_safe, "§13의 두 번째 질문에 답한다");
    assert!(!failure.message.is_empty());

    // 실패해도 원본과 레코드는 그대로다 (INV-1 · INV-3).
    assert!(fixture.audio_path.is_file(), "원본 오디오가 그대로 있어야 한다");
    assert_eq!(
        fixture.recording(&fixture.recording_id).transcription_status,
        ProcessingStatus::Failed
    );
    assert!(fixture.transcripts(&fixture.recording_id).is_empty());

    // 화면이 읽는 모양 그대로인지 본다 — 필드 이름이 어긋나면 실패가 조용히 사라진다
    // (`src/ipc/types.ts`의 TranscriptionStatus).
    let json = serde_json::to_value(&finished).expect("직렬화할 수 있어야 한다");
    assert_eq!(json["state"], "failed");
    assert_eq!(json["recordingId"], fixture.recording_id.as_str());
    assert!(json["transcriptId"].is_null());
    assert_eq!(json["failure"]["kind"], "transcriptionEngineFailed");
    assert_eq!(json["failure"]["sourceDataSafe"], true);
    assert_eq!(json["failure"]["retryable"], true);
}

#[test]
fn a_transcription_without_a_chosen_model_reports_the_model_missing_failure() {
    // 모델을 아직 고르지 않은 것이 앱의 기본 상태다 (설정에 저장된 값이 없다). 그 상태는
    // 조용한 skip이 아니라 §13의 실패이며, 엔진을 부르지도 않는다.
    let fixture = Fixture::new("no-model");
    let (engine, _controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = Transcriber::with_engine(fixture.app_data_dir.clone(), engine.clone());

    transcriber
        .start(&fixture.recording_id)
        .expect("시작 자체는 성공한다");
    let finished = wait_for_finish(&transcriber);

    assert_eq!(finished.state, "failed");
    let failure = finished.failure.expect("실패가 상태에 실려 와야 한다");
    assert_eq!(failure.kind, FailureKind::TranscriptionModelMissing);
    assert!(!failure.retryable, "모델을 먼저 둬야 풀린다");
    assert_eq!(engine.call_count(), 0, "모델 없이 엔진을 부르지 않는다");
    assert!(fixture.audio_path.is_file(), "원본은 그대로다");
}

#[test]
fn a_transcription_for_an_unknown_recording_fails_without_touching_anything() {
    let fixture = Fixture::new("unknown");
    let (engine, _controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = fixture.transcriber(engine.clone());

    transcriber
        .start("없는-녹음")
        .expect("시작 자체는 성공한다");
    let finished = wait_for_finish(&transcriber);

    assert_eq!(finished.state, "failed");
    assert_eq!(
        finished.failure.expect("실패가 실려 온다").kind,
        FailureKind::InvalidInput
    );
    assert_eq!(engine.call_count(), 0);
    assert_eq!(
        fixture.recording(&fixture.recording_id).transcription_status,
        ProcessingStatus::None,
        "다른 녹음의 상태를 건드리지 않는다"
    );
}

#[test]
fn an_empty_recording_id_is_refused_before_anything_starts() {
    let fixture = Fixture::new("blank-id");
    let (engine, _controls) = GatedEngine::new(Ok(spoken()));
    let transcriber = fixture.transcriber(engine.clone());

    let failure = transcriber
        .start("   ")
        .expect_err("무엇을 전사할지 없이 시작할 수 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert_eq!(
        transcriber.status().expect("상태를 읽는다").state,
        "idle",
        "거절된 요청은 상태를 바꾸지 않는다"
    );
    assert_eq!(engine.call_count(), 0);
}
