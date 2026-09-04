//! frontend와 주고받는 값의 모양.
//!
//! domain 타입을 그대로 내보내지 않고 이 계층에서 한 번 옮긴다. 이유는 두 가지다.
//!
//! 1. **wire 형식은 UI와의 계약이다.** 무엇이 나가고 들어오는지 한 파일에서 볼 수 있어야
//!    `src/ipc/types.ts`와 어긋났을 때 눈에 띈다.
//! 2. **domain은 직렬화 형식을 알지 않는다.** 저장소도 AI 벤더도 알지 않는 것과 같은 이유다
//!    (`crate::domain` 모듈 주석). 예외는 [`crate::domain::Failure`] 하나뿐이며,
//!    그것은 §13이 정의한 사용자 대면 계약 자체다.
//!
//! 노트 본문 세 타입([`crate::ai::note`])도 그 예외와 같은 성질을 갖는다 — 그 필드 이름은
//! §9.5의 **출력 섹션 이름 13개** 자체이며 (ADR-0008 §7.1), 같은 이름을 여기 다시 적으면
//! 저장된 노트와 화면이 조용히 어긋날 수 있는 자리가 하나 늘어난다. 그 타입들에 벤더는 없다
//! (INV-9).

use serde::{Deserialize, Serialize};

use crate::ai::note::{decode_content, MeetingNote, StructuredNote, StudyNote, SummaryNote};
use crate::ai::provider::{Availability, ProviderDescriptor};
use crate::audio::{CaptureReport, InputDevice, SessionState, SessionSummary};
use crate::domain::{
    AiNote, Failure, NoteType, NotionSync, ProcessingStatus, Recording, RecordingId, RecordingView,
    Settings, Transcript, TranscriptSegment,
};

use super::notion::NotionSendStatus;

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
    /// 노트를 만들 때 쓸 AI provider의 식별자 (docs/ADR-0008-note-ai-provider.md §11.1).
    ///
    /// **고르지 않은 상태(`null`)가 기본이고 그것도 정상이다** — 화면은 그것을 오류가 아니라
    /// "AI 기능이 아직 켜지지 않았다"로 그린다 (INV-8). 벤더 중립 자유 식별자이므로 이 경계는
    /// 알려진 목록과 대조하지 않는다 (INV-9).
    #[serde(default)]
    pub ai_provider: Option<String>,
    /// AI provider에 연결할 주소. 고르지 않은 상태(`null`)도 정상이며, 그때 무엇을 쓰는지는
    /// backend가 안다 (`Settings::ai_base_url_or_default`).
    ///
    /// **secret이 아니다** — 어디에 연결하는지일 뿐이며, 그래서 INV-7과 충돌하지 않는다.
    #[serde(default)]
    pub ai_base_url: Option<String>,
    /// 노트를 만들 때 쓸 모델 식별자. 고르지 않은 상태(`null`)도 정상이다.
    ///
    /// 이 모델이 지금 그 서버에 있는지는 여기서 말하지 않는다 — 서버에게 물어본 쪽이 알며,
    /// 없다고 해서 저장된 선택이 지워지지 않는다.
    #[serde(default)]
    pub ai_model: Option<String>,
    /// Notion 페이지를 **어느 페이지 아래에** 만드는가 (ADR-0009 §5.1 · §8.4).
    /// 고르지 않은 상태(`null`)도 정상이다.
    ///
    /// **secret이 아니다** — 어디에 쓰는지일 뿐이며, `ai_base_url`이 secret이 아닌 것과 같은
    /// 이유로 INV-7과 충돌하지 않는다. **integration 자격증명은 이 경계를 지나지 않는다** —
    /// 받는 자리도 돌려주는 자리도 여기가 아니다 (ADR-0009 §10.4).
    ///
    /// 이 페이지가 지금도 있는지, integration에 공유돼 있는지는 여기서 말하지 않는다.
    #[serde(default)]
    pub notion_parent_page_id: Option<String>,
}

impl From<Settings> for SettingsPayload {
    fn from(settings: Settings) -> Self {
        Self {
            recordings_directory: settings.recordings_directory,
            automatic_processing: settings.automatic_processing,
            automatic_transcription: settings.automatic_transcription,
            transcription_model: settings.transcription_model,
            default_microphone: settings.default_microphone,
            ai_provider: settings.ai_provider,
            ai_base_url: settings.ai_base_url,
            ai_model: settings.ai_model,
            notion_parent_page_id: settings.notion_parent_page_id,
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
            // AI 설정 세 값도 같은 규칙이다 — 공백뿐인 입력은 '고르지 않음'이며, 그것뿐이다.
            // provider가 실재하는지, 주소가 응답하는지, 모델이 설치돼 있는지를 이 자리에서
            // 확인하지 않고, 아니라고 짐작해서 다른 값으로 바꾸지도 않는다.
            ai_provider: payload
                .ai_provider
                .map(|provider| provider.trim().to_string())
                .filter(|provider| !provider.is_empty()),
            ai_base_url: payload
                .ai_base_url
                .map(|url| url.trim().to_string())
                .filter(|url| !url.is_empty()),
            ai_model: payload
                .ai_model
                .map(|model| model.trim().to_string())
                .filter(|model| !model.is_empty()),
            // Notion destination도 같은 규칙이다 — 공백뿐인 입력은 '고르지 않음'이며
            // **그것뿐이다.** 이 자리에서 페이지 식별자의 모양을 검사하지도, 그 페이지가
            // 실재하는지 물어보지도 않는다 (INV-10 · ADR-0009 §8.4).
            notion_parent_page_id: payload
                .notion_parent_page_id
                .map(|page| page.trim().to_string())
                .filter(|page| !page.is_empty()),
        }
    }
}

/// 고른 AI provider가 지금 어떤 상태인지 (§9.2의 `isAvailable` 자리 · INV-5 · INV-8).
///
/// **이 값에는 실패 채널이 없다** — 만드는 함수가 [`Result`]를 돌려주지 않는다
/// ([`crate::commands::NoteGenerator::provider_status`]). provider를 고르지 않은 것도, 서버가
/// 응답하지 않는 것도 오류가 아니라 **여기 있는 네 상태 중 하나**다 (INV-8 · §13). 그래서
/// 화면은 이 값을 경고가 아니라 담담한 상태로 그릴 수 있다.
///
/// ```text
/// state           providerId/Name/locality   models   failure
/// notConfigured   null                       []       null      고르지 않았다 (정상 상태)
/// ready           있음                       있음     null      지금 노트를 만들 수 있다
/// noModels        있음                       []       null      응답했지만 쓸 모델이 없다
/// unavailable     있음                       []       있음      지금 닿지 못한다
/// ```
///
/// 네 상태를 하나의 boolean으로 뭉개지 않는 이유는 §13이다 — provider를 고르는 것 · 서버를
/// 켜는 것 · 모델을 받는 것은 사용자가 할 일이 서로 다르다 (ADR-0008 §4.2 표 2행).
///
/// **벤더 이름이 타입에도 상태 문자열에도 없다** (INV-9). `provider_id`에 담기는 것은 provider가
/// 스스로 말한 자유 식별자이며, 이 경계는 그 값을 알려진 목록과 대조하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatusPayload {
    /// `notConfigured · ready · noModels · unavailable` 중 하나.
    pub state: String,
    /// provenance에 남는 식별자. 고른 provider가 없으면 없다.
    pub provider_id: Option<String>,
    /// 사람이 읽는 이름. 고른 provider가 없으면 없다.
    pub provider_name: Option<String>,
    /// **전사가 이 기기를 떠나는가** — `local` 또는 `external` (§12 · INV-5).
    ///
    /// 고른 provider가 없으면 없다. 나가는 곳이 없으므로 말할 것도 없다.
    pub locality: Option<String>,
    /// 지금 고를 수 있는 모델들. 쓸 수 없는 상태에서는 빈 목록이다.
    pub models: Vec<String>,
    /// 지금 닿지 못하는 이유. **`unavailable`에서만 있다** — 그 밖의 상태는 실패가 아니다.
    pub failure: Option<Failure>,
}

impl AiProviderStatusPayload {
    /// 쓸 provider를 고르지 않았다. **오류가 아니라 정상 상태다** (INV-8).
    pub(super) fn not_configured() -> Self {
        Self {
            state: "notConfigured".to_string(),
            provider_id: None,
            provider_name: None,
            locality: None,
            models: Vec::new(),
            failure: None,
        }
    }

    /// 고른 provider가 자기 자신에 대해 말한 것과, 지금 쓸 수 있는지에 대한 답을 합친다.
    pub(super) fn describing(descriptor: ProviderDescriptor, availability: Availability) -> Self {
        let (state, models, failure) = match availability {
            Availability::Ready { models } => ("ready", models, None),
            Availability::NoModels => ("noModels", Vec::new(), None),
            Availability::Unavailable(failure) => ("unavailable", Vec::new(), Some(failure)),
        };

        Self {
            state: state.to_string(),
            provider_id: Some(descriptor.id),
            provider_name: Some(descriptor.name),
            locality: Some(descriptor.locality.as_str().to_string()),
            models,
            failure,
        }
    }
}

/// 지금 AI 노트 생성이 어떤 상태인지.
///
/// [`TranscriptionStatusPayload`]와 같은 자리에 있는 값이다 — **진행 중인 생성을 들고 있는 것은
/// backend이고 화면은 그것을 물어본다** ([`crate::commands::NoteGenerator`]). 그래서 화면이 다시
/// 그려지거나 사용자가 다른 화면에 다녀와도 여기서 같은 답이 나온다.
///
/// 다섯 상태는 서로 배타적이며, 상태마다 값이 있는 필드가 다르다.
///
/// ```text
/// state          recordingId   mode    aiNoteId   failure
/// idle           null          null    null       null
/// running        있음          있음    null       null
/// done           있음          있음    있음       null
/// noTranscript   있음          있음    null       null
/// failed         있음          있음    null       있음
/// ```
///
/// `noTranscript`가 `failed`와 따로 있는 이유는 §7.2다 — **재료가 아직 없는 것은 실패가
/// 아니다.** 그 경로는 아무것도 저장하지 않았고 Recording의 AI 상태도 옮기지 않았으며, 사용자가
/// 할 일은 다시 시도하는 것이 아니라 전사를 먼저 돌리는 것이다 ([`crate::ai::run::Outcome`]).
///
/// **실패는 [`Failure`] 그대로 실려 온다.** §13의 여섯 AI 실패는 사용자가 할 일이 전부 다르므로
/// 여기서 문장을 새로 만들거나 종류를 뭉개지 않는다 — provider를 고르지 않은 것도 이 자리로
/// 온다. 사용자가 그 상태에서 굳이 생성을 요청했을 때의 답이며, **command 실패가 아니라
/// 상태값이다** (INV-8 · ADR-0008 §13.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiNoteStatusPayload {
    /// `idle · running · done · noTranscript · failed` 중 하나.
    ///
    /// **[`RecordingPayload::ai_status`]와 다른 값이다.** 저쪽은 녹음 하나에 저장된 후처리
    /// 상태(§7)이고, 이쪽은 지금 이 앱이 실제로 돌리고 있는 생성 한 건이다.
    pub state: String,
    /// 지금 노트를 만들고 있거나 마지막으로 만든 녹음. 아무것도 하지 않았으면 없다.
    pub recording_id: Option<String>,
    /// 어떤 종류의 노트인가 — `meeting · study · summary` (§9.5).
    pub mode: Option<String>,
    /// 성공했을 때 **새로 추가된** 노트 (ADR-0008 §9.2). 이전 노트는 그대로 남는다.
    pub ai_note_id: Option<String>,
    /// 실패했을 때 그 실패 그대로.
    pub failure: Option<Failure>,
}

impl AiNoteStatusPayload {
    /// 진행 중인 생성도, 방금 끝난 생성도 없다. **오류가 아니라 정상 상태다.**
    pub(super) fn idle() -> Self {
        Self {
            state: "idle".to_string(),
            recording_id: None,
            mode: None,
            ai_note_id: None,
            failure: None,
        }
    }

    /// 지금 이 녹음의 이 mode 노트를 만들고 있다.
    pub(super) fn running(recording_id: &str, mode: NoteType) -> Self {
        Self {
            state: "running".to_string(),
            recording_id: Some(recording_id.to_string()),
            mode: Some(mode.as_str().to_string()),
            ..Self::idle()
        }
    }

    /// 노트 하나가 새로 저장됐다.
    pub(super) fn done(recording_id: &str, mode: NoteType, ai_note_id: &str) -> Self {
        Self {
            state: "done".to_string(),
            ai_note_id: Some(ai_note_id.to_string()),
            ..Self::running(recording_id, mode)
        }
    }

    /// 입력이 될 Transcript가 아직 없다 (§7.2). **아무것도 저장하지 않았고 실패도 아니다.**
    pub(super) fn no_transcript(recording_id: &str, mode: NoteType) -> Self {
        Self {
            state: "noTranscript".to_string(),
            ..Self::running(recording_id, mode)
        }
    }

    /// 생성이 실패했다. 오디오 · Transcript · 이미 저장된 노트는 그대로다 (INV-2 · INV-3).
    pub(super) fn failed(recording_id: &str, mode: NoteType, failure: Failure) -> Self {
        Self {
            state: "failed".to_string(),
            failure: Some(failure),
            ..Self::running(recording_id, mode)
        }
    }
}

/// 노트 본문 — mode에 따라 다른 필드를 갖는 **한 값**이다 (§9.5 · ADR-0008 §7.1 · §7.3).
///
/// `{"mode": "meeting", "overview": ..., ...}` 모양으로 나간다. 화면은 `mode`로 갈라 읽는다.
///
/// **세 mode를 옵셔널 필드 하나로 합치지 않는다** — 합치면 "Meeting인데 `thingsToStudy`가 있는"
/// 값이 타입 수준에서 가능해진다 (ADR-0008 §7.3).
///
/// 본문 필드 이름을 여기서 다시 적지 않고 [`crate::ai::note`]의 타입을 그대로 싣는 이유는
/// 하나다 — 그 이름들은 **§9.5의 출력 섹션 이름 13개**이며 그것을 정한 자리는 거기 하나다
/// (ADR-0008 §7.1). 같은 13개를 두 곳에 적으면 한쪽만 고쳐졌을 때 저장된 노트와 화면이
/// 조용히 어긋난다. 그 타입들에는 벤더가 없다 (INV-9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum StructuredNotePayload {
    Meeting(MeetingNote),
    Study(StudyNote),
    Summary(SummaryNote),
}

impl From<StructuredNote> for StructuredNotePayload {
    fn from(note: StructuredNote) -> Self {
        match note {
            StructuredNote::Meeting(note) => Self::Meeting(note),
            StructuredNote::Study(note) => Self::Study(note),
            StructuredNote::Summary(note) => Self::Summary(note),
        }
    }
}

/// 저장된 AI 노트 하나 (§7 · §7.3). Recording Detail의 AI Note 탭이 그대로 쓴다.
///
/// **읽기 전용 값이다.** 이 payload를 되돌려받는 command는 없다 — 저장소가 내놓는 `ai_notes`
/// 쓰기 경로는 추가([`crate::db::store::insert_ai_note`]) 하나뿐이고 그것을 부르는 자리는
/// [`crate::ai::run`]뿐이므로, 화면이 이미 만들어진 노트를 고치거나 지우는 경로는 만들어질 수
/// 없다 (ADR-0008 §9.2).
///
/// **저장된 문자열을 그대로 내보내지 않는다.** `ai_notes.content`의 봉투를 푸는 자리는
/// [`decode_content`] 하나이며 (ADR-0008 §7.5), 화면은 봉투도 schema 버전도 알지 않는다.
///
/// provenance 네 값은 §7.3이 요구하는 것 전부다 — 어느 Transcript version에서 · 어떤
/// provider가 · 어떤 모델로 · 어떤 프롬프트 버전으로 만들었는가. `provider`는 **벤더 중립
/// 자유 식별자**이며 (INV-9), 이 경계는 그 값을 알려진 목록과 대조하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiNotePayload {
    pub id: String,
    pub recording_id: String,
    /// **어떤 Transcript version을 입력으로 썼는가** (§7.3). provenance의 일부다.
    pub transcript_id: String,
    /// `meeting · study · summary` 중 하나. 저장된 행의 종류 그대로다.
    pub mode: String,
    /// 노트 본문. 봉투 안의 `mode`와 [`Self::mode`]가 어긋난 행은 여기까지 오지 못한다 —
    /// [`decode_content`]가 그것을 먼저 거절한다.
    pub note: StructuredNotePayload,
    pub provider: String,
    pub model: String,
    pub prompt_version: String,
    pub generated_at: String,
}

impl AiNotePayload {
    /// 저장된 노트 하나를 화면이 읽을 수 있는 모양으로 옮긴다.
    ///
    /// 봉투를 읽지 못하면 실패다 — **그 실패는 AI 실패가 아니라 저장소 실패이며**, 행은
    /// 건드리지 않는다 (`decode_content`). 읽을 수 없는 노트를 빈 값으로 채워 정상인 척하지
    /// 않는다.
    pub(super) fn decoded(note: AiNote) -> Result<Self, Failure> {
        let content = decode_content(&note.content, note.note_type)?;

        Ok(Self {
            id: note.id.as_str().to_string(),
            recording_id: note.recording_id.as_str().to_string(),
            transcript_id: note.transcript_id.as_str().to_string(),
            mode: note.note_type.as_str().to_string(),
            note: content.into(),
            provider: note.provider,
            model: note.model,
            prompt_version: note.prompt_version,
            generated_at: note.generated_at,
        })
    }
}

/// 방금 만들어진 Markdown 파일 하나 (§11 · ADR-0009 §4).
///
/// **경로가 이 값의 요점이다.** export 위치는 설정으로 노출되지 않으므로 (ADR-0009 §4.1),
/// 화면이 사용자에게 "어디에 만들어졌는가"를 말해 주지 못하면 사용자는 파일을 찾을 수 없다.
///
/// `file_name`이 따로 있는 이유는 **요청한 이름과 다를 수 있기 때문이다** — 같은 이름이 이미
/// 있으면 덮어쓰지 않고 번호가 붙는다 (ADR-0009 §4.3). 화면이 경로 문자열을 잘라 이름을
/// 짐작하지 않도록 실제로 쓰인 이름을 함께 싣는다.
///
/// 내용은 싣지 않는다. 문서 본문은 파일에 있고, 그것을 IPC로 한 번 더 흘려보낼 이유가 없다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedFilePayload {
    /// 내보낸 녹음. 화면이 어느 녹음의 결과인지 확인할 수 있게 함께 싣는다.
    pub recording_id: String,
    /// 만들어진 파일의 전체 경로. 사용자에게 그대로 보여줄 수 있는 값이다.
    pub path: String,
    /// 그 파일의 이름. `2026-09-01-3dgs-study-04-2.md`처럼 번호가 붙어 있을 수 있다.
    pub file_name: String,
}

impl ExportedFilePayload {
    /// 실제로 쓰인 파일 하나를 화면이 읽을 수 있는 모양으로 옮긴다.
    ///
    /// 경로는 [`std::path::Path::display`]로 옮긴다 — UTF-8이 아닌 경로에서도 값을 잃지 않고
    /// 사람이 읽을 수 있는 문자열이 나가며, 여기서 실패를 만들지 않는다.
    pub(super) fn new(recording_id: &str, written: crate::export::WrittenFile) -> Self {
        Self {
            recording_id: recording_id.to_string(),
            path: written.path.display().to_string(),
            file_name: written.name,
        }
    }
}

/// 지금 Notion 전송이 어떤 상태인지 (§10 · ADR-0009 §8).
///
/// [`TranscriptionStatusPayload`] · [`AiNoteStatusPayload`]와 같은 자리에 있는 값이다 —
/// **진행 중인 전송을 들고 있는 것은 backend이고 화면은 그것을 물어본다**
/// ([`crate::commands::NotionSender`]). 그래서 화면이 다시 그려지거나 사용자가 다른 화면에
/// 다녀와도 여기서 같은 답이 나온다.
///
/// ```text
/// state      recordingId   pageId   createdPage   failure
/// idle       null          null     false         null
/// running    있음          null     false         null
/// done       있음          있음     true/false    null
/// failed     있음          null     false         있음
/// ```
///
/// `created_page`가 따로 있는 이유는 **이어 보낸 것과 새로 만든 것이 다른 결과이기 때문이다**
/// (ADR-0009 §8.2 · §8.3). 끝나지 않은 전송을 이어 보냈다면 페이지는 그때 만들어진 그 페이지이며,
/// 화면은 "새 페이지를 만들었다"고 말하면 안 된다.
///
/// **실패는 [`Failure`] 그대로 실려 온다.** §13의 Notion 실패 다섯은 사용자가 할 일이 전부
/// 다르므로 여기서 문장을 새로 만들거나 종류를 뭉개지 않는다. **부분 전송 뒤의 실패도 이 자리로
/// 온다** — 어디까지 갔는지는 저장된 [`NotionSyncPayload`]가 말한다.
///
/// **token은 어느 상태에도 실리지 않는다** (INV-7). 담을 자리가 이 타입에 없다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionSendStatusPayload {
    /// `idle · running · done · failed` 중 하나.
    ///
    /// **[`RecordingPayload::notion_status`]와 다른 값이다.** 저쪽은 녹음 하나에 저장된 후처리
    /// 상태(§7)이고, 이쪽은 지금 이 앱이 실제로 돌리고 있는 전송 한 건이다.
    pub state: String,
    /// 지금 보내고 있거나 마지막으로 보낸 녹음. 아무것도 하지 않았으면 없다.
    pub recording_id: Option<String>,
    /// 성공했을 때 그 녹음이 된 Notion 페이지의 식별자.
    pub page_id: Option<String>,
    /// 이번 실행에서 **새로 만든** 페이지인가. 이어 보낸 것이면 `false`다.
    pub created_page: bool,
    /// 실패했을 때 그 실패 그대로.
    pub failure: Option<Failure>,
}

impl From<NotionSendStatus> for NotionSendStatusPayload {
    fn from(status: NotionSendStatus) -> Self {
        let idle = Self {
            state: "idle".to_string(),
            recording_id: None,
            page_id: None,
            created_page: false,
            failure: None,
        };

        match status {
            NotionSendStatus::Idle => idle,
            NotionSendStatus::Running { recording_id } => Self {
                state: "running".to_string(),
                recording_id: Some(recording_id),
                ..idle
            },
            NotionSendStatus::Done {
                recording_id,
                page_id,
                created_page,
            } => Self {
                state: "done".to_string(),
                recording_id: Some(recording_id),
                page_id: Some(page_id),
                created_page,
                ..idle
            },
            NotionSendStatus::Failed {
                recording_id,
                failure,
            } => Self {
                state: "failed".to_string(),
                recording_id: Some(recording_id),
                failure: Some(failure),
                ..idle
            },
        }
    }
}

/// 저장된 Notion 전송 상태 하나 (§7의 `notion_syncs` · ADR-0009 §8.4).
///
/// [`NotionSendStatusPayload`]와 다른 값이다 — 저쪽은 **앱이 켜져 있는 동안의 진행 상황**이고,
/// 이쪽은 **디스크에 남아 있는 사실**이다. 앱을 다시 켜도 이 값은 그대로 있다.
///
/// `sent_chunks`와 `total_chunks`가 함께 오는 이유는 하나다 — **부분 전송이 상태에서 드러나야
/// 하기 때문이다** (ADR-0009 §8.4-4). 실패한 요청은 세지 않으므로, 둘이 다르면 그 페이지에는
/// 문서의 일부만 들어가 있다.
///
/// **읽기 전용 값이다.** 이 payload를 되돌려받는 command는 없다 — `notion_syncs`에 쓰는 자리는
/// 전송 순서 하나뿐이며 ([`crate::sync::run`]), 화면이 전송 기록을 고치거나 지우는 경로는
/// 만들어질 수 없다.
///
/// `content_fingerprint`는 싣지 않는다. 그것은 **이어 붙여도 되는가**를 판정하기 위한 내부
/// 근거이며 (ADR-0009 §8.2), 그 판정을 하는 자리는 backend 하나다 — 화면에 보낼 이유가 없다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionSyncPayload {
    pub recording_id: String,
    /// 만들어진 페이지의 식별자. 성공한 적이 없으면 없다.
    pub page_id: Option<String>,
    /// 마지막으로 성공한 시각(ISO-8601 UTC 텍스트). 성공한 적이 없으면 없다.
    pub synced_at: Option<String>,
    /// `none · pending · running · done · failed` 중 하나 (§7).
    pub status: String,
    /// 마지막 실패 사유. 실패한 적이 없으면 없다.
    pub error: Option<String>,
    /// 이 페이지에 **성공적으로** 반영된 조각 수. 기록하기 전의 행이면 없다.
    pub sent_chunks: Option<i64>,
    /// 그 문서를 나눈 조각 수. 기록하기 전의 행이면 없다.
    pub total_chunks: Option<i64>,
}

impl From<NotionSync> for NotionSyncPayload {
    fn from(sync: NotionSync) -> Self {
        Self {
            recording_id: sync.recording_id.as_str().to_string(),
            page_id: sync.page_id,
            synced_at: sync.synced_at,
            status: sync.status.as_str().to_string(),
            error: sync.error,
            sent_chunks: sync.sent_chunks,
            total_chunks: sync.total_chunks,
        }
    }
}

/// Notion 연결이 지금 어떤 상태인지 (§5-D의 connection test · ADR-0009 §5.1).
///
/// [`AiProviderStatusPayload`]와 같은 성질의 값이다 — **아직 설정하지 않은 것은 실패가 아니라
/// 상태다** (INV-8). 그래서 화면은 이 값을 경고가 아니라 담담한 상태로 그릴 수 있다.
///
/// ```text
/// state           tokenStored   workspaceName   failure
/// notConfigured   false         null            아직 token을 저장하지 않았다 (요청도 보내지 않았다)
/// connected       true          Notion이 말한 것 이 token으로 지금 말할 수 있다
/// failed          true          null            말하지 못했다 — 무엇이 다른지는 failure가 말한다
/// ```
///
/// **세 상태를 boolean 하나로 뭉개지 않는 이유는 §13이다** — token을 넣는 것 · token을 고치는
/// 것 · 부모 페이지를 integration에 공유하는 것 · 네트워크를 확인하는 것은 사용자가 할 일이 전부
/// 다르다. 그 구분은 `failure.kind`가 그대로 들고 온다
/// ([`crate::notion::NOTION_FAILURE_KINDS`]).
///
/// **token은 여기에 오지 않는다** (INV-7 · ADR-0009 §10.4). 오는 것은 저장돼 있다는 사실
/// 하나뿐이며, 값을 담을 자리가 이 타입에 없다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionConnectionPayload {
    /// `notConfigured · connected · failed` 중 하나.
    pub state: String,
    /// integration token이 저장돼 있는가. **값이 아니라 사실이다.**
    pub token_stored: bool,
    /// 보낼 부모 페이지를 골랐는가 (설정 값 · ADR-0009 §8.4).
    ///
    /// **고르지 않은 것도 정상 상태다** (INV-8). 그 상태에서도 token 확인은 그대로 하며,
    /// 여기서 페이지 식별자를 돌려주지 않는다 — 그 값을 아는 자리는 설정 조회다.
    pub destination_configured: bool,
    /// **어느 워크스페이스에 연결됐는가** (§5-D). `connected`에서만, 그리고 Notion이 말해
    /// 줬을 때만 있다 — 말해 주지 않은 이름을 지어내지 않는다
    /// ([`crate::notion::ConnectedIdentity`]).
    ///
    /// secret이 아니다. token은 여전히 이 타입에 담길 자리가 없다 (INV-7).
    pub workspace_name: Option<String>,
    /// 연결하지 못한 이유. **`failed`에서만 있다** — 그 밖의 상태는 실패가 아니다.
    pub failure: Option<Failure>,
}

impl NotionConnectionPayload {
    /// 아직 token을 저장하지 않았다. **오류가 아니라 정상 상태다** (INV-8).
    pub(super) fn not_configured(destination_configured: bool) -> Self {
        Self {
            state: "notConfigured".to_string(),
            token_stored: false,
            destination_configured,
            workspace_name: None,
            failure: None,
        }
    }

    /// 저장된 token으로 지금 말할 수 있다. **누구에게인지는 Notion이 말한 그대로다.**
    pub(super) fn connected(destination_configured: bool, workspace_name: Option<String>) -> Self {
        Self {
            state: "connected".to_string(),
            token_stored: true,
            workspace_name,
            ..Self::not_configured(destination_configured)
        }
    }

    /// 말하지 못했다. **무엇이 달랐는지는 실려 온 실패가 말한다** (§13).
    ///
    /// 워크스페이스 이름은 없다 — 확인하지 못했으므로 어느 워크스페이스인지도 알지 못한다.
    pub(super) fn failed(destination_configured: bool, failure: Failure) -> Self {
        Self {
            state: "failed".to_string(),
            failure: Some(failure),
            ..Self::connected(destination_configured, None)
        }
    }
}

/// integration token이 저장돼 있는가 (INV-7 · ADR-0009 §10.4).
///
/// **이 앱에서 token에 대해 화면이 알 수 있는 전부다.** 값을 돌려주는 command는 없고, 이
/// 타입에도 값을 담을 자리가 없다 — 저장하는 command의 입력으로 한 번 지나갈 뿐이다.
///
/// 저장 뒤에도 삭제 뒤에도 같은 값이 돌아오므로, 화면은 자기가 방금 무엇을 했는지 추측하지
/// 않고 **자격증명 저장소가 실제로 어떤 상태인지** 그대로 받는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotionTokenStatusPayload {
    /// 저장돼 있으면 `true`. **저장한 적이 없는 것은 실패가 아니다** (INV-8).
    pub stored: bool,
}
