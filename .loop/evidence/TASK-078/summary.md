# TASK-078 — 저장 직전 검사에 붕괴 판정을 더한다

Run: RUN-20260907T041004Z-TASK-078

## 바꾼 것

| 파일 | 무엇 |
| --- | --- |
| `src-tauri/src/transcription/run.rs` | 저장 직전 검사를 `segments.is_empty()` 하나에서 `collapse::assess`의 판정으로 옮겼다. `Empty`는 기존 실패 문장 그대로, `Collapsed`는 새 실패 문장으로 저장을 막는다. `collapsed_output` · `shorten` 두 함수 추가 |
| `src-tauri/tests/transcription_run.rs` | 붕괴한 출력(2026-09-07의 103개짜리)을 내는 test double과 그것을 흘려보내는 테스트 4개 추가 |

판정 규칙(임계값 · 최소 개수 · 비율)은 `src-tauri/src/transcription/collapse.rs`에 그대로
있고 이 Run에서 건드리지 않았다. `run.rs`는 `collapse::assess`를 부르고 그 결과의 수치를
문장에 넣을 뿐이며, 나눗셈도 임계값도 갖지 않는다 (`the_orchestration_neither_copies_the_collapse_rule_nor_drops_the_empty_check`가
소스에서 고정한다).

## 붕괴로 판정됐을 때 (ADR-0007 §18.4 · §18.5)

- `store::append_transcript`에 도달하지 않는다 → Transcript 행이 늘지 않는다
- `store::set_current_transcript`에 도달하지 않는다 → `current_transcript_id`가 그대로다
- `record_failure`가 `transcription_status = failed`를 저장한다
- 실패 종류는 이미 있는 `engine::output_unusable` →
  `FailureKind::TranscriptionOutputUnusable` · `retryable = false` · `source_data_safe = true`
- 사용자가 읽는 문장 (2026-09-07의 값으로 만들어지는 실제 문자열):

  ```text
  전사가 붕괴해 회의 내용이 남지 않았다 — 문장 103개 중 서로 다른 문장이 2개뿐이다.
  "한글자막 by 한효정"이(가) 102번 되풀이됐다
  ```

  detail:

  ```text
  segments=103 · n=103 · u=2 · uniqueRatio=0.019 · r=102 · topRepeatShare=0.990 ·
  Collapsed { unique_ratio_too_low: true, one_sentence_dominates: true } · anomalies=0
  ```

  위 두 문자열의 각 조각을 `a_collapsed_transcription_reaches_the_user_saying_what_went_wrong_and_by_how_much`가
  실제 실행 결과에서 확인한다.

## 새 테스트 (전부 `src-tauri/tests/transcription_run.rs`)

| 테스트 | 고정하는 것 |
| --- | --- |
| `a_collapsed_transcription_is_not_stored_and_leaves_the_previous_one_current` | 재전사가 붕괴했을 때: 종류 · Transcript 행 수(저장소에서 1로 확인) · current(저장소에서 이전 것으로 확인) · `failed` · 원본 오디오 바이트 · Recording 필드 · 이전 Transcript 내용 |
| `a_collapsed_transcription_reaches_the_user_saying_what_went_wrong_and_by_how_much` | 첫 전사가 붕괴했을 때: Transcript 없음 · current `None` · 상태 `pending → running → failed` · 문장이 무엇이 잘못됐는지와 수치(103개 · 2개 · 102번 · 반복 문장)를 담음 · detail의 재현 값 |
| `a_transcription_that_merely_repeats_a_little_is_still_stored` | 고유 94% · 최다 7%(2026-09-05의 정상 관측)는 여전히 저장되고 current가 된다 — 판정이 정상 전사를 버리지 않는다 |
| `the_orchestration_neither_copies_the_collapse_rule_nor_drops_the_empty_check` | 빈 결과 판정이 소스에 그대로 있음 · `collapse::assess`를 부름 · 임계값/비율/상수가 `run.rs`에 복제되지 않음 · 저장된 Transcript를 고치거나 지우는 경로가 없음 (INV-2) |

## Gate 결과 (self-check — 참고용, Runtime이 다시 돌린다)

```text
build: PASS  exit=0
lint:  PASS  exit=0
test:  PASS  exit=0
```

로그: `build-stdout.log` · `lint-stdout.log` · `lint-stderr.log` · `test-stdout.log` ·
`test-stderr.log` (각 gate의 원본 stdout/stderr).

`test-stdout.log`에서 새 테스트 4개가 실제로 돌았음을 확인할 수 있다 (`... ok` 네 줄).

## `changes.diff` 에 대한 주의

`git diff`는 HEAD(`eaf84f4`) 기준이며, 이 두 파일에는 **이 Run 이전의 커밋되지 않은 변경이
이미 있었다** (`run.rs`는 Task 시작 시점에 이미 `M` 상태였다). 따라서 `changes.diff`는 이 Run의
변경만 담고 있지 않다. 이 Run이 만든 것은 위 표의 두 항목이다.
