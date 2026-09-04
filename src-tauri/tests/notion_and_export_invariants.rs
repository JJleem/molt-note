//! **이 Phase의 불변을 이미 만들어진 경로로 못박는다** (PRODUCT-SPEC §2.1 · §17.1 ·
//! `phase-prompt/05` 성공 기준 · `docs/ADR-0009-notion-and-export.md`).
//!
//! `tests/audio_never_reaches_ai.rs`가 Phase 4에 대해 하는 일과 같은 자리다 — **새 제품 코드를
//! 만들지 않고**, 이미 있는 두 출구(Markdown 파일 · Notion 페이지)를 그대로 지나면서 여섯 가지
//! 불변을 각각 실패할 수 있는 검사로 바꾼다.
//!
//! ```text
//! (a) INV-8 · §17.1   AI Note가 하나도 없는 Recording이 두 출구를 모두 지난다
//! (b) 무손실 전송      1시간 규모 transcript가 나뉘어 순서대로 나가고, 이어 붙이면 원문이다
//! (c) INV-3           네 가지 실패 뒤에도 local data가 그대로이고 재시도가 끝까지 간다
//! (d) INV-6           오디오 바이트도 경로도 요청에 실리지 않는다 (행동 · 소스 두 각도)
//! (e) INV-7           token이 SQLite에도 실패 문장에도 없다
//! (f) INV-9           두 renderer가 provider 중립 Structured Note만 소비한다 (행동 · 소스)
//! ```
//!
//! ## 이미 있는 검사를 다시 쓰지 않는다
//!
//! 아래는 **여기서 다시 쓰지 않고** 그 자리를 가리킨다. 같은 사실을 두 번 적으면 둘이 어긋날 때
//! 어느 쪽이 규칙인지 알 수 없게 된다.
//!
//! ```text
//! 조각내기 자체의 무손실성            tests/notion_chunking.rs
//!   (예산 · 줄 경계 · 재조립 · 임의 문서 속성 검사 · 담을 수 없는 단위의 거절)
//! 요청 하나의 계약과 실패 매핑        tests/notion_adapter.rs
//!   (Bearer · API 버전 · 오류 코드 → §13 · 실패에 token이 실리지 않음 · adapter 밖은 Notion을 모름)
//! 자격증명 저장소 경계               tests/secret_store.rs
//!   (double 하나로 전 경로 · Debug/직렬화로 새지 않음 · 실제 저장소를 세우는 테스트가 없음)
//! frontend 소스의 token 부재 (INV-7)  tests/screen-boundary.test.ts · tests/ipc-boundary.test.ts
//!   ("token은 화면에 남지 않는다" · "자격증명은 이 경계를 한 방향으로만 지난다")
//! migration SQL에 secret 열이 없음    src/db/migrations.rs 의 단위 테스트
//!   (여기서는 그 대신 **실제로 만들어진 스키마와 파일 바이트**를 본다)
//! ```
//!
//! ## 무엇을 쓰지 않는가 (§18 · `phase-prompt/05` Important Rules)
//!
//! **실제 Notion에 한 번도 요청하지 않는다** — HTTP 왕복은 `notion::testing::StubServer` 뒤에
//! 있다. **실제 OS 자격증명 저장소를 열지 않는다** — token은 `InMemorySecretStore` 안에만 있다.
//! **사용자의 실제 디렉터리를 건드리지 않는다** — 저장소도 export도 오디오도 전부
//! `std::env::temp_dir()` 아래에서 만들어지고 Drop 때 지워진다. **한 밀리초도 자지 않는다** —
//! 대기는 `sync::pace::testing::RecordedWaits`가 기록만 한다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use molt_note_lib::ai::note::{encode_content, MeetingNote, StructuredNote, StudyNote, SummaryNote};
use molt_note_lib::commands::{
    ExportedFilePayload, Exporter, NotionSendStatus, NotionSender, Storage,
};
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{
    AiNote, AiNoteId, Failure, FailureKind, NoteType, NotionSync, ProcessingStatus, Recording,
    RecordingId, Settings, Transcript, TranscriptId, TranscriptSegment,
};
use molt_note_lib::export::markdown::{
    render, ExportDocument, MEETING_SECTIONS, STUDY_SECTIONS, SUMMARY_SECTIONS, TRANSCRIPT_SECTION,
};
use molt_note_lib::notion::testing::{StubReply, StubRequest, StubServer, CREATED_PAGE_ID};
use molt_note_lib::notion::{
    split_markdown, ApiErrorCode, HttpTransport, NotionClient, TransportError, CHUNK_MAX_BYTES,
};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::secret_store::testing::InMemorySecretStore;
use molt_note_lib::platform::secret_store::{Secret, SecretKey, SecretStore};
use molt_note_lib::sync::pace::testing::RecordedWaits;
use molt_note_lib::sync::run::{send, Confirmation, Destination, Sent};

/// 이 파일이 쓰는 값들. **하나도 실재하지 않는다** (ADR-0009 §10.5).
const NOT_A_REAL_TOKEN: &str = "ntn-invariant-test-double-value-not-a-real-credential";
const PARENT_PAGE_ID: &str = "parent-page-identifier-not-a-real-page";
const CREATED_AT: &str = "2026-09-01T10:00:00.000Z";

/// 1시간. (b)의 입력 규모를 정하는 값이다.
const ONE_HOUR_MS: i64 = 3_600_000;
/// segment 하나의 길이. `ONE_HOUR_MS / SEGMENT_MS = 1,200` segment가 된다.
const SEGMENT_MS: i64 = 3_000;

/// transcript 한 줄에 들어가는 표시. 몇 줄이 실제로 나갔는지 세는 자리다.
const SEGMENT_MARK: &str = "번 구간입니다.";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 자리 ----------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-phase5-invariants-{}-{}-{}",
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

/// 저장소 하나와 그 위에서 도는 **두 출구**.
///
/// 파일로 나가는 쪽([`Exporter`])과 Notion으로 나가는 쪽([`send`])이 같은 저장소를 본다 —
/// 이 파일의 여러 검사가 "두 출구가 같은 것을 낸다"를 묻기 때문이다 (ADR-0009 §14).
struct Fixture {
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
    exporter: Exporter,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.path().join("app-data"));

        // 스키마를 만드는 것은 앱과 같은 경로다.
        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

        let fixture = Self {
            exporter: Exporter::in_directory(app_data_dir.clone()),
            app_data_dir,
            _root: root,
        };

        // 어디로 보낼지는 설정에 있다 (ADR-0009 §8.4). **token은 여기 없다** (INV-7).
        settings::save(
            &fixture.connection(),
            &Settings {
                notion_parent_page_id: Some(PARENT_PAGE_ID.to_string()),
                ..Settings::DEFAULT
            },
        )
        .expect("사전 조건: 설정을 저장한다");

        fixture
    }

    fn connection(&self) -> rusqlite::Connection {
        db::open_in(&self.app_data_dir).expect("DB를 열 수 있어야 한다")
    }

    /// 녹음 하나를 저장한다. `audio_path`는 받은 값 그대로 들어간다.
    fn save_recording(&self, id: &str, title: &str, audio_path: &str) -> RecordingId {
        let recording = Recording {
            id: RecordingId::new(id),
            title: title.to_string(),
            created_at: CREATED_AT.to_string(),
            updated_at: CREATED_AT.to_string(),
            duration_ms: ONE_HOUR_MS,
            audio_path: audio_path.to_string(),
            audio_format: "wav".to_string(),
            microphone: Some("가짜 마이크".to_string()),
            current_transcript_id: None,
            transcription_status: ProcessingStatus::Done,
            ai_status: ProcessingStatus::None,
            notion_status: ProcessingStatus::None,
        };
        store::insert_recording(&self.connection(), &recording).expect("사전 조건: 녹음을 저장한다");

        recording.id
    }

    /// Transcript 하나를 추가하고 current로 지정한다 (§7.2).
    fn save_transcript(&self, recording_id: &RecordingId, count: i64) -> TranscriptId {
        let segments: Vec<TranscriptSegment> = (0..count)
            .map(|index| TranscriptSegment {
                start_ms: index * SEGMENT_MS,
                end_ms: (index + 1) * SEGMENT_MS,
                text: format!(
                    "{index}{SEGMENT_MARK} 발표자는 3D Gaussian Splatting의 학습 파이프라인과 \
                     렌더링 품질 지표를 설명했고, 다음 주까지 실험 결과를 정리해 공유하기로 했습니다.",
                ),
            })
            .collect();

        let transcript = Transcript {
            id: TranscriptId::new("tr-1"),
            recording_id: recording_id.clone(),
            language: Some("ko".to_string()),
            raw_text: segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            segments,
            created_at: CREATED_AT.to_string(),
            engine: "stub".to_string(),
            model: "ggml-base.bin".to_string(),
        };

        let mut connection = self.connection();
        store::append_transcript(&mut connection, &transcript).expect("사전 조건: 전사를 저장한다");
        store::set_current_transcript(&connection, recording_id, Some(&transcript.id), CREATED_AT)
            .expect("사전 조건: current를 지정한다");

        transcript.id
    }

    /// AI 노트 하나를 저장한다. **provider · model은 부르는 쪽이 정한다** — (f)가 그 값을 바꿔
    /// 가며 산출물이 그대로인지 묻기 때문이다.
    fn save_note(
        &self,
        recording_id: &RecordingId,
        transcript_id: &TranscriptId,
        note: &StructuredNote,
        provider: &str,
        model: &str,
    ) {
        let stored = AiNote {
            id: AiNoteId::new("note-1"),
            recording_id: recording_id.clone(),
            transcript_id: transcript_id.clone(),
            note_type: note.mode(),
            content: encode_content(note),
            provider: provider.to_string(),
            model: model.to_string(),
            prompt_version: "v1".to_string(),
            generated_at: "2026-09-01T11:00:00.000Z".to_string(),
        };
        store::insert_ai_note(&self.connection(), &stored).expect("사전 조건: 노트를 저장한다");
    }

    /// 이 Recording이 만들어 낼 Markdown 문서 그대로 — **두 출구가 함께 쓰는 그 렌더러다.**
    fn document(&self, recording_id: &RecordingId, note: Option<&StructuredNote>) -> String {
        let connection = self.connection();
        let recording = store::load_recording(&connection, recording_id)
            .expect("녹음을 읽는다")
            .expect("녹음이 있다");
        let transcript = store::load_transcript(
            &connection,
            recording
                .current_transcript_id
                .as_ref()
                .expect("current가 있다"),
        )
        .expect("전사를 읽는다")
        .expect("전사가 있다");

        render(&ExportDocument {
            recording: &recording,
            transcript: &transcript,
            note,
        })
    }

    /// 파일로 나가는 출구.
    fn export(&self, recording_id: &RecordingId) -> Result<ExportedFilePayload, Failure> {
        self.exporter.export(recording_id.as_str())
    }

    /// Notion으로 나가는 출구. 실행 순서를 그대로 지난다.
    fn send_with(
        &self,
        recording_id: &RecordingId,
        server: &Arc<StubServer>,
        confirmation: Confirmation,
    ) -> Result<Sent, Failure> {
        let transport: Arc<dyn HttpTransport> = server.clone();

        send(
            &self.connection(),
            recording_id,
            &NotionClient::new(transport),
            &Destination {
                token: &Secret::new(NOT_A_REAL_TOKEN),
                parent_page_id: PARENT_PAGE_ID,
            },
            &RecordedWaits::new(),
            confirmation,
        )
    }

    fn sync(&self, recording_id: &RecordingId) -> NotionSync {
        store::load_notion_sync(&self.connection(), recording_id)
            .expect("전송 상태를 읽는다")
            .expect("전송 상태가 있다")
    }

    /// 저장된 것 전부를 한 값으로 찍는다. **실패 전후를 그대로 비교하기 위한 것이다** (INV-3).
    fn snapshot(&self, recording_id: &RecordingId) -> Snapshot {
        let connection = self.connection();
        let recording = store::load_recording(&connection, recording_id)
            .expect("녹음을 읽는다")
            .expect("녹음이 있다");
        let transcripts = store::list_transcripts(&connection, recording_id).expect("전사를 읽는다");
        let notes = transcripts
            .iter()
            .flat_map(|transcript| {
                store::list_ai_notes_for_transcript(&connection, &transcript.id)
                    .expect("노트를 읽는다")
            })
            .collect();

        Snapshot {
            transcripts,
            notes,
            // 전송이 옮기는 것은 이 둘뿐이다 (§8.5). 나머지 필드는 그대로여야 한다.
            recording: Recording {
                notion_status: ProcessingStatus::None,
                updated_at: CREATED_AT.to_string(),
                ..recording
            },
        }
    }
}

/// recording · transcript · ai_note를 한 번에 비교하기 위한 값 (INV-3).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    recording: Recording,
    transcripts: Vec<Transcript>,
    notes: Vec<AiNote>,
}

// --- 관찰 도구 ------------------------------------------------------------------------

/// 요청 하나에 실려 나간 markdown. 페이지 생성과 이어붙이기가 값을 담는 자리가 다르다.
fn sent_markdown(request: &StubRequest) -> String {
    let body: serde_json::Value = serde_json::from_str(request.body.as_deref().expect("본문이 있다"))
        .expect("본문은 JSON이다");

    let value = match request.method.as_str() {
        "POST" => body["markdown"].clone(),
        _ => body["insert_content"]["content"].clone(),
    };

    value
        .as_str()
        .unwrap_or_else(|| panic!("요청에 markdown이 없다: {body}"))
        .to_string()
}

/// 그 서버로 나간 문서 전체 — **보낸 것을 순서대로 이어 붙인 결과다.**
fn reassembled(server: &StubServer) -> String {
    server
        .requests()
        .iter()
        .map(sent_markdown)
        .collect::<Vec<_>>()
        .concat()
}

/// 그 서버가 받은 요청 중 페이지를 **만드는** 것의 수.
fn page_creations(server: &StubServer) -> usize {
    server
        .requests()
        .iter()
        .filter(|request| request.method.as_str() == "POST")
        .count()
}

/// 내보낸 파일을 읽는다.
fn written(exported: &ExportedFilePayload) -> String {
    fs::read_to_string(&exported.path).expect("내보낸 파일을 읽을 수 있어야 한다")
}

/// 실행되는 제품 코드만 남긴다 — 주석과 `#[cfg(test)]` 아래를 잘라낸다
/// (`tests/audio_never_reaches_ai.rs`와 같은 방식이다: 규칙을 문장으로 적는 것과 어기는 코드는
/// 다르다).
fn product_code(source: &str) -> String {
    let executable = match source.find("#[cfg(test)]") {
        Some(boundary) => &source[..boundary],
        None => source,
    };

    // 이 저장소의 주석은 전부 `//` · `///` · `//!`다. 블록 주석은 쓰지 않는다.
    executable
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 주어진 디렉터리들 아래의 모든 `.rs` 파일.
fn rust_sources(directories: &[&str]) -> Vec<PathBuf> {
    fn walk(directory: &Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).expect("소스 디렉터리를 읽는다") {
            let path = entry.expect("디렉터리 항목을 읽는다").path();
            if path.is_dir() {
                walk(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for directory in directories {
        walk(&root.join(directory), &mut files);
    }
    files.sort();
    assert!(!files.is_empty(), "검사 대상 소스가 하나도 없다: {directories:?}");
    files
}

/// 이 파일이 쓰는 세 mode의 노트. **§9.5의 필드 그대로이며 벤더 고유 필드가 없다.**
fn every_note_mode() -> [StructuredNote; 3] {
    [
        StructuredNote::Meeting(MeetingNote {
            overview: "3DGS 스터디 4회차 회의".to_string(),
            key_discussions: vec!["학습 파이프라인".to_string(), "품질 지표".to_string()],
            decisions: vec!["다음 주까지 실험 결과 정리".to_string()],
            action_items: vec!["벤치마크 표 작성".to_string()],
            open_questions: vec!["렌더링 속도 목표".to_string()],
        }),
        StructuredNote::Study(StudyNote {
            overview: "3D Gaussian Splatting 개요".to_string(),
            key_concepts: vec!["splatting".to_string(), "densification".to_string()],
            important_details: vec!["학습률 스케줄".to_string()],
            questions: vec!["왜 SH 계수를 쓰는가".to_string()],
            things_to_study: vec!["원 논문 4장".to_string()],
            references_mentioned: vec!["3DGS 원 논문".to_string()],
        }),
        StructuredNote::Summary(SummaryNote {
            short_summary: "세 줄 요약".to_string(),
            key_points: vec!["첫 번째".to_string(), "두 번째".to_string()],
        }),
    ]
}

// ---------------------------------------------------------------------------
// (a) AI Note가 하나도 없는 Recording이 **두 출구를 모두** 지난다 (INV-8 · §17.1)
// ---------------------------------------------------------------------------
//
// 출구 하나씩은 이미 판정돼 있다 — 파일 쪽은
// `tests/markdown_export.rs::a_recording_with_no_ai_note_at_all_is_exported_as_a_valid_document`,
// Notion 쪽은 `tests/notion_sync.rs::a_recording_with_no_ai_note_at_all_is_sent_with_just_its_transcript`.
// 여기서 새로 못박는 것은 **그 둘이 같은 Recording에서 동시에 성립하고, 두 출구의 산출물이
// 서로 바이트까지 같다**는 것이다 (ADR-0009 §14). 두 문이 서로 다른 문서를 내면 §17.1의
// "AI 없이 완결되는 제품"은 문서 하나가 아니라 둘이 된다.

#[test]
fn a_recording_with_no_ai_note_at_all_goes_out_both_doors_as_one_and_the_same_document() {
    let fixture = Fixture::new("no-ai-note-both-doors");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04", "recordings/rec-1.wav");
    let transcript_id = fixture.save_transcript(&recording_id, 4);

    // 사전 조건: 노트가 **하나도** 없다. AI를 한 번도 쓰지 않은 Recording이다.
    assert!(
        store::list_ai_notes_for_transcript(&fixture.connection(), &transcript_id)
            .expect("노트를 읽는다")
            .is_empty(),
        "사전 조건: AI Note가 없어야 한다"
    );

    let expected = fixture.document(&recording_id, None);

    // --- 출구 1: Markdown 파일 ---
    let exported = fixture.export(&recording_id).expect("AI 없이도 내보낼 수 있다");
    let file_body = written(&exported);

    // --- 출구 2: Notion 페이지 ---
    let server = Arc::new(StubServer::ready());
    let sent = fixture
        .send_with(&recording_id, &server, Confirmation::NotAsked)
        .expect("AI 없이도 보낼 수 있다");

    // ★ 두 출구의 산출물이 서로 같고, 둘 다 그 렌더러의 결과 그대로다.
    assert_eq!(file_body, expected, "파일이 렌더러의 산출물과 다르다");
    assert_eq!(
        reassembled(&server),
        expected,
        "Notion으로 나간 문서가 파일과 다르다 (ADR-0009 §14)"
    );

    // 없는 AI 섹션의 빈 껍데기를 남기지 않는다 — 있는 것은 제목 · 메타 · Transcript다.
    assert!(file_body.starts_with("# 3DGS Study #04\n"), "{file_body}");
    assert!(file_body.contains(&format!("## {TRANSCRIPT_SECTION}\n")), "{file_body}");
    for section in MEETING_SECTIONS
        .iter()
        .chain(STUDY_SECTIONS.iter())
        .chain(SUMMARY_SECTIONS.iter())
    {
        assert!(
            !file_body.contains(&format!("## {section}")),
            "AI를 쓰지 않았는데 AI 섹션이 남았다: {section}"
        );
    }

    // 두 출구 모두 실제로 끝났다 — "성공했다"가 상태에도 남아 있다.
    assert!(Path::new(&exported.path).is_file(), "파일이 실제로 만들어졌다");
    assert!(sent.created_page);
    let stored = fixture.sync(&recording_id);
    assert_eq!(stored.status, ProcessingStatus::Done);
    assert_eq!(stored.sent_chunks, stored.total_chunks);
}

// ---------------------------------------------------------------------------
// (b) 1시간 규모 transcript가 나뉘어 순서대로 나가고, **이어 붙이면 원문이다**
// ---------------------------------------------------------------------------
//
// `tests/notion_chunking.rs::an_hour_long_transcript_rejoins_byte_for_byte`는 **나누는 함수**에
// 대해 같은 성질을 판정한다. 여기서 판정하는 것은 그 뒤다 — 나눈 것이 실제로 전송 경로를 지나
// 요청이 됐을 때도 순서와 내용이 그대로인가. 나누기가 무손실이어도 보내는 쪽이 하나를 건너뛰면
// 사용자가 보는 페이지에서는 그만큼이 조용히 사라진다.

#[test]
fn an_hour_long_transcript_is_sent_in_pieces_that_reassemble_into_the_original_byte_for_byte() {
    let fixture = Fixture::new("one-hour");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04", "recordings/rec-1.wav");
    let segment_count = ONE_HOUR_MS / SEGMENT_MS; // 1,200 segment
    fixture.save_transcript(&recording_id, segment_count);

    let expected = fixture.document(&recording_id, None);

    // 이 입력이 실제로 여러 요청이 되는 규모인지 먼저 고정한다. 한 요청에 들어가는 문서로는
    // 이 테스트가 아무것도 검사하지 못한다.
    assert!(
        expected.len() > 3 * CHUNK_MAX_BYTES,
        "1시간 규모라기에 너무 작다: {} 바이트",
        expected.len()
    );
    let chunks = split_markdown(&expected).expect("나눌 수 있는 문서다");
    assert!(chunks.len() > 4, "chunk가 {}개뿐이다", chunks.len());

    let server = Arc::new(StubServer::ready());
    let sent = fixture
        .send_with(&recording_id, &server, Confirmation::NotAsked)
        .expect("전송이 성공해야 한다");

    // 첫 요청이 페이지를 만들고, 나머지는 **그 페이지에** 순서대로 이어 붙인다.
    let requests = server.requests();
    assert_eq!(requests.len(), chunks.len(), "조각 수만큼 요청이 나간다");
    assert_eq!(page_creations(&server), 1, "페이지는 하나다");
    assert_eq!(requests[0].method.as_str(), "POST");
    for request in &requests[1..] {
        assert_eq!(request.method.as_str(), "PATCH");
        assert!(
            request.url.contains(CREATED_PAGE_ID),
            "만든 그 페이지에 이어 붙인다: {}",
            request.url
        );
    }

    // ★ 보낸 것을 순서대로 이어 붙이면 **문서 전체가 원문과 정확히 같다.**
    // 앞뒤 몇 글자나 길이만 보면 가운데가 사라진 것을 놓친다.
    //
    // 이 문서는 20만 바이트가 넘는다. 어긋났을 때 문서 전체를 쏟아 내면 무엇이 달랐는지
    // 오히려 보이지 않으므로, **어디가** 달랐는지만 말한다.
    let arrived: Vec<String> = requests.iter().map(sent_markdown).collect();
    for (index, (piece, chunk)) in arrived.iter().zip(chunks.iter()).enumerate() {
        assert!(
            piece.as_str() == *chunk,
            "{index}번째로 나간 것이 그 자리의 조각과 다르다 ({}바이트 vs {}바이트)",
            piece.len(),
            chunk.len()
        );
    }
    let rejoined = arrived.concat();
    assert!(
        rejoined == expected,
        "재조립한 문서가 원문과 다르다 — 잘렸거나 유실됐다 ({}바이트 vs {}바이트)",
        rejoined.len(),
        expected.len()
    );

    // 전사의 어느 한 줄도 사라지지 않았다. 재조립 동등성이 이미 그것을 말하지만, 이 문서에서
    // 무엇이 유실될 수 있는지를 이름으로 적어 둔다.
    let lines: usize = arrived
        .iter()
        .map(|chunk| chunk.matches(SEGMENT_MARK).count())
        .sum();
    assert_eq!(lines as i64, segment_count, "segment가 유실됐다");

    // 끝났다는 사실이 상태에도 남는다 — 몇 개짜리를 몇 개 보냈는지까지.
    assert!(sent.created_page);
    let stored = fixture.sync(&recording_id);
    assert_eq!(stored.status, ProcessingStatus::Done);
    assert_eq!(stored.sent_chunks, Some(chunks.len() as i64));
    assert_eq!(stored.total_chunks, Some(chunks.len() as i64));
}

// ---------------------------------------------------------------------------
// (c) 네 가지 Notion 실패 뒤에도 local data가 그대로이고, **재시도가 끝까지 간다** (INV-3)
// ---------------------------------------------------------------------------
//
// `tests/notion_sync.rs::nothing_local_is_lost_or_changed_when_a_send_fails`는 페이지가 만들어지기
// **전에** 갈리는 세 가지(인증 · destination · 네트워크)에 대해 저장된 것이 그대로임을 판정한다.
// 여기서 넓히는 것은 둘이다.
//
//   1. **중간 chunk에서 끊긴 경우**를 같은 잣대로 본다 — 페이지가 이미 만들어졌고 일부가 이미
//      Notion에 있는 상태이며, 그때도 local data는 그대로여야 한다.
//   2. 네 경우 모두 **재시도가 실제로 done까지 간다**. "재시도할 수 있는 상태"로 남는 것과
//      실제로 다시 보내 끝나는 것은 다르다.

/// 전송이 어디서 끊기는가.
#[derive(Debug, Clone)]
enum Breaks {
    /// 페이지가 만들어지기 전에. 결과를 모르므로 재시도는 확인을 요구한다 (§8.5).
    BeforeThePageExists(StubReply),
    /// 첫 chunk가 반영된 뒤에. 같은 문서이므로 재시도는 **이어 보낸다** (§8.2).
    AfterTheFirstChunk(StubReply),
}

impl Breaks {
    fn server(&self) -> StubServer {
        match self {
            Self::BeforeThePageExists(reply) => StubServer::ready().with_create_page(reply.clone()),
            Self::AfterTheFirstChunk(reply) => StubServer::ready().with_append(reply.clone()),
        }
    }

    /// 재시도가 이어 보낼 수 있는가.
    fn resumable(&self) -> bool {
        matches!(self, Self::AfterTheFirstChunk(_))
    }
}

#[test]
fn every_way_a_send_can_fail_leaves_the_local_data_untouched_and_still_finishes_on_a_retry() {
    for (situation, breaks, expected_kind) in [
        (
            "인증 실패",
            Breaks::BeforeThePageExists(StubReply::error(401, ApiErrorCode::Unauthorized)),
            FailureKind::NotionAuthFailed,
        ),
        (
            "권한 없는 destination",
            Breaks::BeforeThePageExists(StubReply::error(404, ApiErrorCode::ObjectNotFound)),
            FailureKind::NotionDestinationUnavailable,
        ),
        (
            "네트워크 없음",
            Breaks::BeforeThePageExists(StubReply::Fail(TransportError::NotConnected)),
            FailureKind::NotionRequestFailed,
        ),
        (
            "중간 chunk 실패",
            Breaks::AfterTheFirstChunk(StubReply::error(500, ApiErrorCode::InternalServerError)),
            FailureKind::NotionRequestFailed,
        ),
    ] {
        let fixture = Fixture::new("failure-keeps-data");
        let recording_id = fixture.save_recording("rec-1", "3DGS Study #04", "recordings/rec-1.wav");
        let transcript_id = fixture.save_transcript(&recording_id, 700);
        let note = StructuredNote::Summary(SummaryNote {
            short_summary: "세 줄 요약".to_string(),
            key_points: vec!["첫 번째".to_string()],
        });
        fixture.save_note(&recording_id, &transcript_id, &note, "stub", "stub-model");

        let expected = fixture.document(&recording_id, Some(&note));
        let chunk_count = split_markdown(&expected).expect("나눌 수 있는 문서다").len();
        assert!(chunk_count >= 3, "{situation}: 이 검사는 여러 chunk를 요구한다");

        let before = fixture.snapshot(&recording_id);
        let failing = Arc::new(breaks.server());

        let failure = fixture
            .send_with(&recording_id, &failing, Confirmation::NotAsked)
            .expect_err("이 상황에서는 끝까지 보낼 수 없다");

        assert_eq!(failure.kind, expected_kind, "{situation}");
        assert!(
            failure.source_data_safe,
            "{situation}: 전송 실패는 원본을 건드리지 않는다"
        );

        // ★ recording · transcript · ai_note가 그대로다 (INV-3).
        let after = fixture.snapshot(&recording_id);
        assert_eq!(after.recording, before.recording, "{situation}: 녹음 레코드");
        assert_eq!(after.transcripts, before.transcripts, "{situation}: 전사");
        assert_eq!(after.notes, before.notes, "{situation}: AI 노트");

        // 바뀌는 것은 전송 상태뿐이고, 거기에 자격증명은 섞이지 않는다 (INV-7).
        let stored = fixture.sync(&recording_id);
        assert_eq!(stored.status, ProcessingStatus::Failed, "{situation}");
        let reason = stored.error.clone().expect("왜 멈췄는지가 남는다");
        assert!(!reason.contains(NOT_A_REAL_TOKEN), "{situation}: {reason}");
        assert_eq!(
            stored.total_chunks,
            Some(chunk_count as i64),
            "{situation}: 몇 개짜리였는지가 남는다"
        );

        // --- 재시도: 네 경우 모두 실제로 끝까지 간다 ---
        let retrying = Arc::new(StubServer::ready());
        let finished = if breaks.resumable() {
            // 같은 문서이므로 확인 없이 이어 보낸다 — 중복 페이지가 생기지 않는다 (§8.2).
            let sent = fixture
                .send_with(&recording_id, &retrying, Confirmation::NotAsked)
                .expect("이어 보내면 끝난다");
            assert!(!sent.created_page, "{situation}: 재시도가 새 페이지를 만들었다");
            assert_eq!(page_creations(&retrying), 0, "{situation}");

            // 두 시도를 합치면 원문 하나다 — 같은 조각이 두 번 나가지도, 하나가 빠지지도 않았다.
            let everything: String = failing
                .requests()
                .iter()
                .take(1)
                .chain(retrying.requests().iter())
                .map(sent_markdown)
                .collect::<Vec<_>>()
                .concat();
            assert!(
                everything == expected,
                "{situation}: 두 시도를 합친 것이 원문과 다르다 ({}바이트 vs {}바이트)",
                everything.len(),
                expected.len()
            );
            sent
        } else {
            // 페이지가 만들어졌는지 모르는 상태다. 조용히 다시 만들지 않는다 (§8.5).
            let asked = fixture
                .send_with(&recording_id, &retrying, Confirmation::NotAsked)
                .expect_err("결과를 모르는 채로 새 페이지를 만들지 않는다");
            assert_eq!(
                asked.detail.as_deref(),
                Some("needsConfirmation=outcomeUnknown"),
                "{situation}"
            );

            let sent = fixture
                .send_with(&recording_id, &retrying, Confirmation::NewPage)
                .expect("확인을 받으면 끝까지 간다");
            assert!(sent.created_page, "{situation}");
            let resent = reassembled(&retrying);
            assert!(
                resent == expected,
                "{situation}: 다시 보낸 문서가 원문과 다르다 ({}바이트 vs {}바이트)",
                resent.len(),
                expected.len()
            );
            sent
        };

        assert_eq!(finished.sync.status, ProcessingStatus::Done, "{situation}");
        assert_eq!(finished.sync.error, None, "{situation}");
        assert_eq!(
            finished.sync.sent_chunks,
            Some(chunk_count as i64),
            "{situation}"
        );

        // 실패도 재시도도 local data를 건드리지 않았다.
        let at_the_end = fixture.snapshot(&recording_id);
        assert_eq!(at_the_end.transcripts, before.transcripts, "{situation}");
        assert_eq!(at_the_end.notes, before.notes, "{situation}");
    }
}

// ---------------------------------------------------------------------------
// (d) 오디오 바이트도 오디오 경로도 Notion 요청 본문에 들어가지 않는다 (INV-6)
// ---------------------------------------------------------------------------
//
// `tests/notion_sync.rs::no_request_ever_carries_the_audio_bytes_or_even_the_path_to_them`은
// 오디오 파일이 **없는** 상태에서 요청을 관찰한다. 여기서는 반대로 **실제로 파일을 만들어 두고**
// 두 출구를 지난다 — 읽는 코드가 있었다면 그 바이트가 어딘가에 나타나고, 여기서 드러난다.
// 그리고 같은 사실을 소스에서도 못박는다 (`tests/audio_never_reaches_ai.rs`의 2번 각도와 같다).

/// 오디오 파일 안에만 있는 표시. **이 문자열이 어디에도 나타나면 안 된다.**
const AUDIO_MARKER: &str = "MOLT-NOTE-AUDIO-BYTES-MARKER";

#[test]
fn an_audio_file_that_really_exists_reaches_neither_the_file_nor_the_request() {
    let fixture = Fixture::new("real-audio-file");

    // 실제 파일을 만든다 — RIFF 헤더와 표시를 담은 바이트다.
    let recordings = fixture.app_data_dir.recordings_dir();
    fs::create_dir_all(&recordings).expect("사전 조건: 녹음 디렉터리를 만든다");
    let audio = recordings.join("rec-1.wav");
    let mut bytes: Vec<u8> = format!("RIFF----WAVEfmt {AUDIO_MARKER}").into_bytes();
    bytes.extend(std::iter::repeat_n(0x7fu8, 4_096));
    fs::write(&audio, &bytes).expect("사전 조건: 오디오 파일을 만든다");

    let audio_path = audio.display().to_string();
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04", &audio_path);
    fixture.save_transcript(&recording_id, 40);

    let server = Arc::new(StubServer::ready());
    let exported = fixture.export(&recording_id).expect("내보낼 수 있다");
    fixture
        .send_with(&recording_id, &server, Confirmation::NotAsked)
        .expect("보낼 수 있다");

    let requests = server.requests();
    assert!(!requests.is_empty(), "사전 조건: 요청이 나갔다");

    // 오디오를 가리킬 수 있는 모든 모양 — 바이트 · 파일 이름 · 경로 · 열 이름.
    let forbidden = [
        AUDIO_MARKER,
        "RIFF",
        "WAVEfmt",
        "rec-1.wav",
        ".wav",
        "recordings",
        "audio_path",
        "audioPath",
        "audio_format",
    ];

    for request in &requests {
        let body = request.body.clone().unwrap_or_default();
        for shape in forbidden {
            assert!(!body.contains(shape), "요청 본문에 오디오가 실렸다: {shape}");
            assert!(!request.url.contains(shape), "요청 주소에 오디오가 실렸다: {shape}");
        }
    }

    // 파일로 나가는 쪽도 같다 — 내보낸 문서에도 오디오는 없다.
    let file_body = written(&exported);
    for shape in forbidden {
        assert!(
            !file_body.contains(shape),
            "내보낸 문서에 오디오가 실렸다: {shape}"
        );
    }

    // 오디오 파일 자체는 손대지 않았다 (INV-3).
    assert_eq!(
        fs::read(&audio).expect("오디오 파일을 읽는다"),
        bytes,
        "전송이나 export가 오디오 파일을 바꿨다"
    );

    // ★ 오디오를 지워도 두 출구가 그대로 성립한다 — 읽는 코드가 있었다면 여기서 드러난다.
    fs::remove_file(&audio).expect("오디오 파일을 지운다");
    assert!(!audio.exists(), "사전 조건: 오디오가 사라졌다");

    fixture
        .export(&recording_id)
        .expect("오디오가 없어도 내보낼 수 있다");
    let sent = fixture
        .send_with(
            &recording_id,
            &Arc::new(StubServer::ready()),
            Confirmation::NewPage,
        )
        .expect("오디오가 없어도 보낼 수 있다");
    assert_eq!(sent.sync.status, ProcessingStatus::Done);
}

#[test]
fn nothing_in_the_notion_boundary_can_open_or_read_a_file() {
    // "그런 코드를 안 짰다"가 아니라 **쓸 수단이 없다**를 본다. 전송 경계에 파일을 여는 수단이
    // 하나라도 있으면 오디오를 요청에 싣는 일은 언제든 한 줄로 가능해진다.
    let mut offenders: Vec<String> = Vec::new();

    for path in rust_sources(&["src/notion", "src/sync"]) {
        let shown = path.display().to_string().replace('\\', "/");
        let source = product_code(&fs::read_to_string(&path).expect("소스 파일을 읽는다"));

        for forbidden in [
            "std::fs",
            "fs::",
            "File::",
            "OpenOptions",
            "include_bytes!",
            "read_dir",
            "audio",
            "PathBuf",
        ] {
            if source.contains(forbidden) {
                offenders.push(format!("{shown} — {forbidden}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Notion 전송 경계의 제품 코드가 파일에 닿을 수 있다 (INV-6): {offenders:?}"
    );
}

// ---------------------------------------------------------------------------
// (e) token이 SQLite에도 실패 문장에도 없다 (INV-7)
// ---------------------------------------------------------------------------
//
// 이미 있는 검사는 셋이다 — 실패 값 자체는 `tests/notion_adapter.rs`, 자격증명 경계의 타입과
// 소스는 `tests/secret_store.rs`, frontend 소스와 wire 계약은 `tests/screen-boundary.test.ts` ·
// `tests/ipc-boundary.test.ts`. `src/db/migrations.rs`의 단위 테스트는 migration **SQL 문자열**에
// secret 열이 없음을 본다.
//
// 여기서 새로 못박는 것은 그 사이에 남는 자리다: **실제로 만들어진 스키마**와 **실제로 디스크에
// 쓰인 바이트**. 앱이 실제로 token을 들고 전송을 성공시키고 실패시킨 뒤에도, 그 값이 SQLite
// 어디에도 남지 않고 사용자가 보는 실패 문장에도 없다는 것을 본다.

#[test]
fn a_token_that_actually_sent_a_recording_is_nowhere_in_the_database_or_in_any_failure() {
    let fixture = Fixture::new("token-nowhere");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04", "recordings/rec-1.wav");
    fixture.save_transcript(&recording_id, 40);

    // token이 사는 자리는 여기 하나다. **실제 Keychain을 열지 않는다.**
    let secrets = Arc::new(InMemorySecretStore::new());
    secrets
        .set(
            SecretKey::NotionIntegrationToken,
            &Secret::new(NOT_A_REAL_TOKEN),
        )
        .expect("사전 조건: token을 저장한다");
    let store_handle: Arc<dyn SecretStore> = secrets.clone();

    // --- 성공하는 전송 하나 (product 경로 — command가 자격증명 저장소에서 token을 읽는다) ---
    let sender = NotionSender::with_transport(
        fixture.app_data_dir.clone(),
        Arc::new(StubServer::ready()),
        store_handle.clone(),
        Arc::new(RecordedWaits::new()),
    );
    sender
        .start(recording_id.as_str(), Confirmation::NotAsked)
        .expect("접수된다");
    match wait_for(&sender) {
        NotionSendStatus::Done { page_id, .. } => assert_eq!(page_id, CREATED_PAGE_ID),
        other => panic!("전송이 끝나지 않았다: {other:?}"),
    }

    // --- 실패하는 전송 하나 (실패 문장이 만들어지는 자리) ---
    let refusing = NotionSender::with_transport(
        fixture.app_data_dir.clone(),
        Arc::new(StubServer::ready().with_create_page(StubReply::error(401, ApiErrorCode::Unauthorized))),
        store_handle,
        Arc::new(RecordedWaits::new()),
    );
    refusing
        .start(recording_id.as_str(), Confirmation::NewPage)
        .expect("접수된다");
    let failure = match wait_for(&refusing) {
        NotionSendStatus::Failed { failure, .. } => failure,
        other => panic!("실패했어야 한다: {other:?}"),
    };

    // ★ 사용자가 보는 문장 어디에도 token이 없다.
    assert_eq!(failure.kind, FailureKind::NotionAuthFailed);
    assert!(!failure.message.contains(NOT_A_REAL_TOKEN), "{}", failure.message);
    assert!(
        !failure
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains(NOT_A_REAL_TOKEN),
        "{:?}",
        failure.detail
    );
    let stored_reason = fixture.sync(&recording_id).error.unwrap_or_default();
    assert!(!stored_reason.contains(NOT_A_REAL_TOKEN), "{stored_reason}");

    // ★ 실제로 만들어진 스키마에 secret을 담을 자리가 없다.
    for (table, column) in schema_columns(&fixture.connection()) {
        for forbidden in ["api_key", "apikey", "token", "password", "secret", "credential"] {
            assert!(
                !column.to_lowercase().contains(forbidden),
                "스키마에 secret을 담을 자리가 있다: {table}.{column} (INV-7)"
            );
        }
    }

    // ★ 앱 데이터 디렉터리의 어떤 파일에도 그 값의 바이트가 없다 — DB 본체 · WAL · export까지.
    let mut files = Vec::new();
    collect_files(fixture.app_data_dir.root(), &mut files);
    assert!(!files.is_empty(), "사전 조건: 앱 데이터 파일이 있다");
    for path in &files {
        let bytes = fs::read(path).expect("파일을 읽는다");
        assert!(
            !contains(&bytes, NOT_A_REAL_TOKEN.as_bytes()),
            "저장된 파일에 token이 남았다: {} (INV-7)",
            path.display()
        );
    }

    // 그 값이 사는 자리는 여전히 자격증명 저장소 하나뿐이다.
    assert_eq!(
        secrets
            .stored(SecretKey::NotionIntegrationToken)
            .map(|secret| secret.expose().to_string()),
        Some(NOT_A_REAL_TOKEN.to_string()),
    );
}

/// 배경 전송이 끝날 때까지 기다린다. 전송은 스레드에서 돌고 상태로만 관찰된다.
fn wait_for(sender: &NotionSender) -> NotionSendStatus {
    let deadline = Instant::now() + Duration::from_secs(60);

    loop {
        match sender.status().expect("상태를 읽을 수 있다") {
            NotionSendStatus::Idle | NotionSendStatus::Running { .. } => {
                assert!(Instant::now() < deadline, "전송이 끝나지 않았다");
                std::thread::sleep(Duration::from_millis(10));
            }
            settled => return settled,
        }
    }
}

/// 실제로 만들어진 스키마의 `(table, column)` 전부.
fn schema_columns(connection: &rusqlite::Connection) -> Vec<(String, String)> {
    let tables: Vec<String> = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .expect("스키마를 읽는다")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("스키마를 읽는다")
        .map(|name| name.expect("이름을 읽는다"))
        .collect();
    assert!(!tables.is_empty(), "사전 조건: 스키마에 테이블이 있다");

    let mut columns = Vec::new();
    for table in tables {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info(\"{table}\")"))
            .expect("열을 읽는다");
        let names = statement
            .query_map([], |row| row.get::<_, String>(1))
            .expect("열을 읽는다");
        for name in names {
            columns.push((table.clone(), name.expect("열 이름을 읽는다")));
        }
    }

    columns
}

/// 그 디렉터리 아래의 모든 파일.
fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };

    for entry in entries {
        let path = entry.expect("디렉터리 항목을 읽는다").path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

/// 바이트 열 안에 그 바이트 열이 있는가.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

// ---------------------------------------------------------------------------
// (f) 두 renderer가 provider 중립 Structured Note만 소비한다 (INV-9)
// ---------------------------------------------------------------------------
//
// `tests/domain_invariants.rs`가 domain · store · migration에 벤더 지식이 없음을 판정한다.
// 여기서 보는 것은 이 Phase가 새로 만든 자리다 — 두 출구가 **어떤 provider가 만든 노트인지와
// 무관하게 같은 문서를 낸다**. provider 이름을 보고 다르게 그리는 코드가 하나라도 있으면
// 산출물은 provider마다 갈리고, 사용자의 Notion 페이지는 어느 AI를 썼는지에 따라 달라진다.

#[test]
fn both_renderers_consume_the_neutral_note_and_never_the_vendors_own_shape() {
    // 벤더 이름을 그대로 provider · model에 넣는다. 그 값이 산출물을 바꾸면 여기서 드러난다.
    let vendors = [
        ("claude", "claude-opus-4"),
        ("gemini", "gemini-2.0-pro"),
        ("ollama", "llama3.1:8b"),
    ];

    for note in every_note_mode() {
        let mut documents: Vec<String> = Vec::new();

        for (provider, model) in vendors {
            let fixture = Fixture::new("neutral-note");
            let recording_id =
                fixture.save_recording("rec-1", "3DGS Study #04", "recordings/rec-1.wav");
            let transcript_id = fixture.save_transcript(&recording_id, 4);
            fixture.save_note(&recording_id, &transcript_id, &note, provider, model);

            let exported = fixture.export(&recording_id).expect("내보낼 수 있다");
            let file_body = written(&exported);

            let server = Arc::new(StubServer::ready());
            fixture
                .send_with(&recording_id, &server, Confirmation::NotAsked)
                .expect("보낼 수 있다");

            // 두 출구가 같은 문서를 낸다 (ADR-0009 §14).
            assert_eq!(
                file_body,
                reassembled(&server),
                "{provider}: 두 출구의 문서가 다르다"
            );

            // 벤더 이름은 노트 행에만 있고 산출물에는 없다.
            for shape in [provider, model] {
                assert!(
                    !file_body.contains(shape),
                    "{provider}: 산출물에 벤더가 실렸다: {shape} (INV-9)"
                );
            }

            documents.push(file_body);
        }

        // ★ provider가 무엇이든 같은 노트는 같은 문서가 된다.
        assert!(
            documents.windows(2).all(|pair| pair[0] == pair[1]),
            "provider에 따라 산출물이 갈렸다 (INV-9)"
        );

        // 그 문서의 섹션 제목은 §9.5의 이름 그대로다 — 벤더 응답의 필드 이름이 아니다.
        let sections = match note.mode() {
            NoteType::Meeting => MEETING_SECTIONS.as_slice(),
            NoteType::Study => STUDY_SECTIONS.as_slice(),
            NoteType::Summary => SUMMARY_SECTIONS.as_slice(),
        };
        let document = documents.first().expect("문서가 있다");
        for section in sections {
            assert!(
                document.contains(&format!("## {section}\n")),
                "§9.5의 섹션이 없다: {section}"
            );
        }
    }
}

#[test]
fn no_source_on_the_way_out_knows_an_ai_vendor() {
    // 렌더링 · 조각내기 · 전송 어디에도 벤더 이름이 없다. 있으면 그 자리는 provider 중립이
    // 아니며, 언제든 "이 provider일 때만" 분기가 생길 수 있다 (INV-9).
    let vendors = [
        "CLAUDE",
        "ANTHROPIC",
        "GEMINI",
        "OPENAI",
        "GPT-",
        "OLLAMA",
        "GROQ",
        "MISTRAL",
        "LLAMA",
    ];

    for path in rust_sources(&["src/export", "src/notion", "src/sync"]) {
        let source = product_code(&fs::read_to_string(&path).expect("소스 파일을 읽는다"));
        let text = source.to_uppercase();

        for vendor in vendors {
            assert!(
                !text.contains(vendor),
                "산출물로 가는 경로에 벤더 지식이 들어왔다: {} — {vendor} (INV-9)",
                path.display()
            );
        }
    }
}
