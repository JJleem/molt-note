# TASK-044 — Acceptance Criteria가 어디서 판정되는가

| AC | 무엇으로 판정되나 | 자리 |
| --- | --- | --- |
| P2-AC1 lint | Gate | `gate-lint.log` — `cargo clippy --all-targets -- -D warnings` exit 0 |
| P2-AC2 test | Gate | `gate-test.log` — `cargo test` 312 passed / vitest 306 passed |
| P2-AC3 순수 · 결정성 | 소스 + 테스트 | `changed-files.md`의 import 목록 · `the_same_input_always_renders_the_same_string`(두 번 렌더해 비교) · `a_recording_with_an_ai_note_renders_exactly_the_document_of_section_11`(기대 문자열 전체 고정) · `the_same_recording_always_gets_the_same_name` |
| P2-AC4 AI Note 없는 입력 | 테스트 | `a_recording_without_an_ai_note_is_still_a_valid_document`(기대 문자열 전체) · `a_document_without_an_ai_note_leaves_no_empty_ai_section_behind`(13개 AI 섹션 이름이 하나도 없음 + 빈 제목·빈 블록 없음) |
| P2-AC5 파일명 정규화 | 테스트 | `filename.rs`의 14개 — 슬래시 · 콜론 · 백슬래시 · 이모지 · 개행 · 제어문자 · `..` · 빈 제목 · 300자 제목 · Windows 예약 이름 · 날짜 이상 모양 |
| P2-AC6 provider 중립 | 소스 + 테스트 | 렌더러의 입력 타입은 `crate::ai::note::StructuredNote` 하나다. `serde_json` · HTTP 응답 · provider 고유 필드가 `export/` 어디에도 없다. `the_renderer_consumes_the_provider_neutral_note_of_section_9_3` · `the_section_titles_are_the_output_section_names_of_section_9_5` |

## 테스트가 고정한 산출물 — AI Note가 있을 때 (§11 그대로)

```markdown
# 3DGS Study #04

Date: 2026-09-01
Duration: 52:31

## Overview
3DGS 스터디 4회차.

## Key Concepts
- Gaussian splatting
- 래스터화

## Questions
- 왜 point cloud로 시작하나?

## Transcript

### 00:00:03
안녕하세요. 오늘은 3DGS를 봅니다.

### 00:00:06
먼저 splat 표현부터 보겠습니다.
```

내용이 없는 Study 섹션 셋(`Important Details` · `Things to Study` ·
`References Mentioned`)은 **제목조차 나오지 않는다** — 배열이 비는 것은 정상이고
(ADR-0008 §7.3), 빈 제목은 그 사실을 말해 주지 않는다.

## 테스트가 고정한 산출물 — AI Note가 없을 때 (INV-8 · §17.1)

```markdown
# 3DGS Study #04

Date: 2026-09-01
Duration: 52:31

## Transcript

### 00:00:03
안녕하세요. 오늘은 3DGS를 봅니다.

### 00:00:06
먼저 splat 표현부터 보겠습니다.
```

같은 렌더러가 같은 구조를 낸다. **AI 섹션의 빈 자리가 남지 않는다.**

## 파일 이름 — 적대적인 제목이 전부 안전한 이름 하나로 떨어진다

```text
"3DGS Study #04"           → 2026-09-01-3dgs-study-04.md
"회의: 로드맵 / Q4 🎯"      → 2026-09-01-회의-로드맵-q4.md
"///" · ".." · "🎯🎯🎯"     → 2026-09-01-untitled.md
"../../etc/passwd"         → 2026-09-01-etc-passwd.md
"..\\..\\Windows\\system32" → 2026-09-01-windows-system32.md
"a/b" "a\\b" "a:b" "a*b" "a?b" "a\"b" "a<b" "a>b" "a|b"
"a\nb" "a\r\nb" "a\tb" "a\0b" "a\u{7}b" "a🎯b" "a\u{200b}b"
                           → 2026-09-01-a-b.md   (열여섯 입력이 같은 하나로)
"CON" · "aux." · "LPT9"     → 2026-09-01-con-file.md · …-aux-file.md · …-lpt9-file.md
"가" × 300                  → 80바이트 경계에서 문자 단위로 잘린다 (26글자)
created_at "not-a-date"     → unknown-date-회의.md   (날짜를 지어내지 않는다)
```

만들어진 이름은 어떤 입력에서도 `/` `\` `:` `*` `?` `"` `<` `>` `|` 제어문자를 갖지
않고, `..`를 포함하지 않고, `.`으로 시작하지 않고, `.md`로 끝난다
(`every_hostile_title_still_produces_one_safe_name`).

## 이 Task가 하지 않은 것 (범위 밖 — TASK-045)

파일 쓰기 · `exports/` 디렉터리 준비 · 같은 이름이 이미 있을 때 번호를 붙이는 규칙
(ADR-0009 §4.3) · Tauri command · frontend. 두 모듈에는 경로도 `std::fs`도 없다.
