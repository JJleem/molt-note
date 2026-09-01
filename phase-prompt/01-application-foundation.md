# Phase 1 — Application Foundation

Implement Phase 1 of `docs/PRODUCT-SPEC.md`.

## Goal

Molt Note가 **실행되고, 화면 사이를 이동할 수 있고, 로컬에 데이터를 영속적으로 저장했다가
앱을 껐다 켜도 다시 읽어올 수 있는** 최소한의 기반을 만든다.

이 Phase가 끝나면 다음이 사실이어야 한다.

> Molt Note를 실행하면 Recordings 화면이 나오고, Settings에서 바꾼 값과
> 로컬 DB에 저장된 Recording 레코드가 **앱을 완전히 종료했다 다시 실행해도 그대로 있다.**

아직 실제 녹음은 하지 않는다. 소리를 다루지 않는다. AI도 다루지 않는다.

## Why This Phase Exists

Phase 2의 녹음 신뢰성(R-001 ~ R-005)은 "녹음이 끝난 뒤 안전하게 기록될 곳"이 이미
존재할 때만 검증할 수 있다. 저장소·데이터 모델·앱 셸이 없으면 Phase 2는 자기 자신을
증명할 수 없다. 그래서 이 Phase가 먼저 온다.

Bootstrap이 만든 것은 **빈 앱 셸과 동작하는 개발 baseline**일 뿐이다. 이 Phase는
거기에 제품의 뼈대를 붙인다.

## Required Outcome

완료된 Phase는 다음을 만족해야 한다.

1. **로컬 애플리케이션 데이터 디렉터리**가 결정론적으로 결정되고, 없으면 생성되며,
   그 경로를 코드가 **한 곳에서만** 알고 있다 (§3.1의 `AppDataDirectory` 경계).
   - 개발·검증은 macOS에서 한다. 그러나 **경로 규약을 domain에 하드코딩하지 않는다** (INV-10).
     플랫폼별 위치 결정은 이 한 모듈 안에 갇혀 있어야 하고, 나머지 코드는
     "앱 데이터 디렉터리"라는 개념만 안다.
   - 테스트에서 이 경로를 주입/치환할 수 있어야 한다 (실제 사용자 디렉터리를 오염시키지 않기 위해).
   - **Windows 구현을 지금 만들 필요는 없다.** 경계만 있으면 된다.
     플랫폼 분기를 미리 다 채우지 말고, macOS 경로가 domain으로 새지 않게만 한다.

2. **로컬 영속 저장소(SQLite 또는 이에 준하는 lightweight local persistence)** 가
   초기화되고, 스키마가 코드로 관리되며, 앱 재시작 시 기존 데이터를 그대로 읽는다.
   스키마 변경을 다룰 수 있는 최소한의 migration 경로가 있어야 한다.
   (스키마가 바뀔 때마다 사용자 데이터를 지우는 방식은 INV-4 정신에 어긋난다.)

   `docs/PRODUCT-SPEC.md` §14.7에 persistence 후보와 각각의 확인 상태가 정리되어 있다.
   **crate와 버전은 사용 전에 현재 공식 출처에서 확인하고 evidence를 남긴다.**
   확인하지 못하면 UNVERIFIED로 남기고 그 사실을 드러낸다.

3. **`docs/PRODUCT-SPEC.md` §7의 데이터 모델이 스키마로 표현**된다 —
   `Recording` · `Transcript` · `AINote` · `NotionSync`.
   - `Transcript`와 `AINote`는 **서로 다른 레코드**여야 한다 (INV-2).
   - 상태 필드는 `none · pending · running · done · failed`를 구분할 수 있어야 한다.
   - **cardinality는 §7.1에서 확정됐다 — 구현 편의로 바꾸지 않는다:**

     ```text
     Recording  1:N  Transcript      (immutable · versioned)
     Transcript 1:N  AINote          (derived · regeneratable)
     ```

     `Transcript`는 자체 `id`를 가진 독립 entity다. 한 Recording에 여러 Transcript가
     저장될 수 있어야 하고, **기존 Transcript row를 UPDATE하는 경로가 없어야 한다.**
   - **`Recording.currentTranscriptId`** — 현재 사용 중인 성공한 Transcript를 식별한다 (§7.2).
     값이 없는 상태(아직 전사 없음)도 정상 상태다.
     FK와 migration의 구체적 형태는 이 Phase의 architecture에 맞게 설계해도 되지만,
     **§7.2의 domain semantics는 고정이다.**
   - **`AINote`는 provenance를 담을 수 있어야 한다** — **`transcriptId`** ·
     `provider` · `model` · `promptVersion` · `generatedAt` (§7.3).
     이 Phase에서 AI를 호출하지는 않지만, 스키마가 **어떤 Transcript version에서 나온
     노트인지**와 벤더 중립 provenance를 표현할 수 있어야 한다 (INV-9).
     `provider`는 특정 벤더를 전제하지 않는 자유 식별자다.

4. **Recording 레코드에 대한 영속성 계층(repository)** 이 존재한다 —
   생성 · 목록 조회 · 단건 조회 · 삭제. 이 계층은 오디오 파일 없이도 동작하고,
   하드웨어 없이 자동 테스트할 수 있다.
   `duration`을 사람이 읽는 형식(`52:31`)으로 바꾸는 로직도 여기서 테스트 가능해야 한다.

5. **네 개 화면의 navigation shell**이 동작한다 —
   Recordings · Recording · Recording Detail · Settings.
   각 화면은 실제 라우팅으로 도달 가능하고, 데이터가 없을 때의 empty state를 보여준다.
   Recordings 화면은 저장소에서 읽은 실제 레코드 목록을 렌더링한다
   (레코드가 있으면 제목 · 날짜 · 길이 · 세 가지 상태를 보여준다).

6. **Settings 값이 영속화**된다. 최소한 recordings directory와
   automatic 처리 ON/OFF 토글이 저장되고 재시작 후에도 유지된다.
   **Secret(API key · integration token)은 이 Phase에서 다루지 않는다** — §Out of Scope 참조.

7. **macOS microphone 권한 선언(`NSMicrophoneUsageDescription`)이 macOS 번들 설정에 존재**한다.
   - 이것은 **macOS adapter / packaging 요구사항**이다.
     Molt Note의 범용(universal) microphone permission 구현이 **아니다.**
     문서와 코드 어디에서도 그렇게 표현하지 않는다.
   - Windows의 microphone 권한 요구사항은 해당 플랫폼 검증 시(Phase 6) 별도로 다룬다.
     지금 Windows 권한 로직을 만들지 않는다.
   - 권한을 **요청**하거나 장치를 **열거**하지는 않는다 (Phase 2가 한다).

8. **실패가 UI에 보인다.** 저장소 초기화 실패처럼 이 Phase에서 실제로 발생 가능한 오류가
   console에만 남지 않고 사용자에게 보이는 상태로 표현된다 (§13).

9. Bootstrap이 설정한 **build · lint · test Gate가 모두 통과**하고, 위 로직에 대한
   실제 자동 테스트가 존재한다.

## Important Rules

- `docs/PRODUCT-SPEC.md`가 source of truth다. 거기에 없는 제품 기능을 추가하지 않는다.
- **불변 규칙(INV-1 ~ INV-10)을 설계 단계에서 위반하지 않는다.** 이 Phase에서 직접 적용되는 것:
  - **INV-2** — Transcript와 AINote 분리
  - **INV-7** — secret을 프론트엔드/저장소에 두지 않음
  - **INV-9** — domain이 특정 AI 벤더 타입에 의존하지 않음
  - **INV-10** — domain이 특정 OS에 의존하지 않음
- **추상화를 선입금하지 않는다 (§20.6).** 이 Phase에서 실제로 플랫폼이 갈리는 지점은
  앱 데이터 디렉터리 정도다. 그 외에 `RecordingBackend` · `TranscriptionRunner` ·
  `SidecarResolver` · `SecretStore` 같은 경계를 **미리 만들지 않는다.**
  각각은 실제로 그것을 필요로 하는 Phase에서 만든다.
  구현이 하나뿐인 인터페이스를 "언젠가 필요할 것"이라는 이유로 만들지 않는다.
- 데이터 모델을 §7과 다르게 바꿔야 할 합리적인 이유가 있다면, **바꾸되 그 이유를 기록한다.**
  조용히 다르게 만들지 않는다. Phase 3·4가 이 스키마를 전제로 시작한다.
- 하드웨어·네트워크·플랫폼·파일시스템 사용자 디렉터리에 의존하는 코드와 순수 로직의
  **경계를 긋는다.** 경계를 긋는 이유는 Phase 2 이후의 자동 검증 가능성이 여기서 결정되기 때문이다 (§18).
- UI는 §19를 따른다: minimal · macOS-like · typography first. 장식용 dashboard를 만들지 않는다.
- 외부 라이브러리를 도입할 때 버전과 실제 지원 범위를 확인한다. 추측으로 API를 쓰지 않는다.
  확인할 수 없으면 UNVERIFIED로 남기고 그 사실을 드러낸다.
- 지금 필요하지 않은 미래 의존성(whisper · Ollama client · AI SDK · Notion SDK)을 설치하지 않는다.
- **Git commit / push는 이 Phase의 작업이 아니다.** Phase commit은 Phase가 완료되고
  검증된 뒤 운영자가 한다 (`docs/GIT-WORKFLOW.md`). Worker가 commit하면 HEAD가 바뀌어
  Gate·Verifier가 묶여 있는 subject fingerprint가 실행 도중에 흔들린다.
  저장소에 파일을 만들고 고치는 것까지가 Task의 일이다.

## Out of Scope

이 Phase에서 구현하지 **않는다.**

- 실제 오디오 캡처 · microphone 열거 · 권한 요청 흐름 · Record / Pause / Resume / Stop
- recording engine 아키텍처 결정 (Phase 2가 검증과 함께 결정한다)
- 오디오 재생
- whisper / 전사 기능 일체
- **AI Provider 일체** — provider 추상화 구현 · Ollama · Claude · Gemini · Groq
  (§7의 AINote 스키마가 provenance를 담을 수 있게 하는 것까지가 이 Phase다.
  provider 인터페이스나 adapter를 만드는 것은 Phase 4다.)
- **재전사 workflow · transcript version 선택 UI · transcript history UI**
  (스키마가 §7.1의 cardinality를 표현할 수 있게 하는 것까지가 이 Phase다.
  version을 실제로 만들고 고르는 흐름은 Phase 3이다.)
- `currentTranscriptId`를 실제로 갱신하는 전사 파이프라인 (Phase 3)
- Notion 연동, Markdown export
- Secret 저장소 구현 (API key · integration token 입력 및 보관)
- **Windows 환경 구축 · Windows E2E · Windows packaging · Windows 권한 로직**
  (경계를 열어두는 것까지가 이 Phase다. 실제 Windows 검증은 Phase 6이다.)
- search · tags · keyboard shortcuts · menu bar (DEFERRED)

Settings 화면에 Transcription / AI Provider / Notion **섹션의 자리**를 만드는 것까지는 허용된다.
그 안의 실제 기능이나 secret 입력을 구현하는 것은 허용되지 않는다.

## Verification Boundary

이 Phase는 다음이 **모두** 사실일 때만 완료다.

- 앱을 종료했다 다시 실행해도 저장된 Recording 레코드와 Settings 값이 그대로 남아 있다.
  이것은 실제로 검증되어야 하며, "그렇게 설계했다"는 서술은 근거가 아니다.
- 영속성 계층과 duration 포맷 로직에 대한 자동 테스트가 존재하고 통과한다.
- `Transcript`와 `AINote`가 서로 다른 레코드로 저장 가능하며, 하나를 쓰는 것이
  다른 하나를 덮어쓰지 않는다는 것이 테스트로 확인된다.
- **한 Recording에 Transcript를 둘 이상 저장해도 앞의 것이 남아 있다** — 테스트로 확인된다 (§7.1).
- **`Recording.currentTranscriptId`가 성공한 Transcript를 가리키고, 값이 없는 상태도
  표현 가능하다** (§7.2).
- **서로 다른 Transcript에 각각 AINote를 붙였을 때 `transcriptId`로 출처가 구분된다** (§7.3).
- `AINote` 스키마가 벤더 중립 provenance(`transcriptId` · `provider` · `model` ·
  `promptVersion` · `generatedAt`)를 저장·복원할 수 있다.
- 앱 데이터 디렉터리 결정이 단일 모듈에 갇혀 있고, 그 밖의 코드에 OS별 경로 규약이 없다.
- `NSMicrophoneUsageDescription`이 macOS 번들 설정에 실제로 존재하며,
  문서가 이를 macOS 요구사항으로(범용 구현이 아니라) 기술한다.
- 네 화면 모두 라우팅으로 도달 가능하다.
- 설정된 build / lint / test Gate가 전부 green이다.
- 이 Phase의 결과물을 쓰기 위해 Phase 2 이후의 기능이 필요하지 않다.

### Human Review 항목

자동으로 판정할 수 없는 것은 사람이 확인한다. 자동 PASS로 적지 않는다.

- 화면 레이아웃이 §19의 방향(minimal · macOS-like)에 맞는가
- macOS 권한 프롬프트 문구가 사용자에게 납득 가능한가
- 번들된 `.app` 안에서 Info.plist가 실제로 병합되는가
  (`npm run tauri build`는 Gate가 아니므로 자동 판정되지 않는다)

## Source of Truth

`docs/PRODUCT-SPEC.md`를 따른다. 특히 §2.1(불변 규칙) · §3(플랫폼) · §3.1(cross-platform 원칙) ·
§5(화면) · §7(데이터 모델) · §13(실패 처리) · §14(검증된 기술 사실) · §18(검증 철학) · §19(UI 방향).

외부 라이브러리나 API가 관련되면 추측하지 말고 실제 현재 지원 범위를 확인한다.

이 Phase 밖으로 나가지 않는다.
