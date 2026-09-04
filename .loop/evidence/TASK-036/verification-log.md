# TASK-036 검증 로그 — 코드를 쓰기 전에 확인한 것과 확인하지 못한 것

Run: RUN-20260903T064637Z-TASK-036 · 2026-09-03

ADR-0008 §14.3은 이 Task에 **재확인 의무**를 넘겼다. 이 문서는 그 의무에 대한 답이며,
"확인했다"와 "확인하지 못했다"를 섞지 않는다 (PRODUCT-SPEC §20.2).

> 이 파일에는 provider 설정값(host · port · 모델 이름)이 적혀 있지 않다 (ADR-0008 §11.3).

---

## 1. 공식 출처 재확인 — **다시 거부됐다**

ADR-0008 §14.3이 요구한 항목들을 현재 공식 출처에서 다시 확인하려 했고, 실패했다.

```text
WebFetch  https://raw.githubusercontent.com/ollama/ollama/main/docs/api.md
          → "Claude requested permissions to use WebFetch, but you haven't granted it yet."
```

TASK-032가 겪은 것과 같은 제약이다 (`.loop/evidence/TASK-032/verification-log.md`).
저장소 안에서도 다른 근거를 찾지 못했다 — `node_modules`에 Ollama 클라이언트가 없고,
`phase-prompt/**` · `docs/**`에도 API 응답 필드에 대한 기록이 없다.

**따라서 이 Run은 §14.5/§14.2의 어떤 값도 "2026-09-03에 재확인했다"고 주장하지 않는다.**
구현이 쓴 값의 등급은 여전히 **[E2] · 확인 시점 2026-09-01**이다.

---

## 2. 구현이 실제로 쓴 벤더 값과 그 근거

전부 ADR-0008 §14.2의 VERIFIED 행에서 왔다. **기억이나 추측으로 채운 이름이 없다.**
쓰인 자리는 `src-tauri/src/ai/ollama/wire.rs` 하나다.

| 쓴 값 | 근거 | 등급 · 확인 시점 |
| --- | --- | --- |
| `GET /api/tags` (가용성 + 모델 목록) | §14.2 항목 2 | VERIFIED · 2026-09-01 |
| 목록 응답의 `models[]`와 항목의 `name` · `model` | §14.2 항목 2 | VERIFIED · 2026-09-01 |
| `POST /api/generate` | §14.2 항목 3 | VERIFIED · 2026-09-01 |
| 요청 필드 `stream`(=`false`면 단일 JSON) | §14.2 항목 4 | VERIFIED · 2026-09-01 |
| 요청 필드 `format`(완전한 JSON Schema 객체) | §14.2 항목 5 | VERIFIED · 2026-09-01 |
| 요청 필드 `options.num_ctx` | §14.2 항목 10 | VERIFIED · 2026-09-01 |
| 요청 필드 `model` · `prompt` | §14.5 · ADR-0008 §6.4의 본문 예시 | VERIFIED · 2026-09-01 |
| 연결 대상(주소) | **쓰지 않았다** — 인자로 주입된다. adapter에 주소가 없다는 것을 `tests/ollama_adapter.rs::the_default_address_lives_in_the_settings_and_not_in_the_adapter`가 소스에서 확인한다 | — |

`GET /` · `/api/version`(헬스 체크)와 `POST /api/chat`은 **쓰지 않았다** (ADR-0008 §6.4).

---

## 3. 확인하지 못한 것과, 설계가 그것을 어떻게 다루는가

### 3.1 생성 응답에서 본문 텍스트가 담기는 필드 이름 — **UNVERIFIED [E4]**

ADR-0008 §14.3이 "§14.5에 명시되어 있지 않다"고 적은 항목이며, 이 Run도 확인하지 못했다.
§6.3의 1단계가 그 값을 읽으므로 그냥 넘어갈 수 없다.

**이름을 지어내지 않았다.** 대신 이름에 의존하지 않는 경로를 택했다
(`wire::generated_note`):

```text
응답 본문(JSON 객체)의 **최상위 문자열 값들**을 후보로 놓고
  → 각 후보에 §6.3의 2~4단계(제품의 ai::note::parse_note)를 그대로 적용하고
  → 그 mode의 노트로 읽히는 첫 후보를 취한다
```

- 느슨해진 것은 **어느 이름에서 꺼내는가** 뿐이다. 무엇을 노트로 받아들이는가는 제품의
  `parse_note`가 그대로 판정하므로 조금도 느슨해지지 않았다
  (`wire::tests::the_size_limits_of_the_domain_still_apply_to_what_the_server_sent`).
- 모델 이름 · 시각 같은 다른 문자열이 노트로 오인되지 않는다는 것을
  `wire::tests::other_strings_in_the_body_never_become_a_note`가 확인한다.
- 이름이 무엇으로 밝혀지든 같은 노트가 나온다는 것을
  `tests/ollama_adapter.rs::the_note_is_recovered_whatever_the_server_calls_the_field_that_holds_it`와
  `wire::tests::the_note_is_recovered_whatever_the_vendor_calls_the_field_that_holds_it`가 확인한다.
- **이름이 확인되면 좁힐 자리는 함수 하나다.**

이것은 ADR-0008 §3.1이 §14.5의 세부값에 대해 내린 세 결정과 같은 방식이다 —
*틀렸을 때 무너지는 범위를 줄이는 쪽*을 고른다.

### 3.2 생성 응답이 말하는 모델 이름 — **UNVERIFIED [E4]**

같은 이유로 응답에서 읽지 않는다. `NoteGeneration::model`에는 **이 요청이 지정한 모델**이
들어간다. 지정한 것이 이 adapter이므로 추정이 아니라 아는 사실이다.

### 3.3 모델이 없을 때의 상태 코드 / `format` 미지원 서버의 동작 — **UNVERIFIED [E4]**

ADR-0008 §14.2 항목 15 · 16 그대로다. 설계가 둘 다에 의존하지 않는다:
모델 존재는 `GET /api/tags`로 **먼저** 판정하고(§13.3), `format`이 무시되거나 거절돼도
파싱 경로가 같은 답을 낸다(§6.2).

---

## 4. HTTP 클라이언트 — ADR-0008 §12.3이 남긴 확인 항목

§12.3은 세 가지를 확인하라고 했다. 이 Run이 답한 것은 아래와 같다.

| 항목 | 결과 | 어떻게 확인했나 |
| --- | --- | --- |
| 최신 버전 | **`ureq` 3.4.0** — `Cargo.lock`이 버전과 checksum(`972d7902…`)까지 고정한다 | crates.io 인덱스에서 **cargo가 실제로 해석**했다. 기억이 아니라 resolver의 출력이다 |
| 동기 API가 async runtime을 요구하는가 | **요구하지 않는다** — `default-features = false`로 컴파일되고, `tokio`도 async 진입점도 없이 `HttpTransport`를 이행한다 | `cargo clippy --all-targets -- -D warnings` 통과 |
| 실제 API 시그니처 | 아래가 **컴파일러가 확인한 것**이다 | 같음 |

```text
ureq::Agent::config_builder().timeout_connect(Some(Duration)) .http_status_as_error(false) .build()
ureq::Agent::new_with_config(config)
agent.get(url).call()                          agent.post(url).content_type(..).send(&str)
response.status().as_u16()                     response.into_body().read_to_string()
ureq::Error::{Timeout(_), ConnectionFailed, Io(std::io::Error)}   (non_exhaustive)
```

**MSRV는 확인하지 못했다 [E4]** — crates.io 페이지를 열지 못했다. 다만 이 저장소의 툴체인
(clippy가 `rust-1.94.0` 문서를 가리킨다)에서 컴파일된다는 사실은 확인됐다.

`default-features = false`인 이유는 Cargo.toml 주석에 적었다 — 이 Phase는 사용자의 로컬
주소로만 나가므로 TLS가 필요 없고, 기본 feature는 rustls/ring을 함께 들여온다.
TLS는 Phase 5가 실제로 필요해질 때 켠다 (ADR-0008 §12.2 · PRODUCT-SPEC §20.5).

**`ollama-rs`를 쓰지 않았다** (§12.2).

---

## 5. 자동 검증이 실제 서버에 닿지 않는다는 것의 근거

"그런 테스트를 안 썼다"가 아니라 **네트워크에 닿을 수 있는 코드가 어디 있는지**로 말한다.

- `tests/ollama_adapter.rs::only_one_file_in_the_repository_can_open_a_socket` — `src/**` 전체를
  훑어 HTTP 클라이언트/소켓을 쓰는 파일이 **`ai/ollama/network.rs` 하나**임을 확인한다.
- `tests/ollama_adapter.rs::the_test_double_is_pure_values_and_the_tests_never_start_a_server` —
  test double에 네트워크가 없음을 소스에서 확인한다.
- 테스트가 쓰는 주소는 전부 `.invalid` TLD다(이름 해석이 성공할 수 없다). 서버를 띄우는
  코드도, `localhost` 접속도 없다.
