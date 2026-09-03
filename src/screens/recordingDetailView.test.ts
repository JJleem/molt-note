// Recording Detail 화면 상태 변환 테스트.
//
// 네 경로를 전부 본다 — **로딩 · 재생 가능 · 파일 없음 · 조회 실패**. 그리고 저장소가
// "그런 녹음은 없다"고 답한 다섯 번째 경로도 함께 본다. DOM도 Tauri도 오디오 파일도
// 필요하지 않다 (PRODUCT-SPEC §18).
//
// 이 테스트가 지키는 것 하나가 더 있다: **파일이 없는 상태가 레코드를 지우거나 덮어쓰지
// 않는다** (INV-3 · INV-4). 그래서 그 상태에서도 레코드가 말하는 값이 그대로 남는지 본다.
import { describe, expect, it, vi } from 'vitest';
import type { Failure } from '../ipc/failure';
import type { MissingAudio, Recording } from '../ipc/types';
import {
  LOADING_RECORDING_DETAIL,
  MISSING_AUDIO_NOTICE,
  failedRecordingDetail,
  loadedRecordingDetail,
  missingRecording,
} from './recordingDetailView';

/** 저장소가 돌려주는 모양 그대로의 레코드. */
function recording(overrides: Partial<Recording> = {}): Recording {
  return {
    id: 'r-1',
    title: '3DGS Study #04',
    createdAt: '2026-09-01T09:12:00.000Z',
    updatedAt: '2026-09-01T09:12:00.000Z',
    durationMs: 3_151_000,
    // Rust가 만들어 보낸 값이다 (src-tauri/src/domain/duration.rs).
    durationLabel: '52:31',
    audioPath: '/Users/someone/Library/Application Support/molt-note/recordings/r-1.wav',
    audioFormat: 'wav',
    microphone: 'MacBook Microphone',
    currentTranscriptId: null,
    transcriptionStatus: 'none',
    aiStatus: 'none',
    notionStatus: 'none',
    ...overrides,
  };
}

/** `list_missing_audio`가 돌려주는 모양 그대로의 항목. */
function missingAudio(overrides: Partial<MissingAudio> = {}): MissingAudio {
  return {
    recordingId: 'r-1',
    title: '3DGS Study #04',
    audioPath: '/Users/someone/Library/Application Support/molt-note/recordings/r-1.wav',
    createdAt: '2026-09-01T09:12:00.000Z',
    ...overrides,
  };
}

/** 앱에서는 Tauri가 하는 변환. 테스트는 무엇이 들어왔는지 볼 수 있는 것으로 바꿔 넣는다. */
const asSource = (audioPath: string) => `asset://localhost/${audioPath}`;

const UTC = { timeZone: 'UTC' };

describe('아직 읽지 못한 상태', () => {
  it('첫 상태는 로딩이다', () => {
    // "아직 모른다"가 "없다"나 "재생할 수 없다"로 읽히지 않게 한다.
    expect(LOADING_RECORDING_DETAIL.kind).toBe('loading');
  });
});

describe('재생 가능한 상태', () => {
  it('레코드도 파일도 있으면 재생할 수 있는 상태가 된다', () => {
    const view = loadedRecordingDetail('r-1', recording(), [], asSource, UTC);

    expect(view.kind).toBe('playable');
    if (view.kind !== 'playable') return;
    expect(view.audioFormat).toBe('wav');
  });

  it('재생 주소는 레코드가 가리키는 경로를 그대로 변환한 값이다', () => {
    // 경로를 화면이 짓지 않는다 — 저장된 값을 변환에 그대로 넘긴다.
    const convert = vi.fn(asSource);
    const view = loadedRecordingDetail('r-1', recording(), [], convert, UTC);

    expect(convert).toHaveBeenCalledTimes(1);
    expect(convert).toHaveBeenCalledWith(recording().audioPath);
    expect(view.kind === 'playable' && view.audioSource).toBe(asSource(recording().audioPath));
  });

  it('제목 · 날짜 · 길이가 레코드에서 온 값 그대로다', () => {
    const view = loadedRecordingDetail('r-1', recording(), [], asSource, UTC);

    expect(view.kind).toBe('playable');
    if (view.kind !== 'playable') return;
    expect(view.recording.id).toBe('r-1');
    expect(view.recording.title).toBe('3DGS Study #04');
    expect(view.recording.recordedAtLabel).toBe('Sep 1');
    expect(view.recording.durationLabel).toBe('52:31');
  });

  it('길이는 Rust가 보낸 문자열이며 화면이 다시 계산하지 않는다', () => {
    // durationMs와 durationLabel이 어긋나 있어도 화면은 label을 쓴다.
    const view = loadedRecordingDetail(
      'r-1',
      recording({ durationMs: 1, durationLabel: '52:31' }),
      [],
      asSource,
      UTC,
    );

    expect(view.kind === 'playable' && view.recording.durationLabel).toBe('52:31');
  });

  it('다른 녹음의 파일이 없다는 것은 이 녹음의 재생을 막지 않는다', () => {
    const view = loadedRecordingDetail(
      'r-1',
      recording(),
      [missingAudio({ recordingId: 'r-2' })],
      asSource,
      UTC,
    );

    expect(view.kind).toBe('playable');
  });
});

describe('레코드는 있는데 파일이 없는 상태 (INV-3 · INV-4)', () => {
  it('감지된 녹음은 재생 가능한 상태가 되지 않는다', () => {
    const view = loadedRecordingDetail('r-1', recording(), [missingAudio()], asSource, UTC);

    expect(view.kind).toBe('audioMissing');
  });

  it('열 수 없는 자리를 가리키는 재생 주소를 만들지 않는다', () => {
    // 주소가 있으면 화면은 "재생할 수 있다"고 말하게 된다.
    const convert = vi.fn(asSource);

    loadedRecordingDetail('r-1', recording(), [missingAudio()], convert, UTC);

    expect(convert).not.toHaveBeenCalled();
  });

  it('파일이 없어도 레코드가 말하는 것은 그대로 남는다', () => {
    // 이 상태는 감지일 뿐이다. 레코드를 지우거나 비우지 않는다 (R-004).
    const view = loadedRecordingDetail('r-1', recording(), [missingAudio()], asSource, UTC);

    expect(view.kind).toBe('audioMissing');
    if (view.kind !== 'audioMissing') return;
    expect(view.recording.id).toBe('r-1');
    expect(view.recording.title).toBe('3DGS Study #04');
    expect(view.recording.recordedAtLabel).toBe('Sep 1');
    expect(view.recording.durationLabel).toBe('52:31');
  });

  it('레코드가 가리키던 경로를 보여준다', () => {
    // 지우지 않는 파일은 사용자가 찾을 수 있어야 한다 (INV-1).
    const view = loadedRecordingDetail('r-1', recording(), [missingAudio()], asSource, UTC);

    expect(view.kind === 'audioMissing' && view.audioPath).toBe(recording().audioPath);
  });

  it('사용자에게 하는 말이 지웠다고 말하지 않는다', () => {
    expect(MISSING_AUDIO_NOTICE).toMatch(/nothing was deleted/i);
    expect(MISSING_AUDIO_NOTICE).not.toMatch(/\b(removed|cleaned up|recreated|restored)\b/i);
  });

  it('파일이 없는 것은 조회 실패가 아니다', () => {
    // 저장소는 정상적으로 답했다. 둘을 섞으면 사용자가 없는 문제를 고치려 하게 된다 (§13).
    const view = loadedRecordingDetail('r-1', recording(), [missingAudio()], asSource, UTC);

    expect(view.kind).not.toBe('failed');
  });
});

describe('그런 녹음이 없는 상태', () => {
  it('저장소가 null로 답하면 찾던 id를 담은 상태가 된다', () => {
    const view = loadedRecordingDetail('r-9', null, [], asSource, UTC);

    expect(view).toEqual({ kind: 'notFound', recordingId: 'r-9' });
    expect(missingRecording('r-9')).toEqual(view);
  });

  it('없는 녹음은 실패도 아니고 파일 없음도 아니다', () => {
    const view = loadedRecordingDetail('r-9', null, [], asSource, UTC);

    expect(view.kind).not.toBe('failed');
    expect(view.kind).not.toBe('audioMissing');
  });
});

describe('조회 실패 상태', () => {
  const storageFailure: Failure = {
    kind: 'storage',
    message: '로컬 저장소를 열지 못했다.',
    detail: 'unable to open database file',
    sourceDataSafe: true,
    retryable: true,
  };

  it('저장소가 거절하면 화면이 실패 상태가 된다', () => {
    expect(failedRecordingDetail(storageFailure)).toEqual({
      kind: 'failed',
      failure: storageFailure,
    });
  });

  it('실패가 "파일이 없다"로 둔갑하지 않는다', () => {
    // 읽지 못한 것과 파일이 없는 것은 사용자가 할 일이 서로 다르다.
    expect(failedRecordingDetail(storageFailure).kind).not.toBe('audioMissing');
  });

  it('계약과 다른 값으로 거절돼도 보여줄 수 있는 실패가 된다', () => {
    const view = failedRecordingDetail(new Error('command not found: get_recording'));

    expect(view.kind).toBe('failed');
    if (view.kind !== 'failed') return;
    expect(view.failure.message.length).toBeGreaterThan(0);
    expect(view.failure.detail).toBe('command not found: get_recording');
  });
});
