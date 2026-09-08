/**
 * AI Note 탭의 **두 줄**과, 아래 줄의 세 번째 자리인 **Export for AI**
 * (`phase-prompt/05.5` 요구 3 · 6 · MH-1 · MH-2 · MH-3 · MH-7 ·
 * docs/ADR-0010-manual-ai-handoff.md §5.2 · §5.6 · §7.5 · PRODUCT-SPEC §13).
 *
 * 이 탭에는 노트를 얻는 길이 둘 있고, **둘은 위아래로 놓인다.**
 *
 * ```text
 * 자동으로 만들기   연결된 provider가 이 기기에서 노트를 쓴다        — 선택이다
 * 또는
 * 내 AI로 하기      Copy AI Prompt · Copy Transcript · Export for AI — provider가 없어도 쓴다
 * ```
 *
 * 이 모듈이 답하는 질문은 셋이다 — **무엇을 보여줄 것인가**(두 줄과 그 안의 자리들) ·
 * **무엇이 지금 가능한가**({@link ManualStep.usable} · {@link ManualHandoffView.available}) ·
 * **복사와 export의 결과와 실패를 어떻게 말할 것인가**(각 자리의 본문). React도 DOM도
 * clipboard도 파일시스템도 Tauri도 알지 않으므로 전부 vitest로 그대로 판정된다
 * (§18 · `copyView.ts` · `exportView.ts`와 같은 형태다).
 *
 * ## 아래 줄은 provider 때문에 막힐 수 없다 (MH-1 · MH-2 · INV-8)
 *
 * {@link ManualHandoffInput}에 **AI provider를 담을 자리가 없다.** 담을 자리가 없으므로
 * "provider가 없어서 복사할 수 없다"는 상태를 이 모듈이 만들 수단 자체가 없다 —
 * 그것이 성공 기준 1을 값의 모양으로 못박는 방식이다. 아래 줄이 보는 사실은 하나뿐이다:
 * **복사하고 내보낼 전사가 있는가** (§7.2 · MH-5).
 *
 * ## provider가 없는 것은 위 줄이 비어 있다는 뜻일 뿐이다 (INV-8 · 요구 6)
 *
 * 위 줄의 상태는 `aiNoteView`가 이미 만든 값을 **그대로** 들고 온다 — 기존 Connected Provider
 * 경로(mode 선택 · 생성 · 재생성 · 이력 · provenance · 실패)를 이 모듈이 다시 만들지 않는다
 * (MH-8). 여기서 더하는 것은 **위계 하나**다: 위 줄이 준비돼 있지 않다는 것이 오류도 설정
 * 요구도 아니라는 사실({@link AutomaticRow.optionalNotice})과, 그때에도 아래 줄이 그대로
 * 쓸 수 있다는 사실({@link ManualHandoffView.noProviderNotice})이 값으로 있다.
 *
 * ## 앱이 아무 데도 보내지 않는다 (MH-3)
 *
 * 아래 줄의 세 동작은 전부 이 기기 안에서 끝난다 — clipboard에 쓰거나 파일 하나를 쓴다.
 * 나가는 행위의 주체는 사람이며, 그 사실이 {@link ManualHandoffView.localNotice}로 언제나
 * 함께 있다.
 *
 * ## 크기 때문에 조용히 실패하지 않는다 (`phase-prompt/05.6` 성공 기준 4 · R-5)
 *
 * 세 자리가 **같은 값으로** 크기와 자리를 말한다 — 복사 둘은 `copyView`가, Export for AI는
 * 이 모듈이 같은 두 함수(`handoffSize` · `portionTaken`)를 지난다. 나뉜 문서를 내보내면
 * 조각마다 파일이 하나씩 생기고 **있던 파일은 그대로다** (ADR-0009 §4.3) — 어느 자리에도
 * 잘린 것을 온전한 것이라고 말하는 상태가 없다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { ExportedAiRequest, NoteMode, Recording } from '../ipc/types';
import type { AiNoteTabView, NoteModeChoice } from './aiNoteView';
import { showFile, type ShowFileAttempt, type ShowFileView } from './savedFileView';
import {
  COPY_PROMPT_LABEL,
  COPY_TRANSCRIPT_LABEL,
  FIRST_PORTION,
  copyPanel,
  handoffSize,
  portionTaken,
  type CopyAttempt,
  type CopyBody,
  type CopyPanelView,
  type HandoffSizeView,
  type PortionView,
} from './copyView';

// --- Export for AI 자리 ---------------------------------------------------------------

/**
 * 이 자리에서 사용자가 할 수 있는 동작 하나.
 *
 * **함수가 아니라 값이다** — 순수 모듈이 command를 알지 않기 때문이며, 그래서 "지금 내보낼 수
 * 있는가"·"재시도 수단이 있는가"가 DOM 없이 판정된다. 실제로 부르는 것은 화면 컴포넌트다
 * (`RecordingDetailScreen`의 `exportAiRequest`).
 *
 * `mode`가 실려 있는 이유는 문서가 mode마다 다른 지시를 담기 때문이다 (ADR-0010 §5.2) —
 * 어떤 mode로 만들지를 컴포넌트가 따로 고르지 않는다.
 */
export interface AiExportAction {
  readonly kind: 'export' | 'again' | 'retry' | 'next';
  readonly label: string;
  readonly recordingId: string;
  readonly mode: NoteMode;
  /**
   * 내보낼 조각 (`phase-prompt/05.6` 성공 기준 4). **1부터 센다.**
   *
   * 한 번 내보내면 파일 하나이며, 이 값이 동작에 실려 있기 때문에 "나머지를 마저 파일로
   * 꺼내는 수단"이 화면 값으로 존재한다. **어느 조각을 써도 있던 파일은 그대로다**
   * (ADR-0009 §4.3).
   */
  readonly portion: number;
}

/** 화면에 처음 보이는 이름. Phase Goal이 부르는 이름 그대로다. */
export const AI_EXPORT_LABEL = 'AI용 파일 내보내기';

function exportAction(recordingId: string, mode: NoteMode): AiExportAction {
  return { kind: 'export', label: AI_EXPORT_LABEL, recordingId, mode, portion: FIRST_PORTION };
}

function againAction(recordingId: string, mode: NoteMode, portion: number): AiExportAction {
  // 다시 내보내면 파일이 하나 더 생긴다 — 있던 파일을 덮어쓰지 않는다 (ADR-0009 §4.3).
  return { kind: 'again', label: 'AI용 파일 하나 더 내보내기', recordingId, mode, portion };
}

function retryAction(recordingId: string, mode: NoteMode, portion: number): AiExportAction {
  return { kind: 'retry', label: '내보내기 다시 시도', recordingId, mode, portion };
}

/** 나머지를 마저 꺼내는 수단. **버튼의 이름이 몇 번째를 쓰는지 말한다.** */
function nextAction(
  recordingId: string,
  mode: NoteMode,
  portion: number,
  count: number,
): AiExportAction {
  return {
    kind: 'next',
    label: `Export part ${portion} of ${count}`,
    recordingId,
    mode,
    portion,
  };
}

/**
 * 이 화면이 건 Export for AI 한 번.
 *
 * `exportView`의 것과 같은 규약이다 — **상태를 물어보지 않는다.** `export_ai_request`는 이미
 * 만들어진 파일을 돌려주므로 진행 상황은 이 호출 하나의 결과이며, 그 네 갈래가 이 타입이다.
 * 두 export를 한 상태로 접지 않는 이유는 **한쪽의 결과가 다른 쪽을 가리지 않게** 하기
 * 위해서다 — Markdown export는 여전히 `share` 섹션의 자기 자리에 있다.
 */
export type AiExportAttempt =
  | { readonly kind: 'none' }
  | {
      readonly kind: 'exporting';
      readonly recordingId: string;
      readonly mode: NoteMode;
      readonly portion: number;
    }
  | {
      readonly kind: 'done';
      /**
       * 쓰인 파일과 **그것이 문서의 어디인가** (`phase-prompt/05.6` 성공 기준 4).
       *
       * 파일 하나만 들고 있으면 화면은 나뉜 문서의 한 조각을 문서 전체라고 말하게 된다.
       */
      readonly written: ExportedAiRequest;
    }
  | {
      readonly kind: 'failed';
      readonly recordingId: string;
      /** 쓰려던 조각. 재시도가 **그 조각으로** 돌아가게 하는 값이다. */
      readonly portion: number;
      readonly failure: Failure;
    };

/** 아무것도 하지 않은 상태. 화면이 열렸을 때의 값이다. */
export const NO_AI_EXPORT_ATTEMPT: AiExportAttempt = { kind: 'none' };

/** 내보내기를 시작했을 때 만드는 값. 어느 조각을 쓰러 갔는지 함께 들고 있다. */
export function startedAiExport(
  recordingId: string,
  mode: NoteMode,
  portion: number,
): AiExportAttempt {
  return { kind: 'exporting', recordingId, mode, portion };
}

/** 파일이 만들어졌을 때 만드는 값. 경로도 조각의 자리도 backend가 준 값 그대로다 (§4.1). */
export function exportedAiRequest(written: ExportedAiRequest): AiExportAttempt {
  return { kind: 'done', written };
}

/** 내보내기가 거절됐을 때 만드는 값 (§13). **console로 흘려보내지 않는다.** */
export function failedAiExport(
  recordingId: string,
  portion: number,
  error: unknown,
): AiExportAttempt {
  return { kind: 'failed', recordingId, portion, failure: toFailure(error) };
}

/** 내보낼 재료가 아직 없다. **실패가 아니다** (§7.2 · MH-5). */
export const NOTHING_FOR_AI_TEXT = '이 녹음에서 아직 AI로 보낼 것이 없다.';

/** 그래서 무엇을 하면 되는가. 이 자리가 전사를 시작하지 않는다 — 그 자리는 Transcript 탭이다. */
export const NOTHING_FOR_AI_HINT =
  '전사 탭에서 이 녹음을 먼저 전사한 뒤에 AI로 가져간다.';

/** 지금 눌러도 되는가. 파일 하나를 쓰는 일이므로 기다릴 서버도 모델도 없다. */
export const AI_EXPORT_READY_TEXT =
  '요청과 전사를 담은 파일 하나를 쓴다. 그 파일을 AI 채팅에 첨부하거나 열어서 복사한다.';

/** 쓰는 중. 짧은 일이며 화면을 떠나도 되는 종류의 일이 아니다. */
export const AI_EXPORT_RUNNING_TEXT = 'AI용 파일 쓰는 중…';

/** 파일이 만들어졌다는 사실 한 줄. **색이 아니라 이 문장이 그것을 말한다** (요구 12). */
export const AI_EXPORT_DONE_HEADLINE = 'AI용 파일이 준비됐다.';

/**
 * **조각 하나만 파일이 됐을 때의 사실 한 줄** (`phase-prompt/05.6` 성공 기준 4).
 *
 * 이때 "파일이 준비됐다"고만 말하면 그것은 **잘린 문서를 완전한 것이라고 말하는 것이다.**
 */
export const AI_EXPORT_DONE_PORTION_HEADLINE =
  '조각 하나를 파일로 썼다 — 아직 문서 전체는 아니다.';

/** 그 파일이 어떤 성질인가 — 여기서부터는 사용자의 문서다 (ADR-0009 §4.3). */
export const AI_EXPORT_DONE_TEXT =
  '이 파일은 이제 사용자의 것이다. 다시 내보내면 그 옆에 새 파일이 생기며 이 파일을 덮어쓰지 않는다.';

/** 나뉜 문서의 조각 하나가 파일이 됐다. 나머지도 같은 자리에 **파일이 더 생긴다.** */
export const AI_EXPORT_DONE_PORTION_TEXT =
  '조각마다 파일 하나로 쓰이고, 파일 이름이 어느 조각인지 말한다. 다음 것을 내보내면 이 옆에 새 파일이 생기며 덮어쓰지 않는다.';

/** 마지막 조각까지 왔다. **여기서만 문서 전체가 나왔다고 말한다.** */
export const AI_EXPORT_DONE_LAST_TEXT =
  '이것이 마지막 조각이다. 파일들을 합치면 문서 전체가 된다 — 순서대로 첨부하거나 열어서 복사한다.';

/** 무엇을 하다 실패했는가 (§13). 원인은 {@link Failure}가 말한다. */
export const AI_EXPORT_FAILED_HEADLINE = 'AI용 파일을 쓰지 못했다.';

/**
 * 실패가 무엇을 남겼는지 (§13 · INV-3 · MH-7).
 *
 * **"복구했다"거나 "정리했다"고 말하지 않는다.** 이 경로는 저장소를 읽고 파일 하나를 더할
 * 뿐이므로 (`src-tauri/src/export/run.rs`), 실패했을 때 바뀐 것이 아무것도 없다.
 */
export const AI_EXPORT_PRESERVED_NOTICE =
  '녹음도 오디오 파일도 전사도 AI 노트도, 이미 내보낸 파일도 그대로다. 지워진 것도 바뀐 것도 없다.';

/** 실패 갈래 중 사용자가 **먼저** 할 일이 달라지는 것 (§13). */
export type AiExportFailureCause = 'nothingToExport' | 'storage' | 'other';

const FAILURE_RESOLUTION: Record<AiExportFailureCause, string | null> = {
  nothingToExport: NOTHING_FOR_AI_HINT,
  storage:
    '파일을 쓰지 못했다. 이 기기에 남은 공간이 있는지, 앱이 자기 데이터 폴더에 쓸 수 있는지 확인한 뒤 다시 시도한다.',
  other: null,
};

function failureCause(failure: Failure): AiExportFailureCause {
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
 * Export for AI 자리가 놓일 수 있는 상태의 전부.
 *
 * ```text
 * loading         아직 레코드를 읽지 못했다
 * nothingToExport 내보낼 전사가 아직 없다 — 실패가 아니다 (§7.2)
 * notAsked        지금 내보낼 수 있다 — 아직 아무것도 하지 않았다
 * exporting       파일을 쓰는 중이다
 * done            파일이 만들어졌다 — 어디에 있는지가 값으로 있다 (§4.1)
 * failed          내보내지 못했다 — 원본은 그대로이고, 다시 시도할 수 있다 (§13 · MH-7)
 * ```
 */
export type AiExportBody =
  | { readonly kind: 'loading' }
  | { readonly kind: 'nothingToExport'; readonly text: string; readonly hint: string }
  | { readonly kind: 'notAsked'; readonly text: string; readonly start: AiExportAction }
  | { readonly kind: 'exporting'; readonly text: string }
  | {
      readonly kind: 'done';
      /**
       * 파일이 만들어졌다는 사실. 색이 아니라 이 문장이 그것을 말한다.
       *
       * **조각 하나만 파일이 됐으면 다른 문장이다** — 잘린 문서를 완전한 것이라고 말하지 않는다.
       */
      readonly headline: string;
      readonly fileName: string;
      /** 사용자에게 그대로 보여줄 수 있는 전체 경로. 화면이 만들어 내는 값이 아니다 (§4.1). */
      readonly path: string;
      /**
       * 그 자리를 여는 수단 (`phase-prompt/05.6` 성공 기준 2 · R-4 · `savedFileView.ts`).
       *
       * **{@link path}의 대체가 아니라 추가다** — 만든 파일을 AI 채팅에 첨부하려면 그 파일에
       * 실제로 도달할 수 있어야 하는데, 경로만으로는 도달하지 못한다는 것이 실사용에서
       * 드러났다. 여는 데 실패해도 경로는 그대로 보이며, 그때 무엇을 할 수 있는지도 이 값이
       * 말한다.
       */
      readonly show: ShowFileView;
      readonly text: string;
      /** 문서 전체가 얼마나 큰가 (`phase-prompt/05.6` 성공 기준 4). */
      readonly size: HandoffSizeView;
      /** 이 파일이 문서의 몇 번째 중 몇 번째인가. */
      readonly portion: PortionView;
      /** 또 내보낼 수 있다. **같은 조각이** 파일로 하나 더 생긴다. */
      readonly again: AiExportAction;
      /** 나머지를 마저 파일로 꺼내는 수단. 남은 조각이 없으면 `null`이다. */
      readonly next: AiExportAction | null;
    }
  | {
      readonly kind: 'failed';
      /** 무엇을 하다 실패했는가. 색이 아니라 이 문장이 그것을 말한다 (§13). */
      readonly headline: string;
      /** 실패 그대로 (§13의 세 질문에 대한 답이 이미 이 안에 있다). */
      readonly failure: Failure;
      readonly cause: AiExportFailureCause;
      /** 원본이 그대로라는 사실 (INV-3 · MH-7). */
      readonly preservedNotice: string;
      /** 이 갈래에서 먼저 해야 하는 일. 없으면 `null`이다. */
      readonly resolution: string | null;
      /** 실패해도 다시 시도할 수 있다 (§13 · MH-7). */
      readonly retry: AiExportAction;
    };

/** Export for AI 자리 하나. 복사 자리 둘과 나란히 놓인다. */
export interface AiExportItemView {
  /** 버튼에 처음 보이는 이름. 상태와 무관하게 이 자리가 무엇인지 말한다. */
  readonly label: string;
  readonly body: AiExportBody;
}

function aiExportItem(
  recording: Recording | null,
  mode: NoteMode,
  attempt: AiExportAttempt,
  show: ShowFileAttempt,
): AiExportItemView {
  if (recording === null) {
    return { label: AI_EXPORT_LABEL, body: { kind: 'loading' } };
  }
  return {
    label: AI_EXPORT_LABEL,
    body: aiExportBody(recording, mode, mine(attempt, recording.id), show),
  };
}

/** 이 녹음에 대한 시도만 본다. 다른 녹음의 결과가 이 자리에 보이지 않는다. */
function mine(attempt: AiExportAttempt, recordingId: string): AiExportAttempt {
  switch (attempt.kind) {
    case 'none':
      return attempt;
    case 'done':
      return attempt.written.file.recordingId === recordingId ? attempt : NO_AI_EXPORT_ATTEMPT;
    default:
      return attempt.recordingId === recordingId ? attempt : NO_AI_EXPORT_ATTEMPT;
  }
}

function aiExportBody(
  recording: Recording,
  mode: NoteMode,
  attempt: AiExportAttempt,
  show: ShowFileAttempt,
): AiExportBody {
  if (attempt.kind === 'exporting') {
    return { kind: 'exporting', text: AI_EXPORT_RUNNING_TEXT };
  }

  if (attempt.kind === 'done') {
    const { file } = attempt.written;
    const portion = portionTaken(attempt.written);

    return {
      kind: 'done',
      // 잘린 문서를 완전한 것이라고 말하지 않는다 (`phase-prompt/05.6` 성공 기준 4).
      headline: portion.whole ? AI_EXPORT_DONE_HEADLINE : AI_EXPORT_DONE_PORTION_HEADLINE,
      fileName: file.fileName,
      path: file.path,
      // 경로를 보여 주는 것에 **더해** 그 자리를 여는 수단이 있다 (성공 기준 2 · R-4).
      show: showFile(file.path, show),
      text: portion.whole
        ? AI_EXPORT_DONE_TEXT
        : portion.remaining === 0
          ? AI_EXPORT_DONE_LAST_TEXT
          : AI_EXPORT_DONE_PORTION_TEXT,
      size: handoffSize(attempt.written),
      portion,
      again: againAction(recording.id, mode, portion.index),
      // 남은 것이 있을 때만 다음 조각을 가리킨다. 없는 조각을 쓰라는 버튼을 만들지 않는다.
      next:
        portion.remaining === 0
          ? null
          : nextAction(recording.id, mode, portion.index + 1, portion.count),
    };
  }

  if (attempt.kind === 'failed') {
    const cause = failureCause(attempt.failure);
    return {
      kind: 'failed',
      headline: AI_EXPORT_FAILED_HEADLINE,
      failure: attempt.failure,
      cause,
      preservedNotice: AI_EXPORT_PRESERVED_NOTICE,
      resolution: FAILURE_RESOLUTION[cause],
      // 쓰려던 **그 조각을** 다시 쓴다 — 건너뛴 조각은 사용자가 알아채지 못한 채 빠진다.
      retry: retryAction(recording.id, mode, attempt.portion),
    };
  }

  // 아직 아무것도 하지 않았다. 내보낼 재료가 있는지는 레코드가 말한다 (§7.2 · MH-5).
  if (recording.currentTranscriptId === null) {
    return { kind: 'nothingToExport', text: NOTHING_FOR_AI_TEXT, hint: NOTHING_FOR_AI_HINT };
  }

  return { kind: 'notAsked', text: AI_EXPORT_READY_TEXT, start: exportAction(recording.id, mode) };
}

// --- 아래 줄 — 내 AI로 하기 ------------------------------------------------------------

/** 아래 줄의 세 자리. 순서도 이 순서다 (요구 6). */
export type ManualStepKey = 'prompt' | 'transcript' | 'export';

/**
 * 세 자리 각각이 **지금 눌릴 수 있는가**.
 *
 * 세 갈래를 한 값으로 모으는 이유는 하나다 — "provider가 하나도 없어도 셋 다 쓸 수 있다"가
 * 화면을 그리지 않고 판정돼야 하기 때문이다 (MH-1 · MH-2 · 성공 기준 1). `usable`은 지금 누를
 * 버튼이 실제로 그 자리에 있다는 뜻이며, 그 판정의 근거는 각 자리의 본문이다.
 */
export interface ManualStep {
  readonly key: ManualStepKey;
  readonly label: string;
  readonly usable: boolean;
}

/** 복사 자리 하나가 지금 누를 수 있는 상태인가. */
function copyUsable(body: CopyBody): boolean {
  return body.kind === 'notAsked' || body.kind === 'copied' || body.kind === 'failed';
}

/** Export for AI 자리가 지금 누를 수 있는 상태인가. */
function exportUsable(body: AiExportBody): boolean {
  return body.kind === 'notAsked' || body.kind === 'done' || body.kind === 'failed';
}

/** 아래 줄의 이름. 이 줄이 무엇인지 한 마디로 말한다 (요구 6). */
export const MANUAL_HEADING = '쓰던 AI 그대로 쓰기';

/** 이 줄이 하는 일. 벤더 이름을 부르지 않는다 (MH-6). */
export const MANUAL_TEXT =
  '이 녹음을 쓰던 AI 채팅으로 가져간다 — 복사하거나, 첨부할 수 있는 파일로 쓴다.';

/**
 * **AI provider를 하나도 설정하지 않아도 이 줄은 그대로다** (MH-1 · MH-2 · INV-8).
 *
 * 위 줄이 비어 있는 것을 아래 줄의 결함으로 읽지 않게 하는 문장이며, 세 자리가 어느 상태든
 * 같은 자리에 있다.
 */
export const MANUAL_NO_PROVIDER_NOTICE =
  '이 셋은 AI provider를 준비하지 않아도 동작한다. 먼저 설치할 것이 없다.';

/** 앱이 아무 데도 보내지 않는다 (MH-3 · INV-6). 나가는 행위의 주체는 사람이다. */
export const MANUAL_LOCAL_NOTICE =
  '여기서는 어디로도 보내지 않는다. 텍스트는 클립보드나 이 기기의 파일로 갈 뿐이고, 그다음 어디로 갈지는 사용자가 정한다. 오디오 파일은 절대 포함되지 않는다.';

/** 아래 줄 전체 — 세 자리와, 그 셋에 함께 해당하는 사실들. */
export interface ManualHandoffView {
  readonly heading: string;
  readonly text: string;
  /**
   * 지금 이 줄을 쓸 수 있는가.
   *
   * **provider는 이 값에 끼어들 수 없다** — {@link ManualHandoffInput}에 provider를 담을 자리가
   * 없기 때문이다. 이 값이 거짓이 되는 경우는 하나뿐이다: 아직 복사하고 내보낼 전사가 없다
   * (§7.2 · MH-5).
   */
  readonly available: boolean;
  /** 세 자리 각각이 지금 눌릴 수 있는가. 순서는 화면에 놓이는 순서 그대로다. */
  readonly steps: readonly ManualStep[];
  /** provider가 없어도 된다는 사실 (MH-1 · MH-2). */
  readonly noProviderNotice: string;
  /** 앱이 아무 데도 보내지 않는다는 사실 (MH-3 · INV-6). */
  readonly localNotice: string;
  /** Copy AI Prompt · Copy Transcript 두 자리 (`copyView.ts`). */
  readonly copy: CopyPanelView;
  /** Export for AI 자리. */
  readonly aiExport: AiExportItemView;
}

/**
 * {@link manualHandoff}가 보는 사실 전부.
 *
 * **여기 없는 것은 이 줄에 영향을 주지 않는다.** AI provider가 그것이다 — provider를 담을
 * 자리가 없으므로 provider 때문에 막히는 상태를 만들 수단이 없다 (MH-1 · MH-2 · INV-8).
 */
export interface ManualHandoffInput {
  /** 아직 읽지 못했으면 `null`이다. */
  readonly recording: Recording | null;
  /** 지금 고른 mode (§9.5). 프롬프트와 AI-ready 문서가 이 값으로 만들어진다. */
  readonly mode: NoteMode;
  /** 이 화면이 건 복사 한 번. 두 복사 자리 중 하나에만 해당한다. */
  readonly copy: CopyAttempt;
  /** 이 화면이 건 Export for AI 한 번. */
  readonly aiExport: AiExportAttempt;
  /**
   * 이 화면이 건 **자리 열기** 한 번 (`phase-prompt/05.6` 성공 기준 2 · R-4).
   *
   * **AI provider와 아무 상관이 없다** — 이 값이 가리키는 것은 이미 만들어진 파일 하나이며,
   * 그것을 여는 데 실패해도 파일과 경로는 그대로다. 다른 파일에 대한 시도는 이 자리에 보이지
   * 않는다 (`savedFileView.ts`의 `showFile`).
   */
  readonly show: ShowFileAttempt;
}

/** 읽어 온 값을 아래 줄의 상태로 바꾼다. */
export function manualHandoff(input: ManualHandoffInput): ManualHandoffView {
  const { recording, mode } = input;

  const copy = copyPanel({ recording, mode, attempt: input.copy });
  const aiExport = aiExportItem(recording, mode, input.aiExport, input.show);

  const steps: readonly ManualStep[] = [
    { key: 'prompt', label: COPY_PROMPT_LABEL, usable: copyUsable(copy.prompt.body) },
    { key: 'transcript', label: COPY_TRANSCRIPT_LABEL, usable: copyUsable(copy.transcript.body) },
    { key: 'export', label: AI_EXPORT_LABEL, usable: exportUsable(aiExport.body) },
  ];

  return {
    heading: MANUAL_HEADING,
    text: MANUAL_TEXT,
    // 재료가 있으면 쓸 수 있다. provider는 이 판정에 들어오지 않는다 (MH-1 · MH-2).
    available: recording !== null && recording.currentTranscriptId !== null,
    steps,
    noProviderNotice: MANUAL_NO_PROVIDER_NOTICE,
    localNotice: MANUAL_LOCAL_NOTICE,
    copy,
    aiExport,
  };
}

// --- 두 줄의 위계 ----------------------------------------------------------------------

/** 위 줄의 이름 (요구 6). */
export const AUTOMATIC_HEADING = '앱이 대신 쓰게 하기';

/** 이 줄이 하는 일. 벤더 이름을 부르지 않는다 — 이름은 provider가 말한다 (INV-9 · MH-6). */
export const AUTOMATIC_TEXT =
  'A connected AI provider turns this transcript into a structured note, right here.';

/**
 * 위 줄이 지금 준비돼 있지 않다는 사실 (INV-8 · 요구 6).
 *
 * **설정하라는 요구가 아니다.** 이 줄이 비어 있는 것은 정상 상태이며, 그때 사용자가 할 일은
 * 아무것도 없다 — 아래 줄이 그대로 있기 때문이다. 켜고 싶은 사람을 위한 안내는 여전히 본문의
 * `resolution` 한 줄에 있고, 이 문장이 그 옆에서 **그것이 선택이라는 사실**을 말한다.
 */
export const AUTOMATIC_OPTIONAL_NOTICE =
  '이 부분은 선택이다. 건너뛰고 아래에서 쓰던 AI를 써도 된다 — 빠진 것도 고장 난 것도 아니다.';

/** 두 줄 사이. 둘 중 하나를 고르는 것이지 순서대로 해야 하는 일이 아니다. */
export const HANDOFF_OR_TEXT = 'or';

/** 위 줄 — 연결된 provider로 만드는 길. */
export interface AutomaticRow {
  readonly heading: string;
  readonly text: string;
  /**
   * 지금 이 줄로 노트를 만들거나 볼 수 있는가.
   *
   * 거짓인 것은 실패가 아니다 — 아직 읽지 못했거나(`loading`), provider가 준비되지 않았거나
   * (`disabled`), 만들 재료가 없다는(`noTranscript`) 뜻이다.
   */
  readonly available: boolean;
  /**
   * provider가 준비되지 않은 것이 **오류도 설정 요구도 아니라는 사실** (INV-8 · 요구 6).
   *
   * `disabled`일 때만 있다. 아직 읽지 못했거나 만들 재료가 없는 것은 provider와 상관없는
   * 사실이며 (그때는 아래 줄도 같은 이유로 기다린다), 그 상태에 "건너뛰어도 된다"고 말하면
   * 사실이 아닌 것을 말하게 된다.
   */
  readonly optionalNotice: string | null;
  /**
   * 기존 AI Note 탭의 상태 그대로 (`aiNoteView.ts`).
   *
   * mode 선택 · 생성 · 재생성 · 이력 · provenance · 실패 표시가 전부 이 값 안에 그대로 있다.
   * 이 모듈은 그것을 **다시 만들지 않는다** (MH-8).
   */
  readonly tab: AiNoteTabView;
}

/**
 * AI Note 탭 전체의 위계 — 위 줄과 아래 줄.
 *
 * 탭 구성(`['AI Note', 'Transcript', 'Recording']`)도, `share` 섹션(Export Markdown ·
 * Send to Notion)도 이 값과 상관이 없다 — 이 모듈이 바꾸는 것은 **AI Note 탭 안의 위계
 * 하나뿐이다** (R-2).
 */
export interface AiNoteTabLayout {
  /**
   * 세 mode (§9.5). **두 줄이 같은 것을 쓴다** — 프롬프트와 AI-ready 문서도 이 mode로
   * 만들어지므로, mode 선택은 어느 한 줄의 소유물이 아니라 탭의 것이다.
   */
  readonly modes: readonly NoteModeChoice[];
  /**
   * 지금 mode를 바꿀 수 있는가.
   *
   * **provider가 없다는 이유로 잠기지 않는다.** 위 줄이 비어 있어도 아래 줄이 이 mode를
   * 쓰기 때문이다 (MH-1 · MH-2) — 잠기는 것은 지금 노트를 만드는 중이거나 아직 아무것도
   * 읽지 못했을 때다.
   */
  readonly modeSelectable: boolean;
  readonly automatic: AutomaticRow;
  /** 두 줄 사이의 접속사. 위 줄이 조건이 아니라는 것을 한 마디로 말한다. */
  readonly orText: string;
  readonly manual: ManualHandoffView;
}

/** 위 줄이 지금 쓸 수 있는 상태인가. */
function automaticAvailable(tab: AiNoteTabView): boolean {
  switch (tab.body.kind) {
    case 'loading':
    case 'disabled':
    case 'noTranscript':
      return false;
    default:
      return true;
  }
}

/**
 * 이미 만들어진 두 값을 **위아래로 놓는다.**
 *
 * 위 줄의 상태를 여기서 다시 판정하지 않는다 — `aiNoteTab`이 만든 값을 그대로 들고 온다
 * (MH-8). 아래 줄도 마찬가지로 {@link manualHandoff}가 만든 값 그대로다. 이 함수가 더하는
 * 것은 **두 줄의 관계**뿐이다: 위가 비어 있어도 아래는 그대로라는 것.
 */
export function aiNoteTabLayout(tab: AiNoteTabView, manual: ManualHandoffView): AiNoteTabLayout {
  const available = automaticAvailable(tab);
  return {
    modes: tab.modes,
    // 위 줄이 잠겨 있어도 아래 줄이 이 mode를 쓴다. 다만 노트를 만드는 중에는 바꾸지 않는다 —
    // 그때 보이는 것은 그 mode의 상태이기 때문이다 (ADR-0008 §9.2).
    modeSelectable: tab.modeSelectable || (tab.body.kind !== 'generating' && manual.available),
    automatic: {
      heading: AUTOMATIC_HEADING,
      text: AUTOMATIC_TEXT,
      available,
      optionalNotice: tab.body.kind === 'disabled' ? AUTOMATIC_OPTIONAL_NOTICE : null,
      tab,
    },
    orText: HANDOFF_OR_TEXT,
    manual,
  };
}
