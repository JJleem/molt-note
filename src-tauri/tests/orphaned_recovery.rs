//! **확정된 녹음 파일이 레코드 없이 남아도 사라지지 않는다** (2026-09-15).
//!
//! ## 무엇이 있었나
//!
//! 정지는 두 걸음이다 — 파일을 확정하고, 레코드를 저장한다. 2026-09-15에 그 사이에서
//! 무언가 어긋나, **15.7분짜리 회의 녹음이 디스크에 온전히 있는데 목록에 없었다.**
//! 사용자에게는 녹음이 사라진 것으로 보였고, 사람이 DB를 직접 고쳐 되살려야 했다.
//!
//! 앱에는 그것을 알아챌 방법도, 되살릴 방법도 없었다. 이 파일이 그 구멍을 지킨다.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::commands::Storage;
use molt_note_lib::platform::app_data_dir::AppDataDirectory;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempRoot(PathBuf);

impl TempRoot {
    fn new(name: &str) -> Self {
        let index = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("molt-orphan-recover-{name}-{}-{index}", std::process::id()));
        std::fs::create_dir_all(&path).expect("만들 수 있어야 한다");
        Self(path)
    }

    fn app_data_dir(&self) -> AppDataDirectory {
        AppDataDirectory::new(self.0.clone())
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// **확정이 끝난** 녹음 파일 하나 — 헤더 크기가 채워져 있다.
fn finalized_recording(directory: &Path, name: &str, seconds: u32) -> PathBuf {
    std::fs::create_dir_all(directory).expect("만들 수 있어야 한다");
    let path = directory.join(name);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).expect("만들 수 있어야 한다");
    for frame in 0..(seconds as usize * 48_000) {
        writer
            .write_sample(((frame % 1_000) as i16) - 500)
            .expect("쓸 수 있어야 한다");
    }
    // **finalize가 크기를 채운다** — 정지가 끝난 파일과 같은 상태다.
    writer.finalize().expect("확정할 수 있어야 한다");
    path
}

#[test]
fn a_finalized_recording_with_no_record_comes_back_when_the_app_starts() {
    // 지키려는 것: **녹음이 조용히 사라지지 않는다.** 2026-09-15에 실제로 일어난 일이다.
    let temp = TempRoot::new("lost");
    let app_data_dir = temp.app_data_dir();

    // 앱이 한 번 열려 디렉터리가 준비된다.
    let _first = Storage::open(&app_data_dir);
    let lost = finalized_recording(&app_data_dir.recordings_dir(), "capture-1789437702.wav", 3);

    // 다시 연다 — 사용자가 하는 일 전부다.
    let storage = Storage::open(&app_data_dir);
    let listed = storage.list_recordings().expect("읽을 수 있어야 한다");

    assert_eq!(listed.len(), 1, "되살아나야 한다");
    assert_eq!(listed[0].audio_path, lost.to_str().expect("경로"));
    assert!(
        listed[0].duration_ms >= 2_900 && listed[0].duration_ms <= 3_100,
        "길이를 파일에서 읽어야 한다: {}",
        listed[0].duration_ms,
    );
    assert!(!listed[0].title.trim().is_empty(), "이름이 있어야 한다");
}

#[test]
fn it_does_not_bring_the_same_recording_back_twice() {
    // 되살리기가 되풀이되면 같은 녹음이 목록에 여럿이 된다.
    let temp = TempRoot::new("twice");
    let app_data_dir = temp.app_data_dir();

    let _first = Storage::open(&app_data_dir);
    finalized_recording(&app_data_dir.recordings_dir(), "capture-1.wav", 2);

    for _ in 0..3 {
        let storage = Storage::open(&app_data_dir);
        assert_eq!(
            storage.list_recordings().expect("읽기").len(),
            1,
            "몇 번을 열어도 하나여야 한다",
        );
    }
}

#[test]
fn a_recording_that_is_already_listed_is_left_alone() {
    let temp = TempRoot::new("known");
    let app_data_dir = temp.app_data_dir();

    let _first = Storage::open(&app_data_dir);
    finalized_recording(&app_data_dir.recordings_dir(), "capture-1.wav", 2);

    let storage = Storage::open(&app_data_dir);
    let before = storage.list_recordings().expect("읽기");

    // 사용자가 제목을 고쳤다고 하자.
    storage
        .rename_recording(&before[0].id, "내가 붙인 이름")
        .expect("고칠 수 있어야 한다");

    let reopened = Storage::open(&app_data_dir);
    let after = reopened.list_recordings().expect("읽기");

    assert_eq!(after.len(), 1);
    assert_eq!(after[0].title, "내가 붙인 이름", "되살리기가 덮어쓰면 안 된다");
}

#[test]
fn a_file_with_only_a_header_is_not_brought_back() {
    // 소리가 없는 파일을 되살리면 **비어 있는 녹음**이 목록에 생기고, 사용자는 그것이
    // 무엇인지 알 수 없다.
    let temp = TempRoot::new("empty");
    let app_data_dir = temp.app_data_dir();

    let _first = Storage::open(&app_data_dir);
    let directory = app_data_dir.recordings_dir();
    std::fs::create_dir_all(&directory).expect("만들 수 있어야 한다");
    let mut file = std::fs::File::create(directory.join("capture-empty.wav")).expect("만들기");
    file.write_all(&[0_u8; 44]).expect("쓸 수 있어야 한다");
    file.flush().expect("내보내기");

    let storage = Storage::open(&app_data_dir);
    assert!(storage.list_recordings().expect("읽기").is_empty());
}

#[test]
fn the_audio_file_is_never_touched() {
    // INV-1 — 되살리기는 읽기만 한다.
    let temp = TempRoot::new("untouched");
    let app_data_dir = temp.app_data_dir();

    let _first = Storage::open(&app_data_dir);
    let path = finalized_recording(&app_data_dir.recordings_dir(), "capture-1.wav", 2);
    let before = std::fs::read(&path).expect("읽기");

    let _storage = Storage::open(&app_data_dir);

    assert_eq!(std::fs::read(&path).expect("읽기"), before, "파일이 바뀌면 안 된다");
}
