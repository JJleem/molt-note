// 화면 라우트 정의와 화면 전이.
//
// DOM · React · Tauri 어디에도 의존하지 않는 순수 모듈이다. 그래서 화면 전이가
// jsdom 없이 vitest로 그대로 판정된다 (§18 — 하드웨어·플랫폼 의존 코드와 순수 로직의 경계).
// React 컴포넌트는 이 모듈을 소비할 뿐 자기 나름의 라우팅 규칙을 갖지 않는다.
//
// URL 요구사항이 없으므로(§5는 화면 4개만 요구하고 deep link를 요구하지 않는다)
// 새로운 runtime 라우팅 의존성을 두지 않는다.

export const SCREEN_IDS = ['recordings', 'recording', 'recording-detail', 'settings'] as const;

export type ScreenId = (typeof SCREEN_IDS)[number];

/** 목록에서만 도달하는 recording-detail을 제외한, 사이드바에 직접 노출되는 화면. */
export type SidebarScreen = Exclude<ScreenId, 'recording-detail'>;

export const SIDEBAR_SCREENS: readonly SidebarScreen[] = ['recordings', 'recording', 'settings'];

/** 화면과, 그 화면에 도달하는 데 필요한 값. recording-detail만 대상 recording을 요구한다. */
export type Route =
  | { readonly screen: 'recordings' }
  | { readonly screen: 'recording' }
  | { readonly screen: 'recording-detail'; readonly recordingId: string }
  | { readonly screen: 'settings' };

export interface RouteDefinition {
  readonly screen: ScreenId;
  /** 화면 header에 쓰는 사람이 읽는 이름. */
  readonly title: string;
}

// Record<ScreenId, ...>이므로 화면을 추가하면 정의 누락이 컴파일 시점에 드러난다.
export const ROUTES: Record<ScreenId, RouteDefinition> = {
  recordings: { screen: 'recordings', title: 'Recordings' },
  recording: { screen: 'recording', title: 'Recording' },
  'recording-detail': { screen: 'recording-detail', title: 'Recording Detail' },
  settings: { screen: 'settings', title: 'Settings' },
};

/** 앱을 열었을 때 보이는 화면 (§5.A — Recordings가 기본 화면이다). */
export const HOME_ROUTE: Route = { screen: 'recordings' };

export interface NavigationState {
  readonly current: Route;
  /** 뒤로 가기 스택. 가장 마지막 항목이 직전 화면이다. */
  readonly history: readonly Route[];
}

export const INITIAL_NAVIGATION_STATE: NavigationState = {
  current: HOME_ROUTE,
  history: [],
};

export function isSameRoute(a: Route, b: Route): boolean {
  if (a.screen !== b.screen) return false;
  if (a.screen === 'recording-detail' && b.screen === 'recording-detail') {
    return a.recordingId === b.recordingId;
  }
  return true;
}

/** 같은 화면으로의 이동은 스택을 늘리지 않는다. */
export function navigate(state: NavigationState, next: Route): NavigationState {
  if (isSameRoute(state.current, next)) return state;
  return { current: next, history: [...state.history, state.current] };
}

export function canGoBack(state: NavigationState): boolean {
  return state.history.length > 0;
}

/** 스택이 비어 있으면(기본 화면) 아무 일도 일어나지 않는다. */
export function goBack(state: NavigationState): NavigationState {
  if (!canGoBack(state)) return state;
  const previous = state.history[state.history.length - 1];
  return { current: previous, history: state.history.slice(0, -1) };
}
