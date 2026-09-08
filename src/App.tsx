import { useCallback, useState } from 'react';
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
import { ErrorBoundary } from './screens/ErrorBoundary';
import { SCREEN_COMPONENTS } from './screens/registry';
import './App.css';

function App() {
  const [nav, setNav] = useState<NavigationState>(INITIAL_NAVIGATION_STATE);

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
