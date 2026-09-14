/**
 * 녹음 화면의 상태 (PRODUCT-SPEC §5 B · §13 · §19).
 *
 * 화면이 보여주는 것은 §5 B가 정한 네 가지다 — **제목 · 선택된 microphone · 경과 시간 ·
 * Record/Pause/Resume/Stop**. 그 값을 만드는 규칙이 전부 여기 있고, 화면 컴포넌트는 그리기만
 * 한다. React도 DOM도 Tauri도 알지 않으므로 상태 전이 · 장치 선택 · 실패 표현이 **마이크 없이,
 * jsdom 없이** vitest로 그대로 판정된다 (§18).
 *
 * ## 진행 중인 녹음은 이 상태가 아니다 (R-001)
 *
 * 여기 있는 `session`은 **backend가 돌려준 답을 옮겨 적은 것**이지 녹음 그 자체가 아니다.
 * 녹음을 들고 있는 것은 Tauri managed state의 `Recorder`이며(`src-tauri/src/lib.rs`),
 * 화면은 `capture_status`로 물어본다. 그래서 화면을 떠났다 돌아와도 같은 답이 오고,
 * 이 값이 사라지는 것과 녹음이 사라지는 것은 아무 관계가 없다
 * (docs/ADR-0004-recording-session-lifecycle.md).
 *
 * 아직 물어보지 못한 상태(`session === null`)를 `idle`로 접지 않는다. 둘은 다른 사실이며,
 * 모르는 것을 "녹음 중이 아니다"로 적으면 화면이 사용자에게 거짓말을 하게 된다.
 *
 * ## 경과 시간은 여기서 만들지 않는다
 *
 * `elapsedLabel`은 Rust가 이미 만들어 보낸 문자열이다
 * (`src-tauri/src/domain/duration.rs` · `RecordingSession::elapsed_label`).
 * 초를 `0:07`로 바꾸는 규칙은 그 한 곳에만 있고 TypeScript에 다시 구현하지 않는다 —
 * 두 벌이 되면 조용히 갈라진다 (`tests/screen-boundary.test.ts`).
 *
 * ## 입력 레벨도 여기서 만들지 않는다
 *
 * dBFS 환산 · 판정 구간(-36 · -60) · 사람이 읽는 문장은 전부
 * `src-tauri/src/audio/level.rs` 한 곳에 있다 (ADR-0003 §16.4). 이 모듈이 하는 일은 그 값을
 * **어디에 어떤 갈래로 놓을지** 정하는 것뿐이다 — 숫자를 다시 재판정하지 않는다.
 * 여기서 정하는 것은 두 가지다: 값이 없는 것과 낮은 것을 구분하는 것, 그리고 녹음이 끝나기
 * 전에 경고를 낼지 정하는 것 ({@link inputLevelDisplay} · {@link inputLevelWarning}).
 */
import { toFailure, type Failure } from '../ipc/failure';
import type {
  CaptureMode,
  InputDevice,
  LiveTranscription,
  InputLevelVerdict,
  SessionState,
  SessionStatus,
  StoppedRecording,
  TranscriptSegment,
} from '../ipc/types';
import { MISSING_DEFAULT_MICROPHONE_LABEL, resolveDefaultMicrophone } from './defaultMicrophone';

/**
 * 이 녹음에 쓸 microphone이 지금 어떤 상태인가.
 *
 * 저장된 default 값(§5 D)을 지금 열거된 장치와 맞춰 본 결과다 — 그 판단은
 * {@link resolveDefaultMicrophone}이 하고 여기서 다시 하지 않는다.
 *
 * **`missing`에서 다른 장치로 바꾸지 않는다.** 저장된 장치가 빠졌을 때 첫 장치로 대체하면
 * 사용자가 고른 적 없는 마이크로 녹음이 시작되고, 장치가 바뀌었다는 사실 자체가 사라진다
 * (`phase-prompt/02-reliable-recording.md` Required Outcome 2).
 */
export type SelectedMicrophone =
  /** 이 장치로 녹음한다. `fromSystemDefault`면 사용자가 고른 것이 아니라 시스템 기본 장치다. */
  | {
      readonly kind: 'selected';
      readonly deviceKey: string;
      readonly label: string;
      readonly fromSystemDefault: boolean;
    }
  /** 저장된 장치가 지금 목록에 없다. 저장된 키는 그대로 남는다. */
  | { readonly kind: 'missing'; readonly savedKey: string }
  /** 고를 수 있는 입력 장치가 하나도 없다. 실패가 아니라 사실이다. */
  | { readonly kind: 'none' }
  /** 아직 물어보지 못했거나(`failure === null`), 물어봤지만 답을 얻지 못했다. */
  | { readonly kind: 'unknown'; readonly failure: Failure | null };

/** 정지가 성공해서 저장된 녹음 (R-002). 이 값이 있으면 목록에도 있다. */
export interface SavedRecording {
  readonly id: string;
  readonly title: string;
  /** Rust가 만든 표시용 길이. 화면은 이 값을 그대로 쓴다. */
  readonly durationLabel: string;
}

/** 화면이 backend에 보내는 요청. 실패가 어느 요청의 것인지 구분하는 데 쓴다. */
export type RecordingAction = 'start' | 'pause' | 'resume' | 'stop' | 'status';

/**
 * 실패의 갈래 (§13).
 *
 * **`microphonePermission`과 `recordingStart`가 갈라져 있는 것이 이 타입의 핵심이다.**
 * 권한이 거부된 것과 녹음을 초기화하지 못한 것은 사용자가 할 일이 서로 다르다 — 하나는
 * 시스템 설정에서 접근을 허용해야 풀리고, 다른 하나는 다시 시도하거나 장치를 바꿔야 한다.
 * 둘을 한 덩어리로 보여주면 사용자는 없는 문제를 고치려 하게 된다.
 */
export type RecordingTroubleKind =
  | 'microphonePermission'
  | 'recordingStart'
  | 'recordingControl'
  | 'recordingStop'
  | 'sessionStatus';

/** 실패 하나가 화면에 놓이는 모습. 문장은 {@link FailureNotice}가 그리고, 갈래는 여기가 정한다. */
export interface RecordingTrouble {
  readonly kind: RecordingTroubleKind;
  /** 무엇을 하다 실패했는지 한 줄. `Failure.message`(원인)와 겹치지 않는다. */
  readonly headline: string;
  readonly failure: Failure;
}

/**
 * 녹음 화면의 상태 전부.
 *
 * `loading`·`failed` 같은 별도의 화면 상태를 두지 않는다 — **녹음 중에 상태 조회 한 번이
 * 실패했다고 해서 Stop 버튼이 화면에서 사라지면 안 되기 때문이다.** 알아내지 못한 것은
 * 각 자리의 "모른다"(`session === null` · `microphone.kind === 'unknown'`)로 남고,
 * 실패는 화면을 덮지 않고 `trouble`에 얹힌다.
 */
export interface RecordingView {
  /** 사용자가 입력한 제목. 비어 있으면 Rust가 저장 시각에서 만든다 (`stop_capture`). */
  readonly title: string;
  /** 무엇을 녹음할 것인가 (§22). **시작할 때 정해지고 녹음 중에는 바뀌지 않는다.** */
  readonly mode: CaptureMode;
  /** backend가 마지막으로 알려준 session 상태. 아직 물어보지 못했으면 `null`이다. */
  readonly session: SessionStatus | null;
  readonly microphone: SelectedMicrophone;
  /** 보낸 요청의 답을 기다리는 중이다. 그동안 같은 요청을 다시 보내지 않는다. */
  readonly busy: boolean;
  readonly trouble: RecordingTrouble | null;
  /** 마지막 정지로 저장된 녹음. 아직 없으면 `null`이다. */
  readonly saved: SavedRecording | null;
}

/** 화면을 열었을 때의 상태. 아직 아무것도 물어보지 않았다. */
export const INITIAL_RECORDING: RecordingView = {
  title: '',
  mode: 'microphone',
  session: null,
  microphone: { kind: 'unknown', failure: null },
  busy: false,
  trouble: null,
  saved: null,
};

/** 아직 경과 시간을 모를 때 그 자리에 놓는 값. **시간처럼 보이는 값을 지어내지 않는다.** */
export const UNKNOWN_ELAPSED = '—';

const STATE_TEXT: Record<SessionState, string> = {
  idle: '준비됨',
  // §5 B의 화면 스케치 그대로다. 깜빡이지 않는다 (§19 — 장식적 시각 효과를 넣지 않는다).
  recording: '● REC',
  paused: '일시정지',
  stopped: '정지됨',
};

/** 아직 상태를 물어보지 못했을 때의 표현. */
const UNKNOWN_STATE_TEXT = '확인 중…';

const TROUBLE_HEADLINE: Record<RecordingTroubleKind, string> = {
  microphonePermission: '마이크를 쓸 수 없다.',
  recordingStart: '녹음을 시작하지 못했다.',
  recordingControl: '녹음을 멈추거나 이어 가지 못했다.',
  recordingStop: '녹음을 정지하지 못했다.',
  sessionStatus: '녹음 상태를 읽지 못했다.',
};

/**
 * 저장된 default microphone과 지금 열거된 장치로 **이 녹음이 쓸 장치**를 정한다.
 *
 * 세 가지 경우가 서로 다른 결과가 된다.
 *
 * ```text
 * 저장된 장치가 지금 있다        → 그 장치로 녹음한다
 * 저장된 장치가 지금 없다        → missing. 바꿔치기하지 않고 그 사실을 보여준다
 * 아직 고른 적이 없다            → 시스템 기본 장치로 녹음한다 (숨기지 않고 그렇다고 말한다)
 * ```
 *
 * 마지막 경우는 사용자의 선택을 덮어쓰는 것이 아니다 — 고른 적이 없으므로 덮을 선택도 없다.
 * 그래도 무엇으로 녹음하는지는 화면에 그대로 적힌다 ({@link microphoneNotice}).
 */
export function selectedMicrophone(
  saved: string | null,
  devices: readonly InputDevice[],
): SelectedMicrophone {
  const resolved = resolveDefaultMicrophone(saved, devices);

  switch (resolved.kind) {
    case 'available':
      return {
        kind: 'selected',
        deviceKey: resolved.device.key,
        label: resolved.device.label,
        fromSystemDefault: false,
      };
    case 'missing':
      return { kind: 'missing', savedKey: resolved.key };
    case 'notChosen': {
      const fallback = devices.find((device) => device.isDefault) ?? devices[0];
      if (fallback === undefined) {
        return { kind: 'none' };
      }
      return {
        kind: 'selected',
        deviceKey: fallback.key,
        label: fallback.label,
        fromSystemDefault: true,
      };
    }
  }
}

/** 화면에 적히는 장치 이름. 이름을 모르는 경우에도 **지어내지 않는다.** */
export function microphoneLabel(microphone: SelectedMicrophone): string {
  switch (microphone.kind) {
    case 'selected':
      return microphone.label;
    case 'missing':
      // 저장된 것은 불투명한 키뿐이라 진짜 이름은 알 수 없다 (`defaultMicrophone.ts`).
      return MISSING_DEFAULT_MICROPHONE_LABEL;
    case 'none':
      return '쓸 수 있는 마이크 없음';
    case 'unknown':
      return microphone.failure === null ? '마이크 확인 중…' : '마이크를 알 수 없음';
  }
}

/** 장치에 대해 사용자에게 할 말. 할 말이 없으면 `null`이다. */
export function microphoneNotice(microphone: SelectedMicrophone): string | null {
  switch (microphone.kind) {
    case 'selected':
      return microphone.fromSystemDefault
        ? '기본 마이크가 지정되지 않아서 시스템 기본값을 쓴다. 고정하려면 설정에서 고른다.'
        : null;
    case 'missing':
      // 사실을 말하고, 그것 때문에 지금 무엇을 할 수 없는지도 말한다 (§13).
      return '저장된 마이크를 지금 쓸 수 없다. 선택은 그대로 남는다 — 녹음하려면 설정에서 다른 것을 고른다.';
    case 'none':
      return '쓸 수 있는 입력 장치가 없다. 마이크를 연결한 뒤 다시 불러온다.';
    case 'unknown':
      return microphone.failure === null ? null : '어떤 마이크인지 확인하지 못했다.';
  }
}

/** 이 장치로 녹음을 시작할 수 있는가. 고른 장치가 실제로 있을 때만 참이다. */
export function canRecord(microphone: SelectedMicrophone): boolean {
  return microphone.kind === 'selected';
}

/** 네 버튼이 각각 눌릴 수 있는가 (§5 B). */
export interface RecordingControls {
  readonly record: boolean;
  readonly pause: boolean;
  readonly resume: boolean;
  readonly stop: boolean;
}

const NO_CONTROLS: RecordingControls = { record: false, pause: false, resume: false, stop: false };

/**
 * 지금 누를 수 있는 버튼.
 *
 * backend의 전이 규칙(`src-tauri/src/audio/session.rs`)과 같은 모양이다 — 거절될 요청을
 * 화면이 보내지 않게 한다. 다만 **판정은 언제나 backend가 한다.** 여기서 막지 못한 요청이
 * 가더라도 그것은 실패 값으로 돌아오며, 그 실패도 화면에 보인다.
 *
 * 상태를 아직 모르거나 보낸 요청의 답을 기다리는 동안에는 아무것도 누를 수 없다.
 */
export function recordingControls(view: RecordingView): RecordingControls {
  if (view.busy || view.session === null) {
    return NO_CONTROLS;
  }

  const state = view.session.state;
  return {
    record: (state === 'idle' || state === 'stopped') && canRecord(view.microphone),
    pause: state === 'recording',
    resume: state === 'paused',
    stop: state === 'recording' || state === 'paused',
  };
}

/**
 * 화면에서 가장 크고 분명해야 하는 두 값 (§19).
 *
 * 둘 다 backend가 준 사실이다 — 상태도, 경과 시간 문자열도 여기서 만들지 않는다.
 */
export interface SessionDisplay {
  readonly stateText: string;
  /** Rust가 만든 문자열 그대로. */
  readonly elapsedLabel: string;
  /** 지금 녹음 중인가. 화면이 이 사실을 가장 분명히 보여준다. */
  readonly live: boolean;
}

export function sessionDisplay(view: RecordingView): SessionDisplay {
  const session = view.session;
  if (session === null) {
    return { stateText: UNKNOWN_STATE_TEXT, elapsedLabel: UNKNOWN_ELAPSED, live: false };
  }

  return {
    stateText: STATE_TEXT[session.state],
    elapsedLabel: session.elapsedLabel,
    live: session.state === 'recording',
  };
}

/**
 * 진행 중인 녹음의 입력 레벨이 아직 없을 때 그 자리에 놓는 문장.
 *
 * **이것은 "낮다"가 아니다.** 아직 샘플이 하나도 쓰이지 않은 순간(녹음을 막 시작했을 때)에
 * 레벨이 낮다고 말하면 화면이 사용자에게 거짓말을 하게 된다 (ADR-0003 §16.3). 모르는 것은
 * 모른다고 적는다 — `UNKNOWN_ELAPSED`가 시간에 대해 하는 일과 같다.
 */
export const UNKNOWN_LEVEL_TEXT = '입력 레벨을 아직 재지 않았다';

/**
 * 쓸 수 없을 만큼 낮을 때 **정지 전에** 보이는 경고 (ADR-0003 §16.1의 관측).
 *
 * 이 문장이 있는 이유는 하나다 — 2026-09-07에 사람이 51분을 녹음하고 전사를 4.3분 기다린
 * 뒤에야 소리가 담기지 않았다는 것을 알았다. 그때 알았어야 할 시점은 정지 전이다.
 *
 * **무엇이 낮은지는 이 문장이 말하지 않는다.** 수치와 갈래는 backend가 만든 문장이 함께
 * 보이며 ({@link inputLevelDisplay}), 여기서 하는 말은 *지금 무엇을 해야 하는가* 하나다.
 *
 * 녹음을 막지도, 멈추지도 않는다 (§16.5) — 이미 녹음된 것은 그대로 남고, 사람이 정한다.
 */
export const WEAK_LEVEL_WARNING =
  '이 녹음은 쓸 수 없을 수 있다. 정지하기 전에 마이크를 확인한다 — 지금까지 녹음된 것은 그대로 남는다.';

/**
 * 입력 레벨이 화면에 놓이는 모습.
 *
 * **값 없음과 낮음이 서로 다른 갈래다.** 하나로 접으면 방금 시작한 녹음이 곧바로 "낮음"으로
 * 보이고, 그 뒤로는 이 표시를 믿을 수 없게 된다.
 */
export interface InputLevelDisplay {
  /**
   * 이 자리를 화면에 두는가.
   *
   * 진행 중인 녹음이 없으면 둘 것이 없다 — 끝난 녹음의 레벨을 계속 붙들고 있지 않는다.
   */
  readonly shown: boolean;
  /** 어느 갈래인가. 아직 값이 없으면 `unknown`이며, 그것은 판정이 아니다. */
  readonly kind: 'unknown' | InputLevelVerdict;
  /** 사람이 읽는 문장. 값이 있으면 **backend가 만든 문장 그대로다.** */
  readonly text: string;
  /** 쓸 수 없을 만큼 낮은가. **값이 없는 것은 낮은 것이 아니다** — 그때는 거짓이다. */
  readonly weak: boolean;
}

/** 진행 중인 녹음인가. 정지한 뒤에는 backend가 레벨을 보내지 않는다. */
function inProgress(state: SessionState): boolean {
  return state === 'recording' || state === 'paused';
}

/**
 * 판정 하나가 "쓸 수 없을 만큼 낮은가".
 *
 * **여기에 임계값은 없다.** 어느 dBFS부터 낮은지는 `level.rs`가 정했고, 이 함수가 하는 일은
 * 그 판정 셋을 화면이 다루는 둘로 묶는 것뿐이다. 갈래가 하나 늘면 tsc가 여기서 먼저 멈춘다.
 */
function isWeak(verdict: InputLevelVerdict): boolean {
  switch (verdict) {
    case 'usable':
      return false;
    case 'low':
    case 'silent':
      return true;
  }
}

/**
 * backend가 마지막으로 말해 준 입력 레벨을 화면에 놓는다.
 *
 * 세 갈래가 서로 다른 결과가 된다.
 *
 * ```text
 * 값이 아직 없다 (null)   → unknown. 모른다고 적는다. 낮다고 말하지 않는다
 * 낮음 · 소리 없음        → 그 갈래 그대로. backend의 문장이 수치와 함께 보인다
 * 쓸 만함                 → 그 갈래 그대로. 경고는 없다
 * ```
 */
export function inputLevelDisplay(view: RecordingView): InputLevelDisplay {
  const session = view.session;
  if (session === null || !inProgress(session.state)) {
    return { shown: false, kind: 'unknown', text: UNKNOWN_LEVEL_TEXT, weak: false };
  }

  const level = session.level;
  if (level === null) {
    return { shown: true, kind: 'unknown', text: UNKNOWN_LEVEL_TEXT, weak: false };
  }

  return {
    shown: true,
    kind: level.verdict,
    // 수치도 갈래도 문장도 backend가 만든 것 그대로다 — 여기서 다시 만들지 않는다.
    text: level.message,
    weak: isWeak(level.verdict),
  };
}

/**
 * 지금 사용자에게 낼 경고. 할 말이 없으면 `null`이다.
 *
 * **녹음 중이 아니면 아무 말도 하지 않는다.** 끝난 녹음이나 아직 시작하지 않은 녹음에 대고
 * "정지 전에 확인하라"고 말하는 것은 사용자가 할 수 있는 일이 없는 경고이며, 그런 경고가
 * 화면에 남아 있으면 정작 녹음 중에 뜬 경고도 배경이 된다.
 *
 * 값을 모르는 동안에도 경고하지 않는다 — 모르는 것은 낮은 것이 아니다.
 */
export function inputLevelWarning(view: RecordingView): string | null {
  if (view.session?.state !== 'recording') {
    return null;
  }

  return inputLevelDisplay(view).weak ? WEAK_LEVEL_WARNING : null;
}

/**
 * 거절된 요청 하나를 화면 상태로 옮긴다 (§13).
 *
 * 권한 실패는 **어느 요청에서 왔든** 권한 실패다 — 그것을 판정하는 곳은 platform 경계이고
 * (`src-tauri/src/platform/microphone.rs`), 화면은 그 판정을 존중한다. 나머지는 무엇을 하다
 * 실패했는지로 갈린다.
 */
export function recordingTrouble(action: RecordingAction, error: unknown): RecordingTrouble {
  const failure = toFailure(error);
  const kind = failure.kind === 'microphonePermission' ? 'microphonePermission' : forAction(action);

  return { kind, headline: TROUBLE_HEADLINE[kind], failure };
}

function forAction(action: RecordingAction): RecordingTroubleKind {
  switch (action) {
    case 'start':
      // 권한 문제가 아닌 시작 실패는 **녹음 초기화 실패**다 — 장치를 열지 못했거나,
      // 출력 파일을 만들지 못했거나, 이미 녹음 중이다 (§13의 `recording initialization failure`).
      return 'recordingStart';
    case 'stop':
      return 'recordingStop';
    case 'status':
      return 'sessionStatus';
    case 'pause':
    case 'resume':
      return 'recordingControl';
  }
}

/**
 * backend가 알려준 session 상태를 옮겨 적는다.
 *
 * 상태를 다시 읽었다고 해서 **이전 실패가 사라지지는 않는다** — 권한 거부는 상태 조회가
 * 성공한다고 풀리지 않기 때문이다. 지워지는 것은 "상태를 읽지 못했다"는 실패뿐이며,
 * 그것은 방금 읽어서 더 이상 사실이 아니다.
 */
export function observedSession(view: RecordingView, session: SessionStatus): RecordingView {
  return {
    ...view,
    session,
    busy: false,
    trouble: view.trouble?.kind === 'sessionStatus' ? null : view.trouble,
  };
}

/**
 * 상태를 읽지 못했다.
 *
 * **마지막으로 알던 session을 버리지 않는다.** 녹음 중에 조회 한 번이 실패했다고 화면에서
 * Stop이 사라지면, 사용자는 진행 중인 녹음을 끝낼 수단을 잃는다 (R-001 · R-005).
 */
export function failedSession(view: RecordingView, error: unknown): RecordingView {
  return { ...view, busy: false, trouble: recordingTrouble('status', error) };
}

/** 저장된 설정과 지금 열거된 장치를 읽었다. */
export function observedDevices(
  view: RecordingView,
  savedMicrophone: string | null,
  devices: readonly InputDevice[],
): RecordingView {
  return { ...view, microphone: selectedMicrophone(savedMicrophone, devices) };
}

/**
 * 장치나 설정을 읽지 못했다.
 *
 * session 상태와 섞지 않는다 — 장치 목록을 읽지 못한 것과 녹음이 어떤 상태인지 모르는 것은
 * 서로 다른 사실이고, 진행 중인 녹음은 이것과 무관하게 계속된다.
 */
export function failedDevices(view: RecordingView, error: unknown): RecordingView {
  return { ...view, microphone: { kind: 'unknown', failure: toFailure(error) } };
}

/**
 * 고를 수 있는 녹음 모드와 그 이름.
 *
 * **이름은 여기 한 곳에만 있다** — 화면이 자기 문자열을 만들지 않는다.
 */
export const CAPTURE_MODES: readonly { readonly value: CaptureMode; readonly label: string }[] = [
  { value: 'microphone', label: '마이크' },
  { value: 'meeting', label: '회의' },
];

/** 이 모드가 무엇을 녹음하는지 한 줄로. **무엇이 파일에 들어가는지를 말한다.** */
export function modeHint(mode: CaptureMode): string {
  return mode === 'meeting'
    ? '내 마이크와 이 Mac에서 나는 소리를 함께 녹음한다 — 화상회의용이다.'
    : '마이크 하나만 녹음한다.';
}

/**
 * 녹음 모드를 골랐다.
 *
 * **녹음 중에는 바꾸지 않는다** — 한 파일 안에서 채널 수가 달라질 수 없기 때문이다.
 * 화면이 이미 그 버튼을 잠그지만, 판정은 여기 한 곳에 둔다.
 */
export function selectedMode(view: RecordingView, mode: CaptureMode): RecordingView {
  return canSelectMode(view) ? { ...view, mode } : view;
}

/** 지금 모드를 바꿀 수 있는가. 녹음이 진행 중이 아닐 때만이다. */
export function canSelectMode(view: RecordingView): boolean {
  if (view.busy || view.session === null) {
    return false;
  }
  return view.session.state === 'idle' || view.session.state === 'stopped';
}

/** 제목을 고쳤다. */
export function editedTitle(view: RecordingView, title: string): RecordingView {
  return { ...view, title };
}

/**
 * 요청을 보냈다. 답이 올 때까지 버튼이 잠긴다.
 *
 * 지난 실패는 여기서 지운다 — 방금 다시 눌렀으므로 그 실패는 더 이상 지금의 상태가 아니다.
 * 새 녹음을 시작할 때는 지난 녹음의 저장 결과도 함께 치운다.
 */
export function requestedAction(view: RecordingView, action: RecordingAction): RecordingView {
  return { ...view, busy: true, trouble: null, saved: action === 'start' ? null : view.saved };
}

/** 보낸 요청이 거절됐다. 어느 요청이었는지가 실패의 갈래가 된다. */
export function failedAction(
  view: RecordingView,
  action: RecordingAction,
  error: unknown,
): RecordingView {
  return { ...view, busy: false, trouble: recordingTrouble(action, error) };
}

/**
 * 정지가 성공했다 — **파일이 확정되고 확인되고 레코드로 저장됐다는 뜻이다** (R-002).
 *
 * 그래서 이 값이 오면 그 녹음은 Recordings 목록에도 있다. 화면은 그 사실을 보여주고
 * 목록으로 이어 준다.
 *
 * 제목은 비운다. 다음 녹음이 지난 녹음의 제목을 물려받지 않게 하기 위해서다 — 저장된 제목은
 * `saved`에 그대로 남아 있다.
 */
export function savedRecording(view: RecordingView, stopped: StoppedRecording): RecordingView {
  return {
    ...view,
    title: '',
    busy: false,
    trouble: null,
    saved: {
      id: stopped.recording.id,
      title: stopped.recording.title,
      // Rust가 만든 값 그대로다. 여기서 길이를 다시 계산하지 않는다.
      durationLabel: stopped.recording.durationLabel,
    },
  };
}

// --- 레벨을 눈으로 볼 수 있게 (2026-09-08) ------------------------------------------------

/**
 * 막대 하나가 화면에 놓이는 모습.
 *
 * **이 모듈은 dBFS도 판정 구간도 모른다** (§16.4 · INV-9). 채우는 길이는 backend가 이미
 * 계산해 보낸 값(`meterFill`)이고, 갈래도 backend가 판정한 것이다 — 여기서 하는 일은
 * *언제 이 자리를 두는가* 하나뿐이며, 그것은 문장을 두는 규칙과 같다.
 *
 * ## 왜 막대가 필요한가
 *
 * 이 자리에는 문장 한 줄만 있었다. 2026-09-08에 녹음된 2시간 회의가 판정 경계 **바로
 * 아래**였고, 문장만으로는 그것이 얼마나 낮은지 — 조금만 올리면 되는지 — 알 수 없었다.
 */
export interface InputLevelMeter {
  /** 채우는 길이 (`0..=1`). **backend가 준 값 그대로다.** */
  readonly fill: number;
  /** 쓸 수 없을 만큼 낮은가. 색을 정하는 것은 화면이 아니라 이 값이다. */
  readonly weak: boolean;
}

/**
 * 레벨 하나를 막대로 옮긴다. 둘 자리가 아니면 `null`이다.
 *
 * 조건은 {@link inputLevelDisplay}와 같다 — 진행 중인 녹음이 없거나 아직 잰 값이 없으면
 * 막대도 없다. **재지 않은 것을 0으로 그리지 않는다.**
 */
export function inputLevelMeter(view: RecordingView): InputLevelMeter | null {
  const session = view.session;
  if (session === null || !inProgress(session.state)) {
    return null;
  }

  const level = session.level;
  if (level === null) {
    return null;
  }

  return { fill: level.meterFill, weak: isWeak(level.verdict) };
}

/**
 * 녹음 중에 받아 적은 것을 화면에 놓는 규칙 (2026-09-14).
 *
 * ## 무엇을 말할지가 상태마다 다르다
 *
 * ```text
 * 받아 적는 중 · 아직 문장 없음   "받아 적는 중…"  — 30초쯤 걸린다는 사실을 함께
 * 받아 적는 중 · 문장 있음        문장을 보여준다
 * 그만둠                          왜 그만뒀는지. **녹음은 계속된다는 말과 함께**
 * 돌지 않음                       아무것도 두지 않는다
 * ```
 *
 * **그만둔 것은 실패 화면이 아니다** (INV-8). 녹음은 그대로 돌고 있고, 정지한 뒤
 * 전사 탭에서 다시 전사하면 된다 — 그 사실을 말하지 않으면 사용자는 녹음까지
 * 잘못된 줄 안다.
 */
export type LiveLinesView =
  | { readonly kind: 'hidden' }
  | { readonly kind: 'waiting'; readonly text: string }
  | { readonly kind: 'lines'; readonly lines: readonly TranscriptSegment[] }
  | { readonly kind: 'gaveUp'; readonly text: string; readonly failure: Failure | null };

/** 아직 첫 문장이 나오기 전에 놓는 말. **얼마나 기다리는지 함께 말한다.** */
export const LIVE_WAITING_TEXT = '받아 적는 중… 첫 문장은 30초쯤 뒤에 나온다.';

/** 받아 적기를 그만뒀다. **녹음은 계속된다는 것이 이 문장의 핵심이다.** */
export const LIVE_GAVE_UP_TEXT =
  '지금은 받아 적지 않는다. 녹음은 계속되고 있으며, 정지한 뒤 전사할 수 있다.';

export function liveLines(live: LiveTranscription | null): LiveLinesView {
  if (live === null || live.state === 'idle') {
    return { kind: 'hidden' };
  }
  if (live.state === 'gaveUp') {
    return { kind: 'gaveUp', text: LIVE_GAVE_UP_TEXT, failure: live.failure };
  }
  if (live.lines.length === 0) {
    return { kind: 'waiting', text: LIVE_WAITING_TEXT };
  }
  return { kind: 'lines', lines: live.lines };
}
