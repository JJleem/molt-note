/**
 * Recordings 화면의 상태 (PRODUCT-SPEC §5 A).
 *
 * command 호출 결과를 화면이 그대로 그릴 수 있는 값으로 바꾼다. React도 DOM도 Tauri도
 * 알지 않는 순수 모듈이라 **목록 · 빈 목록 · 실패** 세 경로를 vitest로 그대로 판정할 수 있다 (§18).
 * 화면 컴포넌트는 여기서 만들어진 값을 그리기만 하고 자기 나름의 변환 규칙을 갖지 않는다.
 *
 * ## 길이 문자열은 여기서 만들지 않는다
 *
 * `durationLabel`은 Rust가 이미 만들어 보낸 값이다 (`src-tauri/src/domain/duration.rs`).
 * 초를 `52:31`로 바꾸는 규칙은 그 한 곳에만 있고, TypeScript에 다시 구현하지 않는다 —
 * 두 벌이 되면 조용히 갈라진다. `durationMs`는 이 모듈이 읽지 않는다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { ProcessingStatus, Recording } from '../ipc/types';

/** 항목 하나가 보여주는 후처리 상태 하나 (§7). */
export interface RecordingStatusBadge {
  /** 무엇의 상태인가 — `Transcript` · `AI Note` · `Notion`. */
  readonly label: string;
  readonly status: ProcessingStatus;
  /** 상태의 사람이 읽는 표현. */
  readonly text: string;
}

/**
 * 목록 항목 하나의 화면 표현 (§5 A — 제목 · 날짜 · 길이 · 세 가지 상태).
 *
 * 세 상태는 배열이 아니라 길이 3의 tuple이다 — 하나가 빠지면 컴파일에서 드러난다.
 */
export interface RecordingListItem {
  readonly id: string;
  readonly title: string;
  readonly recordedAtLabel: string;
  /** Rust가 만든 값 그대로. */
  readonly durationLabel: string;
  readonly statuses: readonly [RecordingStatusBadge, RecordingStatusBadge, RecordingStatusBadge];
}

/**
 * 화면이 놓일 수 있는 상태의 전부.
 *
 * 빈 목록은 실패가 아니라 독립된 정상 상태(`empty`)다. 목록이 비어 있는 것과
 * 아직 읽지 못한 것(`loading`), 읽지 못한 것(`failed`)이 화면에서 섞이지 않는다.
 */
export type RecordingsView =
  | { readonly kind: 'loading' }
  | { readonly kind: 'empty' }
  | { readonly kind: 'list'; readonly items: readonly RecordingListItem[] }
  | { readonly kind: 'failed'; readonly failure: Failure };

export const LOADING_RECORDINGS: RecordingsView = { kind: 'loading' };

/** `none`은 아직 시도하지 않았다는 정상 상태다 — 오류처럼 읽히지 않게 적는다 (§7 · INV-8). */
const STATUS_TEXT: Record<ProcessingStatus, string> = {
  none: 'Not started',
  pending: 'Pending',
  running: 'Running',
  done: 'Done',
  failed: 'Failed',
};

/** 날짜 라벨을 만들 때 쓰는 시간대. 지정하지 않으면 실행 환경의 시간대다. */
export interface RecordedAtOptions {
  readonly timeZone?: string;
}

/**
 * `list_recordings`가 돌려준 목록을 화면 상태로 바꾼다.
 *
 * 하나도 없으면 실패가 아니라 `empty`다 (`Storage::list_recordings`의 계약과 같다).
 */
export function loadedRecordings(
  recordings: readonly Recording[],
  options: RecordedAtOptions = {},
): RecordingsView {
  if (recordings.length === 0) {
    return { kind: 'empty' };
  }
  return { kind: 'list', items: recordings.map((recording) => toListItem(recording, options)) };
}

/**
 * command가 거절되며 넘어온 값을 화면 상태로 바꾼다.
 *
 * 저장소 초기화 실패는 여기로 온다 — 앱이 죽지도 않고, console에만 남지도 않는다 (§13).
 */
export function failedRecordings(error: unknown): RecordingsView {
  return { kind: 'failed', failure: toFailure(error) };
}

function toListItem(recording: Recording, options: RecordedAtOptions): RecordingListItem {
  return {
    id: recording.id,
    title: recording.title,
    recordedAtLabel: formatRecordedAt(recording.createdAt, options),
    // Rust가 보낸 값을 그대로 쓴다. 여기서 다시 계산하지 않는다.
    durationLabel: recording.durationLabel,
    statuses: [
      statusBadge('Transcript', recording.transcriptionStatus),
      statusBadge('AI Note', recording.aiStatus),
      statusBadge('Notion', recording.notionStatus),
    ],
  };
}

/**
 * 후처리 상태 하나를 화면 표현으로 옮긴다 (§7).
 *
 * 목록만의 것이 아니라 **상태를 보여주는 모든 자리**가 이 함수를 쓴다 — 같은 상태가 목록과
 * 상세에서 다른 말로 읽히면 사용자는 두 화면이 다른 것을 말하고 있다고 읽는다
 * (`notionSyncView.ts`가 Recording Detail에서 이 함수를 쓴다).
 */
export function statusBadge(label: string, status: ProcessingStatus): RecordingStatusBadge {
  return { label, status, text: STATUS_TEXT[status] ?? status };
}

/**
 * 저장된 시각(ISO-8601 UTC 텍스트)을 §5 A의 `Sep 1` 형태로 만든다.
 *
 * 날짜 라벨은 Rust가 보내지 않는다 — 저장소는 시각만 알고, 어느 시간대의 며칠로 읽을지는
 * 화면의 문제다. 읽을 수 없는 값은 지어내지 않고 저장된 텍스트를 그대로 보여준다.
 */
export function formatRecordedAt(isoText: string, options: RecordedAtOptions = {}): string {
  const recordedAt = new Date(isoText);
  if (Number.isNaN(recordedAt.getTime())) {
    return isoText;
  }
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    timeZone: options.timeZone,
  }).format(recordedAt);
}
