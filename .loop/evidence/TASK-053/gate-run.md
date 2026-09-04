# Gate 실행 결과 (Worker 로컬 self-check · 참고용)

Runtime이 Worker 종료 뒤 Gate를 독립적으로 다시 돌린다. 아래는 그 전의 로컬 확인이다.

```text
$ node tools/loop-runtime/loopctl.mjs self-check build lint test

[build] npm run build      → PASS  exit=0  1.0s     (tsc && vite build)
[lint]  npm run lint       → PASS  exit=0  1.8s     (eslint . && cargo clippy --all-targets -- -D warnings)
[test]  npm run test       → PASS  exit=0  4.9s     (vitest run && cargo test)

Self-check: all gates passed
```

## 새 테스트 파일의 실행 결과

`src-tauri/tests/notion_and_export_invariants.rs` (전문은 `gate-test-stdout.log` 665–675행):

```text
running 8 tests
test nothing_in_the_notion_boundary_can_open_or_read_a_file ... ok
test no_source_on_the_way_out_knows_an_ai_vendor ... ok
test a_recording_with_no_ai_note_at_all_goes_out_both_doors_as_one_and_the_same_document ... ok
test an_audio_file_that_really_exists_reaches_neither_the_file_nor_the_request ... ok
test a_token_that_actually_sent_a_recording_is_nowhere_in_the_database_or_in_any_failure ... ok
test an_hour_long_transcript_is_sent_in_pieces_that_reassemble_into_the_original_byte_for_byte ... ok
test both_renderers_consume_the_neutral_note_and_never_the_vendors_own_shape ... ok
test every_way_a_send_can_fail_leaves_the_local_data_untouched_and_still_finishes_on_a_retry ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
```

## 저장소 전체

- vitest: `Test Files 21 passed (21)` · `Tests 384 passed (384)` — 이 Task는 frontend를 바꾸지 않았다.
- cargo test: 30개 test binary 전부 `ok`, 실패 0.

## 첨부

| 파일 | 내용 |
| --- | --- |
| `gate-test-stdout.log` | `npm run test`의 stdout 전문 (vitest + cargo test) |
| `gate-test-stderr.log` | 같은 실행의 stderr (컴파일 · test binary 목록) |
| `diff-stat.txt` | 이 Task의 변경 파일 |
| `coverage-map.md` | 불변 (a)~(f) ↔ 테스트 대응표 |
| `mutation-check.md` | (b) 검사가 실제로 실패할 수 있음을 확인한 기록 |
