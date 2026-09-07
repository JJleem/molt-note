# TASK-072 — Acceptance Criteria가 무엇으로 판정되는가

## AC1 · AC2 · AC3 — Gate

`gate-results.md` — build · lint · test 모두 exit 0.

AC2가 요구하는 "§11 형식 불변"의 실제 판정은 `section-11-invariance.txt`에 따로 적었다.

## AC4 — 나눔이 결정적 · 순서 보존 · 이어 붙이면 원본 · 각 조각이 자기 자리를 말한다

규칙: `src-tauri/src/export/portion.rs`

| 요구 | 코드 | 판정하는 테스트 |
| --- | --- | --- |
| 크기가 값으로 나온다 | `TextSize` (portion.rs:81) · `measure` (portion.rs:133) | `measuring_says_how_big_the_text_is_in_bytes_characters_and_lines` |
| 이어 붙이면 원본이다 | 조각이 전부 입력의 부분 슬라이스다 (`Packer::close` — `&self.text[start..end]`) | `assert_portions_are_sound`가 **모든** 나눔 테스트에서 `rejoined(portions) == text`를 확인한다 |
| 순서가 보존된다 | `Packer`가 앞에서 끝난 자리부터만 이어 담는다 (`debug_assert_eq!(open_end, start)`) | 같은 헬퍼 — 이어 붙인 결과가 원본과 **바이트 단위로** 같다는 것이 곧 순서 보존이다 |
| 각 조각이 자기 자리를 말한다 | `Portion { index, total, text }` (portion.rs:101) | `every_portion_says_which_one_it_is_and_how_many_there_are` · 헬퍼가 `index == offset + 1` · `total == portions.len()`를 매번 확인한다 |
| 결정적이다 | 시계 · 난수 · 해시맵 순회가 없다 | `splitting_the_same_text_twice_gives_exactly_the_same_portions` (빈 입력 · 짧은 입력 · 72분 규모 · 3,000줄) |

입력의 크기별 커버리지:

```text
빈 입력          an_empty_text_has_no_size_and_no_portion_at_all            조각 0개
짧은 입력        a_text_under_the_budget_is_one_whole_portion               조각 1개 · 원본 그대로
경계값 = 예산    a_text_of_exactly_the_budget_is_not_split_and_one_byte_more_is
                                                                           40,000B → 1개, +40B → 2개(40,000 / 40)
경계값 + 한 글자 one_character_of_the_budget_is_a_portion_of_its_own        마지막 조각이 "가" 하나
긴 입력          a_text_over_the_budget_is_split_and_rejoins_into_the_original
                                                                           72분 규모(segment 1,711개)를 실제로 쓴다
```

"이어 붙이기 복원 테스트가 실제로 내용 동일성을 확인하는가"(AC4의 Verifier 지시):
`assert_portions_are_sound`는 `assert_eq!(rejoined(portions), text)` — **문자열 전체를 비교한다.**
길이나 개수만 보는 자리가 없다. 마지막 수단(문장보다 잘게 나눈 경우)에서는 글자 수의 합까지
따로 비교한다 (`a_sentence_longer_than_the_budget_is_split_without_losing_a_character` ·
`a_word_longer_than_the_budget_is_split_only_at_character_boundaries`).

조각 경계가 문장 한가운데를 자르지 않는 규칙:

```text
문단(빈 줄) → 줄 → 문장 → 낱말 → 글자        pack() (portion.rs:198)
```

`a_split_lands_on_a_paragraph_boundary_when_it_can` · `a_paragraph_bigger_than_the_budget_is_split_at_its_line_boundaries` ·
`a_line_bigger_than_the_budget_is_split_between_sentences` · `a_number_with_a_dot_in_it_is_not_a_sentence_boundary`.

## AC5 — §11 형식 불변 · 규칙이 한 모듈 안에 있다 · 예산이 벤더 값이 아니다

**§11 형식은 바뀌지 않았다** — `section-11-invariance.txt`.

**규칙이 복제되지 않았다.** transcript 렌더링 규칙은 여전히
`src-tauri/src/export/markdown.rs` 하나에 있다.

```text
markdown.rs:240  enum TranscriptShape { Sectioned, Compact }   ← 모양만 둘이다
markdown.rs:254  segment_block(shape, segment)                 ← 모양이 고르는 것은 이 한 줄뿐
markdown.rs:318  transcript_body(transcript, shape)            ← 어느 segment를 · 어떤 순서로 · 없으면 무엇을
markdown.rs:348  transcript_blocks(transcript, shape)
```

`ai_request.rs`는 그 함수들을 부르기만 하며 `TranscriptShape::Compact`를 상수 하나로 고른다
(`const SHAPE`). 그 파일에 timestamp를 만들거나 segment를 도는 코드는 없다. `both_shapes_choose_the_same_segments_and_fall_back_the_same_way`가
두 모양이 **같은 규칙에서 나온다**는 것을 값으로 확인한다.

**예산이 Notion API 제약에서 온 값이 아니다.**

```text
portion.rs:73   PORTION_MAX_BYTES = 40_000     이 앱의 선택 (72분 실측 99 KB · 5,139줄에서 나왔다)
notion/chunk.rs CHUNK_MAX_BYTES   = 60_000     VERIFIED된 Notion 요청 한도(500KB)에서 유도한 벤더 제약
```

상수 위 주석이 "무엇에서 나왔는가 / 나오지 않았는가"를 나눠 적고, 외부 AI 채팅의 한도는
**UNVERIFIED라고 적는다.** 테스트 `the_budget_is_this_apps_choice_and_not_the_notion_api_constraint`가
두 가지를 함께 못박는다 — 두 상수가 같은 값이 아니라는 것과, 이 모듈의 제품 코드가
`CHUNK_MAX_BYTES` · `CHUNK_MAX_BLOCK_UNITS` · `split_markdown` 어느 것에도 기대지 않는다는 것.

## 이 Task가 하지 않은 것

command 경계도 화면도 건드리지 않았다 (`changed-files.txt`). 크기와 조각을 사람에게 보여 주는
일은 TASK-073의 몫이며, 이 Run이 만든 것은 그 규칙과 단위 테스트까지다.
