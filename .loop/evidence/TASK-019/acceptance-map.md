# TASK-019 — Acceptance Criteria가 무엇으로 판정되는가

## AC1 — test Gate 통과 · 세 상태가 마이크 없이 판정되고, 거부에서 캡처가 시작되지 않는다

`test: PASS exit=0` (`gate-self-check.txt`).

권한 경계에 넣은 가짜 구현: `src-tauri/tests/microphone_permission.rs`의 `FakePermission`
(상태를 값으로 정하고, 물어본 횟수를 센다). 실제 마이크·실제 권한을 부르는 코드가 이 경로에 없다.

| 테스트 | 판정하는 것 |
| --- | --- |
| `a_denied_microphone_stops_the_capture_before_anything_is_opened_or_written` | **거부 → 캡처가 시작되지 않는다.** 장치를 열지 않고(`opened == 0`), `.wav`도 만들지 않고, session은 `idle`·`0ms`로 남는다. 권한을 장치보다 **먼저** 물었다(`asked == 1`) |
| `a_denied_microphone_tells_the_user_what_to_do_and_is_not_a_recording_setup_failure` | 거부가 **사용자에게 보이는 상태**가 된다 — `FailureKind::MicrophonePermission` · 무엇을 해야 하는지 담은 문장 · `retryable == false` · `AudioDevice`도 `Storage`도 아니다 |
| `a_granted_microphone_records_exactly_as_before` | **허용** → 장치가 열리고 파일이 실제로 만들어진다 |
| `allowing_access_afterwards_makes_the_next_start_work` | 거부가 앱에 눌어붙지 않는다. 시작할 때마다 새로 묻는다 |
| `an_undetermined_state_does_not_block_a_microphone_that_opens` | **미결정** → 막지 않는다 ("모른다"를 "거부"로 접지 않는다) |
| `an_undetermined_state_that_cannot_open_the_device_points_at_the_permission_setting` | 미결정 + 장치 열기 실패 → 권한 안내로 도달하고 원래 원인이 `detail`에 남는다 (ADR-0005 §4.2 폴백) |
| `a_device_that_fails_while_access_is_granted_stays_a_device_failure` | 허용된 상태의 장치 실패는 권한 안내로 바뀌지 않는다 |

단위 테스트 (`src-tauri/src/platform/microphone.rs`): 세 상태가 셋으로 남는지 · 거부 문장에 설정
경로가 있는지 · 분류 규칙이 `Storage` 실패와 허용 상태를 건드리지 않는지 · 시스템 구현이
관측하지 못하는 상태를 주장하지 않는지.

화면 쪽 (`src/screens/failureView.test.ts`): 권한 거부 실패가 화면 표현까지 **문장 그대로**
도달하고 "다시 시도해도 같은 결과"로 읽힌다.

경계 계약 (`tests/ipc-boundary.test.ts`, 기존 테스트): Rust의 실패 종류가 frontend `FailureKind`에
전부 있는지 — `microphonePermission`을 추가하지 않으면 이 테스트가 실패한다.

## AC2 — lint Gate

`lint: PASS exit=0` (eslint + `cargo clippy --all-targets -- -D warnings`).

## AC3 — build Gate

`build: PASS exit=0` (`tsc && vite build`).

## AC4 — 경계와 문서

| 요구 | 어디서 확인되는가 |
| --- | --- |
| (1) macOS 전용 지식이 platform 안에만 있고 domain에 없다 | `src-tauri/src/platform/microphone.rs`가 `cfg!(target_os = "macos")`와 설정 경로 문장을 가진 유일한 자리다. `src-tauri/src/domain/**`에는 macOS 문자열도 `cfg(target_os)`도 없다. `commands/mod.rs`는 `MicrophoneAccess::Denied`라는 **값**만 보고 문장은 platform에서 받는다 |
| (2) 거부 안내가 사용자가 읽을 수 있는 문장이다 | `마이크에 접근할 수 없다. 시스템 설정 › 개인정보 보호 및 보안 › 마이크에서 Molt Note의 접근을 허용한 뒤 다시 녹음을 시작해야 한다.` — 단위 테스트와 통합 테스트가 문장 내용을 단언한다 |
| (3) 새 의존성의 버전·지원 범위 기록 | **새 의존성을 도입하지 않았다** (`Cargo.toml` 무변경). 그 이유와 대신 무엇을 했는지가 ADR-0005 §4·§4.1 |
| (4) 자동으로 확인할 수 없는 것이 PASS로 적히지 않았다 | ADR-0005 §4 표(TCC 조회 crate의 현재 버전·지원 범위 = **UNVERIFIED**, 장치 열기 시 프롬프트 발생 = **UNVERIFIED**, 거부 시 cpal 오류 형태 = **UNVERIFIED**)와 §6 Human Review 표(실제 TCC 프롬프트 · 실제 거부 기기에서 보이는 것 · 문구 적절성). 코드 주석도 같은 경계를 적는다 |
| (5) Windows 권한 로직 없음 | macOS가 아닌 곳에서 `SystemMicrophonePermission`은 `Granted`를 돌려주고 아무것도 막지 않는다. Windows privacy 토글 관련 코드가 없다 (ADR-0005 §2.6 · §8) |

## 이 Run이 판정하지 않은 것 (Human Review)

- 번들된 앱에서 실제 macOS TCC 프롬프트가 뜨는지
- 마이크 권한을 실제로 끈 기기에서 사용자가 보는 것 (이때 우리가 `Denied`를 관측하는 것이
  아니라 §4.2 폴백 경로로 도달할 가능성이 높다)
- 안내 문장의 설정 경로 표기가 현재 macOS 버전과 일치하는지
