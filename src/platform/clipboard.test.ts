// clipboard 경계 (docs/ADR-0010-manual-ai-handoff.md §7 · `phase-prompt/05.5` R-4 · 요구 5).
//
// **이 파일은 실제 시스템 clipboard를 건드리지 않는다.** 쓰는 대상은 언제나 아래의
// test double이며, 이 파일 어디에서도 이 창의 clipboard를 집지 않는다 — 그 자리는
// `systemClipboard()` 한 곳뿐이고, 이 테스트는 그것을 부르지 않는다
// (`tests/screen-boundary.test.ts`가 `src/` 전체에서 그 자리가 하나인 것을 지킨다).
//
// 여기서 판정하는 것은 셋이다.
//
//   1. 경계는 **던지지 않는다** — 성공도 실패도 값으로 돌아온다 (§13)
//   2. 능력이 없는 것과 거절된 것이 **갈라진다** — 사용자가 할 수 있는 일이 다르다
//   3. 새 `FailureKind`를 만들지 않는다 — `unexpected`에 표시를 실어 보낸다 (§7.2)
import { describe, expect, it } from 'vitest';
import {
  CLIPBOARD_TROUBLE_DETAIL,
  clipboardTrouble,
  clipboardWriter,
  copyText,
  type ClipboardWriter,
} from './clipboard';
import type { Failure } from '../ipc/failure';

/** 쓴 것을 기억하는 double. 실제 clipboard 대신 이것이 쓰인다. */
function writerDouble(): ClipboardWriter & { readonly written: string[] } {
  const written: string[] = [];
  return {
    written,
    async writeText(text: string): Promise<void> {
      written.push(text);
    },
  };
}

/** 언제나 거절하는 double. 브라우저가 거절할 때의 자리를 대신한다. */
function rejectingWriter(error: unknown): ClipboardWriter {
  return {
    writeText(): Promise<void> {
      return Promise.reject(error);
    },
  };
}

function failure(overrides: Partial<Failure> = {}): Failure {
  return {
    kind: 'unexpected',
    message: '무언가 실패했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

// --- 성공 -----------------------------------------------------------------------------

describe('clipboard에 쓴다', () => {
  it('넘긴 문자열이 그대로 한 번 쓰인다', async () => {
    const writer = writerDouble();

    const result = await copyText('# Molt Note AI Request', writer);

    expect(result).toEqual({ ok: true });
    expect(writer.written).toEqual(['# Molt Note AI Request']);
  });

  it('긴 텍스트도 잘리지 않는다', async () => {
    // 한 시간짜리 전사가 이 경계를 지난다. 여기서 자르면 사용자는 잘린 줄도 모른다.
    const writer = writerDouble();
    const long = 'x'.repeat(500_000);

    await copyText(long, writer);

    expect(writer.written[0]).toHaveLength(500_000);
  });
});

// --- 실패는 값이다 ---------------------------------------------------------------------

describe('실패는 던져지지 않고 값으로 돌아온다 (§13)', () => {
  it('쓸 수단이 없으면 unavailable 실패다', async () => {
    const result = await copyText('text', null);

    expect(result.ok).toBe(false);
    if (result.ok) {
      return;
    }
    expect(clipboardTrouble(result.failure)).toBe('unavailable');
    // §13의 세 질문에 답한다.
    expect(result.failure.message.length).toBeGreaterThan(0);
    expect(result.failure.sourceDataSafe).toBe(true);
    expect(result.failure.retryable).toBe(true);
  });

  it('쓰다 거절되면 rejected 실패이고, 거절한 이름이 detail에 남는다', async () => {
    const denied = new Error('Write permission denied.');
    denied.name = 'NotAllowedError';

    const result = await copyText('text', rejectingWriter(denied));

    expect(result.ok).toBe(false);
    if (result.ok) {
      return;
    }
    expect(clipboardTrouble(result.failure)).toBe('rejected');
    // 어떤 예외가 오는지는 아직 확인되지 않은 사실이다 (ADR-0010 §7.4). 온 이름을 버리지
    // 않아야 나중에 그것을 사실로 적을 수 있다.
    expect(result.failure.detail).toContain('NotAllowedError');
    expect(result.failure.sourceDataSafe).toBe(true);
    expect(result.failure.retryable).toBe(true);
  });

  it('Error가 아닌 값으로 거절돼도 던지지 않는다', async () => {
    for (const thrown of [undefined, null, 'nope', { code: 5 }, 42]) {
      const result = await copyText('text', rejectingWriter(thrown));

      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(clipboardTrouble(result.failure)).toBe('rejected');
      }
    }
  });

  it('동기적으로 던지는 writer에도 예외가 새어 나가지 않는다', async () => {
    const throwing: ClipboardWriter = {
      writeText(): Promise<void> {
        throw new TypeError('not a function');
      },
    };

    const result = await copyText('text', throwing);

    expect(result.ok).toBe(false);
  });

  it('새 FailureKind를 만들지 않는다 (§7.2)', async () => {
    // clipboard 실패는 Rust에서 나지 않는다. 종류를 늘리면 `tests/ipc-boundary.test.ts`의
    // 양방향 검사가 깨진다 — 갈래는 kind가 아니라 detail의 표시가 들고 있다.
    const unavailable = await copyText('text', null);
    const rejected = await copyText('text', rejectingWriter(new Error('no')));

    for (const result of [unavailable, rejected]) {
      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.failure.kind).toBe('unexpected');
        expect(result.failure.detail).toContain(CLIPBOARD_TROUBLE_DETAIL);
      }
    }
  });
});

// --- 능력이 있는지 부르기 전에 본다 ------------------------------------------------------

describe('쓸 수 있는 대상만 writer가 된다', () => {
  it('없거나 쓸 수 없는 값은 writer가 되지 않는다', () => {
    for (const candidate of [null, undefined, {}, { writeText: 'text' }, 7, 'clipboard']) {
      expect(clipboardWriter(candidate)).toBeNull();
    }
  });

  it('쓸 수 있는 값은 원래 객체의 메서드로 불린다', async () => {
    // `this`를 잃으면 실제 브라우저 구현에서 쓰기가 실패한다. double로 그것을 판정한다.
    const api = {
      seen: [] as string[],
      writeText(text: string): Promise<void> {
        this.seen.push(text);
        return Promise.resolve();
      },
    };

    const writer = clipboardWriter(api);
    expect(writer).not.toBeNull();
    await writer?.writeText('hello');

    expect(api.seen).toEqual(['hello']);
  });

  it('약속을 돌려주지 않는 구현도 성공으로 다룬다', async () => {
    // 넘어오는 값이 Promise인지 아닌지는 플랫폼 사실이며 확인되지 않았다 (§7.4).
    const written: string[] = [];
    const writer = clipboardWriter({
      writeText(text: string): void {
        written.push(text);
      },
    });

    expect(await copyText('text', writer)).toEqual({ ok: true });
    expect(written).toEqual(['text']);
  });
});

// --- 실패를 읽는 규칙 -------------------------------------------------------------------

describe('clipboard 실패인지 읽는다', () => {
  it('표시가 없는 실패는 clipboard 실패로 읽지 않는다', () => {
    expect(clipboardTrouble(null)).toBeNull();
    expect(clipboardTrouble(failure())).toBeNull();
    expect(clipboardTrouble(failure({ kind: 'storage', detail: 'recordingId=r-1' }))).toBeNull();
    // 모르는 갈래를 아는 갈래로 접지 않는다.
    expect(clipboardTrouble(failure({ detail: 'clipboard=whatever' }))).toBeNull();
  });

  it('표시가 있으면 갈래를 그대로 돌려준다', () => {
    expect(clipboardTrouble(failure({ detail: 'clipboard=unavailable' }))).toBe('unavailable');
    expect(clipboardTrouble(failure({ detail: 'clipboard=rejected' }))).toBe('rejected');
    expect(clipboardTrouble(failure({ detail: 'clipboard=rejected: NotAllowedError: no' }))).toBe(
      'rejected',
    );
  });
});
