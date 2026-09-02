# TASK-008 변경 내역과 Acceptance Criteria 대응

## 변경 파일

새로 만든 파일

| 파일 | 무엇 |
| --- | --- |
| `src/screens/recordingsView.ts` | `list_recordings` 응답 → Recordings 화면 상태 (loading · empty · list · failed). DOM/React/Tauri 없음 |
| `src/screens/recordingsView.test.ts` | 위 변환의 vitest 테스트 |
| `src/screens/settingsView.ts` | `get_settings` / `update_settings` 응답 → Settings 화면 상태 (loading · ready · failed). DOM/React/Tauri 없음 |
| `src/screens/settingsView.test.ts` | 위 변환의 vitest 테스트 |
| `src/screens/failureView.ts` | `Failure` → §13의 세 가지 답을 담은 화면 표현 |
| `src/screens/failureView.test.ts` | 위 변환의 vitest 테스트 |
| `src/screens/FailureNotice.tsx` | 실패를 그리는 공용 컴포넌트 (두 화면이 같은 모양으로 보여준다) |
| `tests/screen-boundary.test.ts` | 소스 전체 규칙 2개: TS에 길이 포맷 계산이 없다 · `console`로만 끝나는 경로가 없다 |

고친 파일

| 파일 | 무엇 |
| --- | --- |
| `src/screens/RecordingsScreen.tsx` | 하드코딩된 빈 배열을 지우고 `listRecordings()` 결과를 렌더링. 실패 시 `FailureNotice` + 다시 시도 |
| `src/screens/SettingsScreen.tsx` | `getSettings()`로 읽고 `updateSettings()`로 저장. 저장소가 돌려준 값으로 폼을 다시 채운다 |
| `src/App.css` | 목록 상태 3종과 실패 표시용 스타일 (hairline · 여백만, §19) |

Rust(`src-tauri/**`)와 command 표면은 건드리지 않았다. 이 Task는 P6가 이미 만든
여섯 개 command에 화면을 붙이는 일이다.

## AC 대응

### AC1 · AC2 · AC3 — build · lint · test

`self-check.md` 참고. 세 Gate 모두 exit=0.

### AC4 — 목록 렌더링과 empty state

- 목록의 출처: `RecordingsScreen.tsx`의 `useEffect`가 `listRecordings()`(= `list_recordings`
  command)를 부르고, 그 결과를 `loadedRecordings()`에 넣는다. 화면 어디에도 더미 배열이 없다.
  (이전 구현에 있던 `const recordings: readonly RecordingListItem[] = []`는 삭제했다.)
- 항목이 보여주는 것: 제목 · 날짜(`recordedAtLabel`) · 길이(`durationLabel`) ·
  transcription / AI / Notion 세 상태. 세 상태는 배열이 아니라 길이 3의 tuple 타입이라
  하나가 빠지면 컴파일에서 드러난다.
- 빈 목록: `loadedRecordings([])` → `{ kind: 'empty' }` → 화면은 empty state를 그린다.
  실패와 섞이지 않는 별개의 상태다.
- 테스트: `src/screens/recordingsView.test.ts`
  - `레코드가 하나도 없으면 실패가 아니라 empty state다`
  - `저장소에서 온 값이 그대로 목록 항목이 된다`
  - `항목마다 transcription · AI · Notion 세 상태를 보여준다`
  - `아직 시도하지 않은 상태가 실패처럼 읽히지 않는다`
  - `저장소 순서를 화면이 다시 정렬하지 않는다`

### AC5 — 실패가 사용자에게 보인다

- `listRecordings()` / `getSettings()` / `updateSettings()`의 거절은 전부
  `failedRecordings` · `failedSettings` · `failedSave`를 거쳐 **화면 상태**가 된다.
  화면은 그 상태에서 `FailureNotice`를 그린다: 무엇이 실패했는가(`message`) ·
  원본 데이터는 안전한가(`dataSafetyText`) · 다시 시도할 수 있는가(`retryText` +
  retryable일 때만 나오는 `Try Again` 버튼).
- 저장소 초기화 실패는 Rust에서 모든 command 응답으로 돌아오므로(`commands/mod.rs`의
  `StorageState::Unavailable`) 이 경로를 그대로 탄다.
- `src/` 어디에도 `console.*` 호출이 없다는 것을 `tests/screen-boundary.test.ts`가 검사한다.
- 테스트: `src/screens/failureView.test.ts` 전체,
  `recordingsView.test.ts`의 `실패 상태` describe(`실패는 empty state로 둔갑하지 않는다` 포함),
  `settingsView.test.ts`의 `설정을 읽지 못하면 화면이 실패 상태가 된다` ·
  `저장이 실패하면 화면에 실패가 남고 입력한 값은 버려지지 않는다` ·
  `다시 저장을 시작하면 지난 실패는 지워진다`.

### AC6 — 길이 포맷은 Rust에서 온 값

- `RecordingListItem.durationLabel = recording.durationLabel` — Rust
  (`src-tauri/src/domain/duration.rs::format_duration_ms`)가 만든 값을 그대로 쓴다.
  `durationMs`는 화면 코드가 읽지 않는다.
- `tests/screen-boundary.test.ts`의 `src/ 아래에 초를 mm:ss로 바꾸는 계산이 없다`가
  `% 60` · `/ 60` · `/ 1000` · `padStart(2` 같은 계산의 모양을 src/ 전체에서 찾아 없음을 확인한다.
- `recordingsView.test.ts`의 `길이는 Rust가 보낸 문자열이며 화면이 다시 계산하지 않는다`는
  `durationMs`와 `durationLabel`이 어긋난 입력을 주고 화면이 label을 쓰는지 본다.

## 범위 밖으로 나가지 않은 것

- 녹음 · 전사 · AI · Notion 기능은 구현하지 않았다. Settings의 Transcription /
  AI Provider / Notion은 그대로 자리만 있다.
- secret 입력란을 만들지 않았다 (INV-7).
- Rust · command 표면 · 스키마를 바꾸지 않았다.
- git commit을 하지 않았다.
