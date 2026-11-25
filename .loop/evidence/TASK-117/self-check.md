# TASK-117 self-check (advisory)

Command: `node tools/loop-runtime/loopctl.mjs self-check build lint test`

- build: PASS exit=0
- lint: PASS exit=0 (lint:web + lint:rust)
- test: PASS exit=0 (test:web + test:rust; lib unit tests 736 passed, 0 failed)

New Rust tests in src-tauri/src/export/markdown.rs (all `ok` in .loop-local/self-check/gates/test/stdout.log):

- a_numbered_topic_title_gets_the_renderer_number_exactly_once
  — '1. 제목', '1) 제목', '제목', '01. 제목', '１. 제목', '1 . 제목', '  3） 제목', '１） 제목' → `## 1. 제목`;
    '1．제목' (no space after separator) stays.
- a_title_that_only_looks_like_a_number_stays_as_it_is — '2026년 계획', '3D 재구성', '1.5배 성장' unchanged.
- a_number_is_stripped_only_once — '1. 2. 제목' → `## 1. 2. 제목`.
- a_title_that_would_become_empty_keeps_its_original_text — '1.' → `## 1. 1.`.
- the_second_topic_is_numbered_two_and_stored_titles_are_untouched — second topic `## 2.`, note data unchanged after render.

Change: added pure `strip_leading_topic_number(&str) -> &str`; `render_topic` applies it after `single_line`
and renders `## {number}. {title}` once. No code mutates stored note data. No existing Rust or e2e
expectation relied on doubled numbering, so none needed updating.
