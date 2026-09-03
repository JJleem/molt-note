# TASK-016 — AC4 근거 (녹음 session의 소유자 · 상태 조회 · cpal 경계 · 문서)

## (1) 진행 중인 session의 소유자는 Tauri managed state다

`src-tauri/src/lib.rs` — `setup`에서 `app.manage(Recorder::open_for(app))`.
진행 중인 녹음은 `Recorder` 안의 `Mutex<Option<ActiveRecording>>` 하나에만 있고,
`ActiveRecording`은 상태 기계(`RecordingSession`)와 열린 캡처(`ActiveCapture`)를
**한 값으로 묶는다** — 따로 놓이면 "session은 일시정지인데 파일에는 계속 쓰이는"
상태가 만들어질 수 있기 때문이다 (`src-tauri/src/commands/mod.rs`).

React 컴포넌트가 session 핸들을 갖지 않는다:

- `startCapture(deviceKey): Promise<void>` — **돌려주는 핸들이 없다** (`src/ipc/commands.ts`).
- `src/screens/` 아래 어떤 파일도 session 객체를 들고 있지 않다. 화면이 갖는 것은
  `captureSpikeView.ts`의 순수 상태 변환 값(`'idle' | 'recording' | 'finished'` 등)뿐이며,
  그것은 녹음 자체가 아니라 화면 표시다.
- 화면은 `captureStatus()`로 **물어본다.** 그래서 화면이 unmount·재렌더돼도 backend의
  답은 같다 (R-001).

## (2) 상태 조회 command가 상태 · 경과 ms · Rust가 만든 문자열을 돌려준다

`capture_status` → `Recorder::status()` → `SessionStatusPayload`:

| 필드 | 값 |
| --- | --- |
| `state` | `idle · recording · paused · stopped` (`SessionState::as_str`) |
| `elapsedMs` | 일시정지 구간을 뺀 경과 밀리초 (`RecordingSession::elapsed_ms`) |
| `elapsedLabel` | **Rust가 만든** 표시용 문자열 (`RecordingSession::elapsed_label` → `crate::domain::format_duration_ms`) |

TypeScript에 길이 계산이 없다 — `src/ipc/types.ts`의 `SessionStatus`는 두 값을 그대로 읽는다.
`CaptureReportPayload`도 같은 이유로 `durationMs`와 `durationLabel`을 함께 보낸다.

진행 중인 녹음이 없으면 특별한 빈 값을 만들지 않고 **idle session의 답**(`idle` · `0:00`)을
그대로 돌려준다.

## (3) cpal을 아는 파일

```text
$ grep -rln "cpal" src-tauri/src src-tauri/tests src
src-tauri/src/audio/system_capture.rs
src-tauri/src/audio/mod.rs
src-tauri/src/audio/system_devices.rs
```

실제로 cpal에 의존하는 코드는 **`system_capture.rs`와 `system_devices.rs` 둘뿐이다.**
`mod.rs`의 세 번째 히트는 `use`도 타입 참조도 아니고, 모듈 doc comment 안의 경계 설명표에서
그 두 파일을 "cpal을 아는 두 자리"라고 **이름 부르는 문장**이다 (`src-tauri/src/audio/mod.rs`
17–18행). 나머지 코드는 `SampleSource` / `InputDeviceSource` trait만 안다.

## (4) 결정과 근거가 문서에 있다

`docs/ADR-0004-recording-session-lifecycle.md` (Status: Accepted · Task: TASK-016)

- §2 Decision — 소유자는 Tauri managed state의 `Recorder`, 화면은 핸들을 소유하지 않는다
- §3 — **왜** backend인가: 컴포넌트 수명에 묶이면 unmount·hot reload·StrictMode에서
  녹음이 사라진다. ref를 끌어올리거나 전역 스토어에 두는 것은 소유자를 옮긴 것이 아니라
  유실 조건을 어렵게 만든 것뿐이다.
- §4 — 왜 경과 시간 문자열을 Rust가 만드는가 (규칙이 두 곳에 생기는 것을 막는다)
- §5 — pause가 장치도 파일도 닫지 않는 이유와 그 대가
- §7 — 재시작 후 복구는 범위 밖이라는 것

## ADR-0003의 Status

`docs/ADR-0003-recording-engine.md` 4행: `Status:   PROVISIONAL — pending human device validation`
— **건드리지 않았다.** `git diff docs/`가 비어 있고(ADR-0004는 새 파일), ADR-0004 §9가
그 Status를 바꾸지 않는다는 것을 명시한다.
