# TASK-040 — 바꾼 파일

| 파일 | 새로 만듦 | 무엇을 했나 |
| --- | --- | --- |
| `src/screens/aiProviderSettings.ts` | ✔ | AI provider 구역의 **판단 로직 전부**. provider 선택지 · locality · 전송 경계 문구 · 연결 확인 결과 일곱 갈래 · 모델 선택지와 안내 · "확인은 저장된 설정에게 물어본다" 판정. React·DOM·Tauri를 알지 않는다 |
| `src/screens/aiProviderSettings.test.ts` | ✔ | 그 모듈의 단위 테스트 28건. **실제 Ollama도 네트워크도 DOM도 쓰지 않는다** |
| `src/screens/SettingsScreen.tsx` | | AI Provider 그룹을 자리표시(`Not available yet.`)에서 실제 구역으로 바꿨다. **그리기만 한다** — 판단은 전부 위 모듈에서 온다 |
| `src/screens/settingsView.ts` | | `SettingsForm`의 AI 세 값을 `string \| null`에서 `string`으로. 입력 위젯은 `null`을 담을 수 없다 (`recordingsDirectory`·`defaultMicrophone`과 같은 규칙). `toForm`/`toSettings`가 그 변환을 갖는다 |
| `src/screens/settingsView.test.ts` | | 위 표현 변경에 맞춰 기대값 세 곳을 `null` → `''`로. 왕복 테스트는 그대로 두었고 여전히 `null` 왕복을 검사한다 |

Rust는 한 줄도 바꾸지 않았다. `ai_provider_status` command와 `SettingsPayload`의 AI 세 열은
이미 있었고 (TASK-038까지), 이 Task는 그 위에 화면을 얹었다.

## 화면이 부르는 것

```text
SettingsScreen
   ├─ getSettings / updateSettings   (기존)
   └─ aiProviderStatus               ← 이 Task가 화면에서 처음 부른다
```

`src/ipc/commands.ts`는 손대지 않았다 — `aiProviderStatus`가 이미 그 자리에 있었다.
webview에서 AI 서버로 나가는 HTTP 경로는 여전히 없다 (ADR-0008 §5).

## 계획에서 벗어난 판단 하나 — 기본 주소를 화면에 옮겨 적지 않았다

Task 서술은 "연결 대상 host/port 편집(**기본값은 ADR-0008이 재확인한 값**)"을 요구한다.
동작은 그대로 만들었다 — 주소 입력란을 비워 두면 backend가 그 기본값으로 연결한다
(`Settings::ai_base_url_or_default`).

**다만 그 주소 문자열 자체를 frontend에 적지 않았다.** `src-tauri/src/domain/settings.rs`가
`DEFAULT_AI_BASE_URL`을 선언하면서 "이 값은 저장소에도, adapter에도, **화면에도** 복사되지
않는다 — 여기 한 곳에만 있다. 같은 주소가 여러 곳에 적히면 한 곳을 고쳤을 때 나머지가
조용히 달라진다"고 못 박아 두었고, `tests/ipc-boundary.test.ts`와 Rust
`the_default_address_lives_in_the_settings_and_not_in_the_adapter`가 같은 방향을 이미
강제하고 있다.

그래서 화면은 값 대신 **사실**을 말한다:

```text
placeholder  "Leave empty to use the built-in address"
hint         "Where the provider is listening, as host and port.
              Leave it empty and the app connects to its built-in address for that provider."
```

이 판단이 틀렸다면 고칠 곳은 한 군데다 — 그 주소를 backend가 payload로 실어 보내게 하고
화면이 받아 보여 주면 된다. 그것은 Rust 경계 변경이므로 이 Task의 범위 밖으로 두었다.
테스트가 이 규칙을 지키고 있다: `기본 주소가 화면 쪽에 옮겨 적혀 있지 않다`.

## 함께 만든 사실 하나 — 확인은 **저장된** 설정에게 물어본다

`ai_provider_status`는 인자를 받지 않고 저장소의 설정을 읽는다
(`src-tauri/src/commands/mod.rs:901`). 그래서 편집 중인 주소로 확인할 수 없다. 그것을 숨기지
않고 화면에 적는다:

- 언제나 보이는 한 줄: `AI_CHECK_USES_SAVED_SETTINGS`
- 마지막 저장 이후 AI 세 값이 바뀌었을 때만 붙는 한 줄 (`aiSettingsChanged`)

`aiSettingsChanged(form, null)`은 `false`다 — 무엇이 저장돼 있는지 모르면 바뀌었다고
말하지 않는다.
