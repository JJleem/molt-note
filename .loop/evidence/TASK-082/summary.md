# TASK-082 — 입력 레벨을 사람이 보는 자리까지 잇는다

P6이 만든 값(`src-tauri/src/audio/level.rs` → `InputLevelPayload`)을 화면까지 이었다.
계산도 판정도 문장도 backend 것을 그대로 쓴다 — 화면은 **어디에 어떤 갈래로 놓을지**만 정한다.

## 무엇을 했나

### (1) 계약 — `src/ipc/types.ts`

- `InputLevelVerdict = 'usable' | 'low' | 'silent'` — Rust `LevelVerdict`의 wire 이름과 1:1.
- `InputLevel { averageDbfs · peakDbfs · verdict · message }` — `InputLevelPayload`(serde camelCase)와 1:1.
- `SessionStatus.level: InputLevel | null` — Rust는 `Option<InputLevelPayload>`를 언제나 직렬화하므로
  옵셔널 필드가 아니라 **`| null`인 필수 필드**로 적었다. `null`은 "재지 않았다"이며 0이 아니다.

### (2) 표시 규칙 — `src/screens/recordingView.ts` (순수 모듈)

- `inputLevelDisplay(view) → { shown · kind · text · weak }`
  - 진행 중인 녹음이 없으면 `shown: false` (끝난 녹음의 레벨을 붙들지 않는다).
  - `level === null` → `kind: 'unknown'`, `UNKNOWN_LEVEL_TEXT`, `weak: false`.
    **모르는 것을 낮다고 말하지 않는다.**
  - 값이 있으면 `kind`는 backend의 `verdict` 그대로, `text`는 backend의 `message` 그대로.
- `inputLevelWarning(view) → string | null`
  - `state === 'recording'`이고 판정이 `low`/`silent`일 때만 `WEAK_LEVEL_WARNING`.
  - 녹음 중이 아니면(`idle` · `paused` · `stopped` · 아직 모름) 언제나 `null`.
- `isWeak`는 판정 셋을 화면이 다루는 둘로 묶기만 한다 — **임계값도 dBFS 산술도 없다.**

### (3) 그리기 — `src/screens/RecordingScreen.tsx` · `src/App.css`

- 레벨 한 줄이 상태·경과 시간 **아래**에 `--type-secondary`로 온다. `--type-display`(경과 시간)는
  건드리지 않았고, 미터·파형·움직임을 두지 않았다 (§19 · `tests/ui-foundation.test.ts`의 장식 금지 검사 통과).
- 경고는 조작 위에 놓여 Stop 전에 읽힌다. 실패가 아니므로 `FailureNotice`를 쓰지 않고,
  녹음을 막지도 멈추지도 않는다 (ADR-0003 §16.5). 색은 기존 `--warning` 토큰뿐 — 새 색 토큰 없음.

### (4) 고정 — 테스트

`src/screens/recordingView.test.ts`의 새 describe `입력 레벨 (ADR-0003 §16.3 · §16.4)`:

| 케이스 | 고정하는 것 |
| --- | --- |
| 값이 아직 없는 것은 낮은 것이 아니다 | `kind: 'unknown'` · `weak: false` · 경고 없음 |
| 낮으면 그 갈래로 오고 backend의 문장을 그대로 | `kind: 'low'` · `text === LOW_LEVEL.message` |
| 소리 없음도 값 없음과 다른 갈래다 | `kind: 'silent'` · `weak: true` |
| 쓸 만하면 경고하지 않는다 | `kind: 'usable'` · 경고 `null` |
| 세 갈래가 서로 다른 문장 | 네 문장이 전부 다르고 비어 있지 않다 |
| 정지 전에 경고가 나온다 | `low`·`silent` 둘 다 `WEAK_LEVEL_WARNING` |
| 경고 문장이 무엇을 해야 하는지 말한다 | microphone · stop · "stays as it is"(INV-3) |
| 녹음 중이 아니면 경고하지 않는다 | 아직 모름 · idle · paused · stopped 전부 `null` |
| 진행 중인 녹음이 없으면 자리 자체를 두지 않는다 | `shown` |
| 레벨이 상태·경과 시간을 밀어내지 않는다 (§19) | `sessionDisplay`가 레벨과 무관하게 같다 |

`tests/screen-boundary.test.ts`에 새 describe `입력 레벨의 환산과 판정은 Rust에만 있다`:
`src/` 어디에도 `log10`·`32768`·판정 임계값(`-36` · `-60`)이 없고(주석 제외),
`recordingView.ts`가 `kind: level.verdict` · `text: level.message`로 나른다는 것을 원문으로 못박는다.

계약이 필수 필드가 되면서 `SessionStatus` 리터럴을 만들던 두 자리에 `level: null`을 더했다
(`src/screens/coreWithoutAi.test.ts` · `tests/ui-foundation.test.ts`). 검사를 약화한 곳은 없다.

## Acceptance Criteria 대응

| AC | 어떻게 판정되는가 | 결과 |
| --- | --- | --- |
| AC-1 `npm run build` | gate `build` | PASS (exit 0) · `build-stdout.log` |
| AC-2 `npm run lint` | gate `lint` (`eslint .` + `cargo clippy -D warnings`) | PASS (exit 0) · `lint-stdout.log` |
| AC-3 `npm run test` + 새 케이스 + screen-boundary | gate `test` (`vitest run` + `cargo test`) | PASS · vitest 27 files / 542 tests · `test-stdout.log` |
| AC-4 녹음 중 레벨이 보이고 정지 전 경고 · 규칙은 순수 모듈 | verifier | `recordingView.ts`의 두 함수 + `RecordingScreen.tsx`는 `level.shown`/`levelWarning`만 그린다 |
| AC-5 화면에 dBFS 계산·임계값 없음 | verifier | `tests/screen-boundary.test.ts`의 새 describe가 원문으로 검사 |

## 남는 사실 (판정하지 않은 것)

- 실제 마이크로 녹음하며 화면에 레벨이 뜨는 모습은 이 Task가 판정하지 않는다 — 자동 검사는
  마이크 없이 값으로만 돈다 (PRODUCT-SPEC §18). Human Review 항목이다.
- backend의 `message`는 한국어이고 화면이 쓰는 문장은 영어다. 기존에도 Rust의 실패 문장이
  그대로 화면에 오므로 이 Task가 새로 만든 상태는 아니며, 언어 통일은 범위 밖으로 두었다.
