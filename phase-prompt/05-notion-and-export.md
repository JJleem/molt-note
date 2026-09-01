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

6. **§14.9의 한도를 실제로 다룬다.** 이것은 선택이 아니라 필수다 —
   1시간 transcript는 반드시 한도에 걸린다.
   - 단일 `text.content` **2000자** 상한
   - 요청당 `children` 블록 **100개** 상한
   - 초과분은 `PATCH /v1/blocks/{block_id}/children` **순차 반복** (append에는 배치가 없다)

   **긴 transcript가 잘리거나 조용히 유실되어서는 안 된다.**

7. **§14.9의 UNVERIFIED 항목을 가장 먼저 확인한다** — Notion이 블록 JSON 대신
   `markdown` 문자열을 직접 받는지 여부. 사실이면 A의 산출물을 재사용해 크게 단순해지고,
   아니면 블록 JSON을 직접 만든다. **확인 전에 이 사실에 의존하는 설계를 확정하지 않는다.**

8. **중복 sync 정책을 결정하고 문서화한다** (§10) — 기존 페이지 업데이트인가,
   명시적 중복 생성인가. `NotionSync.pageId`가 그 근거 데이터다.
   사용자가 같은 Recording을 두 번 보냈을 때 무슨 일이 일어나는지 UI에서 예측 가능해야 한다.

9. **NotionSync가 §7 모델대로 저장**된다 — `recordingId` · `pageId` · `syncedAt` ·
   `status` · `error`. 상태가 Recordings 목록과 Detail에 보인다.

10. **integration token이 안전하게 다뤄진다 (INV-7)** — Phase 4가 provider 설정 보관에 대해
    정한 방식과 일관되게 처리한다. 새로운 방식을 따로 만들지 않는다.
    호출 주체(프론트엔드 vs Rust backend)도 Phase 4의 결정과 일관되게 간다.

11. **connection test**가 동작한다 (§5-D) — 토큰과 destination이 유효한지 확인할 수 있다.

12. **§13의 실패가 제품 상태로 다뤄진다** — Notion authentication failure · sync failure ·
    네트워크 없음 · 권한 없는 destination.
    **어떤 실패도 local data에 영향을 주지 않는다 (INV-3).** 재시도가 가능해야 한다.
    부분 전송 후 실패한 경우 사용자가 그 사실을 알 수 있어야 한다.

13. **전송되는 데이터가 UI에 드러난다 (INV-5).** **audio는 전송하지 않는다 (INV-6).**

14. **자동 테스트**: Markdown 생성, 파일명 정규화, Notion payload 생성,
    2000자 청킹과 100블록 배치 분할이 **실제 Notion API 호출 없이** 테스트된다 (§18).

15. build · lint · test Gate가 전부 통과한다.

## Important Rules

- **NotebookLM 자동 연동을 만들지 않는다** (§15). Markdown interoperability만 제공한다.
- Markdown은 Notion 전용 포맷이 아니라 **어디서든 쓸 수 있는 형태**여야 한다 (§11).
- §14.9의 값은 2026-09-01 기준이다. 도입 시점에 재확인한다.
  `Notion-Version` 헤더 값을 추측해서 적지 않는다.
- 자동 테스트가 실제 Notion 워크스페이스를 오염시키지 않는다.
- token을 로그·에러 메시지·evidence 파일에 남기지 않는다.

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
- 2000자 청킹과 100블록 분할 로직에 자동 테스트가 있고 통과한다.
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
