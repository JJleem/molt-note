# 녹음 화면이 §5-B · §19 · §13 · R-001을 어디서 만족하는가

RUN-20260902T095416Z-TASK-020 / 2026-09-02

각 항목이 저장소의 **어느 줄**에서 판정되는지 적는다. 서술이 아니라 위치가 근거다.

## §5-B의 네 요소

| 요소 | 화면 | 상태를 만드는 곳 | 테스트 |
| --- | --- | --- | --- |
| 제목 | `RecordingScreen.tsx` `<input id="recording-title">` | `recordingView.editedTitle` | `recordingView.test.ts` — "입력한 제목이 상태에 남는다" |
| 선택된 microphone | `recording__device` + 그 아래 `hint` | `recordingView.selectedMicrophone` / `microphoneLabel` / `microphoneNotice` | "선택된 microphone" describe 8개 |
| 경과 시간 | `recording__elapsed` | `recordingView.sessionDisplay` | "경과 시간" describe 4개 |
| Record/Pause/Resume/Stop | `recording__controls`의 네 버튼 | `recordingView.recordingControls` | "상태 전이와 버튼" describe 6개 |

## §19 — 녹음 상태와 경과 시간이 가장 분명하다

- `recording__elapsed`는 56px로 화면에서 가장 큰 요소다 (`src/App.css`).
- 그 위의 `recording__state`는 녹음 중일 때만 `--live`가 붙어 흐리지 않다.
- 상태 문자열은 §5-B의 스케치 그대로다 (`● REC`).
- 깜빡임 · 그림자 · gradient · 파형은 없다. spike 패널의 점선 상자(`.spike-notice`)와
  결과 `<dl>`(`.spike-result`)은 CSS에서 함께 지웠다.

## 경과 시간은 backend가 준 문자열 그대로다

- `recordingView.sessionDisplay`는 `elapsedLabel: session.elapsedLabel`을 그대로 넘긴다.
  TypeScript에는 밀리초 → `0:07` 계산이 없다.
- `tests/screen-boundary.test.ts`가 두 가지로 그것을 붙잡는다.
  1. src/ 전체에 길이 포맷 계산 모양(`% 60` · `/ 60` · `/ 1000` · `padStart(2` 등)이 없다.
  2. (이 Task가 추가) `recordingView.ts`에 `elapsedLabel: session.elapsedLabel`이 있다.
- 길이 규칙이 사는 곳은 여전히 `src-tauri/src/domain/duration.rs` 하나다.
- 화면은 `capture_status`를 500ms마다 **다시 물어볼 뿐**이며(`ELAPSED_REFRESH_MS`),
  두 조회 사이의 시간을 스스로 세지 않는다. 늦게 도착한 앞선 응답은 버린다
  (`latestStatusRequest`) — 경과 시간이 뒤로 갔다 오지 않게 한다.

## §13 — permission denied와 녹음 초기화 실패가 갈라져 화면에 도달한다

`recordingView.RecordingTroubleKind`가 다섯 갈래를 구분한다.

```text
microphonePermission   권한 문제 (FailureKind::MicrophonePermission)      → 시스템 설정을 열어야 한다
recordingStart         그 밖의 시작 실패 = 녹음 초기화 실패               → 다시 시도하거나 장치를 바꾼다
recordingControl       일시정지 / 재개 실패
recordingStop          정지 실패
sessionStatus          상태 조회 실패
```

- 갈래마다 다른 headline이 붙고(`TROUBLE_HEADLINE`), 실패 자체는 **다른 화면과 같은
  `FailureNotice`**로 그려진다. `FailureNotice`에는 선택적 `headline`만 더했고 모양은 그대로다.
- 권한 판정은 화면이 하지 않는다 — `src-tauri/src/platform/microphone.rs`가 정한
  `FailureKind::MicrophonePermission`을 존중할 뿐이다. 그래서 "파일을 만들지 못했다"가
  권한 안내로 바뀌는 일이 화면 쪽에서도 생기지 않는다.
- 테스트: "실패는 갈래가 나뉘어 화면에 도달한다 (§13)" describe 8개.
  특히 `denied.trouble.kind !== failedToInitialize.trouble.kind`와 headline이 서로 다르다.

## R-001 — session이 이 컴포넌트에 묶여 있지 않다

- 진행 중인 녹음을 들고 있는 것은 `Recorder`(Tauri managed state, `src-tauri/src/lib.rs`)다.
  화면에는 session 핸들이 없고 `capture_status`로 물어본다.
- mount 시 하는 일은 **조회**뿐이다. 이미 녹음 중이면 그 상태 그대로 화면에 나타난다.
- unmount 시 하는 일은 `clearInterval` 하나다 — 정지 command를 부르지 않는다.
  `stopCapture`는 사용자가 Stop을 눌렀을 때만 불린다.
- `tests/audio-boundary.test.ts`가 `startCapture`가 핸들이 아니라 `Promise<void>`를
  돌려주는 것과 `app.manage(Recorder::open_for(app))`을 계속 검사한다.

## Stop 성공 → Recordings 목록으로 이어진다

- `stop_capture`의 성공은 **파일이 확정·확인되고 레코드가 저장됐다**는 뜻이다 (R-002).
  그래서 `savedRecording`이 남긴 값은 목록에도 있는 녹음이다.
- 화면은 저장된 제목과 길이(backend가 만든 `durationLabel`)를 보여주고
  `Show in Recordings` 버튼으로 `{ screen: 'recordings' }`로 이동한다.
  Recordings 화면은 mount 때 `list_recordings`를 다시 읽으므로 방금 저장된 녹음이 보인다.

## 자동으로 판정하지 않은 것 (Human Review)

- 실제 마이크로 녹음한 소리의 품질 · 실제 권한 프롬프트가 뜨는지 · 장시간 녹음 안정성.
  이 Task는 그중 어느 것도 자동 PASS로 적지 않는다 (§18 · phase-prompt/02 Human Review).
