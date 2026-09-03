# ADR-0005 — 마이크 권한은 platform 경계 뒤의 세 상태 값이고, 지금은 그 상태를 안다고 주장하지 않는다

```text
Status:   Accepted (판정 수단은 PROVISIONAL)
Date:     2026-09-02
Phase:    Phase 2 — Reliable Recording
Task:     TASK-019
Scope:    마이크 권한 흐름의 위치 · 상태 표현 · 거부 안내 · 확인한 것과 확인하지 못한 것의 경계
```

---

## 1. Context

`docs/PRODUCT-SPEC.md` §13은 `microphone permission denied`를 **사용자에게 보여야 하는 정상적인
제품 상태**로 규정한다. §3.1은 그 지식이 놓일 자리를 `PlatformPermissions` 경계로 정하고,
INV-10은 **macOS 전용 지식이 domain으로 새지 않을 것**을 요구한다.

`docs/ADR-0002-macos-microphone-usage-description.md`는 packaging 선언(`NSMicrophoneUsageDescription`)
까지만 했고, 권한 요청·거부 감지·안내는 **명시적으로 하지 않았다**(ADR-0002 §3). 이 문서가 그
나머지를 다룬다.

## 2. Decision

1. 권한 상태를 **세 값**으로 표현한다 — `Granted` · `Denied` · `Undetermined`
   (`src-tauri/src/platform/microphone.rs`의 `MicrophoneAccess`).
   **"모른다"를 "허용"으로도 "거부"로도 접지 않는다.** 셋은 사용자가 할 일이 서로 다르다.
2. 그 값을 어디서 얻는지는 `MicrophonePermission` trait 뒤에 둔다. 앱은
   `SystemMicrophonePermission`을 쓰고, 테스트는 자신의 구현을 넣어 **마이크도 실제 권한도 없이**
   세 상태를 전부 지난다 (§18).
3. **캡처를 시작하기 전에 권한을 묻는다** (`Recorder::start`). `Denied`면 디렉터리도 파일도
   만들지 않고 장치도 열지 않은 채 실패로 끝난다.
4. 거부 상태는 domain 공통 실패 `FailureKind::MicrophonePermission`으로 표현한다.
   **`AudioDevice`(장치 문제)와도 `Storage`(녹음 초기화 문제)와도 다른 값이다** — 사용자가 할
   수 있는 일이 다르기 때문이다.
5. **"어디서 무엇을 켜야 하는가"는 platform 경계만 안다.** 안내 문장(`시스템 설정 › 개인정보
   보호 및 보안 › 마이크`)은 `platform/microphone.rs`의 상수이며, domain에도 화면에도 그 지식이
   없다 (INV-10). 화면은 `Failure::message`를 그대로 그린다.
6. **Windows 권한 로직을 만들지 않는다** (Phase 6 · PRODUCT-SPEC §14.3). macOS가 아닌 곳에서
   이 경계는 아무 주장도 하지 않고 녹음을 막지 않는다.
7. **새 의존성을 도입하지 않는다.** 이유는 §4에 있다.

## 3. 왜 캡처 경로 안이 아니라 별도 경계인가

`crate::audio::system_capture`가 실제 장치를 여는 유일한 자리이지만, 그 파일은 **자동 테스트가
실행하지 않는다**(ADR-0003 §12). 권한 판정을 거기에 두면 "권한이 거부된 상태에서 녹음이
시작되지 않는다"를 확인하는 방법이 **실제로 마이크 권한을 꺼 보는 것뿐**이 된다.

`platform/clock.rs`와 같은 이유로 경계를 하나 더 만들었다. 그 자리에 값을 넣으면 같은 확인이
하드웨어 없이 끝난다 (`src-tauri/tests/microphone_permission.rs`).

## 4. 확인한 것과 확인하지 못한 것 (VERIFIED / UNVERIFIED)

이 Run에서 실제로 확인할 수 있었던 범위를 그대로 적는다. **추측한 것을 확인한 것처럼 적지
않는다.**

| 항목 | 상태 | 근거 |
| --- | --- | --- |
| `NSMicrophoneUsageDescription`을 `src-tauri/Info.plist`에 넣으면 Tauri CLI 생성값과 병합된다 | **VERIFIED** | PRODUCT-SPEC §14.3 · ADR-0002 (이전 Task에서 확인) |
| `cpal` 0.18은 macOS에서 CoreAudio 백엔드로 입력 캡처를 지원한다 | **VERIFIED** | PRODUCT-SPEC §14.3 (§14.3 표 · 이 저장소의 `Cargo.toml`이 `cpal = "0.18"`) |
| macOS TCC 권한 상태를 조회·요청하는 Rust crate의 **현재 버전과 실제 지원 범위** | **UNVERIFIED** | 이 Run은 네트워크 조회와 `cargo` 실행이 제한된 Worker 환경에서 수행됐다. `AVCaptureDevice.authorizationStatus(for:)` / `requestAccess(for:completionHandler:)`를 감싸는 crate(예: `objc2-av-foundation` 계열)의 **현재 버전·API 모양·빌드 가능 여부를 이 Run에서 확인하지 못했다** |
| macOS에서 입력 장치를 여는 순간 TCC 프롬프트가 뜬다 | **UNVERIFIED** | 문서로 확인하지 못했다. 코드 주석과 이 문서 모두 이것을 사실로 적지 않는다 |
| 권한이 거부된 상태에서 `cpal`이 돌려주는 **정확한 오류 형태** | **UNVERIFIED** | 실제 거부 상태의 기기에서만 알 수 있다. Human Review 항목(§6) |
| Windows privacy 토글이 차단할 때 앱이 받는 오류 | **UNVERIFIED** | PRODUCT-SPEC §14.3이 이미 UNVERIFIED로 적어 둔 항목. Phase 6 |

### 4.1 그래서 무엇을 했는가

확인하지 못한 판정 수단을 **있는 것처럼 쓰지 않았다.**

- 새 의존성을 추가하지 않았다. `Cargo.toml`은 이 Task에서 바뀌지 않았다. 버전과 지원 범위를
  확인하지 못한 crate를 도입하는 것은 "확인된 것처럼 적는 것"과 같기 때문이다.
- `SystemMicrophonePermission`은 macOS에서 `Undetermined`를 돌려준다. **상태를 안다고 주장하지
  않는다.**
- 그 대신 Task가 허용한 폴백을 쓴다 — **권한을 확정하지 못한 채 장치를 열지 못했다면 그 실패를
  권한 문제로 분류해 안내한다** (`explain_open_failure`). 그 분류가 항상 옳다는 보증은 없으며,
  그것이 이 경계의 알려진 한계다 (§6의 UNVERIFIED 항목).

### 4.2 폴백이 지키는 두 가지

폴백이 모든 실패를 권한 문제로 바꿔 버리면 사용자는 없는 문제를 고치려 하게 된다. 그래서 두
경우에만 분류를 바꾼다.

```text
권한 = Undetermined  AND  실패 = AudioDevice   →  권한 안내로 바꾼다 (원인은 detail에 남긴다)
그 밖의 모든 경우                              →  그대로 둔다
```

- 권한이 **허용된 것으로 확인된** 상태의 장치 실패는 건드리지 않는다 — 그것은 권한 문제가 아니다.
- 장치와 무관한 실패(파일을 만들지 못했다 = `Storage`)도 건드리지 않는다. 그래서 이 분류는
  **녹음 초기화 실패를 권한 실패로 바꿔 놓지 않는다.**
- 분류를 바꿔도 원래의 기술적 원인은 `Failure::detail`에 그대로 남는다.

## 5. 자동으로 판정되는 것 (test Gate)

`src-tauri/tests/microphone_permission.rs` — 마이크도 실제 권한도 없이 돈다.

| 검사 | 내용 |
| --- | --- |
| 거부 → 시작되지 않는다 | 장치를 열지 않고(`opened == 0`), 파일도 만들지 않고, session은 `idle`로 남는다 |
| 거부 → 순서 | 권한을 **장치보다 먼저** 묻는다 (`asked == 1` · `opened == 0`) |
| 거부 → 안내 | 실패 종류가 `MicrophonePermission`이고, 문장에 무엇을 해야 하는지가 있으며, `retryable == false` |
| 거부 ≠ 초기화 실패 | 같은 실패가 `AudioDevice`도 `Storage`도 아니다 |
| 허용 → 그대로 녹음된다 | 파일이 실제로 만들어진다 |
| 나중에 허용 → 다음 시작이 된다 | 거부는 앱에 눌어붙지 않는다 |
| 미결정 → 막지 않는다 | 장치가 열리면 녹음이 시작된다 |
| 미결정 + 장치 실패 → 안내 | 권한 안내로 도달하고 원래 원인이 `detail`에 남는다 |
| 허용 + 장치 실패 → 장치 실패 | 권한 안내로 바뀌지 않는다 |

`src-tauri/src/platform/microphone.rs`의 단위 테스트가 문장과 분류 규칙을,
`src/screens/failureView.test.ts`가 그 문장이 화면 표현까지 그대로 도달하는지를,
`tests/ipc-boundary.test.ts`가 Rust의 실패 종류가 frontend 타입에 있는지를 본다.

## 6. 사람이 확인해야 하는 항목 (자동 PASS로 적지 않는다)

| 항목 | 왜 자동이 아닌가 |
| --- | --- |
| 번들된 앱에서 **실제 TCC 프롬프트가 뜨는가** | 실제 기기·번들·OS 상태가 필요하다. Tauri #11951(dev 실행에서 프롬프트 미표시)과도 얽혀 있다 (§14.3) |
| 마이크 권한을 **끈 상태**에서 시작했을 때 사용자가 실제로 보는 것 | 실제 거부 상태의 기기가 필요하다. 이때 `cpal`이 돌려주는 오류 형태가 UNVERIFIED이므로, 우리가 `Denied`를 보는 것이 아니라 §4.2의 폴백 경로로 도달할 가능성이 높다 |
| 안내 문장이 사용자에게 납득 가능한가 · 설정 경로 표기가 현재 macOS 버전과 맞는가 | 문구의 적절성은 사람의 판단이다 |
| TCC 상태를 직접 조회하는 crate를 도입할지 | §4의 UNVERIFIED를 해소하려면 crate의 현재 버전·지원 범위를 확인할 수 있는 환경에서 결정해야 한다 |

## 7. Consequences

- 권한 거부는 이제 **캡처를 시작하기 전에** 걸러지며, 그 상태에서 파일이나 디렉터리가 생기지 않는다.
- 화면은 실패 종류로 권한 문제를 다른 실패와 구분할 수 있고, 문장을 다시 쓰지 않아도 된다.
- **권한 상태를 직접 조회하지 못하는 동안** macOS 사용자는 §4.2의 폴백 경로로 안내를 받는다.
  이 경로는 장치 문제와 권한 문제를 구분하지 못하며, 문장도 구분한 척하지 않는다.
- §4의 UNVERIFIED가 해소되면 바뀌는 것은 `SystemMicrophonePermission` 하나다. 경계 바깥의
  코드와 테스트는 그대로 남는다.

## 8. 이 결정이 다루지 않는 것

- Windows 마이크 privacy 토글 동작 (Phase 6)
- macOS entitlements · 코드 서명 · 공증
- 권한 요청을 사용자에게 다시 물어보는 재요청 UI 흐름
