/**
 * Settings 화면의 상태 (PRODUCT-SPEC §5 D).
 *
 * command 호출 결과를 화면 상태로 바꾸는 규칙만 있다. React도 DOM도 Tauri도 알지 않으므로
 * 읽기 · 편집 · 저장 · 실패 경로를 vitest로 그대로 판정할 수 있다 (§18).
 *
 * **INV-7: secret이 없다.** API key · integration token은 이 상태에도, 폼에도 없다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { Settings } from '../ipc/types';

/**
 * 입력란이 들고 있는 값.
 *
 * `recordingsDirectory`가 `Settings`와 달리 `null`이 아니라 빈 문자열인 이유는 하나다 —
 * 텍스트 입력은 `null`을 표현할 수 없다. 두 표현 사이의 변환은 이 모듈에만 있다.
 */
export interface SettingsForm {
  readonly recordingsDirectory: string;
  readonly automaticProcessing: boolean;
}

/**
 * 화면이 놓일 수 있는 상태의 전부.
 *
 * `failed`는 **설정을 읽지 못한** 상태다 — 편집할 값 자체가 없다. 저장에 실패한 것은
 * 편집 중인 값이 그대로 남아 있는 `ready`의 한 모습이며(`failure`가 채워진다), 둘을 섞지 않는다.
 */
export type SettingsView =
  | { readonly kind: 'loading' }
  | {
      readonly kind: 'ready';
      readonly form: SettingsForm;
      readonly saving: boolean;
      /** 마지막 저장이 성공했고, 그 뒤로 편집이 없었다. */
      readonly saved: boolean;
      /** 마지막 저장이 실패했다면 그 실패. 성공했거나 아직 저장하지 않았으면 `null`이다. */
      readonly failure: Failure | null;
    }
  | { readonly kind: 'failed'; readonly failure: Failure };

export const LOADING_SETTINGS: SettingsView = { kind: 'loading' };

/** 저장된 설정을 읽었다. 저장된 적이 없어 기본값이 온 것도 정상이다. */
export function loadedSettings(settings: Settings): SettingsView {
  return ready(toForm(settings));
}

/** 설정을 읽지 못했다. 저장소 초기화 실패가 여기로 온다 (§13). */
export function failedSettings(error: unknown): SettingsView {
  return { kind: 'failed', failure: toFailure(error) };
}

/**
 * 입력란 하나가 바뀌었다.
 *
 * 아직 읽지 못했거나 읽지 못한 상태에서는 편집할 값이 없으므로 상태가 바뀌지 않는다.
 */
export function editedSettings(view: SettingsView, change: Partial<SettingsForm>): SettingsView {
  if (view.kind !== 'ready') {
    return view;
  }
  // 편집한 순간 "저장됨"은 더 이상 사실이 아니다. 직전 저장 실패는 아직 사실이므로 남긴다.
  return { ...view, form: { ...view.form, ...change }, saved: false };
}

/** 저장을 시작했다. 이전 저장 실패는 이 시점에 지운다 — 지금 무엇이 진행 중인지가 답이다. */
export function savingSettings(view: SettingsView): SettingsView {
  if (view.kind !== 'ready') {
    return view;
  }
  return { ...view, saving: true, saved: false, failure: null };
}

/**
 * 저장이 끝났다. 폼은 **저장소가 돌려준 값**으로 다시 채운다.
 *
 * 화면이 "무엇이 저장됐는가"를 추측하지 않게 하기 위해서다 — 공백만 있던 디렉터리처럼
 * Rust가 정규화한 값이 있으면 그 값이 그대로 보인다 (`SettingsPayload`).
 */
export function savedSettings(settings: Settings): SettingsView {
  return { kind: 'ready', form: toForm(settings), saving: false, saved: true, failure: null };
}

/**
 * 저장이 실패했다.
 *
 * 편집 중이던 값은 그대로 둔다 — 실패했다고 사용자가 입력한 것을 버리지 않는다.
 * 읽기 자체가 되지 않은 상태에서 온 실패라면 읽지 못한 화면으로 남는다.
 */
export function failedSave(view: SettingsView, error: unknown): SettingsView {
  const failure = toFailure(error);
  if (view.kind !== 'ready') {
    return { kind: 'failed', failure };
  }
  return { ...view, saving: false, saved: false, failure };
}

/** 폼 값을 저장할 수 있는 설정으로 옮긴다. 빈 입력은 "고르지 않음"(`null`)이다. */
export function toSettings(form: SettingsForm): Settings {
  const directory = form.recordingsDirectory.trim();
  return {
    recordingsDirectory: directory === '' ? null : directory,
    automaticProcessing: form.automaticProcessing,
  };
}

/** 설정을 폼 값으로 옮긴다. 고르지 않은 디렉터리(`null`)는 빈 입력이다. */
export function toForm(settings: Settings): SettingsForm {
  return {
    recordingsDirectory: settings.recordingsDirectory ?? '',
    automaticProcessing: settings.automaticProcessing,
  };
}

function ready(form: SettingsForm): SettingsView {
  return { kind: 'ready', form, saving: false, saved: false, failure: null };
}
