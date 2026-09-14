//! **아직 쓰이는 중인 WAV를 따라 읽는다** (2026-09-14 · 실시간 전사).
//!
//! ```text
//! 쓰는 쪽   audio::capture 의 WavFile   ← 계속 append 한다. 헤더 크기는 0인 채다
//! 읽는 쪽   여기                        ← data 청크 위치를 찾아 파일 끝까지 읽는다
//! ```
//!
//! ## 왜 `hound`로 읽지 못하는가
//!
//! WAV 헤더의 RIFF·data 크기는 **쓰기가 끝날 때 채워진다.** 녹음 중에는 0이고, `hound`는
//! 그 0을 믿고 "샘플이 없다"고 답한다. 그래서 이 모듈은 크기 필드를 **읽지 않는다** —
//! `data` 청크가 시작하는 자리만 찾고, 내용은 **지금 이 순간의 파일 길이**까지로 본다.
//!
//! ## 읽는 쪽이 물러선다
//!
//! 쓰는 쪽에게 맞춰 달라고 하지 않는다. 원본 오디오는 읽기 전용이며(INV-1) 이 모듈은
//! 파일을 열어 읽기만 한다 — 크기를 고치지도, 잠그지도, 쓰는 쪽을 기다리게 하지도 않는다.
//! 맨 끝의 덜 쓰인 프레임을 피하는 것은 [`super::live::TAIL_MARGIN_SECONDS`]의 몫이다.
//!
//! ## 이 모듈이 하지 않는 것
//!
//! 언제 얼마를 읽을지 정하지 않는다 (`super::live`). 접고 옮기는 규칙도 갖지 않는다
//! (`super::audio_input`). 여기가 아는 것은 **바이트가 어디에 있는가** 하나뿐이다.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::audio_input::{self, TranscriptionInput};
use super::live::Window;
use crate::domain::{Failure, FailureKind};

/// 표준 WAV에서 청크 머리는 **id 4바이트 + 크기 4바이트**다.
const CHUNK_HEADER_BYTES: u64 = 8;

/// `RIFF????WAVE` 다음부터 청크가 시작한다.
const FIRST_CHUNK_OFFSET: u64 = 12;

/// 청크를 찾다가 이만큼을 넘기면 이 파일은 우리가 아는 WAV가 아니다.
///
/// **무한히 뒤지지 않는다** — 손상된 파일이 읽는 쪽을 영원히 붙잡게 두지 않는다.
const MAX_CHUNK_SCAN_BYTES: u64 = 1 << 20;

/// 아직 쓰이는 중인 WAV 하나.
///
/// **한 번 열면 다시 해석하지 않는다.** 형식과 `data` 위치는 녹음 도중에 바뀌지 않으므로,
/// 창을 읽을 때마다 헤더를 다시 뒤질 이유가 없다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrowingWav {
    path: PathBuf,
    /// `data` 내용이 시작하는 바이트 위치.
    data_offset: u64,
    channels: u16,
    sample_rate_hz: u32,
}

impl GrowingWav {
    /// 파일을 열고 형식과 `data` 위치만 읽는다. **내용은 아직 읽지 않는다.**
    pub fn open(path: &Path) -> Result<Self, Failure> {
        let mut file = File::open(path).map_err(|error| {
            unreadable(path, "녹음 파일을 따라 읽을 수 없다").with_detail(error)
        })?;

        let mut riff = [0_u8; 12];
        read_exact(&mut file, &mut riff, path)?;
        if &riff[0..4] != b"RIFF" || &riff[8..12] != b"WAVE" {
            return Err(unusable(path, "녹음 파일이 WAV 형식이 아니다"));
        }

        let mut format: Option<(u16, u32, u16)> = None;
        let mut offset = FIRST_CHUNK_OFFSET;

        loop {
            if offset > MAX_CHUNK_SCAN_BYTES {
                return Err(unusable(path, "녹음 파일에서 소리가 시작하는 자리를 찾지 못했다"));
            }
            file.seek(SeekFrom::Start(offset)).map_err(|error| {
                unreadable(path, "녹음 파일을 따라 읽을 수 없다").with_detail(error)
            })?;

            let mut header = [0_u8; CHUNK_HEADER_BYTES as usize];
            if file.read_exact(&mut header).is_err() {
                return Err(unusable(path, "녹음 파일에서 소리가 시작하는 자리를 찾지 못했다"));
            }
            let id = &header[0..4];
            let declared = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64;

            if id == b"fmt " {
                let mut fmt = [0_u8; 16];
                read_exact(&mut file, &mut fmt, path)?;
                format = Some((
                    u16::from_le_bytes([fmt[2], fmt[3]]),
                    u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]),
                    u16::from_le_bytes([fmt[14], fmt[15]]),
                ));
            } else if id == b"data" {
                let (channels, sample_rate_hz, bits) = format
                    .ok_or_else(|| unusable(path, "녹음 파일에 형식 정보가 없다"))?;
                if bits != audio_input::SOURCE_BITS_PER_SAMPLE {
                    return Err(unusable(
                        path,
                        "이 녹음 파일의 샘플 형식으로는 전사 입력을 만들 수 없다",
                    )
                    .with_detail(format!(
                        "expected={}-bit actual={bits}-bit",
                        audio_input::SOURCE_BITS_PER_SAMPLE
                    )));
                }
                if channels == 0 || sample_rate_hz == 0 {
                    return Err(unusable(path, "녹음 파일의 형식을 읽지 못했다"));
                }
                return Ok(Self {
                    path: path.to_path_buf(),
                    data_offset: offset + CHUNK_HEADER_BYTES,
                    channels,
                    sample_rate_hz,
                });
            }

            // **크기가 0인 `data`는 정상이다** — 아직 쓰는 중이라는 뜻이다. 다른 청크는
            // 크기를 믿고 건너뛴다. 홀수 크기 뒤에는 정렬을 위한 1바이트가 붙는다.
            offset += CHUNK_HEADER_BYTES + declared + (declared & 1);
        }
    }

    pub const fn channels(&self) -> u16 {
        self.channels
    }

    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// 한 프레임이 차지하는 바이트.
    const fn frame_bytes(&self) -> u64 {
        self.channels as u64 * (audio_input::SOURCE_BITS_PER_SAMPLE as u64 / 8)
    }

    /// **지금 이 순간** 파일에 들어 있는 온전한 프레임 수.
    ///
    /// 크기 필드를 믿지 않는다 (모듈 문서). 덜 쓰인 마지막 프레임은 세지 않는다 —
    /// 그래서 이 값은 언제나 실제로 읽을 수 있는 만큼이다.
    pub fn written_frames(&self) -> Result<usize, Failure> {
        let length = std::fs::metadata(&self.path)
            .map_err(|error| {
                unreadable(&self.path, "녹음 파일의 크기를 읽지 못했다").with_detail(error)
            })?
            .len();

        let content = length.saturating_sub(self.data_offset);
        Ok((content / self.frame_bytes()) as usize)
    }

    /// 구간 하나를 전사 입력으로 읽는다 — 16 kHz mono f32.
    ///
    /// 접고 옮기는 규칙은 [`audio_input`]의 것을 그대로 쓴다. **여기서 다시 정하지 않는다.**
    pub fn read(&self, window: Window) -> Result<TranscriptionInput, Failure> {
        let frames = window.len();
        if frames == 0 {
            return Err(unusable(&self.path, "읽을 구간이 비어 있다"));
        }

        let mut file = File::open(&self.path).map_err(|error| {
            unreadable(&self.path, "녹음 파일을 따라 읽을 수 없다").with_detail(error)
        })?;
        let start = self.data_offset + window.start_frame as u64 * self.frame_bytes();
        file.seek(SeekFrom::Start(start)).map_err(|error| {
            unreadable(&self.path, "녹음 파일에서 그 자리를 찾지 못했다").with_detail(error)
        })?;

        let mut bytes = vec![0_u8; frames * self.frame_bytes() as usize];
        read_exact(&mut file, &mut bytes, &self.path)?;

        let samples: Vec<i16> = bytes
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();

        audio_input::from_interleaved(&self.path, &samples, self.channels, self.sample_rate_hz)
    }
}

fn read_exact(file: &mut File, buffer: &mut [u8], path: &Path) -> Result<(), Failure> {
    file.read_exact(buffer).map_err(|error| {
        unreadable(path, "녹음 파일을 그만큼 읽지 못했다").with_detail(error)
    })
}

fn unreadable(path: &Path, message: &str) -> Failure {
    Failure::retryable(FailureKind::Storage, message.to_owned())
        .with_detail(path.display().to_string())
}

fn unusable(path: &Path, message: &str) -> Failure {
    Failure::permanent(FailureKind::InvalidInput, message.to_owned())
        .with_detail(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "molt-growing-{name}-{}-{unique}.wav",
            std::process::id()
        ))
    }

    /// **녹음 중인 파일을 손으로 만든다.**
    ///
    /// `hound`의 writer를 쓰지 않는 이유가 있다 — 그쪽의 `flush`는 헤더 크기를 갱신해서
    /// **끝난 파일과 같은 모양**이 된다. 앱의 `WavFile::write`는 flush를 부르지 않으므로
    /// 녹음 중 파일의 크기 필드는 0인 채다 (2026-09-08 실측). 여기서 재현하려는 것은
    /// 그 상태다.
    fn zero_sized_header(channels: u16, sample_rate: u32) -> Vec<u8> {
        let bits = 16_u16;
        let block_align = channels * bits / 8;
        let byte_rate = sample_rate * u32::from(block_align);

        let mut header = Vec::new();
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&0_u32.to_le_bytes()); // **0이다** — 아직 쓰는 중
        header.extend_from_slice(b"WAVE");
        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16_u32.to_le_bytes());
        header.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        header.extend_from_slice(&channels.to_le_bytes());
        header.extend_from_slice(&sample_rate.to_le_bytes());
        header.extend_from_slice(&byte_rate.to_le_bytes());
        header.extend_from_slice(&block_align.to_le_bytes());
        header.extend_from_slice(&bits.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&0_u32.to_le_bytes()); // **0이다**
        header
    }

    /// 주기가 없는 값 — 어느 구간이든 서로 다르게 나온다.
    fn sample_at(frame: usize, channel: u16) -> i16 {
        let mixed = frame.wrapping_mul(2_654_435_761) ^ usize::from(channel).wrapping_mul(97);
        ((mixed >> 7) & 0x3fff) as i16 - 0x2000
    }

    fn start_recording(path: &Path, channels: u16, rate: u32, frames: usize) -> File {
        let mut file = File::create(path).expect("만들 수 있어야 한다");
        file.write_all(&zero_sized_header(channels, rate))
            .expect("헤더를 쓸 수 있어야 한다");
        append(&mut file, channels, 0, frames);
        file
    }

    fn append(file: &mut File, channels: u16, from: usize, frames: usize) {
        let mut bytes = Vec::with_capacity(frames * usize::from(channels) * 2);
        for frame in from..from + frames {
            for channel in 0..channels {
                bytes.extend_from_slice(&sample_at(frame, channel).to_le_bytes());
            }
        }
        file.write_all(&bytes).expect("쓸 수 있어야 한다");
        file.flush().expect("내보낼 수 있어야 한다");
    }

    #[test]
    fn a_file_whose_header_still_says_zero_reports_what_is_actually_there() {
        // 지키려는 것: **크기 필드를 믿지 않는다.** 이것이 이 모듈이 존재하는 이유 전부다.
        let path = temp_path("growing");
        let _file = start_recording(&path, 1, 16_000, 8_000);

        let wav = GrowingWav::open(&path).expect("열 수 있어야 한다");
        assert_eq!(wav.channels(), 1);
        assert_eq!(wav.sample_rate_hz(), 16_000);
        assert_eq!(wav.written_frames().expect("셀 수 있어야 한다"), 8_000);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn hound_reads_nothing_from_the_same_file() {
        // 이 모듈이 존재하는 이유를 못박는다. 이 검사가 깨지는 날은 `hound`가 크기 없이도
        // 읽게 된 날이며, 그때는 이 모듈을 지울 수 있다.
        let path = temp_path("hound");
        let _file = start_recording(&path, 1, 16_000, 8_000);

        let counted = hound::WavReader::open(&path).map(|reader| reader.len());
        assert!(
            matches!(counted, Ok(0) | Err(_)),
            "hound가 읽었다면 이 모듈은 필요 없다: {counted:?}",
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn more_frames_appear_as_the_writer_keeps_going() {
        // 따라 읽는다는 것은 **같은 파일을 다시 물었을 때 더 많이 나온다**는 뜻이다.
        let path = temp_path("appending");
        let mut file = start_recording(&path, 1, 16_000, 1_000);

        let wav = GrowingWav::open(&path).expect("열 수 있어야 한다");
        let first = wav.written_frames().expect("셀 수 있어야 한다");

        append(&mut file, 1, 1_000, 1_000);
        let second = wav.written_frames().expect("셀 수 있어야 한다");

        assert_eq!(first, 1_000);
        assert_eq!(second, 2_000, "늘어난 만큼 보여야 한다");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_half_written_frame_is_not_counted() {
        // 맨 끝은 절반만 쓰였을 수 있다. **읽는 쪽이 물러선다.**
        let path = temp_path("half");
        let mut file = start_recording(&path, 2, 48_000, 100);
        file.write_all(&[0x11, 0x22]).expect("반 프레임을 쓸 수 있어야 한다");
        file.flush().expect("내보낼 수 있어야 한다");

        let wav = GrowingWav::open(&path).expect("열 수 있어야 한다");
        assert_eq!(
            wav.written_frames().expect("셀 수 있어야 한다"),
            100,
            "덜 쓰인 프레임을 세면 마지막 샘플이 튄다",
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_window_comes_back_as_sixteen_kilohertz_mono() {
        // 읽은 구간은 배치 경로와 **같은 모양**이어야 한다 — 엔진 경계가 하나이기 때문이다.
        let path = temp_path("window");
        let _file = start_recording(&path, 2, 48_000, 96_000);

        let wav = GrowingWav::open(&path).expect("열 수 있어야 한다");
        let input = wav
            .read(Window { start_frame: 0, end_frame: 48_000 })
            .expect("읽을 수 있어야 한다");

        assert_eq!(input.sample_rate_hz, audio_input::TARGET_SAMPLE_RATE_HZ);
        assert_eq!(input.channels, audio_input::TARGET_CHANNELS);
        assert_eq!(input.source_channels, 2);
        assert!(input.downmixed && input.resampled);
        assert!(
            (15_000..=17_000).contains(&input.samples.len()),
            "1초가 나와야 한다: {}",
            input.samples.len(),
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_window_in_the_middle_is_not_the_one_at_the_start() {
        // 오프셋 계산이 틀리면 전사가 같은 구간을 반복한다 — 화면에 같은 문장이 쌓인다.
        let path = temp_path("offset");
        let _file = start_recording(&path, 1, 16_000, 64_000);

        let wav = GrowingWav::open(&path).expect("열 수 있어야 한다");
        let head = wav.read(Window { start_frame: 0, end_frame: 16_000 }).expect("읽기");
        let middle = wav.read(Window { start_frame: 32_000, end_frame: 48_000 }).expect("읽기");

        assert_ne!(head.samples, middle.samples, "다른 자리는 다른 소리여야 한다");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_empty_window_is_refused_instead_of_returning_silence() {
        let path = temp_path("empty");
        let _file = start_recording(&path, 1, 16_000, 1_000);
        let wav = GrowingWav::open(&path).expect("열 수 있어야 한다");

        assert!(wav.read(Window { start_frame: 10, end_frame: 10 }).is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_file_that_is_not_a_wav_is_refused_quickly() {
        // 손상된 파일이 읽는 쪽을 영원히 붙잡지 않는다.
        let path = temp_path("garbage");
        std::fs::write(&path, vec![0_u8; 4_096]).expect("쓸 수 있어야 한다");

        assert!(GrowingWav::open(&path).is_err());

        let _ = std::fs::remove_file(&path);
    }
}
