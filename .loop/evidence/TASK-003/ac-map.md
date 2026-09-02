# TASK-003 — Acceptance Criterion별 판정 수단

각 AC가 **어떤 검사로 판정되는지** 대응시킨 표다. Worker의 성공 주장이 아니라,
제3자가 다시 실행해 확인할 수 있는 지점을 가리킨다.

```text
재실행: node tools/loop-runtime/loopctl.mjs self-check lint test
개별:   cargo test --manifest-path src-tauri/Cargo.toml <테스트 이름>
```

---

## AC1 — `npm run lint` 통과 (gate)

`.loop/evidence/TASK-003/gate-lint.log` — exit 0.
eslint와 `cargo clippy --all-targets -- -D warnings` 둘 다 통과한다.

## AC2 — `npm run test` 통과 (gate)

`.loop/evidence/TASK-003/gate-test.log` — exit 0.
vitest 22개 + cargo test 41개(기존 14 + 이번 27) 전부 통과.

## AC3 — 두 번째 Transcript를 저장해도 첫 Transcript가 그대로다 (§7.1)

| 테스트 | 파일 | 무엇을 assertion하는가 |
| --- | --- | --- |
| `appending_a_second_transcript_leaves_the_first_one_untouched` | `src-tauri/tests/domain_model.rs` | `tr-1`을 저장하고 내용·모델이 다른 `tr-2`를 저장한 뒤, `load_transcript("tr-1")`이 **저장 당시의 구조체와 완전히 같은지**(`assert_eq!(first_after, first)`) 본다. 이어서 id(`tr-1`) · `raw_text`("첫 번째 전사") · `segments` · `model`("base")를 각각 다시 확인한다. `list_transcripts`가 `[first, second]` 둘 다 돌려주는 것으로 1:N임을 본다. |
| `reusing_a_transcript_id_fails_and_the_original_row_survives` | 〃 | 같은 id로 다른 내용을 다시 쓰면 UNIQUE 위반으로 실패하고(오류 원문에 `UNIQUE` 포함까지 확인), 원본이 그대로 남으며 행 수가 1개 그대로임을 본다. 즉 덮어쓰기가 **조용히 성공하지 않는다.** |
| `status_changes_move_between_the_five_states_without_touching_the_transcript` | 〃 | Recording 상태를 다섯 번 바꾼 뒤에도 Transcript가 동일함을 본다 (INV-3). |

## AC4 — Transcript와 AINote는 별개이고, provenance가 구분·복원된다 (INV-2 · §7.3)

**시나리오 1 — 덮어쓰지 않음:**
`writing_an_ai_note_does_not_overwrite_the_transcript_it_came_from`
(`src-tauri/tests/domain_model.rs`)
AI Note를 쓰고 다시 재생성 노트까지 쓴 뒤 `load_transcript`가 원본과 동일함을
`assert_eq!`로 확인한다. 노트가 2개인 동안 Transcript는 1개이고, 두 노트의 `content`가
Transcript의 `raw_text`와 다름을 확인한다.

**시나리오 2 — transcriptId로 출처 구분 + provenance 5개 round-trip:**
`ai_notes_are_traced_back_to_the_transcript_version_they_came_from` (〃)
같은 Recording에 Transcript A·B를 두고 각각에 노트를 붙인다.
`recording_id`가 서로 같음을 먼저 확인한 뒤(= recordingId만으로는 구분 불가),
`list_ai_notes_for_transcript(A) == [note-a1]`, `(B) == [note-b1]`로 갈리는 것을 본다.
이어서 다시 읽은 노트에서 다섯 항목을 개별 assertion 한다:

```text
transcript_id  == "tr-b"
provider       == "provider-two"
model          == "model-two"
prompt_version == "meeting-v3"
generated_at   == "2026-09-02T13:20:00Z"
```

`an_ai_note_cannot_claim_a_transcript_that_belongs_to_another_recording`가
어긋난 조합(recordingId와 transcriptId가 다른 Recording을 가리킴)이 복합 FK로
거부되는 것까지 확인한다.

## AC5 — current Transcript의 두 상태와 다섯 상태값

| 테스트 | 무엇을 확인하는가 |
| --- | --- |
| `a_recording_round_trips_with_no_current_transcript` | `current_transcript_id: None`으로 저장한 Recording이 그대로 복원되고 값이 `None`이다 — '값 없음'이 정상 상태다 (§7.2). |
| `current_transcript_can_point_at_a_successful_transcript_and_be_cleared_again` | 값 없음 → 성공한 Transcript 지정 → `Some(tr-current)` 복원 → 다시 비움 → `None` 복원. 그동안 Transcript 자체는 변하지 않는다. |
| `all_five_processing_statuses_are_stored_and_restored_distinctly` | `none·pending·running·done·failed` 다섯을 세 상태 열에 넣어 저장·복원하고, 저장된 DISTINCT 값이 정확히 다섯 개로 갈리는지 SQL로 다시 확인한다. |
| `status_changes_move_between_the_five_states_without_touching_the_transcript` | 다섯 상태를 순서대로 갱신하며 매번 복원값을 확인한다. |
| `storage_refuses_a_status_value_outside_the_five_defined_states` | 여섯 번째 값(`cancelled`)은 CHECK 제약이 막는다 — 다섯 상태 구분이 관례가 아니라 스키마 제약이다. |
| `current_transcript_cannot_point_at_another_recordings_transcript` | current는 같은 Recording의 Transcript만 가리킬 수 있다(복합 FK). 거부돼도 기존 값이 바뀌지 않는다. |
| `the_four_concepts_live_in_four_separate_tables_with_the_fields_section_7_lists` | `PRAGMA table_info`로 네 테이블의 열 목록이 §7이 나열한 필드와 일치하는지 본다. |

## AC6 — Transcript UPDATE 경로 부재 · 벤더 중립 · 문서화

`src-tauri/tests/domain_invariants.rs`는 **소스 자체**를 검사한다.
(검사 대상은 `src/domain/mod.rs` · `src/db/store.rs` · `src/db/migrations.rs`이며,
검사 파일 자신은 대상이 아니다.)

| 테스트 | 무엇을 막는가 |
| --- | --- |
| `no_code_path_updates_or_deletes_an_existing_transcript_row` | 세 소스에 `UPDATE transcripts` · `UPDATE transcript_segments` · `DELETE FROM transcripts(_segments)` · `REPLACE INTO transcripts(_segments)` · `DROP TABLE transcripts`가 없음을 확인한다(공백 정규화 후 대문자 비교라 줄바꿈으로 피해 갈 수 없다). |
| `the_store_exposes_no_api_for_changing_a_stored_transcript` | `pub fn` 이름 중 `transcript` + (`update`·`replace`·`overwrite`·`delete`·`edit`) 조합이 없음을 확인하고, 추가 경로가 `append_transcript` 하나임을 확인한다. |
| `the_domain_does_not_know_any_ai_vendor` | 세 소스에 벤더 이름(Claude·Anthropic·Gemini·OpenAI·GPT-·Ollama·Groq·Mistral·Llama)이 없음을 확인한다 (INV-9). |
| `the_ai_note_provider_is_a_free_form_identifier_not_a_vendor_enum` | domain에 `pub provider: String`이 있고(=enum이 아니고), 스키마가 `CHECK (provider IN …)`으로 값을 제한하지 않음을 확인한다. |
| `the_domain_module_does_not_depend_on_the_storage_layer` | `domain/mod.rs`에 `rusqlite`가 등장하지 않음을 확인한다. |

행동 쪽 근거: `an_ai_note_provider_is_an_opaque_identifier_the_domain_does_not_interpret`가
처음 보는 provider 식별자 4종(`local-runtime` · `some-vendor-x` ·
`self-hosted/gateway-7` · `미지정`)을 그대로 저장·복원한다.

**§7에서 벗어난 설계의 근거:** `docs/ADR-0001-local-persistence.md` §5.4
("§7 데이터 모델을 스키마로 옮길 때의 결정")에 바꾸지 않은 것 / 달라진 것과 이유 /
지금 결정하지 않은 것을 표로 기록했다.

---

## 범위 밖으로 두고 만들지 않은 것

AI 호출 · 전사 실행 · Notion 전송은 이 Task에서 구현하지 않았다.
스키마와 저장·복원, 그리고 그것을 판정하는 테스트만 추가했다.
