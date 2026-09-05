# TASK-059 — AI Note 탭의 두 줄 위계 (Manual AI Handoff)

## 무엇을 바꿨나

```text
새 파일  src/screens/aiHandoffView.ts        순수 판정 모듈 (DOM · clipboard · 파일시스템 없음)
새 파일  src/screens/aiHandoffView.test.ts   위 모듈의 vitest (23 it)
수정     src/screens/RecordingDetailScreen.tsx  그리기만 한다 — 판정은 전부 순수 모듈에서 온다
수정     src/App.css                          두 줄과 세 자리의 여백/제목 클래스 6개
```

건드리지 않은 것: `src/screens/aiNoteView.ts` · 탭 구성 `['AI Note','Transcript','Recording']` ·
`share` 섹션(Export Markdown · Send to Notion) · `src/screens/copyView.ts` ·
`src/platform/clipboard.ts` · Rust 쪽 전부.

## 순수 모듈이 내는 값

```text
manualHandoff(input)      아래 줄 — Copy AI Prompt · Copy Transcript · Export for AI
                          입력: { recording, mode, copy, aiExport }  ← provider를 담을 자리가 없다
aiNoteTabLayout(tab,manual)  두 줄의 위계 — { modes, modeSelectable, automatic, orText, manual }
```

Export for AI의 여섯 갈래(loading · nothingToExport · notAsked · exporting · done · failed)와
복사의 네 갈래는 전부 값이며, 실패에는 §13의 세 답(무엇이 실패했는가 · 원본은 안전한가 ·
다시 시도할 수 있는가)과 재시도 동작이 들어 있다.

## Acceptance Criteria 대응

```text
AC-1 build   gate-build.log   npm run build  exit=0
AC-2 lint    gate-lint.log    npm run lint   exit=0 (eslint + cargo clippy -D warnings)
AC-3 test    gate-test.log    npm run test   exit=0 — vitest 24 files / 442 tests,
                              cargo test 전부 통과. 기존 aiNoteView · coreWithoutAi 테스트가
                              그대로 통과한다 (aiNoteView.ts는 이 Task에서 한 줄도 바뀌지 않았다).
AC-4         aiHandoffView.test.ts
             · '아래 줄의 입력에 AI provider를 담을 자리가 없다' (Object.keys 고정)
             · 'provider를 고르지 않은 그 순간에도 세 자리가 전부 눌린다' (steps[*].usable)
             · 'provider 상태가 무엇이든 아래 줄의 값이 글자 하나 달라지지 않는다' (깊은 비교)
             · '고른 mode 셋 전부에서 세 자리가 그대로 쓸 수 있다'
AC-5         aiHandoffView.test.ts
             · '위는 자동으로 만들기, 아래는 내 AI로 하기이며 그 사이에 접속사가 있다'
             · 'provider 부재가 오류로도 설정 요구로도 그려지지 않는다'
               (body.kind === 'disabled' · failure 없음 · optionalNotice에 must/required 없음)
             · 'mode 선택이 provider가 없다는 이유로 잠기지 않는다'
             화면: AutomaticNote / ManualHandoff 두 <section className="note__row">
AC-6         diff-changed-files.patch — 탭 배열과 <section className="share">가 그대로다.
             컴포넌트에는 조건 분기와 문자열이 없고 값(body.kind · action · notice)만 그린다.
             clipboard 호출은 여전히 src/platform/clipboard.ts 하나이며
             tests/screen-boundary.test.ts의 기존 검사가 그대로 통과한다.
```

## 남긴 판단

- `optionalNotice`("This part is optional…")는 **`disabled`일 때만** 낸다. 아직 읽지 못했거나
  전사가 없는 상태에서는 아래 줄도 같은 이유로 기다리므로, 그때 "건너뛰고 아래를 쓰라"고
  말하면 사실이 아닌 것을 말하게 된다.
- `modeSelectable`은 `tab.modeSelectable || (생성 중이 아니고 아래 줄이 쓸 수 있을 때)`다.
  provider가 없어도 mode를 고를 수 있어야 프롬프트와 AI-ready 문서를 mode별로 만들 수 있다.
- 세 동작은 P3의 command(`get_ai_prompt` · `get_transcript_text` · `export_ai_request`)와
  P4의 clipboard 경계(`copyText`)를 통해서만 수행한다. 순수 모듈은 둘 다 부르지 않는다.
