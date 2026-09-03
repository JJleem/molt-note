# TASK-024 — 변경한 파일

`git status --porcelain` (이 Task와 무관한 기존 미추적 항목은 제외):

```text
 M src-tauri/Cargo.lock
 M src-tauri/Cargo.toml
 M src-tauri/src/lib.rs
?? src-tauri/src/transcription/
```

## 새로 만든 것

| 경로 | 줄 수 | 역할 |
| --- | --- | --- |
| `src-tauri/src/transcription/mod.rs` | 18 | 전사 경계 모듈 루트 |
| `src-tauri/src/transcription/audio_input.rs` | 876 | 원본 녹음 → 16 kHz mono f32 파생 입력 (테스트 21개 포함) |

## 고친 것

### `src-tauri/Cargo.toml`

```diff
@@ -36,4 +36,8 @@ cpal = "0.18"
  # 알려진 한계: 3.5.1은 2023-09 이후 릴리스가 없다(dormant) — ADR-0003 §9.
  hound = "3"
+# 샘플레이트 변환. ADR-0007 §9.2가 고른 순수 Rust 경로의 나머지 절반이다
+# (`hound` 읽기 + `rubato` 리샘플 + 수동 다운믹스). 외부 도구를 요구하지 않는 것이
+# 선택 이유이며, 그래서 제품 경로는 ffmpeg을 부르지 않는다 (§14.4.2 · ADR-0007 §7).
+rubato = "5"
```

**추가한 crate는 `rubato` 하나다.** `hound`는 Phase 2가 이미 넣어 둔 것을 읽기용으로
재사용했다 (ADR-0007 §9.2). ffmpeg 같은 외부 도구를 요구하는 경로는 만들지 않았다.

### `src-tauri/src/lib.rs`

```diff
@@ -3,6 +3,7 @@ pub mod commands;
  pub mod db;
  pub mod domain;
  pub mod platform;
+pub mod transcription;
```

### `src-tauri/Cargo.lock`

`rubato` 5.0.0과 그 의존성이 잠겼다 (+136줄). 손으로 고치지 않았다 — cargo가 썼다.

```text
name = "rubato"
version = "5.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a7cb1ffaf8738df50aab642a7f6465df81c6ba9e2818268053487165298114be"
```

## 건드리지 않은 것

- 기존 소스 파일(`src-tauri/src/audio/**` · `src-tauri/src/domain/**` · `src/**`) — 한 줄도
  고치지 않았다. 특히 `domain/failure.rs`와 `src/ipc/failure.ts`의 `FailureKind` 계약은
  **넓히지 않았다** (`verification-log.md` §6).
- 다른 Task 파일 · `.loop/policies/**` · `.loop/project.yaml` · `.loop/KERNEL.md` ·
  `.loop/DESIGN.md`
- `docs/ADR-0007-transcription-engine.md` — 이 Task의 범위가 아니다. rubato 5.0.0의 실제
  API를 빌드로 확인한 사실은 `verification-log.md` §1과 `resample()`의 doc comment에 남겼다.
- 오디오 파일 — 저장소에 하나도 추가하지 않았다.
