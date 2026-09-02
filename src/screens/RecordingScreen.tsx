import { useEffect, useState } from 'react';
import { listInputDevices, startCapture, stopCapture } from '../ipc/commands';
import { FailureNotice } from './FailureNotice';
import {
  LOADING_CAPTURE_SPIKE,
  captureStatusText,
  failedCapture,
  failedInputDevices,
  finishedCapture,
  loadedInputDevices,
  selectedDevice,
  selectedInputDevice,
  startedCapture,
  type CaptureSpikeView,
} from './captureSpikeView';

/**
 * 녹음 화면 (§5.B) — 지금은 **Phase 2A spike 표면**을 담고 있다.
 *
 * ADR-0003이 `PROVISIONAL`이고, 그것을 확정하는 8개 항목은 자동 Gate가 아니라 사람이
 * 실제 기기에서 확인한다 (docs/ADR-0003-recording-engine.md §12). 그 확인을 하려면 사람이
 * 앱에서 장치를 고르고 시작·정지하고 결과를 볼 수 있어야 한다. 이 패널이 그 일만 한다.
 *
 * **최종 Recording 화면이 아니다.** 경과 시간 · pause/resume · 재생 · 레코드 저장은 여기에
 * 없고, 임시라는 사실이 화면 맨 위에 그대로 적혀 있다 — Phase 2B가 이 자리를 대체한다.
 * 그래서 §19가 요구하는 최종 UX(가장 크고 명확한 상태·경과 시간)를 여기서 흉내내지 않는다.
 *
 * 상태를 만드는 규칙은 `captureSpikeView`에 있고 여기에는 그리는 일만 있다 — 그래서
 * 빈 목록 · 상태 전이 · 결과 · 실패 네 경로가 마이크 없이 판정된다 (§18).
 * backend 접근은 `src/ipc/commands.ts`의 세 함수뿐이며, 실패는 다른 화면과 같은
 * {@link FailureNotice}로 보인다 (§13).
 */
export function RecordingScreen() {
  const [view, setView] = useState<CaptureSpikeView>(LOADING_CAPTURE_SPIKE);
  /** 다시 읽은 횟수. 늘어나면 장치 목록을 다시 읽는다. */
  const [attempt, setAttempt] = useState(0);

  /**
   * 장치 목록을 다시 읽는다.
   *
   * 목록은 부를 때마다 새로 만들어진다 — 마이크를 꽂거나 뺀 뒤에는 사람이 이것을 눌러야
   * 화면이 지금의 사실을 보여준다.
   */
  const reload = () => {
    setView(LOADING_CAPTURE_SPIKE);
    setAttempt((count) => count + 1);
  };

  useEffect(() => {
    // 응답이 오기 전에 화면을 떠났다면 그 응답으로 상태를 바꾸지 않는다.
    let current = true;

    listInputDevices().then(
      (devices) => {
        if (current) setView(loadedInputDevices(devices));
      },
      (error: unknown) => {
        // 실패를 console에만 남기지 않는다. 화면 상태가 된다 (§13).
        if (current) setView(failedInputDevices(error));
      },
    );

    return () => {
      current = false;
    };
  }, [attempt]);

  const start = (deviceKey: string) => {
    setView((state) => startedCapture(state));
    startCapture(deviceKey).then(
      () => undefined,
      (error: unknown) => setView((state) => failedCapture(state, error)),
    );
  };

  const stop = () => {
    stopCapture().then(
      (report) => setView((state) => finishedCapture(state, report)),
      (error: unknown) => setView((state) => failedCapture(state, error)),
    );
  };

  /** 임시 표기 아래에 놓이는 본문. 상태 하나에 화면 하나가 대응한다. */
  function body() {
    if (view.kind === 'loading') {
      return <p className="hint">Loading input devices…</p>;
    }

    if (view.kind === 'failed') {
      return <FailureNotice failure={view.failure} onRetry={reload} />;
    }

    if (view.kind === 'empty') {
      // 마이크가 없는 것은 실패가 아니다 — 실패처럼 보이지 않게 적는다.
      return (
        <>
          <p className="empty">No input devices found.</p>
          <p className="hint">Connect a microphone, then reload the list.</p>
          <button type="button" className="action" onClick={reload}>
            Reload devices
          </button>
        </>
      );
    }

    const device = selectedDevice(view);
    const isRecording = view.status === 'recording';

    return (
      <>
        <section className="group">
          <h2 className="group__title">Input device</h2>
          <ul className="list">
            {view.devices.map((candidate) => (
              <li key={candidate.key}>
                <label className="field field--inline">
                  <input
                    type="radio"
                    name="spike-input-device"
                    value={candidate.key}
                    checked={candidate.key === view.selectedKey}
                    disabled={isRecording}
                    onChange={() => setView((state) => selectedInputDevice(state, candidate.key))}
                  />
                  {/* 이름이 같은 장치가 둘 있을 수 있으므로 보여주는 것은 label, 고르는 것은 key다. */}
                  <span className="field__label">
                    {candidate.label}
                    {candidate.isDefault && <span className="hint"> (default)</span>}
                  </span>
                </label>
              </li>
            ))}
          </ul>
          <button type="button" className="action" disabled={isRecording} onClick={reload}>
            Reload devices
          </button>
        </section>

        <section className="group">
          <h2 className="group__title">Capture</h2>
          <p className="recording__state">{captureStatusText(view.status)}</p>
          <p className="hint">{device === null ? 'No device chosen.' : device.label}</p>
          <button
            type="button"
            className="action"
            disabled={isRecording}
            onClick={() => start(view.selectedKey)}
          >
            Start
          </button>{' '}
          <button type="button" className="action" disabled={!isRecording} onClick={stop}>
            Stop
          </button>
        </section>

        {view.result !== null && (
          <section className="group">
            <h2 className="group__title">Result</h2>
            {/* Phase 2A가 확인해야 하는 네 값 그대로다 (ADR-0003 §12 항목 2·5·7). */}
            <dl className="spike-result">
              <dt>Device</dt>
              <dd>{view.result.deviceLabel}</dd>
              <dt>File</dt>
              <dd>{view.result.outputPath}</dd>
              <dt>Format</dt>
              <dd>{view.result.formatText}</dd>
              <dt>Size</dt>
              <dd>{view.result.sizeText}</dd>
            </dl>
            {view.result.isEmptyFile && (
              <p className="hint">The file is empty — nothing was written.</p>
            )}
          </section>
        )}

        {/* 실패는 다른 화면과 같은 모양으로 보인다. 여기만의 실패 표현을 만들지 않는다 (§13). */}
        {view.failure !== null && <FailureNotice failure={view.failure} />}
      </>
    );
  }

  return (
    <div className="screen">
      {/* 임시 표면이라는 사실은 주석이 아니라 화면에 있다. */}
      <p className="spike-notice" role="note">
        <strong>Phase 2A spike — temporary surface.</strong> It exists only so a person can validate
        the recording engine on a real device (ADR-0003 §12). Phase 2B replaces it with the real
        Recording screen.
      </p>
      {body()}
    </div>
  );
}
