# TASK-074 — AC ↔ 검사 대응

## 변경 파일 (둘 다 새 파일이며, 제품 코드는 한 줄도 바뀌지 않았다)

```text
src-tauri/tests/transcription_and_reach_invariants.rs   TR-1 · TR-2 · TR-3(backend) · TR-4
tests/transcription-and-reach-invariants.test.ts        TR-3(화면 done 상태)
```

## AC1 · AC2 · AC3 (Gate)

`.loop/evidence/TASK-074/gate-results.md` — build · lint · test 셋 다 exit=0.

## AC4 — 네 불변 각각에 대응하는 검사가 실재하고, 실제로 그 경로를 지난다

| Phase 5.6 불변 | 검사 | 무엇을 실제로 지나는가 |
| --- | --- | --- |
| (1) 언어가 영어로 강제되지 않는다 | `tr1_the_language_the_settings_surface_stored_is_the_one_the_engine_hears` | `Storage::update_settings` → SQLite → `Transcriber::start` → 배경 스레드 → `TranscriptionEngine::transcribe`. 판정 대상은 **stub 엔진이 받은 `LanguageChoice`** 값이다 |
| | `tr1_no_unset_or_blank_setting_ever_reaches_the_engine_as_a_forced_language` | 같은 경로를 세 설정 값(미선택 · 빈 문자열 · 공백)으로 세 번 지난다. `Chosen("en")`이 아니라는 것까지 본다 |
| | `tr1_the_engine_boundary_never_leaves_the_library_default_in_place` | **원문 각도** — 실제 whisper 추론은 모델이 있어야 실행되므로 (§18) 이 자리에서 값으로 판정할 수 없다. 그 사실을 테스트 주석에 적었다 |
| (2) §D의 language 설정이 실재하고 왕복한다 | `tr2_a_chosen_language_comes_back_from_disk_through_the_settings_surface` | `update_settings` → `settings()`, 그리고 **저장소를 새로 열어** 같은 값을 다시 읽는다 (메모리가 아니라 디스크) |
| | `tr2_saving_a_different_setting_does_not_quietly_erase_the_chosen_language` | 읽어 온 payload에서 **다른 항목만** 바꿔 저장 → 언어가 그대로인지. Falsification에서 **기존 어떤 테스트도 잡지 못한 되돌리기**를 이 검사가 겨냥한다 |
| | `tr2_not_having_chosen_stays_not_chosen_across_other_saves` | 저장하는 김에 언어가 생기지 않는지 |
| (3) 내보낸 파일에 도달하는 수단이 있다 | `tr3_the_file_this_app_just_exported_is_the_one_that_reaches_the_os_boundary` | `Exporter::export`가 **실제로 쓴 파일** → `SavedFiles::show` → 파일 관리자 double에 도착한 정규화 경로 |
| | `tr3_nothing_outside_the_exports_directory_reaches_the_os_boundary` | 저장소 파일 · 밖의 파일 · `..` 탈출 · 디렉터리 자신 넷을 같은 표면으로 보내고, OS 경계가 비어 있는지 본다 |
| | `tr3_the_surface_that_opens_a_saved_file_exists_and_needs_the_owner_that_decides` | command 함수 하나를 **함수 포인터로** 가리킨다. 사라지거나 모양이 바뀌면 컴파일되지 않는다 |
| | (화면) `TR-3a` · `TR-3b` · `TR-3c` | `exportPanel` · `manualHandoff`를 실제로 불러 **done 상태 두 곳의 값**을 읽는다. TR-3c는 파일을 만드는 화면 모듈의 목록을 고정한다 |
| (4) 긴 handoff가 크기 때문에 조용히 실패하지 않는다 | `tr4_a_long_handoff_reports_its_size_instead_of_arriving_silently_truncated` | segment 900개 전사를 저장하고 **세 산출물 전부**(프롬프트 · 전사 텍스트 · 파일)에서 크기·조각 수를 읽는다. 짧은 녹음이 한 조각인 것도 같은 자리에서 본다 |
| | `tr4_the_portions_come_in_order_and_reassemble_into_the_whole_output` | 조각 1..n을 제품 경로로 하나씩 가져와 이어 붙이고, 크기 셋(바이트·글자·줄)과 **문장 900개의 순서·유일성**을 본다 |
| | `tr4_every_portion_of_the_exported_document_is_its_own_file_and_reassembles` | 조각마다 실제 파일을 쓰고, 이름의 표식 · 덮어쓰지 않음 · 디스크에서 다시 읽은 내용의 재조립을 본다 |
| | `tr4_asking_for_a_portion_that_does_not_exist_is_a_failure_not_an_empty_answer` | 없는 조각 요청이 빈 텍스트가 아니라 §13의 실패이고, 그것이 파일을 만들지 않는다는 것 |

되돌리기 실험은 `.loop/evidence/TASK-074/falsification.md`에 있다 — 무엇이 VERIFIED이고 무엇이
UNVERIFIED인지 그 문서가 구분해 적었다.

## AC5 — 제품 동작을 새로 추가하지 않았다 / 판정할 수 없는 것을 판정한다고 주장하지 않는다

- 변경은 위 두 테스트 파일뿐이다. 되돌리기 실험 중 건드린 제품 파일 넷은 전부 복구했고
  `git diff --numstat`이 실험 전 값과 일치한다 (falsification.md).
- 두 파일의 헤더가 **판정하지 않는 것**을 명시한다 — 한국어 전사 품질 · 모델 크기 판정 ·
  Metal 체감 · 실제로 창이 열리는가는 `phase-prompt/05.6`의 Human Review 항목이며, 이 테스트가
  그것을 대신한다고 적은 문장은 없다.
- 실제 whisper · 실제 모델 · 실제 파일 관리자 · 실제 네트워크 · 사용자의 실제 디렉터리를
  세우지 않는다. 모든 임시 파일은 `std::env::temp_dir()` 아래이며 Drop 때 지워진다.
