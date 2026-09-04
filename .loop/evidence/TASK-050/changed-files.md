# TASK-050 · 무엇을 고쳤는가

| 파일 | 변경 |
| --- | --- |
| `src-tauri/src/commands/payload.rs` | payload 넷 추가 — `NotionSendStatusPayload`(+`From<NotionSendStatus>`) · `NotionSyncPayload`(+`From<NotionSync>`) · `NotionConnectionPayload` · `NotionTokenStatusPayload`. 어느 것에도 token 값을 담는 필드가 없다. |
| `src-tauri/src/commands/notion.rs` | `NotionSender`에 `check_connection` · `save_token` · `delete_token` · `token_status` 추가, 화면이 보낸 확인 문자열을 읽는 `parse_confirmation` 추가. 기존 `start` · `status` · `send_one`은 손대지 않았다. 단위 테스트 다섯 추가. |
| `src-tauri/src/commands/mod.rs` | `Storage::notion_sync`(저장된 전송 기록 읽기) 추가, payload 재수출, **Tauri command 여섯** 추가, 모듈 문서에 표면과 token 방향 기록. |
| `src-tauri/src/lib.rs` | `NotionSender`를 managed state로 등록(`app.manage`), `generate_handler![...]`에 command 여섯 등록. |
| `src/ipc/types.ts` | `NotionSendState` · `NotionSendStatus` · `NotionConfirmation` · `NotionSync` · `NotionConnectionState` · `NotionConnection` · `NotionTokenStatus` 추가. |
| `src/ipc/commands.ts` | 함수 여섯 추가와 타입 재수출. 등록된 command와 1:1이다. |
| `src-tauri/tests/command_boundary.rs` | Notion 표면의 command 경계 테스트 일곱 추가(연결 확인 셋 · token 셋 · 저장된 기록 둘). stub transport와 메모리 자격증명 double만 쓴다. |
| `tests/ipc-boundary.test.ts` | 허용 command 목록에 여섯 추가, `outOfScope`에서 `notion` 제거(대신 'Notion 표면은 여섯뿐이다'와 '전송 기록을 고치거나 지우는 command가 없다'로 선을 지킨다), INV-7 검사 셋과 'frontend가 Notion으로 직접 나가지 않는다' 검사 둘 추가. |

## 손대지 않은 것

- `src-tauri/src/notion/**` — Notion API를 아는 자리다. 이 Task는 그 adapter를 부르기만 했고
  엔드포인트도 double도 늘리지 않았다.
- `src-tauri/src/sync/**` — 전송 순서와 영속화 규칙. 그대로 부른다.
- `src-tauri/src/db/**` · `domain/**` — 스키마도 domain 타입도 바뀌지 않았다. 새 `FailureKind`도
  만들지 않았다(이 표면이 내는 실패는 이미 있는 종류로 전부 도달한다).
- 화면(`src/screens/**`) — 사용자가 누르는 자리는 TASK-051 · TASK-052의 몫이다. 이 Task는
  경계와 계약만 만든다.
