import type { Route } from '../navigation/routes';

/**
 * 모든 화면 컴포넌트가 받는 공통 props.
 *
 * 화면은 자기 자신의 라우트와 전이 수단만 받는다. 화면끼리 직접 서로를 알지 않고,
 * 이동은 언제나 navigation 모듈을 거친다.
 */
export interface ScreenProps {
  readonly route: Route;
  readonly navigate: (route: Route) => void;
  readonly goBack: () => void;
}
