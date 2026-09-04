/**
 * command가 주고받는 값의 모양.
 *
 * `src-tauri/src/commands/payload.rs`의 직렬화 형태와 1:1이다. 여기 없는 것은 프론트엔드가
 * 알 수 없다 — 저장소도, SQL도, 스키마도 이 경계를 넘어오지 않는다.
 */
import type { Failure } from './failure';

/** 후처리 상태 (PRODUCT-SPEC §7). `none`은 아직 시도하지 않았다는 정상 상태다. */
export type ProcessingStatus = 'none' | 'pending' | 'running' | 'done' | 'failed';

/** 조회된 녹음 하나 (§5 A · C). */
export interface Recording {
  readonly id: string;
  readonly title: string;
  /** ISO-8601 UTC 텍스트. */
  readonly createdAt: string;
  readonly updatedAt: string;
  readonly durationMs: number;
  /** Rust가 만든 표시용 길이(예: `52:31`). 화면은 이 값을 그대로 쓴다. */
  readonly durationLabel: string;
  readonly audioPath: string;
  readonly audioFormat: string;
  readonly microphone: string | null;
  /** 현재 사용 중인 Transcript. 값이 없는 상태도 정상이다 (§7.2). */
  readonly currentTranscriptId: string | null;
  readonly transcriptionStatus: ProcessingStatus;
  readonly aiStatus: ProcessingStatus;
  readonly notionStatus: ProcessingStatus;
}

/**
 * 새로 저장할 녹음.
 *
 * 식별자 · 시각 · 후처리 상태는 없다 — Rust가 정한다. 프론트엔드가 보낼 수 있는 것은
 * 녹음 자체에 대해 아는 값뿐이다.
 */
export interface NewRecording {
  readonly title: string;
  readonly durationMs: number;
  readonly audioPath: string;
  readonly audioFormat: string;
  readonly microphone?: string | null;
}

/**
 * 고를 수 있는 입력 장치 하나 (§6.1의 `microphone enumeration`).
 *
 * `key`는 고를 때 쓰는 불투명한 값이고 `label`은 사람이 읽는 이름이다 — 이름이 같은 장치가
 * 둘 있을 수 있으므로 화면은 `label`을 보여주고 `key`로 고른다. 목록이 비어 있는 것은
 * 정상 상태다 (마이크가 없거나 뽑혀 있다).
 */
export interface InputDevice {
  readonly key: string;
  readonly label: string;
  readonly isDefault: boolean;
}

/**
 * 정지한 녹음 하나의 보고 값 (docs/ADR-0003-recording-engine.md §12).
 *
 * 장치 이름 · 출력 경로 · 포맷 · 파일 크기(byte)에 **녹음 길이**가 더해진다.
 * `format`과 `durationLabel`은 그대로 보여줄 수 있는 문장이고, 그 문장을 이루는 값도 따로
 * 온다 — 화면이 문자열을 다시 뜯어보거나 길이를 다시 계산하지 않게 하기 위해서다.
 *
 * **저장된 {@link Recording}이 아니다.** 저장된 레코드는 {@link StoppedRecording}의
 * `recording` 쪽이며, 둘은 정지 한 번에 함께 온다.
 */
export interface CaptureReport {
  /** 실제로 열린 장치의 이름. */
  readonly deviceLabel: string;
  readonly outputPath: string;
  /** 사람이 읽는 형식 문장(샘플레이트 · 채널 수 · 비트 심도 · 컨테이너). */
  readonly format: string;
  readonly sampleRateHz: number;
  readonly channels: number;
  readonly bitsPerSample: number;
  readonly container: string;
  /** 파일시스템에서 읽은 파일 크기(byte). */
  readonly byteSize: number;
  /** 일시정지 구간을 뺀 녹음 길이(밀리초). */
  readonly durationMs: number;
  /** Rust가 만든 표시용 길이(예: `52:31`). 화면은 이 값을 그대로 쓴다. */
  readonly durationLabel: string;
}

/**
 * 정지가 **성공했을 때** 오는 값 (PRODUCT-SPEC §6의 R-002).
 *
 * 이 값이 왔다는 것은 네 가지가 모두 성립했다는 뜻이다 — 파일이 존재하고, 크기가 유효
 * 최소치를 넘고, 포맷을 알고 있고, Recording 레코드가 저장됐다. 하나라도 성립하지 않으면
 * 이 값 대신 {@link Failure}가 오며, **그 실패는 확정된 파일이 어디 남아 있는지 말한다** —
 * 어떤 실패도 이미 녹음된 audio를 지우지 않기 때문이다 (INV-3 · INV-4).
 */
export interface StoppedRecording {
  /** 저장된 녹음. 목록 화면이 쓰는 값과 같은 모양이다. */
  readonly recording: Recording;
  /** 방금 확정된 파일에 대한 사실. */
  readonly capture: CaptureReport;
}

/**
 * **레코드는 있는데 오디오 파일이 없는** 녹음 하나.
 *
 * 정지 경로는 이 상태를 만들지 않는다 — 레코드는 파일이 확인된 뒤에만 저장된다. 그래도
 * 파일은 앱 밖에서 옮겨지거나 지워질 수 있으므로 그것을 아는 수단이 있다.
 *
 * **보고일 뿐이다.** 이 상태가 레코드를 지우거나 파일을 새로 만들지 않는다
 * (docs/ADR-0004-recording-session-lifecycle.md).
 */
export interface MissingAudio {
  readonly recordingId: string;
  readonly title: string;
  /** 레코드가 가리키고 있지만 지금 그 자리에 없는 경로. */
  readonly audioPath: string;
  readonly createdAt: string;
}

/**
 * 녹음 session이 있을 수 있는 상태.
 *
 * `src-tauri/src/audio/session.rs`의 네 상태와 1:1이다. 정지한 session은 backend에 남지
 * 않고 다음 녹음이 새 session으로 시작하므로, 실제로 화면에 도달하는 것은 앞의 셋이다.
 */
export type SessionState = 'idle' | 'recording' | 'paused' | 'stopped';

/**
 * 지금 녹음이 어떤 상태인지 (PRODUCT-SPEC §6의 R-001).
 *
 * **진행 중인 session을 들고 있는 것은 backend다.** 화면은 그것을 소유하지 않고 물어본다 —
 * 그래서 화면이 다시 그려지거나 사용자가 다른 화면에 다녀와도 같은 답이 온다
 * (docs/ADR-0004-recording-session-lifecycle.md).
 *
 * 길이는 밀리초와 문장이 함께 온다. 초를 `0:07`로 바꾸는 규칙은 Rust 한 곳에만 있다
 * (`tests/screen-boundary.test.ts`).
 */
export interface SessionStatus {
  readonly state: SessionState;
  /** 일시정지 구간을 뺀, 지금까지의 녹음 길이(밀리초). */
  readonly elapsedMs: number;
  /** 같은 길이를 사람이 읽는 문장으로 (예: `0:07`). */
  readonly elapsedLabel: string;
}

/**
 * 전사 한 건이 있을 수 있는 상태.
 *
 * **{@link ProcessingStatus}와 다른 값이다.** 저쪽은 녹음 하나에 저장된 후처리 상태(§7)이고,
 * 이쪽은 지금 이 앱이 실제로 돌리고 있는 전사 한 건이다. 그래서 `none`도 `pending`도 없다 —
 * 아직 아무것도 걸지 않은 상태가 `idle`이며, 시작한 순간부터 `running`이다.
 */
export type TranscriptionState = 'idle' | 'running' | 'done' | 'failed';

/**
 * 지금 전사가 어떤 상태인지 (`phase-prompt/03` 요구 3).
 *
 * **진행 중인 전사를 들고 있는 것은 backend다.** 화면은 그것을 소유하지 않고 물어본다 —
 * 그래서 화면이 다시 그려지거나 사용자가 다른 화면에 다녀와도 같은 답이 온다. 전사가 도는
 * 동안에도 이 질의는 즉시 답한다 (`src-tauri/src/commands/transcriber.rs`).
 *
 * 상태마다 값이 있는 필드가 다르다.
 *
 * ```text
 * state      recordingId   transcriptId   failure
 * idle       null          null           null
 * running    있음          null           null
 * done       있음          있음           null
 * failed     있음          null           있음
 * ```
 */
export interface TranscriptionStatus {
  readonly state: TranscriptionState;
  /** 지금 전사 중이거나 마지막으로 전사한 녹음. */
  readonly recordingId: string | null;
  /** 성공했을 때 **추가된** Transcript (§7.1). 그 시점에 이미 current다 (§7.2). */
  readonly transcriptId: string | null;
  /**
   * 실패했을 때 그 실패 그대로.
   *
   * 종류가 뭉개지지 않는다 — 모델이 없는 것과 엔진이 죽은 것은 사용자가 할 일이 다르다 (§13).
   */
  readonly failure: Failure | null;
}

/**
 * Transcript 안의 구간 하나 (§7의 `segments[] { start · end · text }`).
 *
 * **밀리초다.** 엔진마다 다른 단위를 하나로 맞추는 자리는 Rust의 통합 경계 한 곳뿐이며
 * (`src-tauri/src/transcription/parse.rs`), 여기까지 오는 값은 이미 정규화돼 있다.
 * 화면이 이 값을 다시 나누거나 곱하지 않는다 — 하는 일은 보여줄 형태로 옮기는 것뿐이다
 * (`src/screens/transcriptView.ts`).
 */
export interface TranscriptSegment {
  /** 녹음 시작 기준 오프셋(밀리초). */
  readonly startMs: number;
  readonly endMs: number;
  readonly text: string;
}

/**
 * 전사 결과 하나 (§7). **immutable · versioned다** (§7.1 · INV-2).
 *
 * 화면이 읽기만 하는 값이다 — 이 타입을 backend로 되돌려보내는 command가 없고, 재전사는
 * 기존 것을 고치지 않고 새 Transcript를 추가한다. 그래서 이미 본 Transcript가 다음 전사
 * 때문에 바뀌는 일은 없다.
 *
 * `language`가 없는 것도 정상이다 — 엔진이 언어를 말하지 않았다는 사실이며, 화면이 추측해서
 * 채우지 않는다. `engine`·`model`은 이 문장들이 무엇으로 만들어졌는지다 (provenance · §7).
 */
export interface Transcript {
  readonly id: string;
  readonly recordingId: string;
  readonly language: string | null;
  /** 저장된 순서 그대로. 화면이 다시 정렬하지 않는다. */
  readonly segments: readonly TranscriptSegment[];
  readonly rawText: string;
  /** ISO-8601 UTC 텍스트. */
  readonly createdAt: string;
  readonly engine: string;
  readonly model: string;
}

/**
 * 설정 값 (§5 D).
 *
 * **INV-7: secret이 없다.** API key · integration token은 이 타입에도, 저장소에도 없다.
 */
export interface Settings {
  /** 아직 고르지 않았으면 `null`이다. */
  readonly recordingsDirectory: string | null;
  readonly automaticProcessing: boolean;
  /**
   * 녹음을 정지해 저장한 직후에 **전사를 자동으로 시작할지** 여부
   * (`phase-prompt/03` 요구 4).
   *
   * **{@link Settings.automaticProcessing}과 다른 값이다.** 한쪽을 켠다고 다른 쪽이 켜지지
   * 않는다 — 하나의 boolean에 두 의미가 겹치지 않는다.
   *
   * 꺼져 있다는 것은 "자동으로 시작하지 않는다"는 뜻이지 "전사할 수 없다"는 뜻이 아니다.
   * 수동 전사(`startTranscription`)는 이 값과 무관하게 언제나 할 수 있다.
   */
  readonly automaticTranscription: boolean;
  /**
   * 전사에 쓸 모델 파일의 **이름 또는 경로** (docs/ADR-0007-transcription-engine.md §8.2).
   * 아직 고르지 않았으면 `null`이며, 그것도 정상 상태다.
   *
   * **secret이 아니다** — 파일이 어디 있는지일 뿐이므로 INV-7과 충돌하지 않는다.
   *
   * `defaultMicrophone`과 같은 성질을 갖는다. 이 값이 가리키는 파일이 지금 그 자리에 있는지는
   * 이 값만으로 알 수 없고, **없다고 해서 앱이 값을 지우거나 다른 모델로 바꾸지 않는다.**
   * 실제로 찾아보는 것은 전사를 시작할 때이며, 없으면 §13의 실패로 드러난다.
   */
  readonly transcriptionModel: string | null;
  /**
   * 기본으로 고를 입력 장치의 **선택 키** ({@link InputDevice.key}). 아직 고르지 않았으면
   * `null`이며, 그것도 정상 상태다.
   *
   * 이 키가 가리키는 장치가 지금 있는지는 이 값만으로 알 수 없다 — 장치는 언제든 빠진다.
   * 그 판단은 목록과 함께 보는 `src/screens/defaultMicrophone.ts`가 하며, **없어진 장치를
   * 조용히 다른 장치로 바꾸지 않는다.**
   */
  readonly defaultMicrophone: string | null;
  /**
   * 노트를 만들 때 쓸 AI provider의 **식별자**
   * (docs/ADR-0008-note-ai-provider.md §11.1).
   *
   * **아직 고르지 않았으면 `null`이고, 그것이 기본이자 정상 상태다** — 오류가 아니다.
   * 고르지 않은 동안에도 녹음 · 전사 · 열람은 그대로 동작하며, 화면은 그 상태를 실패가
   * 아니라 "AI 기능이 아직 켜지지 않았다"로 말한다 (INV-8).
   *
   * 벤더 중립 자유 식별자다 — 이 타입은 어떤 값이 올 수 있는지 열거하지 않는다 (INV-9).
   */
  readonly aiProvider: string | null;
  /**
   * AI provider에 연결할 **주소**. 아직 고르지 않았으면 `null`이며, 그것도 정상 상태다.
   *
   * **`null`이 "연결할 곳이 없다"는 뜻은 아니다** — 고르지 않았을 때 실제로 어디에
   * 연결하는지는 backend가 알고 있고, 그 기본값은 이 화면이 아니라 backend 한 곳에만 있다.
   *
   * **secret이 아니다** — 어디에 연결하는지일 뿐이므로 INV-7과 충돌하지 않는다.
   */
  readonly aiBaseUrl: string | null;
  /**
   * 노트를 만들 때 쓸 **모델 식별자**. 아직 고르지 않았으면 `null`이며, 그것도 정상 상태다.
   *
   * `transcriptionModel` · `defaultMicrophone`과 같은 성질을 갖는다. 이 모델이 지금 그 서버에
   * 설치돼 있는지는 이 값만으로 알 수 없고, **없다고 해서 앱이 값을 지우거나 다른 모델로
   * 바꾸지 않는다.**
   */
  readonly aiModel: string | null;
  /**
   * Notion 페이지를 **어느 페이지 아래에** 만드는가
   * (docs/ADR-0009-notion-and-export.md §5.1 · §8.4).
   * 아직 고르지 않았으면 `null`이며, 그것도 정상 상태다.
   *
   * **secret이 아니다** — 어디에 쓰는지일 뿐이므로 INV-7과 충돌하지 않는다. 그 workspace에
   * 들어가기 위한 자격증명은 **이 타입에도, 어떤 command 응답에도 없다** — backend가 OS
   * 보안 저장소에 넣고, 화면은 "설정돼 있는가"만 묻는다 (ADR-0009 §10.4).
   *
   * `aiBaseUrl` · `transcriptionModel`과 같은 성질을 갖는다. 이 페이지가 지금도 있는지는 이
   * 값만으로 알 수 없고, **없다고 해서 앱이 값을 지우거나 다른 페이지로 바꾸지 않는다.**
   */
  readonly notionParentPageId: string | null;
}

/**
 * 고른 AI provider가 지금 있을 수 있는 상태 (PRODUCT-SPEC §13 · INV-8).
 *
 * **`notConfigured`는 오류가 아니다.** provider를 고르지 않은 것은 정상 상태이며, 화면은 그것을
 * 경고가 아니라 "AI 기능이 아직 켜지지 않았다"는 담담한 상태로 그린다.
 *
 * 나머지 셋을 하나의 boolean으로 합치지 않는 이유는 사용자가 할 일이 다르기 때문이다 —
 * 모델을 받아 오는 것(`noModels`)과 서버를 켜는 것(`unavailable`)은 다른 일이다
 * (docs/ADR-0008-note-ai-provider.md §4.2).
 */
export type AiProviderState = 'notConfigured' | 'ready' | 'noModels' | 'unavailable';

/**
 * 전사가 **이 기기를 떠나는가** (§12 · INV-5).
 *
 * 사용자가 알아야 하는 것은 어디로 나가는지가 아니라 나가는가이며, 그 답은 둘 중 하나다.
 * 이 값을 말하는 것은 화면이 아니라 provider 자신이다.
 */
export type AiProviderLocality = 'local' | 'external';

/**
 * 고른 AI provider의 지금 상태 (`phase-prompt/04` 요구 8 · 9 · 15).
 *
 * **이 값을 얻는 데 실패하는 경우는 provider와 무관하다.** provider를 고르지 않은 것도, 서버가
 * 응답하지 않는 것도 여기 담긴 상태이지 실패가 아니다 (INV-8).
 *
 * ```text
 * state           providerId/Name/locality   models   failure
 * notConfigured   null                       []       null
 * ready           있음                       있음     null
 * noModels        있음                       []       null
 * unavailable     있음                       []       있음
 * ```
 */
export interface AiProviderStatus {
  readonly state: AiProviderState;
  /**
   * 저장되는 provenance 식별자. 고른 provider가 없으면 `null`이다.
   *
   * **벤더 중립 자유 식별자다** — 이 타입은 어떤 값이 올 수 있는지 열거하지 않는다 (INV-9).
   */
  readonly providerId: string | null;
  /** 사람이 읽는 이름. 화면은 이 값을 그대로 보여준다. */
  readonly providerName: string | null;
  /** 전사가 기기를 떠나는지 (§12 · INV-5). 고른 provider가 없으면 `null`이다. */
  readonly locality: AiProviderLocality | null;
  /** 지금 고를 수 있는 모델. 쓸 수 없는 상태에서는 빈 배열이다. */
  readonly models: readonly string[];
  /** 지금 닿지 못하는 이유. `unavailable`에서만 있다. */
  readonly failure: Failure | null;
}

/** 만들 수 있는 노트의 종류 (§9.5). **벤더가 아니라 출력 형태다** (INV-9). */
export type NoteMode = 'meeting' | 'study' | 'summary';

/**
 * AI 노트 생성 한 건이 있을 수 있는 상태.
 *
 * **{@link ProcessingStatus}와 다른 값이다.** 저쪽은 녹음 하나에 저장된 후처리 상태(§7)이고,
 * 이쪽은 지금 이 앱이 실제로 돌리고 있는 생성 한 건이다.
 *
 * `noTranscript`가 `failed`와 따로 있는 이유는 §7.2다 — **재료가 아직 없는 것은 실패가 아니다.**
 * 아무것도 저장되지 않았고, 사용자가 할 일은 다시 시도하는 것이 아니라 전사를 먼저 돌리는 것이다.
 */
export type AiNoteState = 'idle' | 'running' | 'done' | 'noTranscript' | 'failed';

/**
 * 지금 AI 노트 생성이 어떤 상태인지 (`phase-prompt/04` 요구 16).
 *
 * **진행 중인 생성을 들고 있는 것은 backend다.** 화면은 그것을 소유하지 않고 물어본다 — 그래서
 * 화면이 다시 그려지거나 사용자가 다른 화면에 다녀와도 같은 답이 온다. 생성이 도는 동안에도 이
 * 질의는 즉시 답한다 (`src-tauri/src/commands/notes.rs`).
 *
 * ```text
 * state          recordingId   mode    aiNoteId   failure
 * idle           null          null    null       null
 * running        있음          있음    null       null
 * done           있음          있음    있음       null
 * noTranscript   있음          있음    null       null
 * failed         있음          있음    null       있음
 * ```
 *
 * `failure`에는 §13의 AI 실패가 그대로 실려 온다. **provider를 고르지 않은 상태에서 생성을
 * 요청했을 때의 답도 여기로 온다** (`aiProviderNotConfigured`) — 그것은 command의 실패가 아니라
 * 상태값이며, 화면은 오류가 아니라 "AI를 먼저 설정해야 한다"로 그린다 (INV-8).
 */
export interface AiNoteStatus {
  readonly state: AiNoteState;
  /** 지금 노트를 만들고 있거나 마지막으로 만든 녹음. */
  readonly recordingId: string | null;
  readonly mode: NoteMode | null;
  /** 성공했을 때 **새로 추가된** 노트. 이전 노트는 그대로 남는다. */
  readonly aiNoteId: string | null;
  readonly failure: Failure | null;
}

/**
 * Meeting 노트의 본문 (§9.5: Overview · Key Discussions · Decisions · Action Items ·
 * Open Questions).
 *
 * **배열이 비어 있는 것은 정상이다** — "결정된 것이 없었다"는 실제 결과이며, 화면이 그것을
 * 실패로 그리지 않는다 (docs/ADR-0008-note-ai-provider.md §7.3).
 */
export interface MeetingNote {
  readonly mode: 'meeting';
  readonly overview: string;
  readonly keyDiscussions: readonly string[];
  readonly decisions: readonly string[];
  readonly actionItems: readonly string[];
  readonly openQuestions: readonly string[];
}

/**
 * Study 노트의 본문
 * (§9.5: Overview · Key Concepts · Important Details · Questions · Things to Study ·
 * References Mentioned).
 */
export interface StudyNote {
  readonly mode: 'study';
  readonly overview: string;
  readonly keyConcepts: readonly string[];
  readonly importantDetails: readonly string[];
  readonly questions: readonly string[];
  readonly thingsToStudy: readonly string[];
  /** **"언급된" 참고자료다** — 없는 참고문헌을 채우지 않는다. */
  readonly referencesMentioned: readonly string[];
}

/** Summary 노트의 본문 (§9.5: Short Summary · Key Points). */
export interface SummaryNote {
  readonly mode: 'summary';
  readonly shortSummary: string;
  readonly keyPoints: readonly string[];
}

/**
 * 노트 본문 — `mode`로 갈라 읽는 **한 값**이다 (§9.3 · §9.5).
 *
 * 세 mode를 옵셔널 필드 하나로 합치지 않는다. 합치면 "Meeting인데 `thingsToStudy`가 있는" 값이
 * 타입 수준에서 가능해지고, 화면이 없는 섹션을 그리게 된다 (ADR-0008 §7.3).
 *
 * **provider 중립 데이터다** (§9.3 · INV-9) — 어느 벤더가 만들었든 이 모양이며, 렌더링은 한
 * 방향으로만 흐른다: `StructuredNote → 화면`.
 */
export type StructuredNote = MeetingNote | StudyNote | SummaryNote;

/**
 * 저장된 AI 노트 하나 (§7 · §7.3). **derived data이며 언제든 다시 만들 수 있다.**
 *
 * 화면이 읽기만 하는 값이다 — 이 타입을 backend로 되돌려보내는 command가 없고, 재생성은 기존
 * 노트를 고치지 않고 새 노트를 추가한다 (ADR-0008 §9.2). 그래서 이미 본 노트가 다음 생성 때문에
 * 바뀌는 일은 없고, `promptVersion`이 다른 두 노트를 나란히 볼 수 있다.
 *
 * provenance 네 값은 §7.3이 요구하는 것 전부다 — **`transcriptId`가 그중 하나다**: 한 Recording에
 * Transcript가 여럿일 수 있으므로 (§7.1), 어떤 version에서 나온 노트인지 이 값으로 구분된다.
 * `provider`는 벤더 중립 자유 식별자이며, 이 타입은 어떤 값이 올 수 있는지 열거하지 않는다 (INV-9).
 */
export interface AiNote {
  readonly id: string;
  readonly recordingId: string;
  /** **어떤 Transcript version을 입력으로 썼는가** (§7.3). */
  readonly transcriptId: string;
  readonly mode: NoteMode;
  /** 노트 본문. `note.mode`는 언제나 {@link AiNote.mode}와 같다. */
  readonly note: StructuredNote;
  readonly provider: string;
  readonly model: string;
  readonly promptVersion: string;
  /** ISO-8601 UTC 텍스트. */
  readonly generatedAt: string;
}

/**
 * 방금 만들어진 Markdown 파일 하나 (§11 · docs/ADR-0009-notion-and-export.md §4).
 *
 * **경로가 이 값의 요점이다.** export 위치는 설정으로 노출되지 않으므로 (§4.1), 화면이
 * `path`를 보여주지 않으면 사용자는 방금 만든 파일을 찾을 수 없다.
 *
 * `fileName`이 따로 있는 이유는 **요청한 이름과 다를 수 있기 때문이다** — 같은 이름이 이미
 * 있으면 backend가 덮어쓰지 않고 번호를 붙인다 (§4.3: `…-2.md` · `…-3.md`). 화면이 경로
 * 문자열을 잘라 이름을 짐작하지 않도록 실제로 쓰인 이름이 함께 온다.
 *
 * 문서 본문은 오지 않는다 — 그것은 파일에 있고, IPC로 한 번 더 흘려보낼 이유가 없다.
 */
export interface ExportedFile {
  readonly recordingId: string;
  /** 만들어진 파일의 전체 경로. 사용자에게 그대로 보여줄 수 있는 값이다. */
  readonly path: string;
  /** 그 파일의 이름 (`2026-09-01-3dgs-study-04.md`). */
  readonly fileName: string;
}

/** 지금 돌고 있는 Notion 전송 한 건의 상태 (§10). */
export type NotionSendState = 'idle' | 'running' | 'done' | 'failed';

/**
 * 지금 Notion 전송이 어떤 상태인지 (§10 · docs/ADR-0009-notion-and-export.md §8).
 *
 * {@link TranscriptionStatus} · {@link AiNoteStatus}와 같은 자리에 있는 값이다 — **진행 중인
 * 전송을 소유하는 것은 backend이고 화면은 그것을 물어본다.** 화면이 다시 그려지거나 사용자가
 * 다른 화면에 다녀와도 여기서 같은 답이 나온다.
 *
 * ```text
 * state      recordingId   pageId   createdPage   failure
 * idle       null          null     false         null
 * running    있음          null     false         null
 * done       있음          있음     true/false    null
 * failed     있음          null     false         있음
 * ```
 *
 * `createdPage`가 따로 있는 이유는 **이어 보낸 것과 새로 만든 것이 다른 결과이기 때문이다**
 * (§8.2 · §8.3) — 끝나지 않은 전송을 이어 보냈다면 페이지는 그때 만들어진 그 페이지이며, 화면은
 * "새 페이지를 만들었다"고 말하면 안 된다.
 *
 * `failure`에는 §13의 Notion 실패가 그대로 실려 온다. **부분 전송 뒤의 실패도 여기로 온다** —
 * 어디까지 갔는지는 {@link NotionSync}가 말한다.
 */
export interface NotionSendStatus {
  readonly state: NotionSendState;
  /** 지금 보내고 있거나 마지막으로 보낸 녹음. */
  readonly recordingId: string | null;
  /** 성공했을 때 그 녹음이 된 Notion 페이지의 식별자. */
  readonly pageId: string | null;
  /** 이번 실행에서 **새로** 만든 페이지인가. 이어 보낸 것이면 `false`다. */
  readonly createdPage: boolean;
  readonly failure: Failure | null;
}

/**
 * 사용자가 무엇을 확인했는가 (§8.3 · §8.5).
 *
 * **아무것도 보내지 않는 것은 `notAsked`와 같다.** 이어 보낼 수 없는 상태에서 새 페이지를
 * 만드는 일은 사용자가 알고 누른 결과여야 하며, 앱이 스스로 고르지 않는다 — 그래서 화면이 값을
 * 싣지 않았을 때 조용히 페이지가 하나 더 생기지 않는다.
 */
export type NotionConfirmation = 'notAsked' | 'newPage';

/**
 * 저장된 Notion 전송 상태 하나 (§7의 `notion_syncs` · §8.4).
 *
 * {@link NotionSendStatus}와 다른 값이다 — 저쪽은 **앱이 켜져 있는 동안의 진행 상황**이고,
 * 이쪽은 **디스크에 남아 있는 사실**이다. 앱을 다시 켜도 이 값은 그대로 있다.
 *
 * `sentChunks`와 `totalChunks`가 함께 오는 이유는 하나다 — **부분 전송이 상태에서 드러나야
 * 하기 때문이다** (§8.4). 실패한 요청은 세지 않으므로, 둘이 다르면 그 페이지에는 문서의 일부만
 * 들어가 있다. 기록하기 전에 만들어진 행이면 둘 다 `null`이며 그것도 정상 상태다.
 *
 * **읽기 전용 값이다.** 이 타입을 backend로 되돌려보내는 command가 없고, 전송 기록을 고치거나
 * 지우는 이름도 없다 (INV-3).
 */
export interface NotionSync {
  readonly recordingId: string;
  /** 만들어진 페이지의 식별자. 성공한 적이 없으면 `null`이다. */
  readonly pageId: string | null;
  /** 마지막으로 성공한 시각(ISO-8601 UTC 텍스트). */
  readonly syncedAt: string | null;
  readonly status: ProcessingStatus;
  /** 마지막 실패 사유. 실패한 적이 없으면 `null`이다. */
  readonly error: string | null;
  /** 이 페이지에 **성공적으로** 반영된 조각 수. */
  readonly sentChunks: number | null;
  /** 그 문서를 나눈 조각 수. */
  readonly totalChunks: number | null;
}

/** 연결 확인의 결과 (§5-D). `notConfigured`는 오류가 아니라 정상 상태다 (INV-8). */
export type NotionConnectionState = 'notConfigured' | 'connected' | 'failed';

/**
 * Notion 연결이 지금 어떤 상태인지 (§5-D의 connection test).
 *
 * {@link AiProviderStatus}와 같은 성질의 값이다 — **아직 설정하지 않은 것은 실패가 아니라
 * 상태다** (INV-8). 그래서 화면은 이 값을 경고가 아니라 담담한 상태로 그린다.
 *
 * ```text
 * state           tokenStored   workspaceName    failure
 * notConfigured   false         null             아직 token을 저장하지 않았다 (요청도 나가지 않는다)
 * connected       true          Notion이 말한 것  이 token으로 지금 말할 수 있다
 * failed          true          null             말하지 못했다 — 무엇이 다른지는 failure가 말한다
 * ```
 *
 * **세 상태를 boolean 하나로 뭉개지 않는 이유는 §13이다** — token을 넣는 것 · token을 고치는
 * 것(`notionAuthFailed`) · 부모 페이지를 integration에 공유하는 것
 * (`notionDestinationUnavailable`) · 네트워크를 확인하는 것(`notionRequestFailed`)은 사용자가
 * 할 일이 전부 다르다.
 *
 * **token 값은 오지 않는다** (INV-7). 오는 것은 저장돼 있다는 사실 하나뿐이다.
 */
export interface NotionConnection {
  readonly state: NotionConnectionState;
  /** integration token이 저장돼 있는가. **값이 아니라 사실이다.** */
  readonly tokenStored: boolean;
  /** 보낼 부모 페이지를 골랐는가. 고르지 않은 것도 정상 상태다 (INV-8). */
  readonly destinationConfigured: boolean;
  /**
   * **어느 워크스페이스에 연결됐는가** (§5-D). 연결됐고 Notion이 이름을 말해 줬을 때만 있다.
   *
   * 말해 주지 않았으면 `null`이며, 그때 화면은 이름을 지어내지 않고 연결됐다는 사실만 보인다.
   * **secret이 아니다** — 사용자가 자기 워크스페이스를 알아보기 위한 값이고, token은 여전히
   * 이 타입 어디에도 없다 (INV-7).
   */
  readonly workspaceName: string | null;
  /** 연결하지 못한 이유. `failed`에서만 있다. */
  readonly failure: Failure | null;
}

/**
 * integration token이 저장돼 있는가 (INV-7 · §10).
 *
 * **화면이 token에 대해 알 수 있는 전부다.** 값을 돌려주는 command는 없으며, 저장·삭제 뒤에
 * 자격증명 저장소가 실제로 어떤 상태인지가 이 값으로 온다 — 화면은 자기가 방금 무엇을 눌렀는지로
 * 상태를 짐작하지 않는다.
 */
export interface NotionTokenStatus {
  readonly stored: boolean;
}
