import { useEffect, useState } from 'react';
import { aiProviderStatus, getSettings, listInputDevices, updateSettings } from '../ipc/commands';
import { toFailure, type Failure } from '../ipc/failure';
import type { InputDevice } from '../ipc/types';
import {
  AI_BASE_URL_NOTICE,
  AI_BASE_URL_PLACEHOLDER,
  AI_CHECK_USES_SAVED_SETTINGS,
  AI_NOT_CHECKED_TEXT,
  AI_SETTINGS_UNAFFECTED_NOTICE,
  CHECKING_AI_PROVIDER_TEXT,
  NOTHING_LEAVES_THIS_DEVICE,
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
  type AiConnection,
  type AiSettingsSnapshot,
} from './aiProviderSettings';
import {
  chosenMicrophone,
  defaultMicrophoneNotice,
  microphoneOptions,
  resolveDefaultMicrophone,
} from './defaultMicrophone';
import { FailureNotice } from './FailureNotice';
import {
  LOADING_SETTINGS,
  editedSettings,
  failedSave,
  failedSettings,
  loadedSettings,
  savedSettings,
  savingSettings,
  toForm,
  toSettings,
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
 * Transcription 그룹은 **자동 전사 토글**과 **모델 선택** 둘이다 (Phase 3). 모델이 없어서
 * 지금 전사할 수 없다는 것은 여기서 보이는 **제품 상태**이며, 그 사실 때문에 자동 전사 토글이
 * 뒤집히지 않는다 — 사용자가 켠 값은 켜진 채로 남는다 (ADR-0007 §8.2.3).
 *
 * AI Provider 그룹은 **고르기 · 연결 확인 · 모델 선택 · 전송 경계 표시** 넷이다 (Phase 4).
 * 연결 확인은 사용자가 눌러야 나가며, 화면을 열자마자 서버를 찾아 나서지 않는다. 확인이
 * 어떻게 끝나든 — 응답이 없든 모델이 없든 확인 자체가 거절되든 — **그 결과는 나머지 설정의
 * 저장 경로에 닿지 않는다** (INV-8). AI 상태를 `SettingsView`가 아니라 별도의 state로 들고
 * 있는 이유가 이것이며, 장치 목록 실패를 설정 읽기 실패와 섞지 않는 것과 같은 규칙이다.
 *
 * Notion은 섹션 자리만 둔다 — 그 안의 기능은 Phase 5의 일이고, secret(API key ·
 * integration token) 입력은 INV-7에 따라 다루지 않는다.
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
        <p className="hint">Loading settings…</p>
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

  const { form, saving, saved, failure } = view;
  // 저장된 값과 지금 있는 장치를 맞춰 본다. 판단은 순수 모듈이 하고 화면은 그리기만 한다.
  const chosen = chosenMicrophone(form.defaultMicrophone);
  const microphoneNotice = defaultMicrophoneNotice(resolveDefaultMicrophone(chosen, devices));

  // AI 쪽도 같다 — 로컬/외부도, 확인 결과의 갈래도, 모델 목록도 전부 순수 모듈이 정한다.
  const providerChosen = form.aiProvider !== '';
  const transfer = aiTransferNotice(aiProviderLocality(form.aiProvider));
  const modelNotice = aiModelNotice(form.aiModel, connection);
  const staleCheck = aiSettingsChanged(form, savedAi);

  return (
    <div className="screen">
      <section className="group">
        <h2 className="group__title">Recording</h2>

        <label className="field" htmlFor="recordings-directory">
          <span className="field__label">Recordings directory</span>
          <input
            id="recordings-directory"
            type="text"
            className="field__input"
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
            className="field__input"
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
            className="field__input"
            placeholder="Not set"
            value={form.transcriptionModel}
            onChange={(event) => edit({ transcriptionModel: event.currentTarget.value })}
          />
        </label>

        <label className="field field--inline" htmlFor="automatic-transcription">
          <input
            id="automatic-transcription"
            type="checkbox"
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
        <h2 className="group__title">AI Provider</h2>

        <label className="field" htmlFor="ai-provider">
          <span className="field__label">Provider</span>
          <select
            id="ai-provider"
            className="field__input"
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
            <label className="field" htmlFor="ai-base-url">
              <span className="field__label">Address (host and port)</span>
              <input
                id="ai-base-url"
                type="text"
                className="field__input"
                placeholder={AI_BASE_URL_PLACEHOLDER}
                value={form.aiBaseUrl}
                onChange={(event) => edit({ aiBaseUrl: event.currentTarget.value })}
              />
            </label>
            <p className="hint">{AI_BASE_URL_NOTICE}</p>

            <button
              type="button"
              className="action"
              disabled={connection.kind === 'checking'}
              onClick={checkProvider}
            >
              {connection.kind === 'checking' ? 'Checking…' : 'Check the AI provider'}
            </button>
            <p className="hint">{AI_CHECK_USES_SAVED_SETTINGS}</p>
            {staleCheck && (
              <p className="hint">
                The AI settings above have changed since the last save, so this result is about the
                saved ones.
              </p>
            )}

            {/* 실행 중 · 모델 없음 · 미실행 · 확인 거절이 서로 다른 값으로 온다. 화면은
                그것을 다시 뭉치지 않는다 (`AiConnection`). */}
            <p className="hint">{connection.text}</p>
            {connection.kind === 'notConfigured' && <p className="hint">{connection.resolution}</p>}
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
                className="field__input"
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

      <section className="group">
        <h2 className="group__title">Notion</h2>
        <p className="hint">Not available yet.</p>
      </section>

      {/* 설정은 한 벌이므로 저장도 한 번이다. */}
      <section className="group">
        <button type="button" className="action" disabled={saving} onClick={() => save(form)}>
          {saving ? 'Saving…' : 'Save'}
        </button>
        <p className="hint">Saves every setting on this screen.</p>
        {saved && <p className="hint">Saved.</p>}
        {failure !== null && <FailureNotice failure={failure} onRetry={() => save(form)} />}
      </section>
    </div>
  );
}
