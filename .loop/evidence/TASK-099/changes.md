# TASK-099: V2 Meeting E2E Test (no provider)

## Changed Files

### New: `src-tauri/tests/meeting_v2_e2e.rs`
- ~25KB V2 Meeting fixture with: 7 keySummary, previousActionItems (3 done, 3 changed, 3 inProgress, 2 noProgress, 2 notMentioned), 6 topics (each 2-3 groups with multiple items), 16 actionItems (two periods), 6 needsConfirmation
- 6 tests total:
  - `the_fixture_respects_all_size_limits` — AC-8-3: all items within MAX_ITEMS/MAX_ITEM_CHARS/MAX_OVERVIEW_CHARS
  - `the_fixture_is_at_least_25kb` — verifies fixture JSON is ≥25,000 bytes
  - `output_reserve_tokens_covers_the_fixture` — AC-8-3: fixture fits within OUTPUT_RESERVE_TOKENS
  - `the_25kb_v2_fixture_passes_the_full_pipeline` — AC-8-1: parse_note → encode_content → decode_content → Markdown render
  - `the_markdown_contains_v2_sections_in_the_reference_format_order` — AC-8-2: 🔑·🔁·✅·🔍 sections in order
  - `the_markdown_deterministically_renders_the_same_string` — same input = same output

### Modified: `src/screens/aiNoteView.test.ts`
- Added `describe('large V2 Meeting fixture → noteSections (AC-8-1 · AC-8-2)')` with 5 tests:
  - Large V2 fixture passes noteSections and produces correct section structure
  - Section order matches reference format (핵심 요약 → 지난 액션 아이템 → topics → 액션 아이템 → 확인 필요)
  - All 5 previousActionItems categories present in grouped section
  - 16+ action items in list section
  - 6 needsConfirmation items in list section

## Gate Results
- build: PASS
- lint: PASS
- test: PASS
