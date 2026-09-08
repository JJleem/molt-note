/**
 * 화면이 그리다 죽었을 때 사람에게 무엇을 말할 것인가 (PRODUCT-SPEC §13).
 *
 * 이 파일에는 **문장을 만드는 규칙만** 있다. React도 DOM도 모른다 — `FailureNotice`와
 * `failureView`가 세운 것과 같은 형태다(그리는 것과 정하는 것을 가른다).
 *
 * ## 왜 이 자리가 필요한가 — 2026-09-08에 실제로 일어난 일
 *
 * 녹음 중에 화면이 통째로 사라졌다. React 트리가 에러 하나로 언마운트됐고, 그 앱에는
 * 그것을 받아 낼 자리가 없었다. **결과는 까만 화면이었고, 그 뒤에서 녹음은 2시간 4분 동안
 * 계속 돌고 있었다.** 사람은 그 사실을 알 수 없었고 Stop에 손이 닿지 않았다.
 *
 * 그래서 이 문장이 반드시 말해야 하는 것이 하나 있다 — **녹음은 화면이 죽어도 죽지 않는다.**
 * session은 Rust가 들고 있고 화면은 `capture_status`로 물어볼 뿐이다
 * (`src-tauri/src/commands/mod.rs:77` · R-001). 그래서 **다시 불러오면 진행 중인 녹음이
 * 그대로 돌아온다** (`RecordingScreen`이 마운트할 때 `captureStatus()`를 부른다).
 *
 * 이 사실을 화면이 말하지 않으면 사람은 앱을 강제로 죽인다. 그것이 이 문장의 존재 이유다.
 */

export interface CrashView {
  /** 무슨 일이 일어났는가. */
  readonly headline: string;
  /** 지금 어떤 상태이고 무엇을 할 수 있는가. */
  readonly message: string;
  /** 진행 중이던 녹음은 어떻게 되는가 — 이 화면이 반드시 말해야 하는 사실. */
  readonly recordingAnswer: string;
  /** 원인 문자열. 없으면 `null`이다 — 없는 것을 지어내지 않는다. */
  readonly detail: string | null;
}

export const CRASH_HEADLINE = 'This screen stopped working.';

export const CRASH_MESSAGE =
  'The app hit an error it did not expect while drawing this screen. Reloading rebuilds the screen from what the app already has.';

/**
 * **가장 중요한 문장이다.** 오늘의 사고가 이 한 줄이 없어서 일어났다.
 *
 * 녹음은 화면이 아니라 Rust가 들고 있으므로, 화면이 죽어도 계속되고 다시 불러오면 돌아온다.
 */
export const CRASH_RECORDING_ANSWER =
  'A recording in progress is still running and still being written to disk. Reloading brings it back so you can stop it — do not force quit.';

/**
 * 죽은 원인 하나를 화면 상태로 옮긴다.
 *
 * 원인을 읽을 수 없으면 `detail`은 `null`이다. **읽지 못한 것을 읽은 것처럼 적지 않는다.**
 */
export function toCrashView(error: unknown): CrashView {
  return {
    headline: CRASH_HEADLINE,
    message: CRASH_MESSAGE,
    recordingAnswer: CRASH_RECORDING_ANSWER,
    detail: describeCrash(error),
  };
}

/**
 * 알 수 없는 값에서 사람이 읽을 수 있는 원인 문자열을 뽑는다.
 *
 * `src/ipc/failure.ts`의 `describe`와 같은 태도다 — 그 함수는 내보내지지 않았고, 여기서
 * 다루는 값은 IPC 거절이 아니라 **렌더 중에 던져진 값**이므로 이 자리에 따로 둔다.
 * 스택을 함께 남기는 것이 그 차이다: 다음에 같은 일이 나면 어디서 났는지가 이 문자열에 있다.
 */
function describeCrash(error: unknown): string | null {
  if (error === null || error === undefined) {
    return null;
  }
  if (error instanceof Error) {
    return error.stack ?? `${error.name}: ${error.message}`;
  }
  if (typeof error === 'string') {
    return error.length > 0 ? error : null;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}
