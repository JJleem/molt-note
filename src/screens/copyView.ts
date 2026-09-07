/**
 * AI Note 탭의 **Copy AI Prompt · Copy Transcript** 두 자리 (PRODUCT-SPEC §13 ·
 * `phase-prompt/05.5` 요구 A-1 · A-2 · 5 · docs/ADR-0010-manual-ai-handoff.md §7.5).
 *
 * 이 자리가 답해야 하는 질문은 다섯이다.
 *
 * ```text
 * 지금 복사할 수 있는가            §7.2 — 현재 성공한 전사가 있어야 복사할 재료가 있다
 * 지금 무슨 일이 벌어지고 있는가   아직 안 함 · 복사 중 · 복사됨 · 실패
 * 복사됐다는 것을 어떻게 아는가    §7.5 — **텍스트로 말한다.** 색만으로 말하지 않는다
 * 얼마나 크고 몇 번째를 가져갔는가 05.6 성공 기준 4 — 크기와 자리와, 나머지를 가져가는 수단
 * 실패했다면 무엇이 남았고 어디로 갈 수 있는가   §13 · §7.5 — 재시도와 Export for AI
 * ```
 *
 * ## 크기 때문에 조용히 실패하지 않는다 (`phase-prompt/05.6` 성공 기준 4 · R-5)
 *
 * 72분 녹음의 산출물은 채팅 창에 한 번에 들어가지 않았고, 그때 앱은 아무 말도 하지 않았다.
 * 이제 복사가 끝난 자리는 언제나 셋을 말한다 — **얼마나 큰가**({@link HandoffSizeView}) ·
 * **지금 올라간 것이 몇 번째 중 몇 번째인가**({@link PortionView}) · **나머지를 마저 가져가는
 * 수단**(`next`). 조각 하나만 올라갔을 때는 머리글부터 다르다: **잘린 것을 온전한 것이라고
 * 말하는 상태가 이 모듈에 없다.**
 *
 * React도 DOM도 clipboard도 Tauri도 알지 않는다. 그래서 네 갈래 전부를 **실제 clipboard 없이**
 * vitest로 판정할 수 있다 (§18 · `exportView.ts` · `notionSyncView.ts`와 같은 형태다).
 * 실제로 clipboard에 쓰는 자리는 `src/platform/clipboard.ts` 하나이며 (INV-10 · R-4), 이 모듈이
 * 거기서 가져오는 것은 **실패를 읽는 규칙 하나뿐이다** — 쓰기를 부르지 않는다.
 *
 * ## 복사됐다는 표시는 텍스트다 (§7.5)
 *
 * 성공도 실패도 [`headline`]과 [`text`]를 들고 다닌다. 초록/빨강 같은 색 하나로만 말하면
 * 색을 구분하지 못하는 사람에게는 아무 일도 일어나지 않은 것과 같다. 무엇이 일어났는지는
 * 언제나 문장으로 있다.
 *
 * ## 복사가 막혀도 목적은 달성된다 (§7.5)
 *
 * clipboard 쓰기 능력은 이 저장소가 확인하지 못한 플랫폼 사실이다 (ADR-0010 §7.4). 그래서
 * 실패 상태에는 언제나 {@link CopyAlternative}가 함께 있다 — Export for AI는 clipboard를 전혀
 * 쓰지 않으므로, 복사가 거절되는 환경에서도 사용자는 같은 텍스트를 파일로 꺼낼 수 있다.
 */
import { clipboardTrouble } from '../platform/clipboard';
import { toFailure, type Failure } from '../ipc/failure';
import type { HandoffText, NoteMode, PortionOf, Recording } from '../ipc/types';

// --- 산출물의 크기와 나눔 (`phase-prompt/05.6` 성공 기준 4 · R-5) ------------------------
//
// 72분 녹음의 산출물은 99 KB · 5,139줄이었다. 사람은 그것을 채팅 창에 붙여 넣으려다 실패했고,
// **앱은 그 사실을 말해 주지 않았다.** 아래 두 값이 그 침묵을 없앤다 — 화면은 이것으로
// "얼마나 큰가"와 "지금 가져간 것이 몇 번째 중 몇 번째인가"를 문장으로 말할 수 있다.
//
// **나누는 규칙은 여기 없다.** 어디서 자를지도, 예산이 얼마인지도 backend 한 자리에 있고
// (`src-tauri/src/export/portion.rs`), 이 모듈이 하는 일은 그 결과를 사람이 읽는 값으로
// 옮기는 것뿐이다. `Export for AI` 자리도 같은 값을 쓴다 (`aiHandoffView.ts`).

/** 큰 수에 자리를 끊어 준다 — `41320` → `41,320`. 세는 값이지 시각이 아니다. */
function counted(value: number): string {
  return String(value).replace(/\B(?=(\d{3})+$)/g, ',');
}

/** 한 번에 들어가지 않을 수 있다는 사실. **실패가 아니라 크기에 대한 안내다.** */
export const TOO_LONG_NOTICE =
  'This is long enough that a chat may not take it in one paste, so it comes in parts. Take them in order — nothing is left out.';

/** 산출물 하나가 **얼마나 큰가**. 화면이 사용자에게 그대로 말할 수 있는 모양이다. */
export interface HandoffSizeView {
  /** 산출물 전체의 글자 수. */
  readonly characters: number;
  /** 산출물 전체의 줄 수. 전사에서는 segment 수에 가깝다. */
  readonly lines: number;
  /** 사람이 읽는 한 줄. */
  readonly label: string;
  /** 한 조각에 다 들어갔는가. */
  readonly fitsInOne: boolean;
  /** 들어가지 않았을 때만 있는 안내. 그 밖에는 `null`이다. */
  readonly tooLongNotice: string | null;
}

/** 돌아온 값의 크기를 화면이 말할 수 있는 모양으로 옮긴다. */
export function handoffSize(of: PortionOf): HandoffSizeView {
  const fitsInOne = of.portionCount <= 1;

  return {
    characters: of.totalSize.chars,
    lines: of.totalSize.lines,
    label: `About ${counted(of.totalSize.chars)} characters across ${counted(of.totalSize.lines)} lines.`,
    fitsInOne,
    tooLongNotice: fitsInOne ? null : TOO_LONG_NOTICE,
  };
}

/**
 * 지금 가져간 것이 **몇 번째 중 몇 번째인가**.
 *
 * `whole`이 참인 것과 `Part 1 of 4`는 다른 사실이며, 그 차이가 값으로 있어야 화면이 잘린
 * 결과를 온전한 것처럼 말하지 않는다.
 */
export interface PortionView {
  /** 몇 번째인가. **1부터 센다.** */
  readonly index: number;
  /** 전부 몇 개인가. */
  readonly count: number;
  /** 나뉘지 않았는가 — 지금 가져간 것이 전부인가. */
  readonly whole: boolean;
  /** 사람이 읽는 한 줄. 나뉘지 않았으면 그 사실을 말한다. */
  readonly label: string;
  /** 아직 가져가지 않은 조각이 몇 개 남았는가. */
  readonly remaining: number;
}

/** 나뉘지 않았을 때의 문장. **"1/1"이라고 말하지 않는다.** */
export const WHOLE_PORTION_LABEL = 'This is all of it.';

/** 돌아온 값의 자리를 화면이 말할 수 있는 모양으로 옮긴다. */
export function portionTaken(of: PortionOf): PortionView {
  const whole = of.portionCount <= 1;

  return {
    index: of.portion,
    count: of.portionCount,
    whole,
    label: whole ? WHOLE_PORTION_LABEL : `Part ${of.portion} of ${of.portionCount}.`,
    remaining: Math.max(of.portionCount - of.portion, 0),
  };
}

/**
 * 복사할 수 있는 것 둘 (요구 A-1 · A-2).
 *
 * 둘을 한 동작으로 접지 않는다 — 사용자가 붙여 넣을 곳이 다르고, 하나가 실패했다고 해서
 * 다른 하나가 실패한 것도 아니다.
 */
export type CopyTarget = 'prompt' | 'transcript';

/**
 * 이 자리에서 사용자가 할 수 있는 동작 하나.
 *
 * **함수가 아니라 값이다** — 순수 모듈이 command도 clipboard도 알지 않기 때문이며, 그래서
 * "지금 복사할 수 있는가"·"재시도 수단이 있는가"가 DOM 없이 판정된다. 실제로 부르는 것은
 * 화면 컴포넌트이며, 그 호출은 `src/ipc/commands.ts`와 `src/platform/clipboard.ts` 두 경계를
 * 지난다.
 */
export interface CopyAction {
  readonly kind: 'copy' | 'again' | 'retry' | 'next';
  readonly target: CopyTarget;
  readonly label: string;
  readonly recordingId: string;
  /**
   * 프롬프트를 만드는 데 필요한 mode (Meeting · Study · Summary).
   *
   * 전사 복사에는 mode가 없으므로 `null`이다 — 없는 것을 있는 것처럼 싣지 않는다.
   */
  readonly mode: NoteMode | null;
  /**
   * 가져갈 조각 (`phase-prompt/05.6` 성공 기준 4). **1부터 센다.**
   *
   * 이 값이 동작에 실려 있기 때문에 "나머지를 마저 가져가는 수단"이 화면 값으로 존재한다 —
   * 컴포넌트는 누른 동작을 그대로 backend에 넘길 뿐 몇 번째를 달라고 할지 스스로 정하지 않는다.
   */
  readonly portion: number;
}

/** 화면에 처음 보이는 이름. Phase Goal이 부르는 이름 그대로다. */
export const COPY_PROMPT_LABEL = 'Copy AI Prompt';

/** 화면에 처음 보이는 이름. Phase Goal이 부르는 이름 그대로다. */
export const COPY_TRANSCRIPT_LABEL = 'Copy Transcript';

const START_LABEL: Record<CopyTarget, string> = {
  prompt: COPY_PROMPT_LABEL,
  transcript: COPY_TRANSCRIPT_LABEL,
};

const AGAIN_LABEL: Record<CopyTarget, string> = {
  prompt: 'Copy the prompt again',
  transcript: 'Copy the transcript again',
};

const RETRY_LABEL: Record<CopyTarget, string> = {
  prompt: 'Try copying the prompt again',
  transcript: 'Try copying the transcript again',
};

const FIXED_LABEL: Record<Exclude<CopyAction['kind'], 'next'>, Record<CopyTarget, string>> = {
  copy: START_LABEL,
  again: AGAIN_LABEL,
  retry: RETRY_LABEL,
};

/**
 * 처음 누르는 조각은 언제나 첫 번째다.
 *
 * backend도 아무것도 고르지 않은 요청을 첫 조각으로 읽는다 — 두 자리가 같은 기본값을 갖도록
 * 여기서도 그 숫자를 값으로 적는다.
 */
export const FIRST_PORTION = 1;

function action(
  kind: CopyAction['kind'],
  target: CopyTarget,
  recordingId: string,
  mode: NoteMode,
  portion: number,
  count: number,
): CopyAction {
  return {
    kind,
    target,
    // 다음 조각의 이름만 상태에 따라 달라진다 — 몇 번째를 가져오는지가 버튼에 보여야
    // 사용자가 누르기 전에 무엇이 일어날지 안다.
    label: kind === 'next' ? `Copy part ${portion} of ${count}` : FIXED_LABEL[kind][target],
    recordingId,
    mode: target === 'prompt' ? mode : null,
    portion,
  };
}

/**
 * 복사가 막혔을 때 남는 길 (§7.5).
 *
 * Export for AI는 clipboard를 전혀 쓰지 않는다 — 파일 하나를 `exports/`에 쓰고 실제로 쓰인
 * 경로를 돌려준다. 그래서 clipboard가 거절되는 환경에서도 Manual AI Handoff는 성립한다.
 * 이 값은 **실패 상태에 언제나 있다.**
 */
export interface CopyAlternative {
  readonly label: string;
  readonly text: string;
}

/** Export for AI 자리의 이름. 이 모듈이 그 동작을 수행하지 않는다 — 가리키기만 한다. */
export const COPY_ALTERNATIVE_LABEL = 'Export for AI';

export const COPY_ALTERNATIVE_TEXT =
  'Export for AI writes the same text to a file on this device without using the clipboard. You can attach that file to your AI chat, or open it and copy from there.';

const ALTERNATIVE: CopyAlternative = {
  label: COPY_ALTERNATIVE_LABEL,
  text: COPY_ALTERNATIVE_TEXT,
};

/** 복사할 재료가 아직 없다. **실패가 아니다** (§7.2 · ADR-0010 §5.5). */
export const NOTHING_TO_COPY_TEXT = 'There is nothing to copy from this recording yet.';

/** 그래서 무엇을 하면 되는가. 이 자리가 전사를 시작하지 않는다 — 그 자리는 Transcript 탭이다. */
export const NOTHING_TO_COPY_HINT =
  'Transcribe this recording in the Transcript tab first, then copy it.';

/** 지금 눌러도 되는가. provider가 하나도 없어도 이 동작은 그대로 가능하다 (MH-1 · MH-2). */
const READY_TEXT: Record<CopyTarget, string> = {
  prompt:
    'Copy a ready-to-paste prompt for this recording, then paste it into the AI chat you already use.',
  transcript: 'Copy the transcript text of this recording so you can paste it anywhere.',
};

/** 복사 중. 프롬프트·전사 텍스트를 만드는 일과 clipboard에 쓰는 일이 둘 다 여기에 든다. */
const COPYING_TEXT: Record<CopyTarget, string> = {
  prompt: 'Copying the prompt…',
  transcript: 'Copying the transcript…',
};

/** 복사됐다는 사실 한 줄. **색이 아니라 이 문장이 그것을 말한다** (§7.5). */
export const COPIED_HEADLINE = 'Copied to the clipboard.';

/**
 * **조각 하나만 올라갔을 때의 사실 한 줄** (`phase-prompt/05.6` 성공 기준 4).
 *
 * 이때 "Copied to the clipboard."라고만 말하면 그것은 **잘린 결과를 온전한 것이라고 말하는
 * 것이다.** 그래서 머리글부터 다르다 — 무엇이 올라갔는지는 바로 아래 자리 문장이 말한다.
 */
export const COPIED_PORTION_HEADLINE = 'One part is on your clipboard — not the whole thing yet.';

const COPIED_TEXT: Record<CopyTarget, string> = {
  prompt: 'The prompt is on your clipboard. Paste it into your AI chat.',
  transcript: 'The transcript is on your clipboard. Paste it wherever you need it.',
};

const COPIED_PORTION_TEXT: Record<CopyTarget, string> = {
  prompt:
    'Paste this part into your AI chat, then come back for the next one and paste it after this.',
  transcript:
    'Paste this part where you need it, then come back for the next one and paste it after this.',
};

/** 마지막 조각까지 왔다. **여기서만 "다 가져갔다"고 말한다.** */
const LAST_PORTION_TEXT: Record<CopyTarget, string> = {
  prompt: 'That was the last part. Pasted in order, the parts make the whole prompt.',
  transcript: 'That was the last part. Pasted in order, the parts make the whole transcript.',
};

/** 무엇을 하다 실패했는가 (§13). 원인은 {@link Failure}가 말한다. */
const FAILED_HEADLINE: Record<CopyTarget, string> = {
  prompt: 'The prompt could not be copied.',
  transcript: 'The transcript could not be copied.',
};

/**
 * 실패가 무엇을 남겼는지 (§13 · INV-3 · MH-7).
 *
 * **"복구했다"거나 "정리했다"고 말하지 않는다.** 복사는 읽기만 하며 저장소에 쓰지 않는다.
 * 쓰기가 거절됐다면 clipboard에 이미 있던 것도 그대로다.
 */
export const COPY_PRESERVED_NOTICE =
  'The recording, its audio file, the transcript, and any AI note are untouched. Copying only reads, and a copy that failed left whatever was already on your clipboard as it was.';

/**
 * 실패 갈래 중 사용자가 **먼저** 할 일이 달라지는 것 (§13 · ADR-0010 §7.5).
 *
 * ```text
 * clipboardUnavailable  이 창에 clipboard 쓰기 능력이 없다 — 다시 눌러도 같다
 * clipboardRejected     이번 쓰기가 거절됐다 — 다시 눌러 볼 수 있다
 * nothingToCopy         복사할 전사가 없다 — 먼저 전사한다
 * storage               복사할 텍스트를 읽지 못했다
 * other                 그 밖 — 이유를 지어내지 않는다
 * ```
 */
export type CopyFailureCause =
  | 'clipboardUnavailable'
  | 'clipboardRejected'
  | 'nothingToCopy'
  | 'storage'
  | 'other';

const FAILURE_RESOLUTION: Record<CopyFailureCause, string | null> = {
  clipboardUnavailable:
    'This window cannot write to the clipboard, so pressing the button again gives the same result. Use Export for AI instead — it writes the same text to a file.',
  clipboardRejected:
    'The clipboard did not take the text this time. Press the button again, and if it keeps failing use Export for AI instead.',
  nothingToCopy: NOTHING_TO_COPY_HINT,
  storage:
    'The text could not be read from this device. Check that the app can read its data folder, then try again.',
  other: null,
};

/**
 * 실패 하나를 갈래로 옮긴다.
 *
 * clipboard 실패는 `kind`가 아니라 `detail`의 표시로 구분된다 (ADR-0010 §7.2) — 새
 * `FailureKind`를 만들지 않기 때문이며, 그 판정은 경계 모듈이 값으로 내준다.
 */
function failureCause(failure: Failure): CopyFailureCause {
  switch (clipboardTrouble(failure)) {
    case 'unavailable':
      return 'clipboardUnavailable';
    case 'rejected':
      return 'clipboardRejected';
    default:
      break;
  }
  switch (failure.kind) {
    case 'invalidInput':
      return 'nothingToCopy';
    case 'storage':
      return 'storage';
    default:
      return 'other';
  }
}

/**
 * 이 화면이 건 복사 한 번.
 *
 * 전사·노트 생성과 달리 **상태를 물어보는 규약을 쓰지 않는다** — 복사는 command 하나와
 * clipboard 쓰기 하나로 끝나는 짧은 일이며, backend에 남는 상태가 없다. 그래서 진행 상황은
 * 이 호출의 결과이고, 그 네 갈래가 이 타입이다 (아직 안 함 · 복사 중 · 복사됨 · 실패).
 */
export type CopyAttempt =
  | { readonly kind: 'none' }
  | {
      readonly kind: 'copying';
      readonly target: CopyTarget;
      readonly recordingId: string;
      readonly portion: number;
    }
  | {
      readonly kind: 'copied';
      readonly target: CopyTarget;
      readonly recordingId: string;
      /**
       * clipboard에 올라간 것이 **무엇의 어디였는가** (`phase-prompt/05.6` 성공 기준 4).
       *
       * 이 값이 실려 있기 때문에 화면은 "복사됐다"에서 멈추지 않고 크기와 자리를 말할 수 있고,
       * 조각이 남았는지도 안다 — 잘린 것을 온전한 것처럼 말할 수단이 없다.
       */
      readonly taken: HandoffText;
    }
  | {
      readonly kind: 'failed';
      readonly target: CopyTarget;
      readonly recordingId: string;
      /** 가지러 갔던 조각. 재시도가 **그 조각으로** 돌아가게 하는 값이다. */
      readonly portion: number;
      readonly failure: Failure;
    };

/** 아무것도 하지 않은 상태. 화면이 열렸을 때의 값이다. */
export const NO_COPY_ATTEMPT: CopyAttempt = { kind: 'none' };

/** 복사를 시작했을 때 만드는 값. 어느 조각을 가지러 갔는지 함께 들고 있다. */
export function startedCopy(
  target: CopyTarget,
  recordingId: string,
  portion: number,
): CopyAttempt {
  return { kind: 'copying', target, recordingId, portion };
}

/** clipboard가 텍스트를 받았을 때 만드는 값 — backend가 준 크기와 자리를 그대로 들고 있다. */
export function copiedText(
  target: CopyTarget,
  recordingId: string,
  taken: HandoffText,
): CopyAttempt {
  return { kind: 'copied', target, recordingId, taken };
}

/**
 * 복사가 끝나지 못했을 때 만드는 값 (§13).
 *
 * 텍스트를 만드는 command가 거절한 것도, clipboard가 거절한 것도 여기로 온다 — 사용자에게는
 * "복사되지 않았다"는 하나의 사건이며, 무엇 때문이었는지는 {@link CopyFailureCause}가 가른다.
 * **어느 쪽이든 console로 흘려보내지 않는다.**
 */
export function failedCopy(
  target: CopyTarget,
  recordingId: string,
  portion: number,
  error: unknown,
): CopyAttempt {
  return { kind: 'failed', target, recordingId, portion, failure: toFailure(error) };
}

/**
 * 복사 자리 하나가 놓일 수 있는 상태의 전부.
 *
 * ```text
 * loading        아직 레코드를 읽지 못했다
 * nothingToCopy  복사할 전사가 아직 없다 — 실패가 아니다 (§7.2)
 * notAsked       지금 복사할 수 있다 — 아직 아무것도 하지 않았다
 * copying        복사하는 중이다
 * copied         clipboard에 들어갔다 — **그 사실이 문장으로 있다** (§7.5)
 * failed         복사되지 않았다 — 원본은 그대로이고, 다시 시도할 수 있고, 다른 길이 있다
 * ```
 */
export type CopyBody =
  | { readonly kind: 'loading' }
  | { readonly kind: 'nothingToCopy'; readonly text: string; readonly hint: string }
  | { readonly kind: 'notAsked'; readonly text: string; readonly start: CopyAction }
  | { readonly kind: 'copying'; readonly text: string }
  | {
      readonly kind: 'copied';
      /**
       * 복사됐다는 사실. 색이 아니라 이 문장이 그것을 말한다 (§7.5).
       *
       * **조각 하나만 올라갔으면 다른 문장이다** — 잘린 결과를 온전한 것이라고 말하지 않는다.
       */
      readonly headline: string;
      readonly text: string;
      /** 산출물 전체가 얼마나 큰가 (`phase-prompt/05.6` 성공 기준 4). */
      readonly size: HandoffSizeView;
      /** 지금 올라간 것이 몇 번째 중 몇 번째인가. */
      readonly portion: PortionView;
      /** 또 복사할 수 있다. **같은 조각이** 다시 clipboard로 간다. */
      readonly again: CopyAction;
      /** 나머지를 마저 가져가는 수단. 남은 조각이 없으면 `null`이다. */
      readonly next: CopyAction | null;
    }
  | {
      readonly kind: 'failed';
      /** 무엇을 하다 실패했는가. 색이 아니라 이 문장이 그것을 말한다 (§13 · §7.5). */
      readonly headline: string;
      /** 실패 그대로 (§13의 세 질문에 대한 답이 이미 이 안에 있다). */
      readonly failure: Failure;
      readonly cause: CopyFailureCause;
      /** 원본이 그대로라는 사실 (INV-3 · MH-7). */
      readonly preservedNotice: string;
      /** 이 갈래에서 먼저 해야 하는 일. 없으면 `null`이다. */
      readonly resolution: string | null;
      /** 실패해도 다시 시도할 수 있다 (§13). */
      readonly retry: CopyAction;
      /** clipboard가 막혀도 남는 길 (§7.5). 실패에는 언제나 있다. */
      readonly alternative: CopyAlternative;
    };

/** 복사 자리 하나. */
export interface CopyItemView {
  readonly target: CopyTarget;
  /** 버튼에 처음 보이는 이름. 상태와 무관하게 이 자리가 무엇인지 말한다. */
  readonly label: string;
  readonly body: CopyBody;
}

/**
 * 두 복사 자리 전체.
 *
 * 둘을 한 값으로 함께 내는 이유는 **한쪽의 상태가 다른 쪽을 가리지 않는다**는 것을 값의
 * 모양으로 만들기 위해서다 — 프롬프트 복사가 실패해도 전사 복사 버튼은 그대로 있다.
 */
export interface CopyPanelView {
  readonly prompt: CopyItemView;
  readonly transcript: CopyItemView;
}

/**
 * {@link copyPanel}이 보는 사실 전부.
 *
 * **여기 없는 것은 복사에 영향을 주지 않는다.** AI provider가 그것이다 — provider를 담을
 * 자리가 없으므로 provider 때문에 막히는 상태를 만들 수 없다 (MH-1 · MH-2 · INV-8).
 */
export interface CopyPanelInput {
  /** 아직 읽지 못했으면 `null`이다. */
  readonly recording: Recording | null;
  /** 지금 고른 mode. 프롬프트 복사가 이 값으로 만들어진다 (기존 세 mode 그대로다). */
  readonly mode: NoteMode;
  /** 이 화면이 건 복사 한 번. 두 자리 중 하나에만 해당한다. */
  readonly attempt: CopyAttempt;
}

/**
 * 읽어 온 값을 두 복사 자리의 상태로 바꾼다.
 *
 * 규칙이 하나 있다 — **이 화면이 실제로 건 복사의 결과가 다른 무엇보다 먼저다.** 복사에는
 * 저장되는 상태가 없으므로 그 결과를 아는 것은 이 호출뿐이다. 다른 녹음이나 다른 자리의
 * 복사는 이 자리와 상관이 없으므로 보지 않는다.
 */
export function copyPanel(input: CopyPanelInput): CopyPanelView {
  const { recording, mode, attempt } = input;

  return {
    prompt: item('prompt', recording, mode, attempt),
    transcript: item('transcript', recording, mode, attempt),
  };
}

function item(
  target: CopyTarget,
  recording: Recording | null,
  mode: NoteMode,
  attempt: CopyAttempt,
): CopyItemView {
  if (recording === null) {
    return { target, label: START_LABEL[target], body: { kind: 'loading' } };
  }
  return {
    target,
    label: START_LABEL[target],
    body: itemBody(target, recording, mode, mine(attempt, target, recording.id)),
  };
}

/** 이 자리, 이 녹음에 대한 시도만 본다. 다른 결과가 여기 보이지 않는다. */
function mine(attempt: CopyAttempt, target: CopyTarget, recordingId: string): CopyAttempt {
  if (attempt.kind === 'none') {
    return attempt;
  }
  return attempt.target === target && attempt.recordingId === recordingId
    ? attempt
    : NO_COPY_ATTEMPT;
}

function itemBody(
  target: CopyTarget,
  recording: Recording,
  mode: NoteMode,
  attempt: CopyAttempt,
): CopyBody {
  if (attempt.kind === 'copying') {
    return { kind: 'copying', text: COPYING_TEXT[target] };
  }

  if (attempt.kind === 'copied') {
    const portion = portionTaken(attempt.taken);
    const at = (kind: CopyAction['kind'], index: number) =>
      action(kind, target, recording.id, mode, index, portion.count);

    return {
      kind: 'copied',
      // 잘린 것을 온전한 것이라고 말하지 않는다 (`phase-prompt/05.6` 성공 기준 4).
      headline: portion.whole ? COPIED_HEADLINE : COPIED_PORTION_HEADLINE,
      text: portion.whole
        ? COPIED_TEXT[target]
        : portion.remaining === 0
          ? LAST_PORTION_TEXT[target]
          : COPIED_PORTION_TEXT[target],
      size: handoffSize(attempt.taken),
      portion,
      again: at('again', portion.index),
      // 남은 것이 있을 때만 다음 조각을 가리킨다. 없는 조각을 달라는 버튼을 만들지 않는다.
      next: portion.remaining === 0 ? null : at('next', portion.index + 1),
    };
  }

  if (attempt.kind === 'failed') {
    const cause = failureCause(attempt.failure);
    return {
      kind: 'failed',
      headline: FAILED_HEADLINE[target],
      failure: attempt.failure,
      cause,
      preservedNotice: COPY_PRESERVED_NOTICE,
      resolution: FAILURE_RESOLUTION[cause],
      // 실패한 **그 조각을** 다시 가지러 간다 — 건너뛴 조각은 사용자가 알아채지 못한 채 빠진다.
      retry: action('retry', target, recording.id, mode, attempt.portion, 1),
      alternative: ALTERNATIVE,
    };
  }

  // 아직 아무것도 하지 않았다. 복사할 재료가 있는지는 레코드가 말한다 (§7.2 · MH-5).
  if (recording.currentTranscriptId === null) {
    return { kind: 'nothingToCopy', text: NOTHING_TO_COPY_TEXT, hint: NOTHING_TO_COPY_HINT };
  }

  return {
    kind: 'notAsked',
    text: READY_TEXT[target],
    // 크기는 아직 알 수 없다 — 물어보기 전에는 backend만 안다. 그래서 첫 조각을 청한다.
    start: action('copy', target, recording.id, mode, FIRST_PORTION, 1),
  };
}

