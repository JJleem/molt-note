# TASK-031 — 운영자 smoke test 절차 + Phase 3 검증 기록표 + ADR-0007 갱신 (문서 전용)

```text
Run:  RUN-20260903T051135Z-TASK-031
Date: 2026-09-03
Role: impl · 문서만 바꾸는 Task
```

## 1. 무엇을 만들었는가

### 새 문서 — `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`

- **§1~§9 운영자 절차**: 모델 획득(§3) · 놓을 자리(§4) · WAV 준비(§5) · 앱 실행과 Recording
  만들기(§6) · 모델 지정과 전사 시작(§7) · 성공 판정 네 가지(§8) · 실패 기록 양식(§9).
- **§10 기록표**: 자동 검증이 확인한 것(VERIFIED)과 확인되지 않은 것(NOT RUN · UNVERIFIED ·
  DEFERRED)을 나눠 적었다.
- **§11 실행 기록**: 전부 `NOT RUN`이다. **운영자가 실제로 실행한 뒤 채운다.**

### 갱신 — `docs/ADR-0007-transcription-engine.md`

- 헤더 Status를 "구현됨 · 실제 추론과 notarization은 여전히 미확인"으로 바꾸고 §16을 가리킨다.
- §4.3 Gate 비용 · §5 API 표면 · §8.2.4 provenance · §9.2 rubato · §14 표를 구현 결과에 맞게 갱신.
- **새 §16 — 구현 결과**: 결정이 어디에 실현됐는가(§16.1) · 계획과 달라진 것과 이유(§16.2) ·
  여전히 UNVERIFIED/DEFERRED인 것(§16.3) · 되돌리기 경로가 그대로인가(§16.4).

## 2. Acceptance Criteria 대응

| AC | 어디서 충족되는가 | 확인 방법 |
| --- | --- | --- |
| **AC1** 추가 추측 없이 실행 가능한 절차 | 새 문서 §2~§8 | 절차에 적힌 경로·명령·화면 문구를 전부 저장소 파일과 대조했다 — §3의 표 |
| **AC2** 품질 벤치마크가 아니라는 것과 불필요한 것 | 새 문서 §1 ("이것은 품질 벤치마크가 아니다" 표 — 실제 마이크 회의 녹음 · 1시간 오디오 · 한국어 품질 · 혼용 판정 · 성능 측정) | 문서를 읽는다 |
| **AC3** 확인/미확인 기록표 · 네 항목 DEFERRED · 미실행을 PASS로 적지 않음 | 새 문서 §10.1 (VERIFIED) · §10.2 (NOT RUN / UNVERIFIED / DEFERRED) · §11 (전부 `NOT RUN`) | 한국어 품질 · 한국어+영어 혼용 · timestamp와 음성 위치 일치 · 1시간 소요 시간이 **DEFERRED**로 적혀 있다. end-to-end 추론은 **NOT RUN**이다 |
| **AC4** ADR이 구현 결과와 일치하고 UNVERIFIED는 그대로 | ADR §16 · 갱신된 §14 | UNVERIFIED로 남긴 것: 번들 whisper.cpp 버전 · timestamp 단위 · 가속 여부 · in-process abort · release 빌드. DEFERRED: notarization · Windows · 품질 4항목 |
| **AC5** 문서만 변경 | `changed-files.md` · `doc-only-check.md` | 이 Run 시간대(14:20~14:23)에 mtime이 바뀐 파일은 `docs/` 두 개뿐이다 |

## 3. 절차에 적은 경로·명령·문구의 출처 (AC1 검증용)

저장소에서 직접 읽어 확인한 것만 절차에 적었다. **확인하지 못한 것(모델 다운로드 URL ·
macOS `say`/`afconvert`/`afinfo` 실행 결과)은 문서 안에서 `[E4] UNVERIFIED`로 표시했다.**

| 문서가 적은 것 | 저장소 근거 |
| --- | --- |
| 실행 명령 `npm run tauri dev` | `package.json`의 `"tauri": "tauri"` |
| 앱 데이터 디렉터리를 `molt-note.db`로 찾는다 | `src-tauri/src/platform/app_data_dir.rs`의 `DATABASE_FILE_NAME = "molt-note.db"` |
| identifier `com.moltnote.app` | `src-tauri/tauri.conf.json` |
| 모델 디렉터리 `<APP_DATA>/models` | 같은 파일의 `MODELS_DIR_NAME = "models"` |
| **모델 디렉터리를 앱이 만들지 않는다 → 운영자가 `mkdir`** | `ensure_models_dir()`의 호출자가 같은 파일의 테스트뿐이다 (`grep -rn ensure_models_dir src-tauri/src`) |
| 파일명 / 절대 경로 두 가지가 다 된다 | `src-tauri/src/transcription/model.rs`의 `resolve` + 테스트 두 개 |
| WAV 요구(PCM 16-bit int · 1~2채널 · 샘플레이트 무관 · 비어 있지 않음) | `src-tauri/src/transcription/audio_input.rs`의 `load()` |
| 녹음 파일 이름 `capture-<unix초>.wav` · 자리는 앱 데이터 디렉터리 | `src-tauri/src/audio/capture.rs`의 `file_stem`/`output_path` · `commands/mod.rs`의 `ensure_recordings_dir()` |
| **import 기능이 없다 → 레코드는 앱 녹음이 만든다** | `commands/mod.rs`에서 `audio_path`가 `capture.output_path`에서만 온다 |
| 화면 이름 Recordings · Recording · Settings | `src/navigation/routes.ts` |
| 버튼 Record / Stop | `src/screens/RecordingScreen.tsx` |
| 설정 항목 "Whisper model" · "Save" · 모델 없음 안내 문장 | `src/screens/SettingsScreen.tsx` · `src/screens/settingsView.ts` |
| 버튼 "Start transcription" · 진행 문장 · "Try transcription again" | `src/screens/transcriptView.ts` |
| 표시 형태 `HH:MM:SS → HH:MM:SS` · provenance `language · engine · model` | `transcriptView.ts`의 `formatTimestamp`/`RANGE_SEPARATOR` · `RecordingDetailScreen.tsx`의 done 분기 |
| `engine` 값 `whisper-rs/0.16` | `src-tauri/src/transcription/whisper.rs`의 `engine_id` |
| 실패 여섯 종류의 이름과 문장 | `src-tauri/src/domain/failure.rs` · `transcription/engine.rs` · `transcription/audio_input.rs` |
| PASS-3의 단위 확인 근거(계수 ×10이 한 자리에만 있다) | `src-tauri/src/transcription/parse.rs`의 `MILLISECONDS_PER_CENTISECOND` |
| sidecar 배치 절차가 없다는 것 | `tauri.conf.json`에 `bundle.externalBin` 없음 · `src-tauri/`에 `binaries/` 없음 · capabilities 권한은 `core:default`뿐 |
| 모델 크기 (`tiny` 75MiB · `base` 142MiB …) · HF 저장소 이름 | `docs/PRODUCT-SPEC.md` §14.4 |
| Gate가 green이었다는 기록 | `.loop/evidence/TASK-030/` (2026-09-03 · build·lint·test exit 0) |

## 4. Gate

이 Task의 `stop_condition.gates`는 비어 있다. **문서 두 개만 바뀌었으므로 build·lint·test의
입력이 달라지지 않는다** — self-check를 새로 돌리지 않았고, 그 사실을 Result의 `notes`에
적었다. 마지막으로 기록된 Gate 결과는 `.loop/evidence/TASK-030/`(2026-09-03, 셋 다 exit 0)다.

## 5. 이 Task가 하지 않은 것

- **운영자 smoke test를 실행하지 않았다.** 절차를 문서로 남겼을 뿐이다. 그래서 문서 어디에도
  "end-to-end 전사가 검증됐다"는 문장이 없고, §11은 전부 `NOT RUN`이다.
- `docs/SYSTEM-MAP.md`를 건드리지 않았다 — Phase가 최종 DONE이 된 뒤 운영자가 한다.
- 소스 · `tauri.conf.json` · `Cargo.toml` · `package.json`을 건드리지 않았다.
