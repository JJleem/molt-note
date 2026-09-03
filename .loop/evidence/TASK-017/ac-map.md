# TASK-017 — Acceptance Criteria가 무엇으로 판정되는지

Date: 2026-09-02 · Run: RUN-20260902T090601Z-TASK-017 (attempt 2)

Attempt 1은 worker timeout으로 죽었지만 구현 대부분을 작업 트리에 남겼다.
Attempt 2가 확인한 결과, 남아 있던 결함은 **문서 쪽 하나**였다 —
소스가 `docs/ADR-0004-recording-session-lifecycle.md` §10~§13을 순서·보상 정책의
근거로 인용하는데 그 문서에는 §9까지만 있었다. 그 절들을 이번에 작성했다.

## AC1 — test Gate · 네 조건과 두 어긋난 상태

Gate 결과: `self-check.txt` (test PASS exit=0)
전체: cargo 12개 테스트 바이너리 전부 ok, 실패 0.

| 판정 대상 | 테스트 |
| --- | --- |
| 1 파일 경로가 존재한다 | `finalized.rs::a_path_that_holds_no_file_is_a_failure_that_names_the_path` |
| 2 크기가 유효 최소치를 넘는다 | `finalized.rs::a_file_smaller_than_the_minimum_is_refused_and_left_where_it_is` |
| 3 포맷을 알고 있다 | `finalized.rs::a_file_that_cannot_be_read_back_is_refused_rather_than_assumed_to_be_wav`, `a_file_whose_format_differs_from_the_capture_is_refused` |
| 4 메타데이터가 영속화됐다 | `stop_persistence.rs::a_successful_stop_means_the_file_is_confirmed_and_the_record_is_saved` (여섯 값 전부 + 저장소 재조회) |
| 어긋남 A: audio만 있음 | `stop_persistence.rs::a_recording_that_cannot_be_saved_keeps_its_audio_and_tells_the_user_where_it_is` |
| 어긋남 B: row만 있음 | `stop_persistence.rs::a_record_whose_audio_is_gone_is_detected_and_nothing_is_removed_or_repaired` |
| 어떤 실패도 audio를 지우지 않는다 | `stop_persistence.rs::no_failure_on_any_path_removes_audio_that_was_already_written` |

`stop_persistence.rs` 6개 테스트 전부 ok (gate stdout 기준).

## AC2 — lint Gate

`self-check.txt`: lint PASS exit=0 (`eslint . && cargo clippy --all-targets -- -D warnings`)

## AC3 — build Gate

`self-check.txt`: build PASS exit=0 (`tsc && vite build`)

## AC4 — 문서와 구현의 일치 (verifier)

| Verifier 확인 항목 | 문서 | 구현 |
| --- | --- | --- |
| (1) 순서와 보상 정책이 문서에 있고 두 어긋난 상태가 각각 명시됨 | ADR-0004 §11(순서) · §12.1(audio만) · §12.2(row만) · §12.3(왜 보고인가) | — |
| (2) 구현이 그 정책대로다 | §11의 3단계 | `commands/mod.rs:497 finish_recording` — stop(확정+verify) → save_capture |
| (3) audio 삭제 경로가 사용자 명시적 삭제 밖에 없다 | ADR-0004 §13 | `audio-deletion-audit.txt` — 제품 코드에 삭제 호출 0건 |
| (4) DB 저장 실패 시 확정 파일 경로가 사용자에게 간다 | ADR-0004 §12.1 | `not_listed`(mod.rs:519) + `keeping_file`(mod.rs:532) |
| (5) 저장되는 duration이 pause를 제외한 값 | ADR-0004 §5 · §11 | `save_capture`가 `SessionSummary.duration_ms`를 쓴다. 테스트가 1시간 pause 후 5,000ms를 판정한다 |

## 이번 Run에서 바뀐 파일

`docs/ADR-0004-recording-session-lifecycle.md` — §10~§13(+§14) 추가.
소스 변경 없음(attempt 1이 남긴 구현이 이미 정책과 일치했다).
commit하지 않았다 (Task 지시).
