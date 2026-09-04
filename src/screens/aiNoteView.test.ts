// AI Note 탭 상태 변환 테스트.
//
// 일곱 경로를 전부 본다 — **아직 읽지 못함 · AI 비활성 · 전사 없음 · 노트 없음 · 생성 중 ·
// 완료 · 실패**. Ollama도 모델도 네트워크도 DOM도 필요하지 않다 (PRODUCT-SPEC §18 ·
// phase-prompt/04 요구 16).
//
// 이 테스트가 지키는 것이 셋 더 있다:
//   1. 렌더링이 `Structured Note → UI` 한 방향으로만 흐른다 (§9.3). 섹션의 이름과 순서는
//      §9.5의 표에서 오며, provider가 준 Markdown 문자열이 화면 값에 들어올 자리가 없다.
//   2. **provider 미설정·미연결이 실패로 그려지지 않는다** (INV-8 · §13) — 그 상태의 값에는
//      `failure`가 아예 없고, 같은 상황에서 Transcript 탭과 재생 경로는 그대로 동작한다.
//   3. 실패가 아무것도 잃지 않는다 (INV-1 · INV-2 · INV-3) — 이미 있던 노트는 실패 상태에서도
//      그대로 남고, 이 모듈에는 그것을 지우는 함수 자체가 없다.
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type {
  AiNote,
  AiNoteStatus,
  AiProviderStatus,
  MeetingNote,
  Recording,
  StudyNote,
  SummaryNote,
  Transcript,
} from '../ipc/types';
import {
  AI_DISABLED_HEADLINE,
  AI_NOTE_FAILED_HEADLINE,
  AI_NOTE_PRESERVED_NOTICE,
  AI_UNAFFECTED_NOTICE,
  EMPTY_SECTION_TEXT,
  NOTE_MODES,
  NO_AI_NOTE_TEXT,
  NO_TRANSCRIPT_INPUT_TEXT,
  UNKNOWN_AI_NOTE_NOTICE,
  aiNoteTab,
  aiNoteTrouble,
  latestNote,
  noteModeChoices,
  noteSections,
  noteView,
  type AiNoteInput,
} from './aiNoteView';
import { loadedRecordingDetail } from './recordingDetailView';
import { transcriptTab } from './transcriptView';

/** 저장소가 돌려주는 모양 그대로의 레코드. */
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

/** 고른 provider가 쓸 수 있는 상태. 벤더 이름은 provider 자신이 말한 값이다 (INV-9). */
function provider(overrides: Partial<AiProviderStatus> = {}): AiProviderStatus {
  return {
    state: 'ready',
    providerId: 'ollama',
    providerName: 'Ollama',
    locality: 'local',
    models: ['llama3.1:8b'],
    failure: null,
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

function liveStatus(overrides: Partial<AiNoteStatus> = {}): AiNoteStatus {
  return { state: 'idle', recordingId: null, mode: null, aiNoteId: null, failure: null, ...overrides };
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

const MEETING: MeetingNote = {
  mode: 'meeting',
  overview: '이번 주 변환 작업 범위를 정했다.',
  keyDiscussions: ['PLY와 SOG 변환 순서'],
  decisions: ['PLY를 먼저 변환한다'],
  actionItems: ['SOG 변환 결과 확인'],
  openQuestions: [],
};

const STUDY: StudyNote = {
  mode: 'study',
  overview: '3D Gaussian Splatting의 표현 방식을 정리했다.',
  keyConcepts: ['Gaussian primitive'],
  importantDetails: ['SH 계수는 색을 방향에 따라 표현한다'],
  questions: ['압축률과 화질의 균형은?'],
  thingsToStudy: ['SOG 포맷 명세'],
  referencesMentioned: [],
};

const SUMMARY: SummaryNote = {
  mode: 'summary',
  shortSummary: '변환 파이프라인 순서를 정하고 확인 항목을 나눴다.',
  keyPoints: ['PLY 먼저', 'SOG 확인'],
};

/** 저장된 노트 하나. provenance 다섯 값이 전부 있다 (§7.3). */
function aiNote(overrides: Partial<AiNote> = {}): AiNote {
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
    ...overrides,
  };
}

/** 기본 입력 — provider가 준비됐고 전사가 있고 노트는 아직 없다. */
function input(overrides: Partial<AiNoteInput> = {}): AiNoteInput {
  return {
    recording: recording(),
    transcript: transcript(),
    notes: [],
    provider: provider(),
    live: null,
    mode: 'meeting',
    ...overrides,
  };
}

describe('mode 선택 (§9.5)', () => {
  it('세 mode가 표시 순서대로 있다', () => {
    expect(NOTE_MODES).toEqual(['meeting', 'study', 'summary']);
    expect(noteModeChoices('meeting').map((choice) => choice.label)).toEqual([
      'Meeting',
      'Study',
      'Summary',
    ]);
  });

  it('고른 것 하나만 selected다', () => {
    const chosen = noteModeChoices('study');
    expect(chosen.filter((choice) => choice.selected).map((choice) => choice.mode)).toEqual([
      'study',
    ]);
  });

  it('고르기 전에 무엇이 나오는지 알 수 있다 (§9.5의 섹션)', () => {
    const [meeting, , summary] = noteModeChoices('meeting');
    expect(meeting.sections).toBe(
      'Overview · Key Discussions · Decisions · Action Items · Open Questions',
    );
    expect(summary.sections).toBe('Short Summary · Key Points');
  });

  it('세 mode를 골라 생성할 수 있다 — 고른 mode가 그대로 동작에 실린다', () => {
    for (const mode of NOTE_MODES) {
      const view = aiNoteTab(input({ mode }));
      expect(view.modeSelectable).toBe(true);
      expect(view.body.kind).toBe('none');
      if (view.body.kind !== 'none') {
        throw new Error('생성할 수 있는 상태여야 한다');
      }
      expect(view.body.generate.mode).toBe(mode);
      expect(view.body.generate.recordingId).toBe('r-1');
    }
  });
});

describe('Structured Note → UI (§9.3 · §9.5)', () => {
  it('Meeting은 §9.5의 다섯 섹션을 그 순서로 만든다', () => {
    expect(noteSections(MEETING).map((section) => section.title)).toEqual([
      'Overview',
      'Key Discussions',
      'Decisions',
      'Action Items',
      'Open Questions',
    ]);
  });

  it('Study는 여섯 섹션, Summary는 두 섹션이다', () => {
    expect(noteSections(STUDY).map((section) => section.title)).toEqual([
      'Overview',
      'Key Concepts',
      'Important Details',
      'Questions',
      'Things to Study',
      'References Mentioned',
    ]);
    expect(noteSections(SUMMARY).map((section) => section.title)).toEqual([
      'Short Summary',
      'Key Points',
    ]);
  });

  it('문단과 항목이 다른 종류로 남는다 — 화면이 문단을 목록처럼 그리지 않게', () => {
    const [overview, discussions] = noteSections(MEETING);
    expect(overview.kind).toBe('text');
    expect(overview.text).toBe('이번 주 변환 작업 범위를 정했다.');
    expect(overview.items).toEqual([]);

    expect(discussions.kind).toBe('list');
    expect(discussions.text).toBeNull();
    expect(discussions.items).toEqual(['PLY와 SOG 변환 순서']);
  });

  it('빈 섹션은 실패가 아니라 비었다고 말한다 (ADR-0008 §7.3)', () => {
    const openQuestions = noteSections(MEETING)[4];
    expect(openQuestions.items).toEqual([]);
    expect(openQuestions.emptyText).toBe(EMPTY_SECTION_TEXT);
    // 비었다는 사실이지 실패가 아니다.
    expect(openQuestions).not.toHaveProperty('failure');
  });

  it('내용이 있는 섹션에는 emptyText가 없다', () => {
    expect(noteSections(SUMMARY)[1].emptyText).toBeNull();
  });

  it('provider가 준 문자열 한 덩어리가 화면 값에 남지 않는다', () => {
    // 화면이 받는 것은 섹션의 목록이며, Markdown 본문 같은 단일 문자열 필드가 없다.
    const view = noteView(aiNote());
    expect(view.sections).toHaveLength(5);
    expect(view).not.toHaveProperty('content');
    expect(view).not.toHaveProperty('markdown');
    expect(view).not.toHaveProperty('rawText');
  });
});

describe('provenance (§7.3 · 요구 11 · 13)', () => {
  it('다섯 값이 전부 화면 값에 있다', () => {
    const { provenance } = noteView(aiNote());
    expect(provenance.provider).toBe('ollama');
    expect(provenance.model).toBe('llama3.1:8b');
    expect(provenance.promptVersion).toBe('meeting/2026-09-03');
    expect(provenance.generatedAt).toBe('2026-09-03T05:10:00.000Z');
    expect(provenance.transcriptId).toBe('t-1');
  });

  it('한 줄 표시가 다섯 값을 그대로 담는다 — 시각을 새로 만들지 않는다', () => {
    const { provenance } = noteView(aiNote());
    for (const fact of ['ollama', 'llama3.1:8b', 'meeting/2026-09-03', 't-1']) {
      expect(provenance.label).toContain(fact);
    }
    // 저장된 ISO 텍스트 그대로다. 표기 규칙이 두 벌이 되지 않는다.
    expect(provenance.label).toContain('2026-09-03T05:10:00.000Z');
  });

  it('생성된 노트를 볼 때 provenance가 함께 온다', () => {
    const view = aiNoteTab(input({ notes: [aiNote()] }));
    if (view.body.kind !== 'ready') {
      throw new Error('노트를 볼 수 있는 상태여야 한다');
    }
    expect(view.body.note.provenance.model).toBe('llama3.1:8b');
    expect(view.body.note.sections[0].title).toBe('Overview');
  });
});

describe('가장 최근 노트 고르기 (ADR-0008 §9.2)', () => {
  const older = aiNote({ id: 'n-1', generatedAt: '2026-09-03T05:10:00.000Z' });
  const newer = aiNote({ id: 'n-2', generatedAt: '2026-09-03T06:00:00.000Z' });

  it('같은 mode의 노트가 여럿이면 마지막 것이다 — 저장된 순서가 곧 만들어진 순서다', () => {
    expect(latestNote([older, newer], 't-1', 'meeting')?.id).toBe('n-2');
  });

  it('다른 mode의 노트를 대신 보여주지 않는다', () => {
    expect(latestNote([older, newer], 't-1', 'study')).toBeNull();
  });

  it('다른 Transcript version의 노트를 이 Transcript의 것으로 보여주지 않는다 (§7.3)', () => {
    const otherVersion = aiNote({ id: 'n-3', transcriptId: 't-2' });
    expect(latestNote([older, otherVersion], 't-1', 'meeting')?.id).toBe('n-1');
  });

  it('재생성해도 이전 노트가 목록에서 사라지지 않는다 (INV-2)', () => {
    const notes = [older, newer];
    aiNoteTab(input({ notes }));
    expect(notes).toHaveLength(2);
    expect(notes[0]).toBe(older);
  });
});

describe('아직 읽지 못한 상태', () => {
  it('레코드를 읽기 전에는 판단하지 않는다', () => {
    expect(aiNoteTab(input({ recording: null })).body.kind).toBe('loading');
  });

  it('provider 상태를 묻기 전에는 판단하지 않는다 — idle로 접지 않는다', () => {
    expect(aiNoteTab(input({ provider: null })).body.kind).toBe('loading');
  });

  it('레코드가 가리키는 Transcript를 아직 읽지 못했으면 loading이다', () => {
    expect(aiNoteTab(input({ transcript: null })).body.kind).toBe('loading');
  });

  it('저장된 노트를 아직 읽지 못했으면 loading이다 — "노트 없음"과 다른 사실이다', () => {
    expect(aiNoteTab(input({ notes: null })).body.kind).toBe('loading');
  });
});

describe('AI 기능이 비활성인 상태 (INV-8 · §13)', () => {
  it('provider 미설정은 오류가 아니라 담담한 상태다', () => {
    const view = aiNoteTab(input({ provider: noProvider() }));
    expect(view.body.kind).toBe('disabled');
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.state).toBe('notConfigured');
    expect(view.body.notice.headline).toBe(AI_DISABLED_HEADLINE);
    // 실패로 그릴 재료 자체를 주지 않는다.
    expect(view.body).not.toHaveProperty('failure');
    expect(view.body.notice).not.toHaveProperty('failure');
  });

  it('연결되지 않은 것도 실패가 아니라 비활성이다', () => {
    const view = aiNoteTab(
      input({
        provider: provider({ state: 'unavailable', models: [], failure: failure('aiProviderUnreachable') }),
      }),
    );
    expect(view.body.kind).toBe('disabled');
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.state).toBe('unavailable');
    // provider가 준 failure를 화면 상태로 옮기지 않는다 — 이것은 실패 표현이 아니다.
    expect(view.body).not.toHaveProperty('failure');
    expect(view.body.notice.providerName).toBe('Ollama');
  });

  it('모델이 하나도 없는 것도 비활성이다 (요구 8)', () => {
    const view = aiNoteTab(input({ provider: provider({ state: 'noModels', models: [] }) }));
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.state).toBe('noModels');
    expect(view.body.notice.resolution).toContain('Install a model');
  });

  it('어떤 provider 상태에서도 실패로 그려지지 않는다', () => {
    for (const state of ['notConfigured', 'unavailable', 'noModels'] as const) {
      const view = aiNoteTab(input({ provider: provider({ state, models: [] }) }));
      expect(view.body.kind).toBe('disabled');
      expect(view.body.kind).not.toBe('failed');
    }
  });

  it('무엇이 막히지 않는지 함께 말한다 (INV-8)', () => {
    const view = aiNoteTab(input({ provider: noProvider() }));
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.unaffectedNotice).toBe(AI_UNAFFECTED_NOTICE);
    expect(view.body.notice.resolution).toContain('Settings');
  });

  it('고른 provider가 없으면 다시 확인할 대상도 없다', () => {
    const view = aiNoteTab(input({ provider: noProvider() }));
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.recheck).toBeNull();
  });

  it('닿지 못하는 provider는 다시 확인해 볼 수 있다', () => {
    const view = aiNoteTab(input({ provider: provider({ state: 'unavailable', models: [] }) }));
    if (view.body.kind !== 'disabled') {
      throw new Error('비활성 상태여야 한다');
    }
    expect(view.body.notice.recheck?.label).toContain('Check');
  });

  it('비활성 상태에서 생성을 요청했다가 받은 답도 실패로 그리지 않는다', () => {
    // provider를 고르지 않은 채 생성을 걸면 `aiProviderNotConfigured`가 상태로 돌아온다.
    // 그것은 command의 실패가 아니라 "AI가 꺼져 있다"는 사실이다 (INV-8 · ADR-0008 §13.2).
    const view = aiNoteTab(
      input({
        provider: provider(),
        live: liveStatus({
          state: 'failed',
          recordingId: 'r-1',
          mode: 'meeting',
          failure: failure('aiProviderNotConfigured', { retryable: false }),
        }),
      }),
    );
    expect(view.body.kind).toBe('disabled');
  });

  it('AI가 꺼져 있어도 Transcript 탭과 재생은 그대로다 (INV-8)', () => {
    const record = recording();
    const stored = transcript();

    // 같은 사실 위에서 AI 탭은 비활성이고,
    expect(aiNoteTab(input({ recording: record, transcript: stored, provider: noProvider() })).body.kind).toBe(
      'disabled',
    );

    // Transcript 탭은 전사된 문장을 그대로 보여주며,
    const transcriptView = transcriptTab(record, stored, null);
    expect(transcriptView.kind).toBe('done');

    // 재생 경로도 그대로 열린다.
    const detail = loadedRecordingDetail(record.id, record, [], (path) => `asset://${path}`);
    expect(detail.kind).toBe('playable');
  });
});

describe('만들 재료가 없는 상태 (§7.2)', () => {
  it('전사가 없는 것은 실패가 아니다', () => {
    const view = aiNoteTab(
      input({ recording: recording({ currentTranscriptId: null }), transcript: null }),
    );
    expect(view.body.kind).toBe('noTranscript');
    if (view.body.kind !== 'noTranscript') {
      throw new Error('재료가 없는 상태여야 한다');
    }
    expect(view.body.text).toBe(NO_TRANSCRIPT_INPUT_TEXT);
    expect(view.body.hint).toContain('Transcript tab');
    expect(view.body).not.toHaveProperty('failure');
  });

  it('생성을 걸었더니 재료가 없다고 답한 것도 실패가 아니다', () => {
    const view = aiNoteTab(
      input({
        live: liveStatus({ state: 'noTranscript', recordingId: 'r-1', mode: 'meeting' }),
      }),
    );
    expect(view.body.kind).toBe('noTranscript');
  });
});

describe('아직 노트가 없는 상태', () => {
  it('만든 적이 없다는 사실과 만드는 수단이 함께 있다', () => {
    const view = aiNoteTab(input());
    if (view.body.kind !== 'none') {
      throw new Error('노트가 없는 상태여야 한다');
    }
    expect(view.body.text).toBe(NO_AI_NOTE_TEXT);
    expect(view.body.generate.kind).toBe('generate');
    expect(view.body.generate.label).toContain('Meeting');
  });

  it('다른 mode의 노트가 있어도 고른 mode의 노트가 없으면 만들 수 있다', () => {
    const view = aiNoteTab(input({ notes: [aiNote()], mode: 'study' }));
    expect(view.body.kind).toBe('none');
  });
});

describe('생성 중인 상태', () => {
  it('지금 이 녹음의 생성이 돌고 있으면 그것이 먼저다', () => {
    const view = aiNoteTab(
      input({ live: liveStatus({ state: 'running', recordingId: 'r-1', mode: 'meeting' }) }),
    );
    expect(view.body.kind).toBe('generating');
    // 도는 동안에는 mode를 바꾸지 않는다.
    expect(view.modeSelectable).toBe(false);
  });

  it('다른 녹음의 생성은 이 화면과 상관이 없다', () => {
    const view = aiNoteTab(
      input({ live: liveStatus({ state: 'running', recordingId: 'r-9', mode: 'meeting' }) }),
    );
    expect(view.body.kind).toBe('none');
  });

  it('저장된 상태가 pending·running이어도 생성 중이다 — 앱을 다시 켠 뒤에도 남는 사실이다', () => {
    for (const aiStatus of ['pending', 'running'] as const) {
      const view = aiNoteTab(input({ recording: recording({ aiStatus }) }));
      expect(view.body.kind).toBe('generating');
    }
  });

  it('새로 만드는 동안에도 이미 있던 노트가 그대로 보인다 (INV-2)', () => {
    const view = aiNoteTab(
      input({
        notes: [aiNote()],
        live: liveStatus({ state: 'running', recordingId: 'r-1', mode: 'meeting' }),
      }),
    );
    if (view.body.kind !== 'generating') {
      throw new Error('생성 중이어야 한다');
    }
    expect(view.body.kept?.provenance.promptVersion).toBe('meeting/2026-09-03');
  });
});

describe('실패한 상태 (§13)', () => {
  function failed(kind: FailureKind, overrides: Partial<Failure> = {}) {
    return aiNoteTab(
      input({
        notes: [aiNote()],
        live: liveStatus({
          state: 'failed',
          recordingId: 'r-1',
          mode: 'meeting',
          failure: failure(kind, overrides),
        }),
      }),
    );
  }

  it('세 질문에 답한다 — 무엇이 실패했는가 · 원본은 안전한가 · 다시 시도할 수 있는가', () => {
    const view = failed('aiRequestFailed');
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.headline).toBe(AI_NOTE_FAILED_HEADLINE);
    expect(view.body.failure?.kind).toBe('aiRequestFailed');
    expect(view.body.preservedNotice).toBe(AI_NOTE_PRESERVED_NOTICE);
    expect(view.body.retry.kind).toBe('retry');
    expect(view.body.retry.recordingId).toBe('r-1');
    expect(view.body.retry.mode).toBe('meeting');
  });

  it('원본이 그대로라는 사실을 말한다 — 지웠다고도 복구했다고도 하지 않는다', () => {
    const view = failed('aiResponseUnusable');
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.preservedNotice).toContain('untouched');
    expect(view.body.preservedNotice).toContain('Nothing was deleted');
  });

  it('실패해도 이미 있던 노트를 잃지 않는다 (INV-2 · INV-3)', () => {
    const view = failed('aiRequestFailed');
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.kept?.sections).toHaveLength(5);
  });

  it('갈래마다 먼저 할 일이 다르다', () => {
    const cases = [
      ['aiModelUnavailable', 'modelUnavailable', 'Install it or choose another model'],
      ['aiResponseUnusable', 'responseUnusable', 'Generating again'],
      ['aiInputTooLarge', 'inputTooLarge', 'nothing was sent'],
      ['aiProviderUnreachable', 'unreachable', 'Start it'],
    ] as const;

    for (const [kind, cause, resolution] of cases) {
      // provider 상태는 준비됐는데 그 시도만 실패한 경우다.
      const view = failed(kind);
      if (view.body.kind !== 'failed') {
        throw new Error('실패 상태여야 한다');
      }
      expect(view.body.cause).toBe(cause);
      expect(view.body.resolution).toContain(resolution);
    }
  });

  it('다시 시도할 수 없는 갈래에서도 재시도 수단은 남는다', () => {
    // retryable=false는 "지금 그대로 다시 눌러도 같다"는 뜻이지 "영영 만들 수 없다"가 아니다.
    const view = failed('aiModelUnavailable', { retryable: false });
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.failure?.retryable).toBe(false);
    expect(view.body.retry.label).toContain('Meeting');
  });

  it('저장된 상태만 failed면 이유를 지어내지 않는다', () => {
    const view = aiNoteTab(input({ recording: recording({ aiStatus: 'failed' }) }));
    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.failure).toBeNull();
    expect(view.body.cause).toBe('unknown');
    expect(view.body.resolution).toBe(UNKNOWN_AI_NOTE_NOTICE);
    // 이유를 모르는 것과 다시 시도할 수 없는 것은 다르다.
    expect(view.body.retry.kind).toBe('retry');
  });

  it('실패한 뒤에도 mode를 바꿔 다시 만들 수 있다', () => {
    expect(failed('aiRequestFailed').modeSelectable).toBe(true);
  });
});

describe('노트를 볼 수 있는 상태', () => {
  it('구조 그대로 보이고 다시 만들 수단이 있다 (요구 12)', () => {
    const view = aiNoteTab(input({ notes: [aiNote()] }));
    if (view.body.kind !== 'ready') {
      throw new Error('노트를 볼 수 있는 상태여야 한다');
    }
    expect(view.body.note.modeLabel).toBe('Meeting');
    expect(view.body.note.sections.map((section) => section.title)).toContain('Action Items');
    expect(view.body.regenerate.kind).toBe('regenerate');
    expect(view.body.regenerate.mode).toBe('meeting');
  });

  it('mode를 바꾸면 그 mode의 노트를 본다', () => {
    const notes = [aiNote(), aiNote({ id: 'n-2', mode: 'summary', note: SUMMARY })];
    const view = aiNoteTab(input({ notes, mode: 'summary' }));
    if (view.body.kind !== 'ready') {
      throw new Error('노트를 볼 수 있는 상태여야 한다');
    }
    expect(view.body.note.sections.map((section) => section.title)).toEqual([
      'Short Summary',
      'Key Points',
    ]);
  });
});

describe('전사가 어디로 가는가 (§12 · INV-5)', () => {
  it('로컬 provider는 기기를 떠나지 않는다고 말한다', () => {
    const view = aiNoteTab(input());
    expect(view.provider?.locality).toBe('local');
    expect(view.provider?.label).toContain('does not leave it');
    expect(view.provider?.label).toContain('Audio is never sent');
  });

  it('외부 provider는 전사가 나간다고 말한다 — audio는 어느 쪽이든 나가지 않는다', () => {
    const view = aiNoteTab(
      input({ provider: provider({ providerName: 'Some Cloud', locality: 'external' }) }),
    );
    expect(view.provider?.label).toContain('is sent to it');
    expect(view.provider?.label).toContain('Audio is never sent');
  });

  it('고른 provider가 없으면 할 말이 없다', () => {
    expect(aiNoteTab(input({ provider: noProvider() })).provider).toBeNull();
  });
});

describe('거절된 요청 (§13)', () => {
  it('어느 요청이 거절됐는지 구분해서 말한다', () => {
    expect(aiNoteTrouble('provider', 'boom').headline).toContain('AI provider status');
    expect(aiNoteTrouble('status', 'boom').headline).toContain('AI note status');
    expect(aiNoteTrouble('notes', 'boom').headline).toContain('saved AI notes');
    expect(aiNoteTrouble('start', 'boom').headline).toContain('could not be started');
  });

  it('구조화되지 않은 값도 화면에 보여줄 수 있는 실패가 된다', () => {
    const trouble = aiNoteTrouble('start', new Error('IPC 통로가 없다'));
    expect(trouble.failure.kind).toBe('unexpected');
    expect(trouble.failure.detail).toBe('IPC 통로가 없다');
  });

  it('Rust가 보낸 실패는 그대로 쓴다', () => {
    const original = failure('aiRequestFailed');
    expect(aiNoteTrouble('start', original).failure).toBe(original);
  });
});
