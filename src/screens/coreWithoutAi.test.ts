// **AI가 없어도 화면은 화면이다** (INV-8 · phase-prompt/04 성공 기준 2).
//
// 개별 모듈의 동작은 각자의 테스트가 본다. 여기서 보는 것은 **모듈을 가로지르는 한 가지
// 사실**이며, 그래서 한 파일에 모여 있다 — AI가 하나도 설정되지 않았거나, 설정했지만 되지
// 않을 때, 나머지 세 경로가 그대로 도는가.
//
//   1. 녹음      RecordingScreen의 상태 — 시작 · 정지 · 저장 결과      (§5 B · R-002)
//   2. 전사      Transcript 탭의 상태 — 대기 · 진행 · 완료 · 실패      (§7 · phase-prompt/03)
//   3. 열람      Recordings 목록과 Recording 상세                      (§5 A · §5 C)
//
// 판정 방식이 요지다. "AI 값을 하나 바꿔도 나머지 화면 값이 **글자 하나 달라지지 않는다**"를
// 깊은 비교로 확인한다 — 어떤 AI 상태에서도 같은 값이 나온다는 것이 INV-8이 화면에서 뜻하는
// 바이기 때문이다.
//
// 그리고 그 반대편도 함께 본다: AI 자신의 상태는 **오류가 아니라 담담한 상태**로 표현되고
// (`disabled` — `failure`가 아예 없다), 실패했을 때는 §13이 요구하는 세 가지가 값으로 남는다
// (무엇이 실패했는가 · 원본은 안전한가 · 다시 시도할 수 있는가).
//
// Rust도 Ollama도 Whisper도 DOM도 필요하지 않다 (§18).
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type {
  AiNote,
  AiNoteStatus,
  AiProviderStatus,
  MeetingNote,
  ProcessingStatus,
  Recording,
  SessionStatus,
  StoppedRecording,
  Transcript,
} from '../ipc/types';
import {
  AI_DISABLED_HEADLINE,
  AI_NOTE_FAILED_HEADLINE,
  AI_NOTE_PRESERVED_NOTICE,
  AI_UNAFFECTED_NOTICE,
  aiNoteTab,
  type AiNoteInput,
} from './aiNoteView';
import { loadedRecordingDetail } from './recordingDetailView';
import { loadedRecordings } from './recordingsView';
import {
  INITIAL_RECORDING,
  observedSession,
  recordingControls,
  requestedAction,
  savedRecording,
  sessionDisplay,
} from './recordingView';
import { transcriptTab } from './transcriptView';

// --- 사실들 --------------------------------------------------------------------------

/** 저장소가 돌려주는 모양 그대로의 레코드. 전사는 끝났고 AI는 아직 시도한 적이 없다. */
function recording(overrides: Partial<Recording> = {}): Recording {
  return {
    id: 'r-1',
    title: '3DGS Study #04',
    createdAt: '2026-09-01T09:12:00.000Z',
    updatedAt: '2026-09-01T09:12:00.000Z',
    durationMs: 3_151_000,
    durationLabel: '52:31',
    audioPath: '/recordings/r-1.wav',
    audioFormat: 'wav',
    microphone: 'MacBook Microphone',
    currentTranscriptId: 't-1',
    transcriptionStatus: 'done',
    aiStatus: 'none',
    notionStatus: 'none',
    ...overrides,
  };
}

function transcript(overrides: Partial<Transcript> = {}): Transcript {
  return {
    id: 't-1',
    recordingId: 'r-1',
    language: 'ko',
    segments: [{ startMs: 134_000, endMs: 141_000, text: '그러면 이번에는 PLY 먼저 변환하고' }],
    rawText: '그러면 이번에는 PLY 먼저 변환하고',
    createdAt: '2026-09-03T04:50:26.000Z',
    engine: 'whisper.cpp',
    model: 'ggml-base.bin',
    ...overrides,
  };
}

/** provider를 고르지 않은 상태. **기본이자 정상 상태다** (INV-8). */
function noProvider(): AiProviderStatus {
  return {
    state: 'notConfigured',
    providerId: null,
    providerName: null,
    locality: null,
    models: [],
    failure: null,
  };
}

/** 고른 provider가 있지만 지금 쓸 수 없는 상태들. 벤더 이름은 provider가 말한 값이다 (INV-9). */
function brokenProvider(state: 'unavailable' | 'noModels'): AiProviderStatus {
  return {
    state,
    providerId: 'ollama',
    providerName: 'Ollama',
    locality: 'local',
    models: [],
    failure:
      state === 'unavailable'
        ? failure('aiProviderUnreachable', { message: 'AI 서버가 응답하지 않는다.' })
        : null,
  };
}

function readyProvider(): AiProviderStatus {
  return {
    state: 'ready',
    providerId: 'ollama',
    providerName: 'Ollama',
    locality: 'local',
    models: ['llama3.1:8b'],
    failure: null,
  };
}

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: 'AI provider에 닿지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

function liveNote(overrides: Partial<AiNoteStatus> = {}): AiNoteStatus {
  return {
    state: 'idle',
    recordingId: null,
    mode: null,
    aiNoteId: null,
    failure: null,
    ...overrides,
  };
}

const MEETING: MeetingNote = {
  mode: 'meeting',
  overview: '이번 주 변환 작업 범위를 정했다.',
  keyDiscussions: ['PLY와 SOG 변환 순서'],
  decisions: ['PLY를 먼저 변환한다'],
  actionItems: ['SOG 변환 결과 확인'],
  openQuestions: [],
};

function aiNote(): AiNote {
  return {
    id: 'n-1',
    recordingId: 'r-1',
    transcriptId: 't-1',
    mode: 'meeting',
    note: MEETING,
    provider: 'ollama',
    model: 'llama3.1:8b',
    promptVersion: 'meeting/2026-09-03',
    generatedAt: '2026-09-03T05:10:00.000Z',
  };
}

function noteInput(overrides: Partial<AiNoteInput> = {}): AiNoteInput {
  return {
    recording: recording(),
    transcript: transcript(),
    notes: [],
    provider: noProvider(),
    live: null,
    mode: 'meeting',
    ...overrides,
  };
}

/** 정지가 성공했을 때 오는 값 (R-002). AI와는 아무 상관이 없다. */
function stopped(): StoppedRecording {
  return {
    recording: recording(),
    capture: {
      deviceLabel: 'MacBook Microphone',
      outputPath: '/recordings/r-1.wav',
      format: '16 kHz · mono · 16-bit · WAV',
      sampleRateHz: 16_000,
      channels: 1,
      bitsPerSample: 16,
      container: 'WAV',
      byteSize: 100_832_044,
      durationMs: 3_151_000,
      durationLabel: '52:31',
    },
  };
}

/** 녹음 중인 session 하나. */
function liveSession(): SessionStatus {
  return { state: 'recording', elapsedMs: 7_000, elapsedLabel: '0:07' };
}

/**
 * 이 파일이 훑는 AI 상태 전부.
 *
 * 저장된 AI 후처리 상태 다섯 (§7) × 지금 벌어지고 있는 생성의 상태들이다. 이 중 어느 것도
 * 녹음 · 전사 · 열람의 화면 값을 바꾸어서는 안 된다.
 */
const EVERY_AI_STATUS: readonly ProcessingStatus[] = [
  'none',
  'pending',
  'running',
  'done',
  'failed',
];

/** provider가 놓일 수 있는 상태 전부. `ready`도 포함해 **네 상태 다** 훑는다. */
const EVERY_PROVIDER_STATUS: readonly AiProviderStatus[] = [
  noProvider(),
  brokenProvider('unavailable'),
  brokenProvider('noModels'),
  readyProvider(),
];

/** §13이 서로 다른 제품 상태로 구분하는 AI 생성 실패 셋. */
const THREE_GENERATION_FAILURES: readonly (readonly [string, FailureKind, boolean])[] = [
  ['미실행', 'aiProviderUnreachable', true],
  ['모델 없음', 'aiModelUnavailable', false],
  ['잘못된 응답', 'aiResponseUnusable', true],
];

// --- 1. 녹음 -------------------------------------------------------------------------

describe('녹음은 AI를 알지 않는다 (INV-8)', () => {
  it('녹음 화면의 상태에 AI가 들어올 자리가 없다', () => {
    // 이 화면이 AI를 보기 시작하면 그 순간 AI가 녹음을 막을 수 있게 된다. 그래서 상태의
    // 필드를 통째로 고정한다 — 새 필드가 하나 생기면 여기서 먼저 드러난다.
    expect(Object.keys(INITIAL_RECORDING).sort()).toEqual([
      'busy',
      'microphone',
      'saved',
      'session',
      'title',
      'trouble',
    ]);
  });

  it('provider가 하나도 없어도 시작과 정지를 누를 수 있다', () => {
    const ready = observedSession(
      {
        ...INITIAL_RECORDING,
        microphone: { kind: 'selected', deviceKey: '0:mic', label: 'MacBook Microphone', fromSystemDefault: false },
      },
      { state: 'idle', elapsedMs: 0, elapsedLabel: '0:00' },
    );
    expect(recordingControls(ready).record).toBe(true);

    const live = observedSession(ready, liveSession());
    expect(recordingControls(live).stop).toBe(true);
    expect(sessionDisplay(live).live).toBe(true);
    // 경과 시간은 backend가 만든 값 그대로다. AI 상태가 이 값에 끼어들 자리가 없다.
    expect(sessionDisplay(live).elapsedLabel).toBe('0:07');
  });

  it('정지가 성공하면 저장된 녹음이 그대로 화면에 온다 (R-002)', () => {
    const requested = requestedAction(INITIAL_RECORDING, 'stop');
    const saved = savedRecording(requested, stopped());

    expect(saved.saved).toEqual({ id: 'r-1', title: '3DGS Study #04', durationLabel: '52:31' });
    expect(saved.trouble).toBeNull();
    expect(saved.busy).toBe(false);
    // 저장 결과에 AI가 실려 있지 않다 — 정지의 성공이 AI에 걸려 있지 않다는 뜻이다.
    expect(Object.keys(saved.saved ?? {}).sort()).toEqual(['durationLabel', 'id', 'title']);
  });
});

// --- 2. 전사 -------------------------------------------------------------------------

describe('전사는 AI 상태에 흔들리지 않는다 (INV-8)', () => {
  it('저장된 AI 상태가 무엇이든 Transcript 탭의 값이 글자 하나 달라지지 않는다', () => {
    const stored = transcript();
    const baseline = transcriptTab(recording({ aiStatus: 'none' }), stored, null);
    expect(baseline.kind).toBe('done');

    for (const aiStatus of EVERY_AI_STATUS) {
      expect(transcriptTab(recording({ aiStatus }), stored, null), `aiStatus=${aiStatus}`).toEqual(
        baseline,
      );
    }
  });

  it('AI 노트 생성이 실패한 뒤에도 전사 탭은 그대로다', () => {
    const stored = transcript();
    const before = transcriptTab(recording(), stored, null);

    for (const [label, kind] of THREE_GENERATION_FAILURES) {
      const after = transcriptTab(recording({ aiStatus: 'failed' }), stored, null);
      expect(after, `${label}(${kind})`).toEqual(before);
    }
  });

  it('아직 전사하지 않은 녹음은 AI 상태와 무관하게 전사를 시작할 수 있다', () => {
    for (const aiStatus of EVERY_AI_STATUS) {
      const view = transcriptTab(
        recording({ aiStatus, currentTranscriptId: null, transcriptionStatus: 'none' }),
        null,
        null,
      );

      expect(view.kind, `aiStatus=${aiStatus}`).toBe('none');
      if (view.kind !== 'none') {
        throw new Error('전사한 적 없는 상태여야 한다');
      }
      expect(view.start.recordingId).toBe('r-1');
    }
  });

  it('전사의 진행과 실패는 전사 자신의 상태에서만 온다', () => {
    // AI를 아무리 나쁘게 만들어도 전사가 실패로 보이지 않고, 그 반대도 마찬가지다.
    const running = transcriptTab(
      recording({ aiStatus: 'failed', transcriptionStatus: 'running' }),
      null,
      null,
    );
    const failed = transcriptTab(
      recording({ aiStatus: 'done', transcriptionStatus: 'failed' }),
      null,
      null,
    );

    expect(running.kind).toBe('running');
    expect(failed.kind).toBe('failed');
  });
});

// --- 3. 열람 -------------------------------------------------------------------------

describe('열람은 AI 상태에 흔들리지 않는다 (INV-8)', () => {
  it('목록은 어떤 AI 상태에서도 그 녹음을 보여준다', () => {
    for (const aiStatus of EVERY_AI_STATUS) {
      const view = loadedRecordings([recording({ aiStatus })], { timeZone: 'UTC' });

      expect(view.kind, `aiStatus=${aiStatus}`).toBe('list');
      if (view.kind !== 'list') {
        throw new Error('목록이어야 한다');
      }
      expect(view.items).toHaveLength(1);
      expect(view.items[0].title).toBe('3DGS Study #04');
      expect(view.items[0].durationLabel).toBe('52:31');
      // AI는 세 뱃지 중 하나일 뿐이며, 목록 자체를 실패로 만들지 않는다.
      expect(view.items[0].statuses[1].label).toBe('AI Note');
      expect(view.items[0].statuses[1].status).toBe(aiStatus);
    }
  });

  it('아직 AI를 시도한 적이 없다는 것이 오류처럼 적히지 않는다 (§7 · INV-8)', () => {
    const view = loadedRecordings([recording({ aiStatus: 'none' })], { timeZone: 'UTC' });
    if (view.kind !== 'list') {
      throw new Error('목록이어야 한다');
    }

    expect(view.items[0].statuses[1].text).toBe('Not started');
    expect(view.items[0].statuses[1].text).not.toMatch(/fail|error/i);
  });

  it('상세와 재생 경로는 어떤 AI 상태에서도 열린다', () => {
    for (const aiStatus of EVERY_AI_STATUS) {
      const detail = loadedRecordingDetail(
        'r-1',
        recording({ aiStatus }),
        [],
        (audioPath) => `asset://${audioPath}`,
        { timeZone: 'UTC' },
      );

      expect(detail.kind, `aiStatus=${aiStatus}`).toBe('playable');
      if (detail.kind !== 'playable') {
        throw new Error('재생할 수 있어야 한다');
      }
      expect(detail.audioSource).toBe('asset:///recordings/r-1.wav');
      expect(detail.recording.durationLabel).toBe('52:31');
    }
  });

  it('AI 상태가 달라져도 상세 화면의 값이 글자 하나 달라지지 않는다', () => {
    const resolve = (audioPath: string) => `asset://${audioPath}`;
    const baseline = loadedRecordingDetail('r-1', recording({ aiStatus: 'none' }), [], resolve, {
      timeZone: 'UTC',
    });

    for (const aiStatus of EVERY_AI_STATUS) {
      expect(
        loadedRecordingDetail('r-1', recording({ aiStatus }), [], resolve, { timeZone: 'UTC' }),
        `aiStatus=${aiStatus}`,
      ).toEqual(baseline);
    }
  });
});

// --- 4. AI 자신은 담담한 상태다 (INV-8 · §13) ---------------------------------------------

describe('AI가 꺼져 있는 것은 오류가 아니다 (INV-8)', () => {
  it('provider를 고르지 않은 것은 실패가 아니라 상태다', () => {
    const view = aiNoteTab(noteInput({ provider: noProvider() }));

    expect(view.body.kind).toBe('disabled');
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.state).toBe('notConfigured');
    expect(view.body.notice.headline).toBe(AI_DISABLED_HEADLINE);
    // 화면이 실패로 그릴 재료가 아예 없다 — 그것이 이 타입의 목적이다.
    expect(view.body).not.toHaveProperty('failure');
    expect(view.provider).toBeNull();
  });

  it('세 비활성 상태 어디에서도 실패로 그려지지 않고, 무엇이 막히지 않는지 함께 말한다', () => {
    for (const provider of EVERY_PROVIDER_STATUS.filter((status) => status.state !== 'ready')) {
      const view = aiNoteTab(noteInput({ provider }));

      expect(view.body.kind, provider.state).toBe('disabled');
      if (view.body.kind !== 'disabled') {
        throw new Error('비활성 상태여야 한다');
      }
      expect(view.body.notice.unaffectedNotice).toBe(AI_UNAFFECTED_NOTICE);
      expect(view.body.notice.resolution.trim().length).toBeGreaterThan(0);
      expect(view.body).not.toHaveProperty('failure');
    }
  });

  it('AI가 꺼져 있는 그 순간에도 나머지 세 경로가 같은 사실 위에서 그대로 동작한다', () => {
    const record = recording();
    const stored = transcript();

    for (const provider of EVERY_PROVIDER_STATUS) {
      const ai = aiNoteTab(noteInput({ recording: record, transcript: stored, provider }));
      const tab = transcriptTab(record, stored, null);
      const list = loadedRecordings([record], { timeZone: 'UTC' });
      const detail = loadedRecordingDetail('r-1', record, [], (p) => `asset://${p}`, {
        timeZone: 'UTC',
      });

      expect(ai.body.kind, provider.state).toBe(provider.state === 'ready' ? 'none' : 'disabled');
      expect(tab.kind, provider.state).toBe('done');
      expect(list.kind, provider.state).toBe('list');
      expect(detail.kind, provider.state).toBe('playable');
    }
  });
});

// --- 5. 실패는 상태로 남고 다시 시도할 수 있다 (§13 · INV-3) -------------------------------

describe('AI 생성 실패 셋이 화면에 남는 방식 (§13)', () => {
  it('세 실패 각각이 §13의 세 질문에 답하고 재시도 수단을 남긴다', () => {
    for (const [label, kind, retryable] of THREE_GENERATION_FAILURES) {
      const view = aiNoteTab(
        noteInput({
          provider: readyProvider(),
          notes: [aiNote()],
          live: liveNote({
            state: 'failed',
            recordingId: 'r-1',
            mode: 'meeting',
            failure: failure(kind, { retryable }),
          }),
        }),
      );

      expect(view.body.kind, label).toBe('failed');
      if (view.body.kind !== 'failed') {
        throw new Error('실패 상태여야 한다');
      }

      // 1. 무엇이 실패했는가 — Rust가 나눠 보낸 구분이 뭉개지지 않는다.
      expect(view.body.headline, label).toBe(AI_NOTE_FAILED_HEADLINE);
      expect(view.body.failure?.kind, label).toBe(kind);

      // 2. 원본은 안전한가 — 그리고 이미 있던 노트는 그대로 보인다.
      expect(view.body.preservedNotice, label).toBe(AI_NOTE_PRESERVED_NOTICE);
      expect(view.body.kept?.provenance.model, label).toBe('llama3.1:8b');

      // 3. 다시 시도할 수 있는가 — `retryable`이 거짓인 갈래에서도 수단은 남는다.
      expect(view.body.retry.kind, label).toBe('retry');
      expect(view.body.retry.recordingId, label).toBe('r-1');
      expect(view.body.retry.mode, label).toBe('meeting');
      expect(view.modeSelectable, label).toBe(true);
    }
  });

  it('세 실패 어디에서도 녹음 · 전사 · 열람의 화면 값이 달라지지 않는다', () => {
    const stored = transcript();
    const before = {
      tab: transcriptTab(recording(), stored, null),
      list: loadedRecordings([recording()], { timeZone: 'UTC' }),
      detail: loadedRecordingDetail('r-1', recording(), [], (p) => `asset://${p}`, {
        timeZone: 'UTC',
      }),
    };

    for (const [label] of THREE_GENERATION_FAILURES) {
      // 실패는 저장된 AI 상태 하나만 바꾼다 (`ai::run`이 옮기는 것이 그것뿐이기 때문이다).
      const failed = recording({ aiStatus: 'failed' });

      expect(transcriptTab(failed, stored, null), label).toEqual(before.tab);
      expect(
        loadedRecordingDetail('r-1', failed, [], (p) => `asset://${p}`, { timeZone: 'UTC' }),
        label,
      ).toEqual(before.detail);

      const list = loadedRecordings([failed], { timeZone: 'UTC' });
      expect(list.kind, label).toBe(before.list.kind);
      if (list.kind !== 'list' || before.list.kind !== 'list') {
        throw new Error('목록이어야 한다');
      }
      expect(list.items[0].title, label).toBe(before.list.items[0].title);
      expect(list.items[0].durationLabel, label).toBe(before.list.items[0].durationLabel);
      // 달라진 것은 AI 뱃지 하나뿐이다.
      expect(list.items[0].statuses[0], label).toEqual(before.list.items[0].statuses[0]);
      expect(list.items[0].statuses[2], label).toEqual(before.list.items[0].statuses[2]);
      expect(list.items[0].statuses[1].status, label).toBe('failed');
    }
  });

  it('앱을 다시 켠 뒤 이유를 모르는 실패도 이미 있던 노트를 잃지 않는다', () => {
    // 저장된 것은 `failed`라는 사실뿐이다. 이유를 지어내지 않으며, 노트는 그대로 보인다.
    const view = aiNoteTab(
      noteInput({
        recording: recording({ aiStatus: 'failed' }),
        provider: readyProvider(),
        notes: [aiNote()],
        live: null,
      }),
    );

    expect(view.body.kind).toBe('failed');
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.failure).toBeNull();
    expect(view.body.cause).toBe('unknown');
    expect(view.body.kept?.provenance.model).toBe('llama3.1:8b');
    expect(view.body.retry.recordingId).toBe('r-1');
  });
});
