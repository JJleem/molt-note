# Phase 6 — Cross-platform Validation & Hardening (Windows)

Implement Phase 6 of `docs/PRODUCT-SPEC.md`.

## Goal

Phase 1~5가 macOS에서 증명한 기능들이 **Windows에서도 실제로 동작하는지 검증하고,
동작하지 않는 부분을 고친다.**

이 Phase의 성공 기준:

> Windows x64에서 `docs/PRODUCT-SPEC.md` §3.1의 핵심 기능이 실제로 동작한다 —
> Record · Pause · Resume · Stop · Local Audio Storage · Playback ·
> Local Transcription · AI Note Generation · Markdown Export · Notion Sync.

## Why This Phase Exists

Windows는 **지원 대상 플랫폼**이다 (§3). non-goal이 아니다.

그러나 플랫폼 검증은 **검증할 기능이 존재한 뒤에만** 의미가 있다.
Phase 1~5는 각 Phase에서 "Windows를 차단하지 않는다"만 지켰고(§3.1),
실제로 Windows에서 돌려본 적은 없다. 그 격차를 여기서 닫는다.

**이 Phase는 새 제품 기능을 만드는 곳이 아니다.** 이미 있는 기능이 두 번째 플랫폼에서
성립하는지 확인하고, 성립하지 않는 지점을 고치는 곳이다.

## Preconditions

Phase 1~5가 모두 DONE이어야 한다. 미완성 Phase의 기능을 여기서 구현하지 않는다.

**이 Phase는 실제 Windows x64 환경을 필요로 한다.** §14.1 기준 그 환경은 아직 없다.
환경 확보 자체가 이 Phase의 첫 작업이며, 확보되지 않으면 이 Phase는 진행할 수 없다 —
그 사실을 감추지 않는다.

## Required Outcome

1. **Windows 개발/빌드 환경이 확보되고 문서화된다.**
   §14.2의 확인된 요구사항을 근거로 한다 — WebView2 런타임(Windows 10 1803+ / 11에
   기본 설치), Microsoft C++ Build Tools, `rustup default stable-msvc`.

2. **Windows에서 빌드가 성공한다.** Gate 명령(build · lint · test)이 Windows에서도
   실행 가능한지 확인하고, 플랫폼 차이로 깨지는 것이 있으면 고친다.

3. **§3.1의 핵심 기능이 Windows에서 실제로 동작한다.**

   ```text
   Record · Pause · Resume · Stop
   Local Audio Storage · Playback
   Local Transcription
   AI Note Generation
   Markdown Export · Notion Sync
   ```

   각 항목에 대해 **동작함 / 동작하지 않음 / 미검증**을 정직하게 기록한다.

4. **경로 처리가 Windows에서 올바르다.** §14.2에 확인된 대로 Tauri가
   `PathResolver::app_data_dir()` / `appDataDir()`를 제공하며,
   Windows에서 `{FOLDERID_RoamingAppData}/{bundleIdentifier}`로 해석된다.
   Phase 1이 세운 `AppDataDirectory` 경계(§3.1)가 실제로 제 역할을 하는지 확인한다.
   경로 구분자·대소문자·경로 길이 제한에서 깨지는 곳이 있으면 고친다.

5. **Recording engine이 Windows에서 동작한다.**
   Phase 2가 선택한 엔진(§14.3)이 Windows에서 성립하는지 확인한다.
   - cpal 계열이면 백엔드가 **WASAPI**로 바뀐다 (§14.3).
   - webview 계열이면 webview가 **WebView2(Chromium)** 로 바뀌며,
     **출력 컨테이너/코덱이 macOS와 다를 수 있다** — 다르면 Phase 3의 전사 입력이 깨진다.
   차이가 발견되면 그것이 이 Phase의 핵심 작업이다.

6. **Windows microphone 권한 동작을 확인한다.** §14.3에 확인된 사실:
   일반 Win32 앱에는 manifest 선언이 불필요하지만,
   **Settings › Privacy & Security › Microphone 의 "Let desktop apps access your microphone"
   토글이 OS 수준에서 차단할 수 있다.**
   차단됐을 때 앱이 받는 오류 형태는 **UNVERIFIED**이며 여기서 실측한다.
   차단 상태가 §13에 따라 사용자에게 보이는 상태로 표현되어야 한다.

7. **Transcription 바이너리가 Windows에서 확보·실행된다.**
   §14.4에 확인된 대로 whisper.cpp는 **Windows x64 prebuilt를 제공한다**
   (`whisper-bin-x64.zip` 등). sidecar 방식이면 target triple 접미사가
   `-x86_64-pc-windows-msvc.exe`로 **`.exe`를 포함한다.**

8. **AI Provider가 Windows에서 동작한다.** Ollama 연결(§14.5)이 Windows에서도 성립하는지
   확인한다. 기본 주소와 CORS 정책은 플랫폼 무관하나, 실제 확인은 별개다.

9. **패키징 결정을 내리고 기록한다.** §14.2에 확인된 대로 Tauri v2 Windows 번들 타깃은
   `msi`(WiX v3, **Windows에서만 빌드 가능**)와 `nsis`(교차 빌드 가능)다.
   **`msi`는 Windows 머신 없이 만들 수 없다** — 이것이 릴리스 절차에 주는 영향을 기록한다.

10. **플랫폼 차이가 발견된 곳에만 경계를 보강한다.** 이 Phase에서 발견된 실제 차이가
    §3.1의 경계 후보와 다르면, **실제 차이를 따른다.** 예상했지만 실제로는 차이가 없던
    지점에 추상화를 남겨두지 않는다 (§20.6).

11. **`docs/SYSTEM-MAP.md`와 `docs/PRODUCT-SPEC.md`가 실제 Windows 지원 상태를 반영한다.**
    동작하지 않는 것을 동작한다고 적지 않는다.

12. build · lint · test Gate가 **양쪽 플랫폼에서** 통과한다.

## Important Rules

- **이 Phase는 새 제품 기능을 만들지 않는다.** 두 번째 플랫폼에서의 성립을 확인하고 고친다.
- **macOS 동작을 깨뜨리면서 Windows를 고치지 않는다.** 양쪽이 함께 green이어야 한다.
- 플랫폼 분기를 발견된 실제 차이에만 넣는다. "혹시 다를 수 있으니" 분기하지 않는다.
- **동작하지 않는 것을 동작한다고 적지 않는다.** 미검증 항목은 미검증으로 남긴다.
  Windows 환경을 확보하지 못했다면 그 사실이 이 Phase의 결과다.
- §14의 값은 2026-09-01 기준이다. 도입 시점에 재확인한다.
- **Git commit / push는 이 Phase의 작업이 아니다.** Phase commit은 Phase가 완료되고
  검증된 뒤 운영자가 한다 (`docs/GIT-WORKFLOW.md`). Worker가 commit하면 HEAD가 바뀌어
  Gate·Verifier가 묶여 있는 subject fingerprint가 실행 도중에 흔들린다.
  저장소에 파일을 만들고 고치는 것까지가 Task의 일이다.

## Out of Scope

- Linux · 모바일 지원 (§15 non-goal)
- Microsoft Store 배포 · 코드서명 · notarization (§3에서 범위 밖)
- Windows 전용 신규 기능
- Windows 전용 UI 분기 (§19 — 같은 디자인 방향을 유지한다)
- Cloud AI Provider (DEFERRED, §16)
- macOS 재설계

## Verification Boundary

- Windows에서 build · lint · test Gate가 통과한다.
- macOS에서도 여전히 전부 통과한다.
- §3.1의 각 핵심 기능에 대해 Windows 동작 여부가 **증거와 함께** 기록되어 있다.
- 발견된 플랫폼 차이와 그 해결책이 문서로 남아 있다.
- `docs/SYSTEM-MAP.md`가 실제 Windows 지원 상태를 정확히 반영한다.

### Human Review 항목

거의 전부가 사람 확인이다. **이 Phase는 자동 판정 비중이 가장 낮은 Phase다.**

- Windows에서 실제 마이크로 녹음한 오디오의 음질
- Windows에서 실제 재생 음질
- Windows에서 1시간 녹음의 안정성
- Windows에서 전사 결과와 소요 시간
- Windows에서 생성된 Markdown / Notion 페이지
- Windows에서의 UI 체감이 §19의 방향에 맞는가
- 마이크 privacy 토글을 껐을 때의 앱 동작

## Source of Truth

`docs/PRODUCT-SPEC.md`, 특히 §3(플랫폼) · §3.1(cross-platform 원칙) · §2.1(INV-10) ·
§13(실패 처리) · §14.2 · §14.3 · §14.4 · §14.5 · §17.3 · §18 · §19 · §20.6.

이 Phase 밖으로 나가지 않는다.
