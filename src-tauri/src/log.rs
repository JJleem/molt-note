//! 앱 데이터 디렉터리의 이벤트 로그 (best-effort).
//!
//! **오디오 · 전사 내용은 절대 로그에 쓰지 않는다** (INV-6).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

const LOG_FILE_NAME: &str = "molt-note-events.log";

/// 시각(ISO-8601 UTC)과 event 문자열을 한 줄로 append한다.
///
/// 파일이 없으면 만든다. **쓰지 못해도 앱을 죽이지 않는다** — 이 함수는 panic하지 않으며
/// 실패를 조용히 삼킨다 (best-effort).
pub fn write_event(app_data_dir: &Path, event: &str) {
    let _ = try_write_event(app_data_dir, event);
}

fn try_write_event(app_data_dir: &Path, event: &str) -> std::io::Result<()> {
    let path = app_data_dir.join(LOG_FILE_NAME);
    let timestamp = iso8601_utc_now();
    let line = format!("{timestamp}\t{event}\n");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())
}

fn iso8601_utc_now() -> String {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // 수동 UTC 변환 — chrono 의존성을 더하지 않는다.
    let days = secs / 86400;
    let day_secs = secs % 86400;
    let h = day_secs / 3600;
    let m = (day_secs % 3600) / 60;
    let s = day_secs % 60;

    // 날짜 계산 (civil date from days since 1970-01-01).
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// days since 1970-01-01 → (year, month, day). Howard Hinnant's algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn write_event_appends_timestamp_and_message() {
        let dir = std::env::temp_dir().join(format!(
            "molt-note-log-test-append-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        write_event(&dir, "test-event-alpha");
        write_event(&dir, "test-event-beta");

        let content = fs::read_to_string(dir.join(LOG_FILE_NAME)).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("test-event-alpha"));
        assert!(lines[1].contains("test-event-beta"));
        // ISO-8601 형식 확인
        assert!(lines[0].contains('T'));
        assert!(lines[0].ends_with("test-event-alpha"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_event_does_not_panic_on_invalid_path() {
        // 존재하지 않는 경로에 써도 panic하지 않는다.
        let dir = Path::new("/nonexistent/path/that/does/not/exist");
        write_event(dir, "should-not-panic");
    }

    #[test]
    fn iso8601_format_is_valid() {
        let ts = iso8601_utc_now();
        // 기본 형식: YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(ts.len(), 20);
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
    }
}
