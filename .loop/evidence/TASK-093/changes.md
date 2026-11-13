# TASK-093 Changes

## Files Changed

### src-tauri/src/ai/prompt.rs
- **MEETING_PROMPT**: Replaced with V2 format incorporating `docs/MEETING-NOTE-FORMAT.md` structure and rules. Includes:
  - Team weekly format structure (meetingTitle, meetingKind, attendees, reliabilityNotice, keySummary, previousActionItems, topics, actionItems, needsConfirmation)
  - Detailed guidance for each field matching the format doc
  - Timestamp awareness: "The transcript may include [HH:MM:SS] timestamps"
  - "하지 않는 것" rules: no full transcript, no inventing data, explicit uncertainty markers
- **PROMPT_VERSION_MEETING**: Updated from `v1.meeting.3f16c822` to `v1.meeting.54529c96`
- **OUTPUT_RESERVE_TOKENS**: Increased from 1,536 to 16,384 (covers ~25KB V2 fixture at 2 chars/token)
- **ContextBudget::DEFAULT**: Increased from 16,384 to 32,768 to accommodate the larger OUTPUT_RESERVE
- **segments_to_timestamped_text()**: New function — formats segments as `[HH:MM:SS] text` lines, falls back to raw_text when segments are empty. Reuses `export::markdown::format_timestamp_ms`.
- Tests added: AC-2-2 (segment timestamps in prompt), AC-2-3 (output reserve covers 25KB), AC-2-4 (Study/Summary unchanged)

### src-tauri/src/ai/run.rs
- `attempt()` now calls `segments_to_timestamped_text()` to build the input text from segments before passing it to `prompt::prepare` and `TranscriptText`

### src-tauri/src/export/ai_request.rs
- Updated hardcoded `PROMPT_VERSION_MEETING` assertion to new hash

### src-tauri/tests/ai_note_run.rs
- Updated assertions to expect `[HH:MM:SS]`-prefixed transcript text

### src-tauri/tests/audio_never_reaches_ai.rs
- Added `TIMESTAMPED_TEXT` constant
- Updated assertions to expect timestamped text in provider calls and wire format

## Unchanged
- STUDY_PROMPT, SUMMARY_PROMPT: untouched
- PROMPT_VERSION_STUDY (`v1.study.2cfad6a0`), PROMPT_VERSION_SUMMARY (`v1.summary.beca7e6c`): untouched
