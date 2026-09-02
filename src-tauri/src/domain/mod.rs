//! 제품 domain 타입 (PRODUCT-SPEC §7).
//!
//! 이 모듈은 **저장소도 AI 벤더도 알지 않는다.** SQL도, provider 구현도 여기에 없다.
//! - `AiNote::provider`는 벤더 enum이 아니라 자유 문자열 식별자다 (INV-9).
//!   특정 벤더를 아는 것은 이후 Phase의 adapter이며, domain은 그 목록을 갖지 않는다.
//! - `AiNote::content`는 §9.3의 provider 중립 structured note를 **불투명한 문자열**로 담는다.
//!   최종 schema는 Phase 4가 확정하므로 (§9.3) domain이 지금 그 구조를 정하지 않는다.
//!
//! [`Transcript`]는 immutable entity다 — 재전사는 기존 것을 고치지 않고 새 Transcript를
//! 추가한다 (§7.1 · INV-2). 그래서 이 모듈에도, 저장소에도 Transcript를 갱신하는 API가 없다.

pub mod duration;
pub mod failure;
pub mod settings;

use std::fmt;

pub use duration::format_duration_ms;
pub use failure::{Failure, FailureKind};
pub use settings::Settings;

/// 문자열 identity를 감싸는 newtype을 만든다.
///
/// `recordingId`와 `transcriptId`를 서로 바꿔 넣는 실수를 타입이 막는다 —
/// AI Note provenance(§7.3)에서 그 둘을 구분하는 것이 요구사항이기 때문이다.
macro_rules! identifier {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// 주어진 문자열을 그대로 identity로 삼는다. 형식을 강제하지 않는다.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

identifier! {
    /// Recording의 identity.
    RecordingId
}

identifier! {
    /// Transcript의 identity. Recording과 독립적이다 (§7.1).
    TranscriptId
}

identifier! {
    /// AI Note의 identity.
    AiNoteId
}

/// 후처리 상태 (§7). `none`은 "아직 시도한 적 없음"이고 정상 상태다.
///
/// 실패(`Failed`)는 사용자에게 보이고 재시도 가능해야 하므로, `Running`과 구분되어야 한다.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProcessingStatus {
    /// 아직 시도하지 않았다.
    #[default]
    None,
    /// 실행을 기다리는 중이다.
    Pending,
    /// 실행 중이다.
    Running,
    /// 성공했다.
    Done,
    /// 실패했다. 사용자에게 보이고 재시도할 수 있어야 한다.
    Failed,
}

impl ProcessingStatus {
    /// §7이 요구하는 다섯 상태 전부. 저장 형식이 이 다섯을 구분하는지 검사할 때 쓴다.
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Pending,
        Self::Running,
        Self::Done,
        Self::Failed,
    ];

    /// 저장·전송에 쓰는 안정적인 문자열 표현.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    /// [`Self::as_str`]의 역변환. 모르는 값은 추측하지 않고 `None`을 돌려준다.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|status| status.as_str() == value)
    }
}

impl fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// AI Note의 종류 (§7 · §9.5).
///
/// 이것은 **출력 형태**의 종류이지 벤더가 아니다 (INV-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoteType {
    Meeting,
    Study,
    Summary,
}

impl NoteType {
    pub const ALL: [Self; 3] = [Self::Meeting, Self::Study, Self::Summary];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Meeting => "meeting",
            Self::Study => "study",
            Self::Summary => "summary",
        }
    }

    /// 모르는 값은 추측하지 않고 `None`을 돌려준다.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|note_type| note_type.as_str() == value)
    }
}

impl fmt::Display for NoteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 녹음 하나 (§7). source data다 — 후처리 실패가 이 레코드를 훼손하지 않는다 (INV-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recording {
    pub id: RecordingId,
    pub title: String,
    /// ISO-8601 텍스트. 시각을 만드는 주체는 domain이 아니라 호출자다.
    pub created_at: String,
    pub updated_at: String,
    /// 녹음 길이(밀리초).
    pub duration_ms: i64,
    pub audio_path: String,
    /// 컨테이너/코덱 식별자(예: `wav`). domain은 목록을 강제하지 않는다.
    pub audio_format: String,
    /// 녹음에 쓴 입력 장치. 알 수 없으면 `None`이다.
    pub microphone: Option<String>,
    /// 현재 사용 중인 성공한 Transcript (§7.2). **값이 없는 상태도 정상 상태다.**
    ///
    /// 실패한 재전사는 이 값을 바꾸지 않는다 — 실패 때문에 이미 유효한 Transcript를 잃지 않는다.
    pub current_transcript_id: Option<TranscriptId>,
    pub transcription_status: ProcessingStatus,
    pub ai_status: ProcessingStatus,
    pub notion_status: ProcessingStatus,
}

/// 조회된 Recording 하나 (§5 A · C 화면이 그대로 그릴 수 있는 형태).
///
/// 저장된 [`Recording`]에 **사람이 읽는 길이 문자열**을 함께 담는다. 목록·상세 조회가
/// 이 타입을 돌려주므로 UI는 `duration_ms`를 다시 계산할 필요가 없다 —
/// 같은 규칙이 TypeScript에 중복 구현되는 것을 막는 것이 이 타입의 목적이다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingView {
    pub recording: Recording,
    /// [`format_duration_ms`]가 만든 표시용 길이(예: `"52:31"`).
    pub duration_label: String,
}

impl From<Recording> for RecordingView {
    fn from(recording: Recording) -> Self {
        let duration_label = format_duration_ms(recording.duration_ms);
        Self {
            recording,
            duration_label,
        }
    }
}

impl RecordingView {
    /// 조회 결과의 identity. `view.recording.id`의 짧은 표현이다.
    pub fn id(&self) -> &RecordingId {
        &self.recording.id
    }
}

/// Transcript 안의 구간 하나 (§7의 `segments[] { start · end · text }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptSegment {
    /// 녹음 시작 기준 오프셋(밀리초).
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

/// 전사 결과 하나 (§7). **immutable · versioned**다.
///
/// 재전사는 기존 Transcript를 고치지 않고 새 [`Transcript`]를 추가한다 (§7.1 · INV-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    /// Recording과 독립적인 identity (§7.1).
    pub id: TranscriptId,
    pub recording_id: RecordingId,
    /// 감지·지정된 언어. 모르면 `None`이다.
    pub language: Option<String>,
    pub segments: Vec<TranscriptSegment>,
    pub raw_text: String,
    pub created_at: String,
    /// 전사 엔진 식별자. domain은 허용 값 목록을 갖지 않는다.
    pub engine: String,
    /// 전사 모델 식별자.
    pub model: String,
}

/// AI가 만든 노트 하나 (§7 · §7.3). derived data이며 언제든 재생성할 수 있다.
///
/// 재생성은 audio도 Transcript도 덮어쓰지 않는다 (INV-1 · INV-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiNote {
    pub id: AiNoteId,
    pub recording_id: RecordingId,
    /// **어떤 Transcript version을 입력으로 썼는가** (§7.3). provenance의 일부다.
    pub transcript_id: TranscriptId,
    pub note_type: NoteType,
    /// §9.3의 provider 중립 structured note. domain은 내용을 해석하지 않는다 —
    /// 최종 schema는 Phase 4가 확정한다.
    pub content: String,
    /// provenance. **벤더 중립 자유 식별자다** (INV-9).
    ///
    /// domain은 어떤 값이 올 수 있는지 알지 않는다. 벤더 지식은 adapter 안에만 있다.
    pub provider: String,
    /// provenance. provider가 쓴 모델 식별자.
    pub model: String,
    /// provenance. 노트를 만든 프롬프트의 버전.
    pub prompt_version: String,
    /// provenance. 생성 시각(ISO-8601 텍스트).
    pub generated_at: String,
}

/// Recording 하나의 Notion 전송 상태 (§7).
///
/// 이 Task는 스키마와 저장/복원만 다룬다. 실제 전송은 이후 Phase가 구현한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotionSync {
    pub recording_id: RecordingId,
    /// 전송에 성공한 적이 없으면 `None`이다.
    pub page_id: Option<String>,
    pub synced_at: Option<String>,
    pub status: ProcessingStatus,
    /// 마지막 실패 사유. 실패한 적이 없으면 `None`이다.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_processing_status_has_a_distinct_stable_string() {
        let mut seen = Vec::new();
        for status in ProcessingStatus::ALL {
            let text = status.as_str();
            assert!(!seen.contains(&text), "상태 문자열이 겹친다: {text}");
            seen.push(text);
        }
        assert_eq!(
            seen,
            ["none", "pending", "running", "done", "failed"],
            "§7이 요구하는 다섯 상태를 그대로 구분해야 한다"
        );
    }

    #[test]
    fn processing_status_round_trips_through_its_string_form() {
        for status in ProcessingStatus::ALL {
            assert_eq!(ProcessingStatus::parse(status.as_str()), Some(status));
        }
    }

    #[test]
    fn unknown_status_text_is_not_guessed() {
        assert_eq!(ProcessingStatus::parse("cancelled"), None);
        assert_eq!(ProcessingStatus::parse(""), None);
        assert_eq!(ProcessingStatus::parse("NONE"), None);
    }

    #[test]
    fn note_type_round_trips_and_rejects_unknown_values() {
        for note_type in NoteType::ALL {
            assert_eq!(NoteType::parse(note_type.as_str()), Some(note_type));
        }
        assert_eq!(NoteType::parse("diary"), None);
    }

    #[test]
    fn the_default_status_is_none_meaning_never_attempted() {
        assert_eq!(ProcessingStatus::default(), ProcessingStatus::None);
    }

    #[test]
    fn a_recording_view_carries_the_human_readable_length_with_the_record() {
        // UI가 duration_ms를 다시 계산하지 않아도 되는 이유가 이것이다.
        let recording = Recording {
            id: RecordingId::new("rec-view"),
            title: "3DGS Study #04".to_string(),
            created_at: "2026-09-01T10:00:00Z".to_string(),
            updated_at: "2026-09-01T10:00:00Z".to_string(),
            duration_ms: 3_151_000,
            audio_path: "recordings/rec-view.wav".to_string(),
            audio_format: "wav".to_string(),
            microphone: None,
            current_transcript_id: None,
            transcription_status: ProcessingStatus::None,
            ai_status: ProcessingStatus::None,
            notion_status: ProcessingStatus::None,
        };

        let view = RecordingView::from(recording.clone());

        assert_eq!(view.duration_label, "52:31");
        assert_eq!(view.duration_label, format_duration_ms(3_151_000));
        assert_eq!(view.recording, recording, "원본 레코드는 그대로 담긴다");
        assert_eq!(view.id(), &RecordingId::new("rec-view"));
    }

    #[test]
    fn recording_and_transcript_identities_are_different_types() {
        // 같은 문자열이라도 서로 다른 타입이므로 바꿔 넣을 수 없다 (§7.3 provenance).
        let recording = RecordingId::new("shared-text");
        let transcript = TranscriptId::new("shared-text");
        assert_eq!(recording.as_str(), transcript.as_str());
    }
}
