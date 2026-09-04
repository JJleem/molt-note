# TASK-052 · P10-AC4 — connection test가 화면에서 실행되고 결과가 갈래마다 다르게 보인다

## 실행 자리

`src/screens/SettingsScreen.tsx:609-616` — `Check the Notion connection` 버튼.
누르면 `checkNotion()`(`:325-334`)이 `checkNotionConnection()`(TASK-050의 command)을 부른다.

- 화면을 열자마자 나가지 않는다. 초기 상태는 `notChecked`(`:138-141`)이며 사용자가 눌러야
  요청이 나간다 — AI provider 확인과 같은 규칙이다.
- 진행 중에는 버튼이 `Checking…`으로 바뀌고 비활성화된다(`:612-615`).
- 확인이 답한 저장 여부로 token 상태도 맞춘다(`:330`) — 화면이 자기가 누른 버튼으로
  짐작하지 않는다.
- 확인은 **저장된** token·destination에게 물어본다. 그 사실이 `NOTION_CHECK_USES_SAVED_SETTINGS`로
  적혀 있고(`:617`), 마지막 저장 뒤 destination을 고쳤다면 "이 결과는 저장된 값에 대한
  답"이라고 먼저 말한다(`:618-623`).

## 다섯 결과가 서로 다른 값이고 서로 다른 문장이다

규칙은 전부 `src/screens/notionSettings.ts`의 `checkedNotionConnection` ·
`notionCheckCause`에 있고, 화면(`:625-640`)은 그 값을 그리기만 한다.

| 결과 | 어디서 오는가 | 화면이 하는 말 |
| --- | --- | --- |
| **성공** | `state: 'connected'` | 어느 워크스페이스인지 — `Notion answered, and the saved token works in the workspace “…”.` Notion이 이름을 말하지 않으면 이름을 **지어내지 않고** 이름 없는 문장을 쓴다 |
| **인증 실패** | `FailureKind.notionAuthFailed` → `cause: 'auth'` | `Notion did not accept the saved integration token.` + `Paste a working integration token above and save it, then check again.` |
| **권한 없는 destination** | `notionDestinationUnavailable` → `cause: 'destination'` | `Notion answered, but it could not use the parent page that is saved.` + `Open that page in Notion, share it with your integration, or set another parent page below…` |
| **네트워크 없음** | `notionRequestFailed` → `cause: 'offline'` | `This app could not reach Notion.` + `Check that this device is online, then check again.` |
| 저장된 token 없음 | `state: 'notConfigured'` | 오류가 아니라 상태 — `…nothing to check and no request was sent.` (INV-8) |
| 확인 요청 자체가 거절됨 | `failedNotionCheck(error)` | Notion에 대한 사실이 아니라는 것을 따로 말한다 (`checkFailed`) |

`rateLimited`와 `other`도 `auth`로 접히지 않고 따로 있다.

실패에는 §13의 세 답(무엇이 실패했는지 · 원본이 안전한지 · 다시 할 수 있는지)이 실려 오며
`FailureNotice`가 그대로 그리고 재시도 수단을 준다(`:632-640`).

## 이것을 판정하는 테스트

`src/screens/notionSettings.test.ts` (self-check의 test Gate에서 실제로 실행됐다):

- `네 결과 — 성공 · 인증 실패 · 권한 없는 destination · 네트워크 없음 — 이 서로 다른 문장이다`
  (`:111`) — 갈래(`cause`)가 다른 것에 더해, **화면에 실제로 보이는 문장 여덟 개가 전부 서로
  다르다**는 것을 `Set`의 크기로 확인한다. 갈래만 나누고 같은 말을 하면 사용자에게는 뭉친
  것과 같기 때문이다.
- `갈래마다 다음에 할 일이 다르다` (`:133`) — 세 resolution이 각각 token · share/parent page ·
  online을 가리킨다.
- `성공이 어느 워크스페이스에 연결됐는지 말한다` (`:81`) / `…이름을 말하지 않으면 이름을
  지어내지 않는다` (`:91`).
- `연결됐어도 보낼 부모 페이지가 없으면 그 사실이 함께 보인다` (`:102`) — 연결된 것과 보낼
  자리가 있는 것은 다른 사실이다.
- `속도 제한과 그 밖의 실패도 인증 실패로 읽히지 않는다` (`:143`).
- `확인 요청 자체가 거절된 것은 Notion에 대한 사실이 아니다` (`:178`).
