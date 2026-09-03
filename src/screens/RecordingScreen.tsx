import { useCallback, useEffect, useRef, useState } from 'react';
import {
  captureStatus,
  getSettings,
  listInputDevices,
  pauseCapture,
  resumeCapture,
  startCapture,
  stopCapture,
} from '../ipc/commands';
import { FailureNotice } from './FailureNotice';
import {
  INITIAL_RECORDING,
  editedTitle,
  failedAction,
  failedDevices,
  failedSession,
  microphoneLabel,
  microphoneNotice,
  observedDevices,
  observedSession,
  recordingControls,
  requestedAction,
  savedRecording,
  sessionDisplay,
  type RecordingView,
} from './recordingView';
import type { ScreenProps } from './types';

/**
 * 녹음 중에 경과 시간을 다시 물어보는 간격(밀리초).
 *
 * 화면이 시간을 세지 않으므로 표시되는 값은 언제나 backend가 마지막으로 말해 준 값이다.
 * 이 간격은 그 값이 얼마나 자주 새로 도착하는지일 뿐, 녹음 길이와는 아무 관계가 없다 —
 * 길이를 세는 곳은 `RecordingSession` 하나다.
 */
const ELAPSED_REFRESH_MS = 500;

/**
 * 녹음 화면 (§5.B).
 *
 * 보여주는 것은 §5 B가 정한 네 가지다 — **제목 · 선택된 microphone · 경과 시간 ·
 * Record/Pause/Resume/Stop**. §19에 따라 녹음 중이라는 사실과 경과 시간이 가장 크고
 * 분명하며, 그 둘 말고 시선을 끄는 것을 두지 않는다.
 *
 * ## 이 컴포넌트는 녹음을 소유하지 않는다 (R-001)
 *
 * 진행 중인 session을 들고 있는 것은 backend다 (`Recorder`, Tauri managed state).
 * 여기에는 session 핸들이 없고 `capture_status`로 **물어볼 뿐이다.** 그래서 사용자가 다른
 * 화면에 다녀와도, 이 컴포넌트가 unmount됐다 다시 mount돼도 녹음은 그대로 이어진다.
 * unmount가 하는 일은 되풀이 조회를 멈추는 것뿐이며, **녹음을 정지시키지 않는다**
 * (docs/ADR-0004-recording-session-lifecycle.md · tests/audio-boundary.test.ts).
 *
 * ## 상태를 만드는 규칙은 여기 없다
 *
 * 전부 `recordingView`에 있고 여기에는 그리는 일만 있다 — 그래서 상태 전이 · 장치 선택 ·
 * 실패 표현이 마이크 없이 판정된다 (§18). 경과 시간 문자열도 Rust가 만든 것을 그대로 쓴다.
 * backend 접근은 `src/ipc/commands.ts`의 일곱 함수뿐이며, 실패는 다른 화면과 같은
 * {@link FailureNotice}로 보인다 (§13).
 */
export function RecordingScreen({ navigate }: ScreenProps) {
  const [view, setView] = useState<RecordingView>(INITIAL_RECORDING);
  /** 장치·설정을 다시 읽은 횟수. 늘어나면 다시 읽는다. */
  const [deviceAttempt, setDeviceAttempt] = useState(0);

  /**
   * 마지막으로 보낸 상태 조회의 번호.
   *
   * 조회는 되풀이되므로 앞선 응답이 뒤늦게 도착할 수 있고, 그것을 그대로 쓰면 화면의 경과
   * 시간이 잠깐 뒤로 갔다가 다시 오른다. 가장 최근에 보낸 조회의 답만 화면에 반영한다.
   */
  const latestStatusRequest = useRef(0);

  /**
   * 지금 녹음이 어떤 상태인지 backend에 물어본다.
   *
   * 화면이 상태를 만들어 내지 않는 자리다. 시작·일시정지·재개·정지 뒤에도 이것을 부른다 —
   * 요청이 받아들여졌다는 것과 session이 지금 어떤 상태인지는 다른 사실이기 때문이다.
   */
  const refreshStatus = useCallback(() => {
    const request = (latestStatusRequest.current += 1);
    const isLatest = () => request === latestStatusRequest.current;

    captureStatus().then(
      (session) => {
        if (isLatest()) setView((state) => observedSession(state, session));
      },
      // 실패를 console에만 남기지 않는다. 화면 상태가 된다 (§13).
      (error: unknown) => {
        if (isLatest()) setView((state) => failedSession(state, error));
      },
    );
  }, []);

  const sessionState = view.session?.state ?? null;

  useEffect(() => {
    refreshStatus();

    // 녹음 중일 때만 되풀이해 물어본다 — 일시정지·정지 상태의 경과 시간은 시간이 흘러도
    // 변하지 않으므로 물어볼 이유가 없다.
    if (sessionState !== 'recording') {
      return;
    }

    const timer = setInterval(refreshStatus, ELAPSED_REFRESH_MS);
    return () => clearInterval(timer);
  }, [refreshStatus, sessionState]);

  useEffect(() => {
    // 응답이 오기 전에 화면을 떠났다면 그 응답으로 상태를 바꾸지 않는다.
    let current = true;

    // 저장된 default 값과 지금 열거된 장치를 함께 읽는다. 저장된 장치가 지금 없을 수 있고,
    // 그 사실은 숨기지 않고 화면에 나온다 (`recordingView`의 `selectedMicrophone`).
    Promise.all([getSettings(), listInputDevices()]).then(
      ([settings, devices]) => {
        if (current) setView((state) => observedDevices(state, settings.defaultMicrophone, devices));
      },
      (error: unknown) => {
        if (current) setView((state) => failedDevices(state, error));
      },
    );

    return () => {
      current = false;
    };
  }, [deviceAttempt]);

  /** 장치 목록은 부를 때마다 새로 만들어진다 — 마이크를 꽂거나 뺀 뒤에는 이것을 누른다. */
  const reloadDevices = () => setDeviceAttempt((count) => count + 1);

  const start = () => {
    const microphone = view.microphone;
    // 고른 장치가 실제로 있을 때만 시작한다. 없어진 장치의 키로 녹음을 시도하지 않는다.
    if (microphone.kind !== 'selected') {
      return;
    }

    setView((state) => requestedAction(state, 'start'));
    startCapture(microphone.deviceKey).then(
      refreshStatus,
      (error: unknown) => setView((state) => failedAction(state, 'start', error)),
    );
  };

  const pause = () => {
    setView((state) => requestedAction(state, 'pause'));
    pauseCapture().then(
      refreshStatus,
      (error: unknown) => setView((state) => failedAction(state, 'pause', error)),
    );
  };

  const resume = () => {
    setView((state) => requestedAction(state, 'resume'));
    resumeCapture().then(
      refreshStatus,
      (error: unknown) => setView((state) => failedAction(state, 'resume', error)),
    );
  };

  const stop = () => {
    setView((state) => requestedAction(state, 'stop'));
    // 성공은 파일이 확정되고 레코드가 저장됐다는 뜻이다 (R-002) — 그래서 저장된 녹음을
    // 그대로 목록으로 이어 줄 수 있다. 실패해도 파일은 남으며, 그 자리가 실패 문장에 있다.
    stopCapture(view.title).then(
      (stopped) => {
        setView((state) => savedRecording(state, stopped));
        refreshStatus();
      },
      (error: unknown) => {
        setView((state) => failedAction(state, 'stop', error));
        refreshStatus();
      },
    );
  };

  const controls = recordingControls(view);
  const display = sessionDisplay(view);
  const notice = microphoneNotice(view.microphone);
  const deviceFailure = view.microphone.kind === 'unknown' ? view.microphone.failure : null;

  return (
    <div className="screen">
      <label className="field recording__title" htmlFor="recording-title">
        <span className="field__label">Title</span>
        <input
          id="recording-title"
          type="text"
          className="field__input"
          placeholder="Untitled recording"
          value={view.title}
          onChange={(event) => setView((state) => editedTitle(state, event.currentTarget.value))}
        />
      </label>

      {/* 녹음 상태와 경과 시간이 화면에서 가장 크고 분명하다 (§19). */}
      <section className="recording">
        {/* 상태가 바뀌는 순간은 소리로도 알려 준다. 경과 시간은 계속 바뀌므로 알리지 않는다. */}
        <p
          className={display.live ? 'recording__state recording__state--live' : 'recording__state'}
          aria-live="polite"
        >
          {display.stateText}
        </p>
        <p className="recording__elapsed">{display.elapsedLabel}</p>
      </section>

      <div className="recording__controls">
        <button type="button" className="action" disabled={!controls.record} onClick={start}>
          Record
        </button>
        <button type="button" className="action" disabled={!controls.pause} onClick={pause}>
          Pause
        </button>
        <button type="button" className="action" disabled={!controls.resume} onClick={resume}>
          Resume
        </button>
        <button type="button" className="action" disabled={!controls.stop} onClick={stop}>
          Stop
        </button>
      </div>

      <section className="recording__microphone">
        <p className="recording__device">{microphoneLabel(view.microphone)}</p>
        {notice !== null && <p className="hint">{notice}</p>}
        <button type="button" className="action" onClick={reloadDevices}>
          Reload devices
        </button>
      </section>

      {/* 실패는 갈래마다 자기 문장을 갖고, 모양은 다른 화면과 같다 (§13). */}
      {view.trouble !== null && (
        <FailureNotice
          failure={view.trouble.failure}
          headline={view.trouble.headline}
          onRetry={view.trouble.kind === 'sessionStatus' ? refreshStatus : undefined}
        />
      )}
      {/* 무엇이 실패했는지는 위의 문장이 이미 말한다 — 같은 말을 두 번 적지 않는다. */}
      {deviceFailure !== null && <FailureNotice failure={deviceFailure} onRetry={reloadDevices} />}

      {view.saved !== null && (
        <section className="group">
          <h2 className="group__title">Saved</h2>
          <p className="empty">
            {view.saved.title} · {view.saved.durationLabel}
          </p>
          <button
            type="button"
            className="action"
            onClick={() => navigate({ screen: 'recordings' })}
          >
            Show in Recordings
          </button>
        </section>
      )}
    </div>
  );
}
