/**
 * 만들어진 파일이 **놓인 자리를 여는** 자리 (`phase-prompt/05.6` 성공 기준 2 · R-4 ·
 * PRODUCT-SPEC §13 · INV-3 · INV-10).
 *
 * 이 자리가 답해야 하는 질문은 셋이다 — **지금 그 자리를 열 수 있는가** · **여는 중인가** ·
 * **열지 못했을 때 무엇이 그대로인가**. 셋 다 값이며, React도 DOM도 Tauri도 OS도 알지 않으므로
 * vitest로 그대로 판정된다 (§18 · `copyView.ts` · `exportView.ts`와 같은 형태다).
 *
 * ## 왜 이 자리가 생겼는가 (R-4)
 *
 * export는 이미 만들어진 파일의 **전체 경로를 그대로 보여 준다** — 그것이 사용자가 파일을 찾는
 * 길이라고 보았기 때문이다 (`exportView.ts` · ADR-0009 §4.1). 그럼에도 2026-09-05의 첫
 * 실사용에서 사람이 그 파일에 도달하지 못했다: macOS에서 `~/Library`는 Finder 기본 숨김이라
 * 경로를 알아도 걸어 들어갈 수 없다.
 *
 * **그래서 경로를 없애지 않는다.** 여는 수단은 경로의 **대체가 아니라 추가**이며, 그 사실이 이
 * 모듈의 값에 그대로 있다 — 열지 못한 상태에서도 사용자에게 남는 길은 여전히 그 경로다
 * ({@link SHOW_FILE_RESOLUTION}).
 *
 * ## 이 모듈은 어느 OS인지도, 무엇을 부르는지도 알지 않는다 (INV-10)
 *
 * 여기에 `Finder`도 `Explorer`도 `open`도 없다. 그 지식은 backend의 platform 경계 하나에 있고
 * (`src-tauri/src/platform/file_manager.rs`), 이 모듈이 만드는 것은 **무엇을 눌렀는가**와
 * **무엇이 그대로인가**뿐이다. 실제로 부르는 것은 화면 컴포넌트다
 * (`RecordingDetailScreen`의 `beginShowFile`).
 *
 * ## 여는 일은 아무것도 바꾸지 않는다 (INV-3)
 *
 * 성공해도 실패해도 파일 · 녹음 · 전사 · 노트는 그대로다 — 이 경로에는 무엇을 쓰거나 지우는
 * 수단이 없기 때문이다 (`src-tauri/src/commands/saved_file.rs`). 그 사실이 실패 상태의 값으로
 * 남는다 ({@link SHOW_FILE_PRESERVED_NOTICE}).
 */
import { toFailure, type Failure } from '../ipc/failure';

/**
 * 이 자리에서 사용자가 할 수 있는 동작 하나.
 *
 * **함수가 아니라 값이다** — 순수 모듈이 command를 알지 않기 때문이며, 그래서 "지금 열 수단이
 * 그 자리에 있는가"가 DOM 없이 판정된다 (`ExportAction` · `AiExportAction`과 같은 형태다).
 *
 * 실려 있는 것이 `recordingId`가 아니라 `path`인 이유는 여는 대상이 **방금 만들어진 파일
 * 하나**이기 때문이다. 그 값은 backend가 준 것 그대로이며 화면이 지어내지 않는다 —
 * 같은 이름이 있었으면 backend가 번호를 붙였고, 실제로 쓰인 경로가 함께 왔다 (ADR-0009 §4.3).
 */
export interface ShowFileAction {
  readonly kind: 'show' | 'retry';
  readonly label: string;
  readonly path: string;
}

/** 화면에 처음 보이는 이름. **어느 OS의 파일 관리자인지 말하지 않는다** (INV-10). */
export const SHOW_FILE_LABEL = 'Show this file';

/** 실패한 뒤의 이름. 사용자에게 다른 상황이므로 같은 글자를 쓰지 않는다. */
export const SHOW_FILE_RETRY_LABEL = 'Try showing it again';

/**
 * 이 자리가 무엇을 하는가.
 *
 * **파일을 여는 것이 아니라 파일이 있는 자리를 연다**는 것과, 그래도 경로는 그대로 남는다는
 * 것이 한 문장에 있다.
 */
export const SHOW_FILE_TEXT =
  'Opens the folder this file is in and points at it. Some systems keep that folder hidden, so the full path stays here as well.';

/** 여는 중. 창이 뜨기까지 이 자리가 멎은 것처럼 보이지 않게 한다. */
export const SHOWING_FILE_TEXT = 'Opening the folder…';

/** 무엇을 하다 실패했는가 (§13). 원인은 {@link Failure}가 말한다. */
export const SHOW_FILE_FAILED_HEADLINE = 'The folder could not be opened.';

/**
 * 실패가 무엇을 남겼는지 (§13 · INV-3).
 *
 * **"복구했다"거나 "정리했다"고 말하지 않는다.** 이 경로는 창 하나를 여는 일이므로 실패했을 때
 * 바뀐 것이 아무것도 없다.
 */
export const SHOW_FILE_PRESERVED_NOTICE =
  'The file itself is untouched. Nothing was moved, renamed, or deleted — this only asks the system to open a window.';

/**
 * 그래서 지금 무엇을 하면 되는가.
 *
 * **여는 수단은 경로의 대체가 아니라 추가다** — 그것이 없거나 실패해도 파일에 도달하는 길은
 * 남아 있고, 그 길이 바로 위에 보이는 전체 경로다.
 */
export const SHOW_FILE_RESOLUTION =
  'The full path above still says where the file is, and you can open that location with it.';

/**
 * 이 화면이 건 열기 한 번.
 *
 * `exportView`의 것과 같은 규약이다 — **상태를 물어보지 않는다.** `show_saved_file`은 열었거나
 * 열지 못했거나로 끝나므로 진행 상황은 이 호출 하나의 결과이며, 그 세 갈래가 이 타입이다.
 */
export type ShowFileAttempt =
  | { readonly kind: 'none' }
  | { readonly kind: 'showing'; readonly path: string }
  | { readonly kind: 'failed'; readonly path: string; readonly failure: Failure };

/** 아무것도 하지 않은 상태. 화면이 열렸을 때의 값이다. */
export const NO_SHOW_FILE_ATTEMPT: ShowFileAttempt = { kind: 'none' };

/** 열기를 시작했을 때 만드는 값. */
export function showingFile(path: string): ShowFileAttempt {
  return { kind: 'showing', path };
}

/** 열지 못했을 때 만드는 값 (§13). **console로 흘려보내지 않는다.** */
export function failedShowFile(path: string, error: unknown): ShowFileAttempt {
  return { kind: 'failed', path, failure: toFailure(error) };
}

/** 열지 못한 사실 하나 (§13의 세 질문에 대한 답이 이미 {@link Failure} 안에 있다). */
export interface ShowFileTrouble {
  /** 무엇을 하다 실패했는가. 색이 아니라 이 문장이 그것을 말한다. */
  readonly headline: string;
  readonly failure: Failure;
  /** 파일도 저장된 것도 그대로다 (INV-3). */
  readonly preservedNotice: string;
  /** 그래도 파일에 도달하는 길이 남아 있다 — 위에 보이는 경로다. */
  readonly resolution: string;
}

/**
 * 파일 하나가 놓인 자리를 여는 수단.
 *
 * **`action`은 언제나 있다.** 여는 중이든 실패한 뒤든 이 자리는 사라지지 않으며, 그래서
 * 사용자가 한 번 실패했다고 길을 잃지 않는다.
 */
export interface ShowFileView {
  readonly action: ShowFileAction;
  /**
   * 지금 이 자리가 무엇인지 말하는 문장.
   *
   * 여는 중에는 그 사실을 말한다 — **색이나 회전하는 표시 하나로만 말하지 않는다**
   * (`phase-prompt/05.5` 요구 12의 규약과 같다).
   */
  readonly text: string;
  /** 지금 여는 중인가. 그동안에도 경로와 파일 이름은 그대로 보인다. */
  readonly showing: boolean;
  /** 열지 못했을 때만 있다. 그 밖에는 `null`이다. */
  readonly trouble: ShowFileTrouble | null;
}

/**
 * 만들어진 파일 하나에 대한 **여는 수단**을 만든다.
 *
 * 이 파일에 대한 시도만 본다 — 다른 파일을 열다 실패한 사실이 이 자리에 보이지 않는다
 * (`exportView`의 `mine`과 같은 규칙이며, 여기서는 가리키는 것이 경로다).
 */
export function showFile(path: string, attempt: ShowFileAttempt): ShowFileView {
  const mine = attempt.kind !== 'none' && attempt.path === path;
  const failure = mine && attempt.kind === 'failed' ? attempt.failure : null;
  const showing = mine && attempt.kind === 'showing';

  return {
    action: {
      kind: failure === null ? 'show' : 'retry',
      label: failure === null ? SHOW_FILE_LABEL : SHOW_FILE_RETRY_LABEL,
      path,
    },
    text: showing ? SHOWING_FILE_TEXT : SHOW_FILE_TEXT,
    showing,
    trouble:
      failure === null
        ? null
        : {
            headline: SHOW_FILE_FAILED_HEADLINE,
            failure,
            preservedNotice: SHOW_FILE_PRESERVED_NOTICE,
            resolution: SHOW_FILE_RESOLUTION,
          },
  };
}
