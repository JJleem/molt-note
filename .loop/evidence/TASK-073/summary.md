# TASK-073 — P7이 만든 크기·나눔 규칙을 사람이 쓰는 자리까지 잇는다

Run: `RUN-20260907T055932Z-TASK-073` · 2026-09-07

TASK-072가 만든 순수 모듈(`src-tauri/src/export/portion.rs` — 크기 측정과 무손실 나눔)을
handoff 실행 순서 → command 경계 → wire 타입 → 순수 view 모듈 → 화면까지 이었다.

---

## 1. backend — 세 산출물 전부가 크기와 자리를 함께 낸다

`src-tauri/src/export/handoff.rs`

```text
Recording ─→ current Transcript ─┬─→ manual_prompt      ─┬─→ TakenText { text, measure }
                                 ├─→ transcript_text    ─┤
                                 └─→ ai_ready_document  ─┴─→ WrittenPortion { file, measure }
                                                          ↑ take() → portion::split / measure
```

- `Measure { total, index, count, size }` — 전체 크기 · 조각 번호(1부터) · 전체 조각 수 ·
  이 조각의 크기. `is_whole()` · `remaining()`.
- `take()`가 고르기와 거절을 한 자리에 둔다. 없는 조각(`0` · `count+1` …)은 **빈 결과가 아니라
  실패**다 (`no_such_portion` · `InvalidInput` · non-retryable · `source_data_safe`).
- 빈 문자열은 `count = 0`이 아니라 조각 하나로 센다 — "1번째 / 전체 0개"라는 읽을 수 없는
  자리를 만들지 않는다.
- 나누는 규칙은 여전히 `portion.rs` 한 자리다. 이 모듈은 부르기만 한다.

`src-tauri/src/export/filename.rs`

- `ai_request_portion_file_name(created_at, title, portion, count)` 추가.
  `count <= 1`이면 기존 이름 그대로, 아니면 `…-ai-request-part-2-of-4.md`.
- 이름 규칙은 여전히 `export_file_name` 하나에서 파생된다 (`with_marker` 하나를 공유).
- 왜 이름에 적는가: `write_new`의 충돌 번호(`-2`)는 **조각의 자리가 아니라 충돌의 순서**다.
  앞서 한 번 내보낸 적이 있으면 둘이 어긋나고, 사용자는 파일 목록만 보고 몇 번째인지 알 수 없다.

## 2. command 경계 — 이름을 늘리지 않고 인자·응답만 넓혔다

| 이름 | 인자 | 응답 |
| --- | --- | --- |
| `get_ai_prompt` | `recordingId` · `mode` · `portion?` | `HandoffTextPayload` |
| `get_transcript_text` | `recordingId` · `portion?` | `HandoffTextPayload` |
| `export_ai_request` | `recordingId` · `mode` · `portion?` | `ExportedAiRequestPayload` |

- 등록 command 표면은 **그대로 서른둘**이다 (`tests/ipc-boundary.test.ts`의 정확-집합 검사가
  손대지 않은 채 통과한다). ADR-0010 §8.1의 "사용자 동작 하나에 이름 하나"가 유지된다.
- `payload.rs`: `TextSizePayload` · `PortionPayload`(flatten) · `HandoffTextPayload` ·
  `ExportedAiRequestPayload`. 마지막 것은 `ExportedFilePayload`를 **감싼다** — 파일 타입을
  두 벌로 만들지 않았다.
- 셋 다 여전히 `transcriptId`를 받지 않는다 (MH-5). provider · AI 설정 · 벤더 고유 개념은
  payload에도 frontend 타입에도 없다 (MH-1 · MH-2 · INV-9).
- `Exporter::write_file`을 제네릭으로 바꿔 두 export가 각자의 응답 타입을 만든다. 자리를 먼저
  준비하고 저장소를 여는 순서는 그대로다.

## 3. wire 타입과 순수 view 모듈

- `src/ipc/types.ts`: `TextSize` · `PortionOf` · `HandoffText` · `ExportedAiRequest`.
  `ExportedFile`의 필드는 그대로 셋이다 (MH-4 원문 검사가 그것을 고정한다).
- `src/screens/copyView.ts`가 표현 규칙의 자리다 — `handoffSize()` · `portionTaken()`.
  `aiHandoffView.ts`가 **같은 두 함수를 그대로 쓴다** (규칙이 두 벌이 되지 않는다).

```text
HandoffSizeView   characters · lines · label · fitsInOne · tooLongNotice
PortionView       index · count · whole · label · remaining
```

- 잘린 결과를 온전한 것이라고 말하는 상태가 없다:

```text
나뉘지 않음  headline = COPIED_HEADLINE            portion.label = "This is all of it."  next = null
조각 1/4     headline = COPIED_PORTION_HEADLINE    portion.label = "Part 1 of 4."        next = 조각 2
조각 4/4     headline = COPIED_PORTION_HEADLINE    text = "That was the last part. …"    next = null
```

  Export for AI도 같은 모양이다 (`AI_EXPORT_DONE_PORTION_HEADLINE` · `next` ·
  `AI_EXPORT_DONE_LAST_TEXT`).
- 동작(`CopyAction` · `AiExportAction`)이 `portion`을 값으로 들고 있으므로 **나머지를 마저
  가져가는 수단이 화면 값으로 존재한다.** 실패한 재시도는 **그 조각으로** 돌아간다(건너뛰지
  않는다).
- 두 순수 모듈의 입력 필드는 하나도 늘지 않았다 —
  `ManualHandoffInput = [recording, mode, copy, aiExport, show]`,
  `CopyPanelInput = [recording, mode, attempt]`. 크기와 자리는 backend 응답을 들고 있는
  attempt에 실려 온다. **provider를 담을 자리는 여전히 없다** (MH-1 · MH-2).

## 4. 화면

`RecordingDetailScreen.tsx`의 AI Note 탭 — Phase 5.5의 두 줄 구조(자동 / 내 AI로 하기)를
바꾸지 않고 아래 줄의 세 자리 안에 크기·자리·다음 조각 버튼을 얹었다. 새 CSS class를 만들지
않았다(`hint` · `share__headline` 등 기존 자리를 쓴다). `beginCopy` · `beginAiExport`는 누른
동작이 들고 있는 `portion`을 그대로 넘길 뿐 스스로 세지 않는다.

---

## Acceptance Criteria 대조

| AC | 판정 | 근거 |
| --- | --- | --- |
| AC1 `npm run build` | PASS exit=0 | `gates.md` |
| AC2 `npm run lint` | PASS exit=0 | `gates.md` (eslint + clippy `-D warnings`) |
| AC3 `npm run test` | PASS exit=0 · vitest 582 · Rust 872 | `gates.md` · `new-tests.txt` |
| AC4 크기와 조각이 화면 값으로 있다 | 위 3절 · `new-tests.txt`의 view 테스트 | `copyView.test.ts` · `aiHandoffView.test.ts` |
| AC5 command 표면·wire 계약 규약 유지 | 위 2절 | `tests/ipc-boundary.test.ts` · `tests/manual-handoff-invariants.test.ts` 무수정 통과 항목 |

**AC3의 두 tripwire 중 갱신한 자리** — `tests/manual-handoff-invariants.test.ts`의 세 wire
문자열과 두 marker다. 그 파일이 스스로 정한 방식대로(§"필드가 하나 늘었다 — …") **무엇이 왜
늘었는지 주석으로 적고** 정확-일치를 그대로 유지했다. 검사를 무르게 만들지 않았다:
`transcriptId` 부재 · provider 부재 · 벤더 부재 · 입력 필드 목록 · 오디오 부재는 손대지 않은
채 통과한다. `tests/ipc-boundary.test.ts`는 **한 줄도 고치지 않았다.**

## 문서

`docs/ADR-0010-manual-ai-handoff.md`에 §12.7을 더했다 — §8.1 · §12.3의 Phase 5.5 기록을 지우지
않고, 인자와 응답이 언제 왜 넓어졌는지를 뒤에 적었다. `docs/SYSTEM-MAP.md`는 건드리지 않았다
(Phase 종료 시점의 일이다).
