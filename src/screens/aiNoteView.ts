/**
 * Recording Detail의 **AI Note 탭** 상태 (PRODUCT-SPEC §5 C · §9 · §13 ·
 * `phase-prompt/04` 요구 2 · 4 · 5 · 11 · 12 · 14 · 15 · 16).
 *
 * 이 탭이 답해야 하는 질문은 세 가지다 — **AI 기능을 지금 쓸 수 있는가** ·
 * **이 녹음의 노트가 지금 어떤 상태인가** · **보고 있는 노트는 무엇으로 언제 만들어졌는가**.
 * 셋 다 backend가 준 사실에서 나오며, 그 사실을 화면 상태로 옮기는 규칙이 전부 여기 있다.
 * React도 DOM도 Tauri도 알지 않으므로 일곱 경로(아직 읽지 못함 · AI 비활성 · 전사 없음 ·
 * 노트 없음 · 생성 중 · 완료 · 실패)를 **Ollama도 모델도 네트워크도 없이** vitest로 그대로
 * 판정할 수 있다 (§18 · `transcriptView.ts`와 같은 형태다).
 *
 * ## 렌더링은 한 방향으로만 흐른다 — `Structured Note → UI` (§9.3 · INV-9)
 *
 * Provider가 무엇을 돌려줬든 저장되는 것은 provider 중립 데이터이며 (`StructuredNote`),
 * 화면이 읽는 것도 그 구조다. **Markdown 문자열을 받아 그대로 뿌리는 경로가 이 모듈에 없다** —
 * 여기서 만드는 것은 `{@link NoteSection}`의 목록이고, 섹션 제목은 §9.5의 표에서 온다.
 * 그래서 같은 노트를 Phase 5의 Markdown · Notion renderer가 다시 소비할 수 있다.
 *
 * ## provider가 없는 것은 실패가 아니다 (INV-8 · §13)
 *
 * provider를 고르지 않았거나 지금 닿지 못하는 것은 **정상 상태다.** 이 모듈은 그것을
 * {@link AiNoteTabBody}의 `disabled`로 만들며, 그 값에는 `failure`가 없다 — 화면이 실패로
 * 그릴 수 있는 재료 자체를 주지 않는다. 그리고 그 상태는 이 탭 밖으로 나가지 않는다:
 * 이 모듈에는 재생이나 Transcript 탭에 닿는 값이 하나도 없다.
 *
 * ## 시각 문자열을 여기서 만들지 않는다
 *
 * `generatedAt`은 backend가 준 ISO-8601 텍스트 그대로 나간다. 녹음 길이 표시가 Rust 한 곳에만
 * 있는 것과 같은 이유다 (`tests/screen-boundary.test.ts`) — 표기 규칙이 두 벌이 되면 조용히
 * 갈라진다. 노트의 provenance는 **언제 만들어졌는지를 잃지 않는 것**이 목적이므로 저장된 값을
 * 그대로 보이는 편이 맞다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type {
  AiNote,
  AiNoteStatus,
  AiProviderLocality,
  AiProviderState,
  AiProviderStatus,
  NoteMode,
  Recording,
  StructuredNote,
  Transcript,
} from '../ipc/types';

/** 고를 수 있는 mode 셋 (§9.5). 표시 순서도 이 순서다. */
export const NOTE_MODES = ['meeting', 'study', 'summary'] as const;

/** 버튼에 적히는 이름. */
const MODE_LABEL: Record<NoteMode, string> = {
  meeting: 'Meeting',
  study: 'Study',
  summary: 'Summary',
};

/**
 * 그 mode가 만들어 내는 섹션 (§9.5의 표 그대로).
 *
 * 고르기 전에 무엇이 나오는지 알 수 있어야 Meeting과 Study를 구분해 고를 수 있다.
 */
const MODE_SECTIONS: Record<NoteMode, string> = {
  meeting: 'Overview · Key Discussions · Decisions · Action Items · Open Questions',
  study:
    'Overview · Key Concepts · Important Details · Questions · Things to Study · References Mentioned',
  summary: 'Short Summary · Key Points',
};

/** 고를 수 있는 mode 하나 (`phase-prompt/04` 요구 4). */
export interface NoteModeChoice {
  readonly mode: NoteMode;
  readonly label: string;
  /** 이 mode가 만들어 내는 섹션 (§9.5). */
  readonly sections: string;
  readonly selected: boolean;
}

/** 세 mode를 표시 순서대로, 고른 것 하나를 표시해서 내놓는다. */
export function noteModeChoices(selected: NoteMode): readonly NoteModeChoice[] {
  return NOTE_MODES.map((mode) => ({
    mode,
    label: MODE_LABEL[mode],
    sections: MODE_SECTIONS[mode],
    selected: mode === selected,
  }));
}

/**
 * 노트 본문의 섹션 하나.
 *
 * `text`와 `list`를 하나로 접지 않는 이유는 §9.5가 둘을 다르게 정의하기 때문이다 —
 * Overview는 문단이고 Decisions는 항목이다. 화면이 문단을 목록처럼 그리지 않게 하려면
 * 그 구분이 값으로 있어야 한다.
 */
export type NoteSectionKind = 'text' | 'list';

export interface NoteSection {
  /** §9.5의 섹션 이름 그대로. */
  readonly title: string;
  readonly kind: NoteSectionKind;
  /** `text` 섹션의 본문. `list` 섹션에서는 `null`이다. */
  readonly text: string | null;
  /** `list` 섹션의 항목. `text` 섹션에서는 빈 배열이다. */
  readonly items: readonly string[];
  /**
   * 비어 있다는 사실을 담담하게 알리는 문장. 비어 있지 않으면 `null`이다.
   *
   * **비어 있는 것은 실패가 아니다** — "결정된 것이 없었다"는 실제 결과이며
   * (docs/ADR-0008-note-ai-provider.md §7.3), 화면이 그것을 오류로 그리지 않는다.
   */
  readonly emptyText: string | null;
}

/** 섹션이 비었을 때. 지어내지 않고 비었다고 말한다. */
export const EMPTY_SECTION_TEXT = 'Nothing was recorded in this section.';

function textSection(title: string, value: string): NoteSection {
  const text = value.trim();
  return {
    title,
    kind: 'text',
    text: text.length === 0 ? null : text,
    items: [],
    emptyText: text.length === 0 ? EMPTY_SECTION_TEXT : null,
  };
}

function listSection(title: string, values: readonly string[]): NoteSection {
  return {
    title,
    kind: 'list',
    text: null,
    items: values,
    emptyText: values.length === 0 ? EMPTY_SECTION_TEXT : null,
  };
}

/**
 * 구조화된 노트 하나를 화면에 놓을 섹션으로 옮긴다 (§9.3 · §9.5).
 *
 * **여기가 `Structured Note → UI renderer`의 유일한 통로다.** 섹션의 이름도 순서도 §9.5의
 * 표에서 오며, provider가 무엇을 돌려줬는지는 이 함수가 알지 않는다 (INV-9). mode마다 필드가
 * 다르므로 `mode`로 갈라 읽는다 — 그 갈래가 타입에 있기 때문에 "Meeting인데 Things to Study가
 * 있는" 값을 그릴 수 없다.
 */
export function noteSections(note: StructuredNote): readonly NoteSection[] {
  switch (note.mode) {
    case 'meeting':
      return [
        textSection('Overview', note.overview),
        listSection('Key Discussions', note.keyDiscussions),
        listSection('Decisions', note.decisions),
        listSection('Action Items', note.actionItems),
        listSection('Open Questions', note.openQuestions),
      ];
    case 'study':
      return [
        textSection('Overview', note.overview),
        listSection('Key Concepts', note.keyConcepts),
        listSection('Important Details', note.importantDetails),
        listSection('Questions', note.questions),
        listSection('Things to Study', note.thingsToStudy),
        listSection('References Mentioned', note.referencesMentioned),
      ];
    case 'summary':
      return [
        textSection('Short Summary', note.shortSummary),
        listSection('Key Points', note.keyPoints),
      ];
  }
}

/**
 * 이 노트가 **어떤 Transcript version에서 무엇으로 언제** 만들어졌는가 (§7.3 · §9.6).
 *
 * 네 값 전부가 화면에 남는 이유는 하나다 — 프롬프트를 바꾼 뒤 "전보다 나아졌는가"를 물으려면
 * 지금 보고 있는 노트가 어느 `promptVersion`의 것인지 보여야 한다 (ADR-0008 §10).
 * 재생성이 이전 노트를 덮어쓰지 않으므로 (§9.2) 이 값들은 서로 다른 노트를 구분하는 이름이기도 하다.
 */
export interface NoteProvenance {
  readonly provider: string;
  readonly model: string;
  readonly promptVersion: string;
  /** ISO-8601 UTC 텍스트 그대로. 이 모듈이 시각 표기를 새로 만들지 않는다. */
  readonly generatedAt: string;
  /** 입력이 된 Transcript (§7.3). 한 녹음에 Transcript가 여럿일 수 있다 (§7.1). */
  readonly transcriptId: string;
  /** 한 줄로 읽는 형태. 값은 위의 필드 그대로이며 여기서 새로 만드는 사실이 없다. */
  readonly label: string;
}

function noteProvenance(note: AiNote): NoteProvenance {
  return {
    provider: note.provider,
    model: note.model,
    promptVersion: note.promptVersion,
    generatedAt: note.generatedAt,
    transcriptId: note.transcriptId,
    label: `${note.provider} · ${note.model} · prompt ${note.promptVersion} · ${note.generatedAt} · transcript ${note.transcriptId}`,
  };
}

/** 화면에 놓이는 노트 하나 — 구조와 provenance가 함께 있다. */
export interface NoteView {
  readonly mode: NoteMode;
  readonly modeLabel: string;
  readonly sections: readonly NoteSection[];
  readonly provenance: NoteProvenance;
}

/** 저장된 노트 하나를 화면에 놓을 값으로 옮긴다. */
export function noteView(note: AiNote): NoteView {
  return {
    mode: note.mode,
    modeLabel: MODE_LABEL[note.mode],
    sections: noteSections(note.note),
    provenance: noteProvenance(note),
  };
}

/**
 * 그 Transcript의 그 mode에서 **가장 최근에 만들어진** 노트 (ADR-0008 §9.2).
 *
 * 재생성은 대체가 아니라 추가이므로 같은 mode의 노트가 여럿일 수 있다. `list_ai_notes`가
 * `(generated_at, id)` 순으로 주므로 **마지막으로 맞은 것이 가장 최근이다** — 여기서 다시
 * 정렬하지 않는다. 이전 노트는 사라지지 않으며, 이 함수가 그것을 지우지도 않는다.
 *
 * `transcriptId`로 한 번 더 거르는 이유는 provenance 때문이다. 다른 Transcript에서 나온 노트를
 * 지금 보고 있는 Transcript의 노트로 보여 주면 §7.3이 답하려던 질문이 답해지지 않는다.
 */
export function latestNote(
  notes: readonly AiNote[],
  transcriptId: string,
  mode: NoteMode,
): AiNote | null {
  let latest: AiNote | null = null;
  for (const note of notes) {
    if (note.transcriptId === transcriptId && note.mode === mode) {
      latest = note;
    }
  }
  return latest;
}

/**
 * 이 탭에서 사용자가 할 수 있는 동작 하나.
 *
 * **함수가 아니라 값이다** — 순수 모듈이 command를 알지 않기 때문이며, 그래서 "지금 만들 수
 * 있는가"·"재시도 수단이 있는가"가 DOM 없이 판정된다. 실제로 부르는 것은 화면 컴포넌트다
 * (`RecordingDetailScreen`의 `startAiNote`).
 *
 * 셋을 한 종류로 접지 않는 이유는 사용자에게 다른 상황이기 때문이다 — 처음 만드는 것 ·
 * 이미 있는 노트를 다시 만드는 것 · 실패한 뒤 다시 하는 것.
 */
export interface AiNoteAction {
  readonly kind: 'generate' | 'regenerate' | 'retry';
  readonly label: string;
  readonly recordingId: string;
  readonly mode: NoteMode;
}

function generateAction(recordingId: string, mode: NoteMode): AiNoteAction {
  return { kind: 'generate', label: `Generate ${MODE_LABEL[mode]} note`, recordingId, mode };
}

function regenerateAction(recordingId: string, mode: NoteMode): AiNoteAction {
  return { kind: 'regenerate', label: `Generate ${MODE_LABEL[mode]} note again`, recordingId, mode };
}

function retryAction(recordingId: string, mode: NoteMode): AiNoteAction {
  return { kind: 'retry', label: `Try the ${MODE_LABEL[mode]} note again`, recordingId, mode };
}

/**
 * 고른 provider가 무엇이고 **전사가 이 기기를 떠나는가** (§12 · INV-5 · 요구 15).
 *
 * provider를 고르지 않았으면 이 값은 `null`이다 — 어디로도 가지 않으므로 할 말이 없다.
 * 이름을 이 모듈이 정하지 않는다: `providerName`은 provider 자신이 말한 값이다 (INV-9).
 */
export interface NoteProviderNotice {
  readonly name: string;
  readonly locality: AiProviderLocality;
  readonly label: string;
}

/** **audio는 어느 쪽에서도 전송되지 않는다** (INV-6). 나가는 것은 전사 텍스트뿐이다 (§9.6). */
const LOCALITY_TEXT: Record<AiProviderLocality, string> = {
  local: 'runs on this device — the transcript text does not leave it. Audio is never sent.',
  external:
    'runs outside this device — the transcript text is sent to it. Audio is never sent (INV-6).',
};

function providerNotice(status: AiProviderStatus): NoteProviderNotice | null {
  if (status.providerName === null || status.locality === null) {
    return null;
  }
  return {
    name: status.providerName,
    locality: status.locality,
    label: `${status.providerName} · ${LOCALITY_TEXT[status.locality]}`,
  };
}

/** AI 기능이 켜지지 않은 갈래 — {@link AiProviderState}에서 `ready`를 뺀 셋이다. */
export type AiDisabledState = Exclude<AiProviderState, 'ready'>;

/**
 * AI 기능이 비활성이라는 사실 (INV-8 · §13).
 *
 * **`failure`가 없다.** 경고도 오류도 아니기 때문이며, 화면이 실패로 그릴 재료를 아예 주지
 * 않는 것이 이 타입의 목적이다. 대신 세 가지가 값으로 있다 — 지금 어떤 상태인가(`text`) ·
 * 켜려면 무엇을 하면 되는가(`resolution`) · **무엇이 막히지 않았는가**(`unaffectedNotice`).
 */
export interface AiDisabledNotice {
  readonly state: AiDisabledState;
  /** 담담한 사실 한 줄. */
  readonly headline: string;
  readonly text: string;
  /** 켜고 싶다면 무엇을 하면 되는가. 재촉이 아니라 안내다. */
  readonly resolution: string;
  /** provider를 고르지 않았으면 `null`이다. */
  readonly providerName: string | null;
  /** 이 상태가 **막지 않는** 것 (INV-8). */
  readonly unaffectedNotice: string;
  /**
   * 다시 확인해 볼 수단. provider를 고르지 않은 상태에서는 `null`이다 — 확인할 대상 자체가
   * 없으며, 그 상태에서 풀리는 길은 Settings에서 provider를 고르는 것뿐이다.
   */
  readonly recheck: AiRecheck | null;
}

/** provider 상태를 다시 물어보는 동작. **실패의 재시도가 아니다.** */
export interface AiRecheck {
  readonly label: string;
}

/** AI 기능이 꺼져 있어도 나머지는 그대로다 (INV-8 · `phase-prompt/04` 성공 기준 2). */
export const AI_UNAFFECTED_NOTICE =
  'Recording, playback, and the transcript are not affected by this.';

/** 담담한 한 줄. 이 문장이 이 상태의 전부이며, 경고 문구를 덧붙이지 않는다. */
export const AI_DISABLED_HEADLINE = 'AI notes are off.';

const DISABLED_TEXT: Record<AiDisabledState, string> = {
  notConfigured: 'No AI provider is set up yet.',
  unavailable: 'The AI provider is not answering right now.',
  noModels: 'The AI provider has no models installed.',
};

const DISABLED_RESOLUTION: Record<AiDisabledState, string> = {
  notConfigured: 'Choose an AI provider in Settings to turn AI notes on.',
  unavailable:
    'Start the AI provider (or check its address in Settings), then check again from here.',
  noModels: 'Install a model for the AI provider, then check again from here.',
};

function disabledNotice(state: AiDisabledState, providerName: string | null): AiDisabledNotice {
  return {
    state,
    headline: AI_DISABLED_HEADLINE,
    text: DISABLED_TEXT[state],
    resolution: DISABLED_RESOLUTION[state],
    providerName,
    unaffectedNotice: AI_UNAFFECTED_NOTICE,
    // 고른 provider가 없으면 다시 물어볼 대상도 없다.
    recheck: state === 'notConfigured' ? null : { label: 'Check the AI provider again' },
  };
}

/**
 * 노트 생성이 실패한 이유 중 **사용자가 할 일이 달라지는 갈래** (§13 · ADR-0008 §13.1).
 *
 * ```text
 * unreachable        provider에 닿지 못했다            provider를 켜고 다시 시도한다
 * modelUnavailable   고른 모델이 그 서버에 없다        모델을 받아 오거나 다른 모델을 고른다
 * responseUnusable   응답이 기대한 모양이 아니었다     다시 만들면 달라질 수 있다
 * inputTooLarge      요청을 아예 보내지 않았다         더 큰 context나 더 짧은 녹음이 필요하다
 * requestFailed      요청이 거절됐다                   다시 시도할 수 있다
 * other              그 밖의 실패                      다시 시도할 수 있다
 * unknown            이 앱이 켜진 뒤의 시도가 아니다   이유를 지어내지 않는다
 * ```
 *
 * **`aiProviderNotConfigured`가 이 목록에 없다.** 그것은 실패로 그리지 않는 상태이며
 * ({@link AiDisabledNotice}), 여기까지 오지 않는다 (§13 · INV-8).
 */
export type AiNoteFailureCause =
  | 'unreachable'
  | 'modelUnavailable'
  | 'responseUnusable'
  | 'inputTooLarge'
  | 'requestFailed'
  | 'other'
  | 'unknown';

/** 무엇을 하다 실패했는가 (§13). 원인은 {@link Failure}가 말한다. */
export const AI_NOTE_FAILED_HEADLINE = 'This AI note could not be generated.';

/**
 * 실패가 무엇을 남겼는지 (§13 · INV-1 · INV-2 · INV-3).
 *
 * **"복구했다"거나 "정리했다"고 말하지 않는다.** AI 경로는 audio도 Transcript도 쓰지 않으므로
 * 실패했을 때 바뀌는 것은 녹음 하나의 AI 상태뿐이다 (ADR-0008 §9.4 · §13.2).
 */
export const AI_NOTE_PRESERVED_NOTICE =
  'The recording, its audio file, and the transcript are untouched, and any note you already had is kept as it is. Nothing was deleted.';

/** 이유를 모를 때 그 사실을 그대로 말한다. **무엇이 실패했는지 지어내지 않는다.** */
export const UNKNOWN_AI_NOTE_NOTICE =
  'The stored state says the last AI note failed. The reason is not known in this session — generate it again to see what happens.';

/** 갈래마다 사용자가 **먼저** 해야 하는 일 (§13). 없으면 `null`이다. */
const FAILURE_RESOLUTION: Record<AiNoteFailureCause, string | null> = {
  unreachable: 'The AI provider did not answer. Start it, then try again.',
  modelUnavailable:
    'The chosen model is not installed on the AI provider. Install it or choose another model in Settings, then try again.',
  responseUnusable:
    'The model answered in a shape this app could not read. Generating again often gives a usable answer.',
  inputTooLarge:
    'This transcript is longer than the model can take in one request, so nothing was sent. A model with a larger context window, or a shorter recording, is needed.',
  requestFailed: null,
  other: null,
  unknown: UNKNOWN_AI_NOTE_NOTICE,
};

/** 실패 종류를 사용자가 할 일 기준으로 나눈다. Rust가 나눠 보낸 구분을 뭉개지 않는다. */
function failureCause(failure: Failure | null): AiNoteFailureCause {
  if (failure === null) {
    return 'unknown';
  }
  switch (failure.kind) {
    case 'aiProviderUnreachable':
      return 'unreachable';
    case 'aiModelUnavailable':
      return 'modelUnavailable';
    case 'aiResponseUnusable':
      return 'responseUnusable';
    case 'aiInputTooLarge':
      return 'inputTooLarge';
    case 'aiRequestFailed':
      return 'requestFailed';
    default:
      return 'other';
  }
}

/** 아직 노트가 없다. **오류가 아니라 정상 상태다** (§7 · INV-8). */
export const NO_AI_NOTE_TEXT = 'No AI note yet.';

/** 만들 재료가 아직 없다. **실패가 아니다** (§7.2 · ipc `AiNoteState.noTranscript`). */
export const NO_TRANSCRIPT_INPUT_TEXT = 'There is no transcript to make a note from yet.';

/** 그래서 무엇을 하면 되는가. 이 탭이 전사를 시작하지 않는다 — 그 자리는 Transcript 탭이다. */
export const NO_TRANSCRIPT_INPUT_HINT =
  'Transcribe this recording in the Transcript tab first, then come back.';

/**
 * 지금 노트를 만들고 있다.
 *
 * 생성은 backend의 배경 스레드에서 돌고 화면은 그것을 물어볼 뿐이다 — 그래서 이 화면을 떠나도
 * 생성은 계속되고, 도는 동안에도 화면과 재생이 멎지 않는다 (ADR-0008 §5).
 */
export const GENERATING_AI_NOTE_TEXT =
  'Writing the note… This keeps running in the background, so you can leave this screen.';

/**
 * AI Note 탭의 본문이 놓일 수 있는 상태의 전부.
 *
 * ```text
 * loading        아직 읽지 못했다 (레코드 · provider 상태 · 저장된 노트)
 * disabled       AI 기능이 켜지지 않았다 — 오류가 아니다 (INV-8)
 * noTranscript   만들 재료가 아직 없다 — 실패가 아니다 (§7.2)
 * none           이 mode의 노트를 아직 만든 적이 없다
 * generating     지금 만들고 있다 — 이미 있던 노트는 그대로 보인다
 * ready          노트를 구조 그대로 볼 수 있다 (§9.3)
 * failed         생성이 실패했다 — 원본도 기존 노트도 그대로다 (§13 · INV-3)
 * ```
 */
export type AiNoteTabBody =
  | { readonly kind: 'loading' }
  | { readonly kind: 'disabled'; readonly notice: AiDisabledNotice }
  | { readonly kind: 'noTranscript'; readonly text: string; readonly hint: string }
  | { readonly kind: 'none'; readonly text: string; readonly generate: AiNoteAction }
  | {
      readonly kind: 'generating';
      readonly text: string;
      /** 이미 있던 노트. 새 생성이 이것을 지우지 않는다 (ADR-0008 §9.2 · INV-2). */
      readonly kept: NoteView | null;
    }
  | {
      readonly kind: 'ready';
      readonly note: NoteView;
      /** 재생성은 대체가 아니라 추가다 (요구 12 · ADR-0008 §9.2). */
      readonly regenerate: AiNoteAction;
    }
  | {
      readonly kind: 'failed';
      /** 무엇을 하다 실패했는가. */
      readonly headline: string;
      /**
       * 실패 그대로 (§13의 세 질문에 대한 답이 이미 이 안에 있다).
       *
       * `null`이면 **이 앱이 켜진 뒤의 시도가 아니라서 이유를 모른다**는 뜻이다. 저장된 것은
       * `failed`라는 사실뿐이므로 이유를 지어내지 않는다.
       */
      readonly failure: Failure | null;
      readonly cause: AiNoteFailureCause;
      /** 원본과 기존 노트가 그대로라는 사실 (INV-1 · INV-2 · INV-3). */
      readonly preservedNotice: string;
      /** 이 갈래에서 사용자가 먼저 해야 하는 일. 없으면 `null`이다. */
      readonly resolution: string | null;
      /** 실패해도 다시 시도할 수 있다 (§13). */
      readonly retry: AiNoteAction;
      /** 실패가 지우지 않은 것 — 이미 있던 노트는 그대로 보인다. */
      readonly kept: NoteView | null;
    };

/**
 * AI Note 탭 전체.
 *
 * 본문과 나란히 **언제나 보이는 것 둘**이 있다 — 세 mode와, 고른 provider가 로컬인지
 * 외부인지다 (§12 · INV-5). mode를 본문 안에 넣지 않은 이유는 본문이 바뀌어도 "무엇을 만들
 * 것인가"는 남아 있기 때문이고, 지금 바꿀 수 있는지는 `modeSelectable`이 말한다.
 */
export interface AiNoteTabView {
  readonly modes: readonly NoteModeChoice[];
  /** 지금 mode를 바꿀 수 있는가. 생성 중이거나 AI가 꺼져 있으면 거짓이다. */
  readonly modeSelectable: boolean;
  /** 고른 provider와 그 locality. 고르지 않았으면 `null`이다 (§12 · INV-5). */
  readonly provider: NoteProviderNotice | null;
  readonly body: AiNoteTabBody;
}

/** {@link aiNoteTab}이 보는 사실 전부. 여기 없는 것은 이 탭의 상태에 영향을 주지 않는다. */
export interface AiNoteInput {
  /** 아직 읽지 못했으면 `null`이다. */
  readonly recording: Recording | null;
  /** current Transcript (§7.2). 아직 읽지 못했거나 없으면 `null`이다. */
  readonly transcript: Transcript | null;
  /** 그 Transcript의 저장된 노트. **아직 읽지 못했으면 `null`이고 없으면 빈 배열이다.** */
  readonly notes: readonly AiNote[] | null;
  /** 고른 provider의 지금 상태. 아직 물어보지 못했으면 `null`이다. */
  readonly provider: AiProviderStatus | null;
  /** 이 앱이 지금 돌리고 있는 생성 한 건. 아직 물어보지 못했으면 `null`이다. */
  readonly live: AiNoteStatus | null;
  /** 사용자가 고른 mode (§9.5). */
  readonly mode: NoteMode;
}

/**
 * 읽어 온 값을 AI Note 탭의 상태로 바꾼다.
 *
 * 다섯 가지를 함께 본다 — **이 녹음의 저장된 AI 상태**(§7) · **지금 이 앱이 돌리고 있는 생성
 * 한 건** · **고른 provider의 지금 상태** · **current Transcript**(§7.2) · **그 Transcript의
 * 저장된 노트**다. 각자 답할 수 없는 것이 있기 때문에 다섯이 다 필요하다.
 *
 * ```text
 * 저장된 AI 상태   앱을 다시 켜도 남는다              실패한 이유는 모른다
 * 지금의 생성      실패한 이유를 그대로 들고 있다     이 앱이 켜진 뒤의 것만 안다
 * provider 상태    지금 AI를 쓸 수 있는지 안다        노트가 있는지는 모른다
 * Transcript       만들 재료가 있는지 안다            무슨 일이 일어나는지는 모른다
 * 저장된 노트      볼 수 있는 노트 그 자체            지금 상태는 모른다
 * ```
 *
 * 순서에는 규칙이 하나 더 있다. **provider가 준비되지 않았다는 사실은 실패보다 먼저 본다** —
 * 지금 AI를 쓸 수 없다는 것이 사용자가 먼저 알아야 하는 사실이고, 그것을 실패로 그리지 않는 것이
 * INV-8이기 때문이다. 다만 **이미 돌고 있는 생성은 그보다 먼저다**: 실제로 벌어지고 있는 일을
 * 상태값이 덮지 않는다.
 *
 * 다른 녹음의 생성 상태는 이 화면과 아무 상관이 없으므로 보지 않는다. `live`가 `null`인 것은
 * 아직 물어보지 못했다는 뜻이며, 그것을 `idle`로 접지 않는다.
 */
export function aiNoteTab(input: AiNoteInput): AiNoteTabView {
  const { recording, transcript, notes, provider, live, mode } = input;

  const modes = noteModeChoices(mode);
  const notice = provider === null ? null : providerNotice(provider);
  const wrap = (body: AiNoteTabBody, modeSelectable: boolean): AiNoteTabView => ({
    modes,
    modeSelectable,
    provider: notice,
    body,
  });

  // 아직 읽지 못한 것이 있으면 아무 판단도 하지 않는다.
  if (recording === null || provider === null) {
    return wrap({ kind: 'loading' }, false);
  }

  const mine = live !== null && live.recordingId === recording.id ? live : null;
  const kept =
    transcript === null || notes === null
      ? null
      : keptNote(notes, transcript.id, mode);

  // 1. 지금 이 녹음에 대해 벌어지고 있는 일. 상태값이 이것을 덮지 않는다.
  if (mine?.state === 'running') {
    return wrap({ kind: 'generating', text: GENERATING_AI_NOTE_TEXT, kept }, false);
  }

  // 2. AI 기능이 켜져 있는가 (INV-8 · §13). **실패보다 먼저 본다.**
  //    생성을 요청했다가 `aiProviderNotConfigured`를 받은 경우도 여기로 온다 — 그것은
  //    command의 실패가 아니라 "AI가 꺼져 있다"는 상태값이며, 오류로 그리지 않는다.
  if (provider.state !== 'ready' || mine?.failure?.kind === 'aiProviderNotConfigured') {
    const state: AiDisabledState = provider.state === 'ready' ? 'notConfigured' : provider.state;
    return wrap({ kind: 'disabled', notice: disabledNotice(state, provider.providerName) }, false);
  }

  // 3. 이 앱이 돌린 생성 한 건의 결과.
  if (mine?.state === 'noTranscript') {
    return wrap(noTranscriptBody(), false);
  }
  if (mine?.state === 'failed') {
    return wrap(failedBody(recording.id, mode, mine.failure, kept), true);
  }

  // 4. 저장된 AI 상태 (§7). 앱을 다시 켠 뒤에도 남아 있는 사실이다.
  if (recording.aiStatus === 'pending' || recording.aiStatus === 'running') {
    return wrap({ kind: 'generating', text: GENERATING_AI_NOTE_TEXT, kept }, false);
  }
  if (recording.aiStatus === 'failed' && mine === null) {
    // 마지막 시도가 실패한 채로 남아 있다. 이 앱이 그 시도를 하지 않았으므로 이유는 모르며,
    // **이미 있던 노트는 그대로 `kept`에 남는다** — 실패가 그것을 잃게 하지 않는다.
    return wrap(failedBody(recording.id, mode, null, kept), true);
  }

  // 5. 만들 재료 (§7.2).
  if (recording.currentTranscriptId !== null && transcript === null) {
    // 레코드는 Transcript를 가리키는데 아직 그 값을 읽지 못했다.
    return wrap({ kind: 'loading' }, false);
  }
  if (transcript === null) {
    return wrap(noTranscriptBody(), false);
  }
  if (notes === null) {
    return wrap({ kind: 'loading' }, false);
  }

  // 6. 볼 수 있는 노트.
  if (kept !== null) {
    return wrap({ kind: 'ready', note: kept, regenerate: regenerateAction(recording.id, mode) }, true);
  }

  return wrap({ kind: 'none', text: NO_AI_NOTE_TEXT, generate: generateAction(recording.id, mode) }, true);
}

function keptNote(
  notes: readonly AiNote[],
  transcriptId: string,
  mode: NoteMode,
): NoteView | null {
  const note = latestNote(notes, transcriptId, mode);
  return note === null ? null : noteView(note);
}

function noTranscriptBody(): AiNoteTabBody {
  return { kind: 'noTranscript', text: NO_TRANSCRIPT_INPUT_TEXT, hint: NO_TRANSCRIPT_INPUT_HINT };
}

/**
 * 실패 상태 하나를 만든다.
 *
 * §13이 요구하는 세 가지가 전부 값으로 있다 — **무엇이 실패했는가**(`failure`·`headline`) ·
 * **원본은 안전한가**(`preservedNotice`) · **다시 시도할 수 있는가**(`retry`).
 *
 * **재시도 수단은 언제나 있다.** `Failure.retryable`이 거짓인 갈래(모델 없음 등)에서도
 * 마찬가지다 — 그것은 "지금 그대로 다시 눌러도 같다"는 뜻이지 "이 녹음은 영영 노트를 만들 수
 * 없다"는 뜻이 아니기 때문이다. 무엇을 먼저 해야 하는지는 `resolution`이 말한다.
 */
function failedBody(
  recordingId: string,
  mode: NoteMode,
  failure: Failure | null,
  kept: NoteView | null,
): AiNoteTabBody {
  const cause = failureCause(failure);
  return {
    kind: 'failed',
    headline: AI_NOTE_FAILED_HEADLINE,
    failure,
    cause,
    preservedNotice: AI_NOTE_PRESERVED_NOTICE,
    resolution: FAILURE_RESOLUTION[cause],
    retry: retryAction(recordingId, mode),
    kept,
  };
}

/**
 * 요청 자체가 거절됐다 (§13).
 *
 * **노트 생성 실패와 다른 사실이다.** 이미 다른 노트를 만들고 있을 때나 저장된 값을 읽지
 * 못했을 때가 여기로 오며, 그때 이 녹음의 AI 상태는 아무것도 달라지지 않았다 — 그래서 탭의
 * 본문을 덮지 않고 그 옆에 얹힌다. 접수되지 않은 요청이 조용히 사라지지 않게 하는 자리다.
 */
export interface AiNoteTrouble {
  readonly headline: string;
  readonly failure: Failure;
}

/** 화면이 backend에 보내는 AI 관련 요청. 실패가 어느 요청의 것인지 구분하는 데 쓴다. */
export type AiNoteRequest = 'provider' | 'status' | 'notes' | 'start';

const TROUBLE_HEADLINE: Record<AiNoteRequest, string> = {
  provider: 'The AI provider status could not be read.',
  status: 'The AI note status could not be read.',
  notes: 'The saved AI notes could not be read.',
  start: 'The AI note could not be started.',
};

/** 거절된 요청 하나를 화면에 놓을 값으로 옮긴다 (§13). */
export function aiNoteTrouble(request: AiNoteRequest, error: unknown): AiNoteTrouble {
  return { headline: TROUBLE_HEADLINE[request], failure: toFailure(error) };
}
