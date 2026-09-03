//! 원본 녹음에서 **파생 전사 입력**을 만든다 (ADR-0007 §9 · `phase-prompt/03` 요구 4·5).
//!
//! ## 무엇을 만드는가 — 그리고 왜 그 모양인가
//!
//! ```text
//! Phase 2의 raw recording      장치 native sample rate / channels의 PCM16 WAV (예: 48 kHz stereo)
//!                              [crate::audio::capture::CaptureFormat · ADR-0003 §4.2.3]
//!         │
//!         ▼  hound(읽기) + 수동 다운믹스 + rubato(리샘플) — 전부 순수 Rust (ADR-0007 §9.2)
//! 16 kHz mono f32 PCM          메모리 위의 버퍼. 파일이 아니다.
//! ```
//!
//! **산출 형태가 `f32` 버퍼인 것은 이 모듈의 취향이 아니라 선택된 통합 방식의 요구다.**
//! ADR-0007 §2.1이 고른 것은 `whisper-rs`이고, 그것이 받는 것은
//! `full(params, &[f32])` — **파일이 아니라 `f32` 슬라이스**다 (ADR-0007 §4.1 · §9.1 · §15).
//! sidecar(`whisper-cli`)를 골랐다면 요구는 *16-bit WAV 파일*이었고 파생 **파일**을
//! 만들어야 했겠지만, 그 후보는 §12.1에서 탈락했다. 그래서 **파생 입력은 디스크에 내려가지
//! 않는다** (ADR-0007 §9.1의 "B가 요구하는 것은 파일이 아니라 `f32` 슬라이스").
//!
//! ⚠️ ADR-0007 §14는 `whisper-rs`의 **정확한 API 시그니처를 UNVERIFIED [E4]** 로 남겼고,
//! crate를 실제로 추가하는 TASK-026이 빌드로 확인한다. 이 모듈이 확정적으로 다루는 것은
//! **16 kHz · mono · f32** 라는 세 값이며, 그것은 §9.1이 기록한 입력 요구다.
//!
//! ## 원본은 읽기만 한다 (INV-1 · INV-3 · ADR-0007 §9.3)
//!
//! ```text
//! raw recording        immutable · 보존 · 읽기 전용으로만 연다
//! derived 전사 입력     재생성 가능 · 메모리 위의 f32 버퍼 · 앱이 죽으면 그냥 사라진다
//! ```
//!
//! 이 모듈에는 **파일을 쓰는 코드가 없다.** 원본 경로를 덮어쓸 수 없는 이유가 "조심해서
//! 다른 경로를 고르기 때문"이 아니라 **파생 산출물에 경로라는 것이 없기 때문**이다.
//! [`TranscriptionInput`]에는 경로 필드가 없고, 이 파일 어디에도 `File::create` ·
//! `fs::write` · `WavWriter`가 없다. 원본과 같은 자리에 쓸 방법 자체가 없다.
//!
//! ## 실패는 panic이 아니라 제품 상태다 (§13 · ADR-0007 §9.3 규칙 3)
//!
//! 손상된 WAV · 잘린 파일 · 빈 파일 · 지원하지 않는 sample format · 예상과 다른 채널 수는
//! 전부 [`Failure`]로 돌아온다. **새 [`FailureKind`]를 만들지 않는다** — 그 enum은
//! `src/ipc/failure.ts`의 union과 1:1이며, 화면을 다루지 않는 이 Task가 그 계약을 넓히면
//! frontend가 모르는 종류가 조용히 생긴다. 대신 이미 있는 두 종류를 그 뜻대로 나눠 쓴다.
//!
//! ```text
//! Storage       파일을 열지 못했거나 끝까지 읽지 못했다 (crate::audio::finalized와 같은 뜻)
//! InvalidInput  파일은 읽혔지만 그 내용이 전사 입력의 규칙에 맞지 않아 아무것도 하지 않았다
//! ```
//!
//! 어느 쪽이든 **`source_data_safe`는 참으로 남는다.** 이 모듈은 원본을 건드리지 않으므로
//! 실패해도 훼손된 것이 없다 (INV-3). 그리고 전부 `permanent`다 — 같은 파일을 다시 읽어도
//! 답은 같다. (ADR-0007 §9.3 규칙 4의 "전사는 재시도 가능하다"는 **원본이 그대로이므로 파생
//! 입력을 언제든 다시 만들 수 있다**는 뜻이지, 손상된 파일이 다시 읽으면 나아진다는 뜻이 아니다.)

use std::path::Path;

use hound::{SampleFormat, WavReader};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

use crate::domain::{Failure, FailureKind};

/// 전사 입력의 샘플레이트. **whisper 모델의 요구값이다** (ADR-0007 §9.1 · §15).
pub const TARGET_SAMPLE_RATE_HZ: u32 = 16_000;

/// 전사 입력의 채널 수. mono다 (ADR-0007 §9.1).
pub const TARGET_CHANNELS: u16 = 1;

/// 읽을 수 있는 원본의 비트 심도.
///
/// Phase 2가 만드는 것은 이것뿐이다 ([`crate::audio::capture::BITS_PER_SAMPLE`]).
/// **만들어진 적 없는 형식을 추측으로 지원하지 않는다** (§20.6).
pub const SOURCE_BITS_PER_SAMPLE: u16 = 16;

/// 다운믹스할 수 있는 최대 채널 수.
///
/// mono와 stereo다. ADR-0007 §9.2가 적은 수동 다운믹스는 **stereo → mono**이며,
/// 3채널 이상을 mono로 접는 것은 채널 배치마다 가중치가 다른 별개의 문제다.
/// **근거 없는 custom DSP를 직접 구현하지 않는다** (ADR-0007 §9.2) — 평균으로 뭉개서
/// 조용히 잘못된 전사를 만드느니 예상과 다른 채널 수라는 사실을 그대로 알린다.
const MAX_SOURCE_CHANNELS: u16 = 2;

/// `i16`을 `f32`로 옮길 때 나누는 값.
///
/// `i16::MIN`(-32768)이 정확히 `-1.0`이 된다. whisper.cpp 자신이 WAV를 읽을 때 쓰는 것과
/// 같은 환산이다.
const I16_FULL_SCALE: f32 = 32_768.0;

/// 리샘플러에 한 번에 밀어 넣는 프레임 수. 실제 요구량은 리샘플러에게 다시 묻는다.
const RESAMPLER_CHUNK_FRAMES: usize = 1_024;

/// 리샘플 루프가 진행 없이 도는 것을 막는 안전장치.
///
/// 한 chunk가 한 프레임도 내놓지 못하는 상태가 이만큼 이어지면 무한 루프다. 그때는 돌지
/// 않고 실패로 나간다 — 전사가 앱을 멈추게 두지 않는다.
const MAX_BARREN_CHUNKS: usize = 8;

/// whisper에 넘길 준비가 끝난 파생 전사 입력.
///
/// **경로가 없다.** 이것은 메모리 위의 파생물이며 디스크에 자리를 갖지 않는다 (ADR-0007 §9.3).
/// 원본에 대한 값(`source_*`)이 함께 있는 것은 **무엇을 읽어서 만들었는지**를 provenance로
/// 남기기 위해서다 — 기대한 값이 아니라 파일에서 읽은 값이다.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionInput {
    /// 16 kHz mono f32 PCM. 값의 범위는 `[-1.0, 1.0]`이다.
    pub samples: Vec<f32>,
    /// 언제나 [`TARGET_SAMPLE_RATE_HZ`].
    pub sample_rate_hz: u32,
    /// 언제나 [`TARGET_CHANNELS`].
    pub channels: u16,
    /// 원본 파일에서 읽은 샘플레이트.
    pub source_sample_rate_hz: u32,
    /// 원본 파일에서 읽은 채널 수.
    pub source_channels: u16,
    /// 리샘플을 실제로 했는가. 원본이 이미 16 kHz면 거짓이다.
    pub resampled: bool,
    /// 다운믹스를 실제로 했는가. 원본이 이미 mono면 거짓이다.
    pub downmixed: bool,
}

impl TranscriptionInput {
    /// 샘플 프레임 수. mono이므로 샘플 수와 같다.
    pub fn frames(&self) -> usize {
        self.samples.len()
    }

    /// 이 입력이 나타내는 길이(밀리초).
    pub fn duration_ms(&self) -> i64 {
        // 16 kHz 고정이므로 나눗셈의 분모는 상수다.
        (self.frames() as i64) * 1_000 / i64::from(TARGET_SAMPLE_RATE_HZ)
    }

    /// 아무 변환도 필요하지 않았는가 — 원본이 이미 요구 포맷이었다는 뜻이다.
    pub fn passed_through(&self) -> bool {
        !self.resampled && !self.downmixed
    }
}

/// 원본 녹음 파일을 읽어 파생 전사 입력을 만든다.
///
/// **파일은 읽기 전용으로만 열린다** ([`WavReader::open`]). 이 함수가 끝난 뒤 `path`의
/// 바이트는 호출 전과 완전히 동일하며, 그 자리에도 그 옆에도 새 파일이 생기지 않는다
/// (INV-1 · INV-3).
///
/// 입력이 이미 16 kHz mono면 **리샘플도 다운믹스도 하지 않는다** — 불필요한 변환은 시간만
/// 쓰는 것이 아니라 원본에 없던 오차를 만든다 (ADR-0007 §9.2 마지막 줄).
pub fn load(path: &Path) -> Result<TranscriptionInput, Failure> {
    let mut reader = WavReader::open(path)
        .map_err(|error| unreadable(path, "녹음 파일을 전사 입력으로 열지 못했다", error))?;
    let spec = reader.spec();

    if spec.sample_format != SampleFormat::Int || spec.bits_per_sample != SOURCE_BITS_PER_SAMPLE {
        return Err(unusable(
            path,
            "이 녹음 파일의 샘플 형식으로는 전사 입력을 만들 수 없다",
        )
        .with_detail(format!(
            "expected={SOURCE_BITS_PER_SAMPLE}-bit int actual={}",
            describe(spec)
        )));
    }
    if spec.channels == 0 || spec.channels > MAX_SOURCE_CHANNELS {
        return Err(
            unusable(path, "이 녹음 파일의 채널 수로는 전사 입력을 만들 수 없다").with_detail(
                format!("expected=1..={MAX_SOURCE_CHANNELS} actual={}", spec.channels),
            ),
        );
    }
    if spec.sample_rate == 0 {
        return Err(unusable(path, "녹음 파일의 샘플레이트가 0이다"));
    }

    // **읽으면서 바로 mono로 접는다.** interleave된 원본을 통째로 f32로 펼쳤다가 접으면
    // 1시간짜리 48 kHz stereo에서 최대 사용량이 두 배가 된다 (`phase-prompt/03` 요구 3의
    // "1시간 분량"). 어차피 다음 단계가 쓰는 것은 mono 하나뿐이므로 펼친 중간 버퍼를
    // 만들지 않는다.
    //
    // **헤더가 말하는 길이로 미리 자리를 잡지도 않는다.** 손상된 헤더는 실제 파일 크기와
    // 무관한 수를 담을 수 있고, 그 수를 믿고 예약하면 잘린 파일 하나가 앱의 메모리를
    // 통째로 가져간다. 자리는 실제로 읽힌 만큼만 자란다.
    let channels = usize::from(spec.channels);
    let divisor = f32::from(spec.channels);
    let mut mono: Vec<f32> = Vec::new();
    let mut frame_sum = 0.0_f32;
    let mut filled = 0_usize;

    for sample in reader.samples::<i16>() {
        let sample =
            sample.map_err(|error| unreadable(path, "녹음 파일을 끝까지 읽지 못했다", error))?;
        // 한쪽 채널만 골라 쓰지 않는다 — 마이크가 한쪽에만 잡힌 녹음에서 반대쪽을 고르면
        // 무음이 된다. mono 입력에서는 `divisor`가 1이므로 값이 그대로 남는다.
        frame_sum += f32::from(sample) / I16_FULL_SCALE;
        filled += 1;
        if filled == channels {
            mono.push(frame_sum / divisor);
            frame_sum = 0.0;
            filled = 0;
        }
    }

    if mono.is_empty() && filled == 0 {
        return Err(unusable(path, "녹음 파일에 소리가 들어 있지 않다"));
    }
    if filled != 0 {
        // 마지막 프레임이 채널 수만큼 채워지지 않았다 — 헤더는 온전한데 데이터가 잘렸다.
        return Err(
            unusable(path, "녹음 파일의 샘플 수가 채널 수와 맞지 않는다").with_detail(format!(
                "samples={} channels={}",
                mono.len() * channels + filled,
                spec.channels
            )),
        );
    }

    let downmixed = spec.channels != TARGET_CHANNELS;
    let resampled = spec.sample_rate != TARGET_SAMPLE_RATE_HZ;
    let samples = if resampled {
        resample(path, mono, spec.sample_rate)?
    } else {
        mono
    };

    Ok(TranscriptionInput {
        samples,
        sample_rate_hz: TARGET_SAMPLE_RATE_HZ,
        channels: TARGET_CHANNELS,
        source_sample_rate_hz: spec.sample_rate,
        source_channels: spec.channels,
        resampled,
        downmixed,
    })
}

/// mono 신호를 [`TARGET_SAMPLE_RATE_HZ`]로 옮긴다 (rubato · ADR-0007 §9.2).
///
/// **여기 쓰인 rubato API는 이 Task가 빌드로 확인한 것이다.** ADR-0007 §9.2는 crate와 버전
/// (`rubato` 5.0.0)만 기록했고 타입 이름을 적지 않았다. 5.0.0의 실제 표면은
/// `Fft` + `FixedSync` + `audioadapter` 버퍼이며, 흔히 인용되는 0.16 시절의
/// `FftFixedIn`/`SincFixedIn`은 이 버전에 **없다** — 문서에서 옮겨 적은 것이 아니라
/// 컴파일러가 확인해 준 사실이다.
///
/// 리샘플러는 내부에 필터 지연을 갖는다. 그대로 이어 붙이면 **앞에 무음이 붙고 뒤가 잘린
/// 신호**가 나오고, 그러면 transcript의 timestamp가 통째로 밀린다. 그래서 두 가지를 한다.
///
/// ```text
/// 1. 입력이 끝난 뒤에도 무음을 밀어 넣어 필터 안에 남은 꼬리를 빼낸다
/// 2. 앞의 output_delay() 프레임을 버리고, 비율이 정하는 길이만큼만 남긴다
/// ```
fn resample(path: &Path, mono: Vec<f32>, source_rate: u32) -> Result<Vec<f32>, Failure> {
    let expected = expected_frame_count(mono.len(), source_rate, TARGET_SAMPLE_RATE_HZ);

    let channels = usize::from(TARGET_CHANNELS);
    let mut resampler = Fft::<f32>::new(
        source_rate as usize,
        TARGET_SAMPLE_RATE_HZ as usize,
        RESAMPLER_CHUNK_FRAMES,
        channels,
        FixedSync::Input,
    )
    .map_err(|error| {
        unusable(path, "이 녹음 파일의 샘플레이트를 16 kHz로 옮길 수 없다")
            .with_detail(format!("{source_rate} Hz -> {TARGET_SAMPLE_RATE_HZ} Hz: {error}"))
    })?;

    let delay = resampler.output_delay();
    let wanted = expected + delay;

    // 입력은 chunk 단위로 들어간다(`FixedSync::Input`). 마지막 chunk를 반쪽으로 넣는 대신
    // **뒤에 무음을 덧대어** 온전한 chunk로 만든다 — 그 무음이 필터 안에 남은 꼬리를 밀어낸다.
    let chunk = resampler.input_frames_next();
    let input_for_wanted = expected_frame_count(wanted, TARGET_SAMPLE_RATE_HZ, source_rate);
    let padded_len = (mono.len().max(input_for_wanted) + chunk).div_ceil(chunk) * chunk;
    let mut padded = mono;
    padded.resize(padded_len, 0.0);

    let out_capacity = resampler.output_frames_max();
    let mut scratch = vec![0.0_f32; out_capacity * channels];
    let mut out: Vec<f32> = Vec::with_capacity(wanted + scratch.len());
    let mut barren = 0_usize;

    for block in padded.chunks_exact(chunk * channels) {
        if out.len() >= wanted {
            break;
        }
        let input = InterleavedSlice::new(block, channels, chunk)
            .map_err(|error| conversion_stalled(path, error))?;
        let mut output = InterleavedSlice::new_mut(&mut scratch, channels, out_capacity)
            .map_err(|error| conversion_stalled(path, error))?;

        let (_, produced) = resampler
            .process_into_buffer(&input, &mut output, None)
            .map_err(|error| conversion_stalled(path, error))?;
        out.extend_from_slice(&scratch[..produced * channels]);

        if produced == 0 {
            barren += 1;
            if barren >= MAX_BARREN_CHUNKS {
                return Err(unusable(path, "녹음 파일을 16 kHz로 옮기지 못했다")
                    .with_detail(format!("resampler produced no frames for {chunk}-frame chunks")));
            }
        } else {
            barren = 0;
        }
    }

    // 앞의 지연분을 버린다. 남는 것이 비율이 정한 길이 그대로다.
    if out.len() < wanted {
        return Err(unusable(path, "녹음 파일을 16 kHz로 끝까지 옮기지 못했다")
            .with_detail(format!("produced={} expected={}", out.len(), wanted)));
    }
    out.drain(..delay);
    out.truncate(expected);
    Ok(out)
}

/// 리샘플 도중의 실패. 원본은 그대로다.
fn conversion_stalled(path: &Path, error: impl std::fmt::Display) -> Failure {
    unusable(path, "녹음 파일을 16 kHz로 옮기는 중에 멈췄다").with_detail(error)
}

/// 리샘플 결과가 가져야 할 프레임 수. 반올림한 `frames × to / from`이다.
///
/// `u128`로 계산하는 것은 긴 녹음에서 곱이 넘치지 않게 하기 위해서다 — 1시간 48 kHz는
/// 이미 1.7억 프레임이다.
fn expected_frame_count(frames: usize, from: u32, to: u32) -> usize {
    let from = u128::from(from);
    let to = u128::from(to);
    ((frames as u128 * to + from / 2) / from) as usize
}

/// 파일을 열거나 끝까지 읽지 못했다. 원본은 그대로 남아 있다.
fn unreadable(path: &Path, message: &str, error: hound::Error) -> Failure {
    Failure::permanent(
        FailureKind::Storage,
        format!("{message}: {}", path.display()),
    )
    .with_detail(error)
}

/// 파일은 읽혔지만 그 내용으로는 전사 입력을 만들 수 없다.
///
/// **`source_data_safe`를 내리지 않는다.** 이 모듈은 원본을 건드리지 않으므로 훼손된 것이
/// 없다 (INV-3). 문장에 경로가 들어가는 것은 사용자가 그 파일을 찾을 수 있어야 하기 때문이다.
fn unusable(path: &Path, message: impl std::fmt::Display) -> Failure {
    Failure::permanent(
        FailureKind::InvalidInput,
        format!("{message}: {}", path.display()),
    )
}

/// 파일에서 읽은 형식을 사람이 읽는 문장으로. 실패의 기술적 원인에만 쓴다.
fn describe(spec: hound::WavSpec) -> String {
    let sample_format = match spec.sample_format {
        SampleFormat::Int => "int",
        SampleFormat::Float => "float",
    };
    format!(
        "{} Hz · {}ch · {}-bit {}",
        spec.sample_rate, spec.channels, spec.bits_per_sample, sample_format
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
    ///
    /// **오디오 fixture는 저장소에 커밋하지 않는다** (ADR-0007 §8.3 · `.gitignore`가 `*.wav`를
    /// 전역 제외한다). 이 테스트들이 쓰는 WAV는 전부 여기서 합성된다.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let unique = format!(
                "molt-note-transcription-input-{}-{}-{}",
                label,
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// 16-bit PCM WAV 하나를 만든다. `samples`는 interleave된 값 그대로 쓰인다.
    fn pcm16_wav(
        directory: &Path,
        name: &str,
        sample_rate: u32,
        channels: u16,
        samples: &[i16],
    ) -> PathBuf {
        let path = directory.join(name);
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("사전 조건: 파일을 만든다");
        for sample in samples {
            writer.write_sample(*sample).expect("사전 조건: 샘플을 쓴다");
        }
        writer.finalize().expect("사전 조건: 파일을 확정한다");
        path
    }

    /// 일정한 값이 이어지는 mono 신호. 리샘플이 신호를 지우지 않았는지 보는 데 쓴다.
    fn constant(value: i16, frames: usize) -> Vec<i16> {
        vec![value; frames]
    }

    #[test]
    fn a_device_native_recording_becomes_the_16khz_mono_buffer_whisper_asks_for() {
        // ADR-0007 §9.1이 적은 대표 입력: 48 kHz stereo PCM16.
        let temp = TempDir::new("native");
        // 100 ms = 4800 프레임 × 2채널.
        let interleaved: Vec<i16> = (0..4_800).flat_map(|_| [1_000_i16, 3_000_i16]).collect();
        let path = pcm16_wav(temp.path(), "native.wav", 48_000, 2, &interleaved);

        let input = load(&path).expect("장치 native 녹음은 전사 입력이 되어야 한다");

        assert_eq!(input.sample_rate_hz, TARGET_SAMPLE_RATE_HZ, "16 kHz다");
        assert_eq!(input.channels, TARGET_CHANNELS, "mono다");
        assert_eq!(input.source_sample_rate_hz, 48_000, "원본 값이 그대로 남는다");
        assert_eq!(input.source_channels, 2);
        assert!(input.resampled && input.downmixed);

        // 샘플 수와 길이가 기대와 일치한다. 4800 프레임 @48kHz = 100 ms = 1600 프레임 @16kHz.
        assert_eq!(input.frames(), 1_600, "48 kHz 4800 프레임은 16 kHz 1600 프레임이다");
        assert_eq!(input.duration_ms(), 100, "길이는 변환 전후로 같다");
    }

    #[test]
    fn one_second_of_audio_is_still_one_second_after_conversion() {
        // ×10 / ×100 어긋남은 조용하다. 길이를 값으로 못 박는다.
        let temp = TempDir::new("one-second");
        let path = pcm16_wav(temp.path(), "one-second.wav", 44_100, 1, &constant(0, 44_100));

        let input = load(&path).expect("1초짜리 녹음을 읽어야 한다");

        assert_eq!(input.frames(), 16_000, "1초는 16 kHz에서 16000 프레임이다");
        assert_eq!(input.duration_ms(), 1_000);
        assert_ne!(input.duration_ms(), 100);
        assert_ne!(input.duration_ms(), 10_000);
    }

    #[test]
    fn downmixing_stereo_keeps_both_channels_rather_than_picking_one() {
        // 16 kHz로 두어 리샘플을 배제한다 — 이 테스트가 보는 것은 다운믹스 하나다.
        let temp = TempDir::new("downmix");
        let interleaved: Vec<i16> = (0..64).flat_map(|_| [1_000_i16, 3_000_i16]).collect();
        let path = pcm16_wav(temp.path(), "stereo.wav", 16_000, 2, &interleaved);

        let input = load(&path).expect("stereo 녹음을 읽어야 한다");

        assert_eq!(input.frames(), 64, "프레임 수는 채널을 접어도 그대로다");
        assert!(input.downmixed);
        assert!(!input.resampled, "16 kHz는 리샘플하지 않는다");

        let expected = 2_000.0 / I16_FULL_SCALE; // (1000 + 3000) / 2
        for (index, sample) in input.samples.iter().enumerate() {
            assert!(
                (sample - expected).abs() < 1e-6,
                "{index}번째 샘플이 두 채널의 평균이 아니다: {sample}"
            );
        }
        // 한쪽 채널만 골랐다면 나왔을 값들과 다르다.
        assert!((input.samples[0] - 1_000.0 / I16_FULL_SCALE).abs() > 1e-6);
        assert!((input.samples[0] - 3_000.0 / I16_FULL_SCALE).abs() > 1e-6);
    }

    #[test]
    fn a_silent_channel_does_not_disappear_into_the_other_one() {
        // 마이크가 한쪽에만 잡힌 녹음. 반대쪽 채널을 골랐다면 무음이 됐을 것이다.
        let temp = TempDir::new("one-sided");
        let interleaved: Vec<i16> = (0..32).flat_map(|_| [0_i16, 8_000_i16]).collect();
        let path = pcm16_wav(temp.path(), "one-sided.wav", 16_000, 2, &interleaved);

        let input = load(&path).expect("한쪽만 잡힌 녹음도 읽어야 한다");

        let expected = 4_000.0 / I16_FULL_SCALE;
        assert!((input.samples[0] - expected).abs() < 1e-6);
        assert!(
            input.samples.iter().all(|sample| sample.abs() > 1e-6),
            "소리가 사라지지 않는다"
        );
    }

    #[test]
    fn an_input_that_is_already_16khz_mono_passes_through_untouched() {
        let temp = TempDir::new("passthrough");
        let samples: Vec<i16> = (0..256).map(|index| (index * 37) as i16).collect();
        let path = pcm16_wav(temp.path(), "ready.wav", 16_000, 1, &samples);

        let input = load(&path).expect("이미 요구 포맷인 입력을 읽어야 한다");

        assert!(input.passed_through(), "불필요한 변환을 하지 않는다");
        assert!(!input.resampled);
        assert!(!input.downmixed);
        assert_eq!(input.frames(), 256);

        // 값이 그대로다 — 리샘플러를 통과했다면 필터가 값을 바꿨을 것이다.
        let expected: Vec<f32> = samples
            .iter()
            .map(|sample| f32::from(*sample) / I16_FULL_SCALE)
            .collect();
        assert_eq!(input.samples, expected, "샘플 값이 손대지 않은 채로 남는다");
    }

    #[test]
    fn resampling_carries_the_signal_rather_than_emitting_silence() {
        // 길이만 맞고 내용이 사라지는 리샘플을 통과시키지 않는다.
        let temp = TempDir::new("signal");
        let path = pcm16_wav(temp.path(), "dc.wav", 48_000, 1, &constant(16_000, 24_000));

        let input = load(&path).expect("리샘플할 녹음을 읽어야 한다");

        assert_eq!(input.frames(), 8_000);
        let expected = 16_000.0 / I16_FULL_SCALE;
        // 양 끝은 필터 경계라 흔들린다. 가운데 절반만 본다.
        let middle = &input.samples[2_000..6_000];
        for (index, sample) in middle.iter().enumerate() {
            assert!(
                (sample - expected).abs() < 0.01,
                "{index}번째 샘플에서 신호가 유지되지 않았다: {sample} (기대 {expected})"
            );
        }
    }

    #[test]
    fn a_sound_stays_where_it_was_in_time_after_resampling() {
        // DC 신호만으로는 **시간이 밀린 것**을 볼 수 없다. 리샘플러의 필터 지연을 보정하지
        // 않으면 앞에 무음이 붙고 소리가 통째로 뒤로 밀리며, 그러면 transcript의 timestamp가
        // 전부 어긋난다 (ADR-0007 §10). 소리가 시작하는 자리를 값으로 못 박는다.
        let temp = TempDir::new("offset");
        let mut samples = constant(0, 4_800); // 100 ms 무음
        samples.extend(constant(16_000, 4_800)); // 그다음 100 ms 소리
        let path = pcm16_wav(temp.path(), "step.wav", 48_000, 1, &samples);

        let input = load(&path).expect("계단 신호를 읽어야 한다");

        assert_eq!(input.frames(), 3_200, "200 ms는 16 kHz에서 3200 프레임이다");
        let expected = 16_000.0 / I16_FULL_SCALE;

        // **소리가 시작하는 자리를 직접 잰다.** "앞은 조용하고 뒤는 시끄럽다"만 보면 여유
        // 구간보다 작은 밀림을 놓친다 — 이 리샘플러의 지연은 16 kHz에서 43 프레임이고,
        // 그것을 보정하지 않은 결과도 느슨한 검사는 통과해 버린다.
        let onset = input
            .samples
            .iter()
            .position(|sample| sample.abs() > expected / 2.0)
            .expect("소리가 시작하는 자리가 있어야 한다");
        assert!(
            onset.abs_diff(1_600) <= 20,
            "소리가 시작하는 자리가 밀렸다: {onset} (기대 1600 ±20)"
        );

        for (index, sample) in input.samples[..1_500].iter().enumerate() {
            assert!(
                sample.abs() < 0.05,
                "{index}번째에 아직 소리가 없어야 한다: {sample}"
            );
        }
        for (index, sample) in input.samples[1_700..].iter().enumerate() {
            assert!(
                (sample - expected).abs() < 0.05,
                "{}번째에 소리가 있어야 한다: {sample}",
                index + 1_700
            );
        }
    }

    #[test]
    fn a_recording_shorter_than_one_resampler_chunk_still_converts() {
        // 리샘플러는 chunk 단위로 먹는다. 한 chunk도 못 채우는 입력에서 무한 루프도
        // 패닉도 나지 않아야 한다.
        let temp = TempDir::new("tiny");
        let path = pcm16_wav(temp.path(), "tiny.wav", 48_000, 2, &constant(1_200, 60));

        let input = load(&path).expect("아주 짧은 녹음도 전사 입력이 되어야 한다");

        assert_eq!(input.frames(), 10, "30 프레임 @48kHz는 10 프레임 @16kHz다");
        assert_eq!(input.source_channels, 2);
        assert!(input.resampled && input.downmixed);
    }

    #[test]
    fn the_source_file_is_byte_for_byte_identical_after_conversion() {
        // INV-1 · INV-3. 변환은 원본을 읽기만 한다.
        let temp = TempDir::new("immutable");
        let interleaved: Vec<i16> = (0..9_600).flat_map(|_| [500_i16, -500_i16]).collect();
        let path = pcm16_wav(temp.path(), "source.wav", 48_000, 2, &interleaved);

        let before = fs::read(&path).expect("사전 조건: 원본을 읽는다");
        let modified_before = fs::metadata(&path)
            .expect("사전 조건: 메타데이터를 읽는다")
            .modified()
            .expect("사전 조건: 수정 시각을 읽는다");

        let input = load(&path).expect("변환은 성공해야 한다");
        assert!(input.resampled && input.downmixed, "실제로 변환이 일어났다");

        let after = fs::read(&path).expect("원본이 남아 있어야 한다");
        assert_eq!(before, after, "원본 바이트가 완전히 동일하다 (INV-1)");
        assert_eq!(
            modified_before,
            fs::metadata(&path)
                .expect("메타데이터를 읽을 수 있어야 한다")
                .modified()
                .expect("수정 시각을 읽을 수 있어야 한다"),
            "원본을 쓰기로 열지도 않았다"
        );
    }

    #[test]
    fn conversion_leaves_no_derived_file_anywhere_near_the_source() {
        // 파생 산출물의 경로가 원본 경로와 같아질 수 없는 이유는 **경로가 없기 때문**이다.
        // 원본 옆에 무엇도 생기지 않는 것으로 그것을 확인한다 (ADR-0007 §9.3).
        let temp = TempDir::new("no-derived-file");
        let interleaved: Vec<i16> = (0..4_800).flat_map(|_| [100_i16, 200_i16]).collect();
        let path = pcm16_wav(temp.path(), "only.wav", 48_000, 2, &interleaved);

        let before = listing(temp.path());
        assert_eq!(before, vec![path.clone()], "사전 조건: 원본 하나뿐이다");

        let input = load(&path).expect("변환은 성공해야 한다");

        assert_eq!(listing(temp.path()), before, "파생 파일이 생기지 않는다");
        assert!(!input.samples.is_empty(), "파생물은 메모리에만 있다");
    }

    /// 디렉터리 안의 경로를 정렬해서 돌려준다.
    fn listing(directory: &Path) -> Vec<PathBuf> {
        let mut entries: Vec<PathBuf> = fs::read_dir(directory)
            .expect("디렉터리를 읽을 수 있어야 한다")
            .map(|entry| entry.expect("항목을 읽을 수 있어야 한다").path())
            .collect();
        entries.sort();
        entries
    }

    #[test]
    fn an_empty_file_is_a_defined_failure_rather_than_a_panic() {
        let temp = TempDir::new("empty");
        let path = temp.path().join("empty.wav");
        fs::write(&path, []).expect("사전 조건: 빈 파일을 둔다");

        let failure = load(&path).expect_err("빈 파일로는 전사 입력을 만들 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(failure.message.contains(&path.display().to_string()));
        assert!(failure.source_data_safe, "원본을 건드리지 않았다 (INV-3)");
        assert!(!failure.retryable);
        assert!(path.is_file(), "실패해도 파일을 지우지 않는다");
    }

    #[test]
    fn a_wav_with_a_header_but_no_sound_is_a_defined_failure() {
        let temp = TempDir::new("silent");
        let path = pcm16_wav(temp.path(), "silent.wav", 48_000, 2, &[]);

        let failure = load(&path).expect_err("소리가 없는 파일로는 전사 입력을 만들 수 없다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.source_data_safe);
        assert!(path.is_file());
    }

    #[test]
    fn a_truncated_file_is_a_defined_failure_rather_than_a_panic() {
        // 헤더는 온전한데 데이터가 잘렸다. 크기만으로는 정상 파일과 구분되지 않는다.
        let temp = TempDir::new("truncated");
        let path = pcm16_wav(temp.path(), "truncated.wav", 48_000, 2, &constant(700, 4_096));
        let bytes = fs::read(&path).expect("사전 조건: 파일을 읽는다");
        fs::write(&path, &bytes[..bytes.len() / 2]).expect("사전 조건: 파일을 자른다");

        let failure = load(&path).expect_err("잘린 파일로는 전사 입력을 만들 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage, "끝까지 읽지 못한 것이다");
        assert!(failure.detail.is_some(), "기술적 원인이 함께 남는다");
        assert!(failure.source_data_safe);
    }

    #[test]
    fn a_file_that_is_not_wav_at_all_is_a_defined_failure() {
        let temp = TempDir::new("garbage");
        let path = temp.path().join("garbage.wav");
        fs::write(&path, [7_u8; 8_192]).expect("사전 조건: WAV가 아닌 파일을 둔다");

        let failure = load(&path).expect_err("WAV가 아닌 파일은 읽을 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(failure.source_data_safe);
        assert!(path.is_file(), "실패해도 파일을 지우지 않는다");
    }

    #[test]
    fn a_missing_file_is_a_defined_failure_that_names_the_path() {
        let temp = TempDir::new("missing");
        let path = temp.path().join("없는-녹음.wav");

        let failure = load(&path).expect_err("없는 파일로는 전사 입력을 만들 수 없다");

        assert_eq!(failure.kind, FailureKind::Storage);
        assert!(
            failure.message.contains(&path.display().to_string()),
            "어느 파일인지 보인다: {}",
            failure.message
        );
    }

    #[test]
    fn an_unsupported_sample_format_is_refused_instead_of_being_guessed_at() {
        // 32-bit float WAV. Phase 2가 만드는 형식이 아니다.
        let temp = TempDir::new("float");
        let path = temp.path().join("float.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("사전 조건: 파일을 만든다");
        for index in 0..128 {
            writer
                .write_sample(index as f32 / 128.0)
                .expect("사전 조건: 샘플을 쓴다");
        }
        writer.finalize().expect("사전 조건: 파일을 확정한다");

        let failure = load(&path).expect_err("지원하지 않는 sample format은 거절된다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        let detail = failure.detail.expect("기대값과 실제값이 함께 남는다");
        assert!(detail.contains("float"), "실제 형식이 보인다: {detail}");
        assert!(detail.contains("16-bit int"), "기대한 형식이 보인다: {detail}");
    }

    #[test]
    fn an_unsupported_bit_depth_is_refused() {
        let temp = TempDir::new("eight-bit");
        let path = temp.path().join("eight-bit.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 8,
            sample_format: SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).expect("사전 조건: 파일을 만든다");
        for index in 0..128_i32 {
            writer.write_sample(index).expect("사전 조건: 샘플을 쓴다");
        }
        writer.finalize().expect("사전 조건: 파일을 확정한다");

        let failure = load(&path).expect_err("8-bit PCM은 이 경로가 다루는 형식이 아니다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.source_data_safe);
    }

    #[test]
    fn a_channel_count_we_cannot_downmix_is_refused_rather_than_averaged_blindly() {
        // 3채널을 mono로 접는 것은 배치마다 가중치가 다른 별개의 문제다. 조용히 뭉개지 않는다.
        let temp = TempDir::new("three-channel");
        let interleaved: Vec<i16> = (0..300).map(|index| index as i16).collect();
        let path = pcm16_wav(temp.path(), "three.wav", 16_000, 3, &interleaved);

        let failure = load(&path).expect_err("예상과 다른 채널 수는 거절된다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        let detail = failure.detail.expect("실제 채널 수가 남는다");
        assert!(detail.contains('3'), "실제 채널 수가 보인다: {detail}");
        assert!(failure.source_data_safe, "원본은 그대로다");
        assert!(path.is_file());
    }

    #[test]
    fn every_malformed_input_returns_a_failure_and_never_panics() {
        // AC6의 요구는 "정의된 실패"이지 "특정 종류"가 아니다. 어느 계층이 거절하든
        // panic 없이 Failure로 돌아오는지를 한자리에서 확인한다.
        let temp = TempDir::new("malformed");
        let cases: [(&str, Vec<u8>); 4] = [
            ("empty", Vec::new()),
            ("riff-only", b"RIFF".to_vec()),
            // 헤더가 채널 수 0을 주장한다. 프레임 수 계산이 0으로 나누는 자리다.
            ("zero-channels", raw_wav_header(0, 16_000, 16)),
            // 헤더가 샘플레이트 0을 주장한다.
            ("zero-sample-rate", raw_wav_header(1, 0, 16)),
        ];

        for (label, bytes) in cases {
            let path = temp.path().join(format!("{label}.wav"));
            fs::write(&path, &bytes).expect("사전 조건: 파일을 둔다");

            let result = load(&path);

            let failure = result.expect_err(&format!("{label}: 잘못된 입력이 성공으로 통과했다"));
            assert!(
                failure.source_data_safe,
                "{label}: 실패해도 원본은 그대로다 (INV-3)"
            );
            assert!(path.is_file(), "{label}: 실패해도 파일을 지우지 않는다");
        }
    }

    /// 값을 직접 박아 넣은 RIFF/WAVE 헤더. hound가 만들어 주지 않는 잘못된 값을 만들 때 쓴다.
    ///
    /// data chunk는 비어 있다 — 이 헤더들이 검사하는 것은 fmt 값이지 내용이 아니다.
    fn raw_wav_header(channels: u16, sample_rate: u32, bits: u16) -> Vec<u8> {
        let block_align = channels.max(1) * bits / 8;
        let byte_rate = sample_rate * u32::from(block_align);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&36_u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes
    }

    #[test]
    fn the_resampler_reports_the_delay_this_module_compensates_for() {
        // 이 값을 코드가 상수로 갖지 않는 것이 요점이다 — 리샘플러에게 묻는다.
        // 여기서 고정하는 것은 값이 아니라 **0이 아니라는 사실**이다. 0이라면 보정 코드가
        // 무의미하다는 뜻이고, 그때는 [`resample`]의 drain을 다시 봐야 한다.
        let resampler =
            Fft::<f32>::new(48_000, 16_000, RESAMPLER_CHUNK_FRAMES, 1, FixedSync::Input)
                .expect("48 kHz -> 16 kHz 리샘플러를 만들 수 있어야 한다");

        assert!(
            resampler.output_delay() > 0,
            "지연이 없다면 보정할 것도 없다 — resample()의 가정이 깨진 것이다"
        );
        assert_eq!(
            resampler.input_frames_next(),
            RESAMPLER_CHUNK_FRAMES,
            "FixedSync::Input은 입력 chunk 크기를 고정한다 — resample()이 그 위에 서 있다"
        );
    }

    #[test]
    fn the_expected_frame_count_follows_the_ratio_without_drifting() {
        assert_eq!(expected_frame_count(48_000, 48_000, 16_000), 16_000);
        assert_eq!(expected_frame_count(44_100, 44_100, 16_000), 16_000);
        assert_eq!(expected_frame_count(0, 48_000, 16_000), 0);
        // 1시간 48 kHz. u128로 계산하지 않으면 곱이 넘칠 수 있는 크기다.
        assert_eq!(
            expected_frame_count(48_000 * 3_600, 48_000, 16_000),
            16_000 * 3_600
        );
    }
}
