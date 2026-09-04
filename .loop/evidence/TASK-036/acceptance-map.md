# TASK-036 — Acceptance Criteria가 무엇으로 판정되는가

Run: RUN-20260903T064637Z-TASK-036 · 2026-09-03

| AC | 판정 수단 | 결과 |
| --- | --- | --- |
| AC1 build | `npm run build` (Gate) | PASS — `gate-results.md` |
| AC2 lint | `npm run lint` (Gate, `-D warnings`) | PASS — 같음 |
| AC3 test | `npm run test` (Gate) | PASS — 같음 |
| AC4 계약 묶음 | `tests/ollama_adapter.rs::the_adapter_and_the_double_pass_the_very_same_contract_suite` | 아래 §1 |
| AC5 서버·네트워크 없음 | `tests/ollama_adapter.rs::only_one_file_in_the_repository_can_open_a_socket` · `::the_test_double_is_pure_values_and_the_tests_never_start_a_server` | 아래 §2 |
| AC6 벤더 지식 격리 | `tests/ollama_adapter.rs::no_source_outside_the_adapter_knows_this_vendors_endpoints_or_parameters` · `::the_default_address_lives_in_the_settings_and_not_in_the_adapter` | 아래 §3 |
| AC7 실패 매핑 · 설정값 비노출 | `tests/ollama_adapter.rs::the_four_vendor_failures_become_four_different_domain_failures` · `::no_failure_on_any_path_carries_the_host_the_port_or_the_model_name` | 아래 §4 |

---

## 1. AC4 — **같은** 계약 묶음이 두 구현에 적용된다

`ai::testing::assert_note_ai_provider_contract`는 TASK-033이 만든 재사용 가능한 묶음이며,
이 Task는 **그 함수를 그대로 부른다.** adapter 전용으로 다시 쓴 검사가 아니다.

네 상황에서 double과 adapter가 나란히 통과한다.

```text
정상 응답            FakeNoteAiProvider::ready()              ↔ StubServer::ready()
연결 불가            FakeNoteAiProvider::unreachable()        ↔ StubServer::refusing()
모델 없음            FakeNoteAiProvider::without_models()     ↔ StubServer::ready().with_models(vec![])
schema에 어긋난 응답  FakeNoteAiProvider::generating_text(..)  ↔ StubServer .with_generate(GeneratedText(..))
```

같은 테스트가 한 걸음 더 간다 — 같은 상황에서 **두 구현이 같은 종류의 답**(같은 mode의 노트
또는 같은 `FailureKind`)을 내는지 대조한다.

## 2. AC5 — 자동 테스트가 실제 프로세스·네트워크에 닿지 않는다

- HTTP 왕복은 `ai::ollama::http::HttpTransport` 하나 뒤에 있고, 테스트는 언제나
  `ai::ollama::testing::StubServer`를 세운다.
- `only_one_file_in_the_repository_can_open_a_socket`이 `src/**`를 훑어 네트워크에 닿는 파일이
  `ai/ollama/network.rs` **하나**임을 확인한다. 그 파일은 테스트에서 한 번도 호출되지 않고
  Gate가 컴파일만 한다 (`transcription::whisper`와 같은 자리다).
- 테스트가 쓰는 주소는 `.invalid` TLD뿐이다. 서버 기동도, `localhost` 접속도 없다.
- 서버가 없는 상황을 **건너뛰지 않는다** — 연결 불가는 §13의 정의된 실패이고 그 자체가
  검증 대상이다.

## 3. AC6 — 벤더 지식이 adapter 디렉터리 밖에 없다

`no_source_outside_the_adapter_knows_this_vendors_endpoints_or_parameters`가 `src/**`의
adapter 밖 소스 전부에 대해 확인한다.

- 금지: `/api/tags` · `/api/generate` · `api/chat` · `num_ctx` · `OLLAMA_HOST`
- 벤더 이름(`OLLAMA`, 대소문자 무시)은 **`src/ai/mod.rs`의 마운트 한 줄에서만** 허용한다.
  Rust에서 `pub mod ollama;`는 피할 수 없고, 그것은 타입도 URL도 파라미터 이름도 에러 코드도
  아니다. 그 파일에 그 이상이 들어오면 위 금지 목록이 잡는다.
- 연결 대상은 adapter 안에 없다 — `the_default_address_lives_in_the_settings_and_not_in_the_adapter`가
  adapter 소스에 `localhost`/`127.0.0.1`이 없음을 확인하고, 기본 주소를 아는 자리가
  `domain::settings::DEFAULT_AI_BASE_URL` 하나임을 확인한다 (TASK-035가 둔 자리다).

## 4. AC7 — 네 벤더 실패 → 네 domain 실패, 그리고 설정값 비노출

ADR-0008 §13.1 · §13.3의 표 그대로다. 자세한 표는 `failure-mapping.md`.

`no_failure_on_any_path_carries_the_host_the_port_or_the_model_name`은 아홉 가지 상황에서
`availability()`와 `generate_note()`가 만든 실패의 `message`와 `detail`을 모아
host · port · 모델 이름 · base URL이 들어 있지 않은지 확인한다.

그 성질은 규칙이 아니라 **타입이 지탱한다**:

- `http::TransportError`에는 문자열을 담을 자리가 아예 없다 (라이브러리 오류 문장에는 URL이
  들어 있을 수 있다). `http::tests::a_transport_error_can_carry_no_configured_value`가 변형을
  전부 나열해 그 성질이 조용히 깨지지 않게 한다.
- `wire::BodyRejection`도 마찬가지다.
- 비2xx의 `detail`은 `HTTP <상태 코드>`뿐이다 — **응답 본문을 담지 않는다.**
