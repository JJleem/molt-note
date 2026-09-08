import { useCallback, useEffect, useState } from 'react';
import {
  INITIAL_NAVIGATION_STATE,
  ROUTES,
  SIDEBAR_SCREENS,
  canGoBack,
  goBack,
  navigate,
  type NavigationState,
  type Route,
} from './navigation/routes';
import { aiNoteStatus, notionSyncStatus, transcriptionStatus } from './ipc/commands';
import type { AiNoteStatus, NotionSendStatus, TranscriptionStatus } from './ipc/types';
import { ErrorBoundary } from './screens/ErrorBoundary';
import { hasRunningWork, runningWork } from './screens/runningWork';
import { SCREEN_COMPONENTS } from './screens/registry';
import './App.css';

/** 배경에서 도는 일을 다시 물어보는 간격. */
const RUNNING_WORK_REFRESH_MS = 1_500;

function App() {
  const [nav, setNav] = useState<NavigationState>(INITIAL_NAVIGATION_STATE);

  /**
   * 배경에서 도는 일 — **어느 화면에 있든 보인다** (2026-09-08).
   *
   * 전사도 AI 노트도 Notion 전송도 배경 스레드에서 돌지만, 그 사실을 볼 자리가 그 화면
   * 안에만 있었다. 2시간 녹음의 전사를 걸어 두고 다른 화면으로 가면 끝났는지 알 방법이
   * 없었다.
   *
   * **실패해도 화면을 막지 않는다.** 물어보지 못한 것과 도는 것이 없는 것을 같게 둔다 —
   * 이 줄은 알림이지 판정이 아니고, 실패는 그 녹음의 화면이 §13으로 말한다.
   */
  const [transcription, setTranscription] = useState<TranscriptionStatus | null>(null);
  const [note, setNote] = useState<AiNoteStatus | null>(null);
  const [notion, setNotion] = useState<NotionSendStatus | null>(null);

  useEffect(() => {
    let current = true;

    const ask = () => {
      transcriptionStatus().then(
        (status) => {
          if (current) setTranscription(status);
        },
        () => {},
      );
      aiNoteStatus().then(
        (status) => {
          if (current) setNote(status);
        },
        () => {},
      );
      notionSyncStatus().then(
        (status) => {
          if (current) setNotion(status);
        },
        () => {},
      );
    };

    ask();
    const timer = setInterval(ask, RUNNING_WORK_REFRESH_MS);
    return () => {
      current = false;
      clearInterval(timer);
    };
  }, []);

  const working = runningWork(transcription, note, notion);

  const go = useCallback((route: Route) => setNav((state) => navigate(state, route)), []);
  const back = useCallback(() => setNav((state) => goBack(state)), []);

  const definition = ROUTES[nav.current.screen];
  const Screen = SCREEN_COMPONENTS[nav.current.screen];

  return (
    <div className="app">
      <nav className="sidebar" aria-label="화면">
        <p className="sidebar__brand">Molt Note</p>
        <ul className="sidebar__list">
          {SIDEBAR_SCREENS.map((screen) => (
            <li key={screen}>
              <button
                type="button"
                className={
                  nav.current.screen === screen ? 'sidebar__item sidebar__item--active' : 'sidebar__item'
                }
                aria-current={nav.current.screen === screen ? 'page' : undefined}
                onClick={() => go({ screen })}
              >
                {ROUTES[screen].title}
              </button>
            </li>
          ))}
        </ul>

        {/* 지금 배경에서 도는 일. **도는 것이 없으면 이 자리 자체가 없다** — 늘 있는 줄은
            소음이 되고, 정작 무언가 돌 때 눈에 띄지 않는다. 문장을 만드는 규칙은
            `runningWork`에 있고 여기서는 그린다. */}
        {hasRunningWork(working) && (
          <div className="sidebar__working" role="status" aria-live="polite">
            {working.map((one) => (
              <p key={one.text} className="sidebar__working-line">
                {one.text}
              </p>
            ))}
          </div>
        )}
      </nav>

      <main className="main">
        <header className="header">
          {canGoBack(nav) && (
            <button type="button" className="btn btn--ghost header__back" onClick={back}>
              뒤로
            </button>
          )}
          <h1 className="header__title">{definition.title}</h1>
        </header>
        {/* 화면 하나가 죽어도 사이드바와 헤더는 남는다 — 다른 화면으로 갈 수 있어야
            녹음 화면으로 되돌아갈 수 있다. `resetKey`가 route이므로 화면을 옮기면 앞
            화면의 실패는 따라오지 않는다. */}
        <ErrorBoundary resetKey={nav.current.screen}>
          <Screen route={nav.current} navigate={go} goBack={back} />
        </ErrorBoundary>
      </main>
    </div>
  );
}

export default App;
