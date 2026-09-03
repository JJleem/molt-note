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
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { InputDevice, SessionState, SessionStatus, StoppedRecording } from '../ipc/types';
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
  session: null,
  microphone: { kind: 'unknown', failure: null },
  busy: false,
  trouble: null,
  saved: null,
};

/** 아직 경과 시간을 모를 때 그 자리에 놓는 값. **시간처럼 보이는 값을 지어내지 않는다.** */
export const UNKNOWN_ELAPSED = '—';

const STATE_TEXT: Record<SessionState, string> = {
  idle: 'Ready',
  // §5 B의 화면 스케치 그대로다. 깜빡이지 않는다 (§19 — 장식적 시각 효과를 넣지 않는다).
  recording: '● REC',
  paused: 'Paused',
  stopped: 'Stopped',
};

/** 아직 상태를 물어보지 못했을 때의 표현. */
const UNKNOWN_STATE_TEXT = 'Checking…';

const TROUBLE_HEADLINE: Record<RecordingTroubleKind, string> = {
  microphonePermission: 'Microphone access is not available.',
  recordingStart: 'The recording could not be started.',
  recordingControl: 'The recording could not be paused or resumed.',
  recordingStop: 'The recording could not be stopped.',
  sessionStatus: 'The recording status could not be read.',
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
      return 'No microphone available';
    case 'unknown':
      return microphone.failure === null ? 'Checking microphone…' : 'Microphone unknown';
  }
}

/** 장치에 대해 사용자에게 할 말. 할 말이 없으면 `null`이다. */
export function microphoneNotice(microphone: SelectedMicrophone): string | null {
  switch (microphone.kind) {
    case 'selected':
      return microphone.fromSystemDefault
        ? 'No default microphone is set, so the system default is used. Choose one in Settings to pin it.'
        : null;
    case 'missing':
      // 사실을 말하고, 그것 때문에 지금 무엇을 할 수 없는지도 말한다 (§13).
      return 'The saved microphone is not available right now. It stays chosen — pick another one in Settings to record.';
    case 'none':
      return 'No input device is available. Connect a microphone, then reload.';
    case 'unknown':
      return microphone.failure === null ? null : 'The microphone could not be determined.';
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
