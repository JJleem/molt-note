/**
 * webview의 clipboard **쓰기**를 아는 유일한 자리 (docs/ADR-0010-manual-ai-handoff.md §7 ·
 * `phase-prompt/05.5` R-4 · 요구 5 · PRODUCT-SPEC §13 · INV-10).
 *
 * 저장소에서 clipboard API를 실제로 집는 코드는 이 파일 하나에 있고, 화면은 {@link copyText}를
 * 통해서만 복사한다. React 컴포넌트 여기저기에서 직접 부르면 실패를 다루는 방식이 자리마다
 * 달라지고, 플랫폼이 이 능력을 주지 않을 때 갈아 끼울 자리도 사라진다
 * (`tests/screen-boundary.test.ts`가 `src/` 전체에서 그것을 지킨다).
 *
 * 이름을 `platform/`으로 고른 이유는 `src-tauri/src/platform/`이 이미 같은 뜻으로 쓰이기
 * 때문이다 — **플랫폼 지식이 갇혀 있는 경계**다 (§7.2).
 *
 * ## 실패는 던지지 않고 값으로 돌아온다 (§13)
 *
 * {@link copyText}는 어떤 경우에도 예외를 던지지 않는다. 돌려주는 것은 성공 또는 {@link Failure}
 * 하나이며, 그 안에 §13의 세 답이 이미 들어 있다 — **무엇이 실패했는가 · 원본은 안전한가 ·
 * 다시 시도할 수 있는가.** 실패를 `console`로 흘리는 경로를 만들지 않는다.
 *
 * **새 `FailureKind`를 만들지 않는다** (§7.2). `FailureKind`는 Rust의 종류와 1:1이고 예외는
 * 프론트 경계에서만 만들어지는 `unexpected` 하나다 (`tests/ipc-boundary.test.ts`가 양방향으로
 * 강제한다). clipboard 실패는 Rust에서 나지 않으므로 종류를 더하는 대신 `unexpected`에
 * **무엇을 못 했는지 말하는 message**와 {@link CLIPBOARD_TROUBLE_DETAIL} 표시를 싣는다.
 *
 * ## 이 능력이 있다고 단정하지 않는다 (§7.4 — UNVERIFIED)
 *
 * Tauri v2 webview(macOS WKWebView · Windows WebView2)에서 이 API가 이 앱의 origin에서 실제로
 * 동작하는지, 어떤 조건(secure context · 사용자 제스처)을 요구하는지, 거절할 때 어떤 값이
 * 오는지는 **이 저장소가 확인하지 못했다.** 그래서 이 경계는 "동작한다"가 아니라 **"동작하지
 * 않을 수 있다"를 전제로** 만들어졌다 — 능력이 아예 없는 경우와 거절된 경우를 갈라 값으로
 * 돌려주고, 그때 사용자가 갈 수 있는 다른 길(Export for AI)은 화면 값으로 남는다
 * (`src/screens/copyView.ts` · §7.5).
 *
 * ## 테스트는 시스템 clipboard를 건드리지 않는다
 *
 * 쓰는 대상은 {@link ClipboardWriter} 하나이며 인자로 넘길 수 있다. 자동 테스트는 언제나
 * test double을 넘기므로 실제 clipboard에 닿지 않는다 (`clipboard.test.ts`).
 */
import type { Failure } from '../ipc/failure';

/**
 * 문자열 하나를 clipboard에 쓰는 능력.
 *
 * 이 앱이 clipboard에 요구하는 것은 이것뿐이다 — **읽기는 이 Phase에 없다** (§11).
 * 인터페이스가 이 모양이므로 테스트는 double을 넘기고, 플랫폼이 이 능력을 주지 않으면
 * 구현을 갈아 끼우는 것으로 끝난다 (§7.3의 B · C).
 */
export interface ClipboardWriter {
  writeText(text: string): Promise<void>;
}

/** 복사 한 번의 결과. 실패는 던져지지 않고 이 값 안에 있다 (§13). */
export type CopyResult =
  | { readonly ok: true }
  | { readonly ok: false; readonly failure: Failure };

/**
 * clipboard가 하지 못한 일의 갈래.
 *
 * ```text
 * unavailable  이 창에 clipboard 쓰기 능력이 아예 없다 — 눌러도 달라지지 않는다
 * rejected     능력은 있는데 이번 쓰기가 거절됐다 — 다시 눌러 볼 수 있다
 * ```
 *
 * 둘을 하나로 접지 않는 이유는 **사용자가 할 수 있는 일이 다르기 때문이다.**
 */
export type ClipboardTrouble = 'unavailable' | 'rejected';

/**
 * clipboard 실패라는 것을 `detail`에 남기는 표시.
 *
 * 종류를 늘리지 않으면서 갈래를 잃지 않기 위한 자리이며, `notionSyncView`가 Rust의
 * `needsConfirmation=`을 읽는 것과 같은 방식이다. 이 표시를 만드는 곳도 읽는 곳도
 * ({@link clipboardTrouble}) 이 모듈이다.
 */
export const CLIPBOARD_TROUBLE_DETAIL = 'clipboard=';

/**
 * 문자열 하나를 clipboard에 쓴다. **이 앱에서 clipboard에 쓰는 유일한 경로다.**
 *
 * `writer`를 넘기지 않으면 이 창의 clipboard를 쓴다. 자동 테스트는 언제나 double을 넘긴다.
 * 능력이 없으면 `unavailable`, 쓰다 거절되면 `rejected`로 돌아오며 **예외는 나가지 않는다.**
 */
export async function copyText(
  text: string,
  writer: ClipboardWriter | null = systemClipboard(),
): Promise<CopyResult> {
  if (writer === null) {
    return { ok: false, failure: unavailableFailure() };
  }
  try {
    await writer.writeText(text);
    return { ok: true };
  } catch (error) {
    return { ok: false, failure: rejectedFailure(error) };
  }
}

/**
 * 이 창의 clipboard 쓰기 능력. 없으면 `null`이다.
 *
 * **저장소에서 webview의 clipboard API를 집는 자리는 이 함수 한 줄뿐이다** (§7.2 · INV-10).
 * 여기서 `null`이 나오는 것은 오류가 아니라 사실이며, 그것을 실패 **값**으로 옮기는 것은
 * {@link copyText}다.
 */
export function systemClipboard(): ClipboardWriter | null {
  return clipboardWriter(typeof navigator === 'undefined' ? null : navigator.clipboard);
}

/**
 * 주어진 값이 실제로 문자열을 쓸 수 있으면 {@link ClipboardWriter}로 만든다.
 *
 * 있는지 없는지를 **부르기 전에** 판정한다 — 없는 것을 부르다 나는 예외와, 있는데 거절된
 * 것은 사용자에게 다른 상황이기 때문이다. `this`를 유지한 채 부르므로 원래 객체의 메서드가
 * 그대로 동작한다.
 */
export function clipboardWriter(candidate: unknown): ClipboardWriter | null {
  if (candidate === null || candidate === undefined) {
    return null;
  }
  const api = candidate as { writeText?: unknown };
  if (typeof api.writeText !== 'function') {
    return null;
  }
  const write = api.writeText as (text: string) => unknown;
  return {
    async writeText(text: string): Promise<void> {
      await write.call(api, text);
    },
  };
}

/**
 * 이 실패가 clipboard 때문인가, 그렇다면 어느 갈래인가.
 *
 * 복사 경로에는 clipboard 말고도 실패할 자리가 있다 — 복사할 문자열을 만드는 command다.
 * 그 둘은 사용자가 할 일이 다르므로, 화면이 갈라 보여줄 수 있게 이 판정을 값으로 내준다.
 * 모르는 값을 clipboard 실패로 읽지 않는다.
 */
export function clipboardTrouble(failure: Failure | null): ClipboardTrouble | null {
  const detail = failure?.detail;
  if (detail === null || detail === undefined || !detail.startsWith(CLIPBOARD_TROUBLE_DETAIL)) {
    return null;
  }
  const rest = detail.slice(CLIPBOARD_TROUBLE_DETAIL.length);
  if (rest === 'unavailable') {
    return 'unavailable';
  }
  // 거절된 경우에는 원인 문자열이 뒤에 붙는다.
  return rest === 'rejected' || rest.startsWith('rejected:') ? 'rejected' : null;
}

/** 이 창에 쓸 수단 자체가 없다. 눌러도 달라지지 않으므로 다른 길을 안내하는 것은 화면의 일이다. */
function unavailableFailure(): Failure {
  return {
    kind: 'unexpected',
    message: '이 창에서는 클립보드에 쓸 수 없다.',
    detail: `${CLIPBOARD_TROUBLE_DETAIL}unavailable`,
    // 아무것도 읽지도 쓰지도 않았다 (§7.5 · MH-7).
    sourceDataSafe: true,
    // 눌러 볼 수는 있다 — 같은 결과가 나올 뿐이며, 화면이 다른 길을 함께 보인다.
    retryable: true,
  };
}

/** 쓰다 거절됐다. 무엇이 거절했는지는 `detail`에 그대로 남긴다 (§7.4). */
function rejectedFailure(error: unknown): Failure {
  const cause = causeOf(error);
  return {
    kind: 'unexpected',
    message: '클립보드에 쓰지 못했다.',
    detail:
      cause === null
        ? `${CLIPBOARD_TROUBLE_DETAIL}rejected`
        : `${CLIPBOARD_TROUBLE_DETAIL}rejected: ${cause}`,
    sourceDataSafe: true,
    retryable: true,
  };
}

/**
 * 거절 값에서 사람이 읽을 수 있는 원인을 뽑는다.
 *
 * **이름을 버리지 않는다.** 어떤 예외가 오는지가 §7.4의 UNVERIFIED 중 하나이므로,
 * 실제 환경에서 나온 이름(`NotAllowedError` 계열인지)이 `detail`에 남아 있어야 P11이 그것을
 * 사실로 적을 수 있다. 지어내지 않고 온 값을 그대로 옮긴다.
 */
function causeOf(error: unknown): string | null {
  if (error instanceof Error) {
    return error.name === '' ? error.message : `${error.name}: ${error.message}`;
  }
  if (typeof error === 'string' && error.length > 0) {
    return error;
  }
  return null;
}
