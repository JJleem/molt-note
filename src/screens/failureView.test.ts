// 실패 표시 테스트.
//
// §13이 요구하는 세 가지 답이 화면 표현에 실제로 있는지 본다:
// 무엇이 실패했는가 · 원본 데이터는 안전한가 · 다시 시도할 수 있는가.
import { describe, expect, it } from 'vitest';
import type { Failure } from '../ipc/failure';
import { toFailureView } from './failureView';

function failure(overrides: Partial<Failure> = {}): Failure {
  return {
    kind: 'storage',
    message: '로컬 저장소를 열지 못했다.',
    detail: 'unable to open database file',
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

describe('실패 표현', () => {
  it('무엇이 실패했는지 사용자에게 보이는 문장으로 남는다', () => {
    const view = toFailureView(failure());

    expect(view.message).toBe('로컬 저장소를 열지 못했다.');
    expect(view.detail).toBe('unable to open database file');
  });

  it('세 가지 질문에 모두 답한다', () => {
    const view = toFailureView(failure());

    expect(view.message.length).toBeGreaterThan(0);
    expect(view.dataSafetyText.length).toBeGreaterThan(0);
    expect(view.retryText.length).toBeGreaterThan(0);
  });

  it('원본이 안전한 경우와 아닌 경우가 다르게 읽힌다', () => {
    const safe = toFailureView(failure({ sourceDataSafe: true }));
    const unsafe = toFailureView(failure({ sourceDataSafe: false }));

    expect(safe.dataSafetyText).not.toBe(unsafe.dataSafetyText);
  });

  it('다시 시도할 수 없는 실패에는 시도 수단을 내주지 않는다', () => {
    const permanent = toFailureView(failure({ retryable: false }));

    expect(permanent.retryable).toBe(false);
    expect(permanent.retryText).not.toBe(toFailureView(failure({ retryable: true })).retryText);
  });

  it('원인이 없어도 표현이 깨지지 않는다', () => {
    expect(toFailureView(failure({ detail: null })).detail).toBeNull();
  });
});
