# TASK-102 Changes

## src-tauri/src/commands/payload.rs
- Added `warning: Option<String>` field to `StoppedRecordingPayload`

## src-tauri/src/commands/mod.rs
- In `finish_recording`, clone `capture.warning` and pass it to `StoppedRecordingPayload`

## src/ipc/types.ts
- Added `readonly warning?: string | null` to `StoppedRecording` interface

## Already present (from TASK-101)
- `CaptureReportPayload.warning: Option<String>` — already exists
- `CaptureReport.warning: string | null` (frontend) — already exists
- `Recorder::stop` already proceeds with `finalized::verify` regardless of warning
