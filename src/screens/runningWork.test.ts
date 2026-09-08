import { describe, expect, it } from 'vitest';
import { hasRunningWork, runningWork } from './runningWork';
import type { AiNoteStatus, NotionSendStatus, TranscriptionStatus } from '../ipc/types';

const transcription = (
  over: Partial<TranscriptionStatus> = {},
): TranscriptionStatus => ({
  state: 'idle',
  recordingId: null,
  transcriptId: null,
  failure: null,
  progress: null,
  partialLines: [],
  ...over,
});

const aiNote = (over: Partial<AiNoteStatus> = {}): AiNoteStatus => ({
  state: 'idle',
  recordingId: null,
  mode: null,
  aiNoteId: null,
  failure: null,
  ...over,
});

const notion = (over: Partial<NotionSendStatus> = {}): NotionSendStatus => ({
  state: 'idle',
  recordingId: null,
  pageId: null,
  createdPage: false,
  failure: null,
  ...over,
});

describe('배경에서 도는 일 (2026-09-08)', () => {
  it('아무것도 돌지 않으면 아무 말도 하지 않는다', () => {
    // "대기 중" 같은 줄을 늘 두면 화면의 소음이 되고, 정작 돌 때 눈에 띄지 않는다.
    const work = runningWork(transcription(), aiNote(), notion());

    expect(work).toEqual([]);
    expect(hasRunningWork(work)).toBe(false);
  });

  it('아직 아무것도 물어보지 못했어도 죽지 않는다', () => {
    expect(runningWork(null, null, null)).toEqual([]);
  });

  it('전사가 도는 동안 그 사실과 진행률을 말한다', () => {
    const work = runningWork(
      transcription({ state: 'running', recordingId: 'r-1', progress: 0.37 }),
      aiNote(),
      notion(),
    );

    expect(work).toHaveLength(1);
    expect(work[0].text).toContain('전사');
    expect(work[0].text).toContain('37');
    expect(work[0].recordingId).toBe('r-1');
  });

  it('진행률을 모르면 그 부분을 지어내지 않는다', () => {
    const work = runningWork(
      transcription({ state: 'running', recordingId: 'r-1', progress: null }),
      aiNote(),
      notion(),
    );

    expect(work[0].text).toBe('전사 중');
    expect(work[0].text).not.toMatch(/\d/);
  });

  it('진행률을 내림한다 — 끝나기 전에 100%가 보이지 않게', () => {
    const work = runningWork(
      transcription({ state: 'running', recordingId: 'r-1', progress: 0.999 }),
      aiNote(),
      notion(),
    );

    expect(work[0].text).toContain('99');
  });

  /**
   * **끝난 것은 여기서 말하지 않는다.** 결과는 그 녹음의 화면이 말하며, 그 자리가
   * §13의 세 질문에 답하는 자리다. 두 곳이 같은 사실을 서로 다른 문장으로 말하면
   * 사람이 어느 쪽을 믿어야 할지 모르게 된다.
   */
  it('끝난 것도 실패한 것도 이 줄에 오지 않는다', () => {
    for (const state of ['done', 'failed', 'idle'] as const) {
      expect(runningWork(transcription({ state, recordingId: 'r-1' }), aiNote(), notion())).toEqual(
        [],
      );
    }
    for (const state of ['done', 'failed', 'idle', 'noTranscript'] as const) {
      expect(runningWork(transcription(), aiNote({ state, recordingId: 'r-1' }), notion())).toEqual(
        [],
      );
    }
  });

  it('셋이 함께 돌면 파이프라인 순서로 말한다', () => {
    // 사람이 읽는 순서가 일이 일어나는 순서와 같아야 한다.
    const work = runningWork(
      transcription({ state: 'running', recordingId: 'r-1' }),
      aiNote({ state: 'running', recordingId: 'r-2' }),
      notion({ state: 'running', recordingId: 'r-3' }),
    );

    expect(work.map((one) => one.recordingId)).toEqual(['r-1', 'r-2', 'r-3']);
    expect(hasRunningWork(work)).toBe(true);
  });

  it('어느 녹음인지 모르면 지어내지 않는다', () => {
    const work = runningWork(
      transcription({ state: 'running', recordingId: null }),
      aiNote(),
      notion(),
    );

    expect(work[0].recordingId).toBeNull();
  });
});
