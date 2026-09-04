# TASK-050 · 만들어진 표면과 그 계약

## 1. 등록된 command ↔ frontend 함수 ↔ payload (1:1 · P8-AC6)

| Tauri command (`src-tauri/src/lib.rs`) | frontend (`src/ipc/commands.ts`) | 응답 payload | 타입 (`src/ipc/types.ts`) |
| --- | --- | --- | --- |
| `start_notion_sync` | `startNotionSync(recordingId, confirmation?)` | `NotionSendStatusPayload` | `NotionSendStatus` |
| `notion_sync_status` | `notionSyncStatus()` | `NotionSendStatusPayload` | `NotionSendStatus` |
| `get_notion_sync` | `getNotionSync(recordingId)` | `Option<NotionSyncPayload>` | `NotionSync \| null` |
| `check_notion_connection` | `checkNotionConnection()` | `NotionConnectionPayload` | `NotionConnection` |
| `save_notion_token` | `saveNotionToken(token)` | `NotionTokenStatusPayload` | `NotionTokenStatus` |
| `delete_notion_token` | `deleteNotionToken()` | `NotionTokenStatusPayload` | `NotionTokenStatus` |

이 1:1은 주장이 아니라 검사다 — `tests/ipc-boundary.test.ts`가
(a) `generate_handler![...]`에 등록된 이름의 집합과 허용 목록이 **정확히 같은지**,
(b) `commands.ts`가 부르는 이름의 집합이 등록된 집합과 **정확히 같은지**,
(c) `notion`이 들어간 이름이 위 여섯뿐인지를 본다.

## 2. token은 한 방향으로만 지난다 (P8-AC5 · INV-7 · ADR-0009 §10.4)

```text
화면 ──token──▶ save_notion_token ──▶ SecretStore
화면 ◀──있다/없다── save · delete · check_notion_connection
```

- **token을 돌려주는 command가 없다.** 조회하는 이름 자체가 없고, 응답 payload 넷 중 어디에도
  값을 담는 필드가 없다. 있는 것은 사실을 말하는 boolean 둘이다 — `NotionTokenStatusPayload.stored`,
  `NotionConnectionPayload.token_stored`.
- 값이 실제로 읽히는 자리는 전송이 도는 배경 스레드
  (`commands::notion::send_one`)와 연결 확인(`NotionSender::check_connection`) 안의 지역 변수뿐이며,
  둘 다 요청 헤더가 되는 것 말고는 값을 옮기지 않는다.
- 실패에도 섞이지 않는다. 빈 값을 거절하는 실패는 `detail`을 **비워 둔다** — mode를 `detail`에
  적는 `notes::parse_mode`와 갈리는 지점이며, 그 이유는 이 값이 secret이기 때문이다.
- 판정하는 검사:
  - `tests/ipc-boundary.test.ts` — `payload.rs`에 문자열 token 필드가 없다 · `types.ts`에
    `readonly ...token...: string`이 없다 · token을 **돌려주는** 함수 시그니처가 없다.
  - `src-tauri/tests/command_boundary.rs::a_stored_token_is_answered_as_a_fact_and_never_as_a_value`
    — 직렬화된 응답(`{"stored":true}` · 연결 확인 응답 전체)에 token 문자열이 없다.
  - 같은 파일의 연결 확인 테스트 — 네 가지 실패의 `Debug` 출력 어디에도 token이 없다.

## 3. connection test (P8-AC4 · §5-D · §13)

`NotionSender::check_connection`은 저장된 token으로 adapter의 연결 확인 하나를 부른다
(`NotionClient::check_connection` — ADR-0009 §5.1이 정한 호출이며, 그 주소를 아는 자리는
`src/notion` 하나다. 이 경계에는 주소도 헤더 이름도 없다).

```text
token 없음     notConfigured   요청을 보내지 않는다. 정상 상태다 (INV-8)
확인됨         connected
확인하지 못함   failed + §13의 Failure 그대로
```

`destination_configured`는 설정에 부모 페이지를 골랐는지를 **따로** 싣는다 — token이 유효한지와
다른 질문이며, 사용자는 둘 중 무엇이 남았는지 알 수 있어야 한다. 여기서 페이지 식별자를
돌려주지는 않는다.

실패 셋의 구분은 `failure.kind`가 그대로 들고 온다. stub transport로 판정한 네 경우
(`command_boundary.rs::a_connection_test_separates_a_rejected_token_from_a_destination_it_cannot_reach`):

| stub 응답 | state | failure.kind | retryable |
| --- | --- | --- | --- |
| 정상 | `connected` | — | — |
| `401 unauthorized` | `failed` | `notionAuthFailed` | false |
| `403 restricted_resource` | `failed` | `notionDestinationUnavailable` | false |
| `404 object_not_found` | `failed` | `notionDestinationUnavailable` | false |
| 연결되지 않음 | `failed` | `notionRequestFailed` | true |

token을 저장하지 않았을 때는 **요청이 한 건도 나가지 않는다**(stub이 받은 요청 수 0) —
아직 설정하지 않은 상태를 "연결 실패"로 보이지 않기 위해서다.

## 4. 조용히 사라지는 요청이 없다

`start_notion_sync`는 접수 사실(`running`)을 돌려주고, 이미 보내는 중이면 §13의 실패로
**거절한다** — 같은 녹음이어도 마찬가지이며, 어느 녹음이 돌고 있는지가 `detail`에 실린다.
`command_boundary.rs::a_second_send_while_one_is_running_is_refused_instead_of_disappearing`가
전송이 실제로 도는 한가운데를 만들어(자격증명 읽기 자리에서 붙잡는다) 그것을 판정한다.
같은 테스트가 두 가지를 더 고정한다 — 그동안 상태 조회가 즉시 답한다는 것과, token을 저장하지
않은 상태에서의 시작은 **command 실패가 아니라 `failed` 상태값**으로 끝난다는 것이다 (INV-8).

## 5. 이 경계를 넘지 않은 것

- **SQL · 저장소 지식** — `get_notion_sync`는 `Storage::notion_sync` 하나를 부르고, 질의는
  `db::store` 안에 있다. `tests/ipc-boundary.test.ts`가 `src/` 아래에 SQL이 없는지 계속 본다.
- **Notion API 지식** — 주소 · 헤더 이름 · API 버전 · 오류 코드는 `src-tauri/src/notion` 밖으로
  나가지 않는다 (`src-tauri/tests/notion_adapter.rs`가 소스에서 확인하며, 그 검사는 이 Task가
  더한 파일들에도 그대로 적용된다). frontend 쪽도 같은 검사를 새로 뒀다 —
  `src/` 아래에 `fetch(` · `XMLHttpRequest` · `WebSocket` · `EventSource`가 없고,
  Notion 주소 · `Notion-Version` · `Bearer` · `/v1/`도 없다. **화면은 임의의 요청을 만들 수단을
  갖지 않는다.**
- **새 쓰기 이름** — 지우는 command 하나(`delete_notion_token`)가 만지는 것은 이 앱이 자격증명
  저장소에 넣은 항목뿐이다. 녹음 · 전사 · 노트 · 이미 만들어진 Notion 페이지 · 저장된 전송
  기록을 고치거나 지우는 이름은 여전히 없다 (INV-3 · INV-4).
