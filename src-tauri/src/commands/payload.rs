//! frontend와 주고받는 값의 모양.
//!
//! domain 타입을 그대로 내보내지 않고 이 계층에서 한 번 옮긴다. 이유는 두 가지다.
//!
//! 1. **wire 형식은 UI와의 계약이다.** 무엇이 나가고 들어오는지 한 파일에서 볼 수 있어야
//!    `src/ipc/types.ts`와 어긋났을 때 눈에 띈다.
//! 2. **domain은 직렬화 형식을 알지 않는다.** 저장소도 AI 벤더도 알지 않는 것과 같은 이유다
//!    (`crate::domain` 모듈 주석). 예외는 [`crate::domain::Failure`] 하나뿐이며,
//!    그것은 §13이 정의한 사용자 대면 계약 자체다.

use serde::{Deserialize, Serialize};

use crate::audio::{CaptureReport, InputDevice, SessionState, SessionSummary};
use crate::domain::{
    Failure, ProcessingStatus, Recording, RecordingId, RecordingView, Settings, Transcript,
    TranscriptSegment,
};

/// 조회된 녹음 하나. 목록 화면과 상세 화면이 그대로 쓴다 (§5 A · C).
///
/// `duration_label`은 Rust가 이미 만들어 보낸다 — 초를 `52:31`로 바꾸는 규칙이
/// TypeScript에 다시 구현되지 않게 하는 것이 이 필드의 목적이다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingPayload {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub duration_ms: i64,
    pub duration_label: String,
    pub audio_path: String,
    pub audio_format: String,
    pub microphone: Option<String>,
    /// 현재 사용 중인 Transcript. 값이 없는 상태도 정상이다 (§7.2).
    pub current_transcript_id: Option<String>,
    /// `none · pending · running · done · failed` 중 하나 (§7).
    pub transcription_status: String,
    pub ai_status: String,
    pub notion_status: String,
}

impl From<RecordingView> for RecordingPayload {
    fn from(view: RecordingView) -> Self {
        let RecordingView {
            recording,
            duration_label,
        } = view;
        Self {
            id: recording.id.as_str().to_string(),
            title: recording.title,
            created_at: recording.created_at,
            updated_at: recording.updated_at,
            duration_ms: recording.duration_ms,
            duration_label,
            audio_path: recording.audio_path,
            audio_format: recording.audio_format,
            microphone: recording.microphone,
            current_transcript_id: recording
                .current_transcript_id
                .map(|id| id.as_str().to_string()),
            transcription_status: recording.transcription_status.as_str().to_string(),
            ai_status: recording.ai_status.as_str().to_string(),
            notion_status: recording.notion_status.as_str().to_string(),
        }
    }
}

/// 새로 저장할 녹음. 식별자와 시각은 여기에 없다 — **Rust가 만든다.**
///
/// 후처리 상태도 받지 않는다. 새 녹음의 상태는 언제나 "아직 시도하지 않음"이며,
/// 그것을 프론트엔드가 다른 값으로 정할 수 있게 하지 않는다 (§7).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRecording {
    pub title: String,
    pub duration_ms: i64,
    pub audio_path: String,
    pub audio_format: String,
    /// 녹음에 쓴 입력 장치. 알 수 없으면 없어도 된다.
    #[serde(default)]
    pub microphone: Option<String>,
}

impl NewRecording {
    /// 저장할 [`Recording`]을 만든다. 식별자와 시각은 호출자가 저장소에서 받아 넘긴다.
    pub(super) fn into_recording(self, id: String, timestamp: String) -> Recording {
        Recording {
            id: RecordingId::new(id),
            title: self.title,
            created_at: timestamp.clone(),
            updated_at: timestamp,
            duration_ms: self.duration_ms,
            audio_path: self.audio_path,
            audio_format: self.audio_format,
            microphone: self.microphone,
            // 아직 전사가 없다. 값이 없는 상태가 정상이다 (§7.2).
            current_transcript_id: None,
            transcription_status: ProcessingStatus::None,
            ai_status: ProcessingStatus::None,
            notion_status: ProcessingStatus::None,
        }
    }
}

/// 고를 수 있는 입력 장치 하나.
///
/// `key`는 고를 때 쓰는 불투명한 값이고 `label`은 사람이 읽는 이름이다 —
/// 이름이 같은 장치가 둘 있을 수 있으므로 둘을 나눠서 보낸다
/// ([`crate::audio::devices`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDevicePayload {
    pub key: String,
    pub label: String,
    /// 시스템 기본 입력 장치인가. 기본 장치가 없는 목록도 정상이다.
    pub is_default: bool,
}

impl From<InputDevice> for InputDevicePayload {
    fn from(device: InputDevice) -> Self {
        Self {
            key: device.key,
            label: device.label,
            is_default: device.is_default,
        }
    }
}

/// 정지한 녹음 하나의 보고 값.
///
/// 장치 이름 · 출력 경로 · 포맷 · 파일 크기(byte)는 ADR-0003 §12가 사람에게 보여 주기로 한
/// 네 값 그대로다. 여기에 **녹음 길이**가 더해진다 — 일시정지 구간을 뺀 값이며, 그것을 세는
/// 곳은 [`crate::audio::RecordingSession`] 한 곳뿐이다.
///
/// `format`과 `duration_label`은 사람이 읽는 문장이고, 그 문장을 이루는 값도 따로 보낸다 —
/// 화면이 문자열을 다시 뜯어보거나 같은 계산을 TypeScript에 다시 만들지 않게 하기 위해서다
/// ([`RecordingPayload::duration_label`]과 같은 이유).
///
/// **이것은 저장된 Recording이 아니다.** 저장된 레코드는 [`StoppedRecordingPayload::recording`]
/// 쪽이며, 둘은 [`StoppedRecordingPayload`]에서 함께 온다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReportPayload {
    /// 실제로 열린 장치의 이름.
    pub device_label: String,
    /// 확정된 파일의 경로.
    pub output_path: String,
    /// 사람이 읽는 형식 문장(샘플레이트 · 채널 수 · 비트 심도 · 컨테이너).
    pub format: String,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub container: String,
    /// 파일시스템에서 읽은 파일 크기(byte).
    pub byte_size: u64,
    /// 일시정지 구간을 뺀 녹음 길이(밀리초).
    pub duration_ms: i64,
    /// Rust가 만든 표시용 길이(예: `52:31`).
    pub duration_label: String,
}

impl CaptureReportPayload {
    /// 확정된 파일에 대한 사실과 session이 센 길이를 한 값으로 합친다.
    ///
    /// 둘을 여기서 합치는 이유는 **각자가 답할 수 없는 질문이 있기 때문이다** — 파일은
    /// 자신이 몇 초짜리 녹음인지 모르고(일시정지 구간을 알지 못한다), session은 파일이
    /// 실제로 얼마나 쓰였는지 모른다.
    pub(super) fn new(report: CaptureReport, summary: SessionSummary) -> Self {
        Self {
            device_label: report.device_label,
            output_path: report.output_path.display().to_string(),
            format: report.format.describe(),
            sample_rate_hz: report.format.sample_rate_hz,
            channels: report.format.channels,
            bits_per_sample: report.format.bits_per_sample,
            container: report.format.container().to_string(),
            byte_size: report.byte_size,
            duration_ms: summary.duration_ms,
            duration_label: summary.duration_label,
        }
    }
}

/// 정지가 **성공했을 때** 돌아오는 값 (Phase 2B 요구사항 5 · 6 · R-002).
///
/// 두 가지가 함께 온다 — **저장된 레코드**와 **확정된 파일에 대한 사실**이다.
///
/// ```text
/// recording  목록에 나타나는 Recording 그 자체 (Phase 1의 저장소를 지나 저장됐다)
/// capture    그 레코드가 가리키는 파일에 대한 사실 (장치 이름 · 포맷 · 크기)
/// ```
///
/// 이 값이 돌아왔다는 것은 **파일이 확정되고 확인됐으며 레코드가 저장됐다**는 뜻이다.
/// 넷 중 하나라도 성립하지 않으면 이 값 대신 [`Failure`]가 간다 —
/// 그때 그 실패는 확정된 파일이 어디에 남아 있는지 함께 말한다 (INV-4).
///
/// [`Failure`]: crate::domain::Failure
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedRecordingPayload {
    /// 저장된 녹음. 목록 화면이 그대로 쓰는 값과 같은 모양이다.
    pub recording: RecordingPayload,
    /// 방금 확정된 파일에 대한 사실.
    pub capture: CaptureReportPayload,
}

/// **레코드는 있는데 오디오 파일이 없는** 녹음 하나 (Phase 2B 요구사항 6).
///
/// 이 값은 **보고일 뿐이다.** 이런 상태를 발견해도 레코드를 지우거나 고치지 않고, 파일을
/// 새로 만들지도 않는다 (INV-3 · INV-4). 무엇을 할지는 사용자가 정한다 — 이 앱이 대신
/// 정리하지 않는다는 것이 정책이다
/// (`docs/ADR-0004-recording-session-lifecycle.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingAudioPayload {
    pub recording_id: String,
    pub title: String,
    /// 레코드가 가리키고 있지만 지금 그 자리에 없는 경로.
    pub audio_path: String,
    pub created_at: String,
}

impl From<&RecordingPayload> for MissingAudioPayload {
    fn from(recording: &RecordingPayload) -> Self {
        Self {
            recording_id: recording.id.clone(),
            title: recording.title.clone(),
            audio_path: recording.audio_path.clone(),
            created_at: recording.created_at.clone(),
        }
    }
}

/// 지금 녹음이 어떤 상태인지 (Phase 2B 요구사항 4 · R-001).
///
/// 화면은 이 값을 **물어봐서** 안다. 진행 중인 session을 들고 있는 것은 backend이므로,
/// 화면이 다시 그려지거나 사용자가 다른 화면에 다녀와도 여기서 같은 답이 나온다
/// (`docs/ADR-0004-recording-session-lifecycle.md`).
///
/// **길이는 Rust가 세고 Rust가 문장까지 만든다.** `elapsed_ms`와 `elapsed_label`을 함께
/// 보내는 이유가 그것이다 — TypeScript에 길이 계산을 만들지 않는다
/// (`tests/screen-boundary.test.ts` · [`RecordingPayload::duration_label`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusPayload {
    /// `idle · recording · paused · stopped` 중 하나.
    pub state: String,
    /// 일시정지 구간을 뺀, 지금까지의 녹음 길이(밀리초).
    pub elapsed_ms: i64,
    /// 같은 길이를 사람이 읽는 문장으로 (예: `0:07`).
    pub elapsed_label: String,
}

impl SessionStatusPayload {
    /// 상태와 경과 시간 하나를 값으로 만든다.
    pub(super) fn new(state: SessionState, elapsed_ms: i64, elapsed_label: String) -> Self {
        Self {
            state: state.as_str().to_string(),
            elapsed_ms,
            elapsed_label,
        }
    }
}

/// 지금 전사가 어떤 상태인지 (`phase-prompt/03` 요구 3).
///
/// [`SessionStatusPayload`]와 같은 자리에 있는 값이다 — **진행 중인 전사를 들고 있는 것은
/// backend이고 화면은 그것을 물어본다.** 그래서 화면이 다시 그려지거나 사용자가 다른 화면에
/// 다녀와도 여기서 같은 답이 나온다 ([`crate::commands::Transcriber`]).
///
/// 네 상태는 서로 배타적이며, 상태마다 값이 있는 필드가 다르다.
///
/// ```text
/// state      recordingId   transcriptId   failure
/// idle       null          null           null
/// running    있음          null           null
/// done       있음          있음           null
/// failed     있음          null           있음
/// ```
///
/// **실패는 [`Failure`] 그대로 실려 온다.** §13의 세 질문에 대한 답이 이미 그 값 안에 있으므로
/// 여기서 문장을 새로 만들거나 종류를 뭉개지 않는다 — 모델이 없는 것과 엔진이 죽은 것은
/// 사용자가 할 일이 다르고, 그 구분이 화면까지 그대로 도달해야 한다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionStatusPayload {
    /// `idle · running · done · failed` 중 하나.
    ///
    /// **[`RecordingPayload::transcription_status`]와 다른 값이다.** 저쪽은 녹음 하나에 저장된
    /// 후처리 상태(§7)이고, 이쪽은 지금 이 앱이 실제로 돌리고 있는 전사 한 건이다.
    pub state: String,
    /// 지금 전사 중이거나 마지막으로 전사한 녹음. 아무것도 하지 않았으면 없다.
    pub recording_id: Option<String>,
    /// 성공했을 때 **추가된** Transcript (§7.1). 그 시점에 이미 current다 (§7.2).
    pub transcript_id: Option<String>,
    /// 실패했을 때 그 실패 그대로.
    pub failure: Option<Failure>,
}

impl TranscriptionStatusPayload {
    /// 진행 중인 전사도, 방금 끝난 전사도 없다. **오류가 아니라 정상 상태다.**
    pub(super) fn idle() -> Self {
        Self {
            state: "idle".to_string(),
            recording_id: None,
            transcript_id: None,
            failure: None,
        }
    }

    /// 지금 이 녹음을 전사하고 있다.
    pub(super) fn running(recording_id: &str) -> Self {
        Self {
            state: "running".to_string(),
            recording_id: Some(recording_id.to_string()),
            ..Self::idle()
        }
    }

    /// 전사가 끝났고 Transcript가 하나 추가됐다.
    pub(super) fn done(recording_id: &str, transcript_id: &str) -> Self {
        Self {
            state: "done".to_string(),
            transcript_id: Some(transcript_id.to_string()),
            ..Self::running(recording_id)
        }
    }

    /// 전사가 실패했다. 원본 오디오도 Recording도 그대로 남아 있다 (INV-1 · INV-3).
    pub(super) fn failed(recording_id: &str, failure: &Failure) -> Self {
        Self {
            state: "failed".to_string(),
            failure: Some(failure.clone()),
            ..Self::running(recording_id)
        }
    }
}

/// Transcript 안의 구간 하나 (§7의 `segments[] { start · end · text }`).
///
/// **밀리초로 나간다.** 엔진마다 다른 단위(CLI JSON은 밀리초 · `whisper-rs`는 센티초)를
/// 하나로 맞추는 자리는 통합 경계 한 곳뿐이며 (`crate::transcription::parse`), 화면까지 오는
/// 값은 이미 정규화된 밀리초다. 여기서 다시 나누거나 곱하지 않는다.
///
/// `00:02:14 → 00:02:21` 같은 **표시 문자열은 만들지 않는다.** 두 값을 어떤 형태로 보여줄지는
/// 화면의 문제이며 그 규칙은 `src/screens/transcriptView.ts` 한 곳에 있다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegmentPayload {
    /// 녹음 시작 기준 오프셋(밀리초).
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

impl From<TranscriptSegment> for TranscriptSegmentPayload {
    fn from(segment: TranscriptSegment) -> Self {
        Self {
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            text: segment.text,
        }
    }
}

/// 전사 결과 하나 (§7). Recording Detail의 Transcript 탭이 그대로 쓴다
/// (`phase-prompt/03` 요구 6).
///
/// **읽기 전용 값이다.** 이 payload를 되돌려받는 command는 없다 — Transcript는 immutable이며
/// (§7.1 · INV-2), 저장소가 내놓는 쓰기 경로도 추가([`crate::db::store::append_transcript`])
/// 하나뿐이다. 그래서 화면이 이미 저장된 Transcript를 고치거나 지우는 경로는 만들어질 수 없다.
///
/// `language`가 없는 것도 정상이다 — 엔진이 언어를 말하지 않았다는 사실이며, 추측해서
/// 채우지 않는다. `engine`과 `model`은 provenance다 (§7): 이 문장들이 **무엇으로 만들어졌는지**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptPayload {
    /// Recording과 독립적인 identity (§7.1).
    pub id: String,
    pub recording_id: String,
    /// 감지·지정된 언어. 모르면 없다.
    pub language: Option<String>,
    /// 저장된 순서 그대로. 화면이 다시 정렬하지 않는다.
    pub segments: Vec<TranscriptSegmentPayload>,
    pub raw_text: String,
    pub created_at: String,
    /// 전사에 실제로 쓴 엔진 식별자.
    pub engine: String,
    /// 전사에 실제로 쓴 모델 식별자.
    pub model: String,
}

impl From<Transcript> for TranscriptPayload {
    fn from(transcript: Transcript) -> Self {
        Self {
            id: transcript.id.as_str().to_string(),
            recording_id: transcript.recording_id.as_str().to_string(),
            language: transcript.language,
            segments: transcript
                .segments
                .into_iter()
                .map(TranscriptSegmentPayload::from)
                .collect(),
            raw_text: transcript.raw_text,
            created_at: transcript.created_at,
            engine: transcript.engine,
            model: transcript.model,
        }
    }
}

/// 설정 값. 조회와 갱신이 같은 모양을 쓴다 — 화면이 읽은 것을 그대로 돌려보낼 수 있다.
///
/// **INV-7: secret 필드가 없다.** API key · integration token을 담는 자리를 만들지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPayload {
    /// 고르지 않은 상태(`null`)도 정상이다.
    #[serde(default)]
    pub recordings_directory: Option<String>,
    pub automatic_processing: bool,
    /// 정지해 저장한 직후에 전사를 자동으로 시작할지 여부 (`phase-prompt/03` 요구 4).
    ///
    /// **[`Self::automatic_processing`]과 별개의 값이다.** 두 토글은 각자 저장되고 각자
    /// 돌아온다 — 한쪽을 켜 보낸다고 다른 쪽이 켜지지 않는다.
    pub automatic_transcription: bool,
    /// 전사에 쓸 모델 파일의 이름 또는 경로 (ADR-0007 §8.2). 고르지 않은 상태(`null`)도
    /// 정상이다.
    ///
    /// **secret이 아니다** — 파일이 어디 있는지일 뿐이며, 그래서 INV-7과 충돌하지 않는다.
    /// 이 값이 가리키는 파일이 지금 실재하는지는 여기서 말하지 않는다. 그것을 아는 자리는
    /// `crate::transcription::model` 하나이고, 없으면 §13의 정의된 실패로 드러난다.
    #[serde(default)]
    pub transcription_model: Option<String>,
    /// 기본으로 고를 입력 장치의 **선택 키** ([`InputDevicePayload::key`]).
    ///
    /// 고르지 않은 상태(`null`)도 정상이다. 이 값이 가리키는 장치가 지금 목록에 있는지는
    /// 여기서 말하지 않는다 — 목록과 함께 봐야 알 수 있고, 그 해석은 화면 쪽에 있다
    /// (`src/screens/defaultMicrophone.ts`).
    #[serde(default)]
    pub default_microphone: Option<String>,
}

impl From<Settings> for SettingsPayload {
    fn from(settings: Settings) -> Self {
        Self {
            recordings_directory: settings.recordings_directory,
            automatic_processing: settings.automatic_processing,
            automatic_transcription: settings.automatic_transcription,
            transcription_model: settings.transcription_model,
            default_microphone: settings.default_microphone,
        }
    }
}

impl From<SettingsPayload> for Settings {
    fn from(payload: SettingsPayload) -> Self {
        Self {
            // 빈 문자열이나 공백은 디렉터리가 아니다. 그런 값을 그대로 저장해서
            // "고르지 않음"과 구분되지 않는 세 번째 상태를 만들지 않는다 (§5 D).
            recordings_directory: payload
                .recordings_directory
                .map(|directory| directory.trim().to_string())
                .filter(|directory| !directory.is_empty()),
            automatic_processing: payload.automatic_processing,
            // 두 토글은 서로를 보지 않는다. 자동 전사를 켰다고 자동 후처리가 켜지지 않는다.
            automatic_transcription: payload.automatic_transcription,
            // 공백만 있는 값은 모델 이름이 아니다 — '고르지 않음'과 구분되지 않는 세 번째
            // 상태를 만들지 않는다. **그것뿐이다**: 이 자리에서 모델을 찾아보지도, 없는
            // 값을 다른 모델로 바꾸지도 않는다 (ADR-0007 §8.2.3).
            transcription_model: payload
                .transcription_model
                .map(|model| model.trim().to_string())
                .filter(|model| !model.is_empty()),
            // 같은 이유로 빈 선택은 '고르지 않음'이다. **다만 그것뿐이다** — 알아볼 수 없는
            // 키가 와도 여기서 다른 장치로 바꾸지 않는다. 저장된 값과 지금 있는 장치를
            // 맞춰 보는 일은 목록을 아는 쪽의 일이고, 그 결과는 값으로 구분된다.
            default_microphone: payload
                .default_microphone
                .map(|key| key.trim().to_string())
                .filter(|key| !key.is_empty()),
        }
    }
}
