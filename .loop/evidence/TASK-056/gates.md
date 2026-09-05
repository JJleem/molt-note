# TASK-056 — Gate 실행 결과 (Worker self-check · 2026-09-04)

명령: `node tools/loop-runtime/loopctl.mjs self-check lint test`
(Runtime 소유 진입점. `.loop/project.yaml`의 gate 명령만 실행한다.)

```text
[lint] npm run lint     → lint: PASS  exit=0
        eslint . && cargo clippy --all-targets -- -D warnings

[test] npm run test     → test: PASS  exit=0
        vitest run && cargo test --manifest-path src-tauri/Cargo.toml

Self-check: all gates passed
```

원본 출력: `.loop-local/self-check/gates/{lint,test}/{stdout,stderr}.log`
(Runtime 소유 디렉터리이며, 이 파일은 그 결과를 옮겨 적은 것이다. **advisory이지 gate 판정이
아니다** — Runtime이 Worker 종료 후 Gate를 독립적으로 다시 돌린다.)

## 새 모듈의 단위 테스트 16개 (전부 `ok`)

`cargo test` lib 단위 테스트 결과에서 `export::ai_request::tests::*` 만 발췌했다.
(`.loop-local/self-check/gates/test/stdout.log` 221–236행)

```text
test export::ai_request::tests::a_transcript_without_segments_falls_back_to_its_raw_text_everywhere ... ok
test export::ai_request::tests::an_empty_transcript_leaves_no_empty_section_behind ... ok
test export::ai_request::tests::each_output_ends_with_exactly_one_newline_or_none_at_all ... ok
test export::ai_request::tests::no_audio_path_and_no_audio_format_reaches_any_of_the_three_outputs ... ok
test export::ai_request::tests::every_mode_says_its_own_name_and_its_own_sections ... ok
test export::ai_request::tests::no_vendor_name_appears_in_any_of_the_three_outputs ... ok
test export::ai_request::tests::the_ai_ready_document_is_exactly_the_four_sections_and_the_transcript ... ok
test export::ai_request::tests::the_declared_prompt_versions_are_untouched ... ok
test export::ai_request::tests::segments_keep_the_order_they_were_given ... ok
test export::ai_request::tests::the_json_output_contract_is_replaced_exactly_once_in_every_prompt ... ok
test export::ai_request::tests::the_manual_prompt_carries_the_transcript_so_one_paste_is_enough ... ok
test export::ai_request::tests::the_manual_prompt_asks_for_markdown_and_never_for_json ... ok
test export::ai_request::tests::the_manual_prompt_is_built_from_the_untouched_prompt_constants ... ok
test export::ai_request::tests::the_requested_headings_are_the_ones_this_app_already_renders ... ok
test export::ai_request::tests::the_transcript_text_is_the_section_11_block_with_its_heading ... ok
test export::ai_request::tests::the_same_input_always_makes_the_same_three_strings ... ok
```

lib 단위 테스트 전체: `test result: ok. 430 passed; 0 failed` (455행)
vitest: `Test Files 21 passed (21) · Tests 384 passed (384)`

## §11의 산출물 바이트가 그대로인지 (ADR-0010 §5.4의 안전장치)

`markdown.rs`에서 `transcript_body` · `transcript_blocks` · `metadata_block`를 갈라낸 리팩터가
`markdown::render`의 출력을 바꿨다면 **기존 golden 테스트가 먼저 깨진다.** 고치지 않은 채로
전부 통과했다.

```text
export::markdown::tests::a_recording_with_an_ai_note_renders_exactly_the_document_of_section_11 ... ok
export::markdown::tests::a_recording_without_an_ai_note_is_still_a_valid_document ... ok
export::markdown::tests::a_transcript_with_nothing_in_it_leaves_no_empty_section ... ok
export::markdown::tests::a_transcript_without_segments_falls_back_to_its_raw_text ... ok
tests/markdown_export.rs                      → 16 passed
tests/notion_and_export_invariants.rs         → 12 passed
tests/notion_sync.rs                          → 10 passed
```
