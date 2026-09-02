# ADR-0002 — `NSMicrophoneUsageDescription`은 `src-tauri/Info.plist`에 두는 macOS packaging 선언이다

```text
Status:   Accepted
Date:     2026-09-02
Phase:    Phase 1 — Application Foundation
Task:     TASK-009
Scope:    macOS 번들 선언의 위치와 성격 · 이 Phase가 구현하지 않는 것의 경계
```

---

## 1. Context

`docs/PRODUCT-SPEC.md` §14.3의 "마이크 권한 — 플랫폼 차이" 표는 다음을 확인해 두었다.

| 플랫폼 | 확인된 사실 |
| --- | --- |
| macOS | `src-tauri/Info.plist`에 `NSMicrophoneUsageDescription`을 넣으면 Tauri CLI 생성값과 **병합**된다 (VERIFIED · v2.tauri.app/distribute/macos-application-bundle). **`tauri.conf.json` 키가 아니다.** |
| Windows | 일반 Win32 데스크톱 앱에는 **manifest 선언이 불필요하다.** |

같은 절은 이 선언을 이렇게 규정한다 — `NSMicrophoneUsageDescription`은 **macOS packaging
요구사항**이지 Molt Note의 범용 권한 구현이 아니다. `phase-prompt/01-application-foundation.md`
Required Outcome 7도 같은 경계를 요구한다.

이 문서는 그 선언을 어디에 두었고, 무엇이 구현됐고 **무엇이 구현되지 않았는지**를 기록한다.

## 2. Decision

1. `NSMicrophoneUsageDescription`을 **`src-tauri/Info.plist`** 에 선언한다.
   `tauri.conf.json`에는 넣지 않는다 — 그 파일의 키가 아니므로 넣어도 번들에 반영되지 않는다.
2. 설명 문구는 macOS가 TCC 권한 프롬프트에 그대로 보여 주는 한국어 문장으로 둔다.
   무엇을 위해 마이크를 쓰는지, 언제 녹음이 시작되는지, 오디오가 어디에 남는지를 말한다.
3. 이 Task는 **선언만** 한다. 권한을 요청하거나 입력 장치를 열거하는 코드는 만들지 않는다.
4. Windows 쪽 권한 로직은 만들지 않는다.

파일 자체(`src-tauri/Info.plist`)의 주석에도 같은 경계를 적어 두었다.

## 3. 이것은 microphone permission 구현이 **아니다**

명시적으로 적는다. 이 Task 이후에도 Molt Note에는 다음이 **없다.**

- 마이크 권한 요청 흐름
- 권한이 거부된 상태의 감지 · 재요청 안내
- 입력 장치 열거 · 선택
- 오디오 캡처

이 선언이 하는 일은 하나다: **번들된 앱이 마이크에 접근하려 할 때 macOS가 띄우는 프롬프트에
표시할 설명 문구를 미리 준비해 두는 것.** 선언이 없으면 macOS는 프롬프트를 띄우는 대신
접근을 거부하고 앱을 종료시킨다. 즉 이것은 **Phase 2가 실제 권한 흐름을 만들 때 필요한
packaging 전제 조건**이지, 권한 기능 그 자체가 아니다.

실제 권한 요청 · 거부 상태 표현 · 장치 열거는 `phase-prompt/02-reliable-recording.md`가 다룬다.
Windows의 마이크 privacy 토글 동작은 `phase-prompt/06-cross-platform-validation.md`가 다룬다.

## 4. 왜 `Info.plist`이고 `tauri.conf.json`이 아닌가

Tauri v2 macOS 번들에서 `Info.plist`는 CLI가 생성한다. `src-tauri/Info.plist`가 존재하면
CLI는 그것을 **생성값과 병합**한다. 반면 `tauri.conf.json`에는 임의의 plist 키를 넣는 자리가 없다
(`bundle.macOS`가 받는 것은 `entitlements` · `minimumSystemVersion` 같은 정해진 항목이다).

따라서 `tauri.conf.json`에 `NSMicrophoneUsageDescription`을 적으면 **조용히 아무 효과가 없다.**
`tests/macos-bundle.test.ts`가 그 오배치를 회귀로 막는다.

entitlements(예: sandbox 환경의 `com.apple.security.device.audio-input`)는 이 선언과 **별개의
메커니즘**이며 `bundle.macOS.entitlements`가 다룬다. 이 Task의 범위가 아니다.

## 5. 검증 — 자동으로 판정되는 것과 사람이 확인해야 하는 것

### 5.1 자동 (test Gate)

`tests/macos-bundle.test.ts`가 파일을 읽어 다음을 검사한다.

| 검사 | 내용 |
| --- | --- |
| 선언 존재 | `src-tauri/Info.plist`에 `NSMicrophoneUsageDescription` 키가 있다 |
| 문구 비어 있지 않음 | 그 키의 `<string>` 값이 공백이 아니다 |
| 오배치 방지 | `tauri.conf.json`에 같은 키가 들어가 있지 않다 |

### 5.2 사람 확인 항목 (자동 PASS로 적지 않는다)

| 항목 | 왜 자동이 아닌가 |
| --- | --- |
| **번들된 `.app`의 `Contents/Info.plist`에 이 키가 실제로 병합되는가** | 확인하려면 `npm run tauri build`로 번들을 만들어 산출물을 열어야 한다. **`tauri build`는 이 프로젝트의 Gate가 아니다**(`.loop/project.yaml`의 Gate는 build · lint · test 셋뿐이며 `npm run build`는 프론트엔드 빌드다). 따라서 이 Run에서 병합은 **UNVERIFIED**다 |
| 권한 프롬프트 문구가 사용자에게 납득 가능한가 | 문구의 적절성은 사람의 판단이다 (`phase-prompt/01-application-foundation.md` Human Review 항목) |
| 번들 실행 시 TCC 프롬프트가 실제로 뜨는가 | §14.3이 참조하는 Tauri #11951(dev 실행에서 프롬프트 미표시)과 얽혀 있고, 실제 캡처 코드가 있어야 확인 가능하다 — Phase 2 |

확인 방법을 남겨 둔다. 사람이 검증할 때는 번들을 만든 뒤:

```bash
npm run tauri build
plutil -p "src-tauri/target/release/bundle/macos/Molt Note.app/Contents/Info.plist" \
  | grep NSMicrophoneUsageDescription
```

## 6. Consequences

- Phase 2가 마이크 캡처를 붙일 때 packaging 쪽 전제 조건은 이미 갖춰져 있다.
- 문구를 바꾸면 사용자가 보는 프롬프트가 바뀐다. 문구 변경은 Human Review 대상이다.
- 자동 테스트는 **파일에 선언이 있다**까지만 보증한다. **번들에 반영된다**는 보증이 아니다.
  이 구분을 흐리지 않는다.

## 7. 이 결정이 다루지 않는 것

- 권한 요청 · 거부 처리 · 장치 열거 · 오디오 캡처 (Phase 2)
- Windows 마이크 권한 동작 (Phase 6)
- macOS entitlements · 코드 서명 · 공증
