//! 전사가 **도는 동안** 지금까지 나온 문장을 담아 두는 자리 (2026-09-07 추가).
//!
//! ```text
//! 엔진(청크마다)  ─push─→  Progress  ─read─→  Transcriber::status  ─→  화면
//!                            (공유)              (폴링 · 이미 있던 규약)
//! ```
//!
//! ## 왜 필요한가
//!
//! 72분 녹음의 전사는 수십 분이 걸린다. 그동안 화면이 보여 줄 수 있는 것은 "돌고 있다" 한
//! 마디뿐이었고, **무엇이 나오고 있는지 알 방법이 없었다.** 2026-09-07 실행은 40분을 돌고
//! 마지막에 실패해서 그때까지 만든 31개 청크 분량을 사람이 한 글자도 보지 못했다.
//!
//! ## 이 값이 Transcript가 아니다
//!
//! 여기 쌓이는 것은 **미리보기**다. 저장되지 않고, `parse`도 `collapse`도 지나지 않았으며,
//! 반복 차단도 걸리지 않았다. 저장되는 Transcript는 여전히 전사가 끝난 뒤 `run`이 만든다 —
//! 이 모듈은 그 경로에 끼어들지 않는다.
//!
//! 그래서 **여기 보이던 문장이 최종 Transcript에 없을 수 있다.** 붕괴 판정이 저장을 막았거나
//! (§18.5), 반복 차단이 지웠거나 (§20.6), 마지막 글자가 다음 조각으로 넘어갔을 때다.
//! 화면은 그 사실을 감추지 않는다.
//!
//! ## 청크를 밖으로 흘리지 않는다 (ADR-0007 §20.8 · INV-9)
//!
//! §20.8은 *"청크를 화면 · IPC에 드러내는 것: 아무도. 밖에서 보면 전사 하나가 나올 뿐이다"*
//! 라고 정했다. 그래서 이 모듈이 내보내는 진행률은 **"몇 번째 청크"가 아니라 "오디오의 몇
//! 퍼센트를 지났는가"** 다. 청크 길이가 바뀌어도, 청킹이 사라져도 이 값의 뜻은 그대로다.
//!
//! ## 단위 (ADR-0007 §10)
//!
//! 시각은 **원시 센티초**로 담는다. 밀리초로 바꾸지 않는다 — `× 10`이 일어나는 자리는
//! 여전히 `parse.rs` 하나이며, 이 미리보기는 그 자리를 지나지 않으므로 **바꿀 권한이 없다.**
//! 사람이 읽을 시각 문자열을 만드는 것은 화면의 몫이다.

use std::sync::{Arc, Mutex};

/// 전사 도중에 나온 문장 하나. **저장되는 값이 아니다.**
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialLine {
    /// 전체 시간축에서의 시작 시각. **센티초다** (§10 · 원시 단위).
    pub start_centiseconds: i64,
    /// 엔진이 낸 문장. 잘린 UTF-8은 이미 이어 붙여진 뒤다 (`chunking::decode_segment_texts`).
    pub text: String,
}

/// 지금까지 나온 것. 엔진 스레드가 쓰고 화면 폴링이 읽는다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// 지금까지 나온 문장들. 시간 순이다.
    pub lines: Vec<PartialLine>,
    /// **오디오 전체 프레임 수 중 몇 프레임을 지났는가.** 청크 개념이 아니다 (§20.8).
    pub frames_done: usize,
    /// 오디오 전체 프레임 수. 0이면 아직 모른다.
    pub frames_total: usize,
}

impl Snapshot {
    /// 0.0 ~ 1.0. 전체를 모르면 `None`이다 — **모르는 것을 0%라고 말하지 않는다.**
    pub fn fraction(&self) -> Option<f64> {
        if self.frames_total == 0 {
            return None;
        }
        Some((self.frames_done as f64 / self.frames_total as f64).clamp(0.0, 1.0))
    }
}

/// 엔진과 화면이 나눠 갖는 자리. **[`Clone`]은 같은 자리를 가리킨다.**
///
/// 잠금이 깨져도(`PoisonError`) 전사를 실패시키지 않는다 — 미리보기가 없어지는 것은
/// 사용자가 결과를 못 얻는 것과 다른 무게다. 그때는 조용히 아무것도 하지 않는다.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    inner: Arc<Mutex<Snapshot>>,
}

impl Progress {
    pub fn new() -> Self {
        Self::default()
    }

    /// 전사 하나가 시작한다. 지난 전사의 문장이 남아 있지 않게 비운다.
    pub fn begin(&self, frames_total: usize) {
        if let Ok(mut snapshot) = self.inner.lock() {
            *snapshot = Snapshot {
                lines: Vec::new(),
                frames_done: 0,
                frames_total,
            };
        }
    }

    /// 조각 하나가 끝났다. 나온 문장을 이어 붙이고 진행한 프레임 수를 더한다.
    pub fn advance(&self, lines: impl IntoIterator<Item = PartialLine>, frames_done: usize) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.lines.extend(lines);
            snapshot.frames_done = frames_done;
        }
    }

    /// 지금까지의 것. 화면이 읽는 값이다.
    pub fn snapshot(&self) -> Snapshot {
        self.inner.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// 전사가 끝났다(성공이든 실패든). 미리보기를 비운다 — 끝난 뒤의 값은 Transcript가 말한다.
    pub fn clear(&self) {
        if let Ok(mut snapshot) = self.inner.lock() {
            *snapshot = Snapshot::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(start: i64, text: &str) -> PartialLine {
        PartialLine {
            start_centiseconds: start,
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_fresh_progress_has_nothing_and_no_fraction() {
        let progress = Progress::new();
        let snapshot = progress.snapshot();

        assert!(snapshot.lines.is_empty());
        // 전체를 모르면 0%가 아니라 **모른다**이다.
        assert_eq!(snapshot.fraction(), None);
    }

    #[test]
    fn lines_accumulate_in_order_as_pieces_finish() {
        let progress = Progress::new();
        progress.begin(1_000);

        progress.advance([line(0, "첫 조각")], 400);
        progress.advance([line(12_000, "둘째 조각")], 800);

        let snapshot = progress.snapshot();
        assert_eq!(
            snapshot.lines,
            vec![line(0, "첫 조각"), line(12_000, "둘째 조각")]
        );
        assert_eq!(snapshot.frames_done, 800);
        assert_eq!(snapshot.fraction(), Some(0.8));
    }

    #[test]
    fn beginning_a_new_run_drops_the_previous_preview() {
        let progress = Progress::new();
        progress.begin(100);
        progress.advance([line(0, "지난 전사")], 100);

        progress.begin(200);

        let snapshot = progress.snapshot();
        assert!(snapshot.lines.is_empty());
        assert_eq!(snapshot.frames_done, 0);
        assert_eq!(snapshot.frames_total, 200);
    }

    #[test]
    fn clearing_leaves_nothing_and_no_fraction() {
        let progress = Progress::new();
        progress.begin(10);
        progress.advance([line(0, "무엇이든")], 10);

        progress.clear();

        assert_eq!(progress.snapshot(), Snapshot::default());
        assert_eq!(progress.snapshot().fraction(), None);
    }

    #[test]
    fn a_clone_points_at_the_same_place() {
        let progress = Progress::new();
        let engine_side = progress.clone();

        progress.begin(10);
        engine_side.advance([line(0, "엔진이 썼다")], 5);

        // 화면 쪽에서 읽힌다.
        assert_eq!(progress.snapshot().lines.len(), 1);
        assert_eq!(progress.snapshot().frames_done, 5);
    }

    #[test]
    fn the_fraction_never_leaves_zero_to_one() {
        let progress = Progress::new();
        progress.begin(100);
        // 어떤 이유로든 넘치게 보고돼도 1.0을 넘지 않는다.
        progress.advance([], 250);

        assert_eq!(progress.snapshot().fraction(), Some(1.0));
    }

    #[test]
    fn this_module_does_not_know_the_outside_world() {
        // `collapse.rs` · `chunking.rs`가 세운 검사와 같은 방법이다.
        let source = include_str!("progress.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production 부분이 있다");

        for forbidden in [
            "whisper_rs",
            "rusqlite",
            "std::fs",
            "std::process",
            "crate::db",
            "MILLISECONDS_PER_CENTISECOND",
            "CHUNK_SECONDS",
        ] {
            assert!(
                !production.contains(forbidden),
                "진행 모듈에 바깥 세계가 들어왔다: {forbidden}"
            );
        }
    }
}
