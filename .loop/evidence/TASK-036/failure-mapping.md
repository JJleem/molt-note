# TASK-036 — 벤더 실패 → domain 공통 실패 매핑

Run: RUN-20260903T064637Z-TASK-036 · 2026-09-03 · 근거: ADR-0008 §13.1 · §13.3

번역이 일어나는 자리는 `src-tauri/src/ai/ollama/provider.rs`의 세 함수뿐이다
(`transport_failure` · `status_failure` · `unreadable_body`)와, 모델 판정 한 자리다.
**벤더 오류 값이 이 파일 밖으로 나가지 않는다** (INV-9).

| 벤더에서 일어난 일 | 어디서 판정 | `FailureKind` | `retryable` | 확인하는 테스트 |
| --- | --- | --- | --- | --- |
| 연결 자체가 되지 않음 (서버 미실행) | `TransportError::NotConnected` | `AiProviderUnreachable` | **true** — 서버를 켜고 다시 누르면 된다 | `the_four_vendor_failures_…` · `a_refused_connection_is_the_unreachable_failure` |
| 타임아웃 | `TransportError::TimedOut` | `AiRequestFailed` | **true** (§13.3) | `a_timeout_is_separated_from_a_server_that_is_not_running` |
| 요청을 끝내지 못함 (본문 읽기 실패 등) | `TransportError::Incomplete` | `AiRequestFailed` | **true** | `no_failure_on_any_path_…` |
| 비2xx — 4xx | `HttpResponse::is_client_error()` | `AiRequestFailed` | **false** — 같은 요청은 같은 답이다 | `the_four_vendor_failures_…` |
| 비2xx — 그 밖(5xx 등) | `HttpResponse::is_success()` | `AiRequestFailed` | **true** | 같음 |
| 고른 모델이 `GET /api/tags` 목록에 없음 | `provider::generate_note` | `AiModelUnavailable` | **false** — 모델을 받아야 풀린다 | `a_model_that_is_not_installed_is_a_different_failure_from_a_silent_server` |
| 설치된 모델이 하나도 없음 | 같음 (`Availability::NoModels`) | `AiModelUnavailable` | **false** | `a_server_with_no_models_answers_but_cannot_generate` |
| 목록 본문을 해석할 수 없음 | `wire::BodyRejection` | `AiResponseUnusable` | **true** | `a_body_that_is_not_the_expected_shape_is_also_the_schema_failure` |
| 생성 응답이 기대 schema와 다름 | `wire::generated_note` → `ai::note::ResponseRejection` | `AiResponseUnusable` | **true** — 생성은 결정론적이지 않다 | `a_response_that_is_not_the_expected_note_is_the_schema_failure` |

`AiProviderNotConfigured`는 이 adapter가 만들지 않는다 — provider 객체가 없는 상태이며
서비스 계층의 몫이다 (ADR-0008 §4.3).

## 뭉치지 않는다는 것의 확인

`the_four_vendor_failures_become_four_different_domain_failures`는 여섯 상황을 돌린 뒤
나온 `FailureKind`를 dedup해 **네 종류**가 남는지 본다. 하나로 뭉치면 화면이 구분해
안내할 수 없다 (§13).

여섯 상황 전부에서 `source_data_safe == true`임도 함께 확인한다 — 이 경계는 오디오도
`transcripts`도 읽지 않으므로 어떤 실패도 원본을 훼손하지 않는다 (INV-3).

## 모델 존재는 생성 응답으로 판정하지 않는다

ADR-0008 §13.3의 요구다. `generate_note`는 **먼저** `GET /api/tags`로 목록을 확인하고,
고른 모델이 없으면 그 자리에서 `AiModelUnavailable`을 낸다. 모델이 없을 때 서버가 어떤
상태 코드를 돌려주는지는 UNVERIFIED이며(§14.2 항목 15), 이 순서 덕분에 그 값에 의존하지
않는다. 대가는 생성 한 번에 왕복이 하나 느는 것이다 —
`generation_sends_the_prompt_the_domain_built_with_the_context_size_stated`가 요청이 둘임을
확인한다.

## 실패 문장에 설정값이 없다 (ADR-0008 §11.3)

| 자리 | 무엇이 들어가는가 |
| --- | --- |
| `message` | 사용자가 읽을 고정 문장 ("로컬 AI 서버에 연결하지 못했다" 등). 주소도 모델 이름도 없다 |
| `detail` | `TransportError`/`BodyRejection`의 고정 문자열, 또는 `HTTP <상태 코드>` |
| 응답 본문 | **detail에 담지 않는다** — 무엇이 들어 있을지 모른다 |
| 모델 이름 | 목록에 없다는 실패에도 넣지 않는다 — 사용자는 자기가 고른 값을 설정 화면에서 본다 |
