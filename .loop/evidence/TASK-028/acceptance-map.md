# TASK-028 Acceptance Criteria → 무엇이 그것을 판정하는가

Gate 결과는 `gates.md`에, 변경 파일은 `changed-files.md`에 있다.
아래 Rust 테스트는 전부 `src-tauri/tests/transcription_background.rs`에 있고,
TypeScript 테스트는 `tests/ipc-boundary.test.ts`에 있다. 둘 다 `npm run test`가 실행한다.

| AC | 판정 수단 | 결과 |
| --- | --- | --- |
| AC1 `npm run build` | gate `build` | exit 0 |
| AC2 `npm run lint` | gate `lint` (eslint + `cargo clippy --all-targets -D warnings`) | exit 0 |
| AC3 `npm run test` | gate `test` (vitest 155건 + cargo test) | exit 0 |
| AC4 전사가 command 호출을 붙잡지 않는다 | `a_status_query_answers_while_the_engine_is_still_transcribing` · `other_commands_answer_while_a_transcription_is_running` | pass |
| AC5 backend가 소유하고 중복 시작이 거부된다 | `starting_the_same_recording_twice_is_refused_instead_of_being_ignored` · `a_second_recording_does_not_queue_up_behind_the_running_one` · `running_and_finished_are_two_different_answers` · `a_finished_transcription_can_be_started_again` | pass |
| AC6 command 등록 · client 1:1 · Failure 계약 · 임의 질의 표면 없음 | `허용된 목록과 정확히 같은 command만 등록되어 있다` · `frontend가 부르는 이름이 등록된 이름과 정확히 같다` · `src/ 아래에 SQL 문장이 없다` · `a_failed_transcription_arrives_as_the_failure_the_engine_reported` · `a_transcription_without_a_chosen_model_reports_the_model_missing_failure` | pass |
| AC7 동시 전사 큐를 만들지 않았다 | `전사 표면은 한 건 시작과 상태 조회 둘뿐이다` · `아직 만들지 않은 기능의 command가 등록되어 있지 않다` · `a_second_recording_does_not_queue_up_behind_the_running_one` | pass |

## AC4 — 순차 호출이 아니라 실제 동시성이다

AC4의 verifier 지시가 묻는 것이 이것이다: 테스트가 **엔진이 도는 도중에** 물어보는가,
아니면 그냥 차례로 부르고 마는가.

판정의 핵심은 `GatedEngine`이다 (`transcription_background.rs` §멈춰 있는 엔진).
이 test double은 `transcribe` 안에서 **두 개의 채널**을 쓴다.

```text
GatedEngine::transcribe
  1. entered.send(())        ← "나는 지금 전사 안에 들어왔다"
  2. release.recv_timeout()  ← 테스트가 풀어 줄 때까지 여기서 블록한다
  3. answer 반환
```

테스트 스레드는 `controls.wait_until_inside()`로 1번 신호를 **받은 뒤에야** 관찰을 시작한다.
그 시점에 엔진은 2번에서 멈춰 있고, `controls.release()`를 부르기 전까지 나올 수 없다.
따라서 그 사이에 일어나는 모든 호출은 **전사가 진행 중인 동안** 일어난 것이다 —
"엔진이 이미 끝나 버린 뒤에 물어봤다"가 성립할 수 없는 구조다.

`a_status_query_answers_while_the_engine_is_still_transcribing`이 그 구간에서 확인하는 것:

1. `wait_until_inside()` — 엔진이 전사 안에 있음이 확정된다.
2. `transcriber.status()`를 5번 부른다. 전부 `running`으로 즉시 답한다.
3. 그 5번에 걸린 시간이 1초 미만임을 `assert`한다. 상태 조회가 엔진을 기다렸다면
   `FINISH_TIMEOUT`(30초)까지 매달렸을 것이므로 이 단언이 실패한다.
4. `engine.call_count() == 1` — 관찰이 끝난 시점에도 엔진은 **여전히 그 한 번의 전사 안에** 있다.
   (엔진이 이미 빠져나왔다면 이 테스트는 동시성을 본 것이 아니게 되는데,
   그 경우 3번 다음의 `controls.release()` / `wait_for_finish` 순서가 성립하지 않는다.)
5. `release()` 후에야 `done`이 된다.

`other_commands_answer_while_a_transcription_is_running`은 같은 관찰 구간에서
**다른 command 표면**을 확인한다 — `Storage::list_recordings`와 `Storage::settings`가
2초 안에 답한다. 이 테스트가 잡는 것은 스레드가 아니라 **저장소 자물쇠**다:
전사가 앱의 연결(`super::Storage`)을 붙들었다면 스레드를 만들어 놓고 DB에서 다시 막는 셈이고,
그러면 이 두 호출이 전사가 끝날 때까지 돌아오지 않는다. 그래서 배경 스레드는
연결을 **새로 연다** (`transcriber.rs::transcribe_one`).

같은 구간에서 확인되는 것이 하나 더 있다: 목록의 `transcriptionStatus`가 이미 `running`이다 —
화면이 읽는 상태도 전사가 끝나기를 기다리지 않는다.

## AC5 — 소유와 중복 거부

**소유**: `Transcriber`는 `lib.rs`의 `app.manage(Transcriber::open_for(app))`가 들고 있다
(`Recorder`와 같은 규약 · `commands/mod.rs` 모듈 문서 · R-001). 상태는
`Arc<Mutex<TranscriptionState>>` 한 값이고 배경 스레드가 그 값에 결과를 남긴다.
`start_transcription`은 **핸들을 돌려주지 않는다** — 돌려주는 것은 상태 payload뿐이므로
화면이 전사를 들고 있을 방법이 없고, 화면이 사라져도 스레드와 상태는 그대로 남는다.

**거부**: `Transcriber::start`는 상태가 `Running`이면 `already_running`으로 `Err`를 낸다.
`starting_the_same_recording_twice_is_refused_instead_of_being_ignored`가 확인하는 것 —

1. 두 번째 `start`가 `Err`다 (`expect_err`). 조용한 무시라면 `Ok`였을 것이다.
2. `kind == InvalidInput`, 메시지가 비어 있지 않고, `source_data_safe`가 참이다.
3. 거부 후에도 진행 중인 전사는 `running` 그대로다 — 거부가 돌던 것을 흔들지 않는다.
4. 끝난 뒤 `engine.call_count() == 1`, `list_transcripts` 1건 — 거부된 요청이
   전사를 한 번 더 돌리지도, Transcript를 하나 더 만들지도 않았다.

거부는 **돌고 있는 동안에만**이다 — `a_finished_transcription_can_be_started_again`이
끝난 뒤의 재시작이 성공하고 Transcript가 2건이 됨을 확인한다 (INV-2).

## AC6 — 표면과 실패 계약

| 규약 | 어디서 |
| --- | --- |
| `lib.rs` 등록 | `commands::start_transcription` · `commands::transcription_status` (`generate_handler!`) |
| 등록 집합 검사 | `ipc-boundary.test.ts` — 부분집합이 아니라 **정확히 같은 집합**을 요구한다 |
| client 1:1 | `startTranscription` → `'start_transcription'` · `transcriptionStatus` → `'transcription_status'`. `frontend가 부르는 이름이 등록된 이름과 정확히 같다`가 양방향으로 검사한다 |
| 타입 | `src/ipc/types.ts`의 `TranscriptionState` · `TranscriptionStatus`가 `TranscriptionStatusPayload`의 camelCase 직렬화와 1:1 |
| Failure 계약 | payload의 `failure` 필드가 기존 `Failure` 그대로다 — 새 오류 모양을 만들지 않았다 |

실패가 계약대로 도착하는지는 `a_failed_transcription_arrives_as_the_failure_the_engine_reported`가
**직렬화된 JSON까지** 확인한다: `failure.kind == "transcriptionEngineFailed"` ·
`sourceDataSafe == true` · `retryable == true` · `transcriptId`는 null.
필드 이름이 어긋나면 화면에서 실패가 조용히 사라지므로 이름을 값으로 단언한다.
`a_transcription_without_a_chosen_model_reports_the_model_missing_failure`는 다른 종류의
실패가 뭉개지지 않고 `transcriptionModelMissing`으로 구분되어 도달하는 것을 확인한다.

**임의 질의 표면 없음**: `start_transcription`이 받는 것은 `recording_id: String` 하나이고,
`transcription_status`는 인자가 없다. SQL도 경로도 필터도 받지 않는다.
`src/ 아래에 SQL 문장이 없다`가 프론트엔드 전체에 대해 이것을 계속 검사한다 (ADR-0001 · §12).

## AC7 — 큐를 만들지 않았다

범위를 넘는 스케줄링 구조가 없다는 것은 세 곳에서 확인된다.

1. **표면** — `전사 표면은 한 건 시작과 상태 조회 둘뿐이다`가 등록된 command 중
   `transcri`가 들어간 이름이 정확히 그 둘임을 요구한다. 큐가 생기면 목록을 걸거나
   대기열을 묻거나 취소하는 이름이 먼저 필요해지므로 표면에서 드러난다.
2. **이름** — `아직 만들지 않은 기능의 command가 등록되어 있지 않다`의 out-of-scope 정규식에
   `queue|batch|schedule`을 추가했다.
3. **동작** — `a_second_recording_does_not_queue_up_behind_the_running_one`:
   전사 중에 **다른** 녹음을 시작하면 거절되고(`retryable`), 앞의 전사가 끝난 뒤에도
   그 요청은 스스로 돌지 않는다 (`call_count == 1`, `rec-second`의 상태는 `None` 그대로,
   Transcript 없음). 줄을 세우는 구조가 있었다면 여기서 두 번째가 돌았을 것이다.

`Transcriber`의 상태 타입 자체에도 대기열 자리가 없다 — `TranscriptionState`는
`Idle · Running · Done · Failed` 넷이고 `Running`이 들고 있는 것은 `recording_id` 하나다.
