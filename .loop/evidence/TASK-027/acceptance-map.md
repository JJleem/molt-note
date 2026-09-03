# TASK-027 Acceptance Criteria → 무엇이 그것을 판정하는가

Gate 결과는 `self-check.md`에, 변경 파일은 `changed-files.md`에 있다.
아래 테스트는 전부 `src-tauri/tests/transcription_run.rs`에 있고 `npm run test`가 실행한다.

| AC | 판정 수단 | 결과 |
| --- | --- | --- |
| AC1 `npm run build` | gate `build` | exit 0 |
| AC2 `npm run lint` | gate `lint` (eslint + `cargo clippy --all-targets -D warnings`) | exit 0 |
| AC3 `npm run test` | gate `test` (vitest + cargo test) | exit 0 |
| AC4 재전사가 UPDATE가 아니라 추가다 | `a_second_successful_run_adds_a_transcript_instead_of_updating_the_first` | pass |
| AC5 실패한 재전사가 current를 바꾸지 않는다 | `a_failed_re_transcription_leaves_the_previous_transcript_as_current` | pass |
| AC6 실패 경로가 원본 오디오·Recording을 건드리지 않는다 | `a_failed_run_leaves_the_original_audio_file_and_the_recording_record_intact` · `nothing_in_the_orchestration_can_remove_or_rewrite_the_source` · `a_missing_model_reaches_the_user_together_with_the_failed_status` | pass |
| AC7 상태가 실제로 저장되고 전이가 테스트된다 · §7 필드가 채워진다 | `a_successful_run_stores_pending_then_running_then_done` · `a_failed_run_stores_pending_then_running_then_failed` · `the_stored_transcript_carries_every_field_section_7_requires` | pass |

## AC4 — 두 Transcript가 함께 남는다

`a_second_successful_run_adds_a_transcript_instead_of_updating_the_first`

1. 같은 Recording을 두 번 전사한다 (`StubEngine::responding_with`으로 서로 다른 출력 2개).
2. `store::list_transcripts`가 **2건**을 돌려준다.
3. 두 id가 서로 다르고, 목록에 둘 다 들어 있다.
4. 첫 Transcript를 재전사 **전에** 통째로 읽어 두고 재전사 **후에** 다시 읽어 `assert_eq!`로
   비교한다 — segments까지 포함해 한 글자도 바뀌지 않는다.
5. 바뀌는 것은 `current_transcript_id`뿐이며 두 번째 Transcript를 가리킨다.

구조적으로도 UPDATE 경로가 없다: 저장소가 내놓는 것은 `store::append_transcript` 하나이고
(`src-tauri/src/db/store.rs` 모듈 문서), `run.rs`는 그 함수만 부른다.

## AC5 — 실패한 재전사 후에도 current가 A다

`a_failed_re_transcription_leaves_the_previous_transcript_as_current`

```text
Transcript A = success / current      1회차: StubEngine → Ok
        ↓
재전사 시도 → 실패                    2회차: StubEngine → Err(TranscriptionEngineFailed)
        ↓
current = Transcript A (그대로)       assert: current_transcript_id == A
transcription_status = failed         assert
Transcript 수 = 1                     assert (추가도 삭제도 없다)
A의 segments = 그대로                 assert_eq!(after, before), segments.len() == 2
```

실패 경로가 `store::set_current_transcript`를 부르지 않기 때문에 값이 유지된다.

## AC6 — 실패가 원본을 건드리지 않는다 (INV-1 · INV-3)

`a_failed_run_leaves_the_original_audio_file_and_the_recording_record_intact`

- 실패 후 `audio_path.is_file()`이 참이고, 파일의 **바이트 전체**가 실패 전과 동일하다
  (`fs::read` 비교) — 전사가 원본을 리샘플해 덮어쓰지 않는다.
- Recording 레코드의 id · title · created_at · duration_ms · audio_path · audio_format ·
  microphone · ai_status · notion_status가 전부 그대로다. 바뀐 것은
  `transcription_status = failed`(그리고 `updated_at`)뿐이다.
- `failure.source_data_safe == true` — §13의 두 번째 질문에 답이 실려 나간다.

`nothing_in_the_orchestration_can_remove_or_rewrite_the_source`는 `run.rs` 소스에
`fs::remove` · `fs::write` · `File::create` · `WavWriter` · `delete_recording`이
**존재하지 않음**을 확인한다 — 테스트가 지나간 경로만이 아니라 그런 코드 자체가 없다.

`a_recording_whose_audio_file_is_gone_fails_without_touching_the_record`는 파일이 앱 밖에서
사라진 경우에도 레코드를 고치거나 지우지 않고 `failed`로만 남는 것을 확인한다 (INV-4).

## AC7 — 상태 전이와 §7 필드

### 저장된 전이 순서

관찰 방법: 테스트가 `recordings` 테이블에 `AFTER UPDATE OF transcription_status` trigger를
걸어 **실제로 행에 쓰인 값**을 순서대로 `status_log`에 적는다. 제품 코드에 테스트 전용
통로를 내지 않고, 제3자(DB)가 관찰한 결과로 판정한다.

```text
성공: ["pending", "running", "done"]
실패: ["pending", "running", "failed"]
모델 없음: ["pending", "running", "failed"]   + FailureKind::TranscriptionModelMissing
없는 Recording: []                             (상태를 쓸 대상이 없으므로 아무 행도 안 건드린다)
```

`none`은 초기값이며 fixture가 `ProcessingStatus::None`으로 저장한 뒤 시작한다.

### 모델 없음이 사용자에게 전달된다

`a_missing_model_reaches_the_user_together_with_the_failed_status`

- `failure.kind == TranscriptionModelMissing` · `retryable == false` ·
  `source_data_safe == true` · `message`가 비어 있지 않다 (§13의 세 질문).
- 저장된 상태가 `failed`이고, 엔진은 한 번도 불리지 않았다 (`call_count() == 0`).
- 원본 오디오 파일은 그대로 있다.

### §7 필드

`the_stored_transcript_carries_every_field_section_7_requires` — 저장 후 다시 읽어서 확인한다.

| §7 필드 | 확인한 값 |
| --- | --- |
| `language` | `Some("ko")` — 엔진이 보고한 값 그대로 |
| `segments[] {start_ms, end_ms, text}` | `[{134000, 141000, "..."}, {141000, 148000, "..."}]` |
| `rawText` | 살아남은 segment 텍스트를 한 칸 공백으로 이은 전문 |
| `engine` | `"stub-transcription-engine"` — `TranscriptionEngine::engine_id()` |
| `model` | `"ggml-base.bin"` — 설정 값이 아니라 **실제로 해석된 모델 파일 이름** |
| `createdAt` | `store::now`가 만든 ISO-8601 UTC 텍스트 (`20…Z`) |

`the_timestamps_are_not_off_by_a_factor_of_ten_or_a_hundred`는 단위 오류를 따로 잡는다:
100cs → 1000ms, 6000cs → 60000ms. ×10 · ×100 오류가 있으면 실패한다
(`phase-prompt/03` 요구 4).

## 그 밖에 Task 서술이 요구한 것

| 요구 | 판정 |
| --- | --- |
| 성공 시 Transcript 한 건 추가 + current가 그것을 가리킨다 | `a_successful_run_appends_one_transcript_and_makes_it_current` |
| 같은 Recording에 대해 다시 시도할 수 있다 | `a_failed_run_can_be_retried_and_then_succeed` (실패 → 재시도 → 성공, Transcript 1건 · current 갱신 · `done`) |
| 파생 입력만 정리 대상이고 정리 실패가 성공을 되돌리지 않는다 | 파생 입력은 디스크에 자리를 갖지 않는 메모리 버퍼다 (`TranscriptionInput`에 경로 필드가 없다 · ADR-0007 §9.1). `run.rs`는 엔진 호출 직후 `drop(input)`으로 놓아 줄 뿐이며 **실패할 수 있는 정리 절차 자체가 없다.** 지울 파일이 없다는 것은 위 소스 검사(`fs::remove`·`WavWriter` 부재)로도 확인된다. |
| 기존 store API를 쓴다 | `update_recording_statuses` · `append_transcript` · `set_current_transcript` · `load_recording` · `list_transcripts` · `new_id` · `now`. `db/store.rs`는 수정하지 않았다. |
