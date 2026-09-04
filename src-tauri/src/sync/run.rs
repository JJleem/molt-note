//! Recording 하나를 Notion 페이지로 보내는 **실행 순서**
//! (PRODUCT-SPEC §7 · §10 · `docs/ADR-0009-notion-and-export.md` §8 · §9).
//!
//! [`crate::export::run`]이 로컬 파일에 대해 하는 역할과 같은 자리이며, 같은 규칙을 따른다 —
//! 다른 것은 산출물이 파일이 아니라 페이지이고, 그래서 **중간에 멈출 수 있다**는 것뿐이다.
//! 그 한 가지 차이가 이 모듈에 있는 거의 모든 것의 이유다.
//!
//! ```text
//! 1. 문서를 만든다        current Transcript + (있으면) 최신 AI Note → 하나의 markdown
//! 2. 지문을 뜬다          sha256 hex — "그때 나눈 그 문서인가"의 유일한 근거 (§8.2)
//! 3. 나눈다               notion::split_markdown — 순서대로 · 무손실로 (§6)
//! 4. ★ 보내기 전에 적는다  status=running · total_chunks · fingerprint          (§8.4-2)
//! 5. 첫 chunk로 페이지     ★ page_id를 즉시 적는다                              (§8.4-3)
//! 6. 나머지를 순서대로     성공한 것만 sent_chunks로 센다                        (§8.4-4)
//! 7. 상태를 적는다        done + synced_at, 또는 failed + error                  (§8.4-5)
//! ```
//!
//! ## 4번과 5번이 이 모듈의 존재 이유다
//!
//! 요청을 보낸 **뒤에** 처음 기록하면, 그 사이에 앱이 죽었을 때 "보낸 적 있는가"를 답할 자료가
//! 아무것도 없다 — 그때 다음 전송은 반드시 중복 페이지를 만든다. 페이지는 만들어졌는데 그
//! 식별자를 잃어도 같다: 그 페이지는 앱이 다시 찾을 수 없는 고아가 된다.
//!
//! 그래서 **디스크에 적는 것이 다음 요청보다 먼저다.** 그 대가로 저장소 쓰기가 요청 수만큼
//! 늘어나지만, 그것이 "조용히 중복 페이지를 만들지 않는다"의 값이다.
//!
//! ## 실패한 요청은 보낸 것으로 세지 않는다 (§8.4-4)
//!
//! 세면 재시도가 그 chunk를 건너뛴다 — **조용한 유실**이다. 성공에서만 올리면 최악의 경우 같은
//! chunk를 두 번 보내는데, 그것은 페이지에 같은 문단이 두 번 나오는 형태로 **눈에 보인다**.
//! 유실보다 중복이 낫다 — 유실은 사용자가 알 수 없다.
//!
//! ## 여기서 하지 않는 것
//!
//! 스레드를 만들지 않고, Tauri command를 열지 않으며, token을 어디서 얻을지 · 어느 페이지 아래에
//! 만들지를 설정에서 읽지 **않는다**. 그 값들을 읽어 넘기는 것은 부르는 쪽의 일이다
//! ([`crate::commands::NotionSender`] — 노트 생성이 provider를 넘겨받는 것과 같다).

use rusqlite::Connection;

use crate::db::store;
use crate::domain::{
    Failure, FailureKind, NotionSync, ProcessingStatus, Recording, RecordingId, TranscriptId,
};
use crate::export::markdown::{render, ExportDocument};
use crate::notion::{split_markdown, NotionClient, NotionFailure, PageId};
use crate::platform::secret_store::Secret;

use super::pace::{self, Pause, Waiter, MAX_RETRIES_PER_CHUNK, MIN_REQUEST_INTERVAL};

/// 어디로, 무엇을 들고 보내는가.
///
/// **token은 빌려 올 뿐 어디에도 남지 않는다** (INV-7 · ADR-0009 §10.4). 이 타입은 값을 담지
/// 않고 참조만 들고 있으며, 그래서 전송 상태([`NotionSync`])에도 실패 문장에도 실릴 수 없다.
#[derive(Debug, Clone, Copy)]
pub struct Destination<'a> {
    pub token: &'a Secret,
    /// 새 페이지를 만들 부모 페이지의 식별자. **secret이 아니다** (ADR-0009 §8.4).
    pub parent_page_id: &'a str,
}

/// 사용자가 무엇을 확인했는가 (ADR-0009 §8.3 · §8.5).
///
/// **이 값이 중복 페이지를 막는 자리다.** 이어 보낼 수 없는 상태에서 새 페이지를 만드는 일은
/// 사용자가 알고 누른 결과여야 하며, 앱이 스스로 고르지 않는다 — Phase Goal이 금지한 "조용한
/// 중복"이 정확히 그것이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    /// 아무것도 확인하지 않았다. **확인이 필요한 상태면 보내지 않고 거절한다.**
    NotAsked,
    /// 사용자가 '새 페이지 만들기'를 확인했다. 기존 페이지는 그대로 둔다 (§8.3).
    NewPage,
}

/// 전송 한 번이 끝난 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    /// 저장된 전송 상태 그대로. 부르는 쪽이 다시 읽지 않아도 되게 한다.
    pub sync: NotionSync,
    /// 이번 실행에서 **새로 만든** 페이지인가. 이어 보낸 것이면 `false`다.
    pub created_page: bool,
}

/// Recording 하나를 Notion 페이지 하나로 보내고, 그 결과를 [`NotionSync`]로 남긴다.
///
/// 기본 입력은 **`current_transcript_id`가 가리키는 Transcript**다 (§7.2). AI Note는 있으면
/// 넣고 없으면 넣지 않는다 (INV-8) — 없다는 이유로 거절하지 않는다.
///
/// 이미 끝나지 않은 전송이 있고 그때와 같은 문서라면 **새 페이지를 만들지 않고 이어 보낸다**
/// (§8.2). 이어 보낼 수 없는 상태에서는 `confirmation`이 [`Confirmation::NewPage`]일 때만 새
/// 페이지를 만들며, 그렇지 않으면 무엇을 확인해야 하는지를 실패로 알린다 (§8.5).
///
/// 실패해도 recording · transcript · ai_note · 오디오 파일은 그대로다 (INV-3). 바뀌는 것은
/// `notion_syncs` 행과 `recordings.notion_status` · `updated_at`뿐이다.
pub fn send(
    connection: &Connection,
    recording_id: &RecordingId,
    client: &NotionClient,
    destination: &Destination<'_>,
    waiter: &dyn Waiter,
    confirmation: Confirmation,
) -> Result<Sent, Failure> {
    // 상태를 쓸 대상이 실재하는지부터 본다. 없는 Recording에 대해서는 아무것도 쓰지 않는다.
    let recording = store::load_recording(connection, recording_id)?
        .ok_or_else(|| unknown_recording(recording_id))?;

    let document = document_of(connection, &recording)?;
    let chunks = split_markdown(&document.markdown).map_err(cannot_split)?;
    if chunks.is_empty() {
        // 렌더러는 언제나 제목 줄을 내므로 정상적으로는 오지 않는다. 그래도 빈 요청을 만들어
        // 내지 않는다 — 보낼 것이 없다는 사실을 그대로 말한다.
        return Err(nothing_to_send(recording_id));
    }

    // **요청을 보내기 전에** 지금 상태로 무엇을 할 수 있는지 판정한다 (§8.5). 여기서 거절되는
    // 경로는 저장소도 Notion도 건드리지 않는다.
    let existing = store::load_notion_sync(connection, recording_id)?;
    let plan = plan(existing.as_ref(), &document.fingerprint, confirmation)?;

    let mut progress = Progress {
        page: plan.page(),
        sent: plan.sent(chunks.len()),
        total: chunks.len() as i64,
        fingerprint: document.fingerprint,
    };

    // ★ §8.4-2 — 첫 요청보다 먼저 적는다.
    persist(connection, &recording, &progress, ProcessingStatus::Running, None, None)?;

    let created_page = progress.page.is_none();
    match transmit(
        connection,
        &recording,
        client,
        destination,
        waiter,
        &chunks,
        &mut progress,
    ) {
        Ok(()) => {
            let synced_at = store::now(connection)?;
            let sync = persist(
                connection,
                &recording,
                &progress,
                ProcessingStatus::Done,
                Some(synced_at),
                None,
            )?;
            Ok(Sent { sync, created_page })
        }
        Err(failure) => Err(record_failure(connection, &recording, &progress, failure)),
    }
}

/// 보낼 문서 하나와 그 지문.
struct Document {
    markdown: String,
    /// sha256 hex. **이어 붙여도 되는가**를 판정하는 유일한 근거다 (§8.2).
    fingerprint: String,
}

/// 저장소에서 읽어 보낼 문서를 만든다. **읽기뿐이다.**
fn document_of(connection: &Connection, recording: &Recording) -> Result<Document, Failure> {
    // 기본 입력은 `current_transcript_id`가 가리키는 Transcript다 (§7.2). **다른 version을
    // 추측해서 고르지 않는다.**
    let Some(transcript_id) = recording.current_transcript_id.clone() else {
        return Err(nothing_to_send(&recording.id));
    };
    let transcript = store::load_transcript(connection, &transcript_id)?
        .ok_or_else(|| dangling_transcript(&transcript_id))?;

    // **없는 것이 정상이다** (INV-8). 고르는 규칙은 로컬 export와 같은 함수 하나다 — 그래야
    // 같은 Recording의 파일과 페이지가 서로 다른 노트를 담지 않는다.
    let note = crate::export::run::latest_note(connection, &transcript.id)?;

    let markdown = render(&ExportDocument {
        recording,
        transcript: &transcript,
        note: note.as_ref(),
    });
    let fingerprint = content_fingerprint(&markdown);

    Ok(Document {
        markdown,
        fingerprint,
    })
}

/// 문서 하나의 지문 (sha256 hex · ADR-0009 §8.4).
///
/// 같은 문서는 언제나 같은 지문이고, 한 글자라도 다르면 다른 지문이다. 그 성질 하나로 "지금
/// 보내려는 것이 그때 나눈 그 문서인가"를 판정한다 — **아니면 이어 붙이지 않는다.** 이어 붙이면
/// 서로 다른 두 문서가 페이지 하나에 섞이고, 사용자는 그 사실을 알 수 없다.
pub fn content_fingerprint(markdown: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(markdown.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 지금 상태에서 무엇을 할 것인가 (§8.5).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Plan {
    /// 새 페이지를 만든다. 첫 chunk가 페이지가 되고 그 첫 `# h1`이 제목이 된다 (§5.3).
    CreatePage,
    /// 이미 만들어진 페이지에 **아직 보내지 않은 chunk부터** 이어 붙인다 (§8.2).
    Resume { page: PageId, sent: i64 },
}

impl Plan {
    fn page(&self) -> Option<PageId> {
        match self {
            Self::CreatePage => None,
            Self::Resume { page, .. } => Some(page.clone()),
        }
    }

    /// 이미 반영된 조각 수. 저장된 값이 지금 문서의 조각 수를 넘지 않게 잘라 둔다 — 그 값으로
    /// 조각 배열을 인덱싱하기 때문이다.
    fn sent(&self, total: usize) -> i64 {
        match self {
            Self::CreatePage => 0,
            Self::Resume { sent, .. } => (*sent).min(total as i64),
        }
    }
}

/// 저장된 상태 · 지금 문서의 지문 · 사용자가 확인한 것에서 계획 하나를 정한다 (§8.5).
///
/// | 저장된 상태 | 판정 |
/// | --- | --- |
/// | 행이 없다 · `none` | 새 페이지 |
/// | `pending` · `running` | 거절 — 이미 보내는 중이다 |
/// | `failed` + 페이지 + 같은 지문 | **이어 보낸다** (확인 대화 없이 — 중복이 생기지 않는다) |
/// | `failed` + 페이지 + 다른 지문 | 문서가 바뀌었다. 확인 뒤 새 페이지 |
/// | `failed` + 페이지 없음 | 결과를 모른다. 확인 뒤 새 페이지 |
/// | `done` | 확인 뒤 새 페이지. 기존 페이지는 건드리지 않는다 (§8.3) |
fn plan(
    existing: Option<&NotionSync>,
    fingerprint: &str,
    confirmation: Confirmation,
) -> Result<Plan, Failure> {
    let Some(sync) = existing else {
        return Ok(Plan::CreatePage);
    };

    match sync.status {
        ProcessingStatus::None => Ok(Plan::CreatePage),
        // 진행 중인 전송을 두 번 굴리면 같은 페이지에 같은 문단이 두 번 들어간다.
        ProcessingStatus::Pending | ProcessingStatus::Running => Err(already_sending(sync)),
        // 이미 사용자의 Notion 문서다. 덮어쓰지도 지우지도 않는다 (§8.3).
        ProcessingStatus::Done => new_page_after(confirmation, ConfirmBecause::AlreadySent),
        ProcessingStatus::Failed => match resumable(sync, fingerprint) {
            Some(plan) => Ok(plan),
            None => new_page_after(confirmation, why_not_resumable(sync)),
        },
    }
}

/// 이어 보낼 수 있는가 — 세 값이 **전부** 있어야 한다 (§8.2).
///
/// 페이지가 있고, 그때 나눈 문서가 지금 문서와 같고, 어디까지 보냈는지를 안다. 하나라도
/// 모르면 이어 붙이지 않는다 — 모르는 채로 이어 붙이면 조용한 유실이나 조용한 중복이 된다.
fn resumable(sync: &NotionSync, fingerprint: &str) -> Option<Plan> {
    let page = PageId::parse(sync.page_id.as_deref()?)?;
    let stored = sync.content_fingerprint.as_deref()?;
    let sent = sync.sent_chunks?;

    // 페이지가 있다는 것은 첫 chunk가 반영됐다는 뜻이다. 그보다 작은 값은 저장소가 어긋난
    // 것이며, 그 값으로 이어 붙이면 첫 chunk를 다시 보내게 된다.
    (stored == fingerprint && sent >= 1).then_some(Plan::Resume { page, sent })
}

/// 왜 이어 보낼 수 없는가. 사용자가 확인해야 할 것이 서로 다르다.
fn why_not_resumable(sync: &NotionSync) -> ConfirmBecause {
    match sync.page_id.as_deref() {
        // 페이지는 있는데 그때 보낸 문서가 지금 것과 다르다.
        Some(_) => ConfirmBecause::DocumentChanged,
        // 페이지가 만들어졌는지조차 모른다 (§7.3 · 타임아웃 · 읽지 못한 응답).
        None => ConfirmBecause::OutcomeUnknown,
    }
}

/// 확인을 받았으면 새 페이지, 아니면 무엇을 확인해야 하는지를 알린다.
fn new_page_after(confirmation: Confirmation, because: ConfirmBecause) -> Result<Plan, Failure> {
    match confirmation {
        Confirmation::NewPage => Ok(Plan::CreatePage),
        Confirmation::NotAsked => Err(needs_confirmation(because)),
    }
}

/// 새 페이지를 만들기 전에 사용자가 알아야 하는 사실 세 가지 (§8.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfirmBecause {
    /// 이미 이 Recording의 Notion 페이지가 있다.
    AlreadySent,
    /// 마지막으로 보낸 뒤 문서가 바뀌었다.
    DocumentChanged,
    /// 지난번에 페이지가 만들어졌는지 알 수 없다.
    OutcomeUnknown,
}

impl ConfirmBecause {
    /// 사용자가 읽을 문장.
    fn message(self) -> &'static str {
        match self {
            Self::AlreadySent => {
                "이 녹음은 이미 Notion 페이지가 있다. 계속하면 기존 페이지는 그대로 두고 새 페이지를 만든다."
            }
            Self::DocumentChanged => {
                "마지막으로 보낸 뒤 내용이 바뀌었다. 이어 붙이지 않으며, 계속하면 새 페이지를 만든다."
            }
            Self::OutcomeUnknown => {
                "지난번 전송에서 Notion 페이지가 만들어졌는지 확인하지 못했다. Notion을 확인한 뒤 계속하면 새 페이지를 만든다."
            }
        }
    }

    /// 화면이 어느 안내를 띄울지 고를 수 있는 안정적인 값.
    fn as_str(self) -> &'static str {
        match self {
            Self::AlreadySent => "alreadySent",
            Self::DocumentChanged => "documentChanged",
            Self::OutcomeUnknown => "outcomeUnknown",
        }
    }
}

/// 지금까지 어디까지 갔는가. **저장되는 값 그대로다.**
struct Progress {
    page: Option<PageId>,
    sent: i64,
    total: i64,
    fingerprint: String,
}

/// 실제로 보내는 구간. 여기서 나오는 모든 실패는 호출자가 `failed`로 기록한다.
///
/// 순서가 규칙이다 — **성공한 요청 하나마다 곧바로 적고 나서** 다음 요청을 보낸다.
fn transmit(
    connection: &Connection,
    recording: &Recording,
    client: &NotionClient,
    destination: &Destination<'_>,
    waiter: &dyn Waiter,
    chunks: &[&str],
    progress: &mut Progress,
) -> Result<(), Failure> {
    let mut pacer = Pacer::new(waiter);

    if progress.page.is_none() {
        // 첫 chunk가 페이지가 된다. 제목은 그 안의 첫 `# h1`이다 (§5.3).
        let page = pacer.send(|| {
            client.create_page(
                destination.token,
                destination.parent_page_id,
                chunks[0],
            )
        })?;

        // ★ §8.4-3 — **다음 요청을 보내기 전에** 디스크에 있어야 한다. 여기서 잃으면 그
        // 페이지는 앱이 다시 찾을 수 없는 고아가 되고, 재시도는 새 페이지를 만든다.
        progress.page = Some(page);
        progress.sent = 1;
        persist(connection, recording, progress, ProcessingStatus::Running, None, None)?;
    }

    let page = progress
        .page
        .clone()
        .expect("페이지를 만들었거나 이어 보내는 중이다");

    // 나머지를 **순서대로** 이어 붙인다. 건너뛰지 않고, 앞질러 세지 않는다.
    while progress.sent < progress.total {
        let index = usize::try_from(progress.sent).map_err(|_| impossible_progress(progress))?;
        let chunk = chunks.get(index).ok_or_else(|| impossible_progress(progress))?;

        pacer.send(|| client.append_markdown(destination.token, &page, chunk))?;

        // §8.4-4 — 성공에서만 올린다. 실패한 요청을 센 값으로 재시도하면 그 chunk가 조용히
        // 사라진다.
        progress.sent += 1;
        persist(connection, recording, progress, ProcessingStatus::Running, None, None)?;
    }

    Ok(())
}

/// 요청 하나를 보내는 자리 — **간격과 재시도를 아는 유일한 코드다** (§9.2).
struct Pacer<'a> {
    waiter: &'a dyn Waiter,
    /// 이번 실행에서 이미 요청을 보냈는가. 첫 요청 앞에는 간격을 두지 않는다.
    issued: bool,
}

impl<'a> Pacer<'a> {
    fn new(waiter: &'a dyn Waiter) -> Self {
        Self {
            waiter,
            issued: false,
        }
    }

    /// 요청 하나를 보내고, 속도 제한이면 지시받은 만큼 기다렸다가 **같은 요청**을 다시 보낸다.
    ///
    /// 다시 보내는 것은 속도 제한뿐이다 (§9.3) — 다른 실패는 다시 보내도 같거나, 언제 풀리는지
    /// 알 수 없거나, 결과를 모른다.
    fn send<T>(
        &mut self,
        mut request: impl FnMut() -> Result<T, NotionFailure>,
    ) -> Result<T, Failure> {
        let mut retries = 0;
        // 서버가 지시한 대기. 있으면 그것이 이번 요청 앞의 대기이며, **간격을 따로 더하지
        // 않는다** — 지시받은 대기는 언제나 간격보다 길게 맞춰 두기 때문이다.
        let mut instructed: Option<std::time::Duration> = None;

        loop {
            match instructed.take() {
                Some(duration) => self.waiter.wait(duration),
                // 이번 실행의 첫 요청 앞에는 간격을 두지 않는다 — 기다릴 앞 요청이 없다.
                None if self.issued => self.waiter.wait(MIN_REQUEST_INTERVAL),
                None => {}
            }
            self.issued = true;

            let failure = match request() {
                Ok(value) => return Ok(value),
                Err(failure) => failure,
            };

            // 대기 지시가 없는 실패는 자동으로 다시 보내지 않는다.
            let Some(wait) = failure.wait() else {
                return Err(failure.into_failure());
            };

            if retries >= MAX_RETRIES_PER_CHUNK {
                return Err(gave_up_waiting(failure));
            }

            match pace::pause(wait, retries) {
                // 서버가 간격보다 짧은 값을 말하더라도 요청 사이의 최소 간격은 지킨다.
                Pause::For(duration) => instructed = Some(duration.max(MIN_REQUEST_INTERVAL)),
                // 몇 분씩 조용히 멈춰 있지 않는다 (§9.2-4). 대신 언제 다시 오면 되는지 말한다.
                Pause::TooLong(asked) => return Err(asked_to_wait_too_long(failure, asked)),
            }

            retries += 1;
        }
    }
}

/// 지금 진행도를 [`NotionSync`]로 저장하고, `recordings.notion_status`를 함께 옮긴다.
///
/// 두 쓰기가 이 모듈이 저장소에 손대는 전부다. Transcript도 AI Note도 오디오도 여기서 바뀔 수
/// 없다 (INV-3) — 부를 수 있는 쓰기 함수가 이 둘뿐이기 때문이다.
fn persist(
    connection: &Connection,
    recording: &Recording,
    progress: &Progress,
    status: ProcessingStatus,
    synced_at: Option<String>,
    error: Option<String>,
) -> Result<NotionSync, Failure> {
    let sync = NotionSync {
        recording_id: recording.id.clone(),
        page_id: progress.page.as_ref().map(|page| page.as_str().to_owned()),
        synced_at,
        status,
        error,
        sent_chunks: Some(progress.sent),
        total_chunks: Some(progress.total),
        content_fingerprint: Some(progress.fingerprint.clone()),
    };

    store::save_notion_sync(connection, &sync)?;

    // 다른 후처리 상태는 **읽은 그대로 다시 쓴다** — Notion이 남의 파이프라인 상태를 옮기지
    // 않는다 (`crate::ai::run`과 같은 규칙).
    let now = store::now(connection)?;
    store::update_recording_statuses(
        connection,
        &recording.id,
        recording.transcription_status,
        recording.ai_status,
        status,
        &now,
    )?;

    Ok(sync)
}

/// 실패를 `failed`로 남기고 그 실패를 그대로 돌려준다.
///
/// **부분 전송이 상태에서 드러난다** — `sent_chunks`와 `total_chunks`가 남으므로 화면은 "3개 중
/// 2개까지 갔다"를 말할 수 있고, 다음 시도는 그 자리에서 이어 간다 (§8.2).
///
/// 원래 원인을 다른 것으로 바꾸지 않는다. 상태를 남기는 데까지 실패하면 그 사실을 detail에
/// 덧붙일 뿐이다 — 사용자가 읽을 문장은 여전히 전송이 왜 실패했는지다 (§13).
fn record_failure(
    connection: &Connection,
    recording: &Recording,
    progress: &Progress,
    failure: Failure,
) -> Failure {
    let stored = persist(
        connection,
        recording,
        progress,
        ProcessingStatus::Failed,
        None,
        Some(failure.message.clone()),
    );

    match stored {
        Ok(_) => failure,
        Err(storage) => {
            let detail = match failure.detail.as_deref() {
                Some(existing) => format!("{existing} · 실패 상태도 저장하지 못했다: {storage}"),
                None => format!("실패 상태도 저장하지 못했다: {storage}"),
            };
            failure.with_detail(detail)
        }
    }
}

// --- 실패 -----------------------------------------------------------------------------
//
// **Notion 실패 다섯 종류를 여기서 만들지 않는다.** 그 다섯은 Notion과 실제로 말하는 adapter가
// 만드는 것이고 (`crate::notion::client` · `NOTION_FAILURE_KINDS`), 이 모듈이 만드는 것은
// 요청을 보내기도 전에 갈리는 상황들이다. 그래서 새 `FailureKind`를 더하지 않는다 —
// 만들지 않은 실패의 자리를 미리 만들지 않는다는 규칙 그대로다 (§20.6 · `domain::failure`).

/// 그런 Recording이 없다. 상태를 쓸 대상 자체가 없으므로 아무것도 저장하지 않았다.
fn unknown_recording(id: &RecordingId) -> Failure {
    Failure::permanent(FailureKind::InvalidInput, "보낼 녹음을 찾을 수 없다.")
        .with_detail(format!("recordingId={id}"))
}

/// 아직 current Transcript가 없다 (§7.2).
///
/// **AI Note가 없는 것과는 다른 상황이다.** 노트는 선택 입력이지만 (INV-8) Transcript는 문서의
/// 본문이며, 그것이 없으면 제목과 길이만 담긴 페이지가 사용자의 워크스페이스에 남는다 —
/// 지우는 경로는 이 앱에 없다 (§8.3). 전사를 먼저 돌리면 풀리므로 재시도 가능한 실패다.
fn nothing_to_send(id: &RecordingId) -> Failure {
    Failure::retryable(
        FailureKind::InvalidInput,
        "아직 전사가 없어 보낼 내용이 없다. 전사를 먼저 끝내야 한다.",
    )
    .with_detail(format!("recordingId={id}"))
}

/// current가 가리키는 Transcript 행이 없다.
///
/// 스키마의 복합 FK가 막는 상태이므로 정상적으로는 일어나지 않는다. 일어났다면 저장소가
/// 어긋난 것이다 — **추측해서 다른 Transcript를 고르지 않는다** ([`crate::export::run`]과 같다).
fn dangling_transcript(id: &TranscriptId) -> Failure {
    Failure::permanent(FailureKind::Storage, "보낼 전사를 저장소에서 읽지 못했다.")
        .with_detail(format!("transcriptId={id}"))
}

/// 나눌 수 없는 단위가 있어 문서를 조각으로 만들지 못했다 (ADR-0009 §6.3-4).
///
/// **자르지 않는다.** 일부만 보내고 나머지를 버리는 것은 조용한 유실이며, 그것을 성공으로
/// 부르지 않는다. 어느 줄이 얼마나 큰지가 detail에 남으므로 사용자가 그 자리를 찾아갈 수 있다.
fn cannot_split(oversized: crate::notion::OversizedAtom) -> Failure {
    Failure::permanent(
        FailureKind::InvalidInput,
        "한 번에 보낼 수 있는 크기를 넘는 부분이 있어 보내지 않았다.",
    )
    .with_detail(oversized)
}

/// 이미 이 Recording을 보내는 중이다 (§8.5).
///
/// 조용히 무시하면 사용자는 두 번째 요청이 접수됐는지 알 수 없고, 두 번 굴리면 같은 페이지에
/// 같은 문단이 두 번 들어간다.
fn already_sending(sync: &NotionSync) -> Failure {
    let progress = match (sync.sent_chunks, sync.total_chunks) {
        (Some(sent), Some(total)) => format!(" · sentChunks={sent}/{total}"),
        _ => String::new(),
    };

    Failure::retryable(
        FailureKind::InvalidInput,
        "이미 이 녹음을 Notion으로 보내고 있다.",
    )
    .with_detail(format!("status={}{progress}", sync.status.as_str()))
}

/// 이어 보낼 수 없어 **새 페이지가 필요하다**. 그 사실을 확인받기 전에는 만들지 않는다 (§8.5).
fn needs_confirmation(because: ConfirmBecause) -> Failure {
    Failure::retryable(FailureKind::InvalidInput, because.message())
        .with_detail(format!("needsConfirmation={}", because.as_str()))
}

/// 속도 제한이 [`MAX_RETRIES_PER_CHUNK`]번의 자동 재시도 뒤에도 풀리지 않았다 (§9.2-3).
///
/// 원래 실패의 종류와 원인을 그대로 두고 **몇 번 시도했는지만 덧붙인다** — 사용자가 다시
/// 누르면 그 시도는 '이어 보내기'이므로 중복이 생기지 않는다.
fn gave_up_waiting(failure: NotionFailure) -> Failure {
    let failure = failure.into_failure();
    let detail = match failure.detail.as_deref() {
        Some(existing) => format!("{existing} · retries={MAX_RETRIES_PER_CHUNK}"),
        None => format!("retries={MAX_RETRIES_PER_CHUNK}"),
    };

    failure.with_detail(detail)
}

/// 서버가 [`pace::MAX_WAIT`]보다 긴 대기를 지시했다 (§9.2-4).
///
/// **기다리지 않는다.** 대신 얼마 뒤에 다시 시도하면 되는지를 사용자가 읽을 문장에 적는다 —
/// 앱이 몇 분씩 조용히 멈춰 있는 것은 "멈춘 것"과 구분되지 않는다.
fn asked_to_wait_too_long(failure: NotionFailure, asked: std::time::Duration) -> Failure {
    let failure = failure.into_failure();
    let seconds = asked.as_secs();

    Failure {
        message: format!(
            "Notion이 요청 속도를 제한하고 있다. 약 {seconds}초 뒤에 다시 시도할 수 있다."
        ),
        ..failure
    }
}

/// 저장된 진행도로 조각을 가리킬 수 없다. 정상적으로는 일어나지 않는다.
fn impossible_progress(progress: &Progress) -> Failure {
    Failure::permanent(
        FailureKind::Storage,
        "전송 진행도를 읽지 못해 이어서 보내지 않았다.",
    )
    .with_detail(format!(
        "sentChunks={}/{}",
        progress.sent, progress.total
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FINGERPRINT: &str = "0123456789abcdef";
    const PAGE: &str = "page-identifier";

    fn sync(status: ProcessingStatus) -> NotionSync {
        NotionSync {
            recording_id: RecordingId::new("rec-1"),
            page_id: None,
            synced_at: None,
            status,
            error: None,
            sent_chunks: None,
            total_chunks: None,
            content_fingerprint: None,
        }
    }

    fn partially_sent(fingerprint: &str, sent: i64) -> NotionSync {
        NotionSync {
            page_id: Some(PAGE.to_owned()),
            error: Some("보내는 중에 실패했다".to_owned()),
            sent_chunks: Some(sent),
            total_chunks: Some(4),
            content_fingerprint: Some(fingerprint.to_owned()),
            ..sync(ProcessingStatus::Failed)
        }
    }

    #[test]
    fn the_same_document_always_gets_the_same_fingerprint_and_a_different_one_never_does() {
        let document = "# 제목\n\nDate: 2026-09-01\n";

        assert_eq!(content_fingerprint(document), content_fingerprint(document));
        assert_ne!(
            content_fingerprint(document),
            content_fingerprint("# 제목\n\nDate: 2026-09-02\n"),
            "한 글자가 달라도 다른 문서다"
        );
        assert_eq!(
            content_fingerprint(document).len(),
            64,
            "sha256 hex는 64글자다"
        );
        assert!(
            content_fingerprint(document)
                .chars()
                .all(|character| character.is_ascii_hexdigit()),
            "저장되는 값은 hex 문자뿐이다"
        );
    }

    #[test]
    fn a_recording_that_was_never_sent_gets_a_new_page() {
        assert_eq!(
            plan(None, FINGERPRINT, Confirmation::NotAsked).expect("보낸 적이 없다"),
            Plan::CreatePage
        );
        assert_eq!(
            plan(
                Some(&sync(ProcessingStatus::None)),
                FINGERPRINT,
                Confirmation::NotAsked
            )
            .expect("보낸 적이 없다"),
            Plan::CreatePage
        );
    }

    #[test]
    fn a_partial_send_of_the_same_document_continues_on_the_same_page() {
        // §8.2: 재시도가 새 페이지를 만들지 않는다. 확인 대화도 필요 없다 — 중복이 생기지
        // 않기 때문이다.
        let plan = plan(
            Some(&partially_sent(FINGERPRINT, 2)),
            FINGERPRINT,
            Confirmation::NotAsked,
        )
        .expect("이어 보낼 수 있다");

        let Plan::Resume { page, sent } = plan else {
            panic!("이어 보내야 한다: {plan:?}");
        };
        assert_eq!(page.as_str(), PAGE);
        assert_eq!(sent, 2, "성공한 것 다음부터 보낸다");
    }

    #[test]
    fn a_document_that_changed_since_the_partial_send_is_never_appended_to_that_page() {
        // 이어 붙이면 서로 다른 두 문서가 페이지 하나에 섞인다 (§8.2).
        let changed = partially_sent("전혀-다른-지문", 2);

        let failure = plan(Some(&changed), FINGERPRINT, Confirmation::NotAsked)
            .expect_err("이어 붙일 수 없다");
        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.source_data_safe, "아무것도 보내지 않았다");
        assert_eq!(
            failure.detail.as_deref(),
            Some("needsConfirmation=documentChanged")
        );

        assert_eq!(
            plan(Some(&changed), FINGERPRINT, Confirmation::NewPage)
                .expect("확인을 받았으면 새 페이지다"),
            Plan::CreatePage
        );
    }

    #[test]
    fn a_failure_that_never_learned_the_page_asks_before_making_another_one() {
        // §8.5의 '결과를 모름' — 페이지가 만들어졌을 수 있다. 조용히 다시 만들지 않는다.
        let unknown = NotionSync {
            sent_chunks: Some(0),
            total_chunks: Some(3),
            content_fingerprint: Some(FINGERPRINT.to_owned()),
            ..sync(ProcessingStatus::Failed)
        };

        let failure =
            plan(Some(&unknown), FINGERPRINT, Confirmation::NotAsked).expect_err("확인이 필요하다");
        assert_eq!(
            failure.detail.as_deref(),
            Some("needsConfirmation=outcomeUnknown")
        );
        assert!(failure.retryable, "확인하면 진행할 수 있다");
    }

    #[test]
    fn sending_an_already_finished_recording_again_is_an_explicit_new_page() {
        // §8.3: 기존 페이지는 덮어쓰지도 지우지도 않는다. 사용자가 알고 누른 결과여야 한다.
        let done = NotionSync {
            page_id: Some(PAGE.to_owned()),
            synced_at: Some("2026-09-04T00:00:00.000Z".to_owned()),
            sent_chunks: Some(3),
            total_chunks: Some(3),
            content_fingerprint: Some(FINGERPRINT.to_owned()),
            ..sync(ProcessingStatus::Done)
        };

        let failure =
            plan(Some(&done), FINGERPRINT, Confirmation::NotAsked).expect_err("확인이 필요하다");
        assert_eq!(
            failure.detail.as_deref(),
            Some("needsConfirmation=alreadySent")
        );
        assert!(
            !failure.message.trim().is_empty(),
            "무엇을 확인해야 하는지 화면에 띄울 문장이 있다"
        );

        assert_eq!(
            plan(Some(&done), FINGERPRINT, Confirmation::NewPage).expect("확인을 받았다"),
            Plan::CreatePage,
            "같은 문서여도 이미 끝난 것은 이어 붙이지 않는다"
        );
    }

    #[test]
    fn a_send_that_is_still_running_is_never_started_a_second_time() {
        for status in [ProcessingStatus::Pending, ProcessingStatus::Running] {
            let running = NotionSync {
                page_id: Some(PAGE.to_owned()),
                sent_chunks: Some(1),
                total_chunks: Some(3),
                content_fingerprint: Some(FINGERPRINT.to_owned()),
                ..sync(status)
            };

            // 확인을 받았더라도 진행 중인 것을 두 번 굴리지 않는다.
            for confirmation in [Confirmation::NotAsked, Confirmation::NewPage] {
                let failure = plan(Some(&running), FINGERPRINT, confirmation)
                    .expect_err("이미 보내는 중이다");

                assert_eq!(failure.kind, FailureKind::InvalidInput);
                assert_eq!(
                    failure.detail.as_deref(),
                    Some(format!("status={} · sentChunks=1/3", status.as_str()).as_str())
                );
            }
        }
    }

    #[test]
    fn a_page_we_cannot_read_back_is_not_resumed_onto() {
        // 저장된 값이 이 앱이 넣은 모양이 아니면 그 자리로 요청을 보내지 않는다.
        let broken = NotionSync {
            page_id: Some("주소에 실을 수 없는 값".to_owned()),
            ..partially_sent(FINGERPRINT, 2)
        };

        let failure =
            plan(Some(&broken), FINGERPRINT, Confirmation::NotAsked).expect_err("이어갈 수 없다");
        assert_eq!(
            failure.detail.as_deref(),
            Some("needsConfirmation=documentChanged"),
            "페이지 값이 있으므로 사용자가 Notion에서 확인할 대상이 있다"
        );
    }

    #[test]
    fn a_progress_that_does_not_include_the_first_chunk_is_not_trusted() {
        // 페이지가 있다는 것은 첫 chunk가 반영됐다는 뜻이다. 0이면 저장소가 어긋난 것이며,
        // 그 값으로 이어 붙이면 첫 chunk를 다시 보낸다.
        let inconsistent = partially_sent(FINGERPRINT, 0);

        assert!(resumable(&inconsistent, FINGERPRINT).is_none());
    }

    #[test]
    fn a_stored_progress_never_points_past_the_document_it_is_resuming() {
        let plan = Plan::Resume {
            page: PageId::parse(PAGE).expect("모양이 맞다"),
            sent: 9,
        };

        assert_eq!(plan.sent(3), 3, "조각 수를 넘는 값으로 인덱싱하지 않는다");
        assert_eq!(Plan::CreatePage.sent(3), 0);
    }

    #[test]
    fn nothing_to_send_is_told_apart_from_a_recording_that_is_not_there() {
        // 사용자가 할 수 있는 일이 다르다 — 이쪽은 전사를 끝내는 것이고, 저쪽은 다른 녹음을
        // 고르는 것이다 (§13).
        let missing = unknown_recording(&RecordingId::new("rec-1"));
        assert_eq!(missing.kind, FailureKind::InvalidInput);
        assert!(!missing.retryable, "같은 id로 다시 해도 결과가 같다");
        assert!(missing.source_data_safe, "아무것도 건드리지 않았다 (INV-3)");

        let empty = nothing_to_send(&RecordingId::new("rec-1"));
        assert!(empty.retryable, "전사가 끝나면 보낼 수 있다");
        assert!(empty.source_data_safe);
    }

    #[test]
    fn a_wait_longer_than_this_app_will_sit_through_says_when_to_come_back() {
        use crate::notion::RetryAfter;
        use std::time::Duration;

        // adapter가 만든 실패 하나를 그대로 쓴다 — 종류와 원인은 바뀌지 않고 문장만 바뀐다.
        let rate_limited = NotionFailureFixture::rate_limited();
        let failure = asked_to_wait_too_long(rate_limited, Duration::from_secs(600));

        assert_eq!(failure.kind, FailureKind::NotionRateLimited);
        assert!(failure.retryable);
        assert!(failure.message.contains("600"), "언제 다시 오면 되는지: {}", failure.message);
        assert_eq!(
            RetryAfter::Seconds(600).seconds(),
            Some(600),
            "이 값의 출처는 서버가 말한 초다"
        );
    }

    #[test]
    fn giving_up_after_the_retry_limit_keeps_the_original_cause() {
        let failure = gave_up_waiting(NotionFailureFixture::rate_limited());

        assert_eq!(failure.kind, FailureKind::NotionRateLimited);
        assert!(
            failure
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("retries=3")),
            "몇 번 시도했는지가 남는다: {:?}",
            failure.detail
        );
    }

    /// adapter가 실제로 만드는 속도 제한 실패 하나. **여기서 손으로 만들지 않는다** —
    /// `NotionFailure`를 만드는 자리는 adapter 하나이고, 그 규칙을 이 테스트가 깨지 않는다.
    struct NotionFailureFixture;

    impl NotionFailureFixture {
        fn rate_limited() -> NotionFailure {
            use crate::notion::testing::{StubReply, StubServer};
            use std::sync::Arc;

            let server = StubServer::ready()
                .with_create_page(StubReply::rate_limited(429, Some("30")));
            let client = NotionClient::new(Arc::new(server));

            client
                .create_page(&Secret::new("not-a-real-credential"), "parent", "# 제목")
                .expect_err("속도 제한이다")
        }
    }
}
