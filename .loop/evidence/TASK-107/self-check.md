# Self-check results for TASK-107

## Gates

- **build**: PASS (exit=0, 1.8s)
- **lint**: PASS (exit=0, 4.7s)

## Changes summary

### AC1: Recording Detail에서 captureWarning이 있을 때 경고 메시지가 렌더된다

- `recordingDetailView.ts`: `RecordingHeader` 인터페이스에 `captureWarning: string | null` 추가, `loadedRecordingDetail`에서 recording.captureWarning을 header에 전달
- `RecordingDetailScreen.tsx`: `recording.captureWarning !== null`일 때 `<p className="capture-warning" role="alert">` 렌더

### AC2: captureWarning이 없으면 경고가 보이지 않는다

- 조건부 렌더링: `recording.captureWarning !== null`일 때만 경고 요소가 DOM에 들어감
- `captureWarning`이 `null`이면 아무것도 렌더되지 않음

### Additional (정지 직후 경고 + 목록 경고 표시)

- `recordingView.ts`: `SavedRecording`에 `captureWarning` 추가, `savedRecording()`에서 `stopped.warning`을 전달
- `RecordingScreen.tsx`: 저장됨 영역에서 `view.saved.captureWarning !== null`일 때 경고 메시지 렌더
- `recordingsView.ts`: `RecordingListItem`에 `captureWarning` 추가, `toListItem()`에서 recording.captureWarning 전달
- `RecordingsScreen.tsx`: 목록 항목 제목 앞에 `captureWarning !== null`일 때 `⚠` 뱃지 렌더
- `App.css`: `.capture-warning` (경고 배너 스타일) + `.list__warning-badge` (목록 뱃지 스타일) 추가
