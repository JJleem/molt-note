import { useEffect, useState } from 'react';
import { getRecording, listMissingAudio, recordingAudioSource } from '../ipc/commands';
import { FailureNotice } from './FailureNotice';
import {
  LOADING_RECORDING_DETAIL,
  MISSING_AUDIO_NOTICE,
  failedRecordingDetail,
  loadedRecordingDetail,
  type RecordingDetailView,
} from './recordingDetailView';
import type { ScreenProps } from './types';

// §5.C의 세 탭. AI Note가 없어도 화면은 정상 동작해야 한다 (INV-8).
const TABS = ['AI Note', 'Transcript', 'Recording'] as const;
type Tab = (typeof TABS)[number];

const TAB_EMPTY_STATE: Record<Tab, string> = {
  'AI Note': 'No AI note yet.',
  Transcript: 'No transcript yet.',
  Recording: 'No audio file yet.',
};

/**
 * Recording Detail 화면 (§5.C).
 *
 * 두 가지를 읽는다 — 이 녹음(`get_recording`)과, **레코드는 있는데 파일이 없는 녹음의 목록**
 * (`list_missing_audio`)이다. 파일이 있는지를 화면이 직접 보지 않는 이유는 하나다:
 * 파일시스템을 아는 코드는 Rust 안에만 있다 (PRODUCT-SPEC §12 · ADR-0001).
 * 그 목록은 **알리기만 한다** — 부른다고 해서 레코드가 지워지거나 고쳐지지 않는다 (INV-4).
 *
 * 응답을 화면 상태로 바꾸는 규칙은 {@link loadedRecordingDetail} · {@link failedRecordingDetail}에
 * 있다. 여기에는 그리는 일만 있다 — 그래서 네 경로(로딩 · 재생 가능 · 파일 없음 · 조회 실패)가
 * DOM 없이 판정된다 (§18).
 *
 * 재생은 `<audio controls>` 하나다. 파일 바이트는 IPC를 지나지 않고 asset protocol을 지나며
 * (`recordingAudioSource` · docs/ADR-0006-audio-playback.md), 그 주소는 로컬 webview 안에서만
 * 쓰인다 (INV-6). **실제 재생 음질은 자동으로 판정되지 않는다 — 사람이 확인하는 항목이다**
 * (`phase-prompt/02-reliable-recording.md`의 Human Review).
 */
export function RecordingDetailScreen({ route, goBack }: ScreenProps) {
  const [tab, setTab] = useState<Tab>('Transcript');
  const [view, setView] = useState<RecordingDetailView>(LOADING_RECORDING_DETAIL);
  /** 다시 시도 횟수. 늘어나면 다시 읽는다. */
  const [attempt, setAttempt] = useState(0);

  const recordingId = route.screen === 'recording-detail' ? route.recordingId : null;

  /** 다시 읽는다. 상태를 되돌리는 것은 effect가 아니라 이 사용자 동작의 일이다. */
  const retry = () => {
    setView(LOADING_RECORDING_DETAIL);
    setAttempt((count) => count + 1);
  };

  useEffect(() => {
    if (recordingId === null) {
      return;
    }
    // 응답이 오기 전에 화면을 떠났다면 그 응답으로 상태를 바꾸지 않는다.
    let current = true;

    // 둘 중 하나라도 답하지 못하면 실패 상태가 된다 — 파일이 그 자리에 있는지 모르는 채로
    // "재생할 수 있다"고 말하지 않는다.

    Promise.all([getRecording(recordingId), listMissingAudio()]).then(
      ([recording, missingAudio]) => {
        if (current) {
          setView(
            loadedRecordingDetail(recordingId, recording, missingAudio, recordingAudioSource),
          );
        }
      },
      (error: unknown) => {
        // 실패를 console에만 남기지 않는다. 화면 상태가 된다 (§13).
        if (current) setView(failedRecordingDetail(error));
      },
    );

    return () => {
      current = false;
    };
  }, [recordingId, attempt]);

  // 대상 recording 없이 이 화면에 도달하는 것도 정상 상태다.
  if (recordingId === null) {
    return (
      <div className="screen">
        <p className="empty">No recording selected.</p>
        <button type="button" className="action" onClick={goBack}>
          Back
        </button>
      </div>
    );
  }

  if (view.kind === 'loading') {
    return (
      <div className="screen">
        <p className="hint">Loading recording…</p>
      </div>
    );
  }

  if (view.kind === 'failed') {
    return (
      <div className="screen">
        <FailureNotice
          failure={view.failure}
          headline="This recording could not be read."
          onRetry={retry}
        />
        <button type="button" className="action" onClick={goBack}>
          Back
        </button>
      </div>
    );
  }

  if (view.kind === 'notFound') {
    return (
      <div className="screen">
        <p className="empty">This recording is no longer in the list.</p>
        <p className="hint">{view.recordingId}</p>
        <button type="button" className="action" onClick={goBack}>
          Back
        </button>
      </div>
    );
  }

  const { recording } = view;

  return (
    <div className="screen">
      <p className="detail__title">{recording.title}</p>
      <p className="hint">
        {recording.recordedAtLabel} · {recording.durationLabel}
      </p>

      {view.kind === 'playable' ? (
        <div className="detail__player">
          {/* 파일은 asset protocol로 흐른다. 주소를 만드는 것은 ipc 모듈의 일이다. */}
          <audio className="detail__audio" controls preload="metadata" src={view.audioSource} />
        </div>
      ) : (
        // 파일이 없다는 사실을 보여줄 뿐 아무것도 지우지 않는다 (INV-3 · INV-4).
        <div className="detail__player detail__player--missing" role="status">
          <p className="detail__missing">Audio file not found</p>
          <p className="hint">{MISSING_AUDIO_NOTICE}</p>
          <p className="detail__path">{view.audioPath}</p>
        </div>
      )}

      <div className="tabs" role="tablist">
        {TABS.map((name) => (
          <button
            key={name}
            type="button"
            role="tab"
            aria-selected={tab === name}
            className={tab === name ? 'tabs__tab tabs__tab--active' : 'tabs__tab'}
            onClick={() => setTab(name)}
          >
            {name}
          </button>
        ))}
      </div>

      {tab === 'Recording' ? (
        <p className="hint">
          {view.kind === 'playable'
            ? `${view.audioFormat} · ${recording.durationLabel}`
            : TAB_EMPTY_STATE.Recording}
        </p>
      ) : (
        <p className="empty">{TAB_EMPTY_STATE[tab]}</p>
      )}
    </div>
  );
}
