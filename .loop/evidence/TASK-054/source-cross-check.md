# TASK-054 — 문서의 값을 저장소의 실제 코드와 대조한 기록 (P12-AC2 · P12-AC3)

P12-AC2의 Instruction이 요구한 것이 이 표다 — **"문서의 값이 저장소의 실제 코드·Cargo.toml과
일치하는지 대조한다."** 아래는 전부 이 Run이 저장소 파일을 열어 읽은 것이다 ([E1]).
줄 번호는 이 Run 시점의 것이다.

> **실제 자격증명은 여기에도 없다.** 이 Run은 token을 만들지도 읽지도 않았고 네트워크에
> 나가지도 않았다.

---

## 1. ADR-0009 §15.2 — 확정된 값 넷

| 문서가 적은 값 | 저장소의 실제 값 | 어디 |
| --- | --- | --- |
| `CHUNK_MAX_BYTES = 60_000` · **[A] 앱이 고른 값** | `pub const CHUNK_MAX_BYTES: usize = 60_000;` — 주석이 "이 값은 이 앱이 고른 값이다. 확인된 Notion API 한도가 아니다"로 시작한다 | `src-tauri/src/notion/chunk.rs:62` |
| `CHUNK_MAX_BLOCK_UNITS = 300` · **[A]** | `pub const CHUNK_MAX_BLOCK_UNITS: usize = 300;` — block unit 정의(비어 있지 않은 줄 · 코드 펜스는 1)도 주석에 있다 | `chunk.rs:79` |
| 예산이 VERIFIED 500KB 아래임을 테스트가 검사한다 | `VERIFIED_REQUEST_LIMIT_BYTES = 500_000` · `WORST_CASE_JSON_ESCAPE = 6` · `assert!(CHUNK_MAX_BYTES * 6 < 500_000)` | `chunk.rs:631-636` |
| **750KB를 사실로 적지 않았다** | `chunk.rs`의 상수 주석이 "웹에서 보이는 750KB 같은 값은 primary source에서 확인된 적이 없으므로 이 상수의 근거가 아니다"라고 적는다. 저장소 전체에서 750KB가 API 한도로 쓰인 자리는 없다 | `chunk.rs` |
| `allow_async`를 보내지 않는다 | 요청 본문에 `children` · `content` · `properties` · `allow_async`가 없음을 테스트가 강제한다 | `notion/wire.rs:157` · `wire.rs:443` · `tests/notion_adapter.rs:131` |
| 요청하지 않은 `202`를 성공으로 보지 않는다 | `create_page`는 페이지 식별자를 읽지 못하면 실패(`NotionResponseUnusable` · `retryable=false`). `{"object":"async_task",…}` 응답을 넣은 전용 테스트가 있다 | `notion/client.rs:188` · `client.rs:588` |
| **ureq TLS feature = `rustls`** · 확인 버전 `ureq 3.4.0` · 루트는 `webpki-roots`(번들) · `default-features = false` 유지 | `ureq = { version = "3.4", default-features = false, features = ["rustls"] }` + 그 위 주석이 feature 이름 · 확인 버전 · 루트 출처 · 판정 근거 경로를 적는다 | `src-tauri/Cargo.toml:74` (주석 `:59-73`) |
| lock에 TLS 구현이 실제로 들어왔다 | `ureq 3.4.0` · `rustls 0.23.43` · `rustls-pki-types 1.15.1` · `ring 0.17.14` · `webpki-roots 1.0.9` | `src-tauri/Cargo.lock:4334` · `:3051` · `:3066` · `:2967` · `:4628` |
| **자격증명 crate = `keyring` 3.6.3 · `apple-native` + `windows-native`** | `keyring = { version = "3", default-features = false, features = ["apple-native", "windows-native"] }` | `Cargo.toml:75` |
| lock에 플랫폼 자격증명 API가 들어왔다 | `keyring 3.6.3`의 의존성에 `security-framework 2.11.1` · `3.7.0` · `windows-sys 0.60.2` | `Cargo.lock:1945` · `:3158` · `:3171` |
| Keychain 항목 식별자 `molt-note` / `notion-integration-token` | `pub const SECRET_SERVICE: &str = "molt-note";` · `Self::NotionIntegrationToken => "notion-integration-token"` | `platform/secret_store.rs:58` · `:77` |
| 지문 crate `sha2 = "0.10"` (lock 0.10.9) | `sha2 = "0.10"` + 주석 | `Cargo.toml:84` · `Cargo.lock:3382` |
| `Notion-Version: 2026-03-11` 하나 | `pub const NOTION_VERSION: &str = "2026-03-11";` | `notion/wire.rs:46` |
| 재시도 상수 350ms · 3회 · 120초 · 1·2·4초 | `MIN_REQUEST_INTERVAL = 350ms` · `MAX_RETRIES_PER_CHUNK = 3` · `MAX_WAIT = 120s` · `BACKOFF = [1s, 2s, 4s]` | `sync/pace.rs:33` · `:38` · `:44` · `:50` |
| migration 7 · 8 | `(7, "add_notion_settings")` · `(8, "add_notion_sync_progress")` — `settings`에 `notion_parent_page_id`, `notion_syncs`에 `sent_chunks` · `total_chunks` · `content_fingerprint` | `db/migrations.rs:238` · `:261` · `:387-388` |
| `exports/` 는 앱 데이터 루트에서 파생된다 | `const EXPORTS_DIR_NAME: &str = "exports";` · `exports_dir()` · `ensure_exports_dir()` | `platform/app_data_dir.rs:36` · `:126` · `:134` |
| 파일명 규칙 (슬러그 80바이트 · 이름 충돌 1000회) | `MAX_SLUG_BYTES = 80` · `MAX_NAME_ATTEMPTS = 1_000` | `export/filename.rs:38` · `export/file.rs:46` |

## 2. ADR-0009 §15.3 — 계획과 달라진 다섯 가지의 근거

| 달라진 것 | 저장소에서 확인한 사실 |
| --- | --- |
| **D-1** HTTP 경계를 Notion adapter 안에 따로 두었다 | `notion/http.rs`에 `HttpMethod{Get,Post,Patch}` · 요청 헤더 · `retry_after_seconds`가 있고, 그 모듈 주석이 이유(INV-9)를 적는다. `ai/ollama/http.rs`의 `HttpMethod`에는 `Patch`도 headers도 retry_after도 **없다** — 이 Phase에서 바뀌지 않았다 (`git status`에 `src-tauri/src/ai/` 항목이 없다) |
| **D-2** `pending`도 거절한다 | `ProcessingStatus::Pending \| ProcessingStatus::Running => Err(already_sending(sync))` | `sync/run.rs:251` |
| **D-3** 진행 중 갈래가 실패 값이다 | `already_sending`은 `Failure::retryable(InvalidInput, "이미 이 녹음을 Notion으로 보내고 있다.")` + detail `status=… · sentChunks=n/N` | `sync/run.rs:578-589` |
| **D-4** 본문의 `rate_limited`도 대기 경로로 본다 | `if matches!(code, Some(ApiErrorCode::RateLimited)) \|\| matches!(response.status, 429 \| 529)` → `NotionFailure::waiting(…)`. 대기 값은 `response.retry_after_seconds`(정수 초)에서만 온다 | `notion/client.rs:276` 부근 |
| **D-5** `sha2` 추가 | `Cargo.toml:84` (§1의 표) |

## 3. ADR-0009 §15.4 — 중복 sync 정책과 실제 동작 (P12-AC3)

`sync::run::plan`이 상태별로 무엇을 하는지 그대로 읽은 결과다.

| 저장된 상태 | 코드가 하는 일 | 어디 |
| --- | --- | --- |
| `None` | `Ok(Plan::CreatePage)` | `sync/run.rs:249` |
| `Pending` · `Running` | `Err(already_sending(sync))` | `:251` |
| `Done` | `new_page_after(confirmation, ConfirmBecause::AlreadySent)` | `:253` |
| `Failed` | `resumable(...)` 이면 `Plan::Resume`, 아니면 `new_page_after(confirmation, why_not_resumable(sync))` | `:254-257` |
| 이어 보낼 수 있는 조건 | `page_id` · fingerprint 일치 · `sent_chunks` **셋이 전부** 있어야 한다 | `:265` |
| 이어 보낼 수 없는 이유의 구분 | `page_id` 있음 → `DocumentChanged`, 없음 → `OutcomeUnknown` | `:276-282` |
| 확인이 없으면 | `Confirmation::NotAsked` → `Err(needs_confirmation(because))`, 즉 **아무것도 만들지 않는다** | `:286-290` |
| 새 페이지를 만드는 유일한 입력 | `Confirmation::NewPage` | `:288` |

저장 순서(§8.4)도 같은 파일의 모듈 주석이 단계로 적고 있으며(`run.rs:8-16`), **요청 전에
행을 쓰고 · `page_id`를 즉시 쓰고 · 성공한 요청만 센다**는 세 규칙이 그 순서다.

화면 쪽 (`src/screens/notionSyncView.ts`):

| 사실 | 어디 |
| --- | --- |
| 이어 보내는 버튼은 `Continue sending to the same page` (중복을 만들지 않는다는 문장이 함께 간다) | `:142` · `RESUME_OUTCOME :105` |
| 새 페이지를 만드는 유일한 동작이 `Create a new Notion page`이고 이것만 `confirmation: 'newPage'`를 싣는다 | `:168` |
| `done` 상태의 화면이 "기존 페이지는 그대로 둔다"를 **누르기 전에** 말한다 | `CONFIRM_OUTCOME.alreadySent :114` |
| 부분 전송이 숫자로 드러난다 | `"N of M parts of this document are already on that page."` `:227` 부근 |

**adapter에는 기존 페이지를 고치거나 지우는 경로가 없다** — `notion/client.rs`가 여는 것은
`create_page` · `append_markdown` · 연결 확인뿐이고, `lib.rs`의 command 목록에도 전송 기록이나
Notion 페이지를 지우는 이름이 없다 (`lib.rs:146-159`).

## 4. 이 대조가 하지 않은 것

- **실제 Notion 워크스페이스에서의 동작은 확인하지 않았다.** 위 표는 전부 코드를 읽은 것이며,
  실물 판정은 `docs/PHASE-5-NOTION-SMOKE-TEST.md`의 절차다.
- `cargo` 레지스트리 소스 · 외부 문서에 접근하지 않았다. 그래서 `ureq` · `keyring`의 feature
  전체 목록은 **여전히 UNVERIFIED**이며 ADR-0009 §15.5가 그대로 적고 있다.
