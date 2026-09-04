//! **Recording 하나가 실제로 Notion 페이지가 되는 순서** (PRODUCT-SPEC §7 · §10 ·
//! `docs/ADR-0009-notion-and-export.md` §8 · §9).
//!
//! 순수한 조각들은 각자 자기 파일 안에서 값으로 검증된다 — 문서를 만드는 렌더러
//! (`export::markdown`), 나누는 규칙(`notion::chunk`), 요청 하나를 보내는 adapter
//! (`notion::client`), 얼마나 기다릴지 정하는 정책(`sync::pace`). 여기서 판정하는 것은 **그 넷을
//! 잇는 순서**이며, 그 자리에만 있는 질문에 답한다.
//!
//! ```text
//! 1. AI Note가 하나도 없어도 Transcript만으로 전송된다            (INV-8 · P7-AC3)
//! 2. 여러 chunk가 순서대로 나가고, 이어 붙이면 원문과 같다        (P7-AC4)
//! 3. 두 번째 chunk에서 실패한 뒤의 재시도가 중복 페이지를 만들지 않는다 (P7-AC4 · §8.2)
//! 4. 실패 뒤에도 recording · transcript · ai_note가 그대로다      (INV-3 · P7-AC5)
//! 5. 오디오 바이트도 오디오 경로도 요청에 실리지 않는다           (INV-6 · P7-AC6)
//! 6. 상태 조회가 전송을 기다리지 않는다                           (P7-AC7)
//! ```
//!
//! **어떤 테스트도 실제 Notion에 요청하지 않는다** — HTTP 왕복은 `notion::testing::StubServer`
//! 뒤에 있다. **실제 자격증명 저장소도 열지 않는다** — token은 메모리 double 안에만 있다.
//! **한 밀리초도 자지 않는다** — 대기는 `sync::pace::testing::RecordedWaits`가 기록만 한다.
//! 저장소는 전부 시스템 임시 디렉터리 아래에서 만들어지고 Drop 때 지워진다 (§18 ·
//! `phase-prompt/05` Important Rules).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use molt_note_lib::ai::note::{encode_content, StructuredNote, SummaryNote};
use molt_note_lib::commands::{NotionSendStatus, NotionSender, Storage};
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{
    AiNote, AiNoteId, Failure, FailureKind, NotionSync, ProcessingStatus, Recording, RecordingId,
    Settings, Transcript, TranscriptId, TranscriptSegment,
};
use molt_note_lib::export::markdown::{render, ExportDocument};
use molt_note_lib::notion::testing::{StubReply, StubRequest, StubServer, CREATED_PAGE_ID};
use molt_note_lib::notion::wire::ApiErrorCode;
use molt_note_lib::notion::{split_markdown, HttpTransport, NotionClient};
use molt_note_lib::platform::app_data_dir::AppDataDirectory;
use molt_note_lib::platform::secret_store::testing::InMemorySecretStore;
use molt_note_lib::platform::secret_store::{Secret, SecretKey, SecretStore};
use molt_note_lib::sync::pace::testing::RecordedWaits;
use molt_note_lib::sync::pace::{Waiter, MIN_REQUEST_INTERVAL};
use molt_note_lib::sync::run::{send, Confirmation, Destination, Sent};

/// 이 파일이 쓰는 값들. **하나도 실재하지 않는다** (ADR-0009 §10.5).
const NOT_A_REAL_TOKEN: &str = "ntn-test-double-value-not-a-real-credential";
const PARENT_PAGE_ID: &str = "parent-page-identifier-not-a-real-page";
const CREATED_AT: &str = "2026-09-01T10:00:00.000Z";
const DURATION_MS: i64 = 3_151_000;

/// 오디오 파일의 자리. **이 문자열이 어떤 요청에도 나타나지 않는 것이 INV-6이다.**
const AUDIO_FILE: &str = "rec-audio-file-name.wav";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

// --- 자리 ----------------------------------------------------------------------------

/// 시스템 임시 디렉터리 아래의 고유 루트. Drop 시 지운다.
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-notion-sync-{}-{}-{}",
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

/// 저장소 하나와 그 위에서 도는 전송.
struct Fixture {
    _root: TempRoot,
    app_data_dir: AppDataDirectory,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = TempRoot::new(label);
        let app_data_dir = AppDataDirectory::new(root.path().join("app-data"));

        let storage = Storage::open(&app_data_dir);
        assert!(storage.failure().is_none(), "사전 조건: 저장소가 열려야 한다");

        let fixture = Self {
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

    /// 녹음 하나를 저장한다. **오디오 파일은 만들지 않는다** — 전송은 그것을 읽지 않는다 (INV-6).
    fn save_recording(&self, id: &str, title: &str) -> RecordingId {
        let recording = Recording {
            id: RecordingId::new(id),
            title: title.to_string(),
            created_at: CREATED_AT.to_string(),
            updated_at: CREATED_AT.to_string(),
            duration_ms: DURATION_MS,
            audio_path: format!("recordings/{AUDIO_FILE}"),
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
    fn save_transcript(&self, recording_id: &RecordingId, segments: usize) -> TranscriptId {
        let segments: Vec<TranscriptSegment> = (0..segments)
            .map(|index| TranscriptSegment {
                start_ms: (index as i64) * 6_000,
                end_ms: (index as i64) * 6_000 + 5_000,
                text: format!("{index}번째 문장이다. 이 문장은 chunk 예산을 채우기 위해 길게 적혀 있으며 내용에는 의미가 없다."),
            })
            .collect();

        let transcript = Transcript {
            id: TranscriptId::new("tr-1"),
            recording_id: recording_id.clone(),
            language: Some("ko".to_string()),
            raw_text: segments
                .iter()
                .map(|segment| segment.text.clone())
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

    /// AI 노트 하나를 저장한다 (§7.3).
    fn save_note(&self, recording_id: &RecordingId, transcript_id: &TranscriptId) {
        let note = StructuredNote::Summary(SummaryNote {
            short_summary: "세 줄 요약".to_string(),
            key_points: vec!["첫 번째".to_string(), "두 번째".to_string()],
        });
        let stored = AiNote {
            id: AiNoteId::new("note-1"),
            recording_id: recording_id.clone(),
            transcript_id: transcript_id.clone(),
            note_type: note.mode(),
            content: encode_content(&note),
            provider: "stub".to_string(),
            model: "stub-model".to_string(),
            prompt_version: "v1".to_string(),
            generated_at: "2026-09-01T11:00:00.000Z".to_string(),
        };
        store::insert_ai_note(&self.connection(), &stored).expect("사전 조건: 노트를 저장한다");
    }

    /// 이 Recording이 만들어 낼 Markdown 문서 그대로.
    ///
    /// **로컬 export와 같은 렌더러의 같은 호출이다** — 산출물이 하나라는 것이 이 함수의
    /// 존재 이유다 (ADR-0009 §14).
    fn document(&self, recording_id: &RecordingId, with_note: Option<&StructuredNote>) -> String {
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
            note: with_note,
        })
    }

    /// 전송 한 번. 실행 순서를 그대로 지난다.
    fn send_with(
        &self,
        recording_id: &RecordingId,
        server: &Arc<StubServer>,
        waiter: &dyn Waiter,
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
            waiter,
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
        let transcripts =
            store::list_transcripts(&connection, recording_id).expect("전사를 읽는다");
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
            notion_status: recording.notion_status,
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
    notion_status: ProcessingStatus,
}

/// 요청 하나에 실려 나간 markdown. 페이지 생성과 이어붙이기가 값을 담는 자리가 다르다.
fn sent_markdown(request: &StubRequest) -> String {
    let body: serde_json::Value =
        serde_json::from_str(request.body.as_deref().expect("본문이 있다")).expect("본문은 JSON이다");

    let value = match request.method.as_str() {
        "POST" => body["markdown"].clone(),
        _ => body["insert_content"]["content"].clone(),
    };

    value
        .as_str()
        .unwrap_or_else(|| panic!("요청에 markdown이 없다: {body}"))
        .to_string()
}

/// 조각들을 비교할 수 있는 값으로.
fn owned(chunks: &[&str]) -> Vec<String> {
    chunks.iter().map(|chunk| (*chunk).to_string()).collect()
}

/// 그 서버가 받은 요청 중 페이지를 **만드는** 것의 수.
fn page_creations(server: &StubServer) -> usize {
    server
        .requests()
        .iter()
        .filter(|request| request.method.as_str() == "POST")
        .count()
}

// ---------------------------------------------------------------------------
// 1. AI Note가 하나도 없어도 Transcript만으로 전송된다 (INV-8 · P7-AC3)
// ---------------------------------------------------------------------------

#[test]
fn a_recording_with_no_ai_note_at_all_is_sent_with_just_its_transcript() {
    let fixture = Fixture::new("no-ai-note");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, 2);

    let server = Arc::new(StubServer::ready());
    let waits = RecordedWaits::new();

    let sent = fixture
        .send_with(&recording_id, &server, &waits, Confirmation::NotAsked)
        .expect("AI 노트가 없어도 보낼 수 있어야 한다 (INV-8)");

    assert!(sent.created_page, "새 페이지 하나가 만들어졌다");
    assert_eq!(sent.sync.page_id.as_deref(), Some(CREATED_PAGE_ID));
    assert_eq!(sent.sync.status, ProcessingStatus::Done);
    assert_eq!(sent.sync.error, None);
    assert!(sent.sync.synced_at.is_some(), "언제 끝났는지가 남는다");

    // 노트가 없어도 유효한 문서다 — 제목 · Date · Duration · Transcript.
    let requests = server.requests();
    assert_eq!(requests.len(), 1, "짧은 문서는 요청 하나다");
    let document = sent_markdown(&requests[0]);
    assert!(document.starts_with("# 3DGS Study #04"), "{document}");
    assert!(document.contains("## Transcript"), "{document}");
    assert!(
        !document.contains("## Overview"),
        "없는 AI 섹션의 빈 껍데기를 만들지 않는다: {document}"
    );

    // 저장된 상태가 §7의 `NotionSync` 그대로다.
    let stored = fixture.sync(&recording_id);
    assert_eq!(stored, sent.sync, "돌려준 값과 저장된 값이 같다");
    assert_eq!(stored.sent_chunks, Some(1));
    assert_eq!(stored.total_chunks, Some(1));

    // Recording의 후처리 상태도 함께 옮겨진다 (§7).
    let recording = store::load_recording(&fixture.connection(), &recording_id)
        .expect("녹음을 읽는다")
        .expect("녹음이 있다");
    assert_eq!(recording.notion_status, ProcessingStatus::Done);
    assert_eq!(
        recording.ai_status,
        ProcessingStatus::None,
        "Notion이 남의 파이프라인 상태를 옮기지 않는다"
    );
}

#[test]
fn an_ai_note_is_included_when_there_is_one_and_the_body_is_the_local_export_verbatim() {
    let fixture = Fixture::new("with-ai-note");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let transcript_id = fixture.save_transcript(&recording_id, 2);
    fixture.save_note(&recording_id, &transcript_id);

    let server = Arc::new(StubServer::ready());
    fixture
        .send_with(&recording_id, &server, &RecordedWaits::new(), Confirmation::NotAsked)
        .expect("전송이 성공해야 한다");

    let sent: String = server
        .requests()
        .iter()
        .map(sent_markdown)
        .collect::<Vec<_>>()
        .concat();

    let note = StructuredNote::Summary(SummaryNote {
        short_summary: "세 줄 요약".to_string(),
        key_points: vec!["첫 번째".to_string(), "두 번째".to_string()],
    });
    assert_eq!(
        sent,
        fixture.document(&recording_id, Some(&note)),
        "Notion으로 가는 문자열과 로컬 export의 문자열은 같은 렌더러의 같은 결과다 (§14)"
    );
    assert!(sent.contains("## Short Summary"), "노트가 실렸다: {sent}");
}

// ---------------------------------------------------------------------------
// 2. 여러 chunk가 순서대로 나가고, 이어 붙이면 원문과 같다 (P7-AC4)
// ---------------------------------------------------------------------------

#[test]
fn a_long_document_goes_out_in_order_and_arrives_whole() {
    let fixture = Fixture::new("many-chunks");
    let recording_id = fixture.save_recording("rec-1", "긴 녹음");
    fixture.save_transcript(&recording_id, 700);

    let expected = fixture.document(&recording_id, None);
    let chunks = split_markdown(&expected).expect("나눌 수 있는 문서다");
    assert!(chunks.len() >= 3, "이 테스트는 여러 chunk를 요구한다: {}", chunks.len());

    let server = Arc::new(StubServer::ready());
    let waits = RecordedWaits::new();
    let sent = fixture
        .send_with(&recording_id, &server, &waits, Confirmation::NotAsked)
        .expect("전송이 성공해야 한다");

    let requests = server.requests();
    assert_eq!(requests.len(), chunks.len(), "조각 수만큼 요청이 나간다");

    // 첫 요청이 페이지를 만들고, 나머지는 그 페이지에 이어 붙인다.
    assert_eq!(requests[0].method.as_str(), "POST");
    for request in &requests[1..] {
        assert_eq!(request.method.as_str(), "PATCH");
        assert!(
            request.url.contains(CREATED_PAGE_ID),
            "만든 그 페이지에 이어 붙인다: {}",
            request.url
        );
    }

    // **순서대로 · 무손실로.** 보낸 것을 이어 붙이면 원문과 정확히 같다.
    let arrived: Vec<String> = requests.iter().map(sent_markdown).collect();
    assert_eq!(arrived, owned(&chunks), "나눈 순서 그대로 나갔다");
    assert_eq!(arrived.concat(), expected, "잘리지도 유실되지도 않았다");

    // 요청 사이에는 간격이 있다 (ADR-0009 §9.2-6). 첫 요청 앞에는 없다.
    assert_eq!(
        waits.waits(),
        vec![MIN_REQUEST_INTERVAL; chunks.len() - 1],
        "요청 사이에만 간격을 둔다"
    );

    let stored = fixture.sync(&recording_id);
    assert_eq!(stored.status, ProcessingStatus::Done);
    assert_eq!(stored.sent_chunks, Some(chunks.len() as i64));
    assert_eq!(stored.total_chunks, Some(chunks.len() as i64));
    assert!(sent.created_page);
}

// ---------------------------------------------------------------------------
// 3. 두 번째 chunk에서 실패한 뒤의 재시도가 중복 페이지를 만들지 않는다 (P7-AC4 · §8.2)
// ---------------------------------------------------------------------------

#[test]
fn a_retry_after_a_failed_second_chunk_continues_on_the_same_page_and_makes_no_duplicate() {
    let fixture = Fixture::new("resume");
    let recording_id = fixture.save_recording("rec-1", "긴 녹음");
    fixture.save_transcript(&recording_id, 700);

    let expected = fixture.document(&recording_id, None);
    let chunks = split_markdown(&expected).expect("나눌 수 있는 문서다");
    assert!(chunks.len() >= 3, "이 테스트는 여러 chunk를 요구한다");

    // --- 첫 시도: 페이지는 만들어지고 두 번째 chunk에서 실패한다 ---
    let failing = Arc::new(
        StubServer::ready().with_append(StubReply::error(500, ApiErrorCode::InternalServerError)),
    );
    let failure = fixture
        .send_with(&recording_id, &failing, &RecordedWaits::new(), Confirmation::NotAsked)
        .expect_err("두 번째 chunk가 실패한다");

    assert_eq!(failure.kind, FailureKind::NotionRequestFailed);
    assert_eq!(page_creations(&failing), 1, "페이지는 한 번만 만들어졌다");
    assert_eq!(failing.requests().len(), 2, "실패한 요청 뒤로는 보내지 않는다");

    // **부분 전송이 상태에서 드러난다** (§8.4).
    let after_failure = fixture.sync(&recording_id);
    assert_eq!(after_failure.status, ProcessingStatus::Failed);
    assert_eq!(after_failure.page_id.as_deref(), Some(CREATED_PAGE_ID));
    assert_eq!(after_failure.sent_chunks, Some(1), "성공한 것만 센다 (§8.4-4)");
    assert_eq!(after_failure.total_chunks, Some(chunks.len() as i64));
    assert!(after_failure.error.is_some(), "왜 멈췄는지가 남는다");

    // --- 재시도: 같은 페이지에 **아직 보내지 않은 chunk부터** 이어 붙인다 ---
    let resuming = Arc::new(StubServer::ready());
    let sent = fixture
        .send_with(&recording_id, &resuming, &RecordedWaits::new(), Confirmation::NotAsked)
        .expect("이어 보내면 끝난다");

    assert_eq!(
        page_creations(&resuming),
        0,
        "재시도가 새 페이지를 만들었다 — 조용한 중복이다 (§8.2)"
    );
    assert!(!sent.created_page, "이어 보낸 것이지 새로 만든 것이 아니다");
    assert_eq!(sent.sync.page_id.as_deref(), Some(CREATED_PAGE_ID));

    let resumed: Vec<String> = resuming.requests().iter().map(sent_markdown).collect();
    assert_eq!(
        resumed,
        owned(&chunks[1..]),
        "실패한 그 chunk부터 순서대로 이어 보낸다 — 건너뛰지도 앞질러 세지도 않는다"
    );

    // 두 시도를 합치면 원문 하나다. 같은 조각이 두 번 나가지 않았다.
    let everything: String = failing
        .requests()
        .iter()
        .take(1)
        .chain(resuming.requests().iter())
        .map(sent_markdown)
        .collect::<Vec<_>>()
        .concat();
    assert_eq!(everything, expected);

    let finished = fixture.sync(&recording_id);
    assert_eq!(finished.status, ProcessingStatus::Done);
    assert_eq!(finished.error, None, "끝난 전송에 실패 사유가 남지 않는다");
    assert_eq!(finished.sent_chunks, Some(chunks.len() as i64));
}

#[test]
fn a_finished_recording_is_never_sent_again_without_being_asked_first() {
    // §8.3: 이미 `done`인 것을 다시 보내면 **명시적으로** 새 페이지다. 기존 페이지는 그대로 둔다.
    let fixture = Fixture::new("already-done");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, 2);

    let first = Arc::new(StubServer::ready());
    fixture
        .send_with(&recording_id, &first, &RecordedWaits::new(), Confirmation::NotAsked)
        .expect("첫 전송이 성공한다");

    let again = Arc::new(StubServer::ready());
    let refused = fixture
        .send_with(&recording_id, &again, &RecordedWaits::new(), Confirmation::NotAsked)
        .expect_err("확인 없이 두 번째 페이지를 만들지 않는다");

    assert_eq!(refused.kind, FailureKind::InvalidInput);
    assert_eq!(
        refused.detail.as_deref(),
        Some("needsConfirmation=alreadySent")
    );
    assert!(
        again.requests().is_empty(),
        "거절된 요청은 Notion에 닿지도 않는다"
    );
    assert_eq!(
        fixture.sync(&recording_id).status,
        ProcessingStatus::Done,
        "거절이 이미 끝난 전송의 상태를 덮어쓰지 않는다"
    );

    // 확인을 받으면 새 페이지를 만든다.
    let confirmed = Arc::new(StubServer::ready());
    let sent = fixture
        .send_with(&recording_id, &confirmed, &RecordedWaits::new(), Confirmation::NewPage)
        .expect("확인을 받았으면 만든다");

    assert!(sent.created_page);
    assert_eq!(page_creations(&confirmed), 1);
}

// ---------------------------------------------------------------------------
// 4. 실패 뒤에도 local data가 그대로다 (INV-3 · P7-AC5)
// ---------------------------------------------------------------------------

#[test]
fn nothing_local_is_lost_or_changed_when_a_send_fails() {
    for (situation, reply, expected) in [
        (
            "인증 실패",
            StubReply::error(401, ApiErrorCode::Unauthorized),
            FailureKind::NotionAuthFailed,
        ),
        (
            "공유되지 않은 destination",
            StubReply::error(404, ApiErrorCode::ObjectNotFound),
            FailureKind::NotionDestinationUnavailable,
        ),
        (
            "네트워크 없음",
            StubReply::Fail(molt_note_lib::notion::TransportError::NotConnected),
            FailureKind::NotionRequestFailed,
        ),
    ] {
        let fixture = Fixture::new("failure-keeps-data");
        let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
        let transcript_id = fixture.save_transcript(&recording_id, 3);
        fixture.save_note(&recording_id, &transcript_id);

        let before = fixture.snapshot(&recording_id);
        let server = Arc::new(StubServer::ready().with_create_page(reply));

        let failure = fixture
            .send_with(&recording_id, &server, &RecordedWaits::new(), Confirmation::NotAsked)
            .expect_err("이 상황에서는 보낼 수 없다");

        assert_eq!(failure.kind, expected, "{situation}");
        assert!(
            failure.source_data_safe,
            "{situation}: 전송 실패는 원본을 건드리지 않는다"
        );

        // **오디오 · Transcript · AI Note · Recording이 그대로다** (INV-3).
        let after = fixture.snapshot(&recording_id);
        assert_eq!(after.recording, before.recording, "{situation}: 녹음 레코드");
        assert_eq!(after.transcripts, before.transcripts, "{situation}: 전사");
        assert_eq!(after.notes, before.notes, "{situation}: AI 노트");

        // 바뀌는 것은 전송 상태 둘뿐이다 (§8.5).
        assert_eq!(after.notion_status, ProcessingStatus::Failed, "{situation}");
        let sync = fixture.sync(&recording_id);
        assert_eq!(sync.status, ProcessingStatus::Failed, "{situation}");
        assert_eq!(
            sync.error.as_deref(),
            Some(failure.message.as_str()),
            "{situation}: 무엇이 실패했는지가 남는다"
        );
        assert_eq!(sync.page_id, None, "{situation}: 만들어진 페이지가 없다");
        assert_eq!(sync.sent_chunks, Some(0), "{situation}");
        assert!(sync.total_chunks.is_some(), "{situation}: 몇 개짜리였는지가 남는다");
        assert!(
            !sync
                .error
                .as_deref()
                .expect("실패 사유가 있다")
                .contains(NOT_A_REAL_TOKEN),
            "{situation}: 저장된 실패 사유에 자격증명이 섞였다 (INV-7)"
        );

        // 그 실패에서 다시 시도하려면 확인이 필요하다 — 페이지가 만들어졌는지 모르기 때문이다.
        let retry = fixture
            .send_with(&recording_id, &Arc::new(StubServer::ready()), &RecordedWaits::new(), Confirmation::NotAsked)
            .expect_err("결과를 모르는 채로 새 페이지를 만들지 않는다");
        assert_eq!(
            retry.detail.as_deref(),
            Some("needsConfirmation=outcomeUnknown"),
            "{situation}"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. 오디오는 어떤 형태로도 나가지 않는다 (INV-6 · P7-AC6)
// ---------------------------------------------------------------------------

#[test]
fn no_request_ever_carries_the_audio_bytes_or_even_the_path_to_them() {
    let fixture = Fixture::new("no-audio");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    let transcript_id = fixture.save_transcript(&recording_id, 40);
    fixture.save_note(&recording_id, &transcript_id);

    let server = Arc::new(StubServer::ready());
    fixture
        .send_with(&recording_id, &server, &RecordedWaits::new(), Confirmation::NotAsked)
        .expect("전송이 성공해야 한다");

    let requests = server.requests();
    assert!(!requests.is_empty(), "사전 조건: 요청이 나갔다");

    for request in &requests {
        let body = request.body.clone().unwrap_or_default();
        for forbidden in [AUDIO_FILE, "recordings/", ".wav", "RIFF", "audio_path"] {
            assert!(
                !body.contains(forbidden),
                "요청 본문에 오디오가 실렸다: {forbidden}"
            );
            assert!(
                !request.url.contains(forbidden),
                "요청 주소에 오디오가 실렸다: {forbidden}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 6. 속도 제한 — 서버가 말한 만큼 기다렸다가 **같은 chunk**를 다시 보낸다 (§9.2)
// ---------------------------------------------------------------------------

#[test]
fn a_rate_limited_request_waits_the_stated_seconds_and_then_sends_the_same_chunk_again() {
    let fixture = Fixture::new("rate-limit");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, 2);

    // 네 번 다 속도 제한이다 — 자동 재시도 상한(3회)을 넘긴다.
    let server = Arc::new(
        StubServer::ready().with_create_page(StubReply::rate_limited(429, Some("30"))),
    );
    let waits = RecordedWaits::new();

    let failure = fixture
        .send_with(&recording_id, &server, &waits, Confirmation::NotAsked)
        .expect_err("계속 제한되면 멈춘다");

    assert_eq!(failure.kind, FailureKind::NotionRateLimited);
    assert_eq!(
        server.requests().len(),
        4,
        "처음 한 번 + 자동 재시도 3회 (ADR-0009 §9.2-3)"
    );
    assert_eq!(
        waits.waits(),
        vec![Duration::from_secs(30); 3],
        "서버가 말한 초를 그대로 존중한다 — 값을 임의로 줄이지 않는다 (§9.2-1)"
    );
    assert_eq!(fixture.sync(&recording_id).sent_chunks, Some(0));
}

#[test]
fn a_wait_longer_than_this_app_will_sit_through_stops_instead_of_going_quiet() {
    // §9.2-4: 앱이 몇 분씩 조용히 멈춰 있는 것은 "멈춘 것"과 구분되지 않는다.
    let fixture = Fixture::new("long-wait");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, 2);

    let server = Arc::new(
        StubServer::ready().with_create_page(StubReply::rate_limited(529, Some("600"))),
    );
    let waits = RecordedWaits::new();

    let failure = fixture
        .send_with(&recording_id, &server, &waits, Confirmation::NotAsked)
        .expect_err("기다리지 않고 멈춘다");

    assert_eq!(failure.kind, FailureKind::NotionRateLimited);
    assert!(failure.message.contains("600"), "언제 다시 오면 되는지: {}", failure.message);
    assert_eq!(waits.waits(), Vec::<Duration>::new(), "그만큼 기다리지 않았다");
    assert_eq!(server.requests().len(), 1, "다시 보내지도 않았다");
}

// ---------------------------------------------------------------------------
// 7. 배경 스레드 규약 — 상태 조회가 전송을 기다리지 않는다 (P7-AC7)
// ---------------------------------------------------------------------------

/// 요청 사이의 간격에서 **붙잡혀 있는** 대기 자리.
///
/// 전송이 도는 한가운데를 만들어 내기 위한 것이다 — 그 상태에서 [`NotionSender::status`]가
/// 즉시 답하는지가 이 파일이 판정하려는 것이다.
struct HeldWaiter {
    released: Mutex<bool>,
    ready: Condvar,
    entered: Mutex<usize>,
}

impl HeldWaiter {
    fn new() -> Self {
        Self {
            released: Mutex::new(false),
            ready: Condvar::new(),
            entered: Mutex::new(0),
        }
    }

    fn is_waiting(&self) -> bool {
        *self.entered.lock().expect("잠금") > 0
    }

    fn release(&self) {
        *self.released.lock().expect("잠금") = true;
        self.ready.notify_all();
    }
}

impl Waiter for HeldWaiter {
    fn wait(&self, _duration: Duration) {
        *self.entered.lock().expect("잠금") += 1;

        let mut released = self.released.lock().expect("잠금");
        while !*released {
            released = self.ready.wait(released).expect("잠금");
        }
    }
}

#[test]
fn asking_for_the_status_answers_immediately_while_a_send_is_still_running() {
    let fixture = Fixture::new("background");
    let recording_id = fixture.save_recording("rec-1", "긴 녹음");
    fixture.save_transcript(&recording_id, 700);

    let secrets = InMemorySecretStore::new();
    secrets
        .set(
            SecretKey::NotionIntegrationToken,
            &Secret::new(NOT_A_REAL_TOKEN),
        )
        .expect("사전 조건: token을 저장한다");

    let server = Arc::new(StubServer::ready());
    let waiter = Arc::new(HeldWaiter::new());
    let sender = NotionSender::with_transport(
        fixture.app_data_dir.clone(),
        server.clone(),
        Arc::new(secrets),
        waiter.clone(),
    );

    let accepted = sender
        .start(recording_id.as_str(), Confirmation::NotAsked)
        .expect("접수된다");
    assert_eq!(
        accepted,
        NotionSendStatus::Running {
            recording_id: "rec-1".to_string()
        },
        "돌아오는 것은 결과가 아니라 접수 사실이다"
    );

    // 전송이 요청 사이에서 붙잡힐 때까지 기다린다 — 그 순간이 '전송 한가운데'다.
    let deadline = Instant::now() + Duration::from_secs(10);
    while !waiter.is_waiting() {
        assert!(Instant::now() < deadline, "배경 스레드가 시작되지 않았다");
        std::thread::sleep(Duration::from_millis(5));
    }

    // ★ 전송이 멈춰 있는 동안에도 상태 조회는 즉시 답한다.
    let asked_at = Instant::now();
    let status = sender.status().expect("상태를 읽을 수 있다");
    let spent = asked_at.elapsed();

    assert_eq!(
        status,
        NotionSendStatus::Running {
            recording_id: "rec-1".to_string()
        }
    );
    assert!(
        spent < Duration::from_secs(1),
        "상태 조회가 전송을 기다렸다: {spent:?}"
    );

    // 이미 보내는 중이면 두 번째 시작은 거절된다 — 조용히 사라지지 않는다.
    let refused = sender
        .start(recording_id.as_str(), Confirmation::NotAsked)
        .expect_err("한 번에 한 건이다");
    assert_eq!(refused.kind, FailureKind::InvalidInput);

    waiter.release();

    // 끝나면 그 사실이 상태에 남는다.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match sender.status().expect("상태를 읽을 수 있다") {
            NotionSendStatus::Done {
                page_id,
                created_page,
                ..
            } => {
                assert_eq!(page_id, CREATED_PAGE_ID);
                assert!(created_page);
                break;
            }
            NotionSendStatus::Failed { failure, .. } => panic!("전송이 실패했다: {failure:?}"),
            _ => {
                assert!(Instant::now() < deadline, "전송이 끝나지 않았다");
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    assert_eq!(fixture.sync(&recording_id).status, ProcessingStatus::Done);
    assert_eq!(page_creations(&server), 1, "페이지는 하나다");
}
