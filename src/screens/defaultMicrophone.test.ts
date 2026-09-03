// 저장된 default microphone을 지금 있는 장치와 맞춰 보는 규칙의 테스트.
//
// **장치 목록을 값으로 넣는다.** 실제 마이크도 마이크 권한도 필요하지 않다 (§18).
//
// 이 파일이 지키는 것 하나: 저장된 장치가 사라졌을 때 **다른 장치가 그 자리를 대신하지
// 않는다.** 조용한 대체는 화면에서는 아무 일도 없었던 것처럼 보이지만, 사용자가 고른 적
// 없는 마이크로 녹음이 시작된다는 뜻이다.
import { describe, expect, it } from 'vitest';
import type { InputDevice } from '../ipc/types';
import {
  chosenMicrophone,
  defaultMicrophoneNotice,
  microphoneOptions,
  MISSING_DEFAULT_MICROPHONE_LABEL,
  NO_DEFAULT_MICROPHONE,
  resolveDefaultMicrophone,
} from './defaultMicrophone';

/** 열거된 목록 하나. Rust가 보내는 모양 그대로다 (기본 장치가 먼저 온다). */
const DEVICES: InputDevice[] = [
  { key: '0:MacBook Pro Microphone', label: 'MacBook Pro Microphone', isDefault: true },
  { key: '0:USB Microphone', label: 'USB Microphone', isDefault: false },
  { key: '1:USB Microphone', label: 'USB Microphone (2)', isDefault: false },
];

describe('저장된 값과 지금 있는 장치를 맞춰 본다', () => {
  it('고른 적이 없으면 "고르지 않음"이다', () => {
    expect(resolveDefaultMicrophone(null, DEVICES)).toEqual({ kind: 'notChosen' });
  });

  it('저장된 장치가 목록에 있으면 그 장치를 돌려준다', () => {
    const resolved = resolveDefaultMicrophone('1:USB Microphone', DEVICES);

    expect(resolved).toEqual({ kind: 'available', device: DEVICES[2] });
  });

  it('저장된 장치가 지금 없으면 그 사실이 별도의 결과다', () => {
    const resolved = resolveDefaultMicrophone('0:Studio Mic', DEVICES);

    expect(resolved.kind).toBe('missing');
    // 저장된 키는 버려지지 않는다 — 장치가 돌아오면 다시 그 장치다.
    expect(resolved.kind === 'missing' && resolved.key).toBe('0:Studio Mic');
  });

  it('없어진 장치가 첫 번째 장치로 바뀌지 않는다', () => {
    const resolved = resolveDefaultMicrophone('0:Studio Mic', DEVICES);

    expect(resolved.kind).not.toBe('available');
    expect(JSON.stringify(resolved)).not.toContain('MacBook Pro Microphone');
  });

  it('고르지 않음과 없어짐은 서로 다른 답이다', () => {
    // 둘을 같은 것으로 만들면 "장치가 바뀌었다"는 사실이 사라진다.
    expect(resolveDefaultMicrophone(null, DEVICES).kind).toBe('notChosen');
    expect(resolveDefaultMicrophone('0:Studio Mic', DEVICES).kind).toBe('missing');
  });

  it('장치가 하나도 없어도 저장된 선택은 "없어짐"이지 "고르지 않음"이 아니다', () => {
    // 마이크를 전부 뽑아 둔 상태다. 목록이 비어 있는 것은 오류가 아니다.
    expect(resolveDefaultMicrophone('0:Studio Mic', []).kind).toBe('missing');
    expect(resolveDefaultMicrophone(null, []).kind).toBe('notChosen');
  });

  it('키가 정확히 같을 때만 같은 장치다', () => {
    // 이름이 같은 장치가 둘 있을 수 있어서 이름으로 맞추지 않는다.
    expect(resolveDefaultMicrophone('USB Microphone', DEVICES).kind).toBe('missing');
    expect(resolveDefaultMicrophone('0:usb microphone', DEVICES).kind).toBe('missing');
  });
});

describe('고를 수 있는 항목', () => {
  it('첫 항목은 언제나 "고르지 않음"이다', () => {
    const [first] = microphoneOptions(null, DEVICES);

    expect(first.value).toBe(NO_DEFAULT_MICROPHONE);
    expect(first.label.length).toBeGreaterThan(0);
  });

  it('열거된 장치가 목록 순서 그대로 들어온다', () => {
    const options = microphoneOptions(null, DEVICES);

    expect(options.slice(1).map((option) => option.value)).toEqual(
      DEVICES.map((device) => device.key),
    );
    expect(options.slice(1).map((option) => option.label)).toEqual(
      DEVICES.map((device) => device.label),
    );
    expect(options.every((option) => option.available)).toBe(true);
  });

  it('저장된 장치가 지금 없으면 그 선택을 위한 항목이 함께 있다', () => {
    // 없으면 select가 저장된 값을 표현할 수 없어 **다른 항목이 골라진 것처럼 보인다.**
    const options = microphoneOptions('0:Studio Mic', DEVICES);
    const missing = options.find((option) => option.value === '0:Studio Mic');

    expect(missing).toBeDefined();
    expect(missing?.available).toBe(false);
    expect(missing?.label).toBe(MISSING_DEFAULT_MICROPHONE_LABEL);
    // 지금 있는 장치는 그대로 전부 고를 수 있다.
    for (const device of DEVICES) {
      expect(options.some((option) => option.value === device.key && option.available)).toBe(true);
    }
  });

  it('없어진 장치의 항목은 그때만 생긴다', () => {
    for (const saved of [null, '0:USB Microphone']) {
      expect(microphoneOptions(saved, DEVICES).every((option) => option.available)).toBe(true);
    }
  });

  it('고른 값이 언제나 목록 안에 있다', () => {
    // 이것이 깨지면 화면이 보여 주는 선택과 저장된 선택이 조용히 갈라진다.
    for (const saved of [null, '0:USB Microphone', '0:Studio Mic']) {
      const value = saved ?? NO_DEFAULT_MICROPHONE;
      expect(microphoneOptions(saved, DEVICES).some((option) => option.value === value)).toBe(true);
    }
  });
});

describe('사용자에게 할 말', () => {
  it('장치가 있으면 할 말이 없다', () => {
    const resolved = resolveDefaultMicrophone('0:USB Microphone', DEVICES);

    expect(defaultMicrophoneNotice(resolved)).toBeNull();
  });

  it('고르지 않은 상태와 없어진 상태는 서로 다른 문장이다', () => {
    const notChosen = defaultMicrophoneNotice(resolveDefaultMicrophone(null, DEVICES));
    const missing = defaultMicrophoneNotice(resolveDefaultMicrophone('0:Studio Mic', DEVICES));

    expect(notChosen).not.toBeNull();
    expect(missing).not.toBeNull();
    expect(missing).not.toBe(notChosen);
  });
});

describe('폼 값과 저장 값의 변환', () => {
  it('"고르지 않음"은 null이고 나머지는 키 그대로다', () => {
    expect(chosenMicrophone(NO_DEFAULT_MICROPHONE)).toBeNull();
    expect(chosenMicrophone('0:Studio Mic')).toBe('0:Studio Mic');
  });
});
