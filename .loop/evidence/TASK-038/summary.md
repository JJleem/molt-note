# TASK-038 — AI Note command 경계와 frontend IPC client

작성 시각: 2026-09-03 · Run `RUN-20260903T073115Z-TASK-038`

## 무엇을 열었는가

P6의 orchestration(`ai::run`)과 P5의 provider(`ai::provider` · adapter)를 frontend가 쓸 수
있도록 **Tauri command 다섯 개**와 그에 1:1인 **타입 있는 client**를 열었다. 화면 구현은
이 Task의 범위가 아니므로 `src/screens/**`는 건드리지 않았다.

| # | command | 반환 타입 | 무엇 |
| --- | --- | --- | --- |
| 1 | `ai_provider_status` | `Result<AiProviderStatusPayload, Failure>` | provider 상태 — 설정 여부 · 사용 가능 여부 · 모델 목록 · **로컬/외부** |
| 2 | `start_ai_note` | `Result<AiNoteStatusPayload, Failure>` | 지정한 Recording과 mode로 생성/재생성 시작 (접수 사실을 돌려준다) |
| 3 | `ai_note_status` | `Result<AiNoteStatusPayload, Failure>` | 진행 상태 조회 |
| 4 | `list_ai_notes` | `Result<Vec<AiNotePayload>, Failure>` | 그 Transcript의 저장된 노트 전부 (이력 그대로) |
| 5 | `get_ai_note` | `Result<Option<AiNotePayload>, Failure>` | 저장된 노트 하나 |

frontend 대응: `aiProviderStatus` · `startAiNote` · `aiNoteStatus` · `listAiNotes` ·
`getAiNote` (`src/ipc/commands.ts`). `src/ipc/types.ts`에 `AiProviderStatus` ·
`AiProviderState` · `AiProviderLocality` · `AiNoteStatus` · `AiNoteState` · `NoteMode` ·
`StructuredNote`(세 mode의 discriminated union) · `AiNote`를 더했다.

`src/ipc/failure.ts`는 **이미 Rust의 `FailureKind`와 1:1이었다** — AI 실패 여섯이 P4 앞선
Task에서 이미 추가돼 있었고, `tests/ipc-boundary.test.ts`의 "Rust의 실패 종류가 frontend
타입에 전부 있다"가 그것을 계속 강제한다. 그래서 이 파일은 이 Run에서 바뀌지 않았다.

## 실행 방식 — 전사 경로를 그대로 따랐다 (새로 발명하지 않았다)

`commands::NoteGenerator`는 `commands::Transcriber`와 같은 규약이다.

```text
start_ai_note ─→ NoteGenerator ─→ 배경 스레드 ──→ ai::run::generate
                       │             (자물쇠를 쥐지 않고 도는 구간)
ai_note_status ──→ 상태 한 값  ←────┘  끝날 때 결과를 여기에 남긴다
```

- 시작은 스레드를 만든 뒤 **바로** 끝난다 (`Transcriber::start`와 같다).
- 배경 스레드는 상태 Mutex를 **시작할 때와 끝날 때만** 잡는다 → 상태 조회가 즉시 답한다.
- 배경 스레드는 **저장소 연결을 새로 연다** (`transcribe_one`과 같은 이유) → 생성이 도는
  동안 `list_recordings` · `get_settings` · `get_transcript`가 함께 멈추지 않는다.
- 진행 중에 들어온 두 번째 시작은 줄을 세우지 않고 거절된다 (§16 큐는 DEFERRED).

`ai_provider_status`만 `#[tauri::command(async)]`다. 이것은 오래 도는 생성이 아니라 **한 번의
질의**지만, 응답하지 않는 서버를 기다릴 수 있다. Tauri는 `async`가 아닌 command를 main
thread에서 실행하므로 그대로 두면 창 전체가 그 시간만큼 멈춘다.

## provider 부재는 실패가 아니라 상태다 (INV-8)

- `NoteGenerator::provider_status(&Settings) -> AiProviderStatusPayload` — **`Result`가 아니다.**
  실패 채널 자체가 없다. 네 상태: `notConfigured` · `ready` · `noModels` · `unavailable`.
- `ai_provider_status` command가 `Err`를 내는 경우는 **저장소에서 설정을 읽지 못했을 때
  하나뿐**이다. provider와 무관한, 실제로 시도했다가 실패한 일이다.
- `NoteGenerator::start`는 provider를 이유로 거절하지 않는다. provider 선택이 **배경 스레드
  안에서** 일어나므로, §13의 `aiProviderNotConfigured`는 command의 `Err`가 아니라 `failed`
  상태에 실려 화면에 도달한다 (ADR-0008 §13.2 — "사용자가 그 상태에서 굳이 생성을 요청했을
  때의 답").
- 그 경로는 Recording의 `ai_status`도 옮기지 않는다 — 시도한 적 없는 실패를 레코드에 남기지
  않는다 (`ai::run`의 `NoTranscriptYet`과 같은 규칙).

## 벤더 중립 (INV-9 · INV-5)

- payload 타입 이름에 벤더가 없다: `AiProviderStatusPayload` · `AiNoteStatusPayload` ·
  `AiNotePayload` · `StructuredNotePayload`. command 이름에도 없다.
- `locality`가 `"local" | "external"`로 payload와 frontend 타입까지 도달한다 (§12 · INV-5) —
  P9가 그것을 표시할 수 있다.
- 설정 식별자를 실제 adapter로 옮기는 함수는 `ai::provider_for`이며 **`src/ai/mod.rs`에
  있다.** 그 파일은 adapter를 마운트하는 자리이고, 기존 INV-9 소스 검사
  (`tests/ollama_adapter.rs::no_source_outside_the_adapter_knows_this_vendors_endpoints_or_parameters`)가
  벤더 이름을 허용하는 유일한 파일이다. 처음에는 `ai/catalog.rs`를 따로 두었으나 그 검사가
  잡았고, **검사를 고치는 대신 코드를 옮겼다** (검사는 그대로다).

## 저장된 노트를 읽는 경로

`ai_notes.content`의 §7.5 봉투를 푸는 자리는 `ai::note::decode_content` 하나이며,
`AiNotePayload::decoded`가 그것을 부른다 — 화면은 봉투도 schema 버전도 알지 않는다.
읽기뿐이다: 고치거나 지우는 command가 없고, 저장소의 `ai_notes` 쓰기 경로도 추가 하나뿐이라
만들어질 수도 없다. 재생성은 대체가 아니라 추가이므로 목록에는 이력이 그대로 온다.

## 범위 밖으로 나가지 않은 것

- 화면(`src/screens/**`) 구현 없음.
- provider 설정 UI · 모델 선택 UI 없음.
- 일괄 처리 큐 · 취소 · 스트리밍 없음.
- 기존 테스트를 약화하지 않았다. `tests/ipc-boundary.test.ts`의 command 목록은 Phase 4의
  표면이 열렸으므로 **늘렸고**, 벤더 이름 금지는 유지하면서 `ollama|llama|openai|anthropic|
  claude|gemini`로 **넓혔다**. 새 검사도 더했다 (payload/types에 벤더 이름이 없을 것,
  frontend 타입에 주소·엔드포인트가 없을 것, locality가 frontend까지 도달할 것).
