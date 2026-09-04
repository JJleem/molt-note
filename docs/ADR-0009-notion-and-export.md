# ADR-0009 — Notion 전송과 Markdown Export는 하나의 markdown 산출물을 공유하고, token은 앱 밖에 둔다

```text
Status:   Accepted (결정) · **구현됨 — 대조는 §15에 있다** (2026-09-04)
          §4~§11의 여덟 결정은 Phase 5의 뒤이은 Task들이 구현했다. 결정과 달라진 것,
          확정된 값, 여전히 UNVERIFIED인 것은 **§15만** 적는다. 결정 절은 고치지 않았다.
Date:     2026-09-04 (결정) · 2026-09-04 (§15 구현 대조)
Phase:    Phase 5 — Notion & Markdown Export
Task:     TASK-043 (작성) · TASK-054 (§15 구현 대조)
Scope:    export 디렉터리·파일명·이름 충돌 정책 · 쓸 Notion API 계약과 헤더 ·
          markdown chunk 예산 · allow_async · 중복 sync 정책과 재시도가 기대는 상태 ·
          429/529의 Retry-After · SecretStore 경계와 crate · ureq의 TLS 경로
```

> **§4~§11은 결정이다.** 결정 절을 사후에 고쳐 쓰지 않는다 — 그것을 고치면 무엇을 정했고
> 무엇이 실제로 만들어졌는지가 구분되지 않는다 (ADR-0007 §16 · ADR-0008 §17과 같은 방식).
> 구현이 결정과 달라지면 그 자리에 §15로 보내는 표시를 달고, 달라진 내용은 §15가 적는다.

---

## 1. Context

Phase 5는 기록을 Molt Note 밖으로 꺼내는 문 두 개를 만든다
(`phase-prompt/05-notion-and-export.md` · PRODUCT-SPEC §10 · §11).

```text
Recording + Transcript + (선택적) AINote
        │
        ├──▶ Markdown 렌더러 ──▶ 로컬 .md 파일        (외부 의존 없음 · INV-8)
        │                    └─▶ Notion 페이지        (같은 문자열을 그대로 보낸다)
```

**두 문이 같은 산출물을 쓴다는 것이 이 Phase의 핵심 사실이다.** Notion이 블록 JSON 대신
markdown 문자열을 직접 받는 것이 **VERIFIED**가 됐기 때문이다 (PRODUCT-SPEC §14.9.1).
그래서 이 Phase는 두 개의 렌더러를 만들지 않고, **하나의 markdown 문서와 그것을 요청 크기에
맞게 나누는 규칙**을 만든다.

그 전에 되돌리기 어려운 결정이 여덟 개 있다. 이 문서가 그 여덟 개를 확정한다.
전부 같은 질문에 답한다.

```text
이 사실이 틀리거나 바뀌면 무엇이 함께 무너지는가.
```

무너지는 범위가 파일 하나 · 상수 하나 · adapter 하나로 끝나는 쪽을 택했다. 특히 이 Phase는
**앱이 확인하지 못한 외부 사실 위에 서는 자리**(markdown 전용 요청 한도 · crate feature 이름 ·
TLS feature 이름)가 셋이나 있어서, 그 셋이 틀렸을 때 **조용히 잘못 동작하는 대신 눈에 보이게
실패하는가**가 설계 기준이 됐다.

전제 두 가지를 명시한다.

1. **외부로 나가는 호출은 Rust backend에서 나간다** — Phase 4가 이미 정했고 Phase 5도
   같은 규칙을 따른다 (ADR-0008 §5.4). frontend에는 Notion으로 가는 HTTP 경로가 없다.
2. **이 Run은 실제 Notion API를 한 번도 호출하지 않았다.** 이 문서의 Notion 사실은 전부
   PRODUCT-SPEC §14.9.1 · §14.9.2가 2026-09-04에 primary source에서 확인해 기록한 것이며,
   등급은 §3의 [E2]다.

---

## 2. Decision — 여덟 개 결정 요약

| | 항목 | 결정 |
| --- | --- | --- |
| **(1)** | export 위치 · 파일명 · 이름 충돌 | 앱 데이터 루트 아래 **`exports/`**. 파일명은 **`<created_at의 날짜>-<제목 슬러그>.md`**. 같은 이름이 있으면 **덮어쓰지 않고** `-2` · `-3` …을 붙이며, 자리를 못 찾으면 보이는 실패로 끝낸다 (§4) |
| **(2)** | Notion API 계약 | `POST /v1/pages`(`parent.page_id` + **`markdown`**) → `PATCH /v1/pages/:page_id/markdown`(`insert_content` + `position: {type:"end"}`). 연결 확인은 `GET /v1/users/me`. 헤더는 `Authorization: Bearer <secret>` · **`Notion-Version: 2026-03-11`** · `Content-Type: application/json` (§5) |
| **(3)** | chunk 예산 | **이 앱이 고른 값** — 요청 하나에 **markdown UTF-8 60,000 바이트** 그리고 **300 block unit**. **확인된 API 한도가 아니다.** VERIFIED된 일반 한도(1000 block elements · 500KB overall) 아래에서 JSON 이스케이프와 요청 부가분을 감안해 골랐다. markdown 엔드포인트 **전용** 상한은 **UNVERIFIED**이며 750KB 같은 값을 사실로 적지 않는다 (§6) |
| **(4)** | `allow_async` | **쓰지 않는다.** 보내지 않은 요청이 202를 돌려주면 성공으로 간주하지 않고 §8의 '결과를 모름' 상태로 남긴다 (§7) |
| **(5)** | 중복 sync | **Recording 1 ↔ Notion 페이지 1.** 끝나지 않은 sync의 재시도는 **같은 페이지에 남은 chunk부터 이어 보낸다**(중복 생성 없음). 이미 `done`인 것을 다시 보내는 것은 **사용자가 확인한 명시적 새 페이지 생성**이며, 기존 페이지를 덮어쓰거나 지우지 않는다. 그것을 가능하게 하는 비-secret 상태(`page_id` · `sent_chunks` · `total_chunks` · `content_fingerprint`)를 **요청을 보내기 전에** 적는다 (§8) |
| **(6)** | `Retry-After` | `429`·`529`에서 **정수 초**로 읽고 **그 시간만큼 멈춘 뒤 같은 chunk를 다시 보낸다.** 성공한 요청에서만 `sent_chunks`가 오르므로 재시도가 문서를 겹치게 하지 않는다. 헤더가 없거나 읽히지 않으면 앱이 고른 backoff를 쓰고, 그 사실을 값 옆에 적는다 (§9) |
| **(7)** | SecretStore | `src-tauri/src/platform/secret_store.rs`에 **trait 하나** + 닫힌 `SecretKey` enum + 재현되지 않는 `Secret` 타입. macOS는 OS 자격증명 저장소 구현, Windows는 같은 trait 뒤의 구현 자리(검증은 Phase 6). 자동 테스트는 **메모리 test double**만 쓴다. crate는 **`keyring`을 1순위**로 하되 **feature 구성은 이 Run이 확인하지 못했다(UNVERIFIED)** — 구현 Task가 확인하고 증거를 남기며, 확인에 실패하면 §10.6의 조건으로 대체 경로를 쓴다 (§10) |
| **(8)** | ureq TLS | `default-features = false`를 **유지**하고 TLS feature를 **명시적으로 하나 켠다.** 인증서 검증을 끄는 선택지는 없다. **feature 이름은 이 Run이 확인하지 못했다(UNVERIFIED)** — 구현 Task가 pin된 버전에서 확인해 그대로 적고, `Cargo.lock`에 TLS crate가 실제로 들어왔는지로 판정한다 (§11) |

이 Phase에서 **하지 않는 것**은 §13에 따로 적었다.

---

## 3. 근거의 종류 — 이 Run이 확인할 수 있었던 범위

**추측한 것을 확인한 것처럼 적지 않는다** (PRODUCT-SPEC §20.2). 문서 전체에서 아래 표기를 쓴다.

| 표기 | 뜻 |
| --- | --- |
| **[E1] 직접 확인** | 이 Run에서 이 저장소의 실제 파일을 읽어 확인했다 |
| **[E2] §14.9.1 / §14.9.2 (2026-09-04)** | PRODUCT-SPEC이 **2026-09-04에** primary source(developers.notion.com)에서 확인해 기록한 값. **이 Run이 다시 확인한 것이 아니다** |
| **[E3] 2차 출처** | 1차 출처가 아닌 근거로만 확인된 값 |
| **[E4] UNVERIFIED** | 확인하지 못했다. **확인된 사실로 쓰지 않는다** |
| **[A] 앱이 고른 값** | 외부 사실이 아니라 이 앱이 정한 값. 근거는 있지만 **API 한도가 아니다** |

**[A]는 이 문서에서 새로 도입한 표기다.** §6의 chunk 예산처럼 "VERIFIED된 한도 아래에서
앱이 스스로 고른 값"을 외부 사실과 같은 칸에 적으면 안 되기 때문이다
(`phase-prompt/05` P-1 · PRODUCT-SPEC §14.9.1).

### 3.1 ⚠️ 이 Run에는 네트워크 접근이 없었다

이 Run은 두 가지를 직접 확인하려 했고 **거부됐다.**

```text
WebFetch  https://docs.rs/crate/ureq/3.4.0/features
          → "Claude requested permissions to use WebFetch, but you haven't granted it yet."
WebFetch  https://crates.io/api/v1/crates/keyring
          → "Claude requested permissions to use WebFetch, but you haven't granted it yet."
WebSearch "ureq 3.4 cargo features rustls native-tls platform-verifier default features"
          → "Claude requested permissions to use WebSearch, but you haven't granted it yet."
```

로컬 cargo registry(`~/.cargo/registry`)도 이 세션의 작업 디렉터리 밖이라 읽을 수 없었다.

```text
ls /Users/molt/.cargo/registry/src
  → blocked: "may only list files in the allowed working directories"
```

기록: `.loop/evidence/TASK-043/verification-log.md`.

**따라서 이 문서는 `ureq`의 TLS feature 이름과 `keyring`의 플랫폼 feature 구성을
"확인했다"고 적지 않는다.** 둘 다 [E4]이며, §10·§11은 그 사실 위에서 결정을 내린다 —
**모르는 것을 아는 것처럼 적는 대신, 틀렸을 때 조용히 넘어가지 않는 판정 수단을 함께 정했다.**

Notion 쪽 사실은 사정이 다르다. PRODUCT-SPEC §14.9.1 · §14.9.2가 **바로 이 Phase의 계획
시점(2026-09-04)에** primary source에서 확인해 기록한 값이며, 이 문서가 그것을 다시 조사할
이유는 없다. 등급은 [E2]다.

### 3.2 이 Run이 저장소에서 직접 읽어 확인한 것 ([E1])

| 확인한 사실 | 파일 |
| --- | --- |
| `notion_syncs`는 **`recording_id TEXT PRIMARY KEY`** · `page_id` · `synced_at` · `status`(CHECK `none/pending/running/done/failed`) · `error`를 갖는다. **Recording 하나당 행 하나**가 스키마로 강제돼 있다 | `src-tauri/src/db/migrations.rs` (migration 2) |
| `save_notion_sync`는 `ON CONFLICT (recording_id) DO UPDATE`로 **같은 Recording의 기록을 대체**하고, `load_notion_sync`는 시도한 적이 없으면 `None`을 준다 | `src-tauri/src/db/store.rs` |
| 적용된 migration은 1~6이며 **다음 번호는 7**이다. version 3의 규약: 새 값은 앞 migration을 고치지 않고 **열을 더하고**, `NOT NULL`/`DEFAULT`를 두지 않으며 **NULL은 '아직 없음'**이다 | `src-tauri/src/db/migrations.rs` |
| `settings` 테이블에는 **secret 열이 없다.** migration 3·4·5·6 전부 "여기서도 secret 열은 만들지 않는다"고 적혀 있다 (INV-7) | 같음 |
| 앱 데이터 루트에서 파생되는 경로는 `molt-note.db` · `recordings/` · `models/` 셋이며, **경로를 만드는 자리는 `AppDataDirectory` 하나**다. 테스트는 임시 루트를 주입한다 (INV-10) | `src-tauri/src/platform/app_data_dir.rs` |
| `platform` 모듈에는 지금 `app_data_dir` · `clock` · `microphone` 셋이 있고, 모듈 주석이 "플랫폼 지식이 갇혀 있는 경계"라고 규정한다 | `src-tauri/src/platform/mod.rs` |
| 파일 이름 충돌 정책의 선례가 이미 있다 — `capture::output_path`는 `stem.wav` · `stem-2.wav` …를 최대 `MAX_PATH_ATTEMPTS`(1000)까지 시도하고 **존재하는 파일을 덮어쓰지 않으며**, 못 찾으면 `FailureKind::Storage`로 실패한다 | `src-tauri/src/audio/capture.rs` |
| 저장되는 시각은 SQLite가 만든 UTC 텍스트다 (`strftime('%Y-%m-%dT%H:%M:%fZ','now')` → `2026-09-02T10:00:00.000Z`). 기본 제목은 그 문자열의 `0..10`(날짜)과 `11..16`(시:분)을 **그대로 잘라 쓰며 시간대를 계산하지 않는다** | `src-tauri/src/db/store.rs` · `src-tauri/src/commands/mod.rs::title_for` |
| HTTP 경계는 `HttpTransport` trait 하나이고, `HttpRequest`는 **`Get`·`Post`만** 가지며 **헤더를 담을 자리가 없다.** `HttpResponse`는 `status` + `body`뿐이라 **응답 헤더를 읽을 수 없다** | `src-tauri/src/ai/ollama/http.rs` |
| 실제로 소켓을 여는 파일은 `ai/ollama/network.rs` 하나이며 **자동 테스트가 그 파일을 실행하지 않는다**(Gate는 컴파일만 한다). 테스트는 `testing::StubServer`를 쓴다 | `src-tauri/src/ai/ollama/network.rs` |
| `TransportError`에는 문자열이 없다 — "옮길 수 있는 문자열이 애초에 없으면 새어 나갈 수도 없다"가 명시된 설계다 | `src-tauri/src/ai/ollama/http.rs` |
| `ureq = { version = "3.4", default-features = false }`이며 주석이 "TLS는 Phase 5가 필요해질 때 켜는 것이 맞다"고 적어 두었다 | `src-tauri/Cargo.toml` |
| **`Cargo.lock`의 `ureq` 3.4.0 의존성은 `base64` · `log` · `percent-encoding` · `ureq-proto` · `utf8-zero` 다섯뿐이고, lock 전체에 `rustls` · `ring` · `native-tls` · `security-framework` 항목이 하나도 없다.** 즉 **지금 이 저장소는 HTTPS를 열 수 없다** | `src-tauri/Cargo.lock` |
| frontend에는 `localStorage` · `sessionStorage` 사용이 **하나도 없다** | `src/` 전체 grep |
| 상태 갱신은 `update_recording_statuses` 하나를 지나며, 전사·AI 경로는 자기 것이 아닌 상태(`notion_status` 포함)를 **읽은 그대로 다시 쓴다** | `src-tauri/src/transcription/run.rs` · `src-tauri/src/ai/run.rs` |

이 문서의 결정 중 저장소 사실에 근거한 것은 전부 위 표에서 왔다.

---

## 4. (1) Export 디렉터리 · 파일명 · 같은 이름이 이미 있을 때

### 4.1 결정 — 위치

Markdown export 파일은 **앱 데이터 루트 아래 `exports/`** 에 놓는다.

```text
<app data root>/
  molt-note.db
  recordings/
  models/
  exports/          ← 이 Phase가 더하는 자리
    2026-09-01-3dgs-study-04.md
```

경로를 만드는 코드는 **`AppDataDirectory`에 `exports_dir()` · `ensure_exports_dir()`을 더하는
것뿐**이다 [E1 — 지금 있는 셋과 같은 형태]. export 코드는 플랫폼 경로를 스스로 만들지 않는다
(INV-10).

**근거**

| 근거 | 내용 |
| --- | --- |
| 경로를 아는 자리가 하나로 유지된다 | DB · 녹음 · 모델이 이미 한 루트에서 파생된다 [E1]. export만 다른 방식으로 자리를 정하면 "이 앱이 파일을 어디에 두는가"를 답하는 자리가 둘이 된다 |
| 테스트가 사용자 디렉터리를 건드리지 않는다 | `AppDataDirectory::new(임시 경로)`로 전 경로가 검증된다 [E1 — 기존 테스트가 그렇게 돼 있다] |
| 새 플랫폼 지식이 생기지 않는다 | `Documents`·`Downloads` 같은 자리를 고르면 OS별 규약과 권한(특히 macOS의 TCC)이 새 변수로 들어온다. 앱 데이터 루트는 Tauri가 이미 답하고 있다 |
| 파일 저장 대화상자를 V1 경로로 만들지 않는다 | 대화상자는 사용자가 위치를 매번 고른다는 뜻이고, 그러면 **파일명이 결정론적이라는 요구(§11 · Phase Goal A-2)가 검증 대상에서 빠진다.** 결정론적 이름을 먼저 만들고, 위치 선택은 그 위에 얹는 편이 싸다 |

**export 위치를 사용자 설정으로 노출하지 않는다** — `settings.recordings_directory`처럼
열을 더하는 일은 이 Phase의 요구가 아니고, 필요해진 뒤에 더하는 편이 싸다
(PRODUCT-SPEC §20.5 — 미래 의존성 선입금 금지). 대신 **export가 끝나면 만들어진 파일의
전체 경로를 UI가 보여 준다.** 사용자가 파일을 찾지 못하는 상태로 두지 않는다.

### 4.2 결정 — 파일명

```text
<date>-<slug>.md          date = recordings.created_at 의 앞 10글자
                          slug = 제목을 아래 규칙으로 정규화한 값
```

**날짜**는 저장된 UTC 텍스트의 `0..10`을 **그대로 잘라 쓴다.** 시간대를 계산하지 않는다 —
`title_for`가 이미 같은 규칙이고 같은 이유를 적어 두었다 [E1]. 그 대가로 **현지 시각으로 늦은
밤에 녹음한 것은 파일명 날짜가 다음 날일 수 있다.** 그 편차를 감수하는 이유는, 시간대를
계산하는 순간 **같은 Recording이 기기 설정에 따라 다른 파일명을 갖게 되어 "결정론적"이라는
성질 자체가 깨지기 때문**이다. 잘라 낸 값이 예상한 모양이 아니면(문자열이 짧거나 형식이
다르면) 날짜를 지어내지 않고 **`unknown-date`** 를 쓴다.

**슬러그 규칙** (순서대로 적용한다. 전부 순수 함수이며 파일시스템에 묻지 않는다)

1. 유니코드 문자/숫자는 남긴다 — **한글도 남긴다.** 이 제품의 제목은 한국어가 기본이고,
   한글을 버리면 대부분의 제목이 빈 슬러그가 된다.
2. 그 밖의 모든 것(공백 · 문장부호 · `/` `\` `:` `*` `?` `"` `<` `>` `|` · 제어문자 ·
   개행 · 이모지 · 결합문자만 남은 조각)은 **`-` 하나로 바꾼다.**
3. 연속된 `-`를 하나로 줄이고, 앞뒤의 `-`를 없앤다.
4. ASCII 대문자만 소문자로 바꾼다. **유니코드 케이스 폴딩은 하지 않는다** — 언어별 규칙이
   개입하면 같은 제목이 로캘에 따라 다른 파일명을 만든다.
5. **80 바이트를 넘으면 UTF-8 문자 경계에서 자른다.** 파일명 한도(대개 255 바이트)에
   `<date>-`(11 바이트) · `.md` · 충돌 접미사가 함께 들어갈 자리를 남긴다. **이 80은 [A]다.**
6. 결과가 비면 **`untitled`** 를 쓴다.
7. Windows 예약 이름(`con` `prn` `aux` `nul` `com1`~`com9` `lpt1`~`lpt9`)과 정확히 같으면
   **`-file`** 을 덧붙인다. 끝나는 `.`과 공백은 이미 2·3에서 사라진다.

```text
"3DGS Study #04"          → 2026-09-01-3dgs-study-04.md
"회의: 로드맵 / Q4 🎯"     → 2026-09-01-회의-로드맵-q4.md
"///"                     → 2026-09-01-untitled.md
```

Windows 규칙을 macOS 전용 Phase에서 미리 지키는 이유는 하나다 — **파일명은 만들어지고 나면
남는다.** Phase 6에서 규칙을 바꾸면 이미 만들어진 파일들과 규칙이 갈린다. 규칙을 지금
한 번만 정하는 편이 싸다 (INV-10 · §3.1).

### 4.3 결정 — 같은 이름이 이미 있을 때

**덮어쓰지 않는다.** `capture::output_path`와 **같은 규칙**을 쓴다 [E1].

```text
2026-09-01-3dgs-study-04.md        없으면 이것
2026-09-01-3dgs-study-04-2.md      있으면 이것
2026-09-01-3dgs-study-04-3.md      …
1000번째까지 자리가 없으면 → FailureKind::Storage 로 보이는 실패
```

| 후보 | 판정 | 이유 |
| --- | --- | --- |
| 덮어쓰기 | **거절** | export한 파일은 **사용자의 문서다.** Obsidian에서 손댔을 수도 있고 다른 곳으로 옮기는 중일 수도 있다. 덮어쓰면 그것을 되돌릴 방법이 없다. 앱이 자기 산출물이라고 사용자 파일을 지우지 않는다는 규칙은 INV-4의 태도와 같다 |
| 실패 | 거절 | "이미 있습니다"만 말하고 끝나면 사용자는 이름을 직접 바꾸는 일 말고 할 수 있는 게 없다. export는 여러 번 하는 것이 정상이다 (AI Note를 다시 만든 뒤 다시 내보낸다) |
| **번호 붙이기** | **채택** | 기존 파일이 남고, 새 결과도 남고, 시간 순서가 이름에 보인다. **저장소에 이미 같은 규칙이 있어서** 사용자가 두 종류의 규칙을 배우지 않아도 된다 [E1] |
| 내용 해시 접미사 | 거절 | 같은 내용을 다시 내보내면 파일이 늘지 않는다는 장점이 있지만, 이름이 `-a3f9c1`처럼 사람이 읽을 수 없게 된다. §11이 요구하는 것은 "외부 도구에서 그대로 쓸 수 있는 형태"다 |

**자리를 찾는 것과 쓰는 것 사이에는 경합이 있다** — 확인한 뒤 쓰기 전에 다른 프로세스가 같은
이름을 만들 수 있다. 그래서 **파일 생성은 `create_new`(존재하면 실패) 의미로 하고, 그 실패는
다음 번호로 넘어가는 신호로 쓴다.** 확인만으로 안전하다고 가정하지 않는다.

---

## 5. (2) 쓸 Notion API 계약

### 5.1 결정 — 엔드포인트

전부 [E2 · §14.9.1]이다. base는 `https://api.notion.com`이다.

| 목적 | 호출 | 본문 |
| --- | --- | --- |
| 연결 확인 | **`GET /v1/users/me`** | 없음 |
| 페이지 생성 (첫 chunk) | **`POST /v1/pages`** | `{"parent":{"page_id":"<설정된 부모 페이지>"},"markdown":"<chunk 1>"}` |
| 이어붙이기 (나머지 chunk) | **`PATCH /v1/pages/<page_id>/markdown`** | `{"type":"insert_content","insert_content":{"content":"<chunk n>","position":{"type":"end"}}}` |

```text
chunk 1 ──POST /v1/pages──▶ page_id 를 받는다 (여기서 즉시 저장한다 · §8.4)
chunk 2..n ──PATCH /v1/pages/<page_id>/markdown (insert_content · position end)──▶ 순서대로
```

### 5.2 결정 — 헤더

```text
Authorization:  Bearer <integration token>      ← 값의 출처는 SecretStore 하나뿐이다 (§10)
Notion-Version: 2026-03-11
Content-Type:   application/json                ← 본문이 있는 요청에만
```

**`Notion-Version`은 `2026-03-11` 하나다.** 근거는 PRODUCT-SPEC §14.9.2 —
developers.notion.com/reference/versioning의 *"The most recent `Notion-Version` is
`2026-03-11`"* 와, changelog를 2026-09-02 항목까지 열람해 더 새로운 API 버전이 없음을 확인한
기록이다 [E2].

⚠️ **조사 날짜를 버전으로 쓰지 않는다** (§14.9.2).

| 문자열 | 정체 | 헤더 값으로 쓰는가 |
| --- | --- | --- |
| `2026-03-11` | Notion API 버전 | **그렇다. 이 값 하나뿐이다** |
| `2026-09-01` · `2026-09-04` | PRODUCT-SPEC이 **조사한 날짜** | 아니다 |
| `2026-07-28` | Notion **MCP 프로토콜** 버전 | 아니다 |
| `2025-09-03` · `2022-06-28` | 과거 · deprecated 버전 | 아니다 |

값은 **상수 하나**로 두고, 그 상수가 위 표의 다른 값이 아님을 테스트가 확인한다. 버전을
설정으로 노출하지 않는다 — 사용자가 고를 수 있는 값이 아니다.

### 5.3 제목을 어떻게 정하는가 — `properties`를 보내지 않는다

**`properties.title`을 보내지 않고, markdown 첫 줄의 `# h1`이 페이지 제목이 되게 한다**
[E2 · §14.9.1 — *"`properties.title`을 생략하면 첫 `# h1`이 페이지 제목으로 쓰인다"*].

§11의 Markdown 산출물은 이미 `# <제목>`으로 시작한다. 그러므로 제목의 출처가 하나로
유지되고, 렌더러가 만든 문서와 Notion 페이지의 제목이 어긋날 수 없다.

**더 중요한 이유가 있다.** §14.9.1은 `markdown`과 `children`/`content`를 **같은 요청에 함께 쓸
수 없다**는 것을 VERIFIED로 적었지만, **`properties`와 `markdown`을 함께 보낼 수 있는지는
적지 않았다** — 즉 [E4]다. 확인되지 않은 조합 위에 페이지 생성 경로를 세우지 않는다.
확인된 동작(첫 h1)만으로 필요한 결과가 나오므로 그쪽을 택한다.

### 5.4 markdown 문자열을 만들 때 지키는 것 ([E2 · §14.9.1])

- 줄바꿈은 **실제 개행**이며 JSON에서는 `\n`이다. 문단 안 줄바꿈이 필요하면 `<br>`.
- 지원되지 않는 블록(bookmark · embed · link preview · breadcrumb · template)은
  `<unknown url="..." alt="block_type"/>`으로 돌아온다. **§11의 렌더러는 그런 블록을 만들지
  않으므로** 이 Phase가 그것을 다룰 일은 없다. 다루지 않는다는 사실을 여기 적어 둔다.
- `children`/`content` 배열을 만들지 않는다. 블록 JSON 경로는 쓰지 않는다.

### 5.5 이 계약이 저장소에 요구하는 변화

현재 `HttpTransport`는 **`Get`·`Post`만 있고 요청 헤더를 담을 자리가 없으며, 응답 헤더를
읽을 수 없다** [E1]. Notion은 `PATCH`와 세 개의 헤더를 요구하고, §9는 응답의 `Retry-After`를
요구한다. 그러므로 이 Phase는 그 경계를 **넓히되 최소로** 넓힌다.

```text
HttpMethod  에 Patch 를 더한다
HttpRequest 에 headers(이름/값 쌍의 슬라이스)를 더한다
HttpResponse 에 retry_after_seconds: Option<u32> 를 더한다   ← 헤더 원문이 아니라 파싱된 값이다
```

**응답 헤더 전체를 노출하지 않는다.** `TransportError`에 문자열을 두지 않은 것과 같은
이유다 [E1] — 필요한 것 하나만 값으로 꺼내면, 나머지가 실패 문장이나 로그로 새어 나갈 통로가
아예 없다. 요청 헤더에는 token이 실리므로 **`HttpRequest`는 `Debug`로 헤더 값을 출력하지
않는다**(§10.4).

---

## 6. (3) 나누는 단위 — 이 앱이 고른 chunk 예산

### 6.1 먼저, 무엇이 확인됐고 무엇이 확인되지 않았는가

```text
VERIFIED    [E2 · §14.9.1]  일반 요청 한도 — 요청당 1000 block elements · 500KB overall
                            (reference/request-limits)
VERIFIED    [E2 · §14.9.1]  페이지 전체는 약 20,000 블록 이후 잘린다
VERIFIED    [E2 · §14.9.1]  rate limit — 연결당 평균 초당 3회

UNVERIFIED  [E4]            markdown 엔드포인트 **전용** 본문 크기 상한 —
                            **그런 값이 따로 있는지조차 확인되지 않았다.**
                            markdown 가이드는 어떤 숫자도 적지 않으며
                            "keep pages under a few thousand blocks"만 적는다.
```

⚠️ **웹에서 보이는 750KB 같은 값은 primary source에서 확인되지 않았다. 이 문서는 그것을
사실로 기록하지 않으며, 구현 상수의 근거로도 쓰지 않는다.**

### 6.2 결정 — 두 개의 예산. 둘 다 **[A] 앱이 고른 값**이다

```text
CHUNK_MAX_BYTES        60_000     markdown 한 chunk의 UTF-8 바이트 상한   [A]
CHUNK_MAX_BLOCK_UNITS  300        markdown 한 chunk의 block unit 상한     [A]
```

**이 둘은 확인된 API 한도가 아니다.** VERIFIED된 일반 한도 아래에서 이 앱이 고른 값이며,
코드의 상수 주석과 이 절이 그렇게 표시한다. 그 표시를 지우지 않는다.

**60,000 바이트를 고른 계산**

```text
JSON 문자열 이스케이프의 최악 팽창은 문자당 6배다  ("\u00XX" — 제어문자)
  60,000 × 6 = 360,000 B   <  500,000 B (VERIFIED)
흔한 팽창(따옴표·역슬래시·개행이 2배)에서는
  60,000 × 2 = 120,000 B   —  여유가 4배 이상 남는다
남는 자리는 URL · 헤더 · JSON 봉투 · 서버가 세는 방식이 우리와 다를 가능성에 둔다
```

즉 **본문이 전부 제어문자여도 VERIFIED 한도를 넘지 않는다.** 이 여유가 필요한 이유는
§6.1이다 — markdown 전용 한도가 일반 한도보다 **작을 수도 있고**, 우리는 그것을 모른다.
모르는 쪽으로는 보수적으로 간다.

**300 block unit을 고른 계산**

앱은 markdown 한 문서가 Notion에서 정확히 몇 block element가 되는지 **알 수 없다** [E4] —
표 한 줄이나 중첩 리스트가 몇 개로 펼쳐지는지는 문서화돼 있지 않다. 그래서 **세는 방법을
보수적인 대용값으로 정한다.**

```text
block unit = chunk 안의 "비어 있지 않은 줄" 수. 코드 펜스 블록은 통째로 1로 센다.
  300 × 3.3 ≈ 1000 (VERIFIED 요청당 block elements)
```

**한 줄이 여러 block element로 펼쳐져도 3.3배까지는 버틴다.** 이 배수가 충분한지는 확인된
사실이 아니다 — 그래서 **판정 기준을 상수에 두지 않는다**(§6.4).

### 6.3 결정 — 나누는 자리

**나누는 자리는 언제나 줄 경계 위에 있다.** 아래 순서로 자리를 찾는다.

```text
1. 빈 줄(문단 경계)에서 나눈다.                       ← 기본
2. 한 문단이 혼자 예산을 넘으면 그 안의 줄 경계에서 나눈다.
3. 코드 펜스(``` … ```) 안에서는 절대 나누지 않는다.  ← 펜스가 갈리면 문서가 달라진다
4. 한 줄이 혼자 CHUNK_MAX_BYTES를 넘으면 → **나누지 않고 보이는 실패로 끝낸다.**
```

4번을 자르지 않는 이유: 줄 가운데를 자르면 `**강조**`나 링크가 갈려 **보낸 문서가 원본과
다른 문서가 된다.** 그것은 "무손실"이 아니다. 그리고 이 경로는 이 앱이 만드는 데이터에서
사실상 도달하지 않는다 — ADR-0008 §7.4가 노트의 배열 원소를 2,000자, `overview`를 4,000자로
제한하고 있고 [E1 근거는 그 문서다], transcript 한 segment는 초 단위 발화다. 60,000 바이트짜리
한 줄은 그 어느 쪽에서도 나오지 않는다.

**무손실의 정의를 코드가 검사할 수 있게 적는다.**

```text
join(chunks) == 원본 markdown            (바이트 단위로 동일)
```

즉 나누기는 **문자열을 자르는 것이지 바꾸는 것이 아니다.** chunk 경계에서 개행을 먹거나
더하지 않는다. 이것을 property 형태의 테스트가 강제한다(임의 길이의 문서 → 나누고 → 다시
이어붙이면 원본).

> ⚠️ **한 가지는 앱이 보장할 수 없다.** `insert_content`로 이어붙인 결과 페이지가
> `join(chunks)`를 한 번에 보낸 것과 **완전히 같은 블록 구조**가 되는지는 확인되지 않았다
> [E4]. 문단 경계에서만 나누는 것(§6.3-1)이 그 위험을 줄이지만 없애지는 못한다.
> 이것은 Phase Goal의 Human Review 항목 "긴 transcript가 Notion에서 실제로 온전한가"가
> 판정한다. **자동 테스트가 이 항목을 통과했다고 말하지 않는다.**

### 6.4 판정 기준은 상수가 아니다

```text
전체 문서가 · 순서대로 · 무손실로 전송되거나,
실패가 그 사실을 드러내고 재시도된다.
```

그러므로 서버가 우리 예산 안의 요청을 거절하면(413 · `validation_error`) **그것은 상수를
줄여야 한다는 신호이며, 실패는 사용자에게 그대로 보인다.** 조용히 자르거나 남은 부분을
버리는 경로는 없다. 상수를 줄이는 일은 코드 한 줄이고, 그 값이 [A]라고 적혀 있으므로 다음
사람이 "API 한도를 바꾸는 것"으로 오해하지 않는다.

---

## 7. (4) `allow_async` — 쓰지 않는다

### 7.1 결정

**요청에 `allow_async: true`를 넣지 않는다.** 모든 chunk는 동기 요청 하나로 끝난다.

### 7.2 근거

| 근거 | 내용 |
| --- | --- |
| **필요가 없다** | `allow_async`는 큰 본문을 위한 것이다 [E2 · §14.9.1 — `202` + `async_task`의 `status_url`·`poll_after_seconds` 폴링]. 그런데 §6이 본문을 60KB로 이미 나눈다. 큰 본문을 만들지 않기로 한 설계 위에서 큰 본문용 수단을 켜는 것은 앞뒤가 맞지 않는다 |
| **비용이 크다** | 폴링은 새 상태 기계다 — `status_url`을 어디에 두는가 · 언제까지 폴링하는가 · 앱이 꺼지면 그 작업은 어떻게 되는가 · 폴링 중 실패는 어떤 실패인가. 그리고 그 작업 id는 §8이 영속화해야 할 상태가 하나 더 늘었다는 뜻이다 |
| **동기 경계와 맞지 않는다** | 이 저장소의 모든 경계가 동기이고 async runtime이 없다 [E1 · ADR-0008 §12.2]. 폴링은 대기이며, 대기는 스레드를 붙들거나 런타임을 부른다 |
| **되돌리기 싸다** | 나중에 필요해지면 요청 본문에 필드 하나를 더하고 폴링 경로를 adapter 안에 넣으면 된다. §6의 chunk 규칙도 §8의 상태도 바뀌지 않는다 |

### 7.3 요청하지 않았는데 `202`가 오면

**성공으로 간주하지 않는다.** 우리가 `allow_async`를 보내지 않았으므로 `202`는 이 앱이
해석할 수 없는 응답이다. 그때 하는 일은 두 가지다.

1. **그 chunk를 보낸 것으로 세지 않는다** (`sent_chunks`를 올리지 않는다).
2. sync를 **§8.5의 '결과를 모름' 상태**로 남긴다 — 페이지가 만들어졌을 수도 있다는 사실을
   사용자 문장으로 알리고, 다음 전송이 조용히 중복을 만들지 않게 한다.

**"모른다"를 "실패했다"로도 "성공했다"로도 바꿔 적지 않는다.** 그 둘 다 이 앱이 확인하지
못한 진술이다.

---

## 8. (5) 중복 sync 정책과, 재시도가 기대는 상태

### 8.1 결정 — Recording 하나에 Notion 페이지 하나

```text
Recording  1  ↔  1  Notion 페이지
```

저장소가 이미 그렇게 서 있다 — `notion_syncs.recording_id`가 **PRIMARY KEY**이고
`save_notion_sync`가 같은 Recording의 기록을 대체한다 [E1]. 이 결정은 그 스키마를
그대로 쓴다.

### 8.2 결정 — 끝나지 않은 sync의 재시도는 **이어 보낸다**

부분 성공 뒤의 재시도는 **새 페이지를 만들지 않는다.** 이미 만들어진 `page_id`에
**아직 보내지 않은 chunk부터** 이어 보낸다.

```text
chunk 1..3 성공 · chunk 4에서 실패
   → 재시도는 chunk 4부터. 페이지도 chunk 1..3도 그대로 둔다.
```

이것이 성립하려면 두 가지가 참이어야 하고, 둘 다 만든다.

1. `page_id`와 `sent_chunks`가 **영속화돼 있다** (§8.4). 앱이 꺼졌다 켜져도 남는다.
2. 지금 보내려는 문서가 **그때 나눈 그 문서와 같다** — `content_fingerprint`가 같다.
   다르면 이어 보내는 것은 서로 다른 두 문서를 페이지 하나에 이어 붙이는 일이 된다.
   그래서 **fingerprint가 다르면 이어 보내지 않는다** (§8.5).

### 8.3 결정 — 이미 `done`인 것을 다시 보내면 **명시적으로 새 페이지를 만든다**

§10(PRODUCT-SPEC)이 고르라고 한 두 선택지 중 **"명시적 중복 생성"** 을 택한다.
기존 페이지는 **건드리지 않는다** — 덮어쓰지도, 지우지도, 보관 처리하지도 않는다.
`notion_syncs.page_id`는 새 페이지를 가리키게 갱신된다.

UI는 그 사실을 **누르기 전에** 말한다.

```text
이 Recording은 이미 Notion 페이지가 있습니다.
[새 페이지 만들기]   기존 페이지는 그대로 둡니다.
```

| 후보 | 판정 | 이유 |
| --- | --- | --- |
| 기존 페이지 내용 교체 (`replace_content`) | **거절** | 두 가지 이유다. **(가)** 그 페이지는 이미 **사용자의 Notion 문서**다. 사용자가 Notion에서 손댄 내용을 앱이 조용히 날린다 — 우리가 되돌릴 수 없는 파괴다. **(나)** `replace_content`가 PATCH의 유효한 type이라는 것은 VERIFIED지만 [E2 · §14.9.1] **그 요청 본문의 정확한 형태는 문서 인용으로 확인되지 않았다** [E4]. 확인된 형태는 `insert_content`뿐이다. 확인되지 않은 요청 형태 위에 **파괴적 동작**을 세우지 않는다 |
| 매번 조용히 새 페이지 | 거절 | 사용자가 버튼을 다시 눌렀을 때 워크스페이스에 페이지가 늘어나는데 그 사실을 모른다. Phase Goal이 금지한 "조용한 중복"이다 |
| **확인 뒤 새 페이지** | **채택** | 사용자가 무슨 일이 일어나는지 알고 누른다(예측 가능성 — Phase Goal B-8). 파괴가 없다. **확인된 요청 형태(`POST /v1/pages` · `insert_content`)만 쓴다.** 되돌리기도 싸다 — 나중에 in-place 갱신이 필요해지면 §8.5의 표에 한 줄을 더하는 일이며 `page_id`가 이미 그 근거 데이터다 |

### 8.4 결정 — 무엇을, 언제 영속화하는가

**secret이 아닌 상태만** 저장한다. 저장 위치는 이미 있는 `notion_syncs` 행이다.

migration **version 7 · 8**을 더한다 (다음 번호는 7이다 [E1]). 앞 migration은 고치지 않고
**열을 더하며**, `NOT NULL`도 `DEFAULT`도 두지 않는다 — version 3의 규약이다 [E1].

```sql
-- version 7: add_notion_settings
ALTER TABLE settings ADD COLUMN notion_parent_page_id TEXT;

-- version 8: add_notion_sync_progress
ALTER TABLE notion_syncs ADD COLUMN sent_chunks         INTEGER;
ALTER TABLE notion_syncs ADD COLUMN total_chunks        INTEGER;
ALTER TABLE notion_syncs ADD COLUMN content_fingerprint TEXT;
```

**secret 열은 만들지 않는다** (INV-7). `notion_parent_page_id`는 secret이 아니다 — 어디에
쓰는지일 뿐이며, `ai_base_url`이 secret이 아닌 것과 같다 [E1 · migration 6 주석].
기존 테스트 `no_migration_creates_a_place_to_put_a_secret`은 이 두 migration 뒤에도
그대로 통과해야 한다.

**쓰는 시점이 정책의 전부다.**

```text
1. 문서를 렌더한다 → content_fingerprint(sha256 hex) 계산 → chunk 계획(N개)
2. ★ 첫 요청을 보내기 전에 ★ 행을 쓴다:
      status = running · page_id = NULL · sent_chunks = 0
      total_chunks = N · content_fingerprint = … · error = NULL
3. POST /v1/pages 가 2xx + page id  →  **즉시** page_id 저장, sent_chunks = 1
4. PATCH 하나가 2xx 일 때마다      →  sent_chunks += 1
5. 마지막 chunk까지 성공            →  status = done · synced_at = 지금 · error = NULL
   중간에 실패                      →  status = failed · error = 사용자에게 보일 문장
```

- **2번이 앞에 있는 이유**: 요청을 보낸 뒤에 처음 기록하면, 그 사이에 앱이 죽었을 때
  "보낸 적 있는가"를 답할 자료가 아무것도 없다. 그때 다음 전송은 반드시 중복을 만든다.
- **3번이 '즉시'인 이유**: 페이지는 만들어졌는데 그 id를 잃으면, 그 페이지는 앱이 다시 찾을
  수 없는 고아가 되고 재시도는 새 페이지를 만든다. `page_id`는 **다음 요청을 보내기 전에**
  디스크에 있어야 한다.
- **4번이 성공 뒤인 이유**: 실패한 요청을 보낸 것으로 세면 재시도가 그 chunk를 건너뛴다 —
  **조용한 유실**이다. 성공에서만 올리면 최악의 경우 같은 chunk를 두 번 보내는데, 그것은
  §9의 `Retry-After` 경로에서 실제로 일어날 수 있는 유일한 중복이며 **눈에 보인다**
  (페이지에 같은 문단이 두 번 나온다). 유실보다 중복이 낫다 — 유실은 사용자가 알 수 없다.

**token은 여기에 없다.** `notion_syncs`에도 `settings`에도 secret 열은 없다 (§10.5).

### 8.5 그래서 `Send to Notion`을 누르면 무슨 일이 일어나는가

| 현재 상태 | 동작 |
| --- | --- |
| 행이 없다 / `none` | 새 페이지를 만든다 |
| `running` | 아무것도 하지 않는다. 진행 중임을 보인다 |
| `failed` · `page_id` 있음 · fingerprint 일치 | **같은 페이지에 `sent_chunks + 1`번째 chunk부터 이어 보낸다.** 확인 대화 없이 그대로 이어간다 — 중복이 생기지 않기 때문이다 |
| `failed` · `page_id` 있음 · fingerprint **불일치** | 이어 보내지 않는다. **문서가 바뀌었다는 사실을 말하고**, 확인을 받은 뒤 **새 페이지**를 만든다 |
| `failed` · `page_id` 없음 · 결과를 모름 (§7.3 · 타임아웃) | **페이지가 만들어졌을 수 있다**고 말하고, 확인을 받은 뒤 새 페이지를 만든다 |
| `done` | §8.3 — 확인을 받은 뒤 새 페이지를 만든다 |

**어떤 경우에도 local data는 바뀌지 않는다** (INV-3). Notion 전송이 건드리는 것은
`notion_syncs` 행과 `recordings.notion_status`뿐이며, 후자는 이미 있는
`update_recording_statuses`를 지난다 [E1 — 그 함수는 세 상태와 `updated_at`만 만진다].

---

## 9. (6) `429` · `529`와 `Retry-After`

### 9.1 확인된 계약 ([E2 · §14.9.1])

```text
429  "rate_limited"  →  Retry-After 헤더를 읽고 새 요청을 멈춘다.
                        문서 인용: "The header value is an integer number of seconds."
529  일시적 과부하    →  "Respect the Retry-After response header and try again later."
rate limit           →  연결당 평균 초당 3회, 짧은 버스트 허용.
                        workspace 한도는 요금제에 따르며 모든 연결이 공유한다.
```

### 9.2 결정

1. **`429`와 `529`에서 `Retry-After`를 정수 초로 읽고, 그만큼 멈춘 뒤 같은 chunk를 다시
   보낸다.** 값을 임의로 줄이지 않는다.
2. **실패한 요청은 보낸 것으로 세지 않는다** (§8.4-4). 그래서 재시도는 언제나 정확히 같은
   자리에서 다시 시작한다.
3. 한 chunk에 대한 자동 재시도는 **최대 3회**다 [A]. 그 뒤에는 멈추고 `status = failed`로
   남긴다 — 사용자가 다시 누를 수 있고, 그 재시도는 §8.5의 '이어 보내기'다.
4. 한 번의 대기는 **최대 120초**다 [A]. 서버가 그보다 긴 값을 주면 **기다리지 않고** 멈추며,
   `error`에 **얼마 뒤에 다시 시도하면 되는지**를 적는다. 앱이 몇 분씩 조용히 멈춰 있는 것은
   사용자에게 "멈춘 것"과 구분되지 않는다.
5. 헤더가 **없거나 정수로 읽히지 않으면** 앱이 고른 backoff를 쓴다 — **1초 · 2초 · 4초** [A].
   HTTP-date 형식은 **파싱하지 않는다** — 확인된 계약이 "정수 초"이므로 그 밖의 형식을
   지원한다고 가정하지 않는다.
6. 요청 사이에 **최소 350ms** 간격을 둔다 [A]. VERIFIED된 "평균 초당 3회"에서 나온 값이며
   (1/3초 ≈ 333ms), **그 자체가 API 한도는 아니다.** chunk 전송은 순차이므로 왕복 시간이
   대개 이보다 길고, 이 간격이 실제로 지연을 더하는 경우는 드물다.

### 9.3 나머지 상태 코드를 어떻게 보는가

이 절은 **`Retry-After` 규칙이 어디에 적용되는지 경계를 긋기 위한 것**이며, §13의 실패를
domain 타입으로 옮기는 전체 매핑은 구현 Task가 정한다.

| 응답 | 자동 재시도 | 뜻 |
| --- | --- | --- |
| `2xx` | — | 그 chunk는 반영됐다. `sent_chunks`를 올린다 |
| `401` · `403` (`unauthorized` · `restricted_resource`) | **하지 않는다** | token이나 권한의 문제다. 다시 보내도 같다 |
| `404` (`object_not_found`) | **하지 않는다** | 부모 페이지가 없거나 integration에 공유되지 않았다 |
| `400` (`validation_error` · `invalid_json`) | **하지 않는다** | 요청이 잘못됐다. 반복하면 같은 결과다 |
| `409` (`conflict_error`) | 하지 않는다 | 사용자 재시도로 넘긴다 |
| **`429`** | **한다 — §9.2** | rate limit |
| **`529`** | **한다 — §9.2** | 일시적 과부하 |
| `500` · `503` | 하지 않는다 | 재시도 가능한 실패로 표시하되 자동으로 반복하지 않는다. 언제 풀리는지 알 수 없고, `Retry-After`가 온다는 계약도 없다 |
| 응답 자체가 오지 않음 (타임아웃 · 연결 실패) | 하지 않는다 | **결과를 모른다.** 첫 요청이면 §8.5의 '결과를 모름'이다 |

**어떤 응답도 token을 담은 채로 기록되지 않는다** (§10.4).

---

## 10. (7) SecretStore 경계

### 10.1 결정 — `platform` 아래 trait 하나

```text
src-tauri/src/platform/
  app_data_dir.rs      (있음)
  clock.rs             (있음)
  microphone.rs        (있음)
  secret_store.rs      ← 이 Phase가 더한다
```

`platform`은 이미 "플랫폼 지식이 갇혀 있는 경계"로 규정돼 있고, 그 안의 셋은 전부
**바깥 세계를 읽는 자리**다 [E1]. OS 자격증명 저장소는 정확히 그런 자리다. 새 최상위 모듈을
만들지 않는다.

```rust
/// 앱이 보관해야 하는 secret의 **닫힌 목록**. 임의의 문자열 키를 받지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKey {
    NotionIntegrationToken,
}

/// secret 값. **Debug/Display가 내용을 출력하지 않는다.**
/// serde 파생을 붙이지 않는다 — 직렬화될 수 있으면 언젠가 직렬화된다.
pub struct Secret(String);

pub trait SecretStore: Send + Sync {
    fn get(&self, key: SecretKey) -> Result<Option<Secret>, Failure>;
    fn set(&self, key: SecretKey, secret: &Secret) -> Result<(), Failure>;
    fn delete(&self, key: SecretKey) -> Result<(), Failure>;
}
```

| 결정 | 이유 |
| --- | --- |
| **`SecretKey`가 닫힌 enum이다** | 문자열 키를 받으면 이 경계는 순식간에 "아무거나 담는 자격증명 저장소"가 된다. Cloud AI provider(Claude · Gemini · Groq)는 DEFERRED이며 [PRODUCT-SPEC §16], **언젠가 필요할지 모른다는 이유로 자리를 파 두지 않는다** (ADR-0008 §11.3과 같은 규칙). 필요해지면 변형을 하나 더한다 |
| **`Secret`이 별도 타입이다** | `String`이면 로그·에러·IPC 어디로든 갈 수 있다. `Debug`가 내용을 감추고 `Serialize`가 없으면, 새어 나가는 경로가 **규칙이 아니라 타입으로** 막힌다. `TransportError`에 문자열을 두지 않은 것과 같은 방법이다 [E1] |
| **`Send + Sync`** | 전송은 UI 밖 스레드에서 돈다. 기존 경계들과 같다 (`TranscriptionEngine` · `NoteAiProvider`) |
| **실패는 `Failure`다** | OS 오류가 그대로 위로 올라가면 플랫폼 지식이 경계 밖으로 샌다 (INV-10). 그리고 그 문자열에 계정/서비스 이름이 섞여 나올 수 있다 |

### 10.2 결정 — 구현 셋

```text
platform::secret_store
  ├─ OsSecretStore        macOS 자격증명 저장소 (Keychain).  ★ 자동 테스트가 실행하지 않는다
  │                       Windows도 같은 타입 뒤에서 OS 저장소를 쓴다 — 검증은 Phase 6
  ├─ 지원되지 않는 OS      명확한 Failure. 파일에 조용히 떨어뜨리지 않는다
  └─ testing::InMemorySecretStore   자동 테스트가 쓰는 유일한 구현
```

- **자동 테스트는 실제 자격증명 저장소를 건드리지 않는다.** 저장소에 선례가 있다 —
  실제 소켓을 여는 `ai/ollama/network.rs`는 **Gate가 컴파일만 하고 테스트는 실행하지
  않으며**, 테스트는 `testing::StubServer`를 쓴다 [E1]. `OsSecretStore`도 같은 자리에 둔다.
- **지원되지 않는 OS에서 파일로 대체하지 않는다.** "어딘가에는 저장된다"가 되면 INV-7이
  조용히 깨진다. 저장할 수 없으면 저장할 수 없다고 말한다.
- 항목 식별자는 결정론적으로 정한다 — 서비스 `molt-note`, 계정 `notion-integration-token`.
  사용자가 자기 Keychain에서 그 항목을 찾아 지울 수 있어야 하기 때문이다.

### 10.3 결정 — crate는 `keyring`을 1순위로 한다. **feature 구성은 확인하지 못했다**

| 항목 | 상태 |
| --- | --- |
| macOS Keychain과 Windows Credential Manager를 **하나의 API로** 다루는 crate로 `keyring`이 있다 | **[E3] 2차 출처** — 이 Run은 primary source를 열지 못했다 (§3.1) |
| `keyring` 3.x가 **플랫폼별 feature를 명시적으로 켜야** 실제 OS 저장소를 쓰고, 켜지 않으면 in-memory/mock 계열이 된다 | **[E4] UNVERIFIED** — 이 Run이 확인하지 못했다 |
| 그 feature의 **정확한 이름**과 현재 버전 | **[E4] UNVERIFIED** |

**그래서 이렇게 결정한다.**

1. **1순위는 `keyring`이다.** 근거: 이 trait이 필요로 하는 것이 정확히 그 crate의 모양
   (서비스/계정 키 하나에 문자열 하나)이고, macOS와 Windows를 **한 구현 파일**로 덮으면
   Phase 6에서 Windows 구현을 새로 쓰지 않아도 된다.
2. **feature 이름을 이 문서가 지어내지 않는다.** 구현 Task가 pin된 버전에서 확인하고,
   **확인한 버전과 feature 이름을 `Cargo.toml` 주석과 evidence에 그대로 적는다.**
3. **"켜졌다고 믿지 않는다."** mock으로 조용히 떨어지는 것이 이 crate의 알려진 실패
   양식이므로, 구현 Task는 **`Cargo.lock`에 macOS 자격증명 API를 쓰는 crate가 실제로
   들어왔는지**를 evidence로 남긴다 (§3.2가 `ureq`에 대해 같은 방식으로 확인한 것과 같다).
4. **대체 경로의 발동 조건을 미리 적는다** — "나중에 정한다"가 아니라 조건이다.

```text
조건: pin된 keyring 버전에서 macOS 자격증명 저장소를 쓰는 feature 구성을
      확인할 수 없거나, 켰는데도 Cargo.lock에 그 경로가 나타나지 않으면
동작: 같은 trait 뒤에서 macOS 전용 crate로 구현하고 (security-framework 계열),
      Windows 구현은 Phase 6의 자리로 남긴다.
      trait · 호출부 · 테스트는 그대로다 — 바뀌는 것은 파일 하나다.
```

**이 결정 구조가 요점이다.** crate 선택이 틀려도 무너지는 것은 `secret_store.rs`
한 파일이며, 틀렸다는 사실이 **컴파일과 `Cargo.lock`에서 드러난다.**

### 10.4 token이 지나가는 길과, 지나가지 않는 길

```text
사용자가 Settings에 붙여 넣는다
   → Tauri command (Rust)          토큰을 받는 유일한 자리
   → SecretStore::set              OS 자격증명 저장소
                                   ↑
   → Notion adapter가 필요할 때 get 해서 Authorization 헤더에만 넣는다
```

- **frontend는 token을 되돌려 받지 않는다.** 상태 조회 command는 **"설정돼 있는가"(bool)** 만
  돌려준다. 지우기는 `delete`를 부른다. 화면에 값을 다시 채워 보여 주지 않는다.
- **`Failure`의 `message`·`detail`에 token을 넣지 않는다.** `HttpRequest`의 헤더는 `Debug`로
  값을 출력하지 않으며(§5.5), OS 저장소 오류는 §10.1대로 `Failure`로 번역된다.
- **로그에도 evidence 파일에도 남기지 않는다.** 이 Phase의 어떤 Task도 실제 token을 evidence에
  적지 않는다 (`phase-prompt/05` Important Rules).

### 10.5 token을 두지 않는 자리 (전부 금지)

| 자리 | 왜 안 되는가 |
| --- | --- |
| **SQLite** (`settings` · `notion_syncs` · 그 밖의 모든 테이블) | INV-7. DB 파일은 평문이고 백업·동기화 폴더로 딸려 간다. migration 3~6이 전부 "여기서도 secret 열은 만들지 않는다"고 적어 두었고 [E1], 테스트 `no_migration_creates_a_place_to_put_a_secret`이 그것을 강제한다. **편의를 이유로 예외를 만들지 않는다** |
| **frontend 영속화** (`localStorage` · `sessionStorage` · 상태 저장) | INV-7. 지금 저장소에는 그 사용이 **하나도 없다** [E1]. 첫 사례를 token으로 만들지 않는다 |
| **소스 파일** (상수 · 테스트 fixture) | 커밋되는 순간 되돌릴 수 없다 |
| **커밋된 `.env`** | 같은 이유. 환경변수로 token을 읽는 경로 자체를 만들지 않는다 — 있으면 언젠가 쓰인다 |
| **로그 · 에러 메시지** | §10.4 |
| **evidence 파일** (`.loop/evidence/**`) | evidence는 커밋된다 |

**SQLite에 두는 Notion 관련 값은 secret이 아닌 것뿐이다** — `settings.notion_parent_page_id`와
`notion_syncs`의 전송 상태(§8.4).

### 10.6 이 경계를 지금 만들지만, 여기까지만 만든다

Cloud AI provider의 자격증명 · token 자동 갱신 · 여러 workspace · 키 회전은 **만들지
않는다.** `SecretKey`에 변형이 하나뿐인 것이 그 사실을 코드로 적은 것이다 (§10.1).

---

## 11. (8) `ureq`의 TLS 경로

### 11.1 지금 상태 — 확인된 사실 ([E1])

```toml
# src-tauri/Cargo.toml
ureq = { version = "3.4", default-features = false }
```

```text
Cargo.lock  ureq 3.4.0 의 의존성:  base64 · log · percent-encoding · ureq-proto · utf8-zero
Cargo.lock  전체:                  rustls · ring · native-tls · security-framework  → 하나도 없다
```

**그러므로 이 저장소는 지금 HTTPS를 열 수 없다.** Notion은 HTTPS다. 이것은 켜야 하는 것이지
"아마 켜져 있을 것"이 아니다 — `Cargo.toml` 주석도 "TLS는 Phase 5가 필요해질 때 켜는 것이
맞다"고 적어 두었다 [E1].

**upstream이 기본으로 무엇을 켜는지는 여기서 아무 의미가 없다.** `default-features = false`가
그것을 전부 끄기 때문이다.

### 11.2 결정

1. **`default-features = false`를 유지한다.** 기본값을 다시 켜면 압축·쿠키·프록시 같은
   필요 없는 것이 함께 들어온다. 필요한 것을 **하나씩 이름으로 켠다.**
2. **TLS feature를 명시적으로 켠다.** 어떤 feature인지는 §11.3의 순서로 고른다.
3. **인증서 검증을 끄지 않는다.** "invalid cert 허용" 류의 옵션은 만들지 않는다 —
   설정으로도, 개발용 플래그로도 두지 않는다. token이 실려 나가는 연결이다.
4. **켜졌는지를 믿음이 아니라 증거로 판정한다** (§11.4).

### 11.3 어떤 feature를 켜는가 — **이름은 UNVERIFIED다**

```text
[E4]  ureq 3.4.0 이 제공하는 TLS feature의 정확한 이름과 기본 구성 —
      이 Run은 확인하지 못했다 (§3.1: WebFetch · WebSearch 거부 · registry 접근 불가).
```

**그래서 이 문서는 feature 이름을 적지 않는다.** 지어낸 이름을 적으면 다음 사람이 그것을
확인된 사실로 읽는다. 대신 **고르는 순서와 그 이유**를 정한다.

| 순위 | 무엇 | 왜 |
| --- | --- | --- |
| **1** | 순수 Rust TLS 구현(rustls 계열)을 쓰는 feature | 사용자 기기에 아무것도 설치하지 않는다는 제품 규칙과 맞고 (PRODUCT-SPEC §14.4.2), macOS·Windows에서 같은 코드가 선다. 빌드 의존성이 늘지 않는다 |
| **2** | OS 기본 TLS를 쓰는 feature (native-tls 계열) | 1이 pin된 버전에서 불가능할 때. 시스템 라이브러리에 기대므로 플랫폼 차이가 생기지만, **연결이 서지 않는 것보다 낫다** |
| — | 인증서 검증을 끄는 어떤 구성 | **선택지가 아니다** |

루트 인증서를 어디서 얻는가(OS 신뢰 저장소 / 번들된 루트)도 1순위 안에서 함께 정해야 하며,
**둘 중 무엇을 쓰는지 구현 Task가 evidence에 적는다.** 이 문서는 그 feature 이름을 모른다.

### 11.4 틀렸을 때 조용히 넘어가지 않게 하는 것

TLS feature를 잘못 짚으면 **컴파일은 되고 런타임에 https 요청만 실패할 수 있다.** 그래서
구현 Task는 아래를 전부 남긴다.

```text
1. pin된 버전에서 유효한 feature 목록을 확인한 명령과 그 출력
   (예: 잘못된 feature 이름을 주었을 때 cargo가 뱉는 유효 목록 · cargo tree -f "{p} {f}")
2. Cargo.toml 의 최종 한 줄과 그것을 고른 이유 주석
3. ★ Cargo.lock 의 변화 ★ — TLS 구현 crate가 실제로 들어왔는가.
   §11.1의 "lock에 TLS crate가 하나도 없다"가 이 판정의 before 값이다.
4. cargo build 의 exit code
```

**3번이 핵심 판정이다.** feature를 켰다고 적어 두고 lock이 그대로면 켜지지 않은 것이다.
그 사실이 evidence 파일 하나로 드러난다.

---

## 12. 확인한 것 / 확인하지 못한 것

| 사실 | 등급 | 이 문서에서 무엇이 그것에 의존하는가 |
| --- | --- | --- |
| `POST /v1/pages`의 `markdown` body param | [E2 · §14.9.1] | §5.1 — 틀리면 블록 JSON 경로로 돌아가야 한다 (설계 전체가 바뀐다) |
| `PATCH /v1/pages/:page_id/markdown` · `insert_content` · `position:{type:"end"}` | [E2 · §14.9.1] | §5.1 · §6 — 이어붙이기가 안 되면 chunk 전략이 성립하지 않는다 |
| `GET /v1/users/me`로 token 유효성 확인 | [E2 · §14.9.1] | §5.1의 connection test |
| `Notion-Version: 2026-03-11` | [E2 · §14.9.2 — 두 번 확인] | §5.2 |
| `Authorization: Bearer <secret>` | [E2 · §14.9.1] | §5.2 · §10.4 |
| `properties.title` 생략 시 첫 `# h1`이 제목 | [E2 · §14.9.1] | §5.3 |
| `markdown`과 `children`/`content`를 함께 쓸 수 없다 | [E2 · §14.9.1] | §5.4 |
| 요청당 1000 block elements · 500KB overall | [E2 · §14.9.1] | §6.2의 예산이 그 **아래**에 있다 |
| `429`·`529`의 `Retry-After`는 **정수 초** | [E2 · §14.9.1] | §9.2 |
| 연결당 평균 초당 3회 | [E2 · §14.9.1] | §9.2-6의 350ms [A] |
| `allow_async` → `202` + `async_task` 폴링 | [E2 · §14.9.1] | §7 — 쓰지 않기로 한 근거 |
| **markdown 엔드포인트 전용 본문 크기 상한** | **[E4] UNVERIFIED — 값이 존재하는지조차 확인되지 않았다** | §6.2가 그것을 모른다는 전제로 보수적으로 골랐다. **750KB 같은 값을 사실로 적지 않았다** |
| `properties`와 `markdown`을 같은 요청에 함께 보낼 수 있는가 | **[E4] UNVERIFIED** | §5.3 — 그래서 `properties`를 보내지 않는 쪽을 택했다 |
| `replace_content`의 요청 본문 형태 | **[E4] UNVERIFIED** (type 이름 자체는 [E2]) | §8.3 — 그래서 in-place 교체를 택하지 않았다 |
| `insert_content`로 나눠 보낸 결과가 한 번에 보낸 것과 같은 블록 구조가 되는가 | **[E4] UNVERIFIED** | §6.3의 경고 — Human Review 항목이다 |
| `keyring`의 현재 버전과 플랫폼 feature 이름 | **[E4] UNVERIFIED** | §10.3 — 구현 Task가 확인하고, 실패 시 발동 조건이 §10.3-4에 있다 |
| `ureq` 3.4.0의 TLS feature 이름과 기본 구성 | **[E4] UNVERIFIED** | §11.3 — 구현 Task가 확인하고, §11.4가 켜졌는지를 판정한다 |
| 이 저장소의 `Cargo.lock`에 TLS crate가 하나도 없다 | **[E1] 이 Run이 직접 확인** | §11.1 |
| 이 저장소의 스키마·경로·HTTP 경계·파일명 선례 | **[E1] 이 Run이 직접 확인** | §3.2의 표 전체 |

---

## 13. 이 Phase의 범위에 넣지 않는 것

```text
Notion 데이터베이스(data source) 기반 저장     양방향 sync
Notion → Molt Note 역방향 import               자동/백그라운드 주기 sync
페이지 in-place 갱신(replace_content)          Notion 페이지 삭제·보관 처리
allow_async 폴링                               일괄 export · 일괄 sync
export 위치를 고르는 설정 · 저장 대화상자       Cloud AI provider 자격증명
token 자동 갱신 · 키 회전 · 다중 workspace      Windows 실기 검증 (Phase 6)
```

전부 "나중에 할 일"이 아니라 **이 Phase에서 만들지 않기로 한 것**이다. 그중 셋(in-place 갱신 ·
`allow_async` · export 위치 설정)은 §8.3 · §7 · §4.1에 **거절한 이유**가 적혀 있다.

---

## 14. Consequences

**얻는 것**

- Markdown 산출물이 **하나**다. 로컬 파일과 Notion 페이지가 같은 문자열에서 나오므로,
  Markdown 렌더러의 테스트가 곧 Notion 본문의 테스트다.
- 외부 사실이 틀렸을 때 **드러난다.** chunk 예산이 작으면 요청이 늘 뿐이고, 크면 서버가
  거절하며 그 실패가 사용자에게 보인다. TLS feature가 틀리면 `Cargo.lock`이 말한다.
  crate feature가 틀리면 같은 방식으로 드러난다.
- 재시도가 **중복도 유실도 만들지 않는다.** 요청 전에 상태를 적고 성공 뒤에만 진행도를
  올리는 두 규칙이 그것을 만든다 (§8.4).
- token이 앱의 저장소 밖에 있다. INV-7이 "그렇게 짜지 않았다"가 아니라 **"둘 자리가 없다"**
  로 유지된다 — DB에 열이 없고, `Secret`은 직렬화되지 않으며, frontend는 값을 받지 않는다.

**감수하는 것**

- **요청 수가 늘어난다.** 60KB 예산은 보수적이므로 1시간 transcript가 여러 요청이 된다.
  초당 3회 한도와 350ms 간격 안에서 순차로 나가므로 시간이 걸린다. 그 대신 어떤 요청도
  한도에 걸려 통째로 실패하지 않는다.
- **워크스페이스에 중복 페이지가 생길 수 있다.** `done`인 것을 다시 보내면 새 페이지다
  (§8.3). 사용자가 확인하고 누른 결과이며, 그 대가로 사용자가 Notion에서 편집한 내용이
  앱 때문에 사라지는 일은 없다.
- **파일이 쌓인다.** export를 반복하면 `-2` · `-3`이 늘어난다 (§4.3). 지우는 것은 사용자의
  일이며, 앱이 사용자 문서를 지우지 않는다.
- **UTC 날짜가 파일명에 들어간다.** 늦은 밤 녹음은 파일명 날짜가 하루 뒤일 수 있다 (§4.2).
  결정론을 그 편차보다 위에 두었다.
- **두 개의 미확인 crate 사실을 안은 채 Phase가 시작된다** (§10.3 · §11.3). 그래서 둘 다
  **하나의 파일과 하나의 Cargo.toml 줄**로 범위를 묶고, 확인 수단을 결정과 함께 적었다.

---

## 15. 구현 대조 — 실제로 만들어진 것 (2026-09-04 · TASK-054)

**이 절만 구현 결과를 적는다.** §1~§14는 결정이고 이 Task는 그것을 한 글자도 고치지 않았다
(머리말의 Status 줄만 §15의 존재를 가리키도록 바꿨다). 결정과 결과가 갈리는 자리는 전부
여기 아래에 있다.

> ⚠️ **이 절을 쓴 Run도 실제 Notion API를 한 번도 호출하지 않았고, 네트워크 접근이 없었다.**
> 여기 적은 값은 전부 **이 저장소의 파일**([E1])과 **앞선 Task의 evidence 파일**에서 왔다.
> 이 Run이 새로 확인한 외부 사실은 **하나도 없다** — §15.5의 UNVERIFIED 목록이 그 결과다.

### 15.1 여덟 결정이 어떻게 끝났는가

| | 결정 (§2) | 구현 결과 | 어디에 |
| --- | --- | --- | --- |
| **(1)** | `exports/` · `<날짜>-<슬러그>.md` · 덮어쓰지 않고 `-2`·`-3` | **그대로** | `platform/app_data_dir.rs`(`exports_dir` · `ensure_exports_dir`) · `export/filename.rs` · `export/file.rs`(`MAX_NAME_ATTEMPTS = 1_000`) |
| **(2)** | `POST /v1/pages` · `PATCH …/markdown` · `GET /v1/users/me` · `Notion-Version: 2026-03-11` | **그대로** | `notion/wire.rs`(`API_BASE_URL` · `NOTION_VERSION` · 세 경로 상수) · `notion/client.rs` |
| **(3)** | chunk 예산 **60,000 바이트 · 300 block unit** — 둘 다 **[A]** | **그대로. 값도 표시도 유지** | `notion/chunk.rs:62` · `:79` → §15.2.1 |
| **(4)** | `allow_async`를 **쓰지 않는다** | **쓰지 않았다.** 본문에 없음을 테스트가 강제한다 | `notion/wire.rs` · `notion/client.rs` → §15.2.2 |
| **(5)** | Recording 1 ↔ 페이지 1 · 이어 보내기 · 확인 뒤 새 페이지 | **그대로** (표 한 줄이 늘었고, 진행 중 갈래가 실패 값이 됐다 — D-2 · D-3) | `sync/run.rs::plan` → §15.4 |
| **(6)** | `Retry-After` 정수 초 · 재시도 3회 · 대기 상한 120초 · backoff 1·2·4초 · 간격 350ms | **그대로** (판정 자리가 하나 넓어졌다 — D-4) | `sync/pace.rs`(`MIN_REQUEST_INTERVAL` · `MAX_RETRIES_PER_CHUNK` · `MAX_WAIT` · `BACKOFF`) |
| **(7)** | SecretStore trait 하나 · 닫힌 `SecretKey` · `Secret` · crate는 `keyring` 1순위 (**feature UNVERIFIED**) | **`keyring` 3.6.3 · `apple-native` + `windows-native`.** 대체 경로는 발동하지 않았다 | `platform/secret_store.rs` · `Cargo.toml:75` → §15.2.4 |
| **(8)** | `default-features = false` 유지 · TLS feature 하나를 명시적으로 (**이름 UNVERIFIED**) | **`ureq` 3.4.0 · feature `rustls` · 루트는 `webpki-roots`(번들)** | `Cargo.toml:74` → §15.2.3 |

migration은 계획한 번호 그대로 들어갔다 — **version 7 `add_notion_settings`, version 8
`add_notion_sync_progress`** [E1 · `db/migrations.rs`]. `settings`에도 `notion_syncs`에도
secret 열은 생기지 않았고, `no_migration_creates_a_place_to_put_a_secret`이 그대로 돈다.

### 15.2 확정된 값 넷 — §6 · §7 · §10 · §11이 미뤄 둔 것

#### 15.2.1 chunk 예산 — 고른 값은 60,000 바이트 · 300 block unit이고, **앱이 고른 값이다 [A]**

```text
CHUNK_MAX_BYTES        60_000     notion/chunk.rs:62      [A] 앱이 고른 값
CHUNK_MAX_BLOCK_UNITS  300        notion/chunk.rs:79      [A] 앱이 고른 값
```

**§6.2가 정한 값이 그대로 구현됐고, [A] 표시도 코드 안에 그대로 있다** [E1] — 두 상수의 문서
주석이 "이 값은 이 앱이 고른 값이다. 확인된 Notion API 한도가 아니다"로 시작하고, 750KB 같은
값이 근거가 아니라는 사실까지 적혀 있다. **그 표시를 지우지 않는 것이 §6.2의 요구였고, 지우지
않았다.**

값이 VERIFIED 한도 아래에 있다는 것은 **테스트가 검사한다** [E1 · `chunk.rs`]:

```text
VERIFIED_REQUEST_LIMIT_BYTES = 500_000        (§14.9.1)
WORST_CASE_JSON_ESCAPE       = 6
CHUNK_MAX_BYTES * 6 < 500_000                 ← 상수를 늘리면 이 테스트가 먼저 깨진다
```

block unit의 정의도 §6.2 그대로다 — **비어 있지 않은 줄 수, 코드 펜스 블록은 통째로 1**.
나누는 자리(빈 줄 → 줄 경계 → 펜스 안은 절대 안 나눔 → 한 단위가 예산을 넘으면 **자르지 않고
보이는 실패**)와 무손실 성질(`join(chunks) == 원본`)도 그대로이며, 실패는 문서 내용을 담지
않는 `OversizedAtom`(줄 번호 · 바이트 · 예산 · 종류)으로 나온다.

#### 15.2.2 `allow_async` — 쓰지 않았다

- 요청 본문에 **`allow_async`가 없다.** `children` · `content` · `properties`와 함께 **본문에
  나타나지 않는 것이 테스트로 강제된다** [E1 · `notion/wire.rs` · `tests/notion_adapter.rs`].
- **폴링 경로를 만들지 않았다** — `status_url` · `poll_after_seconds` · async task 식별자를
  다루는 코드도, 그것을 저장할 자리도 없다.
- **§7.3이 정한 "결과를 모름"이 실제 동작이다** [E1 · `notion/client.rs`]: `create_page`는
  2xx를 받아도 **페이지 식별자를 읽지 못하면 성공으로 보지 않는다**. 그 실패는
  `NotionResponseUnusable`이고 **`retryable = false`** 라서 앱이 조용히 다시 보내지 않으며,
  `page_id`가 없는 채로 남은 행은 §8.5의 '결과를 모름'(`OutcomeUnknown`) 갈래로 들어간다 —
  사용자가 확인해야 새 페이지가 만들어진다. 전용 테스트가 `{"object":"async_task",…}` 응답을
  그 자리에 넣어 검사한다.

#### 15.2.3 `ureq`의 TLS — feature 이름은 **`rustls`**, 확인한 버전은 **3.4.0**

```toml
ureq = { version = "3.4", default-features = false, features = ["rustls"] }   # Cargo.toml:74
```

| 항목 | 값 | 등급 |
| --- | --- | --- |
| 켠 feature 이름 | **`rustls`** | [E1] — cargo가 이 요구를 해석했고, 없는 feature 이름은 하드 에러다 |
| 확인한 버전 | **`ureq` 3.4.0** (checksum `972d7902…`) | [E1 · `Cargo.lock`] |
| `default-features = false` | **유지했다** (§11.2-1) | [E1] |
| 루트 인증서의 출처 | **`webpki-roots` 1.0.9 — 번들된 루트.** OS 신뢰 저장소가 아니다 | [E1 · `Cargo.lock`] |
| 인증서 검증을 끄는 구성 | **없다.** 설정에도 개발용 플래그에도 없다 (§11.2-3) | [E1] |

**§11.4가 "핵심 판정"이라고 지목한 `Cargo.lock` 검사를 통과했다.** §11.1의 before는
"lock에 TLS crate가 하나도 없다"였고, after는 이것이다 [E1]:

```text
ureq 3.4.0 의 dependencies 에 rustls · rustls-pki-types · webpki-roots 가 들어왔다
rustls 0.23.43 · ring 0.17.14 · rustls-pki-types 1.15.1 · rustls-webpki 0.103.15 · webpki-roots 1.0.9
```

기록: `.loop/evidence/TASK-047/ureq-tls-verification.md` (빌드 산출물 `.rlib` 목록과 Gate
exit code 포함).

**대가는 알려져 있다** — 번들된 루트를 쓰므로 **사용자가 자기 기기에 직접 넣은 루트(사내
프록시 등)는 이 경로에서 신뢰되지 않는다.** 그런 상황이 실제로 보고되면 바뀌는 것은
`Cargo.toml` 한 줄이다 (§11.4가 노린 성질 그대로다).

#### 15.2.4 자격증명 crate — `keyring` 3.6.3 · `apple-native` + `windows-native`

```toml
keyring = { version = "3", default-features = false, features = ["apple-native", "windows-native"] }   # Cargo.toml:75
```

| 항목 | 값 | 등급 |
| --- | --- | --- |
| crate와 해석된 버전 | **`keyring` 3.6.3** (checksum `eebcc3af…`) | [E1 · `Cargo.lock`] |
| 켠 플랫폼 feature | **`apple-native` · `windows-native`** — 두 이름이 pin된 버전에 실재한다 | [E1] — cargo가 해석에 성공했다 |
| `default-features = false` | **유지했다** | [E1] |
| lock이 실제로 들여온 것 | `security-framework` 2.11.1 · 3.7.0 · `security-framework-sys` 2.17.0 (macOS Security framework) · `windows-sys` 0.60.2 | [E1 · `Cargo.lock`] · 빌드 로그가 컴파일을 보인다 |
| §10.3-4의 대체 경로 | **발동하지 않았다** — 발동 조건("켰는데도 lock에 그 경로가 나타나지 않으면")이 충족되지 않았다 | [E1] |
| Keychain 항목 식별자 | 서비스 **`molt-note`** · 계정 **`notion-integration-token`** (§10.2 그대로) | [E1 · `platform/secret_store.rs`] |

기록: `.loop/evidence/TASK-046/keyring-verification.md`.

**"켜졌다고 믿지 않는다"(§10.3-3)가 이 판정의 전부다** — 확인한 것은 "이 두 이름이 실재하고,
켰더니 플랫폼 자격증명 API가 실제로 들어와 컴파일됐다"까지이며, **macOS에서 실제로 저장·조회·
삭제가 되는지는 여전히 확인되지 않았다** (§15.5).

### 15.3 계획과 달라진 결정과 그 이유

다섯 가지가 달라졌다. **결정 절(§4~§11)을 고쳐서 맞춘 것이 아니라, 달라진 것을 여기 적는다.**

#### D-1. HTTP 경계를 **Phase 4의 것을 넓히지 않고** Notion adapter 안에 따로 두었다

```text
계획 (§5.5)   HttpMethod 에 Patch · HttpRequest 에 headers · HttpResponse 에
              retry_after_seconds 를 더한다  ← 어느 파일인지는 적지 않았지만, 문맥은
              지금 있는 경계(ai/ollama/http.rs)였다
구현          같은 셋을 **notion/http.rs 에 새로 두었다.**
              ai/ollama/http.rs 는 이 Phase에서 한 줄도 바뀌지 않았다 [E1]
```

**이유:** 지금 있는 같은 모양의 경계는 **AI 벤더 adapter 디렉터리 안에** 있다. Notion adapter가
그것을 쓰면 두 벤더 adapter가 서로를 알게 되고, **한쪽 벤더 사정으로 넓힌 타입이 다른 쪽의
계약이 된다** — INV-9가 막으려는 결합이 정확히 그것이다. 넓힌 셋을 Notion 디렉터리 안에만
두면 Phase 4의 파일이 이 Phase 때문에 바뀌지 않는다.

**대가:** 이름이 같은 타입이 두 벌 있다. 그 대가를 받아들인 이유는 위와 같고, 두 벌 모두
같은 설계 규칙(값을 담고도 `Debug`로 내지 않는다 · `TransportError`에 문자열이 없다)을
따르므로 읽는 사람이 다른 규칙을 배우지 않는다.

#### D-2. `pending`도 `running`과 같이 **거절**한다 — §8.5의 표에 없던 한 줄

§8.5의 표는 `running`만 적었다. 스키마는 `pending`도 허용하므로 [E1 · migration 2의 CHECK],
구현은 **둘 다 "이미 보내는 중"으로 본다** [E1 · `sync/run.rs::plan`].

**이유:** 표에 없는 상태를 만나면 남는 갈래는 "새 페이지를 만든다"뿐이고, 그것이야말로
이 §가 막으려던 **조용한 중복**이다. 모르는 상태에서는 아무것도 만들지 않는 쪽이 맞다.

#### D-3. `running`에서 "아무것도 하지 않는다"가 **실패 값으로 돌아온다**

§8.5는 "아무것도 하지 않는다. 진행 중임을 보인다"였다. 구현은 `Failure`(`InvalidInput` ·
`retryable = true` · detail `status=running · sentChunks=n/N`)를 돌려준다 [E1].

**이유:** 이 경로에는 화면에 그릴 값이 따로 없다 — `send`가 아무 일도 하지 않고 성공을
돌려주면, 화면은 **버튼이 먹혔는지 아닌지를 구분할 수 없다.** 실패 값 하나면 "지금 보내는
중"이라는 문장과 진행도가 그대로 사용자에게 간다. **부작용은 없다** — 저장소도 Notion도
건드리지 않는다는 §8.5의 뜻은 그대로다.

#### D-4. `Retry-After` 경로의 판정 자리가 하나 넓다 — 본문의 `rate_limited`도 같이 본다

§9.2는 **`429`와 `529`**를 적었다. 구현은 거기에 더해 **본문의 오류 코드가 `rate_limited`이면
상태 코드와 무관하게** 같은 대기 경로로 보낸다 [E1 · `notion/client.rs::status_failure`].

**이유:** 서버가 "속도 제한"이라고 본문으로 말했는데 상태 코드가 우리가 적어 둔 둘이 아니면,
남는 갈래는 "재시도하지 않는 실패"다. 그러면 **기다리면 풀릴 일이 사용자 눈에 영구 실패로
보인다.** 넓힌 것은 **판정 자리이고 값이 아니다** — 기다릴 시간은 여전히 `Retry-After`의
정수 초에서만 오고, 없으면 앱이 고른 backoff이며 (§9.2-5), HTTP-date는 파싱하지 않는다.

#### D-5. 지문 계산에 `sha2` crate를 더했다

§8.4는 `content_fingerprint`를 **sha256 hex**로 정했지만 **crate를 정하지 않았다.**
구현은 `sha2 = "0.10"`(lock 0.10.9)을 쓴다 [E1 · `Cargo.toml:84`] — 이미 이 저장소의
`Cargo.lock`에 전이 의존성으로 들어와 있던 crate이고, 쓰는 자리는
`sync::run::content_fingerprint` 하나다. **자격증명을 다루지 않는다** — 해시가 걸리는 값은
사용자에게 이미 보이는 markdown 문서 하나다.

### 15.4 중복 sync 정책 — 최종 문구와 구현 대조 (PRODUCT-SPEC §10 · 이 문서 §8)

**최종 문구다. 이 문단이 이 제품의 중복 sync 정책이다.**

> **Recording 하나에 Notion 페이지 하나다.**
> 끝나지 않은 전송을 다시 시작하면 **새 페이지를 만들지 않고, 이미 만들어진 같은 페이지에
> 아직 보내지 않은 조각부터 이어 보낸다** — 그때 나눈 문서와 지금 문서가 같을 때만 그렇다.
> 이어 보낼 수 없는 상태(이미 끝난 전송 · 문서가 바뀐 경우 · 페이지가 만들어졌는지 모르는
> 경우)에서 새 페이지를 만드는 일은 **사용자가 그 사실을 읽고 확인한 뒤에만** 일어난다.
> **앱이 스스로 중복 페이지를 만들지 않고, 이미 Notion에 있는 것을 고치거나 지우지도 않는다.**
> 어떤 경우에도 local data는 바뀌지 않는다.

구현이 이 문구와 일치하는가 — `sync::run::plan`이 상태 하나하나에 무엇을 하는지 [E1]:

| 저장된 상태 | §8.5의 정책 | 구현 (`sync/run.rs::plan`) | 일치 |
| --- | --- | --- | --- |
| 행이 없다 · `none` | 새 페이지 | `Plan::CreatePage` — 확인을 묻지 않는다 | ✅ |
| `pending` | (표에 없었다) | 거절 — "이미 보내고 있다" | **D-2** |
| `running` | 아무것도 하지 않고 진행 중임을 보인다 | 거절 값으로 진행 중임과 진행도를 알린다 | **D-3** (부작용 없음은 같다) |
| `failed` + `page_id` 있음 + 지문 일치 + `sent_chunks` 있음 | **같은 페이지에 이어 보낸다.** 확인 대화 없이 | `Plan::Resume { page, sent }` — 확인을 묻지 않는다 | ✅ |
| `failed` + `page_id` 있음 + 지문 불일치 | 문서가 바뀌었다고 말하고, 확인 뒤 새 페이지 | `ConfirmBecause::DocumentChanged` → 확인 없으면 실패, `Confirmation::NewPage`면 새 페이지 | ✅ |
| `failed` + `page_id` 없음 | 결과를 모른다고 말하고, 확인 뒤 새 페이지 | `ConfirmBecause::OutcomeUnknown` → 같음 | ✅ |
| `done` | 확인 뒤 새 페이지. **기존 페이지는 건드리지 않는다** | `ConfirmBecause::AlreadySent` → 같음 | ✅ |

그리고 이 정책이 서 있으려면 참이어야 했던 것들 [E1]:

| 요구 | 구현 |
| --- | --- |
| Recording 하나당 행 하나 | `notion_syncs.recording_id`가 PRIMARY KEY (스키마가 강제한다) |
| **요청을 보내기 전에** 상태를 적는다 (§8.4-2) | `status=running` · `total_chunks` · `content_fingerprint` · `sent_chunks=0`을 첫 요청 전에 쓴다 |
| 페이지 식별자를 **즉시** 적는다 (§8.4-3) | `POST /v1/pages` 성공 직후 `page_id`를 쓰고 다음 요청으로 간다 |
| **성공한 요청만** 센다 (§8.4-4) | `sent_chunks`는 2xx에서만 오른다 — 실패한 chunk는 재시도가 같은 자리에서 다시 시작한다 |
| 기존 페이지를 고치거나 지우지 않는다 | adapter에 `replace_content`도 삭제·보관 경로도 **없다** (`notion/client.rs`는 `create_page`와 `append_markdown`만 연다) |
| 부분 전송이 사용자에게 드러난다 | `sentChunks` / `totalChunks`가 IPC로 나가고 화면이 "N of M parts…"로 적는다 |
| 확인 없이는 새 페이지가 없다 | 새 페이지를 만드는 갈래는 `Confirmation::NewPage`를 실은 **별도의 버튼** 하나뿐이다 (`Create a new Notion page`) |
| local data는 바뀌지 않는다 (INV-3) | 전송이 쓰는 것은 `notion_syncs` 행과 `recordings.notion_status`·`updated_at`뿐이다 |

**⚠️ 이 표는 코드를 읽어 대조한 것이다. 실제 Notion 워크스페이스에서 중복이 생기지 않는지는
여기서 판정되지 않는다** — 그것은 `docs/PHASE-5-NOTION-SMOKE-TEST.md`의 절차다.

### 15.5 여전히 UNVERIFIED인 것 — 구현이 끝나도 확인되지 않았다

**아래 항목은 §12의 등급이 그대로다. 구현했다는 사실이 이것들을 확인해 주지 않는다.**

| 사실 | 등급 | 무엇이 이것에 기대고 있는가 | 누가 판정할 수 있는가 |
| --- | --- | --- | --- |
| **markdown 엔드포인트 전용 본문 크기 상한** — **그런 값이 따로 있는지조차 확인되지 않았다** | **[E4] UNVERIFIED** | §6.2의 60,000 [A]. 그 값은 **VERIFIED된 일반 한도(500KB) 아래에서 앱이 고른 것**이며 markdown 전용 한도에서 유도되지 않았다. **웹에서 보이는 750KB 같은 값은 이 문서에도 코드에도 사실로 적히지 않았다** | primary source 확인. 서버가 우리 예산 안의 요청을 거절하면(413 · `validation_error`) 상수를 줄이라는 신호이며 실패는 사용자에게 그대로 보인다 (§6.4) |
| `insert_content`로 나눠 보낸 결과가 한 번에 보낸 것과 **같은 블록 구조**가 되는가 | **[E4] UNVERIFIED** | §6.3의 경고 | **Human Review** — 긴 transcript가 Notion에서 온전한가 (smoke test §6) |
| `properties`와 `markdown`을 같은 요청에 함께 보낼 수 있는가 | **[E4] UNVERIFIED** | §5.3 — 그래서 `properties`를 보내지 않는 쪽을 택했고, 구현도 보내지 않는다 | primary source 확인 |
| `replace_content`의 요청 본문 형태 | **[E4] UNVERIFIED** (type 이름 자체는 [E2]) | §8.3 — 그래서 in-place 교체를 만들지 않았다 | primary source 확인 |
| `ureq` 3.4.0이 제공하는 **feature 전체 목록** | **[E4] UNVERIFIED** | §11.3. 확인한 것은 "`rustls`가 실재하고 켜졌다"이지 "유일하거나 최선이다"가 아니다 | registry/문서 접근이 있는 Run |
| `keyring` 3.6.3의 **feature 전체 목록**과 `apple-native`가 여는 Keychain 항목 종류 | **[E4] UNVERIFIED** | §10.3. 확인한 것은 "두 이름이 실재하고 플랫폼 API가 들어왔다"까지다 | 같음 |
| **macOS에서 실제로 저장·조회·삭제가 되는가** | **[E4] UNVERIFIED** | §10.2 — 자동 테스트는 실제 자격증명 저장소를 건드리지 않는다 (메모리 test double만 쓴다) | **Human Review** (smoke test §3) |
| **실제 `https://api.notion.com`으로 TLS 핸드셰이크가 서는가** | **[E4] UNVERIFIED** | §11. 자동 테스트는 실제 Notion에 요청하지 않는다 | **Human Review** (smoke test §4) |
| `ureq` 3.4.0의 MSRV | **[E4] UNVERIFIED** | TASK-036 이후 그대로 | — |

**그러므로 이 Phase의 자동 검증이 통과했다는 사실은 "Notion 연동이 실제로 동작한다"는 뜻이
아니다.** 실제 워크스페이스로 판정해야 하는 것은 `docs/PHASE-5-NOTION-SMOKE-TEST.md`에 절차로
있고, 그 문서의 실행 기록이 비어 있는 동안 **Phase 5를 "실제 Notion 전송이 검증됐다"고
표현하지 않는다** (PHASE-3 smoke test 문서와 같은 규칙이다).
