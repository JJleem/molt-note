//! Recording 하나를 전사해서 Transcript로 남기는 **실행 순서** (PRODUCT-SPEC §7 · §7.1 · §7.2).
//!
//! 앞의 모듈들이 각자 한 조각씩 맡았고, 그것들을 잇는 자리는 여기 하나다.
//!
//! ```text
//! Recording 레코드 ─→ audio_input ─→ engine ─→ parse ─→ append_transcript ─→ set_current
//!  (audio_path)       파생 입력       원시 출력   밀리초       §7.1 · INV-2         §7.2
//!        │
//!        └─ transcription_status:  pending ─→ running ─→ done
//!                                                   └─→ failed  (current는 그대로)
//! ```
//!
//! ## 재전사는 덮어쓰기가 아니라 추가다 (§7.1 · INV-2)
//!
//! 성공할 때마다 **새 Transcript가 하나 늘어난다.** 이 모듈에 기존 Transcript를 고치거나
//! 지우는 경로는 없고, 만들 수도 없다 — 저장소가 내놓는 것이 [`store::append_transcript`]
//! 하나이기 때문이다 (`crate::db::store`의 모듈 문서). 새 것을 current로 올리는 것은
//! 그다음이며 ([`store::set_current_transcript`] · §7.2), 그 순서 덕분에 화면이 `done`을
//! 본 시점에는 current가 이미 새 Transcript를 가리킨다.
//!
//! ## 실패는 아무것도 잃지 않는다 (INV-1 · INV-3 · §13)
//!
//! ```text
//! Transcript A = success / current
//!         ↓
//! 재전사 시도  →  실패
//!         ↓
//! current = Transcript A      그대로
//! Transcript A의 segments      그대로
//! 원본 오디오 파일             그대로
//! Recording 레코드             transcription_status와 updated_at 말고는 그대로
//! ```
//!
//! 실패 경로가 [`store::set_current_transcript`]를 부르지 않기 때문에 current가 유지되고,
//! 이 모듈 어디에도 파일을 쓰거나 지우는 코드가 없기 때문에 원본이 유지된다. 파생 입력은
//! 메모리 위의 버퍼 하나이므로([`audio_input::TranscriptionInput`] · ADR-0007 §9.1) 정리는
//! `drop`이 전부이고, **실패할 수 있는 정리 절차가 없다** — 정리가 전사 성공을 되돌리는
//! 경로 자체가 생기지 않는다.
//!
//! 그래서 같은 Recording에 대해 몇 번이든 다시 시도할 수 있다. 이 모듈은 직전 시도의 결과를
//! 상태로 갖지 않는다.
//!
//! ## 여기서 하지 않는 것
//!
//! 스레드를 만들지 않고, Tauri command를 열지 않으며, 어떤 모델을 쓸지 설정에서 읽지
//! **않는다** — 설정을 읽어 [`ModelChoice`]로 넘기는 것은 부르는 쪽의 일이다.
//! 이 함수를 배경 스레드에서 부르고 그 상태를 화면에 여는 자리는
//! [`crate::commands::Transcriber`]다. 모델 위치를 아는 코드는 여전히 [`model`] 하나뿐이므로
//! (INV-10) 이 모듈은 설정 값을 [`ModelChoice`]로 **받아서 넘기기만** 한다.

use std::path::Path;

use rusqlite::Connection;

use crate::db::store;
use crate::domain::{
    Failure, FailureKind, ProcessingStatus, Recording, RecordingId, Transcript, TranscriptId,
    TranscriptSegment,
};

use super::audio_input;
use super::engine::{output_unusable, TranscriptionEngine};
use super::model;
use super::parse::{self, Anomaly};

/// 어떤 모델로 전사할지 정하는 두 값.
///
/// 경로를 짓지 않는다 — 이 두 값을 실제 파일 하나로 해석하는 것은 [`model::resolve`]이며,
/// 그 자리는 코드 전체에 하나다 (INV-10 · ADR-0007 §8.2).
#[derive(Debug, Clone, Copy)]
pub struct ModelChoice<'a> {
    /// 모델 디렉터리. `crate::platform::app_data_dir::AppDataDirectory::models_dir`에서 온다.
    pub models_dir: &'a Path,
    /// 사용자가 고른 값(파일명 또는 절대 경로). 아직 고르지 않았으면 `None`이다.
    pub configured: Option<&'a str>,
}

/// 성공한 전사 한 건의 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completed {
    /// 방금 **추가된** Transcript. 이 시점에 이미 current다 (§7.2).
    pub transcript: Transcript,
    /// 엔진의 원시 출력이 기대와 달랐던 자리들 ([`parse::Transcription::anomalies`]).
    ///
    /// 저장하지 않고 호출자에게 돌려준다 — 저장할 자리가 없다는 이유로 **버리지는 않는다.**
    pub anomalies: Vec<Anomaly>,
}

/// Recording 하나를 전사하고 결과를 영속화한다.
///
/// 상태는 실제로 세 번(또는 두 번 + 실패) 저장된다 — `pending` · `running` 다음에 `done`
/// 또는 `failed`다. 중간 상태를 건너뛰지 않는 이유는 그것이 화면이 읽는 값이기 때문이다
/// (`phase-prompt/03` 요구 3): 전사가 도는 동안 목록과 상세가 `running`을 볼 수 있어야 한다.
///
/// 다른 후처리 상태(`ai_status` · `notion_status`)는 읽은 그대로 다시 쓴다. 전사가 남의
/// 파이프라인 상태를 옮기지 않는다.
///
/// 실패해도 원본 오디오와 Recording 레코드는 그대로다 (INV-1 · INV-3). 바뀌는 것은
/// `transcription_status`와 `updated_at`뿐이며, 그 둘이 바뀌는 것이 §13이 요구하는
/// "실패가 사용자에게 보인다"의 저장 형태다.
pub fn transcribe(
    connection: &mut Connection,
    recording_id: &RecordingId,
    engine: &dyn TranscriptionEngine,
    model_choice: ModelChoice<'_>,
) -> Result<Completed, Failure> {
    // 상태를 쓸 대상이 실재하는지부터 본다. 없는 Recording에 대해서는 아무것도 쓰지 않는다.
    let recording = store::load_recording(connection, recording_id)?
        .ok_or_else(|| unknown_recording(recording_id))?;

    // 접수(`pending`)와 실행 시작(`running`)을 둘 다 남긴다. 이 두 번의 쓰기가 실패하면
    // 그대로 나간다 — 상태를 남기지 못한 채 전사를 시작하면 화면이 진행 중인 일을 볼 수 없다.
    mark(connection, &recording, ProcessingStatus::Pending)?;
    mark(connection, &recording, ProcessingStatus::Running)?;

    attempt(connection, &recording, engine, model_choice)
        .map_err(|failure| record_failure(connection, &recording, failure))
}

/// 전사 한 번. 여기서 나오는 모든 실패는 호출자가 `failed`로 기록한다.
fn attempt(
    connection: &mut Connection,
    recording: &Recording,
    engine: &dyn TranscriptionEngine,
    model_choice: ModelChoice<'_>,
) -> Result<Completed, Failure> {
    // 모델 없음도 여기서 나온다 — 조용한 skip이 아니라 §13의 실패이며, 그래서 아래의
    // `failed` 기록을 거쳐 사용자에게 도달한다.
    let model = model::resolve(model_choice.models_dir, model_choice.configured)?;

    // 원본은 읽기 전용으로만 열린다 ([`audio_input::load`] · INV-1).
    let input = audio_input::load(Path::new(&recording.audio_path))?;
    let raw = engine.transcribe(&input, &model)?;

    // 파생 입력은 여기서 쓸모를 다한다. 1시간짜리 녹음이면 수백 MB이므로 영속화 전에
    // 놓아 준다. **정리는 이것뿐이다** — 디스크에 자리를 갖지 않는 파생물이라 지울 파일도,
    // 실패할 수 있는 절차도 없다 (ADR-0007 §9.1 · §9.3).
    drop(input);

    let transcription = parse::normalize(raw)?;
    if transcription.segments.is_empty() {
        // 엔진 구현이 출력 계약([`super::engine::ensure_usable`])을 지켰다면 여기 오지 않는다.
        // 그래도 확인하는 것은 빈 Transcript가 한 번 저장되면 immutable하게 남기 때문이다
        // (INV-2) — 저장 직전이 그것을 막을 수 있는 마지막 자리다.
        return Err(output_unusable("전사 결과에 남은 문장이 없다").with_detail(format!(
            "anomalies={}",
            transcription.anomalies.len()
        )));
    }

    let transcript = Transcript {
        id: TranscriptId::new(store::new_id(connection)?),
        recording_id: recording.id.clone(),
        language: transcription.language,
        segments: transcription
            .segments
            .into_iter()
            .map(|segment| TranscriptSegment {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
                text: segment.text,
            })
            .collect(),
        raw_text: transcription.raw_text,
        created_at: store::now(connection)?,
        // provenance는 실제로 쓴 것을 적는다 — 설정 값이 아니라 해석된 모델 파일의 이름이다
        // (§7 · ADR-0007 §8.2.4).
        engine: engine.engine_id(),
        model: model.id().to_owned(),
    };

    // 순서가 규칙이다: 추가 → current로 올리기 → `done`.
    // 화면이 `done`을 보는 시점에는 current가 이미 이 Transcript를 가리킨다.
    store::append_transcript(connection, &transcript)?;
    let now = store::now(connection)?;
    store::set_current_transcript(connection, &recording.id, Some(&transcript.id), &now)?;
    mark(connection, recording, ProcessingStatus::Done)?;

    Ok(Completed {
        transcript,
        anomalies: transcription.anomalies,
    })
}

/// 전사 상태 하나를 저장한다. 다른 후처리 상태는 읽은 값을 그대로 다시 쓴다.
fn mark(
    connection: &Connection,
    recording: &Recording,
    status: ProcessingStatus,
) -> Result<(), Failure> {
    let now = store::now(connection)?;
    store::update_recording_statuses(
        connection,
        &recording.id,
        status,
        recording.ai_status,
        recording.notion_status,
        &now,
    )?;
    Ok(())
}

/// 실패를 `failed`로 남기고 그 실패를 그대로 돌려준다.
///
/// **원래 원인을 다른 것으로 바꾸지 않는다.** 상태를 남기는 데까지 실패하면 그 사실을
/// detail에 덧붙일 뿐이다 — 사용자가 읽을 문장은 여전히 전사가 왜 실패했는지다 (§13).
fn record_failure(
    connection: &Connection,
    recording: &Recording,
    failure: Failure,
) -> Failure {
    match mark(connection, recording, ProcessingStatus::Failed) {
        Ok(()) => failure,
        Err(storage) => {
            let detail = match failure.detail.as_deref() {
                Some(existing) => format!("{existing} · 실패 상태도 저장하지 못했다: {storage}"),
                None => format!("실패 상태도 저장하지 못했다: {storage}"),
            };
            failure.with_detail(detail)
        }
    }
}

/// 그런 Recording이 없다. 상태를 쓸 대상 자체가 없으므로 아무것도 저장하지 않았다.
fn unknown_recording(id: &RecordingId) -> Failure {
    Failure::permanent(FailureKind::InvalidInput, "전사할 녹음을 찾을 수 없다")
        .with_detail(format!("recordingId={id}"))
}
