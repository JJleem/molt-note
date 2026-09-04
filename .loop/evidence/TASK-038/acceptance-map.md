# Acceptance Criteria — 어디를 보면 판정되는가

## AC1 · AC2 · AC3 (build · lint · test Gate)

`.loop/evidence/TASK-038/gate-results.md` 참조. 셋 다 exit 0.

## AC4 — provider 미설정 · 연결 불가가 command 실패가 아니라 정상 상태값이다 (INV-8 · §13)

**반환 타입에서 확인할 수 있는 것**

| 자리 | 반환 타입 | 뜻 |
| --- | --- | --- |
| `src-tauri/src/commands/notes.rs` — `NoteGenerator::provider_status` | `AiProviderStatusPayload` (**`Result` 아님**) | 이 함수에는 provider를 실패로 돌려줄 채널이 없다 |
| `src-tauri/src/commands/payload.rs` — `AiProviderStatusPayload::state` | `notConfigured` · `ready` · `noModels` · `unavailable` | 네 가지가 전부 정상 응답값이다 |
| `src-tauri/src/commands/mod.rs` — `ai_provider_status` | `Result<AiProviderStatusPayload, Failure>` | `Err`는 **저장소에서 설정을 읽지 못했을 때만** 나온다 (`storage.settings()?` 한 줄) |
| `src-tauri/src/commands/mod.rs` — `start_ai_note` | `Result<AiNoteStatusPayload, Failure>` | `Err`는 (a) 앱 데이터 디렉터리 실패 (b) 빈 recordingId (c) 이미 생성 중 (d) 모르는 mode — **provider 부재는 없다** |

**provider가 없다는 이유로 `Err`가 되는 경로가 없다는 근거**

`NoteGenerator::start`는 provider를 만들지 않는다. provider 선택은 배경 스레드의
`generate_one` → `Providers::to_generate_with`에서 일어나고, 거기서 나온 §13의
`AiProviderNotConfigured`는 `NoteGenerationState::Failed`에 담겨
`AiNoteStatusPayload::failed(...)`로 조회된다. 즉 **command의 오류 채널을 지나지 않는다.**

> `not_configured(...)`가 `Failure`인 것은 §13이 그 실패를 값으로 요구하기 때문이다
> (ADR-0008 §13.1 — "서비스 계층이 만든다"). 그 값이 도달하는 곳이 command의 `Err`가 아니라
> 상태 payload의 `failure` 필드라는 것이 이 AC의 요구다.

**테스트**

- `asking_for_a_note_without_a_provider_is_accepted_and_answered_as_a_state`
  — `start(...)`가 `Ok(running)`이고, 끝난 상태가 `failed` + `aiProviderNotConfigured`이며,
  Recording의 `ai_status`는 `none` 그대로다.
- `a_provider_that_was_never_chosen_is_a_state_not_a_failure` — `notConfigured`, `failure: None`.
- `a_server_that_does_not_answer_and_a_server_without_models_are_different_states`
  — `unavailable`(failure 있음, retryable) 과 `noModels`(failure 없음)가 서로 다른 상태다.
- `recording_transcript_and_reading_all_work_with_no_ai_provider_at_all`
  — provider가 하나도 없는 상태에서 목록 · 전사 열람 · 설정 · 노트 목록이 전부 정상 응답.

## AC5 — 긴 생성이 UI와 IPC를 막지 않고, 실행 방식이 전사 경로와 일관된다

`commands::Transcriber`와의 대조:

| | `Transcriber` (P3) | `NoteGenerator` (이 Task) |
| --- | --- | --- |
| 시작 | `start()`가 `thread::spawn` 후 즉시 반환 | 같다 (`NoteGenerator::start` → `spawn`) |
| 상태 | `Arc<Mutex<State>>` 한 값, 배경 스레드는 시작·종료에만 잠금 | 같다 |
| 조회 | `status()`가 그 값만 읽는다 | 같다 (`NoteGenerator::status`) |
| DB | 배경 스레드가 **연결을 새로 연다** (`transcribe_one`) | 같다 (`generate_one`의 `db::open_in`) |
| 설정 | 시작할 때 읽는다 (모델 선택) | 시작할 때 읽는다 (provider · 모델) |
| 중복 시작 | 줄 세우지 않고 거절 (`already_running`) | 같다 |
| 주입점 | `Transcriber::with_engine(dir, engine)` | `NoteGenerator::with_provider(dir, provider)` |

command가 스레드를 붙잡지 않는다는 것을 **관찰로** 확인한다 — `GatedProvider`는
`generate_note` 안에서 멈춰 서고, 테스트가 풀어 줄 때까지 나오지 않는다:

- `a_status_query_answers_while_the_provider_is_still_generating`
  — 생성이 provider 안에 있는 동안 `status()` 5회가 1초 안에 답한다.
- `other_commands_answer_while_a_note_is_being_generated`
  — 그 구간에서 `list_recordings` · `get_settings` · `get_transcript`가 2초 안에 답하고,
    목록의 `aiStatus`는 이미 `running`이다.
- `starting_a_second_generation_is_refused_instead_of_being_ignored`
  — 거절이 진행 중인 생성을 흔들지 않고, 노트도 한 건만 남는다.

`ai_provider_status`만 `#[tauri::command(async)]`다 — 오래 도는 생성이 아니라 한 번의 질의이며,
응답하지 않는 서버를 main thread에서 기다리지 않게 하기 위해서다. 이유는 그 command의 doc
주석에 적혀 있다.

## AC6 — payload · frontend 타입에 벤더가 없고, 로컬/외부 구분이 frontend까지 도달한다

**벤더 없음**

- 타입 이름: `AiProviderStatusPayload` · `AiNoteStatusPayload` · `AiNotePayload` ·
  `StructuredNotePayload` / TS: `AiProviderStatus` · `AiNoteStatus` · `AiNote` ·
  `StructuredNote`. 엔드포인트도 에러 코드도 없다.
- `AiNotePayload::provider`는 §7.3 provenance의 **자유 식별자 값**이다(타입 이름이 아니다).
  domain도 이 경계도 그 값을 알려진 목록과 대조하지 않는다 (INV-9).
- 소스로 강제한다:
  - `tests/ipc-boundary.test.ts` → `wire 계약에 벤더가 없다 (INV-9)` 3개
    (payload.rs · types.ts에 벤더 이름 없음, types/commands에 주소·`/api/*` 없음)
  - `src-tauri/tests/ollama_adapter.rs::no_source_outside_the_adapter_knows_this_vendors_endpoints_or_parameters`
    (기존 검사 — 그대로 통과한다)

**로컬/외부 구분이 도달한다 (§12 · INV-5)**

```text
ProviderDescriptor.locality (Local|External)
  → AiProviderStatusPayload.locality : Option<String>  ("local" | "external")
  → AiProviderStatus.locality : AiProviderLocality | null   (src/ipc/types.ts)
```

- `tests/ipc-boundary.test.ts` → `provider가 로컬인지 외부인지가 frontend까지 도달한다 (INV-5)`
- `src-tauri/tests/ai_note_commands.rs::whether_the_transcript_leaves_the_device_reaches_the_screen`
  — 같은 경계에서 `"local"`과 `"external"`이 실제로 갈린다.
