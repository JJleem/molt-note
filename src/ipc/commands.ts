/**
 * 프론트엔드가 부를 수 있는 동작의 전부.
 *
 * `src-tauri/src/lib.rs`가 등록한 여섯 개 command와 1:1이며, 그 밖의 경로는 없다 —
 * **임의의 질의를 보낼 수단이 없다.** 저장소를 아는 코드는 Rust 안에만 있다
 * (`docs/ADR-0001-local-persistence.md` · PRODUCT-SPEC §12).
 *
 * 실패는 예외로 흘리지 않고 언제나 {@link Failure}로 만들어 던진다. 화면은 어떤 실패든
 * 같은 모양으로 받아 사용자에게 보여줄 수 있다 (§13).
 */
import { invoke } from '@tauri-apps/api/core';
import { toFailure } from './failure';
import type { NewRecording, Recording, Settings } from './types';

export type { Failure, FailureKind } from './failure';
export type { NewRecording, ProcessingStatus, Recording, Settings } from './types';

/** command 하나를 부른다. 거절된 값은 언제나 {@link Failure}로 바꿔 던진다. */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toFailure(error);
  }
}

/** 저장된 녹음을 최근 것부터 읽는다. 하나도 없으면 빈 배열이다. */
export function listRecordings(): Promise<Recording[]> {
  return call<Recording[]>('list_recordings');
}

/** 녹음 하나를 읽는다. 그런 id가 없으면 `null`이다. */
export function getRecording(recordingId: string): Promise<Recording | null> {
  return call<Recording | null>('get_recording', { recordingId });
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
