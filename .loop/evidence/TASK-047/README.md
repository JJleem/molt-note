# TASK-047 — Notion adapter (경계 · 요청 조립 · 응답 해석 · Retry-After · TLS)

`src-tauri/src/notion/` 아래에 Notion과 말하는 adapter를 만들었다. 구성은 Phase 4의 AI adapter와
같은 모양이다 — 네트워크에 닿는 얇은 경계 하나, 그 위의 순수한 wire 계층, 그리고 벤더 실패를
§13의 제품 상태로 옮기는 client.

> ⚠️ **이 Run은 실제 Notion API를 한 번도 호출하지 않았고, 실제 자격증명을 만들거나 읽지
> 않았다.** 이 디렉터리의 어떤 파일에도 token이 없다 (ADR-0009 §10.5 · `phase-prompt/05`).

---

## 1. 만든 것

```text
src-tauri/src/notion/
  mod.rs        디렉터리의 지도와 공개 표면
  http.rs       실제 네트워크에 닿는 유일한 경계(trait) — 요청 · 응답 · transport 오류
                · PATCH · 요청 헤더 · 응답의 Retry-After(정수 초)를 여기서 더했다 (ADR-0009 §5.5)
                · HttpRequest/HttpResponse 의 Debug가 값을 내지 않는다 (INV-7)
  wire.rs       벤더 지식(순수 함수) — 주소 · 헤더 이름 · API 버전 · 요청 본문 · 오류 코드
                · PageId · AuthorizationValue · retry_after_seconds
  client.rs     요청 조립 · 응답 해석 · 벤더 실패 → §13의 제품 상태 · NotionFailure/RetryAfter
  network.rs    그 경계의 실제 구현(ureq) — 이 adapter에서 소켓을 여는 유일한 파일
                ★ 자동 테스트가 실행하지 않는다 (Gate는 컴파일만 한다)
  testing.rs    결정론적 test double(StubServer) — 자동 검증은 이것만 쓴다

src-tauri/tests/notion_adapter.rs   통합 테스트 16개 (전부 stub transport)
```

**이 adapter는 요청 하나를 보내고 그 답을 옮기는 것까지만 한다.** 문서를 나누는 것(TASK-048) ·
몇 번 다시 보낼지와 무엇을 영속화할지(TASK-049)는 여기 없다 — 그래서 이 파일들에는 chunk 예산도
재시도 횟수도 `sleep`도 없다.

## 2. 함께 바꾼 것

| 파일 | 무엇 |
| --- | --- |
| `src-tauri/src/domain/failure.rs` | Notion 실패 **다섯 종류** 추가 + `as_str` + 1:1 테스트 갱신 |
| `src/ipc/failure.ts` | 같은 다섯을 union에 추가 (Rust와 1:1) |
| `tests/ipc-boundary.test.ts` | 1:1 검사를 **양방향으로** 강화 (frontend에만 있는 종류도 막는다) |
| `src-tauri/Cargo.toml` | `ureq`의 TLS feature를 이름으로 켜고, 켠 이름과 확인한 버전을 주석에 남김 |
| `src-tauri/Cargo.lock` | 그 결과로 실제로 들어온 TLS crate들 (판정 근거 — `ureq-tls-verification.md`) |
| `src-tauri/src/lib.rs` | `pub mod notion;` (마운트 한 줄) |
| `src-tauri/tests/ollama_adapter.rs` | "소켓을 열 수 있는 파일이 하나"였던 검사를 **선언된 둘**로 갱신 |

**Phase 4의 adapter 파일은 한 줄도 바뀌지 않았다.** 넓힌 HTTP 경계(PATCH · 헤더 · Retry-After)는
`notion/http.rs` 안에만 있다 — 두 벤더 adapter가 서로를 알게 되지 않도록 한 결정이며 그 이유는
`notion/http.rs` 모듈 주석에 적혀 있다 (ADR-0009 §5.5의 의도를 이 디렉터리 안에서 만족시킨다).

### 실패 다섯을 나눈 이유 (§13 · §20.6)

```text
notionAuthFailed              token을 다시 넣어야 한다
notionDestinationUnavailable  부모 페이지를 integration에 공유해야 한다
notionRateLimited             기다렸다가 다시 보낸다 (얼마나는 RetryAfter가 값으로 답한다)
notionRequestFailed           다시 시도한다 (연결 불가 · 타임아웃 · 거절 · 5xx)
notionResponseUnusable        결과를 모른다 — Notion에서 무엇이 만들어졌는지 확인해야 한다
```

**지금 실제로 만들어지는 것만 넣었다.** 다섯 전부 `notion::client`가 만드는 것이며,
`NOTION_FAILURE_KINDS`와 그 테스트(`every_failure_this_adapter_makes_is_one_of_the_five_declared_kinds`)가
"선언한 것과 만드는 것이 같다"를 고정한다. Notion 미설정 · 전송 진행 중 같은 아직 없는 상태의
자리는 만들지 않았다.

---

## 3. Acceptance Criteria 대조

| AC | 무엇으로 판정되는가 |
| --- | --- |
| **P5-AC1** build | `self-check.txt` — `build: PASS exit=0` |
| **P5-AC2** lint | `self-check.txt` — `lint: PASS exit=0` (`cargo clippy --all-targets -- -D warnings` 포함) |
| **P5-AC3** test (`FailureKind` ↔ `failure.ts` 1:1) | `self-check.txt` — `test: PASS exit=0`. 1:1을 고정하는 검사는 `domain/failure.rs::every_kind_has_a_distinct_stable_string`(다섯 추가) · `tests/ipc-boundary.test.ts`의 **두** 검사(Rust→TS, TS→Rust) |
| **P5-AC4** 요청 조립 | `tests/notion_adapter.rs`: `every_request_carries_bearer_auth_and_the_documented_api_version` · `the_api_version_is_the_documented_one_and_never_a_date_we_looked_it_up`(조사 날짜 `2026-09-01`·`2026-09-04`와 MCP 버전 `2026-07-28`이 아님을 고정) · `creating_a_page_sends_the_markdown_string_and_never_a_block_array` · `appending_asks_for_insertion_at_the_end_of_that_one_page` · `a_newline_in_the_document_goes_out_escaped_and_arrives_as_a_real_newline` |
| **P5-AC5** `429`/`529`의 Retry-After | `a_rate_limited_response_says_how_long_to_wait_in_whole_seconds`(429·529 × `30`/`120`/`0`) · `a_wait_this_app_cannot_read_is_left_as_unspecified_instead_of_invented`(HTTP-date·소수·문장) · `nothing_but_a_rate_limit_ever_asks_the_caller_to_wait` · 순수 파싱은 `wire::tests::the_wait_is_read_as_a_whole_number_of_seconds_and_nothing_else` |
| **P5-AC6** 실제 Notion 호출 없음 | 모든 테스트가 `notion::testing::StubServer`를 쓴다. 소스에서 확인하는 검사 셋: `the_test_double_is_pure_values_and_no_test_here_reaches_notion`(double에 네트워크 문자열 없음 · 이 테스트 파일이 실제 transport를 세우지 않음) · `only_the_network_file_of_this_adapter_can_open_a_socket` · `ollama_adapter.rs::only_the_two_declared_files_in_the_repository_can_open_a_socket` |
| **P5-AC7** token·destination 유출 없음 | `no_failure_on_any_path_carries_the_token_or_the_destination`(11개 상황 × 세 경로, `message`·`detail`·`Debug` 출력 전부 검사). stub은 **일부러** 오류 본문에 우리가 보낸 주소를 되돌려준다 — 그것을 옮기면 이 검사가 잡는다. 타입으로 막은 것: `HttpRequest`/`HttpResponse`/`PageId`/`AuthorizationValue`의 `Debug`, 문자열이 없는 `TransportError`/`BodyRejection` |
| **P5-AC8** ureq TLS | `ureq-tls-verification.md` — 켠 feature 이름(`rustls`) · 확인한 버전(3.4.0) · **`Cargo.lock` before/after** · 루트 인증서 출처(`webpki-roots` 번들) · 실제 빌드 산출물(`librustls.rlib` 등) · exit code |

## 4. 이 디렉터리의 파일

```text
README.md                    이 문서
ureq-tls-verification.md     ADR-0009 §11.4가 요구한 네 가지 (P5-AC8)
self-check.txt               build · lint · test Gate 실행 결과 (exit 0)
notion-adapter-tests.txt     notion 관련 테스트 이름과 결과 (통합 16 + 단위 34)
changed-files.txt            `git status --porcelain` 원본
task-047-tracked.diff        이 Task가 고친 **추적 중인** 파일들의 diff
```

⚠️ `changed-files.txt`와 `task-047-tracked.diff`에 대한 주의: **이 Phase는 아직 아무것도
commit되지 않았다** (`phase-prompt/05` Important Rules — commit은 운영자의 일이다). 그래서
`git diff`는 HEAD 기준이며 TASK-043~046이 만든 변경과 섞여 있다. 이 Task가 실제로 만든 파일과
고친 파일은 §1 · §2의 표가 정확한 목록이다 (특히 `src-tauri/src/lib.rs`와
`tests/ipc-boundary.test.ts`의 diff에는 앞선 Task들의 줄이 함께 들어 있고, 이 Task가 더한 것은
`pub mod notion;` 한 줄과 실패 종류 관련 검사뿐이다).

## 5. 이 Task가 하지 않은 것

```text
markdown 문서를 chunk로 나누는 규칙          → TASK-048
전송 순서 · NotionSync 영속화 · 재시도 횟수   → TASK-049
설정 화면 · token 입력 · connection test UI  → 뒤이은 Task
실제 Notion 페이지가 읽을 만한가             → Phase Goal의 Human Review 항목
실제 HTTPS 핸드셰이크가 서는가               → 같음 (자동 테스트가 판정하지 않는다)
```
