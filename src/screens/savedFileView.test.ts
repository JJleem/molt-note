// 만들어진 파일이 **놓인 자리를 여는** 자리 (`phase-prompt/05.6` 성공 기준 2 · R-4 ·
// PRODUCT-SPEC §13 · INV-3 · INV-10).
//
// **OS도 DOM도 Tauri도 없이 전부 판정한다.** 이 파일은 창을 하나도 열지 않는다 (§18).
//
// 여기서 못박는 것 넷:
//
//   1. 여는 수단은 언제나 있고, 여는 대상은 backend가 준 경로 그대로다
//   2. 여는 중이라는 것도 문장으로 있다                      (요구 12의 규약)
//   3. 열지 못했을 때 **무엇이 그대로인지**가 값으로 있다     (§13 · INV-3)
//   4. 다른 파일에 대한 시도가 이 자리에 보이지 않는다
import { describe, expect, it } from 'vitest';
import type { Failure, FailureKind } from '../ipc/failure';
import {
  NO_SHOW_FILE_ATTEMPT,
  SHOW_FILE_FAILED_HEADLINE,
  SHOW_FILE_LABEL,
  SHOW_FILE_PRESERVED_NOTICE,
  SHOW_FILE_RESOLUTION,
  SHOW_FILE_RETRY_LABEL,
  SHOW_FILE_TEXT,
  SHOWING_FILE_TEXT,
  failedShowFile,
  showFile,
  showingFile,
} from './savedFileView';

/** 이 파일이 쓰는 경로. `~/Library` 아래라는 것이 이 자리가 생긴 이유 그 자체다 (R-4). */
const PATH =
  '/Users/someone/Library/Application Support/molt-note/exports/2026-09-01-3dgs-study-04.md';

function failure(kind: FailureKind, overrides: Partial<Failure> = {}): Failure {
  return {
    kind,
    message: '파일이 있는 자리를 열지 못했다.',
    detail: null,
    sourceDataSafe: true,
    retryable: true,
    ...overrides,
  };
}

describe('여는 수단은 경로의 대체가 아니라 추가다', () => {
  it('아직 아무것도 하지 않았으면 누를 수 있는 동작 하나가 있다', () => {
    const view = showFile(PATH, NO_SHOW_FILE_ATTEMPT);

    expect(view.action.kind).toBe('show');
    expect(view.action.label).toBe(SHOW_FILE_LABEL);
    // 여는 대상은 backend가 준 값 그대로다 — 화면이 경로를 지어내거나 잘라내지 않는다.
    expect(view.action.path).toBe(PATH);
    expect(view.showing).toBe(false);
    expect(view.trouble).toBeNull();
    expect(view.text).toBe(SHOW_FILE_TEXT);
  });

  it('이름도 문장도 어느 OS의 파일 관리자인지 말하지 않는다 (INV-10)', () => {
    // 그 지식은 backend의 platform 경계 하나에 있다. 화면 문장이 "Finder"라고 말하는 순간
    // 그 문장은 다른 플랫폼에서 틀린 말이 된다.
    const view = showFile(PATH, failedShowFile(PATH, failure('storage')));
    const words = [
      view.action.label,
      view.text,
      view.trouble?.headline ?? '',
      view.trouble?.preservedNotice ?? '',
      view.trouble?.resolution ?? '',
    ].join(' ');

    expect(words).not.toMatch(/finder|explorer|macos|windows|nautilus/i);
  });

  it('여는 중이라는 사실이 색이 아니라 문장으로 있다', () => {
    const view = showFile(PATH, showingFile(PATH));

    expect(view.showing).toBe(true);
    expect(view.text).toBe(SHOWING_FILE_TEXT);
    expect(view.text.trim()).not.toBe('');
    // 여는 중에도 이 자리가 사라지지 않는다 — 무엇을 누른 것인지 그대로 보인다.
    expect(view.action.path).toBe(PATH);
  });
});

describe('열지 못했을 때 무엇이 그대로인가 (§13 · INV-3)', () => {
  it('세 질문에 답하고, 남아 있는 길을 함께 말한다', () => {
    const view = showFile(PATH, failedShowFile(PATH, failure('storage', { retryable: false })));

    if (view.trouble === null) {
      throw new Error('실패가 값으로 있어야 한다');
    }
    // 1. 무엇이 실패했는가 — 색이 아니라 이 문장이 말한다.
    expect(view.trouble.headline).toBe(SHOW_FILE_FAILED_HEADLINE);
    expect(view.trouble.failure.kind).toBe('storage');
    // 2. 원본은 안전한가 — 여는 일은 아무것도 바꾸지 않는다.
    expect(view.trouble.preservedNotice).toBe(SHOW_FILE_PRESERVED_NOTICE);
    expect(view.trouble.preservedNotice).toMatch(/그대로/);
    // 3. 다시 시도할 수 있는가 — 그리고 그것이 처음과 다른 상황이라는 것을 이름이 말한다.
    expect(view.action.kind).toBe('retry');
    expect(view.action.label).toBe(SHOW_FILE_RETRY_LABEL);
    // 그리고 **여는 수단이 없어도 남는 길**: 위에 그대로 있는 전체 경로다.
    expect(view.trouble.resolution).toBe(SHOW_FILE_RESOLUTION);
    expect(view.trouble.resolution).toMatch(/경로/);
  });

  it('다시 시도할 수 없는 실패에서도 경로로 가는 길은 남는다', () => {
    // 이 시스템에 열 수단이 아예 없는 경우다 (`platform::file_manager`의 Unsupported).
    // 그때에도 사용자가 할 수 있는 일이 있고, 그 사실이 값에 있다.
    const view = showFile(PATH, failedShowFile(PATH, failure('storage', { retryable: false })));

    expect(view.trouble?.failure.retryable).toBe(false);
    expect(view.trouble?.resolution).toBe(SHOW_FILE_RESOLUTION);
  });
});

describe('이 자리는 이 파일에 대한 시도만 본다', () => {
  it('다른 파일의 실패도 다른 파일을 여는 중인 것도 보이지 않는다', () => {
    const other = '/Users/someone/Library/Application Support/molt-note/exports/other.md';

    const failedElsewhere = showFile(PATH, failedShowFile(other, failure('storage')));
    expect(failedElsewhere.trouble).toBeNull();
    expect(failedElsewhere.action.kind).toBe('show');

    const showingElsewhere = showFile(PATH, showingFile(other));
    expect(showingElsewhere.showing).toBe(false);
    expect(showingElsewhere.text).toBe(SHOW_FILE_TEXT);
  });

  it('만들어진 값 어디에도 다른 파일의 경로가 실리지 않는다', () => {
    const other = '/Users/someone/Downloads/not-ours.md';
    const view = showFile(PATH, failedShowFile(other, failure('storage')));

    expect(JSON.stringify(view)).not.toContain(other);
    expect(view.action.path).toBe(PATH);
  });
});
