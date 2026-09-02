import type { Failure } from '../ipc/failure';
import { toFailureView } from './failureView';

/**
 * 실패 하나를 사용자에게 보여준다 (PRODUCT-SPEC §13).
 *
 * 문장을 만드는 규칙은 {@link toFailureView}에 있고 여기에는 없다 — 이 컴포넌트는 그리기만 한다.
 * 다시 시도할 수 있는 실패에만 시도 수단을 내준다. 눌러도 같은 결과가 나오는 버튼을 두지 않는다.
 */
export function FailureNotice({ failure, onRetry }: { failure: Failure; onRetry?: () => void }) {
  const view = toFailureView(failure);

  return (
    <div className="failure" role="alert">
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
