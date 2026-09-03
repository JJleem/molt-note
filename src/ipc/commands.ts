/**
 * 프론트엔드가 부를 수 있는 동작의 전부.
 *
 * `src-tauri/src/lib.rs`가 등록한 열여섯 개 command와 1:1이며, 그 밖의 경로는 없다 —
 * **임의의 질의를 보낼 수단이 없다.** 저장소를 아는 코드는 Rust 안에만 있다
 * (`docs/ADR-0001-local-persistence.md` · PRODUCT-SPEC §12).
 *
 * 실패는 예외로 흘리지 않고 언제나 {@link Failure}로 만들어 던진다. 화면은 어떤 실패든
 * 같은 모양으로 받아 사용자에게 보여줄 수 있다 (§13).
 *
 * ## command가 아닌 통로가 하나 있다 — 저장된 녹음의 재생
 *
 * 오디오 파일만은 command로 흐르지 않는다 ({@link recordingAudioSource}). 파일 바이트를
 * IPC에 실으면 한 시간짜리 녹음을 통째로 메모리에 올려야 하므로, 재생은 Tauri v2의
 * **asset protocol**을 지난다. 그 통로가 열리는 자리는 녹음 디렉터리 하나뿐이며 그 범위를
 * 정하는 것은 backend다 (`src-tauri/src/lib.rs` · docs/ADR-0006-audio-playback.md).
 * 저장소에 대한 질의는 여전히 아래 command가 전부다.
 */
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { toFailure } from './failure';
import type {
  InputDevice,
  MissingAudio,
  NewRecording,
  Recording,
  SessionStatus,
  Settings,
  StoppedRecording,
  Transcript,
  TranscriptionStatus,
} from './types';

export type { Failure, FailureKind } from './failure';
export type {
  CaptureReport,
  InputDevice,
  MissingAudio,
  NewRecording,
  ProcessingStatus,
  Recording,
  SessionState,
  SessionStatus,
  Settings,
  StoppedRecording,
  Transcript,
  TranscriptSegment,
  TranscriptionState,
  TranscriptionStatus,
} from './types';

/** command 하나를 부른다. 거절된 값은 언제나 {@link Failure}로 바꿔 던진다. */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toFailure(error);
  }
}

/**
 * 저장된 녹음 파일을 webview가 재생할 수 있는 주소로 바꾼다 (Phase 2B 요구사항 7).
 *
 * **command가 아니다.** 파일 바이트는 IPC를 지나지 않고 Tauri v2의 asset protocol을 지난다
 * (`src-tauri/Cargo.toml`의 `protocol-asset` feature · `src-tauri/tauri.conf.json`의
 * `app.security.assetProtocol`). 그래서 긴 녹음도 통째로 메모리에 올리지 않고 재생된다.
 *
 * **어떤 경로가 실제로 열리는지는 이 함수가 정하지 않는다.** 주소를 만드는 일과 그 자리를
 * 여는 일은 갈라져 있고, 여는 쪽은 backend가 녹음 디렉터리 하나로 제한한다
 * (`src-tauri/src/lib.rs`) — 다른 경로를 넣어도 그 파일은 열리지 않는다.
 *
 * 만들어진 주소는 로컬 webview 안에서만 쓰인다. 오디오가 기기 밖으로 나가는 경로는 이 앱에
 * 없다 (PRODUCT-SPEC §12 · INV-6).
 */
export function recordingAudioSource(audioPath: string): string {
  return convertFileSrc(audioPath);
}

/** 저장된 녹음을 최근 것부터 읽는다. 하나도 없으면 빈 배열이다. */
export function listRecordings(): Promise<Recording[]> {
  return call<Recording[]>('list_recordings');
}

/** 녹음 하나를 읽는다. 그런 id가 없으면 `null`이다. */
export function getRecording(recordingId: string): Promise<Recording | null> {
  return call<Recording | null>('get_recording', { recordingId });
}

/**
 * 저장된 Transcript 하나를 **segment까지** 읽는다. 그런 id가 없으면 `null`이다.
 *
 * 화면은 `Recording.currentTranscriptId`가 가리키는 것을 이 함수로 읽는다 (§7.2).
 *
 * **읽기뿐이다.** Transcript를 고치거나 지우는 command는 없다 — Transcript는 immutable이고
 * (§7.1 · INV-2) 저장소가 내놓는 쓰기 경로도 추가 하나뿐이다. 그래서 화면에서 시작한 어떤
 * 동작도 이미 저장된 전사 내용을 바꾸지 못한다.
 */
export function getTranscript(transcriptId: string): Promise<Transcript | null> {
  return call<Transcript | null>('get_transcript', { transcriptId });
}

/** 녹음 하나를 저장하고 저장된 모습을 받는다. */
export function createRecording(recording: NewRecording): Promise<Recording> {
  return call<Recording>('create_recording', { recording });
}

/** 녹음 레코드 하나를 지운다. 지웠으면 `true`, 그런 id가 없었으면 `false`다. */
export function deleteRecording(recordingId: string): Promise<boolean> {
  return call<boolean>('delete_recording', { recordingId });
}

/** 저장된 설정을 읽는다. 저장된 적이 없으면 기본값이 온다. */
export function getSettings(): Promise<Settings> {
  return call<Settings>('get_settings');
}

/** 설정을 저장하고, 저장된 결과를 받는다. */
export function updateSettings(settings: Settings): Promise<Settings> {
  return call<Settings>('update_settings', { settings });
}

/**
 * 고를 수 있는 입력 장치를 표시 순서로 읽는다 (기본 장치가 먼저).
 *
 * 하나도 없으면 빈 배열이다 — 오류가 아니다. 목록은 부를 때마다 새로 만들어지므로
 * 장치가 꽂히거나 빠진 뒤에는 다시 부른다.
 */
export function listInputDevices(): Promise<InputDevice[]> {
  return call<InputDevice[]>('list_input_devices');
}

/**
 * 고른 장치를 열고 녹음을 시작한다.
 *
 * `deviceKey`는 {@link listInputDevices}가 준 `key`다. 이미 녹음 중이면 실패한다 —
 * 진행 중인 녹음이 조용히 버려지지 않는다.
 *
 * **진행 중인 session을 소유하는 것은 backend다.** 이 함수는 핸들을 돌려주지 않으며,
 * 화면이 사라져도 녹음은 계속된다 (R-001 · docs/ADR-0004-recording-session-lifecycle.md).
 * 지금 상태는 {@link captureStatus}로 물어보고, 결과는 {@link stopCapture}가 돌려준다.
 */
export function startCapture(deviceKey: string): Promise<void> {
  return call<void>('start_capture', { deviceKey });
}

/**
 * 녹음을 일시정지한다.
 *
 * 장치도 파일도 열려 있는 채로 남는다 — 이 시점 이후의 소리는 파일에 들어가지 않고,
 * 흐르는 시간도 녹음 길이에 더해지지 않는다. 녹음 중이 아니면 실패한다.
 */
export function pauseCapture(): Promise<void> {
  return call<void>('pause_capture');
}

/**
 * 일시정지한 녹음을 다시 시작한다. **같은 파일에 이어 쓴다.**
 *
 * 일시정지 상태가 아니면 실패한다.
 */
export function resumeCapture(): Promise<void> {
  return call<void>('resume_capture');
}

/**
 * 녹음을 정지한다. **성공은 파일이 확정된 것을 뜻한다** (PRODUCT-SPEC §6의 R-002).
 *
 * 이 함수가 값을 돌려줬다면 네 가지가 모두 성립한 것이다 — 파일이 존재하고, 크기가 유효
 * 최소치를 넘고, 포맷을 알고 있고, Recording 레코드가 저장됐다. 그러지 못하면 실패이며,
 * **그 실패는 확정된 파일이 어디 남아 있는지 함께 말한다** — 어떤 실패 경로도 이미 녹음된
 * audio를 지우지 않는다 (INV-3 · INV-4 · docs/ADR-0004-recording-session-lifecycle.md).
 *
 * `title`은 사용자가 입력한 제목이다. 비어 있거나 없으면 Rust가 저장 시각에서 만든다 —
 * 이름이 없다는 이유로 방금 녹음한 것이 유실되지 않게 한다.
 */
export function stopCapture(title?: string | null): Promise<StoppedRecording> {
  return call<StoppedRecording>('stop_capture', { title: title ?? null });
}

/**
 * **레코드는 있는데 오디오 파일이 없는** 녹음을 읽는다. 하나도 없으면 빈 배열이다.
 *
 * 정지 경로는 이 상태를 만들지 않는다 — 레코드는 파일이 확인된 뒤에만 저장된다. 이 목록은
 * 파일이 앱 밖에서 옮겨지거나 지워졌을 때를 위한 **감지 수단**이며, 부른다고 해서 레코드가
 * 지워지거나 고쳐지지 않는다.
 */
export function listMissingAudio(): Promise<MissingAudio[]> {
  return call<MissingAudio[]>('list_missing_audio');
}

/**
 * 지금 녹음이 어떤 상태이고 얼마나 진행됐는지 물어본다.
 *
 * 진행 중인 녹음이 없으면 `idle`과 `0:00`이 온다 — 오류가 아니다. 경과 시간 문자열은
 * Rust가 만들어 보낸다. **화면이 길이를 계산하지 않는다.**
 */
export function captureStatus(): Promise<SessionStatus> {
  return call<SessionStatus>('capture_status');
}

/**
 * 녹음 하나의 전사를 시작한다. **돌아오는 것은 접수 사실이지 전사 결과가 아니다.**
 *
 * 실제 전사는 backend의 배경 스레드에서 돌므로 이 호출은 바로 끝난다 — 1시간짜리 녹음을 걸어도
 * 화면과 다른 command가 멈추지 않는다 (`phase-prompt/03` 요구 3). 결과는
 * {@link transcriptionStatus}로 물어본다.
 *
 * **진행 중인 전사를 소유하는 것은 backend다.** 이 함수는 핸들을 돌려주지 않으며, 화면이
 * 사라져도 전사는 계속된다 (R-001과 같은 규약).
 *
 * 이미 전사 중이면 실패한다 — 같은 녹음이어도 마찬가지다. 두 번째 요청이 조용히 사라지지
 * 않는다. 여러 녹음을 줄 세우는 큐는 아직 없다 (PRODUCT-SPEC §16 DEFERRED).
 */
export function startTranscription(recordingId: string): Promise<TranscriptionStatus> {
  return call<TranscriptionStatus>('start_transcription', { recordingId });
}

/**
 * 지금 전사가 어떤 상태인지 물어본다. **전사가 도는 동안에도 즉시 답한다.**
 *
 * 아직 아무것도 걸지 않았으면 `idle`이 온다 — 오류가 아니다. 실패했으면 그 {@link Failure}가
 * 그대로 실려 오므로 화면은 무엇이 실패했는지, 원본이 안전한지, 다시 시도할 수 있는지를
 * 그 값에서 읽는다 (§13).
 */
export function transcriptionStatus(): Promise<TranscriptionStatus> {
  return call<TranscriptionStatus>('transcription_status');
}
