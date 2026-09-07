# TASK-085 — Phase 5.7 문서 마무리 (SYSTEM-MAP · A-TRANS-001 · 다음 후보 · Human Review)

Run: RUN-20260907T052101Z-TASK-085 · 2026-09-07 · role: impl · **문서 전용**

## 바꾼 파일 — 둘뿐이다

| 파일 | 무엇 |
| --- | --- |
| `docs/SYSTEM-MAP.md` | 수정 (`system-map.diff` · 352줄) |
| `docs/PHASE-5.7-HUMAN-REVIEW.md` | 신규 |

**소스 · 설정 · 의존성 · 테스트는 하나도 바꾸지 않았다** (`changed-files.txt` · `gates.txt`).
저장소에 남아 있는 다른 `M` / `??` 항목은 **이 Run 이전에 이미 있던 것**이며(TASK-066 ~
TASK-084의 결과) 이 Run이 건드리지 않았다.

## Acceptance Criteria 대조

### AC-1 — SYSTEM-MAP이 실제로 구현된 것만 DONE으로 적는다

**DONE으로 적은 것과 저장소의 대조** (전부 실제 코드를 읽어 확인했다):

| SYSTEM-MAP의 기술 | 저장소의 근거 |
| --- | --- |
| 입력 레벨의 계산·판정·문장이 한 모듈에 있다 | `src-tauri/src/audio/level.rs` — `FULL_SCALE` · `USABLE_AT_OR_ABOVE_DBFS = -36.0` · `SILENT_BELOW_DBFS = -60.0` · `FLOOR_DBFS` · `LevelVerdict` · `LevelReading` · `InputLevel::push/reading` · `describe` |
| 레벨을 파일 통로(`drain`)에서 갱신한다 | `src-tauri/src/audio/capture.rs` (레벨 갱신) · `tests/level-and-collapse-boundary.test.ts` (d) |
| status payload로 나가고 없으면 `null` | `src-tauri/src/commands/payload.rs` `InputLevelPayload` · `level: Option<...>` · `verdict_name` / `src/ipc/types.ts` `InputLevel` · `SessionStatus.level: InputLevel \| null` |
| 화면이 세 갈래로 보이고 정지 전에 경고한다 | `src/screens/recordingView.ts` `UNKNOWN_LEVEL_TEXT` · `WEAK_LEVEL_WARNING` · `inputLevelDisplay` · `inputLevelWarning` / `src/screens/RecordingScreen.tsx` (그리기만) |
| 붕괴 판정 규칙이 한 모듈에 있다 | `src-tauri/src/transcription/collapse.rs` — `MINIMUM_SENTENCES_TO_JUDGE = 20` · `COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW = 0.20` · `COLLAPSED_TOP_REPEAT_SHARE_AT_OR_ABOVE = 0.50` · `CollapseVerdict` · `assess` |
| 저장 직전 관문 · 빈 결과 판정 유지 · 새 실패 종류 없음 | `src-tauri/src/transcription/run.rs` L183~198 (`collapse::assess` → `Empty` / `Collapsed` / `Usable`) · `collapsed_output`이 기존 `output_unusable`을 쓴다 |
| 화면의 붕괴 갈래 | `src/screens/transcriptView.ts` `outputUnusable` 갈래 · `TRANSCRIPTION_COLLAPSED_NOTICE` · `RESOLUTION` · `TRANSCRIPTION_PRESERVED_NOTICE` |
| 다섯 불변 전용 테스트 | `tests/level-and-collapse-boundary.test.ts` (a)~(e) |
| 자동 테스트 1,405개 | `gates.txt` (이 Run의 self-check 실행값) |

**PLANNED / DEFERRED / CANDIDATE를 구현된 것으로 적지 않았다** — 청크 분할 · state 재생성 ·
반복 차단 · VAD · 자동 언어 감지 개선 · 입력 정규화는 §1의 CANDIDATE 행과 §5의 C-1 ~ C-6에
**"어느 것도 구현되지 않았다"** 로 명시했고, §2의 흐름에도 "이 Phase가 넣지 않은 것"으로 적었다.

**이전 Phase 기록을 지우지 않았다** — §5의 Bootstrap · Phase 1 · 2A · 3 · 2B · 4 · 5 · 5.5
절은 한 글자도 바꾸지 않았고, 헤더의 2026-09-05 문단도 그대로 두고 그 **뒤에** 덧붙였다.
갱신 이력에 2026-09-07 줄을 **추가**했다.

**한 자리는 사실로 정정했다** — §5의 "Phase 5.6 · **PLANNED**"는 이미 거짓이었다
(TASK-066 ~ TASK-071이 DONE이고 코드가 저장소에 있다). Task별 상태표를 넣어
**부분 완료(6 DONE / 4 TODO)** 로 적었고, 지운 서술은 없다.

### AC-2 — A-TRANS-001이 열린 채로 정확히 기술돼 있다

- `SYSTEM-MAP` §1의 표: "실제 추론은 두 번 실행됐고, 두 번 다 쓸 수 없었다"
- `SYSTEM-MAP` §7: 2026-09-07 실행의 값(고유 2개 · 99.0% · -42.2 vs -25.8 dBFS)과
  **[유력한 설명] / [미검증]** 구분, 그리고 **닫히는 조건 한 줄**
- `SYSTEM-MAP` §5의 Phase 5.7 절: "이 Phase는 이 가정을 닫지 않는다"
- `PHASE-5.7-HUMAN-REVIEW.md` **§7.1** — 지금 상태 · 지금까지 두 실행 · 이 Phase가 한 것과
  하지 않은 것 · **닫히는 조건** · **닫는 주체는 이 문서가 아니라 그 뒤의 Task**

### AC-3 — 다음 Phase 후보와 제외 이유

`SYSTEM-MAP` §5의 **"다음 Phase 후보 — 어느 것도 이 Phase에서 구현되지 않았다 (CANDIDATE)"**
표에 여섯 줄. 요구된 다섯(C-1 청크 분할 · C-2 청크마다 state 재생성 · C-3 반복 차단 ·
C-4 VAD · C-5 자동 언어 감지 개선)과 **C-6 P8 측정 결과에 따른 입력 정규화 결정**이며,
**각 줄에 제외 이유가 한 줄씩** 있다. 표 머리에 "계획도 결정도 아니다 · 코드에 없다"를 적었고,
§1의 CANDIDATE 행과 §2가 같은 말을 한다. C-6은 `PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`
부록2-4가 `[미측정]`이라는 사실과 함께 **측정 대기**로 적혀 있다.

### AC-4 — Human Review 문서

`docs/PHASE-5.7-HUMAN-REVIEW.md` — `docs/PHASE-5.5-HUMAN-REVIEW.md`의 선례를 따랐다.

- **Goal의 네 항목이 HR-1 ~ HR-4로 그대로 들어 있다** (§0의 표 · §3 · §4 · §5 · §6)
- **§0이 "자동 Gate가 판정할 수 없다"를 명시한다** — Phase Goal의 문장을 인용하고,
  **항목마다 자동 검증이 이미 하는 것과 Gate가 답할 수 없는 이유**를 따로 적었다
- **§8의 기록표 결과칸이 전부 비어 있다** (`☐ PASS ☐ FAIL ☐ 미실행` · `(비어 있음)`).
  §8.4의 비교표는 왼쪽 두 열(9/5 · 9/7의 관측값)만 채워져 있고 **새 녹음 열은 비어 있다**
- 인용한 화면 문구 · 상수 · 임계값은 전부 저장소에서 읽은 값이며 `[E1]`로 표시했고,
  확인하지 못한 것은 `[E4]` / `[미측정]`으로 남겼다 (§7이 그 정본)

### AC-5 — 소스 · 설정 · 의존성 · 테스트 무변경

`changed-files.txt`가 이 Run이 만진 둘만 담고 있다. `gates.txt`의 build · lint · test는
전부 PASS이며, 이 Run 전후로 테스트 수(1,405)가 달라지지 않았다.

## 이 Task가 하지 않은 것

- `PRODUCT-SPEC.md` §21 로드맵 갱신 — 이 Task의 범위가 아니다 (요청은 SYSTEM-MAP이다)
- Phase 5.6의 남은 Task(TASK-072 ~ TASK-075) — 상태를 **적기만** 했고 구현하지 않았다
- `A-TRANS-001`을 닫는 것 · 임계값을 고치는 것 · 정규화를 넣는 것
