# TASK-041 — Acceptance Criterion → 판정 수단 대응표

Run: RUN-20260904T013137Z-TASK-041 · 2026-09-04

이 Task는 **새 제품 기능을 하나도 추가하지 않았다.** 바뀐 파일은 테스트 4개뿐이며,
`src-tauri/src/**`와 `src/**`의 제품 코드는 한 줄도 건드리지 않았다 (`changed-files.txt`).

---

## AC1 · AC2 · AC3 — build · lint · test Gate

`gate-summary.txt` — 셋 다 exit 0. lint는 `eslint .`와
`cargo clippy --all-targets -- -D warnings` 둘을 포함한다.

---

## AC4 — provider 미설정 상태에서 녹음 · 전사 · 열람이 동작한다 (INV-8)

### Rust — `src-tauri/tests/core_pipeline_without_ai.rs` (5 tests)

`Settings::DEFAULT`의 `ai_provider` · `ai_model`이 `None`인 앱에서 시작한다. 대체 구현이
들어가는 자리는 넷뿐이다 — 마이크 · 시계 · 전사 엔진 · AI provider. 파일도 저장소도 정지
경로도 제품 코드 그대로 실행된다.

판정의 중심은 헬퍼 `assert_the_three_paths_work`이며 **세 경로를 한 번에** 확인한다.

| 경로 | 무엇을 실제로 지나는가 | 확인하는 것 |
| --- | --- | --- |
| 1. 녹음 | `Recorder::start` → `finish_recording` (제품 코드) | 파일이 실재하고 · 보고된 크기가 파일시스템에서 읽은 값과 같고 · duration이 맞고 · 제목이 있다 (R-002) |
| 2. 전사 | 정지가 건 자동 전사 → `Transcriber` 배경 스레드 → `transcription::run` | `state == "done"` · Transcript 1개 추가 · 센티초→밀리초 변환 유지 · current가 그것을 가리킨다 |
| 3. 열람 | `Storage::list_recordings` · `recording` · `transcript` · `ai_notes` | 목록에 있고 · 상세가 열리고 · segment를 읽을 수 있고 · 노트 목록이 **빈 목록이지 실패가 아니다** |

- `recording_transcription_and_reading_all_run_with_no_ai_provider_configured`
  — 사전 조건으로 `ai_provider == None` · `ai_model == None`을 단언하고 세 경로를 돌린다.
  경로를 지난 뒤에도 여전히 `ai_provider == None`이다(무엇도 대신 골라 주지 않는다).
- `having_no_provider_is_a_normal_state_rather_than_an_error`
  — `provider_status`는 `Result`가 아니다. `state == "notConfigured"` · `failure == None` ·
  `provider_id/name/locality == None` · `models`가 비어 있다. **오류가 아닌 정상 상태**다.
- `asking_for_a_note_without_a_provider_leaves_the_recording_and_its_transcript_untouched`
  — 그 상태에서 굳이 생성을 눌렀을 때: command의 `Err`가 아니라 `failed` 상태 +
  `AiProviderNotConfigured`. 그리고 `ai_status`는 `none` 그대로이고, Transcript와 오디오
  바이트가 그대로다.
- `a_recording_that_never_touched_ai_keeps_the_untried_ai_status`
  — `none`이 "아직 시도하지 않았다"는 정상 상태로 남는다.
- `an_ai_provider_that_does_not_work_blocks_none_of_the_three_paths`
  — **연결 불가 · 모델 없음 · 생성 실패** 셋 각각에 대해, 실패를 실제로 겪은 **뒤에**
  세 경로를 처음부터 다시 돌린다(새 녹음 · 새 전사 · 새 열람). 실패를 겪은 녹음도 목록에
  그대로 남는다.

### frontend — `src/screens/coreWithoutAi.test.ts`

판정 방식이 요지다: **AI 값을 바꿔도 나머지 화면 값이 글자 하나 달라지지 않는다**를
`toEqual` 깊은 비교로 확인한다.

| 경로 | 모듈 | 테스트 |
| --- | --- | --- |
| 1. 녹음 | `recordingView` | `녹음 화면의 상태에 AI가 들어올 자리가 없다` (상태 필드 6개를 통째로 고정) · `provider가 하나도 없어도 시작과 정지를 누를 수 있다` · `정지가 성공하면 저장된 녹음이 그대로 화면에 온다` |
| 2. 전사 | `transcriptView` | `저장된 AI 상태가 무엇이든 Transcript 탭의 값이 글자 하나 달라지지 않는다` (aiStatus 5값 전부) · `AI 노트 생성이 실패한 뒤에도 전사 탭은 그대로다` · `아직 전사하지 않은 녹음은 AI 상태와 무관하게 전사를 시작할 수 있다` · `전사의 진행과 실패는 전사 자신의 상태에서만 온다` |
| 3. 열람 | `recordingsView` · `recordingDetailView` | `목록은 어떤 AI 상태에서도 그 녹음을 보여준다` · `아직 AI를 시도한 적이 없다는 것이 오류처럼 적히지 않는다` (`Not started`) · `상세와 재생 경로는 어떤 AI 상태에서도 열린다` · `AI 상태가 달라져도 상세 화면의 값이 글자 하나 달라지지 않는다` |

그리고 `AI가 꺼져 있는 것은 오류가 아니다 (INV-8)` describe가 **provider 없음이 오류가 아닌
정상 상태로 표현되는 것**을 판정한다 — `body.kind === 'disabled'`이고 그 값에는 `failure`
프로퍼티가 **아예 없으며**(`not.toHaveProperty('failure')`), `unaffectedNotice`가 무엇이
막히지 않는지 함께 말한다. `AI가 꺼져 있는 그 순간에도 나머지 세 경로가 같은 사실 위에서
그대로 동작한다`가 provider 네 상태(`notConfigured` · `unavailable` · `noModels` · `ready`)
전부에 대해 AI 탭 · 전사 탭 · 목록 · 상세를 함께 확인한다.

### 실제 Whisper / 실제 Ollama에 의존하지 않는다 (A-TRANS-001 · §18)

전사는 `transcription::testing::StubEngine`이 서고 "모델"은 몇 바이트짜리 자리표시자
파일이다. AI는 `ai::testing::FakeNoteAiProvider`가 선다. 네트워크에 닿는 코드는 실행되지
않는다.

---

## AC5 — 오디오가 AI provider로 나갈 코드 경로가 없다 (INV-6)

`src-tauri/tests/audio_never_reaches_ai.rs` (6 tests) — 같은 사실을 **네 각도**에서 못박는다.

### 1. 타입 — 담을 자리가 없다

`the_request_that_reaches_a_provider_has_nowhere_to_put_audio`
— `NoteRequest { mode, transcript, context_budget }`를 **필드를 전부 나열해** 구조 분해한다.
오디오 경로·바이트·핸들을 담는 필드가 하나라도 생기면 이 테스트는 실행 전에 **컴파일되지
않는다**. `TranscriptText { text }`도 같은 방식으로 분해한다 — 담기는 것은 문자열 하나뿐이다.

### 2. 소스 — 파일을 열 수단이 없다

`no_product_source_in_the_ai_boundary_can_open_or_read_a_file`
— `src/ai/**` 전부(계약 · 프롬프트 · 노트 파싱 · 벤더 adapter)와 `src/commands/notes.rs`의
**제품 코드**(주석과 `#[cfg(test)]` 아래를 잘라낸 것)에 다음이 하나도 없음을 확인한다:
`std::fs` · `fs::` · `File::` · `OpenOptions` · `include_bytes!` · `PathBuf` · `Path::new` ·
`audio_path` · `audio_format` · `read_dir`.

`the_orchestration_never_reads_the_column_that_points_at_the_audio`
— 더 강하게: 그 제품 코드에 문자열 `audio`가 **아예 없다**. Recording 레코드에는
`audio_path` 열이 있으므로, 그 값을 읽는 순간 보낼 *수단*이 생긴다. AI 경계는 그 열을
이름으로도 알지 않는다.

(기존 `tests/ai_note_run.rs::the_orchestration_reads_transcripts_and_never_writes_them`이
`ai/run.rs` 하나에 대해 하던 검사를 **경계 전체**로 넓힌 것이다.)

### 3. 전선 — 실제로 나가는 본문에 무엇이 들어 있는가

`everything_that_goes_out_is_built_from_the_mode_and_the_transcript_text_alone`
— 실제 orchestration(`ai::run::generate`) → 실제 adapter(`OllamaProvider`) → HTTP 경계
double(`StubServer`)까지 실제 경로를 지나며, 세 mode 전부에 대해 나간 요청을 기록해서 본다.

- 요청은 정확히 2건(목록 1 · 생성 1)이고 목록 요청에는 본문이 없다.
- 생성 본문의 **최상위 키 집합을 그대로 비교**한다:
  `["format", "model", "options", "prompt", "stream"]`. 오디오를 담을 새 필드가 생기면
  여기서 드러난다.
- 사용자 데이터가 실리는 필드는 `prompt` 하나뿐이며, 그 값이
  `build_prompt(mode, TRANSCRIPT_TEXT)`와 **글자 하나까지 같다**. 즉 나가는 본문 전체가
  `mode + transcript 텍스트 + 모델 이름 + context 크기`에서만 만들어졌다.
- 마지막으로 오디오의 흔적을 직접 찾는다 — 오디오 파일에 넣어 둔 표식 바이트
  (`MOLT-NOTE-AUDIO-BYTES-THAT-MUST-NEVER-BE-SENT`) · 오디오 파일의 전체 경로 ·
  `recording.wav` · `.wav` 어느 것도 요청의 URL과 본문에 없다.

`the_provider_is_handed_the_transcript_text_and_nothing_else`
— 계약 쪽에서도 같은 확인: double이 받은 호출 기록에 `mode` · `transcript` ·
`context_tokens`뿐이며 transcript는 current Transcript의 `raw_text` 그대로다.
생성 뒤 오디오 바이트도 그대로다.

### 4. 행동 — 읽는 코드가 있었다면 여기서 드러난다

`a_note_is_generated_even_when_the_audio_file_is_not_there_at_all`
— 파일이 있는 채로 한 번 생성하고 바이트가 그대로임을 확인한 뒤, **오디오 파일을 지우고**
같은 생성을 다시 한다. 그래도 노트가 만들어진다 — AI 경로가 그 파일을 **한 번이라도**
열었다면 여기서 실패한다. 레코드는 여전히 없는 파일을 가리키고(고치지 않는다), 없는 파일을
만들어 내지도 않는다 (INV-4).

### 네트워크에 의존하지 않는다

연결 대상은 `http://configured-host.invalid:65535`이다. `.invalid`는 어떤 이름 해석도
성공하지 않도록 예약된 TLD이며, HTTP 왕복 자리에는 `StubServer`가 선다. 실제 소켓을 여는
파일이 저장소에 하나뿐이라는 사실은 기존
`tests/ollama_adapter.rs::only_one_file_in_the_repository_can_open_a_socket`가 확인한다.

---

## AC6 — provider 실패 세 종류 뒤에도 Recording · Transcript가 온전하고 재시도 가능하다

`src-tauri/tests/ai_failure_recovery.rs` (3 tests) — **화면이 실제로 지나는 경로 전부**를
지난다.

```
NoteGenerator.start ─→ 배경 스레드 ─→ ai::run::generate ─→ OllamaProvider ─→ HttpTransport
  (command 경계)                       (영속화 규칙)        (실제 adapter)      ↑ 여기만 double
```

대체 구현이 들어가는 자리는 HTTP 왕복 하나뿐이다. 실행자 · 배경 스레드 · orchestration ·
벤더 adapter · 저장소가 전부 제품 코드다.

### 세 실패 (`ways_a_provider_fails`)

| 실패 | 어떻게 만드는가 | 기대 FailureKind | retryable |
| --- | --- | --- | --- |
| 미실행 | `StubServer::refusing()` (연결이 거부된다) | `AiProviderUnreachable` | true |
| 모델 없음 | `StubServer::ready().with_models(vec![])` (서버는 답하는데 목록이 비었다) | `AiModelUnavailable` | false |
| 잘못된 응답 | `with_generate(GeneratedText("죄송하지만 JSON으로 답할 수 없습니다"))` | `AiResponseUnusable` | true |

### `every_way_a_provider_can_fail_leaves_the_recording_and_the_transcript_exactly_as_they_were`

세 실패 각각에 대해 **사후 데이터를 사전 스냅샷과 비교**한다.

- **무엇이 실패했는가** — kind가 셋 다 다르고, `retryable`이 표대로이며, 화면에 그대로
  띄울 수 있는 문장이 있다.
- **원본은 안전한가**
  - Recording: `id` · `title` · `created_at` · `duration_ms` · `audio_path` · `audio_format` ·
    `microphone` · `current_transcript_id` · `transcription_status` · `notion_status`를
    **열 단위로** 비교한다. 바뀐 것은 `ai_status`(와 `updated_at`)뿐이다.
  - Transcript: `transcripts`와 `transcript_segments`의 **모든 행 · 모든 열**을 SQL로 떠서
    문자열로 비교한다(`transcript_tables`). domain 타입 비교가 놓치는 열까지 포함한 바이트
    단위 동일성이다 (INV-2).
  - 오디오 파일: 바이트 비교 (INV-1).
  - `ai_notes`: 비어 있다 — 반쯤 채워진 노트를 남기지 않는다.
- **실패가 UI 상태로 남는가** — 저장된 `ai_status == failed`이고, 화면이 읽는
  `Storage::list_recordings()[0].ai_status == "failed"`이며, `generator.status()`를 **다시
  물어봐도 같은 실패가 온다**(한 번 읽고 사라지지 않는다).

### `a_note_can_still_be_generated_after_each_of_the_three_failures`

세 실패 각각에 대해: 실패시킨 뒤 → 상황이 풀린 것(사용자가 서버를 켜거나 모델을 받은 것)을
응답하는 서버로 표현하고 → **같은 녹음 · 같은 mode**를 다시 요청한다.

- 재시도가 `done`이 되고 `ai_status`가 `Done`으로 덮인다.
- 저장된 노트는 **하나**다 — 실패한 시도가 반쯤 채워진 행을 남기지 않았다는 뜻이다.
- 노트의 `transcript_id`가 실제 입력에 쓴 Transcript이고 `model`이 비어 있지 않다 (§7.3).
- 실패도 성공도 Transcript와 오디오를 건드리지 않았다.

### `a_failure_does_not_take_away_the_note_that_was_already_there`

노트 하나를 성공적으로 만든 뒤 재생성을 실패시킨다. 이미 있던 노트가 **한 글자도 바뀌지
않는다**(UPDATE도 DELETE도 아니다). 마지막 시도가 실패했다는 사실만 상태로 남는다.

### frontend 쪽 대응 (`src/screens/coreWithoutAi.test.ts`)

- `세 실패 각각이 §13의 세 질문에 답하고 재시도 수단을 남긴다` — 세 `FailureKind` 각각에
  대해 `body.kind === 'failed'` · `failure.kind`가 뭉개지지 않음 · `preservedNotice` ·
  이미 있던 노트가 `kept`에 남음 · `retry` 동작이 있음. **`retryable`이 거짓인 갈래
  (모델 없음)에서도 재시도 수단이 남는다.**
- `세 실패 어디에서도 녹음 · 전사 · 열람의 화면 값이 달라지지 않는다` — 달라지는 것은 목록의
  AI 뱃지 하나뿐이고, Transcript 뱃지 · Notion 뱃지 · 상세 화면 · Transcript 탭은 `toEqual`로
  동일하다.
- `앱을 다시 켠 뒤 이유를 모르는 실패도 이미 있던 노트를 잃지 않는다` — 저장된 것이
  `failed`라는 사실뿐일 때 이유를 지어내지 않고(`cause === 'unknown'`) 노트는 그대로 보인다.

---

## AC7 — 새 제품 기능을 추가하지 않았고 외부 의존이 없다

- 변경 파일 4개는 **전부 테스트다** (`changed-files.txt`). `src-tauri/src/**`와 `src/**`의
  제품 모듈은 한 줄도 바뀌지 않았다.
- 실제 Ollama 프로세스: 쓰지 않는다. HTTP 경계에 `ai::ollama::testing::StubServer`가 선다.
- 실제 Whisper: 쓰지 않는다. 전사 엔진 자리에 `transcription::testing::StubEngine`이 서고
  "모델"은 `b"not a real model"` 16바이트 파일이다 (A-TRANS-001).
- 외부 네트워크: 닿지 않는다. 설정 주소는 `.invalid` TLD이고 실제 전송 코드는 실행되지
  않는다.
- 파일시스템 쓰기: 전부 `std::env::temp_dir()` 아래의 고유 디렉터리이며 `Drop`에서 지운다.
- 시간 의존: 시계는 `TestClock`이 주입된다. 배경 스레드 대기는 30초 상한을 두고, 넘으면
  매달리지 않고 **실패한다**.
