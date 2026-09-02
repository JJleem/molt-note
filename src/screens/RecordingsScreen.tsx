import { useEffect, useState } from 'react';
import { listRecordings } from '../ipc/commands';
import { FailureNotice } from './FailureNotice';
import {
  LOADING_RECORDINGS,
  failedRecordings,
  loadedRecordings,
  type RecordingsView,
} from './recordingsView';
import type { ScreenProps } from './types';

/**
 * Recordings 화면 (§5.A).
 *
 * 목록은 저장소에서 읽는다 — `list_recordings` 하나가 이 화면이 아는 전부이며,
 * 질의도 스키마도 이 경계를 넘어오지 않는다 (docs/ADR-0001-local-persistence.md).
 *
 * 응답을 화면 상태로 바꾸는 규칙은 {@link loadedRecordings} · {@link failedRecordings}에 있다.
 * 여기에는 그리는 일만 있다 — 그래서 세 경로(목록 · 빈 목록 · 실패)가 DOM 없이 판정된다 (§18).
 */
export function RecordingsScreen({ navigate }: ScreenProps) {
  const [view, setView] = useState<RecordingsView>(LOADING_RECORDINGS);
  /** 다시 시도 횟수. 늘어나면 목록을 다시 읽는다. */
  const [attempt, setAttempt] = useState(0);

  /** 다시 읽는다. 상태를 되돌리는 것은 effect가 아니라 이 사용자 동작의 일이다. */
  const retry = () => {
    setView(LOADING_RECORDINGS);
    setAttempt((count) => count + 1);
  };

  useEffect(() => {
    // 응답이 오기 전에 화면을 떠났다면 그 응답으로 상태를 바꾸지 않는다.
    let current = true;

    listRecordings().then(
      (recordings) => {
        if (current) setView(loadedRecordings(recordings));
      },
      (error: unknown) => {
        // 실패를 console에만 남기지 않는다. 화면 상태가 된다 (§13).
        if (current) setView(failedRecordings(error));
      },
    );

    return () => {
      current = false;
    };
  }, [attempt]);

  if (view.kind === 'loading') {
    return (
      <div className="screen">
        <p className="hint">Loading recordings…</p>
      </div>
    );
  }

  if (view.kind === 'failed') {
    return (
      <div className="screen">
        <FailureNotice failure={view.failure} onRetry={retry} />
      </div>
    );
  }

  if (view.kind === 'empty') {
    return (
      <div className="screen">
        <p className="empty">No recordings yet.</p>
        <p className="hint">Recordings you make will be listed here.</p>
        <button type="button" className="action" onClick={() => navigate({ screen: 'recording' })}>
          New Recording
        </button>
      </div>
    );
  }

  return (
    <div className="screen">
      <ul className="list">
        {view.items.map((item) => (
          <li key={item.id}>
            <button
              type="button"
              className="list__row"
              onClick={() => navigate({ screen: 'recording-detail', recordingId: item.id })}
            >
              <span className="list__title">{item.title}</span>
              <span className="hint">
                {item.recordedAtLabel} · {item.durationLabel}
              </span>
              <span className="list__statuses">
                {item.statuses.map((badge) => (
                  <span key={badge.label} className="list__status">
                    {badge.label} <span className="list__status-value">{badge.text}</span>
                  </span>
                ))}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
