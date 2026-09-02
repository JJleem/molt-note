/**
 * Phase 2A spike 표면의 상태 (docs/ADR-0003-recording-engine.md §12).
 *
 * 이것은 **최종 Recording 화면의 상태가 아니다.** ADR-0003이 아직 `PROVISIONAL`이므로
 * 사람이 실제 기기에서 확인해야 하는 것만 화면에 도달시킨다 —
 * 장치 목록 · 선택 · 시작/정지 · 결과 네 값(장치 이름 · 출력 경로 · 포맷 · 파일 크기).
 * pause/resume · 경과 시간 · 재생 · 영속화는 여기에 없다 (Phase 2B).
 *
 * React도 DOM도 Tauri도 알지 않는 순수 모듈이라 **빈 장치 목록 · 상태 전이 · 결과 표시 ·
 * 실패** 네 경로를 마이크 없이 vitest로 그대로 판정할 수 있다 (PRODUCT-SPEC §18).
 * 화면 컴포넌트는 여기서 만들어진 값을 그리기만 한다.
 *
 * ## 포맷 문장은 여기서 만들지 않는다
 *
 * `format`은 Rust가 이미 만들어 보낸 값이다 (`src-tauri/src/audio/capture.rs`의
 * `CaptureFormat::describe`). 샘플레이트 · 채널 수 · 비트 심도 · 컨테이너를 한 문장으로
 * 만드는 규칙은 그 한 곳에만 있고, TypeScript에 다시 구현하지 않는다 — 두 벌이 되면
 * 사람이 §14.4의 whisper 입력 요구와 대조하는 바로 그 문장이 조용히 갈라진다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { CaptureReport, InputDevice } from '../ipc/types';

/**
 * 캡처가 놓일 수 있는 상태의 전부.
 *
 * 세 가지뿐이다 — 아직 시작하지 않았거나(`idle`), 진행 중이거나(`recording`),
 * 한 번 끝났거나(`finished`). pause/resume 같은 네 번째 상태는 Phase 2B의 일이며
 * 지금은 command 자체가 없다.
 */
export type CaptureStatus = 'idle' | 'recording' | 'finished';

/** 정지한 캡처 하나가 화면에 보이는 모습. 네 값이 Phase 2A의 성공 기준 그대로다. */
export interface CaptureResultView {
  /** 실제로 열린 장치의 이름 — 고른 장치와 같은지 사람이 본다 (§12 항목 2). */
  readonly deviceLabel: string;
  /** 확정된 파일의 경로 — 사람이 이 경로를 열어 재생한다 (§12 항목 5·6). */
  readonly outputPath: string;
  /** Rust가 만든 형식 문장 그대로 (§12 항목 7). */
  readonly formatText: string;
  /** 파일 크기. 사람이 읽는 크기와 정확한 byte 수가 함께 있다 (§12 항목 5). */
  readonly sizeText: string;
  /** 파일이 비어 있는가. 0 byte는 성공처럼 보이는 실패이므로 화면이 따로 말한다. */
  readonly isEmptyFile: boolean;
}

/** 장치 목록을 읽은 뒤의 상태. 고른 장치가 언제나 하나 있다. */
export interface CaptureSpikeReady {
  readonly kind: 'ready';
  readonly devices: readonly InputDevice[];
  /** 고른 장치의 `key`. 목록에 있는 값 중 하나다. */
  readonly selectedKey: string;
  readonly status: CaptureStatus;
  /** 마지막으로 정지한 캡처의 결과. 아직 없거나 실패했다면 `null`이다. */
  readonly result: CaptureResultView | null;
  /** 마지막 시작/정지가 실패했다면 그 실패. 아니면 `null`이다. */
  readonly failure: Failure | null;
}

/**
 * 화면이 놓일 수 있는 상태의 전부.
 *
 * 장치가 하나도 없는 것은 실패가 아니라 독립된 정상 상태(`empty`)다 — 마이크를 뽑아 둔
 * 상태를 실패로 만들면 화면이 사용자에게 없는 문제를 알리게 된다. 목록 자체를 읽지 못한
 * 것(`failed`)과 섞지 않는다.
 */
export type CaptureSpikeView =
  | { readonly kind: 'loading' }
  | { readonly kind: 'empty' }
  | CaptureSpikeReady
  | { readonly kind: 'failed'; readonly failure: Failure };

export const LOADING_CAPTURE_SPIKE: CaptureSpikeView = { kind: 'loading' };

const STATUS_TEXT: Record<CaptureStatus, string> = {
  idle: 'Idle',
  recording: 'Recording…',
  finished: 'Finished',
};

/** 캡처 상태의 사람이 읽는 표현. */
export function captureStatusText(status: CaptureStatus): string {
  return STATUS_TEXT[status];
}

/**
 * `list_input_devices`가 돌려준 목록을 화면 상태로 바꾼다.
 *
 * 하나도 없으면 `empty`다. 있으면 기본 장치가 먼저 골라져 있다 — 사람이 아무것도 고르지
 * 않은 채로 시작 버튼을 누를 수 있는 상태를 만들지 않는다.
 */
export function loadedInputDevices(devices: readonly InputDevice[]): CaptureSpikeView {
  const first = devices.find((device) => device.isDefault) ?? devices[0];
  if (first === undefined) {
    return { kind: 'empty' };
  }
  return {
    kind: 'ready',
    devices,
    selectedKey: first.key,
    status: 'idle',
    result: null,
    failure: null,
  };
}

/** 장치 목록을 읽지 못했다. 열거 자체의 실패가 여기로 온다 (§13). */
export function failedInputDevices(error: unknown): CaptureSpikeView {
  return { kind: 'failed', failure: toFailure(error) };
}

/**
 * 다른 장치를 골랐다.
 *
 * 녹음 중에는 바뀌지 않는다 — 진행 중인 캡처가 열어 둔 장치와 화면이 어긋나지 않게 한다.
 * 장치를 바꾸면 지난 결과는 지운다. 그 결과는 **이 장치의 결과가 아니기 때문이다.**
 */
export function selectedInputDevice(view: CaptureSpikeView, deviceKey: string): CaptureSpikeView {
  if (view.kind !== 'ready' || view.status === 'recording') {
    return view;
  }
  if (!view.devices.some((device) => device.key === deviceKey)) {
    return view;
  }
  return { ...view, selectedKey: deviceKey, status: 'idle', result: null, failure: null };
}

/**
 * 캡처를 시작했다.
 *
 * 요청을 보낸 순간 `recording`이 된다 — 무엇이 진행 중인지가 화면에 바로 보여야 한다.
 * 요청이 거절되면 {@link failedCapture}가 `idle`로 되돌리고 그 실패를 보여준다.
 * 이미 녹음 중이면 아무 일도 일어나지 않는다 (진행 중인 녹음을 조용히 버리지 않는다).
 */
export function startedCapture(view: CaptureSpikeView): CaptureSpikeView {
  if (view.kind !== 'ready' || view.status === 'recording') {
    return view;
  }
  return { ...view, status: 'recording', result: null, failure: null };
}

/**
 * 캡처가 정지되고 보고 값이 왔다.
 *
 * 녹음 중이 아니었다면 이 보고는 이 화면의 것이 아니므로 상태를 바꾸지 않는다.
 */
export function finishedCapture(view: CaptureSpikeView, report: CaptureReport): CaptureSpikeView {
  if (view.kind !== 'ready' || view.status !== 'recording') {
    return view;
  }
  return { ...view, status: 'finished', result: toCaptureResult(report), failure: null };
}

/**
 * 시작 또는 정지가 실패했다.
 *
 * 실패한 캡처는 결과가 없고, 그 캡처는 이 시점에 끝났다 — 그래서 `idle`로 돌아간다.
 * 아직 장치 목록조차 읽지 못한 상태에서 온 실패라면 읽지 못한 화면으로 남는다.
 */
export function failedCapture(view: CaptureSpikeView, error: unknown): CaptureSpikeView {
  const failure = toFailure(error);
  if (view.kind !== 'ready') {
    return { kind: 'failed', failure };
  }
  return { ...view, status: 'idle', result: null, failure };
}

/** 지금 고른 장치. 아직 고를 수 있는 상태가 아니면 `null`이다. */
export function selectedDevice(view: CaptureSpikeView): InputDevice | null {
  if (view.kind !== 'ready') {
    return null;
  }
  return view.devices.find((device) => device.key === view.selectedKey) ?? null;
}

/** 보고 값을 화면이 그대로 그릴 수 있는 네 문장으로 옮긴다. */
export function toCaptureResult(report: CaptureReport): CaptureResultView {
  return {
    deviceLabel: report.deviceLabel,
    outputPath: report.outputPath,
    // Rust가 보낸 문장을 그대로 쓴다. 여기서 다시 만들지 않는다.
    formatText: report.format,
    sizeText: formatByteSize(report.byteSize),
    isEmptyFile: report.byteSize <= 0,
  };
}

/** 한 단계 위 단위로 올라가는 기준. 크기는 2의 거듭제곱으로 읽는다. */
const BYTES_PER_STEP = 1024;
const LARGER_UNITS = ['MB', 'GB'] as const;

/**
 * 파일 크기를 사람이 읽는 문장으로 만든다.
 *
 * 정확한 byte 수를 언제나 함께 남긴다 — 사람이 확인하는 것은 "파일이 비어 있지 않은가"이며
 * (§12 항목 5), 반올림된 크기만으로는 그 답이 되지 않는다.
 */
export function formatByteSize(byteSize: number): string {
  if (!Number.isFinite(byteSize) || byteSize < 0) {
    // 계약과 다른 값이 왔다. 그럴듯한 크기를 지어내지 않고 받은 값을 그대로 보여준다.
    return `${byteSize} bytes`;
  }

  const bytes = Math.trunc(byteSize);
  const exact = `${bytes.toLocaleString('en-US')} bytes`;
  if (bytes < BYTES_PER_STEP) {
    return exact;
  }

  let scaled = bytes / BYTES_PER_STEP;
  let unit = 'KB';
  for (const larger of LARGER_UNITS) {
    if (scaled < BYTES_PER_STEP) {
      break;
    }
    scaled = scaled / BYTES_PER_STEP;
    unit = larger;
  }

  return `${scaled.toFixed(1)} ${unit} (${exact})`;
}
