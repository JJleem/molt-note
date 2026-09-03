# TASK-030 — Recording Detail의 Transcript 탭

Run: `RUN-20260903T045026Z-TASK-030` · 2026-09-03

## 무엇을 했는가

Recording Detail에 Transcript 탭을 만들었다. 표시 규칙은 React·DOM·Tauri를 모르는 순수 모듈
`src/screens/transcriptView.ts` 하나에 있고, `src/screens/transcriptView.test.ts`가 그것을
whisper·모델·오디오·jsdom 없이 판정한다 (§18).

- `formatTimestamp(ms)` — 밀리초를 `HH:MM:SS`로 만드는 순수 함수. 경계는 전부 **리터럴
  기대값**으로 고정했다: `0` · 1초 미만(1·499·500·999) · 분 경계 · 정확히 1시간 · 1시간 초과 ·
  100시간 · 음수 · `NaN` · `Infinity`. 버림이지 반올림이 아니다.
- `transcriptLines(transcript)` — segment를 `00:02:14 → 00:02:21` + 문장으로 옮긴다.
- `transcriptTab(recording, transcript, live)` — 다섯 상태(`none` · `pending` · `running` ·
  `done` · `failed`)와 `loading`을 값으로 구분한다.
- 실패 상태는 §13대로 **무엇이 실패했는지**(`failure` · `headline`) · **원본과 기존 Transcript가
  그대로라는 사실**(`preservedNotice`) · **재시도 수단**(`retry`)을 함께 갖는다. 모델 없음
  (`transcriptionModelMissing`)은 `cause`와 `resolution`으로 일반 실패와 구분된다.
- 실패·진행 중 상태는 이미 있던 Transcript를 `kept`로 그대로 들고 있다 — 화면이 이전
  Transcript를 지우거나 고치는 경로는 없다 (§7.1 · INV-2).

backend에는 저장된 Transcript를 **읽는** command 하나(`get_transcript`)를 열었다. Transcript를
고치거나 지우는 이름은 늘지 않았고, 그것을 `tests/ipc-boundary.test.ts`의 새 검사가 지킨다.

## Gate 결과 (self-check · 2026-09-03)

`node tools/loop-runtime/loopctl.mjs self-check build lint test` → 셋 다 exit 0.

| gate | command | exit |
| --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | 0 |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | 0 |
| test | `npm run test` (`vitest run` + `cargo test`) | 0 |

원본 출력은 `gates/<gate>/stdout.log` · `stderr.log`에 있다.

- vitest: 15 files · **201 tests** passed (`transcriptView.test.ts` 포함).
- cargo test: 18개 test binary 전부 ok. 새 테스트 두 개가 이름으로 확인된다 —
  `a_stored_transcript_comes_back_through_the_command_surface_with_its_segments`,
  `looking_up_an_unknown_transcript_is_an_empty_answer_not_a_failure`
  (`gates/test/stdout.log` 243 · 245행).

이 결과는 참고용이다. 완료 판정은 Runtime과 Verifier가 한다.

## 이 Task가 만든/고친 파일

새로 만든 것:

- `src/screens/transcriptView.ts`
- `src/screens/transcriptView.test.ts`

고친 것:

- `src-tauri/src/commands/payload.rs` — `TranscriptPayload` · `TranscriptSegmentPayload`
- `src-tauri/src/commands/mod.rs` — `Storage::transcript` · `get_transcript` command
- `src-tauri/src/lib.rs` — `get_transcript` 등록
- `src-tauri/tests/command_boundary.rs` — 저장된 Transcript가 command 표면으로 돌아오는지
- `src/ipc/types.ts` · `src/ipc/commands.ts` — `Transcript` · `TranscriptSegment` · `getTranscript`
- `src/screens/RecordingDetailScreen.tsx` — Transcript 탭 · 수동 시작/재시도 · 상태 조회 반복
- `src/screens/recordingsView.test.ts` — 목록 Transcript badge가 다섯 상태를 그대로 보여주는지
- `src/App.css` — transcript 목록 스타일
- `tests/ipc-boundary.test.ts` · `tests/screen-boundary.test.ts` — 아래 참고

`changed.diff`는 위 파일들의 working-tree diff다. **주의**: 이 저장소는 Phase 3 시작 이후
commit이 없으므로 그 diff에는 TASK-023~029가 같은 파일에 남긴 변경도 함께 들어 있다.
새 파일 둘은 아직 untracked이라 diff에 없다 — 파일 자체가 산출물이다.
`changed-files.txt`는 `git status --porcelain -- src src-tauri tests`의 출력이다.

## 경계 테스트를 고친 이유 (약화가 아니다)

두 곳을 고쳤다. 둘 다 **새로 필요해진 것을 명시적으로 허용하고, 원래 지키던 것은 그대로
두거나 검사를 하나 더 붙였다.**

1. `tests/ipc-boundary.test.ts` — 등록 command 집합에 `get_transcript`를 더했다.
   Phase 3 요구 6(“Transcript 탭에서 timestamp와 함께 볼 수 있다”)을 만족하려면 저장된
   Transcript를 읽는 이름이 있어야 하는데, 기존 표면에는 그것이 없었다(`start_transcription` ·
   `transcription_status`뿐이다). 대신 **쓰기 이름이 늘지 않았다는 것을 새 테스트로 못 박았다**
   (`저장된 Transcript를 고치거나 지우는 command가 없다`). 큐(`queue`·`batch`·`schedule`) 금지
   검사도 그대로다.

2. `tests/screen-boundary.test.ts` — 시각 산술 금지에서 `src/screens/transcriptView.ts`
   **한 파일**을 뺐다. 이 Task가 요구하는 `ms → HH:MM:SS` 순수 함수가 TypeScript 쪽에 있어야
   하기 때문이다(요구 6 · 9는 그 변환을 화면 테스트 대상으로 정한다). 원래 규칙이 막으려던
   것은 “녹음 길이 표시가 두 벌이 되는 것”이므로, 그것은 다음 두 검사로 유지된다.
   - 예외 파일이 실제로 존재하는지 (사라지면 면제가 조용히 무효가 되지 않게)
   - 예외 파일이 `durationMs` · `durationLabel` · `elapsedMs` · `elapsedLabel`을 건드리지 않는지
   기존 검사(`durationLabel`은 Rust가 준 값 그대로 · `elapsedLabel`도 그대로)는 손대지 않았다.

## 이 Task가 하지 않은 것

- transcript 편집 · 검색 · 번역 UI (Phase 3 Out of Scope)
- 여러 Recording 동시 전사 큐 (§16 DEFERRED)
- 실제 whisper 추론. 자동 검증은 whisper·모델 없이 돈다 (요구 8). 운영자 smoke test는 별개다.
- 전사 품질 · timestamp가 실제 음성 위치와 맞는지 — `DEFERRED`(Final Integration).
