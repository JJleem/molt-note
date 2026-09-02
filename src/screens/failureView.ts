/**
 * 실패 하나를 화면이 그대로 그릴 수 있는 문장으로 옮긴다 (PRODUCT-SPEC §13).
 *
 * §13은 모든 실패에 대해 사용자가 세 가지를 알 수 있어야 한다고 정한다:
 *
 * ```text
 * 무엇이 실패했는가 · 원본 데이터는 안전한가 · 다시 시도할 수 있는가
 * ```
 *
 * `Failure`는 그 세 가지를 필드로 들고 있을 뿐 문장을 들고 있지 않다. 문장을 만드는 규칙이
 * 화면마다 흩어지면 같은 실패가 화면마다 다르게 읽힌다. 그래서 여기 한 곳에 둔다.
 *
 * React도 DOM도 Tauri도 알지 않는다 — 실패 표시 경로를 vitest로 그대로 판정할 수 있다 (§18).
 */
import type { Failure } from '../ipc/failure';

/** 실패 하나의 화면 표현. 세 가지 질문에 대한 답이 각각 하나의 필드다. */
export interface FailureView {
  /** 무엇이 실패했는가. */
  readonly message: string;
  /** 원인의 기술적 표현. 없을 수 있다. */
  readonly detail: string | null;
  /** 원본 데이터는 안전한가. */
  readonly dataSafetyText: string;
  /** 다시 시도할 수 있는가. */
  readonly retryText: string;
  /** 화면이 다시 시도 수단을 내줘야 하는가. */
  readonly retryable: boolean;
}

export function toFailureView(failure: Failure): FailureView {
  return {
    message: failure.message,
    detail: failure.detail,
    dataSafetyText: failure.sourceDataSafe
      ? '저장된 데이터는 그대로 있다.'
      : '저장된 데이터가 영향을 받았을 수 있다.',
    retryText: failure.retryable
      ? '다시 시도할 수 있다.'
      : '다시 시도해도 같은 결과가 나온다.',
    retryable: failure.retryable,
  };
}
