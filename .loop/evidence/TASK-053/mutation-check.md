# (b) 무손실 전송 검사가 실제로 실패할 수 있는가

통과하는 테스트는 그 자체로 증거가 아니다. **1시간 규모 입력의 재조립 비교가 진짜로 나간 데이터를
보고 있는지** 확인하기 위해, 테스트 파일 안에서만 조각 하나를 잃어버린 척하고 gate를 돌렸다.
(제품 코드는 건드리지 않았다.)

## 넣은 변형 (일시적 · 되돌렸다)

`src-tauri/tests/notion_and_export_invariants.rs`,
`an_hour_long_transcript_is_sent_in_pieces_that_reassemble_into_the_original_byte_for_byte`:

```rust
let mut arrived: Vec<String> = requests.iter().map(sent_markdown).collect();
arrived.pop(); // MUTATION CHECK — 마지막 조각을 잃어버린 척한다
```

## 결과 — 잡혔다

`node tools/loop-runtime/loopctl.mjs self-check test` → `test: FAIL exit=101`

```text
running 8 tests
test nothing_in_the_notion_boundary_can_open_or_read_a_file ... ok
test no_source_on_the_way_out_knows_an_ai_vendor ... ok
test a_recording_with_no_ai_note_at_all_goes_out_both_doors_as_one_and_the_same_document ... ok
test an_audio_file_that_really_exists_reaches_neither_the_file_nor_the_request ... ok
test a_token_that_actually_sent_a_recording_is_nowhere_in_the_database_or_in_any_failure ... ok
test an_hour_long_transcript_is_sent_in_pieces_that_reassemble_into_the_original_byte_for_byte ... FAILED
test both_renderers_consume_the_neutral_note_and_never_the_vendors_own_shape ... ok
test every_way_a_send_can_fail_leaves_the_local_data_untouched_and_still_finishes_on_a_retry ... ok

thread '...' panicked at tests/notion_and_export_invariants.rs:558:5:
assertion `left == right` failed: 나눈 순서 그대로 나갔다
```

조각 하나가 사라지면 그 자리에서 실패한다 — 비교 대상이 실제로 stub transport가 받은 요청 본문
전체라는 뜻이다.

## 되돌린 뒤

변형을 제거하고 gate를 다시 돌렸다 (`gate-run.md`): build · lint · test 전부 PASS.

되돌리면서 실패 메시지도 함께 고쳤다. 위 실패 출력에서 보이듯 `assert_eq!`는 20만 바이트짜리
문서 두 벌을 그대로 쏟아 낸다 — 어디가 달랐는지가 오히려 묻힌다. 지금은 **몇 번째 조각이
몇 바이트로 달랐는지**만 말한다. 비교 자체(원문 전체와의 동등성)는 그대로다.
