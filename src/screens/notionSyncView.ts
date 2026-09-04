/**
 * Recording Detail의 **Send to Notion** 자리 (PRODUCT-SPEC §5 C · §10 · §13 ·
 * `phase-prompt/05` 요구 9 · 12 · 13 · docs/ADR-0009-notion-and-export.md §8).
 *
 * 이 자리가 답해야 하는 질문은 넷이다.
 *
 * ```text
 * 지금 이 녹음의 Notion 상태는 무엇인가          §7의 저장된 상태 · 지금 벌어지고 있는 일
 * 무엇이 전송되는가                              INV-5 · INV-6 — 오디오는 나가지 않는다
 * 지금 누르면 무슨 일이 일어나는가               ADR-0009 §8.5 — **누르기 전에** 알 수 있다
 * 실패했다면 무엇이 남았고 다시 할 수 있는가     §13 · INV-3
 * ```
 *
 * 넷 다 backend가 준 사실에서 나오며, 그 사실을 화면 상태로 옮기는 규칙이 전부 여기 있다.
 * React도 DOM도 Tauri도 알지 않으므로 **실제 Notion 없이** vitest로 그대로 판정할 수 있다
 * (§18 · `aiNoteView.ts`와 같은 형태다).
 *
 * ## 누르기 전에 결과를 안다 (ADR-0009 §8.3 · §8.5)
 *
 * 중복 sync 정책은 결정돼 있다 — **Recording 하나에 Notion 페이지 하나**, 끝나지 않은 전송은
 * **같은 페이지에 이어 보내고**, 이미 끝난 것을 다시 보내는 것은 **확인을 받은 뒤 새 페이지**이며
 * 기존 페이지는 건드리지 않는다. 그 정책이 실제로 지켜지는지는 사용자가 버튼을 누른 **뒤가
 * 아니라 전에** 알 수 있어야 한다. 그래서 모든 동작이 {@link NotionSendAction.outcomeText}를
 * 들고 다니고, 새 페이지가 생기는 갈래는 [`confirmation`]이 `newPage`인 별도의 버튼이다 —
 * 아무것도 확인하지 않은 요청으로는 페이지가 새로 생기지 않는다.
 *
 * ## 오디오는 나가지 않는다 (INV-5 · INV-6)
 *
 * {@link NotionPanelView.contents}는 본문이 어느 상태든 언제나 있다. 나가는 것은 노트와 전사
 * **텍스트**이며, 오디오 파일은 어느 갈래에서도 전송되지 않는다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type {
  NotionConfirmation,
  NotionSendStatus,
  NotionSync,
  ProcessingStatus,
  Recording,
} from '../ipc/types';
import { statusBadge, type RecordingStatusBadge } from './recordingsView';

/**
 * 새 페이지를 만들기 전에 사용자가 알아야 하는 사실 (ADR-0009 §8.5).
 *
 * Rust가 `Failure.detail`에 `needsConfirmation=<값>`으로 실어 보내는 세 값과 1:1이다
 * (`src-tauri/src/sync/run.rs`의 `ConfirmBecause`). 셋을 하나로 접지 않는 이유는 사용자가
 * 확인해야 할 것이 전부 다르기 때문이다.
 */
export type NotionConfirmReason = 'alreadySent' | 'documentChanged' | 'outcomeUnknown';

/** `detail`에서 확인 사유를 읽어 내는 표시. Rust가 정한 값이며 화면이 만들지 않는다. */
const NEEDS_CONFIRMATION = 'needsConfirmation=';

const CONFIRM_REASONS: Record<string, NotionConfirmReason> = {
  alreadySent: 'alreadySent',
  documentChanged: 'documentChanged',
  outcomeUnknown: 'outcomeUnknown',
};

/**
 * 이 실패가 **실패가 아니라 확인 요청**인가 (ADR-0009 §8.5).
 *
 * 확인이 필요한 상태에서 보낸 요청은 아무것도 하지 않고 거절된다 — Notion에도 저장소에도
 * 아무 일이 일어나지 않았다. 그것을 다른 실패와 같은 모양으로 그리면 사용자는 무언가 잘못됐다고
 * 읽는다. 모르는 값은 확인 요청으로 읽지 않는다.
 */
export function confirmReason(failure: Failure | null): NotionConfirmReason | null {
  const detail = failure?.detail;
  if (detail === null || detail === undefined || !detail.startsWith(NEEDS_CONFIRMATION)) {
    return null;
  }
  return CONFIRM_REASONS[detail.slice(NEEDS_CONFIRMATION.length)] ?? null;
}

/**
 * 이 자리에서 사용자가 할 수 있는 동작 하나.
 *
 * **함수가 아니라 값이다** — 순수 모듈이 command를 알지 않기 때문이며, 그래서 "무엇이 실려
 * 나가는가"·"이 버튼이 페이지를 하나 더 만드는가"가 DOM 없이 판정된다. 실제로 부르는 것은
 * 화면 컴포넌트다 (`RecordingDetailScreen`의 `startNotionSync`).
 */
export interface NotionSendAction {
  readonly kind: 'send' | 'resume' | 'newPage' | 'retry';
  readonly label: string;
  readonly recordingId: string;
  /**
   * 이 동작이 싣는 확인 (ADR-0009 §8.3).
   *
   * **`newPage`인 버튼만이 페이지를 새로 만들 수 있다.** 나머지는 `notAsked`이며, 확인이
   * 필요한 상태를 만나면 보내지 않고 무엇을 확인해야 하는지 알린다.
   */
  readonly confirmation: NotionConfirmation;
  /** 누르기 전에 읽는 한 줄 — **누르면 무슨 일이 일어나는가** (§8.5 · 요구 8). */
  readonly outcomeText: string;
}

/** 처음 보낼 때. 화면에 처음 보이는 이름은 Phase Goal이 부르는 이름 그대로다. */
export const SEND_LABEL = 'Send to Notion';

/** 아직 아무것도 보낸 적이 없다. 부모 페이지 밑에 페이지 하나가 생긴다 (§5.1). */
export const NEW_PAGE_OUTCOME =
  'This creates one new page under the parent page set in Settings. Sending the same recording again later never overwrites it.';

/** 끝나지 않은 전송을 이어 보낸다 — **중복 페이지가 생기지 않는다** (§8.2). */
export const RESUME_OUTCOME =
  'This continues on the page that was already created and sends only the parts that never arrived. It does not create a second page. If the note or transcript changed since then, it stops and asks first.';

/** 결과를 모르는 채로 다시 시도한다 — 그래서 조용히 페이지를 하나 더 만들지 않는다 (§8.5). */
export const RETRY_OUTCOME =
  'This tries the same send again. If this app cannot tell whether a page was already created, it stops and asks you instead of making a second one.';

/** 확인 뒤 새 페이지. **기존 페이지는 그대로 둔다** (§8.3). */
const CONFIRM_OUTCOME: Record<NotionConfirmReason, string> = {
  alreadySent:
    'This recording already has a Notion page. Creating a new page leaves that page exactly as it is — nothing there is changed or deleted.',
  documentChanged:
    'The note or transcript changed since the last send, so the parts cannot be appended to the old page. Creating a new page leaves the old page exactly as it is.',
  outcomeUnknown:
    'The last send never learned whether a page was created. Check Notion first — creating a new page here can leave you with two.',
};

/** 확인이 필요한 이유를 사용자가 읽는 한 줄로. */
const CONFIRM_HEADLINE: Record<NotionConfirmReason, string> = {
  alreadySent: 'This recording is already in Notion.',
  documentChanged: 'This recording changed since it was last sent.',
  outcomeUnknown: 'The last send did not finish, and its result is not known.',
};

function sendAction(recordingId: string): NotionSendAction {
  return {
    kind: 'send',
    label: SEND_LABEL,
    recordingId,
    confirmation: 'notAsked',
    outcomeText: NEW_PAGE_OUTCOME,
  };
}

function resumeAction(recordingId: string): NotionSendAction {
  return {
    kind: 'resume',
    label: 'Continue sending to the same page',
    recordingId,
    confirmation: 'notAsked',
    outcomeText: RESUME_OUTCOME,
  };
}

function retryAction(recordingId: string): NotionSendAction {
  return {
    kind: 'retry',
    label: 'Try sending again',
    recordingId,
    confirmation: 'notAsked',
    outcomeText: RETRY_OUTCOME,
  };
}

/**
 * 새 페이지를 만드는 유일한 동작 (§8.3).
 *
 * 이것만이 `confirmation: 'newPage'`를 싣는다 — 사용자가 무슨 일이 일어나는지 읽고 이 버튼을
 * 누른 것이 곧 확인이다.
 */
function newPageAction(recordingId: string, reason: NotionConfirmReason): NotionSendAction {
  return {
    kind: 'newPage',
    label: 'Create a new Notion page',
    recordingId,
    confirmation: 'newPage',
    outcomeText: CONFIRM_OUTCOME[reason],
  };
}

/**
 * **무엇이 Notion으로 나가는가** (§12 · INV-5 · INV-6 · 요구 13).
 *
 * 본문이 어느 상태든 이 사실은 남는다. 나가는 것은 텍스트뿐이며 **오디오 파일은 어떤
 * 갈래에서도 전송되지 않는다.**
 */
export interface NotionSendContents {
  readonly headline: string;
  readonly items: readonly string[];
  /** 오디오는 나가지 않는다 (INV-6). */
  readonly audioNotice: string;
}

export const NOTION_CONTENTS_HEADLINE = 'What is sent to Notion';

export const NOTION_CONTENT_ITEMS: readonly string[] = [
  'The title, date, and length of this recording',
  'The transcript text',
  'The AI note, when this recording has one',
];

/** INV-6을 사용자 문장으로. 오디오 바이트도 오디오 경로도 요청에 실리지 않는다. */
export const NOTION_AUDIO_NOTICE =
  'The audio file is never sent. Only this text leaves this device, and it goes to Notion only.';

const SEND_CONTENTS: NotionSendContents = {
  headline: NOTION_CONTENTS_HEADLINE,
  items: NOTION_CONTENT_ITEMS,
  audioNotice: NOTION_AUDIO_NOTICE,
};

/**
 * 그 페이지에 문서의 어디까지 들어가 있는가 (§8.4 · 요구 12).
 *
 * **부분 전송이 드러나는 자리다.** 실패한 요청은 세지 않으므로, 둘이 다르면 그 페이지에는
 * 문서의 일부만 들어가 있다는 뜻이다.
 */
export interface NotionProgress {
  readonly sentChunks: number;
  readonly totalChunks: number;
  readonly complete: boolean;
  readonly text: string;
}

function progressOf(sync: NotionSync | null): NotionProgress | null {
  if (sync === null || sync.sentChunks === null || sync.totalChunks === null) {
    // 기록하기 전에 만들어진 행이면 둘 다 없다. 그것도 정상 상태이며 숫자를 지어내지 않는다.
    return null;
  }
  const complete = sync.sentChunks >= sync.totalChunks;
  return {
    sentChunks: sync.sentChunks,
    totalChunks: sync.totalChunks,
    complete,
    text: complete
      ? `All ${sync.totalChunks} parts of this document are on that page.`
      : `${sync.sentChunks} of ${sync.totalChunks} parts of this document are already on that page.`,
  };
}

/** 보낼 재료가 아직 없다. **실패가 아니다** (§7.2 · `sync::run`의 `nothing_to_send`). */
export const NOTHING_TO_SEND_TEXT = 'There is nothing to send from this recording yet.';

/** 그래서 무엇을 하면 되는가. 이 자리가 전사를 시작하지 않는다 — 그 자리는 Transcript 탭이다. */
export const NOTHING_TO_SEND_HINT =
  'Transcribe this recording in the Transcript tab first, then send it.';

/** 아직 보낸 적이 없다. **오류가 아니라 정상 상태다** (§7 · INV-8). */
export const NOT_SENT_TEXT = 'This recording has not been sent to Notion.';

/** 지금 보내는 중. 전송은 backend의 배경 스레드에서 돌고 화면은 그것을 물어볼 뿐이다. */
export const SENDING_TEXT =
  'Sending to Notion… This keeps running in the background, so you can leave this screen.';

/** 끝났다는 사실 한 줄. */
export const SENT_HEADLINE = 'This recording is in Notion.';

/** 무엇을 하다 실패했는가 (§13). 원인은 {@link Failure}가 말한다. */
export const SEND_FAILED_HEADLINE = 'This recording could not be sent to Notion.';

/**
 * 실패가 무엇을 남겼는지 (§13 · INV-3).
 *
 * **"복구했다"거나 "정리했다"고 말하지 않는다.** 전송이 건드리는 것은 이 녹음의 Notion 전송
 * 상태뿐이며 (`sync::run`), 이미 Notion에 있는 것도 그대로 둔다.
 */
export const SEND_PRESERVED_NOTICE =
  'The recording, its audio file, the transcript, and any AI note are untouched. Nothing was deleted here, and anything already in Notion is left as it is.';

/** 이유를 모를 때 그 사실을 그대로 말한다. **무엇이 실패했는지 지어내지 않는다.** */
export const UNKNOWN_SEND_NOTICE =
  'The stored state says the last send failed. The reason is not known in this session — send again to see what happens.';

/**
 * 실패 갈래 중 사용자가 **먼저** 할 일이 달라지는 것 (§13 · ADR-0009 §9.3).
 *
 * Rust가 나눠 보낸 구분을 뭉개지 않는다 — token을 고치는 것 · 부모 페이지를 integration에
 * 공유하는 것 · 잠시 기다리는 것 · 네트워크를 확인하는 것은 전부 다른 일이다.
 */
export type NotionFailureCause =
  | 'auth'
  | 'destination'
  | 'rateLimited'
  | 'requestFailed'
  | 'outcomeUnknown'
  | 'nothingToSend'
  | 'other'
  | 'unknown';

const FAILURE_RESOLUTION: Record<NotionFailureCause, string | null> = {
  auth: 'Notion did not accept the integration token. Put a working token in Settings, then send again.',
  destination:
    'Notion could not use the parent page. Share that page with the integration in Notion, or choose another page in Settings, then send again.',
  rateLimited: 'Notion asked this app to slow down. Try again in a little while.',
  requestFailed: 'Check that this device is online, then try again.',
  outcomeUnknown:
    'Notion answered in a way this app could not read, so it does not know whether a page was created. Open Notion and look before sending again.',
  nothingToSend: NOTHING_TO_SEND_HINT,
  other: null,
  unknown: UNKNOWN_SEND_NOTICE,
};

function failureCause(failure: Failure | null): NotionFailureCause {
  if (failure === null) {
    return 'unknown';
  }
  switch (failure.kind) {
    case 'notionAuthFailed':
      return 'auth';
    case 'notionDestinationUnavailable':
      return 'destination';
    case 'notionRateLimited':
      return 'rateLimited';
    case 'notionRequestFailed':
      return 'requestFailed';
    case 'notionResponseUnusable':
      return 'outcomeUnknown';
    case 'invalidInput':
      return 'nothingToSend';
    default:
      return 'other';
  }
}

/**
 * Notion 자리가 놓일 수 있는 상태의 전부.
 *
 * ```text
 * loading            아직 레코드를 읽지 못했다
 * nothingToSend      보낼 전사가 아직 없다 — 실패가 아니다 (§7.2)
 * ready              보낼 수 있다 — 누르면 무슨 일이 일어나는지가 함께 있다
 * sending            지금 보내고 있다
 * sent               페이지가 있다 — 다시 보내면 무슨 일이 일어나는지도 함께 있다 (§8.3)
 * needsConfirmation  보내지 않았다. 새 페이지를 만들지 확인을 묻는다 (§8.5) — 실패가 아니다
 * failed             보내지 못했다 — 원본은 그대로이고 다시 시도할 수 있다 (§13 · INV-3)
 * ```
 */
export type NotionPanelBody =
  | { readonly kind: 'loading' }
  | { readonly kind: 'nothingToSend'; readonly text: string; readonly hint: string }
  | { readonly kind: 'ready'; readonly text: string; readonly send: NotionSendAction }
  | {
      readonly kind: 'sending';
      readonly text: string;
      /** 어디까지 갔는지. 아직 기록되기 전이면 `null`이다. */
      readonly progress: NotionProgress | null;
    }
  | {
      readonly kind: 'sent';
      readonly headline: string;
      /** 그 녹음이 된 Notion 페이지. 저장된 기록을 아직 읽지 못했으면 `null`이다. */
      readonly pageId: string | null;
      /** 마지막으로 성공한 시각. backend가 준 ISO-8601 텍스트 그대로다. */
      readonly syncedAt: string | null;
      /** 이번 실행에서 **새로** 만든 페이지인가. 이어 보낸 것이면 거짓이다 (§8.2). */
      readonly createdPage: boolean;
      readonly progress: NotionProgress | null;
      /** 또 보내면 어떻게 되는가 (§8.3). **누르기 전에 읽는다.** */
      readonly again: NotionSendAction;
    }
  | {
      readonly kind: 'needsConfirmation';
      readonly headline: string;
      readonly reason: NotionConfirmReason;
      /** 무슨 일이 일어나는지 그대로. 이 문장은 Rust가 보낸 것이다. */
      readonly text: string;
      /** 아무것도 보내지 않았다는 사실 (INV-3). */
      readonly preservedNotice: string;
      readonly progress: NotionProgress | null;
      /** 확인하고 누르는 버튼. 이것만이 새 페이지를 만든다. */
      readonly confirm: NotionSendAction;
    }
  | {
      readonly kind: 'failed';
      readonly headline: string;
      /**
       * 실패 그대로 (§13의 세 질문에 대한 답이 이미 이 안에 있다).
       *
       * `null`이면 **이 앱이 켜진 뒤의 시도가 아니라서 이유를 모른다**는 뜻이다. 저장된 것은
       * `failed`라는 사실뿐이므로 이유를 지어내지 않는다.
       */
      readonly failure: Failure | null;
      readonly cause: NotionFailureCause;
      /** 원본이 그대로라는 사실 (INV-3). */
      readonly preservedNotice: string;
      /** 이 갈래에서 먼저 해야 하는 일. 없으면 `null`이다. */
      readonly resolution: string | null;
      /** 부분 전송이 있었다면 그 사실 (§8.4 · 요구 12). */
      readonly progress: NotionProgress | null;
      /** 실패해도 다시 시도할 수 있다 (§13). 무엇이 일어나는지는 `outcomeText`가 말한다. */
      readonly retry: NotionSendAction;
    };

/**
 * Notion 자리 전체.
 *
 * 본문과 나란히 **언제나 보이는 것 둘**이 있다 — 이 녹음의 Notion 상태(§7)와, 무엇이
 * 전송되는가(INV-5 · INV-6)다. 본문이 어느 상태든 그 둘은 남는다.
 */
export interface NotionPanelView {
  /** 목록과 **같은 규칙으로 만든** 상태 표시 (§7 · 요구 9). 아직 읽지 못했으면 `null`이다. */
  readonly status: RecordingStatusBadge | null;
  readonly contents: NotionSendContents;
  readonly body: NotionPanelBody;
}

/** {@link notionPanel}이 보는 사실 전부. */
export interface NotionPanelInput {
  /** 아직 읽지 못했으면 `null`이다. */
  readonly recording: Recording | null;
  /**
   * 디스크에 남아 있는 전송 기록 (§7의 `notion_syncs`).
   *
   * **보낸 적이 없거나 아직 읽지 못했으면 `null`이다.** 둘을 구분할 필요가 없는 이유는
   * `recording.notionStatus`가 이미 그 답을 들고 있기 때문이다 — `none`이면 보낸 적이 없고,
   * 그 밖의 상태에서 `null`이면 아직 읽지 못한 것이며 그때는 세부만 보이지 않는다.
   */
  readonly sync: NotionSync | null;
  /** 이 앱이 켜진 뒤의 전송 한 건. 아직 물어보지 못했으면 `null`이다. */
  readonly live: NotionSendStatus | null;
}

/**
 * 읽어 온 값을 Notion 자리의 상태로 바꾼다.
 *
 * 셋을 함께 본다 — **이 녹음의 저장된 Notion 상태**(§7) · **디스크에 남은 전송 기록**(§8.4) ·
 * **지금 이 앱이 돌리고 있는 전송 한 건**이다. 각자 답할 수 없는 것이 있기 때문에 셋이 다
 * 필요하다.
 *
 * ```text
 * 저장된 상태   앱을 다시 켜도 남는다            실패한 이유는 모른다
 * 전송 기록     어디까지 갔는지 · 어느 페이지인지  지금 무슨 일이 벌어지는지는 모른다
 * 지금의 전송   실패한 이유를 그대로 들고 있다     이 앱이 켜진 뒤의 것만 안다
 * ```
 *
 * 순서에는 규칙이 하나 있다 — **지금 벌어지고 있는 일이 저장된 값보다 먼저다.** 다른 녹음의
 * 전송은 이 자리와 아무 상관이 없으므로 보지 않는다.
 */
export function notionPanel(input: NotionPanelInput): NotionPanelView {
  const { recording, sync, live } = input;

  if (recording === null) {
    return { status: null, contents: SEND_CONTENTS, body: { kind: 'loading' } };
  }

  // 다른 녹음의 사실은 이 자리에 들어오지 않는다 — 화면이 새 녹음으로 옮겨 가는 동안 앞
  // 녹음의 페이지 식별자가 잠깐이라도 보이면, 그것은 사실이 아닌 것을 보여주는 일이다.
  const mine = live !== null && live.recordingId === recording.id ? live : null;
  const stored = sync !== null && sync.recordingId === recording.id ? sync : null;
  const progress = progressOf(stored);

  return {
    status: statusBadge('Notion', badgeStatus(recording, mine)),
    contents: SEND_CONTENTS,
    body: panelBody(recording, stored, mine, progress),
  };
}

/**
 * 상태 표시가 말하는 값 (§7 · 요구 9).
 *
 * **저장된 상태 그대로다** — 목록이 보여주는 값과 같아야 하기 때문이다. 예외는 하나뿐이고
 * 이유가 분명하다: **이 앱이 지금 돌리고 있는 전송은 저장된 값보다 새롭다.** 방금 시작한
 * 전송은 아직 레코드를 다시 읽기 전이므로 저장된 값에 나타나지 않으며, 그때 `Not started`를
 * 보여주면 사용자는 자기가 누른 것이 접수됐는지 알 수 없다.
 *
 * 확인을 묻는 것은 실패가 아니므로 저장된 값을 덮지 않는다 (§8.5) — 아무것도 하지 않고
 * 거절된 요청이며, 그 요청 때문에 이 녹음의 상태가 달라지지는 않았다.
 */
function badgeStatus(recording: Recording, mine: NotionSendStatus | null): ProcessingStatus {
  switch (mine?.state) {
    case 'running':
      return 'running';
    case 'done':
      return 'done';
    case 'failed':
      return confirmReason(mine.failure) === null ? 'failed' : recording.notionStatus;
    default:
      return recording.notionStatus;
  }
}

function panelBody(
  recording: Recording,
  sync: NotionSync | null,
  mine: NotionSendStatus | null,
  progress: NotionProgress | null,
): NotionPanelBody {
  // 1. 지금 이 녹음에 대해 벌어지고 있는 일. 저장된 값이 이것을 덮지 않는다.
  if (mine?.state === 'running') {
    return { kind: 'sending', text: SENDING_TEXT, progress };
  }
  if (mine?.state === 'done') {
    return sentBody(recording.id, mine.pageId, sync, progress, mine.createdPage);
  }
  if (mine?.state === 'failed') {
    return afterFailure(recording.id, mine.failure, sync, progress);
  }

  // 2. 저장된 상태 (§7). 앱을 다시 켠 뒤에도 남아 있는 사실이다.
  if (recording.notionStatus === 'pending' || recording.notionStatus === 'running') {
    return { kind: 'sending', text: SENDING_TEXT, progress };
  }
  if (recording.notionStatus === 'done') {
    return sentBody(recording.id, sync?.pageId ?? null, sync, progress, false);
  }
  if (recording.notionStatus === 'failed') {
    // 마지막 시도가 실패한 채로 남아 있다. 이 앱이 그 시도를 하지 않았으므로 이유는 모른다.
    return afterFailure(recording.id, null, sync, progress);
  }

  // 3. 보낼 재료 (§7.2).
  if (recording.currentTranscriptId === null) {
    return { kind: 'nothingToSend', text: NOTHING_TO_SEND_TEXT, hint: NOTHING_TO_SEND_HINT };
  }

  return { kind: 'ready', text: NOT_SENT_TEXT, send: sendAction(recording.id) };
}

function sentBody(
  recordingId: string,
  pageId: string | null,
  sync: NotionSync | null,
  progress: NotionProgress | null,
  createdPage: boolean,
): NotionPanelBody {
  return {
    kind: 'sent',
    headline: SENT_HEADLINE,
    // 상태가 `done`인데 페이지 식별자가 비어 오는 일은 없다 (§8.4-3). 그래도 빈 문자열을
    // 식별자인 것처럼 보여주지는 않는다.
    pageId: pageId === null || pageId.length === 0 ? (sync?.pageId ?? null) : pageId,
    syncedAt: sync?.syncedAt ?? null,
    createdPage,
    progress,
    // 또 보내는 것은 **새 페이지를 만드는 일**이며, 그 사실을 누르기 전에 말한다 (§8.3).
    again: newPageAction(recordingId, 'alreadySent'),
  };
}

/**
 * 전송이 끝나지 못한 뒤의 상태 (§8.5 · §13).
 *
 * 두 갈래가 갈린다 — **확인을 묻는 것**과 **실패**다. 확인 요청은 아무것도 하지 않고 거절된
 * 요청이므로 실패로 그리지 않는다: Notion에도 저장소에도 아무 일이 일어나지 않았고, 사용자가
 * 할 일은 "무슨 일이 일어날지 읽고 고르는 것"뿐이다.
 */
function afterFailure(
  recordingId: string,
  failure: Failure | null,
  sync: NotionSync | null,
  progress: NotionProgress | null,
): NotionPanelBody {
  const reason = confirmReason(failure);
  if (reason !== null) {
    return {
      kind: 'needsConfirmation',
      headline: CONFIRM_HEADLINE[reason],
      reason,
      // Rust가 보낸 문장 그대로. 같은 사실을 두 벌로 적지 않는다.
      text: failure?.message ?? CONFIRM_OUTCOME[reason],
      preservedNotice: SEND_PRESERVED_NOTICE,
      progress,
      confirm: newPageAction(recordingId, reason),
    };
  }

  const cause = failureCause(failure);
  return {
    kind: 'failed',
    headline: SEND_FAILED_HEADLINE,
    failure,
    cause,
    preservedNotice: SEND_PRESERVED_NOTICE,
    resolution: FAILURE_RESOLUTION[cause],
    progress,
    // 이어 보낼 수 있는 상태면 이어 보낸다 — **중복 페이지가 생기지 않는다** (§8.2).
    retry: canResume(sync) ? resumeAction(recordingId) : retryAction(recordingId),
  };
}

/**
 * 같은 페이지에 이어 보낼 수 있는가 (§8.2).
 *
 * 페이지가 있고 첫 chunk가 반영됐다는 것을 아는 경우다. 지금 문서가 그때 나눈 문서와 같은지는
 * **화면이 알 수 없다** — 그 판정은 fingerprint를 들고 있는 backend의 것이며, 다르면 이어 보내는
 * 대신 확인을 묻는다. 그래서 이 동작의 `outcomeText`가 그 사실까지 함께 말한다.
 */
function canResume(sync: NotionSync | null): boolean {
  return sync !== null && sync.pageId !== null && (sync.sentChunks ?? 0) >= 1;
}

/**
 * 요청 자체가 거절됐다 (§13).
 *
 * **전송의 실패와 다른 사실이다.** 이미 다른 전송이 돌고 있을 때나 저장된 값을 읽지 못했을 때가
 * 여기로 오며, 그때 이 녹음의 Notion 상태는 아무것도 달라지지 않았다 — 그래서 자리의 본문을
 * 덮지 않고 그 옆에 얹힌다. 접수되지 않은 요청이 조용히 사라지지 않게 하는 자리다.
 */
export interface NotionTrouble {
  readonly headline: string;
  readonly failure: Failure;
}

/** 화면이 backend에 보내는 Notion 관련 요청. 실패가 어느 요청의 것인지 구분하는 데 쓴다. */
export type NotionRequest = 'status' | 'sync' | 'start';

const TROUBLE_HEADLINE: Record<NotionRequest, string> = {
  status: 'The Notion send status could not be read.',
  sync: 'The saved Notion send record could not be read.',
  start: 'The Notion send could not be started.',
};

/** 거절된 요청 하나를 화면에 놓을 값으로 옮긴다 (§13). */
export function notionTrouble(request: NotionRequest, error: unknown): NotionTrouble {
  return { headline: TROUBLE_HEADLINE[request], failure: toFailure(error) };
}
