/**
 * Settings 화면의 **Notion 구역** (PRODUCT-SPEC §5 D · §13 ·
 * `docs/ADR-0009-notion-and-export.md` §5.1 · §8.4 · §10.4).
 *
 * 이 구역이 답해야 하는 질문은 넷이다 — **token이 저장돼 있는가** · **어디에 만들 것인가** ·
 * **지금 그 token으로 Notion과 말할 수 있는가** · **말하지 못했다면 무엇을 하면 되는가.**
 * 네 답을 만드는 규칙이 전부 여기 있고, `SettingsScreen`에는 그리는 일만 남는다 (§18).
 *
 * React도 DOM도 Tauri도 알지 않으므로 **실제 Notion 워크스페이스도 실제 자격증명 저장소도 없이**
 * vitest로 그대로 판정된다 — 확인의 결과는 값으로 들어오고, 이 모듈은 그 값을 화면 상태로
 * 옮길 뿐이다.
 *
 * ## token 값이 이 모듈에 없다 (INV-7 · ADR-0009 §10.4)
 *
 * 여기에는 token을 담을 필드도, token을 돌려주는 함수도 없다. 화면이 token에 대해 아는 것은
 * **저장돼 있는가** 하나뿐이며 ({@link NotionTokenState}), 그 사실조차 화면이 짐작하지 않고
 * 자격증명 저장소가 답한 값에서 온다 (`NotionTokenStatus` · `NotionConnection.tokenStored`).
 *
 * 입력한 값은 저장 command의 인자로 한 번 지나갈 뿐이다. 그래서 이 모듈에는 그 값이 지나갈
 * 자리조차 없고, `localStorage`·`sessionStorage`·URL·전역 상태에 쓰는 경로도 없다
 * (`tests/screen-boundary.test.ts`가 `src/` 전체에 대해 그것을 확인한다).
 *
 * ## 실패를 하나로 뭉치지 않는다 (§13)
 *
 * ```text
 * auth          token을 Notion이 받아들이지 않았다      → 다른 token을 넣는다
 * destination   부모 페이지를 쓸 수 없다                → Notion에서 그 페이지를 공유한다
 * offline       Notion에 닿지 못했다                    → 네트워크를 확인한다
 * rateLimited   잠시 천천히 하라고 했다                 → 조금 뒤에 다시 확인한다
 * other         그 밖의 이유                            → 실려 온 실패가 말한다
 * ```
 *
 * 넷을 "연결 실패" 하나로 접으면 사용자가 할 일이 사라진다. 그 구분은 backend가 이미 나눠서
 * 보내며 (`FailureKind`의 `notion*`), 화면이 그것을 다시 뭉치지 않는다 —
 * `notionSyncView.ts`가 전송 실패에 대해 따르는 규칙과 같다.
 *
 * ## 설정하지 않은 것은 오류가 아니다 (INV-8)
 *
 * token이 없는 것도, 부모 페이지를 고르지 않은 것도 **정상 상태다.** 그래서 그 상태의 문구는
 * 경고가 아니라 사실 한 줄과 무엇을 하면 되는지 한 줄이다 (`aiProviderSettings.ts`와 같은 태도).
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { NotionConnection, NotionTokenStatus } from '../ipc/types';

/**
 * 화면이 token에 대해 알 수 있는 **전부**.
 *
 * ```text
 * unknown    아직 물어보지 않았다 — 화면을 열자마자 자격증명 저장소를 뒤지지 않는다
 * stored     저장돼 있다
 * notStored  저장돼 있지 않다 — 오류가 아니다 (INV-8)
 * ```
 *
 * **값은 어느 변형에도 없다** (INV-7). 저장된 token을 돌려주는 command가 없으므로 화면이 그것을
 * 알 방법도 없고, 알 필요도 없다.
 */
export type NotionTokenState = 'unknown' | 'stored' | 'notStored';

/** 저장·삭제가 답한 사실을 화면 상태로 옮긴다. **화면이 자기가 누른 버튼으로 짐작하지 않는다.** */
export function notionTokenState(status: NotionTokenStatus): NotionTokenState {
  return status.stored ? 'stored' : 'notStored';
}

/**
 * 연결 확인이 함께 말해 준 저장 여부.
 *
 * 확인은 token을 실제로 읽어 보고 답하므로, 그 답이 저장 여부에 대해서도 가장 최근의 사실이다.
 */
export function tokenStateOf(connection: NotionConnection): NotionTokenState {
  return connection.tokenStored ? 'stored' : 'notStored';
}

/** token에 대해 화면이 할 말. **없는 것은 담담한 사실이지 경고가 아니다** (INV-8). */
export interface NotionTokenNotice {
  readonly text: string;
  /** 그 상태에서 사용자가 할 수 있는 일. 할 말이 없으면 `null`이다. */
  readonly resolution: string | null;
}

export const TOKEN_STATE_UNKNOWN_TEXT = 'Whether a token is saved has not been checked yet.';

export const TOKEN_STORED_TEXT = 'An integration token is saved on this device.';

/** 저장된 값을 되읽어 보여 줄 수 없다는 사실을 숨기지 않는다 (INV-7). */
export const TOKEN_STORED_RESOLUTION =
  'The token itself is never shown again — this app can only tell whether one is saved. Paste a new one to replace it, or remove it below.';

/** 저장된 token이 없다. **오류가 아니라 정상 상태다** (INV-8). */
export const TOKEN_NOT_STORED_TEXT = 'No integration token is saved, so nothing is sent to Notion.';

export const TOKEN_NOT_STORED_RESOLUTION =
  'Create an internal integration in Notion, then paste its token here and save it.';

const TOKEN_NOTICE: Record<NotionTokenState, NotionTokenNotice> = {
  unknown: { text: TOKEN_STATE_UNKNOWN_TEXT, resolution: null },
  stored: { text: TOKEN_STORED_TEXT, resolution: TOKEN_STORED_RESOLUTION },
  notStored: { text: TOKEN_NOT_STORED_TEXT, resolution: TOKEN_NOT_STORED_RESOLUTION },
};

/** 지금 token 상태에 대해 화면이 그대로 그릴 수 있는 문구. */
export function notionTokenNotice(state: NotionTokenState): NotionTokenNotice {
  return TOKEN_NOTICE[state];
}

/** 입력란에 적히는 안내. **저장된 값이 여기 채워지는 일은 없다** (INV-7). */
export const TOKEN_INPUT_PLACEHOLDER = 'Paste the integration token';

/** 넘긴 뒤 입력란이 비워진다는 사실을 먼저 알린다 — 사라진 것처럼 보이지 않게 한다. */
export const TOKEN_INPUT_NOTICE =
  'The token is handed to this app once and kept in the operating system credential store. It is cleared from this box as soon as it is saved, and it is never written to the app database or the browser.';

/**
 * 보낼 부모 페이지에 대해 화면이 할 말. 고르지 않은 것도 정상 상태다 (INV-8 · ADR-0009 §8.4).
 *
 * **secret이 아니다** — 어디에 쓰는지일 뿐이므로 이 값은 폼에 그대로 있고, 저장도 화면 전체의
 * Save 하나가 한다 (`settingsView.ts`).
 */
export const NO_DESTINATION_TEXT =
  'No parent page is set yet, so there is nowhere to create pages in Notion.';

export const HOW_TO_SET_A_DESTINATION =
  'Open the Notion page you want new pages to be created under, share it with your integration, and paste its page identifier here.';

/** 고른 destination에 대해 할 말. 할 말이 없으면 `null`이다. */
export function notionDestinationNotice(parentPageId: string): string | null {
  return parentPageId.trim() === '' ? NO_DESTINATION_TEXT : null;
}

/**
 * 마지막으로 저장한 뒤 destination이 바뀌었는가.
 *
 * 무엇이 저장돼 있는지 아직 모르면(`null`) 바뀌었다고 말하지 않는다 — `aiSettingsChanged`와
 * 같은 규칙이며, 이유도 같다: 모르는 것을 사실처럼 적지 않는다.
 */
export function notionDestinationChanged(parentPageId: string, saved: string | null): boolean {
  return saved === null ? false : parentPageId.trim() !== saved.trim();
}

/**
 * 연결 확인이 지금까지 말해 준 것 (§5-D · §13).
 *
 * ```text
 * notChecked   아직 물어보지 않았다 — 화면을 열자마자 Notion을 찾아 나서지 않는다
 * checking     물어보는 중이다
 * noToken      저장된 token이 없어 물어볼 것이 없다  — 오류가 아니다 (INV-8). 요청도 나가지 않았다
 * connected    이 token으로 지금 말할 수 있다        — 어느 워크스페이스인지가 함께 온다
 * failed       말하지 못했다                          — 무엇을 하면 되는지가 갈래마다 다르다 (§13)
 * checkFailed  확인 요청 자체가 거절됐다              — Notion에 대한 사실이 아니다
 * ```
 *
 * `checkFailed`가 `failed`와 따로 있는 이유는 `aiProviderSettings.ts`의 그것과 같다 — 자격증명
 * 저장소를 읽지 못한 것이 여기로 오며, 그때 Notion이 답하는지는 **아무도 물어보지 못했다.**
 */
export type NotionConnectionView =
  | { readonly kind: 'notChecked'; readonly text: string }
  | { readonly kind: 'checking'; readonly text: string }
  | { readonly kind: 'noToken'; readonly text: string; readonly resolution: string }
  | {
      readonly kind: 'connected';
      readonly text: string;
      /**
       * 확인이 말해 준 워크스페이스 이름. Notion이 말하지 않았으면 `null`이다 — 이름을
       * 지어내지 않는다 (`aiProviderSettings`의 `providerName`과 같은 규칙 · INV-9).
       */
      readonly workspaceName: string | null;
      /** 보낼 부모 페이지가 아직 없다면 그 사실. 있으면 `null`이다 (INV-8). */
      readonly destinationNotice: string | null;
    }
  | {
      readonly kind: 'failed';
      readonly cause: NotionCheckCause;
      readonly text: string;
      readonly resolution: string;
      readonly failure: Failure;
    }
  | { readonly kind: 'checkFailed'; readonly text: string; readonly failure: Failure };

/**
 * 확인이 실패한 갈래 중 **사용자가 할 일이 달라지는 것** (§13 · ADR-0009 §9.3).
 *
 * `notionSyncView.ts`가 전송 실패에 대해 나눈 것과 같은 구분이다 — 같은 실패를 두 화면이 다르게
 * 뭉치면 사용자는 같은 상황에서 다른 이야기를 듣게 된다.
 */
export type NotionCheckCause = 'auth' | 'destination' | 'offline' | 'rateLimited' | 'other';

/** 무엇이 확인되지 않았는가. 갈래마다 다른 문장이다 — "연결 실패" 하나로 접지 않는다. */
const CHECK_FAILED_TEXT: Record<NotionCheckCause, string> = {
  auth: 'Notion did not accept the saved integration token.',
  destination: 'Notion answered, but it could not use the parent page that is saved.',
  offline: 'This app could not reach Notion.',
  rateLimited: 'Notion asked this app to slow down, so the connection was not confirmed.',
  other: 'The connection to Notion could not be confirmed.',
};

/** 그래서 무엇을 하면 되는가 (§13의 세 번째 질문). */
const CHECK_FAILED_RESOLUTION: Record<NotionCheckCause, string> = {
  auth: 'Paste a working integration token above and save it, then check again.',
  destination:
    'Open that page in Notion, share it with your integration, or set another parent page below — then check again.',
  offline: 'Check that this device is online, then check again.',
  rateLimited: 'Check again in a little while. Nothing was changed in Notion.',
  other: 'Read what failed below, then check again.',
};

/** 실패의 종류를 사용자가 할 일로 옮긴다. **backend가 나눈 구분을 그대로 쓴다.** */
export function notionCheckCause(failure: Failure | null): NotionCheckCause {
  if (failure === null) {
    return 'other';
  }
  switch (failure.kind) {
    case 'notionAuthFailed':
      return 'auth';
    case 'notionDestinationUnavailable':
      return 'destination';
    case 'notionRequestFailed':
      return 'offline';
    case 'notionRateLimited':
      return 'rateLimited';
    default:
      return 'other';
  }
}

/** 아직 물어보지 않았다. **화면을 열자마자 Notion으로 나가지 않는다.** */
export const NOTION_NOT_CHECKED_TEXT = 'The Notion connection has not been checked yet.';

export const CHECKING_NOTION_TEXT = 'Checking the Notion connection…';

/** 저장된 token이 없어 확인할 것이 없다. **요청도 나가지 않았다** (INV-8). */
export const NO_TOKEN_TO_CHECK_TEXT =
  'No integration token is saved, so there is nothing to check and no request was sent.';

/** 연결됐지만 Notion이 워크스페이스 이름을 말하지 않았다. **이름을 지어내지 않는다.** */
export const CONNECTED_TEXT = 'Notion answered, and the saved token works.';

/** 어느 워크스페이스인지까지 말해 줬다. */
export function connectedToWorkspaceText(workspaceName: string): string {
  return `Notion answered, and the saved token works in the workspace “${workspaceName}”.`;
}

/** 확인 요청 자체가 거절됐다. Notion이 답하는지는 여전히 알지 못한다. */
export const NOTION_CHECK_FAILED_TEXT = 'The Notion connection could not be checked.';

/** 확인은 **저장된** 값에게 물어본다 — token은 자격증명 저장소에서, destination은 설정에서 온다. */
export const NOTION_CHECK_USES_SAVED_SETTINGS =
  'The check uses the token and the parent page that are already saved. Save first to check a new parent page.';

/** Notion 쪽이 어떻게 끝나도 이 화면의 나머지는 그대로다 (INV-8). */
export const NOTION_SETTINGS_UNAFFECTED_NOTICE =
  'Every other setting on this screen still saves normally, whether or not Notion answers.';

/**
 * backend가 답한 연결 상태를 화면 상태로 옮긴다.
 *
 * **세 상태를 세 갈래로 그대로 옮기고**, `failed`만 사용자가 할 일에 따라 다시 나눈다 (§13).
 * 여기서 뭉치지 않는 이유는 계약이 그것을 나눠 보냈기 때문이다 (`NotionConnection`).
 */
export function checkedNotionConnection(connection: NotionConnection): NotionConnectionView {
  switch (connection.state) {
    case 'notConfigured':
      return {
        kind: 'noToken',
        text: NO_TOKEN_TO_CHECK_TEXT,
        resolution: TOKEN_NOT_STORED_RESOLUTION,
      };
    case 'connected':
      return {
        kind: 'connected',
        text:
          connection.workspaceName === null
            ? CONNECTED_TEXT
            : connectedToWorkspaceText(connection.workspaceName),
        workspaceName: connection.workspaceName,
        // 연결됐다는 것과 보낼 자리가 있다는 것은 **다른 사실이다.** 확인이 성공했어도 부모
        // 페이지가 없으면 아직 보낼 수 없으며, 그 사실을 여기서 함께 말한다.
        destinationNotice: connection.destinationConfigured ? null : NO_DESTINATION_TEXT,
      };
    case 'failed': {
      const cause = notionCheckCause(connection.failure);
      return {
        kind: 'failed',
        cause,
        text: CHECK_FAILED_TEXT[cause],
        resolution: CHECK_FAILED_RESOLUTION[cause],
        // 계약상 `failed`에는 실패가 실려 온다. 오지 않았다면 그것도 삼키지 않고 §13의 모양으로
        // 만들어 보인다 — 실패를 이유 없이 그리지 않기 위해서다.
        failure: connection.failure ?? toFailure(null),
      };
    }
  }
}

/**
 * 확인 요청 자체가 거절됐다.
 *
 * **`failed`와 다른 사실이다.** 자격증명 저장소를 읽지 못한 것이 여기로 오며, 그때 Notion이
 * 답하는지는 아무도 물어보지 못했다. 둘을 같은 값으로 만들면 화면이 "token을 다시 넣으세요"라고
 * 말하게 되는데, token이 멀쩡해도 달라지지 않는다.
 */
export function failedNotionCheck(error: unknown): NotionConnectionView {
  return { kind: 'checkFailed', text: NOTION_CHECK_FAILED_TEXT, failure: toFailure(error) };
}

/** 저장·삭제가 거절됐을 때 화면이 할 말 (§13). */
export interface NotionTokenTrouble {
  /**
   * 무엇을 하다 실패했는가.
   *
   * 화면이 이 값을 보는 이유는 하나다 — **저장 실패에는 다시 시도 수단이 없다.** 넘긴 값은
   * 이미 화면에 없으므로 같은 버튼이 같은 일을 할 수 없고, 지우기는 값 없이 다시 할 수 있다.
   */
  readonly request: NotionTokenRequest;
  readonly text: string;
  readonly failure: Failure;
}

const TOKEN_TROUBLE_TEXT = {
  save: 'The integration token could not be saved.',
  delete: 'The saved integration token could not be removed.',
} as const;

/** 무엇을 하려다 실패했는가. 값 자체는 어느 쪽에도 실리지 않는다 (INV-7). */
export type NotionTokenRequest = keyof typeof TOKEN_TROUBLE_TEXT;

/**
 * 저장·삭제 실패를 화면 상태로 옮긴다.
 *
 * **입력한 값을 되돌려 적지 않는다.** 실패 문장은 backend가 만든 것 그대로이며, 이 함수에는
 * token이 지나갈 자리가 없다 (ADR-0009 §10.4).
 */
export function notionTokenTrouble(
  request: NotionTokenRequest,
  error: unknown,
): NotionTokenTrouble {
  return { request, text: TOKEN_TROUBLE_TEXT[request], failure: toFailure(error) };
}
