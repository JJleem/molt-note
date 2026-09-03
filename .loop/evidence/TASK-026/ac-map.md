# TASK-026 — Acceptance Criteria 대응표

```text
Date: 2026-09-03
Task: TASK-026 — 실제 whisper를 실행하는 경계
```

| AC | 무엇으로 판정되는가 | 결과 |
| --- | --- | --- |
| AC1 build | Gate `npm run build` | **PASS · exit 0 · 0.9s** (`gate-results.md`) |
| AC2 lint | Gate `npm run lint` | **PASS · exit 0 · 1.0s** |
| AC3 test | Gate `npm run test` | **PASS · exit 0 · 33.3s** |
| AC4 | 아래 §1 | 네 실패가 서로 다른 `FailureKind`이고 각각 테스트가 있다 |
| AC5 | 아래 §2 | 모델도 whisper 실행도 없이 세 Gate가 전부 통과했다 |
| AC6 | 아래 §3 | 모델·바이너리를 커밋하지 않았고 둘 자리를 새로 만들지도 않았다 |
| AC7 | 아래 §4 | 네트워크 경로 없음 · 플랫폼 분기는 기존 경계 하나 |

---

## §1 (AC4) 네 가지가 서로 구분되는 제품 실패로 돌아온다

`FailureKind`(`src-tauri/src/domain/failure.rs`)에 네 종류를 추가했다. **하나로 뭉치지
않은 이유는 사용자가 할 수 있는 일이 넷 다 다르기 때문이다** (§13).

| 실패 | `FailureKind` | 만드는 곳 | `retryable` | 테스트 |
| --- | --- | --- | --- | --- |
| 모델 파일이 없다 | `transcriptionModelMissing` | `model::resolve` | `false` | `no_model_configured_is_a_product_state_not_a_silent_skip` · `a_model_that_is_not_there_is_missing_rather_than_unusable` · `a_missing_model_is_a_defined_failure_rather_than_a_skipped_test` |
| 읽을 수 없거나 지원하지 않는 모델 | `transcriptionModelUnusable` | `model::resolve` · `whisper::WhisperEngine`의 모델 적재 | `false` | `a_directory_in_the_place_of_a_model_is_unusable_rather_than_missing` · `an_empty_file_is_unusable_rather_than_a_model` · `an_unusable_model_is_told_apart_from_a_missing_one` |
| 엔진 실행 실패 / 비정상 종료 | `transcriptionEngineFailed` | `whisper` (state 생성 · `full()`) · double | `true` | `an_engine_that_fails_is_reported_as_a_retryable_product_failure` · `the_double_repeats_its_last_answer_for_later_calls` |
| 출력이 없거나 해석할 수 없다 | `transcriptionOutputUnusable` | `engine::ensure_usable` · `whisper`의 출력 읽기 | `false` | `an_engine_that_produced_nothing_is_a_distinct_product_failure` · `segments_without_usable_text_are_the_same_as_no_output` · `output_that_cannot_be_read_as_a_transcript_is_its_own_failure` |

넷이 절대 겹치지 않는다는 것 자체도 테스트한다 —
`the_four_transcription_failures_never_collapse_into_one` (단위) ·
`the_four_transcription_failures_reach_the_screen_as_four_different_kinds` (통합).

**화면까지 구분되어 도착한다.** `src/ipc/failure.ts`의 union에 같은 네 값이 있고,
`tests/ipc-boundary.test.ts`의 기존 테스트가 Rust와 frontend의 1:1을 강제한다.

**test double이 세 경로를 실제로 태운다** (Task 요구: 정상 출력 · 모델 없음 · 실행 실패):

```text
정상 출력   StubEngine::returning(raw)  → a_normal_run_returns_the_engine_output_untouched
모델 없음   model::resolve(dir, None)   → a_missing_model_is_a_defined_failure_rather_than_a_skipped_test
실행 실패   StubEngine::failing(...)    → an_engine_that_fails_is_reported_as_a_retryable_product_failure
```

double은 **계약을 우회하지 않는다** — 실제 엔진과 같은 `ensure_usable`을 통과한 값만
돌려준다 (`the_double_obeys_the_same_output_contract_as_the_real_engine`).

---

## §2 (AC5) 실제 whisper 바이너리·모델 없이 Gate가 전부 통과한다

- **모델 파일이 이 기기에 없다.** 앱 모델 디렉터리를 만들지도 않았다. 테스트가 쓰는 "모델"은
  임시 디렉터리의 16바이트짜리 파일이며 whisper가 그것을 읽는 일은 없다.
- **어떤 테스트도 실제 엔진을 호출하지 않는다.** `whisper::WhisperEngine::transcribe`를
  부르는 코드는 제품에도 테스트에도 아직 없다 (호출은 orchestration의 몫 · TASK-027).
  `whisper.rs`의 테스트 2개는 `engine_id()`와 스레드 수만 본다 — `WhisperContext`를 만들지
  않는다.
- **모델이 없을 때 skip하지 않는다.** `model::resolve`가 `transcriptionModelMissing`을
  돌려주고 테스트는 그 실패를 단언한다. `#[ignore]`도, 환경 변수 분기도, "모델이 있으면
  검사한다"는 조건도 없다 — 이 Task가 추가한 33개 테스트 중 ignored는 0개다.
- 그럼에도 **실제 구현은 Gate가 컴파일한다.** `whisper-rs`가 정식 의존성이므로 clippy
  (`--all-targets -D warnings`)와 `cargo test`가 `whisper.rs`를 매번 빌드한다. 이것이
  ADR-0007 §4.2가 B를 고른 이유다 — 사람이 손으로 옮겨 둔 파일 없이 저장소가 엔진을
  재현한다.

---

## §3 (AC6) 모델 파일·큰 바이너리가 저장소에 없다

- **커밋한 모델·바이너리가 없다.** 이 Task가 만든 파일은 전부 소스와 문서다
  (`gate-results.md`의 변경 파일 목록).
- **바이너리를 두는 디렉터리가 생기지 않았다.** ADR-0007은 sidecar(A)를 택하지 않았으므로
  `src-tauri/binaries/`도, target triple 파일명도 필요 없다. 없는 디렉터리에 대한 규칙을
  미리 넣지 않았다 (§20.6).
- **모델을 두는 자리는 저장소 밖이다** — 앱 데이터 디렉터리 아래 `models/`
  (`AppDataDirectory::models_dir`). 저장소 안이 아니므로 새 ignore 규칙이 필요하지 않다.
- 그래도 실수로 저장소에 들어오는 것은 기존 규칙이 막는다. **그 규칙이 사라지지 않도록
  테스트가 지킨다** — `model_files_and_large_binaries_stay_out_of_the_repository`가
  `.gitignore`에 `/models/` · `*.gguf` · `*.bin`이 있는지 확인한다.

---

## §4 (AC7) 네트워크 경로 없음 · 선입금한 플랫폼 추상화 없음

**오디오도 전사 결과도 밖으로 나가지 않는다 (§12 · INV-6).**

- 엔진은 같은 프로세스 안의 라이브러리다. 원격 전사 API도, 모델 자동 다운로드도 없다.
- `the_transcription_boundary_has_no_way_to_send_audio_anywhere`가 전사 경계 네 파일에
  `reqwest` · `ureq` · `hyper` · `TcpStream` · `UdpSocket` · `std::net` · `curl` · `socket`이
  들어오지 못하게 한다.
- `the_engine_runs_in_this_process_and_spawns_nothing`이 `process::Command`가 없다는 것과,
  `tauri.conf.json`에 `externalBin`이 없고 `capabilities/default.json`에 shell 권한이
  없다는 것을 확인한다.

**Windows를 위한 추상화를 미리 만들지 않았다 (ADR-0007 §11 · §20.6).**

- `SidecarResolver`를 만들지 않았다 — 해석할 sidecar 경로가 없다.
- 이 Phase에서 플랫폼이 실제로 갈리는 지점은 **모델 파일 위치 하나**이며, 그것은 이미 있는
  `AppDataDirectory` 경계가 처리한다 (`models_dir` 두 줄). 전사 모듈에는 플랫폼 분기가
  하나도 없다 — `no_platform_branch_was_pre_paid_for_windows`가 `#[cfg(target_os` ·
  `#[cfg(windows` · `#[cfg(unix` · `cfg!(…)`를 금지한다.
- Windows 빌드 · Windows 바이너리 확보 · Windows 실행 검증은 하지 않았다 (Phase 6).

**sidecar 조건절은 성립하지 않는다.** Task 요구의 *"sidecar를 택했다면 `bundle.externalBin`과
capabilities의 shell 권한을 target triple 접미사 규칙대로 설정하되…"* 는 ADR-0007 §2가
sidecar를 택하지 않았으므로 적용되지 않는다. **그래서 `tauri.conf.json`과
`capabilities/default.json`을 바꾸지 않았고**, 쓰지 않는 권한을 열지 않았다. 대신 그 두
파일이 그 상태로 남아 있다는 것을 위 테스트가 지킨다.

---

## 이 Task가 확인하지 않은 것

`whisper-rs-api-verification.md` §3에 따로 적었다. 요약하면 **timestamp의 실제 단위 ·
번들 whisper.cpp 버전 · 실제 추론 성공 여부**는 이 Task가 확인하지 않았고, 확인한 것처럼
적지 않았다. `parse.rs`의 변환 계수도 그래서 바꾸지 않았다.
