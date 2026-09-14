// 녹음 화면의 상태 전환 테스트.
//
// 시작 → 일시정지 → 재개 → 정지 동안 화면이 무엇을 보여주는지, 저장된 default 장치가
// 사라졌을 때 무엇이 사용자에게 보이는지, 그리고 실패가 어느 갈래로 도착하는지 본다.
// **마이크도 마이크 권한도 DOM도 필요하지 않다** (PRODUCT-SPEC §18).
//
// 실제 마이크 입력 품질과 녹음된 소리는 이 테스트가 판정하지 않는다 — Human Review 항목이다.
import { describe, expect, it } from 'vitest';
import type { Failure } from '../ipc/failure';
import type {
  InputDevice,
  InputLevel,
  Recording,
  SessionStatus,
  StoppedRecording,
} from '../ipc/types';
import {
  CAPTURE_MODES,
  INITIAL_RECORDING,
  canSelectMode,
  liveLines,
  modeHint,
  selectedMode,
  UNKNOWN_ELAPSED,
  UNKNOWN_LEVEL_TEXT,
  WEAK_LEVEL_WARNING,
  canRecord,
  editedTitle,
  failedAction,
  failedDevices,
  failedSession,
  inputLevelDisplay,
  inputLevelMeter,
  inputLevelWarning,
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

/**
 * backend가 돌려주는 상태. **경과 시간 문자열도 입력 레벨도 Rust가 만든다.**
 *
 * 레벨을 넘기지 않으면 `null`이다 — 진행 중인 녹음이 없거나 아직 샘플이 하나도 쓰이지 않은
 * 상태이며, `0`이 아니다 (ADR-0003 §16.3).
 */
function status(
  state: SessionStatus['state'],
  elapsedMs: number,
  label: string,
  level: InputLevel | null = null,
): SessionStatus {
  return { state, elapsedMs, elapsedLabel: label, level };
}

/**
 * 세 판정의 값. **전부 backend가 만든 것이며 이 파일은 dBFS를 계산하지도 판정하지도 않는다.**
 *
 * 수치는 ADR-0003 §16.1의 실제 관측이다 — 전사가 무너진 2026-09-07의 -42.2 dBFS와 고유 문장
 * 94.0%가 나온 2026-09-04의 -25.8 dBFS.
 */
const USABLE_LEVEL: InputLevel = {
  averageDbfs: -25.8,
  peakDbfs: -6.1,
  verdict: 'usable',
  meterFill: 0.42,
  message: '입력 레벨이 쓸 만하다 (평균 -25.8 dBFS)',
};

const LOW_LEVEL: InputLevel = {
  averageDbfs: -42.2,
  peakDbfs: -19.4,
  verdict: 'low',
  meterFill: 0.42,
  message: '입력 레벨이 낮다 — 마이크와 자리를 확인한다 (평균 -42.2 dBFS)',
};

const SILENT_LEVEL: InputLevel = {
  averageDbfs: -90.3,
  peakDbfs: -90.3,
  verdict: 'silent',
  meterFill: 0.42,
  message: '소리가 들어오지 않는다 — 마이크를 확인한다 (평균 -90.3 dBFS 이하)',
};

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
    expect(notice ?? '').toMatch(/지금 없음|쓸 수 없다/);
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

describe('입력 레벨 (ADR-0003 §16.3 · §16.4)', () => {
  // 지키려는 것: **소리가 담기지 않은 채로 51분이 지나가지 않는다.** 2026-09-07에 실제로
  // 그 일이 있었고(§16.1), 사람이 그것을 안 것은 전사를 4.3분 기다린 뒤였다. 그러므로 세
  // 갈래가 화면에서 갈려야 하고, 쓸 수 없을 만큼 낮으면 **정지 전에** 말해야 한다.
  //
  // 이 파일은 dBFS를 계산하지도 임계값을 두지도 않는다 — 값도 문장도 backend가 만든 것이다.

  /** 녹음 중이고, backend가 이 레벨을 마지막으로 말해 줬다. */
  function live(level: InputLevel | null): RecordingView {
    return observedSession(recording(), status('recording', 7_000, '0:07', level));
  }

  it('값이 아직 없는 것은 낮은 것이 아니다', () => {
    // 녹음을 막 시작한 순간이다. 여기서 "낮음"이라고 말하면 그 표시는 그 뒤로 믿을 수 없다.
    const display = inputLevelDisplay(live(null));

    expect(display.shown).toBe(true);
    expect(display.kind).toBe('unknown');
    expect(display.text).toBe(UNKNOWN_LEVEL_TEXT);
    expect(display.weak).toBe(false);
    expect(inputLevelWarning(live(null))).toBeNull();
  });

  it('낮으면 그 갈래로 오고 backend의 문장을 그대로 보여준다', () => {
    const display = inputLevelDisplay(live(LOW_LEVEL));

    expect(display.shown).toBe(true);
    expect(display.kind).toBe('low');
    expect(display.text).toBe(LOW_LEVEL.message);
    expect(display.weak).toBe(true);
  });

  it('소리가 들어오지 않는 것도 값 없음과 다른 갈래다', () => {
    const display = inputLevelDisplay(live(SILENT_LEVEL));

    expect(display.kind).toBe('silent');
    expect(display.text).toBe(SILENT_LEVEL.message);
    expect(display.weak).toBe(true);
  });

  it('쓸 만하면 그렇다고 말하고 경고하지 않는다', () => {
    const display = inputLevelDisplay(live(USABLE_LEVEL));

    expect(display.kind).toBe('usable');
    expect(display.text).toBe(USABLE_LEVEL.message);
    expect(display.weak).toBe(false);
    expect(inputLevelWarning(live(USABLE_LEVEL))).toBeNull();
  });

  it('세 갈래가 서로 다른 문장으로 온다', () => {
    const texts = [null, LOW_LEVEL, SILENT_LEVEL, USABLE_LEVEL].map(
      (level) => inputLevelDisplay(live(level)).text,
    );

    expect(new Set(texts).size).toBe(texts.length);
    for (const text of texts) {
      expect(text.trim()).not.toBe('');
    }
  });

  it('쓸 수 없을 만큼 낮으면 정지 전에 경고가 나온다', () => {
    // 두 갈래 모두 같은 경고다 — 무엇이 얼마나 낮은지는 backend의 문장이 함께 말한다.
    expect(inputLevelWarning(live(LOW_LEVEL))).toBe(WEAK_LEVEL_WARNING);
    expect(inputLevelWarning(live(SILENT_LEVEL))).toBe(WEAK_LEVEL_WARNING);
  });

  it('경고 문장이 무엇을 해야 하는지 말한다', () => {
    // 문장을 통째로 고정하면 낱말 하나를 다듬을 때마다 테스트가 깨진다. 고정하는 것은
    // 이 경고가 반드시 말해야 하는 두 가지다 — 마이크를 확인하라, 그리고 정지 전에.
    expect(WEAK_LEVEL_WARNING).toMatch(/마이크/);
    expect(WEAK_LEVEL_WARNING).toMatch(/정지/);
    // 녹음이 사라진다고 말하지 않는다 — 어떤 실패도 이미 녹음된 것을 지우지 않는다 (INV-3).
    expect(WEAK_LEVEL_WARNING).toMatch(/그대로 남는다/);
  });

  it('녹음 중이 아니면 경고하지 않는다', () => {
    // 정지한 뒤나 시작하기 전에 "정지 전에 확인하라"고 말하는 것은 할 수 있는 일이 없는
    // 경고이며, 그런 경고가 남아 있으면 정작 녹음 중의 경고도 배경이 된다.
    const notRecording: readonly RecordingView[] = [
      INITIAL_RECORDING,
      ready(),
      observedSession(recording(), status('paused', 7_000, '0:07', LOW_LEVEL)),
      observedSession(recording(), status('stopped', 7_000, '0:07', LOW_LEVEL)),
      observedSession(recording(), status('idle', 0, '0:00', LOW_LEVEL)),
    ];

    for (const view of notRecording) {
      const where = view.session?.state ?? '아직 모르는 상태';
      expect(inputLevelWarning(view), `${where}에서 경고가 나왔다`).toBeNull();
    }
  });

  it('진행 중인 녹음이 없으면 레벨 자리 자체를 두지 않는다', () => {
    expect(inputLevelDisplay(INITIAL_RECORDING).shown).toBe(false);
    expect(inputLevelDisplay(ready()).shown).toBe(false);
    // 일시정지는 아직 끝난 녹음이 아니다 — 지금까지 담긴 것이 무엇인지는 계속 보인다.
    expect(
      inputLevelDisplay(observedSession(recording(), status('paused', 7_000, '0:07', LOW_LEVEL))),
    ).toMatchObject({ shown: true, kind: 'low' });
  });

  it('레벨이 상태와 경과 시간을 밀어내지 않는다 (§19)', () => {
    // 화면에서 가장 크고 분명해야 하는 두 값은 레벨이 무엇이든 그대로다.
    const withLevel = sessionDisplay(live(SILENT_LEVEL));

    expect(withLevel).toEqual(sessionDisplay(live(null)));
    expect(withLevel.elapsedLabel).toBe('0:07');
    expect(withLevel.live).toBe(true);
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

describe('입력 레벨 막대 (2026-09-08)', () => {
  const withLevel = (meterFill: number, verdict: 'usable' | 'low' | 'silent') =>
    observedSession(INITIAL_RECORDING, {
      state: 'recording',
      elapsedMs: 1_000,
      elapsedLabel: '0:01',
      level: {
        averageDbfs: -30,
        peakDbfs: -12,
        verdict,
        meterFill,
        message: 'backend가 만든 문장',
      },
    });

  it('녹음 중이 아니면 막대가 없다', () => {
    // 끝난 녹음의 레벨을 계속 붙들고 있지 않는다.
    expect(inputLevelMeter(INITIAL_RECORDING)).toBeNull();
  });

  it('아직 잰 값이 없으면 막대가 없다 — 0으로 그리지 않는다', () => {
    const view = observedSession(INITIAL_RECORDING, {
      state: 'recording',
      elapsedMs: 1_000,
      elapsedLabel: '0:01',
      level: null,
    });

    expect(inputLevelMeter(view)).toBeNull();
  });

  /**
   * **화면이 dBFS에서 길이를 만들지 않는다** (INV-9). 판정 구간을 아는 자리가
   * `audio/level.rs` 하나이므로 길이도 거기서 온다 — 여기서 계산하면 임계값이 두 곳에
   * 살게 되고, 막대와 문장이 언젠가 어긋난다.
   */
  it('채우는 길이는 backend가 준 값 그대로다', () => {
    expect(inputLevelMeter(withLevel(0.42, 'low'))!.fill).toBe(0.42);
    expect(inputLevelMeter(withLevel(1, 'usable'))!.fill).toBe(1);
    expect(inputLevelMeter(withLevel(0, 'silent'))!.fill).toBe(0);
  });

  it('색을 화면이 정하지 않는다 — 갈래가 값으로 온다', () => {
    expect(inputLevelMeter(withLevel(0.7, 'usable'))!.weak).toBe(false);
    expect(inputLevelMeter(withLevel(0.4, 'low'))!.weak).toBe(true);
    expect(inputLevelMeter(withLevel(0.1, 'silent'))!.weak).toBe(true);
  });
});

describe('녹음 모드 (§22 · ADR-0012)', () => {
  const at = (state: 'idle' | 'recording' | 'paused' | 'stopped') =>
    observedSession(INITIAL_RECORDING, {
      state,
      elapsedMs: 0,
      elapsedLabel: '0:00',
      level: null,
    });

  it('처음에는 마이크 모드다', () => {
    // 회의 모드가 기본이면, 회의가 아닌 녹음도 시스템 소리를 함께 담는다.
    expect(INITIAL_RECORDING.mode).toBe('microphone');
  });

  it('시작하기 전에는 고를 수 있다', () => {
    expect(canSelectMode(at('idle'))).toBe(true);
    expect(selectedMode(at('idle'), 'meeting').mode).toBe('meeting');
  });

  it('정지한 뒤에도 다음 녹음을 위해 고를 수 있다', () => {
    expect(canSelectMode(at('stopped'))).toBe(true);
  });

  it('녹음 중이나 일시정지 중에는 바꾸지 않는다', () => {
    // 지키려는 것: **한 파일 안에서 채널 수가 달라지지 않는다.** WAV 헤더는 파일 하나에
    // 하나뿐이므로, 도중에 모드가 바뀌면 헤더가 내용과 어긋난다.
    for (const state of ['recording', 'paused'] as const) {
      const view = { ...at(state), mode: 'microphone' as const };
      expect(canSelectMode(view)).toBe(false);
      expect(selectedMode(view, 'meeting').mode).toBe('microphone');
    }
  });

  it('상태를 아직 모르거나 답을 기다리는 동안에는 고르지 않는다', () => {
    expect(canSelectMode(INITIAL_RECORDING)).toBe(false);
    expect(canSelectMode({ ...at('idle'), busy: true })).toBe(false);
  });

  it('두 모드가 각각 무엇을 녹음하는지 말한다', () => {
    // 사용자가 판단하는 데 필요한 것은 모드 이름이 아니라 **무엇이 파일에 들어가는가**다.
    expect(modeHint('meeting')).toContain('함께');
    expect(modeHint('microphone')).toContain('마이크 하나');
    expect(modeHint('meeting')).not.toBe(modeHint('microphone'));
  });

  it('고를 수 있는 모드가 두 가지이고 이름이 겹치지 않는다', () => {
    expect(CAPTURE_MODES.map((mode) => mode.value)).toEqual(['microphone', 'meeting']);
    expect(new Set(CAPTURE_MODES.map((mode) => mode.label)).size).toBe(CAPTURE_MODES.length);
  });
});

describe('녹음 중에 받아 적은 말 (2026-09-14)', () => {
  const at = (
    state: 'idle' | 'running' | 'gaveUp',
    lines: { startMs: number; endMs: number; text: string }[] = [],
    failure: Failure | null = null,
  ) => ({ state, lines, failure }) as const;

  it('돌지 않으면 아무것도 두지 않는다', () => {
    // 아직 물어보지 못한 것과 돌지 않는 것 둘 다 조용해야 한다.
    expect(liveLines(null).kind).toBe('hidden');
    expect(liveLines(at('idle')).kind).toBe('hidden');
  });

  it('아직 첫 문장이 없으면 얼마나 기다리는지 말한다', () => {
    // 지키려는 것: **멎은 것처럼 보이지 않는다.** 창 하나가 30초라 첫 문장까지 그만큼
    // 걸리는데, 아무 말이 없으면 고장난 줄 안다.
    const view = liveLines(at('running'));
    expect(view.kind).toBe('waiting');
    expect(view.kind === 'waiting' && view.text).toContain('30초');
  });

  it('문장이 나오면 그대로 보여준다', () => {
    const view = liveLines(at('running', [{ startMs: 0, endMs: 900, text: '들린 말' }]));
    expect(view.kind).toBe('lines');
    expect(view.kind === 'lines' && view.lines).toHaveLength(1);
  });

  it('그만뒀을 때 녹음은 계속된다고 말한다', () => {
    // 지키려는 것: **실패 화면이 아니다** (INV-8). 이 말이 없으면 사용자는 녹음까지
    // 잘못된 줄 알고 정지해 버린다.
    const view = liveLines(at('gaveUp'));
    expect(view.kind).toBe('gaveUp');
    expect(view.kind === 'gaveUp' && view.text).toContain('녹음은 계속');
  });

  it('그만둔 이유가 있으면 함께 나른다', () => {
    const failure: Failure = {
      kind: 'transcriptionModelMissing',
      message: '모델 파일을 찾지 못했다.',
      detail: null,
      retryable: false,
      sourceDataSafe: true,
    };
    const view = liveLines(at('gaveUp', [], failure));
    expect(view.kind === 'gaveUp' && view.failure).toBe(failure);
  });

  it('그만뒀으면 그때까지 받아 적은 것보다 그 사실을 먼저 말한다', () => {
    // 문장이 조금 있더라도 "지금은 받아 적지 않는다"가 먼저다 — 그것을 모르면
    // 사용자는 계속 쌓이는 줄 알고 기다린다.
    const view = liveLines(at('gaveUp', [{ startMs: 0, endMs: 900, text: '들린 말' }]));
    expect(view.kind).toBe('gaveUp');
  });
});

describe('정지 뒤 마무리 (2026-09-14)', () => {
  // 지키려는 것: **정지가 끝났다는 사실이 먼저 보인다.**
  //
  // 2026-09-14에 정지 경로에서 남은 구간 전사를 그대로 돌렸다가 UI가 7분 멈췄다.
  // 이제 그 일은 배경에서 돌지만, 그 사이 화면이 아무 말도 없으면 사용자는 정지가
  // 안 된 줄 알고 다시 누른다.

  it('마무리 중에는 녹음이 이미 저장됐다고 말한다', () => {
    const view = liveLines({ state: 'finishing', lines: [], failure: null });
    expect(view.kind).toBe('waiting');
    expect(view.kind === 'waiting' && view.text).toContain('저장됐다');
  });

  it('마무리 중이라는 말이 "아직 받아 적는 중"과 다르다', () => {
    // 둘을 같은 문장으로 두면 정지가 됐는지 알 수 없다.
    const finishing = liveLines({ state: 'finishing', lines: [], failure: null });
    const running = liveLines({ state: 'running', lines: [], failure: null });
    expect(finishing.kind === 'waiting' && finishing.text).not.toBe(
      running.kind === 'waiting' && running.text,
    );
  });
});
