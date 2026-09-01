# Phase 4 — AI Provider System + Local AI

Implement Phase 4 of `docs/PRODUCT-SPEC.md`.

## Goal

Raw Transcript를 **vendor 중립 Provider 경계를 통해** structured note로 바꾸고,
첫 번째 Provider로 **로컬 Ollama**를 붙인다.

```text
Transcript  →  Note AI Provider  →  Structured Note
                      ↑
              첫 구현: Local Ollama
```

이 Phase의 성공 기준은 **두 개**다.

> 1. 사용자가 별도로 실행 중인 로컬 Ollama로 Meeting / Study / Summary 노트를 생성해
>    Recording Detail의 AI Note 탭에서 볼 수 있고, **Raw Transcript는 전혀 변하지 않는다.**
> 2. **AI Provider를 전혀 설정하지 않아도 제품이 정상 동작한다** — 녹음 · 전사 · 열람이
>    막히지 않고, "provider 없음"이 오류가 아닌 정상 상태로 표현된다 (INV-8).

두 번째가 첫 번째보다 덜 중요하지 않다.

## Why This Phase Exists

Transcript 그 자체는 읽기 어렵다. 이 Phase가 "녹음 파일 더미"를 "다시 찾아볼 수 있는 기록"으로
바꾼다.

동시에 이 Phase는 제품의 가장 중요한 **경계 하나**를 세운다.

```text
Molt Note domain   ≠   특정 AI 벤더의 domain      (INV-9)
```

벤더는 바뀐다. 가격 정책도, 무료 티어도, API 형태도 바뀐다.
그것들이 바뀔 때 흔들리는 것이 adapter 한 개여야지 제품 전체여서는 안 된다.

## Required Outcome

### A. Provider 경계

1. **`NoteAIProvider` 계약이 존재하고, core/domain이 특정 벤더 타입을 모른다 (INV-9).**
   §9.2가 설계 의도를 보여준다. **더 적합한 contract가 있으면 바꿔도 되지만,
   바꾼 이유를 기록한다.** 바뀌면 안 되는 것은 계약의 형태가 아니라 INV-9다.
   벤더 지식(엔드포인트 · 요청 형태 · 에러 코드 · SDK)은 adapter 안에만 있어야 한다.

2. **provider가 없는 상태를 정상으로 다룬다 (INV-8).**
   `isAvailable()`에 해당하는 확인 수단이 있고, provider가 없거나 연결되지 않을 때
   앱은 경고나 에러가 아니라 "AI 기능이 비활성"이라는 담담한 상태를 보여준다.
   **AI 설정 실패나 provider 부재가 Recording / Transcript / 열람을 block하지 않는다.**

3. **추상화가 실제로 벤더 중립인지 검증된다.**
   구현이 하나뿐인 추상화는 검증된 추상화가 아니다.
   이 Phase는 Ollama adapter 외에 **결정론적 test double(fake provider)** 로 같은 계약을
   통과시켜 경계를 검증한다.
   - fake provider는 **테스트 전용**이며 제품 UI에 노출되는 선택지가 아니다.
   - 이것으로 두 번째 벤더를 V1에 넣지 않고도 추상화를 검증한다 (§21).

### B. Structured Note

4. **세 가지 mode가 동작한다** (§9.5): Meeting · Study · Summary. 사용자가 선택할 수 있다.

5. **Structured Note가 provider 중립 데이터로 저장된다** (§9.3).
   Provider가 돌려준 임의의 Markdown을 core data model로 쓰지 않는 것을 우선 검토한다.
   **최종 schema를 이 Phase에서 확정하고 기록한다.**
   렌더링 방향은 한 방향으로 흐른다 — `Structured Note → UI renderer`.
   (Markdown / Notion renderer는 Phase 5가 같은 구조를 소비한다.)

6. **구조화된 출력을 신뢰성 있게 얻는다.**
   §14.5에 Ollama의 구조화 출력 수단이 확인되어 있다. 이를 쓸지, 응답을 파싱할지
   결정하고 근거를 기록한다. 수단은 provider마다 다를 수 있으며 그 차이는 adapter가 흡수한다.
   **어느 쪽이든 응답이 기대 schema와 다를 때 앱이 깨지지 않아야 한다.**
   이것은 로컬 소형 모델에서 실제로 자주 일어나는 일이므로 예외가 아니라 기본 경로다.

### C. Ollama Adapter

7. **사용자가 별도로 실행 중인 로컬 Ollama에 연결한다.**
   **Ollama를 앱 installer에 bundle하지 않는다** (§15).
   연결 대상(host/port)은 설정 가능해야 하고, 기본값은 §14.5의 확인된 값을 쓴다.

8. **설치된 모델 목록을 읽어 사용자가 고를 수 있다.** 모델이 하나도 없을 때의 상태가 있다.

9. **연결 확인 수단**이 Settings에 있다 (§5-D) — Ollama가 실행 중인지, 어떤 모델이 있는지.

10. **호출 주체(프론트엔드 vs Rust backend)를 결정하고 근거를 기록한다.**
    §14.5에 이 선택에 영향을 주는 사실(로컬 서버의 origin 정책 등)이 정리되어 있다.
    Phase 5의 Notion 호출과 일관된 방식을 택하는 것이 바람직하다.

### D. 데이터와 실패

11. **AINote가 §7 모델대로 저장**된다 — `id` · `recordingId` · **`transcriptId`** ·
    `type` · `content` ·
    **provenance(`provider` · `model` · `promptVersion` · `generatedAt`)**.
    **`transcriptId`는 provenance의 일부다 (§7.3)** — 한 Recording에 Transcript가
    여럿일 수 있으므로(§7.1), 어떤 version에서 나온 노트인지 식별 가능해야 한다.
    기본 입력은 `Recording.currentTranscriptId`가 가리키는 Transcript다 (§7.2).
    **Transcript와는 별개의 레코드다 (INV-2).**
    AI 생성이 Raw Transcript를 덮어쓰지 않는다는 것이 테스트로 확인되어야 한다.
    AI Provider는 recording metadata 중 AI 상태 필드 외의 것을 변경하지 않는다.

12. **재생성이 가능하다.** 재생성이 기존 Transcript를 훼손하지 않는다.
    기존 AINote를 대체할지 이력으로 남길지 정책을 정하고 기록한다.

13. **`promptVersion`이 실제로 기록**된다. 프롬프트가 바뀌면 값이 바뀌어야 한다.

14. **§13의 AI Provider 실패가 제품 상태로 다뤄진다** — provider 미설정 ·
    연결 불가(Ollama 미실행) · 모델 없음 · 요청 실패 · 응답이 기대 schema와 다름.
    provider별 에러는 adapter가 **domain 공통 실패 타입**으로 변환한다 (INV-9).
    **어떤 실패도 Transcript와 Recording에 영향을 주지 않는다 (INV-3).**

15. **전송되는 데이터가 UI에 드러난다 (INV-5).**
    사용자는 선택한 provider가 **로컬인지 외부인지** 알 수 있어야 한다 (§12).
    **audio는 전송하지 않는다 (INV-6)** — 코드 수준에서 보장되어야 한다.

16. **자동 테스트**: provider 계약 준수, structured note schema 검증,
    기대와 다른 응답에 대한 방어, 실패 상태 매핑, **provider 부재 시 core pipeline이
    정상 동작함(INV-8)** 이 **실제 Ollama 없이** 테스트된다 (§18).
    프로세스/네트워크 경계와 파싱·도메인 로직을 분리해서 설계한다.

17. build · lint · test Gate가 전부 통과한다.

## Important Rules

- **Claude를 필수 dependency로 만들지 않는다.** Cloud provider는 DEFERRED다 (§16).
  이 Phase에서 Claude · Gemini · Groq adapter를 구현하지 않는다.
- **Claude Agent SDK를 쓰지 않는다** (§9.4).
- **Free Tier나 현재 가격 정책을 architecture dependency로 만들지 않는다.**
  외부에서 바뀔 수 있는 값이다.
- **AI는 보조 수단이다 (§2).** AI가 실패해도 제품은 계속 쓸 수 있어야 한다.
- 자동 테스트가 실제 Ollama 프로세스나 외부 네트워크에 의존하지 않는다.
- §14.5의 값은 확인 시점 기준이다. 도입 시점에 재확인한다. 엔드포인트·파라미터 이름을
  기억이나 추측으로 쓰지 않는다.
- provider 설정값을 로그·에러 메시지·evidence 파일에 남기지 않는다.
- **추상화를 선입금하지 않는다** — provider 경계는 실제로 필요하므로 만든다.
  그 외의 경계(예: 임베딩·RAG·스트리밍 파이프라인)를 미리 만들지 않는다.
- **Git commit / push는 이 Phase의 작업이 아니다.** Phase commit은 Phase가 완료되고
  검증된 뒤 운영자가 한다 (`docs/GIT-WORKFLOW.md`). Worker가 commit하면 HEAD가 바뀌어
  Gate·Verifier가 묶여 있는 subject fingerprint가 실행 도중에 흔들린다.
  저장소에 파일을 만들고 고치는 것까지가 Task의 일이다.

## Out of Scope

- Cloud AI Provider 구현 — Claude · Gemini · Groq (DEFERRED, §16)
- Ollama 설치 · 번들링 · 프로세스 생명주기 관리 (§15) — 사용자가 실행한 것에 연결만 한다
- 모델 다운로드 관리 UI
- Notion 연동 · Markdown export (Phase 5)
- AI prompt customization UI (DEFERRED, §16)
- 실시간 AI assistant · Agent 기반 자동 업무 수행 (§15)
- RAG · 여러 녹음 교차 질의 (§15)
- 스트리밍 응답 UI
- 오디오를 AI Provider로 전송하는 어떤 경로도 만들지 않는다 (INV-6)
- 여러 Recording 일괄 AI 처리 큐 (DEFERRED, §16)
- Windows 검증 (Phase 6)

## Verification Boundary

- Transcript로부터 세 가지 mode의 structured note가 생성되어 저장되고 화면에 보인다.
- **AI Note 생성 전후로 Raw Transcript가 바이트 단위로 동일하다** — 테스트로 확인된다.
  Transcript가 여럿인 Recording에서도 **어느 Transcript도 변경되지 않는다** (§7.1).
- 서로 다른 Transcript에서 생성된 AI Note가 `transcriptId`로 구분된다 (§7.3).
- AI Note와 Transcript가 서로 다른 레코드로 저장되고, provenance가 기록된다.
- **provider가 하나도 설정되지 않은 상태에서 녹음 · 전사 · 열람이 정상 동작한다** —
  테스트로 확인된다 (INV-8). 이것이 이 Phase의 가장 중요한 검증 항목 중 하나다.
- 같은 provider 계약을 Ollama adapter와 test double이 모두 통과한다 (INV-9).
- core/domain 코드에 벤더 고유 타입·엔드포인트·에러 코드가 없다.
- provider 실패(미실행 · 모델 없음 · 잘못된 응답) 시 Transcript와 Recording이 온전하고,
  실패가 UI에 보이며 재시도 가능하다.
- 파싱·실패 처리에 대한 자동 테스트가 실제 Ollama 없이 통과한다.
- build / lint / test Gate가 green이다.

### Human Review 항목

- 실제 스터디/회의 녹음에 대해 로컬 모델이 생성한 노트가 **읽을 가치가 있는가** —
  이것이 이 Phase의 실질적 성공 조건이며 자동으로 판정할 수 없다
- 로컬 모델이 한국어+영어 혼용 transcript를 다룰 수 있는가
- 1시간 분량 transcript 처리 시간이 실사용 가능한 수준인가
- Meeting과 Study mode의 출력이 실제로 서로 다른 유용성을 갖는가
- provider가 로컬인지 외부인지가 사용자에게 분명한가

## Source of Truth

`docs/PRODUCT-SPEC.md`, 특히 §2.1(INV-2 · INV-3 · INV-5 · INV-6 · INV-8 · INV-9) ·
§7(데이터 모델) · §9(Note AI Provider System 전체) · §12(Privacy Boundary) ·
§13(실패 처리) · §14.5(AI Provider 확인된 사실) · §18.

외부 API는 추측하지 말고 실제 현재 지원 범위를 확인한다.
확인할 수 없으면 UNVERIFIED로 남기고 그 사실을 드러낸다.

이 Phase 밖으로 나가지 않는다.
