# TASK-006 — Evidence notes

Attempt 2. Attempt 1 timed out **after** writing the implementation but before running the
gates or producing a Result, so most of the code below already existed in the working tree
when this attempt started. This attempt verified it against the gates, found one real
failure, fixed it, and re-ran all three gates to green.

## Gate results (self-check, advisory)

| Gate  | Command         | Result |
|-------|-----------------|--------|
| build | `npm run build` | PASS exit=0 |
| lint  | `npm run lint`  | PASS exit=0 |
| test  | `npm run test`  | PASS exit=0 |

Full output: `self-check.log` (both the failing run and the final passing run).

## AC → where it is checked

- **AC1 / AC2 / AC3** — gates above.
- **AC4** (init failure is a structured, user-displayable value, not a panic/console log):
  - `src-tauri/src/commands/mod.rs` — `Storage` is either `Ready(Mutex<Connection>)` or
    `Unavailable(Failure)`. `Storage::open` / `open_for` map every initialization error into
    a `Failure` value; there is no `unwrap`/`expect`/`panic!` on the init path.
    `lib.rs` `setup` calls `app.manage(Storage::open_for(app))` and returns `Ok(())` — a
    failed store still lets the window open.
  - `src-tauri/src/domain/failure.rs` — the §13 failure type (`kind`, `message`, `detail`,
    `sourceDataSafe`, `retryable`) plus serialization tests.
  - `src-tauri/src/db/mod.rs` — `From<DatabaseError> for Failure`, with per-variant
    retryable/permanent mapping and unit tests for each variant.
  - `src-tauri/tests/command_boundary.rs` — creates a **real** failure (a regular file where
    the app-data directory must be) and asserts (a) `Storage::open` does not panic,
    (b) the failure is retained in app state, (c) **all six** commands return that same
    failure instead of an empty list / default settings. A second test builds a DB with a
    schema newer than the app and asserts a non-retryable failure with the DB left intact.
- **AC5** (no SQL in the frontend; command surface stays within Phase 1):
  - `tests/ipc-boundary.test.ts` — scans every file under `src/` for SQL statement shapes,
    asserts `invoke` is called only from `src/ipc/commands.ts`, asserts the
    `generate_handler![...]` list equals exactly the six Phase 1 commands, asserts no
    recording/transcription/AI/Notion command names appear, asserts the frontend's called
    names match the registered names, and asserts no `greet` scaffold remnant remains in
    `lib.rs` or anywhere under `src/`.
  - `src/ipc/failure.ts` + `src/ipc/failure.test.ts` — the TypeScript counterpart of the
    Rust failure type, including the `unexpected` kind produced only at the IPC boundary.

## The one failure found and fixed

`src-tauri/tests/recording_repository.rs::nothing_deletes_a_recording_on_its_own` asserted
`!LIB_SOURCE.contains("delete_recording")` as a source-text proxy for INV-4 ("a recording is
deleted only when the user explicitly asks"). That assertion was written when `lib.rs` had no
command surface at all. This Task requires registering a user-invoked `delete_recording`
command, which necessarily puts that string into `lib.rs`, so the literal check and the Task
contract cannot both hold.

The test was **not** deleted, skipped, or weakened. It was made precise: `split_invoke_handler`
splits `lib.rs` into the `generate_handler![...]` registration list and everything else, then
the test asserts

- the name appears **exactly once** in the registration list (deletion is exposed as a single
  user-invoked command), and
- the name appears **nowhere** in the rest of `lib.rs` (the startup path never deletes).

The store-level half of the test (exactly one `DELETE FROM`, and it targets a single caller-
supplied id) is untouched. The invariant being checked is the same one; the check no longer
conflates "the user can ask for a deletion" with "the app deletes on its own".

## Not done (out of scope, per the Task)

`git commit` was not run, as the Task instructs.
