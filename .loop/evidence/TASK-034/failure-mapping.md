# TASK-034 — AI provider 실패 매핑 대조표

ADR-0008 §13.1의 매핑 표 ↔ Rust `FailureKind` ↔ `src/ipc/failure.ts`의 union.

| ADR-0008 §13.1의 실패 | Rust `FailureKind` | `as_str()` | frontend union | `retryable` | `source_data_safe` |
| --- | --- | --- | --- | --- | --- |
| provider 미설정 | `AiProviderNotConfigured` | `aiProviderNotConfigured` | `'aiProviderNotConfigured'` | **false** | true |
| provider 연결 불가 | `AiProviderUnreachable` | `aiProviderUnreachable` | `'aiProviderUnreachable'` | **true** | true |
| 모델 없음 | `AiModelUnavailable` | `aiModelUnavailable` | `'aiModelUnavailable'` | **false** | true |
| 요청 실패 | `AiRequestFailed` | `aiRequestFailed` | `'aiRequestFailed'` | §13.3의 규칙 | true |
| 응답 schema 불일치 | `AiResponseUnusable` | `aiResponseUnusable` | `'aiResponseUnusable'` | **true** | true |

`retryable` 값은 `src-tauri/src/ai/provider.rs`의 생성 함수가 정하고
`retrying_is_worth_it_only_where_the_situation_can_change` 테스트가 고정한다.

## §13.3 — `aiRequestFailed`의 재시도 규칙

같은 종류 하나에 재시도 값 둘이 있으므로 생성 함수를 둘로 나눴다.

| 상황 | 함수 | `retryable` |
| --- | --- | --- |
| 타임아웃 · 5xx | `provider::request_failed_temporarily` | true |
| 4xx | `provider::request_rejected` | false |
| 연결 자체가 안 됨 | (여기로 오지 않는다) `provider::unreachable` | true |

`both_request_outcomes_are_the_same_kind_of_failure` 테스트가 두 함수의 `kind`가 하나임을 고정한다.

## 이 Task가 더하지 **않은** 것

- **ADR-0008 §13.1의 여섯 번째 행 `aiInputTooLarge`를 더하지 않았다.** Task request가
  더할 다섯을 명시적으로 열거했고(provider 미설정 · 연결 불가 · 모델 없음 · 요청 실패 ·
  응답 schema 불일치) AC5도 다섯을 말한다. 그리고 §13.1 표 자신이 그 행을 *(추가)*로 적으며
  만드는 주체를 adapter가 아니라 **domain(요청 전 계산 · §8.5)**으로 둔다 — 이 Task에는 그
  계산을 실행하는 경로가 없다. `domain/failure.rs`의 기존 규약("만들지 않은 실패의 자리를
  미리 만들어 두지 않는다" · §20.6)에 따라, `prompt::ContextOverflow`를 공통 실패로 옮기는
  Task가 그때 이 종류를 더한다. 이 판단은 `src-tauri/src/ai/mod.rs`의 모듈 주석에도 남겼다.
- §13.4의 `rate limit` · `인증 실패` — ADR이 이 Phase에서 만들지 않기로 한 것이며,
  `provider::AI_PROVIDER_FAILURE_KINDS`의 주석이 그 이유를 적어 둔다.
