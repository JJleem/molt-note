# TASK-052 · P10-AC5 · P10-AC6 — token은 화면에 남지 않고, 테스트는 실물에 닿지 않는다

## P10-AC5 — token이 화면 상태·브라우저 저장소에 남지 않는다 (INV-7)

### 입력란이 uncontrolled다

`src/screens/SettingsScreen.tsx:556-563`

```tsx
<input id="notion-token" type="password" className="field__input"
       placeholder={TOKEN_INPUT_PLACEHOLDER} autoComplete="off" ref={tokenInput} />
```

`value`도 `defaultValue`도 없다. 값은 `useRef`가 가리키는 DOM 노드에만 있고 **React 상태에
들어가지 않는다**(`:162`). 저장은 값을 command로 넘긴 **직후 응답을 기다리지 않고** 입력란을
비운다 (`:282-287`):

```ts
const typed = input.value;
input.value = '';
setTokenBusy('save'); …
saveNotionToken(typed).then(…)
```

실패해도 그 값을 붙들지 않는다 — 다시 쓰려고 들고 있으면 그것이 곧 화면에 남은 secret이
되기 때문이다. 실패 경로(`notionTokenTrouble`)에는 token이 지나갈 자리 자체가 없다.

### 되읽어 채우지 않는다 — 채울 방법이 없다

저장된 token을 돌려주는 command가 없다. `src/ipc/commands.ts`의 Notion token 표면은
`saveNotionToken(token)`과 `deleteNotionToken()` **둘뿐이고**, 둘 다 값이 아니라
`NotionTokenStatus`(`{ stored: boolean }`)를 돌려준다. 저장 여부를 따로 물어보는 조회
경로조차 없다 — 화면은 저장·삭제가 답한 값과 연결 확인이 함께 들고 온 `tokenStored`로만
그 사실을 안다.

화면이 token에 대해 아는 전부는 `NotionTokenState = 'unknown' | 'stored' | 'notStored'`이며
(`notionSettings.ts:57`), **어느 변형에도 값이 없다.** 처음이 `'unknown'`인 것도 의도적이다 —
화면을 열자마자 자격증명 저장소를 뒤지지 않으므로, 없다고 적으면 모르는 것을 아는 것처럼
말하게 된다. 저장돼 있을 때는 그 값을 다시 볼 수 없다는 사실을 숨기지 않고 적는다
(`TOKEN_STORED_RESOLUTION`).

### 브라우저 저장소에 쓰지 않는다

`tests/screen-boundary.test.ts`의 `token은 화면에 남지 않는다 (INV-7)` 네 검사가 원문으로
못박는다. 파일 하나가 새로 생기는 것만으로 조용히 깨질 수 있는 규칙이라 원문 검사다.

1. `src/ 아래에 브라우저 저장소를 쓰는 경로가 없다` — `frontendSources` **전체**에 대해
   `localStorage.` · `sessionStorage.` · `indexedDB.` · `document.cookie`의 실사용 모양이
   없음을 확인한다.
2. `token 입력란이 화면 상태를 갖지 않고, 저장된 값을 되읽어 채우지 않는다` —
   `SettingsScreen.tsx` 원문에서 token 입력란 구간을 잘라 `value=` · `defaultValue=`가 없고
   `ref={tokenInput}`이 있으며, 넘긴 뒤 `input.value = ''`로 비운다는 것까지 본다.
3. `token 값을 돌려주는 조회 경로가 없다` — `get_notion_token`류 이름이 없고, **token을 인자로
   받는 함수가 `saveNotionToken` 하나뿐**임을 정규식 추출로 확인한다.
4. `순수 view 모듈이 token 값을 들고 있지 않다` — `notionSettings.ts`에 `token: string` 필드도
   `invoke(` 호출도 없다.

`notionSettings.test.ts`의 `어떤 화면 상태에도 token 값이 실릴 자리가 없다`(`:238`)와
`저장 · 삭제가 실패해도 입력한 값을 되돌려 적지 않는다`(`:258`)가 값 수준에서 같은 것을 본다.

### 저장된 token이 없는 것은 오류가 아니다 (INV-8)

`TOKEN_NOT_STORED_TEXT`는 경고가 아니라 사실 한 줄(`No integration token is saved, so nothing
is sent to Notion.`)과 무엇을 하면 되는지 한 줄이다. 화면에서도 `FailureNotice`가 아니라
`hint`로 그려진다(`SettingsScreen.tsx:549-550`). 연결 확인도 이때 요청을 보내지 않고
`noToken`으로 답한다.

## P10-AC6 — 테스트가 실물에 닿지 않는다

`src/screens/notionSettings.test.ts`가 import하는 것은 `vitest` · 타입 둘 · 검사 대상 모듈뿐이다
(`:9-38`). 네트워크 client도, Tauri `invoke`도, 자격증명 저장소도 없다.

- **실제 Notion 워크스페이스** — 이 테스트는 HTTP를 부르지 않는다. 연결 확인의 결과는
  `connection()` 헬퍼가 만든 **평범한 값**(`NotionConnection`)으로 들어오고, 검사 대상
  함수는 그 값을 화면 상태로 옮기는 순수 함수다. 모듈 자체에 `fetch`도 `invoke`도 없다
  (위 원문 검사 4번이 이것을 계속 지킨다).
- **실제 OS 자격증명 저장소** — 이 테스트는 token을 저장하지도 읽지도 않는다.
  `tokenStatus(stored)`는 `{ stored }` 하나를 만드는 헬퍼다. 파일에 등장하는
  `NOT_A_REAL_TOKEN`은 **입력에조차 들어가지 않으며**, 화면 상태에 값이 새어 나갈 자리가
  있는지 확인할 때 "찾을 것"으로만 쓰인다.
- `tests/screen-boundary.test.ts`는 `node:fs`로 저장소의 원문을 읽을 뿐이다.

실행 방법도 같은 것을 말한다 — `npm run test:web`은 `vitest run`이며 DOM 환경도 앱 실행도
없이 끝난다 (self-check 로그: `Test Files 21 passed (21)`, `Duration 416ms`).
