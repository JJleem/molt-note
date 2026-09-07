// AI Note 탭의 **두 줄**과 Export for AI 자리 (`phase-prompt/05.5` 요구 3 · 6 ·
// MH-1 · MH-2 · MH-3 · MH-7 · MH-8 · PRODUCT-SPEC §13 · INV-8).
//
// **provider도 clipboard도 파일시스템도 없이 전부 판정한다.** 이 파일은 DOM도 Tauri도 부르지
// 않는다. 값만 본다 (§18).
//
// 여기 모여 있는 것 다섯:
//
//   1. 두 줄의 위계 — 위는 연결된 provider, 아래는 내 AI                  (요구 6)
//   2. **provider가 하나도 없어도 아래 줄 세 자리가 전부 쓸 수 있다**      (MH-1 · MH-2)
//   3. provider 부재가 오류로도 설정 요구로도 그려지지 않는다              (INV-8 · 요구 6)
//   4. Export for AI 여섯 갈래와, 실패가 남기는 것 (§13 · MH-7)
//   5. 기존 Connected Provider 경로의 값이 그대로 실려 있다                (MH-8)
//   6. **문서가 얼마나 크고 이 파일이 몇 번째인가** (`phase-prompt/05.6` 성공 기준 4)
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type {
  AiNote,
  AiProviderStatus,
  ExportedAiRequest,
  ExportedFile,
  MeetingNote,
  NoteMode,
  Recording,
  TextSize,
  Transcript,
} from '../ipc/types';
import {
  AI_EXPORT_DONE_HEADLINE,
  AI_EXPORT_DONE_PORTION_HEADLINE,
  AI_EXPORT_FAILED_HEADLINE,
  AI_EXPORT_LABEL,
  AI_EXPORT_PRESERVED_NOTICE,
  AUTOMATIC_OPTIONAL_NOTICE,
  MANUAL_LOCAL_NOTICE,
  MANUAL_NO_PROVIDER_NOTICE,
  NO_AI_EXPORT_ATTEMPT,
  aiNoteTabLayout,
  exportedAiRequest,
  failedAiExport,
  manualHandoff,
  startedAiExport,
  type AiExportAttempt,
  type ManualHandoffInput,
} from './aiHandoffView';
import { aiNoteTab, type AiNoteInput } from './aiNoteView';
import {
  COPY_PROMPT_LABEL,
  COPY_TRANSCRIPT_LABEL,
  NO_COPY_ATTEMPT,
  TOO_LONG_NOTICE,
  WHOLE_PORTION_LABEL,
  startedCopy,
} from './copyView';
import {
  NO_SHOW_FILE_ATTEMPT,
  SHOW_FILE_LABEL,
  SHOW_FILE_PRESERVED_NOTICE,
  SHOW_FILE_RESOLUTION,
  SHOW_FILE_RETRY_LABEL,
  failedShowFile,
  showingFile,
} from './savedFileView';

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

function transcript(): Transcript {
  return {
    id: 't-1',
    recordingId: 'r-1',
    language: 'ko',
    segments: [{ startMs: 134_000, endMs: 141_000, text: '그러면 이번에는 PLY 먼저 변환하고' }],
    rawText: '그러면 이번에는 PLY 먼저 변환하고',
    createdAt: '2026-09-03T04:50:26.000Z',
    engine: 'whisper.cpp',
    model: 'ggml-base.bin',
    transcriptionMs: null,
    transcriptionLabel: null,
  };
}

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: '파일을 쓰지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

function file(overrides: Partial<ExportedFile> = {}): ExportedFile {
  return {
    recordingId: 'r-1',
    path: '/Users/me/Library/Application Support/molt-note/exports/2026-09-01-3dgs-study-04-ai-request.md',
    fileName: '2026-09-01-3dgs-study-04-ai-request.md',
    ...overrides,
  };
}

function size(chars: number, lines: number): TextSize {
  return { bytes: chars * 3, chars, lines };
}

/**
 * 쓰인 파일 하나. 기본값은 **나뉘지 않은 문서**다 — 짧은 녹음에서 늘 그렇고, 그때의 화면이
 * 이 Phase 이전과 달라지지 않아야 한다 (`phase-prompt/05.6` 성공 기준 4).
 */
function written(overrides: Partial<ExportedAiRequest> = {}): ExportedAiRequest {
  return {
    file: file(),
    totalSize: size(2_400, 61),
    portion: 1,
    portionCount: 1,
    portionSize: size(2_400, 61),
    ...overrides,
  };
}

/** 예산을 넘어 넷으로 나뉜 문서의 `index`번째 조각이 파일이 된 결과. */
function writtenPart(index: number, count = 4): ExportedAiRequest {
  return written({
    file: file({ fileName: `2026-09-01-3dgs-study-04-ai-request-part-${index}-of-${count}.md` }),
    totalSize: size(41_320, 1_711),
    portion: index,
    portionCount: count,
    portionSize: size(12_000, 430),
  });
}

function input(overrides: Partial<ManualHandoffInput> = {}): ManualHandoffInput {
  return {
    recording: recording(),
    mode: 'meeting',
    copy: NO_COPY_ATTEMPT,
    aiExport: NO_AI_EXPORT_ATTEMPT,
    // 자리를 여는 시도는 아직 하지 않았다 (`phase-prompt/05.6` 성공 기준 2).
    show: NO_SHOW_FILE_ATTEMPT,
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

/** 위 줄과 아래 줄을 함께 만든다 — 화면이 하는 것과 같은 조합이다. */
function layout(note: Partial<AiNoteInput> = {}, manual: Partial<ManualHandoffInput> = {}) {
  return aiNoteTabLayout(aiNoteTab(noteInput(note)), manualHandoff(input(manual)));
}

const MODES: readonly NoteMode[] = ['meeting', 'study', 'summary'];

// --- 1. provider가 없어도 아래 줄은 완전히 쓸 수 있다 (MH-1 · MH-2) ------------------------

describe('provider가 하나도 없어도 세 동작이 전부 가능하다 (MH-1 · MH-2)', () => {
  it('아래 줄의 입력에 AI provider를 담을 자리가 없다', () => {
    // 담을 자리가 없으므로 "provider가 없어서 복사할 수 없다"는 상태를 만들 수단이 없다.
    // 필드를 통째로 고정한다 — 새 필드가 하나 생기면 여기서 먼저 드러난다.
    //
    // **필드가 하나 늘었다** — `show`다 (`phase-prompt/05.6` 성공 기준 2 · R-4). 그것은
    // provider가 아니라 **이미 만들어진 파일 하나가 놓인 자리를 여는 시도**이며, 담고 있는
    // 것은 파일 경로다. 그 시도가 어떤 상태든 세 자리가 눌리는 사실은 달라지지 않는다
    // (아래 '자리를 여는 수단'이 그것을 값으로 못박는다).
    expect(Object.keys(input()).sort()).toEqual(['aiExport', 'copy', 'mode', 'recording', 'show']);
  });

  it('provider를 고르지 않은 그 순간에도 세 자리가 전부 눌린다', () => {
    const view = layout({ provider: noProvider() });

    // 위 줄은 비어 있다 — 그리고 그것이 아래 줄을 막지 않는다.
    expect(view.automatic.tab.body.kind).toBe('disabled');
    expect(view.automatic.available).toBe(false);

    expect(view.manual.available).toBe(true);
    expect(view.manual.steps.map((step) => step.key)).toEqual(['prompt', 'transcript', 'export']);
    expect(view.manual.steps.map((step) => step.label)).toEqual([
      COPY_PROMPT_LABEL,
      COPY_TRANSCRIPT_LABEL,
      AI_EXPORT_LABEL,
    ]);
    for (const step of view.manual.steps) {
      expect(step.usable, step.key).toBe(true);
    }
  });

  it('세 자리의 시작 동작이 실제로 값으로 있다 — 눌릴 것이 있다', () => {
    const view = manualHandoff(input());

    expect(view.copy.prompt.body.kind).toBe('notAsked');
    expect(view.copy.transcript.body.kind).toBe('notAsked');
    expect(view.aiExport.body.kind).toBe('notAsked');
    if (view.aiExport.body.kind !== 'notAsked') {
      throw new Error('내보낼 수 있어야 한다');
    }
    expect(view.aiExport.body.start.recordingId).toBe('r-1');
    expect(view.aiExport.body.start.mode).toBe('meeting');
  });

  it('provider 상태가 무엇이든 아래 줄의 값이 글자 하나 달라지지 않는다', () => {
    // 아래 줄은 provider를 보지 않는다. 그 사실을 깊은 비교로 확인한다 (INV-8의 화면 판정).
    const baseline = layout({ provider: noProvider() }).manual;

    for (const provider of [readyProvider(), noProvider()]) {
      expect(layout({ provider }).manual).toEqual(baseline);
    }
  });

  it('고른 mode 셋 전부에서 세 자리가 그대로 쓸 수 있다', () => {
    for (const mode of MODES) {
      const view = manualHandoff(input({ mode }));

      expect(view.available, mode).toBe(true);
      expect(
        view.steps.every((step) => step.usable),
        mode,
      ).toBe(true);
      if (view.aiExport.body.kind !== 'notAsked') {
        throw new Error('내보낼 수 있어야 한다');
      }
      // 문서는 고른 mode로 만들어진다 — 컴포넌트가 mode를 따로 고르지 않는다.
      expect(view.aiExport.body.start.mode, mode).toBe(mode);
    }
  });

  it('전사가 없을 때만 쓸 수 없고, 그것은 실패가 아니다 (§7.2 · MH-5)', () => {
    const view = manualHandoff(input({ recording: recording({ currentTranscriptId: null }) }));

    expect(view.available).toBe(false);
    expect(view.aiExport.body.kind).toBe('nothingToExport');
    expect(view.copy.prompt.body.kind).toBe('nothingToCopy');
    // 실패로 그릴 재료를 주지 않는다.
    expect(view.aiExport.body).not.toHaveProperty('failure');
  });
});

// --- 2. 두 줄의 위계 (요구 6) -----------------------------------------------------------

describe('AI Note 탭은 두 줄이다 (요구 6)', () => {
  it('위는 자동으로 만들기, 아래는 내 AI로 하기이며 그 사이에 접속사가 있다', () => {
    const view = layout();

    expect(view.automatic.heading.trim().length).toBeGreaterThan(0);
    expect(view.manual.heading.trim().length).toBeGreaterThan(0);
    expect(view.automatic.heading).not.toBe(view.manual.heading);
    // 위 줄은 조건이 아니다 — 둘 중 하나를 고르는 것이다.
    expect(view.orText).toBe('or');
    expect(Object.keys(view).sort()).toEqual([
      'automatic',
      'manual',
      'modeSelectable',
      'modes',
      'orText',
    ]);
  });

  it('mode 선택은 두 줄이 함께 쓰고, provider가 없다는 이유로 잠기지 않는다', () => {
    const view = layout({ provider: noProvider() });

    // 위 줄만 보면 잠겨 있다 (기존 규칙 그대로다).
    expect(view.automatic.tab.modeSelectable).toBe(false);
    // 그러나 아래 줄이 이 mode를 쓰므로 탭에서는 고를 수 있다 (MH-1 · MH-2).
    expect(view.modeSelectable).toBe(true);
    expect(view.modes.map((choice) => choice.mode)).toEqual(MODES);
    expect(view.modes.filter((choice) => choice.selected)).toHaveLength(1);
  });

  it('노트를 만드는 중에는 mode가 잠긴다 — 기존 규칙 그대로다', () => {
    const view = layout({
      provider: readyProvider(),
      live: {
        state: 'running',
        recordingId: 'r-1',
        mode: 'meeting',
        aiNoteId: null,
        failure: null,
      },
    });

    expect(view.automatic.tab.body.kind).toBe('generating');
    expect(view.modeSelectable).toBe(false);
  });

  it('아직 아무것도 읽지 못했으면 고를 것도 없다', () => {
    const view = aiNoteTabLayout(
      aiNoteTab(noteInput({ recording: null, provider: null })),
      manualHandoff(input({ recording: null })),
    );

    expect(view.automatic.tab.body.kind).toBe('loading');
    expect(view.modeSelectable).toBe(false);
  });

  it('provider 부재가 오류로도 설정 요구로도 그려지지 않는다 (INV-8)', () => {
    const view = layout({ provider: noProvider() });

    // 위 줄의 본문은 여전히 담담한 `disabled`이며 실패 재료가 없다.
    expect(view.automatic.tab.body.kind).toBe('disabled');
    expect(view.automatic.tab.body).not.toHaveProperty('failure');
    // 그리고 그것이 **선택**이라는 사실이 값으로 있다 — 재촉이 아니다.
    expect(view.automatic.optionalNotice).toBe(AUTOMATIC_OPTIONAL_NOTICE);
    expect(view.automatic.optionalNotice).toMatch(/optional/i);
    expect(view.automatic.optionalNotice).not.toMatch(/must|required|need to set/i);
    // 아래 줄은 provider가 없어도 된다고 말한다.
    expect(view.manual.noProviderNotice).toBe(MANUAL_NO_PROVIDER_NOTICE);
  });

  it('아직 읽지 못했거나 재료가 없는 것은 provider와 상관없는 사실이다', () => {
    // 그 상태에 "건너뛰고 아래 줄을 쓰라"고 말하면 사실이 아닌 것을 말하게 된다 —
    // 아래 줄도 같은 이유로 기다리고 있기 때문이다.
    const loading = aiNoteTabLayout(
      aiNoteTab(noteInput({ recording: null, provider: null })),
      manualHandoff(input({ recording: null })),
    );
    expect(loading.automatic.available).toBe(false);
    expect(loading.automatic.optionalNotice).toBeNull();

    const empty = layout(
      { recording: recording({ currentTranscriptId: null }), transcript: null, provider: readyProvider() },
      { recording: recording({ currentTranscriptId: null }) },
    );
    expect(empty.automatic.tab.body.kind).toBe('noTranscript');
    expect(empty.automatic.optionalNotice).toBeNull();
  });

  it('위 줄이 준비돼 있으면 선택이라는 안내를 덧붙이지 않는다', () => {
    const view = layout({ provider: readyProvider() });

    expect(view.automatic.available).toBe(true);
    expect(view.automatic.optionalNotice).toBeNull();
    // 준비돼 있어도 아래 줄은 그대로 있다 — 둘은 배타적이지 않다.
    expect(view.manual.available).toBe(true);
  });

  it('앱이 아무 데도 보내지 않는다는 사실이 아래 줄에 언제나 있다 (MH-3 · INV-6)', () => {
    for (const provider of [noProvider(), readyProvider()]) {
      const view = layout({ provider });
      expect(view.manual.localNotice).toBe(MANUAL_LOCAL_NOTICE);
      expect(view.manual.localNotice).toMatch(/audio/i);
    }
  });
});

// --- 3. 기존 Connected Provider 경로는 그대로다 (MH-8) -------------------------------------

describe('기존 생성 경로가 그대로 살아 있다 (MH-8)', () => {
  it('위 줄이 들고 있는 것은 aiNoteTab이 만든 값 그대로다', () => {
    const tab = aiNoteTab(noteInput({ provider: readyProvider() }));
    const view = aiNoteTabLayout(tab, manualHandoff(input()));

    // 다시 만들지 않는다 — 같은 값이다.
    expect(view.automatic.tab).toBe(tab);
    expect(view.automatic.tab.modes.map((choice) => choice.mode)).toEqual(MODES);
    expect(view.automatic.tab.modeSelectable).toBe(true);
  });

  it('노트가 있으면 재생성과 provenance가 그대로 위 줄에 있다', () => {
    const view = layout({ provider: readyProvider(), notes: [aiNote()] });

    expect(view.automatic.available).toBe(true);
    if (view.automatic.tab.body.kind !== 'ready') {
      throw new Error('노트를 볼 수 있어야 한다');
    }
    expect(view.automatic.tab.body.regenerate.kind).toBe('regenerate');
    expect(view.automatic.tab.body.note.provenance.model).toBe('llama3.1:8b');
  });

  it('생성 중에도 아래 줄은 그대로 쓸 수 있다', () => {
    const view = layout(
      {
        provider: readyProvider(),
        live: {
          state: 'running',
          recordingId: 'r-1',
          mode: 'meeting',
          aiNoteId: null,
          failure: null,
        },
      },
      {},
    );

    expect(view.automatic.tab.body.kind).toBe('generating');
    expect(view.manual.steps.every((step) => step.usable)).toBe(true);
  });
});

// --- 4. Export for AI 여섯 갈래 (요구 3 · §13) ---------------------------------------------

describe('Export for AI 자리 (요구 3)', () => {
  it('레코드를 아직 읽지 못했으면 판단하지 않는다', () => {
    const view = manualHandoff(input({ recording: null }));

    expect(view.aiExport.body.kind).toBe('loading');
    expect(view.aiExport.label).toBe(AI_EXPORT_LABEL);
    expect(view.available).toBe(false);
  });

  it('쓰는 중에는 다시 누를 자리가 없다', () => {
    const view = manualHandoff(input({ aiExport: startedAiExport('r-1', 'meeting', 1) }));

    expect(view.aiExport.body.kind).toBe('exporting');
    expect(view.steps[2].usable).toBe(false);
    // 복사 두 자리는 그대로다 — 한쪽의 상태가 다른 쪽을 가리지 않는다.
    expect(view.steps[0].usable).toBe(true);
    expect(view.steps[1].usable).toBe(true);
  });

  it('파일이 만들어지면 이름과 전체 경로가 문장과 함께 보인다 (§4.1)', () => {
    const view = manualHandoff(input({ aiExport: exportedAiRequest(written()) }));

    if (view.aiExport.body.kind !== 'done') {
      throw new Error('파일이 만들어진 상태여야 한다');
    }
    // 색이 아니라 문장이 결과를 말한다 (요구 12).
    expect(view.aiExport.body.headline).toBe(AI_EXPORT_DONE_HEADLINE);
    expect(view.aiExport.body.fileName).toBe('2026-09-01-3dgs-study-04-ai-request.md');
    expect(view.aiExport.body.path).toContain('/exports/');
    // 또 내보낼 수 있고, 있던 파일은 그대로다.
    expect(view.aiExport.body.again.kind).toBe('again');
    expect(view.aiExport.body.text).toMatch(/never overwrites/);
    // 나뉘지 않았으면 더 꺼낼 것이 없다 — 없는 조각을 가리키는 버튼을 만들지 않는다.
    expect(view.aiExport.body.next).toBeNull();
    expect(view.aiExport.body.portion.label).toBe(WHOLE_PORTION_LABEL);
    expect(view.aiExport.body.size.fitsInOne).toBe(true);
  });

  it('만들어진 파일이 놓인 자리를 여는 수단이 경로와 함께 있다 (05.6 성공 기준 2 · R-4)', () => {
    // 만든 파일을 AI 채팅에 첨부하려면 그 파일에 실제로 도달할 수 있어야 한다. 경로만으로는
    // 도달하지 못한다는 것이 2026-09-05의 실사용에서 드러났다 (macOS의 `~/Library` 숨김).
    const document = written();
    const view = manualHandoff(input({ aiExport: exportedAiRequest(document) }));

    if (view.aiExport.body.kind !== 'done') {
      throw new Error('파일이 만들어진 상태여야 한다');
    }
    // 대체가 아니라 추가다 — 경로는 그대로다.
    expect(view.aiExport.body.path).toBe(document.file.path);
    expect(view.aiExport.body.show.action.kind).toBe('show');
    expect(view.aiExport.body.show.action.label).toBe(SHOW_FILE_LABEL);
    expect(view.aiExport.body.show.action.path).toBe(document.file.path);
    expect(view.aiExport.body.show.trouble).toBeNull();
    // 이 자리는 여전히 눌린다 — 여는 수단이 생겼다고 다시 내보낼 수 없게 되지 않는다.
    expect(view.steps[2].usable).toBe(true);
  });

  it('자리를 열지 못해도 파일 · 경로 · 세 자리는 그대로다 (§13 · INV-3 · MH-7)', () => {
    const document = written();
    const view = manualHandoff(
      input({
        aiExport: exportedAiRequest(document),
        show: failedShowFile(document.file.path, failure('storage')),
      }),
    );

    if (view.aiExport.body.kind !== 'done') {
      throw new Error('여는 데 실패해도 파일은 만들어진 채다');
    }
    expect(view.aiExport.body.headline).toBe(AI_EXPORT_DONE_HEADLINE);
    expect(view.aiExport.body.path).toBe(document.file.path);

    const trouble = view.aiExport.body.show.trouble;
    if (trouble === null) {
      throw new Error('실패가 값으로 있어야 한다');
    }
    expect(trouble.failure.kind).toBe('storage');
    expect(trouble.preservedNotice).toBe(SHOW_FILE_PRESERVED_NOTICE);
    // 그래도 도달할 길이 남아 있다 — 위에 그대로 있는 경로다.
    expect(trouble.resolution).toBe(SHOW_FILE_RESOLUTION);
    expect(view.aiExport.body.show.action.label).toBe(SHOW_FILE_RETRY_LABEL);
    // 복사 두 자리도, 다시 내보내는 자리도 그대로 쓸 수 있다.
    expect(view.steps.every((step) => step.usable)).toBe(true);
  });

  it('여는 중이라는 사실이 문장으로 있고, 다른 파일의 시도는 보이지 않는다', () => {
    const document = written();

    const showing = manualHandoff(
      input({ aiExport: exportedAiRequest(document), show: showingFile(document.file.path) }),
    );
    if (showing.aiExport.body.kind !== 'done') {
      throw new Error('파일이 만들어진 상태여야 한다');
    }
    expect(showing.aiExport.body.show.showing).toBe(true);
    expect(showing.aiExport.body.show.text.trim()).not.toBe('');

    const elsewhere = manualHandoff(
      input({
        aiExport: exportedAiRequest(document),
        show: failedShowFile('/somewhere/else.md', failure('storage')),
      }),
    );
    if (elsewhere.aiExport.body.kind !== 'done') {
      throw new Error('파일이 만들어진 상태여야 한다');
    }
    expect(elsewhere.aiExport.body.show.trouble).toBeNull();
    expect(elsewhere.aiExport.body.show.showing).toBe(false);
  });

  it('실패는 §13의 세 질문에 답하고 재시도 수단을 남긴다 (MH-7)', () => {
    const cases: readonly (readonly [FailureKind, string, string | null])[] = [
      ['invalidInput', 'nothingToExport', 'Transcribe this recording'],
      ['storage', 'storage', 'The file could not be written'],
      ['unexpected', 'other', null],
    ];

    for (const [kind, cause, resolution] of cases) {
      const view = manualHandoff(input({ aiExport: failedAiExport('r-1', 1, failure(kind)) }));

      if (view.aiExport.body.kind !== 'failed') {
        throw new Error('실패 상태여야 한다');
      }
      // 1. 무엇이 실패했는가.
      expect(view.aiExport.body.headline, kind).toBe(AI_EXPORT_FAILED_HEADLINE);
      expect(view.aiExport.body.failure.kind, kind).toBe(kind);
      expect(view.aiExport.body.cause, kind).toBe(cause);
      // 2. 원본은 안전한가 (INV-3 · MH-7).
      expect(view.aiExport.body.preservedNotice, kind).toBe(AI_EXPORT_PRESERVED_NOTICE);
      // 3. 다시 시도할 수 있는가.
      expect(view.aiExport.body.retry.kind, kind).toBe('retry');
      expect(view.aiExport.body.retry.recordingId, kind).toBe('r-1');
      expect(view.steps[2].usable, kind).toBe(true);

      if (resolution === null) {
        expect(view.aiExport.body.resolution, kind).toBeNull();
      } else {
        expect(view.aiExport.body.resolution, kind).toContain(resolution);
      }
    }
  });

  it('실패해도 복사 두 자리는 그대로 쓸 수 있다 (MH-7)', () => {
    const view = manualHandoff(input({ aiExport: failedAiExport('r-1', 1, failure('storage')) }));

    expect(view.copy.prompt.body.kind).toBe('notAsked');
    expect(view.copy.transcript.body.kind).toBe('notAsked');
    expect(view.available).toBe(true);
  });

  it('다른 녹음의 결과가 이 자리에 보이지 않는다', () => {
    const others: readonly AiExportAttempt[] = [
      startedAiExport('r-2', 'meeting', 1),
      exportedAiRequest(written({ file: file({ recordingId: 'r-2' }) })),
      failedAiExport('r-2', 1, failure('storage')),
    ];

    for (const aiExport of others) {
      const view = manualHandoff(input({ aiExport }));
      expect(view.aiExport.body.kind, aiExport.kind).toBe('notAsked');
    }
  });

  it('복사 중인 자리와 내보내기 자리가 서로를 가리지 않는다', () => {
    const view = manualHandoff(input({ copy: startedCopy('prompt', 'r-1', 1) }));

    expect(view.steps[0].usable).toBe(false);
    expect(view.steps[1].usable).toBe(true);
    expect(view.steps[2].usable).toBe(true);
  });
});

// --- 6. 크기와 나눔 (`phase-prompt/05.6` 성공 기준 4 · R-5) --------------------------------

describe('나뉜 문서를 완전한 것처럼 말하지 않는다', () => {
  it('조각 하나만 파일이 됐으면 문서가 준비됐다고 말하지 않는다', () => {
    const view = manualHandoff(input({ aiExport: exportedAiRequest(writtenPart(1)) }));

    if (view.aiExport.body.kind !== 'done') {
      throw new Error('파일이 만들어진 상태여야 한다');
    }
    expect(view.aiExport.body.headline).toBe(AI_EXPORT_DONE_PORTION_HEADLINE);
    expect(view.aiExport.body.headline).not.toBe(AI_EXPORT_DONE_HEADLINE);
    // 얼마나 크고 몇 번째인가가 값으로 있다.
    expect(view.aiExport.body.portion.label).toBe('Part 1 of 4.');
    expect(view.aiExport.body.size.label).toContain('41,320');
    expect(view.aiExport.body.size.tooLongNotice).toBe(TOO_LONG_NOTICE);
    // 파일 이름 자체가 몇 번째인지 말한다 (backend가 붙인 이름 그대로다).
    expect(view.aiExport.body.fileName).toContain('part-1-of-4');
  });

  it('나머지를 마저 파일로 꺼내는 수단이 그 자리에 있다', () => {
    const view = manualHandoff(input({ aiExport: exportedAiRequest(writtenPart(2)) }));

    if (view.aiExport.body.kind !== 'done') {
      throw new Error('파일이 만들어진 상태여야 한다');
    }
    expect(view.aiExport.body.next?.kind).toBe('next');
    expect(view.aiExport.body.next?.portion).toBe(3);
    expect(view.aiExport.body.next?.label).toBe('Export part 3 of 4');
    expect(view.aiExport.body.next?.mode).toBe('meeting');
    // 같은 조각을 다시 쓰는 수단은 그대로다 — 있던 파일을 덮어쓰지 않는다.
    expect(view.aiExport.body.again.portion).toBe(2);
    expect(view.aiExport.body.text).toMatch(/never overwrites/);
    // 이 자리는 여전히 눌린다.
    expect(view.steps[2].usable).toBe(true);
  });

  it('마지막 조각에서만 문서 전체가 나왔다고 말한다', () => {
    const view = manualHandoff(input({ aiExport: exportedAiRequest(writtenPart(4)) }));

    if (view.aiExport.body.kind !== 'done') {
      throw new Error('파일이 만들어진 상태여야 한다');
    }
    expect(view.aiExport.body.next).toBeNull();
    expect(view.aiExport.body.portion.remaining).toBe(0);
    expect(view.aiExport.body.text).toContain('last part');
  });

  it('쓰려던 조각으로 다시 시도한다 — 건너뛰지 않는다', () => {
    const view = manualHandoff(input({ aiExport: failedAiExport('r-1', 3, failure('storage')) }));

    if (view.aiExport.body.kind !== 'failed') {
      throw new Error('실패 상태여야 한다');
    }
    expect(view.aiExport.body.retry.portion).toBe(3);
  });

  it('provider가 하나도 없어도 크기와 조각을 그대로 말한다 (MH-1 · MH-2)', () => {
    // 나눔은 provider와 아무 상관이 없다 — 입력에 provider를 담을 자리가 없으므로 그것 때문에
    // 달라질 수단 자체가 없다.
    const withProvider = layout({ provider: readyProvider() }, {
      aiExport: exportedAiRequest(writtenPart(2)),
    });
    const withoutProvider = layout({ provider: noProvider() }, {
      aiExport: exportedAiRequest(writtenPart(2)),
    });

    expect(withoutProvider.manual).toEqual(withProvider.manual);
  });
});
