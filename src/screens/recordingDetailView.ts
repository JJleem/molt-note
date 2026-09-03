/**
 * Recording Detail 화면의 상태 (PRODUCT-SPEC §5 C · Phase 2B 요구사항 7).
 *
 * 이 화면이 답해야 하는 질문은 두 가지다 — **이 녹음은 무엇인가**와 **지금 재생할 수 있는가**.
 * 둘 다 backend가 준 사실에서 나오며, 그 사실을 화면 상태로 옮기는 규칙이 전부 여기 있다.
 * React도 DOM도 Tauri도 알지 않으므로 네 경로(로딩 · 재생 가능 · 파일 없음 · 조회 실패)를
 * **오디오 파일 없이, jsdom 없이** vitest로 그대로 판정할 수 있다 (§18).
 *
 * ## 파일이 없는 것은 실패가 아니다 (INV-3 · INV-4)
 *
 * 레코드는 있는데 오디오 파일이 없는 상태는 조회 실패와 다른 사실이다. 저장소는 답했고,
 * 레코드도 온전하며, 다만 파일이 그 자리에 없을 뿐이다. 그래서 이 모듈은 그것을 독립된
 * 상태(`audioMissing`)로 만들고 **레코드가 말하는 것을 그대로 보여준다** — 제목도 길이도
 * 저장된 경로도 남는다.
 *
 * 이 상태는 아무것도 지우지 않는다. 감지는 감지일 뿐이며, 그것을 판정하는 자리도 여기가
 * 아니라 backend다 (`list_missing_audio` · `src-tauri/src/audio/finalized.rs`).
 * 화면이 레코드를 지우거나 파일을 다시 만드는 경로는 존재하지 않는다 (R-004).
 *
 * ## 재생 주소를 여기서 만들지 않는다
 *
 * 파일 경로를 webview가 읽을 수 있는 주소로 바꾸는 일은 Tauri가 한다
 * ({@link recordingAudioSource}). 이 모듈은 그 함수를 **받아서** 쓸 뿐이라 테스트가 자신의
 * 구현을 넣어 변환 없이 이 경로를 그대로 지날 수 있고, 파일이 없는 상태에서는 애초에
 * 부르지 않는다는 것도 그대로 확인된다.
 *
 * ## 길이 문자열은 여기서 만들지 않는다
 *
 * `durationLabel`은 Rust가 이미 만들어 보낸 값이다 (`src-tauri/src/domain/duration.rs`).
 * 목록 화면과 같은 규칙이며, TypeScript에 다시 구현하지 않는다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { MissingAudio, Recording } from '../ipc/types';
import { formatRecordedAt, type RecordedAtOptions } from './recordingsView';

/**
 * 파일 경로를 webview가 읽을 수 있는 주소로 바꾸는 함수.
 *
 * 앱은 `recordingAudioSource`를 넣고, 테스트는 자신의 구현을 넣는다.
 */
export type AudioSourceResolver = (audioPath: string) => string;

/** 이 녹음이 무엇인지 (§5 C의 머리 부분). 값은 전부 레코드에서 그대로 온다. */
export interface RecordingHeader {
  readonly id: string;
  readonly title: string;
  readonly recordedAtLabel: string;
  /** Rust가 만든 값 그대로. */
  readonly durationLabel: string;
}

/**
 * 상세 화면이 놓일 수 있는 상태의 전부.
 *
 * 다섯 상태가 서로 다른 사실을 말한다.
 *
 * ```text
 * loading       아직 읽지 못했다
 * playable      레코드도 파일도 있다 — 재생할 수 있다
 * audioMissing  레코드는 있는데 파일이 그 자리에 없다 (실패가 아니다)
 * notFound      그런 id의 레코드가 없다 (저장소가 정상적으로 답한 결과다)
 * failed        저장소에 물어보지 못했다
 * ```
 */
export type RecordingDetailView =
  | { readonly kind: 'loading' }
  | {
      readonly kind: 'playable';
      readonly recording: RecordingHeader;
      /** webview가 재생에 쓰는 주소. 로컬 asset protocol 주소이며 밖으로 나가지 않는다. */
      readonly audioSource: string;
      readonly audioFormat: string;
    }
  | {
      readonly kind: 'audioMissing';
      readonly recording: RecordingHeader;
      /** 레코드가 가리키고 있지만 지금 그 자리에 없는 경로. 사용자가 찾을 수 있게 남긴다. */
      readonly audioPath: string;
    }
  | { readonly kind: 'notFound'; readonly recordingId: string }
  | { readonly kind: 'failed'; readonly failure: Failure };

/** 화면을 열었을 때의 상태. 아직 아무것도 읽지 못했다. */
export const LOADING_RECORDING_DETAIL: RecordingDetailView = { kind: 'loading' };

/**
 * 파일이 없는 녹음에 대해 사용자에게 하는 말 (§13).
 *
 * 세 가지를 말한다 — 무슨 일이 일어났는가 · 원본은 어떻게 됐는가 · 지금 무엇을 할 수 있는가.
 * **"복구했다"거나 "정리했다"고 말하지 않는다.** 앱은 아무것도 지우지 않았고 아무것도
 * 만들지 않았다 (INV-3 · INV-4 · R-004).
 */
export const MISSING_AUDIO_NOTICE =
  'The audio file is not where this recording points. Nothing was deleted — the recording entry is kept as it is. Move the file back to that path to play it again.';

/**
 * 읽어 온 값을 화면 상태로 바꾼다.
 *
 * `missingAudio`는 `list_missing_audio`가 돌려준 목록이다 — **레코드는 있는데 파일이 없는**
 * 녹음 전부이며, 이 화면이 보는 녹음이 거기 있는지만 확인한다. 파일의 존재를 판정하는 것은
 * 이 모듈이 아니다.
 *
 * 파일이 없으면 재생 주소를 만들지 않는다. 열 수 없는 자리를 가리키는 주소를 만들어 두면
 * 화면이 "재생할 수 있다"고 말하게 되기 때문이다.
 */
export function loadedRecordingDetail(
  recordingId: string,
  recording: Recording | null,
  missingAudio: readonly MissingAudio[],
  toAudioSource: AudioSourceResolver,
  options: RecordedAtOptions = {},
): RecordingDetailView {
  if (recording === null) {
    // 저장소가 "그런 id는 없다"고 답했다. 무엇을 찾고 있었는지는 잃지 않는다.
    return missingRecording(recordingId);
  }

  const header: RecordingHeader = {
    id: recording.id,
    title: recording.title,
    recordedAtLabel: formatRecordedAt(recording.createdAt, options),
    // Rust가 보낸 값을 그대로 쓴다. 여기서 다시 계산하지 않는다.
    durationLabel: recording.durationLabel,
  };

  if (missingAudio.some((missing) => missing.recordingId === recording.id)) {
    return { kind: 'audioMissing', recording: header, audioPath: recording.audioPath };
  }

  return {
    kind: 'playable',
    recording: header,
    audioSource: toAudioSource(recording.audioPath),
    audioFormat: recording.audioFormat,
  };
}

/**
 * 그런 id의 레코드가 없다.
 *
 * `get_recording`이 `null`을 돌려준 것은 저장소가 정상적으로 답한 결과다 — 실패가 아니며,
 * 파일이 없는 것과도 다른 사실이다. 어느 녹음을 찾고 있었는지는 화면이 알고 있으므로
 * 그것을 그대로 담는다.
 */
export function missingRecording(recordingId: string): RecordingDetailView {
  return { kind: 'notFound', recordingId };
}

/**
 * 저장소에 물어보지 못했다 (§13).
 *
 * 저장소 초기화 실패는 여기로 온다 — 앱이 죽지도 않고, console에만 남지도 않는다.
 * **"녹음이 없다"로 둔갑시키지 않는다.** 읽지 못한 것과 없는 것은 다른 사실이다.
 */
export function failedRecordingDetail(error: unknown): RecordingDetailView {
  return { kind: 'failed', failure: toFailure(error) };
}
