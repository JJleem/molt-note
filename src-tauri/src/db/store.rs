//! §7 domain 레코드의 저장·복원.
//!
//! 이 모듈이 SQL을 아는 유일한 지점이고, `domain`은 SQL을 알지 않는다 (ADR-0001).
//!
//! **Transcript를 갱신하는 API가 없다** — 있는 것은 [`append_transcript`]뿐이다.
//! 재전사는 기존 행을 고치지 않고 새 Transcript를 추가하는 행위이며 (§7.1 · INV-2),
//! 그 규칙을 관례가 아니라 **API 표면**으로 표현한다. 같은 이유로 `INSERT OR REPLACE`도
//! 쓰지 않는다 — 이미 있는 id로 다시 쓰면 조용히 덮어쓰는 대신 실패한다.

use rusqlite::{Connection, Row};

use super::DatabaseError;
use crate::domain::{
    AiNote, AiNoteId, NotionSync, NoteType, ProcessingStatus, Recording, RecordingId, RecordingView,
    Transcript, TranscriptId, TranscriptSegment,
};

/// Recording 조회에 쓰는 열 목록. 목록·단건 조회가 같은 순서를 쓰도록 한 곳에 둔다.
const RECORDING_COLUMNS: &str = "id, title, created_at, updated_at, duration_ms, audio_path,
     audio_format, microphone, current_transcript_id,
     transcription_status, ai_status, notion_status";

/// 새 레코드에 쓸 식별자를 만든다.
///
/// SQLite의 난수원을 그대로 쓴다 — 식별자 하나 때문에 새 의존성을 들이지 않는다.
/// 16바이트를 16진수로 적으므로 같은 값이 두 번 나오는 것은 사실상 일어나지 않고,
/// 값의 형식을 domain이 강제하지 않으므로([`RecordingId::new`]) 나중에 바꿔도 domain은 그대로다.
pub fn new_id(connection: &Connection) -> Result<String, DatabaseError> {
    connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(DatabaseError::Sql)
}

/// 지금 시각을 ISO-8601 UTC 텍스트로 만든다 (`2026-09-01T10:00:00.000Z`).
///
/// 시각의 출처도 한 곳에 둔다. migration 기록이 이미 SQLite의 시계를 쓰고 있고
/// (`datetime('now')`), 날짜 포맷을 위해 새 crate를 들이지 않는다.
/// domain은 시각을 만들지 않는다 — [`Recording::created_at`]의 주석대로 만드는 쪽은 호출자다.
pub fn now(connection: &Connection) -> Result<String, DatabaseError> {
    connection
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
            row.get(0)
        })
        .map_err(DatabaseError::Sql)
}

/// Recording 하나를 새로 저장한다. 이미 같은 id가 있으면 실패한다.
pub fn insert_recording(
    connection: &Connection,
    recording: &Recording,
) -> Result<(), DatabaseError> {
    connection
        .execute(
            "INSERT INTO recordings (
                 id, title, created_at, updated_at, duration_ms, audio_path, audio_format,
                 microphone, current_transcript_id, transcription_status, ai_status, notion_status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                recording.id.as_str(),
                recording.title,
                recording.created_at,
                recording.updated_at,
                recording.duration_ms,
                recording.audio_path,
                recording.audio_format,
                recording.microphone,
                recording.current_transcript_id.as_ref().map(TranscriptId::as_str),
                recording.transcription_status.as_str(),
                recording.ai_status.as_str(),
                recording.notion_status.as_str(),
            ],
        )
        .map_err(DatabaseError::Sql)?;
    Ok(())
}

/// Recording 하나를 읽는다. 없으면 `None`이다.
pub fn load_recording(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Option<Recording>, DatabaseError> {
    let row = connection.query_row(
        &format!("SELECT {RECORDING_COLUMNS} FROM recordings WHERE id = ?1"),
        [id.as_str()],
        read_recording_row,
    );
    match row {
        Ok(raw) => decode_recording(raw).map(Some),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(source) => Err(DatabaseError::Sql(source)),
    }
}

/// Recording 하나를 화면 표시용 형태로 읽는다 (§5 C). 없으면 `None`이다.
///
/// 길이 문자열은 [`RecordingView`]가 domain 규칙으로 만든다 — 저장소가 따로 계산하지 않고,
/// UI도 다시 계산하지 않는다.
pub fn load_recording_view(
    connection: &Connection,
    id: &RecordingId,
) -> Result<Option<RecordingView>, DatabaseError> {
    Ok(load_recording(connection, id)?.map(RecordingView::from))
}

/// 저장된 Recording을 최근 것부터 전부 읽는다 (§5 A의 목록 화면).
///
/// 아직 아무것도 녹음하지 않았으면 빈 목록이다 — 오류가 아니라 정상 상태다.
pub fn list_recordings(connection: &Connection) -> Result<Vec<RecordingView>, DatabaseError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {RECORDING_COLUMNS} FROM recordings ORDER BY created_at DESC, id DESC"
        ))
        .map_err(DatabaseError::Sql)?;
    let rows = statement
        .query_map([], read_recording_row)
        .map_err(DatabaseError::Sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DatabaseError::Sql)?;

    rows.into_iter()
        .map(|raw| decode_recording(raw).map(RecordingView::from))
        .collect()
}

/// Recording 레코드 하나를 지운다. 지웠으면 `true`, 그런 id가 없었으면 `false`다.
///
/// **사용자가 명시적으로 요청했을 때만 부르는 경로다** (INV-4 · R-004). 저장소에는
/// 오래된 녹음을 스스로 정리하는 경로가 없고, 이 함수를 자동으로 부르는 코드도 없다.
///
/// 지우는 것은 **레코드뿐이다.** original audio 파일은 건드리지 않는다 — 파일을 지우는
/// 것은 별개의 명시적 행위이며, 이 계층은 파일시스템에 접근하지 않는다.
///
/// Transcript나 AI Note가 딸린 Recording은 참조 무결성이 막는다. 그때 이 함수는
/// 실패를 그대로 돌려주고 아무것도 지우지 않는다 — 파생 데이터를 조용히 함께 지우거나
/// Transcript를 건드리지 않는다 (INV-2). 딸린 데이터까지 포함하는 삭제 정책은
/// 전사가 실제로 존재하는 Phase에서 결정한다.
pub fn delete_recording(
    connection: &Connection,
    id: &RecordingId,
) -> Result<bool, DatabaseError> {
    let removed = connection
        .execute("DELETE FROM recordings WHERE id = ?1", [id.as_str()])
        .map_err(DatabaseError::Sql)?;
    Ok(removed > 0)
}

/// Recording의 후처리 상태를 갱신한다 (§7). Transcript는 건드리지 않는다.
pub fn update_recording_statuses(
    connection: &Connection,
    id: &RecordingId,
    transcription: ProcessingStatus,
    ai: ProcessingStatus,
    notion: ProcessingStatus,
    updated_at: &str,
) -> Result<(), DatabaseError> {
    connection
        .execute(
            "UPDATE recordings
             SET transcription_status = ?2, ai_status = ?3, notion_status = ?4, updated_at = ?5
             WHERE id = ?1",
            rusqlite::params![
                id.as_str(),
                transcription.as_str(),
                ai.as_str(),
                notion.as_str(),
                updated_at,
            ],
        )
        .map_err(DatabaseError::Sql)?;
    Ok(())
}

/// current Transcript를 지정하거나(§7.2) `None`으로 비운다.
///
/// 가리킬 수 있는 것은 **같은 Recording에 속한** Transcript뿐이다 (스키마의 복합 FK).
/// 실패한 재전사가 이 값을 바꾸지 않는 것은 호출자의 책임이다 — 실패 경로가 이 함수를
/// 부르지 않으면 기존 current가 그대로 유지된다.
pub fn set_current_transcript(
    connection: &Connection,
    id: &RecordingId,
    transcript: Option<&TranscriptId>,
    updated_at: &str,
) -> Result<(), DatabaseError> {
    connection
        .execute(
            "UPDATE recordings SET current_transcript_id = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id.as_str(), transcript.map(TranscriptId::as_str), updated_at],
        )
        .map_err(DatabaseError::Sql)?;
    Ok(())
}

/// Transcript를 **추가**한다. 기존 Transcript는 어떤 경우에도 바뀌지 않는다 (§7.1 · INV-2).
///
/// Transcript 본문과 segment를 한 트랜잭션으로 쓰므로, 중간에 실패하면 이 Transcript는
/// 아예 저장되지 않는다 — segment가 반쯤 붙은 Transcript는 생기지 않는다.
pub fn append_transcript(
    connection: &mut Connection,
    transcript: &Transcript,
) -> Result<(), DatabaseError> {
    let transaction = connection.transaction().map_err(DatabaseError::Sql)?;
    transaction
        .execute(
            "INSERT INTO transcripts (id, recording_id, language, raw_text, created_at, engine, model)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                transcript.id.as_str(),
                transcript.recording_id.as_str(),
                transcript.language,
                transcript.raw_text,
                transcript.created_at,
                transcript.engine,
                transcript.model,
            ],
        )
        .map_err(DatabaseError::Sql)?;

    for (ordinal, segment) in transcript.segments.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO transcript_segments (transcript_id, ordinal, start_ms, end_ms, text)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    transcript.id.as_str(),
                    ordinal as i64,
                    segment.start_ms,
                    segment.end_ms,
                    segment.text,
                ],
            )
            .map_err(DatabaseError::Sql)?;
    }

    transaction.commit().map_err(DatabaseError::Sql)
}

/// Transcript 하나를 segment까지 읽는다. 없으면 `None`이다.
pub fn load_transcript(
    connection: &Connection,
    id: &TranscriptId,
) -> Result<Option<Transcript>, DatabaseError> {
    let row = connection.query_row(
        "SELECT id, recording_id, language, raw_text, created_at, engine, model
         FROM transcripts WHERE id = ?1",
        [id.as_str()],
        read_transcript_row,
    );
    match row {
        Ok(raw) => Ok(Some(with_segments(connection, raw)?)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(source) => Err(DatabaseError::Sql(source)),
    }
}

/// 한 Recording의 Transcript를 오래된 순서로 전부 읽는다 (§7.1의 1:N).
pub fn list_transcripts(
    connection: &Connection,
    recording_id: &RecordingId,
) -> Result<Vec<Transcript>, DatabaseError> {
    let mut statement = connection
        .prepare(
            "SELECT id, recording_id, language, raw_text, created_at, engine, model
             FROM transcripts WHERE recording_id = ?1 ORDER BY created_at, id",
        )
        .map_err(DatabaseError::Sql)?;
    let rows = statement
        .query_map([recording_id.as_str()], read_transcript_row)
        .map_err(DatabaseError::Sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DatabaseError::Sql)?;

    rows.into_iter()
        .map(|raw| with_segments(connection, raw))
        .collect()
}

/// AI Note 하나를 저장한다. Transcript 테이블은 건드리지 않는다 (INV-2).
pub fn insert_ai_note(connection: &Connection, note: &AiNote) -> Result<(), DatabaseError> {
    connection
        .execute(
            "INSERT INTO ai_notes (
                 id, recording_id, transcript_id, note_type, content,
                 provider, model, prompt_version, generated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                note.id.as_str(),
                note.recording_id.as_str(),
                note.transcript_id.as_str(),
                note.note_type.as_str(),
                note.content,
                note.provider,
                note.model,
                note.prompt_version,
                note.generated_at,
            ],
        )
        .map_err(DatabaseError::Sql)?;
    Ok(())
}

/// AI Note 하나를 읽는다. 없으면 `None`이다.
pub fn load_ai_note(
    connection: &Connection,
    id: &AiNoteId,
) -> Result<Option<AiNote>, DatabaseError> {
    let row = connection.query_row(
        "SELECT id, recording_id, transcript_id, note_type, content,
                provider, model, prompt_version, generated_at
         FROM ai_notes WHERE id = ?1",
        [id.as_str()],
        read_ai_note_row,
    );
    match row {
        Ok(raw) => decode_ai_note(raw).map(Some),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(source) => Err(DatabaseError::Sql(source)),
    }
}

/// 특정 Transcript version에서 나온 AI Note만 읽는다 (§7.3의 provenance 구분).
pub fn list_ai_notes_for_transcript(
    connection: &Connection,
    transcript_id: &TranscriptId,
) -> Result<Vec<AiNote>, DatabaseError> {
    let mut statement = connection
        .prepare(
            "SELECT id, recording_id, transcript_id, note_type, content,
                    provider, model, prompt_version, generated_at
             FROM ai_notes WHERE transcript_id = ?1 ORDER BY generated_at, id",
        )
        .map_err(DatabaseError::Sql)?;
    let rows = statement
        .query_map([transcript_id.as_str()], read_ai_note_row)
        .map_err(DatabaseError::Sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DatabaseError::Sql)?;

    rows.into_iter().map(decode_ai_note).collect()
}

/// Recording 하나의 Notion 전송 상태를 기록한다. 같은 Recording의 기록을 대체한다.
pub fn save_notion_sync(connection: &Connection, sync: &NotionSync) -> Result<(), DatabaseError> {
    connection
        .execute(
            "INSERT INTO notion_syncs (recording_id, page_id, synced_at, status, error)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (recording_id) DO UPDATE
             SET page_id = excluded.page_id, synced_at = excluded.synced_at,
                 status = excluded.status, error = excluded.error",
            rusqlite::params![
                sync.recording_id.as_str(),
                sync.page_id,
                sync.synced_at,
                sync.status.as_str(),
                sync.error,
            ],
        )
        .map_err(DatabaseError::Sql)?;
    Ok(())
}

/// Notion 전송 상태를 읽는다. 시도한 적이 없으면 `None`이다.
pub fn load_notion_sync(
    connection: &Connection,
    recording_id: &RecordingId,
) -> Result<Option<NotionSync>, DatabaseError> {
    let row = connection.query_row(
        "SELECT recording_id, page_id, synced_at, status, error
         FROM notion_syncs WHERE recording_id = ?1",
        [recording_id.as_str()],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        },
    );
    match row {
        Ok((recording_id, page_id, synced_at, status, error)) => Ok(Some(NotionSync {
            recording_id: RecordingId::new(recording_id),
            page_id,
            synced_at,
            status: status_or_error(status, "notion_syncs", "status")?,
            error,
        })),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(source) => Err(DatabaseError::Sql(source)),
    }
}

// --- 행 읽기 -----------------------------------------------------------------
//
// 상태·종류 같은 열은 문자열로 먼저 읽고 domain 타입으로는 나중에 옮긴다.
// rusqlite의 row mapper는 `rusqlite::Error`만 돌려줄 수 있어서, 해석 실패를
// 그 안에서 [`DatabaseError::Decode`]로 표현할 수 없기 때문이다.

/// 읽어 온 Recording 행. 상태 열은 아직 문자열이다.
struct RawRecording {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
    duration_ms: i64,
    audio_path: String,
    audio_format: String,
    microphone: Option<String>,
    current_transcript_id: Option<String>,
    transcription_status: String,
    ai_status: String,
    notion_status: String,
}

fn read_recording_row(row: &Row<'_>) -> rusqlite::Result<RawRecording> {
    Ok(RawRecording {
        id: row.get(0)?,
        title: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        duration_ms: row.get(4)?,
        audio_path: row.get(5)?,
        audio_format: row.get(6)?,
        microphone: row.get(7)?,
        current_transcript_id: row.get(8)?,
        transcription_status: row.get(9)?,
        ai_status: row.get(10)?,
        notion_status: row.get(11)?,
    })
}

fn decode_recording(raw: RawRecording) -> Result<Recording, DatabaseError> {
    Ok(Recording {
        id: RecordingId::new(raw.id),
        title: raw.title,
        created_at: raw.created_at,
        updated_at: raw.updated_at,
        duration_ms: raw.duration_ms,
        audio_path: raw.audio_path,
        audio_format: raw.audio_format,
        microphone: raw.microphone,
        // NULL은 "current가 없다"는 정상 상태다 (§7.2).
        current_transcript_id: raw.current_transcript_id.map(TranscriptId::new),
        transcription_status: status_or_error(
            raw.transcription_status,
            "recordings",
            "transcription_status",
        )?,
        ai_status: status_or_error(raw.ai_status, "recordings", "ai_status")?,
        notion_status: status_or_error(raw.notion_status, "recordings", "notion_status")?,
    })
}

/// 읽어 온 Transcript 행. segment는 아직 붙지 않았다.
struct RawTranscript {
    id: String,
    recording_id: String,
    language: Option<String>,
    raw_text: String,
    created_at: String,
    engine: String,
    model: String,
}

fn read_transcript_row(row: &Row<'_>) -> rusqlite::Result<RawTranscript> {
    Ok(RawTranscript {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        language: row.get(2)?,
        raw_text: row.get(3)?,
        created_at: row.get(4)?,
        engine: row.get(5)?,
        model: row.get(6)?,
    })
}

fn with_segments(
    connection: &Connection,
    raw: RawTranscript,
) -> Result<Transcript, DatabaseError> {
    let mut statement = connection
        .prepare(
            "SELECT start_ms, end_ms, text FROM transcript_segments
             WHERE transcript_id = ?1 ORDER BY ordinal",
        )
        .map_err(DatabaseError::Sql)?;
    let segments = statement
        .query_map([raw.id.as_str()], |row| {
            Ok(TranscriptSegment {
                start_ms: row.get(0)?,
                end_ms: row.get(1)?,
                text: row.get(2)?,
            })
        })
        .map_err(DatabaseError::Sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(DatabaseError::Sql)?;

    Ok(Transcript {
        id: TranscriptId::new(raw.id),
        recording_id: RecordingId::new(raw.recording_id),
        language: raw.language,
        segments,
        raw_text: raw.raw_text,
        created_at: raw.created_at,
        engine: raw.engine,
        model: raw.model,
    })
}

/// 읽어 온 AI Note 행. 종류 열은 아직 문자열이다.
struct RawAiNote {
    id: String,
    recording_id: String,
    transcript_id: String,
    note_type: String,
    content: String,
    provider: String,
    model: String,
    prompt_version: String,
    generated_at: String,
}

fn read_ai_note_row(row: &Row<'_>) -> rusqlite::Result<RawAiNote> {
    Ok(RawAiNote {
        id: row.get(0)?,
        recording_id: row.get(1)?,
        transcript_id: row.get(2)?,
        note_type: row.get(3)?,
        content: row.get(4)?,
        provider: row.get(5)?,
        model: row.get(6)?,
        prompt_version: row.get(7)?,
        generated_at: row.get(8)?,
    })
}

fn decode_ai_note(raw: RawAiNote) -> Result<AiNote, DatabaseError> {
    let note_type = NoteType::parse(&raw.note_type).ok_or(DatabaseError::Decode {
        table: "ai_notes",
        column: "note_type",
        value: raw.note_type,
    })?;
    Ok(AiNote {
        id: AiNoteId::new(raw.id),
        recording_id: RecordingId::new(raw.recording_id),
        transcript_id: TranscriptId::new(raw.transcript_id),
        note_type,
        content: raw.content,
        // provider는 알려진 목록과 대조하지 않는다 — domain은 벤더를 알지 않는다 (INV-9).
        provider: raw.provider,
        model: raw.model,
        prompt_version: raw.prompt_version,
        generated_at: raw.generated_at,
    })
}

/// 저장된 상태 문자열을 domain 상태로 옮긴다. 모르는 값은 추측하지 않고 실패한다.
fn status_or_error(
    value: String,
    table: &'static str,
    column: &'static str,
) -> Result<ProcessingStatus, DatabaseError> {
    ProcessingStatus::parse(&value).ok_or(DatabaseError::Decode {
        table,
        column,
        value,
    })
}
