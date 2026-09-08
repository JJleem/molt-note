/**
 * Recording Detail의 **Transcript 탭** 상태 (PRODUCT-SPEC §5 C · §7 · §13 ·
 * `phase-prompt/03` 요구 3 · 6 · 7).
 *
 * 이 탭이 답해야 하는 질문은 두 가지다 — **이 녹음의 전사가 지금 어떤 상태인가**와
 * **무엇을 볼 수 있는가**. 둘 다 backend가 준 사실에서 나오며, 그 사실을 화면 상태로 옮기는
 * 규칙이 전부 여기 있다. React도 DOM도 Tauri도 알지 않으므로 다섯 경로(아직 없음 · 대기 ·
 * 진행 중 · 완료 · 실패)를 **whisper도 모델도 오디오도 없이** vitest로 그대로 판정할 수 있다
 * (§18).
 *
 * ## 이 모듈은 아무것도 지우지 않는다 (INV-1 · INV-2 · INV-3)
 *
 * Transcript는 immutable이며 재전사는 기존 것을 고치지 않고 새 것을 추가한다 (§7.1).
 * 그 규칙을 화면 쪽에서도 지킬 수 있는 이유는 하나다 — **여기에 값을 만들어 내는 함수만
 * 있고, 무엇을 지우거나 고치는 함수가 없다.** 전사가 실패해도 이 모듈은 이미 읽어 둔
 * Transcript를 버리지 않고 `kept`에 그대로 실어 보낸다. 화면에서 사라지는 것과 저장소에서
 * 사라지는 것은 다른 일이지만, 사용자에게는 둘이 같아 보이기 때문이다.
 *
 * ## timestamp 문자열은 여기서 만든다 — 녹음 길이와는 다른 값이다
 *
 * 녹음 하나의 **길이 문자열**은 여전히 Rust가 만든 값을 그대로 쓴다
 * (`src-tauri/src/domain/duration.rs` · `recordingsView.ts`). 이 모듈이 만드는 것은 그것이
 * 아니라 **segment의 시작·종료 위치**이며, 형식도 목적도 다르다.
 *
 * ```text
 * 길이       52:31       이 녹음은 얼마나 긴가        Rust가 만든다
 * 걸린 시간  1:47        전사에 얼마나 걸렸는가       Rust가 만든다
 * timestamp  00:02:14    이 문장은 녹음의 어디인가    여기서 만든다
 * ```
 *
 * 전사에 걸린 시간도 Rust가 만든 문장을 그대로 받는다 (`transcriptionLabel`) — 이 모듈은 그
 * 밀리초를 보지 않는다. 그리고 **잰 적이 없으면 `null`이 그대로 지나간다**: 재지 않은 것을
 * `0:00`이라고 말하면 Metal 전후 비교가 거짓을 말하기 때문이다 (`phase-prompt/05.6` 기준 3).
 *
 * §7의 요구 6이 정한 표시 형태(`00:02:14 → 00:02:21`)는 화면의 문제이고, 그것을 만드는 규칙은
 * **여기 한 곳**에 있다 ({@link formatTimestamp}). 밀리초를 어떤 단위로 볼지는 이미 Rust의 통합
 * 경계에서 한 번 정규화됐으므로 (`src-tauri/src/transcription/parse.rs`), 여기서 하는 일은
 * 나누고 곱하는 것이 아니라 **이미 밀리초인 값을 읽는 형태로 옮기는 것**뿐이다.
 */
import { toFailure, type Failure } from '../ipc/failure';
import type { Recording, Transcript, TranscriptionStatus } from '../ipc/types';

const MILLIS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;
const SECONDS_PER_HOUR = SECONDS_PER_MINUTE * MINUTES_PER_HOUR;

/**
 * 녹음 시작 기준 오프셋(밀리초)을 `HH:MM:SS`로 만든다 (`phase-prompt/03` 요구 6 · 9).
 *
 * ```text
 *         0ms  →  00:00:00
 *       999ms  →  00:00:00      초 미만은 버린다 (반올림하지 않는다)
 *   134_000ms  →  00:02:14
 * 3_600_000ms  →  01:00:00      정확히 1시간
 * 3_601_000ms  →  01:00:01
 * ```
 *
 * **버림이지 반올림이 아니다.** `00:00:01`이라고 적힌 문장을 그 자리에서 찾을 수 없으면
 * timestamp는 쓸모가 없다 — 반올림하면 표시가 실제 음성보다 뒤로 갈 수 있다. 같은 이유로
 * Rust의 길이 표시도 버림이다 (`format_duration_ms`).
 *
 * 시간 자리는 언제나 두 자리 이상이다. 1시간을 넘는 녹음에서 자릿수가 늘어나도 잘리지 않는다.
 * 음수나 숫자가 아닌 값은 존재할 수 없는 오프셋이므로 `0`으로 본다 — **추측해서 다른 값으로
 * 바꾸지 않는다.**
 */
export function formatTimestamp(offsetMs: number): string {
  const safeMs = Number.isFinite(offsetMs) ? Math.max(0, offsetMs) : 0;
  const totalSeconds = Math.floor(safeMs / MILLIS_PER_SECOND);

  const hours = Math.floor(totalSeconds / SECONDS_PER_HOUR);
  const minutes = Math.floor(totalSeconds / SECONDS_PER_MINUTE) % MINUTES_PER_HOUR;
  const seconds = totalSeconds % SECONDS_PER_MINUTE;

  return `${twoDigits(hours)}:${twoDigits(minutes)}:${twoDigits(seconds)}`;
}

/** 두 자리로 적는다. 세 자리가 되는 값(100시간 이상)은 자르지 않는다. */
function twoDigits(value: number): string {
  return String(value).padStart(2, '0');
}

/** 시작과 끝 사이에 놓는 기호. §7의 요구 6이 보여준 형태 그대로다. */
const RANGE_SEPARATOR = '→';

/** segment 하나가 화면에 놓이는 모습 (§7의 `segments[] { start · end · text }`). */
export interface TranscriptLine {
  /** `00:02:14`. */
  readonly startLabel: string;
  /** `00:02:21`. */
  readonly endLabel: string;
  /** `00:02:14 → 00:02:21`. 시작과 끝이 **함께** 보인다 (요구 6). */
  readonly rangeLabel: string;
  readonly text: string;
}

/**
 * Transcript 하나를 화면에 놓을 줄로 옮긴다.
 *
 * 순서를 바꾸지 않는다 — 저장된 순서가 곧 말한 순서다. 문장을 자르거나 합치지도 않는다.
 */
export function transcriptLines(transcript: Transcript): readonly TranscriptLine[] {
  return transcript.segments.map((segment) => ({
    startLabel: formatTimestamp(segment.startMs),
    endLabel: formatTimestamp(segment.endMs),
    rangeLabel: `${formatTimestamp(segment.startMs)} ${RANGE_SEPARATOR} ${formatTimestamp(segment.endMs)}`,
    text: segment.text,
  }));
}

/**
 * 전사 도중에 나온 문장들을 화면 줄로 옮긴다 (2026-09-07 추가).
 *
 * 끝 시각은 **없다** — 엔진이 아직 그 자리를 말하지 않았기 때문이며, 모르는 값을 시작 시각과
 * 같게 만들어 `00:02:14 → 00:02:14`처럼 그럴듯하게 보이지 않게 한다. 그래서 `rangeLabel`은
 * 시작 시각 하나이고 `endLabel`은 빈 문자열이다.
 */
export function partialLines(status: TranscriptionStatus): readonly TranscriptLine[] {
  return status.partialLines.map((line) => {
    const startLabel = formatTimestamp(line.startMs);
    return { startLabel, endLabel: '', rangeLabel: startLabel, text: line.text };
  });
}

/**
 * `0.37` → `37%`. 모르면 `null`이다.
 *
 * 내림한다 — 아직 끝나지 않았는데 `100%`가 보이지 않게 한다.
 */
export function progressLabel(progress: number | null): string | null {
  if (progress === null || Number.isNaN(progress)) {
    return null;
  }
  const clamped = Math.min(Math.max(progress, 0), 1);
  return `${Math.floor(clamped * 100)}%`;
}

/**
 * 이 탭에서 사용자가 할 수 있는 동작 하나 (`phase-prompt/03` 요구 2 · 7).
 *
 * **함수가 아니라 값이다.** 순수 모듈이 command를 알지 않기 때문이며, 그래서 "지금 시작할 수
 * 있는가"·"재시도 수단이 있는가"가 DOM 없이 판정된다. 실제로 부르는 것은 화면 컴포넌트다
 * (`RecordingDetailScreen`의 `startTranscription`).
 */
export interface TranscriptAction {
  /**
   * 처음 시작하는가 · 실패한 뒤 다시 하는가 · **이미 있는 전사를 두고 다시 하는가.**
   * 사용자에게는 셋 다 다른 상황이다.
   *
   * `redo`가 나머지 둘과 다른 점은 **잃을 것이 있다고 오해할 수 있다**는 것이다.
   * 실제로는 잃지 않는다 — Transcript는 immutable이고 다시 돌리면 **새 Transcript가
   * 추가된다** (INV-2 · `run`의 `a_second_successful_run_adds_a_transcript_instead_of_updating_the_first`).
   * 그 사실을 문구가 말해야 한다.
   */
  readonly kind: 'start' | 'retry' | 'redo';
  /** 버튼에 적히는 말. */
  readonly label: string;
  readonly recordingId: string;
}

/**
 * 전사가 실패한 이유 중 **사용자가 할 일이 달라지는 갈래** (§13).
 *
 * 모델이 없는 것을 일반 실패와 같이 보여주면 사용자는 없는 문제를 고치려 하게 된다 —
 * 다시 눌러도 모델은 생기지 않는다. 그래서 이 갈래가 값으로 남는다.
 *
 * ```text
 * modelMissing     전사에 쓸 모델 파일이 없다        모델을 두고 고른 뒤에 다시 시작한다
 * modelUnusable    고른 모델을 쓸 수 없다            다른 모델을 고른다
 * outputUnusable   전사가 붕괴해 결과를 쓸 수 없다   입력 레벨을 확인하거나 모델을 바꾼다
 * other            그 밖의 실패                      다시 시도할 수 있다
 * unknown          이 앱이 켜진 뒤의 시도가 아니다   이유를 지어내지 않는다
 * ```
 *
 * `outputUnusable`이 `other`와 갈라져 있는 이유는 §13의 갈래가 이미 그렇게 나뉘어 도착하기
 * 때문이다 (ADR-0007 §18.4). 엔진은 정상적으로 끝났고 결과도 있지만 그 결과를 전사로 쓸 수
 * 없는 상황이며 (`transcriptionOutputUnusable`), **그대로 다시 눌러도 같은 결과가 나온다** —
 * 같은 오디오를 같은 조건으로 다시 돌리는 것이기 때문이다. 사용자가 바꿔야 하는 것은
 * 입력(녹음 레벨)이거나 조건(모델)이므로, 그 사실을 `other`로 뭉개면 화면이 사용자를 같은
 * 실패로 되돌려 보낸다. **Rust가 나눠 보낸 구분을 화면이 다시 뭉개지 않는다.**
 */
export type TranscriptFailureCause =
  | 'modelMissing'
  | 'modelUnusable'
  | 'outputUnusable'
  | 'other'
  | 'unknown';

/**
 * Transcript 탭이 놓일 수 있는 상태의 전부.
 *
 * ```text
 * loading   가리키는 Transcript를 아직 읽지 못했다
 * none      아직 전사한 적이 없다 (실패가 아니다 · §7 · INV-8)
 * pending   전사가 접수됐고 아직 시작되지 않았다
 * running   지금 전사하고 있다
 * done      전사된 문장을 timestamp와 함께 볼 수 있다
 * failed    전사가 실패했다 — 원본도 기존 Transcript도 그대로다
 * ```
 *
 * `pending`과 `running`을 접지 않는 이유는 §7의 후처리 상태가 둘을 따로 정의하기 때문이고,
 * `none`을 실패와 섞지 않는 이유는 **아직 하지 않은 것과 해 보고 실패한 것이 다른 사실**이기
 * 때문이다 (INV-8).
 */
export type TranscriptTabView =
  | { readonly kind: 'loading' }
  | {
      readonly kind: 'none';
      readonly text: string;
      /** 이 화면에서 수동으로 전사를 시작할 수 있다 (요구 2). */
      readonly start: TranscriptAction;
    }
  | {
      readonly kind: 'pending';
      readonly text: string;
      /** 이미 있던 Transcript. 새 전사가 이것을 지우지 않는다 (§7.1 · INV-2). */
      readonly kept: readonly TranscriptLine[];
    }
  | {
      readonly kind: 'running';
      readonly text: string;
      readonly kept: readonly TranscriptLine[];
      /**
       * **지금까지 나온 문장들.** 아직 아무것도 안 나왔으면 빈 배열이다.
       *
       * 저장된 Transcript가 아니라 미리보기다 — 붕괴 판정도 반복 차단도 아직 지나지 않았고,
       * 그래서 **여기 보이던 문장이 최종 결과에 없을 수 있다.** 화면은 그 사실을 감추지 않는다.
       */
      readonly partial: readonly TranscriptLine[];
      /** `37%`. 아직 모르면 `null`이다 — 모르는 것을 `0%`라고 말하지 않는다. */
      readonly progressLabel: string | null;
    }
  | {
      readonly kind: 'done';
      readonly lines: readonly TranscriptLine[];
      /** 엔진이 말한 언어. 모르면 `null`이다 — 지어내지 않는다. */
      readonly language: string | null;
      /** 이 문장들이 무엇으로 만들어졌는가 (provenance · §7). */
      readonly engine: string;
      readonly model: string;
      /**
       * 이 전사에 **얼마나 걸렸는가** — Rust가 만든 문장 그대로다 (예: `1:47`).
       *
       * `null`이면 **그때는 재지 않았다**는 뜻이다 (`phase-prompt/05.6` 성공 기준 3).
       * 그 상태를 `0:00`으로 말하지 않는다 — 화면은 그 줄을 아예 보이지 않는다.
       * 모르는 것을 지어내지 않는 규칙은 {@link language}와 같다.
       */
      readonly transcriptionLabel: string | null;
      /**
       * 같은 녹음을 **다시 전사한다.**
       *
       * 2026-09-08까지 이 자리에 수단이 없었다. 모델을 바꾸거나 조건이 달라져 다시 돌리고
       * 싶을 때 화면에서 할 수 있는 일이 없었고, 그래서 저장소에 레코드를 직접 넣어
       * 우회했다 — **제품이 만든 적 없는 레코드가 DB에 생겼다.**
       */
      readonly redo: TranscriptAction;
      /** 다시 돌려도 지금 것을 잃지 않는다는 사실 (INV-2). 버튼 옆에 그대로 놓인다. */
      readonly redoNotice: string;
    }
  | {
      readonly kind: 'failed';
      /** 무엇을 하다 실패했는가. */
      readonly headline: string;
      /**
       * 실패 그대로 (§13의 세 질문에 대한 답이 이미 이 안에 있다).
       *
       * `null`이면 **이 앱이 켜진 뒤의 시도가 아니라서 이유를 모른다**는 뜻이다.
       * 저장된 것은 `failed`라는 사실뿐이므로 이유를 지어내지 않는다.
       */
      readonly failure: Failure | null;
      readonly cause: TranscriptFailureCause;
      /** 원본과 기존 Transcript가 그대로라는 사실 (INV-1 · INV-2 · INV-3). */
      readonly preservedNotice: string;
      /** 이 갈래에서 사용자가 먼저 해야 하는 일. 없으면 `null`이다. */
      readonly resolution: string | null;
      /** 실패해도 다시 시도할 수 있다 (요구 7). */
      readonly retry: TranscriptAction;
      /** 실패가 지우지 않은 것 — 이미 있던 Transcript는 그대로 보인다. */
      readonly kept: readonly TranscriptLine[];
    };

/** 화면을 열었을 때. 아직 아무것도 읽지 못했다. */
/**
 * 다시 돌려도 **지금 보고 있는 전사를 잃지 않는다**는 사실.
 *
 * Transcript는 immutable이므로 다시 돌리면 새 것이 추가되고 그것이 current가 된다.
 * 이 문장이 없으면 사용자는 버튼을 누르기를 망설이고, 그래서 2026-09-08처럼 저장소를
 * 직접 건드리는 우회가 다시 생긴다.
 */
export const TRANSCRIPTION_REDO_NOTICE =
  '다시 돌리면 새 전사가 추가되고 그것이 보인다. 지금 이 전사는 그대로 남는다.';

export const LOADING_TRANSCRIPT_TAB: TranscriptTabView = { kind: 'loading' };

/** 아직 전사한 적이 없다. **오류가 아니라 정상 상태다** (§7 · INV-8). */
export const NO_TRANSCRIPT_TEXT = '아직 전사가 없다.';

/** 접수됐지만 아직 시작되지 않았다. */
export const PENDING_TRANSCRIPT_TEXT = '전사가 대기 중이다.';

/**
 * 지금 전사하고 있다.
 *
 * 전사는 backend의 배경 스레드에서 돌고 화면은 그것을 물어볼 뿐이다 — 그래서 이 화면을
 * 떠나도 전사는 계속되고, 도는 동안에도 화면이 멎지 않는다 (요구 3).
 */
export const RUNNING_TRANSCRIPT_TEXT =
  '전사 중… 화면을 떠나도 배경에서 계속 돈다.';

/** 무엇을 하다 실패했는가 (§13). 원인은 {@link Failure}가 말한다. */
export const TRANSCRIPTION_FAILED_HEADLINE = '이 녹음을 전사하지 못했다.';

/**
 * 실패가 무엇을 남겼는지 (§13 · INV-1 · INV-2 · INV-3).
 *
 * **"복구했다"거나 "정리했다"고 말하지 않는다.** 앱은 아무것도 지우지 않았다 — 실패 경로는
 * 원본 오디오도, Recording 레코드도, 이미 있던 Transcript도 건드리지 않는다
 * (`src-tauri/src/transcription/run.rs`).
 */
export const TRANSCRIPTION_PRESERVED_NOTICE =
  '녹음도 오디오 파일도 그대로이고, 이미 있던 전사도 그대로 남아 있다. 지워진 것은 없다.';

/** 이유를 모를 때 그 사실을 그대로 말한다. **무엇이 실패했는지 지어내지 않는다.** */
export const UNKNOWN_FAILURE_NOTICE =
  '저장된 상태가 지난 전사의 실패를 말한다. 이 세션에서는 그 이유를 알 수 없다 — 다시 시작해 보면 무엇이 일어나는지 알 수 있다.';

/**
 * 전사가 붕괴했을 때 사용자가 할 수 있는 일 (§13 · ADR-0007 §18.4 · `phase-prompt/05.7` 기준 2).
 *
 * **무엇이 일어났는가**(전사가 붕괴해 결과를 쓸 수 없다)와 **무엇을 하면 되는가**(입력 레벨을
 * 확인하고 다시 녹음한다 · 다른 모델을 고른다 · 다시 시도한다)가 함께 있다. 원인을 단정하지
 * 않는 이유는 Rust가 이미 그렇게 하고 있기 때문이다 (ADR-0007 §18.6) — 같은 판정이 낮은
 * 입력 레벨에서도 부족한 모델에서도 나오며, 어느 쪽인지는 화면이 더 알지 못한다. 그래서
 * 하나를 지목하는 대신 **사람이 고를 수 있는 것을 늘어놓는다.**
 *
 * 얼마나 붕괴했는지(문장 수 · 반복 횟수)는 이 문장에 없다. 그 수치는 Rust가 만든
 * `Failure.message`에 이미 들어 있고 화면은 그것을 그대로 보인다 — 여기서 다시 세지 않는다.
 */
export const TRANSCRIPTION_COLLAPSED_NOTICE =
  '전사가 무너져서 그 결과를 전사로 쓸 수 없었다. 그대로 다시 돌리면 같은 결과가 나온다 — 녹음의 입력 레벨을 확인해 다시 녹음하거나, 설정에서 다른 모델을 고른 뒤 전사를 다시 시작한다.';

/**
 * 갈래마다 사용자가 **먼저** 해야 하는 일 (§13).
 *
 * 모델이 없는 실패에 "다시 시도"만 보여주면 사용자는 같은 실패를 반복한다 — 그래서 그 갈래는
 * 다시 시도 수단과 **함께** 이 문장을 보여준다. 붕괴한 전사도 같은 이유로 문장을 갖는다.
 */
const RESOLUTION: Record<TranscriptFailureCause, string | null> = {
  modelMissing:
    '전사에 쓸 모델을 찾지 못했다. 모델 파일을 자리에 두고 설정에서 고른 뒤 전사를 다시 시작한다.',
  modelUnusable:
    '고른 모델을 쓸 수 없었다. 설정에서 다른 모델을 고른 뒤 전사를 다시 시작한다.',
  outputUnusable: TRANSCRIPTION_COLLAPSED_NOTICE,
  other: null,
  unknown: null,
};

/**
 * 읽어 온 값을 Transcript 탭의 상태로 바꾼다.
 *
 * 세 가지를 함께 본다 — **이 녹음의 저장된 후처리 상태**(§7) · **지금 이 앱이 돌리고 있는
 * 전사 한 건**(`transcription_status`) · **읽어 온 current Transcript**(§7.2)다.
 * 셋이 필요한 이유는 각자 답할 수 없는 것이 있기 때문이다.
 *
 * ```text
 * 저장된 상태     앱을 다시 켜도 남는다              실패한 이유는 모른다
 * 지금의 전사     실패한 이유를 그대로 들고 있다     이 앱이 켜진 뒤의 것만 안다
 * Transcript      볼 수 있는 문장 그 자체            지금 무슨 일이 일어나는지는 모른다
 * ```
 *
 * 그래서 순서가 규칙이다: **지금 이 녹음에 대해 벌어지고 있는 일이 먼저이고**, 그것이 없으면
 * 저장된 상태를 읽고, 마지막으로 볼 수 있는 Transcript를 본다. 다른 녹음의 전사 상태는
 * 이 화면과 아무 상관이 없으므로 보지 않는다.
 *
 * `live`가 `null`인 것은 아직 물어보지 못했다는 뜻이며, 그것을 `idle`로 접지 않는다.
 */
export function transcriptTab(
  recording: Recording,
  transcript: Transcript | null,
  live: TranscriptionStatus | null,
): TranscriptTabView {
  const kept = transcript === null ? [] : transcriptLines(transcript);
  const mine = live !== null && live.recordingId === recording.id ? live : null;

  // 1. 지금 이 녹음에 대해 벌어지고 있는 일.
  if (mine?.state === 'running') {
    return {
      kind: 'running',
      text: RUNNING_TRANSCRIPT_TEXT,
      kept,
      partial: partialLines(mine),
      progressLabel: progressLabel(mine.progress),
    };
  }
  if (mine?.state === 'failed') {
    return failedTranscript(recording.id, mine.failure, kept);
  }

  // 2. 저장된 후처리 상태 (§7). 앱을 다시 켠 뒤에도 남아 있는 사실이다.
  if (recording.transcriptionStatus === 'pending') {
    return { kind: 'pending', text: PENDING_TRANSCRIPT_TEXT, kept };
  }
  if (recording.transcriptionStatus === 'running') {
    // 저장된 상태만 아는 자리다 — 이 앱이 돌리는 전사가 아니므로 보여 줄 미리보기가 없다.
    return { kind: 'running', text: RUNNING_TRANSCRIPT_TEXT, kept, partial: [], progressLabel: null };
  }
  if (recording.transcriptionStatus === 'failed') {
    // 마지막 시도가 실패한 채로 남아 있다. 이 앱이 그 시도를 하지 않았으므로 이유는 모르며,
    // **이미 있던 Transcript는 그대로 `kept`에 남는다** — 실패가 그것을 잃게 하지 않는다.
    return failedTranscript(recording.id, null, kept);
  }

  // 3. 볼 수 있는 Transcript.
  if (transcript !== null) {
    return {
      kind: 'done',
      lines: kept,
      language: transcript.language,
      engine: transcript.engine,
      model: transcript.model,
      // 받은 문장을 그대로 나른다. 없으면 없는 채로 나른다 — 여기서 만들지 않는다.
      transcriptionLabel: transcript.transcriptionLabel,
      redo: redoTranscript(recording.id),
      redoNotice: TRANSCRIPTION_REDO_NOTICE,
    };
  }
  if (recording.currentTranscriptId !== null) {
    // 레코드는 Transcript를 가리키는데 아직 그 값을 읽지 못했다.
    return LOADING_TRANSCRIPT_TAB;
  }

  return { kind: 'none', text: NO_TRANSCRIPT_TEXT, start: startTranscript(recording.id) };
}

/**
 * 시작 요청 자체가 거절됐다 (§13).
 *
 * **전사 실패와 다른 사실이다.** 이미 다른 녹음을 전사하고 있을 때가 여기로 오며, 그때
 * 이 녹음의 전사 상태는 아무것도 달라지지 않았다 — 그래서 탭의 상태를 덮지 않고 그 옆에
 * 얹힌다. 접수되지 않은 요청이 조용히 사라지지 않게 하는 자리다.
 */
export interface TranscriptTrouble {
  readonly headline: string;
  readonly failure: Failure;
}

/** 화면이 backend에 보내는 전사 관련 요청. 실패가 어느 요청의 것인지 구분하는 데 쓴다. */
export type TranscriptRequest = 'start' | 'status';

/** 무엇을 하다 실패했는가. 원인은 {@link Failure}가 말한다. */
export const TRANSCRIPTION_START_REJECTED_HEADLINE = '전사를 시작하지 못했다.';

/** 상태를 물어보지 못한 것은 전사가 실패한 것과 다른 사실이다. */
export const TRANSCRIPTION_STATUS_HEADLINE = '전사 상태를 읽지 못했다.';

const TROUBLE_HEADLINE: Record<TranscriptRequest, string> = {
  start: TRANSCRIPTION_START_REJECTED_HEADLINE,
  status: TRANSCRIPTION_STATUS_HEADLINE,
};

/** 거절된 요청 하나를 화면에 놓을 값으로 옮긴다 (§13). */
export function transcriptTrouble(request: TranscriptRequest, error: unknown): TranscriptTrouble {
  return { headline: TROUBLE_HEADLINE[request], failure: toFailure(error) };
}

/**
 * 실패 상태 하나를 만든다.
 *
 * §13이 요구하는 세 가지가 전부 값으로 있다 — **무엇이 실패했는가**(`failure`·`headline`) ·
 * **원본은 안전한가**(`preservedNotice`) · **다시 시도할 수 있는가**(`retry`).
 *
 * **재시도 수단은 언제나 있다.** `Failure.retryable`이 거짓인 갈래(모델 없음 등)에서도
 * 마찬가지다 — 그것은 "지금 그대로 다시 눌러도 같다"는 뜻이지 "이 녹음은 영영 전사할 수
 * 없다"는 뜻이 아니기 때문이다. 무엇을 먼저 해야 하는지는 `resolution`이 말한다.
 */
function failedTranscript(
  recordingId: string,
  failure: Failure | null,
  kept: readonly TranscriptLine[],
): TranscriptTabView {
  const cause = failureCause(failure);
  return {
    kind: 'failed',
    headline: TRANSCRIPTION_FAILED_HEADLINE,
    failure,
    cause,
    preservedNotice: TRANSCRIPTION_PRESERVED_NOTICE,
    resolution: cause === 'unknown' ? UNKNOWN_FAILURE_NOTICE : RESOLUTION[cause],
    retry: { kind: 'retry', label: '전사 다시 시도', recordingId },
    kept,
  };
}

/** 실패 종류를 사용자가 할 일 기준으로 나눈다. Rust가 나눠 보낸 구분을 뭉개지 않는다. */
function failureCause(failure: Failure | null): TranscriptFailureCause {
  if (failure === null) {
    return 'unknown';
  }
  switch (failure.kind) {
    case 'transcriptionModelMissing':
      return 'modelMissing';
    case 'transcriptionModelUnusable':
      return 'modelUnusable';
    // 엔진은 끝났는데 그 결과를 전사로 쓸 수 없다 — 붕괴가 여기로 온다 (ADR-0007 §18.4).
    case 'transcriptionOutputUnusable':
      return 'outputUnusable';
    default:
      return 'other';
  }
}

/** 수동으로 전사를 시작하는 동작. 자동 전사 설정과 무관하게 언제나 할 수 있다 (요구 2). */
function startTranscript(recordingId: string): TranscriptAction {
  return { kind: 'start', label: '전사 시작', recordingId };
}

/**
 * 이미 전사된 녹음을 **다시** 전사하는 동작.
 *
 * 라벨이 `Transcribe again`인 이유는 이것이 **덮어쓰기가 아니기 때문이다.** `Re-transcribe`나
 * `Redo`는 지금 것을 버린다는 인상을 주는데, 실제로는 새 Transcript가 추가되고 그것이
 * current가 된다. 앞의 것은 그대로 남는다 (INV-2).
 */
function redoTranscript(recordingId: string): TranscriptAction {
  return { kind: 'redo', label: '다시 전사', recordingId };
}
