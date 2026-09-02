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

use crate::audio::{CaptureReport, InputDevice};
use crate::domain::{ProcessingStatus, Recording, RecordingId, RecordingView, Settings};

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

/// 정지한 캡처 하나의 보고 값 (Phase 2A spike).
///
/// **Phase 2A의 성공 기준이 그대로 필드다** —
/// 장치 이름 · 출력 경로 · 포맷 · 파일 크기(byte).
/// 사람이 이 값을 보고 잠정 결정을 지지할지 반박할지 판단한다 (ADR-0003 §12).
///
/// `format`은 사람이 읽는 한 문장이고, 그 문장을 이루는 값도 따로 보낸다 — 화면이 문자열을
/// 다시 뜯어보지 않아도 되게 하기 위해서다 ([`RecordingPayload::duration_label`]과 같은 이유).
///
/// **이것은 저장된 Recording이 아니다.** 캡처 결과를 DB에 남기는 것은 Phase 2B의 일이며,
/// 이 값은 어떤 레코드도 만들지 않는다.
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
}

impl From<CaptureReport> for CaptureReportPayload {
    fn from(report: CaptureReport) -> Self {
        Self {
            device_label: report.device_label,
            output_path: report.output_path.display().to_string(),
            format: report.format.describe(),
            sample_rate_hz: report.format.sample_rate_hz,
            channels: report.format.channels,
            bits_per_sample: report.format.bits_per_sample,
            container: report.format.container().to_string(),
            byte_size: report.byte_size,
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
}

impl From<Settings> for SettingsPayload {
    fn from(settings: Settings) -> Self {
        Self {
            recordings_directory: settings.recordings_directory,
            automatic_processing: settings.automatic_processing,
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
        }
    }
}
