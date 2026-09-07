//! 전사 orchestration: Transcript 추가 · current 갱신 · 상태 전이 · 실패가 잃는 것이 없음.
//!
//! **실제 whisper도 모델 파일도 요구하지 않는다** (`phase-prompt/03` 요구 8 · 9 · §18).
//! 엔진 자리에는 계약이 같은 test double이 서고(`transcription::testing::StubEngine`),
//! "모델"은 임시 디렉터리에 만든 몇 바이트짜리 파일이며, 오디오는 0.1초짜리 WAV다.
//!
//! 여기서 판정하는 것은 **영속성 규칙**이다 — 전사 품질이 아니다 (`phase-prompt/03`의
//! Human Review 항목은 DEFERRED다).
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::db::{self, store};
use molt_note_lib::domain::{
    Failure, FailureKind, ProcessingStatus, Recording, RecordingId, Transcript, TranscriptId,
    TranscriptSegment,
};
use molt_note_lib::transcription::engine::{engine_failed, LanguageChoice};
use molt_note_lib::transcription::run::{self, ModelChoice};
use molt_note_lib::transcription::testing::StubEngine;
use molt_note_lib::transcription::{RawSegment, RawTranscription};
use rusqlite::Connection;

/// 실패 경로가 원본을 지우거나 고치는 코드를 갖고 있지 않다는 것은 소스에서도 확인한다
/// (INV-1 · INV-3).
const RUN_SOURCE: &str = include_str!("../src/transcription/run.rs");

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-transcription-run-{}-{}-{}",
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
    dir: TempDir,
    connection: Connection,
    recording_id: RecordingId,
    audio_path: PathBuf,
    models_dir: PathBuf,
}

impl Fixture {
    /// 모델 파일이 **있는** 상태로 준비한다.
    fn new(label: &str) -> Self {
        let fixture = Self::without_model(label);
        fs::write(fixture.models_dir.join("ggml-base.bin"), b"not a real model")
            .expect("사전 조건: 자리표시자 모델을 만든다");
        fixture
    }

    /// 모델 파일이 **없는** 상태로 준비한다. 모델 디렉터리 자체는 있다.
    fn without_model(label: &str) -> Self {
        let dir = TempDir::new(label);
        let models_dir = dir.join("models");
        fs::create_dir_all(&models_dir).expect("사전 조건: 모델 디렉터리를 만든다");

        let audio_path = dir.join("recording.wav");
        write_silence_wav(&audio_path);

        let connection = db::open(dir.join("molt-note.db")).expect("임시 DB를 열 수 있어야 한다");
        let recording_id = RecordingId::new("rec-transcribe");
        store::insert_recording(
            &connection,
            &Recording {
                id: recording_id.clone(),
                title: "3DGS Study #04".to_owned(),
                created_at: "2026-09-03T10:00:00.000Z".to_owned(),
                updated_at: "2026-09-03T10:00:00.000Z".to_owned(),
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
        self.transcribe_in(engine, &LanguageChoice::Detect)
    }

    fn transcribe_in(
        &mut self,
        engine: &StubEngine,
        language: &LanguageChoice,
    ) -> Result<run::Completed, Failure> {
        let choice = ModelChoice {
            models_dir: &self.models_dir,
            configured: Some("ggml-base.bin"),
        };
        run::transcribe(
            &mut self.connection,
            &self.recording_id,
            engine,
            choice,
            language,
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

    fn transcript(&self, id: &TranscriptId) -> Transcript {
        store::load_transcript(&self.connection, id)
            .expect("Transcript를 읽을 수 있어야 한다")
            .expect("그 Transcript가 남아 있어야 한다")
    }

    /// 저장된 `transcription_status`를 **저장된 순서대로** 기록하기 시작한다.
    ///
    /// 값을 기록하는 것은 제품 코드가 아니라 DB의 trigger다 — 무엇이 실제로 행에 쓰였는지를
    /// 제3자가 관찰한 결과이며, 그래서 orchestration에 테스트 전용 통로를 내지 않는다.
    fn record_status_writes(&self) {
        self.connection
            .execute_batch(
                "CREATE TABLE status_log (seq INTEGER PRIMARY KEY, status TEXT NOT NULL);
                 CREATE TRIGGER log_transcription_status
                 AFTER UPDATE OF transcription_status ON recordings
                 BEGIN
                     INSERT INTO status_log (status) VALUES (NEW.transcription_status);
                 END;",
            )
            .expect("사전 조건: 상태 기록용 trigger를 만든다");
    }

    fn stored_statuses(&self) -> Vec<String> {
        let mut statement = self
            .connection
            .prepare("SELECT status FROM status_log ORDER BY seq")
            .expect("기록을 읽을 수 있어야 한다");
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("기록을 질의할 수 있어야 한다");
        rows.collect::<Result<Vec<_>, _>>()
            .expect("기록 행을 읽을 수 있어야 한다")
    }
}

/// 0.1초짜리 16 kHz mono PCM16 WAV. 짧고 결정론적인 fixture다 (`phase-prompt/03` 요구 8).
fn write_silence_wav(path: &Path) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
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

/// 엔진이 냈다고 가정하는 원시 출력. **센티초다** (ADR-0007 §10).
fn raw_output(first: &str, second: &str) -> RawTranscription {
    RawTranscription {
        language: Some("ko".to_owned()),
        segments: vec![
            RawSegment {
                start_centiseconds: 13_400,
                end_centiseconds: 14_100,
                text: Some(format!(" {first}")),
            },
            RawSegment {
                start_centiseconds: 14_100,
                end_centiseconds: 14_800,
                text: Some(format!(" {second}")),
            },
        ],
    }
}

fn first_run() -> RawTranscription {
    raw_output(
        "그러면 이번에는 PLY 먼저 변환하고",
        "그다음 SOG 변환 확인하면 될 것 같아요.",
    )
}

fn second_run() -> RawTranscription {
    raw_output("다시 전사한 첫 문장", "다시 전사한 두 번째 문장")
}

/// 2026-09-07에 실제로 나온 **붕괴한** 출력 (ADR-0007 §18.1).
///
/// 51분 회의의 전사가 segment 103개였고 그중 102개가 같은 문장이었다 — 고유 1.9% ·
/// 최다 반복 99.0%. 그 결과가 `done`으로 저장된 것이 붕괴 판정을 만든 관측이며, 여기서는
/// 같은 모양의 출력을 값으로 만들어 낸다. **실제 whisper도 모델도 필요하지 않다** (§18).
fn collapsed_run() -> RawTranscription {
    let mut segments: Vec<RawSegment> = (0..102)
        .map(|index| thirty_second_segment(index, "한글자막 by 한효정"))
        .collect();
    segments.push(thirty_second_segment(102, "감사합니다."));

    RawTranscription {
        language: Some("ko".to_owned()),
        segments,
    }
}

/// 관측된 붕괴처럼 경계가 정확히 30초인 segment 하나. **센티초다** (ADR-0007 §10).
fn thirty_second_segment(index: i64, text: &str) -> RawSegment {
    let start_centiseconds = index * 3_000;
    RawSegment {
        start_centiseconds,
        end_centiseconds: start_centiseconds + 2_998,
        text: Some(format!(" {text}")),
    }
}

// --- 성공 경로 -----------------------------------------------------------------------

#[test]
fn a_successful_run_appends_one_transcript_and_makes_it_current() {
    let mut fixture = Fixture::new("success");
    let engine = StubEngine::returning(first_run());

    let completed = fixture.transcribe(&engine).expect("전사가 성공해야 한다");

    let stored = fixture.transcripts();
    assert_eq!(stored.len(), 1, "Transcript가 한 건 추가된다 (§7.1)");
    assert_eq!(stored[0], completed.transcript, "돌려준 것이 저장된 것이다");

    let recording = fixture.recording();
    assert_eq!(
        recording.current_transcript_id.as_ref(),
        Some(&completed.transcript.id),
        "성공한 Transcript가 current가 된다 (§7.2)"
    );
    assert_eq!(recording.transcription_status, ProcessingStatus::Done);
}

#[test]
fn the_stored_transcript_carries_every_field_section_7_requires() {
    let mut fixture = Fixture::new("fields");
    let engine = StubEngine::returning(first_run());

    let completed = fixture.transcribe(&engine).expect("전사가 성공해야 한다");
    let transcript = fixture.transcript(&completed.transcript.id);

    assert_eq!(transcript.recording_id, fixture.recording_id);
    assert_eq!(transcript.language.as_deref(), Some("ko"), "language");
    assert_eq!(
        transcript.segments,
        vec![
            TranscriptSegment {
                start_ms: 134_000,
                end_ms: 141_000,
                text: "그러면 이번에는 PLY 먼저 변환하고".to_owned(),
            },
            TranscriptSegment {
                start_ms: 141_000,
                end_ms: 148_000,
                text: "그다음 SOG 변환 확인하면 될 것 같아요.".to_owned(),
            },
        ],
        "segments[] {{start_ms, end_ms, text}} — 센티초 13400은 밀리초 134000이다 (×10)"
    );
    assert_eq!(
        transcript.raw_text,
        "그러면 이번에는 PLY 먼저 변환하고 그다음 SOG 변환 확인하면 될 것 같아요.",
        "rawText"
    );
    assert_eq!(transcript.engine, "stub-transcription-engine", "engine");
    assert_eq!(transcript.model, "ggml-base.bin", "model — 실제로 쓴 파일 이름");
    assert!(
        transcript.created_at.starts_with("20") && transcript.created_at.ends_with('Z'),
        "createdAt이 ISO-8601 UTC 텍스트여야 한다: {}",
        transcript.created_at
    );
    assert!(completed.anomalies.is_empty(), "정상 출력에는 이상이 없다");
}

#[test]
fn a_successful_run_stores_how_long_the_transcription_actually_took() {
    // `phase-prompt/05.6` 성공 기준 3: 걸린 시간이 **기록으로 남아야** 사람이 Metal 전후를
    // 비교할 수 있다. 값이 그럴듯한지는 여기서 판정하지 않는다(그것은 실행마다 다르다) —
    // 판정하는 것은 **잰 값이 Transcript와 함께 실제로 저장되고 다시 읽힌다**는 것이다.
    let mut fixture = Fixture::new("elapsed");
    let engine = StubEngine::returning(first_run());

    let completed = fixture.transcribe(&engine).expect("전사가 성공해야 한다");
    let stored = fixture.transcript(&completed.transcript.id);

    let measured = stored
        .transcription_ms
        .expect("전사 한 건에 걸린 시간이 저장돼야 한다 — 재지 않으면 비교할 것이 남지 않는다");
    assert!(
        measured >= 0,
        "단조 시계로 재므로 음수가 될 수 없다: {measured}"
    );
    assert_eq!(
        completed.transcript.transcription_ms,
        Some(measured),
        "돌려준 값과 저장된 값이 같아야 한다"
    );

    // 재는 자리가 한 곳이라는 것은 소스에서도 확인한다 — 두 곳에서 재기 시작하면 어느 값이
    // 저장된 것인지 말할 수 없게 된다.
    assert_eq!(
        RUN_SOURCE.matches("Instant::now()").count(),
        1,
        "경과 시간을 재기 시작하는 자리는 하나여야 한다"
    );
    assert!(
        !RUN_SOURCE.contains("SystemTime::now"),
        "벽시계로 재면 시스템 시각 조정이 음수 소요 시간을 만들 수 있다"
    );
}

#[test]
fn a_failed_run_leaves_no_transcript_and_therefore_no_measured_time() {
    // 걸린 시간은 Transcript와 함께만 존재한다. 실패한 시도의 시간이 어딘가에 남아 다음
    // 비교를 흐리지 않는다 (INV-2).
    let mut fixture = Fixture::without_model("elapsed-failed");
    let engine = StubEngine::returning(first_run());

    fixture.transcribe(&engine).expect_err("모델이 없으면 실패해야 한다");

    assert!(
        fixture.transcripts().is_empty(),
        "실패는 Transcript를 만들지 않는다"
    );
}

#[test]
fn the_timestamps_are_not_off_by_a_factor_of_ten_or_a_hundred() {
    // 조용히 100배 어긋난 transcript가 저장되면 Gate는 그것을 잡지 못한다
    // (`phase-prompt/03` 요구 4). 저장된 값으로 확인한다.
    let mut fixture = Fixture::new("units");
    let engine = StubEngine::returning(RawTranscription {
        language: None,
        segments: vec![RawSegment {
            start_centiseconds: 100,
            end_centiseconds: 6_000,
            text: Some("1초에서 60초까지".to_owned()),
        }],
    });

    let completed = fixture.transcribe(&engine).expect("전사가 성공해야 한다");

    let segment = &fixture.transcript(&completed.transcript.id).segments[0];
    assert_eq!(segment.start_ms, 1_000, "100cs는 1초다 — 100ms도 10초도 아니다");
    assert_eq!(segment.end_ms, 60_000, "6000cs는 60초다");
}

// --- 재전사 (INV-2 · §7.1 · §7.2) ------------------------------------------------------

#[test]
fn a_second_successful_run_adds_a_transcript_instead_of_updating_the_first() {
    let mut fixture = Fixture::new("retranscribe");
    let engine = StubEngine::responding_with(vec![Ok(first_run()), Ok(second_run())]);

    let first = fixture.transcribe(&engine).expect("첫 전사가 성공해야 한다");
    let before = fixture.transcript(&first.transcript.id);
    let second = fixture.transcribe(&engine).expect("재전사가 성공해야 한다");

    assert_ne!(first.transcript.id, second.transcript.id, "새 identity다");

    let stored = fixture.transcripts();
    assert_eq!(stored.len(), 2, "두 Transcript가 함께 남는다 (INV-2)");
    let ids: Vec<&str> = stored.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&first.transcript.id.as_str()), "이전 것이 남는다");
    assert!(ids.contains(&second.transcript.id.as_str()), "새 것이 추가된다");

    assert_eq!(
        fixture.transcript(&first.transcript.id),
        before,
        "이전 Transcript는 한 글자도 바뀌지 않는다 — UPDATE가 아니다 (§7.1)"
    );
    assert_eq!(
        fixture.recording().current_transcript_id.as_ref(),
        Some(&second.transcript.id),
        "바뀌는 것은 current뿐이다 (§7.2)"
    );
}

#[test]
fn a_failed_re_transcription_leaves_the_previous_transcript_as_current() {
    // Transcript A = success / current → 실패한 시도 → current는 여전히 A다.
    let mut fixture = Fixture::new("failed-retranscribe");
    let engine = StubEngine::responding_with(vec![
        Ok(first_run()),
        Err(engine_failed("전사 도중 엔진이 실패했다")),
    ]);

    let transcript_a = fixture
        .transcribe(&engine)
        .expect("첫 전사가 성공해야 한다")
        .transcript;
    let a_before = fixture.transcript(&transcript_a.id);

    let failure = fixture.transcribe(&engine).expect_err("재전사는 실패한다");

    assert_eq!(failure.kind, FailureKind::TranscriptionEngineFailed);
    let recording = fixture.recording();
    assert_eq!(
        recording.current_transcript_id.as_ref(),
        Some(&transcript_a.id),
        "실패한 시도 때문에 이미 유효한 Transcript를 잃지 않는다 (§7.2)"
    );
    assert_eq!(recording.transcription_status, ProcessingStatus::Failed);
    assert_eq!(
        fixture.transcripts().len(),
        1,
        "실패는 Transcript를 추가하지도 지우지도 않는다"
    );
    assert_eq!(
        fixture.transcript(&transcript_a.id),
        a_before,
        "A의 segments도 그대로다"
    );
    assert_eq!(
        a_before.segments.len(),
        2,
        "사전 조건: 비교 대상에 segment가 실제로 들어 있어야 한다"
    );
}

// --- 붕괴한 전사는 저장되지 않는다 (ADR-0007 §18.5) --------------------------------------

#[test]
fn a_collapsed_transcription_is_not_stored_and_leaves_the_previous_one_current() {
    // 2026-09-07: 붕괴한 전사가 `done`으로 저장됐다 (ADR-0007 §18.1). 저장 직전 검사가 물었던
    // 것이 개수 하나뿐이었기 때문이다. 여기서 고정하는 것은 그 자리의 다섯 가지 결과다.
    let mut fixture = Fixture::new("collapsed-retranscribe");
    let engine = StubEngine::responding_with(vec![Ok(first_run()), Ok(collapsed_run())]);

    let transcript_a = fixture
        .transcribe(&engine)
        .expect("첫 전사가 성공해야 한다")
        .transcript;
    let a_before = fixture.transcript(&transcript_a.id);
    let recording_before = fixture.recording();
    let audio_before = fs::read(&fixture.audio_path).expect("사전 조건: 원본을 읽는다");

    let failure = fixture
        .transcribe(&engine)
        .expect_err("붕괴한 출력은 전사 결과로 저장되지 않는다");

    // (1) 실패 종류는 §13의 넷째다 — 새 종류를 만들지 않는다 (ADR-0007 §18.4).
    assert_eq!(failure.kind, FailureKind::TranscriptionOutputUnusable);
    assert!(
        !failure.retryable,
        "같은 오디오를 같은 조건으로 다시 돌리면 같은 결과다"
    );
    assert!(failure.source_data_safe, "원본은 그대로다 (INV-1 · INV-3)");

    // (2) Transcript가 늘지 않았다 — 저장소에서 직접 센다.
    let stored = fixture.transcripts();
    assert_eq!(
        stored.len(),
        1,
        "붕괴한 전사는 Transcript를 추가하지 않는다 (§18.5)"
    );
    assert_eq!(stored[0].id, transcript_a.id, "남은 것은 이전 것 하나다");

    // (3) current가 그대로다 — 저장소에서 직접 읽는다.
    let recording = fixture.recording();
    assert_eq!(
        recording.current_transcript_id.as_ref(),
        Some(&transcript_a.id),
        "붕괴한 시도 때문에 이미 유효한 Transcript를 잃지 않는다 (§7.2)"
    );

    // (4) 상태는 `failed`다 — 붕괴가 사용자에게 보인다 (§13).
    assert_eq!(recording.transcription_status, ProcessingStatus::Failed);

    // (5) 원본 오디오 · Recording 레코드 · 이미 있던 Transcript는 그대로다 (INV-1 · INV-2 · INV-3).
    assert_eq!(
        fixture.transcript(&transcript_a.id),
        a_before,
        "이미 저장된 Transcript를 고치지도 지우지도 않는다 (INV-2)"
    );
    assert_eq!(
        a_before.segments.len(),
        2,
        "사전 조건: 비교 대상에 segment가 실제로 들어 있어야 한다"
    );
    assert_eq!(
        fs::read(&fixture.audio_path).expect("원본을 다시 읽는다"),
        audio_before,
        "붕괴는 원본 오디오를 건드리지 않는다 (INV-1)"
    );
    assert_eq!(recording.audio_path, recording_before.audio_path);
    assert_eq!(recording.title, recording_before.title);
    assert_eq!(recording.duration_ms, recording_before.duration_ms);
    assert_eq!(
        recording.ai_status, recording_before.ai_status,
        "남의 파이프라인 상태를 옮기지 않는다"
    );
    assert_eq!(recording.notion_status, recording_before.notion_status);
}

#[test]
fn a_collapsed_transcription_reaches_the_user_saying_what_went_wrong_and_by_how_much() {
    // 첫 전사가 붕괴한 경우다 — 지킬 이전 Transcript조차 없다.
    let mut fixture = Fixture::new("collapsed-first-run");
    fixture.record_status_writes();
    let engine = StubEngine::returning(collapsed_run());

    let failure = fixture
        .transcribe(&engine)
        .expect_err("붕괴한 출력은 전사 결과로 저장되지 않는다");

    assert!(
        fixture.transcripts().is_empty(),
        "붕괴한 첫 전사는 Transcript를 남기지 않는다"
    );
    assert!(
        fixture.recording().current_transcript_id.is_none(),
        "붕괴가 current를 만들지 않는다"
    );
    assert_eq!(fixture.stored_statuses(), ["pending", "running", "failed"]);

    // 사용자가 읽는 문장이 **무엇이 잘못됐는지** 말한다.
    assert!(
        failure.message.contains("붕괴"),
        "그대로 화면에 띄울 수 있는 문장이어야 한다: {}",
        failure.message
    );
    // 그리고 판정을 만든 수치를 담는다 (ADR-0007 §18.3) — 사람이 §18.1의 표를 손으로 다시
    // 세지 않아도 되게.
    for number in ["103개", "2개", "102번"] {
        assert!(
            failure.message.contains(number),
            "판정을 만든 수치 {number}이(가) 문장에 없다: {}",
            failure.message
        );
    }
    assert!(
        failure.message.contains("한글자막 by 한효정"),
        "무엇이 되풀이됐는지가 남아야 한다: {}",
        failure.message
    );

    // 기술적 표현에는 판정을 재현할 수 있는 값 전부가 남는다. 비율은 판정 모듈이 낸 값이다.
    let detail = failure.detail.expect("붕괴 실패는 수치를 detail에도 남긴다");
    for value in [
        "segments=103",
        "n=103",
        "u=2",
        "uniqueRatio=0.019",
        "r=102",
        "topRepeatShare=0.990",
        "Collapsed",
    ] {
        assert!(detail.contains(value), "{value}이(가) 없다: {detail}");
    }
}

#[test]
fn a_transcription_that_merely_repeats_a_little_is_still_stored() {
    // 붕괴 판정이 정상 전사를 버리기 시작하면 그 대가는 사람이 실제로 녹음한 회의다
    // (ADR-0007 §18.3). 고유 94% · 최다 7%는 2026-09-05에 사람이 쓸 수 있다고 판정한 값이다.
    let mut fixture = Fixture::new("usable-with-repeats");
    let mut segments: Vec<RawSegment> = (0..94)
        .map(|index| thirty_second_segment(index, &format!("문장 {index}")))
        .collect();
    segments.extend((94..100).map(|index| thirty_second_segment(index, "문장 0")));
    let engine = StubEngine::returning(RawTranscription {
        language: Some("ko".to_owned()),
        segments,
    });

    let completed = fixture
        .transcribe(&engine)
        .expect("반복이 조금 있다고 전사를 버리지 않는다");

    // **저장된다** — 그것이 이 테스트가 지키는 것이다. 다만 끝의 여섯 개는 같은 문장이
    // 연속이므로 §20.6.1의 차단이 네 번째부터 셋을 버린다 (2026-09-07에 실행 경로에 연결됐다).
    // 판정(`assess`)이 본 수치는 여전히 **차단 전**의 100개이며, 그래서 붕괴로 읽히지 않았다.
    assert_eq!(completed.transcript.segments.len(), 97);
    assert_eq!(completed.removed_segments, 3, "연속 반복 넷째부터 버린다");
    assert_eq!(
        completed.shortened_segments, 0,
        "segment 안쪽에는 되풀이가 없다"
    );
    assert_eq!(fixture.transcripts().len(), 1, "그대로 저장된다");
    assert_eq!(
        fixture.recording().current_transcript_id.as_ref(),
        Some(&completed.transcript.id)
    );
    assert_eq!(
        fixture.recording().transcription_status,
        ProcessingStatus::Done
    );
}

#[test]
fn the_orchestration_neither_copies_the_collapse_rule_nor_drops_the_empty_check() {
    // 붕괴 판정은 빈 결과 판정을 **대체하지 않고 더해진다** (ADR-0007 §18.5).
    assert!(
        RUN_SOURCE.contains("전사 결과에 남은 문장이 없다"),
        "빈 결과 판정이 사라지면 안 된다"
    );
    assert!(
        RUN_SOURCE.contains("collapse::assess"),
        "붕괴 여부는 판정 모듈에 물어본다 — 여기서 다시 판정하지 않는다"
    );

    // 임계값도 비율도 이 모듈이 만들지 않는다 (ADR-0007 §18.3). 규칙이 두 자리에 있으면
    // 한쪽만 고쳐지는 날이 온다.
    for forbidden in [
        "0.20",
        "0.50",
        "as f64",
        "MINIMUM_SENTENCES_TO_JUDGE",
        "COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW",
        "COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE",
    ] {
        assert!(
            !RUN_SOURCE.contains(forbidden),
            "판정 규칙이 orchestration으로 복제됐다: {forbidden}"
        );
    }

    // 그리고 저장된 Transcript를 고치거나 지우는 경로가 새로 생기지 않았다 (INV-2).
    // 이 모듈이 Transcript에 대해 부르는 저장소 함수는 여전히 둘뿐이다.
    for forbidden in ["update_transcript", "delete_transcript", "remove_transcript"] {
        assert!(
            !RUN_SOURCE.contains(forbidden),
            "저장된 Transcript를 고치거나 지우는 경로가 생겼다: {forbidden}"
        );
    }
}

// --- 실패가 원본을 건드리지 않는다 (INV-1 · INV-3) --------------------------------------

#[test]
fn a_failed_run_leaves_the_original_audio_file_and_the_recording_record_intact() {
    let mut fixture = Fixture::new("source-intact");
    let audio_before = fs::read(&fixture.audio_path).expect("사전 조건: 원본을 읽는다");
    let recording_before = fixture.recording();
    let engine = StubEngine::failing(engine_failed("엔진이 죽었다"));

    let failure = fixture.transcribe(&engine).expect_err("전사는 실패한다");

    assert!(failure.source_data_safe, "§13의 두 번째 질문에 답한다");
    assert!(
        fixture.audio_path.is_file(),
        "원본 오디오 파일이 그대로 있어야 한다 (INV-1)"
    );
    assert_eq!(
        fs::read(&fixture.audio_path).expect("원본을 다시 읽는다"),
        audio_before,
        "원본의 바이트도 그대로다 — 전사가 원본을 변환해 덮어쓰지 않는다"
    );

    let after = fixture.recording();
    assert_eq!(after.id, recording_before.id);
    assert_eq!(after.title, recording_before.title);
    assert_eq!(after.created_at, recording_before.created_at);
    assert_eq!(after.duration_ms, recording_before.duration_ms);
    assert_eq!(after.audio_path, recording_before.audio_path);
    assert_eq!(after.audio_format, recording_before.audio_format);
    assert_eq!(after.microphone, recording_before.microphone);
    assert_eq!(after.ai_status, recording_before.ai_status, "남의 상태를 옮기지 않는다");
    assert_eq!(after.notion_status, recording_before.notion_status);
    assert_eq!(
        after.transcription_status,
        ProcessingStatus::Failed,
        "바뀌는 것은 전사 상태뿐이다 (INV-3)"
    );
}

#[test]
fn nothing_in_the_orchestration_can_remove_or_rewrite_the_source() {
    // 위 테스트가 지나간 경로만 안전한 것이 아니라, 그런 코드가 아예 없다.
    for forbidden in [
        "fs::remove",
        "fs::write",
        "File::create",
        "WavWriter",
        "delete_recording",
    ] {
        assert!(
            !RUN_SOURCE.contains(forbidden),
            "전사 orchestration에 {forbidden}이(가) 있으면 안 된다 (INV-1 · INV-3 · INV-4)"
        );
    }
}

#[test]
fn a_failed_run_can_be_retried_and_then_succeed() {
    let mut fixture = Fixture::new("retry");
    let engine = StubEngine::responding_with(vec![
        Err(engine_failed("첫 시도가 죽었다")),
        Ok(first_run()),
    ]);

    fixture.transcribe(&engine).expect_err("첫 시도는 실패한다");
    assert_eq!(fixture.recording().transcription_status, ProcessingStatus::Failed);

    let completed = fixture.transcribe(&engine).expect("같은 Recording을 다시 시도할 수 있다");

    assert_eq!(fixture.transcripts().len(), 1);
    assert_eq!(
        fixture.recording().current_transcript_id.as_ref(),
        Some(&completed.transcript.id)
    );
    assert_eq!(fixture.recording().transcription_status, ProcessingStatus::Done);
}

// --- 언어 선택 (ADR-0007 §17.1.4) ----------------------------------------------------

#[test]
fn the_language_choice_reaches_the_engine_exactly_as_it_was_given() {
    // orchestration은 언어를 해석하지 않는다 — 모델 값이 model::resolve로 지나가는 것처럼
    // 그대로 엔진 경계에 도달한다. 실제 whisper 없이 판정된다.
    let mut fixture = Fixture::new("language-passthrough");
    let engine = StubEngine::returning(first_run());

    fixture
        .transcribe_in(&engine, &LanguageChoice::Detect)
        .expect("고르지 않아도 전사는 돈다");
    fixture
        .transcribe_in(&engine, &LanguageChoice::Chosen("ko".to_owned()))
        .expect("고른 언어로도 전사는 돈다");

    let calls = engine.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].language,
        LanguageChoice::Detect,
        "고르지 않음은 영어가 아니라 감지다 (§17.1.4-1)"
    );
    assert_eq!(calls[1].language.chosen(), Some("ko"), "고른 값이 그대로 온다");
}

#[test]
fn the_stored_language_is_what_the_engine_reported_not_what_the_user_chose() {
    // §17.1.4-3 · §16.2: 감지를 켰다는 것이 "언제나 값이 있다"는 뜻이 아니고, 사용자가 골랐다는
    // 것이 "그 값이 결과다"라는 뜻도 아니다. Transcript에 남는 것은 엔진이 말한 것뿐이다.
    let mut fixture = Fixture::new("language-provenance");
    let engine = StubEngine::responding_with(vec![
        Ok(RawTranscription {
            // 엔진은 자기가 들은 것을 말한다. 아래에서 고른 값은 "ja"다.
            language: Some("ko".to_owned()),
            segments: first_run().segments,
        }),
        Ok(RawTranscription {
            // 엔진이 언어를 말하지 못한 경우.
            language: None,
            segments: second_run().segments,
        }),
    ]);

    let chosen = LanguageChoice::Chosen("ja".to_owned());
    let first = fixture
        .transcribe_in(&engine, &chosen)
        .expect("첫 전사가 성공해야 한다");
    let second = fixture
        .transcribe_in(&engine, &chosen)
        .expect("재전사가 성공해야 한다");

    assert_eq!(
        first.transcript.language.as_deref(),
        Some("ko"),
        "설정 값(ja)이 아니라 엔진이 보고한 값(ko)이 남는다"
    );
    assert_eq!(
        second.transcript.language, None,
        "엔진이 말하지 못하면 지어내지 않는다 — 고른 값을 대신 적지도 않는다"
    );
    assert_eq!(
        fixture.transcript(&second.transcript.id).language,
        None,
        "저장된 것도 같다"
    );
}

// --- 상태 전이 (§7 · `phase-prompt/03` 요구 3) -------------------------------------------

#[test]
fn a_successful_run_stores_pending_then_running_then_done() {
    let mut fixture = Fixture::new("statuses-done");
    fixture.record_status_writes();
    let engine = StubEngine::returning(first_run());

    fixture.transcribe(&engine).expect("전사가 성공해야 한다");

    assert_eq!(
        fixture.stored_statuses(),
        ["pending", "running", "done"],
        "중간 상태를 건너뛰지 않는다 — 화면이 읽는 값이다"
    );
}

#[test]
fn a_failed_run_stores_pending_then_running_then_failed() {
    let mut fixture = Fixture::new("statuses-failed");
    fixture.record_status_writes();
    let engine = StubEngine::failing(engine_failed("엔진이 죽었다"));

    fixture.transcribe(&engine).expect_err("전사는 실패한다");

    assert_eq!(fixture.stored_statuses(), ["pending", "running", "failed"]);
}

#[test]
fn a_missing_model_reaches_the_user_together_with_the_failed_status() {
    // 모델이 없는 것은 조용한 skip이 아니라 §13의 실패다. 그 사실이 상태로도 남는다.
    let mut fixture = Fixture::without_model("missing-model");
    fixture.record_status_writes();
    let engine = StubEngine::returning(first_run());

    let failure = fixture.transcribe(&engine).expect_err("모델이 없으면 실패한다");

    assert_eq!(failure.kind, FailureKind::TranscriptionModelMissing);
    assert!(!failure.retryable, "모델을 먼저 둬야 풀린다");
    assert!(failure.source_data_safe, "원본은 그대로다 (INV-3)");
    assert!(
        !failure.message.is_empty(),
        "그대로 화면에 띄울 수 있는 문장이어야 한다"
    );
    assert_eq!(engine.call_count(), 0, "모델 없이 엔진을 부르지 않는다");
    assert_eq!(fixture.stored_statuses(), ["pending", "running", "failed"]);
    assert_eq!(fixture.recording().transcription_status, ProcessingStatus::Failed);
    assert!(
        fixture.audio_path.is_file(),
        "모델이 없다고 원본을 건드리지 않는다"
    );
}

#[test]
fn an_unknown_recording_changes_nothing_at_all() {
    let mut fixture = Fixture::new("unknown");
    fixture.record_status_writes();
    let engine = StubEngine::returning(first_run());

    let failure = run::transcribe(
        &mut fixture.connection,
        &RecordingId::new("없는-녹음"),
        &engine,
        ModelChoice {
            models_dir: fixture.dir.path(),
            configured: Some("ggml-base.bin"),
        },
        &LanguageChoice::Detect,
    )
    .expect_err("없는 Recording은 전사할 수 없다");

    assert_eq!(failure.kind, FailureKind::InvalidInput);
    assert!(
        fixture.stored_statuses().is_empty(),
        "상태를 쓸 대상이 없으므로 아무 행도 건드리지 않는다"
    );
    assert_eq!(
        fixture.recording().transcription_status,
        ProcessingStatus::None,
        "다른 녹음의 상태도 그대로다"
    );
}

#[test]
fn a_recording_whose_audio_file_is_gone_fails_without_touching_the_record() {
    // 파일은 앱 밖에서 옮겨지거나 지워질 수 있다. 그때도 레코드는 그대로 남는다 (INV-3 · INV-4).
    let mut fixture = Fixture::new("missing-audio");
    fs::remove_file(&fixture.audio_path).expect("사전 조건: 원본을 앱 밖에서 지운다");
    let engine = StubEngine::returning(first_run());

    let failure = fixture.transcribe(&engine).expect_err("입력을 만들 수 없다");

    assert!(failure.source_data_safe);
    assert_eq!(engine.call_count(), 0, "입력 없이 엔진을 부르지 않는다");
    assert_eq!(fixture.recording().transcription_status, ProcessingStatus::Failed);
    assert!(
        fixture.recording().current_transcript_id.is_none(),
        "실패가 current를 만들지 않는다"
    );
    assert!(fixture.transcripts().is_empty());
}
