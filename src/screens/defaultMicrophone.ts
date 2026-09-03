/**
 * 저장된 default microphone을 **지금 열거된 장치와 맞춰 보는 규칙** (PRODUCT-SPEC §5 D · §6.1).
 *
 * 저장된 값은 선택 키 하나뿐이고, 장치는 언제든 꽂히고 빠진다. 그래서 "저장돼 있다"는 것과
 * "지금 쓸 수 있다"는 것은 같은 말이 아니다. 이 모듈이 하는 일은 그 둘을 **세 가지 결과로
 * 갈라 놓는 것**이다.
 *
 * ```text
 * notChosen  아직 고른 적이 없다            — 정상 상태다
 * available  저장된 장치를 지금 목록에서 찾았다
 * missing    저장된 장치가 지금 목록에 없다  — 장치가 빠졌다
 * ```
 *
 * **`missing`에서 조용히 다른 장치로 바꾸지 않는다.** 첫 번째 장치로 대체하면 사용자가
 * 고른 적 없는 마이크로 녹음이 시작되고, **장치가 바뀌었다는 사실 자체가 사라진다**
 * (`phase-prompt/02-reliable-recording.md` Required Outcome 2). 무엇을 쓸지는 사용자가
 * 정하고, 이 모듈은 지금 무엇이 사실인지만 말한다.
 *
 * 순수 함수만 있다. React도 DOM도 Tauri도 알지 않으므로 **장치 목록을 값으로 넣어**
 * 마이크 없이 그대로 판정된다 (§18).
 */
import type { InputDevice } from '../ipc/types';

/** `<select>`가 "고르지 않음"을 나타낼 때 쓰는 값. 폼은 `null`을 담을 수 없다. */
export const NO_DEFAULT_MICROPHONE = '';

/** 아무것도 고르지 않은 항목의 이름. */
export const NO_DEFAULT_MICROPHONE_LABEL = 'No default microphone';

/**
 * 저장돼 있지만 지금 없는 장치의 이름.
 *
 * 장치의 진짜 이름은 알 수 없다 — 저장된 것은 불투명한 선택 키뿐이고, 이름은 그 장치를
 * 열거할 수 있을 때만 나온다. 없는 이름을 지어내지 않고 **없다는 사실을 보여 준다.**
 */
export const MISSING_DEFAULT_MICROPHONE_LABEL = 'Saved microphone (not available)';

/**
 * `<select>`가 들고 있는 값을 저장할 값으로 옮긴다. "고르지 않음"은 `null`이다.
 *
 * 빈 문자열과 `null` 사이의 변환은 이 함수 하나에만 있다 — 두 곳에 생기면 한쪽이
 * "고르지 않음"을 키로 착각하는 순간이 온다.
 */
export function chosenMicrophone(value: string): string | null {
  return value === NO_DEFAULT_MICROPHONE ? null : value;
}

/** 저장된 default microphone이 지금 어떤 상태인가. 이 셋이 전부다. */
export type DefaultMicrophone =
  | { readonly kind: 'notChosen' }
  | { readonly kind: 'available'; readonly device: InputDevice }
  /** 저장된 키는 그대로 남는다 — 없어졌다고 해서 저장된 값을 버리지 않는다. */
  | { readonly kind: 'missing'; readonly key: string };

/**
 * 저장된 선택 키를 지금 열거된 목록과 맞춰 본다.
 *
 * 목록이 비어 있는 것은 오류가 아니다 (마이크가 없거나 전부 빠져 있다). 저장된 값이 있다면
 * 그때의 답은 `missing`이다 — **고를 수 있는 장치가 없다는 이유로 `notChosen`이 되지 않는다.**
 * 둘은 다른 사실이다.
 */
export function resolveDefaultMicrophone(
  saved: string | null,
  devices: readonly InputDevice[],
): DefaultMicrophone {
  if (saved === null) {
    return { kind: 'notChosen' };
  }
  const device = devices.find((candidate) => candidate.key === saved);
  // 찾지 못했을 때 목록의 다른 장치를 꺼내 오는 경로는 여기에도, 아래에도 없다.
  return device === undefined ? { kind: 'missing', key: saved } : { kind: 'available', device };
}

/** 고를 수 있는 항목 하나. */
export interface MicrophoneOption {
  /** 고르면 저장되는 값. `NO_DEFAULT_MICROPHONE`은 "고르지 않음"이다. */
  readonly value: string;
  readonly label: string;
  /** 지금 열거된 장치인가. 저장돼 있지만 지금 없는 장치만 `false`다. */
  readonly available: boolean;
}

/**
 * 화면이 그대로 그릴 수 있는 선택지 목록.
 *
 * "고르지 않음"이 언제나 첫 항목이고, 그다음은 열거된 순서(기본 장치가 먼저) 그대로다.
 *
 * **저장된 장치가 지금 없으면 그 자리를 위한 항목이 하나 더 붙는다.** 없는 값을 고를 수
 * 없는 `<select>`는 저장된 선택을 **말없이 다른 항목으로 보여 주기 때문이다** — 화면에는
 * 첫 장치가 골라진 것처럼 보이고 사용자는 자기 선택이 바뀐 줄 모른다. 그 항목이 있으면
 * 저장된 선택이 선택인 채로 남고, 지금 쓸 수 없다는 사실이 함께 보인다.
 */
export function microphoneOptions(
  saved: string | null,
  devices: readonly InputDevice[],
): MicrophoneOption[] {
  const options: MicrophoneOption[] = [
    { value: NO_DEFAULT_MICROPHONE, label: NO_DEFAULT_MICROPHONE_LABEL, available: true },
    ...devices.map((device) => ({ value: device.key, label: device.label, available: true })),
  ];

  const resolved = resolveDefaultMicrophone(saved, devices);
  if (resolved.kind === 'missing') {
    options.push({
      value: resolved.key,
      label: MISSING_DEFAULT_MICROPHONE_LABEL,
      available: false,
    });
  }
  return options;
}

/**
 * 지금 상태에 대해 사용자에게 할 말. 할 말이 없으면 `null`이다.
 *
 * 저장된 장치가 없어진 것은 실패가 아니라 **사실**이므로 `Failure`로 만들지 않는다.
 * 사용자가 아무것도 하지 않아도 되지만, 모르고 있어서는 안 되는 종류의 사실이다.
 */
export function defaultMicrophoneNotice(resolved: DefaultMicrophone): string | null {
  switch (resolved.kind) {
    case 'notChosen':
      return 'No default microphone chosen yet.';
    case 'missing':
      return 'The saved microphone is not available right now. It stays chosen until you pick another one.';
    case 'available':
      return null;
  }
}
