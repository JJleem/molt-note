/**
 * Rust의 domain 공통 실패 타입에 대응하는 frontend 타입 (PRODUCT-SPEC §13 ·
 * `src-tauri/src/domain/failure.rs`).
 *
 * §13이 요구하는 세 가지 답이 그대로 필드로 있다:
 * 무엇이 실패했는가 · 원본 데이터는 안전한가 · 다시 시도할 수 있는가.
 *
 * 화면은 이 타입만 알면 실패를 그릴 수 있다. 어느 계층에서 났는지는 알 필요가 없다.
 */

/**
 * 실패의 종류.
 *
 * `unexpected`를 뺀 나머지는 전부 Rust의 `FailureKind`와 1:1이다
 * (`tests/ipc-boundary.test.ts`가 그것을 강제한다).
 * `unexpected`는 **frontend 경계에서만 만들어진다** — command가 계약과 다른 값으로
 * 거절했을 때 (예: 없는 command 이름, IPC 자체의 오류) 그 사실을 삼키지 않기 위해서다.
 *
 * `microphonePermission`은 `audioDevice`와 따로 있다. 사용자가 할 수 있는 일이 다르기
 * 때문이다 — 장치를 바꾸거나 다시 시도해서 풀리지 않고, 시스템 설정에서 접근을 허용해야 한다.
 * 무엇을 허용해야 하는지는 `message`에 문장으로 들어 있다 (Rust의 `platform::microphone`).
 *
 * `transcription*` 넷도 같은 이유로 서로 따로 있다 (§13 · Rust의 `transcription::engine`).
 * 모델을 구해 와야 하는 것 · 다른 모델을 골라야 하는 것 · 다시 시도해 볼 수 있는 것 ·
 * 다시 시도해도 같은 것은 사용자에게 전부 다른 상황이다.
 *
 * `ai*` 여섯도 마찬가지다 (§13 · Rust의 `ai::provider` · `ai::run` ·
 * `docs/ADR-0008-note-ai-provider.md` §13.1).
 * provider를 고르는 것 · 로컬 AI 서버를 켜는 것 · 모델을 받아 오는 것 · 다시 시도하는 것 ·
 * 다른 요청을 만드는 것은 전부 다른 상황이다. 그중 `aiProviderNotConfigured`는 **오류로 그리는
 * 상태가 아니다** — provider를 고르지 않은 것은 정상 상태이며 (INV-8), 화면은 AI 기능이
 * 비활성이라는 담담한 상태를 보인다.
 *
 * `aiInputTooLarge`는 **요청을 보내지 않았다**는 뜻이다. 전사를 잘라서 보내는 대신 그 사실을
 * 상태로 보이며, 사용자가 할 수 있는 일은 context 크기를 키우거나 더 짧은 녹음을 고르는
 * 것이다 (ADR-0008 §8.2).
 *
 * `notion*` 다섯도 같은 이유로 서로 따로 있다 (§13 · Rust의 `notion::client` ·
 * `docs/ADR-0009-notion-and-export.md` §9.3). token을 다시 넣는 것 · 부모 페이지를
 * integration에 공유하는 것 · 잠시 기다렸다 다시 보내는 것 · 다시 시도하는 것은 전부 다른
 * 상황이다.
 *
 * `notionResponseUnusable`은 **결과를 모른다**는 뜻이다 (ADR-0009 §7.3 · §8.5). Notion이
 * 응답했지만 만들어진 페이지를 확인하지 못했으므로, 그대로 다시 보내면 사용자가 모르는 사이에
 * 페이지가 둘이 될 수 있다. 화면은 "실패했다"도 "성공했다"도 아닌 그 사실을 그대로 말하고,
 * 사용자가 Notion을 확인한 뒤 고르게 한다.
 */
export type FailureKind =
  | 'storage'
  | 'invalidInput'
  | 'audioDevice'
  | 'microphonePermission'
  | 'transcriptionModelMissing'
  | 'transcriptionModelUnusable'
  | 'transcriptionEngineFailed'
  | 'transcriptionOutputUnusable'
  | 'aiProviderNotConfigured'
  | 'aiProviderUnreachable'
  | 'aiModelUnavailable'
  | 'aiRequestFailed'
  | 'aiResponseUnusable'
  | 'aiInputTooLarge'
  | 'notionAuthFailed'
  | 'notionDestinationUnavailable'
  | 'notionRateLimited'
  | 'notionRequestFailed'
  | 'notionResponseUnusable'
  | 'unexpected';

export interface Failure {
  readonly kind: FailureKind;
  /** 무엇이 실패했는가 — 그대로 화면에 보여줄 수 있는 문장. */
  readonly message: string;
  /** 원인의 기술적 표현. 없을 수 있다. */
  readonly detail: string | null;
  /** 원본 데이터는 안전한가. */
  readonly sourceDataSafe: boolean;
  /** 다시 시도할 수 있는가. */
  readonly retryable: boolean;
}

/** Rust가 돌려준 구조화된 실패인지 확인한다. */
export function isFailure(value: unknown): value is Failure {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const candidate = value as Partial<Failure>;
  return (
    typeof candidate.kind === 'string' &&
    typeof candidate.message === 'string' &&
    typeof candidate.sourceDataSafe === 'boolean' &&
    typeof candidate.retryable === 'boolean'
  );
}

/**
 * command 호출이 거절되며 넘어온 값을 {@link Failure}로 만든다.
 *
 * Rust가 보낸 구조화된 실패는 그대로 쓴다. 그 밖의 값(문자열 · Error · 알 수 없는 값)도
 * 화면에 보여줄 수 있는 모양으로 옮긴다 — 실패를 console에만 남기고 끝내지 않기 위해서다 (§13).
 */
export function toFailure(error: unknown): Failure {
  if (isFailure(error)) {
    return error;
  }
  return {
    kind: 'unexpected',
    message: '앱이 예상하지 못한 문제로 요청을 끝내지 못했다.',
    detail: describe(error),
    // 예상하지 못한 실패라도 저장된 데이터를 건드리는 경로는 Rust 안에만 있다.
    // 여기까지 온 값은 응답을 받지 못했다는 뜻이므로 원본이 훼손됐다고 단정하지 않는다.
    sourceDataSafe: true,
    retryable: true,
  };
}

/** 알 수 없는 값에서 사람이 읽을 수 있는 원인 문자열을 뽑는다. */
function describe(error: unknown): string | null {
  if (error === null || error === undefined) {
    return null;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}
