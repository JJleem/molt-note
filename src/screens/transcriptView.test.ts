// Transcript 탭 상태 변환 테스트.
//
// 다섯 경로를 전부 본다 — **아직 없음 · 대기 · 진행 중 · 완료 · 실패**. whisper도 모델도
// 오디오 파일도 DOM도 필요하지 않다 (PRODUCT-SPEC §18 · phase-prompt/03 요구 9).
//
// 이 테스트가 지키는 것이 둘 더 있다:
//   1. ms → HH:MM:SS 변환이 **리터럴 기대값**으로 고정된다. 계산식을 다시 적어 비교하면
//      같은 실수를 두 번 하게 되므로, 기대값은 사람이 읽고 쓴 문자열이다.
//   2. 실패가 아무것도 잃지 않는다 (INV-1 · INV-2 · INV-3) — 이미 있던 Transcript는 실패
//      상태에서도 그대로 남고, 이 모듈에는 그것을 지우는 함수 자체가 없다.
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import type { Recording, Transcript, TranscriptionStatus } from '../ipc/types';
import {
  LOADING_TRANSCRIPT_TAB,
  NO_TRANSCRIPT_TEXT,
  TRANSCRIPTION_FAILED_HEADLINE,
  TRANSCRIPTION_PRESERVED_NOTICE,
  TRANSCRIPTION_START_REJECTED_HEADLINE,
  TRANSCRIPTION_STATUS_HEADLINE,
  UNKNOWN_FAILURE_NOTICE,
  formatTimestamp,
  transcriptLines,
  transcriptTab,
  transcriptTrouble,
} from './transcriptView';

/** 저장소가 돌려주는 모양 그대로의 레코드. */
function recording(overrides: Partial<Recording> = {}): Recording {
  return {
    id: 'r-1',
    title: '3DGS Study #04',
    createdAt: '2026-09-01T09:12:00.000Z',
    updatedAt: '2026-09-01T09:12:00.000Z',
    durationMs: 3_151_000,
    durationLabel: '52:31',
    audioPath: '/recordings/r-1.wav',
    audioFormat: 'wav',
    microphone: 'MacBook Microphone',
    currentTranscriptId: null,
    transcriptionStatus: 'none',
    aiStatus: 'none',
    notionStatus: 'none',
    ...overrides,
  };
}

/** `get_transcript`가 돌려주는 모양 그대로의 Transcript. §7 요구 6의 예시 그대로다. */
function transcript(overrides: Partial<Transcript> = {}): Transcript {
  return {
    id: 't-1',
    recordingId: 'r-1',
    language: 'ko',
    segments: [
      { startMs: 134_000, endMs: 141_000, text: '그러면 이번에는 PLY 먼저 변환하고' },
      { startMs: 141_000, endMs: 148_500, text: '그다음 SOG 변환 확인하면 될 것 같아요.' },
    ],
    rawText: '그러면 이번에는 PLY 먼저 변환하고\n그다음 SOG 변환 확인하면 될 것 같아요.',
    createdAt: '2026-09-03T04:50:26.000Z',
    engine: 'whisper.cpp',
    model: 'ggml-base.bin',
    ...overrides,
  };
}

function status(overrides: Partial<TranscriptionStatus> = {}): TranscriptionStatus {
  return { state: 'idle', recordingId: null, transcriptId: null, failure: null, ...overrides };
}

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: '전사에 쓸 모델 파일을 찾지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: false,
    ...overrides,
  };
}

describe('ms → HH:MM:SS', () => {
  it('요구 6의 예시를 그대로 만든다', () => {
    // phase-prompt/03 요구 6: `00:02:14 → 00:02:21`.
    expect(formatTimestamp(134_000)).toBe('00:02:14');
    expect(formatTimestamp(141_000)).toBe('00:02:21');
  });

  it('0은 00:00:00이다', () => {
    expect(formatTimestamp(0)).toBe('00:00:00');
  });

  it('1초 미만은 버린다 — 반올림하지 않는다', () => {
    // 반올림하면 표시가 실제 음성보다 뒤로 간다.
    expect(formatTimestamp(1)).toBe('00:00:00');
    expect(formatTimestamp(499)).toBe('00:00:00');
    expect(formatTimestamp(500)).toBe('00:00:00');
    expect(formatTimestamp(999)).toBe('00:00:00');
    expect(formatTimestamp(1_000)).toBe('00:00:01');
    expect(formatTimestamp(1_999)).toBe('00:00:01');
  });

  it('분 경계에서 올림 자리가 바뀐다', () => {
    expect(formatTimestamp(59_000)).toBe('00:00:59');
    expect(formatTimestamp(59_999)).toBe('00:00:59');
    expect(formatTimestamp(60_000)).toBe('00:01:00');
    expect(formatTimestamp(61_000)).toBe('00:01:01');
  });

  it('정확히 1시간과 그 직전이 갈린다', () => {
    expect(formatTimestamp(3_599_000)).toBe('00:59:59');
    expect(formatTimestamp(3_599_999)).toBe('00:59:59');
    expect(formatTimestamp(3_600_000)).toBe('01:00:00');
  });

  it('1시간을 넘어도 자릿수가 깨지지 않는다', () => {
    expect(formatTimestamp(3_601_000)).toBe('01:00:01');
    expect(formatTimestamp(3_661_000)).toBe('01:01:01');
    expect(formatTimestamp(3_909_000)).toBe('01:05:09');
    expect(formatTimestamp(36_000_000)).toBe('10:00:00');
    // 100시간이 넘어도 시간 자리를 자르지 않는다.
    expect(formatTimestamp(360_001_000)).toBe('100:00:01');
  });

  it('있을 수 없는 값은 0으로 보되 지어내지 않는다', () => {
    expect(formatTimestamp(-1)).toBe('00:00:00');
    expect(formatTimestamp(-3_600_000)).toBe('00:00:00');
    expect(formatTimestamp(Number.NaN)).toBe('00:00:00');
    expect(formatTimestamp(Number.POSITIVE_INFINITY)).toBe('00:00:00');
  });

  it('같은 입력은 언제나 같은 문자열이 된다', () => {
    // 순수 함수다 — 시계도 로케일도 시간대도 보지 않는다.
    for (const ms of [0, 999, 134_000, 3_600_000]) {
      expect(formatTimestamp(ms)).toBe(formatTimestamp(ms));
    }
  });
});

describe('segment 표시', () => {
  it('시작과 종료 timestamp가 문장과 함께 보인다', () => {
    const lines = transcriptLines(transcript());

    expect(lines).toHaveLength(2);
    expect(lines[0].startLabel).toBe('00:02:14');
    expect(lines[0].endLabel).toBe('00:02:21');
    expect(lines[0].rangeLabel).toBe('00:02:14 → 00:02:21');
    expect(lines[0].text).toBe('그러면 이번에는 PLY 먼저 변환하고');
    expect(lines[1].rangeLabel).toBe('00:02:21 → 00:02:28');
  });

  it('저장된 순서를 화면이 다시 정렬하지 않는다', () => {
    const lines = transcriptLines(
      transcript({
        segments: [
          { startMs: 5_000, endMs: 6_000, text: '두 번째' },
          { startMs: 1_000, endMs: 2_000, text: '첫 번째' },
        ],
      }),
    );

    expect(lines.map((line) => line.text)).toEqual(['두 번째', '첫 번째']);
  });

  it('문장이 하나도 없는 Transcript도 지어내지 않는다', () => {
    expect(transcriptLines(transcript({ segments: [] }))).toEqual([]);
  });
});

describe('전사가 아직 없다', () => {
  it('첫 상태는 로딩이며 "없음"과 구분된다', () => {
    expect(LOADING_TRANSCRIPT_TAB.kind).toBe('loading');
  });

  it('아직 전사한 적이 없으면 none이고 시작 수단이 있다', () => {
    const view = transcriptTab(recording(), null, status());

    expect(view.kind).toBe('none');
    if (view.kind !== 'none') return;
    expect(view.text).toBe(NO_TRANSCRIPT_TEXT);
    // 자동 전사 설정과 무관하게 이 화면에서 수동으로 시작할 수 있다 (요구 2).
    expect(view.start).toEqual({
      kind: 'start',
      label: 'Start transcription',
      recordingId: 'r-1',
    });
  });

  it('아직 물어보지 못한 전사 상태를 idle로 접지 않는다', () => {
    // live가 null인 것은 "전사 중이 아니다"가 아니라 "아직 모른다"이다.
    expect(transcriptTab(recording(), null, null).kind).toBe('none');
  });

  it('레코드가 가리키는 Transcript를 아직 읽지 못했으면 로딩이다', () => {
    const view = transcriptTab(recording({ currentTranscriptId: 't-1' }), null, status());

    expect(view.kind).toBe('loading');
  });
});

describe('대기와 진행 중', () => {
  it('저장된 pending과 running이 서로 다른 상태다', () => {
    const pending = transcriptTab(recording({ transcriptionStatus: 'pending' }), null, status());
    const running = transcriptTab(recording({ transcriptionStatus: 'running' }), null, status());

    expect(pending.kind).toBe('pending');
    expect(running.kind).toBe('running');
  });

  it('지금 이 녹음을 전사 중이면 저장된 상태보다 그것이 먼저다', () => {
    // 상태가 저장되기 전에도 화면은 진행 중이라는 것을 안다.
    const view = transcriptTab(
      recording({ transcriptionStatus: 'none' }),
      null,
      status({ state: 'running', recordingId: 'r-1' }),
    );

    expect(view.kind).toBe('running');
  });

  it('다른 녹음의 전사는 이 화면의 상태가 되지 않는다', () => {
    const view = transcriptTab(
      recording({ id: 'r-1' }),
      null,
      status({ state: 'running', recordingId: 'r-2' }),
    );

    expect(view.kind).toBe('none');
  });

  it('재전사가 도는 동안에도 이미 있던 Transcript가 사라지지 않는다', () => {
    // 새 Transcript가 생겨도 이전 것을 화면이 지우지 않는다 (§7.1 · INV-2).
    const view = transcriptTab(
      recording({ transcriptionStatus: 'done', currentTranscriptId: 't-1' }),
      transcript(),
      status({ state: 'running', recordingId: 'r-1' }),
    );

    expect(view.kind).toBe('running');
    if (view.kind !== 'running') return;
    expect(view.kept.map((line) => line.rangeLabel)).toEqual([
      '00:02:14 → 00:02:21',
      '00:02:21 → 00:02:28',
    ]);
  });
});

describe('완료', () => {
  it('전사된 문장을 timestamp와 함께 보여준다', () => {
    const view = transcriptTab(
      recording({ transcriptionStatus: 'done', currentTranscriptId: 't-1' }),
      transcript(),
      status({ state: 'done', recordingId: 'r-1', transcriptId: 't-1' }),
    );

    expect(view.kind).toBe('done');
    if (view.kind !== 'done') return;
    expect(view.lines[0].rangeLabel).toBe('00:02:14 → 00:02:21');
    expect(view.lines[0].text).toBe('그러면 이번에는 PLY 먼저 변환하고');
    // 무엇으로 만들어졌는지가 함께 보인다 (provenance · §7).
    expect(view.language).toBe('ko');
    expect(view.engine).toBe('whisper.cpp');
    expect(view.model).toBe('ggml-base.bin');
  });

  it('언어를 모르는 Transcript도 그대로 보여준다', () => {
    const view = transcriptTab(
      recording({ transcriptionStatus: 'done', currentTranscriptId: 't-1' }),
      transcript({ language: null }),
      status(),
    );

    expect(view.kind === 'done' && view.language).toBeNull();
  });
});

describe('실패', () => {
  it('무엇이 실패했는지 · 원본이 남아 있다는 사실 · 재시도 수단이 함께 보인다', () => {
    // §13이 요구하는 세 가지가 전부 값으로 있다.
    const view = transcriptTab(
      recording({ transcriptionStatus: 'failed' }),
      null,
      status({
        state: 'failed',
        recordingId: 'r-1',
        failure: failure('transcriptionEngineFailed', {
          message: '전사 엔진이 끝내지 못했다.',
          retryable: true,
        }),
      }),
    );

    expect(view.kind).toBe('failed');
    if (view.kind !== 'failed') return;
    expect(view.headline).toBe(TRANSCRIPTION_FAILED_HEADLINE);
    expect(view.failure?.message).toBe('전사 엔진이 끝내지 못했다.');
    expect(view.cause).toBe('other');
    expect(view.preservedNotice).toBe(TRANSCRIPTION_PRESERVED_NOTICE);
    expect(view.retry).toEqual({
      kind: 'retry',
      label: 'Try transcription again',
      recordingId: 'r-1',
    });
  });

  it('모델이 없어서 실패한 것이 일반 실패와 구분되고 해결 방법이 함께 나온다', () => {
    const view = transcriptTab(
      recording({ transcriptionStatus: 'failed' }),
      null,
      status({
        state: 'failed',
        recordingId: 'r-1',
        failure: failure('transcriptionModelMissing'),
      }),
    );

    expect(view.kind).toBe('failed');
    if (view.kind !== 'failed') return;
    expect(view.cause).toBe('modelMissing');
    expect(view.resolution).not.toBeNull();
    expect(view.resolution).toContain('Settings');
    // 다시 시도할 수 없는 실패라도 재시도 수단은 남는다 — 모델을 둔 뒤에 다시 하면 된다.
    expect(view.failure?.retryable).toBe(false);
    expect(view.retry.kind).toBe('retry');
  });

  it('쓸 수 없는 모델은 없는 모델과 또 다른 상황이다', () => {
    const view = transcriptTab(
      recording({ transcriptionStatus: 'failed' }),
      null,
      status({
        state: 'failed',
        recordingId: 'r-1',
        failure: failure('transcriptionModelUnusable'),
      }),
    );

    expect(view.kind === 'failed' && view.cause).toBe('modelUnusable');
  });

  it('일반 실패에는 없는 해결 절차를 지어내지 않는다', () => {
    const view = transcriptTab(
      recording({ transcriptionStatus: 'failed' }),
      null,
      status({
        state: 'failed',
        recordingId: 'r-1',
        failure: failure('transcriptionEngineFailed', { retryable: true }),
      }),
    );

    expect(view.kind === 'failed' && view.resolution).toBeNull();
  });

  it('앱을 다시 켠 뒤에는 실패했다는 사실만 알고 이유를 지어내지 않는다', () => {
    // 저장된 것은 상태뿐이다. 지금 이 앱은 그 시도를 하지 않았다.
    const view = transcriptTab(recording({ transcriptionStatus: 'failed' }), null, status());

    expect(view.kind).toBe('failed');
    if (view.kind !== 'failed') return;
    expect(view.failure).toBeNull();
    expect(view.cause).toBe('unknown');
    expect(view.resolution).toBe(UNKNOWN_FAILURE_NOTICE);
    expect(view.retry.recordingId).toBe('r-1');
  });

  it('재전사가 실패해도 이미 있던 Transcript가 그대로 보인다', () => {
    // 실패한 시도 때문에 이미 유효한 Transcript를 잃지 않는다 (§7.2 · INV-2).
    const view = transcriptTab(
      recording({ transcriptionStatus: 'failed', currentTranscriptId: 't-1' }),
      transcript(),
      status({
        state: 'failed',
        recordingId: 'r-1',
        failure: failure('transcriptionEngineFailed', { retryable: true }),
      }),
    );

    expect(view.kind).toBe('failed');
    if (view.kind !== 'failed') return;
    expect(view.kept.map((line) => line.text)).toEqual([
      '그러면 이번에는 PLY 먼저 변환하고',
      '그다음 SOG 변환 확인하면 될 것 같아요.',
    ]);
    expect(view.preservedNotice).toContain('Nothing was deleted');
  });

  it('저장된 실패 상태보다 지금 성공한 재전사가 먼저다', () => {
    // 실패한 뒤 다시 시도해서 성공하면 화면은 새 Transcript를 보여준다.
    const view = transcriptTab(
      recording({ transcriptionStatus: 'done', currentTranscriptId: 't-2' }),
      transcript({ id: 't-2' }),
      status({ state: 'done', recordingId: 'r-1', transcriptId: 't-2' }),
    );

    expect(view.kind).toBe('done');
  });
});

describe('요청이 거절됐을 때', () => {
  it('시작 거절이 전사 실패와 다른 사실로 남는다', () => {
    // 이미 다른 녹음을 전사 중일 때가 여기로 온다 (Transcriber::already_running).
    const trouble = transcriptTrouble(
      'start',
      failure('invalidInput', {
        message: '다른 녹음을 전사하고 있다. 그것이 끝난 뒤에 시작할 수 있다.',
        retryable: true,
      }),
    );

    expect(trouble.headline).toBe(TRANSCRIPTION_START_REJECTED_HEADLINE);
    expect(trouble.headline).not.toBe(TRANSCRIPTION_FAILED_HEADLINE);
    expect(trouble.failure.message).toContain('다른 녹음');
  });

  it('상태를 읽지 못한 것과 전사가 실패한 것이 갈라진다', () => {
    const trouble = transcriptTrouble('status', failure('unexpected', { retryable: true }));

    expect(trouble.headline).toBe(TRANSCRIPTION_STATUS_HEADLINE);
    expect(trouble.headline).not.toBe(TRANSCRIPTION_START_REJECTED_HEADLINE);
  });

  it('구조화되지 않은 거절도 화면에 보일 수 있는 모양이 된다', () => {
    // 실패가 console에만 남고 끝나지 않는다 (§13).
    const trouble = transcriptTrouble('start', '알 수 없는 오류');

    expect(trouble.failure.kind).toBe('unexpected');
    expect(trouble.failure.detail).toBe('알 수 없는 오류');
  });
});

describe('이 모듈이 하지 않는 일', () => {
  it('Recording이나 Transcript를 지우거나 고치는 함수가 없다', () => {
    // 화면에서 시작할 수 있는 동작은 시작과 재시도 둘뿐이다. transcript 편집·삭제 UI는
    // 범위 밖이며(phase-prompt/03의 Out of Scope), 그 수단이 값에도 없다.
    const view = transcriptTab(
      recording({ transcriptionStatus: 'done', currentTranscriptId: 't-1' }),
      transcript(),
      status({ state: 'done', recordingId: 'r-1', transcriptId: 't-1' }),
    );

    expect(Object.keys(view).sort()).toEqual(['engine', 'kind', 'language', 'lines', 'model']);
  });

  it('완료 상태가 저장된 값을 그대로 옮긴다', () => {
    // 화면이 문장을 다듬거나 합치지 않는다 — 저장된 것이 그대로 보인다 (INV-2).
    const stored = transcript();
    const view = transcriptTab(
      recording({ transcriptionStatus: 'done', currentTranscriptId: 't-1' }),
      stored,
      status(),
    );

    expect(view.kind === 'done' && view.lines.map((line) => line.text)).toEqual(
      stored.segments.map((segment) => segment.text),
    );
  });
});
