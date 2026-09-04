# TASK-051 · 화면에 실제로 무엇이 놓이는가

순수 모듈이 만든 값이 **어느 요소로 그려지는지**의 대조표다. 컴포넌트에는 판단이 없다 —
`RecordingDetailScreen.tsx`의 `ExportPanel` · `NotionPanel`은 값을 놓기만 한다.

## 놓이는 자리

```text
Recording Detail (§5 C)
  제목 · 날짜 · 길이
  <audio>  또는  파일 없음 알림
  ┌ section.share ────────────────────────────────────────────┐
  │  Markdown                     Notion   <상태>              │
  │  [Export Markdown]            누르면 무슨 일이 일어나는가   │
  │  만들어진 파일의 전체 경로     [Send to Notion]             │
  │  무엇이 파일에 들어가는가      무엇이 전송되는가            │
  └───────────────────────────────────────────────────────────┘
  탭: AI Note · Transcript · Recording
```

**탭 위에 있다.** 어느 탭을 보고 있든 이 녹음의 Notion 상태가 보여야 하기 때문이다 (요구 9).

## 값 → 요소

| view 값 | 요소 | 왜 그 자리인가 |
| --- | --- | --- |
| `notionPanel(...).status.text` | `h2.share__title > span.share__status` | 상태가 제목 옆에 붙어 본문 상태와 무관하게 언제나 보인다 (§7 · 요구 9) |
| `body.send/again/retry/confirm.outcomeText` | `SendButton`의 `p.hint` — **버튼보다 먼저** | 누르기 전에 읽힌다 (ADR-0009 §8.5 · 요구 8) |
| `…action.confirmation` | `onClick={() => onSend(id, action.confirmation)}` | 버튼이 확인 값을 **스스로 고르지 않는다.** `newPage`를 싣는 버튼은 `newPageAction` 하나뿐이다 (§8.3) |
| `body.progress.text` | 전송 중 · 완료 · 실패 · 확인 요청 모두에 `p.hint` | 부분 전송이 어느 갈래에서도 드러난다 (§8.4 · 요구 12) |
| `body.preservedNotice` | 실패·확인 요청의 `p.hint` | 원본이 그대로라는 사실 (§13 · INV-3) |
| `body.failure` | `FailureNotice` (§13의 세 질문을 그리는 기존 컴포넌트) | 실패 표시 규칙이 화면마다 갈라지지 않는다 |
| `contents.items` · `contents.audioNotice` | `div.share__contents` — **본문 아래에 언제나** | 무엇이 나가는가 · 오디오는 나가지 않는다 (INV-5 · INV-6 · 요구 13) |
| `exportPanel(...).body.file.path` · `.fileName` | `p.share__path` · `p.share__file` | export 위치는 설정으로 노출되지 않으므로 이 줄이 없으면 사용자가 파일을 찾지 못한다 (§4.1). 번호가 붙은 실제 이름이 함께 보인다 (§4.3) |

## 확인 요청을 실패로 그리지 않는다 (§8.5)

`needsConfirmation` 갈래는 `role="status"`인 `div.share__confirm`이며 `FailureNotice`도
`role="alert"`도 쓰지 않는다. 아무것도 하지 않고 거절된 요청이고, 사용자가 할 일은 무슨 일이
일어나는지 읽고 고르는 것뿐이기 때문이다 — AI가 꺼져 있는 상태를 `note__off`로 그리는 것과
같은 규칙이다 (INV-8 · `aiNoteView.ts`).

## 상태 조회 규약

```text
Export      exportMarkdown(id) → 이미 만들어진 파일. 상태를 물어보는 규약을 쓰지 않는다.
Notion      startNotionSync(id, confirmation) → 접수 사실.
            notionSyncStatus() 를 2초마다 물어본다 (보내는 동안에만).
            끝나는 순간 저장된 값을 다시 읽는다 (get_recording · get_notion_sync).
```

전사·노트 생성과 같은 규약이며 (R-001), 그래서 화면을 떠나도 전송은 이어지고 되풀이 조회만
멈춘다.
