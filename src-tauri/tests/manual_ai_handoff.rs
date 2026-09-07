//! **저장된 녹음 하나가 사람이 가져갈 수 있는 산출물 셋이 된다**
//! (`docs/ADR-0010-manual-ai-handoff.md` §8 · `phase-prompt/05.5` Required Outcome A · B).
//!
//! 순수 렌더러(`export::ai_request` · `export::filename`)는 자기 파일 안에서 값으로 검증된다 —
//! 세 산출물의 기대 문자열도, 벤더 부재(MH-6)도, audio 부재(MH-4)도 거기 있다. 여기서 판정하는
//! 것은 **그 문자열이 저장소·파일시스템과 만나는 자리**이며, 그 자리에만 있는 네 가지 질문에
//! 답한다.
//!
//! ```text
//! 1. AI Provider를 하나도 고르지 않아도 셋이 전부 동작하는가          (MH-1 · MH-2)
//! 2. current가 가리키는 Transcript만 쓰는가                            (MH-5 · §7.2)
//! 3. 실패가 domain Failure이고, 그 뒤에도 저장된 것이 그대로인가       (MH-7 · INV-3)
//! 4. Export for AI가 기존 자리를 재사용하고 덮어쓰지 않는가            (ADR-0010 §5.6 · §4.3)
//! ```
//!
//! **전부 임시 디렉터리에서 돈다.** 경로는 `std::env::temp_dir()`에서만 나오므로 사용자의 실제
//! export 디렉터리에는 아무것도 쓰이지 않는다 (§18).
//!
//! AI 서버도, 실제 whisper도, 마이크도, 네트워크도, clipboard도, Tauri 런타임도 필요하지 않다 —
//! 저장소에 값을 직접 넣고 제품 경로([`Storage`] · [`Exporter`])를 그대로 지난다. 통합
//! 테스트이므로 crate의 공개 API만 쓴다.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::commands::{ExportedFilePayload, Exporter, Storage};
use molt_note_lib::db::{self, settings, store};
use molt_note_lib::domain::{
    AiNote, Failure, FailureKind, NoteType, ProcessingStatus, Recording, RecordingId, Settings,
    Transcript, TranscriptId, TranscriptSegment,
};
use molt_note_lib::export::{AiRequest, PORTION_MAX_BYTES};
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
            "molt-note-manual-handoff-{}-{}-{}",
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

/// 저장소 하나와 그 위에서 도는 두 경계 — 읽어서 문자열을 만드는 쪽과 파일을 쓰는 쪽.
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

    /// 녹음 하나를 저장한다. 오디오 파일은 만들지 않는다 — 이 경로는 그것을 읽지 않는다 (INV-6).
    fn save_recording(&self, id: &str, title: &str) -> RecordingId {
        let recording = Recording {
            id: RecordingId::new(id),
            title: title.to_string(),
            created_at: CREATED_AT.to_string(),
            updated_at: CREATED_AT.to_string(),
            duration_ms: DURATION_MS,
            audio_path: format!("recordings/{id}.wav"),
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
            transcription_ms: None,
        };

        let mut connection = self.connection();
        store::append_transcript(&mut connection, &transcript).expect("사전 조건: 전사를 저장한다");
        if current {
            store::set_current_transcript(
                &connection,
                recording_id,
                Some(&transcript.id),
                CREATED_AT,
            )
            .expect("사전 조건: current를 지정한다");
        }

        transcript.id
    }

    /// **72분 규모의 전사** — segment 하나가 한 줄이 되는 압축 모양에서 예산을 넘긴다
    /// (`phase-prompt/05.6` R-5의 실측: segment 1,711개 · 99 KB).
    fn save_long_transcript(
        &self,
        recording_id: &RecordingId,
        id: &str,
        segments: usize,
    ) -> TranscriptId {
        let segments: Vec<TranscriptSegment> = (0..segments)
            .map(|index| {
                let start_ms = (index as i64) * 3_000;
                TranscriptSegment {
                    start_ms,
                    end_ms: start_ms + 3_000,
                    text: format!(
                        "발표자는 {index}번째 구간에서 3D Gaussian Splatting의 학습 파이프라인과 \
                         렌더링 품질 지표를 설명했고, 다음 주까지 실험 결과를 정리해 공유하기로 했다."
                    ),
                }
            })
            .collect();

        let transcript = Transcript {
            id: TranscriptId::new(id),
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
            transcription_ms: None,
        };

        let mut connection = self.connection();
        store::append_transcript(&mut connection, &transcript).expect("사전 조건: 전사를 저장한다");
        store::set_current_transcript(&connection, recording_id, Some(&transcript.id), CREATED_AT)
            .expect("사전 조건: current를 지정한다");

        transcript.id
    }

    /// 저장된 녹음 하나. 순수 렌더러가 만드는 기대 문자열을 세우는 데 쓴다.
    fn recording(&self, recording_id: &RecordingId) -> Recording {
        store::load_recording(&self.connection(), recording_id)
            .expect("녹음을 읽는다")
            .expect("녹음이 있다")
    }

    /// 저장된 전사 하나.
    fn transcript(&self, id: &str) -> Transcript {
        store::load_transcript(&self.connection(), &TranscriptId::new(id))
            .expect("전사를 읽는다")
            .expect("전사가 있다")
    }

    /// 프롬프트의 **첫 조각**. 이 파일의 전사는 전부 한 조각에 들어가므로 그것이 전체다
    /// (`phase-prompt/05.6` 성공 기준 4의 나눔은 `src/export/handoff.rs`가 따로 본다).
    fn ai_prompt(&self, recording_id: &RecordingId, mode: NoteType) -> Result<String, Failure> {
        let taken = self.storage.ai_prompt(recording_id.as_str(), mode, None)?;
        assert!(taken.portion.portion_count == 1, "사전 조건: 한 조각이다");

        Ok(taken.text)
    }

    fn transcript_text(&self, recording_id: &RecordingId) -> Result<String, Failure> {
        let taken = self.storage.transcript_text(recording_id.as_str(), None)?;
        assert!(taken.portion.portion_count == 1, "사전 조건: 한 조각이다");

        Ok(taken.text)
    }

    fn export_ai_request(
        &self,
        recording_id: &RecordingId,
        mode: NoteType,
    ) -> Result<ExportedFilePayload, Failure> {
        let written = self
            .exporter
            .export_ai_request(recording_id.as_str(), mode, None)?;
        assert!(written.portion.portion_count == 1, "사전 조건: 한 조각이다");

        Ok(written.file)
    }

    /// 저장된 것 전부를 한 값으로 찍는다. **실패 전후를 그대로 비교하기 위한 것이다** (MH-7).
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

    /// `exports/`에 지금 있는 파일 이름과 내용 전부. 없으면 빈 목록이다.
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

/// recording · transcript · ai_note를 한 번에 비교하기 위한 값 (INV-3 · MH-7).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Snapshot {
    recording: Recording,
    transcripts: Vec<Transcript>,
    notes: Vec<AiNote>,
}

// --- 1. AI Provider가 하나도 없어도 셋이 전부 동작한다 (MH-1 · MH-2) ---------------------

#[test]
fn all_three_outputs_are_produced_without_any_ai_provider_configured() {
    // Phase 5.5의 성공 기준 그 자체다 — **AI Provider가 하나도 없어도 AI의 값을 얻는다.**
    let fixture = Fixture::new("no-provider");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "안녕하세요. 오늘은 3DGS를 봅니다.", true);

    // 사전 조건: 고른 provider도, 모델도, 주소도 없다. 그것이 기본이자 정상 상태다 (INV-8).
    let stored = settings::load(&fixture.connection()).expect("설정을 읽을 수 있어야 한다");
    assert_eq!(stored, Settings::DEFAULT);
    assert!(stored.ai_provider.is_none(), "사전 조건: provider가 없다");
    assert!(stored.ai_model.is_none());
    assert!(stored.ai_base_url.is_none());

    for mode in NoteType::ALL {
        let prompt = fixture
            .ai_prompt(&recording_id, mode)
            .expect("provider가 없어도 프롬프트를 만들 수 있다");
        assert!(prompt.contains("안녕하세요. 오늘은 3DGS를 봅니다."));
        assert!(prompt.contains("Return the note as Markdown and nothing else."));

        let exported = fixture
            .export_ai_request(&recording_id, mode)
            .expect("provider가 없어도 내보낼 수 있다");
        assert!(Path::new(&exported.path).is_file());
    }

    let text = fixture
        .transcript_text(&recording_id)
        .expect("provider가 없어도 전사를 복사할 수 있다");
    // 압축 모양이다 — segment 하나가 `HH:MM:SS 문장` 한 줄이다 (`phase-prompt/05.6` R-5).
    assert_eq!(text, "## Transcript\n00:00:03 안녕하세요. 오늘은 3DGS를 봅니다.\n");
}

#[test]
fn the_written_document_is_exactly_what_the_pure_renderer_makes() {
    // 파일이 되는 자리가 문자열을 다시 손대지 않는다는 것 — 그래야 순수 모듈의 golden 테스트가
    // 실제 산출물에 대한 판정이 된다.
    let fixture = Fixture::new("same-bytes");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "먼저 splat 표현부터 보겠습니다.", true);

    let exported = fixture
        .export_ai_request(&recording_id, NoteType::Study)
        .expect("내보낼 수 있어야 한다");

    let connection = fixture.connection();
    let recording = store::load_recording(&connection, &recording_id)
        .expect("녹음을 읽는다")
        .expect("녹음이 있다");
    let transcript = store::load_transcript(&connection, &TranscriptId::new("tr-1"))
        .expect("전사를 읽는다")
        .expect("전사가 있다");

    assert_eq!(
        fs::read_to_string(&exported.path).expect("내보낸 파일을 읽는다"),
        AiRequest::new(NoteType::Study, &recording, &transcript).ai_ready_document()
    );
}

// --- 2. current가 가리키는 Transcript만 쓴다 (MH-5 · §7.2) ------------------------------

#[test]
fn only_the_transcript_that_current_points_at_reaches_any_of_the_three_outputs() {
    // 같은 녹음에 version이 둘 있고, current는 나중 것이다. **옛 version을 고를 수단이 이
    // 경계에 없다** — command가 `transcriptId`를 받지 않기 때문이다 (ADR-0010 §8.2).
    let fixture = Fixture::new("current-only");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-old", "옛 version의 문장", false);
    fixture.save_transcript(&recording_id, "tr-current", "지금 version의 문장", true);

    // 사전 조건: 저장소에는 둘 다 있다.
    assert_eq!(
        fixture.snapshot(&recording_id).transcripts.len(),
        2,
        "사전 조건: 두 version이 함께 있어야 한다"
    );

    let exported = fixture
        .export_ai_request(&recording_id, NoteType::Meeting)
        .expect("내보낼 수 있어야 한다");

    for text in [
        fixture
            .ai_prompt(&recording_id, NoteType::Meeting)
            .expect("프롬프트를 만들 수 있어야 한다"),
        fixture
            .transcript_text(&recording_id)
            .expect("전사를 복사할 수 있어야 한다"),
        fs::read_to_string(&exported.path).expect("내보낸 파일을 읽는다"),
    ] {
        assert!(text.contains("지금 version의 문장"), "current가 쓰여야 한다");
        assert!(
            !text.contains("옛 version의 문장"),
            "옛 version이 조용히 섞였다: {text}"
        );
    }
}

// --- 3. 실패가 저장된 것을 훼손하지 않는다 (MH-7 · INV-3) -------------------------------

#[test]
fn a_recording_without_a_current_transcript_is_refused_and_nothing_changes() {
    let fixture = Fixture::new("no-transcript");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    // 전사는 저장돼 있지만 current가 아니다 — 그래서 **고를 수 있는 입력이 없다** (§7.2).
    fixture.save_transcript(&recording_id, "tr-1", "current가 아닌 문장", false);

    let before = fixture.snapshot(&recording_id);

    let failures = vec![
        fixture
            .ai_prompt(&recording_id, NoteType::Study)
            .expect_err("고를 입력이 없다"),
        fixture
            .transcript_text(&recording_id)
            .expect_err("고를 입력이 없다"),
        fixture
            .export_ai_request(&recording_id, NoteType::Study)
            .map(|_| ())
            .expect_err("고를 입력이 없다"),
    ];

    for failure in &failures {
        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.retryable, "전사가 끝나면 성공할 수 있다");
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (MH-7)");
        // provider를 이유로 거절하지 않는다 (MH-1 · MH-2).
        assert!(!failure.message.to_lowercase().contains("provider"));
    }

    assert_eq!(fixture.snapshot(&recording_id), before, "저장된 것이 바뀌었다");
    assert_eq!(fixture.exported_files(), Vec::new(), "파일이 남았다");
}

#[test]
fn a_transcript_with_nothing_in_it_is_refused_instead_of_making_an_empty_request() {
    // 빈 요청을 만드는 것은 사용자가 원한 일이 아니다 (ADR-0010 §5.5). 순수 렌더러는 받은 값을
    // 그대로 렌더하므로, 거절하는 자리는 이 경계다.
    let fixture = Fixture::new("empty-transcript");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "   ", true);

    let before = fixture.snapshot(&recording_id);

    for failure in [
        fixture
            .ai_prompt(&recording_id, NoteType::Summary)
            .expect_err("적을 것이 없다"),
        fixture
            .transcript_text(&recording_id)
            .expect_err("적을 것이 없다"),
        fixture
            .export_ai_request(&recording_id, NoteType::Summary)
            .map(|_| ())
            .expect_err("적을 것이 없다"),
    ] {
        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.source_data_safe);
    }

    assert_eq!(fixture.snapshot(&recording_id), before);
    assert_eq!(
        fixture.exported_files(),
        Vec::new(),
        "빈 문서가 export 디렉터리에 쌓이지 않는다"
    );
}

#[test]
fn asking_for_a_recording_that_is_not_there_leaves_the_others_alone() {
    let fixture = Fixture::new("unknown-recording");
    let kept = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&kept, "tr-1", "남아 있어야 하는 문장", true);
    let before = fixture.snapshot(&kept);

    let missing = RecordingId::new("rec-없음");
    for failure in [
        fixture.ai_prompt(&missing, NoteType::Meeting).expect_err("없다"),
        fixture.transcript_text(&missing).expect_err("없다"),
        fixture
            .export_ai_request(&missing, NoteType::Meeting)
            .map(|_| ())
            .expect_err("없다"),
    ] {
        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(!failure.retryable, "같은 id로 다시 해도 결과가 같다");
        assert!(failure.source_data_safe);
    }

    assert_eq!(fixture.snapshot(&kept), before);
    assert_eq!(fixture.exported_files(), Vec::new());
}

// --- 4. Export for AI는 기존 자리를 재사용하고 덮어쓰지 않는다 (ADR-0010 §5.6 · §4.3) ----

#[test]
fn the_ai_request_lands_in_the_same_exports_directory_under_its_own_name() {
    let fixture = Fixture::new("same-directory");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "첫 문장", true);

    let markdown = fixture
        .exporter
        .export(recording_id.as_str())
        .expect("Markdown export가 성공해야 한다");
    let ai_request = fixture
        .export_ai_request(&recording_id, NoteType::Study)
        .expect("AI-ready 문서를 내보낼 수 있어야 한다");

    // 같은 자리다 — 두 번째 디렉터리를 만들지 않는다 (ADR-0009 §4.1).
    assert_eq!(
        Path::new(&ai_request.path).parent(),
        Some(fixture.exports_dir().as_path())
    );
    // 이름의 표식 하나로 구분된다 — `-2` 접미사에 기대지 않는다 (ADR-0010 §5.6).
    assert_eq!(markdown.file_name, "2026-09-01-3dgs-study-04.md");
    assert_eq!(ai_request.file_name, "2026-09-01-3dgs-study-04-ai-request.md");
    assert_eq!(ai_request.recording_id, "rec-1");
    assert!(Path::new(&markdown.path).is_file(), "먼저 만든 파일이 그대로 있다");
    assert!(Path::new(&ai_request.path).is_file());

    // 첫 줄에서 두 문서가 구분된다 (ADR-0010 §5.2).
    let document = fs::read_to_string(&ai_request.path).expect("파일을 읽는다");
    assert!(document.starts_with("# Molt Note AI Request\n"));
    assert!(
        fs::read_to_string(&markdown.path)
            .expect("파일을 읽는다")
            .starts_with("# 3DGS Study #04\n"),
        "Markdown export의 산출물은 이 Phase가 바꾸지 않는다"
    );
}

#[test]
fn a_second_ai_request_gets_a_number_instead_of_overwriting_the_first() {
    // ADR-0009 §4.3의 정책을 그대로 물려받는다 — 내보낸 파일은 **사용자의 문서**다.
    let fixture = Fixture::new("no-overwrite");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "첫 문장", true);

    let first = fixture
        .export_ai_request(&recording_id, NoteType::Study)
        .expect("첫 번째");
    fs::write(&first.path, "사용자가 손댄 문서").expect("사전 조건: 파일을 손댄다");

    let second = fixture
        .export_ai_request(&recording_id, NoteType::Meeting)
        .expect("이름이 겹쳐도 성공한다");

    assert_eq!(second.file_name, "2026-09-01-3dgs-study-04-ai-request-2.md");
    assert_eq!(
        fs::read_to_string(&first.path).expect("첫 파일을 읽는다"),
        "사용자가 손댄 문서",
        "이미 있는 파일은 덮어써지지 않는다"
    );
    assert_ne!(first.path, second.path);
    assert!(
        fs::read_to_string(&second.path)
            .expect("둘째 파일을 읽는다")
            .contains("## Mode\nMeeting\n"),
        "두 번째 요청의 mode가 그대로 들어간다"
    );
}

// --- 5. 크기 때문에 조용히 실패하지 않는다 (`phase-prompt/05.6` 성공 기준 4 · R-5) --------

#[test]
fn a_long_recording_says_how_big_it_is_and_hands_over_in_ordered_portions() {
    // 2026-09-05의 실사용: 72분 녹음의 산출물이 99 KB였고, 사람은 그것을 채팅 창에 붙여 넣으려다
    // 실패했으며 **앱은 그 사실을 말해 주지 않았다.** 여기서 판정하는 것은 그 침묵이 사라졌는가다.
    let fixture = Fixture::new("long-recording");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_long_transcript(&recording_id, "tr-1", 1_711);

    for mode in NoteType::ALL {
        let first = fixture
            .storage
            .ai_prompt(recording_id.as_str(), mode, None)
            .expect("provider가 없어도 첫 조각을 가져올 수 있다");

        // 1. 얼마나 큰가가 값으로 온다 — 이 값이 없으면 화면은 크기를 말할 수 없다.
        assert!(
            first.portion.total_size.bytes > 2 * PORTION_MAX_BYTES,
            "{mode}: 72분 규모라기에 너무 작다: {} 바이트",
            first.portion.total_size.bytes
        );
        assert!(first.portion.total_size.chars > 0);
        assert!(first.portion.total_size.lines > 0);

        // 2. 나뉘었다는 사실과 자기 자리가 값으로 온다.
        assert!(first.portion.portion_count > 2, "{mode}: 나뉘지 않았다");
        assert_eq!(first.portion.portion, 1);
        assert!(
            first.portion.portion_size.bytes < first.portion.total_size.bytes,
            "{mode}: 조각이 전체와 같다고 말한다"
        );

        // 3. 순서대로 다 가져가면 원본이다 — 크기 때문에 잃는 글자가 없다.
        let rejoined: String = (1..=first.portion.portion_count)
            .map(|portion| {
                let taken = fixture
                    .storage
                    .ai_prompt(recording_id.as_str(), mode, Some(portion))
                    .expect("조각을 가져올 수 있다");
                assert_eq!(taken.portion.portion, portion);
                assert_eq!(taken.portion.portion_count, first.portion.portion_count);
                assert_eq!(taken.portion.total_size, first.portion.total_size);
                assert_eq!(taken.recording_id, "rec-1");
                taken.text
            })
            .collect();

        let whole = AiRequest::new(mode, &fixture.recording(&recording_id), &fixture.transcript("tr-1"))
            .manual_prompt();
        assert_eq!(rejoined, whole, "{mode}: 이어 붙인 결과가 원본과 다르다");
    }

    // 전사 텍스트도 같은 규칙을 지난다 — 여기서만 조용하면 사용자는 같은 자리에서 다시 막힌다.
    let text = fixture
        .storage
        .transcript_text(recording_id.as_str(), None)
        .expect("전사 텍스트의 첫 조각");
    assert!(text.portion.portion_count > 2);
    assert!(text.portion.total_size.lines > 1_000, "segment 하나가 한 줄이다");
}

#[test]
fn asking_for_a_portion_that_is_not_there_is_refused_instead_of_coming_back_empty() {
    // 빈 텍스트로 답하면 사용자는 그것을 "가져갈 것이 더 없다"로 읽는다 — **잘린 결과를 완전한
    // 것처럼 말하는 상태를 만들지 않는다** (성공 기준 4).
    let fixture = Fixture::new("no-such-portion");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_transcript(&recording_id, "tr-1", "한 조각에 들어가는 짧은 전사.", true);

    let taken = fixture
        .storage
        .transcript_text(recording_id.as_str(), None)
        .expect("첫 조각");
    assert_eq!(taken.portion.portion_count, 1, "사전 조건: 한 조각이다");

    for portion in [0, 2, 9] {
        let failure = fixture
            .storage
            .transcript_text(recording_id.as_str(), Some(portion))
            .expect_err("없는 조각이다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.source_data_safe, "아무것도 건드리지 않았다 (MH-7)");
        assert!(!failure.message.trim().is_empty(), "화면에 띄울 문장이 있다");
    }

    // 저장된 것도, export 디렉터리도 그대로다.
    assert_eq!(fixture.exported_files(), Vec::new());
}

#[test]
fn every_portion_of_a_long_document_becomes_its_own_file_and_none_overwrites_another() {
    // 나눔이 사용자의 문서를 지우는 경로가 되지 않는다 (ADR-0009 §4.3).
    let fixture = Fixture::new("portion-files");
    let recording_id = fixture.save_recording("rec-1", "3DGS Study #04");
    fixture.save_long_transcript(&recording_id, "tr-1", 1_711);

    let first = fixture
        .exporter
        .export_ai_request(recording_id.as_str(), NoteType::Study, None)
        .expect("첫 조각을 파일로 쓸 수 있다");
    let count = first.portion.portion_count;
    assert!(count > 2, "나뉘지 않았다");

    let mut written = vec![first];
    for portion in 2..=count {
        written.push(
            fixture
                .exporter
                .export_ai_request(recording_id.as_str(), NoteType::Study, Some(portion))
                .expect("나머지 조각도 파일로 쓸 수 있다"),
        );
    }

    // 1. 파일 이름이 **몇 번째 조각인지** 말한다 — 충돌 번호(`-2`)에 기대지 않는다.
    let names: Vec<String> = written
        .iter()
        .map(|one| one.file.file_name.clone())
        .collect();
    assert_eq!(names[0], format!("2026-09-01-3dgs-study-04-ai-request-part-1-of-{count}.md"));
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), count, "같은 이름으로 쓰인 조각이 있다");

    // 2. 조각마다 파일이 하나씩 생겼고, 어느 것도 덮어써지지 않았다.
    let files = fixture.exported_files();
    assert_eq!(files.len(), count);
    for one in &written {
        assert!(Path::new(&one.file.path).is_file());
    }

    // 3. 파일들을 순서대로 이어 붙이면 문서 전체다 — 크기 때문에 잃은 글자가 없다.
    let rejoined: String = written
        .iter()
        .map(|one| fs::read_to_string(&one.file.path).expect("내보낸 파일을 읽는다"))
        .collect();
    assert_eq!(
        rejoined,
        AiRequest::new(
            NoteType::Study,
            &fixture.recording(&recording_id),
            &fixture.transcript("tr-1")
        )
        .ai_ready_document()
    );

    // 4. 같은 조각을 또 내보내도 앞서 쓴 파일은 그대로다 (§4.3).
    fs::write(&written[1].file.path, "사용자가 손댄 문서").expect("사전 조건: 파일을 손댄다");
    let again = fixture
        .exporter
        .export_ai_request(recording_id.as_str(), NoteType::Study, Some(2))
        .expect("같은 조각을 또 쓸 수 있다");

    assert_ne!(again.file.path, written[1].file.path);
    assert_eq!(again.portion.portion, 2);
    assert_eq!(
        fs::read_to_string(&written[1].file.path).expect("손댄 파일을 읽는다"),
        "사용자가 손댄 문서"
    );
}

// --- 소스로 못박는다 — 이 경계에 provider도, 네트워크도, 저장소 쓰기도 없다 ---------------

#[test]
fn nothing_in_the_manual_handoff_boundary_can_reach_a_provider_a_network_or_a_write() {
    // "그런 코드를 안 짰다"가 아니라 **쓸 수단이 없다**를 본다 (MH-1 · MH-2 · MH-3 · MH-7).
    // provider를 읽는 줄이 하나라도 있으면, provider가 없다는 이유로 거절하는 일은 언제든 한
    // 줄로 가능해진다.
    let mut offenders: Vec<String> = Vec::new();

    for relative in ["src/export/handoff.rs", "src/export/ai_request.rs"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
        let source = product_code(&fs::read_to_string(&path).expect("소스 파일을 읽는다"));

        for forbidden in [
            // provider · AI 설정 (MH-1 · MH-2)
            "provider",
            "ai_base_url",
            "ai_model",
            "settings::",
            "Settings",
            // 벤더 (MH-2 · MH-6)
            "ollama",
            "Ollama",
            // 네트워크 (MH-3)
            "ureq",
            "http",
            // 저장소 쓰기 (MH-7 · INV-3)
            "insert_",
            "update_",
            "delete_",
            "append_",
            "set_current",
            "execute",
        ] {
            if source.contains(forbidden) {
                offenders.push(format!("{relative} — {forbidden}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "Manual AI Handoff 경계의 제품 코드가 닿아서는 안 되는 것에 닿는다: {offenders:?}"
    );
}

#[test]
fn the_three_commands_take_a_recording_id_and_never_a_transcript_id() {
    // MH-5를 **계약의 모양**으로 만든 자리다 (ADR-0010 §8.2). 인자 하나가 늘면 화면이든
    // 누구든 옛 version을 고를 수 있게 되므로, 그 사실이 여기서 먼저 드러난다.
    let source = product_code(
        &fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands/mod.rs"))
            .expect("소스 파일을 읽는다"),
    );

    for command in ["get_ai_prompt", "get_transcript_text", "export_ai_request"] {
        let start = source
            .find(&format!("pub fn {command}("))
            .unwrap_or_else(|| panic!("{command}가 command 표면에 있어야 한다"));
        let signature = &source[start..start + source[start..].find(')').expect("인자 목록이 닫힌다")];

        assert!(
            signature.contains("recording_id: String"),
            "{command}는 recordingId를 받는다: {signature}"
        );
        assert!(
            !signature.contains("transcript_id"),
            "{command}가 transcriptId를 받는다 (MH-5): {signature}"
        );
    }
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
