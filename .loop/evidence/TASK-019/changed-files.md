# TASK-019 — 변경한 파일

commit하지 않았다 (Task 요구). 아래는 working tree의 변경이다.

## 새로 만든 파일

| 경로 | 내용 |
| --- | --- |
| `src-tauri/src/platform/microphone.rs` | 마이크 권한 경계. `MicrophoneAccess`(Granted · Denied · Undetermined) · `MicrophonePermission` trait · `SystemMicrophonePermission` · `access_denied()` · `explain_open_failure()` · 안내 문장 상수 · 단위 테스트 6개 |
| `src-tauri/tests/microphone_permission.rs` | 가짜 권한 경계로 세 상태를 마이크 없이 지나는 통합 테스트 7개 |
| `docs/ADR-0005-microphone-permission.md` | 결정 · VERIFIED/UNVERIFIED 표 · 자동 판정 항목 · Human Review 항목 |

## 고친 파일

| 경로 | 내용 |
| --- | --- |
| `src-tauri/src/platform/mod.rs` | `pub mod microphone;` 추가, 모듈 지도 갱신 |
| `src-tauri/src/commands/mod.rs` | `Recorder`에 `microphone` 경계 필드 · `with_microphone()` 추가. `start()`가 **장치를 열기 전에** 권한을 묻고, `Denied`면 아무것도 만들지 않고 실패로 끝난다. 권한 미확정 상태의 장치 열기 실패는 `explain_open_failure`로 분류 |
| `src-tauri/tests/capture.rs` | 장치 실패를 보는 기존 두 테스트에 `GrantedPermission`을 명시(단언 변경 없음) |
| `src/ipc/failure.ts` | `FailureKind` union에 `microphonePermission` 추가 (Rust의 `FailureKind`와 1:1 · `tests/ipc-boundary.test.ts`가 회귀로 막는다) |
| `src/screens/failureView.test.ts` | 권한 거부 실패가 화면 표현까지 문장 그대로 도달하는지 보는 테스트 1개 추가 |

## 이 Task가 건드리지 않은 것

- `src-tauri/Cargo.toml` — **새 의존성을 추가하지 않았다.** 이유는 ADR-0005 §4에 있다.
- `src-tauri/src/domain/**` — macOS 지식이 들어가지 않았다 (INV-10).
  `FailureKind::MicrophonePermission` 자체는 이 Task 이전에 이미 domain에 있었고, 그 doc 주석이
  가리키던 `crate::platform::microphone`이 이번에 실제로 생겼다.
- Windows 권한 로직 — 만들지 않았다 (Phase 6).
- `.loop/` 아래의 Runtime 소유 파일.
