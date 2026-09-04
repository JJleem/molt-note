# TASK-033 — Acceptance Criteria별 판정 근거

변경한 파일 (전부 신규, 기존 파일 수정은 `lib.rs` 한 줄):

```text
src-tauri/src/ai/mod.rs      모듈 경계와 그 안에 무엇이 없는지
src-tauri/src/ai/note.rs     세 mode 타입 · content 봉투 · JSON Schema · 방어 경로 + 단위 테스트
src-tauri/src/ai/prompt.rs   프롬프트 상수 · promptVersion · context 예산 판정 + 단위 테스트
src-tauri/src/lib.rs         `pub mod ai;` 한 줄
```

이 Task가 하지 않은 것(다른 Task의 범위): provider 계약 trait · AI `FailureKind` 추가 ·
fake provider · 설정 migration · adapter · orchestration · command · 화면.

---

## AC1 · AC2 · AC3 — build · lint · test Gate

`.loop/evidence/TASK-033/self-check.md` 참조. 세 Gate 전부 exit 0.

---

## AC4 — 세 mode 타입이 ADR-0008 §(d)의 schema와 일치하고 content가 round-trip한다

### 필드 단위 대조 (ADR-0008 §7.2 ↔ `note.rs`)

| mode | ADR §7.2 필드 (§9.5 섹션) | 타입 | 코드의 필드 (serde 이름) |
| --- | --- | --- | --- |
| Meeting | `overview` (Overview) | string | `MeetingNote::overview` → `overview` |
| Meeting | `keyDiscussions` (Key Discussions) | string[] | `key_discussions` → `keyDiscussions` |
| Meeting | `decisions` (Decisions) | string[] | `decisions` |
| Meeting | `actionItems` (Action Items) | string[] | `action_items` → `actionItems` |
| Meeting | `openQuestions` (Open Questions) | string[] | `open_questions` → `openQuestions` |
| Study | `overview` (Overview) | string | `StudyNote::overview` |
| Study | `keyConcepts` (Key Concepts) | string[] | `key_concepts` |
| Study | `importantDetails` (Important Details) | string[] | `important_details` |
| Study | `questions` (Questions) | string[] | `questions` |
| Study | `thingsToStudy` (Things to Study) | string[] | `things_to_study` |
| Study | `referencesMentioned` (References Mentioned) | string[] | `references_mentioned` |
| Summary | `shortSummary` (Short Summary) | string | `SummaryNote::short_summary` |
| Summary | `keyPoints` (Key Points) | string[] | `key_points` |

13개 섹션 = 13개 필드. 타입은 `string`과 `string[]` 둘뿐이며 중첩 객체가 없다 (§7.3).
§7.1이 §9.3 예시에서 바꾼 세 이름(`keyDiscussions` · `openQuestions` · `referencesMentioned`)을
그대로 따랐다.

이 대조를 사람이 다시 하지 않아도 되도록 테스트가 고정한다:

- `the_three_modes_carry_every_output_section_of_section_9_5` — 13개 필드가 직렬화 결과에 있다
- `the_field_names_are_the_section_names_of_section_9_5` — 바뀐 세 이름과, `keyPoints`가
  meeting의 이름이 **아니라는** 것
- `the_schema_asks_for_exactly_the_fields_of_the_type` — `json_schema()`의 `required`가
  타입의 필드 목록과 **같은 순서로 같다**. schema만 조용히 어긋날 수 없다

### content 직렬화 (ADR-0008 §7.5)

`{"schemaVersion":1,"mode":"meeting","note":{...}}` compact JSON 한 줄. 키 순서는 §7.2 표의
순서(선언 순서)로 고정된다 — 봉투를 `serde_json::Value`로 만들지 않고 직접 직렬화한 이유다.

- `content_round_trips_through_storage_unchanged` — 세 mode 전부
  `encode_content → decode_content == 원래 값`, 그리고 다시 담아도 **같은 문자열**
- `the_stored_envelope_is_one_compact_line_in_the_declared_order` — ADR §7.5의 예시 문자열과
  글자 단위로 같다
- `a_note_from_a_newer_schema_version_is_not_guessed_at` — 모르는 `schemaVersion`은
  `FailureKind::Storage`(AI 실패가 아니다), `retryable: false`, `source_data_safe: true`
- `a_mode_that_disagrees_with_the_column_is_a_storage_failure` — 봉투의 `mode`와 `note_type`
  열이 다르면 한쪽을 추측해 고르지 않는다
- `a_parsed_note_can_be_stored_and_read_back` — 방어 경로를 통과한 값이 그대로 저장 형태로 이어진다

---

## AC5 — 기대와 다른 응답에 대한 방어 (다섯 가지 이상, panic 없음)

`parse_note(mode, generated_text)`가 그 경로다. 예외 경로가 아니라 **기본 경로**로 설계했다.

| # | 비정상 입력 | 결과 | 테스트 |
| --- | --- | --- | --- |
| 1 | 필드 누락 | `Shape` 거절 (없는 필드를 지어내지 않는다) | `a_missing_field_is_rejected_rather_than_filled_in` |
| 2 | 타입 불일치 (문자열 자리에 숫자) | `Shape` 거절 | `a_field_of_the_wrong_type_is_rejected` |
| 3 | 배열 자리에 문자열 | `Shape` 거절 | `a_string_where_an_array_belongs_is_rejected` |
| 4 | 알 수 없는 추가 필드 | **무시하고 정상 통과** | `unknown_extra_fields_do_not_break_the_note` |
| 5 | JSON이 아닌 산문 (3종) | `NotJson` 거절 | `prose_that_is_not_json_is_rejected` |
| 6 | 코드펜스로 감싼 JSON + 앞뒤 산문 | **회수해서 정상 통과** | `json_wrapped_in_a_code_fence_is_recovered` |
| 7 | 빈 본문 / 공백뿐 (4종) | `Empty` 거절 | `an_empty_body_is_rejected` |
| 8 | 필수 문자열이 공백뿐 | `BlankRequiredText` 거절 | `a_blank_required_text_is_not_a_usable_note` |
| 9 | 배열 안의 빈 원소 | 버린다 (실패가 아니다) | `blank_array_items_are_dropped_but_an_empty_array_is_normal` |
| 10 | 크기 상한 초과 (원소 길이 · 원소 수 · overview 길이) | **자르지 않고** 거절 | `oversized_output_is_rejected_rather_than_silently_truncated` |
| 11 | 객체 두 개가 이어진 응답 | 바깥쪽 하나만 취한다 (합치지 않는다) | `recovery_happens_once_and_does_not_guess_fields` |
| 12 | 잘린 JSON · 최상위 배열 · `null` · 숫자 · 닫히지 않은 펜스 · 중괄호가 든 문자열 · 제어문자 · 300겹 중첩 (17종 × 3 mode = 51회) | 전부 값으로 돌아온다 | `no_response_body_can_end_the_app` |

**panic이 없다는 것의 근거**

- 위 다섯 단계는 전부 `Result`다. 응답 본문에 대해 `unwrap()`/`expect()`를 쓰는 자리가
  제품 코드에 **하나도 없다** (`unwrap_or`/`unwrap_or_else`만 있으며, 이들은 panic하지 않는다).
- `note.rs`의 유일한 `unwrap_or_else`는 파싱이 아니라 **직렬화** 쪽이며, 실패하면 앱을 끝내는
  대신 스스로를 설명하는 봉투를 남긴다.
- `expect(...)`는 `#[cfg(test)]` 안에만 있다 — 테스트가 스스로에 대해 하는 단언이지 응답 본문에
  대한 것이 아니다.
- 균형 잡힌 `{...}`를 찾는 함수는 재귀가 아니라 반복이며, 첫 `{` 이전에서 시작하지 않으므로
  깊이가 음수가 될 수 없다(underflow 없음). 300겹 중첩 입력이 그 경로를 지난다.

---

## AC6 — 프롬프트가 바뀌면 promptVersion이 바뀐다 (테스트로 강제)

- 값의 형태: `v<SET>.<mode>.<hash8>` (ADR-0008 §10.2-2). 예: `v1.meeting.5c6b8a90`
- 해시 대상: **프롬프트 원문 ‖ 그 모드의 JSON Schema 직렬화 ‖ content schemaVersion**
  (§10.2-3 — 노트를 만든 것은 프롬프트 혼자가 아니다)
- 해시 함수: 저장소 안의 FNV-1a 64bit. `std::hash::DefaultHasher`를 쓰지 않는다 —
  Rust 버전 간 안정성이 없어 툴체인을 올리면 저장된 provenance와 어긋난다 (§10.2-5)
- `prompt_version(mode)`는 **선언된 상수**를 돌려준다. 계산값을 돌려주면 프롬프트를 고칠 때
  값이 조용히 따라 바뀌고 아무도 그 변화를 보지 못한다

강제하는 테스트:

| 테스트 | 무엇을 고정하는가 |
| --- | --- |
| `prompt_version_is_bound_to_the_prompt_text` | 선언된 세 상수 == 지금 상수들로 계산한 값. 어긋나면 세 mode 전부를 한 번에 보고한다 |
| `changing_one_character_of_the_prompt_changes_the_version` | 프롬프트를 한 글자 고치거나 한 문장을 바꾸면 계산값이 **반드시** 달라진다 |
| `changing_the_required_schema_changes_the_version` | 요구하는 schema가 바뀌어도 값이 달라진다 |
| `each_mode_has_its_own_version_value` | 세트 버전과 mode가 값 안에 있고, 세 값이 서로 겹치지 않는다 |
| `the_hash_function_does_not_depend_on_the_toolchain` | FNV-1a의 알려진 값 세 개를 고정한다 |
| `every_prompt_has_exactly_one_place_for_the_transcript` | 실행 중 조립되는 부분이 transcript 자리 하나뿐이다 |
| `the_prompt_asks_for_exactly_the_fields_of_the_schema` | 프롬프트가 요구하는 키와 schema가 요구하는 키가 어긋나지 않는다 |

**이 Gate가 실제로 작동하는 것을 이 Run이 관찰했다** — `self-check.md`의 1·2회차 실패가 그것이다.
버전 상수가 프롬프트와 독립적으로 존재해 조용히 어긋날 수 있는 상태가 아니다.

### transcript를 프롬프트 입력으로 준비하는 자리 (ADR-0008 §8)

`prompt::prepare(mode, transcript, budget)`. §8.2가 고른 것은 **context 크기 명시 + 사전 판정**이며
청킹도 절단도 하지 않는다.

- `estimate_tokens` = `ceil(문자수 / 2)` — tokenizer 사실이 아니라 안전한 과대추정 (§8.5)
- 예산 = `context_tokens − PROMPT_RESERVE(1024) − OUTPUT_RESERVE(1536)`, 기본 context 16384 (§8.4)
- 넘치면 프롬프트를 만들지 않고 `ContextOverflow`로 돌려준다 — **조용히 자르지 않는다**
- 테스트: `the_estimate_is_a_conservative_over_estimate` ·
  `the_budget_keeps_room_for_the_instructions_and_the_output` ·
  `a_transcript_that_fits_is_prepared_with_its_version_and_context_size` ·
  `a_transcript_that_does_not_fit_is_not_truncated_and_not_sent` ·
  `the_boundary_of_the_budget_is_inclusive`

---

## AC7 — 벤더 지식 · 네트워크 · 파일 · 오디오가 없다

`src-tauri/src/ai/{mod,note,prompt}.rs` 세 파일 전체를 대상으로:

| 확인 항목 | 결과 |
| --- | --- |
| 벤더 식별자 (특정 제공자 · 모델 제공사 이름) | **없다.** 문서와 코드 모두 "provider" · "adapter" · "서버"라는 일반 명사만 쓴다 |
| 엔드포인트 · URL · host · port | **없다** |
| 벤더 파라미터 이름 (구조화 출력 파라미터 · context 옵션 이름 · 스트리밍 플래그 등) | **없다.** `json_schema()`는 schema 그 자체를 돌려주며, 그것이 어떤 이름의 파라미터에 실리는지 이 모듈은 알지 않는다. context 크기도 `context_tokens`라는 domain 이름으로만 다닌다 |
| 벤더 에러 코드 · 상태 코드 해석 | **없다.** 응답 본문의 봉투를 여는 1단계는 adapter의 몫이고, 이 모듈은 **생성된 텍스트**부터 받는다 |
| HTTP / 네트워크 의존성 | **없다.** `Cargo.toml`을 바꾸지 않았다 — 이 Task는 의존성을 하나도 추가하지 않았다 |
| 파일 · 경로 · I/O | **없다.** `std::fs` · 경로 타입 · 환경변수 접근이 없다 |
| 오디오 | **없다.** 오디오를 가리킬 수 있는 타입도 필드도 없다 (INV-6) |
| 스레드 · 프로세스 · 시간 | **없다.** 전부 값에서 값을 만드는 동기 함수다 |

`use` 선언 전체(제품 코드):

```rust
// note.rs
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::domain::{Failure, FailureKind, NoteType};

// prompt.rs
use crate::domain::NoteType;
use super::note::{json_schema, CONTENT_SCHEMA_VERSION};
```

`crate::domain`은 벤더가 아니라 이 제품의 domain이다. `NoteType`을 새로 만들지 않고 그대로 쓴
것은 ADR-0008 §4.2(표 5행)의 결정이다 — **같은 세 값을 세는 타입을 두 개 두지 않는다.**

`ResponseRejection`("응답이 기대 schema와 다름")과 `ContextOverflow`("입력이 예산을 넘음")을
§13의 domain 공통 실패로 옮기는 것은 계약을 만드는 Task의 몫이며, 이 모듈은 화면 문자열 계약을
갖지 않는다.
