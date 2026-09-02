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
      <nav className="sidebar" aria-label="Screens">
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
            <button type="button" className="header__back" onClick={back}>
              Back
            </button>
          )}
          <h1 className="header__title">{definition.title}</h1>
        </header>
        <Screen route={nav.current} navigate={go} goBack={back} />
      </main>
    </div>
  );
}

export default App;
