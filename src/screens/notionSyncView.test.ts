// Send to Notion 자리의 상태 (PRODUCT-SPEC §5 C · §7 · §10 · §13 ·
// `phase-prompt/05` 요구 8 · 9 · 12 · 13 · docs/ADR-0009-notion-and-export.md §8).
//
// 실제 Notion도 token도 네트워크도 DOM도 없이 판정한다 (§18).
//
// 이 파일이 보는 것은 넷이다.
//
//   1. 이 녹음의 Notion 상태가 보인다 — 목록과 **같은 규칙으로** 만들어진 값이다 (요구 9)
//   2. 무엇이 전송되는지가 보이고, 오디오는 나가지 않는다 (INV-5 · INV-6 · 요구 13)
//   3. **누르기 전에** 무슨 일이 일어나는지 알 수 있다 — ADR-0009 §8.5의 표 그대로 (요구 8)
//   4. 실패가 §13의 세 질문에 답하고, 부분 전송이 드러나며, 재시도 수단이 있다 (요구 12)
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type {
  NotionSendStatus,
  NotionSync,
  ProcessingStatus,
  Recording,
} from '../ipc/types';
import {
  NEW_PAGE_OUTCOME,
  NOTION_AUDIO_NOTICE,
  NOTION_CONTENTS_HEADLINE,
  NOT_SENT_TEXT,
  RESUME_OUTCOME,
  SEND_FAILED_HEADLINE,
  SEND_LABEL,
  SEND_PRESERVED_NOTICE,
  SENT_HEADLINE,
  UNKNOWN_SEND_NOTICE,
  confirmReason,
  notionPanel,
  notionTrouble,
  type NotionConfirmReason,
  type NotionPanelInput,
} from './notionSyncView';
import { loadedRecordings, statusBadge } from './recordingsView';

// --- 사실들 --------------------------------------------------------------------------

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

function sync(overrides: Partial<NotionSync> = {}): NotionSync {
  return {
    recordingId: 'r-1',
    pageId: null,
    syncedAt: null,
    status: 'failed',
    error: null,
    sentChunks: null,
    totalChunks: null,
    ...overrides,
  };
}

function live(overrides: Partial<NotionSendStatus> = {}): NotionSendStatus {
  return {
    state: 'idle',
    recordingId: null,
    pageId: null,
    createdPage: false,
    failure: null,
    ...overrides,
  };
}

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: 'Notion으로 보내지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

/** Rust가 확인을 물을 때 보내는 실패 그대로 (`sync::run`의 `needs_confirmation`). */
function needsConfirmation(reason: NotionConfirmReason, message: string): Failure {
  return failure('invalidInput', { message, detail: `needsConfirmation=${reason}` });
}

function input(overrides: Partial<NotionPanelInput> = {}): NotionPanelInput {
  return { recording: recording(), sync: null, live: null, ...overrides };
}

const EVERY_STATUS: readonly ProcessingStatus[] = ['none', 'pending', 'running', 'done', 'failed'];

// --- 1. 상태가 보인다 (요구 9) ----------------------------------------------------------

describe('이 녹음의 Notion 상태가 Detail에 보인다 (§7 · 요구 9)', () => {
  it('저장된 다섯 상태가 목록과 같은 문자열로 보인다', () => {
    for (const notionStatus of EVERY_STATUS) {
      const view = notionPanel(input({ recording: recording({ notionStatus }) }));
      const list = loadedRecordings([recording({ notionStatus })], { timeZone: 'UTC' });

      if (list.kind !== 'list') {
        throw new Error('목록이어야 한다');
      }
      // 같은 상태가 목록과 상세에서 다른 말로 읽히면 두 화면이 다른 것을 말하는 것처럼 보인다.
      expect(view.status, `notionStatus=${notionStatus}`).toEqual(list.items[0].statuses[2]);
      expect(view.status?.label).toBe('Notion');
    }
  });

  it('아직 보낸 적이 없다는 것이 오류처럼 적히지 않는다 (INV-8)', () => {
    const view = notionPanel(input());

    expect(view.status).toEqual(statusBadge('Notion', 'none'));
    expect(view.status?.text).not.toMatch(/fail|error/i);
    expect(view.body.kind).toBe('ready');
    if (view.body.kind !== 'ready') {
      throw new Error('보낼 수 있어야 한다');
    }
    expect(view.body.text).toBe(NOT_SENT_TEXT);
  });

  it('레코드를 아직 읽지 못했으면 상태를 지어내지 않는다', () => {
    const view = notionPanel(input({ recording: null }));

    expect(view.body.kind).toBe('loading');
    expect(view.status).toBeNull();
  });

  it('방금 시작한 전송은 저장된 값보다 새롭다', () => {
    // 레코드를 다시 읽기 전이므로 저장된 값은 아직 `none`이다. 그 값이 "보내는 중"을 덮으면
    // 사용자는 자기가 누른 것이 접수됐는지 알 수 없다.
    const view = notionPanel(
      input({ live: live({ state: 'running', recordingId: 'r-1' }) }),
    );

    expect(view.body.kind).toBe('sending');
    expect(view.status?.status).toBe('running');
  });

  it('다른 녹음의 전송은 이 자리에 보이지 않는다', () => {
    const view = notionPanel(input({ live: live({ state: 'running', recordingId: 'r-2' }) }));

    expect(view.body.kind).toBe('ready');
    expect(view.status?.status).toBe('none');
  });

  it('다른 녹음의 전송 기록도 이 자리에 보이지 않는다', () => {
    // 화면이 다른 녹음으로 옮겨 가는 동안 앞 녹음의 페이지 식별자가 잠깐이라도 보이면
    // 그것은 사실이 아닌 것을 보여주는 일이다.
    const view = notionPanel(
      input({
        recording: recording({ notionStatus: 'done' }),
        sync: sync({ recordingId: 'r-2', status: 'done', pageId: 'page-of-another', sentChunks: 5, totalChunks: 5 }),
      }),
    );

    if (view.body.kind !== 'sent') {
      throw new Error('보낸 상태여야 한다');
    }
    expect(view.body.pageId).toBeNull();
    expect(view.body.progress).toBeNull();
  });

  it('보낼 전사가 없는 것은 실패가 아니다 (§7.2)', () => {
    const view = notionPanel(
      input({ recording: recording({ currentTranscriptId: null, transcriptionStatus: 'none' }) }),
    );

    expect(view.body.kind).toBe('nothingToSend');
    expect(view.body).not.toHaveProperty('failure');
  });
});

// --- 2. 무엇이 전송되는가 (INV-5 · INV-6 · 요구 13) -----------------------------------

describe('무엇이 전송되는지 화면에 드러난다 (INV-5 · INV-6)', () => {
  it('나가는 것이 텍스트라는 것과, 오디오는 나가지 않는다는 것이 함께 있다', () => {
    const view = notionPanel(input());

    expect(view.contents.headline).toBe(NOTION_CONTENTS_HEADLINE);
    expect(view.contents.items.join(' · ')).toMatch(/transcript/i);
    expect(view.contents.items.join(' · ')).toMatch(/AI note/i);
    expect(view.contents.audioNotice).toBe(NOTION_AUDIO_NOTICE);
    expect(view.contents.audioNotice).toMatch(/audio file is never sent/i);
  });

  it('오디오 파일 경로가 이 자리의 어떤 값에도 실리지 않는다 (INV-6)', () => {
    // 화면이 오디오를 언급하는 순간 "무엇이 나가는가"의 답이 흔들린다. 값 전체를 훑는다.
    const record = recording();
    const everywhere = JSON.stringify(
      EVERY_STATUS.map((notionStatus) =>
        notionPanel(
          input({
            recording: recording({ notionStatus }),
            sync: sync({ pageId: 'page-1', sentChunks: 2, totalChunks: 5 }),
            live: live({ state: 'failed', recordingId: 'r-1', failure: failure('notionRequestFailed') }),
          }),
        ),
      ),
    );

    expect(everywhere).not.toContain(record.audioPath);
    expect(everywhere).not.toContain('.wav');
  });

  it('본문이 어느 상태든 그 사실은 남는다', () => {
    const bodies: readonly NotionPanelInput[] = [
      input(),
      input({ live: live({ state: 'running', recordingId: 'r-1' }) }),
      input({ recording: recording({ notionStatus: 'done' }), sync: sync({ status: 'done', pageId: 'page-1' }) }),
      input({ live: live({ state: 'failed', recordingId: 'r-1', failure: failure('notionAuthFailed') }) }),
    ];

    for (const one of bodies) {
      expect(notionPanel(one).contents.audioNotice).toBe(NOTION_AUDIO_NOTICE);
    }
  });
});

// --- 3. 누르기 전에 결과를 안다 (ADR-0009 §8.5 · 요구 8) --------------------------------

describe('같은 Recording을 두 번 보내면 무슨 일이 일어나는가 (ADR-0009 §8.3 · §8.5)', () => {
  it('보낸 적이 없으면 새 페이지 하나가 생긴다고 미리 말한다', () => {
    const view = notionPanel(input());

    if (view.body.kind !== 'ready') {
      throw new Error('보낼 수 있어야 한다');
    }
    expect(view.body.send.label).toBe(SEND_LABEL);
    expect(view.body.send.outcomeText).toBe(NEW_PAGE_OUTCOME);
    // 확인이 필요 없는 갈래다 — 앱이 대신 고른 것이 아니라 만들 페이지가 없기 때문이다.
    expect(view.body.send.confirmation).toBe('notAsked');
  });

  it('이미 보낸 녹음은 **누르기 전에** 새 페이지가 생긴다는 것과 기존 페이지가 남는다는 것을 말한다', () => {
    const view = notionPanel(
      input({
        recording: recording({ notionStatus: 'done' }),
        sync: sync({
          status: 'done',
          pageId: 'page-1',
          syncedAt: '2026-09-04T02:00:00.000Z',
          sentChunks: 5,
          totalChunks: 5,
        }),
      }),
    );

    expect(view.body.kind).toBe('sent');
    if (view.body.kind !== 'sent') {
      throw new Error('이미 보낸 상태여야 한다');
    }
    expect(view.body.headline).toBe(SENT_HEADLINE);
    expect(view.body.pageId).toBe('page-1');
    expect(view.body.syncedAt).toBe('2026-09-04T02:00:00.000Z');
    // ADR §8.3 — 명시적 중복 생성이며, 기존 페이지는 건드리지 않는다.
    expect(view.body.again.kind).toBe('newPage');
    expect(view.body.again.confirmation).toBe('newPage');
    expect(view.body.again.outcomeText).toMatch(/leaves that page exactly as it is/i);
    expect(view.body.again.outcomeText).toMatch(/nothing there is changed or deleted/i);
    // 기존 페이지를 갈아 끼우는 갈래는 ADR-0009 §8.3이 거절했다. 그런 말이 여기 없다.
    expect(view.body.again.outcomeText).not.toMatch(/overwrit|replaces/i);
  });

  it('끝나지 않은 전송은 같은 페이지에 이어 보낸다 — 중복 페이지를 만들지 않는다 (§8.2)', () => {
    const view = notionPanel(
      input({
        recording: recording({ notionStatus: 'failed' }),
        sync: sync({ pageId: 'page-1', sentChunks: 3, totalChunks: 7 }),
      }),
    );

    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.retry.kind).toBe('resume');
    expect(view.body.retry.confirmation).toBe('notAsked');
    expect(view.body.retry.outcomeText).toBe(RESUME_OUTCOME);
    expect(view.body.retry.outcomeText).toMatch(/does not create a second page/i);
  });

  it('페이지가 만들어졌는지 모르는 실패는 그대로 다시 보내지 않고 먼저 묻는다고 말한다', () => {
    const view = notionPanel(
      input({
        recording: recording({ notionStatus: 'failed' }),
        sync: sync({ pageId: null, sentChunks: 0, totalChunks: 7 }),
      }),
    );

    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.retry.kind).toBe('retry');
    expect(view.body.retry.confirmation).toBe('notAsked');
    expect(view.body.retry.outcomeText).toMatch(/asks you instead of making a second one/i);
  });

  it('확인을 물어 온 것은 실패가 아니라 고를 차례다 — 세 이유가 갈린다 (§8.5)', () => {
    const reasons: readonly (readonly [NotionConfirmReason, string])[] = [
      ['alreadySent', '이 녹음은 이미 Notion 페이지가 있다.'],
      ['documentChanged', '마지막으로 보낸 뒤 내용이 바뀌었다.'],
      ['outcomeUnknown', '지난번 전송에서 Notion 페이지가 만들어졌는지 확인하지 못했다.'],
    ];

    for (const [reason, message] of reasons) {
      const view = notionPanel(
        input({
          live: live({
            state: 'failed',
            recordingId: 'r-1',
            failure: needsConfirmation(reason, message),
          }),
        }),
      );

      expect(view.body.kind, reason).toBe('needsConfirmation');
      if (view.body.kind !== 'needsConfirmation') {
        throw new Error('확인을 묻는 상태여야 한다');
      }
      expect(view.body.reason, reason).toBe(reason);
      // Rust가 보낸 문장을 그대로 쓴다 — 같은 사실을 두 벌로 적지 않는다.
      expect(view.body.text, reason).toBe(message);
      // 아무것도 하지 않고 거절된 요청이다 — 상태 표시도 실패로 바뀌지 않는다.
      expect(view.body.preservedNotice, reason).toBe(SEND_PRESERVED_NOTICE);
      expect(view.status?.status, reason).toBe('none');
      // 새 페이지는 **이 버튼으로만** 만들어진다.
      expect(view.body.confirm.confirmation, reason).toBe('newPage');
      expect(view.body.confirm.label, reason).toBe('Create a new Notion page');
    }
  });

  it('확인을 싣는 동작은 그 하나뿐이다 — 조용한 중복이 생길 통로가 없다', () => {
    const everyAction = [
      notionPanel(input()),
      notionPanel(
        input({
          recording: recording({ notionStatus: 'failed' }),
          sync: sync({ pageId: 'page-1', sentChunks: 3, totalChunks: 7 }),
        }),
      ),
      notionPanel(
        input({
          recording: recording({ notionStatus: 'failed' }),
          sync: sync({ pageId: null }),
        }),
      ),
    ];

    for (const view of everyAction) {
      const action =
        view.body.kind === 'ready'
          ? view.body.send
          : view.body.kind === 'failed'
            ? view.body.retry
            : null;
      expect(action?.confirmation).toBe('notAsked');
    }
  });

  it('모르는 확인 사유를 확인 요청으로 읽지 않는다', () => {
    expect(confirmReason(failure('invalidInput', { detail: 'needsConfirmation=whatever' }))).toBeNull();
    expect(confirmReason(failure('invalidInput', { detail: 'confirmation=newPage' }))).toBeNull();
    expect(confirmReason(failure('notionAuthFailed'))).toBeNull();
    expect(confirmReason(null)).toBeNull();
  });
});

// --- 4. 실패 (§13 · INV-3 · 요구 12) ----------------------------------------------------

describe('전송 실패가 화면에 남는 방식 (§13 · INV-3)', () => {
  /** §13이 서로 다른 제품 상태로 구분하는 실패들. 사용자가 할 일이 전부 다르다. */
  const FAILURES: readonly (readonly [string, FailureKind, string])[] = [
    ['인증 실패', 'notionAuthFailed', 'auth'],
    ['권한 없는 destination', 'notionDestinationUnavailable', 'destination'],
    ['속도 제한', 'notionRateLimited', 'rateLimited'],
    ['네트워크 없음', 'notionRequestFailed', 'requestFailed'],
    ['결과를 모름', 'notionResponseUnusable', 'outcomeUnknown'],
  ];

  it('다섯 실패 각각이 §13의 세 질문에 답하고 재시도 수단을 남긴다', () => {
    for (const [label, kind, cause] of FAILURES) {
      const view = notionPanel(
        input({ live: live({ state: 'failed', recordingId: 'r-1', failure: failure(kind) }) }),
      );

      expect(view.body.kind, label).toBe('failed');
      if (view.body.kind !== 'failed') {
        throw new Error('실패 상태여야 한다');
      }

      // 1. 무엇이 실패했는가 — Rust가 나눠 보낸 구분이 뭉개지지 않는다.
      expect(view.body.headline, label).toBe(SEND_FAILED_HEADLINE);
      expect(view.body.failure?.kind, label).toBe(kind);
      expect(view.body.cause, label).toBe(cause);
      // 사용자가 먼저 할 일이 갈래마다 다르다.
      expect(view.body.resolution, label).not.toBeNull();

      // 2. 원본은 안전한가.
      expect(view.body.preservedNotice, label).toBe(SEND_PRESERVED_NOTICE);
      expect(view.body.preservedNotice, label).toMatch(/untouched/i);
      expect(view.body.preservedNotice, label).toMatch(/already in Notion is left as it is/i);

      // 3. 다시 시도할 수 있는가.
      expect(view.body.retry.recordingId, label).toBe('r-1');
      expect(view.status?.status, label).toBe('failed');
    }
  });

  it('실패한 갈래마다 안내가 서로 다르다', () => {
    const resolutions = FAILURES.map(([, kind]) => {
      const view = notionPanel(
        input({ live: live({ state: 'failed', recordingId: 'r-1', failure: failure(kind) }) }),
      );
      return view.body.kind === 'failed' ? view.body.resolution : null;
    });

    expect(new Set(resolutions).size).toBe(FAILURES.length);
  });

  it('부분 전송 뒤의 실패는 어디까지 갔는지를 드러낸다 (§8.4 · 요구 12)', () => {
    const view = notionPanel(
      input({
        recording: recording({ notionStatus: 'failed' }),
        sync: sync({ pageId: 'page-1', sentChunks: 3, totalChunks: 7 }),
        live: live({ state: 'failed', recordingId: 'r-1', failure: failure('notionRequestFailed') }),
      }),
    );

    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.progress).not.toBeNull();
    expect(view.body.progress?.complete).toBe(false);
    expect(view.body.progress?.text).toBe('3 of 7 parts of this document are already on that page.');
  });

  it('기록되기 전에 실패한 것은 숫자를 지어내지 않는다', () => {
    const view = notionPanel(
      input({
        recording: recording({ notionStatus: 'failed' }),
        sync: sync({ pageId: null, sentChunks: null, totalChunks: null }),
      }),
    );

    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.progress).toBeNull();
  });

  it('앱을 다시 켠 뒤 이유를 모르는 실패는 이유를 지어내지 않는다', () => {
    const view = notionPanel(input({ recording: recording({ notionStatus: 'failed' }), live: null }));

    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.failure).toBeNull();
    expect(view.body.cause).toBe('unknown');
    expect(view.body.resolution).toBe(UNKNOWN_SEND_NOTICE);
    expect(view.body.retry.recordingId).toBe('r-1');
  });

  it('거절된 요청은 전송 상태를 덮지 않고 그 옆에 남는다', () => {
    const trouble = notionTrouble('start', failure('invalidInput', { message: '이미 보내는 중이다.' }));

    expect(trouble.headline).toMatch(/could not be started/i);
    expect(trouble.failure.message).toBe('이미 보내는 중이다.');
  });
});

// --- 5. 성공 ---------------------------------------------------------------------------

describe('전송이 끝났을 때', () => {
  it('이어 보내 끝낸 것을 "새 페이지를 만들었다"고 말하지 않는다 (§8.2)', () => {
    const resumed = notionPanel(
      input({
        live: live({ state: 'done', recordingId: 'r-1', pageId: 'page-1', createdPage: false }),
        sync: sync({ status: 'done', pageId: 'page-1', sentChunks: 7, totalChunks: 7 }),
      }),
    );
    const created = notionPanel(
      input({
        live: live({ state: 'done', recordingId: 'r-1', pageId: 'page-2', createdPage: true }),
      }),
    );

    if (resumed.body.kind !== 'sent' || created.body.kind !== 'sent') {
      throw new Error('보낸 상태여야 한다');
    }
    expect(resumed.body.createdPage).toBe(false);
    expect(created.body.createdPage).toBe(true);
    expect(resumed.body.progress?.complete).toBe(true);
    expect(resumed.body.progress?.text).toBe('All 7 parts of this document are on that page.');
    expect(resumed.status?.status).toBe('done');
  });
});
