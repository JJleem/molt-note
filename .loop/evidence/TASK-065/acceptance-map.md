# TASK-065 — Acceptance Criteria가 어디서 판정되는가

| AC | 판정 수단 | 어디에 |
| --- | --- | --- |
| **AC-1** build Gate | gate | `gate-self-check.md` — `npm run build` exit 0 |
| **AC-2** lint Gate | gate | `gate-self-check.md` — `npm run lint` exit 0 (eslint + clippy `-D warnings`) |
| **AC-3** test Gate | gate | `gate-self-check.md` — `npm run test` exit 0 (vitest 494 + Rust 744) |
| **AC-4** PRODUCT-SPEC §9 · §14.5 · §21 | verifier | 아래 §1 |
| **AC-5** ADR-0010 확정 · VERIFIED/UNVERIFIED 구분 | verifier | 아래 §2 |
| **AC-6** SYSTEM-MAP | verifier | 아래 §3 |
| **AC-7** Human Review 문서의 빈 기록표 · 소스 미변경 · commit 없음 | verifier | 아래 §4 · `changed-files.md` |

---

## 1. AC-4 — `docs/PRODUCT-SPEC.md`

| 요구 | 어디 |
| --- | --- |
| §9 갱신 — Manual AI Handoff를 §11의 연장으로 더한다 | **§9.7 신설**. §11(Markdown interoperability)의 연장임을 근거표에 적었다. §9.1~§9.6은 **한 줄도 지우지 않았다** |
| §14.5 갱신 — Ollama를 '선택적 · 로컬 · 고급'으로 | **§14.5.1 신설**. §14.5 본문(엔드포인트 · CORS · num_ctx 등 외부 사실)은 **그대로**이며, 바뀐 것이 사실이 아니라 제품상 위치임을 명시했다 |
| §21에 Phase 5.5 삽입 | **§21 표에 `5.5` 행 추가** + 상태 열을 실제 구현 상태로 정정(D-3) + 순서의 근거에 Phase 5.5 항목 추가 |
| 소급 삭제 금지 · 언제 왜 바뀌었는지 | 개정 이력 **rev 9**(2026-09-06) · §9.4 머리의 ⚠️ 주석(rev 8까지의 표기를 남겨 둔다는 명시) · §14.5.1의 "rev 8까지의 서술 / rev 9의 서술" 두 행 · §21 표 아래의 변경 사유 표 |
| **삭제가 아니라 재배치** | §14.5.1의 `**삭제가 아니다**` 행 · §9.4 주석 · §9.7 머리말 |

## 2. AC-5 — `docs/ADR-0010-manual-ai-handoff.md`

| 요구 | 어디 |
| --- | --- |
| 실제 구현 결과로 확정 | **§12 신설** (12.1 만들어진 자리 · 12.2 달라진 것 · 12.3 확정된 값 · 12.4 UNVERIFIED · 12.5 tripwire 네 자리 · 12.6 Gate) |
| 계획과 달라진 것 | **§12.2의 셋** — (a) 끌어올린 함수가 둘이 아니라 넷 (b) `transcript_text`가 빈 문자열을 낼 수 있다 (c) clipboard 실패가 두 갈래로 갈린다. 각각 이유를 적었다 |
| VERIFIED / UNVERIFIED 구분 | §12는 §3의 표기(**[E1] / [E3] / [E4]**)를 그대로 쓴다. §12.3은 [E1], §12.6은 [E3], §12.4는 전부 [E4] |
| **clipboard가 실제 환경에서 동작하는지** | **§12.4 — 확인되지 않았다고 적었다.** §7.4의 여섯 항목이 **하나도 확인되지 않았고 전부 UNVERIFIED로 남는다**고 명시. `NOT RUN` 두 줄을 따로 두었다 |
| 결정 절 보존 | §4~§9는 고치지 않았다. Status 블록과 §12만 바뀌었다 |

## 3. AC-6 — `docs/SYSTEM-MAP.md`

| 요구 | 어디 |
| --- | --- |
| Build Evolution Map에 Phase 5.5 | **§5에 `Phase 5.5` 절 신설** (Task 11개 표 · 핵심 성질 · 검증 · IMPLEMENTED로 적지 않는 것) + `Phase 5.6 PLANNED` 절 |
| §2 흐름 | Manual AI Handoff 갈래를 흐름도에 추가 (AI Note 탭 두 줄 → 순수 모듈 → clipboard / write_new → 사람) |
| §3 컴포넌트 | `export/ai_request.rs` · `export/handoff.rs` · command 셋 · `src/platform/clipboard.ts` · `copyView`/`aiHandoffView` · UI 기반 여섯 행 추가 |
| §6 검증 모델의 테스트 수 | **1,081 → 1,238** (vitest 494 · Rust 744). Human validation 열에 셋을 추가 |
| §9 결정 이력 | D-1 · D-2 · D-3 · D-4 · D-5 · clipboard 경계 · 프롬프트 상수 재사용 · UI 기반 방침 **여덟 행 추가** |
| history를 덮어쓰지 않음 | 상태 블록은 **"직전 상태: Phase 5 완료"**를 남기고 그 위에 쌓았다. 갱신 이력에 2026-09-06 줄을 **추가**했다. §5의 이전 Phase 절은 하나도 고치지 않았다 |
| **확인되지 않은 것을 DONE으로 적지 않음** | §1에 "⚠️ 구현됐으나 실물 미확인 — Phase 5.5" 행 · §5의 `NOT RUN` 블록 · §7의 clipboard UNVERIFIED 항목 · §6이 Human Review 기록표가 **비어 있다**고 적는다 |

**함께 정정한 것 (이 Task가 SYSTEM-MAP을 여는 김에 사실과 맞춘 자리):**
2026-09-05 운영자의 첫 실제 전사 실행 결과. 기존 문장은 "전사가 실제로 실행된 적이 없다"였는데
그것은 이제 사실이 아니다. **`A-TRANS-001`이 해소되지 않았다는 것은 그대로 적었다** —
엔진 경로는 지났으나 언어가 설정되지 않아 결과를 쓸 수 없었다
(`docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록 · Phase 5.6).

## 4. AC-7 — `docs/PHASE-5.5-HUMAN-REVIEW.md`

| 요구 | 어디 |
| --- | --- |
| 세 항목의 절차 | **HR-1** §3(화면 · 11개 소항목) · **HR-2** §4(Manual AI Handoff · 산출물 확인 + 실제 채팅 P-1/P-2/P-3) · **HR-3** §5(로컬 provider의 위치 · 9개 소항목) |
| **빈** 기록표 | **§8.** 실행 날짜 · 실행자 · 앱 버전 · OS · 사용한 AI 채팅이 전부 `(비어 있음)`. §8.1 · §8.2 · §8.3의 모든 행이 `☐ PASS ☐ FAIL ☐ 미실행`이고 **체크된 칸이 하나도 없다.** §8.4 자유 기술도 `(비어 있음)` |
| 미리 채우지 않음 | 결과가 적힌 칸이 없다. 문서 머리의 Status가 **"Human Review는 아직 실행되지 않았다"** 로 적혀 있다 |
| 소스 · 설정 · 테스트 미변경 · commit 없음 | `changed-files.md` |
