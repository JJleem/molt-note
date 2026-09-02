// Settings 화면 상태 변환 테스트.
//
// 읽기 → 편집 → 저장 → 다시 읽기가 상태로 어떻게 나타나는지, 그리고 실패가 어디로 가는지 본다.
// DOM도 Tauri도 필요하지 않다 (§18).
import { describe, expect, it } from 'vitest';
import type { Failure } from '../ipc/failure';
import type { Settings } from '../ipc/types';
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
} from './settingsView';

const DEFAULT_SETTINGS: Settings = { recordingsDirectory: null, automaticProcessing: false };

const storageFailure: Failure = {
  kind: 'storage',
  message: '로컬 저장소를 열지 못했다.',
  detail: 'unable to open database file',
  sourceDataSafe: true,
  retryable: true,
};

/** 읽기까지 끝난 상태를 만든다. */
function ready(settings: Settings = DEFAULT_SETTINGS) {
  return loadedSettings(settings);
}

describe('설정 읽기', () => {
  it('첫 상태는 아직 읽지 못한 상태다', () => {
    expect(LOADING_SETTINGS.kind).toBe('loading');
  });

  it('저장된 적이 없어 기본값이 와도 정상 상태다', () => {
    const view = ready();

    expect(view.kind).toBe('ready');
    if (view.kind !== 'ready') return;
    // 고르지 않은 디렉터리(null)는 빈 입력이다.
    expect(view.form).toEqual({ recordingsDirectory: '', automaticProcessing: false });
    expect(view.saving).toBe(false);
    expect(view.saved).toBe(false);
    expect(view.failure).toBeNull();
  });

  it('저장된 값이 폼에 그대로 들어온다', () => {
    const view = ready({ recordingsDirectory: '/Users/someone/Recordings', automaticProcessing: true });

    expect(view.kind === 'ready' && view.form).toEqual({
      recordingsDirectory: '/Users/someone/Recordings',
      automaticProcessing: true,
    });
  });

  it('설정을 읽지 못하면 화면이 실패 상태가 된다', () => {
    // 저장소 초기화 실패가 여기로 온다 (§13).
    expect(failedSettings(storageFailure)).toEqual({ kind: 'failed', failure: storageFailure });
  });

  it('계약과 다른 값으로 거절돼도 보여줄 수 있는 실패가 된다', () => {
    const view = failedSettings('rejected');

    expect(view.kind).toBe('failed');
    if (view.kind !== 'failed') return;
    expect(view.failure.message.length).toBeGreaterThan(0);
    expect(view.failure.detail).toBe('rejected');
  });
});

describe('편집', () => {
  it('입력한 값이 폼에 남는다', () => {
    const view = editedSettings(ready(), { recordingsDirectory: '/tmp/notes' });

    expect(view.kind === 'ready' && view.form.recordingsDirectory).toBe('/tmp/notes');
  });

  it('토글은 디렉터리를 건드리지 않는다', () => {
    const edited = editedSettings(
      editedSettings(ready(), { recordingsDirectory: '/tmp/notes' }),
      { automaticProcessing: true },
    );

    expect(edited.kind === 'ready' && edited.form).toEqual({
      recordingsDirectory: '/tmp/notes',
      automaticProcessing: true,
    });
  });

  it('편집하면 "저장됨" 표시가 사라진다', () => {
    const saved = savedSettings(DEFAULT_SETTINGS);
    expect(saved.kind === 'ready' && saved.saved).toBe(true);

    const edited = editedSettings(saved, { automaticProcessing: true });
    expect(edited.kind === 'ready' && edited.saved).toBe(false);
  });

  it('읽지 못한 상태에서는 편집할 값이 없다', () => {
    const failed = failedSettings(storageFailure);

    expect(editedSettings(failed, { automaticProcessing: true })).toBe(failed);
    expect(editedSettings(LOADING_SETTINGS, { automaticProcessing: true })).toBe(LOADING_SETTINGS);
  });
});

describe('저장', () => {
  it('저장 중에는 진행 중이라는 사실이 상태로 보인다', () => {
    const saving = savingSettings(editedSettings(ready(), { recordingsDirectory: '/tmp/notes' }));

    expect(saving.kind === 'ready' && saving.saving).toBe(true);
    // 편집 중이던 값은 그대로다.
    expect(saving.kind === 'ready' && saving.form.recordingsDirectory).toBe('/tmp/notes');
  });

  it('빈 입력은 "고르지 않음"으로 보낸다', () => {
    expect(toSettings({ recordingsDirectory: '', automaticProcessing: true })).toEqual({
      recordingsDirectory: null,
      automaticProcessing: true,
    });
    expect(toSettings({ recordingsDirectory: '   ', automaticProcessing: false })).toEqual({
      recordingsDirectory: null,
      automaticProcessing: false,
    });
  });

  it('저장 뒤 폼은 저장소가 돌려준 값으로 다시 채워진다', () => {
    // Rust가 정규화한 값이 있으면 화면은 그 값을 본다 — 보낸 값을 그대로 믿지 않는다.
    const view = savedSettings({ recordingsDirectory: '/tmp/notes', automaticProcessing: true });

    expect(view.kind).toBe('ready');
    if (view.kind !== 'ready') return;
    expect(view.form).toEqual({ recordingsDirectory: '/tmp/notes', automaticProcessing: true });
    expect(view.saving).toBe(false);
    expect(view.saved).toBe(true);
    expect(view.failure).toBeNull();
  });

  it('저장이 실패하면 화면에 실패가 남고 입력한 값은 버려지지 않는다', () => {
    const editing = savingSettings(editedSettings(ready(), { recordingsDirectory: '/tmp/notes' }));
    const view = failedSave(editing, storageFailure);

    expect(view.kind).toBe('ready');
    if (view.kind !== 'ready') return;
    expect(view.failure).toEqual(storageFailure);
    expect(view.saving).toBe(false);
    expect(view.saved).toBe(false);
    expect(view.form.recordingsDirectory).toBe('/tmp/notes');
  });

  it('다시 저장을 시작하면 지난 실패는 지워진다', () => {
    const failed = failedSave(savingSettings(ready()), storageFailure);
    const retrying = savingSettings(failed);

    expect(retrying.kind === 'ready' && retrying.failure).toBeNull();
    expect(retrying.kind === 'ready' && retrying.saving).toBe(true);
  });

  it('저장 실패가 "저장됨"으로 남지 않는다', () => {
    const view = failedSave(savedSettings(DEFAULT_SETTINGS), storageFailure);

    expect(view.kind === 'ready' && view.saved).toBe(false);
  });
});

describe('폼과 설정의 변환', () => {
  it('읽은 값을 그대로 돌려보낼 수 있다', () => {
    for (const settings of [
      DEFAULT_SETTINGS,
      { recordingsDirectory: '/tmp/notes', automaticProcessing: true },
    ] satisfies Settings[]) {
      expect(toSettings(toForm(settings))).toEqual(settings);
    }
  });
});
