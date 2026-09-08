import { Component, type ErrorInfo, type ReactNode } from 'react';
import { toCrashView } from './crashView';

/**
 * 화면 하나가 그리다 죽어도 **앱 전체가 사라지지 않게** 받아 내는 자리.
 *
 * 문장을 만드는 규칙은 {@link toCrashView}에 있고 여기에는 없다 — 이 컴포넌트는 받아서
 * 그리기만 한다. `FailureNotice`가 IPC 거절에 대해 하는 일을 **렌더 중에 던져진 값**에 대해
 * 한다. React는 error boundary를 class로만 만들 수 있어서 이 파일만 class다.
 *
 * ## 2026-09-08 — 이 자리가 없어서 일어난 일
 *
 * 녹음 중에 에러 하나가 났고, 받아 낼 자리가 없어 React가 트리 전체를 언마운트했다.
 * **화면이 까맣게 됐고, 그 뒤에서 녹음은 2시간 4분 동안 계속 돌았다.** 사람은 Stop에
 * 손이 닿지 않았고 파일이 날아갔다고 생각했다. 실제로는 아무것도 잃지 않았지만,
 * **화면이 그 사실을 말해 주지 못한 것**이 사고였다.
 *
 * 그래서 이 자리는 두 가지를 한다.
 *
 * ```text
 * 1. 트리가 통째로 사라지지 않게 한다      → 남은 화면으로 계속 갈 수 있다
 * 2. 진행 중인 녹음이 살아 있다고 말한다    → 사람이 앱을 강제로 죽이지 않는다
 * ```
 *
 * **녹음을 여기서 멈추지 않는다.** session은 Rust가 들고 있고, 화면이 죽었다는 것은
 * 녹음에 대해 아무것도 뜻하지 않는다 (R-001). 화면이 backend를 대신해 판단하지 않는다는
 * 규칙이 여기서도 그대로다.
 */
interface ErrorBoundaryProps {
  readonly children: ReactNode;
  /**
   * 이 값이 바뀌면 죽은 상태를 지우고 다시 그려 본다.
   *
   * 화면을 옮겼는데 앞 화면의 실패가 그대로 남아 있으면 멀쩡한 화면을 볼 수 없다.
   */
  readonly resetKey?: string;
}

interface ErrorBoundaryState {
  readonly error: unknown;
  readonly caught: boolean;
  /** 어느 컴포넌트에서 났는가. React가 준 것 그대로이며, 없으면 `null`이다. */
  readonly componentStack: string | null;
  readonly resetKey: string | undefined;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = {
    error: null,
    caught: false,
    componentStack: null,
    resetKey: this.props.resetKey,
  };

  static getDerivedStateFromError(error: unknown): Partial<ErrorBoundaryState> {
    return { error, caught: true };
  }

  static getDerivedStateFromProps(
    props: ErrorBoundaryProps,
    state: ErrorBoundaryState,
  ): Partial<ErrorBoundaryState> | null {
    if (props.resetKey !== state.resetKey) {
      return { error: null, caught: false, componentStack: null, resetKey: props.resetKey };
    }
    return null;
  }

  /**
   * 어디서 났는지를 **화면에** 남긴다.
   *
   * console로 흘려보내지 않는다 — 그러면 사용자는 아무것도 알지 못하고
   * (`tests/screen-boundary.test.ts`의 §13 검사가 그것을 막는다), 2026-09-08처럼
   * **아무 흔적도 남지 않는다.** 화면에 있으면 사람이 읽고 그대로 옮길 수 있다.
   */
  componentDidCatch(_error: unknown, info: ErrorInfo) {
    this.setState({ componentStack: info.componentStack ?? null });
  }

  private reload = () => {
    window.location.reload();
  };

  private dismiss = () => {
    this.setState({ error: null, caught: false, componentStack: null });
  };

  render() {
    if (!this.state.caught) {
      return this.props.children;
    }

    const view = toCrashView(this.state.error);

    return (
      <div className="failure" role="alert">
        <p className="failure__headline">{view.headline}</p>
        <p className="failure__message">{view.message}</p>
        {/* 이 문단이 이 컴포넌트의 존재 이유다 — 녹음은 살아 있다. */}
        <p className="failure__answers">{view.recordingAnswer}</p>
        {view.detail !== null && <pre className="failure__detail">{view.detail}</pre>}
        {this.state.componentStack !== null && (
          <pre className="failure__detail">{this.state.componentStack}</pre>
        )}
        <button type="button" className="btn btn--secondary" onClick={this.reload}>
          Reload
        </button>
        <button type="button" className="btn btn--ghost" onClick={this.dismiss}>
          Try This Screen Again
        </button>
      </div>
    );
  }
}
