# Phase 2 — Reliable Recording

Implement Phase 2 of `docs/PRODUCT-SPEC.md`.

## Goal

Molt Note에서 **실제 macOS 마이크로 녹음하고, 일시정지·재개하고, 정지하면 로컬 파일이
확실히 생성되고, 그 녹음을 다시 재생할 수 있게** 만든다.

이 Phase의 성공 기준은 하나다.

> Molt Note에서 실제 macOS microphone으로 녹음한 뒤 **앱을 종료하고 다시 실행해도**
> 해당 녹음을 목록에서 찾아 재생할 수 있다.

이것이 제품의 심장이다. §2의 우선순위에 따라 **녹음 신뢰성은 AI 기능보다 우선한다.**

## Why This Phase Exists

Phase 1이 만든 저장소와 데이터 모델은 아직 아무것도 담고 있지 않다. 이 Phase에서
처음으로 **진짜 end-to-end 능력**이 증명된다 — 마이크에서 시작해 재생 가능한 파일로 끝나는 경로.

Phase 3 이후(전사 · AI · Notion)는 전부 "여기서 만들어진 audio 파일"을 입력으로 받는다.
이 Phase가 부실하면 그 위의 모든 것이 부실하다.

## The Architecture Decision This Phase Must Make

**이 Phase는 recording engine을 선택하고 그 근거를 ADR로 기록해야 한다.**
`docs/PRODUCT-SPEC.md` §6.1이 평가 기준을, §14.3이 2026-09-01 기준 확인된 사실을 담고 있다.

평가 대상은 최소한 다음 둘이다.

```text
WebView / MediaRecorder      vs      Rust/native audio path (예: cpal)
```

ADR은 §6.1의 기준 전체를 다뤄야 한다.

```text
recording reliability      microphone enumeration     microphone selection
pause/resume               file finalization          data-loss behavior
audio format               transcription compatibility
macOS permission behavior  Windows compatibility
Tauri integration          packaging
testability                maintenance cost
```

**"macOS에서 가장 쉬운 구현"이 아니라 "Molt Note 요구사항에 가장 적합한 구현"을 선택한다.**

특히 결정적인 것 넷:

1. **transcription compatibility** — Phase 3의 whisper는 **16-bit WAV만** 받는다 (§14.4).
   녹음 단계에서 그 포맷을 직접 만들 수 있는지, 변환 단계가 필요한지가 전체 파이프라인의
   복잡도를 바꾼다.
2. **Windows compatibility** — Windows는 지원 대상 플랫폼이다 (§3).
   §14.3에 확인된 사실: webview 후보는 macOS(WKWebView)와 Windows(WebView2, Chromium)에서
   **코덱과 동작이 다를 수 있다.** cpal은 두 플랫폼(CoreAudio / WASAPI)에서 같은 PCM을 준다.
   **여기서 Windows를 무시하고 고르면 Phase 6에서 엔진을 다시 고르게 된다.**
3. **data-loss behavior (R-005)** — 앱이 예기치 않게 종료되어도 이미 녹음된 부분이
   얼마나 살아남는가.
4. **testability** — 상태 기계를 하드웨어 없이 검증할 수 있는가 (§18).

§14.3의 **UNVERIFIED 항목은 이 Phase에서 실제로 확인한다.** 문서만 읽고 결정하지 않는다 —
이 기기에서 동작을 확인한 뒤 결정한다.
확인 결과와 채택된 결정, 그리고 **탈락한 후보와 탈락 이유**를 저장소에 기록한다.

### Windows에 대해 이 Phase가 하는 일 / 하지 않는 일

**한다**: 엔진 선택 시 Windows 실현 가능성을 근거와 함께 평가하고 ADR에 남긴다.
플랫폼이 실제로 갈리는 지점에만 경계를 둔다 (§3.1의 `RecordingBackend`).

**하지 않는다**: Windows 환경 구축 · Windows 빌드 · Windows 실행 검증 · Windows 권한 로직.
전부 Phase 6이다. **추상화를 선입금하지 않는다 (§20.6)** — 구현이 하나뿐인 인터페이스를
"Windows에서 언젠가 필요하다"는 이유로 미리 만들지 않는다.

## Required Outcome

1. **Recording engine이 선택되고, 그 근거와 실제 검증 결과가 문서로 남는다.**
   UNVERIFIED였던 항목이 이 기기에서 어떻게 나왔는지가 포함되어야 한다.

2. **Microphone 열거와 선택** — 사용 가능한 입력 장치 목록을 보여주고 사용자가 고를 수 있다.
   Settings의 default microphone 값이 실제로 사용된다.

3. **macOS microphone 권한 흐름** — 권한을 요청하고, 거부된 상태를 감지하며,
   거부되었을 때 사용자가 무엇을 해야 하는지 화면에서 알 수 있다 (§13).
   Phase 1이 넣은 `NSMicrophoneUsageDescription`이 여기서 실제로 쓰인다.

4. **Record · Pause · Resume · Stop이 동작**하고, 경과 시간이 정확하다.
   Pause 구간은 duration에 포함되지 않아야 한다.
   Recording 화면은 §5-B와 §19를 따른다 — 녹음 상태와 시간이 가장 명확해야 한다.

5. **Stop 시 결과 파일이 filesystem에 실제로 생성된 것이 확인된다 (R-002).**
   "저장했다"는 서술이 아니라, 파일 존재와 크기가 확인되어야 한다.
   확인에 실패하면 그것은 사용자에게 보이는 실패다.

6. **Recording 레코드가 저장된다** — Phase 1의 영속성 계층을 통해 title · duration ·
   audioPath · audioFormat · microphone · createdAt이 기록된다.
   파일과 레코드가 어긋나는 상태(레코드는 있는데 파일이 없음)를 감지할 수 있어야 한다.

7. **재생** — Recording Detail에서 저장된 audio를 재생할 수 있다.

8. **Recording state machine이 하드웨어 없이 자동 테스트된다.**
   idle → recording → paused → recording → stopped 전이, 잘못된 전이 거부,
   pause를 고려한 duration 누적이 테스트로 검증된다 (§18).
   이것이 가능하도록 상태 기계와 실제 캡처 장치를 분리해서 설계한다.

9. **§13의 녹음 관련 실패가 제품 상태로 다뤄진다** — permission denied ·
   microphone disconnected · initialization failure · write failure · disk write failure.
   최소한 permission denied와 초기화 실패는 실제로 UI에 표현되어야 한다.

10. **R-001** — 화면을 이동하거나 UI state가 바뀌어도 진행 중인 녹음이 유실되지 않는다.

11. build · lint · test Gate가 전부 통과한다.

## Important Rules

- **INV-3 / INV-4가 이 Phase의 핵심이다.** 어떤 실패도 이미 저장된 audio를 지우지 않는다.
  사용자가 명시적으로 삭제하기 전까지 원본은 남는다.
- 외부 라이브러리를 도입하기 전에 **현재 버전과 실제 지원 범위를 확인한다.**
  Spec §14.3의 값은 2026-09-01 기준이며, 그대로 신뢰하지 말고 도입 시점에 재확인한다.
- 커뮤니티 플러그인을 쓸 경우 유지보수 상태를 확인하고, 그 판단 근거를 기록한다.
- 자동으로 검증할 수 없는 것(실제 음질, 실제 마이크 입력)을 자동 PASS로 적지 않는다.
  Human Review 항목으로 명시한다.
- 녹음 실패를 감추기 위해 테스트를 약화시키지 않는다.

## Out of Scope

- 전사 · whisper · 오디오 전처리 파이프라인 (Phase 3)
- Claude · AI Note (Phase 4)
- Notion · Markdown export (Phase 5)
- 화자 분리 · 실시간 transcription (제품 non-goal, §15)
- 파형 시각화, 오디오 편집, 트리밍
- 백그라운드 녹음, 메뉴바 녹음 (DEFERRED, §16)
- 녹음 중 자동 후처리 트리거
- **Windows 환경 구축 · Windows 빌드 · Windows 실행 검증 · Windows 권한 로직** (Phase 6)

Phase 3이 쓸 수 있도록 audio 파일 경로와 포맷을 레코드에 정확히 남기는 것까지가 이 Phase다.
전사를 시작하는 것은 이 Phase가 아니다.

## Verification Boundary

- 실제 macOS 마이크로 녹음한 파일이 filesystem에 존재하고 재생된다.
- **앱을 완전히 종료한 뒤 다시 실행해도** 그 녹음이 목록에 있고 재생된다.
- Recording state machine과 duration 로직에 대한 자동 테스트가 통과한다.
- Pause 구간이 duration에서 제외된다는 것이 테스트로 확인된다.
- microphone permission이 거부된 상태가 UI에 표현된다.
- recording engine 결정이 근거와 함께 문서로 남아 있다.
- build / lint / test Gate가 green이다.

### Human Review 항목

- 실제 녹음된 오디오의 음질이 사람이 알아들을 수준인가
- 1시간 규모의 긴 녹음에서 안정적인가 (§17의 실사용 시나리오)
- 녹음 중 UI가 §19의 방향에 맞게 상태를 분명히 보여주는가
- macOS 권한 프롬프트가 실제로 뜨는가 — dev 실행과 번들된 `.app` 양쪽에서
  (Spec §14.3에 이것이 알려진 위험으로 기록되어 있다)
- webview 후보를 택했다면, 실제로 나오는 컨테이너/코덱이 무엇인가 (§14.3의 UNVERIFIED 항목)

## Source of Truth

`docs/PRODUCT-SPEC.md`, 특히 §2.1(INV-3 · INV-4 · INV-10) · §3(플랫폼) · §3.1(cross-platform 원칙) ·
§5-B · §6(R-001~R-005) · §6.1(엔진 결정 기준) · §7(데이터 모델) · §13(실패 처리) ·
§14.3(검증된 사실) · §18 · §19.

외부 라이브러리·API는 추측하지 말고 실제 현재 지원 범위를 확인한다.
확인할 수 없으면 UNVERIFIED로 남기고 그 사실을 드러낸다.

이 Phase 밖으로 나가지 않는다.
