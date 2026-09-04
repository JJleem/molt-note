// Settings 화면 상태 변환 테스트.
//
// 읽기 → 편집 → 저장 → 다시 읽기가 상태로 어떻게 나타나는지, 그리고 실패가 어디로 가는지 본다.
// DOM도 Tauri도 필요하지 않다 (§18).
import { describe, expect, it } from 'vitest';
import type { Failure } from '../ipc/failure';
import type { Settings } from '../ipc/types';
import {
  AUTOMATIC_TRANSCRIPTION_STAYS_ON_NOTICE,
  HOW_TO_SET_A_TRANSCRIPTION_MODEL,
  LOADING_SETTINGS,
  NO_TRANSCRIPTION_MODEL_NOTICE,
  editedSettings,
  failedSave,
  failedSettings,
  loadedSettings,
  savedSettings,
  savingSettings,
  toForm,
  toSettings,
  transcriptionModel,
  transcriptionNotices,
  type SettingsForm,
} from './settingsView';

const DEFAULT_SETTINGS: Settings = {
  recordingsDirectory: null,
  automaticProcessing: false,
  automaticTranscription: false,
  transcriptionModel: null,
  defaultMicrophone: null,
  // provider를 고르지 않은 것이 기본이자 정상 상태다 (ADR-0008 §11.1 · INV-8).
  aiProvider: null,
  aiBaseUrl: null,
  aiModel: null,
};

/** 기본값에서 몇 가지만 다른 폼 값. 테스트가 관심 있는 값만 적는다. */
function form(changes: Partial<SettingsForm> = {}): SettingsForm {
  return { ...toForm(DEFAULT_SETTINGS), ...changes };
}

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
    // 고르지 않은 디렉터리(null)는 빈 입력이다. 고르지 않은 마이크·모델도 마찬가지다.
    expect(view.form).toEqual({
      recordingsDirectory: '',
      automaticProcessing: false,
      automaticTranscription: false,
      transcriptionModel: '',
      defaultMicrophone: '',
      aiProvider: '',
      aiBaseUrl: '',
      aiModel: '',
    });
    expect(view.saving).toBe(false);
    expect(view.saved).toBe(false);
    expect(view.failure).toBeNull();
  });

  it('저장된 값이 폼에 그대로 들어온다', () => {
    const view = ready({
      ...DEFAULT_SETTINGS,
      recordingsDirectory: '/Users/someone/Recordings',
      automaticProcessing: true,
      automaticTranscription: true,
      transcriptionModel: 'ggml-base.bin',
      defaultMicrophone: '0:Studio Mic',
    });

    expect(view.kind === 'ready' && view.form).toEqual({
      recordingsDirectory: '/Users/someone/Recordings',
      automaticProcessing: true,
      automaticTranscription: true,
      transcriptionModel: 'ggml-base.bin',
      defaultMicrophone: '0:Studio Mic',
      aiProvider: '',
      aiBaseUrl: '',
      aiModel: '',
    });
  });

  it('고치지 않은 AI 설정은 읽은 그대로 다시 나간다', () => {
    // AI 세 값에는 입력란이 있지만(`aiProviderSettings.ts`), 손대지 않은 값이 저장하는 순간
    // 달라지면 다른 설정을 한 번 저장한 것만으로 사용자가 고른 provider가 바뀐다.
    const stored: Settings = {
      ...DEFAULT_SETTINGS,
      aiProvider: 'some-provider',
      aiBaseUrl: 'http://127.0.0.1:9999',
      aiModel: 'some-model',
    };

    const view = editedSettings(ready(stored), { automaticProcessing: true });
    expect(view.kind).toBe('ready');
    if (view.kind !== 'ready') return;

    expect(toSettings(view.form)).toEqual({ ...stored, automaticProcessing: true });
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

    expect(edited.kind === 'ready' && edited.form).toEqual(
      form({ recordingsDirectory: '/tmp/notes', automaticProcessing: true }),
    );
  });

  it('두 자동 토글은 서로를 켜지 않는다', () => {
    // 하나의 boolean에 두 의미가 겹치지 않는다 — 값도, 편집도 따로다.
    const processing = editedSettings(ready(), { automaticProcessing: true });
    const transcription = editedSettings(ready(), { automaticTranscription: true });

    expect(processing.kind === 'ready' && processing.form).toEqual(
      form({ automaticProcessing: true }),
    );
    expect(transcription.kind === 'ready' && transcription.form).toEqual(
      form({ automaticTranscription: true }),
    );
  });

  it('모델을 적는 것이 다른 값을 건드리지 않는다', () => {
    const edited = editedSettings(
      editedSettings(ready(), { automaticTranscription: true }),
      { transcriptionModel: 'ggml-medium.bin' },
    );

    expect(edited.kind === 'ready' && edited.form).toEqual(
      form({ automaticTranscription: true, transcriptionModel: 'ggml-medium.bin' }),
    );
  });

  it('마이크를 고르는 것이 다른 값을 건드리지 않는다', () => {
    const edited = editedSettings(
      editedSettings(ready(), { recordingsDirectory: '/tmp/notes' }),
      { defaultMicrophone: '1:USB Microphone' },
    );

    expect(edited.kind === 'ready' && edited.form).toEqual(
      form({ recordingsDirectory: '/tmp/notes', defaultMicrophone: '1:USB Microphone' }),
    );
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
    expect(toSettings(form({ automaticProcessing: true }))).toEqual({
      ...DEFAULT_SETTINGS,
      automaticProcessing: true,
    });
    expect(
      toSettings(form({ recordingsDirectory: '   ', transcriptionModel: '  \n ' })),
    ).toEqual(DEFAULT_SETTINGS);
  });

  it('고른 마이크 키는 그대로 저장하러 간다', () => {
    // 지금 없는 장치의 키라도 화면이 바꾸지 않는다 — 사용자가 고른 값이다.
    expect(toSettings(form({ defaultMicrophone: '0:Studio Mic' })).defaultMicrophone).toBe(
      '0:Studio Mic',
    );
  });

  it('적은 모델 값은 그대로 저장하러 간다', () => {
    // 지금 그 자리에 없는 파일이라도 화면이 바꾸지 않는다 — 사용자가 고른 값이다.
    // 파일을 찾아보는 것은 전사를 시작할 때이며, 그것은 화면의 일이 아니다.
    expect(
      toSettings(form({ transcriptionModel: '  /Users/someone/models/ggml-large-v3.bin ' }))
        .transcriptionModel,
    ).toBe('/Users/someone/models/ggml-large-v3.bin');
  });

  it('모델이 없어도 자동 전사 토글은 사용자가 둔 값 그대로 간다', () => {
    // 앱이 대신 끄지 않는다 (ADR-0007 §8.2.3).
    const saved = toSettings(form({ automaticTranscription: true, transcriptionModel: '' }));

    expect(saved.automaticTranscription).toBe(true);
    expect(saved.transcriptionModel).toBeNull();
  });

  it('저장 뒤 폼은 저장소가 돌려준 값으로 다시 채워진다', () => {
    // Rust가 정규화한 값이 있으면 화면은 그 값을 본다 — 보낸 값을 그대로 믿지 않는다.
    const view = savedSettings({
      ...DEFAULT_SETTINGS,
      recordingsDirectory: '/tmp/notes',
      automaticProcessing: true,
      automaticTranscription: true,
      transcriptionModel: 'ggml-base.bin',
      defaultMicrophone: '0:Studio Mic',
    });

    expect(view.kind).toBe('ready');
    if (view.kind !== 'ready') return;
    expect(view.form).toEqual({
      recordingsDirectory: '/tmp/notes',
      automaticProcessing: true,
      automaticTranscription: true,
      transcriptionModel: 'ggml-base.bin',
      defaultMicrophone: '0:Studio Mic',
      aiProvider: '',
      aiBaseUrl: '',
      aiModel: '',
    });
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
      { ...DEFAULT_SETTINGS, recordingsDirectory: '/tmp/notes', automaticProcessing: true },
      // 지금 목록에 없는 장치의 키도 돌려보낼 때 그대로여야 한다.
      { ...DEFAULT_SETTINGS, defaultMicrophone: '3:USB Microphone' },
      // 두 토글의 네 조합이 전부 그대로 돌아와야 서로 다른 값이라고 말할 수 있다.
      { ...DEFAULT_SETTINGS, automaticProcessing: true, automaticTranscription: false },
      { ...DEFAULT_SETTINGS, automaticProcessing: false, automaticTranscription: true },
      { ...DEFAULT_SETTINGS, automaticProcessing: true, automaticTranscription: true },
      // 지금 그 자리에 없을 수 있는 모델 값도 마찬가지다.
      { ...DEFAULT_SETTINGS, automaticTranscription: true, transcriptionModel: '없는-모델.bin' },
      // AI 설정 세 값도 왕복에서 사라지거나 달라지지 않는다 (ADR-0008 §11.1).
      {
        ...DEFAULT_SETTINGS,
        aiProvider: 'some-provider',
        aiBaseUrl: 'http://127.0.0.1:9999',
        aiModel: 'some-model',
      },
      // 지금 응답하지 않는 서버나 지워진 모델을 가리키더라도 값은 그대로 돌아온다.
      { ...DEFAULT_SETTINGS, aiProvider: 'some-provider', aiModel: '없는-모델' },
    ] satisfies Settings[]) {
      expect(toSettings(toForm(settings))).toEqual(settings);
    }
  });
});

describe('모델이 없는 상태는 화면 상태다', () => {
  // 조용한 skip도, 설정을 대신 고치는 것도 아니다 (ADR-0007 §8.2.3 · §13).

  it('모델을 고르지 않았다는 것과 골랐다는 것이 서로 다른 상태다', () => {
    expect(transcriptionModel(form())).toEqual({ kind: 'notChosen' });
    expect(transcriptionModel(form({ transcriptionModel: '   ' }))).toEqual({ kind: 'notChosen' });
    expect(transcriptionModel(form({ transcriptionModel: ' ggml-base.bin ' }))).toEqual({
      kind: 'chosen',
      value: 'ggml-base.bin',
    });
  });

  it('모델이 없으면 지금 전사할 수 없다는 사실과 푸는 방법을 함께 보여준다', () => {
    const notices = transcriptionNotices(form());

    expect(notices).toContain(NO_TRANSCRIPTION_MODEL_NOTICE);
    expect(notices).toContain(HOW_TO_SET_A_TRANSCRIPTION_MODEL);
  });

  it('자동 전사가 켜져 있으면 그 값이 그대로 남는다는 사실도 말한다', () => {
    const on = form({ automaticTranscription: true });

    expect(transcriptionNotices(on)).toContain(AUTOMATIC_TRANSCRIPTION_STAYS_ON_NOTICE);
    // 말이 늘어날 뿐 값은 그대로다 — 앱이 사용자의 토글을 대신 끄지 않는다.
    expect(on.automaticTranscription).toBe(true);
    expect(toSettings(on).automaticTranscription).toBe(true);
  });

  it('모델을 골랐으면 할 말이 없다', () => {
    // 그 파일이 실제로 그 자리에 있는지는 화면이 알 수 없다. 아는 척하지 않는다.
    expect(transcriptionNotices(form({ transcriptionModel: 'ggml-base.bin' }))).toEqual([]);
    expect(
      transcriptionNotices(
        form({ transcriptionModel: 'ggml-base.bin', automaticTranscription: true }),
      ),
    ).toEqual([]);
  });

  it('상태를 읽는 것이 폼 값을 바꾸지 않는다', () => {
    const before = form({ automaticTranscription: true });

    transcriptionNotices(before);
    transcriptionModel(before);

    expect(before).toEqual(form({ automaticTranscription: true }));
  });
});
