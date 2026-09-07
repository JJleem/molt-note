// Copy AI Prompt · Copy Transcript 두 자리의 상태 (PRODUCT-SPEC §13 ·
// `phase-prompt/05.5` 요구 A-1 · A-2 · 5 · docs/ADR-0010-manual-ai-handoff.md §7.5).
//
// **clipboard 없이 네 갈래를 전부 판정한다** — 아직 안 함 · 복사 중 · 복사됨 · 실패.
// 이 파일은 DOM도 Tauri도 clipboard도 부르지 않는다. 값만 본다 (§18).
//
// 여기 모여 있는 것 넷:
//
//   1. 네 갈래가 각각 무엇을 보여주는가                      (요구 5 · §7.5)
//   2. 성공도 실패도 **문장으로 말한다** — 색만으로 말하지 않는다 (§7.5)
//   3. 실패에는 재시도와 **Export for AI라는 다른 길**이 언제나 있다 (§7.5 · §13)
//   4. AI provider가 하나도 없어도 이 자리는 그대로 동작한다   (MH-1 · MH-2 · INV-8)
//   5. **얼마나 크고 몇 번째를 가져갔는가**가 값으로 있다      (`phase-prompt/05.6` 성공 기준 4)
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type { HandoffText, NoteMode, ProcessingStatus, Recording, TextSize } from '../ipc/types';
import {
  COPIED_HEADLINE,
  COPIED_PORTION_HEADLINE,
  COPY_ALTERNATIVE_LABEL,
  COPY_PRESERVED_NOTICE,
  COPY_PROMPT_LABEL,
  COPY_TRANSCRIPT_LABEL,
  NO_COPY_ATTEMPT,
  NOTHING_TO_COPY_HINT,
  TOO_LONG_NOTICE,
  WHOLE_PORTION_LABEL,
  copiedText,
  copyPanel,
  failedCopy,
  handoffSize,
  portionTaken,
  startedCopy,
  type CopyBody,
  type CopyPanelInput,
  type CopyTarget,
} from './copyView';

// --- 사실들 --------------------------------------------------------------------------

function size(chars: number, lines: number): TextSize {
  // 한글은 한 글자가 3바이트다 — 바이트와 글자 수는 같지 않다.
  return { bytes: chars * 3, chars, lines };
}

/**
 * backend가 돌려준 한 조각.
 *
 * 기본값은 **나뉘지 않은 산출물**이다 — 짧은 녹음에서 늘 그렇고, 그때의 화면이 이 Phase
 * 이전과 달라지지 않아야 한다.
 */
function taken(overrides: Partial<HandoffText> = {}): HandoffText {
  return {
    recordingId: 'r-1',
    text: '## Transcript\n00:00:03 안녕하세요.\n',
    totalSize: size(32, 2),
    portion: 1,
    portionCount: 1,
    portionSize: size(32, 2),
    ...overrides,
  };
}

/** 예산을 넘어 넷으로 나뉜 산출물의 `index`번째 조각. */
function part(index: number, count = 4): HandoffText {
  return taken({
    totalSize: size(41_320, 1_711),
    portion: index,
    portionCount: count,
    portionSize: size(12_000, 430),
  });
}

function recording(overrides: Partial<Recording> = {}): Recording {
  return {
    id: 'r-1',
    title: '3DGS Study #04',
    createdAt: '2026-09-01T09:12:00.000Z',
    updatedAt: '2026-09-01T09:12:00.000Z',
    durationMs: 3_151_000,
    durationLabel: '52:31',
    audioPath: '/recordings/r-1.wav',
    audioFormat: 'wav',
    microphone: 'MacBook Microphone',
    currentTranscriptId: 't-1',
    transcriptionStatus: 'done',
    aiStatus: 'none',
    notionStatus: 'none',
    ...overrides,
  };
}

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: '복사하지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

function input(overrides: Partial<CopyPanelInput> = {}): CopyPanelInput {
  return {
    recording: recording(),
    mode: 'meeting',
    attempt: NO_COPY_ATTEMPT,
    ...overrides,
  };
}

const TARGETS: readonly CopyTarget[] = ['prompt', 'transcript'];

/** 두 자리 중 하나를 꺼낸다. */
function bodyOf(view: ReturnType<typeof copyPanel>, target: CopyTarget): CopyBody {
  return target === 'prompt' ? view.prompt.body : view.transcript.body;
}

// --- 네 갈래 -------------------------------------------------------------------------

describe('복사 동작이 놓이는 네 갈래', () => {
  it('아직 아무것도 하지 않았으면 복사할 수 있다고 말한다', () => {
    for (const target of TARGETS) {
      const body = bodyOf(copyPanel(input()), target);

      expect(body.kind).toBe('notAsked');
      if (body.kind !== 'notAsked') {
        continue;
      }
      expect(body.text.length).toBeGreaterThan(0);
      expect(body.start.kind).toBe('copy');
      expect(body.start.recordingId).toBe('r-1');
    }
  });

  it('복사 중이면 그 사실을 말한다', () => {
    for (const target of TARGETS) {
      const body = bodyOf(copyPanel(input({ attempt: startedCopy(target, 'r-1', 1) })), target);

      expect(body.kind).toBe('copying');
      if (body.kind === 'copying') {
        expect(body.text.length).toBeGreaterThan(0);
      }
    }
  });

  it('복사됐으면 그 사실을 문장으로 말하고 또 복사할 수 있다', () => {
    for (const target of TARGETS) {
      const body = bodyOf(copyPanel(input({ attempt: copiedText(target, 'r-1', taken()) })), target);

      expect(body.kind).toBe('copied');
      if (body.kind !== 'copied') {
        continue;
      }
      // 색이 아니라 이 문장이 복사됐다는 것을 말한다 (§7.5).
      expect(body.headline).toBe(COPIED_HEADLINE);
      expect(body.text.length).toBeGreaterThan(0);
      expect(body.again.kind).toBe('again');
      expect(body.again.label.length).toBeGreaterThan(0);
      // 나뉘지 않았으면 가져갈 것이 더 없다 — 없는 조각을 가리키는 버튼을 만들지 않는다.
      expect(body.next).toBeNull();
      expect(body.portion.whole).toBe(true);
    }
  });

  it('실패하면 무엇이 실패했는지 · 원본이 안전한지 · 다시 할 수 있는지가 남는다', () => {
    for (const target of TARGETS) {
      const denied = failure('unexpected', {
        message: '클립보드에 쓰지 못했다.',
        detail: 'clipboard=rejected: NotAllowedError: denied',
      });
      const body = bodyOf(
        copyPanel(input({ attempt: failedCopy(target, 'r-1', 1, denied) })),
        target,
      );

      expect(body.kind).toBe('failed');
      if (body.kind !== 'failed') {
        continue;
      }
      // §13의 세 질문.
      expect(body.headline.length).toBeGreaterThan(0);
      expect(body.failure.message).toBe('클립보드에 쓰지 못했다.');
      expect(body.preservedNotice).toBe(COPY_PRESERVED_NOTICE);
      expect(body.failure.sourceDataSafe).toBe(true);
      expect(body.retry.kind).toBe('retry');
      expect(body.failure.retryable).toBe(true);
    }
  });

  it('복사할 전사가 없으면 실패가 아니라 그 사실을 말한다', () => {
    const view = copyPanel(input({ recording: recording({ currentTranscriptId: null }) }));

    for (const target of TARGETS) {
      const body = bodyOf(view, target);
      expect(body.kind).toBe('nothingToCopy');
      if (body.kind === 'nothingToCopy') {
        expect(body.hint).toBe(NOTHING_TO_COPY_HINT);
      }
    }
  });

  it('레코드를 아직 읽지 못했으면 판단하지 않는다', () => {
    const view = copyPanel(input({ recording: null }));

    expect(view.prompt.body.kind).toBe('loading');
    expect(view.transcript.body.kind).toBe('loading');
  });
});

// --- 상태가 서로를 가리지 않는다 --------------------------------------------------------

describe('한 자리의 상태가 다른 자리를 가리지 않는다', () => {
  it('프롬프트 복사가 실패해도 전사 복사는 그대로 누를 수 있다', () => {
    const view = copyPanel(
      input({ attempt: failedCopy('prompt', 'r-1', 1, failure('unexpected')) }),
    );

    expect(view.prompt.body.kind).toBe('failed');
    expect(view.transcript.body.kind).toBe('notAsked');
  });

  it('다른 녹음의 복사 결과는 이 자리에 보이지 않는다', () => {
    const view = copyPanel(input({ attempt: copiedText('prompt', 'r-2', taken({ recordingId: 'r-2' })) }));

    expect(view.prompt.body.kind).toBe('notAsked');
  });

  it('버튼 이름은 상태와 무관하게 그 자리가 무엇인지 말한다', () => {
    const view = copyPanel(input({ attempt: startedCopy('prompt', 'r-1', 1) }));

    expect(view.prompt.label).toBe(COPY_PROMPT_LABEL);
    expect(view.transcript.label).toBe(COPY_TRANSCRIPT_LABEL);
  });
});

// --- 실패 갈래 -----------------------------------------------------------------------

describe('실패 갈래마다 먼저 할 일이 다르다', () => {
  const CAUSES: readonly { readonly failure: Failure; readonly cause: string }[] = [
    {
      failure: failure('unexpected', { detail: 'clipboard=unavailable' }),
      cause: 'clipboardUnavailable',
    },
    {
      failure: failure('unexpected', { detail: 'clipboard=rejected: NotAllowedError: x' }),
      cause: 'clipboardRejected',
    },
    { failure: failure('invalidInput'), cause: 'nothingToCopy' },
    { failure: failure('storage'), cause: 'storage' },
    { failure: failure('aiProviderUnreachable'), cause: 'other' },
  ];

  it('갈래를 뭉개지 않는다', () => {
    for (const { failure: cause, cause: expected } of CAUSES) {
      const body = bodyOf(copyPanel(input({ attempt: failedCopy('prompt', 'r-1', 1, cause) })), 'prompt');

      expect(body.kind).toBe('failed');
      if (body.kind === 'failed') {
        expect(body.cause).toBe(expected);
      }
    }
  });

  it('clipboard 갈래에는 먼저 할 일이 문장으로 있다', () => {
    for (const { failure: cause } of CAUSES.slice(0, 2)) {
      const body = bodyOf(copyPanel(input({ attempt: failedCopy('prompt', 'r-1', 1, cause) })), 'prompt');

      if (body.kind === 'failed') {
        expect(body.resolution).not.toBeNull();
      }
    }
  });

  it('이유를 모르는 실패에 이유를 지어내지 않는다', () => {
    const body = bodyOf(
      copyPanel(input({ attempt: failedCopy('prompt', 'r-1', 1, failure('aiRequestFailed')) })),
      'prompt',
    );

    if (body.kind === 'failed') {
      expect(body.cause).toBe('other');
      expect(body.resolution).toBeNull();
    }
  });

  it('구조화되지 않은 값으로 실패해도 화면에 보여줄 수 있는 실패가 된다', () => {
    // 복사 경로에서 무엇이 던져지든 console로 끝나지 않는다 (§13).
    const body = bodyOf(
      copyPanel(input({ attempt: failedCopy('transcript', 'r-1', 1, new Error('boom')) })),
      'transcript',
    );

    expect(body.kind).toBe('failed');
    if (body.kind === 'failed') {
      expect(body.failure.kind).toBe('unexpected');
      expect(body.failure.message.length).toBeGreaterThan(0);
      expect(body.failure.detail).toBe('boom');
    }
  });
});

// --- 막혀도 다른 길이 있다 --------------------------------------------------------------

describe('clipboard가 막혀도 목적은 달성된다 (§7.5)', () => {
  it('어느 실패에도 Export for AI가 값으로 남는다', () => {
    const failures = [
      failure('unexpected', { detail: 'clipboard=unavailable' }),
      failure('unexpected', { detail: 'clipboard=rejected' }),
      failure('storage'),
    ];

    for (const target of TARGETS) {
      for (const cause of failures) {
        const body = bodyOf(copyPanel(input({ attempt: failedCopy(target, 'r-1', 1, cause) })), target);

        expect(body.kind).toBe('failed');
        if (body.kind !== 'failed') {
          continue;
        }
        expect(body.alternative.label).toBe(COPY_ALTERNATIVE_LABEL);
        expect(body.alternative.text.length).toBeGreaterThan(0);
      }
    }
  });
});

// --- 색만으로 말하지 않는다 -------------------------------------------------------------

describe('상태는 언제나 문장을 동반한다 (§7.5)', () => {
  it('여섯 갈래 전부가 읽을 수 있는 문장을 들고 있다', () => {
    const bodies: CopyBody[] = [
      bodyOf(copyPanel(input({ recording: null })), 'prompt'),
      bodyOf(copyPanel(input({ recording: recording({ currentTranscriptId: null }) })), 'prompt'),
      bodyOf(copyPanel(input()), 'prompt'),
      bodyOf(copyPanel(input({ attempt: startedCopy('prompt', 'r-1', 1) })), 'prompt'),
      bodyOf(copyPanel(input({ attempt: copiedText('prompt', 'r-1', taken()) })), 'prompt'),
      bodyOf(
        copyPanel(input({ attempt: failedCopy('prompt', 'r-1', 1, failure('unexpected')) })),
        'prompt',
      ),
    ];

    expect(bodies.map((body) => body.kind)).toEqual([
      'loading',
      'nothingToCopy',
      'notAsked',
      'copying',
      'copied',
      'failed',
    ]);

    for (const body of bodies) {
      if (body.kind === 'loading') {
        // 아직 아무 사실도 없다. 없는 문장을 지어내지 않는다.
        continue;
      }
      const sentence = 'text' in body ? body.text : body.headline;
      expect(sentence.length, `${body.kind}에 문장이 없다`).toBeGreaterThan(0);
    }
  });
});

// --- 크기와 나눔 (`phase-prompt/05.6` 성공 기준 4 · R-5) ---------------------------------

describe('크기 때문에 조용히 실패하지 않는다', () => {
  it('크기가 값으로 있고, 한 번에 들어가는지도 값으로 있다', () => {
    const whole = handoffSize(taken());
    const big = handoffSize(part(1));

    expect(whole.characters).toBe(32);
    expect(whole.lines).toBe(2);
    expect(whole.fitsInOne).toBe(true);
    // 들어가는 크기에 "너무 길다"고 말하지 않는다.
    expect(whole.tooLongNotice).toBeNull();

    expect(big.characters).toBe(41_320);
    expect(big.lines).toBe(1_711);
    expect(big.fitsInOne).toBe(false);
    expect(big.tooLongNotice).toBe(TOO_LONG_NOTICE);
    // 사람이 읽는 한 줄에 실제 숫자가 들어 있다 — 크기를 말하지 않는 문장이 아니다.
    expect(big.label).toContain('41,320');
    expect(big.label).toContain('1,711');
  });

  it('몇 번째 중 몇 번째인가가 값으로 있다', () => {
    expect(portionTaken(taken())).toEqual({
      index: 1,
      count: 1,
      whole: true,
      label: WHOLE_PORTION_LABEL,
      remaining: 0,
    });

    expect(portionTaken(part(2))).toEqual({
      index: 2,
      count: 4,
      whole: false,
      label: 'Part 2 of 4.',
      remaining: 2,
    });
  });

  it('조각 하나만 복사됐으면 "복사됐다"고만 말하지 않는다', () => {
    const body = bodyOf(copyPanel(input({ attempt: copiedText('prompt', 'r-1', part(1)) })), 'prompt');

    expect(body.kind).toBe('copied');
    if (body.kind !== 'copied') {
      return;
    }
    // 잘린 결과를 온전한 것이라고 말하는 상태가 없다 (성공 기준 4).
    expect(body.headline).toBe(COPIED_PORTION_HEADLINE);
    expect(body.headline).not.toBe(COPIED_HEADLINE);
    expect(body.portion.label).toBe('Part 1 of 4.');
    expect(body.size.tooLongNotice).toBe(TOO_LONG_NOTICE);
  });

  it('나머지를 마저 가져가는 수단이 그 자리에 있고, 다음 조각을 가리킨다', () => {
    const body = bodyOf(copyPanel(input({ attempt: copiedText('prompt', 'r-1', part(2)) })), 'prompt');

    expect(body.kind).toBe('copied');
    if (body.kind !== 'copied') {
      return;
    }
    expect(body.next).not.toBeNull();
    expect(body.next?.kind).toBe('next');
    expect(body.next?.portion).toBe(3);
    expect(body.next?.recordingId).toBe('r-1');
    // 버튼 이름이 몇 번째를 가져오는지 말한다 — 누르기 전에 알 수 있다.
    expect(body.next?.label).toBe('Copy part 3 of 4');
    // 같은 조각을 다시 가져가는 수단은 그대로 있다.
    expect(body.again.portion).toBe(2);
  });

  it('마지막 조각에서는 다음이 없고, 거기서만 전부 가져갔다고 말한다', () => {
    const body = bodyOf(copyPanel(input({ attempt: copiedText('prompt', 'r-1', part(4)) })), 'prompt');

    expect(body.kind).toBe('copied');
    if (body.kind !== 'copied') {
      return;
    }
    expect(body.next).toBeNull();
    expect(body.portion.remaining).toBe(0);
    expect(body.text).toContain('last part');
  });

  it('처음 누르는 것은 언제나 첫 조각이고, 실패하면 그 조각으로 돌아간다', () => {
    const start = bodyOf(copyPanel(input()), 'prompt');
    expect(start.kind === 'notAsked' && start.start.portion).toBe(1);

    // 세 번째 조각을 가져오다 실패했으면 재시도는 세 번째다 — 건너뛴 조각은 사용자가
    // 알아채지 못한 채 빠진다.
    const failed = bodyOf(
      copyPanel(input({ attempt: failedCopy('prompt', 'r-1', 3, failure('storage')) })),
      'prompt',
    );
    expect(failed.kind === 'failed' && failed.retry.portion).toBe(3);
  });
});

// --- provider가 없어도 그대로다 ---------------------------------------------------------

describe('MH-1 · MH-2 — provider가 하나도 없어도 복사는 그대로 가능하다', () => {
  it('이 모듈의 입력에 provider를 담을 자리가 없다', () => {
    // 입력에 provider가 없으므로 "provider가 없어서 복사할 수 없다"는 상태를 만들 수단 자체가
    // 없다 (INV-8). 모듈이 안에서 몰래 provider를 보지 않는다는 것은 원문을 읽는 검사가
    // 확인한다 (`tests/screen-boundary.test.ts`) — `src/`는 브라우저 코드로 타입 검사되므로
    // 이 파일에서는 node:fs를 쓸 수 없다.
    const keys = Object.keys(input()).sort();
    expect(keys).toEqual(['attempt', 'mode', 'recording']);
  });

  it('저장된 AI 상태가 무엇이든 복사 동작이 글자 하나 다르지 않다', () => {
    const statuses: readonly ProcessingStatus[] = ['none', 'pending', 'running', 'done', 'failed'];
    const first = copyPanel(input({ recording: recording({ aiStatus: 'none' }) }));

    for (const aiStatus of statuses) {
      expect(copyPanel(input({ recording: recording({ aiStatus }) }))).toEqual(first);
    }
  });

  it('mode는 프롬프트 복사에만 실린다', () => {
    const modes: readonly NoteMode[] = ['meeting', 'study', 'summary'];

    for (const mode of modes) {
      const view = copyPanel(input({ mode }));
      const prompt = view.prompt.body;
      const transcript = view.transcript.body;

      if (prompt.kind === 'notAsked') {
        expect(prompt.start.mode).toBe(mode);
      }
      if (transcript.kind === 'notAsked') {
        // 전사 복사는 mode를 쓰지 않는다. 없는 것을 있는 것처럼 싣지 않는다.
        expect(transcript.start.mode).toBeNull();
      }
    }
  });
});
