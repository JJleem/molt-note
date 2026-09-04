// Settings 화면 Notion 구역의 판단 규칙 테스트 (§5-D · §13 · INV-7 · INV-8).
//
// 여기서 보는 것은 넷이다 — **연결 확인의 네 결과가 서로 구분되어 표현되는가** ·
// **성공이 어느 워크스페이스인지 말하는가** · **설정하지 않은 상태가 오류로 그려지지 않는가** ·
// **token 값이 화면 상태에 담길 자리가 없는가**.
//
// **실제 Notion 워크스페이스도, 실제 OS 자격증명 저장소도, DOM도 Tauri도 필요하지 않다**
// (PRODUCT-SPEC §18). 확인의 결과는 값으로 들어오고, 이 모듈은 그 값을 화면 상태로 옮길 뿐이다.
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type { NotionConnection, NotionTokenStatus } from '../ipc/types';
import {
  CHECKING_NOTION_TEXT,
  CONNECTED_TEXT,
  HOW_TO_SET_A_DESTINATION,
  NO_DESTINATION_TEXT,
  NO_TOKEN_TO_CHECK_TEXT,
  NOTION_CHECK_FAILED_TEXT,
  NOTION_CHECK_USES_SAVED_SETTINGS,
  NOTION_NOT_CHECKED_TEXT,
  NOTION_SETTINGS_UNAFFECTED_NOTICE,
  TOKEN_INPUT_NOTICE,
  TOKEN_NOT_STORED_RESOLUTION,
  TOKEN_NOT_STORED_TEXT,
  TOKEN_STORED_TEXT,
  checkedNotionConnection,
  connectedToWorkspaceText,
  failedNotionCheck,
  notionCheckCause,
  notionDestinationChanged,
  notionDestinationNotice,
  notionTokenNotice,
  notionTokenState,
  notionTokenTrouble,
  tokenStateOf,
  type NotionCheckCause,
  type NotionConnectionView,
} from './notionSettings';

// --- 사실들 --------------------------------------------------------------------------

/**
 * 이 파일 어디에도 실제 token은 없다. 이 문자열은 **입력에조차 들어가지 않으며**, 화면 상태에
 * 값이 새어 나갈 자리가 있는지 확인할 때 "찾을 것"으로만 쓰인다.
 */
const NOT_A_REAL_TOKEN = 'ntn_this-is-not-a-real-token';

function connection(overrides: Partial<NotionConnection> = {}): NotionConnection {
  return {
    state: 'connected',
    tokenStored: true,
    destinationConfigured: true,
    workspaceName: null,
    failure: null,
    ...overrides,
  };
}

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: '무엇이 실패했는지 backend가 만든 문장.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

function rejected(kind: FailureKind): NotionConnectionView {
  return checkedNotionConnection(connection({ state: 'failed', failure: failure(kind) }));
}

function tokenStatus(stored: boolean): NotionTokenStatus {
  return { stored };
}

// --- 연결 확인 -----------------------------------------------------------------------

describe('연결 확인의 결과가 갈래마다 다르게 보인다 (§5-D · §13)', () => {
  it('성공이 어느 워크스페이스에 연결됐는지 말한다', () => {
    const view = checkedNotionConnection(
      connection({ workspaceName: 'Ada의 워크스페이스' }),
    );

    expect(view.kind).toBe('connected');
    expect(view.text).toBe(connectedToWorkspaceText('Ada의 워크스페이스'));
    expect(view.text).toContain('Ada의 워크스페이스');
  });

  it('Notion이 워크스페이스 이름을 말하지 않으면 이름을 지어내지 않는다', () => {
    const view = checkedNotionConnection(connection({ workspaceName: null }));

    expect(view).toEqual({
      kind: 'connected',
      text: CONNECTED_TEXT,
      workspaceName: null,
      destinationNotice: null,
    });
  });

  it('연결됐어도 보낼 부모 페이지가 없으면 그 사실이 함께 보인다', () => {
    // 연결됐다는 것과 보낼 자리가 있다는 것은 다른 사실이다. 하나로 뭉치면 사용자는 아직
    // 보낼 수 없다는 것을 누른 뒤에야 알게 된다.
    const view = checkedNotionConnection(connection({ destinationConfigured: false }));

    expect(view.kind).toBe('connected');
    expect(view.kind === 'connected' && view.destinationNotice).toBe(NO_DESTINATION_TEXT);
  });

  it('네 결과 — 성공 · 인증 실패 · 권한 없는 destination · 네트워크 없음 — 이 서로 다른 문장이다', () => {
    const connected = checkedNotionConnection(connection({ workspaceName: '워크스페이스' }));
    const auth = rejected('notionAuthFailed');
    const destination = rejected('notionDestinationUnavailable');
    const offline = rejected('notionRequestFailed');

    expect([auth.kind, destination.kind, offline.kind]).toEqual(['failed', 'failed', 'failed']);
    expect([
      auth.kind === 'failed' && auth.cause,
      destination.kind === 'failed' && destination.cause,
      offline.kind === 'failed' && offline.cause,
    ]).toEqual<NotionCheckCause[]>(['auth', 'destination', 'offline']);

    // 화면에 실제로 보이는 문장 여덟(문제 넷 + 할 일 넷)이 전부 다르다. 갈래만 다르고 같은
    // 말을 하면 사용자에게는 뭉친 것과 같다.
    const sentences = [connected, auth, destination, offline].flatMap((view) => [
      view.text,
      view.kind === 'failed' ? view.resolution : '',
    ]);
    expect(new Set(sentences).size).toBe(sentences.length);
  });

  it('갈래마다 다음에 할 일이 다르다', () => {
    const auth = rejected('notionAuthFailed');
    const destination = rejected('notionDestinationUnavailable');
    const offline = rejected('notionRequestFailed');

    expect(auth.kind === 'failed' && auth.resolution).toMatch(/token/i);
    expect(destination.kind === 'failed' && destination.resolution).toMatch(/share|parent page/i);
    expect(offline.kind === 'failed' && offline.resolution).toMatch(/online/i);
  });

  it('속도 제한과 그 밖의 실패도 인증 실패로 읽히지 않는다', () => {
    expect(notionCheckCause(failure('notionRateLimited'))).toBe('rateLimited');
    expect(notionCheckCause(failure('notionResponseUnusable'))).toBe('other');
    expect(notionCheckCause(failure('storage'))).toBe('other');
    expect(notionCheckCause(null)).toBe('other');
  });

  it('실패한 확인은 무엇이 실패했는지 · 원본이 안전한지 · 다시 할 수 있는지를 그대로 들고 온다 (§13)', () => {
    const carried = failure('notionAuthFailed', {
      message: 'Notion이 이 token을 받아들이지 않았다.',
      detail: '401 unauthorized',
      sourceDataSafe: true,
      retryable: false,
    });

    const view = checkedNotionConnection(connection({ state: 'failed', failure: carried }));

    expect(view.kind === 'failed' && view.failure).toEqual(carried);
  });

  it('저장된 token이 없는 것은 실패가 아니라 상태다 (INV-8)', () => {
    const view = checkedNotionConnection(
      connection({ state: 'notConfigured', tokenStored: false, destinationConfigured: false }),
    );

    expect(view).toEqual({
      kind: 'noToken',
      text: NO_TOKEN_TO_CHECK_TEXT,
      resolution: TOKEN_NOT_STORED_RESOLUTION,
    });
    // 요청이 나가지 않았다는 사실까지 문장에 있다 — 사용자는 "연결 실패"를 보지 않는다.
    expect(NO_TOKEN_TO_CHECK_TEXT).toMatch(/no request was sent/i);
    expect(view.kind).not.toBe('failed');
  });

  it('확인 요청 자체가 거절된 것은 Notion에 대한 사실이 아니다', () => {
    // 자격증명 저장소를 읽지 못한 것이 여기로 온다. 그때 Notion이 답하는지는 아무도
    // 물어보지 못했으므로 "token을 다시 넣으세요"라고 말하지 않는다.
    const view = failedNotionCheck(failure('storage'));

    expect(view.kind).toBe('checkFailed');
    expect(view.text).toBe(NOTION_CHECK_FAILED_TEXT);
    expect(view.kind === 'checkFailed' && view.failure.kind).toBe('storage');
    expect(view.text).not.toBe(rejected('notionAuthFailed').text);
  });

  it('구조화되지 않은 거절도 삼키지 않고 화면에 도달한다 (§13)', () => {
    const view = failedNotionCheck(new Error('IPC가 답하지 않았다'));

    expect(view.kind === 'checkFailed' && view.failure.kind).toBe('unexpected');
    expect(view.kind === 'checkFailed' && view.failure.detail).toBe('IPC가 답하지 않았다');
  });

  it('아직 확인하지 않은 상태와 확인 중인 상태가 결과와 섞이지 않는다', () => {
    // 화면을 열자마자 Notion으로 나가지 않는다. 그 사실이 담담한 문장으로 있다.
    expect(NOTION_NOT_CHECKED_TEXT).not.toBe(CHECKING_NOTION_TEXT);
    expect(NOTION_NOT_CHECKED_TEXT).not.toBe(CONNECTED_TEXT);
    expect(NOTION_CHECK_USES_SAVED_SETTINGS).toMatch(/already saved/i);
  });
});

// --- token (INV-7) -------------------------------------------------------------------

describe('token은 사실로만 다뤄진다 (INV-7)', () => {
  it('저장 · 삭제가 답한 사실이 그대로 화면 상태가 된다', () => {
    expect(notionTokenState(tokenStatus(true))).toBe('stored');
    expect(notionTokenState(tokenStatus(false))).toBe('notStored');
    expect(tokenStateOf(connection({ tokenStored: true }))).toBe('stored');
    expect(tokenStateOf(connection({ tokenStored: false }))).toBe('notStored');
  });

  it('저장된 token이 없는 것을 담담한 상태로 말한다 (INV-8)', () => {
    const notice = notionTokenNotice('notStored');

    expect(notice.text).toBe(TOKEN_NOT_STORED_TEXT);
    expect(notice.resolution).toBe(TOKEN_NOT_STORED_RESOLUTION);
    expect(notice.text).not.toMatch(/error|failed|problem/i);
  });

  it('저장된 값을 다시 보여 줄 수 없다는 사실을 숨기지 않는다', () => {
    const notice = notionTokenNotice('stored');

    expect(notice.text).toBe(TOKEN_STORED_TEXT);
    expect(notice.resolution).toMatch(/never shown again/i);
    // 입력란이 비워진다는 것도 누르기 전에 적혀 있다.
    expect(TOKEN_INPUT_NOTICE).toMatch(/cleared from this box/i);
  });

  it('아직 물어보지 않은 것과 저장돼 있지 않은 것이 다른 상태다', () => {
    // 화면을 열자마자 자격증명 저장소를 뒤지지 않으므로, 처음 상태는 "모른다"이지
    // "없다"가 아니다 — 없다고 적으면 알지 못하는 것을 아는 것처럼 말하는 것이 된다.
    expect(notionTokenNotice('unknown').text).not.toBe(TOKEN_NOT_STORED_TEXT);
    expect(notionTokenNotice('unknown').resolution).toBeNull();
  });

  it('어떤 화면 상태에도 token 값이 실릴 자리가 없다', () => {
    // 이 모듈이 만드는 값 전부를 훑는다. 입력(계약)에 값이 없고 출력에도 자리가 없으므로,
    // 저장된 token이 화면으로 돌아오는 경로 자체가 존재하지 않는다.
    const views: unknown[] = [
      checkedNotionConnection(connection({ workspaceName: 'Ada의 워크스페이스' })),
      checkedNotionConnection(connection({ state: 'notConfigured', tokenStored: false })),
      rejected('notionAuthFailed'),
      failedNotionCheck(failure('storage')),
      notionTokenNotice('stored'),
      notionTokenNotice('notStored'),
      notionTokenNotice('unknown'),
      notionTokenTrouble('save', failure('storage')),
      notionTokenTrouble('delete', failure('storage')),
    ];

    for (const view of views) {
      expect(JSON.stringify(view)).not.toContain(NOT_A_REAL_TOKEN);
    }
  });

  it('저장 · 삭제가 실패해도 입력한 값을 되돌려 적지 않는다', () => {
    const save = notionTokenTrouble('save', failure('storage'));
    const remove = notionTokenTrouble('delete', failure('storage'));

    expect(save.text).not.toBe(remove.text);
    expect(save.failure.kind).toBe('storage');
    expect(JSON.stringify([save, remove])).not.toContain(NOT_A_REAL_TOKEN);
  });
});

// --- destination ---------------------------------------------------------------------

describe('보낼 부모 페이지', () => {
  it('고르지 않은 것은 오류가 아니라 상태다 (INV-8)', () => {
    expect(notionDestinationNotice('')).toBe(NO_DESTINATION_TEXT);
    expect(notionDestinationNotice('   ')).toBe(NO_DESTINATION_TEXT);
    expect(notionDestinationNotice('parent-page-identifier')).toBeNull();
    expect(HOW_TO_SET_A_DESTINATION).toMatch(/share it with your integration/i);
  });

  it('마지막 저장 뒤에 destination이 바뀌었으면 확인 결과가 그 값에 대한 답이 아니라고 말할 수 있다', () => {
    expect(notionDestinationChanged('new-page', 'saved-page')).toBe(true);
    expect(notionDestinationChanged('  saved-page  ', 'saved-page')).toBe(false);
    // 무엇이 저장돼 있는지 아직 모르면 바뀌었다고 말하지 않는다.
    expect(notionDestinationChanged('anything', null)).toBe(false);
  });

  it('Notion 쪽이 어떻게 끝나도 나머지 설정 저장이 막히지 않는다는 것을 화면이 말한다 (INV-8)', () => {
    expect(NOTION_SETTINGS_UNAFFECTED_NOTICE).toMatch(/still saves normally/i);
  });
});
