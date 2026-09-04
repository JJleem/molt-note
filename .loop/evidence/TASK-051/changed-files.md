# TASK-051 · 이 Task가 바꾼 파일

frontend만 바꿨다. `src-tauri/**` · `Cargo.toml` · `package.json` · `.loop/**`(evidence 제외)에
이 Task의 변경은 없다.

## 새로 만든 파일

| 파일 | 무엇인가 |
| --- | --- |
| `src/screens/exportView.ts` | Export Markdown 자리의 순수 view 모듈 (§11 · §13 · P-3) |
| `src/screens/exportView.test.ts` | 그 모듈의 테스트. **P-3의 세 조건이 여기 있다** |
| `src/screens/notionSyncView.ts` | Send to Notion 자리의 순수 view 모듈 (§10 · §13 · ADR-0009 §8) |
| `src/screens/notionSyncView.test.ts` | 그 모듈의 테스트. 상태 표시 · 전송 대상 공개 · 중복 sync 예측 · 실패 |

## 고친 파일

| 파일 | 무엇을 |
| --- | --- |
| `src/screens/RecordingDetailScreen.tsx` | `Export Markdown` · `Send to Notion` 자리를 그린다. 상태 조회(`get_notion_sync` · `notion_sync_status`)와 되풀이 조회, 두 동작의 호출부. **그리는 일만 있다** |
| `src/screens/recordingsView.ts` | 상태 표시를 만드는 함수를 `statusBadge`로 내보낸다 — 목록과 상세가 **같은 규칙**을 쓰게 하기 위해서다. 목록의 동작은 그대로다 |
| `tests/screen-boundary.test.ts` | `exportView.ts` 원문에 AI provider가 등장하지 않는다는 검사 하나 (INV-8). 원문을 읽는 검사들과 같은 자리다 |
| `src/App.css` | `.share*` 클래스. 기존 `.note*` · `.detail*`와 같은 идиом |

## 하지 않은 것

- Settings의 Notion 구역(token 입력 · connection test)은 **TASK-052의 일이다.** 이 Task는
  `SettingsScreen.tsx`를 건드리지 않았다.
- `src/ipc/**`의 command·타입은 TASK-050이 이미 만든 것을 **그대로 쓴다.** 새 command도, 새
  타입도 더하지 않았다.
- 실제 Notion 전송·export 실행 경로(Rust)는 TASK-046~049가 만든 것이며 이 Task가 고치지 않았다.
