# TASK-061 — Gate self-check

실행 명령 (Runtime 소유 진입점):

```text
node tools/loop-runtime/loopctl.mjs self-check build lint test
```

실행 시각: 2026-09-06 (Worker Run `RUN-20260905T145815Z-TASK-061`)

두 번 돌렸다 — 첫 번째는 토큰·foundation·font-size 치환까지, 두 번째는 `:root`의 기준
`font-size` / `line-height`까지 토큰으로 옮긴 뒤다. 아래는 **마지막 실행**의 출력 원문이며,
첫 실행도 세 Gate 모두 exit 0이었다 (build 1.2s · lint 2.2s · test 12.7s).

출력 원문:

```text
Self-check (advisory — the runtime reruns gates independently after the worker finishes)
Gates: build, lint, test

[build] npm run build
[lint] npm run lint
[test] npm run test

build: PASS  exit=0  1.2s

lint: PASS  exit=0  2.0s

test: PASS  exit=0  5.0s

Self-check: all gates passed
Artifacts: .loop-local/self-check/  (advisory only — not a gate report)
```

- `build` = `npm run build` (`tsc && vite build`) — exit 0
- `lint` = `npm run lint` (`eslint . && cargo clippy --all-targets -- -D warnings`) — exit 0
- `test` = `npm run test` (`vitest run && cargo test`) — exit 0

이 결과는 참고용이다. 완료 판정은 Runtime과 Verifier가 Worker 종료 후 독립적으로 다시 돌린
Gate로 한다.

## 기존 테스트가 깨지지 않았다 (AC-3)

`tests/screen-boundary.test.ts`와 `tests/ipc-boundary.test.ts`는 `src/` 아래의 `.css`를
**원문으로 함께 읽는다** (`sourceFiles`의 확장자 목록에 `css`가 있다). 즉 이 Task의 변경은
그 두 파일의 원문 규칙(SQL 모양 · clipboard 호출 · console 출력 · 네트워크 통로 · 길이 포맷
계산)에 실제로 노출되며, `test` Gate의 PASS가 그것을 함께 판정한 결과다.
