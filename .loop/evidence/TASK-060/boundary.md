# TASK-060 — 경계 확인 (AC-4 · AC-5 · AC-6)

이 Run이 실제로 실행해서 얻은 값만 적는다. 전체 diff는 `diff-ai-section.patch`에 있다.

## AC-4 — `src-tauri/src/ai/`가 삭제·축소되지 않았다

```text
$ git status --short -- src-tauri/src/ai/
(출력 없음)

$ git diff --stat -- src-tauri/src/ai/
(출력 없음)
```

즉 Phase 4의 provider 추상화와 Ollama adapter는 **이 Phase 전체에서 한 줄도 바뀌지 않았다.**
파일 목록도 그대로다:

```text
src-tauri/src/ai/mod.rs        src-tauri/src/ai/ollama/http.rs
src-tauri/src/ai/note.rs       src-tauri/src/ai/ollama/mod.rs
src-tauri/src/ai/prompt.rs     src-tauri/src/ai/ollama/network.rs
src-tauri/src/ai/provider.rs   src-tauri/src/ai/ollama/provider.rs
src-tauri/src/ai/run.rs        src-tauri/src/ai/ollama/testing.rs
src-tauri/src/ai/testing.rs    src-tauri/src/ai/ollama/wire.rs
```

`src-tauri/tests/ollama_adapter.rs`도 그대로 있고 통과한다 (`gates.md` AC-3).
작업 트리에 남아 있는 다른 `src-tauri/**` 수정(`commands/` · `export/` · `lib.rs`)은
이 Phase의 앞선 Task(TASK-056~059)의 것이며 `ai/` 아래에는 하나도 없다.

## AC-5 — AI 구역이 두 부분이고, Ollama가 선택적·고급 위치에 있다

`SettingsScreen.tsx`의 AI `<section>` 구조 (구현: 449–580행):

```text
<h2>{AI_SECTION_TITLE}</h2>                     "AI"  — 벤더 이름이 아니다
  {AI_IS_OPTIONAL_TEXT}                         구역 전체가 선택이라는 사실이 맨 앞
  {AI_WITHOUT_A_PROVIDER_TEXT}                  provider 없이도 쓸 길이 있다 (벤더 부르지 않음)

  <section class="group__part">                 ── 첫 부분
    <h3>{CONNECTED_PROVIDER_TITLE}</h3>         "Connected provider"
    provider 고르기 · 전송 경계 · 주소 · 연결 확인 · 모델 선택   ← Phase 4 그대로 (MH-8)

  {localProviderSetups(form.aiProvider).map(…)} ── 뒷 부분 (가장 뒤)
    <h3>{setup.title}</h3>                      "Ollama — runs on this device · optional"
    {setup.standing}   Optional and advanced…
    {setup.text}       You install and run it on this machine yourself…
    {setup.statusText} 고르지 않음 = "Nothing in the app is waiting for it."
    {setup.howToTurnOn} "To use it: …" — 켜라는 요구가 아니라 방법
```

- 배치: Ollama 부분은 `Connected provider` **뒤에** 있고 구역의 마지막이다.
- 언어: 고르지 않은 상태에 오류 어휘(`error/failed/must/required/missing`)를 쓰지 않는다 —
  `aiProviderSettings.test.ts`의 "고르지 않은 상태가 결함이 아니라 정상으로 적힌다 (INV-8)"가
  이것을 정규식으로 판정한다.
- 편집 수단 없음: 뒷 부분에는 `<input>` · `<select>` · `<button>`이 하나도 없다. 고르기 ·
  확인 · 모델 선택은 전부 앞 부분에 그대로 남아 있다 (MH-8).

**새 cloud provider가 추가되지 않았다:**

```text
$ grep -rin "ollama|openai|gemini|anthropic|codex" \
    src/screens/SettingsScreen.tsx src/ipc/types.ts src/ipc/commands.ts
src/ipc/commands.ts:360:  * … Ollama를    ← 이 Task와 무관한 기존 주석 한 줄
```

`SettingsScreen.tsx`에는 벤더 이름 리터럴이 **하나도 없다** — 이름과 로컬 표시는 전부
`SELECTABLE_AI_PROVIDERS`에서 온다 (INV-9). 그 목록은 이 Run에서 바뀌지 않았고 항목은
`{ id: 'ollama', name: 'Ollama', locality: 'local' }` 하나뿐이다
(`diff-ai-section.patch`에 목록을 건드린 hunk가 없다).

## AC-6 — 표현 규칙이 순수 모듈에 있고, 경계에 벤더 개념이 없다

이 Run이 추가한 표현 규칙은 전부 `src/screens/aiProviderSettings.ts`에 있다:

```text
AI_SECTION_TITLE · AI_IS_OPTIONAL_TEXT · AI_WITHOUT_A_PROVIDER_TEXT
CONNECTED_PROVIDER_TITLE · CONNECTED_PROVIDER_TEXT
OPTIONAL_TITLE_SUFFIX · SELF_HOSTED_STANDING_TEXT · SELF_HOSTED_PROVIDER_TEXT
HOW_TO_TURN_ON_A_LOCAL_PROVIDER · LOCAL_PROVIDER_{CHOSEN,NOT_CHOSEN}_TEXT
interface LocalProviderSetup · function localProviderSetups(chosen)
```

`SettingsScreen.tsx`는 이 값들을 **그리기만 한다** — 새로 만든 문자열도 조건 분기도 없다.
`localProviderSetups`는 `SELECTABLE_AI_PROVIDERS`에서 `locality === 'local'`인 것만
걸러 오므로 provider를 새로 만들 수 없다.

core/domain · command payload · frontend 타입은 **이 Run에서 열지 않았다.** `src/ipc/types.ts`에
벤더 이름이 없고, `tests/ipc-boundary.test.ts`의 벤더 부재 검사가 그대로 통과한다:

```text
tests/ipc-boundary.test.ts:182  /…|ollama|llama|openai|anthropic|claude|gemini|…/i
tests/ipc-boundary.test.ts:300  const VENDORS = /ollama|llama|openai|gpt-|anthropic|claude|…/i
```

`aiProviderSettings.test.ts`에도 같은 성질의 검사를 새로 두었다 — `localProviderSetups`가
만드는 다섯 문장 어디에도 다른 벤더가 나타나지 않는다는 것을 정규식으로 판정한다 (MH-6).
