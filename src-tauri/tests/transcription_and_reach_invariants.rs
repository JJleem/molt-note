//! **Phase 5.6의 네 불변을 이미 만들어진 경로로 못박는다**
//! (`phase-prompt/05.6` 성공 기준 · `docs/ADR-0007-transcription-engine.md` §17 ·
//! `docs/ADR-0010-manual-ai-handoff.md` §5.6 · PRODUCT-SPEC §5 D · §12 · §13 · §18).
//!
//! Phase 4의 `tests/core_pipeline_without_ai.rs`, Phase 5의
//! `tests/notion_and_export_invariants.rs`, Phase 5.5의 `tests/manual_handoff_invariants.rs`가
//! 각자의 Phase에 대해 하는 일과 같은 자리다 — **새 제품 코드를 만들지 않고**, 이미 있는 경로를
//! 그대로 지나면서 이 Phase의 불변을 각각 실패할 수 있는 검사로 바꾼다.
//!
//! ```text
//! TR-1  언어가 영어로 강제되지 않는다 — 설정의 선택이 엔진 경계까지 값으로 도달하고,
//!       고르지 않았을 때 도착하는 것은 "영어"가 아니라 "감지하라"다
//!         tr1_the_language_the_settings_surface_stored_is_the_one_the_engine_hears
//!         tr1_no_unset_or_blank_setting_ever_reaches_the_engine_as_a_forced_language
//!         tr1_the_engine_boundary_never_leaves_the_library_default_in_place
//! TR-2  §D의 language 설정이 실재하고 왕복한다 — 저장 → 재조회, 그리고 다른 설정을 저장할 때
//!       조용히 지워지지 않는다
//!         tr2_a_chosen_language_comes_back_from_disk_through_the_settings_surface
//!         tr2_saving_a_different_setting_does_not_quietly_erase_the_chosen_language
//!         tr2_not_having_chosen_stays_not_chosen_across_other_saves
//! TR-3  내보낸 파일에 도달하는 수단이 있다 — 여는 표면이 있고, 그것이 이 앱의 exports 밖을
//!       열지 못한다
//!         tr3_the_file_this_app_just_exported_is_the_one_that_reaches_the_os_boundary
//!         tr3_nothing_outside_the_exports_directory_reaches_the_os_boundary
//!         tr3_the_surface_that_opens_a_saved_file_exists_and_needs_the_owner_that_decides
//! TR-4  긴 handoff가 크기 때문에 조용히 실패하지 않는다 — 크기가 값으로 나오고, 순서대로
//!       나뉘며, 이어 붙이면 내용이 그대로다
//!         tr4_a_long_handoff_reports_its_size_instead_of_arriving_silently_truncated
//!         tr4_the_portions_come_in_order_and_reassemble_into_the_whole_output
//!         tr4_every_portion_of_the_exported_document_is_its_own_file_and_reassembles
//!         tr4_asking_for_a_portion_that_does_not_exist_is_a_failure_not_an_empty_answer
//! ```
//!
//! ## 이 파일이 판정하지 **않는** 것 (`phase-prompt/05.6` Human Review)
//!
//! **한국어 회의가 읽을 만하게 전사되는가 · `ggml-base`로 충분한가 · Metal 전후로 체감이
//! 달라지는가는 여기서 판정되지 않는다.** 그것은 사람의 귀와 사람의 시간이 필요한 항목이고
//! (Phase 문서의 Human Review), 자동 Gate가 그것을 대신한다고 말하는 검사는 이 파일에 하나도
//! 없다. 여기서 보는 것은 **선택이 값으로 도달하는가 · 값이 왕복하는가 · 여는 수단이 범위를
//! 지키는가 · 큰 산출물이 크기를 말하고 무손실로 나뉘는가**뿐이다.
//!
//! 그래서 실제 whisper도 모델 파일도 쓰지 않는다 (§18) — 엔진 자리에는 계약이 같은 test
//! double이 서고, 그 double이 받은 언어 선택을 테스트가 그대로 읽는다. 실제 추론은 운영자의
//! smoke test가 한 번 한다 (PRODUCT-SPEC §14.4.3).
//!
//! ## 이미 있는 검사를 다시 쓰지 않는다
//!
//! 아래는 **여기서 다시 쓰지 않고** 그 자리를 가리킨다. 같은 사실을 두 번 적으면 둘이 어긋날 때
//! 어느 쪽이 규칙인지 알 수 없게 된다.
//!
//! ```text
//! 언어 선택의 세부 갈래 (공백 · 미지의 코드 · 보고된 언어)  tests/transcription_language.rs
//! 여는 경계의 소스 규약 (OS 호출이 한 파일 안에만 있다)      tests/show_saved_file.rs
//! 나눔 규칙 자체 (문단 → 줄 → 문장 → 낱말 → 글자)           src/export/portion.rs 의 단위 테스트
//! command 표면이 정확히 서른둘이라는 사실                    tests/ipc-boundary.test.ts
//! 화면의 done 상태가 여는 동작을 값으로 들고 있다는 사실     tests/transcription-and-reach-invariants.test.ts
//! ```
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다. Tauri 런타임도 창도 하드웨어도 네트워크도 없다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use molt_note_lib::commands::{
    Exporter, ExportedAiRequestPayload, SavedFiles, SettingsPayload, Storage, TextSizePayload,
    Transcriber,
};
use molt_note_lib::db::{self, store};
use molt_note_lib::domain::{
    Failure, FailureKind, NoteType, ProcessingStatus, Recording, RecordingId, Settings, Transcript,
    TranscriptId, TranscriptSegment,
};
use molt_note_lib::export::portion::{measure, PORTION_MAX_BYTES};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::file_manager::testing::RecordedFileManager;
use molt_note_lib::transcription::engine::{LanguageChoice, TranscriptionEngine};
use molt_note_lib::transcription::model::ModelFile;
use molt_note_lib::transcription::testing::{StubCall, StubEngine};
use molt_note_lib::transcription::{RawSegment, RawTranscription, TranscriptionInput};

/// 이 파일이 쓰는 녹음 시각. 파일 이름의 날짜가 여기서 나온다 (ADR-0009 §4.2).
const CREATED_AT: &str = "2026-09-06T10:00:00.000Z";

/// 모델 디렉터리에 두는 자리표시자 파일의 이름. **실제 모델이 아니다** — 엔진 자리에는
/// double이 서므로 이 파일은 열리지 않는다.
const MODEL_FILE: &str = "ggml-base.bin";

/// 배경 전사가 끝나기를 기다리는 한계. 넘으면 테스트가 실패한다 — 매달리지 않는다.
const FINISH_TIMEOUT: Duration = Duration::from_secs(30);

/// 언어 선택을 실제 엔진 호출로 옮기는 유일한 자리 (ADR-0007 §17.1.4-4).
const ENGINE_BOUNDARY: &str = "src/transcription/whisper.rs";

/// TR-4가 쓰는 전사의 segment 수. **예산을 확실히 넘기기 위한 값이다** — 72분 회의의 실측이
/// 99 KB였고 (R-5), 예산은 40,000 B다 (`export::portion::PORTION_MAX_BYTES`).
const LONG_SEGMENT_COUNT: usize = 900;

/// segment 하나를 이어 붙인 결과에서 되찾기 위한 표식. 번호가 네 자리라 서로 접두사가 되지 않는다.
const SENTENCE_MARKER: &str = "문장-";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 자리 ------------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
///
/// 경로는 `std::env::temp_dir()`에서만 나오므로 이 파일의 어떤 검사도 사용자의 실제 앱 데이터
/// 디렉터리나 export 디렉터리를 만들거나 건드리지 않는다 (§18).
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "molt-note-transcription-and-reach-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
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

/// 저장소 하나와 그 위에서 도는 세 경계 — 설정을 왕복시키는 쪽, 파일을 쓰는 쪽, 그 파일이 놓인
/// 자리를 여는 쪽. 셋 다 제품이 실제로 세우는 자리 그대로다.
struct App {
    /// Drop 시 임시 디렉터리를 지운다.
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    storage: Storage,
    exporter: Exporter,
    /// 자동 테스트가 쓰는 유일한 파일 관리자. **창을 하나도 띄우지 않는다** (§18).
    file_manager: Arc<RecordedFileManager>,
    saved_files: SavedFiles,
}

impl App {
    fn new(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.path().join("app-data"));

        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

        let file_manager = Arc::new(RecordedFileManager::new());

        Self {
            exporter: Exporter::in_directory(app_data_dir.clone()),
            saved_files: SavedFiles::in_directory(app_data_dir.clone(), file_manager.clone()),
            file_manager,
            storage,
            app_data_dir,
            _root: root,
        }
    }

    fn connection(&self) -> rusqlite::Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    /// 지금 저장돼 있는 설정. **화면이 읽는 것과 같은 표면을 지난다.**
    fn settings(&self) -> SettingsPayload {
        self.storage
            .settings()
            .expect("설정을 읽을 수 있어야 한다")
    }

    /// 설정 하나를 저장하고 backend가 돌려준 값을 그대로 준다 (`update_settings`).
    fn save_settings(&self, payload: SettingsPayload) -> SettingsPayload {
        self.storage
            .update_settings(payload)
            .expect("설정을 저장할 수 있어야 한다")
    }

    /// 녹음 하나와 그 current Transcript를 저장한다. **오디오 파일은 만들지 않는다** —
    /// export도 handoff도 그것을 읽지 않는다 (INV-6).
    fn save_recording_with_transcript(&self, id: &str, title: &str, sentences: &[String]) {
        let recording = Recording {
            id: RecordingId::new(id),
            title: title.to_owned(),
            created_at: CREATED_AT.to_owned(),
            updated_at: CREATED_AT.to_owned(),
            duration_ms: 3_151_000,
            audio_path: format!("recordings/{id}.wav"),
            audio_format: "wav".to_owned(),
            microphone: Some("MacBook Pro Microphone".to_owned()),
            current_transcript_id: None,
            transcription_status: ProcessingStatus::Done,
            ai_status: ProcessingStatus::None,
            notion_status: ProcessingStatus::None,
        };
        store::insert_recording(&self.connection(), &recording)
            .expect("사전 조건: 녹음을 저장한다");

        let segments: Vec<TranscriptSegment> = sentences
            .iter()
            .enumerate()
            .map(|(index, text)| TranscriptSegment {
                start_ms: (index as i64) * 3_000,
                end_ms: (index as i64) * 3_000 + 2_900,
                text: text.clone(),
            })
            .collect();

        let transcript = Transcript {
            id: TranscriptId::new(format!("{id}-transcript")),
            recording_id: recording.id.clone(),
            language: Some("ko".to_owned()),
            raw_text: sentences.join("\n"),
            segments,
            created_at: CREATED_AT.to_owned(),
            engine: "stub".to_owned(),
            model: MODEL_FILE.to_owned(),
            transcription_ms: None,
        };

        let mut connection = self.connection();
        store::append_transcript(&mut connection, &transcript)
            .expect("사전 조건: 전사를 저장한다");
        store::set_current_transcript(
            &self.connection(),
            &recording.id,
            Some(&transcript.id),
            CREATED_AT,
        )
        .expect("사전 조건: current를 지정한다");
    }

    /// AI-ready 문서의 조각 하나를 **제품 경로 그대로** 파일로 쓴다.
    fn export_ai_request(&self, recording_id: &str, portion: usize) -> ExportedAiRequestPayload {
        self.exporter
            .export_ai_request(recording_id, NoteType::Meeting, Some(portion))
            .expect("AI-ready 문서를 쓸 수 있어야 한다")
    }
}

/// 설정에 저장된 언어 하나가 엔진 경계에 무엇으로 도착하는지 보기 위한 자리.
///
/// 녹음 · 모델 자리표시자 · 설정이 이미 있는 앱이며, 다른 것은 아무것도 바꾸지 않는다 —
/// **달라지는 것은 설정 값 하나뿐이다.**
struct Listening {
    /// Drop 시 임시 디렉터리를 지운다.
    _root: TempRoot,
    transcriber: Transcriber,
    engine: Arc<StubEngine>,
    recording_id: String,
}

impl Listening {
    fn new(label: &str, language: Option<&str>) -> Self {
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

        // **설정은 화면이 쓰는 표면을 그대로 지난다** — 여기서 DB에 직접 쓰면 TR-2가 지키는
        // 왕복 경로를 건너뛰게 되고, TR-1은 저장된 적 없는 값을 보게 된다.
        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");
        storage
            .update_settings(SettingsPayload::from(Settings {
                transcription_model: Some(MODEL_FILE.to_owned()),
                transcription_language: language.map(str::to_owned),
                ..Settings::DEFAULT
            }))
            .expect("사전 조건: 설정을 저장한다");

        let recording_id = "rec-listening".to_owned();
        store::insert_recording(
            &db::open_in(&app_data_dir).expect("사전 조건: DB를 연다"),
            &Recording {
                id: RecordingId::new(&recording_id),
                title: "3DGS Study #04".to_owned(),
                created_at: CREATED_AT.to_owned(),
                updated_at: CREATED_AT.to_owned(),
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

        let engine = Arc::new(StubEngine::returning(RawTranscription {
            language: Some("ko".to_owned()),
            segments: vec![RawSegment {
                start_centiseconds: 100,
                end_centiseconds: 240,
                text: Some(" 그러면 이번에는 PLY 먼저 변환하고".to_owned()),
            }],
        }));

        Self {
            transcriber: Transcriber::with_engine(
                app_data_dir.clone(),
                SharedEngine(Arc::clone(&engine)),
            ),
            engine,
            recording_id,
            _root: root,
        }
    }

    /// 전사 한 건을 제품 경로 그대로 돌리고, **엔진이 받은 호출 하나**를 돌려준다.
    fn heard(&self) -> StubCall {
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
}

/// [`Transcriber`]에게 넘긴 double을 테스트도 함께 들고 있기 위한 얇은 껍데기.
///
/// [`Transcriber::with_engine`]은 엔진을 **가져간다**. 그 뒤에도 무엇이 넘어갔는지 보려면 같은
/// double을 가리키는 손잡이가 하나 더 있어야 한다 — 이 타입이 하는 일은 그것뿐이며, 계약은
/// 그대로 [`StubEngine`]이 이행한다.
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

// --- TR-1. 언어가 영어로 강제되지 않는다 -------------------------------------------------

#[test]
fn tr1_the_language_the_settings_surface_stored_is_the_one_the_engine_hears() {
    // 무엇이 깨지면 이 검사가 잡는가: 설정에서 엔진까지의 **어느 한 칸이라도** 언어를 흘리면
    // 잡는다 — `Transcriber`가 설정을 읽지 않게 되거나, `run`이 선택을 넘기지 않게 되거나,
    // 엔진 계약에서 `language` 인자가 사라지면 여기서 실패한다. 2026-09-05에 무너진 것이
    // 정확히 이 경로였고, 그때 이 경로에는 아무 검사도 없었다 (ADR-0007 §17.1.1).
    //
    // **전사 품질을 판정하지 않는다.** double이 돌려주는 문장은 고정돼 있으며, 한국어가
    // 읽을 만하게 전사되는가는 사람의 Human Review 항목이다.
    let chosen = Listening::new("chosen", Some("ko")).heard();

    assert_eq!(chosen.language, LanguageChoice::Chosen("ko".to_owned()));
    assert_eq!(
        chosen.language.chosen(),
        Some("ko"),
        "고른 언어가 엔진에 지정돼야 한다"
    );
}

#[test]
fn tr1_no_unset_or_blank_setting_ever_reaches_the_engine_as_a_forced_language() {
    // 무엇이 깨지면 이 검사가 잡는가: **"고르지 않음"을 언어로 해석하는 자리**가 생기면 잡는다 —
    // 저장소가 NULL을 기본 언어로 채우거나, 경계가 빈 값을 `Chosen("en")`으로 바꾸거나,
    // 로캘을 짐작해 채워 넣으면 여기서 실패한다 (§17.1.4-1).
    //
    // 세 입력이 같은 답을 내야 한다 — 고르지 않음 · 빈 문자열 · 공백뿐인 값.
    for (label, configured) in [
        ("unset", None),
        ("empty", Some("")),
        ("blank", Some("   \n\t ")),
    ] {
        let call = Listening::new(label, configured).heard();

        assert_eq!(
            call.language,
            LanguageChoice::Detect,
            "{label}: 고르지 않음은 감지다"
        );
        assert_eq!(
            call.language.chosen(),
            None,
            "{label}: 지정할 코드가 없다 — whisper.cpp의 기본값 \"en\"이 대신 들어가서는 안 된다"
        );
        assert_ne!(
            call.language,
            LanguageChoice::Chosen("en".to_owned()),
            "{label}: 아무도 고른 적 없는 영어가 엔진에 도착했다 (ADR-0007 §17.1.1)"
        );
    }
}

#[test]
fn tr1_the_engine_boundary_never_leaves_the_library_default_in_place() {
    // **이것은 소스 각도다.** 위 두 검사는 선택이 엔진 경계까지 도달하는 것을 실행으로
    // 판정하지만, 그 선택이 whisper.cpp의 파라미터로 어떻게 옮겨지는지는 실제 모델과 실제
    // 추론이 있어야 실행된다 — Gate는 모델을 두지 않으며 (§18), 실제 추론의 확인은 운영자의
    // smoke test다 (PRODUCT-SPEC §14.4.3). 그 사이에 남는 각도가 이것이다.
    //
    // 무엇이 깨지면 이 검사가 잡는가: **두 함수 중 하나라도 부르지 않게 되면** 잡는다.
    // 라이브러리의 기본값은 자동 감지가 아니라 `language = "en"` · `detect_language = false`이며
    // (ADR-0007 §17.1.2), 그것을 아무도 바꾸지 않은 채로 72분짜리 한국어 회의가 통째로 못 쓰게
    // 됐다. "고르지 않으면 아무것도 설정하지 않는다"로 되돌리는 순간 여기서 실패한다.
    let source = std::fs::read_to_string(Path::new(ENGINE_BOUNDARY))
        .expect("엔진 경계 파일을 읽을 수 있어야 한다");
    // 검사 대상은 **제품 경로의 코드**다. 이 파일의 모듈 문서는 두 갈래를 설명하며 그 이름을 쓴다.
    let product = source
        .split_once("#[cfg(test)]")
        .map(|(before, _)| before)
        .unwrap_or(&source);
    let code: String = product
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("//") && !trimmed.starts_with("*") && !trimmed.starts_with("/*")
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        code.contains("set_detect_language(true)"),
        "고르지 않았을 때 감지를 켜는 자리가 없다 (§17.1.4-1)"
    );
    assert!(
        code.contains("set_language(None)"),
        "고르지 않았을 때 지정할 언어가 없다는 사실을 넘기는 자리가 없다"
    );
    assert!(
        code.contains("set_language(Some(code))") && code.contains("set_detect_language(false)"),
        "고른 언어를 지정하고 감지를 끄는 자리가 없다"
    );
}

// --- TR-2. §D의 language 설정이 실재하고 왕복한다 ----------------------------------------

#[test]
fn tr2_a_chosen_language_comes_back_from_disk_through_the_settings_surface() {
    // 무엇이 깨지면 이 검사가 잡는가: **저장 경로와 조회 경로 어느 쪽에서든 이 열이 빠지면**
    // 잡는다 — migration에 열이 없거나, `INSERT`가 그 열을 쓰지 않거나, `SELECT`가 읽지 않으면
    // 여기서 실패한다. §5 D는 처음부터 이 항목을 요구했고, Phase 3은 그것을 빠뜨렸다 (R-2).
    let app = App::new("language-roundtrip");

    // 저장된 적 없는 상태의 기본값은 **고르지 않음**이다. 앱이 언어 하나를 미리 굳혀 두지 않는다.
    assert_eq!(app.settings().transcription_language, None);

    let saved = app.save_settings(SettingsPayload::from(Settings {
        transcription_language: Some("ko".to_owned()),
        ..Settings::DEFAULT
    }));
    assert_eq!(
        saved.transcription_language.as_deref(),
        Some("ko"),
        "저장한 쪽이 돌려주는 값에 이미 그 언어가 있어야 한다"
    );
    assert_eq!(app.settings().transcription_language.as_deref(), Some("ko"));

    // **디스크에 남았는가**를 따로 본다 — 같은 프로세스의 메모리에만 있으면 앱을 다시 켠
    // 사용자에게는 없는 것과 같다.
    let reopened = Storage::open(&app.app_data_dir);
    assert!(reopened.failure().is_none());
    assert_eq!(
        reopened
            .settings()
            .expect("다시 연 저장소에서 설정을 읽을 수 있어야 한다")
            .transcription_language
            .as_deref(),
        Some("ko")
    );
}

#[test]
fn tr2_saving_a_different_setting_does_not_quietly_erase_the_chosen_language() {
    // 무엇이 깨지면 이 검사가 잡는가: **다른 값을 저장하는 경로가 이 열을 함께 덮어쓰면** 잡는다.
    // 설정은 한 행짜리 테이블이므로 (`db::settings`), 저장문에서 열 하나를 빠뜨리거나 조회에서
    // 빠뜨리면 사용자가 고른 언어가 다음 저장에 조용히 사라진다 — 그리고 그 상태는 "고르지
    // 않음"과 구분되지 않으므로 아무도 알아채지 못한 채 다시 감지로 돌아간다.
    let app = App::new("language-survives");
    app.save_settings(SettingsPayload::from(Settings {
        transcription_language: Some("ko".to_owned()),
        ..Settings::DEFAULT
    }));

    // 화면이 하는 그대로 — 읽어 온 값에서 **다른 항목만** 바꿔 되돌려 보낸다.
    let mut edited = app.settings();
    edited.automatic_transcription = true;
    edited.transcription_model = Some(MODEL_FILE.to_owned());
    edited.notion_parent_page_id = Some("page-1".to_owned());
    let after = app.save_settings(edited);

    assert_eq!(
        after.transcription_language.as_deref(),
        Some("ko"),
        "다른 설정을 저장했다고 고른 언어가 사라지면 안 된다"
    );
    assert_eq!(app.settings().transcription_language.as_deref(), Some("ko"));
    // 바꾸려던 값은 실제로 바뀌었다 — "아무것도 저장되지 않았다"가 위 성공의 이유가 아니다.
    let stored = app.settings();
    assert!(stored.automatic_transcription);
    assert_eq!(stored.transcription_model.as_deref(), Some(MODEL_FILE));
    assert_eq!(stored.notion_parent_page_id.as_deref(), Some("page-1"));
}

#[test]
fn tr2_not_having_chosen_stays_not_chosen_across_other_saves() {
    // 무엇이 깨지면 이 검사가 잡는가: **저장하는 김에 언어를 하나 채워 넣는 자리**가 생기면
    // 잡는다 — 로캘을 짐작하거나 기본값을 굳혀 두면, 고른 적 없는 값이 그때부터 사용자가 고른
    // 값처럼 보인다 (`Settings::DEFAULT`의 주석 · ADR-0007 §17.1). 그 짐작이 틀렸을 때 무너지는
    // 모습은 이미 봤다.
    let app = App::new("unset-stays-unset");

    let mut edited = app.settings();
    edited.automatic_processing = true;
    edited.default_microphone = Some("mic-1".to_owned());
    let after = app.save_settings(edited);

    assert_eq!(
        after.transcription_language, None,
        "고르지 않은 자리에 언어가 생겼다"
    );
    assert_eq!(app.settings().transcription_language, None);
    assert!(app.settings().automatic_processing, "저장 자체는 일어났다");
}

// --- TR-3. 내보낸 파일에 도달하는 수단이 있다 --------------------------------------------

#[test]
fn tr3_the_file_this_app_just_exported_is_the_one_that_reaches_the_os_boundary() {
    // 무엇이 깨지면 이 검사가 잡는가: **방금 만든 파일을 여는 길이 끊기면** 잡는다 — 여는
    // 표면이 사라지거나, 허용 판정이 export가 실제로 쓰는 자리를 인정하지 않게 되거나,
    // 요청이 OS 경계까지 도달하지 못하면 여기서 실패한다. 경로를 글자로 보여 주는 것만으로는
    // 사람이 파일에 도달하지 못한다는 것이 2026-09-05에 드러났다 (R-4).
    //
    // **실제로 내보낸 파일을 쓴다** — 테스트가 자기 손으로 둔 파일이 아니라 제품의 export
    // 경로가 만든 그 파일이다. 두 자리가 어긋나면(예: export 위치가 바뀌면) 여기서 드러난다.
    let app = App::new("exported-then-opened");
    app.save_recording_with_transcript("rec-1", "3DGS Study #04", &sentences(3));

    let exported = app
        .exporter
        .export("rec-1")
        .expect("Markdown 파일을 쓸 수 있어야 한다");

    app.saved_files
        .show(&exported.path)
        .expect("이 앱이 방금 만든 파일은 열 수 있어야 한다");

    // 경계에 도착하는 것은 **정규화된 경로 하나**다 — 화면이 보낸 문자열 그대로가 아니다.
    assert_eq!(
        app.file_manager.shown(),
        vec![std::fs::canonicalize(&exported.path).expect("정규화할 수 있어야 한다")]
    );
    // 여는 일은 아무것도 바꾸지 않는다 (INV-3) — 파일도 그 내용도 그대로다.
    assert!(Path::new(&exported.path).is_file());
}

#[test]
fn tr3_nothing_outside_the_exports_directory_reaches_the_os_boundary() {
    // 무엇이 깨지면 이 검사가 잡는가: **여는 표면이 임의 경로를 열게 되면** 잡는다 —
    // 허용 판정이 사라지거나, 문자열 비교로 바뀌어 `..`이 풀리지 않거나, 디렉터리까지 열게
    // 되면 여기서 실패한다. webview가 보낸 경로를 그대로 OS에 넘기는 순간 이 앱은 저장소
    // 파일이든 사용자의 홈 디렉터리든 열어 주는 통로가 된다 (PRODUCT-SPEC §12).
    let app = App::new("outside-refused");
    app.save_recording_with_transcript("rec-1", "3DGS Study #04", &sentences(3));
    let exported = app
        .exporter
        .export("rec-1")
        .expect("사전 조건: 파일 하나를 내보낸다");

    let database = app.app_data_dir.database_path();
    let outside = app.app_data_dir.root().join("escaped.md");
    std::fs::write(&outside, "밖의 파일").expect("사전 조건: 파일을 둔다");

    let refused = [
        database.clone(),
        outside.clone(),
        app.app_data_dir
            .exports_dir()
            .join("..")
            .join("escaped.md"),
        app.app_data_dir.exports_dir(),
    ];

    for path in refused {
        let failure = match app.saved_files.show(&path.display().to_string()) {
            Ok(()) => panic!("{} 는 거절되어야 한다", path.display()),
            Err(failure) => failure,
        };

        assert_eq!(failure.kind, FailureKind::InvalidInput, "{}", path.display());
        assert!(
            failure.source_data_safe,
            "거절이 무언가를 건드렸다: {}",
            path.display()
        );
    }

    // 거절된 요청은 **하나도** OS 경계까지 가지 않았고, 허용된 파일은 여전히 열린다.
    assert!(
        app.file_manager.is_empty(),
        "거절된 요청이 OS 경계까지 갔다 (§12)"
    );
    assert!(database.exists() && outside.exists(), "거절이 파일을 건드렸다");

    app.saved_files
        .show(&exported.path)
        .expect("허용되는 파일은 그대로 열려야 한다");
    assert_eq!(app.file_manager.shown().len(), 1);
}

#[test]
fn tr3_the_surface_that_opens_a_saved_file_exists_and_needs_the_owner_that_decides() {
    // 무엇이 깨지면 이 검사가 잡는가: **여는 command 자체가 표면에서 사라지거나 모양이 바뀌면**
    // 컴파일되지 않는다. 값이 아니라 함수 하나를 가리키는 검사인 이유는 그것이 이 자리에서
    // 확인할 수 있는 사실이기 때문이다 — Tauri 런타임 없이 command를 실행할 수는 없다.
    //
    // 함께 고정되는 것이 하나 더 있다: 이 command는 **무엇을 열어도 되는지 판정하는 소유자**를
    // 받는다 (`SavedFiles`). 그 인자가 사라지면 판정을 지나지 않고 여는 경로가 생기는 것이며,
    // 그때 이 줄이 컴파일되지 않는다.
    //
    // 이 이름이 프론트엔드 표면에도 실재한다는 사실은 `tests/ipc-boundary.test.ts`가 본다.
    let opens: for<'a> fn(tauri::State<'a, SavedFiles>, String) -> Result<(), Failure> =
        molt_note_lib::commands::show_saved_file;
    let _ = opens;
}

// --- TR-4. 긴 handoff가 크기 때문에 조용히 실패하지 않는다 ------------------------------

#[test]
fn tr4_a_long_handoff_reports_its_size_instead_of_arriving_silently_truncated() {
    // 무엇이 깨지면 이 검사가 잡는가: **산출물이 크기를 말하지 않게 되면** 잡는다 — 크기가
    // 응답에서 빠지거나, 나뉜 것을 전체라고 말하거나, 조각 번호가 사라지면 여기서 실패한다.
    // 72분 회의의 산출물은 99 KB였고 사람은 그것을 채팅 창에 붙여 넣으려다 실패했으며, **앱은
    // 그 사실을 말해 주지 않았다** (R-5).
    //
    // 세 산출물을 함께 본다 — 하나만 크기를 말하면 나머지 둘에서 다시 조용해진다.
    let app = App::new("long-handoff-size");
    let spoken = sentences(LONG_SEGMENT_COUNT);
    app.save_recording_with_transcript("rec-long", "3DGS Study #04", &spoken);

    let prompt = app
        .storage
        .ai_prompt("rec-long", NoteType::Meeting, None)
        .expect("프롬프트를 만들 수 있어야 한다");
    let transcript = app
        .storage
        .transcript_text("rec-long", None)
        .expect("전사 텍스트를 만들 수 있어야 한다");
    let exported = app.export_ai_request("rec-long", 1);

    for (label, portion) in [
        ("prompt", &prompt.portion),
        ("transcript", &transcript.portion),
        ("export", &exported.portion),
    ] {
        assert!(
            portion.total_size.bytes > PORTION_MAX_BYTES,
            "{label}: 사전 조건 — 이 전사는 예산을 넘어야 한다 ({} B)",
            portion.total_size.bytes
        );
        assert!(
            portion.total_size.chars > 0 && portion.total_size.lines > 0,
            "{label}: 크기가 세 가지로 말해져야 한다"
        );
        assert!(
            portion.portion_count > 1,
            "{label}: 예산을 넘는 산출물이 한 조각이라고 말한다"
        );
        assert_eq!(portion.portion, 1, "{label}: 아무것도 고르지 않으면 첫 조각이다");
        assert!(
            portion.portion_size.bytes <= PORTION_MAX_BYTES,
            "{label}: 조각 하나가 예산을 넘는다"
        );
        assert!(
            portion.portion_size.bytes < portion.total_size.bytes,
            "{label}: 나뉜 조각을 전체라고 말한다"
        );
    }

    // 짧은 녹음에서는 이 모든 것이 보이지 않는다 — 그때는 한 조각이 곧 전체다.
    app.save_recording_with_transcript("rec-short", "3DGS Study #05", &sentences(3));
    let short = app
        .storage
        .ai_prompt("rec-short", NoteType::Meeting, None)
        .expect("짧은 프롬프트도 만들 수 있어야 한다");
    assert_eq!(short.portion.portion_count, 1);
    assert_eq!(short.portion.portion_size, short.portion.total_size);
}

#[test]
fn tr4_the_portions_come_in_order_and_reassemble_into_the_whole_output() {
    // 무엇이 깨지면 이 검사가 잡는가: **나눔이 내용을 잃거나 순서를 바꾸면** 잡는다 — 경계에서
    // 공백을 먹거나, 조각에 표식을 섞어 넣거나, 조각을 건너뛰면 이어 붙인 결과가 전체와 달라져
    // 여기서 실패한다. 크기 때문에 나누는 일이 **사용자의 문장을 조용히 지우는 일**이 되지
    // 않게 하는 것이 이 검사의 전부다 (`export::portion`의 재조립 동등성).
    let app = App::new("portions-reassemble");
    let spoken = sentences(LONG_SEGMENT_COUNT);
    app.save_recording_with_transcript("rec-long", "3DGS Study #04", &spoken);

    let first = app
        .storage
        .ai_prompt("rec-long", NoteType::Meeting, Some(1))
        .expect("첫 조각을 가져올 수 있어야 한다");
    let count = first.portion.portion_count;
    assert!(count > 1, "사전 조건: 이 산출물은 나뉘어야 한다");

    let mut joined = String::new();
    for index in 1..=count {
        let taken = app
            .storage
            .ai_prompt("rec-long", NoteType::Meeting, Some(index))
            .expect("조각 하나를 가져올 수 있어야 한다");

        // 요청한 조각이 그 조각으로 오고, 전체에 대한 말은 조각마다 달라지지 않는다.
        assert_eq!(taken.portion.portion, index);
        assert_eq!(taken.portion.portion_count, count);
        assert_eq!(taken.portion.total_size, first.portion.total_size);
        assert_eq!(
            taken.portion.portion_size,
            TextSizePayload::from(measure(&taken.text)),
            "조각 {index}: 말한 크기와 실제 크기가 다르다"
        );

        joined.push_str(&taken.text);
    }

    // **이어 붙이면 전체다.** 바이트 · 글자 · 줄 셋 다 전체와 같아야 한다 — 하나라도 어긋나면
    // 나눔이 내용을 바꾼 것이다.
    assert_eq!(
        measure(&joined).bytes,
        first.portion.total_size.bytes,
        "이어 붙인 결과의 크기가 전체와 다르다"
    );
    assert_eq!(measure(&joined).chars, first.portion.total_size.chars);
    assert_eq!(measure(&joined).lines, first.portion.total_size.lines);

    // 크기가 같다고 내용이 같은 것은 아니다 — 문장이 하나도 빠지지 않았고 순서도 그대로다.
    assert_sentences_in_order(&joined, spoken.len());
}

#[test]
fn tr4_every_portion_of_the_exported_document_is_its_own_file_and_reassembles() {
    // 무엇이 깨지면 이 검사가 잡는가: **나뉜 문서를 파일로 꺼내는 길이 깨지면** 잡는다 —
    // 조각마다 파일이 하나씩 생기지 않거나, 뒤 조각이 앞 파일을 덮어쓰거나, 파일에 담긴 것이
    // 그 조각이 아니면 여기서 실패한다. 사람이 실제로 AI에 첨부하는 것은 이 파일들이며,
    // 덮어쓰기는 사용자의 문서를 지우는 일이다 (ADR-0009 §4.3).
    let app = App::new("exported-portions");
    let spoken = sentences(LONG_SEGMENT_COUNT);
    app.save_recording_with_transcript("rec-long", "3DGS Study #04", &spoken);

    let first = app.export_ai_request("rec-long", 1);
    let count = first.portion.portion_count;
    assert!(count > 1, "사전 조건: 이 문서는 나뉘어야 한다");

    let mut paths: Vec<String> = vec![first.file.path.clone()];
    let mut joined = std::fs::read_to_string(&first.file.path).expect("쓰인 파일을 읽을 수 있어야 한다");

    for index in 2..=count {
        let written = app.export_ai_request("rec-long", index);

        assert_eq!(written.portion.portion, index);
        assert_eq!(written.portion.portion_count, count);
        assert!(
            written.file.file_name.contains(&format!("-part-{index}-of-{count}")),
            "파일 이름이 그 조각의 자리를 말하지 않는다: {}",
            written.file.file_name
        );
        assert!(
            !paths.contains(&written.file.path),
            "앞서 쓴 파일을 덮어썼다: {}",
            written.file.path
        );

        let body = std::fs::read_to_string(&written.file.path).expect("쓰인 파일을 읽을 수 있어야 한다");
        assert_eq!(
            measure(&body).bytes,
            written.portion.portion_size.bytes,
            "조각 {index}: 파일에 담긴 크기가 말한 크기와 다르다"
        );

        paths.push(written.file.path.clone());
        joined.push_str(&body);
    }

    // 앞서 쓴 파일은 전부 그 자리에 그대로 있다.
    for path in &paths {
        assert!(Path::new(path).is_file(), "{path} 가 사라졌다");
    }

    // 그리고 조각 파일을 순서대로 이어 붙이면 문서 전체다.
    assert_eq!(
        measure(&joined).bytes,
        first.portion.total_size.bytes,
        "파일들을 이어 붙인 결과가 문서 전체와 다르다"
    );
    assert_sentences_in_order(&joined, spoken.len());
}

#[test]
fn tr4_asking_for_a_portion_that_does_not_exist_is_a_failure_not_an_empty_answer() {
    // 무엇이 깨지면 이 검사가 잡는가: **없는 조각에 빈 텍스트로 답하게 되면** 잡는다. 조각 수는
    // 산출물이 달라지면 함께 달라지므로(재전사 뒤에는 더 짧을 수 있다), 빈 답을 받은 사용자는
    // 그것을 "다 가져갔다"로 읽는다 — 그것이 크기 때문에 조용히 실패하는 모습이다.
    let app = App::new("no-such-portion");
    app.save_recording_with_transcript("rec-long", "3DGS Study #04", &sentences(LONG_SEGMENT_COUNT));

    let count = app
        .storage
        .ai_prompt("rec-long", NoteType::Meeting, None)
        .expect("첫 조각")
        .portion
        .portion_count;

    for requested in [0, count + 1] {
        let failure = app
            .storage
            .ai_prompt("rec-long", NoteType::Meeting, Some(requested))
            .expect_err("없는 조각을 달라는 요청은 실패다");

        assert_eq!(failure.kind, FailureKind::InvalidInput, "{requested}");
        assert!(!failure.retryable, "{requested}: 같은 요청은 같은 답이다");
        assert!(
            failure.source_data_safe,
            "{requested}: 가져가려던 일이 저장된 것을 건드렸다 (INV-3)"
        );
    }

    // 파일을 쓰는 쪽도 같다 — 없는 조각을 요청했다고 빈 파일이 export 디렉터리에 쌓이지 않는다.
    let before = exported_files(&app);
    app.exporter
        .export_ai_request("rec-long", NoteType::Meeting, Some(count + 1))
        .expect_err("없는 조각은 파일이 되지 않는다");
    assert_eq!(exported_files(&app), before, "거절이 파일을 만들었다");
}

// --- 도구 ------------------------------------------------------------------------------

/// 되찾을 수 있는 표식이 붙은 문장들. 번호가 네 자리라 서로 접두사가 되지 않는다.
fn sentences(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| {
            format!(
                "{SENTENCE_MARKER}{index:04} 그러면 이번에는 PLY를 먼저 변환하고 결과를 비교한다."
            )
        })
        .collect()
}

/// 이어 붙인 결과에 문장이 **하나도 빠지지 않았고 순서도 그대로인가.**
///
/// 크기 비교만으로는 부족하다 — 같은 바이트 수로 순서가 뒤집힐 수 있고, 그때 사용자가 받는
/// 것은 읽을 수 없는 회의록이다.
fn assert_sentences_in_order(joined: &str, count: usize) {
    let mut previous = 0_usize;

    for index in 0..count {
        let marker = format!("{SENTENCE_MARKER}{index:04}");
        let occurrences = joined.matches(&marker).count();
        assert_eq!(occurrences, 1, "{marker} 가 {occurrences}번 나타난다");

        let at = joined.find(&marker).expect("있어야 한다");
        assert!(at >= previous, "{marker} 의 자리가 앞 문장보다 앞이다");
        previous = at;
    }
}

/// 지금 export 디렉터리에 있는 파일 이름들. 순서를 고정해 비교할 수 있게 정렬한다.
fn exported_files(app: &App) -> Vec<String> {
    let directory = app.app_data_dir.exports_dir();
    if !directory.is_dir() {
        return Vec::new();
    }

    let mut names: Vec<String> = std::fs::read_dir(&directory)
        .expect("export 디렉터리를 읽을 수 있어야 한다")
        .map(|entry| {
            entry
                .expect("항목을 읽을 수 있어야 한다")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
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
