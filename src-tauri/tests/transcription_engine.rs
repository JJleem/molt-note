//! 전사 실행 경계: 네 가지 제품 실패의 구분 · 실제 whisper 없이 도는 검증 · §12의 경계.
//!
//! **이 파일은 whisper 바이너리도 모델 파일도 요구하지 않는다** (`phase-prompt/03` 요구 9 ·
//! §18). 실제 엔진(`transcription::whisper`)은 여기서 한 번도 호출되지 않고, 그 자리에는
//! 계약이 같은 test double이 선다. 모델이 없는 경우는 **skip 조건이 아니라 검증 대상**이다 —
//! 모델 없음은 §13의 정의된 실패이기 때문이다.
//!
//! 통합 테스트이므로 crate의 공개 API만 쓴다.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use molt_note_lib::domain::FailureKind;
use molt_note_lib::transcription::audio_input::{TARGET_CHANNELS, TARGET_SAMPLE_RATE_HZ};
use molt_note_lib::transcription::engine::{engine_failed, TranscriptionEngine};
use molt_note_lib::transcription::model;
use molt_note_lib::transcription::testing::StubEngine;
use molt_note_lib::transcription::{ModelFile, RawSegment, RawTranscription, TranscriptionInput};

/// §12 · §11: 이 경계가 무엇을 하지 않는지는 소스에서도 확인한다.
const ENGINE_SOURCE: &str = include_str!("../src/transcription/engine.rs");
const MODEL_SOURCE: &str = include_str!("../src/transcription/model.rs");
const WHISPER_SOURCE: &str = include_str!("../src/transcription/whisper.rs");
const TESTING_SOURCE: &str = include_str!("../src/transcription/testing.rs");
const GITIGNORE: &str = include_str!("../../.gitignore");
const TAURI_CONF: &str = include_str!("../tauri.conf.json");
const CAPABILITIES: &str = include_str!("../capabilities/default.json");

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// 시스템 임시 디렉터리 아래의 고유 디렉터리. Drop 시 지운다.
///
/// **모델 파일도 오디오도 저장소에 커밋하지 않는다** (ADR-0007 §8.3). 이 테스트가 쓰는
/// "모델"은 여기서 만드는 몇 바이트짜리 파일이다.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = format!(
            "molt-note-transcription-engine-{}-{}-{}",
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

fn placeholder_model(directory: &Path) -> ModelFile {
    fs::write(directory.join("ggml-base.bin"), b"not a real model")
        .expect("사전 조건: 자리표시자 모델을 만든다");
    model::resolve(directory, Some("ggml-base.bin")).expect("해석할 수 있어야 한다")
}

/// 0.1초짜리 무음 파생 입력. 엔진에 무엇이 넘어가는지 보기 위한 것이다.
fn derived_input() -> TranscriptionInput {
    TranscriptionInput {
        samples: vec![0.0; 1_600],
        sample_rate_hz: TARGET_SAMPLE_RATE_HZ,
        channels: TARGET_CHANNELS,
        source_sample_rate_hz: 48_000,
        source_channels: 2,
        resampled: true,
        downmixed: true,
    }
}

/// 엔진이 냈다고 가정하는 원시 출력. **센티초다** (ADR-0007 §10).
fn raw_output() -> RawTranscription {
    RawTranscription {
        language: Some("ko".to_owned()),
        segments: vec![
            RawSegment {
                start_centiseconds: 13_400,
                end_centiseconds: 14_100,
                text: Some(" 그러면 이번에는 PLY 먼저 변환하고".to_owned()),
            },
            RawSegment {
                start_centiseconds: 14_100,
                end_centiseconds: 14_800,
                text: Some(" 그다음 SOG 변환 확인하면 될 것 같아요.".to_owned()),
            },
        ],
    }
}

#[test]
fn a_normal_run_returns_the_engine_output_untouched() {
    // 실행 경계는 값을 옮기기만 한다 — 단위 변환은 parse가 한다 (ADR-0007 §10).
    let temp = TempDir::new("normal");
    let model = placeholder_model(temp.path());
    let engine = StubEngine::returning(raw_output());

    let output = engine
        .transcribe(&derived_input(), &model)
        .expect("정상 출력이어야 한다");

    assert_eq!(output, raw_output(), "경계가 값을 바꾸지 않는다");
    assert_eq!(
        output.segments[0].start_centiseconds, 13_400,
        "센티초 그대로다 — 밀리초로 바꾸는 것은 이 경계의 일이 아니다"
    );
    assert_eq!(engine.call_count(), 1);
    assert_eq!(engine.calls()[0].model_id, "ggml-base.bin");
    assert_eq!(engine.calls()[0].frames, 1_600, "파생 입력이 그대로 넘어간다");
}

#[test]
fn a_missing_model_is_a_defined_failure_rather_than_a_skipped_test() {
    // 자동 검증이 모델 유무에 따라 갈라지지 않는다. 모델이 없는 것은 제품 상태다 (§13).
    let temp = TempDir::new("missing-model");

    let not_configured = model::resolve(temp.path(), None).expect_err("고르지 않은 상태");
    let not_there =
        model::resolve(temp.path(), Some("ggml-base.bin")).expect_err("그 자리에 없는 상태");

    for failure in [&not_configured, &not_there] {
        assert_eq!(failure.kind, FailureKind::TranscriptionModelMissing);
        assert!(!failure.retryable, "모델을 먼저 둬야 풀린다");
        assert!(failure.source_data_safe, "원본은 그대로다 (INV-3)");
    }
}

#[test]
fn an_unusable_model_is_told_apart_from_a_missing_one() {
    let temp = TempDir::new("unusable-model");
    fs::create_dir(temp.path().join("ggml-directory.bin")).expect("사전 조건: 디렉터리");
    fs::write(temp.path().join("ggml-empty.bin"), b"").expect("사전 조건: 빈 파일");

    for name in ["ggml-directory.bin", "ggml-empty.bin"] {
        let failure = model::resolve(temp.path(), Some(name)).expect_err("쓸 수 없는 모델");

        assert_eq!(
            failure.kind,
            FailureKind::TranscriptionModelUnusable,
            "{name}: 없는 것과 쓸 수 없는 것은 사용자가 할 일이 다르다"
        );
    }
}

#[test]
fn an_engine_that_fails_is_reported_as_a_retryable_product_failure() {
    let temp = TempDir::new("engine-failure");
    let model = placeholder_model(temp.path());
    let engine = StubEngine::failing(
        engine_failed("전사 도중 엔진이 실패했다").with_detail("SIGABRT"),
    );

    let failure = engine
        .transcribe(&derived_input(), &model)
        .expect_err("실행 실패여야 한다");

    assert_eq!(failure.kind, FailureKind::TranscriptionEngineFailed);
    assert!(failure.retryable, "원본이 남아 있으므로 다시 만들 수 있다 (INV-1)");
    assert!(failure.source_data_safe);
    assert_eq!(failure.detail.as_deref(), Some("SIGABRT"));
}

#[test]
fn output_that_cannot_be_read_as_a_transcript_is_its_own_failure() {
    let temp = TempDir::new("empty-output");
    let model = placeholder_model(temp.path());
    let engine = StubEngine::returning(RawTranscription {
        language: Some("ko".to_owned()),
        segments: vec![RawSegment {
            start_centiseconds: 0,
            end_centiseconds: 0,
            text: None,
        }],
    });

    let failure = engine
        .transcribe(&derived_input(), &model)
        .expect_err("해석할 수 있는 출력이 없다");

    assert_eq!(failure.kind, FailureKind::TranscriptionOutputUnusable);
    assert!(!failure.retryable, "같은 입력은 같은 출력을 낸다");
    assert!(failure.source_data_safe);
}

#[test]
fn the_four_transcription_failures_reach_the_screen_as_four_different_kinds() {
    // 화면은 넷을 구분해 안내해야 한다 (§13 · TASK-030). 하나로 뭉치면 그럴 수 없다.
    let temp = TempDir::new("four-kinds");
    fs::write(temp.path().join("ggml-empty.bin"), b"").expect("사전 조건: 빈 파일");
    let model = placeholder_model(temp.path());

    let kinds = [
        model::resolve(temp.path(), None)
            .expect_err("모델 없음")
            .kind,
        model::resolve(temp.path(), Some("ggml-empty.bin"))
            .expect_err("쓸 수 없는 모델")
            .kind,
        StubEngine::failing(engine_failed("죽었다"))
            .transcribe(&derived_input(), &model)
            .expect_err("실행 실패")
            .kind,
        StubEngine::returning(RawTranscription {
            language: None,
            segments: Vec::new(),
        })
        .transcribe(&derived_input(), &model)
        .expect_err("빈 출력")
        .kind,
    ];

    let mut seen: Vec<&str> = Vec::new();
    for kind in kinds {
        let text = kind.as_str();
        assert!(!seen.contains(&text), "전사 실패 종류가 겹친다: {text}");
        seen.push(text);
    }
    assert_eq!(seen.len(), 4);
}

#[test]
fn the_transcription_boundary_has_no_way_to_send_audio_anywhere() {
    // §12 · INV-6: 오디오도 전사 결과도 기기를 떠나지 않는다. 원격 전사 API를 쓰지 않는다.
    // 네트워크 클라이언트가 이 경계에 **들어오지 못하게** 한다 — 나중에 누군가 추가하면
    // 여기서 실패한다.
    let forbidden = [
        "reqwest", "ureq", "hyper", "TcpStream", "UdpSocket", "std::net", "curl", "socket",
    ];

    for (label, source) in [
        ("engine.rs", ENGINE_SOURCE),
        ("model.rs", MODEL_SOURCE),
        ("whisper.rs", WHISPER_SOURCE),
        ("testing.rs", TESTING_SOURCE),
    ] {
        for symbol in forbidden {
            assert!(
                !source.contains(symbol),
                "{label}에 네트워크 경로가 생겼다: {symbol}"
            );
        }
    }
}

#[test]
fn the_engine_runs_in_this_process_and_spawns_nothing() {
    // ADR-0007은 sidecar(A)를 택하지 않았다. 자식 프로세스를 실행하는 코드가 생기면
    // 바이너리 확보·배치·번들 설정이 통째로 따라오므로 그 사실이 조용히 들어오면 안 된다.
    for (label, source) in [
        ("engine.rs", ENGINE_SOURCE),
        ("model.rs", MODEL_SOURCE),
        ("whisper.rs", WHISPER_SOURCE),
    ] {
        assert!(
            !source.contains("process::Command"),
            "{label}에 프로세스 실행이 생겼다"
        );
    }

    // sidecar를 택하지 않았으므로 번들에 넣는 실행 파일도, 그것을 부를 shell 권한도 없다.
    assert!(
        !TAURI_CONF.contains("externalBin"),
        "sidecar를 쓰지 않는데 externalBin이 설정돼 있다"
    );
    assert!(
        !CAPABILITIES.contains("shell:"),
        "전사가 shell 권한을 요구하지 않는다"
    );
}

#[test]
fn no_platform_branch_was_pre_paid_for_windows() {
    // ADR-0007 §11 · §20.6: 이 Phase에서 플랫폼이 실제로 갈리는 지점은 모델 파일 위치
    // 하나이고, 그것은 이미 있는 AppDataDirectory 경계가 처리한다. 전사 경계에는 쓰지 않는
    // 플랫폼 분기를 미리 만들지 않는다.
    //
    // 검사 대상은 **분기 문법 자체**다. 문서가 "여기에는 `cfg(target_os)`가 없다"고 적는 것은
    // 분기가 아니므로 걸리지 않는다.
    for (label, source) in [
        ("engine.rs", ENGINE_SOURCE),
        ("model.rs", MODEL_SOURCE),
        ("whisper.rs", WHISPER_SOURCE),
        ("testing.rs", TESTING_SOURCE),
    ] {
        for branch in [
            "#[cfg(target_os",
            "#[cfg(windows",
            "#[cfg(unix",
            "cfg!(target_os",
            "cfg!(windows",
        ] {
            assert!(
                !source.contains(branch),
                "{label}에 플랫폼 분기가 생겼다: {branch}"
            );
        }
    }
}

#[test]
fn model_files_and_large_binaries_stay_out_of_the_repository() {
    // ADR-0007 §8.3: 모델은 466MiB~2.9GiB다. 한 번 커밋되면 히스토리에서 지워지지 않는다.
    for rule in ["/models/", "*.gguf", "*.bin"] {
        assert!(
            GITIGNORE.lines().any(|line| line.trim() == rule),
            ".gitignore에 규칙이 없다: {rule}"
        );
    }
}
