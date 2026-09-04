# TASK-037 — AI 노트 생성 orchestration (provenance · 원본 불변)

```text
Run:   RUN-20260903T071517Z-TASK-037
Date:  2026-09-03
Role:  impl
```

이 문서는 **무엇을 만들었고 어디서 확인할 수 있는지**를 적는다. 완료 판정은 Gate와 Verifier가
한다.

---

## 1. 만든 것

| 파일 | 상태 | 무엇 |
| --- | --- | --- |
| `src-tauri/src/ai/run.rs` | **새로 만듦** (321줄) | Recording 하나에 대한 AI 노트 생성 실행 순서. 저장소에 닿는 유일한 자리다 |
| `src-tauri/tests/ai_note_run.rs` | **새로 만듦** (830줄 · 17 테스트) | 영속성 규칙과 불변식 검증. fake provider + fixture Transcript로만 돈다 |
| `src-tauri/src/ai/mod.rs` | 수정 | `pub mod run` · 재수출 · 모듈 지도 갱신 |
| `src-tauri/src/domain/failure.rs` | 수정 | `FailureKind::AiInputTooLarge` 추가 (ADR-0008 §13.1의 여섯 번째) |
| `src/ipc/failure.ts` | 수정 | 위 실패를 frontend union에 1:1로 추가 (`tests/ipc-boundary.test.ts`가 강제) |
| `docs/ADR-0008-note-ai-provider.md` | 수정 | §17.2 — 구현이 계획에서 달라진 점 보고 (문서가 요구한 절) |

흐름은 Task가 요구한 그대로다.

```text
Recording 레코드 ─→ current Transcript ─→ prompt::prepare ─→ provider ─→ encode_content ─→ insert_ai_note
       │                §7.2 기본 입력      크기 판정 §8      계약뿐        §7.5 봉투        §7.3 provenance
       └─ ai_status:  pending ─→ running ─→ done | failed
```

---

## 2. Gate 결과 (self-check · 참고용)

Runtime이 Worker 종료 후 독립적으로 다시 돌린다. 아래는 이 Run에서 관찰한 값이다.

| Gate | command | exit | 로그 |
| --- | --- | --- | --- |
| build | `npm run build` | **0** | `gates/build/stdout.log` |
| lint | `npm run lint` (`eslint` + `cargo clippy --all-targets -- -D warnings`) | **0** | `gates/lint/stdout.log` |
| test | `npm run test` (`vitest run` + `cargo test`) | **0** | `gates/test/stdout.log` |

- frontend: `Test Files 15 passed (15)` · `Tests 202 passed (202)`
- Rust: 20개 테스트 바이너리 전부 `test result: ok`
- 새 통합 테스트 `ai_note_run`: **17 passed**, 새 단위 테스트 `ai::run::tests`: 5 passed

---

## 3. Acceptance Criteria가 어느 테스트로 판정되는가

| AC | 판정 수단 |
| --- | --- |
| AC1 · AC2 · AC3 | 위 Gate 표 |
| **AC4** Transcript 바이트 불변 · Transcript 여럿 | `generating_a_note_changes_no_byte_of_any_transcript` — `transcripts`와 `transcript_segments`의 **모든 행·모든 열**을 SQL로 떠서 생성 전후를 비교한다(`Fixture::transcript_tables`). fixture는 Transcript **둘**을 갖고, current를 옮겨 양쪽에서 노트를 만든 뒤에도 두 테이블이 그대로다. 실패 경로에서도 같은 비교를 한다(`every_provider_failure_...`). `the_orchestration_reads_transcripts_and_never_writes_them`이 그런 코드가 아예 없다는 것을 소스에서 확인한다 |
| **AC5** transcriptId 구분 · 기본 입력은 current | `notes_from_different_transcripts_are_told_apart_by_transcript_id` — current를 옮겨 두 Transcript에서 각각 노트를 만들고, 저장된 `transcript_id`가 다르며 각 Transcript의 목록에 자기 노트만 있는 것을 본다. **provider에게 넘어간 텍스트도 서로 다르다**(id만 다르게 적은 것이 아니다). `only_the_transcript_text_is_handed_to_the_provider`가 넘어간 값이 current Transcript의 텍스트뿐임을 double의 기록으로 확인한다 |
| **AC6** 실패 세 종류 · 원본 온전 · 상태로 남고 재시도 가능 | `every_provider_failure_leaves_the_source_data_intact_and_stays_visible` — 연결 불가 / 모델 없음 / schema 불일치 셋을 각각 돌려 `kind` · `retryable` · `source_data_safe`를 확인하고, `ai_status = failed`가 **저장되며**(console 로그가 아니다), Transcript 두 테이블 · 오디오 바이트 · Recording의 AI 상태 외 모든 열이 그대로임을 본다. `a_failed_run_can_be_retried_and_then_succeed`가 실패 뒤 같은 요청이 성공하는 것을 확인한다. `a_failed_run_stores_pending_then_running_then_failed`는 DB trigger로 실제 쓰인 상태 순서를 관찰한다 |
| **AC7** provenance · promptVersion | `a_successful_run_stores_one_note_with_every_provenance_value` — **돌려받은 값이 아니라 저장된 행을 다시 읽어** 일곱 값을 전부 확인한다. `the_stored_prompt_version_is_the_version_of_the_prompt_that_was_used`는 상수를 옮겨 적지 않고 **지금의 프롬프트 원문 + JSON Schema에서 다시 계산한 값**(`prompt::computed_prompt_version`)과 비교한다 — 프롬프트를 고치면 이 테스트가 깨진다 |
| **AC8** 재생성 정책 | `regenerating_appends_a_new_note_instead_of_replacing_the_old_one` — ADR-0008 §9.2(append-only)대로 새 행이 INSERT되고 이전 행은 한 글자도 바뀌지 않으며, Transcript와 오디오가 그대로다. `regenerating_keeps_the_provenance_of_the_earlier_note`가 §9.3의 이유(이전 `model`이 지워지지 않는다)를 확인한다 |

Task가 요구했지만 AC에 따로 없는 것도 테스트로 고정했다.

- **current가 없는 Recording은 오류가 아니다** — `a_recording_without_a_current_transcript_is_not_an_error` ·
  `a_recording_without_input_writes_no_status_at_all`. `Outcome::NoTranscriptYet`이며 provider를
  부르지 않고 `ai_status`도 `updated_at`도 건드리지 않는다.
- **AI 상태 외의 recording metadata를 바꾸지 않는다** — 성공·실패 양쪽에서 열 단위로 확인한다.
  `transcription_status`와 `notion_status`는 읽은 값 그대로 다시 쓴다.

---

## 4. 계획에서 달라진 점

`docs/ADR-0008-note-ai-provider.md` **§17.2**에 적었다 (그 문서가 요구한 형식). 요약:

1. §13.1의 여섯 번째 실패(`AiInputTooLarge`)를 이 Task가 추가했다 (TASK-036 §17.1.5가 남긴 자리).
   **계약의 `AI_PROVIDER_FAILURE_KINDS`(다섯)에는 넣지 않았다** — provider가 만드는 실패가 아니다.
2. 입력으로 쓰는 것은 `Transcript::raw_text`다 (§7.2가 어느 필드인지는 정하지 않았다).
3. §9.4의 "바뀌는 것" 목록에 **아무것도 바뀌지 않는 경우**(입력 없음)를 하나 더했다.
4. `prompt::prepare`가 만든 프롬프트 문자열은 쓰이지 않는다 — 요청 본문은 adapter가 만든다.
   이 자리가 `prepare`를 부르는 이유는 사전 크기 판정과 `promptVersion` 둘이다.
5. 저장 직전에 mode 일치를 한 번 더 확인한다 (`ai_notes`에 UPDATE 경로가 없으므로 마지막 자리다).

---

## 5. 이 Run이 확인하지 **않은** 것

- **실제 Ollama를 한 번도 부르지 않았다.** 검증은 전부 `FakeNoteAiProvider`로 돌았다.
- **실제 Whisper 추론 결과를 쓰지 않았다** — Transcript는 손으로 쓴 fixture다 (`A-TRANS-001` 유효).
- 따라서 **노트의 품질에 대해서는 아무것도 말하지 않는다.** 그것은 ADR-0008 §16.3의 Human Review
  항목이며 이 Task가 대신 판정하지 않는다.
