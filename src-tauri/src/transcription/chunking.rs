//! 전사를 어디서 자르고 · 어떻게 되돌려 잇고 · 무엇을 차단하는가 —
//! **그 규칙이 사는 자리는 이 모듈 하나다** (ADR-0007 §20.8).
//!
//! ```text
//! 파생 입력의 프레임 수 (16 kHz mono)
//!         │
//!         │  ① 어디서 자르는가        plan  → 청크 구간 + 청크별 오프셋(센티초)
//!         ▼
//! 청크마다:  state 재생성 → 엔진 호출 → 청크 로컬 원시 segment (센티초)
//!         │
//!         │  ② 어떻게 이어 붙이는가   shift · merge → RawTranscription 하나
//!         ▼
//!   parse (단위 변환은 여기 한 곳 · §10) → collapse::assess (§18 · 차단 전)
//!         │
//!         │  ③ 무엇을 차단하는가      block_consecutive_repeats
//!         ▼
//!   저장되는 Transcript
//! ```
//!
//! **이 모듈은 오디오도 엔진도 모른다.** 파일시스템 · 데이터베이스 · 네트워크 · 시계 ·
//! whisper 라이브러리 타입이 여기 없다 — `collapse.rs` · `parse.rs`가 세운 선례 그대로이며,
//! 들어오는 것도 나가는 것도 전부 값이다. 그래서 실제 whisper도 모델도 없이 테스트된다
//! (PRODUCT-SPEC §18). [`plan`]이 내는 것은 **구간**이고, 그 구간으로 버퍼를 자르는 것은
//! 부르는 쪽이다 — 그래야 이 모듈이 오디오를 모르는 채로 남는다 (§20.8).
//!
//! ## 값은 셋이고, 전부 여기 있다 (ADR-0007 §20.8)
//!
//! ```text
//! CHUNK_SECONDS          = 120   청크 길이 (§20.2)
//! CHUNK_OVERLAP_FRAMES   = 0     겹침을 두지 않는다 (§20.4)
//! MAX_CONSECUTIVE_REPEATS = 3    연속 3회까지 통과 · 4회째부터 차단 (§20.6.1)
//! ```
//!
//! `whisper.rs` · `run.rs` · payload · 화면 어디에도 이 숫자들이 복제되지 않는다.
//! 두 자리에 있으면 한쪽만 고쳐지는 날이 온다 (`collapse.rs`가 임계값 셋에 대해 세운 선례다).
//!
//! ## 단위 — §10을 깨지 않는 이유
//!
//! | 값 | 단위 | 성질 |
//! | --- | --- | --- |
//! | 청크가 낸 segment의 start/end | 센티초 | **원시** |
//! | 청크 오프셋 [`AudioChunk::offset_centiseconds`] | 센티초 | **원시** |
//! | 둘의 합 ([`shift`]) | 센티초 | **원시** |
//!
//! **같은 단위끼리의 덧셈은 단위 변환이 아니다** (ADR-0007 §20.5). 센티초 → 밀리초 변환은
//! 여전히 `parse.rs`의 `MILLISECONDS_PER_CENTISECOND` 한 자리에서만 일어나며, **이 모듈은
//! 밀리초라는 단위를 알지 않는다.** 여기서 한 번 일어나는 것은 *오디오 위치를 시각으로
//! 말하는 것*(프레임 → 센티초)이고, `CHUNK_SECONDS`가 정수 초이므로
//! `1,920,000 프레임 × 100 / 16,000 = 12,000 센티초`가 나머지 없이 떨어진다 — 나눗셈도 실수
//! 연산도 없다.
//!
//! **넘침은 조용히 접지 않는다.** `parse.rs`가 `× 10`에서 그랬듯, 표현할 수 없는 값을
//! saturate해서 그럴듯하게 만들지 않는다 — 틀린 시각은 **영구히** 저장된다 (INV-2).
//! 실패는 [`FailureKind::InvalidInput`]이며 **새 [`FailureKind`]를 만들지 않는다.**
//!
//! ## 이 모듈이 하지 않는 것
//!
//! | 하지 않는 것 | 어디가 하는가 |
//! | --- | --- |
//! | 오디오 샘플을 복사해 들고 있는 것 | 부르는 쪽이 [`AudioChunk::range`]로 버퍼를 자른다 (§20.8) |
//! | 겹친 구간의 병합 · 중복 제거 | **겹침이 없다** (§20.4). 다룰 겹친 구간 자체가 없다 |
//! | 단위 변환 · segment 순서 재정렬 · 시각 추정 | `parse.rs` (§10) — 이 모듈은 더하기만 한다 |
//! | 붕괴 판정 | `collapse.rs` (§18). **차단은 판정을 대체하지 않는다** (§20.6.2) |
//! | 설정을 보는 것 | 아무도 — 청킹 모듈에 `LanguageChoice`도 `Settings`도 들어오지 않는다 (§20.7) |
//! | 청크를 화면 · IPC에 드러내는 것 | 아무도. **밖에서 보면 전사 하나가 나올 뿐이다** (INV-9) |

use std::ops::Range;

use crate::domain::{Failure, FailureKind};

use super::collapse::sentence_key;
use super::parse::{RawSegment, RawTranscription, TranscriptSegment};

/// 청크 하나의 길이. **초다** (ADR-0007 §20.2).
///
/// 근거: 2026-09-05 실험이 **실제로 쓴 값**이며, 그 조건이 72.85분 오디오에서 고유 94.0%를
/// 냈다 [E5]. 추론으로 고른 값이 아니다 — 60초 · 180초 · 300초는 이 프로젝트가 한 번도 재지
/// 않았고, 측정되지 않은 값으로 바꾸면 그 94.0%와 비교할 대상이 사라진다.
///
/// **정수 초다.** 실수 초로 나누면 청크마다 반올림이 쌓여 경계가 어긋난다.
pub const CHUNK_SECONDS: u32 = 120;

/// 청크 사이의 겹침. **0이다** (ADR-0007 §20.4).
///
/// 청크 k는 프레임 `[k·L, (k+1)·L)`이며 프레임 하나도 두 번 들어가지 않는다. 겹침을 두면
/// *무엇을 같은 문장으로 볼 것인가 · 어느 쪽을 남길 것인가 · timestamp를 어느 쪽 것으로
/// 할 것인가*가 전부 새 규칙이 되고, 그 규칙이 맞았는지 판정할 관측이 이 저장소에 없다.
/// `parse.rs`가 이미 겹침을 [`super::parse::AnomalyKind::Overlap`]으로 다루고 있어 규칙이 두
/// 자리에 생기기도 한다.
///
/// **대가는 감추지 않는다** — 경계에 걸친 문장은 두 조각으로 잘리거나 한 마디가 어느 쪽에도
/// 남지 않을 수 있다. 그것이 실제로 얼마나 일어나는지는 **[미검증]**이다 (§20.4).
///
/// 이 값을 0이 아닌 것으로 바꾸는 일은 상수 하나를 고치는 일이 아니다 — [`merge`]에 겹친
/// 구간의 병합 규칙이 **없기** 때문이며, 그 규칙을 먼저 §20.4가 적은 형태로 써야 한다.
pub const CHUNK_OVERLAP_FRAMES: usize = 0;

/// 연속으로 같은 문장이 몇 번까지 남는가. **3이다** (ADR-0007 §20.6.1).
///
/// 근거: 2026-09-05가 적은 *"연속 3회 초과 차단"*이 뜻하는 것이 이것이다 —
/// **3회까지 통과 · 4회째부터 차단** [E5]. 연속 2회 · 5회와 비교한 측정은 **없다**.
///
/// **대가는 [미검증]이다** (§20.6.3): 같은 짧은 대답("네" · "맞아요")이 네 번 이상 연달아
/// 나오는 회의는 실재하고, 그때 4번째부터는 Transcript에 남지 않는다. Transcript는
/// immutable하므로(INV-2) 그 문장이 나중에 돌아오는 경로는 없다.
pub const MAX_CONSECUTIVE_REPEATS: usize = 3;

/// 1초는 몇 센티초인가. 오프셋을 **원시 단위**로 말하기 위한 계수다 (ADR-0007 §20.5).
///
/// 이것은 §10이 한 자리로 묶은 *센티초 → 밀리초* 변환이 **아니다.** 여기 있는 것은
/// *오디오 위치를 시각으로 말하는 것*이며, 나온 값은 여전히 원시 센티초다.
const CENTISECONDS_PER_SECOND: u32 = 100;

/// 자를 자리 하나. **오디오를 담지 않는다 — 구간과 오프셋뿐이다** (ADR-0007 §20.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioChunk {
    /// 파생 입력(16 kHz mono) 안에서 이 청크가 시작하는 프레임 번호.
    pub start_frame: usize,
    /// 이 청크의 길이(프레임). **마지막 청크는 남은 만큼이다** — 짧다고 앞 청크에 합치지 않고,
    /// 무음으로 채우지도 않는다 (§20.2).
    pub frame_count: usize,
    /// 이 청크가 낸 원시 timestamp에 더할 값. **센티초다** (§20.5).
    ///
    /// `offset_centiseconds = k × CHUNK_SECONDS × 100` — 청크 번호로만 정해지므로 **앞 청크가
    /// 문장을 하나도 내지 못해도 다음 청크의 오프셋이 밀리지 않는다.**
    pub offset_centiseconds: i64,
}

impl AudioChunk {
    /// 이 청크가 끝나는(포함하지 않는) 프레임 번호.
    pub fn end_frame(&self) -> usize {
        self.start_frame + self.frame_count
    }

    /// 부르는 쪽이 파생 입력 버퍼를 자를 구간. **자르는 일은 부르는 쪽의 몫이다** (§20.8).
    pub fn range(&self) -> Range<usize> {
        self.start_frame..self.end_frame()
    }
}

/// 청크 하나가 낸 원시 출력과 그 청크의 오프셋. [`merge`]의 입력이다.
///
/// 여기 들어오는 것은 값뿐이다 — `LanguageChoice`도 `Settings`도 이 자리에 없으므로
/// **설정 값이 `language`에 베껴 들어갈 경로 자체가 없다** (ADR-0007 §20.7 · §17.1.4-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkTranscription {
    /// 이 청크의 [`AudioChunk::offset_centiseconds`].
    pub offset_centiseconds: i64,
    /// 엔진이 이 청크에 대해 낸 출력. timestamp는 **청크 로컬**이다.
    pub output: RawTranscription,
}

/// 연속 반복을 차단한 결과. **지워진 개수를 잃지 않는다** (ADR-0007 §20.6.2).
///
/// §18의 붕괴 판정이 본 수치(n · u · r)는 **차단 전** 열의 것이므로 저장된 segment 수보다 클
/// 수 있다. 그 차이가 [`Self::removed_count`]이며, 이 값이 남지 않으면 다음 사람이 두 수치의
/// 차이를 설명하지 못한다 (`parse`가 `anomalies`를 남기는 것과 같은 태도다).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatBlocking {
    /// 살아남은 segment들. 순서는 그대로다.
    pub segments: Vec<TranscriptSegment>,
    /// 연속 반복으로 지워진 segment의 개수.
    pub removed_count: usize,
}

/// ① **어디서 자르는가** — 전체 프레임 수를 청크 구간 목록으로 만든다 (ADR-0007 §20.2).
///
/// ```text
/// L = CHUNK_SECONDS × sample_rate_hz          (16 kHz면 1,920,000 프레임)
/// 청크 k = 프레임 [k·L, min((k+1)·L, n))      겹치지 않는다 (§20.4)
/// 오프셋   offset_cs(k) = k × 12,000 센티초
/// ```
///
/// **샘플레이트를 짐작하지 않는다.** 파생 입력은 언제나 16 kHz mono지만
/// ([`super::audio_input::TARGET_SAMPLE_RATE_HZ`]), 그 사실을 아는 것은 부르는 쪽이고 이
/// 모듈은 값으로 받는다.
///
/// 프레임이 하나도 없으면 청크도 없다 — 전사할 것이 없는데 빈 청크를 지어내지 않는다.
/// 전체가 `CHUNK_SECONDS` 이하면 청크는 하나이며, **지금과 완전히 같은 실행**이 된다.
///
/// 실패는 두 가지다: 샘플레이트가 0이거나(프레임을 시각으로 말할 수 없다), 오프셋이 `i64`를
/// 넘치는 경우다. 어떤 입력으로도 panic하지 않는다.
pub fn plan(total_frames: usize, sample_rate_hz: u32) -> Result<Vec<AudioChunk>, Failure> {
    if sample_rate_hz == 0 {
        return Err(Failure::permanent(
            FailureKind::InvalidInput,
            "전사할 오디오의 샘플레이트가 0이라 청크를 나눌 수 없다",
        )
        .with_detail("sample_rate_hz=0: 프레임을 시각으로 말할 수 없다".to_owned()));
    }

    // 64-bit에서 120 × 4,294,967,295가 usize에 들어간다. 실제로 오는 값은 16,000이다.
    let frames_per_chunk = CHUNK_SECONDS as usize * sample_rate_hz as usize;

    let mut chunks: Vec<AudioChunk> = Vec::new();
    let mut start_frame: usize = 0;
    let mut index: usize = 0;

    while start_frame < total_frames {
        // 마지막 청크는 남은 만큼이다. 무음으로 채우지 않는다 (§20.2).
        let frame_count = frames_per_chunk.min(total_frames - start_frame);

        chunks.push(AudioChunk {
            start_frame,
            frame_count,
            offset_centiseconds: offset_centiseconds(index)?,
        });

        // 겹치지 않으므로 다음 청크는 이 청크가 끝난 자리에서 맞닿아 시작한다 (§20.4).
        start_frame += frame_count - CHUNK_OVERLAP_FRAMES;
        index += 1;
    }

    Ok(chunks)
}

/// ② **청크 로컬 timestamp를 전체 시간축으로** — 오프셋을 더한다 (ADR-0007 §20.5).
///
/// **단위 변환을 하지 않는다. 더하기만 한다** — 센티초 + 센티초 = 센티초다. 텍스트는 손대지
/// 않는다. 엔진이 텍스트를 주지 못한 segment(`text: None`)도 그대로 지나간다 — 그것을 어떻게
/// 할지는 `parse`가 정한 자리가 이미 있다 ([`super::parse::AnomalyKind::TextMissing`]).
///
/// 합이 `i64`를 넘치면 **접지 않고 실패로 나간다** (§20.5).
pub fn shift(segments: Vec<RawSegment>, offset_centiseconds: i64) -> Result<Vec<RawSegment>, Failure> {
    let mut shifted: Vec<RawSegment> = Vec::with_capacity(segments.len());

    for (index, segment) in segments.into_iter().enumerate() {
        shifted.push(RawSegment {
            start_centiseconds: add_offset(segment.start_centiseconds, offset_centiseconds, index, "start")?,
            end_centiseconds: add_offset(segment.end_centiseconds, offset_centiseconds, index, "end")?,
            text: segment.text,
        });
    }

    Ok(shifted)
}

/// ② **여러 청크의 결과를 하나로** (ADR-0007 §20.7).
///
/// ```text
/// segment    청크 순서대로 이어 붙인다. 순서를 다시 정렬하지 않는다
/// language   값을 보고한 첫 청크의 값. 아무 청크도 보고하지 않았으면 None
/// ```
///
/// **첫 값이 이기는 이유:** 결정적이고, **엔진이 말한 적 없는 값을 만들지 않는다.** 다수결은
/// 엔진 중 누구도 말하지 않은 집계값을 새로 만들고 동점 규칙까지 요구하는데, 그것을 정할
/// 관측이 하나도 없다.
///
/// 사용자가 언어를 골랐어도 **엔진이 아무 말도 하지 않으면 `language`는 비어 있다** —
/// 설정 값은 이 함수에 도달하지 않는다 (§17.1.4-3).
pub fn merge(chunks: Vec<ChunkTranscription>) -> Result<RawTranscription, Failure> {
    let mut language: Option<String> = None;
    let mut segments: Vec<RawSegment> = Vec::new();

    for chunk in chunks {
        if language.is_none() {
            language = chunk.output.language;
        }
        segments.extend(shift(chunk.output.segments, chunk.offset_centiseconds)?);
    }

    Ok(RawTranscription { language, segments })
}

/// ③ **연속 반복을 차단한다** (ADR-0007 §20.6.1).
///
/// ```text
/// 문장   collapse::sentence_key 와 같은 정규화다 — 두 번째 정의를 만들지 않고 그 함수를 부른다
/// 연속   이어진 segment들의 문장이 같으면 한 묶음이다. 다른 문장이 하나라도 끼면 묶음이
///        끊기고 세는 값이 1로 돌아간다
/// 차단   한 묶음에서 MAX_CONSECUTIVE_REPEATS 번째까지 남기고 그다음부터 버린다
/// ```
///
/// **떨어져서 다시 나오는 같은 문장은 지우지 않는다** — 그것은 반복이지 연속이 아니다.
///
/// **문장이 없는 segment(공백뿐인 텍스트)는 세지도 지우지도 않으며, 묶음을 끊는다.**
/// `collapse`가 빈 문장을 분모에 넣지 않는 것과 같은 태도다 — 있지도 않은 문장을 반복으로
/// 세지 않는다.
///
/// **이 함수는 붕괴 판정 뒤에 온다** (§20.6.2). 차단을 판정보다 먼저 두면 §18이 세는 증거를
/// 정확히 지워 **붕괴한 전사가 통과한다.** 차단은 안전망을 대체하지 않는다.
pub fn block_consecutive_repeats(segments: &[TranscriptSegment]) -> RepeatBlocking {
    let mut kept: Vec<TranscriptSegment> = Vec::with_capacity(segments.len());
    let mut removed_count: usize = 0;

    // 지금 이어지고 있는 묶음의 문장과 그 길이. 문장이 없는 segment는 묶음을 끊는다.
    let mut run_sentence: Option<String> = None;
    let mut run_length: usize = 0;

    for segment in segments {
        let sentence = sentence_key(&segment.text);

        if sentence.is_empty() {
            run_sentence = None;
            run_length = 0;
            kept.push(segment.clone());
            continue;
        }

        if run_sentence.as_deref() == Some(sentence.as_str()) {
            run_length += 1;
        } else {
            run_sentence = Some(sentence);
            run_length = 1;
        }

        if run_length > MAX_CONSECUTIVE_REPEATS {
            removed_count += 1;
            continue;
        }

        kept.push(segment.clone());
    }

    RepeatBlocking {
        segments: kept,
        removed_count,
    }
}

/// 청크 번호 하나의 오프셋. **청크 길이에서만 나온다** (ADR-0007 §20.5).
///
/// `CHUNK_SECONDS`가 정수 초이므로 `120 × 100 = 12,000`이 나머지 없이 떨어진다 —
/// 나눗셈도 실수 연산도 없다.
fn offset_centiseconds(index: usize) -> Result<i64, Failure> {
    let per_chunk = i64::from(CHUNK_SECONDS) * i64::from(CENTISECONDS_PER_SECOND);

    i64::try_from(index)
        .ok()
        .and_then(|k| k.checked_mul(per_chunk))
        .ok_or_else(|| {
            Failure::permanent(
                FailureKind::InvalidInput,
                "전사할 오디오가 길어 청크 오프셋이 다룰 수 있는 범위를 넘었다",
            )
            .with_detail(format!(
                "chunk[{index}] offset = {index} × {per_chunk}cs overflows i64"
            ))
        })
}

/// 원시 센티초 하나에 오프셋을 더한다. **같은 단위끼리의 덧셈이다** (ADR-0007 §20.5).
///
/// 넘치면 값을 접지 않고 실패로 나간다 — saturate한 시각은 조용히 틀린 채로 영구히 저장된다.
fn add_offset(
    centiseconds: i64,
    offset_centiseconds: i64,
    index: usize,
    field: &str,
) -> Result<i64, Failure> {
    centiseconds.checked_add(offset_centiseconds).ok_or_else(|| {
        Failure::permanent(
            FailureKind::InvalidInput,
            "전사 결과의 시간 값이 다룰 수 있는 범위를 넘었다",
        )
        .with_detail(format!(
            "segment[{index}].{field}={centiseconds}cs + {offset_centiseconds}cs overflows i64"
        ))
    })
}

/// ④ **잘린 글자를 이어 붙이며 디코딩한다** (2026-09-07 실사용 수정).
///
/// # 무엇을 고치는가
///
/// whisper.cpp는 segment 경계를 **바이트가 아니라 토큰**으로 정한다. 한글은 UTF-8로 글자당
/// 3바이트이고 BPE 토큰은 그 3바이트를 쪼개 놓으므로, 경계가 글자 한가운데에 떨어지면
/// **앞 segment는 끝이 잘리고 뒤 segment는 이어지는 바이트로 시작한다.**
///
/// ```text
/// segment k    "… 그리고 얘"  + [EC 96]      ← 3바이트 글자의 앞 2바이트
/// segment k+1  [B0] + "에서 …"               ← 나머지 1바이트로 시작한다
/// ```
///
/// 2026-09-07 실행이 여기서 죽었다 — `chunk 32/37: segment 2/14: Invalid UTF-8 detected in a
/// string from Whisper. Index: 0, Length: 1`. 그때까지 만든 31개 청크가 함께 버려졌다.
///
/// # 왜 lossy가 답이 아닌가
///
/// 바인딩이 주는 lossy 변환은 잘린 바이트를 `U+FFFD`로 바꾼다. 그러면
/// **경계마다 글자가 하나씩 사라진다** — 72분 녹음에서 그 경계는 수백 곳이다. 여기서는 잘린
/// 바이트를 버리지 않고 **다음 segment의 앞으로 넘겨** 글자를 복원한다. 잃는 글자가 없다.
///
/// # 규칙
///
/// ```text
/// 이어붙임   앞 segment에서 남은 바이트(carry)를 다음 segment 앞에 붙인 뒤 디코딩한다
/// 끝이 잘림  UTF-8로 완성되지 않은 **꼬리**만 carry로 남긴다 (error_len() == None)
/// 진짜 오류  UTF-8로 성립할 수 없는 바이트는 U+FFFD로 바꾸고 넘어간다 (error_len() == Some)
/// 마지막     마지막 segment의 carry는 넘길 곳이 없다 — U+FFFD로 바꿔 흘린다
/// ```
///
/// **시각은 건드리지 않는다.** 넘어간 바이트가 만드는 글자는 뒤 segment의 것이 되며, 두
/// segment의 timestamp는 엔진이 준 값 그대로다. 이 함수는 텍스트만 만든다.
pub fn decode_segment_texts(raw: &[Vec<u8>]) -> Vec<String> {
    let mut texts = Vec::with_capacity(raw.len());
    let mut carry: Vec<u8> = Vec::new();

    for (index, bytes) in raw.iter().enumerate() {
        let mut buffer = std::mem::take(&mut carry);
        buffer.extend_from_slice(bytes);

        let mut text = String::new();
        let mut rest = buffer.as_slice();

        loop {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    text.push_str(valid);
                    rest = &[];
                    break;
                }
                Err(error) => {
                    let valid_up_to = error.valid_up_to();
                    // SAFETY 아님 — 검증된 앞부분이다. `from_utf8`이 그 길이를 보장한다.
                    if let Ok(valid) = std::str::from_utf8(&rest[..valid_up_to]) {
                        text.push_str(valid);
                    }
                    match error.error_len() {
                        // 꼬리가 잘린 것이다. 다음 segment 앞에 붙인다.
                        None => {
                            carry = rest[valid_up_to..].to_vec();
                            rest = &[];
                            break;
                        }
                        // UTF-8로 성립할 수 없는 바이트다. 그것만 버리고 계속 읽는다.
                        Some(bad) => {
                            text.push(char::REPLACEMENT_CHARACTER);
                            rest = &rest[valid_up_to + bad..];
                        }
                    }
                }
            }
        }

        // 마지막 segment의 꼬리는 넘길 곳이 없다. 조용히 사라지게 두지 않는다.
        let is_last = index + 1 == raw.len();
        if is_last && !carry.is_empty() {
            text.push(char::REPLACEMENT_CHARACTER);
            carry.clear();
        }

        texts.push(text);
    }

    texts
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 파생 입력의 샘플레이트. 값은 `audio_input`의 것과 같지만, 이 모듈은 그것을 **받는다**.
    const SAMPLE_RATE: u32 = 16_000;
    /// 16 kHz에서 청크 하나의 프레임 수 — 120 × 16,000.
    const L: usize = 1_920_000;
    /// 청크 하나의 오프셋 — 120초 × 100.
    const OFFSET_STEP: i64 = 12_000;

    fn planned(total_frames: usize) -> Vec<AudioChunk> {
        plan(total_frames, SAMPLE_RATE).expect("16 kHz에서 청크 나누기는 실패하지 않는다")
    }

    fn raw(start_centiseconds: i64, end_centiseconds: i64, text: &str) -> RawSegment {
        RawSegment {
            start_centiseconds,
            end_centiseconds,
            text: Some(text.to_owned()),
        }
    }

    fn chunk_output(offset_centiseconds: i64, segments: Vec<RawSegment>) -> ChunkTranscription {
        ChunkTranscription {
            offset_centiseconds,
            output: RawTranscription {
                language: None,
                segments,
            },
        }
    }

    fn normalized(texts: &[&str]) -> Vec<TranscriptSegment> {
        texts
            .iter()
            .enumerate()
            .map(|(index, text)| {
                let start_ms = index as i64 * 30_000;
                TranscriptSegment {
                    start_ms,
                    end_ms: start_ms + 29_980,
                    text: (*text).to_owned(),
                }
            })
            .collect()
    }

    fn texts(segments: &[TranscriptSegment]) -> Vec<&str> {
        segments.iter().map(|s| s.text.as_str()).collect()
    }

    // ── ① 어디서 자르는가 ────────────────────────────────────────────────

    #[test]
    fn no_samples_means_no_chunks() {
        // 전사할 것이 없는데 빈 청크를 지어내지 않는다.
        assert!(planned(0).is_empty());
    }

    #[test]
    fn audio_shorter_than_one_chunk_is_a_single_chunk_of_its_own_length() {
        // 1초짜리 녹음. 무음으로 채우지 않으므로 길이는 받은 그대로다 (§20.2).
        let chunks = planned(SAMPLE_RATE as usize);

        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0],
            AudioChunk {
                start_frame: 0,
                frame_count: 16_000,
                offset_centiseconds: 0,
            },
            "전체가 120초 이하면 청크는 하나이며 지금과 완전히 같은 실행이 된다"
        );
    }

    #[test]
    fn exactly_one_chunk_length_is_one_chunk() {
        let chunks = planned(L);

        assert_eq!(chunks.len(), 1, "정확히 L이면 청크는 하나다 — 빈 두 번째 청크가 붙지 않는다");
        assert_eq!(chunks[0].frame_count, L);
        assert_eq!(chunks[0].end_frame(), L);
    }

    #[test]
    fn an_exact_multiple_of_the_chunk_length_has_no_short_last_chunk() {
        let chunks = planned(3 * L);

        assert_eq!(chunks.len(), 3);
        for (index, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.frame_count, L, "청크 {index}는 온전한 길이다");
            assert_eq!(chunk.start_frame, index * L);
        }
        assert_eq!(chunks[2].end_frame(), 3 * L, "마지막 프레임까지 덮는다");
    }

    #[test]
    fn one_frame_past_a_multiple_makes_a_short_last_chunk() {
        // 배수 + 1 프레임. 남은 1프레임은 앞 청크에 합쳐지지도, 무음으로 채워지지도 않는다.
        let chunks = planned(2 * L + 1);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].frame_count, L);
        assert_eq!(chunks[1].frame_count, L);
        assert_eq!(chunks[2].frame_count, 1, "마지막 청크는 남은 만큼이다 (§20.2)");
        assert_eq!(chunks[2].start_frame, 2 * L);
        assert_eq!(chunks[2].offset_centiseconds, 2 * OFFSET_STEP);
    }

    #[test]
    fn chunks_are_adjacent_and_no_frame_is_used_twice() {
        // 겹침이 0이라는 것이 실제로 구간에 나타난다 (§20.4).
        assert_eq!(CHUNK_OVERLAP_FRAMES, 0);

        let total = 2 * L + 7;
        let chunks = planned(total);

        assert_eq!(chunks[0].start_frame, 0, "첫 프레임에서 시작한다");
        for pair in chunks.windows(2) {
            assert_eq!(
                pair[0].end_frame(),
                pair[1].start_frame,
                "청크는 맞닿아 있고 겹치지 않는다"
            );
        }
        assert_eq!(
            chunks.iter().map(|chunk| chunk.frame_count).sum::<usize>(),
            total,
            "모든 프레임이 정확히 한 번씩 들어간다"
        );
        assert_eq!(chunks.last().expect("청크가 있다").end_frame(), total);
    }

    #[test]
    fn the_offset_of_chunk_k_is_twelve_thousand_centiseconds_times_k() {
        let chunks = planned(5 * L);

        for (index, chunk) in chunks.iter().enumerate() {
            assert_eq!(
                chunk.offset_centiseconds,
                OFFSET_STEP * index as i64,
                "offset_cs(k) = 12,000 × k (§20.5)"
            );
        }
    }

    #[test]
    fn twelve_thousand_centiseconds_comes_out_of_the_frame_count_without_remainder() {
        // 프레임 → 센티초가 나머지 없이 떨어진다는 정수 관계를 고정한다 (§20.5).
        // 이 관계가 깨지면 청크마다 반올림이 쌓여 경계가 어긋난다.
        let frames_per_chunk = CHUNK_SECONDS as usize * SAMPLE_RATE as usize;
        assert_eq!(frames_per_chunk, L, "120 × 16,000 = 1,920,000");
        assert_eq!(
            frames_per_chunk * CENTISECONDS_PER_SECOND as usize % SAMPLE_RATE as usize,
            0,
            "1,920,000 × 100 / 16,000 은 나머지가 없다"
        );
        assert_eq!(
            frames_per_chunk * CENTISECONDS_PER_SECOND as usize / SAMPLE_RATE as usize,
            OFFSET_STEP as usize,
            "= 12,000 센티초"
        );
    }

    #[test]
    fn the_chunk_range_is_what_the_caller_slices_with() {
        // 이 모듈은 오디오를 들지 않는다. 내는 것은 구간이고 자르는 것은 부르는 쪽이다 (§20.8).
        let buffer: Vec<f32> = vec![0.0; 40];
        let chunks = plan(buffer.len(), 10).expect("10 Hz도 값일 뿐이다");

        assert_eq!(chunks.len(), 1, "10 Hz × 120초 = 1,200 프레임 > 40");
        assert_eq!(chunks[0].range(), 0..40);
        assert_eq!(buffer[chunks[0].range()].len(), 40);
    }

    #[test]
    fn a_zero_sample_rate_is_a_failure_not_a_guess() {
        let failure = plan(1_000, 0).expect_err("샘플레이트 0으로 시각을 말할 수 없다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(failure.source_data_safe, "이 모듈은 파일도 데이터베이스도 건드리지 않는다");
    }

    #[test]
    fn an_offset_that_cannot_be_represented_is_a_failure_not_a_saturated_value() {
        // 실제 오디오로는 닿지 않는 자리지만, 넘침을 조용히 접지 않는다는 규칙은 값으로 고정한다.
        assert_eq!(offset_centiseconds(0).expect("0은 언제나 된다"), 0);
        let failure = offset_centiseconds(usize::MAX).expect_err("표현할 수 없는 오프셋이다");
        assert_eq!(failure.kind, FailureKind::InvalidInput);
    }

    // ── ② 전체 시간축으로 되돌려 잇는다 ──────────────────────────────────

    #[test]
    fn shifting_adds_the_offset_and_changes_nothing_else() {
        let shifted = shift(vec![raw(0, 250, " 안녕하세요")], OFFSET_STEP).expect("넘치지 않는다");

        assert_eq!(
            shifted,
            vec![raw(12_000, 12_250, " 안녕하세요")],
            "더하기만 한다 — 단위도 텍스트도 건드리지 않는다"
        );
    }

    #[test]
    fn the_second_chunk_does_not_rewind_to_the_start_of_the_timeline() {
        // 청크 로컬 timestamp는 청크마다 0부터 다시 시작한다. 오프셋이 더해지지 않으면
        // 두 번째 청크의 시각이 첫 청크 위로 되감긴다 — 그것을 이 검사가 막는다.
        let merged = merge(vec![
            chunk_output(0, vec![raw(0, 300, "첫 청크"), raw(11_700, 12_000, "첫 청크 끝")]),
            chunk_output(OFFSET_STEP, vec![raw(0, 300, "둘째 청크"), raw(500, 800, "둘째 청크 둘")]),
        ])
        .expect("넘치지 않는다");

        let starts: Vec<i64> = merged
            .segments
            .iter()
            .map(|segment| segment.start_centiseconds)
            .collect();

        assert_eq!(starts, vec![0, 11_700, 12_000, 12_500]);
        assert!(
            starts.windows(2).all(|pair| pair[0] <= pair[1]),
            "전체 시간축에서 뒤로 가지 않는다: {starts:?}"
        );
        assert_eq!(merged.segments.len(), 4, "청크 순서대로 이어 붙인다");
        assert_eq!(
            merged.segments[2].end_centiseconds, 12_300,
            "끝 시각에도 같은 오프셋이 더해진다"
        );
    }

    #[test]
    fn a_chunk_that_produced_nothing_does_not_shift_the_chunks_after_it() {
        // 오프셋은 청크 번호에서만 나온다 (§20.8의 "다음 청크의 오프셋이 밀리지 않는다").
        let merged = merge(vec![
            chunk_output(0, vec![raw(10, 300, "첫 청크")]),
            chunk_output(OFFSET_STEP, vec![]),
            chunk_output(2 * OFFSET_STEP, vec![raw(40, 90, "셋째 청크")]),
        ])
        .expect("넘치지 않는다");

        assert_eq!(merged.segments.len(), 2);
        assert_eq!(
            merged.segments[1].start_centiseconds, 24_040,
            "빈 청크가 있어도 셋째 청크의 오프셋은 24,000 그대로다"
        );
    }

    #[test]
    fn segments_without_text_pass_through_the_shift_untouched() {
        // 엔진이 텍스트를 주지 못한 자리를 이 모듈이 지어내지도, 버리지도 않는다 —
        // 그것을 어떻게 할지는 parse가 정한 자리가 이미 있다 (AnomalyKind::TextMissing).
        let merged = merge(vec![chunk_output(
            OFFSET_STEP,
            vec![
                RawSegment {
                    start_centiseconds: 100,
                    end_centiseconds: 200,
                    text: None,
                },
                raw(200, 300, ""),
                raw(300, 400, "말이 있는 구간"),
            ],
        )])
        .expect("넘치지 않는다");

        assert_eq!(merged.segments.len(), 3, "텍스트가 없다고 버리지 않는다");
        assert_eq!(merged.segments[0].text, None, "없음을 그대로 넘긴다");
        assert_eq!(merged.segments[0].start_centiseconds, 12_100, "시각은 그래도 옮긴다");
        assert_eq!(merged.segments[1].text.as_deref(), Some(""), "빈 문자열도 그대로다");
        assert_eq!(merged.segments[2].start_centiseconds, 12_300);
    }

    #[test]
    fn merging_nothing_gives_an_empty_transcription() {
        let merged = merge(Vec::new()).expect("넘치지 않는다");

        assert_eq!(merged.language, None);
        assert!(merged.segments.is_empty());
    }

    #[test]
    fn the_first_reported_language_wins() {
        let merged = merge(vec![
            ChunkTranscription {
                offset_centiseconds: 0,
                output: RawTranscription {
                    language: None,
                    segments: Vec::new(),
                },
            },
            ChunkTranscription {
                offset_centiseconds: OFFSET_STEP,
                output: RawTranscription {
                    language: Some("ko".to_owned()),
                    segments: Vec::new(),
                },
            },
            ChunkTranscription {
                offset_centiseconds: 2 * OFFSET_STEP,
                output: RawTranscription {
                    language: Some("en".to_owned()),
                    segments: Vec::new(),
                },
            },
        ])
        .expect("넘치지 않는다");

        assert_eq!(
            merged.language.as_deref(),
            Some("ko"),
            "값을 보고한 첫 청크가 이긴다 — 다수결도 마지막 값도 아니다 (§20.7)"
        );
    }

    #[test]
    fn a_language_nobody_reported_is_not_invented() {
        let merged = merge(vec![chunk_output(0, vec![raw(0, 100, "말은 있다")])])
            .expect("넘치지 않는다");

        assert_eq!(
            merged.language, None,
            "엔진이 아무 말도 하지 않으면 language는 비어 있다 (§17.1.4-3)"
        );
    }

    #[test]
    fn a_shifted_timestamp_that_cannot_be_represented_is_a_failure() {
        let failure = shift(vec![raw(i64::MAX, i64::MAX, "끝에 닿은 값")], 1)
            .expect_err("표현할 수 없는 시각이다");

        assert_eq!(failure.kind, FailureKind::InvalidInput);
        assert!(
            failure.detail.is_some(),
            "무엇이 넘쳤는지 남는다 — 조용히 접지 않는다"
        );
    }

    // ── ③ 연속 반복을 어디서 끊는가 ──────────────────────────────────────

    #[test]
    fn three_consecutive_repeats_survive() {
        // 임계값 **바로 아래**. MAX_CONSECUTIVE_REPEATS 번째까지는 남는다.
        let segments = normalized(&["네", "네", "네"]);
        assert_eq!(segments.len(), MAX_CONSECUTIVE_REPEATS);

        let blocked = block_consecutive_repeats(&segments);

        assert_eq!(blocked.segments, segments, "3회까지 통과한다 (§20.6.1)");
        assert_eq!(blocked.removed_count, 0);
    }

    #[test]
    fn the_fourth_consecutive_repeat_is_blocked() {
        // 임계값 **바로 위**. 4회째부터 지워지고, 앞의 3개는 남는다.
        let segments = normalized(&["네", "네", "네", "네"]);

        let blocked = block_consecutive_repeats(&segments);

        assert_eq!(texts(&blocked.segments), vec!["네", "네", "네"]);
        assert_eq!(blocked.removed_count, 1, "지워진 개수를 잃지 않는다 (§20.6.2)");
        assert_eq!(
            blocked.segments[0].start_ms, 0,
            "살아남은 segment의 시각은 그대로다"
        );
    }

    #[test]
    fn everything_past_the_third_in_one_run_is_blocked() {
        let segments = normalized(&["같은 말"; 10]);

        let blocked = block_consecutive_repeats(&segments);

        assert_eq!(blocked.segments.len(), MAX_CONSECUTIVE_REPEATS);
        assert_eq!(blocked.removed_count, 7);
    }

    #[test]
    fn a_different_sentence_restarts_the_count() {
        // "다른 문장이 하나라도 끼면 묶음이 끊기고 세는 값이 1로 돌아간다" (§20.6.1).
        let segments = normalized(&["네", "네", "네", "그렇군요", "네", "네", "네"]);

        let blocked = block_consecutive_repeats(&segments);

        assert_eq!(blocked.removed_count, 0, "묶음이 끊겼으므로 지울 것이 없다");
        assert_eq!(blocked.segments.len(), 7);
    }

    #[test]
    fn repeats_that_are_not_consecutive_are_not_blocked() {
        // 떨어져서 다시 나오는 같은 문장은 반복이지 연속이 아니다 — 지우지 않는다.
        // 2026-09-05의 실행에서 최다 반복이 10회로 남은 것이 이 규칙의 결과다 [E5].
        let alternating: Vec<&str> = (0..20)
            .map(|index| if index % 2 == 0 { "네" } else { "아니요" })
            .collect();
        let segments = normalized(&alternating);

        let blocked = block_consecutive_repeats(&segments);

        assert_eq!(blocked.removed_count, 0);
        assert_eq!(
            blocked.segments.len(),
            20,
            "같은 문장이 10번 나왔지만 연속이 아니다"
        );
    }

    #[test]
    fn sentences_that_differ_only_in_whitespace_are_the_same_sentence() {
        // 문장 비교 규칙은 collapse::sentence_key 하나다 — 두 번째 정의를 만들지 않는다.
        let segments = normalized(&[
            "한글자막 by 한효정",
            "  한글자막 by 한효정  ",
            "한글자막  by  한효정",
            "한글자막\tby\n한효정",
        ]);

        let blocked = block_consecutive_repeats(&segments);

        assert_eq!(blocked.removed_count, 1, "네 표기는 같은 문장이므로 4번째가 지워진다");
        assert_eq!(
            texts(&blocked.segments),
            vec!["한글자막 by 한효정", "  한글자막 by 한효정  ", "한글자막  by  한효정"],
            "살아남은 텍스트는 정규화되지 않는다 — 비교에만 쓴 규칙이다"
        );
    }

    #[test]
    fn segments_without_a_sentence_are_neither_counted_nor_blocked() {
        // 있지도 않은 문장을 반복으로 세지 않는다 (collapse가 빈 문장을 분모에 넣지 않는 것과
        // 같은 태도다). 그리고 그것이 끼면 묶음이 끊긴다.
        let segments = normalized(&["네", "네", "네", "   ", "네", "네", "네", "네"]);

        let blocked = block_consecutive_repeats(&segments);

        assert_eq!(
            texts(&blocked.segments),
            vec!["네", "네", "네", "   ", "네", "네", "네"],
            "공백뿐인 segment는 지워지지 않고, 그 뒤의 묶음은 1부터 다시 센다"
        );
        assert_eq!(blocked.removed_count, 1, "다시 센 묶음의 4번째만 지워진다");
    }

    #[test]
    fn many_blank_segments_in_a_row_are_all_kept() {
        let segments = normalized(&["", "  ", "\t", "\n", ""]);

        let blocked = block_consecutive_repeats(&segments);

        assert_eq!(blocked.segments.len(), 5, "문장이 아닌 것을 연속 반복으로 세지 않는다");
        assert_eq!(blocked.removed_count, 0);
    }

    #[test]
    fn blocking_nothing_gives_nothing() {
        let blocked = block_consecutive_repeats(&[]);

        assert!(blocked.segments.is_empty());
        assert_eq!(blocked.removed_count, 0);
    }

    // ── 이 모듈이 무엇을 모르는가 ────────────────────────────────────────

    #[test]
    fn this_module_does_not_know_the_outside_world() {
        // 규칙이 파일 · 데이터베이스 · 네트워크 · 시계 · 엔진을 알기 시작하면 실제 whisper
        // 없이 값으로 검증할 수 없게 된다 (ADR-0007 §20.8).
        // needle을 이어 붙여 만드는 것은 이 검사가 자기 자신에 걸리지 않게 하기 위해서다
        // (collapse.rs · parse.rs의 같은 검사와 같은 방법이다).
        let production = production_source();
        let forbidden = [
            ["use ", "std::fs"].concat(),
            ["std::", "process"].concat(),
            ["Command", "::new"].concat(),
            ["rusqlite", "::"].concat(),
            ["whisper", "_rs"].concat(),
            ["Whisper", "Context"].concat(),
            ["crate::", "db"].concat(),
            ["crate::", "platform"].concat(),
            ["crate::", "notion"].concat(),
            ["super::", "engine"].concat(),
            ["super::", "whisper"].concat(),
            ["super::", "run"].concat(),
            ["super::", "model"].concat(),
            ["Instant", "::now"].concat(),
            ["SystemTime", "::now"].concat(),
            ["reqwest", "::"].concat(),
        ];

        for needle in forbidden {
            assert!(
                !production.contains(&needle),
                "청킹 모듈에 바깥 세계가 들어왔다: {needle}"
            );
        }

        // 이 파일이 실제로 읽혔는지 확인한다 — 빈 문자열이면 위 검사는 아무것도 막지 못한다.
        assert!(production.contains("MAX_CONSECUTIVE_REPEATS"));
    }

    #[test]
    fn the_three_values_live_in_exactly_one_place() {
        // 값이 두 자리에 있으면 한쪽만 고쳐지는 날이 온다 (ADR-0007 §20.8).
        let production = production_source();
        let definitions = [
            "pub const CHUNK_SECONDS: u32 = 120;",
            "pub const CHUNK_OVERLAP_FRAMES: usize = 0;",
            "pub const MAX_CONSECUTIVE_REPEATS: usize = 3;",
        ];
        for definition in definitions {
            assert_eq!(
                production.matches(definition).count(),
                1,
                "이 값은 상수 한 자리에만 있다: {definition}"
            );
        }

        // 그리고 그 값들이 엔진 · 실행 경로 · 판정 쪽에 흩어져 있지 않다.
        // (`whisper.rs`가 이 모듈을 **부르는** 것은 복제가 아니다 — 여기서 막는 것은 숫자의 복제다.)
        for (name, source) in [
            ("run.rs", include_str!("run.rs")),
            ("engine.rs", include_str!("engine.rs")),
            ("whisper.rs", include_str!("whisper.rs")),
            ("parse.rs", include_str!("parse.rs")),
            ("collapse.rs", include_str!("collapse.rs")),
        ] {
            for literal in ["CHUNK_SECONDS:", "CHUNK_OVERLAP_FRAMES:", "MAX_CONSECUTIVE_REPEATS:", "1_920_000", "12_000"] {
                assert!(
                    !source.contains(literal),
                    "{name}에 청킹 값이 복제됐다: {literal}"
                );
            }
        }
    }

    /// 이 파일에서 테스트를 뺀 부분. 검사가 테스트 코드 자신에 걸리지 않게 한다.
    fn production_source() -> &'static str {
        include_str!("chunking.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("테스트 앞의 코드가 있어야 한다")
    }
}

#[cfg(test)]
mod repair_tests {
    use super::*;

    // ── ④ 잘린 UTF-8을 이어 붙인다 ──────────────────────────────────────────

    #[test]
    fn a_korean_character_split_across_two_segments_is_restored() {
        // "얘"는 EC 96 98이다. 앞 segment가 두 바이트에서 잘리고 뒤가 나머지로 시작한다.
        let mut first = " 그리고".as_bytes().to_vec();
        first.extend_from_slice(&[0xEC, 0x96]);
        let mut second = vec![0x98];
        second.extend_from_slice("에서".as_bytes());

        let texts = decode_segment_texts(&[first, second]);

        assert_eq!(texts, vec![" 그리고".to_owned(), "얘에서".to_owned()]);
        assert!(!texts.concat().contains(char::REPLACEMENT_CHARACTER));
    }

    #[test]
    fn the_2026_09_07_crash_input_no_longer_fails() {
        // `chunk 32/37: segment 2/14: Invalid UTF-8 ... Index: 0, Length: 1`이 난 모양이다 —
        // segment가 이어지는 바이트 하나로 시작했다. 이제 실패가 아니라 값이 나온다.
        let texts = decode_segment_texts(&[vec![0xEA, 0xB0], vec![0x80, b'!']]);
        assert_eq!(texts, vec![String::new(), "가!".to_owned()]);
    }

    #[test]
    fn genuinely_invalid_bytes_become_one_replacement_and_reading_continues() {
        // UTF-8로 성립할 수 없는 바이트다. 그것만 바꾸고 뒤를 계속 읽는다.
        let texts = decode_segment_texts(&[vec![b'a', 0xFF, b'b']]);
        assert_eq!(texts, vec![format!("a{}b", char::REPLACEMENT_CHARACTER)]);
    }

    #[test]
    fn a_dangling_tail_on_the_last_segment_is_not_lost_silently() {
        // 넘길 곳이 없는 꼬리다. 조용히 사라지지 않고 자리를 남긴다.
        let texts = decode_segment_texts(&[vec![b'x', 0xEC, 0x96]]);
        assert_eq!(texts, vec![format!("x{}", char::REPLACEMENT_CHARACTER)]);
    }

    #[test]
    fn clean_input_passes_through_untouched() {
        let texts = decode_segment_texts(&[
            "안녕하세요".as_bytes().to_vec(),
            " 반갑습니다".as_bytes().to_vec(),
        ]);
        assert_eq!(texts, vec!["안녕하세요".to_owned(), " 반갑습니다".to_owned()]);
    }

    #[test]
    fn an_empty_sequence_produces_nothing() {
        assert!(decode_segment_texts(&[]).is_empty());
    }
}
