//! command 경계의 계약: **초기화 실패가 앱을 죽이지 않고 사용자에게 도달한다** (§13),
//! 그리고 Phase 1이 노출하는 동작(recording CRUD · settings)이 실제로 동작한다.
//!
//! 여기에 Phase 5의 **Notion 표면 여섯**이 더해진다 (§5-D · §10 ·
//! `docs/ADR-0009-notion-and-export.md` §8 · §10). 이 파일이 판정하는 것은 전송의 순서가 아니라
//! **경계가 무엇을 답하는가**다 — 순서는 `tests/notion_sync.rs`가 본다.
//!
//! ```text
//! 1. 아직 설정하지 않은 것은 실패가 아니라 상태다             (INV-8)
//! 2. 연결 확인이 성공 · 인증 실패 · 권한 없는 destination을 구분한다 (§5-D · §13)
//! 3. token은 저장 command의 입력으로만 지나가고, 답은 '있다/없다'뿐이다 (INV-7)
//! 4. 이미 보내는 중일 때의 두 번째 요청은 조용히 사라지지 않는다
//! 5. 저장된 전송 기록은 부분 전송까지 그대로 읽힌다            (ADR-0009 §8.4)
//! ```
//!
//! **어떤 테스트도 실제 Notion에 요청하지 않고, 실제 자격증명 저장소도 열지 않는다** —
//! HTTP 왕복은 `notion::testing::StubServer` 뒤에 있고 token은 메모리 안에만 있다 (§18).
//!
//! 이 테스트는 임시 디렉터리 하나만 있으면 돌아간다. Tauri 런타임도, 창도, 하드웨어도
//! 필요하지 않다 (§18) — [`Storage`]가 Tauri 없이 열리도록 만들어졌기 때문이다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다. 여기에 SQL은 한 줄도 없다.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use molt_note_lib::commands::{
    NewRecording, NotionSendStatusPayload, NotionSender, SettingsPayload, Storage,
};
use molt_note_lib::db::{self, store};
use molt_note_lib::domain::{
    Failure, FailureKind, NotionSync, ProcessingStatus, RecordingId, Transcript, TranscriptId,
    TranscriptSegment,
};
use molt_note_lib::notion::testing::{StubReply, StubServer};
use molt_note_lib::notion::wire::ApiErrorCode;
use molt_note_lib::notion::TransportError;
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::secret_store::testing::InMemorySecretStore;
use molt_note_lib::platform::secret_store::{Secret, SecretKey, SecretStore};
use molt_note_lib::sync::pace::testing::RecordedWaits;
use molt_note_lib::sync::run::Confirmation;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 경로. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-command-test-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 정상적으로 열린 저장소.
fn open_storage(temp: &TempRoot) -> Storage {
    let storage = Storage::open(&AppDataDirectory::new(temp.path().join("app-data")));
    assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");
    storage
}

/// 저장할 수 있는 최소한의 녹음 하나. 오디오 파일은 없어도 된다.
fn a_recording(title: &str) -> NewRecording {
    NewRecording {
        title: title.to_string(),
        duration_ms: 3_151_000,
        audio_path: "recordings/sample.wav".to_string(),
        audio_format: "wav".to_string(),
        microphone: None,
    }
}

// --- 초기화 실패 (§13) --------------------------------------------------------------

#[test]
fn a_storage_that_cannot_be_opened_reports_the_failure_instead_of_killing_the_app() {
    // 실제 실패를 만든다: 앱 데이터 디렉터리가 있어야 할 자리에 파일이 있다.
    let temp = TempRoot::new("blocked");
    let occupied = temp.path().join("app-data");
    std::fs::write(&occupied, "디렉터리가 아니다").expect("사전 조건: 파일을 둔다");

    // 이 호출이 panic하면 테스트가 실패한다 — 그것이 이 테스트의 첫 번째 주장이다.
    let storage = Storage::open(&AppDataDirectory::new(&occupied));

    let failure = storage.failure().expect("실패가 앱 상태로 남아 있어야 한다");
    assert_eq!(failure.kind, FailureKind::Storage);
    assert!(!failure.message.is_empty(), "보여줄 문장이 있어야 한다");
    assert!(failure.source_data_safe, "원본 데이터는 안전하다");
    assert!(failure.retryable, "조건이 바뀌면 다시 시도할 수 있다");
}

#[test]
fn every_command_returns_the_initialization_failure_rather_than_pretending_to_work() {
    let temp = TempRoot::new("every-command");
    let occupied = temp.path().join("app-data");
    std::fs::write(&occupied, "디렉터리가 아니다").expect("사전 조건: 파일을 둔다");
    let storage = Storage::open(&AppDataDirectory::new(&occupied));
    let initial = storage.failure().expect("사전 조건: 초기화가 실패한다").clone();

    // 여섯 개 command 전부가 같은 실패를 돌려준다. 빈 목록이나 기본값으로 얼버무리지 않는다.
    let list = storage.list_recordings().expect_err("목록 조회가 실패해야 한다");
    let single = storage.recording("어떤-id").expect_err("단건 조회가 실패해야 한다");
    let created = storage
        .create_recording(a_recording("3DGS Study #04"))
        .expect_err("생성이 실패해야 한다");
    let deleted = storage
        .delete_recording("어떤-id")
        .expect_err("삭제가 실패해야 한다");
    let settings = storage.settings().expect_err("설정 조회가 실패해야 한다");
    let updated = storage
        .update_settings(SettingsPayload {
            recordings_directory: Some("/tmp/recordings".to_string()),
            automatic_processing: true,
            automatic_transcription: false,
            transcription_model: None,
            default_microphone: None,
            ai_provider: None,
            ai_base_url: None,
            ai_model: None,
            notion_parent_page_id: None,
        })
        .expect_err("설정 갱신이 실패해야 한다");

    for failure in [list, single, created, deleted, settings, updated] {
        assert_eq!(
            failure, initial,
            "초기화 실패가 그대로 전달돼야 한다 — 다른 말로 바꾸지 않는다"
        );
    }
}

#[test]
fn a_database_newer_than_the_app_is_reported_as_a_failure_retrying_will_not_fix() {
    let temp = TempRoot::new("ahead");
    let root = temp.path().join("app-data");
    let app_data_dir = AppDataDirectory::new(&root);

    // 이 앱보다 새로운 스키마의 DB를 만든다.
    app_data_dir.ensure().expect("사전 조건: 디렉터리를 만든다");
    {
        let connection = db::open_in(&app_data_dir).expect("사전 조건: DB를 만든다");
        let future = db::migrations::latest_version() + 100;
        connection
            .pragma_update(None, "user_version", future)
            .expect("사전 조건: 미래 버전을 흉내낸다");
        connection
            .close()
            .map_err(|(_, error)| error)
            .expect("사전 조건: 연결을 닫는다");
    }

    let storage = Storage::open(&app_data_dir);

    let failure = storage.failure().expect("실패가 앱 상태로 남아 있어야 한다");
    assert_eq!(failure.kind, FailureKind::Storage);
    assert!(
        !failure.retryable,
        "같은 앱으로 다시 시도해도 결과가 같다: {}",
        failure.message
    );
    assert!(failure.source_data_safe, "DB를 지우거나 되돌리지 않았다");

    // 실패한 뒤에도 DB 파일은 그대로 있다 — 실패가 사용자 데이터를 없애지 않는다.
    assert!(app_data_dir.database_path().is_file());
}

// --- 잘못된 입력은 저장소 실패와 구분된다 -------------------------------------------

#[test]
fn a_blank_title_is_reported_as_invalid_input_not_as_a_storage_failure() {
    let temp = TempRoot::new("blank-title");
    let storage = open_storage(&temp);

    let failure = storage
        .create_recording(a_recording("   "))
        .expect_err("제목 없는 녹음은 저장되지 않는다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(!failure.retryable, "같은 값으로 다시 보내도 결과가 같다");
    assert_eq!(
        storage.list_recordings().expect("목록을 읽을 수 있어야 한다").len(),
        0,
        "거절된 입력은 저장소에 남지 않는다"
    );
}

#[test]
fn a_negative_duration_is_rejected_before_it_reaches_the_store() {
    let temp = TempRoot::new("negative");
    let storage = open_storage(&temp);
    let mut recording = a_recording("음수 길이");
    recording.duration_ms = -1;

    let failure = storage
        .create_recording(recording)
        .expect_err("존재할 수 없는 길이는 저장되지 않는다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
}

// --- Phase 1의 동작 ------------------------------------------------------------------

#[test]
fn a_created_recording_comes_back_from_the_list_and_from_a_single_lookup() {
    let temp = TempRoot::new("crud");
    let storage = open_storage(&temp);

    let created = storage
        .create_recording(a_recording("3DGS Study #04"))
        .expect("녹음을 저장할 수 있어야 한다");

    assert!(!created.id.is_empty(), "식별자는 Rust가 만든다");
    assert_eq!(created.title, "3DGS Study #04");
    assert_eq!(
        created.duration_label, "52:31",
        "표시용 길이를 Rust가 만들어 보낸다 — frontend가 다시 계산하지 않는다"
    );
    assert_eq!(created.transcription_status, "none");
    assert_eq!(created.ai_status, "none");
    assert_eq!(created.notion_status, "none");
    assert_eq!(created.current_transcript_id, None, "아직 전사가 없다");
    assert!(
        created.created_at.starts_with(char::is_numeric) && created.created_at.ends_with('Z'),
        "생성 시각은 ISO-8601 UTC 텍스트다: {}",
        created.created_at
    );

    let listed = storage.list_recordings().expect("목록을 읽을 수 있어야 한다");
    assert_eq!(listed, vec![created.clone()]);

    let found = storage
        .recording(&created.id)
        .expect("단건 조회가 성공해야 한다");
    assert_eq!(found, Some(created));
}

#[test]
fn two_recordings_get_different_identities() {
    let temp = TempRoot::new("identities");
    let storage = open_storage(&temp);

    let first = storage
        .create_recording(a_recording("첫 번째"))
        .expect("저장할 수 있어야 한다");
    let second = storage
        .create_recording(a_recording("두 번째"))
        .expect("저장할 수 있어야 한다");

    assert_ne!(first.id, second.id);
    assert_eq!(
        storage.list_recordings().expect("목록을 읽는다").len(),
        2,
        "두 번째 저장이 첫 번째를 덮어쓰지 않는다"
    );
}

#[test]
fn looking_up_an_unknown_recording_is_an_empty_answer_not_a_failure() {
    let temp = TempRoot::new("unknown");
    let storage = open_storage(&temp);

    assert_eq!(
        storage.recording("없는-id").expect("조회 자체는 성공한다"),
        None
    );
    assert!(
        !storage.delete_recording("없는-id").expect("삭제 자체는 성공한다"),
        "지울 것이 없었다는 사실을 그대로 알린다"
    );
}

#[test]
fn deleting_a_recording_removes_it_from_the_list() {
    let temp = TempRoot::new("delete");
    let storage = open_storage(&temp);
    let created = storage
        .create_recording(a_recording("지울 녹음"))
        .expect("저장할 수 있어야 한다");

    assert!(storage
        .delete_recording(&created.id)
        .expect("삭제가 성공해야 한다"));

    assert!(storage.list_recordings().expect("목록을 읽는다").is_empty());
    assert_eq!(storage.recording(&created.id).expect("조회한다"), None);
}

// --- 저장된 Transcript 읽기 (phase-prompt/03 요구 6) ----------------------------------

#[test]
fn a_stored_transcript_comes_back_through_the_command_surface_with_its_segments() {
    // 화면이 Transcript 탭에 무엇을 그릴 수 있는지가 여기서 정해진다 — segment마다
    // **밀리초 두 개와 문장**이다. 단위를 여기서 다시 바꾸지 않는다 (§14.4.1의 ×10 · ×100 사고).
    let temp = TempRoot::new("transcript");
    let app_data_dir = AppDataDirectory::new(temp.path().join("app-data"));
    let storage = Storage::open(&app_data_dir);
    assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

    let recording = storage
        .create_recording(a_recording("전사된 녹음"))
        .expect("녹음을 저장할 수 있어야 한다");

    // 전사 경로가 하는 일을 저장소 API로 그대로 재현한다 — 이 테스트가 보는 것은 읽기 쪽이다.
    let mut connection = db::open_in(&app_data_dir).expect("사전 조건: 같은 DB를 연다");
    let transcript = Transcript {
        id: TranscriptId::new("t-1"),
        recording_id: RecordingId::new(&recording.id),
        language: Some("ko".to_string()),
        segments: vec![
            TranscriptSegment {
                start_ms: 134_000,
                end_ms: 141_000,
                text: "그러면 이번에는 PLY 먼저 변환하고".to_string(),
            },
            TranscriptSegment {
                start_ms: 141_000,
                end_ms: 148_500,
                text: "그다음 SOG 변환 확인하면 될 것 같아요.".to_string(),
            },
        ],
        raw_text: "그러면 이번에는 PLY 먼저 변환하고\n그다음 SOG 변환 확인하면 될 것 같아요.".to_string(),
        created_at: "2026-09-03T04:50:26.000Z".to_string(),
        engine: "stub".to_string(),
        model: "ggml-base.bin".to_string(),
    };
    store::append_transcript(&mut connection, &transcript).expect("사전 조건: Transcript를 남긴다");

    let found = storage
        .transcript("t-1")
        .expect("조회가 성공해야 한다")
        .expect("저장한 Transcript가 있어야 한다");

    assert_eq!(found.id, "t-1");
    assert_eq!(found.recording_id, recording.id);
    assert_eq!(found.language.as_deref(), Some("ko"));
    assert_eq!(found.engine, "stub", "무엇으로 만들어졌는지가 함께 온다 (§7)");
    assert_eq!(found.model, "ggml-base.bin");

    // 순서도 밀리초 값도 저장된 그대로다.
    assert_eq!(
        found
            .segments
            .iter()
            .map(|segment| (segment.start_ms, segment.end_ms))
            .collect::<Vec<_>>(),
        vec![(134_000, 141_000), (141_000, 148_500)]
    );
    assert_eq!(found.segments[0].text, "그러면 이번에는 PLY 먼저 변환하고");
}

#[test]
fn looking_up_an_unknown_transcript_is_an_empty_answer_not_a_failure() {
    // Recording에 아직 Transcript가 없는 것은 정상 상태다 (§7.2) — 실패로 만들지 않는다.
    let temp = TempRoot::new("transcript-unknown");
    let storage = open_storage(&temp);

    assert_eq!(storage.transcript("없는-id").expect("조회 자체는 성공한다"), None);
}

#[test]
fn an_empty_store_answers_with_an_empty_list_and_the_default_settings() {
    let temp = TempRoot::new("empty");
    let storage = open_storage(&temp);

    assert!(
        storage.list_recordings().expect("목록을 읽는다").is_empty(),
        "아직 아무것도 녹음하지 않은 것은 오류가 아니다"
    );
    assert_eq!(
        storage.settings().expect("설정을 읽는다"),
        SettingsPayload {
            recordings_directory: None,
            automatic_processing: false,
            // 자동 전사도 기본값은 OFF다. 두 토글은 서로 다른 값이다.
            automatic_transcription: false,
            transcription_model: None,
            default_microphone: None,
            // AI provider를 고르지 않은 것도 기본값이자 정상 상태다 (ADR-0008 §11.1 · INV-8).
            ai_provider: None,
            ai_base_url: None,
            ai_model: None,
            // 어느 페이지 아래에 만들지 고르지 않은 것도 기본값이자 정상 상태다
            // (ADR-0009 §8.4 · INV-8).
            notion_parent_page_id: None,
        },
        "저장된 적이 없으면 기본값이다"
    );
}

#[test]
fn updated_settings_are_stored_and_read_back() {
    let temp = TempRoot::new("settings");
    let storage = open_storage(&temp);

    let saved = storage
        .update_settings(SettingsPayload {
            recordings_directory: Some("/Users/someone/Recordings".to_string()),
            automatic_processing: true,
            automatic_transcription: true,
            transcription_model: Some("ggml-base.bin".to_string()),
            default_microphone: Some("0:Studio Mic".to_string()),
            ai_provider: Some("some-provider".to_string()),
            ai_base_url: Some("http://127.0.0.1:9999".to_string()),
            ai_model: Some("some-model".to_string()),
            notion_parent_page_id: Some("some-parent-page".to_string()),
        })
        .expect("설정을 저장할 수 있어야 한다");

    assert_eq!(
        saved,
        storage.settings().expect("설정을 다시 읽는다"),
        "갱신이 돌려준 값과 다시 읽은 값이 같아야 한다"
    );
    assert_eq!(
        saved.recordings_directory.as_deref(),
        Some("/Users/someone/Recordings")
    );
    assert!(saved.automatic_processing);
    assert!(
        saved.automatic_transcription,
        "자동 전사 토글도 그대로 저장되고 그대로 돌아와야 한다"
    );
    assert_eq!(
        saved.transcription_model.as_deref(),
        Some("ggml-base.bin"),
        "고른 모델도 그대로 저장되고 그대로 돌아와야 한다"
    );
    assert_eq!(
        saved.default_microphone.as_deref(),
        Some("0:Studio Mic"),
        "고른 장치 키가 그대로 저장되고 그대로 돌아와야 한다"
    );
    assert_eq!(
        (
            saved.ai_provider.as_deref(),
            saved.ai_base_url.as_deref(),
            saved.ai_model.as_deref()
        ),
        (
            Some("some-provider"),
            Some("http://127.0.0.1:9999"),
            Some("some-model")
        ),
        "고른 AI 설정 셋도 그대로 저장되고 그대로 돌아와야 한다 (ADR-0008 §11.1)"
    );
}

#[test]
fn the_two_automatic_toggles_do_not_share_one_value() {
    // 하나를 켜고 다른 하나를 끈 채로 저장했을 때 그대로 돌아오지 않으면, 두 토글이 사실은
    // 한 값이라는 뜻이다.
    let temp = TempRoot::new("independent-toggles");
    let storage = open_storage(&temp);

    let processing_only = storage
        .update_settings(SettingsPayload {
            recordings_directory: None,
            automatic_processing: true,
            automatic_transcription: false,
            transcription_model: None,
            default_microphone: None,
            ai_provider: None,
            ai_base_url: None,
            ai_model: None,
            notion_parent_page_id: None,
        })
        .expect("설정을 저장할 수 있어야 한다");
    assert!(processing_only.automatic_processing);
    assert!(
        !processing_only.automatic_transcription,
        "후처리를 켜는 것이 자동 전사를 켜지 않는다"
    );

    let transcription_only = storage
        .update_settings(SettingsPayload {
            recordings_directory: None,
            automatic_processing: false,
            automatic_transcription: true,
            transcription_model: None,
            default_microphone: None,
            ai_provider: None,
            ai_base_url: None,
            ai_model: None,
            notion_parent_page_id: None,
        })
        .expect("설정을 저장할 수 있어야 한다");
    assert!(
        !transcription_only.automatic_processing,
        "자동 전사를 켜는 것이 후처리를 켜지 않는다"
    );
    assert!(transcription_only.automatic_transcription);
    assert_eq!(
        transcription_only.transcription_model, None,
        "모델을 고르지 않은 채로 자동 전사를 켤 수 있다 — 앱이 대신 고르지 않는다"
    );
}

#[test]
fn a_blank_directory_is_stored_as_not_chosen_rather_than_as_an_empty_path() {
    let temp = TempRoot::new("blank-directory");
    let storage = open_storage(&temp);

    let saved = storage
        .update_settings(SettingsPayload {
            recordings_directory: Some("   ".to_string()),
            automatic_processing: false,
            automatic_transcription: false,
            transcription_model: Some("  \n ".to_string()),
            default_microphone: Some("  ".to_string()),
            // 공백뿐인 AI 설정도 같은 규칙으로 '고르지 않음'이 된다.
            ai_provider: Some(" ".to_string()),
            ai_base_url: Some("   ".to_string()),
            ai_model: Some("\t".to_string()),
            // 공백뿐인 Notion destination도 같은 규칙으로 '고르지 않음'이 된다.
            notion_parent_page_id: Some("  ".to_string()),
        })
        .expect("설정을 저장할 수 있어야 한다");

    assert_eq!(
        saved.recordings_directory, None,
        "빈 값은 '아직 고르지 않음'과 같은 뜻이다 — 세 번째 상태를 만들지 않는다"
    );
    assert_eq!(
        saved.default_microphone, None,
        "빈 선택도 '아직 고르지 않음'이다 — 어떤 장치의 키도 아닌 값을 저장하지 않는다"
    );
    assert_eq!(
        saved.transcription_model, None,
        "공백뿐인 모델 값도 '아직 고르지 않음'이다 — 어떤 파일도 가리키지 않는 값을 저장하지 않는다"
    );
    assert_eq!(
        (saved.ai_provider, saved.ai_base_url, saved.ai_model),
        (None, None, None),
        "공백뿐인 AI 설정도 '아직 고르지 않음'이다"
    );
    assert_eq!(
        saved.notion_parent_page_id, None,
        "공백뿐인 Notion destination도 '아직 고르지 않음'이다"
    );
}

#[test]
fn a_model_that_is_not_there_is_stored_as_chosen_not_replaced() {
    // 저장 경로는 파일을 찾아보지 않는다. 그 자리에 파일이 없어도 **사용자가 고른 값 그대로**
    // 남는다 — 지금 쓸 수 있는지 말하는 것은 전사를 시작할 때다 (ADR-0007 §8.2.3).
    let temp = TempRoot::new("missing-model");
    let storage = open_storage(&temp);

    let saved = storage
        .update_settings(SettingsPayload {
            recordings_directory: None,
            automatic_processing: false,
            automatic_transcription: true,
            transcription_model: Some("  없는-모델.bin  ".to_string()),
            default_microphone: None,
            ai_provider: None,
            ai_base_url: None,
            ai_model: None,
            notion_parent_page_id: None,
        })
        .expect("설정을 저장할 수 있어야 한다");

    assert_eq!(
        saved.transcription_model.as_deref(),
        Some("없는-모델.bin"),
        "앞뒤 공백만 다듬고, 없는 파일이라고 해서 지우거나 다른 모델로 바꾸지 않는다"
    );
    assert!(
        saved.automatic_transcription,
        "모델을 찾지 못했다고 해서 토글이 뒤집히지 않는다"
    );
}

// --- Notion 표면 (§5-D · §10 · ADR-0009 §8 · §10) --------------------------------------

/// 이 파일이 쓰는 token. **실재하지 않는다** (ADR-0009 §10.5).
const NOT_A_REAL_TOKEN: &str = "ntn-command-boundary-double-value-not-a-real-credential";

/// 실제 Notion에도 실제 자격증명 저장소에도 닿지 않는 전송자 하나.
fn notion_sender(
    temp: &TempRoot,
    server: Arc<StubServer>,
    secrets: Arc<InMemorySecretStore>,
) -> NotionSender {
    NotionSender::with_transport(
        AppDataDirectory::new(temp.path().join("app-data")),
        server,
        secrets,
        // 이 파일의 어떤 테스트도 자지 않는다 — 대기는 기록만 된다.
        Arc::new(RecordedWaits::new()),
    )
}

/// token 하나가 이미 저장돼 있는 자격증명 저장소.
fn secrets_with_token() -> Arc<InMemorySecretStore> {
    let secrets = Arc::new(InMemorySecretStore::new());
    secrets
        .set(
            SecretKey::NotionIntegrationToken,
            &Secret::new(NOT_A_REAL_TOKEN),
        )
        .expect("사전 조건: token을 저장한다");
    secrets
}

#[test]
fn a_connection_test_without_a_stored_token_is_a_state_not_a_failure() {
    // INV-8 — 아직 설정하지 않은 것은 오류가 아니다. provider 상태 조회와 같은 규칙이며,
    // 화면은 이 값을 경고가 아니라 담담한 상태로 그린다.
    let temp = TempRoot::new("notion-not-configured");
    let server = Arc::new(StubServer::ready());
    let sender = notion_sender(
        &temp,
        server.clone(),
        Arc::new(InMemorySecretStore::new()),
    );

    let connection = sender
        .check_connection(None)
        .expect("연결 확인 자체는 성공한다");

    assert_eq!(connection.state, "notConfigured");
    assert!(!connection.token_stored);
    assert!(!connection.destination_configured, "부모 페이지도 고르지 않았다");
    assert_eq!(connection.failure, None, "설정하지 않은 것은 실패가 아니다");
    assert!(
        server.requests().is_empty(),
        "보낼 token이 없는데 요청이 나갔다"
    );
}

#[test]
fn a_connection_test_separates_a_rejected_token_from_a_destination_it_cannot_reach() {
    // §5-D · §13 — 사용자가 할 수 있는 일이 전부 다르다: token을 다시 넣는 것 · 부모 페이지를
    // integration에 공유하는 것 · 네트워크를 확인하는 것. 그 구분이 화면까지 도달해야 한다.
    let temp = TempRoot::new("notion-connection");

    let check = |reply: Option<StubReply>| {
        let server = Arc::new(match reply {
            Some(reply) => StubServer::ready().with_users_me(reply),
            None => StubServer::ready(),
        });
        let connection = notion_sender(&temp, server.clone(), secrets_with_token())
            .check_connection(Some("  parent-page-identifier  "))
            .expect("연결 확인 자체는 성공한다");

        assert_eq!(server.requests().len(), 1, "확인은 왕복 한 번이다");
        connection
    };

    let connected = check(None);
    assert_eq!(connected.state, "connected");
    assert!(connected.token_stored);
    assert!(connected.destination_configured);
    assert_eq!(connected.failure, None);

    let cases = [
        (
            StubReply::error(401, ApiErrorCode::Unauthorized),
            FailureKind::NotionAuthFailed,
            false,
        ),
        (
            StubReply::error(403, ApiErrorCode::RestrictedResource),
            FailureKind::NotionDestinationUnavailable,
            false,
        ),
        (
            StubReply::error(404, ApiErrorCode::ObjectNotFound),
            FailureKind::NotionDestinationUnavailable,
            false,
        ),
        (
            StubReply::Fail(TransportError::NotConnected),
            FailureKind::NotionRequestFailed,
            true,
        ),
    ];

    for (reply, expected, retryable) in cases {
        let connection = check(Some(reply));

        assert_eq!(connection.state, "failed");
        assert!(connection.token_stored, "저장돼 있다는 사실은 그대로다");
        let failure = connection.failure.expect("무엇이 달랐는지가 실려 온다");
        assert_eq!(failure.kind, expected);
        assert_eq!(failure.retryable, retryable);
        assert!(
            failure.source_data_safe,
            "확인 한 번이 저장된 것을 건드리지 않는다 (INV-3)"
        );
        assert!(
            !format!("{failure:?}").contains(NOT_A_REAL_TOKEN),
            "token이 실패 문장에 실렸다 (INV-7): {failure:?}"
        );
    }
}

#[test]
fn a_successful_connection_test_says_which_workspace_answered() {
    // §5-D — 성공이 "됐다" 한마디로 끝나면 사용자는 **어느 워크스페이스의 token을 넣었는지**
    // 알 수 없다. 그 이름은 확인이 말해 준 값이며, 이 경계는 해석하지 않고 옮긴다.
    let temp = TempRoot::new("notion-connected-workspace");
    let named = Arc::new(StubServer::ready().with_users_me(StubReply::Body(
        r#"{"object":"user","id":"bot-user","type":"bot","bot":{"workspace_name":"Ada의 워크스페이스"}}"#
            .to_owned(),
    )));

    let connection = notion_sender(&temp, named, secrets_with_token())
        .check_connection(Some("parent-page-identifier"))
        .expect("연결 확인 자체는 성공한다");

    assert_eq!(connection.state, "connected");
    assert_eq!(
        connection.workspace_name.as_deref(),
        Some("Ada의 워크스페이스")
    );

    // 말해 주지 않으면 **지어내지 않는다** — stub의 기본 응답에는 워크스페이스 이름이 없다.
    let unnamed = notion_sender(&temp, Arc::new(StubServer::ready()), secrets_with_token())
        .check_connection(Some("parent-page-identifier"))
        .expect("연결 확인 자체는 성공한다");

    assert_eq!(unnamed.state, "connected");
    assert_eq!(unnamed.workspace_name, None, "없는 이름을 만들어 냈다");
}

#[test]
fn a_stored_token_is_answered_as_a_fact_and_never_as_a_value() {
    // INV-7 · ADR-0009 §10.4 — 값이 이 경계를 지나는 방향은 하나다: 화면에서 자격증명
    // 저장소로. 돌아오는 길에는 '있다/없다'뿐이며, 그것은 wire 형식에서도 그렇다.
    let temp = TempRoot::new("notion-token");
    let secrets = Arc::new(InMemorySecretStore::new());
    let sender = notion_sender(&temp, Arc::new(StubServer::ready()), Arc::clone(&secrets));

    let saved = sender
        .save_token(&format!("  {NOT_A_REAL_TOKEN}\n"))
        .expect("저장할 수 있어야 한다");
    assert!(saved.stored);
    assert_eq!(
        serde_json::to_string(&saved).expect("직렬화된다"),
        r#"{"stored":true}"#,
        "저장 결과가 말하는 것은 '있다'뿐이다"
    );
    assert_eq!(
        secrets
            .stored(SecretKey::NotionIntegrationToken)
            .expect("값이 있어야 한다")
            .expose(),
        NOT_A_REAL_TOKEN,
        "붙여 넣은 값의 앞뒤 공백만 벗기고 그대로 저장한다"
    );

    // 저장된 뒤의 연결 확인도 값을 돌려주지 않는다 — 직렬화된 응답 어디에도 token이 없다.
    let connection = sender
        .check_connection(Some("parent-page-identifier"))
        .expect("연결 확인 자체는 성공한다");
    let wire = serde_json::to_string(&connection).expect("직렬화된다");
    assert!(!wire.contains(NOT_A_REAL_TOKEN), "{wire}");
    assert!(wire.contains(r#""tokenStored":true"#), "{wire}");

    let deleted = sender.delete_token().expect("지울 수 있어야 한다");
    assert!(!deleted.stored);
    assert!(secrets.is_empty(), "지운 뒤에는 담고 있는 것이 없다");
    assert!(
        !sender
            .delete_token()
            .expect("없던 것을 지우는 것은 실패가 아니다")
            .stored
    );
}

#[test]
fn a_blank_token_is_refused_without_echoing_what_was_sent() {
    let temp = TempRoot::new("notion-blank-token");
    let secrets = Arc::new(InMemorySecretStore::new());
    let sender = notion_sender(&temp, Arc::new(StubServer::ready()), Arc::clone(&secrets));

    let failure = sender.save_token("   ").expect_err("빈 값은 저장하지 않는다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(!failure.retryable, "같은 값을 다시 보내도 같다");
    assert_eq!(failure.detail, None, "입력값이 실패에 실리지 않는다 (INV-7)");
    assert!(secrets.is_empty(), "거절된 입력은 저장소에 남지 않는다");
}

/// token을 읽는 자리에서 **붙잡혀 있는** 자격증명 저장소.
///
/// 전송이 도는 한가운데를 만들어 내기 위한 것이다 — 그 상태에서 두 번째 시작 요청이 어떻게
/// 되는지가 판정 대상이다. 실제 저장소도 파일도 열지 않는다.
struct PausedSecretStore {
    released: Mutex<bool>,
    ready: Condvar,
    entered: Mutex<usize>,
}

impl PausedSecretStore {
    fn new() -> Self {
        Self {
            released: Mutex::new(false),
            ready: Condvar::new(),
            entered: Mutex::new(0),
        }
    }

    fn is_reading(&self) -> bool {
        *self.entered.lock().expect("잠금") > 0
    }

    fn release(&self) {
        *self.released.lock().expect("잠금") = true;
        self.ready.notify_all();
    }
}

impl SecretStore for PausedSecretStore {
    fn get(&self, _key: SecretKey) -> Result<Option<Secret>, Failure> {
        *self.entered.lock().expect("잠금") += 1;

        let mut released = self.released.lock().expect("잠금");
        while !*released {
            released = self.ready.wait(released).expect("잠금");
        }

        // 붙잡혀 있던 전송은 '아직 token을 저장하지 않았다'로 끝난다 — 그것도 상태값이다.
        Ok(None)
    }

    fn set(&self, _key: SecretKey, _secret: &Secret) -> Result<(), Failure> {
        Ok(())
    }

    fn delete(&self, _key: SecretKey) -> Result<(), Failure> {
        Ok(())
    }
}

#[test]
fn a_second_send_while_one_is_running_is_refused_instead_of_disappearing() {
    let temp = TempRoot::new("notion-second-send");
    let storage = open_storage(&temp);
    storage
        .update_settings(SettingsPayload {
            recordings_directory: None,
            automatic_processing: false,
            automatic_transcription: false,
            transcription_model: None,
            default_microphone: None,
            ai_provider: None,
            ai_base_url: None,
            ai_model: None,
            notion_parent_page_id: Some("parent-page-identifier".to_string()),
        })
        .expect("사전 조건: 보낼 자리를 고른다");
    let recording = storage
        .create_recording(a_recording("보낼 녹음"))
        .expect("사전 조건: 녹음을 저장한다");

    let secrets = Arc::new(PausedSecretStore::new());
    let sender = NotionSender::with_transport(
        AppDataDirectory::new(temp.path().join("app-data")),
        Arc::new(StubServer::ready()),
        Arc::clone(&secrets) as Arc<dyn SecretStore>,
        Arc::new(RecordedWaits::new()),
    );

    let accepted = NotionSendStatusPayload::from(
        sender
            .start(&recording.id, Confirmation::NotAsked)
            .expect("접수된다"),
    );
    assert_eq!(accepted.state, "running", "돌아오는 것은 접수 사실이다");
    assert_eq!(accepted.recording_id.as_deref(), Some(recording.id.as_str()));
    assert_eq!(accepted.page_id, None);
    assert!(!accepted.created_page);
    assert_eq!(accepted.failure, None);

    // 배경 스레드가 실제로 돌기 시작할 때까지 기다린다 — 그 순간이 '전송 한가운데'다.
    let deadline = Instant::now() + Duration::from_secs(10);
    while !secrets.is_reading() {
        assert!(Instant::now() < deadline, "배경 스레드가 시작되지 않았다");
        std::thread::sleep(Duration::from_millis(5));
    }

    let refused = sender
        .start(&recording.id, Confirmation::NotAsked)
        .expect_err("한 번에 한 건이다 — 두 번째 요청이 조용히 사라지지 않는다");
    assert_eq!(refused.kind, FailureKind::InvalidInput);
    assert!(refused.source_data_safe, "거절은 아무것도 건드리지 않았다");
    assert!(
        refused.detail.as_deref().is_some_and(|detail| detail.contains(&recording.id)),
        "무엇이 돌고 있는지 알려 준다: {:?}",
        refused.detail
    );

    // 상태 조회는 전송이 멈춰 있는 동안에도 즉시 답한다.
    let asked_at = Instant::now();
    let status = NotionSendStatusPayload::from(sender.status().expect("상태를 읽을 수 있다"));
    assert_eq!(status.state, "running");
    assert!(
        asked_at.elapsed() < Duration::from_secs(1),
        "상태 조회가 전송을 기다렸다"
    );

    secrets.release();

    // 끝나면 그 사실이 상태에 남는다. **token을 저장하지 않은 것은 command의 실패가 아니라
    // 상태값이다** (INV-8) — 시작은 접수됐고, 무엇이 남았는지는 여기에 실려 온다.
    let deadline = Instant::now() + Duration::from_secs(30);
    let finished = loop {
        let status = NotionSendStatusPayload::from(sender.status().expect("상태를 읽을 수 있다"));
        if status.state != "running" {
            break status;
        }
        assert!(Instant::now() < deadline, "전송이 끝나지 않았다");
        std::thread::sleep(Duration::from_millis(10));
    };

    assert_eq!(finished.state, "failed");
    let failure = finished.failure.expect("무엇이 남았는지 말한다");
    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(failure.retryable, "설정을 채우면 진행할 수 있다");
    assert!(failure.source_data_safe, "녹음도 전사도 그대로다 (INV-3)");
}

#[test]
fn a_recording_that_was_never_sent_has_no_stored_sync_and_that_is_not_a_failure() {
    let temp = TempRoot::new("notion-sync-unknown");
    let storage = open_storage(&temp);
    let recording = storage
        .create_recording(a_recording("보낸 적 없는 녹음"))
        .expect("사전 조건: 녹음을 저장한다");

    assert_eq!(
        storage.notion_sync(&recording.id).expect("조회 자체는 성공한다"),
        None
    );
    assert_eq!(storage.notion_sync("없는-id").expect("조회 자체는 성공한다"), None);
}

#[test]
fn a_partly_sent_recording_says_so_through_the_command_surface() {
    // ADR-0009 §8.4 — 실패한 요청은 세지 않으므로, 둘이 다르면 문서의 일부만 들어가 있다.
    // 화면이 그것을 말할 수 있으려면 두 수가 이 경계를 지나야 한다.
    let temp = TempRoot::new("notion-sync-partial");
    let app_data_dir = AppDataDirectory::new(temp.path().join("app-data"));
    let storage = Storage::open(&app_data_dir);
    assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

    let recording = storage
        .create_recording(a_recording("중간에 멈춘 전송"))
        .expect("사전 조건: 녹음을 저장한다");

    // 전송 경로가 하는 일을 저장소 API로 그대로 재현한다 — 이 테스트가 보는 것은 읽기 쪽이다.
    let connection = db::open_in(&app_data_dir).expect("사전 조건: 같은 DB를 연다");
    store::save_notion_sync(
        &connection,
        &NotionSync {
            recording_id: RecordingId::new(&recording.id),
            page_id: Some("created-page-identifier".to_string()),
            synced_at: None,
            status: ProcessingStatus::Failed,
            error: Some("두 번째 조각에서 멈췄다".to_string()),
            sent_chunks: Some(1),
            total_chunks: Some(3),
            content_fingerprint: Some("a-content-fingerprint".to_string()),
        },
    )
    .expect("사전 조건: 전송 상태를 남긴다");

    let found = storage
        .notion_sync(&recording.id)
        .expect("조회가 성공해야 한다")
        .expect("저장한 전송 상태가 있어야 한다");

    assert_eq!(found.recording_id, recording.id);
    assert_eq!(found.status, "failed");
    assert_eq!(found.page_id.as_deref(), Some("created-page-identifier"));
    assert_eq!(found.synced_at, None, "성공한 적이 없다");
    assert_eq!(found.error.as_deref(), Some("두 번째 조각에서 멈췄다"));
    assert_eq!(
        (found.sent_chunks, found.total_chunks),
        (Some(1), Some(3)),
        "부분 전송이 상태에서 드러난다"
    );

    // 지문은 화면으로 나가지 않는다 — 이어 붙여도 되는지 판정하는 자리는 backend 하나다.
    let wire = serde_json::to_string(&found).expect("직렬화된다");
    assert!(!wire.contains("a-content-fingerprint"), "{wire}");
}

#[test]
fn a_default_microphone_that_no_longer_exists_is_stored_as_chosen_not_replaced() {
    // 저장 경로는 장치 목록을 알지 않는다. 알아볼 수 없는 키가 와도 **다른 장치로 바꾸지
    // 않고** 받은 값 그대로 남긴다 — 그것이 사용자의 선택이기 때문이다.
    // 지금 그 장치가 있는지 말하는 것은 화면 쪽이다 (`src/screens/defaultMicrophone.ts`).
    let temp = TempRoot::new("missing-microphone");
    let storage = open_storage(&temp);

    let saved = storage
        .update_settings(SettingsPayload {
            recordings_directory: None,
            automatic_processing: false,
            automatic_transcription: false,
            transcription_model: None,
            default_microphone: Some("7:장치가 빠진 마이크".to_string()),
            ai_provider: None,
            ai_base_url: None,
            ai_model: None,
            notion_parent_page_id: None,
        })
        .expect("설정을 저장할 수 있어야 한다");

    assert_eq!(
        saved.default_microphone.as_deref(),
        Some("7:장치가 빠진 마이크"),
        "알아볼 수 없는 키를 조용히 지우거나 다른 값으로 바꾸지 않는다"
    );
}
