// Export Markdown 자리의 상태 (PRODUCT-SPEC §11 · §13 · `phase-prompt/05` 요구 A-1~3 · P-3).
//
// 파일시스템도 Rust도 DOM도 없이 여섯 경로를 판정한다 (§18).
//
// ## P-3이 약화되면 안 된다고 못박은 세 가지가 이 파일에 있다
//
//   1. AI provider 없이 Markdown export가 된다      (INV-8)
//   2. AI Note 없이 Markdown export가 된다          (INV-8 · §17.1)
//   3. export 실패가 원본 데이터를 훼손하지 않는다   (INV-3)
//
// 셋 다 `describe('P-3 …')` 아래에 모여 있다. 판정 방식이 요지다 — 첫째는 **provider를 담을
// 자리 자체가 없다**는 것을 입력의 필드와 모듈 원문으로 확인하고, 둘째는 노트가 하나도 없는
// 입력에서 내보내기 동작이 **글자 하나 다르지 않다**는 것을 깊은 비교로 확인하며, 셋째는
// 실패 상태가 §13의 세 질문에 답하면서 다른 화면 값이 그대로라는 것을 확인한다.
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type { AiNote, ExportedFile, MeetingNote, ProcessingStatus, Recording } from '../ipc/types';
import {
  EXPORT_AUDIO_NOTICE,
  EXPORT_CONTENTS_HEADLINE,
  EXPORT_DONE_HEADLINE,
  EXPORT_FAILED_HEADLINE,
  EXPORT_LABEL,
  EXPORT_PRESERVED_NOTICE,
  EXPORTING_TEXT,
  NO_EXPORT_ATTEMPT,
  NOTHING_TO_EXPORT_HINT,
  exportPanel,
  exportedFile,
  failedExport,
  type ExportAttempt,
  type ExportPanelInput,
} from './exportView';
import { loadedRecordingDetail } from './recordingDetailView';
import { loadedRecordings } from './recordingsView';
import { transcriptTab } from './transcriptView';

// --- 사실들 --------------------------------------------------------------------------

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

const MEETING: MeetingNote = {
  mode: 'meeting',
  overview: '이번 주 변환 작업 범위를 정했다.',
  keyDiscussions: ['PLY와 SOG 변환 순서'],
  decisions: ['PLY를 먼저 변환한다'],
  actionItems: [],
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

function file(overrides: Partial<ExportedFile> = {}): ExportedFile {
  return {
    recordingId: 'r-1',
    path: '/Users/someone/Library/Application Support/molt-note/exports/2026-09-01-3dgs-study-04.md',
    fileName: '2026-09-01-3dgs-study-04.md',
    ...overrides,
  };
}

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: '내보내지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

function input(overrides: Partial<ExportPanelInput> = {}): ExportPanelInput {
  return {
    recording: recording(),
    notes: [],
    attempt: NO_EXPORT_ATTEMPT,
    ...overrides,
  };
}

/** 저장된 후처리 상태 다섯 (§7). 이 중 어느 것도 내보내기를 막지 않는다. */
const EVERY_STATUS: readonly ProcessingStatus[] = ['none', 'pending', 'running', 'done', 'failed'];

// --- 상태 여섯 갈래 -------------------------------------------------------------------

describe('내보내기 자리가 놓이는 상태', () => {
  it('레코드를 아직 읽지 못했으면 판단하지 않는다', () => {
    const view = exportPanel(input({ recording: null }));

    expect(view.body.kind).toBe('loading');
    // 무엇이 파일에 들어가는지는 그때도 말할 수 있다.
    expect(view.contents.headline).toBe(EXPORT_CONTENTS_HEADLINE);
  });

  it('전사가 아직 없는 것은 실패가 아니라 상태다 (§7.2)', () => {
    const view = exportPanel(
      input({ recording: recording({ currentTranscriptId: null, transcriptionStatus: 'none' }) }),
    );

    expect(view.body.kind).toBe('nothingToExport');
    if (view.body.kind !== 'nothingToExport') {
      throw new Error('내보낼 것이 없는 상태여야 한다');
    }
    expect(view.body.hint).toBe(NOTHING_TO_EXPORT_HINT);
    // 화면이 실패로 그릴 재료가 없다.
    expect(view.body).not.toHaveProperty('failure');
    expect(view.body.text).not.toMatch(/fail|error/i);
  });

  it('전사가 있으면 그 자리에서 내보낼 수 있다', () => {
    const view = exportPanel(input());

    expect(view.body.kind).toBe('ready');
    if (view.body.kind !== 'ready') {
      throw new Error('내보낼 수 있어야 한다');
    }
    expect(view.body.start).toEqual({ kind: 'export', label: EXPORT_LABEL, recordingId: 'r-1' });
  });

  it('쓰는 중에는 그 사실만 말한다', () => {
    const view = exportPanel(input({ attempt: { kind: 'running', recordingId: 'r-1' } }));

    expect(view.body.kind).toBe('exporting');
    if (view.body.kind !== 'exporting') {
      throw new Error('쓰는 중이어야 한다');
    }
    expect(view.body.text).toBe(EXPORTING_TEXT);
  });

  it('성공하면 쓰인 파일의 전체 경로와 실제 이름을 그대로 알린다 (§4.1 · §4.3)', () => {
    const written = file({ fileName: '2026-09-01-3dgs-study-04-2.md' });
    const view = exportPanel(input({ attempt: exportedFile(written) }));

    expect(view.body.kind).toBe('done');
    if (view.body.kind !== 'done') {
      throw new Error('만들어진 파일이 있어야 한다');
    }
    expect(view.body.file.headline).toBe(EXPORT_DONE_HEADLINE);
    // 경로는 backend가 준 값 그대로다 — 화면이 잘라 짐작하지 않는다.
    expect(view.body.file.path).toBe(written.path);
    // 같은 이름이 있어서 번호가 붙었다면 그 이름이 그대로 보인다 (§4.3).
    expect(view.body.file.fileName).toBe('2026-09-01-3dgs-study-04-2.md');
    // 또 내보낼 수 있고, 그것이 있던 파일을 덮어쓰지 않는다는 사실도 함께 있다.
    expect(view.body.again.kind).toBe('again');
    expect(view.body.text).toMatch(/never overwrites/i);
  });

  it('다른 녹음의 결과가 이 자리에 보이지 않는다', () => {
    const other = exportedFile(file({ recordingId: 'r-2' }));

    expect(exportPanel(input({ attempt: other })).body.kind).toBe('ready');
    expect(
      exportPanel(input({ attempt: { kind: 'running', recordingId: 'r-2' } })).body.kind,
    ).toBe('ready');
    expect(exportPanel(input({ attempt: failedExport('r-2', failure('storage')) })).body.kind).toBe(
      'ready',
    );
  });
});

// --- 무엇이 파일에 들어가는가 (§11 · INV-6) ---------------------------------------------

describe('무엇이 파일에 들어가는지 말한다', () => {
  it('언제나 담기는 것과, 오디오는 복사되지 않는다는 사실이 함께 있다', () => {
    const view = exportPanel(input());

    expect(view.contents.items.length).toBeGreaterThan(0);
    expect(view.contents.audioNotice).toBe(EXPORT_AUDIO_NOTICE);
    expect(view.contents.audioNotice).toMatch(/audio file is not copied/i);
    expect(view.contents.audioNotice).toMatch(/nothing is sent anywhere/i);
  });

  it('노트가 들어가는지를 있는 그대로 말한다', () => {
    expect(exportPanel(input({ notes: [aiNote()] })).contents.note).toBe('included');
    expect(exportPanel(input({ notes: [] })).contents.note).toBe('none');
    // 아직 읽지 못한 것은 없는 것과 다른 사실이다.
    expect(exportPanel(input({ notes: null })).contents.note).toBe('unknown');
  });

  it('본문이 어느 상태든 그 사실은 남는다', () => {
    const attempts: readonly ExportAttempt[] = [
      NO_EXPORT_ATTEMPT,
      { kind: 'running', recordingId: 'r-1' },
      exportedFile(file()),
      failedExport('r-1', failure('storage')),
    ];

    for (const attempt of attempts) {
      expect(exportPanel(input({ attempt })).contents.audioNotice, attempt.kind).toBe(
        EXPORT_AUDIO_NOTICE,
      );
    }
  });
});

// --- P-3: 약화하면 안 되는 세 가지 -------------------------------------------------------

describe('P-3 (1) AI provider 없이 Markdown export가 된다 (INV-8)', () => {
  it('이 자리의 입력에 AI provider를 담을 자리가 없다', () => {
    // 자리가 생기는 순간 provider 하나 때문에 내보내기가 막힐 수 있게 된다. 그래서 입력의
    // 필드를 통째로 고정한다 — 새 필드가 하나 생기면 여기서 먼저 드러난다.
    expect(Object.keys(input()).sort()).toEqual(['attempt', 'notes', 'recording']);
  });

  // 값의 모양만 고정하면 "모듈 안에서 몰래 본다"는 경로가 남는다. 그래서 **모듈 원문에
  // provider가 등장하지 않는다**는 것도 함께 못박혀 있다 — 그 검사는 원문을 읽는 다른 검사들과
  // 같은 자리에 있다 (`tests/screen-boundary.test.ts`). 이 파일은 node:fs를 쓰지 않는다:
  // `src/`는 브라우저 코드로 타입 검사되며 node 타입이 없다.

  it('AI 상태가 무엇이든 내보내기 동작이 글자 하나 달라지지 않는다', () => {
    const baseline = exportPanel(input({ recording: recording({ aiStatus: 'none' }) }));

    for (const aiStatus of EVERY_STATUS) {
      const view = exportPanel(input({ recording: recording({ aiStatus }) }));

      expect(view.body.kind, `aiStatus=${aiStatus}`).toBe('ready');
      expect(view, `aiStatus=${aiStatus}`).toEqual(baseline);
    }
  });
});

describe('P-3 (2) AI Note 없이 Markdown export가 된다 (INV-8 · §17.1)', () => {
  it('노트가 하나도 없어도 내보낼 수 있다', () => {
    const view = exportPanel(input({ notes: [] }));

    expect(view.body.kind).toBe('ready');
    if (view.body.kind !== 'ready') {
      throw new Error('내보낼 수 있어야 한다');
    }
    expect(view.body.start.recordingId).toBe('r-1');
  });

  it('노트가 있든 없든 내보내기 동작이 같다 — 노트는 문서의 내용일 뿐이다', () => {
    const withNote = exportPanel(input({ notes: [aiNote()] }));
    const without = exportPanel(input({ notes: [] }));
    const unknown = exportPanel(input({ notes: null }));

    expect(without.body).toEqual(withNote.body);
    expect(unknown.body).toEqual(withNote.body);
  });

  it('노트가 없는 것이 결함처럼 적히지 않는다 — 그것도 완결된 문서다', () => {
    const notice = exportPanel(input({ notes: [] })).contents;

    expect(notice.noteText).toMatch(/complete document/i);
    expect(notice.noteText).not.toMatch(/fail|error|cannot/i);
  });

  it('노트도 전사도 없는 녹음은 실패가 아니라 "아직 없다"로 남는다', () => {
    const view = exportPanel(
      input({
        recording: recording({ currentTranscriptId: null, transcriptionStatus: 'none' }),
        notes: [],
      }),
    );

    expect(view.body.kind).toBe('nothingToExport');
  });
});

describe('P-3 (3) export 실패가 원본 데이터를 훼손하지 않는다 (INV-3 · §13)', () => {
  /** export가 만날 수 있는 실패들. 어느 것도 저장소를 고치지 않는다. */
  const FAILURES: readonly (readonly [string, FailureKind])[] = [
    ['내보낼 것이 없다', 'invalidInput'],
    ['파일을 쓰지 못했다', 'storage'],
    ['예상하지 못한 실패', 'unexpected'],
  ];

  it('세 실패 각각이 §13의 세 질문에 답하고 재시도 수단을 남긴다', () => {
    for (const [label, kind] of FAILURES) {
      const view = exportPanel(input({ attempt: failedExport('r-1', failure(kind)) }));

      expect(view.body.kind, label).toBe('failed');
      if (view.body.kind !== 'failed') {
        throw new Error('실패 상태여야 한다');
      }

      // 1. 무엇이 실패했는가 — Rust가 나눠 보낸 구분이 뭉개지지 않는다.
      expect(view.body.headline, label).toBe(EXPORT_FAILED_HEADLINE);
      expect(view.body.failure.kind, label).toBe(kind);

      // 2. 원본은 안전한가 — 지워진 것도 바뀐 것도 없다.
      expect(view.body.preservedNotice, label).toBe(EXPORT_PRESERVED_NOTICE);
      expect(view.body.preservedNotice, label).toMatch(/untouched/i);
      expect(view.body.preservedNotice, label).toMatch(/Nothing was deleted or changed/i);
      expect(view.body.failure.sourceDataSafe, label).toBe(true);

      // 3. 다시 시도할 수 있는가.
      expect(view.body.retry.kind, label).toBe('retry');
      expect(view.body.retry.recordingId, label).toBe('r-1');
    }
  });

  it('다시 시도해도 같은 실패에서도 재시도 수단은 남는다', () => {
    // `retryable`이 거짓인 것은 "지금 그대로 다시 눌러도 같다"는 뜻이지 "이 녹음은 영영
    // 내보낼 수 없다"는 뜻이 아니다. 무엇을 먼저 해야 하는지는 resolution이 말한다.
    const view = exportPanel(
      input({ attempt: failedExport('r-1', failure('invalidInput', { retryable: false })) }),
    );

    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.retry.recordingId).toBe('r-1');
    expect(view.body.resolution).toBe(NOTHING_TO_EXPORT_HINT);
  });

  it('실패한 뒤에도 녹음 · 전사 · 열람의 화면 값이 달라지지 않는다', () => {
    // export는 읽기만 하므로 실패해도 레코드가 바뀌지 않는다 (`export::run`). 그래서 같은
    // 레코드로 만든 다른 화면 값들이 실패 전후로 **같은 값**이어야 한다.
    const record = recording();
    const resolve = (audioPath: string) => `asset://${audioPath}`;
    const before = {
      detail: loadedRecordingDetail('r-1', record, [], resolve, { timeZone: 'UTC' }),
      list: loadedRecordings([record], { timeZone: 'UTC' }),
      transcript: transcriptTab(record, null, null),
    };

    for (const [label, kind] of FAILURES) {
      const view = exportPanel(input({ recording: record, attempt: failedExport('r-1', failure(kind)) }));
      expect(view.body.kind, label).toBe('failed');

      expect(loadedRecordingDetail('r-1', record, [], resolve, { timeZone: 'UTC' }), label).toEqual(
        before.detail,
      );
      expect(loadedRecordings([record], { timeZone: 'UTC' }), label).toEqual(before.list);
      expect(transcriptTab(record, null, null), label).toEqual(before.transcript);
    }
  });

  it('실패가 오디오 파일을 건드렸다고 말하지 않는다 (INV-1 · INV-6)', () => {
    const view = exportPanel(input({ attempt: failedExport('r-1', failure('storage')) }));

    if (view.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.body.preservedNotice).toMatch(/audio file/i);
    // "복구했다"거나 "정리했다"고 말하지 않는다.
    expect(view.body.preservedNotice).not.toMatch(/recovered|cleaned/i);
  });
});
