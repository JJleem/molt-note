import { describe, expect, it } from 'vitest';
import {
  CRASH_HEADLINE,
  CRASH_MESSAGE,
  CRASH_RECORDING_ANSWER,
  toCrashView,
} from './crashView';

describe('toCrashView', () => {
  it('Error에서 스택을 원인으로 남긴다 — 다음에 어디서 났는지 알 수 있어야 한다', () => {
    const error = new Error('boom');
    const view = toCrashView(error);

    expect(view.detail).not.toBeNull();
    expect(view.detail).toContain('boom');
  });

  it('스택이 없는 Error도 이름과 메시지로 읽힌다', () => {
    const error = new Error('boom');
    error.stack = undefined;

    expect(toCrashView(error).detail).toBe('Error: boom');
  });

  it('문자열로 던져진 값도 원인이 된다', () => {
    expect(toCrashView('something broke').detail).toBe('something broke');
  });

  it('읽을 수 없는 원인은 null이다 — 없는 것을 지어내지 않는다', () => {
    expect(toCrashView(null).detail).toBeNull();
    expect(toCrashView(undefined).detail).toBeNull();
    expect(toCrashView('').detail).toBeNull();
  });

  it('객체는 JSON으로 남는다', () => {
    expect(toCrashView({ code: 7 }).detail).toBe('{"code":7}');
  });

  it('순환 참조가 있어도 던지지 않는다 — 죽은 자리에서 또 죽지 않는다', () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;

    expect(() => toCrashView(circular)).not.toThrow();
    expect(toCrashView(circular).detail).not.toBeNull();
  });

  it('문장 셋은 원인과 무관하게 언제나 있다', () => {
    const view = toCrashView(null);

    expect(view.headline).toBe(CRASH_HEADLINE);
    expect(view.message).toBe(CRASH_MESSAGE);
    expect(view.recordingAnswer).toBe(CRASH_RECORDING_ANSWER);
  });

  /**
   * 2026-09-08의 사고가 이 검사의 이유다. 화면이 죽었을 때 **진행 중인 녹음이 살아 있다는
   * 사실**을 말하지 않으면 사람은 앱을 강제로 죽인다. 그 문장이 사라지면 이 테스트가 깨진다.
   */
  it('진행 중인 녹음이 살아 있다는 사실과 강제 종료하지 말라는 것을 말한다', () => {
    const answer = toCrashView(new Error('x')).recordingAnswer;

    expect(answer).toContain('still running');
    expect(answer).toContain('do not force quit');
  });
});
