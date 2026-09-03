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
 * `storage` · `invalidInput` · `audioDevice` · `microphonePermission`은 Rust의 `FailureKind`와
 * 1:1이다. `unexpected`는 **frontend 경계에서만 만들어진다** — command가 계약과 다른 값으로
 * 거절했을 때 (예: 없는 command 이름, IPC 자체의 오류) 그 사실을 삼키지 않기 위해서다.
 *
 * `microphonePermission`은 `audioDevice`와 따로 있다. 사용자가 할 수 있는 일이 다르기
 * 때문이다 — 장치를 바꾸거나 다시 시도해서 풀리지 않고, 시스템 설정에서 접근을 허용해야 한다.
 * 무엇을 허용해야 하는지는 `message`에 문장으로 들어 있다 (Rust의 `platform::microphone`).
 */
export type FailureKind =
  | 'storage'
  | 'invalidInput'
  | 'audioDevice'
  | 'microphonePermission'
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
