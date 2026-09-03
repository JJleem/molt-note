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
        "SELECT recordings_directory, automatic_processing, default_microphone
         FROM settings WHERE id = ?1",
        [SETTINGS_ROW_ID],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        },
    );

    match row {
        Ok((recordings_directory, automatic_processing, default_microphone)) => Ok(Settings {
            recordings_directory,
            automatic_processing: decode_toggle(automatic_processing)?,
            // 저장된 키가 지금 목록에 있는지는 여기서 묻지 않는다. 저장소는 장치를 알지
            // 않으며, 없어진 장치를 읽는 김에 지우거나 다른 값으로 바꾸지도 않는다.
            default_microphone,
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
            "INSERT INTO settings (id, recordings_directory, automatic_processing, default_microphone)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (id) DO UPDATE
             SET recordings_directory = excluded.recordings_directory,
                 automatic_processing = excluded.automatic_processing,
                 default_microphone = excluded.default_microphone",
            rusqlite::params![
                SETTINGS_ROW_ID,
                settings.recordings_directory,
                i64::from(settings.automatic_processing),
                settings.default_microphone,
            ],
        )
        .map_err(DatabaseError::Sql)?;
    Ok(())
}

/// 저장된 0/1을 토글 값으로 옮긴다. 그 밖의 값은 추측하지 않고 실패한다.
fn decode_toggle(value: i64) -> Result<bool, DatabaseError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(DatabaseError::Decode {
            table: "settings",
            column: "automatic_processing",
            value: other.to_string(),
        }),
    }
}
