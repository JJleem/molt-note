# 이 Task가 바꾼 파일

```text
수정  src/App.css                            토큰 적용 · 화면별 절 전부 · 새 규칙 몇
수정  src/App.tsx                            Back 버튼을 ghost 버튼 규칙 위로
수정  src/screens/FailureNotice.tsx          Try Again 버튼을 secondary 규칙 위로
수정  src/screens/RecordingsScreen.tsx       빈 상태 · 로딩 · 목록 레이블
수정  src/screens/RecordingScreen.tsx        주 조작 넷 · 제목 입력 · 조작 group
수정  src/screens/RecordingDetailScreen.tsx  빈 상태 · 로딩 · 버튼 · tabpanel 연결
수정  src/screens/SettingsScreen.tsx         빈 상태 · 로딩 · 버튼 · 입력 primitive
신규  src/screens/EmptyState.tsx             빈 상태 한 모양
신규  src/screens/Loading.tsx                로딩 한 모양
```

`.loop/evidence/TASK-062/screens.diff`에 수정 일곱의 diff 원문과, 그 끝에
`git status --porcelain -- src`의 출력이 있다.

## diff를 읽을 때 주의할 것

`git diff`의 기준은 HEAD이고, **작업 트리에는 이 Task 이전의 미커밋 변경이 함께 있다**
(TASK-055 ~ TASK-061). 그래서 `RecordingDetailScreen.tsx` · `SettingsScreen.tsx` ·
`ipc/commands.ts` · `ipc/types.ts` · `aiProviderSettings.ts`의 diff에는 앞선 Task들의 변경이
섞여 있다. **이 Task가 만든 것은 위 아홉 줄이 전부이며, 그중 순수 view 모듈은 하나도 없다.**

`src/ipc/*` · `src/screens/*View.ts` · `src/screens/aiProviderSettings.ts` ·
`src/navigation/*` · `src/platform/*` · `src-tauri/**` · `tests/**`는 이 Task가 열지 않았다.
