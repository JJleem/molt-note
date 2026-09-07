# 이 Task가 손댄 파일

`git diff --stat`의 수치는 **HEAD 기준**이다. 이 저장소의 working tree에는 앞선 Task들의
커밋되지 않은 변경이 함께 있으므로, 아래 표의 줄 수 중 일부(특히
`RecordingDetailScreen.tsx` · `commands/mod.rs`)는 앞선 Task의 변경을 포함한다.
**이 Task가 실제로 편집한 파일 목록은 그 아래에 따로 적는다.**

```text
 docs/ADR-0010-manual-ai-handoff.md           |  33 ++++
 src-tauri/src/commands/export.rs             |  39 ++--
 src-tauri/src/commands/mod.rs                | 115 +++++++++--
 src-tauri/src/commands/payload.rs            | 218 ++++++++++++++++++++-
 src-tauri/src/export/filename.rs             | 122 +++++++++++-
 src-tauri/src/export/handoff.rs              | 279 +++++++++++++++++++++++++--
 src-tauri/src/export/mod.rs                  |  11 +-
 src-tauri/tests/manual_ai_handoff.rs         | 251 +++++++++++++++++++++++-
 src-tauri/tests/manual_handoff_invariants.rs |  19 +-
 src/ipc/commands.ts                          |  69 ++++++-
 src/ipc/types.ts                             | 140 +++++++++++++-
 src/screens/RecordingDetailScreen.tsx        | 184 +++++++++++++++---
 src/screens/aiHandoffView.test.ts            | 225 ++++++++++++++++++++-
 src/screens/aiHandoffView.ts                 | 188 +++++++++++++++---
 src/screens/copyView.test.ts                 | 169 ++++++++++++++--
 src/screens/copyView.ts                      | 237 ++++++++++++++++++++---
 tests/manual-handoff-invariants.test.ts      |  32 ++-
 tests/ui-foundation.test.ts                  |  46 ++++-
 18 files changed, 2174 insertions(+), 203 deletions(-)
```

## 제품 코드

```text
src-tauri/src/export/handoff.rs        Measure · TakenText · WrittenPortion · take() · no_such_portion()
src-tauri/src/export/filename.rs       ai_request_portion_file_name() · PORTION_MARKER · with_marker()
src-tauri/src/export/mod.rs            re-export
src-tauri/src/commands/payload.rs      TextSizePayload · PortionPayload · HandoffTextPayload · ExportedAiRequestPayload
src-tauri/src/commands/mod.rs          Storage::ai_prompt · Storage::transcript_text · 세 command의 인자와 응답
src-tauri/src/commands/export.rs       Exporter::export_ai_request · write_file<T>
src/ipc/types.ts                       TextSize · PortionOf · HandoffText · ExportedAiRequest
src/ipc/commands.ts                    세 wrapper의 인자와 응답
src/screens/copyView.ts                handoffSize() · portionTaken() · portion을 실은 동작 · copied의 새 값
src/screens/aiHandoffView.ts           같은 두 함수를 쓰는 Export for AI 자리 · next 동작
src/screens/RecordingDetailScreen.tsx  beginCopy · beginAiExport · CopyItem · AiExportItem의 그리기
```

## 테스트

```text
src-tauri/src/export/handoff.rs        (단위 4개 추가)
src-tauri/src/export/filename.rs       (단위 4개 추가)
src-tauri/tests/manual_ai_handoff.rs   (통합 3개 추가 · fixture 헬퍼 3개 추가 · 기존 세 helper 적응)
src-tauri/tests/manual_handoff_invariants.rs  (fixture 세 helper 적응 — 판정하는 불변은 그대로)
src/screens/copyView.test.ts           (describe 1개 · it 6개 추가 · 호출부 적응)
src/screens/aiHandoffView.test.ts      (describe 1개 · it 5개 추가 · 호출부 적응)
tests/ui-foundation.test.ts            (호출부 적응 — 여섯 갈래 문장 검사는 그대로)
tests/manual-handoff-invariants.test.ts (wire 문자열 3개 · marker 2개 갱신 + 이유 주석)
```

**`tests/ipc-boundary.test.ts`는 한 줄도 고치지 않았다** — command 표면은 늘지 않았다.

## 문서

```text
docs/ADR-0010-manual-ai-handoff.md     §12.7 추가 (§8.1 · §12.3의 기록은 지우지 않았다)
```

`docs/SYSTEM-MAP.md`는 손대지 않았다 — Phase가 최종 DONE에 이를 때의 일이다
(`CLAUDE.local.md`의 System Map 규칙).
