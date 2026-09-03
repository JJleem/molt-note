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
}
