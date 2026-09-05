# TASK-056 — 세 산출물의 고정된 기대 문자열

아래는 `export::ai_request`의 단위 테스트가 **문자열 전체로 고정한** 값이다. 테스트가 통과했으므로
(`gates.md`) 이것이 실제 산출물이다. 입력은 `markdown.rs`의 테스트가 쓰는 것과 같은 Recording ·
Transcript다 — `3DGS Study #04` · `2026-09-01T10:00:00.000Z` · `3_151_000ms` · segment 둘.

---

## 1. Manual 프롬프트 (Summary mode) — `the_manual_prompt_carries_the_transcript_so_one_paste_is_enough`

`SUMMARY_PROMPT` 원문에 치환 두 번을 적용한 결과다. **transcript를 포함한다** — 한 번의 붙여
넣기로 완결된다 (ADR-0010 §6.2). 끝에 개행을 남기지 않는다.

```text
You turn a transcript into a short structured summary.

Return the note as Markdown and nothing else. No JSON object, no code fence, no prose before or
after it.

Write one `## ` section for each key described below, in that order, using exactly these
headings and nothing else:

## Short Summary
## Key Points

Write a key whose value is a list as one `- ` line per item. Leave out the heading of a section
that has nothing in it.

The object has exactly these keys:
- "shortSummary": string. A few sentences covering the whole transcript.
- "keyPoints": array of strings. The points a reader should take away.

Rules:
- Use only what the transcript says. Do not invent facts.
- Every key must be present. If there are no key points, use an empty array.
- One item is one line of plain text. Do not nest objects or arrays inside an item.
- Write the summary in the main language of the transcript.

Transcript:
### 00:00:03
안녕하세요. 오늘은 3DGS를 봅니다.

### 00:00:06
먼저 splat 표현부터 보겠습니다.
```

치환된 것은 두 자리뿐이다.

```text
(a) "Return exactly one JSON object and nothing else. No prose before or after it, no code fence,
     no explanation."                                    → 위의 Markdown 출력 계약 블록
(b) {{transcript}}                                       → transcript 본문
```

`## Short Summary` · `## Key Points`는 손으로 적은 것이 아니라
`export::markdown::SUMMARY_SECTIONS`(=§9.5의 출력 섹션 이름)에서 왔다.
`the_requested_headings_are_the_ones_this_app_already_renders`가 세 mode 전부에서
**요구 제목 == `*_SECTIONS`** 와 **키 순서 == 섹션 순서**를 판정한다.

---

## 2. Transcript 텍스트 — `the_transcript_text_is_the_section_11_block_with_its_heading`

`## Transcript` 제목을 포함하고, 블록은 §11의 것 그대로다. timestamp는
`format_timestamp_ms`가 만든 `### HH:MM:SS`이며 새 형식을 만들지 않았다.

```text
## Transcript

### 00:00:03
안녕하세요. 오늘은 3DGS를 봅니다.

### 00:00:06
먼저 splat 표현부터 보겠습니다.
```

(개행 하나로 끝난다.)

---

## 3. AI-ready 문서 (Study mode) — `the_ai_ready_document_is_exactly_the_four_sections_and_the_transcript`

섹션 순서는 `Mode` → `Instructions` → `Recording` → `Transcript` 고정이다.
`## Instructions`의 `{{transcript}}` 자리에는 본문 대신 **가리키는 한 문장**이 들어간다 —
같은 문서에서 transcript가 두 번 나오지 않는다.

```text
# Molt Note AI Request

## Mode
Study

## Instructions
You turn a lecture or study session transcript into a structured note.

Return the note as Markdown and nothing else. No JSON object, no code fence, no prose before or
after it.

Write one `## ` section for each key described below, in that order, using exactly these
headings and nothing else:

## Overview
## Key Concepts
## Important Details
## Questions
## Things to Study
## References Mentioned

Write a key whose value is a list as one `- ` line per item. Leave out the heading of a section
that has nothing in it.

The object has exactly these keys:
- "overview": string. A short paragraph describing what was taught or studied.
- "keyConcepts": array of strings. The concepts the material is built on.
- "importantDetails": array of strings. Details worth remembering.
- "questions": array of strings. Questions that help check understanding of this material.
- "thingsToStudy": array of strings. What to study next, based on what was said.
- "referencesMentioned": array of strings. Books, papers, tools or links that were actually
  mentioned. Do not add references that were not mentioned.

Rules:
- Use only what the transcript says. Do not invent facts, sources or numbers.
- Every key must be present. If a section has nothing, use an empty array.
- One item is one line of plain text. Do not nest objects or arrays inside an item.
- Write the note in the main language of the transcript.

Transcript:
The transcript is in the "## Transcript" section of this document.

## Recording
Title: 3DGS Study #04
Date: 2026-09-01
Duration: 52:31

## Transcript

### 00:00:03
안녕하세요. 오늘은 3DGS를 봅니다.

### 00:00:06
먼저 splat 표현부터 보겠습니다.
```

`## Recording`의 세 줄 중 `Date:` · `Duration:`은 `markdown::metadata_block`이 만든 두 줄
그대로이고, `Title:`의 값은 `markdown::heading_text`가 만든 한 줄이다.
**`audio_path` · `audio_format` · 오디오 바이트는 어디에도 없다** (INV-6 · MH-4).

---

## 함께 본 경계

| 무엇 | 테스트 | 결과 |
| --- | --- | --- |
| 결정성 (같은 입력 → 같은 문자열, 세 mode × 세 산출물) | `the_same_input_always_makes_the_same_three_strings` | ok |
| segment가 없는 transcript → `raw_text` 한 문단 (세 산출물 전부) | `a_transcript_without_segments_falls_back_to_its_raw_text_everywhere` | ok |
| 빈 transcript → 빈 제목을 만들지 않는다 (`transcript_text` == `""`) | `an_empty_transcript_leaves_no_empty_section_behind` | ok |
| segment 순서 보존 (정렬하지 않는다) | `segments_keep_the_order_they_were_given` | ok |
| 세 mode 전부의 이름 · 섹션 · wire 값 부재 | `every_mode_says_its_own_name_and_its_own_sections` | ok |
| 끝 개행 규칙 (문서·전사는 개행 하나, 프롬프트는 없음) | `each_output_ends_with_exactly_one_newline_or_none_at_all` | ok |
| audio 부재 (MH-4) | `no_audio_path_and_no_audio_format_reaches_any_of_the_three_outputs` | ok |
| 벤더 부재 (MH-6) | `no_vendor_name_appears_in_any_of_the_three_outputs` | ok |

## 이 Task가 하지 않은 것 (범위 밖)

`get_ai_prompt` · `get_transcript_text` · `export_ai_request` command, `-ai-request` 파일 이름
표식, clipboard 경계, UI는 **만들지 않았다** — ADR-0010 §5.5 · §7 · §8의 P3~P5 몫이다.
그러므로 `tests/ipc-boundary.test.ts`의 네 자리는 이 Task에서 깨지지 않았고, 실제로 vitest
384개가 그대로 통과했다.
