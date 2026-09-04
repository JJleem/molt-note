# TASK-034 — 이 Run이 바꾼 파일

| 파일 | 상태 | 무엇을 했는가 |
| --- | --- | --- |
| `src-tauri/src/ai/provider.rs` | 새 파일 | 벤더 중립 provider 계약 — `NoteAiProvider` trait · `ProviderDescriptor` · `Locality` · `Availability` · `NoteRequest`/`TranscriptText` · `NoteGeneration` · §13의 AI 실패 다섯을 만드는 함수 · `AI_PROVIDER_FAILURE_KINDS` |
| `src-tauri/src/ai/testing.rs` | 새 파일 | 재사용 가능한 계약 준수 검사(`assert_note_ai_provider_contract` 외 3) · 결정론적 `FakeNoteAiProvider` · 표본 노트 |
| `src-tauri/src/ai/mod.rs` | 수정 | 새 모듈 선언과 재export, 모듈 주석 갱신 (test double은 재export하지 않는다) |
| `src-tauri/src/domain/failure.rs` | 수정 | `FailureKind`에 AI provider 실패 다섯 추가 · `as_str` 문자열 · 기존 문자열 테스트 확장 |
| `src/ipc/failure.ts` | 수정 | frontend `FailureKind` union에 같은 다섯 문자열 추가 |

`src-tauri/src/lib.rs`의 `pub mod ai;`는 **이 Run이 만든 변경이 아니다** — Run 시작 시점의
git status에 이미 수정된 상태로 있었다(이전 Task의 미커밋 변경).

## AC별 근거 위치

| AC | 어디서 확인되는가 |
| --- | --- |
| AC1 · AC2 · AC3 | `.loop/evidence/TASK-034/self-check.md` (build · lint · test 전부 exit 0) |
| AC4 (INV-6) | `provider.rs`의 `NoteRequest`(필드 셋: `mode` · `transcript` · `context_budget`)와 `TranscriptText`(필드 하나: `text: &str`). `the_generation_request_has_nowhere_to_put_audio` 테스트가 **필드를 전부 구조 분해**하므로 오디오를 담는 필드가 생기면 컴파일되지 않는다 |
| AC5 | `.loop/evidence/TASK-034/failure-mapping.md` · `domain::failure`의 `every_kind_has_a_distinct_stable_string` · `ai::provider`의 `the_five_ai_failures_never_collapse_into_one` · `retrying_is_worth_it_only_where_the_situation_can_change` · frontend 1:1은 기존 `tests/ipc-boundary.test.ts` |
| AC6 | `testing::assert_note_ai_provider_contract(provider: &dyn NoteAiProvider)` — 구현을 인자로 받는다. `the_fake_passes_the_shared_contract_in_every_state`(fake의 네 상태)와 `the_contract_suite_is_not_written_against_the_fake`(테스트 안에서 만든 별개 구현)가 같은 묶음을 통과한다 |
| AC7 | fake는 `ai::testing`에만 있고 `ai/mod.rs`에서 재export하지 않는다. 제품 provider 선택 목록에 없으며, `product_code_never_constructs_the_fake_provider` 테스트가 `src-tauri/src` 전체를 훑어 **테스트 경계 밖에서 `FakeNoteAiProvider::`를 만드는 파일이 없음**을 확인한다 |
