# TASK-023 검증 로그 — ADR-0007을 쓰면서 실제로 확인한 것

Run: RUN-20260903T022206Z-TASK-023 · 2026-09-03

이 Task는 문서만 만든다. 따라서 Evidence는 "테스트가 통과했다"가 아니라
**문서에 적은 사실이 어디서 왔는지**와 **저장소가 실제로 바뀐 범위**다.

---

## 1. 네트워크 접근 — 없었다 (이것이 근거 등급을 정한다)

upstream 사실을 이 Run에서 직접 재확인하려고 다음 두 URL을 가져오려 했고, **둘 다 거부됐다.**

```text
https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest   → 도구 권한 거부
https://crates.io/api/v1/crates/whisper-rs                          → 도구 권한 거부
```

결과 메시지: `Claude requested permissions to use WebFetch, but you haven't granted it yet.`

**그래서 ADR-0007은 외부 사실을 [E1] 직접 확인으로 올리지 않았다.** 전부 [E2]
(PRODUCT-SPEC §14.4.1이 **2026-09-03에** primary source에서 재확인한 기록)이거나 [E4]
UNVERIFIED다. §14.4.1의 확인 시점이 이 결정을 내리는 날과 같은 날이라는 사실을
문서 §3에 명시했다.

**추측으로 채운 값은 없다.** CLI 플래그·JSON 필드명·crate API 시그니처를 확인된 것처럼
적지 않았다 (ADR-0007 §5 · §10 · §14).

---

## 2. 저장소에서 직접 읽어 확인한 것 ([E1])

| 확인한 사실 | 파일 |
| --- | --- |
| raw recording은 장치가 정한 sample rate/channels의 PCM16 WAV다 (`CaptureFormat` · `hound::SampleFormat::Int` · `BITS_PER_SAMPLE`). 리샘플링/다운믹스 코드가 없다 | `src-tauri/src/audio/capture.rs` |
| `transcript_segments`가 `start_ms` · `end_ms` INTEGER를 갖는다. `recordings.transcription_status`는 `none/pending/running/done/failed` CHECK다. `transcripts`는 `recording_id`로 1:N이다 | `src-tauri/src/db/migrations.rs` (migration 2) |
| 현재 의존성에 whisper·rubato·symphonia가 **없다.** `hound 3` · `cpal 0.18` · `rusqlite 0.40` · `tauri 2 (protocol-asset)`만 있다 | `src-tauri/Cargo.toml` · `src-tauri/Cargo.lock` |
| `tauri.conf.json`에 `bundle.externalBin`이 **없다.** capabilities에 shell 권한이 없다 (`core:default`만) | `src-tauri/tauri.conf.json` · `src-tauri/capabilities/default.json` |
| `.gitignore`가 `/models/` · `*.gguf` · `*.bin`과 모든 오디오 확장자를 제외한다 | `.gitignore` |
| 기존 실패 계약은 `FailureKind { Storage, InvalidInput, AudioDevice, MicrophonePermission }`이며 전사 실패는 아직 없다 | `src-tauri/src/domain/failure.rs` |
| Gate는 build/lint/test 셋이고 lint·test timeout이 900초다 (ADR-0007 §4.3의 cold build 위험 근거) | `.loop/project.yaml` |
| Phase 2 ADR이 "16kHz mono는 장치가 정하며 이 저장소에 리샘플링 코드가 없다"를 이미 기록해 두었다 | `docs/ADR-0003-recording-engine.md` §4.2.3 · §6 · §12 항목 7 |

---

## 3. 변경 범위 — 문서 하나뿐 (AC7)

`git status --porcelain` 실행 결과:

```text
?? .loop/tasks/TASK-023.yaml      ← Runtime이 만든 Task 파일 (이 Run 이전부터 untracked)
?? .loop/tasks/TASK-024.yaml
?? .loop/tasks/TASK-025.yaml
?? .loop/tasks/TASK-026.yaml
?? .loop/tasks/TASK-027.yaml
?? .loop/tasks/TASK-028.yaml
?? .loop/tasks/TASK-029.yaml
?? .loop/tasks/TASK-030.yaml
?? .loop/tasks/TASK-031.yaml
?? docs/ADR-0007-transcription-engine.md   ← 이 Run이 만든 유일한 파일
```

- **수정된 tracked 파일이 하나도 없다** (`M` 항목 없음).
- `src/**` · `src-tauri/**` · `tauri.conf.json` · `Cargo.toml` · `package.json` 모두 그대로다.
- `.loop/tasks/*.yaml`은 Runtime이 PLAN 승인 시점에 만든 파일이며 이 Run은 건드리지 않았다.
- Gate는 이 Task에 하나도 활성화돼 있지 않다 (`stop_condition.gates: []`) — 실행하지 않았다.

---

## 4. Acceptance Criteria가 문서의 어디에 있는가

| AC | 위치 |
| --- | --- |
| AC1 세 후보 비교·선택·탈락 이유, 금지된 세 근거를 쓰지 않음 | ADR-0007 §4 (특히 §4.0 · §4.2 · §4.4) · §12 |
| AC2 아티팩트 (a)(b)(c)(d) 구분, XCFramework≠CLI, 근거·시점·UNVERIFIED | ADR-0007 §5 |
| AC3 timestamp 실제 단위 + 정규화 단일 경계 | ADR-0007 §10 |
| AC4 모델 관리 정책 · 모델 없음 = 제품 상태 · 커밋 금지 | ADR-0007 §8 (§8.1 · §8.2 · §8.3) |
| AC5 입력 변환 책임·수단, 설치 요구 없음, INV-1·INV-3 | ADR-0007 §7 · §9 |
| AC6 #11992를 단정하지 않음 · Windows 평가 · Phase 6 경계 | ADR-0007 §6 · §11 |
| AC7 문서만 변경 | 이 파일 §3 |
