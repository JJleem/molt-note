//! **이 Phase의 여덟 불변을 이미 만들어진 경로로 못박는다**
//! (`phase-prompt/05.5` 성공 기준 · `docs/ADR-0010-manual-ai-handoff.md` · PRODUCT-SPEC §13 · §18).
//!
//! Phase 4의 `tests/core_pipeline_without_ai.rs`와 Phase 5의
//! `tests/notion_and_export_invariants.rs`가 각자의 Phase에 대해 하는 일과 같은 자리다 —
//! **새 제품 코드를 만들지 않고**, 이미 있는 세 출구(Manual 프롬프트 · Transcript 텍스트 ·
//! AI-ready 문서 파일)를 그대로 지나면서 여덟 불변을 각각 실패할 수 있는 검사로 바꾼다.
//!
//! ```text
//! MH-1 · MH-2  provider가 하나도 없어도 셋이 전부 동작한다 (로컬 provider를 요구하지 않는다)
//!                mh1_mh2_all_three_outputs_are_produced_with_no_ai_provider_configured
//!                mh1_mh2_a_configured_but_absent_provider_changes_none_of_the_three
//!                mh1_mh2_the_command_boundary_cannot_read_a_provider_or_the_ai_settings
//! MH-3         복사와 export 경로에 네트워크로 나가는 코드가 없다
//!                mh3_no_code_on_the_copy_and_export_path_can_reach_the_network
//! MH-4         산출물에도 그 입력 타입에도 audio 경로도 바이트도 없다
//!                mh4_no_audio_path_and_no_audio_format_reaches_the_outputs_or_the_written_file
//!                mh4_the_types_on_this_path_have_no_place_to_carry_audio
//! MH-5         current가 가리키는 Transcript만 쓰인다
//!                mh5_the_three_outputs_follow_the_current_pointer_and_nothing_else
//!                mh5_no_code_on_this_path_can_pick_a_transcript_version_by_itself
//! MH-6         산출물과 core/domain · payload 타입에 벤더 이름도 벤더 schema도 없다
//!                mh6_no_vendor_name_reaches_the_outputs_or_the_written_file
//!                mh6_no_vendor_name_or_vendor_schema_is_in_the_domain_and_payload_types
//! MH-7         실패해도 저장된 것과 이미 내보낸 파일이 그대로다
//!                mh7_four_failures_leave_the_database_bytes_and_the_exported_files_untouched
//!                mh7_a_place_that_cannot_be_written_changes_nothing_either
//!                mh7_the_command_boundary_has_no_way_to_write_to_the_repository
//! MH-8         기존 Connected Provider 생성 경로가 그대로 동작한다
//!                mh8_the_connected_provider_path_still_makes_notes_next_to_the_manual_path
//! ```
//!
//! ## 이미 있는 검사를 다시 쓰지 않는다
//!
//! 아래는 **여기서 다시 쓰지 않고** 그 자리를 가리킨다. 같은 사실을 두 번 적으면 둘이 어긋날 때
//! 어느 쪽이 규칙인지 알 수 없게 된다.
//!
//! ```text
//! 세 산출물의 기대 문자열 · 프롬프트 상수 무손상   src/export/ai_request.rs 의 단위 테스트
//! 순수 모듈 둘의 소스 (provider · 네트워크 · 쓰기) tests/manual_ai_handoff.rs
//! 세 command가 transcriptId를 받지 않는다는 계약   tests/manual_ai_handoff.rs
//! 덮어쓰지 않는 쓰기 · 파일 이름 규칙              tests/markdown_export.rs · src/export/file.rs
//! adapter 밖에 벤더 지식이 없다 (INV-9)            tests/ollama_adapter.rs · tests/notion_adapter.rs
//! 네트워크에 닿을 수 있는 파일이 둘뿐이다 (§18)    tests/ollama_adapter.rs
//! 화면 쪽 원문 (clipboard 경계 · 벤더 · 네트워크)  tests/manual-handoff-invariants.test.ts ·
//!                                                 tests/screen-boundary.test.ts · tests/ipc-boundary.test.ts
//! ```
//!
//! ## 무엇을 쓰지 않는가 (§18 · `phase-prompt/05.5` Important Rules)
//!
//! **실제 AI provider에 한 번도 요청하지 않는다** — 연결된 provider 자리에는 계약이 같은
//! test double이 선다 (`ai::testing::FakeNoteAiProvider`). **실제 Notion도, 실제 OS 자격증명
//! 저장소도 세우지 않는다** — 이 경로는 그 둘을 알지 않으므로 세울 것도 없다. **시스템
//! clipboard를 건드리지 않는다** — clipboard에 쓰는 자리는 이 경계 밖(프론트엔드의 platform
//! 모듈)이며, 여기서 다루는 것은 문자열과 파일뿐이다. **사용자의 실제 디렉터리를 건드리지
//! 않는다** — 저장소도 export도 전부 `std::env::temp_dir()` 아래에서 만들어지고 Drop 때 지워진다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::ai::prompt::ContextBudget;
use molt_note_lib::ai::run;
use molt_note_lib::ai::testing::{FakeNoteAiProvider, FAKE_MODEL_ID, FAKE_PROVIDER_ID};
use molt_note_lib::commands::{ExportedFilePayload, Exporter, Storage};
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{
    AiNote, Failure, NoteType, ProcessingStatus, Recording, RecordingId, Settings, Transcript,
    TranscriptId, TranscriptSegment,
};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;

/// 이 파일이 쓰는 녹음 시각. 파일 이름의 날짜가 여기서 나온다 (ADR-0009 §4.2).
const CREATED_AT: &str = "2026-09-01T10:00:00.000Z";

/// 3151초 = `52:31`.
const DURATION_MS: i64 = 3_151_000;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 자리 ----------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-handoff-invariants-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("사전 조건: 빈 디렉터리를 만든다");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 저장소 하나와 그 위에서 도는 두 경계 — 문자열을 만드는 쪽과 파일을 쓰는 쪽.
struct Fixture {
    /// Drop 시 임시 디렉터리를 지운다.
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    storage: Storage,
    exporter: Exporter,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.path().join("app-data"));

        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

        Self {
            exporter: Exporter::in_directory(app_data_dir.clone()),
            storage,
            app_data_dir,
            _root: root,
        }
    }

    fn connection(&self) -> rusqlite::Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    fn exports_dir(&self) -> PathBuf {
        self.app_data_dir.exports_dir()
    }

    /// 녹음 하나를 저장한다. **오디오 파일은 만들지 않는다** — 이 경로는 그것을 읽지 않는다.
    fn save_recording(&self, id: &str, title: &str) -> RecordingId {
        self.save_recording_with_audio(id, title, &format!("recordings/{id}.wav"), "wav")
    }

    /// 오디오 경로와 형식을 골라 넣는다 — MH-4가 그 값이 어디에도 나오지 않는 것을 본다.
    fn save_recording_with_audio(
        &self,
        id: &str,
        title: &str,
        audio_path: &str,
        audio_format: &str,
    ) -> RecordingId {
        let recording = Recording {
            id: RecordingId::new(id),
            title: title.to_string(),
            created_at: CREATED_AT.to_string(),
            updated_at: CREATED_AT.to_string(),
            duration_ms: DURATION_MS,
            audio_path: audio_path.to_string(),
            audio_format: audio_format.to_string(),
            microphone: Some("가짜 마이크".to_string()),
            current_transcript_id: None,
            transcription_status: ProcessingStatus::Done,
            ai_status: ProcessingStatus::None,
            notion_status: ProcessingStatus::None,
        };
        store::insert_recording(&self.connection(), &recording)
            .expect("사전 조건: 녹음을 저장한다");

        recording.id
    }

    /// Transcript 하나를 추가한다. `current`면 그것을 current로 지정한다 (§7.2).
    fn save_transcript(
        &self,
        recording_id: &RecordingId,
        id: &str,
        sentence: &str,
        current: bool,
    ) -> TranscriptId {
        let transcript = Transcript {
            id: TranscriptId::new(id),
            recording_id: recording_id.clone(),
            language: Some("ko".to_string()),
            segments: vec![TranscriptSegment {
                start_ms: 3_000,
                end_ms: 6_000,
                text: sentence.to_string(),
            }],
            raw_text: sentence.to_string(),
            created_at: CREATED_AT.to_string(),
            engine: "stub".to_string(),
            model: "ggml-base.bin".to_string(),
        };

        let mut connection = self.connection();
        store::append_transcript(&mut connection, &transcript)
            .expect("사전 조건: 전사를 저장한다");
        if current {
            self.set_current(recording_id, Some(&transcript.id));
        }

        transcript.id
    }

    /// current 포인터를 옮긴다 (§7.2). **버전을 지우지 않는다 — 가리키는 것만 바꾼다.**
    fn set_current(&self, recording_id: &RecordingId, transcript: Option<&TranscriptId>) {
        store::set_current_transcript(&self.connection(), recording_id, transcript, CREATED_AT)
            .expect("사전 조건: current를 지정한다");
    }

    /// 연결된 provider로 노트 한 건을 만든다 (MH-8). **실제 AI 서버가 아니다** (§18).
    fn generate_note(&self, recording_id: &RecordingId, mode: NoteType) -> AiNote {
        let outcome = run::generate(
            &self.connection(),
            recording_id,
            mode,
            &FakeNoteAiProvider::ready(),
            ContextBudget::DEFAULT,
        )
        .expect("연결된 provider 경로가 노트를 만들 수 있어야 한다");

        outcome
            .generated()
            .cloned()
            .expect("노트 하나가 저장돼야 한다")
    }

    fn ai_prompt(&self, recording_id: &RecordingId, mode: NoteType) -> Result<String, Failure> {
        self.storage.ai_prompt(recording_id.as_str(), mode)
    }

    fn transcript_text(&self, recording_id: &RecordingId) -> Result<String, Failure> {
        self.storage.transcript_text(recording_id.as_str())
    }

    fn export_ai_request(
        &self,
        recording_id: &RecordingId,
        mode: NoteType,
    ) -> Result<ExportedFilePayload, Failure> {
        self.exporter.export_ai_request(recording_id.as_str(), mode)
    }

    /// 세 산출물을 한 값으로 만든다 — **같은 입력에서 언제나 같아야 하는 것**이다.
    fn three_outputs(&self, recording_id: &RecordingId, mode: NoteType) -> ThreeOutputs {
        let exported = self
            .export_ai_request(recording_id, mode)
            .expect("AI-ready 문서를 내보낼 수 있어야 한다");

        ThreeOutputs {
            prompt: self
                .ai_prompt(recording_id, mode)
                .expect("프롬프트를 만들 수 있어야 한다"),
            transcript_text: self
                .transcript_text(recording_id)
                .expect("전사를 복사할 수 있어야 한다"),
            document: fs::read_to_string(&exported.path).expect("내보낸 파일을 읽는다"),
        }
    }

    /// 저장된 것 전부를 한 값으로 찍는다 (INV-3 · MH-7).
    fn snapshot(&self, recording_id: &RecordingId) -> Snapshot {
        let connection = self.connection();
        let recording = store::load_recording(&connection, recording_id)
            .expect("녹음을 읽을 수 있어야 한다")
            .expect("녹음이 있어야 한다");
        let transcripts =
            store::list_transcripts(&connection, recording_id).expect("전사를 읽을 수 있어야 한다");
        let notes = transcripts
            .iter()
            .flat_map(|transcript| {
                store::list_ai_notes_for_transcript(&connection, &transcript.id)
                    .expect("노트를 읽을 수 있어야 한다")
            })
            .collect();

        Snapshot {
            recording,
            transcripts,
            notes,
        }
    }

    /// DB 파일의 **바이트 그대로**.
    ///
    /// domain 타입으로 비교하면 저장소가 복원하지 않는 열이 바뀌어도 알 수 없다. 여기서는
    /// 파일을 그대로 읽으므로 **한 바이트라도 달라지면 드러난다** (`tests/ai_note_run.rs`가
    /// 테이블 덤프로 하는 일을 파일 수준에서 하는 것이다).
    fn database_bytes(&self) -> Vec<u8> {
        fs::read(self.app_data_dir.database_path()).expect("DB 파일을 읽을 수 있어야 한다")
    }

    /// `exports/`에 지금 있는 파일 이름과 내용 전부. 디렉터리가 없으면 빈 목록이다.
    fn exported_files(&self) -> Vec<(String, String)> {
        let Ok(entries) = fs::read_dir(self.exports_dir()) else {
            return Vec::new();
        };

        let mut files: Vec<(String, String)> = entries
            .map(|entry| {
                let path = entry.expect("디렉터리 항목을 읽는다").path();
                (
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .expect("이름이 있어야 한다")
                        .to_string(),
                    fs::read_to_string(&path).expect("내보낸 파일을 읽을 수 있어야 한다"),
                )
            })
            .collect();
        files.sort();
        files
    }
}

/// 사람이 가져갈 수 있는 산출물 셋 한 벌.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ThreeOutputs {
    prompt: String,
    transcript_text: String,
    document: String,
}

impl ThreeOutputs {
    /// 셋을 차례로 훑는다 — 어느 하나만 지키는 불변은 이 Phase에 없다.
    fn all(&self) -> [&str; 3] {
        [&self.prompt, &self.transcript_text, &self.document]
    }
}

/// recording · transcript · ai_note를 한 번에 비교하기 위한 값 (INV-3 · MH-7).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    recording: Recording,
    transcripts: Vec<Transcript>,
    notes: Vec<AiNote>,
}

// --- 소스를 읽는 자리 ------------------------------------------------------------------

/// 이 저장소의 Rust 소스 하나.
fn source(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("소스 파일을 읽을 수 있어야 한다: {relative} ({error})"))
}

/// 실행되는 코드만 남긴다 — 주석과 `#[cfg(test)]` 아래는 제품 코드가 아니다.
///
/// 이 저장소의 주석은 전부 `//` · `///` · `//!`다. 블록 주석은 쓰지 않는다.
fn product_code(source: &str) -> String {
    let executable = match source.find("#[cfg(test)]") {
        Some(boundary) => &source[..boundary],
        None => source,
    };

    executable
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 선언 한 줄에서 시작해 그 블록이 닫힐 때까지의 원문.
///
/// 파일 전체가 아니라 **문제의 자리만** 본다 — `commands/mod.rs`는 이 Phase와 상관없는
/// command도 담고 있으므로, 파일 단위로 금지어를 찾으면 검사가 아무 뜻도 없게 된다.
fn block(source: &str, header: &str, close: &str) -> String {
    let start = source
        .find(header)
        .unwrap_or_else(|| panic!("{header}가 소스에 있어야 한다"));
    let rest = &source[start..];
    let end = rest
        .find(close)
        .unwrap_or_else(|| panic!("{header}의 블록이 닫혀야 한다"));

    rest[..end + close.len()].to_owned()
}

/// 복사와 export가 실제로 지나는 **command 경계**의 제품 코드 전부.
///
/// ```text
/// commands/export.rs                  Exporter — 자리를 준비하고 파일 하나를 쓴다
/// commands/mod.rs 의 다섯 자리        Storage의 두 메서드와 세 command 함수
/// ```
///
/// 순수 모듈 둘(`export/handoff.rs` · `export/ai_request.rs`)의 소스는
/// `tests/manual_ai_handoff.rs`가 같은 방식으로 본다 — 여기서 다시 보지 않는다.
fn command_boundary() -> String {
    let commands = product_code(&source("src/commands/mod.rs"));

    let mut boundary = product_code(&source("src/commands/export.rs"));
    for (header, close) in [
        ("    pub fn ai_prompt(", "\n    }\n"),
        ("    pub fn transcript_text(", "\n    }\n"),
        ("pub fn get_ai_prompt(", "\n}\n"),
        ("pub fn get_transcript_text(", "\n}\n"),
        ("pub fn export_ai_request(", "\n}\n"),
    ] {
        boundary.push('\n');
        boundary.push_str(&block(&commands, header, close));
    }
    boundary
}

/// 복사와 export가 지나는 **모든** 제품 소스 — 경계와 그 아래의 렌더링·쓰기까지.
fn whole_path_sources() -> Vec<(&'static str, String)> {
    [
        "src/export/mod.rs",
        "src/export/ai_request.rs",
        "src/export/handoff.rs",
        "src/export/run.rs",
        "src/export/file.rs",
        "src/export/filename.rs",
        "src/export/markdown.rs",
    ]
    .into_iter()
    .map(|relative| (relative, product_code(&source(relative))))
    .chain(std::iter::once(("command boundary", command_boundary())))
    .collect()
}

/// 산출물에도 타입에도 나오면 안 되는 AI 채팅 벤더의 이름 (MH-6 · §10).
///
/// **Notion은 여기 없다.** Notion은 고를 수 있는 provider가 아니라 이 제품이 보내기로 한
/// 목적지 그 자체이며 (PRODUCT-SPEC §10), 그 지식이 adapter 밖으로 나가지 않는다는 것은
/// `tests/notion_adapter.rs`가 따로 본다.
const VENDOR_NAMES: [&str; 9] = [
    "ChatGPT",
    "OpenAI",
    "Claude",
    "Anthropic",
    "Gemini",
    "Copilot",
    "Codex",
    "Ollama",
    "ollama",
];

/// 벤더 고유의 요청 모양 — 어느 벤더의 API를 아는지 드러내는 문자열 (MH-6 · INV-9).
///
/// 기본 주소 상수(`domain::settings::DEFAULT_AI_BASE_URL`)는 여기 없다. 그 값이 설정 한
/// 자리에만 있다는 것은 이미 `tests/ollama_adapter.rs`가 규칙으로 못박았고, 그것을 여기서
/// 뒤집지 않는다 — 이 검사가 보는 것은 **산출물과 그 타입이 벤더의 schema를 담는가**다.
const VENDOR_SCHEMA: [&str; 6] = [
    "/api/generate",
    "/api/tags",
    "num_ctx",
    "OLLAMA_HOST",
    "chat/completions",
    "\"choices\"",
];

// --- 원문 검사가 실제로 그 코드를 읽고 있는가 -------------------------------------------

#[test]
fn the_source_checks_actually_read_the_code_they_claim_to_read() {
    // **아무것도 읽지 못한 검사는 언제나 통과한다.** 슬라이스가 빗나가거나 주석 제거가 너무
    // 많이 지우면 아래의 모든 금지어 검사가 조용히 무의미해지므로, 그것을 여기서 먼저 막는다.
    let boundary = command_boundary();

    for marker in [
        "handoff::ai_prompt(",
        "handoff::transcript_text(",
        "export::handoff::export_ai_request(",
        "storage.ai_prompt(",
        "storage.transcript_text(",
        "exporter.export_ai_request(",
    ] {
        assert!(
            boundary.contains(marker),
            "command 경계를 읽지 못했다: {marker}"
        );
    }

    for (relative, code) in whole_path_sources() {
        assert!(
            code.len() > 200,
            "{relative}의 제품 코드를 읽지 못했다 ({}자)",
            code.len()
        );
    }

    // 주석 제거가 실제로 일어난다 — `ai_request.rs`는 오디오를 **주석과 테스트에서만** 말한다.
    // 그래서 이 파일은 원문에는 `audio_path`가 있고 제품 코드에는 없어야 하며, 그 둘이 다르다는
    // 사실이 MH-4의 소스 검사가 진짜 검사라는 증거다.
    let raw = source("src/export/ai_request.rs");
    assert!(raw.contains("audio_path"), "사전 조건: 원문에는 있다");
    assert!(
        !product_code(&raw).contains("audio_path"),
        "제품 코드에 audio_path가 있다 (MH-4)"
    );
}

// --- MH-1 · MH-2 — provider가 하나도 없어도 셋이 전부 동작한다 --------------------------

#[test]
fn mh1_mh2_all_three_outputs_are_produced_with_no_ai_provider_configured() {
    // Phase 5.5의 성공 기준 1 그 자체다 — **AI Provider가 하나도 없어도 AI의 값을 얻는다.**
    let fixture = Fixture::new("no-provider");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "안녕하세요. 오늘은 3DGS를 봅니다.", true);

    // 사전 조건: 고른 provider도, 모델도, 주소도 없다. 그것이 기본이자 정상 상태다 (INV-8).
    let stored = settings::load(&fixture.connection()).expect("설정을 읽을 수 있어야 한다");
    assert_eq!(stored, Settings::DEFAULT);
    assert!(stored.ai_provider.is_none(), "사전 조건: provider가 없다");
    assert!(stored.ai_model.is_none(), "사전 조건: 모델이 없다");
    assert!(stored.ai_base_url.is_none(), "사전 조건: 주소가 없다");

    for mode in NoteType::ALL {
        let outputs = fixture.three_outputs(&recording_id, mode);

        for text in outputs.all() {
            assert!(
                text.contains("안녕하세요. 오늘은 3DGS를 봅니다."),
                "{mode}: 전사 본문이 산출물에 있어야 한다"
            );
        }
        assert!(outputs.prompt.contains("Return the note as Markdown and nothing else."));
        assert!(outputs.document.starts_with("# Molt Note AI Request\n"));
    }
}

#[test]
fn mh1_mh2_a_configured_but_absent_provider_changes_none_of_the_three() {
    // 성공 기준 1의 다른 쪽 절반이다 — **로컬 provider를 요구하지 않는다.** 설정에 provider가
    // 적혀 있고 그 provider가 이 기기에 없더라도(주소가 닫혀 있어도) 세 산출물은 글자 하나
    // 달라지지 않는다. 이 경로가 그 값을 읽지도, 그 주소에 닿지도 않기 때문이다.
    let fixture = Fixture::new("absent-provider");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "먼저 splat 표현부터 보겠습니다.", true);

    let before = fixture.three_outputs(&recording_id, NoteType::Study);

    // 아무것도 듣고 있지 않은 주소다. **여기에 연결을 시도하는 코드가 있었다면** 이 테스트는
    // 실패하거나 멈춘다 — 그것이 이 사전 조건의 목적이다.
    settings::save(
        &fixture.connection(),
        &Settings {
            ai_provider: Some("a-provider-that-is-not-installed".to_string()),
            ai_base_url: Some("http://127.0.0.1:1".to_string()),
            ai_model: Some("a-model-that-is-not-here".to_string()),
            ..Settings::DEFAULT
        },
    )
    .expect("설정을 저장할 수 있어야 한다");

    let after = fixture.three_outputs(&recording_id, NoteType::Study);

    assert_eq!(after.prompt, before.prompt, "프롬프트가 설정을 탄다");
    assert_eq!(
        after.transcript_text, before.transcript_text,
        "전사 텍스트가 설정을 탄다"
    );
    assert_eq!(after.document, before.document, "AI-ready 문서가 설정을 탄다");
}

#[test]
fn mh1_mh2_the_command_boundary_cannot_read_a_provider_or_the_ai_settings() {
    // "그런 코드를 안 짰다"가 아니라 **읽을 수단이 없다**를 본다. provider를 읽는 줄이 하나라도
    // 있으면, provider가 없다는 이유로 거절하는 일은 언제든 한 줄로 가능해진다.
    //
    // 순수 모듈 둘은 `tests/manual_ai_handoff.rs`가 같은 방식으로 본다 — 여기는 그 위의
    // **command 경계**다.
    let boundary = command_boundary();
    let mut offenders: Vec<&str> = Vec::new();

    for forbidden in [
        "provider", "Provider", "ai_base_url", "ai_model", "settings", "Settings",
    ]
    .into_iter()
    .chain(VENDOR_NAMES)
    {
        if boundary.contains(forbidden) {
            offenders.push(forbidden);
        }
    }

    assert!(
        offenders.is_empty(),
        "Manual AI Handoff의 command 경계가 provider·AI 설정에 닿는다: {offenders:?}"
    );
}

// --- MH-3 — 나가는 행위의 주체는 사람이다 -----------------------------------------------

#[test]
fn mh3_no_code_on_the_copy_and_export_path_can_reach_the_network() {
    // 복사도 export도 **이 기기 안에서 끝나는 일이다** (ADR-0010 §5.1 · §7). 여기까지가 앱이
    // 하는 일이고, 그다음은 사용자가 자기 채팅에 붙여 넣거나 파일을 첨부하는 것이다.
    //
    // 저장소 전체에서 네트워크에 닿을 수 있는 파일이 둘뿐이라는 것은 `tests/ollama_adapter.rs`가
    // 본다. 여기서 보는 것은 **그 둘이 이 경로에 없다**는 것이다 — 목록이 늘어나는 날에도 이
    // 경로만은 그대로여야 하기 때문이다.
    let mut offenders: Vec<String> = Vec::new();

    for (relative, code) in whole_path_sources() {
        for forbidden in [
            "ureq",
            "reqwest",
            "std::net",
            "TcpStream",
            "TcpListener",
            "http://",
            "https://",
            "localhost",
            "127.0.0.1",
            "crate::notion",
            "notion::",
            "crate::sync",
        ] {
            if code.contains(forbidden) {
                offenders.push(format!("{relative} — {forbidden}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "복사·export 경로에 네트워크로 나가는 코드가 있다: {offenders:?}"
    );
}

// --- MH-4 — audio는 이 경로에 들어올 자리가 없다 -----------------------------------------

#[test]
fn mh4_no_audio_path_and_no_audio_format_reaches_the_outputs_or_the_written_file() {
    // INV-6이 이 Phase에서 뜻하는 바다 — 사람이 가져가는 것은 **텍스트**이고, 오디오는 이
    // 기기를 떠나지 않는다. 값으로 확인한다: 알아볼 수 있는 경로와 형식을 넣고 셋을 훑는다.
    let fixture = Fixture::new("no-audio");
    let recording_id = fixture.save_recording_with_audio(
        "rec-1",
        "3DGS Study #04",
        "recordings/secret-audio-path.wav",
        "flac",
    );
    fixture.save_transcript(&recording_id, "tr-1", "오디오는 여기 없다.", true);

    let exported = fixture
        .export_ai_request(&recording_id, NoteType::Meeting)
        .expect("내보낼 수 있어야 한다");
    let outputs = fixture.three_outputs(&recording_id, NoteType::Meeting);

    for text in outputs.all() {
        for absent in ["secret-audio-path", ".wav", "flac", "audio"] {
            assert!(!text.contains(absent), "산출물에 {absent}가 나갔다");
        }
    }
    // 파일 **이름**도 마찬가지다 — 이름 하나로도 원본 오디오의 자리를 알려 줄 수 있다.
    assert!(!exported.file_name.contains("secret-audio-path"));
    assert!(!exported.file_name.contains(".wav"));
}

#[test]
fn mh4_the_types_on_this_path_have_no_place_to_carry_audio() {
    // 값이 아니라 **모양**을 본다 — 담을 자리가 없으면 실수로도 실릴 수 없다.
    let ai_request = product_code(&source("src/export/ai_request.rs"));
    let payload = product_code(&source("src/commands/payload.rs"));

    // 요청 하나의 타입과 그 입력 타입.
    for declaration in [
        block(&ai_request, "pub struct AiRequest<'a> {", "\n}\n"),
        block(
            &product_code(&source("src/export/markdown.rs")),
            "pub struct ExportDocument<'a> {",
            "\n}\n",
        ),
        // 화면으로 돌아가는 값.
        block(&payload, "pub struct ExportedFilePayload {", "\n}\n"),
    ] {
        for absent in ["audio", "bytes", "Vec<u8>", "PathBuf"] {
            assert!(
                !declaration.contains(absent),
                "이 경로의 타입이 {absent}를 담는다: {declaration}"
            );
        }
    }

    // 그리고 이 경로의 어느 코드도 오디오 파일을 열지 않는다.
    for (relative, code) in whole_path_sources() {
        for forbidden in ["audio_path", "audio_format", "recordings_dir", "fs::read("] {
            assert!(
                !code.contains(forbidden),
                "{relative}가 오디오에 닿는다: {forbidden}"
            );
        }
    }
}

// --- MH-5 — current가 가리키는 Transcript만 쓴다 -----------------------------------------

#[test]
fn mh5_the_three_outputs_follow_the_current_pointer_and_nothing_else() {
    // 실패했거나 대체된 옛 version이 조용히 섞이지 않는다 (§7.2). **포인터를 옮기면 셋이 함께
    // 따라온다** — 옛 version이 사라져서가 아니라, 고르는 자리가 한 곳이기 때문이다.
    let fixture = Fixture::new("current-only");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let old = fixture.save_transcript(&recording_id, "tr-old", "옛 version의 문장", false);
    fixture.save_transcript(&recording_id, "tr-current", "지금 version의 문장", true);

    // 사전 조건: 저장소에는 둘 다 있다.
    assert_eq!(
        fixture.snapshot(&recording_id).transcripts.len(),
        2,
        "사전 조건: 두 version이 함께 있어야 한다"
    );

    for text in fixture.three_outputs(&recording_id, NoteType::Meeting).all() {
        assert!(text.contains("지금 version의 문장"), "current가 쓰여야 한다");
        assert!(!text.contains("옛 version의 문장"), "옛 version이 섞였다");
    }

    // 포인터를 옛 것으로 옮기면 셋이 전부 그것을 쓴다 — 고르는 규칙이 한 자리에 있다는 뜻이다.
    fixture.set_current(&recording_id, Some(&old));
    for text in fixture.three_outputs(&recording_id, NoteType::Meeting).all() {
        assert!(text.contains("옛 version의 문장"), "포인터를 따르지 않는다");
        assert!(!text.contains("지금 version의 문장"));
    }

    // 포인터가 비면 version이 둘 다 남아 있어도 고를 것이 없다. **앱이 대신 고르지 않는다.**
    fixture.set_current(&recording_id, None);
    assert!(fixture.ai_prompt(&recording_id, NoteType::Meeting).is_err());
    assert!(fixture.transcript_text(&recording_id).is_err());
    assert!(fixture
        .export_ai_request(&recording_id, NoteType::Meeting)
        .is_err());
    assert_eq!(
        fixture.snapshot(&recording_id).transcripts.len(),
        2,
        "거절이 전사를 지우지 않는다 (INV-2)"
    );
}

#[test]
fn mh5_no_code_on_this_path_can_pick_a_transcript_version_by_itself() {
    // 세 command가 `transcriptId`를 받지 않는다는 계약은 `tests/manual_ai_handoff.rs`가 본다.
    // 여기서 보는 것은 그 **안쪽**이다 — 경계 아래 어디에도 version을 고르는 두 번째 규칙이
    // 없다. 목록을 훑거나 정렬해서 "가장 그럴듯한 것"을 고르기 시작하면 §7.2가 두 벌이 된다.
    let mut offenders: Vec<String> = Vec::new();

    for (relative, code) in whole_path_sources() {
        for forbidden in [
            "list_transcripts",
            "latest_transcript",
            "ORDER BY",
            "sort_by",
            "TranscriptId::new",
        ] {
            if code.contains(forbidden) {
                offenders.push(format!("{relative} — {forbidden}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "이 경로가 Transcript version을 스스로 고른다: {offenders:?}"
    );

    // 고르는 자리는 하나다 — `export::run::current_input`이며, 그것이 읽는 것은 포인터다.
    let run = product_code(&source("src/export/run.rs"));
    assert!(run.contains("recording.current_transcript_id"));
    assert!(
        product_code(&source("src/export/handoff.rs")).contains("current_input("),
        "handoff가 그 한 자리를 쓰지 않는다"
    );
}

// --- MH-6 — 벤더를 알지 않는다 -----------------------------------------------------------

#[test]
fn mh6_no_vendor_name_reaches_the_outputs_or_the_written_file() {
    // 요청이 요구하는 것은 **Markdown 하나**이며, 그것은 어느 채팅에나 붙여 넣을 수 있다.
    let fixture = Fixture::new("no-vendor");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "벤더 이름은 여기 없다.", true);

    for mode in NoteType::ALL {
        for text in fixture.three_outputs(&recording_id, mode).all() {
            for vendor in VENDOR_NAMES {
                assert!(!text.contains(vendor), "{mode}: 벤더 이름 {vendor}가 나갔다");
            }
            for schema in VENDOR_SCHEMA {
                assert!(!text.contains(schema), "{mode}: 벤더 schema {schema}가 나갔다");
            }
        }
    }
}

#[test]
fn mh6_no_vendor_name_or_vendor_schema_is_in_the_domain_and_payload_types() {
    // 산출물이 지금 깨끗한 것만으로는 부족하다 — **타입이 벤더를 알면** 언제든 그 값이 산출물로
    // 흘러나올 수 있다. core/domain과 IPC payload를 원문으로 본다 (INV-9 · §9.3).
    let mut offenders: Vec<String> = Vec::new();

    for relative in [
        "src/domain/mod.rs",
        "src/domain/settings.rs",
        "src/domain/failure.rs",
        "src/domain/duration.rs",
        "src/commands/payload.rs",
    ] {
        let code = product_code(&source(relative));
        for forbidden in VENDOR_NAMES.into_iter().chain(VENDOR_SCHEMA) {
            if code.contains(forbidden) {
                offenders.push(format!("{relative} — {forbidden}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "core/domain·payload 타입이 벤더를 안다: {offenders:?}"
    );
}

// --- MH-7 — 실패해도 잃는 것이 없다 ------------------------------------------------------

#[test]
fn mh7_four_failures_leave_the_database_bytes_and_the_exported_files_untouched() {
    // 복사도 export도 **읽기와 파일 하나 더하기**뿐이다 (INV-3). 실패하면 아무 일도 일어나지
    // 않은 것과 같아야 하며, 그것을 저장된 값이 아니라 **DB 파일의 바이트**로 확인한다.
    let fixture = Fixture::new("failures-change-nothing");

    // 지켜져야 하는 것들: 녹음 · 전사 · 연결된 provider가 만든 노트 · 이미 내보낸 파일 둘.
    let kept = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&kept, "tr-1", "남아 있어야 하는 문장", true);
    let note = fixture.generate_note(&kept, NoteType::Study);
    fixture
        .exporter
        .export(kept.as_str())
        .expect("Markdown export가 성공해야 한다");
    fixture
        .export_ai_request(&kept, NoteType::Study)
        .expect("AI-ready 문서를 내보낼 수 있어야 한다");

    // 실패하게 될 자리들도 미리 만들어 둔다 — 스냅샷 뒤에는 아무것도 저장하지 않는다.
    let without_current = fixture.save_recording("rec-2", "current가 없는 녹음");
    fixture.save_transcript(&without_current, "tr-2", "current가 아닌 문장", false);
    let empty = fixture.save_recording("rec-3", "적을 것이 없는 녹음");
    fixture.save_transcript(&empty, "tr-3", "   ", true);
    let missing = RecordingId::new("rec-없음");

    let before_snapshot = fixture.snapshot(&kept);
    let before_database = fixture.database_bytes();
    let before_exports = fixture.exported_files();
    assert_eq!(before_exports.len(), 2, "사전 조건: 내보낸 파일이 둘 있다");

    let mut failures: Vec<Failure> = Vec::new();
    for target in [&missing, &without_current, &empty] {
        failures.push(
            fixture
                .ai_prompt(target, NoteType::Study)
                .expect_err("만들 것이 없다"),
        );
        failures.push(
            fixture
                .transcript_text(target)
                .expect_err("만들 것이 없다"),
        );
        failures.push(
            fixture
                .export_ai_request(target, NoteType::Study)
                .map(|_| ())
                .expect_err("만들 것이 없다"),
        );
    }
    // 네 번째 실패 — 있지도 않은 mode를 고르는 길은 이 경계에 없으므로, 빈 recordingId다.
    failures.push(
        fixture
            .storage
            .ai_prompt("   ", NoteType::Meeting)
            .expect_err("고른 녹음이 없다"),
    );

    for failure in &failures {
        assert!(
            failure.source_data_safe,
            "실패가 원본이 안전하다고 말하지 않는다 (§13 · MH-7)"
        );
    }

    assert_eq!(fixture.snapshot(&kept), before_snapshot, "저장된 것이 바뀌었다");
    assert_eq!(
        fixture.database_bytes(),
        before_database,
        "DB 파일의 바이트가 달라졌다"
    );
    assert_eq!(
        fixture.exported_files(),
        before_exports,
        "이미 내보낸 파일이 달라졌다"
    );
    // 노트도 그 자리에 그대로다 (MH-8과 만나는 자리다).
    assert!(fixture.snapshot(&kept).notes.contains(&note));
}

#[test]
fn mh7_a_place_that_cannot_be_written_changes_nothing_either() {
    // 앞의 실패들은 **쓰기 전에** 났다. 이번에는 쓸 자리 자체가 없는 경우다 — 그때에도 저장된
    // 것은 그대로이고, 그 자리에 있던 사용자의 파일도 그대로다.
    let fixture = Fixture::new("no-place-to-write");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "남아 있어야 하는 문장", true);

    // `exports/`가 되어야 할 자리에 파일이 하나 있다 — 디렉터리를 만들 수 없다.
    let blocking = fixture.exports_dir();
    fs::write(&blocking, "사용자의 파일").expect("사전 조건: 그 자리에 파일을 둔다");

    let before_snapshot = fixture.snapshot(&recording_id);
    let before_database = fixture.database_bytes();

    let failure = fixture
        .export_ai_request(&recording_id, NoteType::Study)
        .map(|_| ())
        .expect_err("쓸 자리가 없다");

    assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (MH-7)");
    assert_eq!(
        fs::read_to_string(&blocking).expect("그 파일을 읽는다"),
        "사용자의 파일",
        "쓸 자리를 만들려고 사용자의 파일을 치우지 않는다"
    );
    assert_eq!(fixture.snapshot(&recording_id), before_snapshot);
    assert_eq!(fixture.database_bytes(), before_database);

    // 복사 둘은 파일에 닿지도 않으므로 이 상황과 무관하게 그대로 된다 (ADR-0010 §7.5).
    assert!(fixture.ai_prompt(&recording_id, NoteType::Study).is_ok());
    assert!(fixture.transcript_text(&recording_id).is_ok());
}

#[test]
fn mh7_the_command_boundary_has_no_way_to_write_to_the_repository() {
    // 실패 경로에 저장소 쓰기가 없다는 것을 **쓸 수단이 없다**로 본다. 순수 모듈 둘은
    // `tests/manual_ai_handoff.rs`가 같은 방식으로 보므로, 여기는 그 위의 command 경계다.
    let boundary = command_boundary();
    let mut offenders: Vec<&str> = Vec::new();

    for forbidden in [
        "insert_",
        "update_",
        "delete_",
        "append_",
        "set_current",
        "execute(",
        "remove_file",
        "remove_dir",
    ] {
        if boundary.contains(forbidden) {
            offenders.push(forbidden);
        }
    }

    assert!(
        offenders.is_empty(),
        "command 경계가 저장된 것을 고치거나 지울 수 있다: {offenders:?}"
    );

    // 파일을 만드는 자리는 하나이며, 그것은 **이미 있는 파일을 열지 않는 방식**이다
    // (ADR-0009 §4.3). 이 한 줄이 바뀌면 export가 사용자의 문서를 덮어쓸 수 있게 된다.
    let file = product_code(&source("src/export/file.rs"));
    assert!(file.contains("create_new(true)"), "쓰기가 create_new가 아니다");
    assert!(!file.contains("remove_file"));
    assert!(!file.contains("remove_dir"));
}

// --- MH-8 — 기존 Connected Provider 경로가 그대로다 --------------------------------------

#[test]
fn mh8_the_connected_provider_path_still_makes_notes_next_to_the_manual_path() {
    // 이 Phase가 더한 것은 **또 하나의 길**이지 대체가 아니다 (요구 6). 연결된 provider로
    // 노트를 만드는 경로는 그대로 동작하며, 두 길은 서로를 건드리지 않는다.
    //
    // provider 자리에는 계약이 같은 test double이 선다 — 실제 AI 서버도 모델도 요구하지 않는다
    // (§18 · ADR-0008 §4.3).
    let fixture = Fixture::new("connected-provider");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let transcript_id =
        fixture.save_transcript(&recording_id, "tr-1", "노트를 만들 문장이다.", true);

    let before_manual = fixture.three_outputs(&recording_id, NoteType::Study);

    // 연결된 provider가 노트를 만든다 — 이 Phase 전과 같은 호출이다.
    let note = fixture.generate_note(&recording_id, NoteType::Meeting);

    assert_eq!(note.recording_id, recording_id);
    assert_eq!(note.transcript_id, transcript_id, "current에서 만들어진다");
    assert_eq!(note.note_type, NoteType::Meeting);
    // provenance가 그대로 남는다 (ADR-0008 §9.1).
    assert_eq!(note.provider, FAKE_PROVIDER_ID);
    assert_eq!(note.model, FAKE_MODEL_ID);
    assert!(!note.prompt_version.is_empty());
    assert!(!note.generated_at.is_empty());

    let stored = fixture.snapshot(&recording_id);
    assert_eq!(stored.notes, vec![note.clone()], "노트가 저장돼 있어야 한다");
    assert_eq!(
        stored.recording.ai_status,
        ProcessingStatus::Done,
        "생성 경로가 상태를 남긴다"
    );

    // 노트가 생겼다고 해서 사람이 가져가는 세 산출물이 달라지지 않는다 — AI-ready 문서는 노트를
    // **요청하는** 문서이지 노트를 담는 문서가 아니다 (ADR-0010 §5.4).
    let after_manual = fixture.three_outputs(&recording_id, NoteType::Study);
    assert_eq!(after_manual, before_manual);

    // 반대쪽도 그대로다 — 세 산출물을 만든 뒤에도 노트는 그 자리에 그대로 있다 (MH-7 · MH-8).
    assert_eq!(fixture.snapshot(&recording_id).notes, vec![note]);
}
