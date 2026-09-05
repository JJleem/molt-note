# TASK-057 — 변경 범위 (2026-09-04 · base `358ab79`)

이 Task가 만들거나 고친 파일은 아래 열셋이다.

```text
?? src-tauri/src/export/handoff.rs      새 실행 순서 모듈 (저장소 ↔ 순수 모듈) + 단위 테스트 2개
?? src-tauri/tests/manual_ai_handoff.rs 새 통합 테스트 10개 (MH-1 · MH-2 · MH-3 · MH-5 · MH-7 · §5.6)
 M src-tauri/src/export/filename.rs     ai_request_file_name + AI_REQUEST_MARKER · 단위 테스트 3개
 M src-tauri/src/export/run.rs          current Transcript를 고르는 규칙을 current_input 한 자리로
 M src-tauri/src/export/mod.rs          handoff 모듈 등록 · 두 이름 재export · 모듈 doc
 M src-tauri/src/commands/export.rs     Exporter::export_ai_request + 두 export가 공유하는 write_file
 M src-tauri/src/commands/mod.rs        Storage::{ai_prompt,transcript_text} + command 셋 + 모듈 doc
 M src-tauri/src/commands/payload.rs    ExportedFilePayload doc (돌려주는 command가 둘이 됐다)
 M src-tauri/src/lib.rs                 command 셋 등록 + 왜 늘었는지 주석
 M src/ipc/commands.ts                  getAiPrompt · getTranscriptText · exportAiRequest
 M src/ipc/types.ts                     ExportedFile doc (두 command가 같은 값을 돌려준다)
 M tests/ipc-boundary.test.ts           tripwire 네 자리를 **의식적으로** 갱신 (아래 boundary.md)
```

```text
 src-tauri/src/commands/export.rs  |  94 ++++++++++++++++++++++++++++--
 src-tauri/src/commands/mod.rs     | 116 +++++++++++++++++++++++++++++++++++++-
 src-tauri/src/commands/payload.rs |   5 ++
 src-tauri/src/export/filename.rs  |  78 +++++++++++++++++++++++++
 src-tauri/src/export/mod.rs       |  10 +++-
 src-tauri/src/export/run.rs       |  52 +++++++++++++----
 src-tauri/src/lib.rs              |  17 ++++++
 src/ipc/commands.ts               |  60 +++++++++++++++++++-
 src/ipc/types.ts                  |  11 +++-
 tests/ipc-boundary.test.ts        |  48 ++++++++++++----
```

> `src-tauri/src/export/markdown.rs`와 `src-tauri/src/export/ai_request.rs`도 working tree에서
> dirty로 보이지만 **이 Task의 변경이 아니다** — TASK-056이 같은 작업 트리에 남긴 커밋되지 않은
> 산출물이며, 이 Task는 그 두 파일을 한 글자도 고치지 않았다 (위 목록에 없다).

## 열린 이름은 셋이다 (ADR-0010 §8.1 · 28 → 31)

`src-tauri/src/lib.rs`의 `generate_handler![…]`에 더해진 것:

```rust
commands::get_ai_prompt,
commands::get_transcript_text,
commands::export_ai_request,
```

늘지 않은 것 — 저장된 것을 고치거나 지우는 이름, `transcriptId`를 받는 이름, provider·벤더를
아는 이름. `tests/ipc-boundary.test.ts`의 기존 검사 넷이 그것을 그대로 판정한다
(`저장된 Transcript를 고치거나 지우는 command가 없다` · `저장된 AI 노트를 …` ·
`저장된 Notion 전송 기록을 …` · `wire 계약에 벤더가 없다`).

## 화면이 부르는 이름과 등록된 이름이 정확히 같다

```ts
export function getAiPrompt(recordingId: string, mode: NoteMode): Promise<string>
export function getTranscriptText(recordingId: string): Promise<string>
export function exportAiRequest(recordingId: string, mode: NoteMode): Promise<ExportedFile>
```

`frontend가 부르는 이름이 등록된 이름과 정확히 같다`가 이것을 판정하며, 고치지 않은 채로
통과했다. **새 `FailureKind`를 만들지 않았다** — 이 경계의 실패는 전부 기존
`invalidInput` · `storage`이므로 `src/ipc/failure.ts`도, `domain/failure.rs`도 diff가 없다.
그래서 실패 종류 양방향 검사(`declared.length === kinds.size + 1`)도 그대로 통과한다.

## 새 타입을 만들지 않았다

문자열 둘은 `string`이고, 파일 하나는 이미 있는 `ExportedFile`/`ExportedFilePayload`다.
두 번째 payload 타입을 만들지 않은 이유는 화면이 알아야 하는 것이 둘 다 같기 때문이다 —
**어디에 무엇이 만들어졌는가**. 두 문서는 이름으로 구분된다 (`…-ai-request.md`).
