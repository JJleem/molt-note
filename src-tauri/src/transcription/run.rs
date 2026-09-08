//! Recording 하나를 전사해서 Transcript로 남기는 **실행 순서** (PRODUCT-SPEC §7 · §7.1 · §7.2).
//!
//! 앞의 모듈들이 각자 한 조각씩 맡았고, 그것들을 잇는 자리는 여기 하나다.
//!
//! ```text
//! Recording 레코드 ─→ audio_input ─→ engine ─→ parse ─→ collapse ─→ append_transcript ─→ set_current
//!  (audio_path)       파생 입력       원시 출력   밀리초    저장 직전      §7.1 · INV-2         §7.2
//!        │                                                 (ADR-0007 §18.5)
//!        └─ transcription_status:  pending ─→ running ─→ done
//!                                                   └─→ failed  (current는 그대로)
//! ```
//!
//! ## 저장 직전에 두 가지를 묻는다 (ADR-0007 §18.5)
//!
//! ```text
//! 정규화된 결과 ─→ ┌ 빈 결과인가 (n == 0)        ─→ 실패, 저장하지 않는다
//!                  ├ 붕괴했는가 (§18.2의 두 조건) ─→ 실패, 저장하지 않는다
//!                  └ 그 밖                       ─→ 저장한다
//! ```
//!
//! 붕괴 판정은 빈 결과 판정을 **대체하지 않고 더해진다.** 2026-09-07에 51분짜리 회의가
//! 고유 문장 2개(한 문장이 99.0%)로 전사됐고 제품은 그것을 `done`으로 저장했다 — 이 자리가
//! 물었던 것이 개수 하나뿐이었기 때문이다 (ADR-0007 §18.1).
//!
//! **판정 규칙은 여기 없다.** 무엇을 붕괴로 볼 것인가(임계값 · 최소 개수 · 비율)는 전부
//! [`collapse`]가 정하며, 이 모듈은 그 판정과 그 판정이 낸 수치를 받아 **저장을 막고 실패를
//! 만드는 일**만 한다 (ADR-0007 §18.3 · §18.4). 실패 종류도 새로 만들지 않는다 — 이미 있는
//! [`output_unusable`]이다.
//!
//! ## 재전사는 덮어쓰기가 아니라 추가다 (§7.1 · INV-2)
//!
//! 성공할 때마다 **새 Transcript가 하나 늘어난다.** 이 모듈에 기존 Transcript를 고치거나
//! 지우는 경로는 없고, 만들 수도 없다 — 저장소가 내놓는 것이 [`store::append_transcript`]
//! 하나이기 때문이다 (`crate::db::store`의 모듈 문서). 새 것을 current로 올리는 것은
//! 그다음이며 ([`store::set_current_transcript`] · §7.2), 그 순서 덕분에 화면이 `done`을
//! 본 시점에는 current가 이미 새 Transcript를 가리킨다.
//!
//! ## 실패는 아무것도 잃지 않는다 (INV-1 · INV-3 · §13)
//!
//! ```text
//! Transcript A = success / current
//!         ↓
//! 재전사 시도  →  실패 (엔진 실패 · 모델 없음 · 빈 결과 · 붕괴)
//!         ↓
//! current = Transcript A      그대로
//! Transcript A의 segments      그대로
//! 원본 오디오 파일             그대로
//! Recording 레코드             transcription_status와 updated_at 말고는 그대로
//! ```
//!
//! 실패 경로가 [`store::set_current_transcript`]를 부르지 않기 때문에 current가 유지되고,
//! 이 모듈 어디에도 파일을 쓰거나 지우는 코드가 없기 때문에 원본이 유지된다. 파생 입력은
//! 메모리 위의 버퍼 하나이므로([`audio_input::TranscriptionInput`] · ADR-0007 §9.1) 정리는
//! `drop`이 전부이고, **실패할 수 있는 정리 절차가 없다** — 정리가 전사 성공을 되돌리는
//! 경로 자체가 생기지 않는다.
//!
//! 그래서 같은 Recording에 대해 몇 번이든 다시 시도할 수 있다. 이 모듈은 직전 시도의 결과를
//! 상태로 갖지 않는다.
//!
//! ## 여기서 하지 않는 것
//!
//! 스레드를 만들지 않고, Tauri command를 열지 않으며, 어떤 모델을 쓸지도 무슨 언어로 들을지도
//! 설정에서 읽지 **않는다** — 설정을 읽어 [`ModelChoice`]와 [`LanguageChoice`]로 넘기는 것은
//! 부르는 쪽의 일이다. 이 함수를 배경 스레드에서 부르고 그 상태를 화면에 여는 자리는
//! [`crate::commands::Transcriber`]다. 모델 위치를 아는 코드는 여전히 [`model`] 하나뿐이고
//! (INV-10) 언어를 실제 엔진 호출로 옮기는 자리는 [`super::whisper`] 하나이므로
//! (ADR-0007 §17.1.4-4), 이 모듈은 두 값을 **받아서 넘기기만** 한다.

use std::path::Path;
use std::time::Instant;

use rusqlite::Connection;

use crate::db::store;
use crate::domain::{
    Failure, FailureKind, ProcessingStatus, Recording, RecordingId, Transcript, TranscriptId,
    TranscriptSegment,
};

use super::audio_input;
use super::collapse::{self, CollapseAssessment, CollapseVerdict};
// 반복 차단은 전사 모듈의 공개 표면에서 온다 — 그 규칙이 어느 파일에 사는지 이 모듈은 모른다.
use super::gain;
use super::{block_consecutive_repeats, MAX_CONSECUTIVE_REPEATS};
use super::engine::{output_unusable, LanguageChoice, TranscriptionEngine};
use super::model;
use super::parse::{self, Anomaly};

/// 어떤 모델로 전사할지 정하는 두 값.
///
/// 경로를 짓지 않는다 — 이 두 값을 실제 파일 하나로 해석하는 것은 [`model::resolve`]이며,
/// 그 자리는 코드 전체에 하나다 (INV-10 · ADR-0007 §8.2).
#[derive(Debug, Clone, Copy)]
pub struct ModelChoice<'a> {
    /// 모델 디렉터리. `crate::platform::app_data_dir::AppDataDirectory::models_dir`에서 온다.
    pub models_dir: &'a Path,
    /// 사용자가 고른 값(파일명 또는 절대 경로). 아직 고르지 않았으면 `None`이다.
    pub configured: Option<&'a str>,
}

/// 성공한 전사 한 건의 결과.
///
/// `Eq`가 아닌 것은 [`gain::Applied`]가 dBFS를 `f64`로 갖기 때문이다 — 잰 값이지
/// 동치를 따질 값이 아니다.
#[derive(Debug, Clone, PartialEq)]
pub struct Completed {
    /// 방금 **추가된** Transcript. 이 시점에 이미 current다 (§7.2).
    pub transcript: Transcript,
    /// 엔진의 원시 출력이 기대와 달랐던 자리들 ([`parse::Transcription::anomalies`]).
    ///
    /// 저장하지 않고 호출자에게 돌려준다 — 저장할 자리가 없다는 이유로 **버리지는 않는다.**
    pub anomalies: Vec<Anomaly>,
    /// 연속 반복으로 **통째로 버려진** segment 수 (ADR-0007 §20.6).
    ///
    /// [`collapse::assess`]가 본 수치는 **차단 전** 열의 것이므로 저장된 segment 수보다 크다.
    /// 그 차이가 이 값이다 — 남기지 않으면 다음 사람이 두 수치의 차이를 설명하지 못한다.
    pub removed_segments: usize,
    /// 전사 입력을 얼마나 키웠는가. 키우지 않았으면 `gain_db`가 `0.0`이다.
    ///
    /// **값으로 남겨야 비교할 수 있다** — 같은 녹음을 다시 돌렸을 때 무엇이 달랐는지
    /// 말할 수 있어야 하고, 2026-09-08의 실측이 정확히 그 비교였다.
    pub gain: gain::Applied,
    /// segment **안쪽**의 되풀이를 줄여서 텍스트가 짧아진 segment 수 (§20.6.1의 빈자리).
    pub shortened_segments: usize,
}

/// Recording 하나를 전사하고 결과를 영속화한다.
///
/// 상태는 실제로 세 번(또는 두 번 + 실패) 저장된다 — `pending` · `running` 다음에 `done`
/// 또는 `failed`다. 중간 상태를 건너뛰지 않는 이유는 그것이 화면이 읽는 값이기 때문이다
/// (`phase-prompt/03` 요구 3): 전사가 도는 동안 목록과 상세가 `running`을 볼 수 있어야 한다.
///
/// 다른 후처리 상태(`ai_status` · `notion_status`)는 읽은 그대로 다시 쓴다. 전사가 남의
/// 파이프라인 상태를 옮기지 않는다.
///
/// 실패해도 원본 오디오와 Recording 레코드는 그대로다 (INV-1 · INV-3). 바뀌는 것은
/// `transcription_status`와 `updated_at`뿐이며, 그 둘이 바뀌는 것이 §13이 요구하는
/// "실패가 사용자에게 보인다"의 저장 형태다.
///
/// 언어 선택([`LanguageChoice`])은 이 모듈이 해석하지 않는다. 모델 값이 [`model::resolve`]로
/// 가는 것처럼 그대로 엔진 경계로 지나가며, 저장되는 `language`는 여전히 **엔진이 보고한
/// 값**이다 (§7 · ADR-0007 §17.1.4-3).
pub fn transcribe(
    connection: &mut Connection,
    recording_id: &RecordingId,
    engine: &dyn TranscriptionEngine,
    model_choice: ModelChoice<'_>,
    language: &LanguageChoice,
) -> Result<Completed, Failure> {
    // 상태를 쓸 대상이 실재하는지부터 본다. 없는 Recording에 대해서는 아무것도 쓰지 않는다.
    let recording = store::load_recording(connection, recording_id)?
        .ok_or_else(|| unknown_recording(recording_id))?;

    // 접수(`pending`)와 실행 시작(`running`)을 둘 다 남긴다. 이 두 번의 쓰기가 실패하면
    // 그대로 나간다 — 상태를 남기지 못한 채 전사를 시작하면 화면이 진행 중인 일을 볼 수 없다.
    mark(connection, &recording, ProcessingStatus::Pending)?;
    mark(connection, &recording, ProcessingStatus::Running)?;

    attempt(connection, &recording, engine, model_choice, language)
        .map_err(|failure| record_failure(connection, &recording, failure))
}

/// 전사 한 번. 여기서 나오는 모든 실패는 호출자가 `failed`로 기록한다.
fn attempt(
    connection: &mut Connection,
    recording: &Recording,
    engine: &dyn TranscriptionEngine,
    model_choice: ModelChoice<'_>,
    language: &LanguageChoice,
) -> Result<Completed, Failure> {
    // **걸린 시간을 재는 자리는 여기 하나다** (`phase-prompt/05.6` 성공 기준 3). 재는 것은
    // 오디오를 문장으로 옮기는 일 전체이며 — 모델을 찾고, 읽고, 엔진을 돌리고, 출력을
    // 정규화하는 데까지다 — 그 뒤의 영속화는 세지 않는다. 저장이 느린 것은 전사가 느린 것이
    // 아니고, 사람이 Metal 전후로 비교하려는 것은 전사 쪽이다.
    //
    // `Instant`는 **단조 시계다.** 시스템 시각이 도중에 조정돼도 값이 뒤로 가지 않는다 —
    // 벽시계 두 번을 빼서 재면 음수 소요 시간이 저장될 수 있다.
    let started = Instant::now();

    // 모델 없음도 여기서 나온다 — 조용한 skip이 아니라 §13의 실패이며, 그래서 아래의
    // `failed` 기록을 거쳐 사용자에게 도달한다.
    let model = model::resolve(model_choice.models_dir, model_choice.configured)?;

    // 원본은 읽기 전용으로만 열린다 ([`audio_input::load`] · INV-1).
    let mut input = audio_input::load(Path::new(&recording.audio_path))?;

    // **낮게 녹음된 오디오를 여기서 키운다** (`super::gain` · 2026-09-08의 실측).
    //
    // 만지는 것은 방금 메모리에 만든 파생 버퍼뿐이며 **녹음 파일은 손대지 않는다** (INV-1).
    // 이미 충분한 녹음은 지나가고, 소리가 없는 것은 키우지 않는다 — 무음을 목표까지
    // 끌어올리면 잡음만 커지고 그것이 whisper가 자막 상투구를 뱉는 조건이다.
    let gain = gain::normalize(&mut input.samples);

    let raw = engine.transcribe(&input, &model, language)?;

    // 파생 입력은 여기서 쓸모를 다한다. 1시간짜리 녹음이면 수백 MB이므로 영속화 전에
    // 놓아 준다. **정리는 이것뿐이다** — 디스크에 자리를 갖지 않는 파생물이라 지울 파일도,
    // 실패할 수 있는 절차도 없다 (ADR-0007 §9.1 · §9.3).
    drop(input);

    let transcription = parse::normalize(raw)?;

    // **저장 직전이 마지막 자리다** (ADR-0007 §18.5). 한 번 저장된 Transcript는 지우지도
    // 고치지도 못하므로 (INV-2), 쓸 수 없는 결과를 막을 수 있는 자리는 여기뿐이다.
    //
    // 판정은 이 모듈이 하지 않는다 — 세는 것도 나누는 것도 임계값을 아는 것도 전부
    // [`collapse::assess`]의 몫이며 (ADR-0007 §18.3), 여기서 하는 일은 그 판정을 보고
    // **저장을 그만두는 것**이다.
    let assessment = collapse::assess(&transcription.segments);
    match assessment.verdict {
        // 빈 결과 — 이미 있던 판정이며 붕괴 판정이 이것을 대체하지 않는다 (ADR-0007 §18.5).
        // 엔진 구현이 출력 계약([`super::engine::ensure_usable`])을 지켰다면 여기 오지 않는다.
        CollapseVerdict::Empty => {
            return Err(output_unusable("전사 결과에 남은 문장이 없다").with_detail(format!(
                "anomalies={}",
                transcription.anomalies.len()
            )));
        }
        // 붕괴 — 2026-09-07의 103개짜리가 `done`으로 저장되어 나간 자리다 (ADR-0007 §18.1).
        CollapseVerdict::Collapsed { .. } => {
            return Err(collapsed_output(&assessment, transcription.anomalies.len()));
        }
        CollapseVerdict::Usable => {}
    }

    // **판정이 끝난 뒤에 차단한다** (ADR-0007 §20.6). 위 `assess`가 본 수치(n · u · r)는
    // **차단 전** 열의 것이며, 그래서 저장된 segment 수보다 클 수 있다. 그 차이를 설명하는
    // 값이 `removed_count`다 — 순서를 바꾸면 판정이 스스로 고친 결과를 보게 된다.
    //
    // 2026-09-07까지 이 함수를 부르는 제품 코드가 없었다. 모듈과 테스트만 있었고, 그래서
    // Phase 5.8이 만든 차단은 실사용에서 한 번도 동작하지 않았다.
    let blocked = block_consecutive_repeats(&transcription.segments);
    let removed_segments = blocked.removed_count;

    // segment **안쪽**의 되풀이는 위 차단이 잡지 못한다 — 묶음이 성립하지 않기 때문이다
    // (§20.6.1). "엉덩이 × 13"이 한 segment였던 자리다.
    let mut shortened_segments = 0usize;
    let segments: Vec<_> = blocked
        .segments
        .into_iter()
        .map(|mut segment| {
            let collapsed = collapse::collapse_repeated_phrases(&segment.text, MAX_CONSECUTIVE_REPEATS);
            if collapsed != segment.text {
                shortened_segments += 1;
                segment.text = collapsed;
            }
            segment
        })
        .collect();

    // 여기까지가 "전사 한 건"이다. 실패한 시도의 시간은 남지 않는다 — 실패 경로는 Transcript를
    // 만들지 않으며, 이 값은 Transcript와 함께만 존재한다.
    let transcription_ms = elapsed_ms(started);

    // **차단·축약 뒤의 segment에서 다시 만든다.** `parse`가 세운 규칙과 같은 함수를 쓴다 —
    // 규칙을 두 번 정의하지 않는다 (`parse::join_text`).
    //
    // 2026-09-08까지 `raw_text`는 `transcription.raw_text`, 즉 **차단 전** segment로 만든
    // 값이었다. 그래서 차단이 지운 되풀이가 `raw_text`에 그대로 남았고, `raw_text`를 읽는
    // `ai/run.rs`만 정제되지 않은 텍스트를 받았다 (실측: 되풀이 24회 vs segments 3회 ·
    // 1,201자 차이). `export/ai_request.rs`는 segment를 쓰므로 영향이 없었다.
    let raw_text = super::parse::join_text(&segments);

    let transcript = Transcript {
        id: TranscriptId::new(store::new_id(connection)?),
        recording_id: recording.id.clone(),
        // **엔진이 보고한 값이다** — 사용자가 고른 언어를 여기에 베껴 넣지 않는다. 엔진이
        // 말하지 못했으면 비어 있다 (ADR-0007 §16.2 · §17.1.4-3 · parse::normalize).
        language: transcription.language,
        segments: segments
            .into_iter()
            .map(|segment| TranscriptSegment {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
                text: segment.text,
            })
            .collect(),
        raw_text,
        created_at: store::now(connection)?,
        // provenance는 실제로 쓴 것을 적는다 — 설정 값이 아니라 해석된 모델 파일의 이름이다
        // (§7 · ADR-0007 §8.2.4).
        engine: engine.engine_id(),
        model: model.id().to_owned(),
        // 이 전사에 실제로 걸린 시간. 저장하지 않으면 비교할 것이 남지 않는다.
        transcription_ms: Some(transcription_ms),
    };

    // 순서가 규칙이다: 추가 → current로 올리기 → `done`.
    // 화면이 `done`을 보는 시점에는 current가 이미 이 Transcript를 가리킨다.
    store::append_transcript(connection, &transcript)?;
    let now = store::now(connection)?;
    store::set_current_transcript(connection, &recording.id, Some(&transcript.id), &now)?;
    mark(connection, recording, ProcessingStatus::Done)?;

    Ok(Completed {
        transcript,
        anomalies: transcription.anomalies,
        removed_segments,
        shortened_segments,
        gain,
    })
}

/// 붕괴한 전사를 §13의 실패 하나로 옮긴다 (ADR-0007 §18.4).
///
/// **새 실패 종류를 만들지 않는다.** 이미 있는 [`output_unusable`]을 그대로 쓰므로 종류는
/// `TranscriptionOutputUnusable`이고, `retryable = false`(같은 오디오를 같은 조건으로 다시
/// 돌리면 같은 결과다) · `source_data_safe = true`(원본도 Recording 레코드도 건드리지 않았다 ·
/// INV-1 · INV-3)가 따라온다.
///
/// 사용자가 읽는 문장은 **무엇이 잘못됐는지**를 말하고 [`collapse::assess`]가 센 수치를
/// 함께 담는다 (ADR-0007 §18.3) — 수치가 문장에 남지 않으면 다음 사람이 같은 붕괴를 처음부터
/// 다시 세야 한다. 이 함수는 그 수치를 **다시 계산하지 않는다.** 받은 값을 문장에 넣을 뿐이다.
///
/// 원인은 말하지 않는다 — 같은 판정이 낮은 입력 레벨 · 잘못된 언어 · 부족한 모델 어디서도
/// 나오며, 어느 쪽인지는 이 자리가 알 수 없다 (ADR-0007 §18.6).
fn collapsed_output(assessment: &CollapseAssessment, anomalies: usize) -> Failure {
    let metrics = &assessment.metrics;

    let mut message = format!(
        "전사가 붕괴해 회의 내용이 남지 않았다 — 문장 {}개 중 서로 다른 문장이 {}개뿐이다",
        metrics.sentence_count, metrics.unique_count
    );
    if let Some(sentence) = metrics.top_sentence.as_deref() {
        // *무엇이* 되풀이됐는지가 사람에게는 가장 빠른 단서다 (2026-09-07의
        // `한글자막 by 한효정`). 길면 줄여서 넣는다 — 문장 하나로 읽혀야 한다.
        message.push_str(&format!(
            ". \"{}\"이(가) {}번 되풀이됐다",
            shorten(sentence),
            metrics.top_repeat_count
        ));
    }

    // 기술적 표현에는 판정을 재현할 수 있는 값 전부를 둔다 — 비율은 판정 모듈이 낸 그대로이고,
    // `verdict`가 두 조건 중 어느 쪽에 걸렸는지를 말한다 (ADR-0007 §18.2의 [미검증] 항목).
    let detail = format!(
        "segments={} · n={} · u={} · uniqueRatio={:.3} · r={} · topRepeatShare={:.3} · {:?} · anomalies={anomalies}",
        metrics.segment_count,
        metrics.sentence_count,
        metrics.unique_count,
        metrics.unique_ratio(),
        metrics.top_repeat_count,
        metrics.top_repeat_share(),
        assessment.verdict,
    );

    output_unusable(message).with_detail(detail)
}

/// 사람이 읽을 문장에 끼워 넣을 만큼으로 줄인다. 자르는 자리는 **문자 경계**다.
///
/// 붕괴한 전사의 반복 문장이 한 문단만큼 길 수 있다. 줄이는 것은 표시뿐이며, 판정에 쓰인
/// 값은 [`collapse::CollapseMetrics::top_sentence`]에 그대로 남아 있다.
fn shorten(sentence: &str) -> String {
    const VISIBLE_CHARS: usize = 40;

    let mut shortened: String = sentence.chars().take(VISIBLE_CHARS).collect();
    if sentence.chars().nth(VISIBLE_CHARS).is_some() {
        shortened.push('…');
    }
    shortened
}

/// 잰 시간을 밀리초 정수로 옮긴다.
///
/// 단조 시계로 잰 값이므로 음수가 될 수 없고, 넘칠 만큼 긴 전사에서도 패닉하지 않는다 —
/// 오래 걸렸다는 사실 때문에 전사 결과를 잃지 않는다.
fn elapsed_ms(started: Instant) -> i64 {
    i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)
}

/// 전사 상태 하나를 저장한다. 다른 후처리 상태는 읽은 값을 그대로 다시 쓴다.
fn mark(
    connection: &Connection,
    recording: &Recording,
    status: ProcessingStatus,
) -> Result<(), Failure> {
    let now = store::now(connection)?;
    store::update_recording_statuses(
        connection,
        &recording.id,
        status,
        recording.ai_status,
        recording.notion_status,
        &now,
    )?;
    Ok(())
}

/// 실패를 `failed`로 남기고 그 실패를 그대로 돌려준다.
///
/// **원래 원인을 다른 것으로 바꾸지 않는다.** 상태를 남기는 데까지 실패하면 그 사실을
/// detail에 덧붙일 뿐이다 — 사용자가 읽을 문장은 여전히 전사가 왜 실패했는지다 (§13).
fn record_failure(
    connection: &Connection,
    recording: &Recording,
    failure: Failure,
) -> Failure {
    match mark(connection, recording, ProcessingStatus::Failed) {
        Ok(()) => failure,
        Err(storage) => {
            let detail = match failure.detail.as_deref() {
                Some(existing) => format!("{existing} · 실패 상태도 저장하지 못했다: {storage}"),
                None => format!("실패 상태도 저장하지 못했다: {storage}"),
            };
            failure.with_detail(detail)
        }
    }
}

/// 그런 Recording이 없다. 상태를 쓸 대상 자체가 없으므로 아무것도 저장하지 않았다.
fn unknown_recording(id: &RecordingId) -> Failure {
    Failure::permanent(FailureKind::InvalidInput, "전사할 녹음을 찾을 수 없다")
        .with_detail(format!("recordingId={id}"))
}
