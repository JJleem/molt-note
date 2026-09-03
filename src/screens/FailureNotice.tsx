import type { Failure } from '../ipc/failure';
import { toFailureView } from './failureView';

/**
 * 실패 하나를 사용자에게 보여준다 (PRODUCT-SPEC §13).
 *
 * 문장을 만드는 규칙은 {@link toFailureView}에 있고 여기에는 없다 — 이 컴포넌트는 그리기만 한다.
 * 다시 시도할 수 있는 실패에만 시도 수단을 내준다. 눌러도 같은 결과가 나오는 버튼을 두지 않는다.
 *
 * `headline`은 **무엇을 하다 실패했는가**다. `Failure`는 원인만 알고 있으므로, 같은 원인이
 * 서로 다른 동작에서 왔을 때 그것을 구분하는 것은 부르는 쪽의 일이다 — 녹음 화면이
 * 권한 거부와 녹음 초기화 실패를 갈라 보여주는 자리가 여기다. 없으면 없는 대로 그린다.
 */
export function FailureNotice({
  failure,
  headline,
  onRetry,
}: {
  failure: Failure;
  headline?: string;
  onRetry?: () => void;
}) {
  const view = toFailureView(failure);

  return (
    <div className="failure" role="alert">
      {headline !== undefined && <p className="failure__headline">{headline}</p>}
      <p className="failure__message">{view.message}</p>
      <p className="failure__answers">
        {view.dataSafetyText} {view.retryText}
      </p>
      {view.detail !== null && <p className="failure__detail">{view.detail}</p>}
      {view.retryable && onRetry !== undefined && (
        <button type="button" className="action" onClick={onRetry}>
          Try Again
        </button>
      )}
    </div>
  );
}
