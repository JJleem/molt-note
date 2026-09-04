//! **오디오가 AI provider로 나갈 수 있는 경로가 없다** (INV-6 · PRODUCT-SPEC §12 ·
//! `docs/ADR-0008-note-ai-provider.md` §4.2).
//!
//! "그런 코드를 안 짰다"는 검증이 아니다. 이 파일은 같은 사실을 **서로 다른 네 각도**에서
//! 못박으며, 넷 중 어느 하나만 깨져도 실패한다.
//!
//! ```text
//! 1. 타입      요청에 오디오를 담을 자리가 없다 — 필드를 전부 나열하는 구조 분해가 그것을
//!              컴파일 시점에 고정한다
//! 2. 소스      ai 경계의 제품 코드에 파일을 열거나 읽는 수단이 하나도 없다
//! 3. 전선      실제 adapter가 실제로 보내는 본문이 **mode · transcript 텍스트 · 모델 이름 ·
//!              context 크기**에서만 만들어진다 (키 집합과 프롬프트 문자열을 그대로 비교한다)
//! 4. 행동      오디오 파일을 지워도 노트 생성이 그대로 성공한다 — 읽는 코드가 있었다면
//!              여기서 드러난다
//! ```
//!
//! **실제 Ollama도 네트워크도 요구하지 않는다** (§18). HTTP 경계 자리에는 요청을 그대로
//! 기록하는 test double(`ai::ollama::testing::StubServer`)이 서고, 그 기록이 "무엇이 나갔는가"에
//! 대한 제3자의 관찰이다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use molt_note_lib::ai::ollama::testing::{StubServer, MODEL_IN_THE_LIST};
use molt_note_lib::ai::ollama::{HttpTransport, OllamaProvider};
use molt_note_lib::ai::prompt::ContextBudget;
use molt_note_lib::ai::run::{self, Outcome};
use molt_note_lib::ai::testing::FakeNoteAiProvider;
use molt_note_lib::ai::{build_prompt, NoteRequest, TranscriptText};
use molt_note_lib::db::{self, store};
use molt_note_lib::domain::{
    NoteType, ProcessingStatus, Recording, RecordingId, Transcript, TranscriptId, TranscriptSegment,
};
use rusqlite::Connection;

/// 테스트가 쓰는 연결 대상. **실재하지 않는다** — `.invalid`는 어떤 이름 해석도 성공하지
/// 않도록 예약된 TLD이며, 그래서 이 파일은 소켓을 열 수 없다.
const CONFIGURED_BASE_URL: &str = "http://configured-host.invalid:65535";

/// 오디오 파일에 담는 바이트. **이 문자열이 요청 어디에서 발견되면 INV-6이 깨진 것이다.**
///
/// 진짜 오디오일 필요가 없다 — AI 경계는 이 파일을 열지 않으므로 내용이 무엇이든 상관없고,
/// 확인 대상은 "이 바이트가 밖으로 나갔는가"와 "그대로 남아 있는가" 둘뿐이다.
const AUDIO_BYTES: &[u8] = b"MOLT-NOTE-AUDIO-BYTES-THAT-MUST-NEVER-BE-SENT";

/// 전사 텍스트. 프롬프트에 실려 **나가는 것이 정상인** 유일한 사용자 데이터다 (§9.6).
const TRANSCRIPT_TEXT: &str = "오늘 회의에서는 다음 분기 일정과 남은 과제를 이야기했다.";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-audio-never-reaches-ai-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 오디오 파일과 current Transcript를 가진 Recording 하나.
struct Fixture {
    _dir: TempDir,
    connection: Connection,
    recording_id: RecordingId,
    audio_path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let dir = TempDir::new(label);
        let audio_path = dir.join("recording.wav");
        fs::write(&audio_path, AUDIO_BYTES).expect("사전 조건: 오디오 파일을 만든다");

        let mut connection = db::open(dir.join("molt-note.db")).expect("임시 DB를 열 수 있어야 한다");
        let recording_id = RecordingId::new("rec-audio-boundary");
        let transcript_id = TranscriptId::new("tr-audio-boundary");

        store::insert_recording(
            &connection,
            &Recording {
                id: recording_id.clone(),
                title: "3DGS Study #04".to_owned(),
                created_at: "2026-09-03T10:00:00.000Z".to_owned(),
                updated_at: "2026-09-03T10:00:00.000Z".to_owned(),
                duration_ms: 3_151_000,
                audio_path: audio_path.to_str().expect("경로 문자열").to_owned(),
                audio_format: "wav".to_owned(),
                microphone: Some("MacBook Pro Microphone".to_owned()),
                current_transcript_id: None,
                transcription_status: ProcessingStatus::Done,
                ai_status: ProcessingStatus::None,
                notion_status: ProcessingStatus::None,
            },
        )
        .expect("사전 조건: 녹음 레코드를 저장한다");

        store::append_transcript(
            &mut connection,
            &Transcript {
                id: transcript_id.clone(),
                recording_id: recording_id.clone(),
                language: Some("ko".to_owned()),
                segments: vec![TranscriptSegment {
                    start_ms: 0,
                    end_ms: 2_000,
                    text: TRANSCRIPT_TEXT.to_owned(),
                }],
                raw_text: TRANSCRIPT_TEXT.to_owned(),
                created_at: "2026-09-03T10:01:00.000Z".to_owned(),
                engine: "fixture-engine".to_owned(),
                model: "fixture-model".to_owned(),
            },
        )
        .expect("사전 조건: Transcript를 추가한다");

        store::set_current_transcript(
            &connection,
            &recording_id,
            Some(&transcript_id),
            "2026-09-03T10:02:00.000Z",
        )
        .expect("사전 조건: current Transcript를 지정한다");

        Self {
            _dir: dir,
            connection,
            recording_id,
            audio_path,
        }
    }

    /// 실제 orchestration + 실제 adapter로 노트 하나를 만든다. HTTP 경계만 double이다.
    fn generate_through(&self, mode: NoteType, server: &Arc<StubServer>) -> Outcome {
        let transport: Arc<dyn HttpTransport> = server.clone();
        let provider = OllamaProvider::new(CONFIGURED_BASE_URL, MODEL_IN_THE_LIST, transport);

        run::generate(
            &self.connection,
            &self.recording_id,
            mode,
            &provider,
            ContextBudget::DEFAULT,
        )
        .expect("노트 생성이 성공해야 한다")
    }

    fn audio_path_text(&self) -> String {
        self.audio_path.display().to_string()
    }
}

// --- 1. 타입 — 담을 자리가 없다 -----------------------------------------------------------

#[test]
fn the_request_that_reaches_a_provider_has_nowhere_to_put_audio() {
    // 이 구조 분해는 **필드를 전부 나열한다.** 오디오 경로·바이트·핸들·파일 핸들을 담는 필드가
    // 하나라도 생기면 이 테스트는 실행되기 전에 컴파일되지 않는다.
    let NoteRequest {
        mode,
        transcript,
        context_budget,
    } = NoteRequest::new(NoteType::Meeting, TRANSCRIPT_TEXT);

    assert_eq!(mode, NoteType::Meeting);
    assert_eq!(context_budget, ContextBudget::DEFAULT);

    // 전사를 나르는 타입이 담을 수 있는 것도 문자열 하나뿐이다.
    let TranscriptText { text } = transcript;
    assert_eq!(text, TRANSCRIPT_TEXT);

    // 그리고 그 문자열을 만드는 방법도 문자열에서 오는 것뿐이다 — 경로도 바이트도 아니다.
    assert_eq!(TranscriptText::new("말해진 것").text, "말해진 것");
}

// --- 2. 소스 — 파일을 열 수단이 없다 -------------------------------------------------------

/// 벤더 adapter를 포함한 AI 경계 전부.
const AI_BOUNDARY: &str = "src/ai";

/// AI 노트 생성을 소유하는 command. 저장소는 열지만 **오디오 파일은 열지 않는다.**
const AI_COMMAND: &str = "src/commands/notes.rs";

#[test]
fn no_product_source_in_the_ai_boundary_can_open_or_read_a_file() {
    // `ai::run`에 대해서는 `tests/ai_note_run.rs`가 같은 검사를 한다. 여기서는 **경계 전체**로
    // 넓힌다 — 계약 · 프롬프트 · 노트 파싱 · 벤더 adapter까지 포함해서다.
    //
    // 검사 대상은 실행되는 줄이다. 주석은 빼고 보고(규칙을 문장으로 적는 것과 어기는 코드는
    // 다르다), `#[cfg(test)]` 아래도 뺀다 — 소스를 훑는 테스트 자신이 파일을 읽기 때문이다.
    let mut offenders: Vec<String> = Vec::new();

    for path in ai_boundary_sources() {
        let shown = path.display().to_string().replace('\\', "/");
        let source = fs::read_to_string(&path).expect("소스 파일을 읽는다");

        for forbidden in [
            "std::fs",
            "fs::",
            "File::",
            "OpenOptions",
            "include_bytes!",
            "PathBuf",
            "Path::new",
            "audio_path",
            "audio_format",
            "read_dir",
        ] {
            if product_code(&source).contains(forbidden) {
                offenders.push(format!("{shown} — {forbidden}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "AI 경계의 제품 코드가 파일에 닿을 수 있다 (INV-6): {offenders:?}"
    );
}

#[test]
fn the_orchestration_never_reads_the_column_that_points_at_the_audio() {
    // Recording 레코드에는 `audio_path`가 있다. 그 값을 읽는 순간 오디오를 보낼 **수단**이
    // 생기므로, AI 경계는 그 열을 이름으로도 알지 않는다.
    for path in ai_boundary_sources() {
        let source = product_code(&fs::read_to_string(&path).expect("소스 파일을 읽는다"));
        assert!(
            !source.contains("audio"),
            "AI 경계가 오디오를 이름으로 알고 있다: {} (INV-6)",
            path.display()
        );
    }
}

/// `src/ai` 아래의 모든 `.rs` 파일과, AI 생성을 소유하는 command 하나.
fn ai_boundary_sources() -> Vec<PathBuf> {
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
    walk(&root.join(AI_BOUNDARY), &mut files);
    files.push(root.join(AI_COMMAND));
    files.sort();
    files
}

/// 실행되는 제품 코드만 남긴다 — 주석과 `#[cfg(test)]` 아래를 잘라낸다.
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

// --- 3. 전선 — 실제로 나가는 본문에 무엇이 들어 있는가 -------------------------------------

#[test]
fn everything_that_goes_out_is_built_from_the_mode_and_the_transcript_text_alone() {
    // 실제 orchestration이 실제 adapter를 지나 실제 요청 본문을 만든다. 그 본문을 기록하는 것은
    // 제품 코드가 아니라 HTTP 경계의 double이다 — 제3자가 관찰한 결과다.
    for mode in NoteType::ALL {
        let fixture = Fixture::new("wire");
        let server = Arc::new(StubServer::ready());

        fixture.generate_through(mode, &server);

        let requests = server.requests();
        assert_eq!(requests.len(), 2, "{mode}: 목록 한 번 · 생성 한 번이다");

        // 목록 요청에는 본문 자체가 없다.
        assert_eq!(requests[0].body, None, "{mode}");

        let body: serde_json::Value =
            serde_json::from_str(requests[1].body.as_deref().expect("생성 요청에는 본문이 있다"))
                .expect("본문은 JSON이다");
        let object = body.as_object().expect("본문은 JSON 객체다");

        // **키 집합을 그대로 비교한다.** 오디오를 담을 새 필드가 생기면 여기서 드러난다.
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["format", "model", "options", "prompt", "stream"],
            "{mode}: 나가는 본문의 필드가 달라졌다"
        );

        // 그리고 값도 전부 알려진 것에서 나왔다 — 사용자 데이터는 `prompt` 하나뿐이며,
        // 그것은 domain이 전사 **텍스트**로 만든 문자열과 글자 하나까지 같다.
        assert_eq!(
            body["prompt"].as_str().expect("프롬프트는 문자열이다"),
            build_prompt(mode, TRANSCRIPT_TEXT),
            "{mode}: 프롬프트가 전사 텍스트 말고 다른 것에서 만들어졌다"
        );
        assert_eq!(body["model"], MODEL_IN_THE_LIST, "{mode}");
        assert_eq!(body["stream"], false, "{mode}");
        assert_eq!(
            body["options"]["num_ctx"],
            ContextBudget::DEFAULT.context_tokens,
            "{mode}"
        );

        // 마지막으로 오디오의 흔적을 직접 찾는다 — 바이트도, 경로도, 파일 이름도 없다.
        let audio_text = std::str::from_utf8(AUDIO_BYTES).expect("검사용 문자열이다");
        let audio_path = fixture.audio_path_text();
        for request in &requests {
            let sent = format!("{} {}", request.url, request.body.clone().unwrap_or_default());
            for trace in [audio_text, audio_path.as_str(), "recording.wav", ".wav"] {
                assert!(
                    !sent.contains(trace),
                    "{mode}: 오디오의 흔적이 밖으로 나갔다 ({trace}): {sent}"
                );
            }
        }
    }
}

#[test]
fn the_provider_is_handed_the_transcript_text_and_nothing_else() {
    // 계약이 실제로 무엇을 받는지는 double의 기록으로 확인한다. **기록에 오디오가 없는 것은
    // double의 선택이 아니다** — 계약에 그것을 담을 자리가 없기 때문이다.
    let fixture = Fixture::new("handed");
    let provider = FakeNoteAiProvider::ready();

    run::generate(
        &fixture.connection,
        &fixture.recording_id,
        NoteType::Summary,
        &provider,
        ContextBudget::DEFAULT,
    )
    .expect("노트 생성이 성공해야 한다");

    let calls = provider.calls();
    assert_eq!(calls.len(), 1, "한 번의 생성은 한 번의 호출이다");
    assert_eq!(calls[0].mode, NoteType::Summary);
    assert_eq!(calls[0].transcript, TRANSCRIPT_TEXT, "current Transcript의 텍스트다");
    assert_eq!(calls[0].context_tokens, ContextBudget::DEFAULT.context_tokens);

    // 오디오는 한 바이트도 바뀌지 않았다 (INV-1).
    assert_eq!(
        fs::read(&fixture.audio_path).expect("오디오를 읽을 수 있어야 한다"),
        AUDIO_BYTES
    );
}

// --- 4. 행동 — 읽는 코드가 있었다면 여기서 드러난다 ----------------------------------------

#[test]
fn a_note_is_generated_even_when_the_audio_file_is_not_there_at_all() {
    // AI 경로가 오디오를 **한 번이라도** 열었다면 이 테스트는 실패한다. 파일이 없기 때문이다.
    //
    // (이런 상태는 제품이 만들지 않는다 — 파일은 앱 밖에서 지워질 수 있을 뿐이며, 그것을 아는
    // 수단은 `Storage::missing_audio`다. 여기서는 그 상태를 **INV-6의 관찰 도구**로 쓴다.)
    let fixture = Fixture::new("no-audio-file");
    let server = Arc::new(StubServer::ready());

    // 먼저 파일이 있는 채로 한 번 — 그리고 그 바이트가 그대로인지 본다.
    fixture.generate_through(NoteType::Meeting, &server);
    assert_eq!(
        fs::read(&fixture.audio_path).expect("오디오를 읽을 수 있어야 한다"),
        AUDIO_BYTES,
        "생성이 오디오를 고쳤다 (INV-1)"
    );

    // 이제 파일을 치우고 같은 일을 다시 한다.
    fs::remove_file(&fixture.audio_path).expect("사전 조건: 오디오 파일을 치운다");
    assert!(!fixture.audio_path.exists(), "사전 조건: 파일이 없다");

    let outcome = fixture.generate_through(NoteType::Meeting, &server);

    assert!(
        outcome.generated().is_some(),
        "오디오 없이도 노트가 만들어진다 — AI 경로는 그 파일을 열지 않는다 (INV-6)"
    );

    // 레코드는 여전히 없는 파일을 가리킨다. AI 경로가 그것을 고치거나 지우지 않는다 (INV-4).
    let recording = store::load_recording(&fixture.connection, &fixture.recording_id)
        .expect("녹음을 읽을 수 있어야 한다")
        .expect("녹음이 남아 있어야 한다");
    assert_eq!(recording.audio_path, fixture.audio_path_text());
    assert!(!fixture.audio_path.exists(), "없는 파일을 만들어 내지도 않는다");
}
