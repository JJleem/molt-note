# TASK-038 — 열린 command 표면

`src-tauri/src/lib.rs`의 `invoke_handler`에 등록된 다섯 이름. 벤더 이름은 하나도 없다.

| command | 반환 | provider 미설정일 때 |
|---------|------|----------------------|
| `ai_provider_status` | `Result<AiProviderStatusPayload, Failure>` | `state: "notConfigured"` — **정상 응답** (INV-8) |
| `start_ai_note` | `Result<AiNoteStatusPayload, Failure>` | 접수하고 `failed` 상태에 §13 실패를 실어 보낸다 — 거절하지 않는다 |
| `ai_note_status` | `Result<AiNoteStatusPayload, Failure>` | 생성이 도는 중에도 즉시 답한다 |
| `list_ai_notes` | `Result<Vec<AiNotePayload>, Failure>` | 없으면 빈 배열 |
| `get_ai_note` | `Result<Option<AiNotePayload>, Failure>` | 없으면 `null` |

`ai_provider_status`가 `Err`를 내는 유일한 경로는 **저장소에서 설정을 읽지 못했을 때**다.
provider 자체가 없다·닿지 않는다는 이유로 `Err`가 되는 경로는 없다.

## 실행 방식 (AC5)

`start_ai_note` → `NoteGenerator::start` → `thread::spawn` → `ai::run::generate`.
`Transcriber::start`와 같은 구조이며 새 방식을 만들지 않았다:

- 배경 스레드가 상태 자물쇠를 **시작할 때와 끝날 때만** 잡는다 → `status`가 즉시 답한다
- 배경 스레드가 DB 연결을 **새로 연다** → 앱의 연결이 생성 내내 잡히지 않는다
- `ai_provider_status`만 `#[tauri::command(async)]`다. 이것은 오래 도는 시작이 아니라 한 번의
  질의이므로 스레드를 만들지 않고, 대신 응답하지 않는 서버를 main thread에서 기다리지 않게 한다.

## frontend (AC6)

`src/ipc/types.ts` — `AiProviderState` · `AiProviderLocality`(`'local' | 'external'`) ·
`AiProviderStatus` · `NoteMode` · `AiNoteState` · `AiNoteStatus` · `MeetingNote` · `StudyNote` ·
`SummaryNote` · `StructuredNote` · `AiNote`.

`src/ipc/commands.ts` — `aiProviderStatus` · `startAiNote` · `aiNoteStatus` · `listAiNotes` ·
`getAiNote`. frontend는 이 다섯 말고 다른 경로로 부르지 않는다.

`src/ipc/failure.ts` — §13의 AI 실패 여섯(`aiProviderNotConfigured` · `aiProviderUnreachable` ·
`aiModelUnavailable` · `aiRequestFailed` · `aiResponseUnusable` · `aiInputTooLarge`).

로컬/외부 구분은 payload의 `locality: Option<String>`에서 나와 `AiProviderLocality`로 도달한다.
`tests/ipc-boundary.test.ts`가 그 두 소스를 읽어 확인하고, 같은 파일이 벤더 이름·엔드포인트가
없다는 것도 확인한다.
