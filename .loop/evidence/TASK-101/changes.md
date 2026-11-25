# TASK-101 Changes Summary

## Core Changes

### `src-tauri/src/audio/capture.rs`
- Added `warning: Option<String>` field to `CaptureReport`
- Changed `ActiveCapture::stop`: when `stop_device()` returns Err but `writer.join()` succeeds, the error message becomes a warning in `CaptureReport` instead of returning `Err`
- When `writer.join()` returns Err, still returns `Err(Failure)` as before
- Added 3 unit tests: `stop_device_err_with_writer_ok_produces_warning_instead_of_err`, `writer_err_still_returns_err`, `a_clean_stop_has_no_warning`

### `src-tauri/src/audio/system_capture.rs`
- Changed `Interrupted` type from `Arc<Mutex<Option<Failure>>>` to `Arc<Mutex<InterruptSlot>>` where `InterruptSlot` holds `started_at: Instant` and `event: Option<(Failure, Duration)>`
- Updated `note()` to compute elapsed duration from recording start
- Updated stop closure to format elapsed time as "HH:MM 무렵 일부 구간이 유실됐을 수 있다"
- Added `format_elapsed_hhmm()` helper
- Added unit test: `elapsed_time_formats_as_zero_padded_hours_and_minutes`

### `src-tauri/src/commands/payload.rs`
- Added `warning: Option<String>` field to `CaptureReportPayload`
- Propagated from `CaptureReport` in `new()`

## Test Changes

### `src-tauri/tests/capture.rs`
- Updated `a_capture_that_was_cut_short_says_so_and_keeps_what_was_recorded` → `a_capture_that_was_cut_short_produces_a_warning_and_keeps_what_was_recorded`: now expects `Ok` with warning instead of `Err`

### `src/ipc/types.ts`
- Added `warning: string | null` to `CaptureReport` interface

### `src/screens/coreWithoutAi.test.ts`, `src/screens/recordingView.test.ts`
- Added `warning: null` to test fixtures

## meeting_capture (AC4)
No code change needed in `meeting_capture.rs`. The `Stopper::stop` still returns `Err` when `dropped_frames > 0`, but `ActiveCapture::stop` now converts that `Err` to a warning in `CaptureReport` when the writer succeeds. The same pattern applies to both `system_capture` and `meeting_capture` stop paths.
