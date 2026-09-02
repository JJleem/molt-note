// Phase 2A spike 표면의 상태 변환 테스트.
//
// 장치 목록을 읽고 → 하나를 고르고 → 시작하고 → 정지하는 동안 상태가 어떻게 움직이는지,
// 그리고 실패가 어디로 가는지 본다. **마이크도 마이크 권한도 DOM도 필요하지 않다** (§18 ·
// docs/ADR-0003-recording-engine.md §12.1 — 자동 테스트는 실제 장치를 전제하지 않는다).
//
// 사람이 실제 장치에서 확인해야 하는 8개 항목은 이 테스트가 대신 판정하지 않는다.
import { describe, expect, it } from 'vitest';
import type { Failure } from '../ipc/failure';
import type { CaptureReport, InputDevice } from '../ipc/types';
import {
  LOADING_CAPTURE_SPIKE,
  captureStatusText,
  failedCapture,
  failedInputDevices,
  finishedCapture,
  formatByteSize,
  loadedInputDevices,
  selectedDevice,
  selectedInputDevice,
  startedCapture,
  toCaptureResult,
} from './captureSpikeView';

const BUILT_IN: InputDevice = { key: 'builtin', label: 'MacBook Pro Microphone', isDefault: true };
const HEADSET: InputDevice = { key: 'headset', label: 'USB Headset', isDefault: false };

const REPORT: CaptureReport = {
  deviceLabel: 'USB Headset',
  outputPath: '/Users/someone/Library/Application Support/molt-note/recordings/capture-1756800000.wav',
  format: '48000 Hz · mono · 16-bit PCM · WAV',
  sampleRateHz: 48_000,
  channels: 1,
  bitsPerSample: 16,
  container: 'WAV',
  byteSize: 1_536_044,
};

const deviceFailure: Failure = {
  kind: 'audioDevice',
  message: '입력 장치를 열지 못했다.',
  detail: 'device not available',
  sourceDataSafe: true,
  retryable: true,
};

/** 장치 두 개를 읽은 뒤의 상태. */
function ready() {
  return loadedInputDevices([BUILT_IN, HEADSET]);
}

/** 녹음 중인 상태. */
function recording() {
  return startedCapture(ready());
}

describe('장치 목록', () => {
  it('첫 상태는 아직 읽지 못한 상태다', () => {
    expect(LOADING_CAPTURE_SPIKE.kind).toBe('loading');
  });

  it('장치가 하나도 없는 것은 실패가 아니라 빈 상태다', () => {
    // 마이크를 뽑아 둔 상태는 정상이다 — 없는 문제를 사용자에게 알리지 않는다.
    expect(loadedInputDevices([])).toEqual({ kind: 'empty' });
  });

  it('목록을 읽으면 기본 장치가 미리 골라져 있다', () => {
    const view = ready();

    expect(view.kind).toBe('ready');
    if (view.kind !== 'ready') return;
    expect(view.devices).toEqual([BUILT_IN, HEADSET]);
    expect(view.selectedKey).toBe(BUILT_IN.key);
    expect(view.status).toBe('idle');
    expect(view.result).toBeNull();
    expect(view.failure).toBeNull();
  });

  it('기본 장치가 없으면 첫 장치가 골라져 있다', () => {
    const view = loadedInputDevices([HEADSET]);

    expect(view.kind === 'ready' && view.selectedKey).toBe(HEADSET.key);
  });

  it('목록을 읽지 못하면 화면이 실패 상태가 된다', () => {
    expect(failedInputDevices(deviceFailure)).toEqual({ kind: 'failed', failure: deviceFailure });
  });

  it('계약과 다른 값으로 거절돼도 보여줄 수 있는 실패가 된다', () => {
    const view = failedInputDevices('rejected');

    expect(view.kind).toBe('failed');
    if (view.kind !== 'failed') return;
    expect(view.failure.message.length).toBeGreaterThan(0);
    expect(view.failure.detail).toBe('rejected');
  });
});

describe('장치 선택', () => {
  it('고른 장치가 상태에 남는다', () => {
    const view = selectedInputDevice(ready(), HEADSET.key);

    expect(view.kind === 'ready' && view.selectedKey).toBe(HEADSET.key);
    expect(selectedDevice(view)).toEqual(HEADSET);
  });

  it('녹음 중에는 장치가 바뀌지 않는다', () => {
    const during = recording();

    expect(selectedInputDevice(during, HEADSET.key)).toBe(during);
  });

  it('목록에 없는 장치는 고를 수 없다', () => {
    const view = ready();

    expect(selectedInputDevice(view, 'unplugged')).toBe(view);
  });

  it('장치를 바꾸면 지난 결과가 사라진다', () => {
    // 그 결과는 이 장치의 결과가 아니다.
    const finished = finishedCapture(recording(), REPORT);
    expect(finished.kind === 'ready' && finished.result).not.toBeNull();

    const switched = selectedInputDevice(finished, HEADSET.key);
    expect(switched.kind === 'ready' && switched.result).toBeNull();
    expect(switched.kind === 'ready' && switched.status).toBe('idle');
  });

  it('읽지 못한 상태에서는 고를 장치가 없다', () => {
    const failed = failedInputDevices(deviceFailure);

    expect(selectedInputDevice(failed, HEADSET.key)).toBe(failed);
    expect(selectedDevice(failed)).toBeNull();
    expect(selectedDevice(LOADING_CAPTURE_SPIKE)).toBeNull();
  });
});

describe('시작 · 정지 상태 전이', () => {
  it('idle에서 시작하면 녹음 중이 된다', () => {
    const view = recording();

    expect(view.kind === 'ready' && view.status).toBe('recording');
    expect(view.kind === 'ready' && view.result).toBeNull();
    expect(view.kind === 'ready' && view.failure).toBeNull();
  });

  it('이미 녹음 중이면 시작이 아무것도 하지 않는다', () => {
    const during = recording();

    expect(startedCapture(during)).toBe(during);
  });

  it('정지하면 끝난 상태가 되고 결과가 남는다', () => {
    const view = finishedCapture(recording(), REPORT);

    expect(view.kind).toBe('ready');
    if (view.kind !== 'ready') return;
    expect(view.status).toBe('finished');
    expect(view.result).toEqual(toCaptureResult(REPORT));
    expect(view.failure).toBeNull();
  });

  it('끝난 뒤 다시 시작하면 지난 결과가 사라진다', () => {
    const again = startedCapture(finishedCapture(recording(), REPORT));

    expect(again.kind === 'ready' && again.status).toBe('recording');
    expect(again.kind === 'ready' && again.result).toBeNull();
  });

  it('녹음 중이 아닐 때 온 보고는 상태를 바꾸지 않는다', () => {
    const idle = ready();

    expect(finishedCapture(idle, REPORT)).toBe(idle);
    expect(finishedCapture(LOADING_CAPTURE_SPIKE, REPORT)).toBe(LOADING_CAPTURE_SPIKE);
  });

  it('상태마다 사람이 읽는 표현이 있다', () => {
    for (const status of ['idle', 'recording', 'finished'] as const) {
      expect(captureStatusText(status).length).toBeGreaterThan(0);
    }
    expect(captureStatusText('idle')).not.toBe(captureStatusText('recording'));
  });
});

describe('실패', () => {
  it('시작이 실패하면 idle로 돌아가고 실패가 보인다', () => {
    const view = failedCapture(recording(), deviceFailure);

    expect(view.kind).toBe('ready');
    if (view.kind !== 'ready') return;
    expect(view.status).toBe('idle');
    expect(view.failure).toEqual(deviceFailure);
    expect(view.result).toBeNull();
    // 고른 장치는 그대로다 — 실패했다고 선택을 버리지 않는다.
    expect(view.selectedKey).toBe(BUILT_IN.key);
  });

  it('정지가 실패하면 결과 없이 끝난다', () => {
    const view = failedCapture(recording(), '녹음 중이 아니다.');

    expect(view.kind === 'ready' && view.status).toBe('idle');
    expect(view.kind === 'ready' && view.result).toBeNull();
    expect(view.kind === 'ready' && view.failure?.detail).toBe('녹음 중이 아니다.');
  });

  it('다시 시작하면 지난 실패는 지워진다', () => {
    const retried = startedCapture(failedCapture(recording(), deviceFailure));

    expect(retried.kind === 'ready' && retried.failure).toBeNull();
    expect(retried.kind === 'ready' && retried.status).toBe('recording');
  });

  it('장치 목록조차 읽지 못한 상태에서 온 실패는 읽지 못한 화면으로 남는다', () => {
    expect(failedCapture(LOADING_CAPTURE_SPIKE, deviceFailure)).toEqual({
      kind: 'failed',
      failure: deviceFailure,
    });
  });
});

describe('결과 표시', () => {
  it('보고된 네 값이 그대로 화면 값이 된다', () => {
    const result = toCaptureResult(REPORT);

    expect(result.deviceLabel).toBe('USB Headset');
    expect(result.outputPath).toBe(REPORT.outputPath);
    // 포맷 문장은 Rust가 만든 것을 그대로 쓴다 — 여기서 다시 만들지 않는다.
    expect(result.formatText).toBe(REPORT.format);
    expect(result.sizeText).toContain('1,536,044 bytes');
    expect(result.isEmptyFile).toBe(false);
  });

  it('빈 파일은 성공처럼 보이지 않는다', () => {
    // 0 byte는 §12 항목 5가 걸러내야 하는 실패다.
    const result = toCaptureResult({ ...REPORT, byteSize: 0 });

    expect(result.isEmptyFile).toBe(true);
    expect(result.sizeText).toBe('0 bytes');
  });

  it('작은 파일은 정확한 byte 수로만 보인다', () => {
    expect(formatByteSize(1)).toBe('1 bytes');
    expect(formatByteSize(1023)).toBe('1,023 bytes');
  });

  it('큰 파일은 읽을 수 있는 크기와 정확한 byte 수를 함께 보여준다', () => {
    expect(formatByteSize(1024)).toBe('1.0 KB (1,024 bytes)');
    expect(formatByteSize(1_536_044)).toBe('1.5 MB (1,536,044 bytes)');
    expect(formatByteSize(3_221_225_472)).toBe('3.0 GB (3,221,225,472 bytes)');
  });

  it('계약과 다른 크기가 와도 지어내지 않는다', () => {
    expect(formatByteSize(-1)).toBe('-1 bytes');
    expect(formatByteSize(Number.NaN)).toBe('NaN bytes');
  });
});
