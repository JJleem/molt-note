# Phase 5 — Notion & Markdown Export

Implement Phase 5 of `docs/PRODUCT-SPEC.md`.

## Goal

기록을 Molt Note 밖으로 꺼낼 수 있게 만든다.

```text
AI Note              → Notion 페이지
AI Note / Transcript → Markdown 파일
```

이 Phase의 성공 기준:

> Recording Detail에서 `Send to Notion`을 누르면 실제 Notion 페이지가 생기고,
> `Export Markdown`을 누르면 로컬에 Markdown 파일이 생긴다.
> 둘 중 무엇이 실패해도 local data는 온전하다.
> **그리고 AI Note가 하나도 없는 Recording도 export와 전송이 가능하다 (INV-8).**

이 Phase의 두 renderer는 **Phase 4가 확정한 provider 중립 Structured Note**(§9.3)를
소비한다. 특정 AI 벤더의 응답 형태를 소비하지 않는다 (INV-9).

```text
Structured Note ──→ Markdown renderer
                └─→ Notion renderer
```

## ⚠️ Preconditions — 2026-09-04 운영자 재확인 · **Human Review에서 승인됨**

이 Phase를 계획하면서 §14.9의 외부 사실을 primary source에서 재확인했다.
결과는 `docs/PRODUCT-SPEC.md` **§14.9.1 · §14.9.2**에 있으며, **아래 항목이 이 문서의 원문과
어긋난다. §14.9.1 · §14.9.2와 이 절이 우선한다.**

```text
Human Review 결정 (2026-09-04)

P-1  Markdown Content API 해석      APPROVED
P-2  SecretStore 해석                APPROVED
P-3  Task 응집도 (Detail export)     지시됨
P-4  TLS                             지시됨
P-5  재시도 / 멱등성                 지시됨
```

### P-1. `markdown` 직접 전송이 VERIFIED가 됐다 — **APPROVED** (아래 6·7번 항목에 영향)

Notion은 블록 JSON 대신 **`markdown` 문자열을 직접 받는다** —
`POST /v1/pages`의 `markdown` body param, `PATCH /v1/pages/:page_id/markdown`으로 이어붙이기.
아래 **7번 항목이 요구한 확인이 끝났고, 그 결과가 "사실"이다.**

따라서 아래 **6번 항목의 "2000자 청킹 · 100블록 배치"는 블록 JSON 경로의 요구사항이며,
그 경로를 쓰지 않으므로 그 형태로는 구현하지 않는다.**

**그러나 6번이 지키려던 것은 그대로 필수다:**

```text
1시간 transcript가 잘리거나 조용히 유실되어서는 안 된다.
```

한 요청에 담기지 않는 분량은 여전히 존재한다 (요청당 1000 블록 / 500KB · 페이지당 20,000 블록 ·
연결당 평균 초당 3회). 그러므로 **markdown 문자열을 안전한 경계에서 나누어 순차 전송**하고,
`allow_async` 사용 여부를 결정하며, **어떤 경우에도 조용히 자르지 않는다.**

#### 나누는 기준은 **markdown API의 실제 요청 계약**이다

옛 2000자 rich text 규칙에서 chunk 크기를 유도하지 않는다. 그것은 블록 JSON 경로의 규칙이다.

```text
VERIFIED    일반 요청 한도 — 요청당 1000 block elements · 500KB overall
UNVERIFIED  markdown 엔드포인트 **전용** 상한 — 그런 값이 따로 있는지조차 확인되지 않았다
```

⚠️ **markdown 전용 상수를 외부 사실로 지어내지 않는다.** 특히 750KB 같은 값은 primary
source에서 확인되지 않았으므로 사실로 적지 않는다. ADR-0009는 **VERIFIED된 일반 한도(500KB)
아래에서 이 앱이 고른 보수적인 내부 chunk 예산**을 정할 수 있으며(JSON 이스케이프·요청
부가분을 감안한다), 그것이 **확인된 API 한도가 아니라 이 앱이 고른 값**임을 코드와 문서에
명시한다.

판정 기준은 상수의 크기가 아니라 이것이다:

```text
전체 문서가 · 순서대로 · 무손실로 전송되거나, 실패가 그 사실을 드러내고 재시도된다.
```

### P-2. Phase 4는 secret 보관 수단을 만들지 않았다 — **APPROVED** (아래 10번 항목에 영향)

아래 10번은 "Phase 4가 provider 설정 보관에 대해 정한 방식과 일관되게 처리한다.
**새로운 방식을 따로 만들지 않는다**"고 적고 있다. 그러나 **Phase 4는 secret을 하나도
보관하지 않았다** — 로컬 Ollama가 token을 요구하지 않아서, 저장한 것은 `ai_provider` ·
`ai_base_url` · `ai_model` 뿐이고 **secret 열 자체를 만들지 않았다** (INV-7 ·
`no_migration_creates_a_place_to_put_a_secret`).

Notion integration token은 **진짜 secret이다.** 그러므로 10번을 다음과 같이 읽는다.

```text
일관되게 갈 것    호출 주체(Rust backend) · adapter 경계 · 실패를 domain 타입으로 변환
새로 만들 것      secret 보관 경계 하나 (OS 자격증명 저장소)
만들지 않을 것    SQLite의 secret 열 · frontend 보관 · 미래 Cloud AI 자격증명 인프라
```

원하는 형태:

```text
platform/
└─ SecretStore
   ├─ macOS 보안 저장소 구현
   └─ Windows 호환 구현 경계
```

crate(예: `keyring`)는 **ADR-0009가 현재 플랫폼 feature 구성을 확인한 뒤** 고른다.
자동 테스트는 **메모리 test double**로 지나며 실제 자격증명 저장소를 건드리지 않는다.

token을 두지 않는 자리 (전부 금지):

```text
SQLite · frontend 상태 영속화 · 소스 파일 · Git에 커밋된 .env · 로그 · evidence 파일
```

SQLite에는 **secret이 아닌 Notion 설정**(destination / page identifier 등)만 둔다.

**편의를 이유로 token을 SQLite에 넣지 않는다.** Product Spec은 SQLite를 안전한 secret
저장소로 규정한 적이 없다. 이것을 Claude · Gemini · Groq를 위한 추측성 자격증명 인프라로
일반화하지 않는다 (선입금 금지). 플랫폼 지식은 그 경계 안에만 둔다 (INV-10 · §3.1).

### P-3. Task 응집도 — Detail의 Export 버튼은 독립 Task가 아니다

앞선 Plan(`PLAN-20260904T024053Z`, **승인되지 않았고 실행되지 않았다**)은
`Recording Detail의 Export Markdown` 버튼을 독립 Task로 두었다. **버튼 하나와 Detail 통합은
의미 있는 아키텍처 경계도 검증 경계도 아니다.**

그 작업과 **acceptance criteria를 응집적인 UI 통합 Task 안으로 옮긴다.**

약화하면 안 되는 것 (반드시 테스트로 남는다):

```text
AI provider 없이 Markdown export가 된다
AINote 없이 Markdown export가 된다
export 실패가 원본 데이터를 훼손하지 않는다
```

나머지 Task 경계는 그대로 두어도 된다 — 다만 Planner가 비슷한 저가치 분할을 더 발견하면
합친다. **Task 개수 자체는 요구사항이 아니다.**

### P-4. TLS — 지금 이 저장소에는 없다

`src-tauri/Cargo.toml`의 `ureq`는 **`default-features = false`** 다. upstream ureq가 보통
`rustls`를 기본으로 켠다는 이유로 **TLS가 있다고 가정하지 않는다.** Notion은 HTTPS다.

실제 현재 `ureq` feature 구성을 확인하고 **지원되는 TLS 경로를 명시적으로 켜고 설정한다.**
확인한 버전과 feature 이름을 기록한다.

### P-5. 재시도 / 멱등성

V1 방향을 유지한다:

```text
Recording 하나  ↔  Notion 페이지 하나       (실무적으로 가능한 범위에서)
```

- **부분 성공 뒤의 재시도가 조용히 중복 페이지를 만들어서는 안 된다.**
- 안전하게 이어가거나 다시 시도할 수 있을 만큼의 **secret이 아닌 상태**를 영속화한다.
- rate limit 응답을 만나면 **`Retry-After`를 API 계약대로 존중한다** — `429`와 `529` 모두
  `Retry-After`를 주며 **값은 정수 초**다 (`PRODUCT-SPEC` §14.9.1).

만들지 않는다:

```text
양방향 sync · 백그라운드 연속 sync · 충돌 해결 엔진 · Notion → Molt Note 역방향 import
```

---

## Why This Phase Exists

§2의 원칙 중 "외부 서비스에 종속되지 않는다"가 실제로 성립하려면 **나가는 문**이 있어야 한다.
Markdown export는 Notion·NotebookLM·Obsidian 어디서든 쓸 수 있는 형태를 만들고,
Notion 연동은 실사용 워크플로를 완성한다.

Markdown이 Notion보다 먼저다 — Markdown은 외부 의존이 없고,
§14.6에 따르면 Notion 전송이 Markdown을 그대로 재사용할 수 있는 가능성이 있기 때문이다.

## Required Outcome

### A. Markdown Export

1. **Recording 하나를 Markdown 파일로 export**할 수 있다. §11의 구조를 따른다.

   ```markdown
   # 3DGS Study #04

   Date: 2026-09-01
   Duration: 52:31

   ## Overview
   ...

   ## Transcript

   ### 00:00:03
   ...
   ```

2. **파일명이 결정론적이고 안전**하다 — `exports/2026-09-01-3dgs-study-04.md` 형태.
   제목에 슬래시·콜론·이모지·개행이 있어도 안전한 파일명이 나와야 한다.
   같은 이름이 이미 있을 때의 정책을 정하고 기록한다.

3. **AI Note와 Transcript가 모두 포함**될 수 있다.
   **AI Note가 없는 Recording도 export 가능해야 하며, 그 결과가 유효한 문서여야 한다** —
   Transcript와 메타데이터만으로도 읽을 수 있는 Markdown이 나온다 (INV-8).
   이것은 선택적 편의가 아니라 §17.1의 core 성공 기준이다.

4. **Markdown 생성이 순수 함수로 테스트**된다 — Recording + Transcript + (선택적) AINote를 넣으면
   결정론적인 문자열이 나온다. 파일 쓰기와 분리한다 (§18).
   **AINote가 없는 입력에 대한 테스트가 반드시 포함된다** (INV-8).

### B. Notion Sync

5. **AI Note(또는 Transcript)를 Notion 페이지로 전송**할 수 있다. §10의 구조를 따른다.
   **AI Note가 없으면 Transcript만으로 전송 가능해야 한다** (INV-8).

6. **§14.9의 한도를 실제로 다룬다.** 이것은 선택이 아니라 필수다.
   ⚠️ **P-1을 먼저 읽는다** — 아래 세 줄은 블록 JSON 경로의 형태이며,
   `markdown` 경로가 VERIFIED가 되어 그 형태로는 구현하지 않는다.
   **지켜야 하는 것은 "잘리거나 조용히 유실되지 않는다"이다.** —
   1시간 transcript는 반드시 한도에 걸린다.
   - 단일 `text.content` **2000자** 상한
   - 요청당 `children` 블록 **100개** 상한
   - 초과분은 `PATCH /v1/blocks/{block_id}/children` **순차 반복** (append에는 배치가 없다)

   **긴 transcript가 잘리거나 조용히 유실되어서는 안 된다.**

7. ✅ **완료됨 (2026-09-04 · P-1).** Notion이 `markdown` 문자열을 직접 받는 것이
   **VERIFIED**다. 그러므로 A의 Markdown 산출물을 재사용한다. 블록 JSON을 직접 만들지 않는다.
   **이 Task를 다시 수행하지 않는다** — 근거는 `PRODUCT-SPEC.md` §14.9.1이다.
   다만 §14.9.1이 `UNVERIFIED`로 남긴 항목(markdown 요청 본문 크기 상한)은 여전히 미확인이다.

8. **중복 sync 정책을 결정하고 문서화한다** (§10) — 기존 페이지 업데이트인가,
   명시적 중복 생성인가. `NotionSync.pageId`가 그 근거 데이터다.
   사용자가 같은 Recording을 두 번 보냈을 때 무슨 일이 일어나는지 UI에서 예측 가능해야 한다.

9. **NotionSync가 §7 모델대로 저장**된다 — `recordingId` · `pageId` · `syncedAt` ·
   `status` · `error`. 상태가 Recordings 목록과 Detail에 보인다.

10. **integration token이 안전하게 다뤄진다 (INV-7)** — ⚠️ **P-2를 먼저 읽는다.**
    Phase 4는 secret 보관 수단을 만들지 않았으므로 "일관되게 처리한다"가 가리킬 기존 수단이
    없다. **OS 자격증명 저장소를 쓰는 최소 경계 하나**를 만들고, token은 SQLite에도
    frontend에도 두지 않는다. 호출 주체는 Phase 4와 같이 **Rust backend**다 (ADR-0008 §5).

11. **connection test**가 동작한다 (§5-D) — 토큰과 destination이 유효한지 확인할 수 있다.

12. **§13의 실패가 제품 상태로 다뤄진다** — Notion authentication failure · sync failure ·
    네트워크 없음 · 권한 없는 destination.
    **어떤 실패도 local data에 영향을 주지 않는다 (INV-3).** 재시도가 가능해야 한다.
    부분 전송 후 실패한 경우 사용자가 그 사실을 알 수 있어야 한다.

13. **전송되는 데이터가 UI에 드러난다 (INV-5).** **audio는 전송하지 않는다 (INV-6).**

14. **자동 테스트**: Markdown 생성, 파일명 정규화, Notion payload 생성,
    **긴 문서 분할과 무손실 재조립**(⚠️ P-1 — "2000자 청킹과 100블록 배치 분할"이 아니다)이
    **실제 Notion API 호출 없이** 테스트된다 (§18). 실제 OS 자격증명 저장소도 건드리지 않는다.

15. build · lint · test Gate가 전부 통과한다.

## Important Rules

- **NotebookLM 자동 연동을 만들지 않는다** (§15). Markdown interoperability만 제공한다.
- Markdown은 Notion 전용 포맷이 아니라 **어디서든 쓸 수 있는 형태**여야 한다 (§11).
- §14.9의 값은 2026-09-01 기준이며, **§14.9.1(2026-09-04) · §14.9.2(2026-09-04 재확인)가
  최신이다.** `Notion-Version` 헤더 값은 **`2026-03-11`** 이며 그 근거는 §14.9.2에 있다.
  **조사 날짜를 API 버전으로 쓰지 않는다** — `2026-09-01` · `2026-09-04` 같은 값을 헤더로
  보내는 것은 오류다. `2026-07-28`은 MCP 프로토콜 버전이지 API 버전이 아니다.
- 자동 테스트가 실제 Notion 워크스페이스를 오염시키지 않는다.
- token을 로그·에러 메시지·evidence 파일에 남기지 않는다.
- **Git commit / push는 이 Phase의 작업이 아니다.** Phase commit은 Phase가 완료되고
  검증된 뒤 운영자가 한다 (`docs/GIT-WORKFLOW.md`). Worker가 commit하면 HEAD가 바뀌어
  Gate·Verifier가 묶여 있는 subject fingerprint가 실행 도중에 흔들린다.
  저장소에 파일을 만들고 고치는 것까지가 Task의 일이다.

## Out of Scope

- NotebookLM API 연동 — 제품 non-goal (§15)
- Notion 데이터베이스(data source) 기반 구조화 저장 — 부모 페이지 밑 페이지 생성으로 충분하다 (§14.9)
- Notion → Molt Note 역방향 동기화
- 자동 주기 sync · 백그라운드 sync (DEFERRED, §16)
- PDF · DOCX 등 다른 export 포맷
- 일괄 export / 일괄 sync (DEFERRED, §16)
- 오디오 파일 업로드 (INV-6 위반)
- Windows 검증 (Phase 6)

## Verification Boundary

- Recording 하나가 실제 Markdown 파일로 export되고, 그 파일이 §11의 구조를 가진다.
- Markdown 생성이 결정론적이며 자동 테스트가 통과한다.
- 실제 Notion 페이지가 생성되고, **1시간 분량 transcript가 잘리지 않고 전부 올라간다.**
- **AI Note가 없는 Recording의 export와 Notion 전송이 동작한다** — 테스트로 확인된다 (INV-8).
- renderer가 Structured Note를 소비하며, 벤더 고유 응답 형태에 의존하지 않는다 (INV-9).
- 긴 문서 분할과 **무손실 재조립** 로직에 자동 테스트가 있고 통과한다 (⚠️ P-1 — 옛 2000자 ·
  100블록 규칙이 아니라 markdown API의 실제 요청 계약을 기준으로 한다).
- Notion 실패 시 local data가 온전하고, 실패가 UI에 보이며 재시도 가능하다.
- 중복 sync 정책이 문서화되어 있고 실제 동작이 그와 일치한다.
- token이 프론트엔드 소스와 저장소에 없다.
- build / lint / test Gate가 green이다.

### Human Review 항목

- 생성된 Notion 페이지가 **실제로 읽을 만한 구조**인가 (§10의 섹션 구성이 살아 있는가)
- export된 Markdown을 Obsidian / NotebookLM 같은 외부 도구에서 열었을 때 쓸 만한가
- 긴 transcript가 Notion에서 실제로 온전한가

## Source of Truth

`docs/PRODUCT-SPEC.md`, 특히 §2.1(INV-3 · INV-5 · INV-6 · INV-7 · INV-8 · INV-9) ·
§7 · §9.3(Structured Note) · §10 · §11 · §12 · §13 · §14.9(Notion 확인된 사실) · §18.

외부 API는 추측하지 말고 실제 현재 지원 범위를 확인한다.
확인할 수 없으면 UNVERIFIED로 남기고 그 사실을 드러낸다.

이 Phase 밖으로 나가지 않는다.
