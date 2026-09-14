//! **녹음이 도는 동안 창을 하나씩 전사해 나간다** (2026-09-14).
//!
//! ```text
//! growing_wav  지금까지 쓰인 프레임 수를 묻는다
//!      ↓
//! live         전사할 구간이 있는가 (30초 · 꼬리 1초는 물러선다)
//!      ↓
//! engine       그 구간을 전사한다 — 모델은 물고 있다 (whisper::holding_the_model)
//!      ↓
//! 여기         시각을 전체 시간축으로 옮기고, 꼬리를 다음 창의 문맥으로 남긴다
//! ```
//!
//! ## 스레드를 만들지 않는다
//!
//! 이 모듈은 **한 걸음**과 **마무리**만 안다. 언제 부를지, 어느 스레드에서 부를지,
//! 멈춤을 어떻게 알릴지는 부르는 쪽의 일이다 — 그래야 실제 엔진도 스레드도 없이
//! 이 규칙을 그대로 지나며 확인할 수 있다 (§18).
//!
//! ## 시간축은 하나다
//!
//! 엔진은 자기가 받은 구간의 0초부터 시각을 매긴다. 여기서 창의 시작만큼 밀어
//! **녹음 전체의 시간축**으로 옮긴다 — 그래야 화면에 쌓인 문장의 시각이 녹음의
//! 시각과 같은 뜻을 갖는다 (ADR-0007 §10 · §20.5).

use super::engine::{LanguageChoice, TranscriptionEngine};
use super::model::ModelFile;
use super::growing_wav::GrowingWav;
use super::live::{self, Window};
use super::parse::{self, TranscriptSegment};
use crate::domain::Failure;

/// 다음 창에 문맥으로 넘길 꼬리의 길이 (문자 수).
///
/// **길수록 좋은 것이 아니다.** 프롬프트가 길면 모델이 그것을 되풀이해 뱉는 일이 생기고,
/// 짧으면 문맥이 되지 못한다. 한 문장 남짓을 넘긴다.
///
/// **[미검증]** 200자가 옳은 값인지는 재본 적이 없다.
pub const CARRY_CHARS: usize = 200;

/// 지금까지 실시간 전사가 만들어 낸 것.
///
/// **여기에 쌓인 것이 곧 최종본이다** (2026-09-14 운영자 결정). 녹음이 끝나면 이 값이
/// Transcript가 된다 — 정지 후에 다시 전사하지 않는다.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LiveProgress {
    /// 어디까지 전사했는가. 다음 창은 여기서 시작한다.
    pub done_frames: usize,
    /// 지금까지 나온 문장 전부 — 이미 전체 시간축으로 옮겨져 있다.
    pub segments: Vec<TranscriptSegment>,
    /// 엔진이 보고한 언어. 처음 보고한 것을 남긴다.
    pub language: Option<String>,
}

impl LiveProgress {
    /// 다음 창에 넘길 문맥. **마지막 문장들의 꼬리다.**
    pub fn carry(&self) -> String {
        let mut carried = String::new();
        for segment in self.segments.iter().rev() {
            if carried.chars().count() >= CARRY_CHARS {
                break;
            }
            if !carried.is_empty() {
                carried.insert(0, ' ');
            }
            carried.insert_str(0, &segment.text);
        }

        let excess = carried.chars().count().saturating_sub(CARRY_CHARS);
        if excess == 0 {
            return carried;
        }
        carried.chars().skip(excess).collect()
    }
}

/// 한 걸음 나아갔는가.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// 창 하나를 전사했다. 바로 다시 물어도 된다.
    Advanced,
    /// 아직 전사할 만큼 쌓이지 않았다. **실패가 아니다** — 조금 뒤에 다시 물으면 된다.
    NotYet,
}

/// 창 하나만큼 나아간다.
///
/// 전사할 것이 없으면 [`Step::NotYet`]이며 `progress`는 그대로다.
pub fn advance(
    wav: &GrowingWav,
    engine: &dyn TranscriptionEngine,
    model: &ModelFile,
    language: &LanguageChoice,
    progress: &mut LiveProgress,
) -> Result<Step, Failure> {
    let written = wav.written_frames()?;
    let Some(window) = live::next_window(written, progress.done_frames, wav.sample_rate_hz())
    else {
        return Ok(Step::NotYet);
    };

    transcribe_window(wav, engine, model, language, window, progress)?;
    Ok(Step::Advanced)
}

/// 녹음이 끝났다. **남은 꼬리를 마저 전사한다.**
///
/// 최소 길이를 요구하지 않는다 — 더 기다려도 더 오지 않는다 ([`live::final_window`]).
pub fn finish(
    wav: &GrowingWav,
    engine: &dyn TranscriptionEngine,
    model: &ModelFile,
    language: &LanguageChoice,
    progress: &mut LiveProgress,
) -> Result<(), Failure> {
    let written = wav.written_frames()?;
    let Some(window) = live::final_window(written, progress.done_frames) else {
        return Ok(());
    };
    transcribe_window(wav, engine, model, language, window, progress)
}

fn transcribe_window(
    wav: &GrowingWav,
    engine: &dyn TranscriptionEngine,
    model: &ModelFile,
    language: &LanguageChoice,
    window: Window,
    progress: &mut LiveProgress,
) -> Result<(), Failure> {
    let carried = progress.carry();
    let input = wav.read(window)?.with_preceding_text(&carried);

    let raw = engine.transcribe(&input, model, language)?;
    let transcription = parse::normalize(raw)?;

    if progress.language.is_none() {
        progress.language = transcription.language.clone();
    }

    // **엔진이 매긴 시각은 이 구간의 0초 기준이다.** 창의 시작만큼 밀어 전체 시간축으로
    // 옮긴다 — 옮기지 않으면 모든 문장이 0초 근처에 겹쳐 쌓인다.
    let offset = window.offset_ms(wav.sample_rate_hz());
    progress
        .segments
        .extend(transcription.segments.into_iter().map(|segment| {
            TranscriptSegment {
                start_ms: segment.start_ms + offset,
                end_ms: segment.end_ms + offset,
                ..segment
            }
        }));

    // **전사한 만큼만 나아간다.** 엔진이 무엇을 냈든 이 창은 끝났으므로 다음은 그다음부터다.
    progress.done_frames = window.end_frame;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcription::audio_input::TranscriptionInput;
    use crate::transcription::parse::{RawSegment, RawTranscription};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// 엔진 자리에 들어가는 double. **무엇을 받았는지 기록한다.**
    struct Echo {
        seen: Mutex<Vec<(usize, Option<String>)>>,
    }

    impl Echo {
        fn new() -> Self {
            Self { seen: Mutex::new(Vec::new()) }
        }
        fn calls(&self) -> Vec<(usize, Option<String>)> {
            self.seen.lock().expect("읽을 수 있어야 한다").clone()
        }
    }

    impl TranscriptionEngine for Echo {
        fn engine_id(&self) -> String {
            "echo".to_owned()
        }
        fn transcribe(
            &self,
            input: &TranscriptionInput,
            _model: &ModelFile,
            _language: &LanguageChoice,
        ) -> Result<RawTranscription, Failure> {
            let index = {
                let mut seen = self.seen.lock().expect("기록할 수 있어야 한다");
                seen.push((input.frames(), input.preceding_text.clone()));
                seen.len()
            };
            Ok(RawTranscription {
                language: Some("ko".to_owned()),
                // **언제나 자기 구간의 0초부터** 시각을 매긴다 — 실제 엔진과 같다.
                segments: vec![RawSegment {
                    start_centiseconds: 0,
                    end_centiseconds: 100,
                    text: Some(format!("창{index}의 말")),
                }],
            })
        }
    }

    /// 자리표시자 모델. **엔진 double은 이 파일을 열지 않는다.**
    fn model() -> ModelFile {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "molt-liverun-model-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("만들 수 있어야 한다");
        std::fs::write(dir.join("ggml-base.bin"), b"not a real model").expect("쓸 수 있어야 한다");
        crate::transcription::model::resolve(&dir, Some("ggml-base.bin"))
            .expect("해석할 수 있어야 한다")
    }

    fn zero_sized_header(channels: u16, sample_rate: u32) -> Vec<u8> {
        let bits = 16_u16;
        let block_align = channels * bits / 8;
        let mut header = Vec::new();
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&0_u32.to_le_bytes());
        header.extend_from_slice(b"WAVE");
        header.extend_from_slice(b"fmt ");
        header.extend_from_slice(&16_u32.to_le_bytes());
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&channels.to_le_bytes());
        header.extend_from_slice(&sample_rate.to_le_bytes());
        header.extend_from_slice(&(sample_rate * u32::from(block_align)).to_le_bytes());
        header.extend_from_slice(&block_align.to_le_bytes());
        header.extend_from_slice(&bits.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&0_u32.to_le_bytes());
        header
    }

    struct Recording {
        path: PathBuf,
        file: std::fs::File,
        rate: u32,
    }

    impl Recording {
        fn start(name: &str, rate: u32) -> Self {
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "molt-liverun-{name}-{}-{unique}.wav",
                std::process::id()
            ));
            let mut file = std::fs::File::create(&path).expect("만들 수 있어야 한다");
            file.write_all(&zero_sized_header(1, rate)).expect("헤더");
            Self { path, file, rate }
        }

        /// 초 단위로 소리를 더 쌓는다.
        fn record(&mut self, seconds: usize) {
            let frames = seconds * self.rate as usize;
            let mut bytes = Vec::with_capacity(frames * 2);
            for frame in 0..frames {
                let value = ((frame.wrapping_mul(2_654_435_761) >> 9) & 0x3fff) as i16 - 0x2000;
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            self.file.write_all(&bytes).expect("쓸 수 있어야 한다");
            self.file.flush().expect("내보낼 수 있어야 한다");
        }

        fn wav(&self) -> GrowingWav {
            GrowingWav::open(&self.path).expect("열 수 있어야 한다")
        }
    }

    impl Drop for Recording {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    fn step(recording: &Recording, engine: &Echo, progress: &mut LiveProgress) -> Step {
        advance(
            &recording.wav(),
            engine,
            &model(),
            &LanguageChoice::Chosen("ko".to_owned()),
            progress,
        )
        .expect("나아갈 수 있어야 한다")
    }

    #[test]
    fn nothing_happens_until_enough_has_been_recorded() {
        // 지키려는 것: **아직 아니다는 실패가 아니다.** 녹음을 막 시작한 순간에도
        // 조용히 기다릴 뿐이어야 한다.
        let mut recording = Recording::start("early", 16_000);
        let engine = Echo::new();
        let mut progress = LiveProgress::default();

        assert_eq!(step(&recording, &engine, &mut progress), Step::NotYet);

        recording.record(10);
        assert_eq!(step(&recording, &engine, &mut progress), Step::NotYet);

        assert!(progress.segments.is_empty());
        assert_eq!(progress.done_frames, 0);
        assert!(engine.calls().is_empty(), "엔진을 부르지 않아야 한다");
    }

    #[test]
    fn a_window_is_transcribed_once_enough_is_there() {
        let mut recording = Recording::start("first", 16_000);
        let engine = Echo::new();
        let mut progress = LiveProgress::default();

        // 30초 창 + 꼬리 1초를 물러서므로 31초는 있어야 한다.
        recording.record(32);
        assert_eq!(step(&recording, &engine, &mut progress), Step::Advanced);

        assert_eq!(progress.segments.len(), 1);
        assert_eq!(progress.done_frames, 30 * 16_000);
        assert_eq!(progress.language.as_deref(), Some("ko"));
        assert_eq!(engine.calls().len(), 1);
    }

    #[test]
    fn the_second_window_starts_where_the_first_ended() {
        // 지키려는 것: **같은 구간을 두 번 전사하지 않는다.** 겹치면 화면에 같은 문장이
        // 두 번 쌓이고, 그것이 최종본에 그대로 남는다.
        let mut recording = Recording::start("second", 16_000);
        let engine = Echo::new();
        let mut progress = LiveProgress::default();

        recording.record(32);
        assert_eq!(step(&recording, &engine, &mut progress), Step::Advanced);
        recording.record(30);
        assert_eq!(step(&recording, &engine, &mut progress), Step::Advanced);

        assert_eq!(progress.done_frames, 60 * 16_000);
        assert_eq!(progress.segments.len(), 2);
    }

    #[test]
    fn every_sentence_sits_on_the_recording_timeline() {
        // 엔진은 자기 구간의 0초부터 시각을 매긴다. 옮기지 않으면 모든 문장이 0초 근처에
        // 겹쳐 쌓이고, 화면의 시각이 녹음의 시각과 다른 뜻이 된다.
        let mut recording = Recording::start("timeline", 16_000);
        let engine = Echo::new();
        let mut progress = LiveProgress::default();

        recording.record(32);
        step(&recording, &engine, &mut progress);
        recording.record(30);
        step(&recording, &engine, &mut progress);

        assert_eq!(progress.segments[0].start_ms, 0);
        assert_eq!(
            progress.segments[1].start_ms, 30_000,
            "둘째 창은 30초 자리에서 시작해야 한다",
        );
    }

    #[test]
    fn the_tail_of_what_was_said_is_carried_into_the_next_window() {
        // 창을 그냥 자르면 경계에서 문장이 토막 난다. 앞의 꼬리를 문맥으로 준다.
        let mut recording = Recording::start("carry", 16_000);
        let engine = Echo::new();
        let mut progress = LiveProgress::default();

        recording.record(32);
        step(&recording, &engine, &mut progress);
        recording.record(30);
        step(&recording, &engine, &mut progress);

        let calls = engine.calls();
        assert_eq!(calls[0].1, None, "첫 창에는 앞이 없다");
        assert_eq!(
            calls[1].1.as_deref(),
            Some("창1의 말"),
            "둘째 창은 앞 창의 꼬리를 받아야 한다",
        );
    }

    #[test]
    fn a_long_history_is_carried_only_as_a_tail() {
        // **길수록 좋은 것이 아니다.** 프롬프트가 길면 모델이 그것을 되풀이해 뱉는다.
        let mut progress = LiveProgress::default();
        for index in 0..200 {
            progress.segments.push(TranscriptSegment {
                start_ms: index * 1_000,
                end_ms: index * 1_000 + 900,
                text: format!("문장 번호 {index} 입니다"),
            });
        }
        assert!(progress.carry().chars().count() <= CARRY_CHARS);
        assert!(
            progress.carry().ends_with("199 입니다"),
            "가장 최근의 말이 남아야 한다: {}",
            progress.carry(),
        );
    }

    #[test]
    fn stopping_transcribes_what_is_left_without_waiting_for_a_full_window() {
        // 지키려는 것: **마지막 몇 초가 사라지지 않는다.** 최소 길이를 요구하면
        // 30초가 안 되는 꼬리는 영원히 전사되지 않는다.
        let mut recording = Recording::start("finish", 16_000);
        let engine = Echo::new();
        let mut progress = LiveProgress::default();

        recording.record(32);
        step(&recording, &engine, &mut progress);
        recording.record(5);

        // 창이 모자라 더 나아가지 못한다.
        assert_eq!(step(&recording, &engine, &mut progress), Step::NotYet);

        finish(
            &recording.wav(),
            &engine,
            &model(),
            &LanguageChoice::Chosen("ko".to_owned()),
            &mut progress,
        )
        .expect("마무리할 수 있어야 한다");

        assert_eq!(progress.segments.len(), 2, "남은 꼬리가 전사돼야 한다");
        assert_eq!(progress.done_frames, 37 * 16_000, "끝까지 나아가야 한다");
    }

    #[test]
    fn finishing_an_already_finished_recording_does_nothing() {
        let mut recording = Recording::start("twice", 16_000);
        let engine = Echo::new();
        let mut progress = LiveProgress::default();

        recording.record(32);
        step(&recording, &engine, &mut progress);

        let language = LanguageChoice::Chosen("ko".to_owned());
        finish(&recording.wav(), &engine, &model(), &language, &mut progress).expect("한 번");
        let after_first = progress.clone();
        finish(&recording.wav(), &engine, &model(), &language, &mut progress).expect("두 번");

        assert_eq!(progress, after_first, "두 번 불러도 같아야 한다");
    }
}
