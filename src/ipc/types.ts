/**
 * command가 주고받는 값의 모양.
 *
 * `src-tauri/src/commands/payload.rs`의 직렬화 형태와 1:1이다. 여기 없는 것은 프론트엔드가
 * 알 수 없다 — 저장소도, SQL도, 스키마도 이 경계를 넘어오지 않는다.
 */

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
 * 설정 값 (§5 D).
 *
 * **INV-7: secret이 없다.** API key · integration token은 이 타입에도, 저장소에도 없다.
 */
export interface Settings {
  /** 아직 고르지 않았으면 `null`이다. */
  readonly recordingsDirectory: string | null;
  readonly automaticProcessing: boolean;
}
