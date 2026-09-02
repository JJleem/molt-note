// 실패 경계 테스트.
//
// command가 거절되며 넘어온 값이 무엇이든 화면이 그릴 수 있는 하나의 모양이 되는지 본다 (§13).
// Tauri도 DOM도 필요하지 않다 — 순수 변환이다.
import { describe, expect, it } from 'vitest';
import { isFailure, toFailure, type Failure } from './failure';

const storageFailure: Failure = {
  kind: 'storage',
  message: '로컬 저장소를 열지 못했다: /Users/someone/molt-note.db',
  detail: 'unable to open database file',
  sourceDataSafe: true,
  retryable: true,
};

describe('command 실패 변환', () => {
  it('Rust가 보낸 구조화된 실패는 그대로 쓴다', () => {
    expect(toFailure(storageFailure)).toEqual(storageFailure);
  });

  it('구조화된 실패를 알아본다', () => {
    expect(isFailure(storageFailure)).toBe(true);
    expect(isFailure({ message: '모양이 다르다' })).toBe(false);
    expect(isFailure('저장소 오류')).toBe(false);
    expect(isFailure(null)).toBe(false);
  });

  it('예상하지 못한 값도 보여줄 수 있는 실패가 된다', () => {
    // 계약과 다른 값이 와도 console에만 남기고 끝내지 않는다.
    const failure = toFailure(new Error('command not found: list_recordings'));

    expect(failure.kind).toBe('unexpected');
    expect(failure.message.length).toBeGreaterThan(0);
    expect(failure.detail).toBe('command not found: list_recordings');
  });

  it('문자열로 거절된 경우 그 문자열이 원인으로 남는다', () => {
    expect(toFailure('invalid args').detail).toBe('invalid args');
  });

  it('아무 값도 없이 거절돼도 세 가지 질문에 답한다', () => {
    const failure = toFailure(undefined);

    expect(failure.message.length).toBeGreaterThan(0);
    expect(failure.detail).toBeNull();
    expect(typeof failure.sourceDataSafe).toBe('boolean');
    expect(typeof failure.retryable).toBe('boolean');
  });
});
