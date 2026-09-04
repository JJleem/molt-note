/**
 * Recording Detail의 **Export Markdown** 자리 (PRODUCT-SPEC §11 · §13 ·
 * `phase-prompt/05` 요구 A-1~3 · P-3 · docs/ADR-0009-notion-and-export.md §4).
 *
 * 이 자리가 답해야 하는 질문은 셋이다 — **지금 내보낼 수 있는가** · **무엇이 파일에
 * 들어가는가** · **방금 만들어진 파일은 어디에 있는가**. 셋 다 backend가 준 사실에서 나오며,
 * 그 사실을 화면 상태로 옮기는 규칙이 전부 여기 있다. React도 DOM도 Tauri도 알지 않으므로
 * 여섯 경로(아직 읽지 못함 · 전사 없음 · 내보낼 수 있음 · 쓰는 중 · 완료 · 실패)를
 * **파일시스템 없이** vitest로 그대로 판정할 수 있다 (§18 · `aiNoteView.ts`와 같은 형태다).
 *
 * ## AI를 보지 않는다 (INV-8 · `phase-prompt/05` P-3)
 *
 * **이 모듈의 입력에 AI provider가 없다.** provider의 상태를 담을 자리 자체가 없으므로
 * "provider가 없어서 내보낼 수 없다"는 상태를 만들 수단이 없다. 노트는 [`ExportPanelInput.notes`]
 * 하나로만 들어오고, 그것이 하는 일은 **무엇이 파일에 들어가는지 알려 주는 것뿐이다** —
 * 비어 있어도 내보내기는 그대로 가능하다. 그것이 §17.1의 core 성공 기준이다.
 *
 * ## 내보내기는 읽고 파일 하나를 더하는 일이다 (INV-3 · INV-6)
 *
 * 실패해도 녹음 · 오디오 파일 · 전사 · 노트는 그대로다 — export 경로에는 저장소에 쓰는 코드가
 * 없기 때문이다 (`src-tauri/src/export/run.rs`). 그 사실이 실패 상태의 값으로 남는다
 * ({@link EXPORT_PRESERVED_NOTICE}). 오디오는 복사되지도 전송되지도 않는다.
 *
 * ## 경로는 backend가 정하고 화면이 보여 준다 (§4.1 · INV-10)
 *
 * export 위치는 설정으로 노출되지 않으므로, 만들어진 파일의 전체 경로를 화면이 보여 주지 않으면
 * 사용자는 방금 만든 파일을 찾을 수 없다. 이 모듈은 그 경로를 **backend가 준 값 그대로** 들고
 * 있으며 문자열을 잘라 이름을 짐작하지 않는다 — 같은 이름이 있었으면 backend가 번호를 붙였고
 * (§4.3), 실제로 쓰인 이름이 함께 온다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { AiNote, ExportedFile, Recording } from '../ipc/types';

/**
 * 이 자리에서 사용자가 할 수 있는 동작 하나.
 *
 * **함수가 아니라 값이다** — 순수 모듈이 command를 알지 않기 때문이며, 그래서 "지금 내보낼 수
 * 있는가"·"재시도 수단이 있는가"가 DOM 없이 판정된다. 실제로 부르는 것은 화면 컴포넌트다
 * (`RecordingDetailScreen`의 `exportMarkdown`).
 *
 * 셋을 한 종류로 접지 않는 이유는 사용자에게 다른 상황이기 때문이다 — 처음 내보내는 것 ·
 * 이미 내보낸 것을 또 내보내는 것(파일이 하나 더 생긴다) · 실패한 뒤 다시 하는 것.
 */
export interface ExportAction {
  readonly kind: 'export' | 'again' | 'retry';
  readonly label: string;
  readonly recordingId: string;
}

/** 화면에 처음 보이는 이름. Task와 Phase Goal이 부르는 이름 그대로다. */
export const EXPORT_LABEL = 'Export Markdown';

function exportAction(recordingId: string): ExportAction {
  return { kind: 'export', label: EXPORT_LABEL, recordingId };
}

function againAction(recordingId: string): ExportAction {
  // 다시 내보내면 파일이 하나 더 생긴다 — 있던 파일을 덮어쓰지 않는다 (§4.3).
  return { kind: 'again', label: 'Export another Markdown file', recordingId };
}

function retryAction(recordingId: string): ExportAction {
  return { kind: 'retry', label: 'Try the export again', recordingId };
}

/**
 * 방금 만들어진 파일 하나 (§4.1).
 *
 * `path`가 이 값의 요점이다. `fileName`을 따로 두는 이유는 **요청한 이름과 다를 수 있기
 * 때문이다** — 같은 이름이 있으면 backend가 `-2` · `-3`을 붙인다.
 */
export interface ExportedFileView {
  readonly headline: string;
  readonly fileName: string;
  /** 사용자에게 그대로 보여줄 수 있는 전체 경로. 화면이 만들어 내는 값이 아니다. */
  readonly path: string;
}

/** 파일이 만들어졌다는 사실 한 줄. */
export const EXPORT_DONE_HEADLINE = 'The Markdown file was written.';

/** 그 파일이 어떤 성질인가 — 여기서부터는 사용자의 문서다 (§4.3). */
export const EXPORT_DONE_TEXT =
  'This file is yours now. Exporting again writes another file next to it and never overwrites this one.';

/** AI 노트가 이 문서에 들어가는가. 아직 읽지 못한 것은 없는 것과 다른 사실이다. */
export type ExportNoteInclusion = 'included' | 'none' | 'unknown';

/**
 * **무엇이 파일에 들어가는가** (§11 · INV-5 · INV-6).
 *
 * 내보내기는 아무 데도 보내지 않지만, 무엇이 담기는지는 누르기 전에 보인다 — 그리고
 * **오디오 파일은 복사되지 않는다**는 사실이 같은 자리에 있다.
 */
export interface ExportContentsNotice {
  readonly headline: string;
  /** 언제나 들어가는 것. */
  readonly items: readonly string[];
  readonly note: ExportNoteInclusion;
  /** 노트가 들어가는지를 있는 그대로 말한다. 없다고 해서 못 내보내는 것이 아니다 (INV-8). */
  readonly noteText: string;
  /** 오디오는 복사되지 않는다 (INV-6). */
  readonly audioNotice: string;
}

export const EXPORT_CONTENTS_HEADLINE = 'What goes into the file';

/** 렌더러가 언제나 넣는 것 (§11의 구조). */
export const EXPORT_CONTENT_ITEMS: readonly string[] = [
  'The title, date, and length of this recording',
  'The transcript text, with a heading for each segment',
];

const NOTE_TEXT: Record<ExportNoteInclusion, string> = {
  included: 'The AI note for this recording is included.',
  none: 'There is no AI note for this recording, so the file has the details and the transcript. That is a complete document on its own.',
  unknown: 'Whether this recording has an AI note is not known right now. If it has one, it is included.',
};

/** 파일 하나를 쓰는 일이다. 어떤 바이트도 이 기기를 떠나지 않는다 (INV-6). */
export const EXPORT_AUDIO_NOTICE =
  'The audio file is not copied, and nothing is sent anywhere — this writes one text file on this device.';

function contentsNotice(notes: readonly AiNote[] | null): ExportContentsNotice {
  const note: ExportNoteInclusion =
    notes === null ? 'unknown' : notes.length === 0 ? 'none' : 'included';
  return {
    headline: EXPORT_CONTENTS_HEADLINE,
    items: EXPORT_CONTENT_ITEMS,
    note,
    noteText: NOTE_TEXT[note],
    audioNotice: EXPORT_AUDIO_NOTICE,
  };
}

/** 내보낼 재료가 아직 없다. **실패가 아니다** (§7.2 · `export::run`의 `nothing_to_export`). */
export const NOTHING_TO_EXPORT_TEXT = 'There is nothing to export from this recording yet.';

/** 그래서 무엇을 하면 되는가. 이 자리가 전사를 시작하지 않는다 — 그 자리는 Transcript 탭이다. */
export const NOTHING_TO_EXPORT_HINT =
  'Transcribe this recording in the Transcript tab first, then export it.';

/** 지금 눌러도 되는가. 파일 하나를 쓰는 일이므로 기다릴 서버도 모델도 없다. */
export const EXPORT_READY_TEXT =
  'Write this recording to a Markdown file you can open in Obsidian, NotebookLM, or any editor.';

/** 쓰는 중. 전사·노트와 달리 이 일은 짧고, 화면을 떠나도 되는 종류의 일이 아니다. */
export const EXPORTING_TEXT = 'Writing the Markdown file…';

/** 무엇을 하다 실패했는가 (§13). 원인은 {@link Failure}가 말한다. */
export const EXPORT_FAILED_HEADLINE = 'This recording could not be exported.';

/**
 * 실패가 무엇을 남겼는지 (§13 · INV-1 · INV-2 · INV-3).
 *
 * **"복구했다"거나 "정리했다"고 말하지 않는다.** export 경로는 저장소를 읽기만 하므로
 * 실패했을 때 바뀐 것이 아무것도 없다 (`src-tauri/src/export/run.rs`).
 */
export const EXPORT_PRESERVED_NOTICE =
  'The recording, its audio file, the transcript, and any AI note are untouched. Nothing was deleted or changed — exporting only reads.';

/** 실패 갈래 중 사용자가 **먼저** 할 일이 달라지는 것 (§13). */
export type ExportFailureCause = 'nothingToExport' | 'storage' | 'other';

const FAILURE_RESOLUTION: Record<ExportFailureCause, string | null> = {
  nothingToExport: NOTHING_TO_EXPORT_HINT,
  storage:
    'The file could not be written. Check that this device has room and that the app can write to its data folder, then try again.',
  other: null,
};

function failureCause(failure: Failure): ExportFailureCause {
  switch (failure.kind) {
    case 'invalidInput':
      return 'nothingToExport';
    case 'storage':
      return 'storage';
    default:
      return 'other';
  }
}

/**
 * 이 화면이 건 내보내기 한 번.
 *
 * 전사·노트 생성과 달리 **상태를 물어보는 규약을 쓰지 않는다** — `export_markdown`은 이미
 * 만들어진 파일을 돌려주기 때문이다 (`src/ipc/commands.ts`). 그래서 진행 상황은 backend가
 * 아니라 이 호출 하나의 결과이며, 그 네 갈래가 이 타입이다.
 */
export type ExportAttempt =
  | { readonly kind: 'none' }
  | { readonly kind: 'running'; readonly recordingId: string }
  | { readonly kind: 'done'; readonly file: ExportedFile }
  | { readonly kind: 'failed'; readonly recordingId: string; readonly failure: Failure };

/** 아무것도 하지 않은 상태. 화면이 열렸을 때의 값이다. */
export const NO_EXPORT_ATTEMPT: ExportAttempt = { kind: 'none' };

/** 내보내기가 끝났을 때 (실패든 성공이든) 만드는 값. */
export function exportedFile(file: ExportedFile): ExportAttempt {
  return { kind: 'done', file };
}

/** 내보내기가 거절됐을 때 만드는 값 (§13). */
export function failedExport(recordingId: string, error: unknown): ExportAttempt {
  return { kind: 'failed', recordingId, failure: toFailure(error) };
}

/**
 * Export 자리가 놓일 수 있는 상태의 전부.
 *
 * ```text
 * loading         아직 레코드를 읽지 못했다
 * nothingToExport 내보낼 전사가 아직 없다 — 실패가 아니다 (§7.2)
 * ready           지금 내보낼 수 있다
 * exporting       파일을 쓰는 중이다
 * done            파일이 만들어졌다 — 어디에 있는지가 값으로 있다 (§4.1)
 * failed          내보내지 못했다 — 원본은 그대로다 (§13 · INV-3)
 * ```
 */
export type ExportPanelBody =
  | { readonly kind: 'loading' }
  | { readonly kind: 'nothingToExport'; readonly text: string; readonly hint: string }
  | { readonly kind: 'ready'; readonly text: string; readonly start: ExportAction }
  | { readonly kind: 'exporting'; readonly text: string }
  | {
      readonly kind: 'done';
      readonly file: ExportedFileView;
      readonly text: string;
      /** 또 내보낼 수 있다. 있던 파일은 그대로 둔 채 하나가 더 생긴다 (§4.3). */
      readonly again: ExportAction;
    }
  | {
      readonly kind: 'failed';
      readonly headline: string;
      /** 실패 그대로 (§13의 세 질문에 대한 답이 이미 이 안에 있다). */
      readonly failure: Failure;
      readonly cause: ExportFailureCause;
      /** 원본이 그대로라는 사실 (INV-3). */
      readonly preservedNotice: string;
      /** 이 갈래에서 먼저 해야 하는 일. 없으면 `null`이다. */
      readonly resolution: string | null;
      /** 실패해도 다시 시도할 수 있다 (§13). */
      readonly retry: ExportAction;
    };

/**
 * Export 자리 전체.
 *
 * 본문과 나란히 **언제나 보이는 것 하나**가 있다 — 무엇이 파일에 들어가는가다 (§11 · INV-6).
 * 본문이 어느 상태든 그 사실은 남는다.
 */
export interface ExportPanelView {
  readonly contents: ExportContentsNotice;
  readonly body: ExportPanelBody;
}

/**
 * {@link exportPanel}이 보는 사실 전부.
 *
 * **여기 없는 것은 내보내기에 영향을 주지 않는다.** AI provider가 없는 것이 그 뜻이다 —
 * provider를 담을 자리가 없으므로 provider 때문에 막히는 상태를 만들 수 없다 (INV-8).
 */
export interface ExportPanelInput {
  /** 아직 읽지 못했으면 `null`이다. */
  readonly recording: Recording | null;
  /**
   * current Transcript의 저장된 노트. **아직 읽지 못했으면 `null`이고 없으면 빈 배열이다.**
   *
   * 이 값이 하는 일은 무엇이 파일에 들어가는지 알려 주는 것뿐이다. 비어 있어도, 읽지
   * 못했어도 내보내기는 그대로 가능하다 (INV-8).
   */
  readonly notes: readonly AiNote[] | null;
  /** 이 화면이 건 내보내기 한 번. */
  readonly attempt: ExportAttempt;
}

/**
 * 읽어 온 값을 Export 자리의 상태로 바꾼다.
 *
 * 순서에 규칙이 하나 있다 — **이 화면이 실제로 건 내보내기의 결과가 저장된 사실보다 먼저다.**
 * 방금 만들어진 파일의 경로는 이 호출에서만 알 수 있고, 저장소는 그것을 모르기 때문이다.
 * 다른 녹음에 대한 내보내기는 이 자리와 아무 상관이 없으므로 보지 않는다.
 */
export function exportPanel(input: ExportPanelInput): ExportPanelView {
  const { recording, notes, attempt } = input;
  const contents = contentsNotice(notes);

  if (recording === null) {
    return { contents, body: { kind: 'loading' } };
  }

  const body = panelBody(recording, mine(attempt, recording.id));
  return { contents, body };
}

/** 이 녹음에 대한 시도만 본다. 다른 녹음의 결과가 이 자리에 보이지 않는다. */
function mine(attempt: ExportAttempt, recordingId: string): ExportAttempt {
  switch (attempt.kind) {
    case 'none':
      return attempt;
    case 'done':
      return attempt.file.recordingId === recordingId ? attempt : NO_EXPORT_ATTEMPT;
    default:
      return attempt.recordingId === recordingId ? attempt : NO_EXPORT_ATTEMPT;
  }
}

function panelBody(recording: Recording, attempt: ExportAttempt): ExportPanelBody {
  if (attempt.kind === 'running') {
    return { kind: 'exporting', text: EXPORTING_TEXT };
  }

  if (attempt.kind === 'done') {
    return {
      kind: 'done',
      file: {
        headline: EXPORT_DONE_HEADLINE,
        fileName: attempt.file.fileName,
        path: attempt.file.path,
      },
      text: EXPORT_DONE_TEXT,
      again: againAction(recording.id),
    };
  }

  if (attempt.kind === 'failed') {
    const cause = failureCause(attempt.failure);
    return {
      kind: 'failed',
      headline: EXPORT_FAILED_HEADLINE,
      failure: attempt.failure,
      cause,
      preservedNotice: EXPORT_PRESERVED_NOTICE,
      resolution: FAILURE_RESOLUTION[cause],
      retry: retryAction(recording.id),
    };
  }

  // 아직 아무것도 하지 않았다. 내보낼 재료가 있는지는 레코드가 말한다 (§7.2).
  if (recording.currentTranscriptId === null) {
    return { kind: 'nothingToExport', text: NOTHING_TO_EXPORT_TEXT, hint: NOTHING_TO_EXPORT_HINT };
  }

  return { kind: 'ready', text: EXPORT_READY_TEXT, start: exportAction(recording.id) };
}
