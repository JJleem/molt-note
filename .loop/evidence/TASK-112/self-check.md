# TASK-112 self-check (advisory)

Command: `node tools/loop-runtime/loopctl.mjs self-check build lint test`

- build: PASS exit=0
- lint: PASS exit=0
- test: PASS exit=0
- Self-check: all gates passed (raw artifacts under `.loop-local/self-check/`)

First run before fixes: `prompt_version_is_bound_to_the_prompt_text` reported
`meeting: 선언 v1.meeting.1fc7ae99 ≠ 계산 v1.meeting.b2f3ba75` → declared value updated to
`v1.meeting.b2f3ba75`. Study `v1.study.2cfad6a0` and Summary `v1.summary.beca7e6c` unchanged.

## Changes
- `src-tauri/src/ai/prompt.rs`
  - MEETING_PROMPT: removed "timestamps before each segment / Use these timestamps",
    "reference the transcript timestamp in a > quote block", "(include transcript timestamp)",
    "(with timestamps)". Added: input is plain text with no times; "Do not write transcript times
    (e.g. [HH:MM:SS], 00:03:23) anywhere in the output"; refer to uncertain/broken parts by what was
    being discussed. Tables: only in the group's optional `table` field; never table syntax
    (|, |---|) in string items.
  - New `segments_to_plain_text` and `transcript_input(mode, segments, raw_text)`
    (Meeting → plain, Study/Summary → existing `[HH:MM:SS]` text).
  - PROMPT_VERSION_MEETING → v1.meeting.b2f3ba75.
  - Tests: `the_meeting_prompt_has_no_instruction_to_use_or_include_timestamps`,
    `the_meeting_prompt_tells_the_model_not_to_write_times`,
    `the_meeting_prompt_keeps_tables_in_the_table_field_only`,
    `the_meeting_input_has_no_times_and_keeps_the_text` (fake segments; prepared prompt has no
    `[dd:dd:dd]`), `the_meeting_input_falls_back_to_raw_text`,
    `the_study_and_summary_input_is_unchanged`. Existing timestamp test now prepares with Study.
- `src-tauri/src/ai/run.rs`: input assembly uses `prompt::transcript_input(mode, …)`.
- `src-tauri/src/export/ai_request.rs`: pinned Meeting version updated to the new value.
- `src-tauri/tests/ai_note_run.rs`: `a_meeting_is_handed_plain_text_without_times`
  (FakeNoteAiProvider via real `run::generate`; Meeting plain, Study unchanged).
- `src-tauri/tests/audio_never_reaches_ai.rs`: wire-body exact assertion now expects the
  per-mode input (Meeting plain text, Study/Summary timestamped).

Not changed (out of scope): the manual AI request path in `export/ai_request.rs` still builds
its own input; only its version pin was updated.
