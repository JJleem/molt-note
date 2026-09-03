// 녹음 화면의 상태 전환 테스트.
//
// 시작 → 일시정지 → 재개 → 정지 동안 화면이 무엇을 보여주는지, 저장된 default 장치가
// 사라졌을 때 무엇이 사용자에게 보이는지, 그리고 실패가 어느 갈래로 도착하는지 본다.
// **마이크도 마이크 권한도 DOM도 필요하지 않다** (PRODUCT-SPEC §18).
//
// 실제 마이크 입력 품질과 녹음된 소리는 이 테스트가 판정하지 않는다 — Human Review 항목이다.
import { describe, expect, it } from 'vitest';
import type { Failure } from '../ipc/failure';
import type { InputDevice, Recording, SessionStatus, StoppedRecording } from '../ipc/types';
import {
  INITIAL_RECORDING,
  UNKNOWN_ELAPSED,
  canRecord,
  editedTitle,
  failedAction,
  failedDevices,
  failedSession,
  microphoneLabel,
  microphoneNotice,
  observedDevices,
  observedSession,
  recordingControls,
  recordingTrouble,
  requestedAction,
  savedRecording,
  selectedMicrophone,
  sessionDisplay,
  type RecordingView,
} from './recordingView';

const BUILT_IN: InputDevice = { key: 'builtin', label: 'MacBook Pro Microphone', isDefault: true };
const HEADSET: InputDevice = { key: 'headset', label: 'USB Headset', isDefault: false };

/** backend가 돌려주는 상태. **경과 시간 문자열은 Rust가 만든다.** */
function status(state: SessionStatus['state'], elapsedMs: number, label: string): SessionStatus {
  return { state, elapsedMs, elapsedLabel: label };
}

const IDLE = status('idle', 0, '0:00');
const RECORDING = status('recording', 7_000, '0:07');
const PAUSED = status('paused', 7_000, '0:07');

const permissionDenied: Failure = {
  kind: 'microphonePermission',
  message: '마이크에 접근할 수 없다. 시스템 설정에서 접근을 허용해야 한다.',
  detail: null,
  sourceDataSafe: true,
  retryable: false,
};

const initializationFailure: Failure = {
  kind: 'storage',
  message: '녹음 파일을 만들지 못했다.',
  detail: 'permission denied while creating the recordings directory',
  sourceDataSafe: true,
  retryable: true,
};

const RECORD: Recording = {
  id: 'rec-1',
  title: '3DGS Study #04',
  createdAt: '2026-09-02T10:00:00.000Z',
  updatedAt: '2026-09-02T10:00:00.000Z',
  durationMs: 3_151_000,
  durationLabel: '52:31',
  audioPath: '/Users/someone/Library/Application Support/molt-note/recordings/capture-1.wav',
  audioFormat: 'wav',
  microphone: 'MacBook Pro Microphone',
  currentTranscriptId: null,
  transcriptionStatus: 'none',
  aiStatus: 'none',
  notionStatus: 'none',
};

const STOPPED: StoppedRecording = {
  recording: RECORD,
  capture: {
    deviceLabel: 'MacBook Pro Microphone',
    outputPath: RECORD.audioPath,
    format: '48000 Hz · mono · 16-bit PCM · WAV',
    sampleRateHz: 48_000,
    channels: 1,
    bitsPerSample: 16,
    container: 'WAV',
    byteSize: 302_496_044,
    durationMs: 3_151_000,
    durationLabel: '52:31',
  },
};

/** 장치와 상태를 모두 읽은 화면. 아직 녹음하지 않았다. */
function ready(saved: string | null = BUILT_IN.key): RecordingView {
  return observedDevices(observedSession(INITIAL_RECORDING, IDLE), saved, [BUILT_IN, HEADSET]);
}

/** 녹음 중인 화면. */
function recording(): RecordingView {
  return observedSession(requestedAction(ready(), 'start'), RECORDING);
}

describe('화면을 열었을 때', () => {
  it('아직 아무것도 모른다 — 모르는 것을 idle로 접지 않는다', () => {
    expect(INITIAL_RECORDING.session).toBeNull();
    expect(INITIAL_RECORDING.microphone).toEqual({ kind: 'unknown', failure: null });
    expect(INITIAL_RECORDING.trouble).toBeNull();
    expect(INITIAL_RECORDING.saved).toBeNull();
    expect(INITIAL_RECORDING.title).toBe('');
  });

  it('상태를 모르는 동안에는 시간처럼 보이는 값을 지어내지 않는다', () => {
    const display = sessionDisplay(INITIAL_RECORDING);

    expect(display.elapsedLabel).toBe(UNKNOWN_ELAPSED);
    expect(display.live).toBe(false);
    expect(display.stateText.length).toBeGreaterThan(0);
  });

  it('상태를 모르는 동안에는 아무 버튼도 누를 수 없다', () => {
    expect(recordingControls(INITIAL_RECORDING)).toEqual({
      record: false,
      pause: false,
      resume: false,
      stop: false,
    });
  });
});

describe('선택된 microphone', () => {
  it('저장된 장치가 지금 있으면 그 장치로 녹음한다', () => {
    const chosen = selectedMicrophone(HEADSET.key, [BUILT_IN, HEADSET]);

    expect(chosen).toEqual({
      kind: 'selected',
      deviceKey: HEADSET.key,
      label: HEADSET.label,
      fromSystemDefault: false,
    });
    expect(microphoneLabel(chosen)).toBe(HEADSET.label);
    expect(microphoneNotice(chosen)).toBeNull();
    expect(canRecord(chosen)).toBe(true);
  });

  it('저장된 장치가 지금 없으면 다른 장치로 바꿔치기하지 않는다', () => {
    // 이것이 이 모듈의 핵심이다. 첫 장치로 대체하면 사용자가 고른 적 없는 마이크로
    // 녹음이 시작되고, 장치가 바뀐 사실 자체가 사라진다.
    const chosen = selectedMicrophone('unplugged-headset', [BUILT_IN, HEADSET]);

    expect(chosen).toEqual({ kind: 'missing', savedKey: 'unplugged-headset' });
    expect(canRecord(chosen)).toBe(false);
    expect(microphoneLabel(chosen)).not.toBe(BUILT_IN.label);
    expect(microphoneLabel(chosen)).not.toBe(HEADSET.label);
  });

  it('저장된 장치가 없어진 사실이 사용자에게 문장으로 보인다', () => {
    const notice = microphoneNotice(selectedMicrophone('unplugged-headset', [BUILT_IN]));

    expect(notice).not.toBeNull();
    expect(notice ?? '').toMatch(/not available/i);
  });

  it('고른 적이 없으면 시스템 기본 장치를 쓰고, 그렇다고 말한다', () => {
    const chosen = selectedMicrophone(null, [HEADSET, BUILT_IN]);

    expect(chosen).toEqual({
      kind: 'selected',
      deviceKey: BUILT_IN.key,
      label: BUILT_IN.label,
      fromSystemDefault: true,
    });
    expect(microphoneNotice(chosen)).not.toBeNull();
    expect(canRecord(chosen)).toBe(true);
  });

  it('기본 장치 표시가 없으면 첫 장치를 쓴다', () => {
    const chosen = selectedMicrophone(null, [HEADSET]);

    expect(chosen).toEqual({
      kind: 'selected',
      deviceKey: HEADSET.key,
      label: HEADSET.label,
      fromSystemDefault: true,
    });
  });

  it('장치가 하나도 없는 것은 실패가 아니라 사실이다', () => {
    const chosen = selectedMicrophone(null, []);

    expect(chosen).toEqual({ kind: 'none' });
    expect(canRecord(chosen)).toBe(false);
    expect(microphoneNotice(chosen)).not.toBeNull();
  });

  it('장치가 없어도 저장된 선택은 저장된 채로 남는다', () => {
    expect(selectedMicrophone(HEADSET.key, [])).toEqual({ kind: 'missing', savedKey: HEADSET.key });
  });

  it('장치를 읽지 못하면 이름을 지어내지 않는다', () => {
    const view = failedDevices(ready(), initializationFailure);

    expect(view.microphone).toEqual({ kind: 'unknown', failure: initializationFailure });
    expect(canRecord(view.microphone)).toBe(false);
    expect(microphoneNotice(view.microphone)).not.toBeNull();
    expect(microphoneLabel(view.microphone).length).toBeGreaterThan(0);
  });

  it('장치를 읽지 못한 것이 진행 중인 녹음을 건드리지 않는다', () => {
    const view = failedDevices(recording(), 'device enumeration failed');

    expect(view.session).toEqual(RECORDING);
    expect(recordingControls(view).stop).toBe(true);
  });
});

describe('상태 전이와 버튼', () => {
  it('아직 시작하지 않았으면 Record만 누를 수 있다', () => {
    // Stop은 진행 중인 녹음에만 있다 — 시작하지 않은 녹음을 정지할 수는 없다.
    expect(recordingControls(ready())).toEqual({
      record: true,
      pause: false,
      resume: false,
      stop: false,
    });
  });

  it('고른 장치가 없으면 Record를 누를 수 없다', () => {
    const view = observedDevices(observedSession(INITIAL_RECORDING, IDLE), 'unplugged', [BUILT_IN]);

    expect(recordingControls(view).record).toBe(false);
  });

  it('요청의 답을 기다리는 동안에는 아무 버튼도 누를 수 없다', () => {
    const view = requestedAction(ready(), 'start');

    expect(view.busy).toBe(true);
    expect(recordingControls(view)).toEqual({
      record: false,
      pause: false,
      resume: false,
      stop: false,
    });
  });

  it('녹음 중에는 Pause와 Stop만 누를 수 있다', () => {
    expect(recordingControls(recording())).toEqual({
      record: false,
      pause: true,
      resume: false,
      stop: true,
    });
  });

  it('일시정지 중에는 Resume과 Stop만 누를 수 있다', () => {
    const paused = observedSession(requestedAction(recording(), 'pause'), PAUSED);

    expect(recordingControls(paused)).toEqual({
      record: false,
      pause: false,
      resume: true,
      stop: true,
    });
  });

  it('정지한 뒤에는 다시 시작할 수 있다', () => {
    const stopped = observedSession(savedRecording(recording(), STOPPED), IDLE);

    expect(recordingControls(stopped).record).toBe(true);
    expect(recordingControls(stopped).stop).toBe(false);
  });
});

describe('경과 시간', () => {
  it('backend가 준 문자열을 그대로 보여준다', () => {
    // 화면은 밀리초를 문자열로 바꾸지 않는다. 그 규칙은 Rust 한 곳에만 있다.
    const display = sessionDisplay(observedSession(ready(), status('recording', 3_151_000, '52:31')));

    expect(display.elapsedLabel).toBe('52:31');
    expect(display.live).toBe(true);
  });

  it('한 시간을 넘겨도 backend가 만든 문장 그대로다', () => {
    const view = observedSession(ready(), status('recording', 3_661_000, '1:01:01'));

    expect(sessionDisplay(view).elapsedLabel).toBe('1:01:01');
  });

  it('일시정지 중에는 녹음 중이라고 말하지 않는다', () => {
    const display = sessionDisplay(observedSession(recording(), PAUSED));

    expect(display.live).toBe(false);
    expect(display.elapsedLabel).toBe(PAUSED.elapsedLabel);
  });

  it('상태마다 서로 다른 표현이 있다', () => {
    const texts = ([IDLE, RECORDING, PAUSED] as const).map(
      (session) => sessionDisplay(observedSession(ready(), session)).stateText,
    );

    expect(new Set(texts).size).toBe(texts.length);
    for (const text of texts) {
      expect(text.length).toBeGreaterThan(0);
    }
  });
});

describe('실패는 갈래가 나뉘어 화면에 도달한다 (§13)', () => {
  it('권한 거부와 초기화 실패가 서로 다른 상태다', () => {
    // 사용자가 할 일이 다르다 — 하나는 시스템 설정을 열어야 하고, 하나는 다시 시도하거나
    // 장치를 바꿔야 한다. 둘을 한 덩어리로 보여주면 없는 문제를 고치려 하게 된다.
    const denied = failedAction(requestedAction(ready(), 'start'), 'start', permissionDenied);
    const failedToInitialize = failedAction(
      requestedAction(ready(), 'start'),
      'start',
      initializationFailure,
    );

    expect(denied.trouble?.kind).toBe('microphonePermission');
    expect(failedToInitialize.trouble?.kind).toBe('recordingStart');
    expect(denied.trouble?.headline).not.toBe(failedToInitialize.trouble?.headline);
    expect(denied.trouble?.failure).toEqual(permissionDenied);
    expect(failedToInitialize.trouble?.failure).toEqual(initializationFailure);
  });

  it('실패해도 화면은 남는다 — 다시 시작할 수 있다', () => {
    const view = failedAction(requestedAction(ready(), 'start'), 'start', permissionDenied);

    expect(view.busy).toBe(false);
    expect(recordingControls(view).record).toBe(true);
    expect(view.microphone.kind).toBe('selected');
  });

  it('정지 실패와 일시정지 실패가 섞이지 않는다', () => {
    const stopFailed = failedAction(recording(), 'stop', initializationFailure);
    const pauseFailed = failedAction(recording(), 'pause', initializationFailure);

    expect(stopFailed.trouble?.kind).toBe('recordingStop');
    expect(pauseFailed.trouble?.kind).toBe('recordingControl');
    expect(stopFailed.trouble?.headline).not.toBe(pauseFailed.trouble?.headline);
  });

  it('권한 실패는 어느 요청에서 왔든 권한 실패다', () => {
    expect(recordingTrouble('resume', permissionDenied).kind).toBe('microphonePermission');
  });

  it('계약과 다른 값으로 거절돼도 보여줄 수 있는 실패가 된다', () => {
    const trouble = recordingTrouble('start', 'rejected');

    expect(trouble.failure.message.length).toBeGreaterThan(0);
    expect(trouble.failure.detail).toBe('rejected');
  });

  it('상태를 읽지 못해도 진행 중인 녹음을 끝낼 수단이 남는다', () => {
    // 조회 한 번이 실패했다고 Stop이 사라지면 사용자는 녹음을 끝낼 방법을 잃는다 (R-005).
    const view = failedSession(recording(), 'status unavailable');

    expect(view.trouble?.kind).toBe('sessionStatus');
    expect(view.session).toEqual(RECORDING);
    expect(recordingControls(view).stop).toBe(true);
  });

  it('상태를 다시 읽으면 조회 실패만 사라진다', () => {
    const readAgain = observedSession(failedSession(recording(), 'status unavailable'), RECORDING);

    expect(readAgain.trouble).toBeNull();
  });

  it('상태를 다시 읽어도 권한 실패는 사라지지 않는다', () => {
    // 권한은 상태 조회가 성공한다고 해서 풀리지 않는다.
    const denied = failedAction(ready(), 'start', permissionDenied);
    const readAgain = observedSession(denied, IDLE);

    expect(readAgain.trouble?.kind).toBe('microphonePermission');
  });

  it('다시 누르면 지난 실패는 지워진다', () => {
    const retried = requestedAction(failedAction(ready(), 'start', permissionDenied), 'start');

    expect(retried.trouble).toBeNull();
  });
});

describe('제목과 저장된 녹음', () => {
  it('입력한 제목이 상태에 남는다', () => {
    expect(editedTitle(ready(), '3DGS Study #04').title).toBe('3DGS Study #04');
  });

  it('정지가 성공하면 저장된 녹음이 화면에 남는다', () => {
    // 이 값이 왔다는 것은 파일이 확정·확인되고 레코드가 저장됐다는 뜻이다 (R-002) —
    // 그래서 그 녹음은 Recordings 목록에도 있다.
    const view = savedRecording(editedTitle(recording(), '3DGS Study #04'), STOPPED);

    expect(view.saved).toEqual({ id: RECORD.id, title: RECORD.title, durationLabel: '52:31' });
    expect(view.busy).toBe(false);
    expect(view.trouble).toBeNull();
  });

  it('저장된 길이는 backend가 만든 값 그대로다', () => {
    expect(savedRecording(recording(), STOPPED).saved?.durationLabel).toBe(
      STOPPED.recording.durationLabel,
    );
  });

  it('다음 녹음이 지난 제목을 물려받지 않는다', () => {
    const view = savedRecording(editedTitle(recording(), '3DGS Study #04'), STOPPED);

    expect(view.title).toBe('');
    expect(view.saved?.title).toBe(RECORD.title);
  });

  it('새 녹음을 시작하면 지난 녹음의 저장 결과가 치워진다', () => {
    const saved = observedSession(savedRecording(recording(), STOPPED), IDLE);

    expect(requestedAction(saved, 'start').saved).toBeNull();
    // 일시정지·정지는 지금 녹음의 일이므로 지난 결과를 치우지 않는다.
    expect(requestedAction(saved, 'stop').saved).not.toBeNull();
  });
});
