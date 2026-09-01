# Final Integrated Goal — Molt Note V1

Complete the first integrated V1 described in `docs/PRODUCT-SPEC.md`.

이 Goal의 목적은 **새 기능 영역을 추가하는 것이 아니다.** Phase 1~5가 각각 만들어 놓은
흐름들을 하나의 제품으로 통합하고, 경계에 남은 불일치를 정리하고,
§17의 V1 성공 기준을 실제로 검증하는 것이다.

핵심 질문은 하나다.

```text
이것이 지금 하나의 일관된 제품처럼 동작하는가,
아니면 완료된 Phase들의 모음인가?
```

## Preconditions

Phase 1~6이 모두 DONE이어야 한다. 미완성 Phase의 기능을 이 Goal에서 몰래 구현하지 않는다.

## Primary Flows

`docs/PRODUCT-SPEC.md` §17의 흐름이 처음부터 끝까지 끊기지 않고 동작해야 한다.
**흐름은 둘이며, 첫 번째가 더 중요하다.**

### Flow A — Core (AI 없이 성립해야 한다 · §17.1 · INV-8)

```text
Molt Note 실행
  → "3DGS Study" 입력 → Record → 긴 녹음 → Stop
  → 앱에 녹음 저장
  → Local Whisper transcription
  → Transcript 검토
  → Markdown export
  → 나중에 다시 열고 원본 음성 / Transcript 확인
```

**AI Provider가 하나도 설정되지 않은 상태에서 이 흐름 전체가 동작해야 한다.**
이것이 V1의 core 성공 조건이다.

### Flow B — Full (AI 포함 · §17.2)

```text
Flow A + Local Ollama로 Study Note 생성 → 내용 확인 → Notion으로 전송
```

## The Integrated Product Must Answer

1. **한 번의 세션으로 끝까지 갈 수 있는가?**
   녹음부터 Notion 전송까지, 사용자가 문서를 읽거나 터미널을 열지 않고 앱 안에서만
   진행할 수 있는가. 각 단계로 넘어가는 경로가 화면에서 자연스럽게 드러나는가.

2. **앱을 껐다 켜도 모든 것이 그대로인가?**
   Recording · Transcript · AINote · NotionSync 상태 전부.

3. **중간에 실패해도 제품이 무너지지 않는가?**
   §13의 실패들이 각각 발생했을 때, 사용자는 (a) 무엇이 실패했고 (b) 원본이 안전하며
   (c) 다시 시도할 수 있다는 것을 알 수 있는가.
   특히 **후처리 단계 하나가 실패해도 그 앞 단계의 산출물은 남아 있는가** (INV-3).

4. **불변 규칙이 실제로 지켜지는가?**
   INV-1 ~ INV-10이 코드 수준에서 성립하는지 확인한다. 특히:
   - AI가 원본 audio나 Raw Transcript를 건드리지 않는다 (INV-1 · INV-2)
   - audio가 AI Provider나 Notion으로 나가는 경로가 **존재하지 않는다** (INV-6)
   - secret이 프론트엔드 소스나 저장소에 없다 (INV-7)
   - **AI Provider를 제거하거나 중지해도 Flow A가 그대로 동작한다 (INV-8)**
   - **core/domain에 벤더 고유 타입이 없다 (INV-9)**
   - **core/domain에 OS 고유 가정이 없다 (INV-10)**

5. **사용자가 요청하지 않은 외부 전송이 일어나지 않는가?**
   §12의 Privacy Boundary가 실제 동작과 일치하는가. 자동 처리 설정이 켜져 있을 때도
   사용자가 그 사실을 알고 켠 것인가.
   **로컬 provider(Ollama)와 외부 provider가 UI에서 구분되는가.**

6. **상태 표현이 화면들 사이에서 일관되는가?**
   Recordings 목록의 상태 배지와 Detail 화면의 상태가 같은 것을 말하는가.
   Phase마다 다른 용어나 다른 상태 집합을 쓰고 있지 않은가.

7. **Settings가 실제로 존중되는가?**
   default microphone · recordings directory · whisper model · language ·
   automatic 토글들 · AI provider와 모델 · Notion destination —
   저장만 되고 무시되는 설정이 없는가.

8. **두 플랫폼이 같은 제품인가?**
   Phase 6이 확인한 Windows 동작이 여전히 유효한가. macOS와 Windows에서 같은 흐름이
   같은 결과를 내는가. 한쪽에서만 되는 기능이 있다면 그 사실이 문서에 정확히 적혀 있는가.

## What This Goal Should Fix

Phase 경계에서 생긴 것들을 정리한다.

- 화면 간 상태 용어·표현 불일치
- Phase마다 따로 만들어진 중복 로직 (특히 에러 표현과 상태 전이)
- 각 Phase가 따로 정한 정책들 사이의 모순
  (재전사 정책 · AI 재생성 정책 · 중복 sync 정책이 서로 어긋나지 않는가)
- 실제로는 플랫폼 차이가 없던 곳에 남아 있는 불필요한 추상화 (§20.6)
- 구현이 하나뿐인데 존재 이유가 사라진 경계
- 문서와 실제 동작의 차이 — `docs/PRODUCT-SPEC.md` §7의 데이터 모델이 구현과 다르다면
  **어느 쪽이 옳은지 결정하고 문서를 맞춘다**
- Phase 진행 중 UNVERIFIED로 남은 항목 중 아직 확인되지 않은 것

## Final Verification

`docs/PRODUCT-SPEC.md` §17의 V1 성공 기준을 acceptance target으로 쓴다.

자동으로 검증 가능한 것과 사람이 확인해야 하는 것을 §18에 따라 명확히 구분한다.

### 자동 검증

- 전체 도메인 로직 · 상태 기계 · 파싱 · 직렬화 · 영속성 · 실패 상태 처리 테스트
- build · lint · test Gate 전부 green (**양쪽 플랫폼에서**)
- INV-6(audio 외부 전송 없음)이 코드 수준에서 확인 가능한 형태로 검증
- **INV-8(AI 없이 Flow A 동작)이 자동 테스트로 검증**
- **INV-9(core에 벤더 타입 없음) · INV-10(core에 OS 가정 없음) 확인**

### Human Witness (자동 PASS로 적지 않는다)

- 실제 1시간 녹음으로 Flow A를 끝까지 완주 (**AI Provider 없이**)
- 실제 1시간 녹음으로 Flow B를 끝까지 완주
- 실제 마이크 음질 · 실제 재생 음질
- 실제 생성된 Notion 페이지의 품질
- 실제 AI Note가 읽을 가치가 있는가
- §19 UI 방향(minimal · macOS-like · calm)에 부합하는가
- Windows에서의 실제 동작 (Phase 6의 결과가 여전히 유효한가)

## Explicit Non-Goals

이 Goal에서 추가하지 **않는다.** §15의 non-goal 전체가 그대로 유지되며, 추가로:

- **Cloud AI Provider (Claude · Gemini · Groq)** — DEFERRED (§16).
  provider 추상화가 존재한다는 것과 provider를 더 구현하는 것은 다른 문제다.
- search · tags · processing queue · keyboard shortcuts · menu bar (§16 DEFERRED)
- AI prompt customization UI
- 새로운 export 포맷
- 새로운 화면
- Linux · 모바일 (§15 non-goal)
- 성능 최적화를 위한 대규모 재작성
- App Store / Microsoft Store 배포 · 코드서명 파이프라인 (§3에서 범위 밖)

**`Goal.md`를 미완성 기능을 숨기는 곳으로 쓰지 않는다.**
Phase 1~6에서 완료되지 않은 것이 있다면 그것을 여기서 구현하는 것이 아니라,
완료되지 않았다는 사실을 드러낸다.

## Source of Truth

`docs/PRODUCT-SPEC.md` 전체. 특히 §2.1(불변 규칙 INV-1~INV-10) · §3(플랫폼) ·
§9(Provider System) · §12 · §13 · §17 · §18 · §20.
