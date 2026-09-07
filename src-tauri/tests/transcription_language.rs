//! **무슨 언어로 전사할지가 설정에서 엔진 경계까지 값으로 도달한다** (ADR-0007 §17.1.4).
//!
//! ```text
//! settings.transcription_language   →  Transcriber  →  run  →  TranscriptionEngine
//!         고르지 않음 (None)         →                        →  LanguageChoice::Detect
//!         고름 ("ko")                →                        →  LanguageChoice::Chosen("ko")
//! ```
//!
//! 여기서 판정하는 것은 **선택이 엔진에 도달하는가**이지 전사 품질이 아니다. 그래서 실제
//! whisper도 모델 파일도 필요하지 않다 (§18) — 엔진 자리에는 계약이 같은 test double이 서고,
//! 그 double이 받은 언어 선택을 테스트가 그대로 읽는다.
//!
//! 이 파일이 존재하는 이유는 2026-09-05의 실사용이다. 아무도 고른 적 없는 `"en"` 하나가
//! 72분짜리 한국어 회의를 통째로 못 쓰게 만들었고 (ADR-0007 §17.1.1), 그 값이 엔진에 도달한
//! 경로에는 **아무 테스트도 없었다.**
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다. Tauri 런타임도 창도 하드웨어도 없다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use molt_note_lib::commands::Transcriber;
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{
    Failure, ProcessingStatus, Recording, RecordingId, Settings, Transcript,
};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::transcription::engine::{LanguageChoice, TranscriptionEngine};
use molt_note_lib::transcription::model::ModelFile;
use molt_note_lib::transcription::testing::{StubCall, StubEngine};
use molt_note_lib::transcription::{RawSegment, RawTranscription, TranscriptionInput};

/// 모델 디렉터리에 두는 자리표시자 파일의 이름. 실제 모델이 아니다.
const MODEL_FILE: &str = "ggml-base.bin";

/// 배경 전사가 끝나기를 기다리는 한계. 넘으면 테스트가 실패한다 — 매달리지 않는다.
const FINISH_TIMEOUT: Duration = Duration::from_secs(30);

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-transcription-language-{}-{}-{}",
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

/// [`Transcriber`]에게 넘긴 double을 테스트도 함께 들고 있기 위한 얇은 껍데기.
///
/// [`Transcriber::with_engine`]은 엔진을 **가져간다**. 그 뒤에도 무엇이 넘어갔는지 보려면
/// 같은 double을 가리키는 손잡이가 하나 더 있어야 한다 — 이 타입이 하는 일은 그것뿐이며,
/// 계약은 그대로 [`StubEngine`]이 이행한다.
struct SharedEngine(Arc<StubEngine>);

impl TranscriptionEngine for SharedEngine {
    fn engine_id(&self) -> String {
        self.0.engine_id()
    }

    fn transcribe(
        &self,
        input: &TranscriptionInput,
        model: &ModelFile,
        language: &LanguageChoice,
    ) -> Result<RawTranscription, Failure> {
        self.0.transcribe(input, model, language)
    }
}

/// 녹음 하나와 모델 하나가 이미 있는 앱. 설정 값만 테스트가 정한다.
struct Fixture {
    /// Drop 시 임시 디렉터리를 지운다.
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    transcriber: Transcriber,
    engine: Arc<StubEngine>,
    recording_id: String,
}

impl Fixture {
    /// 주어진 언어 설정으로 준비한다. `None`은 **고르지 않은 상태**이며 그것이 기본값이다.
    fn new(label: &str, language: Option<&str>, reported: RawTranscription) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.path().join("app-data"));
        app_data_dir.ensure().expect("사전 조건: 앱 데이터 디렉터리");
        let models_dir = app_data_dir
            .ensure_models_dir()
            .expect("사전 조건: 모델 디렉터리");
        std::fs::write(models_dir.join(MODEL_FILE), b"not a real model")
            .expect("사전 조건: 자리표시자 모델을 둔다");

        let audio_path = root.path().join("recording.wav");
        write_silence_wav(&audio_path);

        let connection = db::open_in(&app_data_dir).expect("사전 조건: DB를 연다");
        settings::save(
            &connection,
            &Settings {
                transcription_model: Some(MODEL_FILE.to_owned()),
                transcription_language: language.map(str::to_owned),
                ..Settings::DEFAULT
            },
        )
        .expect("사전 조건: 설정을 저장한다");

        let recording_id = "rec-language".to_owned();
        store::insert_recording(
            &connection,
            &Recording {
                id: RecordingId::new(&recording_id),
                title: "3DGS Study #04".to_owned(),
                created_at: "2026-09-06T10:00:00.000Z".to_owned(),
                updated_at: "2026-09-06T10:00:00.000Z".to_owned(),
                duration_ms: 100,
                audio_path: audio_path.to_str().expect("경로 문자열").to_owned(),
                audio_format: "wav".to_owned(),
                microphone: Some("MacBook Pro Microphone".to_owned()),
                current_transcript_id: None,
                transcription_status: ProcessingStatus::None,
                ai_status: ProcessingStatus::None,
                notion_status: ProcessingStatus::None,
            },
        )
        .expect("사전 조건: 녹음 레코드를 저장한다");
        drop(connection);

        let engine = Arc::new(StubEngine::returning(reported));

        Self {
            transcriber: Transcriber::with_engine(
                app_data_dir.clone(),
                SharedEngine(Arc::clone(&engine)),
            ),
            engine,
            app_data_dir,
            recording_id,
            _root: root,
        }
    }

    /// 전사 한 건을 제품 경로 그대로 돌리고, 엔진이 받은 호출 하나를 돌려준다.
    fn transcribe(&self) -> StubCall {
        self.transcriber
            .start(&self.recording_id)
            .expect("전사를 시작할 수 있어야 한다");

        let deadline = Instant::now() + FINISH_TIMEOUT;
        loop {
            let status = self
                .transcriber
                .status()
                .expect("상태를 물어볼 수 있어야 한다");
            if status.state != "running" {
                assert_eq!(status.state, "done", "전사가 성공해야 한다: {status:?}");
                break;
            }
            assert!(
                Instant::now() < deadline,
                "전사가 끝나기를 기다리다 시간이 지났다"
            );
            thread::sleep(Duration::from_millis(10));
        }

        let calls = self.engine.calls();
        assert_eq!(calls.len(), 1, "엔진은 한 번 불린다");
        calls.into_iter().next().expect("호출 하나")
    }

    fn transcripts(&self) -> Vec<Transcript> {
        let connection = db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다");
        store::list_transcripts(&connection, &RecordingId::new(&self.recording_id))
            .expect("Transcript 목록을 읽을 수 있어야 한다")
    }
}

/// 0.1초짜리 16 kHz mono PCM16 WAV. 짧고 결정론적인 fixture다.
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
fn heard(language: Option<&str>) -> RawTranscription {
    RawTranscription {
        language: language.map(str::to_owned),
        segments: vec![RawSegment {
            start_centiseconds: 100,
            end_centiseconds: 240,
            text: Some(" 그러면 이번에는 PLY 먼저 변환하고".to_owned()),
        }],
    }
}

#[test]
fn not_choosing_a_language_reaches_the_engine_as_detection() {
    // §17.1.4-1: 아무도 고르지 않은 상태에서 엔진이 받는 것은 "영어"가 아니라 "감지하라"다.
    let fixture = Fixture::new("unset", None, heard(Some("ko")));

    let call = fixture.transcribe();

    assert_eq!(call.language, LanguageChoice::Detect);
    assert_eq!(
        call.language.chosen(),
        None,
        "지정할 코드가 없다 — whisper.cpp의 기본값 \"en\"이 대신 들어가서는 안 된다"
    );
}

#[test]
fn a_chosen_language_reaches_the_engine_as_that_language() {
    let fixture = Fixture::new("chosen", Some("ko"), heard(Some("ko")));

    let call = fixture.transcribe();

    assert_eq!(call.language, LanguageChoice::Chosen("ko".to_owned()));
    assert_eq!(call.language.chosen(), Some("ko"));
}

#[test]
fn a_blank_setting_is_the_same_as_not_having_chosen() {
    // 저장소는 이 값을 검사하지 않는다 (INV-10). 공백만 남은 값이 언어로 해석되면, 아무도
    // 고르지 않은 것과 같은 상태가 조용히 다른 뜻을 갖게 된다.
    let fixture = Fixture::new("blank", Some("   "), heard(Some("ko")));

    assert_eq!(fixture.transcribe().language, LanguageChoice::Detect);
}

#[test]
fn the_stored_language_is_the_one_the_engine_reported() {
    // §17.1.4-3 · §16.2: 설정 값을 결과에 베껴 넣지 않는다. 고른 것은 "ja"지만 엔진이 들었다고
    // 말한 것은 "ko"이고, Transcript에 남는 것은 후자다.
    let fixture = Fixture::new("provenance", Some("ja"), heard(Some("ko")));

    let call = fixture.transcribe();

    assert_eq!(call.language.chosen(), Some("ja"), "고른 값은 엔진으로 갔다");
    let transcripts = fixture.transcripts();
    assert_eq!(transcripts.len(), 1);
    assert_eq!(
        transcripts[0].language.as_deref(),
        Some("ko"),
        "저장된 것은 엔진이 보고한 값이다"
    );
}

#[test]
fn detection_being_on_does_not_mean_a_language_is_always_known() {
    // "감지를 켰다"가 "항상 값이 있다"는 뜻이 되지 않는다 (§17.1.4-3). 엔진이 말하지 못하면
    // 비어 있고, 그 자리를 설정 값이나 짐작으로 채우지 않는다.
    let fixture = Fixture::new("unknown", None, heard(None));

    fixture.transcribe();

    let transcripts = fixture.transcripts();
    assert_eq!(transcripts.len(), 1);
    assert_eq!(transcripts[0].language, None);
}
