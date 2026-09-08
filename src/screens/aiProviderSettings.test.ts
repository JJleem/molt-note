// Settings 화면 AI provider 구역의 판단 규칙 테스트.
//
// 여기서 보는 것은 네 가지다 — **무엇을 고를 수 있는가** · **연결 확인의 결과가 서로 구분되어
// 표현되는가** · **로컬/외부와 오디오 미전송이 값에서 나오는가** · **AI가 안 되는 것이 다른
// 설정 저장을 막지 않는가**.
//
// **Ollama도 네트워크도 DOM도 Tauri도 필요하지 않다** (PRODUCT-SPEC §18). 확인의 결과는 값으로
// 들어오고, 이 모듈은 그 값을 화면 상태로 옮길 뿐이기 때문이다.
import { describe, expect, it } from 'vitest';
import type { Failure } from '../ipc/failure';
import type { AiProviderStatus, Settings } from '../ipc/types';
import {
  AI_BASE_URL_NOTICE,
  AI_CHECK_FAILED_TEXT,
  AI_IS_OPTIONAL_TEXT,
  AI_KEY_INPUT_NOTICE,
  AI_NOT_CHECKED_TEXT,
  AI_PROVIDER_HAS_NO_MODELS_TEXT,
  AI_PROVIDER_NOT_RUNNING_TEXT,
  AI_PROVIDER_RUNNING_TEXT,
  AI_SECTION_TITLE,
  AI_SETTINGS_UNAFFECTED_NOTICE,
  AI_WITHOUT_A_PROVIDER_TEXT,
  AUDIO_IS_NEVER_SENT,
  CONNECTED_PROVIDER_TEXT,
  CONNECTED_PROVIDER_TITLE,
  MISSING_AI_MODEL_NOTICE,
  MISSING_AI_MODEL_SUFFIX,
  NO_AI_MODEL,
  NO_AI_MODEL_CHOSEN_NOTICE,
  NO_AI_PROVIDER,
  NO_AI_PROVIDER_TEXT,
  OPTIONAL_TITLE_SUFFIX,
  UNKNOWN_AI_PROVIDER_LABEL,
  aiCredentialNotice,
  aiCredentialState,
  aiModelNotice,
  aiModelOptions,
  aiProviderChoices,
  aiProviderLocality,
  aiSettingsChanged,
  aiSettingsSnapshot,
  aiTransferNotice,
  checkedAiProvider,
  confirmedAiModels,
  failedAiCheck,
  localProviderSetups,
  needsApiKey,
  type AiConnection,
} from './aiProviderSettings';
import {
  editedSettings,
  loadedSettings,
  savedSettings,
  savingSettings,
  toForm,
  toSettings,
  type SettingsForm,
} from './settingsView';

const DEFAULT_SETTINGS: Settings = {
  recordingsDirectory: null,
  automaticProcessing: false,
  automaticTranscription: false,
  transcriptionModel: null,
  // 언어를 고르지 않은 것이 기본이며, 그것은 자동 감지다 (ADR-0007 §17.1.4-1).
  transcriptionLanguage: null,
  defaultMicrophone: null,
  aiProvider: null,
  aiBaseUrl: null,
  aiModel: null,
  notionParentPageId: null,
};

function form(changes: Partial<SettingsForm> = {}): SettingsForm {
  return { ...toForm(DEFAULT_SETTINGS), ...changes };
}

/** backend가 답한 provider 상태 하나. 테스트가 관심 있는 값만 적는다. */
function status(changes: Partial<AiProviderStatus> = {}): AiProviderStatus {
  return {
    state: 'notConfigured',
    providerId: null,
    providerName: null,
    locality: null,
    models: [],
    failure: null,
    ...changes,
  };
}

const unreachable: Failure = {
  kind: 'aiProviderUnreachable',
  message: '로컬 AI 서버에 연결하지 못했다.',
  detail: 'connection refused',
  sourceDataSafe: true,
  retryable: true,
};

const storageFailure: Failure = {
  kind: 'storage',
  message: '로컬 저장소를 열지 못했다.',
  detail: 'unable to open database file',
  sourceDataSafe: true,
  retryable: true,
};

describe('provider 선택지', () => {
  it('고르지 않음이 언제나 첫 항목이다', () => {
    // 고르지 않은 것이 기본이고 정상 상태다 (INV-8).
    const [first] = aiProviderChoices(NO_AI_PROVIDER);

    expect(first.value).toBe(NO_AI_PROVIDER);
    expect(first.locality).toBeNull();
    expect(first.usable).toBe(true);
  });

  it('지금 고를 수 있는 provider는 둘이고, 하나는 기기 밖으로 나간다', () => {
    // 2026-09-08에 하나가 늘었다 (PRODUCT-SPEC §16.1). **목록은 `provider_for`가 실제로
    // 세울 수 있는 것과 같아야 한다** — 여기에만 있고 backend에 없으면 사용자가 고를 수는
    // 있는데 아무 일도 일어나지 않는 항목이 생긴다.
    const values = aiProviderChoices(NO_AI_PROVIDER).map((choice) => choice.value);

    expect(values).toEqual([NO_AI_PROVIDER, 'ollama', 'anthropic']);
    expect(aiProviderLocality('ollama')).toBe('local');
    expect(aiProviderLocality('anthropic')).toBe('external');
  });

  /**
   * **§12 · INV-5.** 기기를 떠나는 provider가 있다는 사실이 값으로 있어야 하고, 화면은 그
   * 값에서 문구를 만든다. 이 검사가 깨지면 사용자가 무엇을 고르는지 모른 채 전사를
   * 밖으로 보내게 된다.
   */
  it('기기를 떠나는 provider는 선택지에서 그 사실을 값으로 말한다', () => {
    const external = aiProviderChoices(NO_AI_PROVIDER).filter(
      (choice) => choice.locality === 'external',
    );

    expect(external.length).toBeGreaterThan(0);
    for (const choice of external) {
      expect(choice.usable).toBe(true);
      // locality는 label의 글자가 아니라 값에서 온다 — label에도 드러나야 한다.
      expect(choice.label.length).toBeGreaterThan(0);
    }
  });

  it('테스트 전용 fake provider가 선택지에 없다', () => {
    // 계약을 검증하는 double은 제품 기능이 아니다 (ADR-0008 §4.3). 목록에 있으면 사용자의
    // 노트가 지어낸 값으로 채워진다.
    for (const choice of aiProviderChoices(NO_AI_PROVIDER)) {
      expect(choice.value).not.toMatch(/fake|stub|double|dummy|test/i);
      expect(choice.label).not.toMatch(/fake|stub|double|dummy|test/i);
    }
    // 저장된 값으로 들어오면 사라지지 않고 남지만, **고를 수 있는 것**이 되지는 않는다 —
    // 이 앱은 그것을 세우지 못하며 (`ai::provider_for`), locality도 말하지 못한다.
    expect(aiProviderChoices('fake').find((choice) => choice.value === 'fake')?.usable).toBe(false);
    expect(aiProviderLocality('fake')).toBeNull();
  });

  it('이 앱이 세울 수 없는 저장된 값도 선택지에 남는다', () => {
    // 없는 값을 고를 수 없는 <select>는 저장된 선택을 말없이 다른 항목으로 보여 준다.
    const choices = aiProviderChoices('어떤-다른-provider');
    const saved = choices.find((choice) => choice.value === '어떤-다른-provider');

    expect(saved).toEqual({
      value: '어떤-다른-provider',
      label: UNKNOWN_AI_PROVIDER_LABEL,
      // 무엇인지 모르는 provider가 로컬인지도 모른다. 둘 중 하나로 찍지 않는다.
      locality: null,
      usable: false,
    });
  });

  it('선택지 이름의 로컬/외부 표시가 provider의 locality에서 나온다', () => {
    const ollama = aiProviderChoices('ollama').find((choice) => choice.value === 'ollama');

    expect(ollama?.locality).toBe('local');
    expect(ollama?.label).toContain('runs on this device');
  });
});

describe('구역의 두 부분 (요구 7 · ADR-0010 §4.3)', () => {
  /** 이 앱이 고르게 하지 않는 벤더들. 재배치가 새 provider를 데려오지 않았다는 것을 본다. */
  const OTHER_VENDORS = /openai|gpt|anthropic|claude|gemini|codex|groq|mistral|huggingface/i;

  it('스스로 세우는 provider의 이름과 로컬 표시가 provider 목록에서 나온다 (INV-9 · INV-5)', () => {
    // 벤더 이름을 두 번째로 적는 자리를 만들지 않는다. 제목은 선택지 label 그대로에
    // '선택'이라는 표시만 붙은 것이어야 한다.
    const setups = localProviderSetups(NO_AI_PROVIDER);

    expect(setups.length).toBeGreaterThan(0);
    for (const setup of setups) {
      const choice = aiProviderChoices(NO_AI_PROVIDER).find((one) => one.value === setup.id);

      expect(choice, '목록에 없는 provider가 이 부분에 있다').toBeDefined();
      expect(choice?.locality).toBe('local');
      expect(setup.title).toBe(`${choice?.label}${OPTIONAL_TITLE_SUFFIX}`);
      expect(setup.title).toContain('runs on this device');
    }
  });

  it('재배치가 새 provider를 데려오지 않았다 (요구 8)', () => {
    const values = aiProviderChoices(NO_AI_PROVIDER).map((choice) => choice.value);
    const setups = localProviderSetups(NO_AI_PROVIDER);

    // 고를 수 있는 것은 그대로다 — 이 부분은 목록에서 걸러 오기만 한다.
    for (const setup of setups) {
      expect(values).toContain(setup.id);
    }
    // 그리고 어느 문장도 다른 벤더를 부르지 않는다 (MH-6).
    for (const setup of setups) {
      for (const sentence of [
        setup.title,
        setup.standing,
        setup.text,
        setup.statusText,
        setup.howToTurnOn,
      ]) {
        expect(sentence).not.toMatch(OTHER_VENDORS);
      }
    }
  });

  it('선택이고 고급이라는 것이 자리가 아니라 문장으로 있다 (요구 7)', () => {
    const [setup] = localProviderSetups(NO_AI_PROVIDER);

    expect(setup.standing).toMatch(/optional/i);
    expect(setup.standing).toMatch(/advanced/i);
    // 설치와 실행의 주체가 사용자이고, 앱이 그것을 필요로 하지 않는다는 사실.
    expect(setup.text).toMatch(/yourself/i);
    expect(setup.text).toMatch(/does not need it/i);
    // 켜라는 요구가 아니라 켜는 방법이다.
    expect(setup.howToTurnOn).toMatch(/to use it/i);
  });

  it('고르지 않은 상태가 결함이 아니라 정상으로 적힌다 (INV-8)', () => {
    const [notChosen] = localProviderSetups(NO_AI_PROVIDER);
    const [chosen] = localProviderSetups('ollama');

    expect(notChosen.chosen).toBe(false);
    expect(chosen.chosen).toBe(true);
    // 두 상태의 문장이 다르다 — 화면이 둘을 같은 말로 뭉치지 않는다.
    expect(notChosen.statusText).not.toBe(chosen.statusText);
    // 그리고 고르지 않은 쪽이 오류의 언어를 쓰지 않는다.
    expect(notChosen.statusText).not.toMatch(/error|failed|must|required|missing/i);
    expect(notChosen.statusText).toMatch(/nothing in the app is waiting/i);
  });

  it('고른 값이 무엇이든 이 부분이 값을 바꾸지 않는다', () => {
    // 여기에는 편집할 값도 누를 버튼도 없다 — 고른 provider가 바뀌면 달라지는 것은
    // '지금 고른 것인가'와 그 문장뿐이다 (MH-8).
    const notChosen = localProviderSetups(NO_AI_PROVIDER)[0];
    const chosen = localProviderSetups('ollama')[0];
    const unknown = localProviderSetups('어떤-다른-provider')[0];

    expect(unknown.chosen).toBe(false);
    for (const other of [chosen, unknown]) {
      expect(other.id).toBe(notChosen.id);
      expect(other.title).toBe(notChosen.title);
      expect(other.standing).toBe(notChosen.standing);
      expect(other.text).toBe(notChosen.text);
      expect(other.howToTurnOn).toBe(notChosen.howToTurnOn);
    }
  });

  it('구역 전체가 선택이라는 사실이 먼저 오고, provider 없이도 쓸 길이 있다고 말한다', () => {
    // 구역의 이름은 특정 provider의 이름이 아니다.
    expect(AI_SECTION_TITLE).not.toMatch(/ollama/i);
    expect(AI_SECTION_TITLE).not.toMatch(OTHER_VENDORS);

    expect(AI_IS_OPTIONAL_TEXT).toMatch(/optional/i);
    // 첫 부분은 '연결된 provider'이고, 고르지 않은 것이 고칠 일이 아니라고 적혀 있다.
    expect(CONNECTED_PROVIDER_TITLE).toMatch(/connected provider/i);
    expect(CONNECTED_PROVIDER_TEXT).toMatch(/normal state/i);
    // provider가 없어도 AI를 쓸 길이 있다는 사실. 벤더를 부르지 않는다 (MH-1 · MH-2 · MH-6).
    expect(AI_WITHOUT_A_PROVIDER_TEXT).toMatch(/ai chat you already use/i);
    expect(AI_WITHOUT_A_PROVIDER_TEXT).not.toMatch(OTHER_VENDORS);
    expect(AI_WITHOUT_A_PROVIDER_TEXT).not.toMatch(/ollama/i);
  });
});

describe('전송 경계 (§12 · INV-5 · INV-6)', () => {
  it('로컬과 외부의 문구가 서로 다르다', () => {
    const local = aiTransferNotice('local');
    const external = aiTransferNotice('external');

    expect(local).not.toBeNull();
    expect(external).not.toBeNull();
    expect(local?.headline).not.toBe(external?.headline);
    expect(local?.transcriptText).not.toBe(external?.transcriptText);
    expect(local?.headline).toContain('on this device');
    expect(external?.headline).toContain('outside this device');
  });

  it('오디오가 전송되지 않는다는 사실이 양쪽 모두에 있다', () => {
    // 계약에 오디오를 가리킬 필드가 없다 (ADR-0008 §4.2). locality에 따라 달라지지 않는다.
    expect(aiTransferNotice('local')?.audioText).toBe(AUDIO_IS_NEVER_SENT);
    expect(aiTransferNotice('external')?.audioText).toBe(AUDIO_IS_NEVER_SENT);
    expect(AUDIO_IS_NEVER_SENT).toMatch(/audio is never sent/i);
  });

  it('고르지 않았거나 모르는 provider에 대해 "나가지 않는다"고 말하지 않는다', () => {
    expect(aiTransferNotice(aiProviderLocality(NO_AI_PROVIDER))).toBeNull();
    expect(aiTransferNotice(aiProviderLocality('어떤-다른-provider'))).toBeNull();
  });
});

describe('연결 확인 (요구 8 · ADR-0008 §4.2)', () => {
  it('실행 중 · 모델 없음 · 미실행이 서로 다른 상태다', () => {
    const running = checkedAiProvider(
      status({ state: 'ready', providerName: 'Ollama', locality: 'local', models: ['llama3.1:8b'] }),
    );
    const noModels = checkedAiProvider(
      status({ state: 'noModels', providerName: 'Ollama', locality: 'local' }),
    );
    const notRunning = checkedAiProvider(
      status({ state: 'unavailable', providerName: 'Ollama', locality: 'local', failure: unreachable }),
    );

    // 셋의 kind가 다르다 — 한 덩어리로 뭉쳐 있지 않다.
    expect([running.kind, noModels.kind, notRunning.kind]).toEqual([
      'running',
      'noModels',
      'notRunning',
    ]);
    // 화면에 나가는 문장도 셋 다 다르다.
    expect(new Set([running.text, noModels.text, notRunning.text]).size).toBe(3);
    expect(running.text).toBe(AI_PROVIDER_RUNNING_TEXT);
    expect(noModels.text).toBe(AI_PROVIDER_HAS_NO_MODELS_TEXT);
    expect(notRunning.text).toBe(AI_PROVIDER_NOT_RUNNING_TEXT);
  });

  it('모델이 없는 것은 실패가 아니다', () => {
    const noModels = checkedAiProvider(status({ state: 'noModels', providerName: 'Ollama' }));

    expect(noModels.kind).toBe('noModels');
    if (noModels.kind !== 'noModels') return;
    // 이 값에는 실패를 그릴 재료가 아예 없다.
    expect(noModels).not.toHaveProperty('failure');
    expect(noModels.text).not.toMatch(/error|failed|problem/i);
    expect(noModels.resolution).toMatch(/install a model/i);
  });

  it('닿지 못한 것은 이유와 함께 오고, 무엇을 하면 되는지가 붙는다', () => {
    const notRunning = checkedAiProvider(
      status({ state: 'unavailable', providerName: 'Ollama', locality: 'local', failure: unreachable }),
    );

    expect(notRunning.kind).toBe('notRunning');
    if (notRunning.kind !== 'notRunning') return;
    expect(notRunning.failure).toEqual(unreachable);
    // §13 — 재촉이 아니라 안내다.
    expect(notRunning.resolution).toMatch(/start the provider/i);
    expect(notRunning.resolution).not.toMatch(/must|warning|immediately/i);
  });

  it('고른 provider가 없으면 물어볼 대상이 없다고 말한다', () => {
    const none = checkedAiProvider(status({ state: 'notConfigured' }));

    expect(none.kind).toBe('notConfigured');
    expect(none.text).toBe(NO_AI_PROVIDER_TEXT);
    // 오류가 아니다 (INV-8).
    expect(none).not.toHaveProperty('failure');
  });

  it('확인 요청이 거절된 것과 provider가 응답하지 않는 것이 다른 상태다', () => {
    const failed = failedAiCheck(storageFailure);
    const notRunning = checkedAiProvider(status({ state: 'unavailable', failure: unreachable }));

    expect(failed.kind).toBe('checkFailed');
    expect(failed.kind === 'checkFailed' && failed.failure).toEqual(storageFailure);
    expect(failed.text).toBe(AI_CHECK_FAILED_TEXT);
    expect(failed.kind).not.toBe(notRunning.kind);
    // 저장소 실패를 "서버를 켜세요"로 바꾸지 않는다.
    expect(failed).not.toHaveProperty('resolution');
  });

  it('아직 물어보지 않은 것과 응답이 없는 것이 다른 상태다', () => {
    expect(AI_NOT_CHECKED_TEXT).not.toBe(AI_PROVIDER_NOT_RUNNING_TEXT);
  });

  it('provider가 스스로 말한 이름과 locality가 그대로 실려 온다 (INV-5 · INV-9)', () => {
    const running = checkedAiProvider(
      status({ state: 'ready', providerName: 'Ollama (로컬)', locality: 'local', models: ['m'] }),
    );

    expect(running.kind === 'running' && running.providerName).toBe('Ollama (로컬)');
    expect(running.kind === 'running' && running.locality).toBe('local');
  });
});

describe('확인된 목록에서 모델을 고른다', () => {
  const running = (models: readonly string[]): AiConnection =>
    checkedAiProvider(status({ state: 'ready', providerName: 'Ollama', locality: 'local', models }));

  it('목록은 확인이 돌려준 것이다', () => {
    expect(confirmedAiModels(running(['a', 'b']))).toEqual(['a', 'b']);
    // 확인이 그 답을 주지 않은 상태에서는 빈 목록이다 — "모델이 없다"가 아니다.
    expect(confirmedAiModels(checkedAiProvider(status({ state: 'noModels' })))).toEqual([]);
    expect(confirmedAiModels(failedAiCheck(storageFailure))).toEqual([]);
  });

  it('고르지 않음이 첫 항목이고 확인된 모델이 그 뒤에 온다', () => {
    const options = aiModelOptions(NO_AI_MODEL, ['llama3.1:8b', 'qwen2.5:7b']);

    expect(options.map((option) => option.value)).toEqual([
      NO_AI_MODEL,
      'llama3.1:8b',
      'qwen2.5:7b',
    ]);
    expect(options.every((option) => option.installed)).toBe(true);
  });

  it('저장된 모델이 지금 없어도 선택은 그대로 남는다', () => {
    const options = aiModelOptions('없어진-모델', ['llama3.1:8b']);
    const saved = options.find((option) => option.value === '없어진-모델');

    expect(saved?.installed).toBe(false);
    expect(saved?.label).toBe(`없어진-모델${MISSING_AI_MODEL_SUFFIX}`);
    // 목록의 첫 모델로 바꿔 고르는 경로가 없다.
    expect(options.filter((option) => option.value === 'llama3.1:8b')).toHaveLength(1);
  });

  it('모델에 대한 말은 확인된 목록이 있을 때만 한다', () => {
    // 물어보지 않은 상태에서 "그 모델은 없다"고 말하지 않는다.
    expect(aiModelNotice('없어진-모델', failedAiCheck(storageFailure))).toBeNull();
    expect(aiModelNotice('없어진-모델', checkedAiProvider(status({ state: 'noModels' })))).toBeNull();

    expect(aiModelNotice(NO_AI_MODEL, running(['a']))).toBe(NO_AI_MODEL_CHOSEN_NOTICE);
    expect(aiModelNotice('없어진-모델', running(['a']))).toBe(MISSING_AI_MODEL_NOTICE);
    expect(aiModelNotice('a', running(['a']))).toBeNull();
  });
});

describe('확인은 저장된 설정에게 물어본다', () => {
  it('저장한 뒤 AI 값을 고치면 그 사실을 말할 수 있다', () => {
    const saved = aiSettingsSnapshot(form({ aiProvider: 'ollama' }));

    expect(aiSettingsChanged(form({ aiProvider: 'ollama' }), saved)).toBe(false);
    // 앞뒤 공백은 저장할 때와 같은 규칙으로 무시된다.
    expect(aiSettingsChanged(form({ aiProvider: ' ollama ' }), saved)).toBe(false);
    expect(aiSettingsChanged(form({ aiProvider: 'ollama', aiModel: 'a' }), saved)).toBe(true);
    expect(aiSettingsChanged(form({ aiProvider: 'ollama', aiBaseUrl: 'x' }), saved)).toBe(true);
  });

  it('무엇이 저장돼 있는지 모르면 바뀌었다고 말하지 않는다', () => {
    expect(aiSettingsChanged(form({ aiProvider: 'ollama' }), null)).toBe(false);
  });

  it('AI 설정을 고치지 않은 다른 편집은 확인 결과를 낡게 만들지 않는다', () => {
    const saved = aiSettingsSnapshot(form({ aiProvider: 'ollama' }));

    expect(aiSettingsChanged(form({ aiProvider: 'ollama', recordingsDirectory: '/tmp' }), saved)).toBe(
      false,
    );
  });
});

describe('AI가 안 되는 것이 나머지를 막지 않는다 (INV-8)', () => {
  it('provider 확인이 실패한 상태에서도 다른 설정의 저장 경로가 그대로 동작한다', () => {
    // 확인이 거절된 사실은 여기 있다 —
    const failed = failedAiCheck(storageFailure);
    expect(failed.kind).toBe('checkFailed');

    // — 그리고 그 사실은 저장 경로의 입력에 없다.
    const editing = editedSettings(loadedSettings(DEFAULT_SETTINGS), {
      recordingsDirectory: '/tmp/notes',
      automaticTranscription: true,
    });
    expect(editing.kind).toBe('ready');
    if (editing.kind !== 'ready') return;

    const sending = savingSettings(editing);
    expect(sending.kind === 'ready' && sending.saving).toBe(true);

    const outgoing = toSettings(editing.form);
    expect(outgoing.recordingsDirectory).toBe('/tmp/notes');
    expect(outgoing.automaticTranscription).toBe(true);

    const done = savedSettings(outgoing);
    expect(done.kind).toBe('ready');
    expect(done.kind === 'ready' && done.saved).toBe(true);
    expect(done.kind === 'ready' && done.failure).toBeNull();
  });

  it('provider를 고르지 않은 상태에서도 다른 설정이 그대로 저장된다', () => {
    const none = checkedAiProvider(status({ state: 'notConfigured' }));
    expect(none.kind).toBe('notConfigured');

    const edited = editedSettings(loadedSettings(DEFAULT_SETTINGS), { automaticProcessing: true });
    expect(edited.kind).toBe('ready');
    if (edited.kind !== 'ready') return;

    const outgoing = toSettings(edited.form);
    expect(outgoing.automaticProcessing).toBe(true);
    // AI를 고르지 않았다는 이유로 다른 값이 비틀리지 않는다.
    expect(outgoing.aiProvider).toBeNull();
  });

  it('응답하지 않는 provider를 골라 둔 채로도 저장은 나간다', () => {
    const notRunning = checkedAiProvider(status({ state: 'unavailable', failure: unreachable }));
    expect(notRunning.kind).toBe('notRunning');

    const edited = editedSettings(loadedSettings(DEFAULT_SETTINGS), {
      aiProvider: 'ollama',
      aiBaseUrl: 'http://127.0.0.1:9999',
      aiModel: '없어진-모델',
      recordingsDirectory: '/tmp/notes',
    });
    expect(edited.kind).toBe('ready');
    if (edited.kind !== 'ready') return;

    // 응답하지 않는다고 해서 고른 값이 지워지거나 다른 값으로 바뀌지 않는다.
    expect(toSettings(edited.form)).toEqual({
      ...DEFAULT_SETTINGS,
      recordingsDirectory: '/tmp/notes',
      aiProvider: 'ollama',
      aiBaseUrl: 'http://127.0.0.1:9999',
      aiModel: '없어진-모델',
    });
  });

  it('그 사실이 화면에 적을 수 있는 문장으로 있다', () => {
    expect(AI_SETTINGS_UNAFFECTED_NOTICE).toMatch(/every other setting/i);
  });
});

describe('연결 대상 주소', () => {
  it('빈 입력은 고르지 않음이며 backend의 기본 주소를 쓴다', () => {
    expect(toSettings(form({ aiBaseUrl: '   ' })).aiBaseUrl).toBeNull();
    expect(toSettings(form({ aiBaseUrl: ' http://127.0.0.1:9999 ' })).aiBaseUrl).toBe(
      'http://127.0.0.1:9999',
    );
  });

  it('기본 주소가 화면 쪽에 옮겨 적혀 있지 않다', () => {
    // 그 값을 아는 자리는 src-tauri/src/domain/settings.rs 하나다. 두 곳에 있으면 한 곳을
    // 고쳤을 때 화면이 조용히 거짓말을 한다.
    expect(AI_BASE_URL_NOTICE).not.toMatch(/https?:\/\//);
    expect(AI_BASE_URL_NOTICE).not.toMatch(/\d{4,5}/);
  });
});

describe('AI provider 자격증명 (2026-09-08 · PRODUCT-SPEC §16.1)', () => {
  it('저장 여부만 알 수 있다 — 값을 읽는 경로가 없다', () => {
    expect(aiCredentialState({ stored: true })).toBe('stored');
    expect(aiCredentialState({ stored: false })).toBe('notStored');
  });

  it('키가 없는 것은 오류가 아니라 상태다', () => {
    // provider를 하나도 쓰지 않는 것이 정상이다 (INV-8). 문장이 실패처럼 들리면 안 된다.
    const notice = aiCredentialNotice('notStored');

    expect(notice.text).not.toMatch(/error|failed|invalid/i);
    // 그러나 무엇을 하면 되는지는 말해 준다.
    expect(notice.resolution).not.toBeNull();
  });

  it('저장돼 있으면 더 할 일을 말하지 않는다', () => {
    expect(aiCredentialNotice('stored').resolution).toBeNull();
  });

  it('입력란 안내가 값이 어디 저장되고 어떻게 사라지는지 말한다', () => {
    // 넘긴 뒤 입력란이 비워지므로, 그 사실을 먼저 알리지 않으면 사라진 것처럼 보인다.
    expect(AI_KEY_INPUT_NOTICE).toContain('credential store');
    expect(AI_KEY_INPUT_NOTICE).toContain('cleared');
    // 앱 DB에 쓰이지 않는다는 사실 (INV-7).
    expect(AI_KEY_INPUT_NOTICE).toContain('never written to the app database');
  });

  /**
   * 자격증명 입력란을 **기기 밖으로 나가는 provider에서만** 보인다.
   *
   * "기기를 떠난다"는 경고 자체는 `aiTransferNotice`가 이미 하고 있으므로 여기서 다시
   * 만들지 않았다 — 두 문구가 언젠가 어긋나는 것을 막는다.
   */
  it('키 입력란은 기기 밖으로 나가는 provider에서만 필요하다', () => {
    expect(needsApiKey('anthropic')).toBe(true);
    expect(needsApiKey('ollama')).toBe(false);
    // 고르지 않은 것도 요구하지 않는다.
    expect(needsApiKey(NO_AI_PROVIDER)).toBe(false);
  });

  it('전송 경계 문구는 이 구역이 새로 만들지 않고 기존 규칙이 낸다', () => {
    // 같은 것을 두 번 정의하지 않는다. 외부 provider의 문구는 여기가 아니라
    // `aiTransferNotice`에서 나오며, 그것이 전사와 오디오를 구분해 말한다 (INV-5 · INV-6).
    const notice = aiTransferNotice('external');

    expect(notice).not.toBeNull();
    expect(notice?.transcriptText.length).toBeGreaterThan(0);
    expect(notice?.audioText.length).toBeGreaterThan(0);
  });
});
