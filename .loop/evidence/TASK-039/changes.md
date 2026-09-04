# TASK-039 — Recording Detail의 AI Note 탭

## 바꾼 파일

| 파일 | 상태 | 무엇 |
| --- | --- | --- |
| `src/screens/aiNoteView.ts` | 새로 만듦 (712줄) | 순수 모듈. 이 탭의 모든 판단·표시 규칙 |
| `src/screens/aiNoteView.test.ts` | 새로 만듦 (687줄) | 그 모듈의 단위 테스트 (vitest) |
| `src/screens/RecordingDetailScreen.tsx` | 수정 | placeholder → `AiNoteTab` 컴포넌트. 그리기만 한다 |
| `src/App.css` | 수정 | `.note*` 클래스. AI 비활성 상태에는 경고색·테두리가 없다 |

`src/ipc/*`는 이 Task에서 건드리지 않았다 — command와 타입은 TASK-038까지 이미 있었다.

## 형태 — `transcriptView.ts` / `TranscriptTab`과 같다 (AC7)

```text
backend가 준 사실  →  aiNoteView.ts (순수)  →  AiNoteTabView  →  AiNoteTab (그리기만)
```

- `aiNoteView.ts`는 React도 DOM도 Tauri도 import하지 않는다. import는 `../ipc/failure`와
  `../ipc/types`(타입만) 둘뿐이다.
- 컴포넌트에는 `if (state === ...)`로 무엇을 보일지 정하는 코드가 없다. `body.kind`로 갈라
  이미 만들어진 값을 놓기만 한다. 문장 · 버튼 라벨 · 재시도 가능 여부 · mode를 바꿀 수 있는지가
  전부 순수 모듈이 만든 값이다.
- command를 부르는 것은 컴포넌트다. 순수 모듈은 동작을 **값**으로만 내놓는다
  (`AiNoteAction { kind · label · recordingId · mode }`).

## 상태 일곱 갈래

```text
loading        레코드 · provider 상태 · Transcript · 저장된 노트 중 아직 읽지 못한 것이 있다
disabled       AI 기능이 켜지지 않았다 — 오류가 아니다 (INV-8)
noTranscript   만들 재료가 아직 없다 — 실패가 아니다 (§7.2)
none           이 mode의 노트를 아직 만든 적이 없다
generating     지금 만들고 있다 — 이미 있던 노트는 그대로 보인다
ready          노트를 구조 그대로 볼 수 있다 (§9.3)
failed         생성이 실패했다 — 원본도 기존 노트도 그대로다 (§13 · INV-3)
```

판정 순서에 규칙이 둘 있고 `aiNoteTab`의 주석에 적었다.

1. **이미 돌고 있는 생성이 가장 먼저다** — 실제로 벌어지고 있는 일을 저장된 상태값이 덮지 않는다.
2. **provider가 준비되지 않았다는 사실은 실패보다 먼저다** — 지금 AI를 쓸 수 없다는 것이
   먼저 알아야 하는 사실이고, 그것을 실패로 그리지 않는 것이 INV-8이기 때문이다.
   그래서 `provider.state !== 'ready'`이면 어떤 경우에도 `failed`가 되지 않는다.

## Acceptance Criteria와 근거

### AC1 · AC2 · AC3 — build · lint · test Gate

`.loop/evidence/TASK-039/self-check.md` 참고. 셋 다 exit 0이다.

### AC4 — 세 mode 선택 · 생성 · structured note 구조 그대로 렌더

- mode 선택: `noteModeChoices(selected)`가 §9.5 순서로 세 값을 만들고, 고른 것 하나만
  `selected`다. 컴포넌트는 그 목록을 버튼으로 놓고 `onMode`로 되돌려줄 뿐이다.
  `modeSelectable`이 지금 바꿀 수 있는지를 말한다 (생성 중 · AI 비활성일 때 거짓).
- 생성: `AiNoteAction`이 `recordingId`와 **고른 mode**를 들고 있고, 컴포넌트가 그대로
  `start_ai_note(recordingId, mode)`에 넘긴다.
  테스트 `세 mode를 골라 생성할 수 있다`가 세 mode 모두를 판정한다.
- 렌더: `noteSections(StructuredNote)`가 §9.5의 섹션 이름과 순서를 만든다.
  - Meeting 5 · Study 6 · Summary 2 — 테스트가 제목 배열을 리터럴로 고정한다.
  - 문단(`text`)과 항목(`list`)이 다른 종류로 남아 화면이 문단을 목록처럼 그리지 않는다.
  - **content 문자열을 그대로 출력하는 경로가 없다.** `NoteView`에는 `content` ·
    `markdown` · `rawText` 같은 단일 본문 필드가 없고 (테스트가 `not.toHaveProperty`로 고정),
    컴포넌트가 그리는 것은 `section.title` · `section.text` · `section.items`뿐이다.
  - 빈 배열은 실패가 아니라 `emptyText`로 담담히 표시된다 (ADR-0008 §7.3).

### AC5 — provider 미설정/미연결이 비활성 상태 (INV-8)

- `notConfigured` · `unavailable` · `noModels` 셋 모두 `body.kind === 'disabled'`다.
  테스트 `어떤 provider 상태에서도 실패로 그려지지 않는다`가 셋을 한 번에 판정한다.
- **그 값에는 `Failure`가 없다.** `AiDisabledNotice`에 `failure` 필드가 아예 없고,
  테스트가 `expect(view.body).not.toHaveProperty('failure')`로 고정한다.
  `provider.failure`(unavailable일 때 오는 값)도 화면 상태로 옮기지 않는다.
- 컴포넌트도 이 상태를 `FailureNotice`로 그리지 않는다 — `role="status"`인 `.note__off`이며,
  `.failure` 클래스도 `role="alert"`도 쓰지 않는다.
- provider를 고르지 않았을 때 생성을 걸어 받은 `aiProviderNotConfigured`도 같은 비활성
  상태로 간다 (테스트 `비활성 상태에서 생성을 요청했다가 받은 답도 실패로 그리지 않는다`).
- **Transcript 탭과 재생이 막히지 않는다**: 테스트
  `AI가 꺼져 있어도 Transcript 탭과 재생은 그대로다`가 같은 사실 위에서 세 모듈을 함께 부른다 —
  `aiNoteTab(...)` → `disabled`, `transcriptTab(...)` → `done`, `loadedRecordingDetail(...)` →
  `playable`. AI 탭의 어떤 상태도 다른 둘의 입력이 아니다.
- 컴포넌트 쪽에서도 AI 조회 실패가 상세 화면을 막지 않는다. `ai_provider_status` ·
  `list_ai_notes` · `ai_note_status`는 레코드를 읽는 effect와 **갈라진 effect**에서 부르며,
  실패는 AI 탭 안의 알림(`aiNoteTrouble`) 하나가 된다.

### AC6 — 실패가 §13의 세 질문에 답하고 재시도 가능 · provenance 표시

- 세 질문: `headline`+`failure`(무엇이) · `preservedNotice`(원본은 안전한가) ·
  `retry`(다시 시도할 수 있는가). 테스트가 셋을 함께 판정한다.
- **재시도 수단은 언제나 있다.** `Failure.retryable === false`인 갈래(모델 없음 등)에서도
  버튼이 남고, 무엇을 먼저 해야 하는지는 `resolution`이 말한다 — 갈래별 문장을 테스트가
  네 종류(`modelUnavailable` · `responseUnusable` · `inputTooLarge` · `unreachable`) 고정한다.
- 이유를 모를 때(저장된 상태만 `failed`) 지어내지 않고 `cause: 'unknown'`과 그 사실을 말한다.
- 실패해도 이미 있던 노트는 `kept`에 그대로 남는다 (INV-2 · INV-3).
- provenance: `NoteProvenance`가 §7.3의 다섯 값(provider · model · promptVersion ·
  generatedAt · **transcriptId**)을 전부 들고 있고 `label` 한 줄로도 나간다. 컴포넌트는
  `.note__provenance`에 그 줄을 그린다. `generatedAt`은 backend가 준 ISO 텍스트 그대로다 —
  시각 표기 규칙을 화면에 두 벌로 만들지 않는다 (`tests/screen-boundary.test.ts`와 같은 이유).

### AC7 — 순수 모듈 · 단위 테스트 · 컴포넌트는 그리기만

위의 "형태" 절과 `src/screens/aiNoteView.test.ts`. 테스트는 Ollama도 네트워크도 DOM도 쓰지
않으며 `vitest run` 하나로 끝난다 (§18 · `phase-prompt/04` 요구 16).

## 하지 않은 것

- Settings 화면의 provider 설정 UI — 이 Task의 범위가 아니다 (TASK-040 이후).
- 노트 이력 UI. 데이터는 이력을 지탱하지만 화면은 `(transcriptId, mode)`별 **가장 최근**
  노트를 보여준다 — ADR-0008 §9.2가 정한 그대로이며, 이력 화면은 이 Phase의 요구가 아니다.
- 실제 Ollama 호출. 이 Run은 네트워크를 쓰지 않았고 실제 추론 결과를 만들어 내지 않았다.
