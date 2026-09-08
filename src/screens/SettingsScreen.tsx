import { useEffect, useRef, useState } from 'react';
import {
  aiProviderStatus,
  checkNotionConnection,
  deleteNotionToken,
  getSettings,
  listInputDevices,
  aiApiKeyStatus,
  saveAiApiKey,
  deleteAiApiKey,
  saveNotionToken,
  updateSettings,
} from '../ipc/commands';
import { toFailure, type Failure } from '../ipc/failure';
import type { InputDevice } from '../ipc/types';
import {
  AI_BASE_URL_NOTICE,
  AI_BASE_URL_PLACEHOLDER,
  AI_CHECK_USES_SAVED_SETTINGS,
  AI_IS_OPTIONAL_TEXT,
  AI_NOT_CHECKED_TEXT,
  AI_SECTION_TITLE,
  AI_SETTINGS_UNAFFECTED_NOTICE,
  AI_WITHOUT_A_PROVIDER_TEXT,
  CHECKING_AI_PROVIDER_TEXT,
  CONNECTED_PROVIDER_TEXT,
  CONNECTED_PROVIDER_TITLE,
  NOTHING_LEAVES_THIS_DEVICE,
  aiModelNotice,
  aiModelOptions,
  aiCredentialNotice,
  aiCredentialState,
  type AiCredentialState,
  aiProviderChoices,
  needsApiKey,
  AI_KEY_INPUT_NOTICE,
  AI_KEY_INPUT_PLACEHOLDER,
  aiProviderLocality,
  aiSettingsChanged,
  aiSettingsSnapshot,
  aiTransferNotice,
  checkedAiProvider,
  confirmedAiModels,
  failedAiCheck,
  localProviderSetups,
  type AiConnection,
  type AiSettingsSnapshot,
} from './aiProviderSettings';
import {
  chosenMicrophone,
  defaultMicrophoneNotice,
  microphoneOptions,
  resolveDefaultMicrophone,
} from './defaultMicrophone';
import { EmptyState } from './EmptyState';
import { FailureNotice } from './FailureNotice';
import { Loading } from './Loading';
import {
  CHECKING_NOTION_TEXT,
  HOW_TO_SET_A_DESTINATION,
  NOTION_CHECK_USES_SAVED_SETTINGS,
  NOTION_NOT_CHECKED_TEXT,
  NOTION_SETTINGS_UNAFFECTED_NOTICE,
  TOKEN_INPUT_NOTICE,
  TOKEN_INPUT_PLACEHOLDER,
  checkedNotionConnection,
  failedNotionCheck,
  notionDestinationChanged,
  notionDestinationNotice,
  notionTokenNotice,
  notionTokenState,
  notionTokenTrouble,
  tokenStateOf,
  type NotionConnectionView,
  type NotionTokenState,
  type NotionTokenTrouble,
} from './notionSettings';
import {
  LOADING_SETTINGS,
  TRANSCRIPTION_LANGUAGE_PLACEHOLDER,
  editedSettings,
  failedSave,
  failedSettings,
  loadedSettings,
  savedSettings,
  savingSettings,
  toForm,
  toSettings,
  transcriptionLanguageNotices,
  transcriptionNotices,
  type SettingsForm,
  type SettingsView,
} from './settingsView';

/**
 * Settings 화면 (§5.D).
 *
 * Recording 그룹의 세 값은 저장소에 영속화된다 — `get_settings`로 읽고 `update_settings`로
 * 저장한 뒤, **저장소가 돌려준 값으로** 폼을 다시 채운다. 무엇이 저장됐는지 화면이
 * 추측하지 않는다.
 *
 * default microphone은 **열거된 장치 중에서** 고른다. 목록은 설정과 따로 읽는다 — 장치를
 * 읽지 못해도 나머지 설정은 계속 편집할 수 있어야 하기 때문이다. 저장된 장치가 지금 목록에
 * 없을 수 있고, 그때 **다른 장치로 바꿔 놓지 않는다** — 저장된 선택은 그대로 두고 지금
 * 쓸 수 없다는 사실을 보여 준다 (`defaultMicrophone.ts`).
 *
 * Transcription 그룹은 **자동 전사 토글**과 **모델 선택**과 **언어**다 (Phase 3 · Phase 5.6).
 * 모델이 없어서 지금 전사할 수 없다는 것은 여기서 보이는 **제품 상태**이며, 그 사실 때문에
 * 자동 전사 토글이 뒤집히지 않는다 — 사용자가 켠 값은 켜진 채로 남는다 (ADR-0007 §8.2.3).
 *
 * 언어는 §5 D가 처음부터 요구하던 항목이며 여기가 그 자리다 (ADR-0007 §17.1.5). **비어 있는
 * 것이 결핍이 아니라 자동 감지**이므로 그 사실이 화면의 말로 나온다 — 무엇을 쓸지는 화면이
 * 아니라 순수 모듈이 정하고, 여기서는 그리기만 한다 (§18).
 *
 * AI 그룹은 **두 부분**이다 (Phase 5.5 요구 7 · `docs/ADR-0010` §4.3). 앞은 `Connected
 * provider` — **고르기 · 연결 확인 · 모델 선택 · 전송 경계 표시** 넷이며 Phase 4가 만든 그대로다.
 * 뒤는 스스로 설치해서 쓰는 로컬 provider에 대한 안내이고, 거기에는 편집할 값도 버튼도 없다.
 * **재배치이지 삭제가 아니다** — provider 추상화도 adapter도 이 화면의 어느 경로도 줄지
 * 않았다 (MH-8). 뒤로 가는 이유는 그것이 기본 AI 경험이 아니라 선택지 하나이기 때문이며,
 * 그 부분의 이름과 로컬 표시는 provider 목록에서 온다 — 화면이 벤더를 새로 적지 않는다 (INV-9).
 *
 * 연결 확인은 사용자가 눌러야 나가며, 화면을 열자마자 서버를 찾아 나서지 않는다. 확인이
 * 어떻게 끝나든 — 응답이 없든 모델이 없든 확인 자체가 거절되든 — **그 결과는 나머지 설정의
 * 저장 경로에 닿지 않는다** (INV-8). AI 상태를 `SettingsView`가 아니라 별도의 state로 들고
 * 있는 이유가 이것이며, 장치 목록 실패를 설정 읽기 실패와 섞지 않는 것과 같은 규칙이다.
 *
 * Notion 그룹은 **token 저장과 삭제 · destination · 연결 확인** 셋이다 (Phase 5 · §5-D).
 * AI 구역과 나란히 있고 같은 규칙을 따른다 — 확인은 사용자가 눌러야 나가고, 그 결과가 어떻든
 * 나머지 설정의 저장 경로에 닿지 않는다 (INV-8).
 *
 * **token만은 다른 규칙으로 다룬다** (INV-7 · ADR-0009 §10.4). 입력란은 `value`를 갖지 않는
 * uncontrolled 입력이며, 그래서 그 값은 **React 상태에도 이 컴포넌트에도 남지 않는다.** 저장
 * command에 한 번 넘긴 직후 입력란을 비우고, 저장된 값을 되읽어 채우지 않는다 — 되읽을
 * command 자체가 없다. 화면이 아는 것은 '저장돼 있다/없다'뿐이며 ({@link NotionTokenState}),
 * 그 사실조차 자기가 누른 버튼이 아니라 자격증명 저장소가 답한 값에서 온다.
 *
 * **Save는 화면 전체에 하나다.** 설정은 한 벌이고 한 번에 저장되므로, 그룹마다 버튼을 두어
 * "이 그룹만 저장된다"처럼 보이게 하지 않는다.
 *
 * 응답을 화면 상태로 바꾸는 규칙은 `settingsView`에 있다. 여기에는 그리는 일만 있다 (§18).
 */
export function SettingsScreen() {
  const [view, setView] = useState<SettingsView>(LOADING_SETTINGS);
  /** 다시 시도 횟수. 늘어나면 설정을 다시 읽는다. */
  const [attempt, setAttempt] = useState(0);
  /** 지금 열거된 입력 장치. 비어 있는 것은 정상 상태다 (마이크가 없거나 빠져 있다). */
  const [devices, setDevices] = useState<InputDevice[]>([]);
  /** 목록 자체를 얻지 못했다면 그 실패. 설정 읽기 실패와 섞지 않는다. */
  const [deviceFailure, setDeviceFailure] = useState<Failure | null>(null);
  const [deviceAttempt, setDeviceAttempt] = useState(0);
  /**
   * 연결 확인이 지금까지 말해 준 것.
   *
   * **`view`와 따로 있다.** 이 값이 무엇이 되든 저장 경로는 그것을 보지 않으며, 그래서
   * provider가 응답하지 않아도 다른 설정은 그대로 저장된다 (INV-8).
   */
  const [connection, setConnection] = useState<AiConnection>({
    kind: 'notChecked',
    text: AI_NOT_CHECKED_TEXT,
  });
  /** 확인이 실제로 물어본 대상 — 마지막으로 저장소에서 온 AI 세 값. */
  const [savedAi, setSavedAi] = useState<AiSettingsSnapshot | null>(null);
  /**
   * Notion 연결 확인이 지금까지 말해 준 것. **`view`와 따로 있다** — AI 쪽과 같은 이유이며,
   * 이 값이 무엇이 되든 다른 설정은 그대로 저장된다 (INV-8).
   */
  const [notion, setNotion] = useState<NotionConnectionView>({
    kind: 'notChecked',
    text: NOTION_NOT_CHECKED_TEXT,
  });
  /**
   * integration token이 저장돼 있는가. **값이 아니라 사실이다** (INV-7).
   *
   * 화면을 열자마자 자격증명 저장소를 뒤지지 않으므로 처음에는 `unknown`이다 — 없다고 적으면
   * 알지 못하는 것을 아는 것처럼 말하는 것이 된다.
   */
  const [tokenState, setTokenState] = useState<NotionTokenState>('unknown');
  /** 지금 진행 중인 token 작업. 없으면 `null`이다. */
  const [tokenBusy, setTokenBusy] = useState<'save' | 'delete' | null>(null);
  /** 저장·삭제가 거절됐다면 그 실패 (§13). 확인 결과와 섞지 않는다. */
  const [tokenTrouble, setTokenTrouble] = useState<NotionTokenTrouble | null>(null);
  /** 확인이 실제로 물어본 destination — 마지막으로 저장소에서 온 부모 페이지 값. */
  const [savedDestination, setSavedDestination] = useState<string | null>(null);
  /**
   * token 입력란 그 자체.
   *
   * **state가 아니라 ref인 것이 이 화면의 INV-7이다.** 입력한 값은 React 상태에 들어가지
   * 않고, 저장 command로 한 번 지나간 뒤 곧바로 지워진다 — 그래서 이 컴포넌트가 다시 그려질
   * 때 값이 어디에도 남아 있지 않는다.
   */
  const tokenInput = useRef<HTMLInputElement>(null);

  /** 저장된 AI API 키의 유무. **값이 아니라 유무다** (INV-7). */
  const [aiKeyState, setAiKeyState] = useState<AiCredentialState>('notStored');
  const [aiKeyBusy, setAiKeyBusy] = useState<'save' | 'delete' | null>(null);
  const [aiKeyTrouble, setAiKeyTrouble] = useState<Failure | null>(null);
  const aiKeyInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let current = true;
    // 실패해도 화면을 막지 않는다 — 키가 없는 것과 물어보지 못한 것 모두 "없음"으로 두고,
    // 저장을 눌렀을 때 진짜 실패가 §13으로 드러난다.
    aiApiKeyStatus().then(
      (status) => {
        if (current) setAiKeyState(aiCredentialState(status));
      },
      () => {},
    );
    return () => {
      current = false;
    };
  }, []);


  /** 다시 읽는다. 상태를 되돌리는 것은 effect가 아니라 이 사용자 동작의 일이다. */
  const retryLoad = () => {
    setView(LOADING_SETTINGS);
    setAttempt((count) => count + 1);
  };

  /** 장치 목록만 다시 읽는다. 장치는 언제든 꽂히고 빠지므로 매번 새로 물어본다. */
  const retryDevices = () => {
    setDeviceAttempt((count) => count + 1);
  };

  useEffect(() => {
    // 응답이 오기 전에 화면을 떠났다면 그 응답으로 상태를 바꾸지 않는다.
    let current = true;

    getSettings().then(
      (settings) => {
        if (!current) return;
        setView(loadedSettings(settings));
        setSavedAi(aiSettingsSnapshot(toForm(settings)));
        // Notion 확인도 **저장된** destination에게 물어본다. 그 대상이 무엇인지 여기서 안다.
        setSavedDestination(toForm(settings).notionParentPageId);
      },
      (error: unknown) => {
        // 실패를 console에만 남기지 않는다. 화면 상태가 된다 (§13).
        if (current) setView(failedSettings(error));
      },
    );

    return () => {
      current = false;
    };
  }, [attempt]);

  useEffect(() => {
    let current = true;

    listInputDevices().then(
      (listed) => {
        if (!current) return;
        setDevices(listed);
        setDeviceFailure(null);
      },
      (error: unknown) => {
        // 장치를 읽지 못한 것은 **설정을 읽지 못한 것과 다르다.** 나머지 설정은 계속
        // 편집할 수 있고, 저장된 선택도 그대로 남는다.
        if (!current) return;
        setDevices([]);
        setDeviceFailure(toFailure(error));
      },
    );

    return () => {
      current = false;
    };
  }, [deviceAttempt]);

  if (view.kind === 'loading') {
    return (
      <div className="screen">
        <Loading text="Loading settings…" />
      </div>
    );
  }

  if (view.kind === 'failed') {
    return (
      <div className="screen">
        <FailureNotice failure={view.failure} onRetry={retryLoad} />
      </div>
    );
  }

  const edit = (change: Partial<SettingsForm>) => {
    setView((state) => editedSettings(state, change));
  };

  const save = (form: SettingsForm) => {
    setView((state) => savingSettings(state));
    updateSettings(toSettings(form)).then(
      (settings) => {
        setView(savedSettings(settings));
        // 확인이 물어보는 대상은 저장된 값이다. 방금 저장한 것이 그 대상이 됐다.
        setSavedAi(aiSettingsSnapshot(toForm(settings)));
        setSavedDestination(toForm(settings).notionParentPageId);
      },
      (error: unknown) => setView((state) => failedSave(state, error)),
    );
  };

  /**
   * 지금 그 provider가 실제로 응답하는지, 어떤 모델이 설치돼 있는지 물어본다 (요구 8).
   *
   * **저장된 설정에게 물어본다** — `ai_provider_status`는 저장소의 값을 읽는다. 거절되면
   * 그것도 화면 상태가 되며, 어느 쪽이든 `view`는 건드리지 않는다 (INV-8).
   */
  const checkProvider = () => {
    setConnection({ kind: 'checking', text: CHECKING_AI_PROVIDER_TEXT });
    aiProviderStatus().then(
      (status) => setConnection(checkedAiProvider(status)),
      (error: unknown) => setConnection(failedAiCheck(error)),
    );
  };

  /**
   * 입력한 token을 저장 command로 **넘기고 곧바로 입력란을 비운다** (INV-7 · ADR-0009 §10.4).
   *
   * 응답을 기다렸다가 비우지 않는다 — 값이 화면에 머무는 시간이 길어질 이유가 없고, 실패했을
   * 때 그 값을 다시 쓰기 위해 붙들고 있으면 그것이 곧 화면에 남은 secret이 된다. 실패하면
   * 무엇이 실패했는지가 §13으로 보이고, 사용자는 다시 붙여 넣는다.
   *
   * **빈 값을 여기서 걸러 내지 않는다.** 무엇이 저장될 수 있는 값인지는 backend가 정하며
   * (`save_notion_token`), 그 답도 §13의 실패로 온다 — 같은 규칙이 두 벌이 되지 않게 한다.
   */
  /**
   * API 키를 넘긴다. **넘긴 뒤 입력란을 비운다** — 값이 화면 상태에 남지 않는다 (INV-7).
   *
   * `saveToken`이 세운 규칙 그대로다: 빈 값을 여기서 걸러 내지 않는다. 무엇이 저장될 수
   * 있는 값인지는 backend가 정하며 그 답도 §13의 실패로 온다.
   */
  const saveAiKey = () => {
    const input = aiKeyInput.current;
    if (input === null) return;

    const typed = input.value;
    input.value = '';

    setAiKeyBusy('save');
    setAiKeyTrouble(null);
    saveAiApiKey(typed).then(
      (status) => {
        setAiKeyBusy(null);
        setAiKeyState(aiCredentialState(status));
      },
      (error: unknown) => {
        setAiKeyBusy(null);
        setAiKeyTrouble(toFailure(error));
      },
    );
  };

  /** 저장된 키를 지운다. **없던 것을 지우는 것도 실패가 아니다** (INV-3 · INV-8). */
  const removeAiKey = () => {
    setAiKeyBusy('delete');
    setAiKeyTrouble(null);
    deleteAiApiKey().then(
      (status) => {
        setAiKeyBusy(null);
        setAiKeyState(aiCredentialState(status));
      },
      (error: unknown) => {
        setAiKeyBusy(null);
        setAiKeyTrouble(toFailure(error));
      },
    );
  };

  const saveToken = () => {
    const input = tokenInput.current;
    if (input === null) return;

    const typed = input.value;
    input.value = '';

    setTokenBusy('save');
    setTokenTrouble(null);
    saveNotionToken(typed).then(
      (status) => {
        setTokenBusy(null);
        setTokenState(notionTokenState(status));
        // 확인된 것은 방금 저장한 token이 아니라 그 전 것이다. 결과를 그대로 두면 화면이
        // 확인하지 않은 것을 확인한 것처럼 말하게 된다.
        setNotion({ kind: 'notChecked', text: NOTION_NOT_CHECKED_TEXT });
      },
      (error: unknown) => {
        setTokenBusy(null);
        setTokenTrouble(notionTokenTrouble('save', error));
      },
    );
  };

  /** 저장된 token을 지운다. **없던 것을 지우는 것도 실패가 아니다** (INV-3 · INV-8). */
  const removeToken = () => {
    setTokenBusy('delete');
    setTokenTrouble(null);
    deleteNotionToken().then(
      (status) => {
        setTokenBusy(null);
        setTokenState(notionTokenState(status));
        setNotion({ kind: 'notChecked', text: NOTION_NOT_CHECKED_TEXT });
      },
      (error: unknown) => {
        setTokenBusy(null);
        setTokenTrouble(notionTokenTrouble('delete', error));
      },
    );
  };

  /**
   * 저장된 token으로 지금 Notion과 말할 수 있는지 물어본다 (§5-D).
   *
   * **저장된 것에게 물어본다** — token은 자격증명 저장소에서, destination은 설정에서 온다.
   * 확인은 저장 여부에 대한 가장 최근의 사실도 함께 들고 오므로 그것으로 화면을 맞춘다.
   */
  const checkNotion = () => {
    setNotion({ kind: 'checking', text: CHECKING_NOTION_TEXT });
    checkNotionConnection().then(
      (connection) => {
        setNotion(checkedNotionConnection(connection));
        setTokenState(tokenStateOf(connection));
      },
      (error: unknown) => setNotion(failedNotionCheck(error)),
    );
  };

  const { form, saving, saved, failure } = view;
  // 저장된 값과 지금 있는 장치를 맞춰 본다. 판단은 순수 모듈이 하고 화면은 그리기만 한다.
  const chosen = chosenMicrophone(form.defaultMicrophone);
  const microphoneNotice = defaultMicrophoneNotice(resolveDefaultMicrophone(chosen, devices));

  // AI 쪽도 같다 — 로컬/외부도, 확인 결과의 갈래도, 모델 목록도 전부 순수 모듈이 정한다.
  const providerChosen = form.aiProvider !== '';
  const transfer = aiTransferNotice(aiProviderLocality(form.aiProvider));
  const modelNotice = aiModelNotice(form.aiModel, connection);
  const staleCheck = aiSettingsChanged(form, savedAi);

  // Notion 쪽도 같다 — 저장 여부의 문구도, destination에 대한 말도, 확인 결과의 갈래도 전부
  // 순수 모듈이 정한다 (`notionSettings.ts`).
  const tokenNotice = notionTokenNotice(tokenState);
  const destinationNotice = notionDestinationNotice(form.notionParentPageId);
  const staleDestination = notionDestinationChanged(form.notionParentPageId, savedDestination);

  return (
    <div className="screen">
      <section className="group">
        <h2 className="group__title">Recording</h2>

        <label className="field" htmlFor="recordings-directory">
          <span className="field__label">Recordings directory</span>
          <input
            id="recordings-directory"
            type="text"
            className="input"
            placeholder="Not set"
            value={form.recordingsDirectory}
            onChange={(event) => edit({ recordingsDirectory: event.currentTarget.value })}
          />
        </label>
        {form.recordingsDirectory === '' && <p className="hint">No recordings directory set yet.</p>}

        <label className="field" htmlFor="default-microphone">
          <span className="field__label">Default microphone</span>
          <select
            id="default-microphone"
            className="select"
            value={form.defaultMicrophone}
            onChange={(event) => edit({ defaultMicrophone: event.currentTarget.value })}
          >
            {/* 저장된 장치가 지금 없으면 그 항목이 목록에 함께 있다 — 그래서 고른 값이
                말없이 다른 장치로 보이지 않는다 (`microphoneOptions`). */}
            {microphoneOptions(chosen, devices).map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        {microphoneNotice !== null && <p className="hint">{microphoneNotice}</p>}
        {deviceFailure !== null && (
          <FailureNotice failure={deviceFailure} onRetry={retryDevices} />
        )}

        <label className="field field--inline" htmlFor="automatic-processing">
          <input
            id="automatic-processing"
            type="checkbox"
            className="toggle__input"
            checked={form.automaticProcessing}
            onChange={(event) => edit({ automaticProcessing: event.currentTarget.checked })}
          />
          <span className="field__label">Automatic processing after a recording ends</span>
        </label>
      </section>

      <section className="group">
        <h2 className="group__title">Transcription</h2>

        <label className="field" htmlFor="transcription-model">
          <span className="field__label">Whisper model</span>
          <input
            id="transcription-model"
            type="text"
            className="input"
            placeholder="Not set"
            value={form.transcriptionModel}
            onChange={(event) => edit({ transcriptionModel: event.currentTarget.value })}
          />
        </label>

        <label className="field" htmlFor="transcription-language">
          <span className="field__label">Language</span>
          {/* 모델과 **다른 값이다** — 어떤 모델로 듣는가와 무슨 언어로 듣는가는 서로 다른
              질문이다 (ADR-0007 §17.1). 비어 있는 것이 자동 감지라는 사실은 placeholder와
              아래 문장이 말하며, 그 문구를 정하는 것은 순수 모듈이다. */}
          <input
            id="transcription-language"
            type="text"
            className="input"
            placeholder={TRANSCRIPTION_LANGUAGE_PLACEHOLDER}
            value={form.transcriptionLanguage}
            onChange={(event) => edit({ transcriptionLanguage: event.currentTarget.value })}
          />
        </label>

        {/* 고르지 않은 상태가 **오류가 아니라 자동 감지**라는 것이 여기서 말해진다. 빈칸을
            말없이 두면 "아직 안 한 일"로 읽히고, 그렇게 읽히는 순간 사용자는 무엇이든
            적어 넣게 된다 (ADR-0007 §17.1.4-1). */}
        {transcriptionLanguageNotices(form).map((notice) => (
          <p className="hint" key={notice}>
            {notice}
          </p>
        ))}

        <label className="field field--inline" htmlFor="automatic-transcription">
          <input
            id="automatic-transcription"
            type="checkbox"
            className="toggle__input"
            checked={form.automaticTranscription}
            onChange={(event) => edit({ automaticTranscription: event.currentTarget.checked })}
          />
          {/* 후처리 토글과 **다른 값이다.** 하나를 켜는 것이 다른 하나를 켜지 않는다. */}
          <span className="field__label">Transcribe automatically after a recording is saved</span>
        </label>

        {/* 모델이 없다는 사실과 그것을 푸는 방법이 여기 나온다. 토글 값은 건드리지 않는다 —
            보이는 것이 늘어날 뿐이며, 사용자가 켠 것은 켜진 채로 남는다 (ADR-0007 §8.2.3). */}
        {transcriptionNotices(form).map((notice) => (
          <p className="hint" key={notice}>
            {notice}
          </p>
        ))}
      </section>

      <section className="group">
        <h2 className="group__title">{AI_SECTION_TITLE}</h2>

        {/* 구역 전체가 선택이라는 사실이 먼저 온다 (INV-8). 그래서 아래의 어느 것도
            "아직 안 한 일"로 읽히지 않는다. */}
        <p className="hint">{AI_IS_OPTIONAL_TEXT}</p>
        <p className="hint">{AI_WITHOUT_A_PROVIDER_TEXT}</p>

        <section className="group__part">
          <h3 className="group__subtitle">{CONNECTED_PROVIDER_TITLE}</h3>
          <p className="hint">{CONNECTED_PROVIDER_TEXT}</p>

          <label className="field" htmlFor="ai-provider">
            <span className="field__label">Provider</span>
            <select
              id="ai-provider"
              className="select"
              value={form.aiProvider}
              onChange={(event) => edit({ aiProvider: event.currentTarget.value })}
            >
              {/* 저장된 값을 이 앱이 모르면 그 항목이 목록에 함께 있다 — 그래서 고른 값이
                  말없이 다른 provider로 보이지 않는다 (`aiProviderChoices`). */}
              {aiProviderChoices(form.aiProvider).map((choice) => (
                <option key={choice.value} value={choice.value}>
                  {choice.label}
                </option>
              ))}
            </select>
          </label>

          {/* 전송 경계 (§12 · INV-5 · INV-6). 문구는 provider의 locality 값에서 나온다. */}
          {transfer === null ? (
            <p className="hint">{NOTHING_LEAVES_THIS_DEVICE}</p>
          ) : (
            <>
              <p className="hint">{transfer.headline}</p>
              <p className="hint">{transfer.transcriptText}</p>
              <p className="hint">{transfer.audioText}</p>
            </>
          )}

          {providerChosen && (
            <>
              {/* **기기 밖에서 도는 provider만 자격증명을 요구한다** (`needsApiKey`).
                  "기기를 떠난다"는 사실은 위 `transfer` 문단이 이미 말했으므로 여기서
                  다시 말하지 않는다 — 같은 것을 두 번 적으면 언젠가 어긋난다. */}
              {needsApiKey(form.aiProvider) && (
                <>
                  <p className="hint">{aiCredentialNotice(aiKeyState).text}</p>
                  {aiCredentialNotice(aiKeyState).resolution !== null && (
                    <p className="hint">{aiCredentialNotice(aiKeyState).resolution}</p>
                  )}

                  <label className="field" htmlFor="ai-api-key">
                    <span className="field__label">API key</span>
                    {/* **`value`가 없다.** 입력한 값은 React 상태에 들어가지 않고, 저장된
                        값이 여기 채워지는 일도 없다 — 되읽는 command 자체가 없다 (INV-7). */}
                    <input
                      id="ai-api-key"
                      type="password"
                      className="input"
                      placeholder={AI_KEY_INPUT_PLACEHOLDER}
                      autoComplete="off"
                      ref={aiKeyInput}
                    />
                  </label>
                  <p className="hint">{AI_KEY_INPUT_NOTICE}</p>

                  <button
                    type="button"
                    className="btn btn--secondary"
                    disabled={aiKeyBusy !== null}
                    onClick={saveAiKey}
                  >
                    {aiKeyBusy === 'save' ? 'Saving the key…' : 'Save the key'}
                  </button>
                  {/* 저장된 것을 없애는 동작이므로 다른 버튼과 같은 무게로 두지 않는다 (§19). */}
                  <button
                    type="button"
                    className="btn btn--danger"
                    disabled={aiKeyBusy !== null}
                    onClick={removeAiKey}
                  >
                    {aiKeyBusy === 'delete' ? 'Removing the key…' : 'Remove the saved key'}
                  </button>
                  {aiKeyTrouble !== null && (
                    <FailureNotice
                      failure={aiKeyTrouble}
                      headline="The API key could not be saved."
                      // 저장 실패에는 재시도를 두지 않는다 — 넘긴 값이 이미 화면에 없다.
                      onRetry={aiKeyBusy === null && aiKeyState === 'stored' ? removeAiKey : undefined}
                    />
                  )}
                </>
              )}

              <label className="field" htmlFor="ai-base-url">
                <span className="field__label">Address (host and port)</span>
                <input
                  id="ai-base-url"
                  type="text"
                  className="input"
                  placeholder={AI_BASE_URL_PLACEHOLDER}
                  value={form.aiBaseUrl}
                  onChange={(event) => edit({ aiBaseUrl: event.currentTarget.value })}
                />
              </label>
              <p className="hint">{AI_BASE_URL_NOTICE}</p>

              <button
                type="button"
                className="btn btn--secondary"
                disabled={connection.kind === 'checking'}
                onClick={checkProvider}
              >
                {connection.kind === 'checking' ? 'Checking…' : 'Check the AI provider'}
              </button>
              <p className="hint">{AI_CHECK_USES_SAVED_SETTINGS}</p>
              {staleCheck && (
                <p className="hint">
                  The AI settings above have changed since the last save, so this result is about
                  the saved ones.
                </p>
              )}

              {/* 실행 중 · 모델 없음 · 미실행 · 확인 거절이 서로 다른 값으로 온다. 화면은
                  그것을 다시 뭉치지 않는다 (`AiConnection`). 물어보는 동안에는 응답을
                  기다리는 중이라는 사실이 보인다 — 대답이 없는 주소일 수도 있다 (요구 9). */}
              {connection.kind === 'checking' ? (
                <Loading text={connection.text} live />
              ) : (
                <p className="hint">{connection.text}</p>
              )}
              {connection.kind === 'notConfigured' && (
                <p className="hint">{connection.resolution}</p>
              )}
              {connection.kind === 'noModels' && <p className="hint">{connection.resolution}</p>}
              {connection.kind === 'notRunning' && (
                <>
                  <p className="hint">{connection.resolution}</p>
                  {connection.failure !== null && (
                    <FailureNotice failure={connection.failure} onRetry={checkProvider} />
                  )}
                </>
              )}
              {connection.kind === 'checkFailed' && (
                <FailureNotice failure={connection.failure} onRetry={checkProvider} />
              )}

              <label className="field" htmlFor="ai-model">
                <span className="field__label">Model</span>
                <select
                  id="ai-model"
                  className="select"
                  value={form.aiModel}
                  onChange={(event) => edit({ aiModel: event.currentTarget.value })}
                >
                  {/* 목록은 확인이 돌려준 것이다. 저장된 모델이 지금 없으면 그 항목도 함께
                      남는다 — 고른 값이 말없이 다른 모델로 바뀌지 않는다. */}
                  {aiModelOptions(form.aiModel, confirmedAiModels(connection)).map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
              {modelNotice !== null && <p className="hint">{modelNotice}</p>}
            </>
          )}

          {/* AI 쪽이 어떻게 끝나든 나머지 설정은 그대로 저장된다 (INV-8). */}
          <p className="hint">{AI_SETTINGS_UNAFFECTED_NOTICE}</p>
        </section>

        {/* 스스로 설치해서 쓰는 provider는 **뒤에** 온다 (요구 7 · ADR-0010 §4.3). 여기에는
            고르는 자리도 확인 버튼도 없다 — 그것은 전부 위 부분에 그대로 있고 (MH-8), 이
            부분이 하는 일은 그 선택지 하나가 무엇이고 어떤 위치에 있는지 말하는 것뿐이다.
            제목의 이름과 로컬 표시는 provider 목록에서 온다 (INV-9). */}
        {localProviderSetups(form.aiProvider).map((setup) => (
          <section className="group__part" key={setup.id}>
            <h3 className="group__subtitle">{setup.title}</h3>
            <p className="hint">{setup.standing}</p>
            <p className="hint">{setup.text}</p>
            <p className="hint">{setup.statusText}</p>
            <p className="hint">{setup.howToTurnOn}</p>
          </section>
        ))}
      </section>

      <section className="group">
        <h2 className="group__title">Notion</h2>

        {/* 저장돼 있는가 — 그것이 화면이 token에 대해 아는 전부다 (INV-7). 저장된 값이
            없는 것은 오류가 아니라 상태이므로 담담한 문장 한 줄로 보인다 (INV-8).
            빈 상태 다섯 — Notion이 아직 설정되지 않았다. 다른 네 자리와 같은 모양을 쓰며
            경고색도 테두리도 없다 (요구 9). 무엇을 저장했는지 판정하는 것은 여기가 아니라
            `notionTokenState`이고, 화면은 그 값에 맞는 모양을 고를 뿐이다. */}
        {tokenState === 'notStored' ? (
          <EmptyState title={tokenNotice.text} body={tokenNotice.resolution} />
        ) : (
          <>
            <p className="hint">{tokenNotice.text}</p>
            {tokenNotice.resolution !== null && <p className="hint">{tokenNotice.resolution}</p>}
          </>
        )}

        <label className="field" htmlFor="notion-token">
          <span className="field__label">Integration token</span>
          {/* **`value`가 없다.** 입력한 값은 React 상태에 들어가지 않고, 저장된 값이 여기
              채워지는 일도 없다 — 되읽는 command 자체가 없다 (INV-7). */}
          <input
            id="notion-token"
            type="password"
            className="input"
            placeholder={TOKEN_INPUT_PLACEHOLDER}
            autoComplete="off"
            ref={tokenInput}
          />
        </label>
        <p className="hint">{TOKEN_INPUT_NOTICE}</p>

        <button
          type="button"
          className="btn btn--secondary"
          disabled={tokenBusy !== null}
          onClick={saveToken}
        >
          {tokenBusy === 'save' ? 'Saving the token…' : 'Save the token'}
        </button>
        {/* 저장된 것을 없애는 동작이므로 다른 버튼과 같은 무게로 두지 않는다 (§19). */}
        <button
          type="button"
          className="btn btn--danger"
          disabled={tokenBusy !== null}
          onClick={removeToken}
        >
          {tokenBusy === 'delete' ? 'Removing the token…' : 'Remove the saved token'}
        </button>
        {tokenTrouble !== null && (
          <FailureNotice
            failure={tokenTrouble.failure}
            headline={tokenTrouble.text}
            // 저장 실패에는 다시 시도 수단을 두지 않는다 — 넘긴 값은 이미 화면에 없으므로
            // 같은 버튼이 같은 일을 할 수 없다. 지우기는 값 없이 다시 할 수 있다.
            onRetry={tokenTrouble.request === 'delete' ? removeToken : undefined}
          />
        )}

        <label className="field" htmlFor="notion-parent-page">
          <span className="field__label">Parent page (destination)</span>
          {/* secret이 아니라 **어디에 쓰는지**다. 그래서 다른 설정과 같은 폼 값이며 화면
              전체의 Save 하나가 저장한다 (ADR-0009 §8.4 · `settingsView.ts`). */}
          <input
            id="notion-parent-page"
            type="text"
            className="input"
            placeholder="Not set"
            value={form.notionParentPageId}
            onChange={(event) => edit({ notionParentPageId: event.currentTarget.value })}
          />
        </label>
        {destinationNotice !== null && <p className="hint">{destinationNotice}</p>}
        <p className="hint">{HOW_TO_SET_A_DESTINATION}</p>

        <button
          type="button"
          className="btn btn--secondary"
          disabled={notion.kind === 'checking'}
          onClick={checkNotion}
        >
          {notion.kind === 'checking' ? 'Checking…' : 'Check the Notion connection'}
        </button>
        <p className="hint">{NOTION_CHECK_USES_SAVED_SETTINGS}</p>
        {staleDestination && (
          <p className="hint">
            The parent page above has changed since the last save, so this result is about the
            saved one.
          </p>
        )}

        {/* 성공 · token 없음 · 인증 실패 · 권한 없는 destination · 네트워크 없음 · 확인 거절이
            서로 다른 값으로 온다. 화면은 그것을 다시 뭉치지 않는다 (`NotionConnectionView`).
            물어보는 동안에는 기다리는 중이라는 사실이 보인다 — 왕복이므로 즉시 끝나지
            않는다 (요구 9). */}
        {notion.kind === 'checking' ? (
          <Loading text={notion.text} live />
        ) : (
          <p className="hint">{notion.text}</p>
        )}
        {notion.kind === 'noToken' && <p className="hint">{notion.resolution}</p>}
        {notion.kind === 'connected' && notion.destinationNotice !== null && (
          <p className="hint">{notion.destinationNotice}</p>
        )}
        {notion.kind === 'failed' && (
          <>
            <p className="hint">{notion.resolution}</p>
            <FailureNotice failure={notion.failure} onRetry={checkNotion} />
          </>
        )}
        {notion.kind === 'checkFailed' && (
          <FailureNotice failure={notion.failure} onRetry={checkNotion} />
        )}

        {/* Notion 쪽이 어떻게 끝나든 나머지 설정은 그대로 저장된다 (INV-8). */}
        <p className="hint">{NOTION_SETTINGS_UNAFFECTED_NOTICE}</p>
      </section>

      {/* 설정은 한 벌이므로 저장도 한 번이다. 이 화면의 primary는 이것 하나다 (§19). */}
      <section className="group">
        <button
          type="button"
          className="btn btn--primary"
          disabled={saving}
          onClick={() => save(form)}
        >
          {saving ? 'Saving…' : 'Save'}
        </button>
        <p className="hint">Saves every setting on this screen.</p>
        {/* 저장됐다는 사실도 글자로 말한다 — 색만으로 말하는 자리를 두지 않는다 (요구 12). */}
        {saved && (
          <p className="hint" role="status">
            Saved.
          </p>
        )}
        {failure !== null && <FailureNotice failure={failure} onRetry={() => save(form)} />}
      </section>
    </div>
  );
}
