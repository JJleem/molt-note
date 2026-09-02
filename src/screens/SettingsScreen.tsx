import { useEffect, useState } from 'react';
import { getSettings, updateSettings } from '../ipc/commands';
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
 * Recording 그룹의 두 값은 저장소에 영속화된다 — `get_settings`로 읽고 `update_settings`로
 * 저장한 뒤, **저장소가 돌려준 값으로** 폼을 다시 채운다. 무엇이 저장됐는지 화면이
 * 추측하지 않는다.
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

  /** 다시 읽는다. 상태를 되돌리는 것은 effect가 아니라 이 사용자 동작의 일이다. */
  const retryLoad = () => {
    setView(LOADING_SETTINGS);
    setAttempt((count) => count + 1);
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
