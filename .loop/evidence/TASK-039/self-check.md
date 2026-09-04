# TASK-039 self-check

Runtime 소유 진입점으로 실행한 Gate 결과다. **완료 판정이 아니다** — Runtime이 Worker 종료 후
같은 Gate를 독립적으로 다시 돌린다.

```text
$ node tools/loop-runtime/loopctl.mjs self-check build lint test

Self-check (advisory — the runtime reruns gates independently after the worker finishes)
Gates: build, lint, test

[build] npm run build
[lint] npm run lint
[test] npm run test

build: PASS  exit=0  0.9s
lint:  PASS  exit=0  1.4s
test:  PASS  exit=0  2.3s

Self-check: all gates passed
```

## Gate 명령 (`.loop/project.yaml`)

| Gate | 명령 | exit |
| --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | 0 |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | 0 |
| test | `npm run test` (`vitest run` + `cargo test`) | 0 |

## test Gate가 실제로 무엇을 돌렸는가

`.loop-local/self-check/gates/test/stdout.log`에서:

```text
 Test Files  16 passed (16)
      Tests  261 passed (261)
```

이 Task 이전의 frontend 테스트 파일은 15개였다 (`src/**/*.test.ts` 10 + `tests/*.test.ts` 5).
새로 더한 `src/screens/aiNoteView.test.ts` 하나가 16번째이며, 그 파일이 실행되지 않았다면
파일 수가 15로 남는다.

Rust 쪽은 이 Task에서 바뀐 것이 없다 (277 + 통합 테스트 전부 pass, 변경 없음).

## 중간에 한 번 실패했고 그 원인

첫 lint 실행에서 `react-hooks/set-state-in-effect` 오류가 하나 났다 —
노트 목록 effect가 "가리키는 Transcript가 없다"를 `setAiNotes([])`로 저장하고 있었다.
effect에서 상태를 만들지 않고 렌더 시점에 그대로 읽도록 고쳤다
(`RecordingDetailScreen.tsx`의 `notes: noteTranscriptId === null ? [] : aiNotes`).
테스트를 지우거나 규칙을 끈 것이 아니라 코드를 고쳤다.
