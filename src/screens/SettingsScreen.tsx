import { useEffect, useState } from 'react';
import { getSettings, listInputDevices, updateSettings } from '../ipc/commands';
import { toFailure, type Failure } from '../ipc/failure';
import type { InputDevice } from '../ipc/types';
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
  toSettings,
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
 * Transcription / AI Provider / Notion은 섹션 자리만 둔다 — 그 안의 기능은 Phase 3·4·5의 일이고,
 * secret(API key · integration token) 입력은 INV-7에 따라 Phase 1에서 다루지 않는다.
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
        if (current) setView(loadedSettings(settings));
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
      (settings) => setView(savedSettings(settings)),
      (error: unknown) => setView((state) => failedSave(state, error)),
    );
  };

  const { form, saving, saved, failure } = view;
  // 저장된 값과 지금 있는 장치를 맞춰 본다. 판단은 순수 모듈이 하고 화면은 그리기만 한다.
  const chosen = chosenMicrophone(form.defaultMicrophone);
  const microphoneNotice = defaultMicrophoneNotice(resolveDefaultMicrophone(chosen, devices));

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

        <button type="button" className="action" disabled={saving} onClick={() => save(form)}>
          {saving ? 'Saving…' : 'Save'}
        </button>
        {saved && <p className="hint">Saved.</p>}
        {failure !== null && <FailureNotice failure={failure} onRetry={() => save(form)} />}
      </section>

      <section className="group">
        <h2 className="group__title">Transcription</h2>
        <p className="hint">Not available yet.</p>
      </section>

      <section className="group">
        <h2 className="group__title">AI Provider</h2>
        <p className="hint">Not available yet.</p>
      </section>

      <section className="group">
        <h2 className="group__title">Notion</h2>
        <p className="hint">Not available yet.</p>
      </section>
    </div>
  );
}
