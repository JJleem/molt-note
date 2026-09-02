// 라우트 정의와 화면 전이 테스트.
// routes.ts가 DOM에 의존하지 않으므로 jsdom 없이 그대로 판정된다.
import { describe, expect, it } from 'vitest';
import {
  HOME_ROUTE,
  INITIAL_NAVIGATION_STATE,
  ROUTES,
  SCREEN_IDS,
  SIDEBAR_SCREENS,
  canGoBack,
  goBack,
  isSameRoute,
  navigate,
  type Route,
  type ScreenId,
} from './routes';

// §5의 화면 4개. 이 목록이 줄어들면 테스트가 깨진다.
const EXPECTED_SCREENS: readonly ScreenId[] = [
  'recordings',
  'recording',
  'recording-detail',
  'settings',
];

/** 각 화면에 도달하는 라우트 값. recording-detail만 대상 recording을 요구한다. */
const routeFor = (screen: ScreenId): Route =>
  screen === 'recording-detail' ? { screen, recordingId: 'rec-1' } : { screen };

describe('route definitions', () => {
  it('§5의 네 화면이 모두 등록되어 있다', () => {
    expect([...SCREEN_IDS]).toEqual([...EXPECTED_SCREENS]);
  });

  it('모든 화면이 사람이 읽는 제목을 가진다', () => {
    for (const screen of EXPECTED_SCREENS) {
      expect(ROUTES[screen].screen).toBe(screen);
      expect(ROUTES[screen].title.length).toBeGreaterThan(0);
    }
  });

  it('사이드바는 Recordings · Recording · Settings만 노출하고 detail은 목록에서만 도달한다', () => {
    expect([...SIDEBAR_SCREENS]).toEqual(['recordings', 'recording', 'settings']);
    expect(SIDEBAR_SCREENS).not.toContain('recording-detail');
    for (const screen of SIDEBAR_SCREENS) {
      expect(SCREEN_IDS).toContain(screen);
    }
  });

  it('기본 화면은 Recordings다', () => {
    expect(HOME_ROUTE.screen).toBe('recordings');
    expect(INITIAL_NAVIGATION_STATE.current).toEqual(HOME_ROUTE);
    expect(canGoBack(INITIAL_NAVIGATION_STATE)).toBe(false);
  });
});

describe('screen transitions', () => {
  it('네 화면 모두 초기 상태에서 전이로 도달한다', () => {
    for (const screen of EXPECTED_SCREENS) {
      const next = navigate(INITIAL_NAVIGATION_STATE, routeFor(screen));
      expect(next.current.screen).toBe(screen);
    }
  });

  it('recording-detail 전이가 대상 recording을 유지한다', () => {
    const next = navigate(INITIAL_NAVIGATION_STATE, {
      screen: 'recording-detail',
      recordingId: 'rec-42',
    });
    expect(next.current).toEqual({ screen: 'recording-detail', recordingId: 'rec-42' });
  });

  it('같은 화면으로의 이동은 스택을 늘리지 않는다', () => {
    const settings = navigate(INITIAL_NAVIGATION_STATE, { screen: 'settings' });
    expect(navigate(settings, { screen: 'settings' })).toBe(settings);
    expect(settings.history).toHaveLength(1);
  });

  it('다른 recording의 detail은 서로 다른 화면으로 취급한다', () => {
    const first = navigate(INITIAL_NAVIGATION_STATE, {
      screen: 'recording-detail',
      recordingId: 'rec-1',
    });
    const second = navigate(first, { screen: 'recording-detail', recordingId: 'rec-2' });
    expect(second.current).toEqual({ screen: 'recording-detail', recordingId: 'rec-2' });
    expect(second.history).toHaveLength(2);
  });

  it('뒤로 가기가 직전 화면으로 되돌린다', () => {
    const settings = navigate(INITIAL_NAVIGATION_STATE, { screen: 'settings' });
    const detail = navigate(settings, { screen: 'recording-detail', recordingId: 'rec-1' });

    expect(canGoBack(detail)).toBe(true);
    const backToSettings = goBack(detail);
    expect(backToSettings.current).toEqual({ screen: 'settings' });

    const backToRecordings = goBack(backToSettings);
    expect(backToRecordings.current).toEqual(HOME_ROUTE);
    expect(canGoBack(backToRecordings)).toBe(false);
  });

  it('기본 화면에서의 뒤로 가기는 아무 일도 하지 않는다', () => {
    expect(goBack(INITIAL_NAVIGATION_STATE)).toBe(INITIAL_NAVIGATION_STATE);
  });
});

describe('isSameRoute', () => {
  it('화면이 다르면 다른 라우트다', () => {
    expect(isSameRoute({ screen: 'recordings' }, { screen: 'settings' })).toBe(false);
  });

  it('detail은 recordingId까지 같아야 같은 라우트다', () => {
    const a: Route = { screen: 'recording-detail', recordingId: 'rec-1' };
    expect(isSameRoute(a, { screen: 'recording-detail', recordingId: 'rec-1' })).toBe(true);
    expect(isSameRoute(a, { screen: 'recording-detail', recordingId: 'rec-2' })).toBe(false);
  });
});
