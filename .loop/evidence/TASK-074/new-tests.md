# TASK-074 — 새로 생긴 검사 목록

## `src-tauri/tests/transcription_and_reach_invariants.rs` (13건)

```text
382  tr1_the_language_the_settings_surface_stored_is_the_one_the_engine_hears
401  tr1_no_unset_or_blank_setting_ever_reaches_the_engine_as_a_forced_language
433  tr1_the_engine_boundary_never_leaves_the_library_default_in_place
476  tr2_a_chosen_language_comes_back_from_disk_through_the_settings_surface
511  tr2_saving_a_different_setting_does_not_quietly_erase_the_chosen_language
543  tr2_not_having_chosen_stays_not_chosen_across_other_saves
566  tr3_the_file_this_app_just_exported_is_the_one_that_reaches_the_os_boundary
596  tr3_nothing_outside_the_exports_directory_reaches_the_os_boundary
650  tr3_the_surface_that_opens_a_saved_file_exists_and_needs_the_owner_that_decides
668  tr4_a_long_handoff_reports_its_size_instead_of_arriving_silently_truncated
729  tr4_the_portions_come_in_order_and_reassemble_into_the_whole_output
780  tr4_every_portion_of_the_exported_document_is_its_own_file_and_reassembles
838  tr4_asking_for_a_portion_that_does_not_exist_is_a_failure_not_an_empty_answer
```

## `tests/transcription-and-reach-invariants.test.ts` (4건)

```text
TR-3a  두 done 상태가 둘 다 여는 동작을 들고 있고, 그 대상은 backend가 준 경로다
TR-3b  실패한 뒤에도 두 자리에 다시 여는 동작과 전체 경로가 남는다
TR-3b  다른 파일을 열다 실패한 사실은 이 자리에 보이지 않는다
TR-3c  만들어진 파일을 값으로 받는 화면 모듈이 늘어나면 이 검사가 그것을 알린다
```

각 검사에는 **무엇이 깨지면 이 검사가 무엇을 잡는가**가 주석으로 함께 있다 — 저장소의 기존
불변 테스트(`tests/manual_handoff_invariants.rs` · `tests/show_saved_file.rs`)와 같은 밀도다.

## 두 파일이 세우지 않는 것 (§18)

실제 whisper · 실제 모델 파일 · 실제 파일 관리자(창) · 실제 AI provider · 실제 Notion ·
네트워크 · 시스템 clipboard · 사용자의 실제 디렉터리. 임시 파일은 전부
`std::env::temp_dir()` 아래이며 Drop 때 지워진다.
