# TASK-052 · 이 Task의 변경 파일

frontend만 바꿨다. `src-tauri/**` · `Cargo.toml` · `package.json` ·
`.loop/**`(evidence 제외)에 이 Task의 변경은 없다.

## 이 Run이 시작될 때의 작업 트리 상태 (숨기지 않고 적는다)

이 Run(`RUN-20260904T065233Z-TASK-052`)이 시작됐을 때 아래 구현이 **이미 작업 트리에
uncommitted 상태로 존재했다.** 파일 timestamp는 15:45, 이 Run의 시작은 15:52다.
`failure_memo`는 비어 있으므로 Runtime이 기록한 이전 Attempt는 없다 — 이전 Worker
invocation이 산출물을 쓴 뒤 Result를 남기지 못하고 끝났을 가능성이 있으나, 이 Worker가
확인할 수 있는 것은 파일이 거기 있었다는 사실까지다.

이 Run이 한 일은 **그 산출물을 Acceptance Criteria에 대해 검증하고 · 그 과정에서 낡아진
주석 하나를 고치고 · Gate 셋을 실제로 돌려 Evidence를 남긴 것**이다. 아래 표의 "이 Run"
열이 그 구분이다.

## 새로 만든 파일

| 파일 | 무엇인가 | 이 Run |
| --- | --- | --- |
| `src/screens/notionSettings.ts` | Settings의 Notion 구역 순수 view 모듈 — token 상태 문구 · destination 문구 · 연결 확인 결과의 갈래(§5-D · §13) | 시작 시 존재 |
| `src/screens/notionSettings.test.ts` | 그 모듈의 테스트 23개. 네 결과 구분 · 워크스페이스 표시 · INV-7 · INV-8 | 시작 시 존재 |

## 고친 파일

| 파일 | 무엇을 | 이 Run |
| --- | --- | --- |
| `src/screens/SettingsScreen.tsx` | `Not available yet.` 자리표시를 실제 Notion 구역으로 대체 — token 입력(uncontrolled `ref`)·저장·삭제, destination 입력, `Check the Notion connection` 버튼과 결과 표시. AI Provider 구역 **바로 다음**에 온다 | 시작 시 존재 |
| `src/screens/settingsView.ts` | `SettingsForm`에 `notionParentPageId` 추가와 `toForm`/`toSettings` 왕복 | 시작 시 존재 |
| `src/screens/settingsView.test.ts` | 왕복 fixture와 '다른 값을 저장할 때 destination이 지워지지 않는다' 테스트 | 주석 한 줄 수정 |
| `src/screens/aiProviderSettings.test.ts` | fixture에 `notionParentPageId: null` | 시작 시 존재 |
| `tests/screen-boundary.test.ts` | INV-7 원문 검사 넷 — 브라우저 저장소 사용 없음 · token 입력란에 `value`/`defaultValue` 없음 · token을 돌려주는 조회 경로 없음 · 순수 모듈에 token 값 없음 | 시작 시 존재 |

### 이 Run이 직접 고친 것

`src/screens/settingsView.test.ts`의 주석 한 줄. "지금 이 값을 편집하는 입력란은 없지만"이
더 이상 사실이 아니다 — 이 Task가 그 입력란을 만들었다. 문장을 사실에 맞췄고 테스트의
동작은 바꾸지 않았다.

## AI provider 구역을 바꾸지 않았다

`git diff src/screens/SettingsScreen.tsx`에서 사라진 줄은 넷뿐이다 —
`import` 두 줄(useRef·command 추가로 바뀐 줄), Notion을 "Phase 5의 일"이라고 적어 둔
낡은 주석, 그리고 `<p className="hint">Not available yet.</p>`.
**AI Provider 섹션(`<h2>AI Provider</h2>` ~ `AI_SETTINGS_UNAFFECTED_NOTICE`)에서 지워지거나
바뀐 줄은 하나도 없다.** `aiProviderSettings.ts`도 손대지 않았다(테스트 fixture 한 줄 제외).

## 하지 않은 것

- `src-tauri/**` — 이 구역이 부르는 command 여섯은 TASK-050이 이미 만들었다. 새 command도
  새 `FailureKind`도 만들지 않았다.
- `src/ipc/**` — 타입도 command 함수도 그대로 쓴다.
- Recording Detail·목록의 Notion 자리 — TASK-051의 것이며 건드리지 않았다.
