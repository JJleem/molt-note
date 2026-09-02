//! Recording 영속성 계층: 생성 · 목록 조회 · 단건 조회 · 삭제.
//!
//! 이 테스트는 **임시 디렉터리의 DB 파일 하나**만 있으면 돌아간다. 마이크도, 실제 오디오
//! 파일도, 사용자 앱 데이터 디렉터리도 필요하지 않다 (§18 — 하드웨어 경계와 core logic을
//! 분리해 둔 결과가 여기서 드러난다).
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::db::{self, store, DatabaseError};
use molt_note_lib::domain::{
    format_duration_ms, ProcessingStatus, Recording, RecordingId, RecordingView, Transcript,
    TranscriptId, TranscriptSegment,
};
use rusqlite::Connection;

/// 삭제 경로를 자동으로 부르는 코드가 없다는 것을 소스에서 확인하기 위한 대상.
const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const STORE_SOURCE: &str = include_str!("../src/db/store.rs");

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 빈 디렉터리. Drop 시 지운다.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-recording-repo-test-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
        Self(path)
    }

    fn database_path(&self) -> PathBuf {
        self.0.join("molt-note.db")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// 스키마가 최신인 빈 DB를 연다.
fn open(dir: &TempDir) -> Connection {
    db::open(dir.database_path()).expect("임시 DB를 열 수 있어야 한다")
}

/// 저장할 Recording 하나.
///
/// `audio_path`는 **존재하지 않는 경로**다. 저장소는 파일을 열지도 만들지도 않으므로
/// 그래도 모든 연산이 성공해야 한다.
fn recording(id: &str, created_at: &str, duration_ms: i64) -> Recording {
    Recording {
        id: RecordingId::new(id),
        title: format!("녹음 {id}"),
        created_at: created_at.to_string(),
        updated_at: created_at.to_string(),
        duration_ms,
        audio_path: format!("recordings/{id}.wav"),
        audio_format: "wav".to_string(),
        microphone: None,
        current_transcript_id: None,
        transcription_status: ProcessingStatus::None,
        ai_status: ProcessingStatus::None,
        notion_status: ProcessingStatus::None,
    }
}

fn ids_of(views: &[RecordingView]) -> Vec<&str> {
    views.iter().map(|view| view.id().as_str()).collect()
}

// --- 생성 · 단건 조회 -----------------------------------------------------------

#[test]
fn a_created_recording_is_read_back_by_its_id_with_a_human_readable_length() {
    let dir = TempDir::new("create");
    let connection = open(&dir);
    let saved = recording("rec-1", "2026-09-01T10:00:00Z", 3_151_000);

    store::insert_recording(&connection, &saved).expect("Recording을 만들 수 있어야 한다");

    let view = store::load_recording_view(&connection, &saved.id)
        .expect("읽을 수 있어야 한다")
        .expect("방금 만든 Recording이 있어야 한다");
    assert_eq!(view.recording, saved, "저장한 그대로 복원돼야 한다");
    assert_eq!(
        view.duration_label, "52:31",
        "조회 결과가 사람이 읽는 길이를 함께 준다 — UI가 다시 계산하지 않는다"
    );
    assert_eq!(view.duration_label, format_duration_ms(saved.duration_ms));
}

#[test]
fn loading_an_id_that_was_never_stored_returns_none_instead_of_failing() {
    let dir = TempDir::new("missing");
    let connection = open(&dir);
    store::insert_recording(&connection, &recording("rec-real", "2026-09-01T10:00:00Z", 60_000))
        .expect("Recording을 만들 수 있어야 한다");

    assert_eq!(
        store::load_recording_view(&connection, &RecordingId::new("rec-does-not-exist"))
            .expect("없는 id 조회 자체는 성공해야 한다"),
        None,
        "없는 id는 오류가 아니라 '값 없음'이다"
    );
    // 빈 저장소에서도 마찬가지다.
    let empty = TempDir::new("missing-empty");
    let empty_connection = open(&empty);
    assert_eq!(
        store::load_recording_view(&empty_connection, &RecordingId::new("rec-real"))
            .expect("빈 저장소 조회도 성공해야 한다"),
        None
    );
}

#[test]
fn creating_the_same_id_twice_fails_and_the_stored_record_survives() {
    let dir = TempDir::new("duplicate");
    let connection = open(&dir);
    let original = recording("rec-dup", "2026-09-01T10:00:00Z", 3_151_000);
    store::insert_recording(&connection, &original).expect("Recording을 만들 수 있어야 한다");

    let mut impostor = recording("rec-dup", "2026-09-02T10:00:00Z", 1_000);
    impostor.title = "덮어쓰기를 노린 녹음".to_string();
    let error = store::insert_recording(&connection, &impostor)
        .expect_err("같은 id로 다시 만드는 것은 실패해야 한다");
    assert!(
        matches!(&error, DatabaseError::Sql(source)
            if source.to_string().to_uppercase().contains("UNIQUE")),
        "identity 충돌로 실패해야 한다: {error}"
    );

    let after = store::load_recording(&connection, &original.id)
        .expect("읽을 수 있어야 한다")
        .expect("원본이 남아 있어야 한다");
    assert_eq!(after, original, "실패한 재작성이 원본을 바꾸지 않아야 한다");
    assert_eq!(
        store::list_recordings(&connection)
            .expect("목록을 읽을 수 있어야 한다")
            .len(),
        1,
        "실패한 시도가 행을 남기지 않아야 한다"
    );
}

// --- 목록 조회 -----------------------------------------------------------------

#[test]
fn the_list_is_empty_before_anything_is_recorded() {
    let dir = TempDir::new("empty-list");
    let connection = open(&dir);

    let listed = store::list_recordings(&connection).expect("빈 목록도 읽을 수 있어야 한다");

    assert!(listed.is_empty(), "아직 녹음이 없는 것은 정상 상태다");
}

#[test]
fn the_list_returns_every_recording_newest_first_with_its_length_label() {
    let dir = TempDir::new("list");
    let connection = open(&dir);
    // §5 A 화면의 예시 순서: 최근 녹음이 위에 온다.
    let older = recording("rec-old", "2026-08-31T09:00:00Z", 2_302_000);
    let newer = recording("rec-new", "2026-09-01T09:00:00Z", 3_151_000);
    let longest = recording("rec-long", "2026-08-30T09:00:00Z", 3_661_000);
    store::insert_recording(&connection, &older).expect("만들 수 있어야 한다");
    store::insert_recording(&connection, &newer).expect("만들 수 있어야 한다");
    store::insert_recording(&connection, &longest).expect("만들 수 있어야 한다");

    let listed = store::list_recordings(&connection).expect("목록을 읽을 수 있어야 한다");

    assert_eq!(ids_of(&listed), ["rec-new", "rec-old", "rec-long"]);
    assert_eq!(
        listed
            .iter()
            .map(|view| view.duration_label.as_str())
            .collect::<Vec<_>>(),
        ["52:31", "38:22", "1:01:01"],
        "목록 항목마다 사람이 읽는 길이가 함께 온다 (§5 A)"
    );
    assert_eq!(listed[0].recording, newer, "레코드 자체도 그대로 담긴다");
}

// --- 삭제 (명시적 요청 경로만) ---------------------------------------------------

#[test]
fn deleting_a_recording_removes_only_that_one() {
    let dir = TempDir::new("delete");
    let connection = open(&dir);
    let kept = recording("rec-keep", "2026-09-01T10:00:00Z", 3_151_000);
    let doomed = recording("rec-delete", "2026-09-01T11:00:00Z", 60_000);
    store::insert_recording(&connection, &kept).expect("만들 수 있어야 한다");
    store::insert_recording(&connection, &doomed).expect("만들 수 있어야 한다");

    let deleted = store::delete_recording(&connection, &doomed.id).expect("삭제할 수 있어야 한다");

    assert!(deleted, "요청한 Recording을 지웠음을 알려야 한다");
    assert_eq!(
        store::load_recording_view(&connection, &doomed.id).expect("읽을 수 있어야 한다"),
        None,
        "지운 Recording은 더 이상 조회되지 않는다"
    );
    let remaining = store::list_recordings(&connection).expect("목록을 읽을 수 있어야 한다");
    assert_eq!(ids_of(&remaining), ["rec-keep"], "다른 Recording은 남는다");
    assert_eq!(remaining[0].recording, kept);
}

#[test]
fn deleting_an_id_that_does_not_exist_changes_nothing() {
    let dir = TempDir::new("delete-missing");
    let connection = open(&dir);
    let kept = recording("rec-keep", "2026-09-01T10:00:00Z", 3_151_000);
    store::insert_recording(&connection, &kept).expect("만들 수 있어야 한다");

    let deleted = store::delete_recording(&connection, &RecordingId::new("rec-never-existed"))
        .expect("없는 id 삭제 자체는 성공해야 한다");

    assert!(!deleted, "지운 것이 없음을 알려야 한다");
    assert_eq!(
        ids_of(&store::list_recordings(&connection).expect("목록을 읽을 수 있어야 한다")),
        ["rec-keep"],
        "다른 Recording을 대신 지우지 않는다"
    );
}

#[test]
fn deleting_a_recording_does_not_remove_the_file_at_its_audio_path() {
    // INV-1 · INV-4: 저장소는 레코드만 지운다. 파일을 지우는 것은 별개의 명시적 행위다.
    // 아래 파일은 실제 오디오가 아니라 "이 경로의 파일을 건드리는가"만 보기 위한 자리표시다.
    let dir = TempDir::new("delete-file");
    let connection = open(&dir);
    let placeholder = dir.0.join("placeholder.bin");
    std::fs::write(&placeholder, b"not audio").expect("사전 조건: 자리표시 파일을 둔다");

    let mut saved = recording("rec-file", "2026-09-01T10:00:00Z", 3_151_000);
    saved.audio_path = placeholder.to_string_lossy().into_owned();
    store::insert_recording(&connection, &saved).expect("만들 수 있어야 한다");

    assert!(
        store::delete_recording(&connection, &saved.id).expect("삭제할 수 있어야 한다"),
        "레코드는 지워진다"
    );

    assert!(
        placeholder.is_file(),
        "audio_path가 가리키는 파일은 그대로 남아야 한다 (INV-1 · INV-4)"
    );
    assert_eq!(
        std::fs::read(&placeholder).expect("파일을 읽을 수 있어야 한다"),
        b"not audio",
        "내용도 바뀌지 않는다"
    );
}

#[test]
fn a_recording_that_has_a_transcript_is_not_deleted_silently() {
    // 파생 데이터가 딸린 Recording을 조용히 함께 지우지 않는다 (INV-2).
    let dir = TempDir::new("delete-with-transcript");
    let mut connection = open(&dir);
    let saved = recording("rec-tr", "2026-09-01T10:00:00Z", 3_151_000);
    store::insert_recording(&connection, &saved).expect("만들 수 있어야 한다");
    let transcript = Transcript {
        id: TranscriptId::new("tr-1"),
        recording_id: RecordingId::new("rec-tr"),
        language: Some("ko".to_string()),
        segments: vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 3_000,
            text: "지워지면 안 되는 전사".to_string(),
        }],
        raw_text: "지워지면 안 되는 전사".to_string(),
        created_at: "2026-09-01T10:05:00Z".to_string(),
        engine: "whisper.cpp".to_string(),
        model: "base".to_string(),
    };
    store::append_transcript(&mut connection, &transcript).expect("Transcript를 추가할 수 있어야 한다");

    let error = store::delete_recording(&connection, &saved.id)
        .expect_err("딸린 데이터가 있으면 삭제가 실패해야 한다");
    assert!(
        matches!(&error, DatabaseError::Sql(source)
            if source.to_string().to_uppercase().contains("FOREIGN KEY")),
        "참조 무결성이 막아야 한다: {error}"
    );

    // 실패한 삭제가 아무것도 훼손하지 않았다.
    assert_eq!(
        store::load_recording(&connection, &saved.id)
            .expect("읽을 수 있어야 한다")
            .expect("Recording이 남아 있어야 한다"),
        saved
    );
    assert_eq!(
        store::load_transcript(&connection, &transcript.id)
            .expect("읽을 수 있어야 한다")
            .expect("Transcript가 남아 있어야 한다"),
        transcript
    );
}

/// lib.rs를 command 등록 목록과 그 밖의 코드(앱이 스스로 실행하는 경로)로 나눈다.
///
/// 등록 목록에 이름이 있다는 것은 **사용자가 부를 수 있다**는 뜻일 뿐, 앱이 부른다는 뜻이
/// 아니다. INV-4가 금지하는 것은 후자이므로 둘을 갈라서 본다.
fn split_invoke_handler(source: &str) -> (&str, String) {
    let marker = "generate_handler![";
    let start = source.find(marker).expect("command 등록 목록이 있어야 한다");
    let body = start + marker.len();
    let end = body + source[body..].find(']').expect("등록 목록이 닫혀야 한다");

    (
        &source[body..end],
        format!("{}{}", &source[..start], &source[end..]),
    )
}

#[test]
fn nothing_deletes_a_recording_on_its_own() {
    // INV-4 · R-004: 삭제는 사용자가 명시적으로 요청했을 때만 일어난다.
    // 앱 시작 경로(lib.rs)에는 삭제 호출이 없어야 하고, 저장소의 DELETE는
    // 호출자가 준 id 하나만 지우는 문장 하나뿐이어야 한다.
    let (registered, startup) = split_invoke_handler(LIB_SOURCE);
    assert_eq!(
        registered.matches("delete_recording").count(),
        1,
        "삭제는 사용자가 부르는 command 하나로만 노출된다"
    );
    assert!(
        !startup.contains("delete_recording"),
        "앱 시작 경로가 Recording을 지우고 있다 (INV-4)"
    );

    let deletes: Vec<&str> = STORE_SOURCE
        .lines()
        .map(str::trim)
        .filter(|line| line.to_uppercase().contains("DELETE FROM"))
        .collect();
    assert_eq!(
        deletes.len(),
        1,
        "저장소에는 삭제 문장이 하나만 있어야 한다: {deletes:?}"
    );
    assert!(
        deletes[0].contains("DELETE FROM recordings WHERE id = ?1"),
        "삭제는 호출자가 준 id 하나만 지워야 한다: {}",
        deletes[0]
    );
}

// --- 재시작 후에도 남는다 ---------------------------------------------------------

#[test]
fn recordings_survive_closing_and_reopening_the_database() {
    let dir = TempDir::new("reopen");
    let path = dir.database_path();
    let first = recording("rec-a", "2026-09-01T10:00:00Z", 3_151_000);
    let second = recording("rec-b", "2026-08-31T10:00:00Z", 2_302_000);
    let removed = recording("rec-c", "2026-08-30T10:00:00Z", 60_000);

    {
        let connection = db::open(&path).expect("처음 열 수 있어야 한다");
        store::insert_recording(&connection, &first).expect("만들 수 있어야 한다");
        store::insert_recording(&connection, &second).expect("만들 수 있어야 한다");
        store::insert_recording(&connection, &removed).expect("만들 수 있어야 한다");
        // 사용자가 명시적으로 지운 것은 다시 열어도 돌아오지 않는다.
        assert!(store::delete_recording(&connection, &removed.id).expect("삭제할 수 있어야 한다"));
        connection
            .close()
            .map_err(|(_, error)| error)
            .expect("연결이 닫혀야 한다");
    }

    let reopened = db::open(&path).expect("다시 열 수 있어야 한다");

    let listed = store::list_recordings(&reopened).expect("목록을 읽을 수 있어야 한다");
    assert_eq!(ids_of(&listed), ["rec-a", "rec-b"]);
    assert_eq!(
        listed
            .iter()
            .map(|view| view.duration_label.as_str())
            .collect::<Vec<_>>(),
        ["52:31", "38:22"],
        "길이 표시도 그대로 다시 만들어진다"
    );
    let single = store::load_recording_view(&reopened, &first.id)
        .expect("읽을 수 있어야 한다")
        .expect("Recording이 남아 있어야 한다");
    assert_eq!(single.recording, first, "저장한 그대로 복원돼야 한다");
    assert_eq!(single.duration_label, "52:31");
    assert_eq!(
        store::load_recording_view(&reopened, &removed.id).expect("읽을 수 있어야 한다"),
        None,
        "지운 Recording은 재시작 후에도 없다"
    );
}

// --- 하드웨어·오디오 파일 없이 동작한다 (§18) ---------------------------------------

#[test]
fn every_operation_works_without_an_audio_file_or_a_microphone() {
    let dir = TempDir::new("no-hardware");
    let connection = open(&dir);
    let mut saved = recording("rec-no-audio", "2026-09-01T10:00:00Z", 3_151_000);
    saved.audio_path = dir.0.join("never-created.wav").to_string_lossy().into_owned();
    saved.microphone = None;
    assert!(
        !Path::new(&saved.audio_path).exists(),
        "사전 조건: 오디오 파일은 존재하지 않는다"
    );

    // 생성 · 단건 조회 · 목록 조회 · 삭제 네 연산 모두 파일 없이 성공한다.
    store::insert_recording(&connection, &saved).expect("만들 수 있어야 한다");
    assert!(store::load_recording_view(&connection, &saved.id)
        .expect("읽을 수 있어야 한다")
        .is_some());
    assert_eq!(
        store::list_recordings(&connection)
            .expect("목록을 읽을 수 있어야 한다")
            .len(),
        1
    );
    assert!(store::delete_recording(&connection, &saved.id).expect("삭제할 수 있어야 한다"));

    assert!(
        !Path::new(&saved.audio_path).exists(),
        "저장소는 오디오 파일을 만들지도 지우지도 않는다"
    );
}

#[test]
fn tests_never_write_outside_the_system_temp_directory() {
    let dir = TempDir::new("sandbox");
    assert!(
        dir.0.starts_with(std::env::temp_dir()),
        "테스트는 실제 사용자 앱 데이터 디렉터리를 건드리지 않는다"
    );
}
