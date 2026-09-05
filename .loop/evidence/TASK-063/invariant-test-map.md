# TASK-063 — MH-1 ~ MH-8 불변과 테스트의 대응 (AC-4)

이 Task는 **제품 동작을 추가하지 않았다.** 이미 만들어진 경로(`export::handoff` ·
`export::ai_request` · `commands` 경계 · 화면의 순수 모듈들)를 그대로 지나면서 여덟 불변을 각각
실패할 수 있는 검사로 바꿨다.

새로 만든 파일 둘:

- `src-tauri/tests/manual_handoff_invariants.rs` — 값 수준(실제 저장소 · 실제 파일시스템) + Rust 원문
- `tests/manual-handoff-invariants.test.ts` — 화면 쪽 원문(타입 · 경로 · 호출 자리)

Gate 실행 결과는 `gate-summary.txt`, 전체 목록은 `rust-test-names.txt` ·
`vitest-summary.txt`에 있다.

---

## MH-1 · MH-2 — AI Provider가 하나도 없어도 셋이 전부 동작한다 (로컬 provider를 요구하지 않는다)

| 판정 | 자리 |
| --- | --- |
| 값 | `manual_handoff_invariants.rs::mh1_mh2_all_three_outputs_are_produced_with_no_ai_provider_configured` — `Settings::DEFAULT`(provider·모델·주소 전부 `None`)에서 세 mode × 세 산출물이 전부 만들어진다 |
| 값 | `manual_handoff_invariants.rs::mh1_mh2_a_configured_but_absent_provider_changes_none_of_the_three` — 설치되지 않은 provider와 닫힌 주소를 설정에 저장해도 세 산출물이 **글자 하나 달라지지 않는다** (연결을 시도하는 코드가 있었다면 이 테스트가 멈추거나 실패한다) |
| 원문 | `manual_handoff_invariants.rs::mh1_mh2_the_command_boundary_cannot_read_a_provider_or_the_ai_settings` — command 경계(`commands/export.rs` + `Storage::ai_prompt` · `Storage::transcript_text` + 세 tauri command)에 provider·AI 설정·벤더 이름을 읽는 이름이 없다 |
| 원문 | `manual-handoff-invariants.test.ts::복사와 export가 지나는 어느 자리도 provider 상태나 AI 설정을 읽지 않는다` |
| 타입 | `manual-handoff-invariants.test.ts::아래 줄과 복사 자리의 입력 타입에 provider를 담을 자리가 없다` — `ManualHandoffInput` · `CopyPanelInput`의 필드 목록을 정확히 고정 |
| wire | `manual-handoff-invariants.test.ts::세 command가 recordingId와 mode 말고는 아무것도 보내지 않는다` |

## MH-3 — 복사와 export 경로에 네트워크로 나가는 코드가 없다

| 판정 | 자리 |
| --- | --- |
| 원문 | `manual_handoff_invariants.rs::mh3_no_code_on_the_copy_and_export_path_can_reach_the_network` — 경로의 여덟 자리 전부에 `ureq` · `reqwest` · `std::net` · `Tcp*` · 주소 문자열 · `crate::notion` · `crate::sync`가 없다 |
| 원문 | `manual-handoff-invariants.test.ts::이 경로에 나가는 통로도 주소도 없다` — `fetch(` · `XMLHttpRequest` · `WebSocket` · `EventSource` · `sendBeacon` · 주소 문자열 |
| 값 | `manual-handoff-invariants.test.ts::앱이 아무 데도 보내지 않는다는 사실이 화면 값으로 있다` |

## MH-4 — AI-ready 산출물과 그 입력 타입에 audio 경로도 바이트도 없다

| 판정 | 자리 |
| --- | --- |
| 값 | `manual_handoff_invariants.rs::mh4_no_audio_path_and_no_audio_format_reaches_the_outputs_or_the_written_file` — 알아볼 수 있는 오디오 경로·형식을 넣고 세 산출물과 **쓰인 파일의 이름**까지 훑는다 |
| 타입 | `manual_handoff_invariants.rs::mh4_the_types_on_this_path_have_no_place_to_carry_audio` — `AiRequest` · `ExportDocument` · `ExportedFilePayload` 선언에 오디오를 담을 필드가 없고, 경로의 어느 코드도 `audio_path` · `audio_format` · `fs::read(`에 닿지 않는다 |
| 타입 | `manual-handoff-invariants.test.ts::돌아오는 값의 타입에 오디오를 담을 자리가 없다` (`ExportedFile`) |
| 원문 | `manual-handoff-invariants.test.ts::이 경로의 어느 자리도 오디오 파일을 읽거나 실어 나르지 않는다` |

## MH-5 — 현재 성공한 Transcript(`current_transcript_id`)만 쓰인다

| 판정 | 자리 |
| --- | --- |
| 값 | `manual_handoff_invariants.rs::mh5_the_three_outputs_follow_the_current_pointer_and_nothing_else` — version 둘 중 current만 쓰이고, 포인터를 옮기면 셋이 함께 따라오며, 포인터가 비면 둘 다 남아 있어도 거절된다 |
| 원문 | `manual_handoff_invariants.rs::mh5_no_code_on_this_path_can_pick_a_transcript_version_by_itself` — 경로 어디에도 `list_transcripts` · `ORDER BY` · `sort_by` 같은 두 번째 선택 규칙이 없고, 고르는 자리는 `export::run::current_input` 하나다 |
| wire | `manual-handoff-invariants.test.ts::세 wrapper 어디에도 transcriptId가 실리지 않는다` |
| 원문 | `manual-handoff-invariants.test.ts::복사와 export 자리가 보는 것은 current 포인터 하나다` |

세 command의 계약(`recordingId`만 받는다)은 이미 `src-tauri/tests/manual_ai_handoff.rs`가
본다 — 여기서 다시 쓰지 않았다.

## MH-6 — 산출물과 core/domain · payload · frontend 타입에 벤더 이름과 벤더 고유 schema가 없다

| 판정 | 자리 |
| --- | --- |
| 값 | `manual_handoff_invariants.rs::mh6_no_vendor_name_reaches_the_outputs_or_the_written_file` — 세 mode × 세 산출물 |
| 원문 | `manual_handoff_invariants.rs::mh6_no_vendor_name_or_vendor_schema_is_in_the_domain_and_payload_types` — `domain/**` · `commands/payload.rs` |
| 원문 | `manual-handoff-invariants.test.ts::벤더 이름을 글자로 아는 제품 파일이 provider를 고르는 자리 하나뿐이다` |
| 원문 | `manual-handoff-invariants.test.ts::IPC 타입과 이 경로의 코드에 벤더 고유 schema가 없다` |

**발견 사항(불변을 무르게 고치지 않고 그대로 드러낸 것):** frontend 제품 소스 중 벤더 이름을
글자로 담고 있는 파일이 하나 있다 — `src/screens/aiProviderSettings.ts`의
`SELECTABLE_AI_PROVIDERS`(사용자가 provider를 **고르는** 목록)이다. 고를 수 있는 것의 이름은
고르기 전에도 화면에 있어야 하므로 이것은 Rust 쪽 INV-9가 adapter 디렉터리에 두는 예외와 같은
성질이다. 검사를 없애는 대신 **파일 하나로 못박았고**(목록이 늘면 테스트가 먼저 알린다), 그
파일이 Manual Handoff의 세 동작을 알지 않는다는 것도 함께 확인한다. Manual Handoff 경로 자체와
IPC 타입에는 벤더 이름이 하나도 없다.

## MH-7 — 복사와 export가 실패해도 저장된 것과 이미 내보낸 파일이 그대로다

| 판정 | 자리 |
| --- | --- |
| 값 | `manual_handoff_invariants.rs::mh7_four_failures_leave_the_database_bytes_and_the_exported_files_untouched` — 없는 녹음 · current 없음 · 빈 전사 · 빈 recordingId의 **열 가지 실패** 뒤에 DB 파일의 **바이트**와 이미 내보낸 파일 둘의 내용이 그대로다 (연결된 provider가 만든 AI Note 포함) |
| 값 | `manual_handoff_invariants.rs::mh7_a_place_that_cannot_be_written_changes_nothing_either` — 쓸 자리가 막혀 있어도 그 자리의 사용자 파일과 DB가 그대로이고, 복사 둘은 그 상황과 무관하게 성공한다 |
| 원문 | `manual_handoff_invariants.rs::mh7_the_command_boundary_has_no_way_to_write_to_the_repository` — 저장소 쓰기·삭제 이름이 없고, 파일을 만드는 유일한 자리가 `create_new`다 |
| 원문 | `manual-handoff-invariants.test.ts::두 순수 모듈은 command를 부를 수단 자체가 없다` |
| 원문 | `manual-handoff-invariants.test.ts::실제로 거는 두 자리가 읽기와 파일 하나 더하기 말고는 아무것도 하지 않는다` |
| 값 | `manual-handoff-invariants.test.ts::실패해도 원본이 그대로라는 사실이 화면 값으로 있다` |

## MH-8 — 기존 Connected Provider 생성 경로가 그대로 동작한다

| 판정 | 자리 |
| --- | --- |
| 값 | `manual_handoff_invariants.rs::mh8_the_connected_provider_path_still_makes_notes_next_to_the_manual_path` — `ai::run::generate`가 노트를 만들고 provenance가 남으며, 세 산출물을 만든 전후로 노트가 그대로다 (provider 자리에는 계약이 같은 test double이 선다) |
| 원문 | `manual-handoff-invariants.test.ts::위 줄은 aiNoteView가 만든 값을 그대로 들고 있을 뿐 다시 만들지 않는다` |
| 원문 | `manual-handoff-invariants.test.ts::화면은 여전히 기존 생성 경로를 건다` |
| 회귀 | 기존 테스트 전부가 계속 통과한다 — `test` Gate 결과 (`vitest 468 passed` · `cargo test` 전체 PASS) |

---

## 검사가 진짜 검사인가 (원문 검사의 공허함 방지)

원문 검사는 **아무것도 읽지 못하면 언제나 통과한다.** 그래서 두 파일 모두 그것을 먼저 막는다.

- `manual_handoff_invariants.rs::the_source_checks_actually_read_the_code_they_claim_to_read`
  — 슬라이스한 command 경계에 여섯 호출이 실제로 들어 있고, 경로의 여덟 자리가 전부 200자를
  넘으며, 주석 제거가 실제로 일어난다(`ai_request.rs`의 `audio_path`는 원문에는 있고 제품
  코드에는 없다).
- `manual-handoff-invariants.test.ts::경로의 여덟 자리를 전부 읽었고, 그 안에 실제 코드가 있다`
  · `주석 제거가 실제로 일어난다`.

## 이 테스트들이 쓰지 않는 것 (AC-5)

- 실제 AI provider에 요청하지 않는다 — `ai::testing::FakeNoteAiProvider`(계약이 같은 test double)
- 실제 Notion에 요청하지 않는다 — 이 경로는 Notion을 알지 않으므로 세울 것도 없다
- 실제 OS 자격증명 저장소를 열지 않는다 — 이 경로에 token이 없다
- 실제 시스템 clipboard를 건드리지 않는다 — clipboard 쓰기는 이 경계 밖이며,
  `tests/screen-boundary.test.ts`가 저장소의 모든 테스트에 대해 그것을 이미 지킨다
- 사용자의 실제 디렉터리를 건드리지 않는다 — 전부 `std::env::temp_dir()` 아래이며 Drop 때 지워진다
- 픽셀/스크린샷 비교 테스트를 추가하지 않았다 —
  `manual-handoff-invariants.test.ts::픽셀 비교 테스트가 없다`가 저장소 전체에 대해 확인한다
- 위 넷 중 셋(provider · Notion · 자격증명 저장소)은
  `manual-handoff-invariants.test.ts::실제 provider도 실제 Notion도 실제 자격증명 저장소도 세우지 않는다`가
  이 Phase의 테스트 파일 다섯을 원문으로 확인한다
