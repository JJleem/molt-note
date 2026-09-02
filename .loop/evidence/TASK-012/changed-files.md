# TASK-012 — 변경 파일

`git status --porcelain` / `git diff --stat` 기준 (commit하지 않았다 — Task가 금지한다).
working tree에는 TASK-010 · TASK-011의 미커밋 결과도 함께 있다. 아래는 **TASK-012가
건드린 것**만 골라 적은 것이다.

## 이번 attempt(2)가 직접 편집한 파일

attempt 1이 timeout으로 죽으며 남긴 **컴파일되지 않는 코드**를 고쳤다. 자세한 내용은
`gates.md`의 마지막 절.

- `src-tauri/src/audio/system_capture.rs` — `cpal` 0.18 API에 맞춤 (4곳)
- `src-tauri/src/audio/capture.rs` — 테스트의 non-ASCII byte string literal (3곳)
- `src-tauri/src/platform/app_data_dir.rs` — 같은 이유 (2곳)

## TASK-012 범위의 전체 변경

새 파일:

- `src-tauri/src/audio/capture.rs` — 경로 결정 · 포맷 기술 · WAV 확정 · 크기 읽기 ·
  `SampleSource` 경계. 하드웨어 없이 테스트되는 쪽 전부.
- `src-tauri/src/audio/system_capture.rs` — 실제 장치를 여는 유일한 자리 (`cpal`).
- `src-tauri/tests/capture.rs` — 가짜 샘플 소스로 start → stop → 확정 → 보고를 지나는
  통합 테스트 9개.

수정한 파일:

- `src-tauri/src/audio/mod.rs` — `capture` · `system_capture` 모듈과 재수출.
- `src-tauri/src/commands/mod.rs` — `Capture` 상태와 `start_capture` · `stop_capture` command.
- `src-tauri/src/commands/payload.rs` — `CaptureReportPayload` (wire 형태).
- `src-tauri/src/lib.rs` — `Capture::open_for(app)` 등록, handler 2개 추가.
- `src-tauri/src/platform/app_data_dir.rs` — `recordings_dir()` · `ensure_recordings_dir()`.
  **앱 데이터 경로를 파생시키는 자리는 계속 이 파일 하나다.**
- `src-tauri/Cargo.toml` · `Cargo.lock` — `hound = "3"` (WAV writer).
- `src/ipc/types.ts` — `CaptureReport` 타입.
- `src/ipc/commands.ts` — `startCapture` · `stopCapture`.
- `tests/ipc-boundary.test.ts` — 등록 command 집합에 두 이름 추가, 범위 밖 이름 차단 유지.

## diffstat (tracked 파일, TASK-010 · 011 · 012 합계)

```
 src-tauri/Cargo.lock                   | 284 ++++++++++++++++++++++++++++--
 src-tauri/Cargo.toml                   |   8 +
 src-tauri/src/commands/mod.rs          | 179 ++++++++++++++++++++-
 src-tauri/src/commands/payload.rs      |  68 ++++++++
 src-tauri/src/domain/failure.rs        |  52 +++++-
 src-tauri/src/lib.rs                   |  15 +-
 src-tauri/src/platform/app_data_dir.rs |  94 ++++++++++-
 src/ipc/commands.ts                    |  45 +++++-
 src/ipc/failure.ts                     |   4 +-
 src/ipc/types.ts                       |  36 +++++
 tests/ipc-boundary.test.ts             |  28 +++-
 11 files changed, 775 insertions(+), 38 deletions(-)
```

untracked (신규): `src-tauri/src/audio/` · `src-tauri/tests/capture.rs` ·
`src-tauri/tests/input_devices.rs`(TASK-011) · `docs/ADR-0003-recording-engine.md`(TASK-011)
