/**
 * Settings 화면의 **AI provider 구역** (PRODUCT-SPEC §5 D · §12 · §13 ·
 * `docs/ADR-0008-note-ai-provider.md` §4.2 · §11.1).
 *
 * 이 구역이 답해야 하는 질문은 넷이다 — **무엇을 고를 수 있는가** · **어디에 연결하는가** ·
 * **지금 그것이 실제로 응답하는가** · **여기서 무엇이 기기 밖으로 나가는가.** 네 답을 만드는
 * 규칙이 전부 여기 있고, `SettingsScreen`에는 그리는 일만 남는다 (§18).
 *
 * React도 DOM도 Tauri도 알지 않으므로 **실제 Ollama 없이** vitest로 그대로 판정된다 —
 * 연결 확인의 결과는 값으로 들어오고, 이 모듈은 그 값을 화면 상태로 옮길 뿐이다.
 *
 * ## 세 결과를 하나로 뭉치지 않는다 (ADR-0008 §4.2)
 *
 * ```text
 * running     응답했고 쓸 수 있는 모델이 있다
 * noModels    응답했지만 설치된 모델이 하나도 없다   — 오류가 아니다
 * notRunning  지금 응답하지 않는다                   — 서버를 켜면 된다
 * ```
 *
 * 셋을 boolean 하나로 접으면 사용자가 할 일이 사라진다. 모델을 받아 오는 것과 서버를 켜는
 * 것은 다른 일이며, 그래서 backend의 계약도 이 셋을 나눠서 보낸다 ({@link AiProviderState}).
 * 여기에 **아직 확인하지 않았다** · **확인 중이다** · **provider를 고르지 않았다** ·
 * **확인 요청 자체가 거절됐다**가 각각 따로 있다.
 *
 * ## 저장된 선택을 대신 고치지 않는다
 *
 * 고른 모델이 지금 그 서버에 없어도, 고른 provider를 이 앱이 세울 수 없어도 **값을 바꾸거나
 * 지우지 않는다.** `defaultMicrophone.ts`가 없어진 장치에 대해 이미 같은 규칙을 따르고 있고,
 * 이유도 같다 — 조용히 바꾸면 무엇이 바뀐 것인지 말할 수 없게 된다.
 *
 * ## AI가 안 되는 것이 나머지를 막지 않는다 (INV-8)
 *
 * 이 모듈이 만드는 값 중 어떤 것도 {@link SettingsView}가 아니다. 저장 경로가 보는 것은 폼
 * 값뿐이며 (`toSettings`), 연결 확인 결과는 그 입력에 없다 — 그래서 provider가 응답하지
 * 않아도 다른 설정은 그대로 저장된다. 그 사실을 화면에도 적는다
 * ({@link AI_SETTINGS_UNAFFECTED_NOTICE}).
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { AiProviderLocality, AiProviderStatus } from '../ipc/types';
import type { SettingsForm } from './settingsView';

/** `<select>`가 "고르지 않음"을 나타낼 때 쓰는 값. 폼은 `null`을 담을 수 없다. */
export const NO_AI_PROVIDER = '';

/** 아무 provider도 고르지 않은 항목의 이름. **오류가 아니라 정상 상태다** (INV-8). */
export const NO_AI_PROVIDER_LABEL = 'Not set — AI notes are off';

/** `<select>`가 "모델을 고르지 않음"을 나타낼 때 쓰는 값. */
export const NO_AI_MODEL = '';

export const NO_AI_MODEL_LABEL = 'No model chosen';

/**
 * 이 앱이 실제로 세울 수 있는 provider 하나.
 *
 * `locality`가 label 안의 글자가 아니라 **값**인 이유는 §12 때문이다 — 화면이 그리는 문구는
 * 전부 이 값에서 나오며, 그래서 provider가 늘었을 때 문구를 따로 고칠 자리가 없다 (INV-5).
 */
interface SelectableAiProvider {
  /** 저장되는 식별자. `ai_notes.provider`에 그대로 남는다 (ADR-0008 §7.3). */
  readonly id: string;
  /** 사람이 읽는 이름. locality는 여기 섞지 않는다. */
  readonly name: string;
  readonly locality: AiProviderLocality;
}

/**
 * 고를 수 있는 provider의 **전부**.
 *
 * `src-tauri/src/ai/mod.rs`의 `provider_for`가 실제로 세울 수 있는 것과 같아야 한다 — 지금
 * 그것은 Ollama 하나다. 여기에 없는 식별자를 고르면 backend는 아무 provider도 만들지 않으며,
 * 다른 provider로 바꿔 고르지도 않는다.
 *
 * **테스트 전용 double(fake provider)은 이 목록에 없다.** 그것은 계약을 검증하기 위한 것이지
 * 사용자가 고를 수 있는 제품 기능이 아니며 (ADR-0008 §4.3), 목록에 있으면 사용자의 노트가
 * 지어낸 값으로 채워진다. 목록이 리터럴 하나뿐인 이유도 이것이다 — 어디선가 모아 오면
 * 테스트용 구현이 섞여 들어올 자리가 생긴다.
 */
const SELECTABLE_AI_PROVIDERS: readonly SelectableAiProvider[] = [
  { id: 'ollama', name: 'Ollama', locality: 'local' },
];

/** 선택지에 붙는 locality 표시. **provider의 값에서 나오며 문구가 값을 정하지 않는다.** */
const LOCALITY_CHOICE_LABEL: Record<AiProviderLocality, string> = {
  local: 'runs on this device',
  external: 'runs outside this device',
};

/**
 * 저장돼 있지만 이 앱이 세울 수 없는 provider의 이름.
 *
 * 진짜 이름은 알 수 없다 — 저장된 것은 식별자뿐이다. 없는 이름을 지어내지 않고 **저장된
 * 값이 그대로 남아 있다는 사실**을 보여 준다 (`defaultMicrophone.ts`와 같은 규칙).
 */
export const UNKNOWN_AI_PROVIDER_LABEL = 'Saved provider (this version cannot use it)';

/** 고를 수 있는 항목 하나. */
export interface AiProviderChoice {
  /** 고르면 저장되는 값. `NO_AI_PROVIDER`는 "고르지 않음"이다. */
  readonly value: string;
  readonly label: string;
  /** 이 provider가 기기를 떠나는가 (§12 · INV-5). "고르지 않음"에서는 `null`이다. */
  readonly locality: AiProviderLocality | null;
  /** 이 앱이 실제로 세울 수 있는가. 저장돼 있지만 세울 수 없는 항목만 `false`다. */
  readonly usable: boolean;
}

/**
 * 화면이 그대로 그릴 수 있는 provider 선택지.
 *
 * "고르지 않음"이 언제나 첫 항목이다 — 그것이 기본값이고 정상 상태이기 때문이다 (INV-8).
 *
 * **저장된 값을 이 앱이 모르면 그 항목이 하나 더 붙는다.** 없는 값을 고를 수 없는 `<select>`는
 * 저장된 선택을 말없이 다른 항목으로 보여 주고, 그러면 사용자는 자기 선택이 바뀐 줄 모른 채
 * 저장 한 번으로 그 값을 잃는다.
 */
export function aiProviderChoices(chosen: string): readonly AiProviderChoice[] {
  const choices: AiProviderChoice[] = [
    { value: NO_AI_PROVIDER, label: NO_AI_PROVIDER_LABEL, locality: null, usable: true },
    ...SELECTABLE_AI_PROVIDERS.map((provider) => ({
      value: provider.id,
      label: `${provider.name} — ${LOCALITY_CHOICE_LABEL[provider.locality]}`,
      locality: provider.locality,
      usable: true,
    })),
  ];

  if (chosen !== NO_AI_PROVIDER && !choices.some((choice) => choice.value === chosen)) {
    choices.push({
      value: chosen,
      label: UNKNOWN_AI_PROVIDER_LABEL,
      // 무엇인지 모르는 provider가 로컬인지 외부인지도 모른다. 둘 중 하나로 찍지 않는다.
      locality: null,
      usable: false,
    });
  }
  return choices;
}

/**
 * 고른 provider가 기기를 떠나는가. 고르지 않았거나 이 앱이 모르는 식별자면 `null`이다.
 *
 * **모르는 것을 `local`로 두지 않는다.** 그것은 "나가지 않는다"는 말이 되고, §12가 사용자에게
 * 약속하는 것이 정확히 그 문장이기 때문이다.
 */
export function aiProviderLocality(chosen: string): AiProviderLocality | null {
  return SELECTABLE_AI_PROVIDERS.find((provider) => provider.id === chosen)?.locality ?? null;
}

/**
 * 이 provider를 골랐을 때 **무엇이 어디로 나가는가** (§12 · INV-5 · INV-6).
 *
 * 세 문장이 따로 있는 이유는 셋이 다른 사실이기 때문이다 — provider가 어디서 도는가 ·
 * transcript 텍스트가 어떻게 되는가 · **오디오는 어떻게 되는가.** 앞의 둘은 locality에 따라
 * 달라지고, 마지막 하나는 어느 쪽에서도 같다.
 */
export interface AiTransferNotice {
  readonly locality: AiProviderLocality;
  /** provider가 어디서 도는가. */
  readonly headline: string;
  /** transcript 텍스트가 어떻게 되는가. */
  readonly transcriptText: string;
  /** 오디오는 나가지 않는다 (INV-6). locality와 무관하게 같은 사실이다. */
  readonly audioText: string;
}

/** provider가 어디서 도는가. */
const LOCALITY_HEADLINE: Record<AiProviderLocality, string> = {
  local: 'This provider runs on this device.',
  external: 'This provider runs outside this device.',
};

/** 그래서 transcript 텍스트가 어떻게 되는가. */
const LOCALITY_TRANSCRIPT_TEXT: Record<AiProviderLocality, string> = {
  local: 'The transcript text is sent to it and stays on this device.',
  external: 'The transcript text is sent to it, so it leaves this device.',
};

/**
 * **오디오는 어느 provider에게도 전송되지 않는다** (INV-6).
 *
 * 이것은 문구가 아니라 계약이다 — provider에게 넘기는 입력에 오디오를 가리킬 수 있는 필드가
 * 아예 없어서 adapter는 보내고 싶어도 보낼 것이 없다 (ADR-0008 §4.2). 그래서 이 문장은
 * locality에 따라 달라지지 않는다.
 */
export const AUDIO_IS_NEVER_SENT =
  'Audio is never sent. Only the transcript text is used, and the recording file stays on this device.';

/** 아무 provider도 고르지 않았을 때. 나가는 것이 없으므로 나가는 이야기를 하지 않는다. */
export const NOTHING_LEAVES_THIS_DEVICE = 'No AI provider is set, so nothing is sent anywhere.';

/**
 * 고른 provider의 전송 경계. 고르지 않았거나 이 앱이 모르는 식별자면 `null`이다.
 *
 * `null`일 때 화면이 그리는 것은 {@link NOTHING_LEAVES_THIS_DEVICE}다 — 모르는 provider에
 * 대해 "나가지 않는다"고 말하지 않기 위해서다.
 */
export function aiTransferNotice(locality: AiProviderLocality | null): AiTransferNotice | null {
  if (locality === null) {
    return null;
  }
  return {
    locality,
    headline: LOCALITY_HEADLINE[locality],
    transcriptText: LOCALITY_TRANSCRIPT_TEXT[locality],
    audioText: AUDIO_IS_NEVER_SENT,
  };
}

/**
 * 연결 확인이 지금까지 말해 준 것 (`phase-prompt/04` 요구 8 · §13).
 *
 * ```text
 * notChecked      아직 물어보지 않았다
 * checking        물어보는 중이다
 * notConfigured   고른 provider가 없어 물어볼 대상이 없다  — 오류가 아니다 (INV-8)
 * running         응답했고 쓸 수 있는 모델이 있다
 * noModels        응답했지만 설치된 모델이 하나도 없다     — 오류가 아니다
 * notRunning      지금 응답하지 않는다                     — 서버를 켜면 된다 (§13)
 * checkFailed     확인 요청 자체가 거절됐다                — provider와 무관한 실패다
 * ```
 *
 * `noModels`와 `notRunning`을 나누는 이유는 사용자가 할 일이 다르기 때문이다. `checkFailed`가
 * 따로 있는 이유는 그것이 **AI에 대한 사실이 아니기 때문이다** — 저장된 설정을 읽지 못한 것이
 * 여기로 오며, provider가 응답하는지는 여전히 알지 못한다.
 */
export type AiConnection =
  | { readonly kind: 'notChecked'; readonly text: string }
  | { readonly kind: 'checking'; readonly text: string }
  | { readonly kind: 'notConfigured'; readonly text: string; readonly resolution: string }
  | {
      readonly kind: 'running';
      readonly text: string;
      /** provider가 스스로 말한 이름 (INV-9). 말하지 않았으면 `null`이다. */
      readonly providerName: string | null;
      /** provider가 스스로 말한 locality (§12 · INV-5). */
      readonly locality: AiProviderLocality | null;
      /** 실제로 설치돼 있는 모델. 여기서 고른다. */
      readonly models: readonly string[];
    }
  | {
      readonly kind: 'noModels';
      readonly text: string;
      readonly resolution: string;
      readonly providerName: string | null;
      readonly locality: AiProviderLocality | null;
    }
  | {
      readonly kind: 'notRunning';
      readonly text: string;
      readonly resolution: string;
      readonly providerName: string | null;
      readonly locality: AiProviderLocality | null;
      /** 닿지 못한 이유 (§13). backend가 말해 주지 않았으면 `null`이다. */
      readonly failure: Failure | null;
    }
  | { readonly kind: 'checkFailed'; readonly text: string; readonly failure: Failure };

/** 아직 물어보지 않았다. 화면을 열자마자 확인하러 나가지 않는다. */
export const AI_NOT_CHECKED_TEXT = 'The AI provider has not been checked yet.';

export const CHECKING_AI_PROVIDER_TEXT = 'Checking the AI provider…';

/** 고른 provider가 없다. **경고가 아니다** (INV-8). */
export const NO_AI_PROVIDER_TEXT = 'No AI provider is set, so there is nothing to check.';

export const HOW_TO_SET_AN_AI_PROVIDER = 'Choose a provider above to turn AI notes on.';

/** 응답했다. */
export const AI_PROVIDER_RUNNING_TEXT = 'The AI provider answered.';

/** 응답했지만 모델이 없다. **오류가 아니라 사실이다.** */
export const AI_PROVIDER_HAS_NO_MODELS_TEXT =
  'The AI provider answered, but no models are installed on it.';

export const HOW_TO_INSTALL_A_MODEL =
  'Install a model on the provider, then check again — the list here comes from the provider itself.';

/** 응답하지 않는다. **재촉하지 않고 무엇을 하면 되는지만 적는다** (§13). */
export const AI_PROVIDER_NOT_RUNNING_TEXT = 'The AI provider did not answer.';

export const HOW_TO_REACH_THE_AI_PROVIDER =
  'Start the provider on this machine, or point the address below at where it is running, then check again.';

/** 확인 요청 자체가 거절됐다. provider가 응답하는지는 여전히 알지 못한다. */
export const AI_CHECK_FAILED_TEXT = 'The AI provider could not be checked.';

/** 확인은 **저장된** 설정에게 물어본다 — `ai_provider_status`는 저장소의 값을 읽는다. */
export const AI_CHECK_USES_SAVED_SETTINGS =
  'The check asks the settings that are already saved. Save first to check a new provider, address, or model.';

/** AI 쪽이 안 되어도 이 화면의 나머지는 그대로다 (INV-8). */
export const AI_SETTINGS_UNAFFECTED_NOTICE =
  'Every other setting on this screen still saves normally, whether or not the AI provider answers.';

/**
 * backend가 답한 provider 상태를 화면 상태로 옮긴다.
 *
 * **네 상태를 네 갈래로 그대로 옮긴다.** 여기서 다시 뭉치거나 나누지 않는다 — 나눈 것은
 * 계약이고 (ADR-0008 §4.2), 화면이 그것을 다시 접으면 계약이 나눈 이유가 사라진다.
 *
 * `providerName`과 `locality`는 **provider가 스스로 말한 값**이므로 이 모듈이 채워 넣지
 * 않는다 (INV-9). 말하지 않았으면 `null`인 채로 남는다.
 */
export function checkedAiProvider(status: AiProviderStatus): AiConnection {
  const said = { providerName: status.providerName, locality: status.locality };

  switch (status.state) {
    case 'notConfigured':
      return {
        kind: 'notConfigured',
        text: NO_AI_PROVIDER_TEXT,
        resolution: HOW_TO_SET_AN_AI_PROVIDER,
      };
    case 'ready':
      return { kind: 'running', text: AI_PROVIDER_RUNNING_TEXT, models: status.models, ...said };
    case 'noModels':
      return {
        kind: 'noModels',
        text: AI_PROVIDER_HAS_NO_MODELS_TEXT,
        resolution: HOW_TO_INSTALL_A_MODEL,
        ...said,
      };
    case 'unavailable':
      return {
        kind: 'notRunning',
        text: AI_PROVIDER_NOT_RUNNING_TEXT,
        resolution: HOW_TO_REACH_THE_AI_PROVIDER,
        failure: status.failure,
        ...said,
      };
  }
}

/**
 * 확인 요청 자체가 거절됐다.
 *
 * **`notRunning`과 다른 사실이다.** 저장된 설정을 읽지 못한 것이 여기로 오며 (§13), 그때
 * provider가 응답하는지는 아무도 물어보지 못했다. 둘을 같은 값으로 만들면 화면이 "서버를
 * 켜세요"라고 말하게 되는데, 켜져 있어도 달라지지 않는다.
 */
export function failedAiCheck(error: unknown): AiConnection {
  return { kind: 'checkFailed', text: AI_CHECK_FAILED_TEXT, failure: toFailure(error) };
}

/** 확인된 목록에서 고르는 모델 항목 하나. */
export interface AiModelOption {
  /** 고르면 저장되는 값. `NO_AI_MODEL`은 "고르지 않음"이다. */
  readonly value: string;
  readonly label: string;
  /** 방금 확인한 목록에 있는가. 저장돼 있지만 지금 없는 모델만 `false`다. */
  readonly installed: boolean;
}

/** 저장돼 있지만 지금 그 서버에 없는 모델임을 label에 남긴다. */
export const MISSING_AI_MODEL_SUFFIX = ' (not installed right now)';

/**
 * 화면이 그대로 그릴 수 있는 모델 선택지.
 *
 * 목록은 **확인이 돌려준 것**이다 — 이 모듈이 모델 이름을 알지 않으며, 아직 확인하지 않았으면
 * 빈 목록이 온다. 그것은 "모델이 없다"가 아니라 "아직 물어보지 않았다"이고, 그 구분은
 * {@link AiConnection}이 말한다.
 *
 * `microphoneOptions`와 같은 규칙으로 **저장된 선택이 목록에 없으면 항목이 하나 더 붙는다.**
 */
export function aiModelOptions(
  chosen: string,
  models: readonly string[],
): readonly AiModelOption[] {
  const options: AiModelOption[] = [
    { value: NO_AI_MODEL, label: NO_AI_MODEL_LABEL, installed: true },
    ...models.map((model) => ({ value: model, label: model, installed: true })),
  ];

  if (chosen !== NO_AI_MODEL && !models.includes(chosen)) {
    options.push({ value: chosen, label: `${chosen}${MISSING_AI_MODEL_SUFFIX}`, installed: false });
  }
  return options;
}

/** 확인이 돌려준 모델 목록. 확인이 그 답을 주지 않은 상태에서는 빈 목록이다. */
export function confirmedAiModels(connection: AiConnection): readonly string[] {
  return connection.kind === 'running' ? connection.models : [];
}

/** 아직 모델을 고르지 않았다. 확인된 목록이 있을 때만 할 수 있는 말이다. */
export const NO_AI_MODEL_CHOSEN_NOTICE =
  'No model chosen yet. Pick one from the list the provider reported.';

/** 고른 모델이 지금 그 서버에 없다. **그래도 고른 값은 그대로 남는다.** */
export const MISSING_AI_MODEL_NOTICE =
  'The chosen model is not installed on the provider right now. It stays chosen until you pick another one.';

/**
 * 고른 모델에 대해 사용자에게 할 말. 할 말이 없으면 `null`이다.
 *
 * **확인이 모델 목록을 돌려준 뒤에만 말한다.** 물어보지 않았거나 응답이 없는 상태에서
 * "그 모델은 없다"고 말하면 알지 못하는 것을 아는 것처럼 적는 것이 된다.
 */
export function aiModelNotice(chosen: string, connection: AiConnection): string | null {
  if (connection.kind !== 'running') {
    return null;
  }
  if (chosen === NO_AI_MODEL) {
    return NO_AI_MODEL_CHOSEN_NOTICE;
  }
  return connection.models.includes(chosen) ? null : MISSING_AI_MODEL_NOTICE;
}

/**
 * 저장된 AI 설정 세 값 — 확인이 실제로 물어본 대상.
 *
 * 폼 값과 이것이 다르면 화면에 보이는 확인 결과는 **지금 편집 중인 값에 대한 답이 아니다.**
 */
export interface AiSettingsSnapshot {
  readonly provider: string;
  readonly baseUrl: string;
  readonly model: string;
}

/** 폼에서 AI 세 값만 떼어 낸다. 저장할 때와 같은 규칙으로 앞뒤 공백을 뺀다. */
export function aiSettingsSnapshot(form: SettingsForm): AiSettingsSnapshot {
  return {
    provider: form.aiProvider.trim(),
    baseUrl: form.aiBaseUrl.trim(),
    model: form.aiModel.trim(),
  };
}

/**
 * 마지막으로 저장한 뒤 AI 설정이 바뀌었는가.
 *
 * 아직 무엇이 저장돼 있는지 모르면(`null`) 바뀌었다고 말하지 않는다 — 모르는 것을 사실처럼
 * 적지 않는다.
 */
export function aiSettingsChanged(form: SettingsForm, saved: AiSettingsSnapshot | null): boolean {
  if (saved === null) {
    return false;
  }
  const current = aiSettingsSnapshot(form);
  return (
    current.provider !== saved.provider ||
    current.baseUrl !== saved.baseUrl ||
    current.model !== saved.model
  );
}

/** 주소 입력란에 적히는 안내. **기본 주소를 여기에 옮겨 적지 않는다.** */
export const AI_BASE_URL_PLACEHOLDER = 'Leave empty to use the built-in address';

/**
 * 주소를 고르지 않아도 된다는 사실.
 *
 * 기본 주소가 무엇인지 이 화면이 적지 않는 이유는 그 값이 한 곳에만 있기 때문이다
 * (`src-tauri/src/domain/settings.rs`의 `DEFAULT_AI_BASE_URL`). 두 곳에 적히면 한 곳을 고쳤을
 * 때 화면이 조용히 거짓말을 하게 된다.
 */
export const AI_BASE_URL_NOTICE =
  'Where the provider is listening, as host and port. Leave it empty and the app connects to its built-in address for that provider.';
