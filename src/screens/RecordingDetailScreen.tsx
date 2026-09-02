import { useState } from 'react';
import type { ScreenProps } from './types';

// §5.C의 세 탭. AI Note가 없어도 화면은 정상 동작해야 한다 (INV-8).
const TABS = ['AI Note', 'Transcript', 'Recording'] as const;
type Tab = (typeof TABS)[number];

const TAB_EMPTY_STATE: Record<Tab, string> = {
  'AI Note': 'No AI note yet.',
  Transcript: 'No transcript yet.',
  Recording: 'No audio file yet.',
};

export function RecordingDetailScreen({ route, goBack }: ScreenProps) {
  const [tab, setTab] = useState<Tab>('Transcript');

  // 대상 recording 없이 이 화면에 도달하는 것도 정상 상태다.
  if (route.screen !== 'recording-detail') {
    return (
      <div className="screen">
        <p className="empty">No recording selected.</p>
        <button type="button" className="action" onClick={goBack}>
          Back
        </button>
      </div>
    );
  }

  return (
    <div className="screen">
      <p className="hint">{route.recordingId}</p>
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
      <p className="empty">{TAB_EMPTY_STATE[tab]}</p>
    </div>
  );
}
