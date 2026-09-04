//! 설정 값의 저장·복원 (PRODUCT-SPEC §5 D).
//!
//! 설정은 **한 행짜리 테이블**이다 — 설정 집합은 하나뿐이고, 행이 여럿 생겨서 어느 것이
//! 진짜인지 물어야 하는 상태를 스키마가 애초에 허용하지 않는다 (`CHECK (id = 1)`).
//!
//! 저장된 행이 없으면 그것은 오류가 아니라 **"아직 아무것도 바꾸지 않았다"**는 정상 상태이며,
//! [`load`]는 [`Settings::DEFAULT`]를 돌려준다. 기본값을 대신 써 넣지 않으므로,
//! 값을 저장한 적이 없는 DB는 계속 값이 없는 상태로 남는다.
//!
//! **INV-7: 이 모듈에는 secret을 저장하는 경로가 없다.** API key · integration token ·
//! password 류 값을 받는 함수도, 그것을 담는 열도 만들지 않는다.

use rusqlite::Connection;

use super::DatabaseError;
use crate::domain::Settings;

/// 설정 행의 고정 id. 스키마의 `CHECK (id = 1)`과 짝이다.
const SETTINGS_ROW_ID: i64 = 1;

/// 저장된 설정을 읽는다. 저장된 적이 없으면 [`Settings::DEFAULT`]를 돌려준다.
pub fn load(connection: &Connection) -> Result<Settings, DatabaseError> {
    let row = connection.query_row(
        "SELECT recordings_directory, automatic_processing, default_microphone,
                automatic_transcription, transcription_model,
                ai_provider, ai_base_url, ai_model, notion_parent_page_id
         FROM settings WHERE id = ?1",
        [SETTINGS_ROW_ID],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        },
    );

    match row {
        Ok((
            recordings_directory,
            automatic_processing,
            default_microphone,
            automatic_transcription,
            transcription_model,
            ai_provider,
            ai_base_url,
            ai_model,
            notion_parent_page_id,
        )) => Ok(Settings {
            recordings_directory,
            automatic_processing: decode_toggle("automatic_processing", automatic_processing)?,
            // 이 열은 version 5에서 더해졌고 NULL을 허용한다. 그 이전에 저장된 행에는 값이
            // 없으며, **그것은 '아직 저장한 적 없음'이지 꺼져 있음이 아니다** — 무엇을 쓸지는
            // 기본값 정책이 답한다 (`crate::db::migrations`의 version 5 주석).
            automatic_transcription: match automatic_transcription {
                Some(value) => decode_toggle("automatic_transcription", value)?,
                None => Settings::DEFAULT.automatic_transcription,
            },
            // 저장된 키가 지금 목록에 있는지는 여기서 묻지 않는다. 저장소는 장치를 알지
            // 않으며, 없어진 장치를 읽는 김에 지우거나 다른 값으로 바꾸지도 않는다.
            default_microphone,
            // 같은 이유로 그 모델 파일이 지금 그 자리에 있는지도 묻지 않는다. 파일을 찾는 일은
            // `crate::transcription::model`의 몫이며, 없다고 해서 저장된 선택이 지워지지 않는다.
            transcription_model,
            // 이 세 열은 version 6에서 더해졌고 NULL을 허용한다. 그 이전에 저장된 행에는 값이
            // 없으며, **NULL은 '아직 고르지 않았다'는 정상 상태다** — 특히 provider를 고르지
            // 않은 것은 오류가 아니다 (INV-8 · ADR-0008 §11.1).
            //
            // 저장소는 셋 중 어느 것도 지금 유효한지 묻지 않는다. 고른 provider의 서버가
            // 응답하는지, 고른 모델이 그 서버에 있는지는 물어본 쪽이 알며, 아니라고 해서
            // 저장된 선택을 여기서 지우거나 바꾸지 않는다.
            ai_provider,
            ai_base_url,
            ai_model,
            // 이 열은 version 7에서 더해졌고 NULL을 허용한다. **NULL은 '아직 고르지 않았다'는
            // 정상 상태다** — 어느 페이지 아래에 만들지 고르지 않은 것은 오류가 아니다
            // (INV-8 · ADR-0009 §8.4). 그 페이지가 지금도 있는지, integration에 공유돼
            // 있는지를 저장소는 묻지 않으며, 아니라고 해서 저장된 선택을 지우지 않는다.
            notion_parent_page_id,
        }),
        // 행이 없는 것은 오류가 아니다 — 기본값 정책이 답을 갖고 있다.
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Settings::DEFAULT),
        Err(source) => Err(DatabaseError::Sql(source)),
    }
}

/// 설정을 저장한다. 이미 저장된 값이 있으면 그 한 행을 갱신한다.
///
/// 행을 지웠다 다시 만들지 않는다 — 갱신 중간에 "설정이 없는 상태"가 생기지 않는다.
pub fn save(connection: &Connection, settings: &Settings) -> Result<(), DatabaseError> {
    connection
        .execute(
            "INSERT INTO settings (id, recordings_directory, automatic_processing,
                                   default_microphone, automatic_transcription,
                                   transcription_model, ai_provider, ai_base_url, ai_model,
                                   notion_parent_page_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT (id) DO UPDATE
             SET recordings_directory = excluded.recordings_directory,
                 automatic_processing = excluded.automatic_processing,
                 default_microphone = excluded.default_microphone,
                 automatic_transcription = excluded.automatic_transcription,
                 transcription_model = excluded.transcription_model,
                 ai_provider = excluded.ai_provider,
                 ai_base_url = excluded.ai_base_url,
                 ai_model = excluded.ai_model,
                 notion_parent_page_id = excluded.notion_parent_page_id",
            rusqlite::params![
                SETTINGS_ROW_ID,
                settings.recordings_directory,
                i64::from(settings.automatic_processing),
                settings.default_microphone,
                // 두 토글은 서로의 값을 보지 않는다 — 각자 자기 열에만 쓰인다.
                i64::from(settings.automatic_transcription),
                settings.transcription_model,
                // 세 값도 각자 자기 열에만 쓰인다. 고르지 않은 값(`None`)은 NULL로 남고,
                // 다른 값에서 채워 넣지 않는다 — 주소를 골랐다고 provider가 골라지지 않는다.
                settings.ai_provider,
                settings.ai_base_url,
                settings.ai_model,
                // Notion destination도 자기 열에만 쓰인다. 고르지 않은 값(`None`)은 NULL로
                // 남고, 다른 값에서 채워 넣지 않는다.
                settings.notion_parent_page_id,
            ],
        )
        .map_err(DatabaseError::Sql)?;
    Ok(())
}

/// 저장된 0/1을 토글 값으로 옮긴다. 그 밖의 값은 추측하지 않고 실패한다.
///
/// 어느 열이었는지를 함께 받는 이유는 실패가 그 사실을 말해야 하기 때문이다 — 두 토글은
/// 서로 다른 값이므로 "설정 어딘가가 이상하다"로 뭉뚱그리지 않는다.
fn decode_toggle(column: &'static str, value: i64) -> Result<bool, DatabaseError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(DatabaseError::Decode {
            table: "settings",
            column,
            value: other.to_string(),
        }),
    }
}
