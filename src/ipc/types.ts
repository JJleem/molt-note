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
 * 정지한 캡처 하나의 보고 값 (Phase 2A spike · docs/ADR-0003-recording-engine.md §12).
 *
 * Phase 2A의 성공 기준이 그대로 필드다 — 장치 이름 · 출력 경로 · 포맷 · 파일 크기(byte).
 * `format`은 그대로 보여줄 수 있는 한 문장이고, 그 문장을 이루는 값도 따로 온다.
 *
 * **저장된 {@link Recording}이 아니다.** 이 값은 어떤 레코드도 만들지 않는다 —
 * 캡처 결과를 DB에 남기는 것은 Phase 2B의 일이다.
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
