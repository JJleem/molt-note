// **Phase 5.6의 불변 중 화면에서만 판정되는 하나 — 만들어진 파일에 도달하는 수단**
// (`phase-prompt/05.6` 성공 기준 2 · R-4 · PRODUCT-SPEC §13 · INV-10).
//
// 이 Phase의 네 불변 중 셋(언어 · 설정 왕복 · 나눔의 무손실)은 저장소와 파일시스템을 지나므로
// `src-tauri/tests/transcription_and_reach_invariants.rs`가 본다. 여기서 보는 것은 그 파일이
// 볼 수 없는 한 가지다 — **파일이 만들어졌다고 말하는 자리마다 그 자리를 여는 동작이 값으로
// 함께 있는가.**
//
//   TR-3a  파일을 만드는 두 자리의 done 상태가 **둘 다** 여는 동작을 값으로 들고 있다
//   TR-3b  여는 데 실패해도 그 동작과 전체 경로가 그 자리에 남는다 (대체가 아니라 추가다)
//   TR-3c  파일을 만들었다고 말하는 화면 모듈이 **그 둘뿐이고**, 둘 다 여는 자리를 지난다
//
// TR-3c가 원문 검사인 이유는 나머지 둘이 잡지 못하는 것을 잡기 위해서다 — 세 번째 export
// 표면이 생기면서 여는 수단만 빠지면, 값 수준 검사는 그런 자리가 생겼다는 것조차 모른다.
//
// ## 이 파일이 판정하지 않는 것
//
// **실제로 파일이 열리는가는 여기서 판정되지 않는다.** 창을 띄우는 것은 OS이며, 그 확인은
// `phase-prompt/05.6`의 Human Review 항목이다. 이 파일이 보는 것은 화면이 그 동작을 **값으로**
// 들고 있는가뿐이고, 무엇을 열어도 되는지의 판정은 backend에 있다
// (`src-tauri/tests/show_saved_file.rs`).
//
// ## 이미 있는 검사를 다시 쓰지 않는다
//
//   여는 자리 하나의 값 갈래 (label · 문장 · trouble)   src/screens/savedFileView.test.ts
//   각 화면의 done 상태 갈래 전부                        src/screens/exportView.test.ts ·
//                                                        src/screens/aiHandoffView.test.ts
//   command 표면에 `show_saved_file`이 있고 그것이
//   파일을 만들지 않는다는 사실                          tests/ipc-boundary.test.ts
//   화면이 OS나 파일 관리자를 알지 않는다는 사실         tests/screen-boundary.test.ts
//
// 이 파일은 DOM도 Tauri도 OS도 부르지 않는다 — 순수 모듈의 값을 읽고 원문을 볼 뿐이다.
import { readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

import type { ExportedAiRequest, ExportedFile, Recording } from '../src/ipc/types';
import type { Failure } from '../src/ipc/failure';
import { NO_COPY_ATTEMPT } from '../src/screens/copyView';
import { exportPanel, exportedFile } from '../src/screens/exportView';
import { exportedAiRequest, manualHandoff } from '../src/screens/aiHandoffView';
import {
  failedShowFile,
  NO_SHOW_FILE_ATTEMPT,
  SHOW_FILE_LABEL,
  SHOW_FILE_RETRY_LABEL,
} from '../src/screens/savedFileView';

const path = (relative: string) => fileURLToPath(new URL(relative, import.meta.url));

/** 화면 순수 모듈이 사는 자리. TR-3c가 이 디렉터리 전체를 본다. */
const VIEWS = path('../src/screens');

/** 사용자가 실제로 보는 자리 — macOS에서 Finder 기본 숨김인 디렉터리다 (R-4). */
const EXPORTED_PATH =
  '/Users/someone/Library/Application Support/molt-note/exports/2026-09-01-3dgs-study-04.md';

function recording(overrides: Partial<Recording> = {}): Recording {
  return {
    id: 'r-1',
    title: '3DGS Study #04',
    createdAt: '2026-09-01T10:00:00.000Z',
    updatedAt: '2026-09-01T10:52:31.000Z',
    durationMs: 3_151_000,
    durationLabel: '52:31',
    audioPath: '/tmp/r-1.wav',
    audioFormat: 'wav',
    microphone: 'MacBook Pro Microphone',
    currentTranscriptId: 't-1',
    transcriptionStatus: 'done',
    aiStatus: 'none',
    notionStatus: 'none',
    ...overrides,
  };
}

function file(overrides: Partial<ExportedFile> = {}): ExportedFile {
  return {
    recordingId: 'r-1',
    path: EXPORTED_PATH,
    fileName: '2026-09-01-3dgs-study-04.md',
    ...overrides,
  };
}

/** AI-ready 문서 하나가 파일이 된 결과. 나뉘지 않은 문서이며 크기는 이 검사와 무관하다. */
function writtenForAi(): ExportedAiRequest {
  const size = { bytes: 7_200, chars: 2_400, lines: 61 };
  return {
    file: file({ fileName: '2026-09-01-3dgs-study-04-ai-request.md' }),
    totalSize: size,
    portion: 1,
    portionCount: 1,
    portionSize: size,
  };
}

function failure(): Failure {
  return {
    kind: 'storage',
    message: '폴더를 열지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
  };
}

/**
 * 파일이 만들어졌다고 말하는 두 자리의 done 상태를, **같은 열기 시도 하나**에서 만든다.
 *
 * 둘을 함께 만드는 이유는 이 파일이 보는 것이 "한 자리에 있는가"가 아니라 "말하는 자리마다
 * 있는가"이기 때문이다.
 */
function doneStates(show = NO_SHOW_FILE_ATTEMPT) {
  const markdown = exportPanel({
    recording: recording(),
    notes: [],
    attempt: exportedFile(file()),
    show,
  }).body;

  const forAi = manualHandoff({
    recording: recording(),
    mode: 'meeting',
    copy: NO_COPY_ATTEMPT,
    aiExport: exportedAiRequest(writtenForAi()),
    show,
  }).aiExport.body;

  if (markdown.kind !== 'done' || forAi.kind !== 'done') {
    throw new Error('사전 조건: 두 자리 다 done 상태여야 한다');
  }

  // **두 자리를 같은 모양으로 본다.** 값이 놓인 자리는 서로 다르지만(Markdown 쪽은 파일 하나에
  // 대한 값을 `file` 아래에 모아 두고, AI 쪽은 조각 정보와 나란히 둔다), 사용자에게 답해야 하는
  // 것은 같다 — 어디에 만들어졌는가와 그 자리를 여는 수단이 있는가.
  return [
    {
      label: 'markdown',
      show: markdown.file.show,
      path: markdown.file.path,
      fileName: markdown.file.fileName,
    },
    { label: 'forAi', show: forAi.show, path: forAi.path, fileName: forAi.fileName },
  ] as const;
}

describe('TR-3a — 파일을 만드는 자리마다 그 자리를 여는 동작이 값으로 있다', () => {
  it('두 done 상태가 둘 다 여는 동작을 들고 있고, 그 대상은 backend가 준 경로다', () => {
    // 무엇이 깨지면 이 검사가 잡는가: **두 자리 중 하나에서 여는 수단이 빠지면** 잡는다.
    // 2026-09-05의 실사용에서 사람이 파일에 도달하지 못한 이유는 경로를 몰라서가 아니라
    // 그 경로로 걸어 들어갈 수 없어서였다 (R-4). 한쪽에만 여는 수단이 있으면 사용자는
    // 어느 자리에서 만들었는지에 따라 도달할 수도, 못 할 수도 있게 된다.
    //
    // 그리고 **경로는 화면이 지어내지 않는다** — 같은 이름이 이미 있었으면 backend가 번호를
    // 붙였으므로, 여는 대상은 실제로 쓰인 그 경로여야 한다 (ADR-0009 §4.3).
    const places = doneStates();

    for (const { label, show, path: shownPath } of places) {
      expect(show.action.kind, label).toBe('show');
      expect(show.action.label, label).toBe(SHOW_FILE_LABEL);
      expect(show.action.path, label).toBe(EXPORTED_PATH);
      expect(show.trouble, label).toBeNull();
      // 여는 수단은 경로의 **대체가 아니라 추가다** — 전체 경로가 그대로 남아 있다.
      expect(shownPath, label).toBe(EXPORTED_PATH);
    }

    // 두 자리가 가리키는 것은 **각자 만든 파일**이다 — 이름까지 같아지면 사용자는 방금 만든
    // 것이 어느 문서인지 알 수 없다.
    expect(places[0].fileName).not.toBe(places[1].fileName);
  });
});

describe('TR-3b — 열지 못해도 그 자리는 사라지지 않는다', () => {
  it('실패한 뒤에도 두 자리에 다시 여는 동작과 전체 경로가 남는다', () => {
    // 무엇이 깨지면 이 검사가 잡는가: **실패를 그 자리의 끝으로 만들면** 잡는다 — 실패했을 때
    // 동작이 사라지거나, 경로가 함께 사라지거나, "파일이 없어졌다"고 읽히는 상태가 되면 여기서
    // 실패한다. 창 하나를 띄우지 못한 것은 파일에 일어난 일이 아니다 (INV-3).
    const places = doneStates(failedShowFile(EXPORTED_PATH, failure()));

    for (const { label, show, path: shownPath } of places) {
      expect(show.action.kind, label).toBe('retry');
      expect(show.action.label, label).toBe(SHOW_FILE_RETRY_LABEL);
      expect(show.action.path, label).toBe(EXPORTED_PATH);

      const trouble = show.trouble;
      expect(trouble, label).not.toBeNull();
      // §13의 세 질문 중 둘이 값으로 있다 — 무엇이 그대로인가, 그래서 무엇을 할 수 있는가.
      expect(trouble?.preservedNotice.trim(), label).not.toBe('');
      expect(trouble?.resolution.trim(), label).not.toBe('');

      // 파일에 대한 사실은 하나도 달라지지 않았다 — 경로는 그 자리에 그대로다.
      expect(shownPath, label).toBe(EXPORTED_PATH);
    }
  });

  it('다른 파일을 열다 실패한 사실은 이 자리에 보이지 않는다', () => {
    // 무엇이 깨지면 이 검사가 잡는가: **열기 시도를 화면 전체의 상태로 다루면** 잡는다.
    // 한 파일을 열다 실패한 것이 방금 만든 다른 파일의 자리에 실패로 보이면, 사용자는 만들지도
    // 않은 문제를 보게 된다.
    const places = doneStates(failedShowFile('/somewhere/else.md', failure()));

    for (const { label, show } of places) {
      expect(show.trouble, label).toBeNull();
      expect(show.action.kind, label).toBe('show');
    }
  });
});

describe('TR-3c — 파일을 만들었다고 말하는 화면 모듈은 둘뿐이고, 둘 다 여는 자리를 지난다', () => {
  it('만들어진 파일을 값으로 받는 화면 모듈이 늘어나면 이 검사가 그것을 알린다', () => {
    // 무엇이 깨지면 이 검사가 잡는가: **세 번째 export 표면이 여는 수단 없이 생기면** 잡는다.
    // 값 수준 검사는 자기가 아는 두 자리만 보므로 그런 자리가 생겼다는 것조차 모른다. 목록을
    // 고정해 두면 늘어난 자리는 반드시 이 줄을 지나며, 그때 여는 수단을 함께 두는지 사람이
    // 결정하게 된다.
    //
    // **여는 자리 자신은 `savedFileView.ts` 하나다** (INV-10) — 판정도 문장도 거기 있고,
    // 두 화면 모듈은 그것을 부를 뿐이다.
    const reporting = readdirSync(VIEWS)
      .filter((entry) => /\.tsx?$/.test(entry) && !entry.includes('.test.'))
      .filter((entry) => {
        const source = readFileSync(`${VIEWS}/${entry}`, 'utf8');
        return /import type \{[^}]*\bExported(File|AiRequest)\b/s.test(source);
      })
      .sort();

    expect(reporting).toEqual(['aiHandoffView.ts', 'exportView.ts']);

    for (const entry of reporting) {
      const source = readFileSync(`${VIEWS}/${entry}`, 'utf8');
      expect(source, entry).toContain("from './savedFileView'");
      expect(source, entry).toContain('showFile(');
    }
  });
});
