//! **Phase 5.8의 행동 불변을 이미 만들어진 경로로 못박는다** (`phase-prompt/05.8` ·
//! `docs/ADR-0007-transcription-engine.md` §20 · PRODUCT-SPEC §7 · §18 · INV-1 · INV-2 · INV-3).
//!
//! Phase 4의 `tests/core_pipeline_without_ai.rs`, Phase 5의
//! `tests/notion_and_export_invariants.rs`, Phase 5.6의
//! `tests/transcription_and_reach_invariants.rs`가 각자의 Phase에 대해 하는 일과 같은 자리다 —
//! **새 제품 코드를 만들지 않고**, 이미 있는 경로를 그대로 지나면서 이 Phase의 불변을 각각
//! 실패할 수 있는 검사로 바꾼다.
//!
//! ```text
//! CH-1  청크가 여럿인 결과가 하나의 Transcript로 저장되고 timestamp가 전체 시간축에서
//!       뒤로 가지 않는다 — 청크 경계에서 되감기지 않는다 (ADR-0007 §20.5)
//!         ch1_chunk_local_times_rewind_which_is_exactly_what_the_offset_undoes
//!         ch1_a_result_that_arrived_in_chunks_is_stored_as_one_transcript_that_never_rewinds
//! CH-2  청킹이 들어와도 붕괴한 결과는 저장 직전에 막힌다 (ADR-0007 §18.5 · §20.6.2)
//!         ch2_a_collapsed_result_is_blocked_even_when_it_arrived_in_chunks
//!         ch2_blocking_first_would_have_hidden_this_collapse_so_the_product_judges_first
//! CH-3  실패해도 원본 오디오도 기존 current Transcript도 그대로다 (INV-1 · INV-2 · INV-3)
//!         ch3_a_failed_chunked_run_leaves_the_audio_and_the_current_transcript_alone
//! CH-4  반복 차단이 정상적인(비연속) 반복을 지우지 않는다 (ADR-0007 §20.6.1)
//!         ch4_repeats_that_are_not_consecutive_survive_the_blocking
//!         ch4_a_run_of_repeats_that_crosses_a_chunk_boundary_is_still_one_run
//! ```
//!
//! ## 실제 whisper도 모델도 72분 오디오도 요구하지 않는다 (PRODUCT-SPEC §18)
//!
//! 엔진 자리에는 계약이 같은 test double이 서고(`transcription::testing::StubEngine`), "모델"은
//! 임시 디렉터리의 몇 바이트짜리 파일이며, 오디오는 0.1초짜리 WAV다. **조건이 없으면 건너뛰는
//! 경로도 없다** — 모델 없음은 skip 조건이 아니라 §13의 정의된 실패다.
//!
//! 그래서 여기서 값으로 만들어 내는 것은 **엔진이 청크마다 낸 출력**이다. 청크 구간과 오프셋은
//! 지어내지 않고 `chunking::plan`에게 물어보며, 그것을 하나로 합치는 것도 `chunking::merge`다 —
//! 제품이 `whisper.rs` 안에서 하는 것과 같은 두 호출이다 (`transcription/whisper.rs`). 합쳐진
//! 결과 하나가 double을 통해 실행 경로로 들어가 `parse::normalize` · 저장 직전 붕괴 판정 ·
//! 영속화를 그대로 지난다.
//!
//! ## 이 파일이 판정하지 **않는** 것
//!
//! **고유 문장 비율이 94%에 닿는가 · 한국어로 읽히는가 · 소요 시간이 6.0분과 비슷한가는 여기서
//! 판정되지 않는다.** 그것은 `phase-prompt/05.8`의 Human Review 항목이고 사람이 앱으로 72분
//! 오디오를 전사해 판정한다 — 자동 Gate가 그것을 대신한다고 말하는 검사는 이 파일에 하나도 없다.
//!
//! **실제 오디오 버퍼가 청크 구간으로 잘리는 자리도 여기서 실행되지 않는다.** 그 자리는
//! `WhisperEngine::transcribe` 안이며 모델 파일을 요구한다. 이 파일이 보는 것은 그 뒤 —
//! **청크 결과가 합쳐진 다음의 경로 전부**다.
//!
//! **연속 반복 차단이 저장 경로의 어느 자리에서 도는지도 여기서 판정하지 않는다.** ADR-0007
//! §20.6.2가 정한 자리는 `run.rs`의 붕괴 판정 **뒤**이며, 2026-09-07 기준으로 저장소에서
//! `chunking::block_consecutive_repeats`를 부르는 제품 코드는 아직 없다(모듈과 재수출뿐이다).
//! 이 Task는 제품 소스를 바꾸지 않으므로, 여기서는 그 규칙이 **값에 대해 무엇을 하는가**만
//! 고정한다 — 그 규칙이 정상적인 반복을 지우지 않는다는 것이 §20.6.3이 [미검증]으로 남긴
//! 대가의 경계이기 때문이다.
//!
//! ## 이미 있는 검사를 다시 쓰지 않는다
//!
//! ```text
//! 청크 경계값 자체 (0 프레임 · 배수 · 배수+1 · 오프셋 넘침)   src/transcription/chunking.rs 의 단위 테스트
//! 붕괴 판정의 임계값과 공식                                    src/transcription/collapse.rs 의 단위 테스트
//! 규칙이 한 파일에만 있다는 사실 · 새 의존성이 없다는 사실      tests/transcription-chunking-boundary.test.ts
//! 전사 실행의 상태 전이와 재전사 일반                          tests/transcription_run.rs
//! ```
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다. Tauri 런타임도 창도 하드웨어도 네트워크도 없다.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::db::{self, store};
use molt_note_lib::domain::{
    Failure, FailureKind, ProcessingStatus, Recording, RecordingId, Transcript,
};
use molt_note_lib::transcription::chunking::{self, ChunkTranscription};
use molt_note_lib::transcription::collapse::{self, CollapseVerdict};
use molt_note_lib::transcription::engine::LanguageChoice;
use molt_note_lib::transcription::parse;
use molt_note_lib::transcription::run::{self, ModelChoice};
use molt_note_lib::transcription::testing::StubEngine;
use molt_note_lib::transcription::{RawSegment, RawTranscription};
use rusqlite::Connection;

/// 파생 입력의 샘플레이트. `chunking`은 이 값을 **받는다** — 짐작하지 않는다 (ADR-0007 §20.2).
const SAMPLE_RATE_HZ: u32 = 16_000;

/// 4분. 120초 청크로 나누면 **정확히 둘**이다.
const FOUR_MINUTES_OF_FRAMES: usize = 240 * SAMPLE_RATE_HZ as usize;

/// 5분. 120초 청크로 나누면 셋이며 마지막이 짧다.
const FIVE_MINUTES_OF_FRAMES: usize = 300 * SAMPLE_RATE_HZ as usize;

/// 10분. 청크 다섯 — 붕괴한 결과가 여러 청크에 걸쳐 도착하는 경우를 만든다.
const TEN_MINUTES_OF_FRAMES: usize = 600 * SAMPLE_RATE_HZ as usize;

/// 2026-09-07의 실사용에서 실제로 되풀이된 문장 (ADR-0007 §18.1).
const HALLUCINATION: &str = "한글자막 by 한효정";

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-transcription-chunking-{}-{}-{}",
            label,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("사전 조건: 임시 디렉터리를 만든다");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 전사 한 건을 돌리는 데 필요한 자리 전부 — DB · 녹음 파일 · 모델 디렉터리.
struct Fixture {
    #[allow(dead_code)]
    dir: TempDir,
    connection: Connection,
    recording_id: RecordingId,
    audio_path: PathBuf,
    models_dir: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let dir = TempDir::new(label);
        let models_dir = dir.join("models");
        fs::create_dir_all(&models_dir).expect("사전 조건: 모델 디렉터리를 만든다");
        fs::write(models_dir.join("ggml-base.bin"), b"not a real model")
            .expect("사전 조건: 자리표시자 모델을 만든다");

        let audio_path = dir.join("recording.wav");
        write_silence_wav(&audio_path);

        let connection = db::open(dir.join("molt-note.db")).expect("임시 DB를 열 수 있어야 한다");
        let recording_id = RecordingId::new("rec-chunked");
        store::insert_recording(
            &connection,
            &Recording {
                id: recording_id.clone(),
                title: "3DGS Study #04".to_owned(),
                created_at: "2026-09-07T10:00:00.000Z".to_owned(),
                updated_at: "2026-09-07T10:00:00.000Z".to_owned(),
                duration_ms: 100,
                audio_path: audio_path.to_str().expect("경로 문자열").to_owned(),
                audio_format: "wav".to_owned(),
                microphone: Some("MacBook Pro Microphone".to_owned()),
                current_transcript_id: None,
                transcription_status: ProcessingStatus::None,
                ai_status: ProcessingStatus::None,
                notion_status: ProcessingStatus::None,
            },
        )
        .expect("사전 조건: 녹음 레코드를 저장한다");

        Self {
            dir,
            connection,
            recording_id,
            audio_path,
            models_dir,
        }
    }

    /// 언어를 고르지 않은 상태로 전사한다. **그것이 앱의 기본 상태다** (ADR-0007 §17.1.4-1).
    fn transcribe(&mut self, engine: &StubEngine) -> Result<run::Completed, Failure> {
        let choice = ModelChoice {
            models_dir: &self.models_dir,
            configured: Some("ggml-base.bin"),
        };
        run::transcribe(
            &mut self.connection,
            &self.recording_id,
            engine,
            choice,
            &LanguageChoice::Detect,
        )
    }

    fn recording(&self) -> Recording {
        store::load_recording(&self.connection, &self.recording_id)
            .expect("녹음을 읽을 수 있어야 한다")
            .expect("녹음이 남아 있어야 한다")
    }

    fn transcripts(&self) -> Vec<Transcript> {
        store::list_transcripts(&self.connection, &self.recording_id)
            .expect("Transcript 목록을 읽을 수 있어야 한다")
    }

    /// 원본 오디오 파일의 바이트 전부. **읽기 전용 불변을 값으로 대조하기 위한 것이다** (INV-1).
    fn audio_bytes(&self) -> Vec<u8> {
        fs::read(&self.audio_path).expect("원본 오디오를 읽을 수 있어야 한다")
    }
}

/// 0.1초짜리 16 kHz mono PCM16 WAV. 짧고 결정론적인 fixture다 (PRODUCT-SPEC §18).
///
/// **이 파일의 길이는 청크 수와 무관하다.** 실제 버퍼를 자르는 것은 `WhisperEngine`이고 여기
/// 서는 double이 그 자리에 서므로, 청크 구간과 오프셋은 `chunking::plan`에게 직접 물어본다.
fn write_silence_wav(path: &Path) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE_HZ,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("사전 조건: WAV를 만든다");
    for index in 0..1_600_i32 {
        writer
            .write_sample((index % 128) as i16)
            .expect("사전 조건: 샘플을 쓴다");
    }
    writer.finalize().expect("사전 조건: WAV를 닫는다");
}

/// 청크 **로컬** 시각의 segment 하나. **센티초다** (ADR-0007 §10 · §20.5).
///
/// 청크마다 0 근처에서 다시 시작한다 — 그것이 "청크 로컬"의 뜻이며, 오프셋이 더해지지 않으면
/// 두 번째 청크의 시각이 첫 청크 위로 되감긴다.
fn local_segment(position: usize, text: &str) -> RawSegment {
    let start_centiseconds = position as i64 * 450;
    RawSegment {
        start_centiseconds,
        end_centiseconds: start_centiseconds + 448,
        text: Some(format!(" {text}")),
    }
}

/// 엔진이 청크마다 냈다고 가정하는 출력들.
///
/// 청크 구간과 오프셋은 **지어내지 않는다** — `chunking::plan`이 낸 것을 그대로 쓴다. 그래서
/// 이 fixture는 청크 길이가 바뀌면 함께 움직이고, 값을 두 번째 자리에 적지 않는다.
fn chunk_outputs(
    total_frames: usize,
    per_chunk: usize,
    sentence: impl Fn(usize, usize) -> String,
) -> Vec<ChunkTranscription> {
    let chunks = chunking::plan(total_frames, SAMPLE_RATE_HZ)
        .expect("16 kHz에서 청크 나누기는 실패하지 않는다");

    chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| ChunkTranscription {
            offset_centiseconds: chunk.offset_centiseconds,
            output: RawTranscription {
                language: Some("ko".to_owned()),
                segments: (0..per_chunk)
                    .map(|position| local_segment(position, &sentence(index, position)))
                    .collect(),
            },
        })
        .collect()
}

/// 서로 다른 문장만 나온 정상적인 전사. 청크 셋에 걸쳐 아홉 문장이다.
fn healthy_chunks() -> Vec<ChunkTranscription> {
    chunk_outputs(FIVE_MINUTES_OF_FRAMES, 3, |chunk, position| {
        format!("{chunk}번째 구간의 {position}번째 문장")
    })
}

/// 2026-09-07처럼 **붕괴한** 전사. 청크 다섯에 걸쳐 같은 문장 130개다 (ADR-0007 §18.1).
///
/// 고유 비율 1/130 ≈ 0.008 · 최다 반복 점유율 1.000 — §18.2의 두 조건에 **둘 다** 걸린다.
fn collapsed_chunks() -> Vec<ChunkTranscription> {
    chunk_outputs(TEN_MINUTES_OF_FRAMES, 26, |_, _| HALLUCINATION.to_owned())
}

/// 청크 결과들을 제품과 같은 순서로 하나로 만든다 — `whisper.rs`가 하는 그 호출이다.
fn merged(chunks: Vec<ChunkTranscription>) -> RawTranscription {
    chunking::merge(chunks).expect("이 길이에서는 오프셋이 넘치지 않는다")
}

/// 정규화까지 마친 열. 저장 직전 판정과 반복 차단이 보는 것이 이 열이다 (ADR-0007 §20.6.2).
fn normalized(raw: RawTranscription) -> Vec<parse::TranscriptSegment> {
    parse::normalize(raw)
        .expect("이 값들은 정규화에서 실패하지 않는다")
        .segments
}

fn is_collapsed(verdict: CollapseVerdict) -> bool {
    matches!(verdict, CollapseVerdict::Collapsed { .. })
}

fn texts(segments: &[parse::TranscriptSegment]) -> Vec<&str> {
    segments.iter().map(|segment| segment.text.as_str()).collect()
}

// ── CH-1 청크 경계에서 시각이 되감기지 않는다 (ADR-0007 §20.5) ──────────────

#[test]
fn ch1_chunk_local_times_rewind_which_is_exactly_what_the_offset_undoes() {
    // 아래 검사가 "원래 되감길 일이 없어서" 통과하는 것이 아님을 먼저 못박는다.
    // 청크가 낸 원시 시각은 청크마다 0에서 다시 시작하며, 그것을 그대로 이어 붙이면
    // 두 번째 청크의 시각이 첫 청크 위로 겹쳐진다.
    let chunks = healthy_chunks();
    assert_eq!(chunks.len(), 3, "5분이면 120초 청크가 셋이다");

    let naive: Vec<i64> = chunks
        .iter()
        .flat_map(|chunk| chunk.output.segments.iter())
        .map(|segment| segment.start_centiseconds)
        .collect();

    assert_eq!(naive, vec![0, 450, 900, 0, 450, 900, 0, 450, 900]);
    assert!(
        !naive.windows(2).all(|pair| pair[0] <= pair[1]),
        "오프셋을 더하지 않으면 시각이 되감긴다 — 그것이 이 Phase가 막는 것이다"
    );
}

#[test]
fn ch1_a_result_that_arrived_in_chunks_is_stored_as_one_transcript_that_never_rewinds() {
    let mut fixture = Fixture::new("timeline");
    let engine = StubEngine::returning(merged(healthy_chunks()));

    let completed = fixture
        .transcribe(&engine)
        .expect("정상적인 전사는 저장된다");

    // 밖으로 나가는 계약은 지금과 같다 — 청크가 셋이어도 엔진 호출은 하나이고
    // 저장되는 것도 Transcript 하나다 (`phase-prompt/05.8` Constraints · INV-9).
    assert_eq!(engine.call_count(), 1, "밖에서 보면 전사 하나가 나올 뿐이다");
    assert_eq!(fixture.transcripts().len(), 1, "Transcript 하나가 늘어난다");

    let starts: Vec<i64> = completed
        .transcript
        .segments
        .iter()
        .map(|segment| segment.start_ms)
        .collect();

    assert_eq!(starts.len(), 9, "세 청크의 문장이 하나도 빠지지 않는다");
    assert!(
        starts.windows(2).all(|pair| pair[0] <= pair[1]),
        "전체 시간축에서 뒤로 가지 않는다: {starts:?}"
    );
    // 청크 경계의 두 자리를 값으로 못박는다 — 120초 · 240초다. 되감김은 여기서 먼저 드러난다.
    assert_eq!(starts[3], 120_000, "두 번째 청크의 첫 문장은 2분 자리다");
    assert_eq!(starts[6], 240_000, "세 번째 청크의 첫 문장은 4분 자리다");
    assert_eq!(
        completed.transcript.segments[3].end_ms, 124_480,
        "끝 시각에도 같은 오프셋이 더해진다 — 시작만 옮기고 끝을 두고 오지 않는다"
    );

    // `parse`는 앞 segment보다 이른 시작을 **정정하지 않고 기록한다**
    // (`AnomalyKind::OutOfOrder` · `Overlap`). 그 목록이 비어 있다는 것이 되감김도 겹침도
    // 없었다는 제3자의 관찰이다.
    assert!(
        completed.anomalies.is_empty(),
        "청크 경계에서 되감김도 겹침도 기록되지 않는다: {:?}",
        completed.anomalies
    );

    // 저장된 것이 돌려받은 것과 같다 — 값이 화면으로만 가고 디스크에는 다른 것이 남는 일이 없다.
    let stored = fixture.transcripts().remove(0);
    assert_eq!(stored, completed.transcript);
    assert_eq!(
        fixture.recording().current_transcript_id.as_ref(),
        Some(&completed.transcript.id),
        "새 Transcript가 current다 (PRODUCT-SPEC §7.2)"
    );
    assert_eq!(
        fixture.recording().transcription_status,
        ProcessingStatus::Done
    );
}

// ── CH-2 청킹이 붕괴 판정을 우회하지 않는다 (ADR-0007 §18.5 · §20.6.2) ──────

#[test]
fn ch2_a_collapsed_result_is_blocked_even_when_it_arrived_in_chunks() {
    let mut fixture = Fixture::new("collapsed");
    let output = merged(collapsed_chunks());
    assert_eq!(output.segments.len(), 130, "다섯 청크에 걸쳐 도착한 결과다");

    let engine = StubEngine::returning(output);
    let failure = fixture
        .transcribe(&engine)
        .expect_err("붕괴한 결과는 저장되지 않는다");

    // 새 실패 종류를 만들지 않는다 — 이미 있는 §13의 하나다 (ADR-0007 §18.4).
    assert_eq!(failure.kind, FailureKind::TranscriptionOutputUnusable);
    assert!(!failure.retryable, "같은 오디오를 같은 조건으로 다시 돌리면 같은 결과다");
    assert!(failure.source_data_safe, "원본도 레코드도 건드리지 않았다");
    assert!(
        failure.message.contains("붕괴"),
        "사람이 읽을 문장이 무엇이 잘못됐는지 말한다: {}",
        failure.message
    );

    let detail = failure.detail.as_deref().expect("판정 수치가 함께 남는다");
    assert!(
        detail.contains("n=130") && detail.contains("u=1"),
        "판정을 재현할 수치가 그대로 남는다: {detail}"
    );

    // 저장 직전에 막혔다는 것은 **아무것도 늘지 않았다**는 뜻이다 (INV-2).
    assert!(fixture.transcripts().is_empty(), "Transcript가 늘지 않는다");
    let recording = fixture.recording();
    assert_eq!(recording.transcription_status, ProcessingStatus::Failed);
    assert_eq!(recording.current_transcript_id, None);
}

#[test]
fn ch2_blocking_first_would_have_hidden_this_collapse_so_the_product_judges_first() {
    // ADR-0007 §20.6.2: *"차단을 판정보다 먼저 두면 붕괴한 전사가 통과한다."*
    // 그 문장이 이 fixture에서 실제로 참이라는 것을 값으로 보이고, 그럼에도 제품이 막는다는
    // 것을 같은 값으로 확인한다 — 순서가 뒤집히는 날 이 검사가 먼저 실패한다.
    let output = merged(collapsed_chunks());
    let segments = normalized(output.clone());

    let before = collapse::assess(&segments);
    assert!(
        is_collapsed(before.verdict),
        "차단 전 열은 붕괴로 판정된다: {:?}",
        before.verdict
    );
    assert_eq!(before.metrics.sentence_count, 130);
    assert_eq!(before.metrics.unique_count, 1);

    let blocked = chunking::block_consecutive_repeats(&segments);
    assert_eq!(blocked.segments.len(), 3, "한 묶음에서 3개만 남는다");
    assert_eq!(blocked.removed_count, 127, "지워진 개수를 잃지 않는다 (§20.6.2)");

    let after = collapse::assess(&blocked.segments);
    assert!(
        !is_collapsed(after.verdict),
        "차단 뒤에는 같은 붕괴가 보이지 않는다 — 증거가 지워졌기 때문이다: {:?}",
        after.verdict
    );

    // 그래서 제품은 **차단 전 열로 판정한다.** 같은 값이 실행 경로에서 실패로 끝난다.
    let mut fixture = Fixture::new("order");
    let engine = StubEngine::returning(output);
    let failure = fixture
        .transcribe(&engine)
        .expect_err("판정이 차단보다 앞에 있으므로 이 결과는 막힌다");

    assert_eq!(failure.kind, FailureKind::TranscriptionOutputUnusable);
    assert!(fixture.transcripts().is_empty());
}

// ── CH-3 실패가 아무것도 잃지 않는다 (INV-1 · INV-2 · INV-3) ────────────────

#[test]
fn ch3_a_failed_chunked_run_leaves_the_audio_and_the_current_transcript_alone() {
    let mut fixture = Fixture::new("keeps");
    let audio_before = fixture.audio_bytes();

    let first = fixture
        .transcribe(&StubEngine::returning(merged(healthy_chunks())))
        .expect("첫 전사는 성공한다");
    let kept = first.transcript.clone();

    let failure = fixture
        .transcribe(&StubEngine::returning(merged(collapsed_chunks())))
        .expect_err("두 번째 전사는 붕괴로 막힌다");
    assert_eq!(failure.kind, FailureKind::TranscriptionOutputUnusable);

    // 원본 오디오 — 바이트 하나 다르지 않다 (INV-1).
    assert_eq!(
        fixture.audio_bytes(),
        audio_before,
        "실패가 원본 오디오를 건드리지 않는다"
    );

    // 기존 Transcript — 늘지도 바뀌지도 않았고 여전히 current다 (INV-2 · INV-3).
    let transcripts = fixture.transcripts();
    assert_eq!(transcripts.len(), 1, "실패한 시도는 Transcript를 남기지 않는다");
    assert_eq!(transcripts[0], kept, "저장된 Transcript가 그대로다");

    let recording = fixture.recording();
    assert_eq!(
        recording.current_transcript_id.as_ref(),
        Some(&kept.id),
        "current는 앞선 성공을 그대로 가리킨다"
    );
    // 사용자에게 실패가 보이는 형태는 상태 하나다 (PRODUCT-SPEC §13). 원본을 가리키는 경로도
    // 그대로여서, 사람이 같은 파일로 몇 번이든 다시 시도할 수 있다.
    assert_eq!(recording.transcription_status, ProcessingStatus::Failed);
    assert_eq!(
        recording.audio_path,
        fixture
            .audio_path
            .to_str()
            .expect("경로 문자열")
            .to_owned(),
        "실패가 원본 오디오의 경로를 옮기지 않는다"
    );
}

// ── CH-4 반복 차단은 연속만 끊는다 (ADR-0007 §20.6.1) ───────────────────────

#[test]
fn ch4_repeats_that_are_not_consecutive_survive_the_blocking() {
    // "떨어져서 다시 나오는 같은 문장은 반복이지 연속이 아니다" (§20.6.1). 2026-09-05의 실행에서
    // 최다 반복이 10회로 **남은 것**이 이 규칙의 결과다 — 차단이 정상적인 대화를 지우지 않는다.
    let segments = normalized(merged(chunk_outputs(
        FOUR_MINUTES_OF_FRAMES,
        10,
        |_, position| {
            if position % 2 == 0 {
                "네".to_owned()
            } else {
                "그렇군요".to_owned()
            }
        },
    )));
    assert_eq!(segments.len(), 20, "두 청크에 걸쳐 스무 문장이다");

    let blocked = chunking::block_consecutive_repeats(&segments);

    assert_eq!(blocked.removed_count, 0, "연속이 아닌 반복은 지우지 않는다");
    assert_eq!(
        blocked.segments, segments,
        "같은 문장이 열 번 나왔지만 하나도 사라지지 않는다"
    );
}

#[test]
fn ch4_a_run_of_repeats_that_crosses_a_chunk_boundary_is_still_one_run() {
    // 청크 경계는 **오디오를 자른 자리**이지 대화가 끊긴 자리가 아니다. 한 묶음이 경계를 넘어
    // 이어지면 세는 값도 이어져야 한다 — 경계마다 1로 돌아가면 임계값이 청크 수만큼 느슨해진다.
    //
    // 첫 청크의 끝 둘과 다음 청크의 처음 둘이 같은 문장이다 → 이어진 네 개 중 4번째가 지워진다.
    let segments = normalized(merged(chunk_outputs(
        FOUR_MINUTES_OF_FRAMES,
        4,
        |chunk, position| match (chunk, position) {
            (0, 2 | 3) | (1, 0 | 1) => "네".to_owned(),
            _ => format!("{chunk}-{position}번째 문장"),
        },
    )));
    assert_eq!(
        texts(&segments),
        vec![
            "0-0번째 문장",
            "0-1번째 문장",
            "네",
            "네",
            "네",
            "네",
            "1-2번째 문장",
            "1-3번째 문장",
        ]
    );

    let blocked = chunking::block_consecutive_repeats(&segments);

    assert_eq!(
        blocked.removed_count, 1,
        "경계를 넘어 이어진 묶음의 4번째가 지워진다"
    );
    assert_eq!(
        texts(&blocked.segments),
        vec![
            "0-0번째 문장",
            "0-1번째 문장",
            "네",
            "네",
            "네",
            "1-2번째 문장",
            "1-3번째 문장",
        ]
    );
    // 살아남은 문장의 시각은 손대지 않는다 — 차단은 지우기만 한다.
    assert_eq!(blocked.segments[2].start_ms, segments[2].start_ms);
}

// --- VAD: 켜는 순서와 "없어도 된다"는 것 (2026-09-08) ------------------------------------

/// VAD를 켜는 순서는 실행 시점에만 드러나므로 **소스에서 고정한다.**
const WHISPER_SOURCE: &str = include_str!("../src/transcription/whisper.rs");

/// `enable_vad(true)`는 `vad_model_path`가 없으면 **panic한다**
/// (`whisper-rs 0.16.0` · `whisper_params.rs:823`).
///
/// panic은 Gate가 잡아 주지 않는 자리(실행 시점)에서 앱 전체를 죽인다. 그래서 **순서를
/// 소스에서 고정한다**: 경로 설정이 켜기보다 먼저 나와야 하고, 켜는 것은 경로가 있을 때뿐이다.
#[test]
fn the_vad_model_path_is_always_set_before_vad_is_enabled() {
    let source = WHISPER_SOURCE;

    let set_path = source
        .find("params.set_vad_model_path(")
        .expect("VAD 모델 경로를 설정하는 자리가 있어야 한다");
    let enable = source
        .find("params.enable_vad(")
        .expect("VAD를 켜는 자리가 있어야 한다");

    assert!(
        set_path < enable,
        "set_vad_model_path 가 enable_vad 보다 먼저 와야 한다 — 아니면 panic한다"
    );
}

/// 켜는 것은 **경로가 있을 때뿐**이다. 무조건 켜면 모델 파일이 없는 기기에서 panic한다.
#[test]
fn vad_is_only_enabled_when_a_path_was_found() {
    let source = WHISPER_SOURCE;
    let enable = source
        .find("params.enable_vad(")
        .expect("VAD를 켜는 자리가 있어야 한다");

    let before = &source[..enable];
    let guard = before
        .rfind("if let Some(path) = &vad_path {")
        .expect("경로가 있을 때만 켜는 분기 안에 있어야 한다");

    // 그 분기가 닫히기 전에 enable_vad 가 온다.
    let closing = source[guard..].find("\n            }").map(|at| guard + at);
    assert!(
        closing.is_none_or(|closing| enable < closing),
        "enable_vad 가 경로 분기 밖에 있다"
    );
}

/// VAD 모델이 없는 것은 **실패가 아니다.** 없으면 전사는 지금까지처럼 그대로 돈다.
///
/// 이 규칙이 깨지면 모델을 받지 않은 기기에서 전사가 통째로 실패한다 — 2026-09-08 이전보다
/// 나빠진다.
#[test]
fn a_missing_vad_model_does_not_fail_transcription() {
    use molt_note_lib::transcription::vad::{self, VadChoice};

    let empty = std::env::temp_dir().join("molt-note-vad-absent-for-sure-9f2c1a");
    let _ = std::fs::remove_dir_all(&empty);

    // Result 가 아니라 VadChoice 다 — 없는 것이 실패로 표현될 수 없는 형태다.
    assert_eq!(vad::find(&empty), VadChoice::Disabled);
}
