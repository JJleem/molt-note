// Recordings 화면 상태 변환 테스트.
//
// 저장소에서 온 응답이 화면이 그릴 수 있는 값으로 바뀌는지 본다 —
// 목록 · 빈 목록 · 실패 세 경로 전부다. DOM도 Tauri도 필요하지 않다 (§18).
import { describe, expect, it } from 'vitest';
import type { Failure } from '../ipc/failure';
import type { Recording } from '../ipc/types';
import {
  LOADING_RECORDINGS,
  failedRecordings,
  formatRecordedAt,
  loadedRecordings,
} from './recordingsView';

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
    audioPath: '/tmp/r-1.wav',
    audioFormat: 'wav',
    microphone: null,
    currentTranscriptId: null,
    transcriptionStatus: 'done',
    aiStatus: 'none',
    notionStatus: 'failed',
    ...overrides,
  };
}

const UTC = { timeZone: 'UTC' };

describe('목록 상태', () => {
  it('첫 상태는 아직 읽지 못한 상태다', () => {
    // 빈 목록과 "아직 모른다"가 화면에서 같은 모양이 되지 않게 한다.
    expect(LOADING_RECORDINGS.kind).toBe('loading');
  });

  it('레코드가 하나도 없으면 실패가 아니라 empty state다', () => {
    expect(loadedRecordings([])).toEqual({ kind: 'empty' });
  });

  it('저장소에서 온 값이 그대로 목록 항목이 된다', () => {
    const view = loadedRecordings([recording()], UTC);

    expect(view.kind).toBe('list');
    if (view.kind !== 'list') return;

    const [item] = view.items;
    expect(item.id).toBe('r-1');
    expect(item.title).toBe('3DGS Study #04');
    expect(item.durationLabel).toBe('52:31');
    expect(item.recordedAtLabel).toBe('Sep 1');
  });

  it('저장소 순서를 화면이 다시 정렬하지 않는다', () => {
    // 정렬은 저장소의 계약이다(최근 것부터). 화면이 그것을 뒤집지 않는다.
    const view = loadedRecordings(
      [recording({ id: 'r-2' }), recording({ id: 'r-1' })],
      UTC,
    );

    expect(view.kind === 'list' && view.items.map((item) => item.id)).toEqual(['r-2', 'r-1']);
  });

  it('항목마다 transcription · AI · Notion 세 상태를 보여준다', () => {
    const view = loadedRecordings([recording()], UTC);
    expect(view.kind).toBe('list');
    if (view.kind !== 'list') return;

    const [item] = view.items;
    expect(item.statuses).toHaveLength(3);
    expect(item.statuses.map((badge) => badge.label)).toEqual(['Transcript', 'AI Note', 'Notion']);
    expect(item.statuses.map((badge) => badge.status)).toEqual(['done', 'none', 'failed']);
    // 각 상태에 사람이 읽는 표현이 있다.
    for (const badge of item.statuses) {
      expect(badge.text.length).toBeGreaterThan(0);
    }
  });

  it('Transcript badge가 저장된 전사 상태를 그대로 보여준다', () => {
    // 전사 상태는 목록과 상세 **양쪽에서** 실제 값으로 보여야 한다
    // (phase-prompt/03 요구 3). 목록 쪽이 이 badge다 — 다섯 상태가 서로 다르게 읽힌다.
    const statuses = ['none', 'pending', 'running', 'done', 'failed'] as const;

    const texts = statuses.map((transcriptionStatus) => {
      const view = loadedRecordings([recording({ transcriptionStatus })], UTC);
      if (view.kind !== 'list') throw new Error('목록이어야 한다');

      const [badge] = view.items[0].statuses;
      expect(badge.label).toBe('Transcript');
      // 저장된 값이 그대로 실린다 — 화면이 다른 상태로 바꾸지 않는다.
      expect(badge.status).toBe(transcriptionStatus);
      return badge.text;
    });

    expect(new Set(texts).size, `다섯 상태가 같은 말로 읽힌다: ${texts.join(' · ')}`).toBe(
      statuses.length,
    );
  });

  it('아직 시도하지 않은 상태가 실패처럼 읽히지 않는다', () => {
    // none은 정상 상태다 (§7 · INV-8).
    const view = loadedRecordings(
      [recording({ transcriptionStatus: 'none', aiStatus: 'none', notionStatus: 'none' })],
      UTC,
    );
    expect(view.kind).toBe('list');
    if (view.kind !== 'list') return;

    for (const badge of view.items[0].statuses) {
      expect(badge.text).toBe('Not started');
    }
  });

  it('길이는 Rust가 보낸 문자열이며 화면이 다시 계산하지 않는다', () => {
    // durationMs와 durationLabel이 어긋나 있어도 화면은 label을 쓴다 —
    // 초를 mm:ss로 바꾸는 규칙이 여기에 없다는 뜻이다.
    const view = loadedRecordings([recording({ durationMs: 1, durationLabel: '52:31' })], UTC);

    expect(view.kind === 'list' && view.items[0].durationLabel).toBe('52:31');
  });
});

describe('날짜 라벨', () => {
  it('저장된 시각을 §5 A의 형태로 만든다', () => {
    expect(formatRecordedAt('2026-09-01T09:12:00.000Z', UTC)).toBe('Sep 1');
    expect(formatRecordedAt('2026-08-31T23:59:59.000Z', UTC)).toBe('Aug 31');
  });

  it('읽을 수 없는 시각은 지어내지 않고 저장된 값을 그대로 보여준다', () => {
    expect(formatRecordedAt('not-a-timestamp', UTC)).toBe('not-a-timestamp');
    expect(formatRecordedAt('', UTC)).toBe('');
  });
});

describe('실패 상태', () => {
  const storageFailure: Failure = {
    kind: 'storage',
    message: '로컬 저장소를 열지 못했다.',
    detail: 'unable to open database file',
    sourceDataSafe: true,
    retryable: true,
  };

  it('저장소가 거절하면 화면이 실패 상태가 된다', () => {
    // 저장소 초기화 실패는 모든 command 응답으로 돌아온다 (commands/mod.rs).
    const view = failedRecordings(storageFailure);

    expect(view).toEqual({ kind: 'failed', failure: storageFailure });
  });

  it('실패는 empty state로 둔갑하지 않는다', () => {
    // 읽지 못한 것과 하나도 없는 것은 다른 사실이다.
    expect(failedRecordings(storageFailure).kind).not.toBe('empty');
  });

  it('계약과 다른 값으로 거절돼도 보여줄 수 있는 실패가 된다', () => {
    const view = failedRecordings(new Error('command not found: list_recordings'));

    expect(view.kind).toBe('failed');
    if (view.kind !== 'failed') return;
    expect(view.failure.message.length).toBeGreaterThan(0);
    expect(view.failure.detail).toBe('command not found: list_recordings');
  });
});
