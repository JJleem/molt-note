# TASK-008 self-check

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
실행 시각: 2026-09-02 (worker run RUN-20260902T052815Z-TASK-008)

Runtime이 Worker 종료 후 Gate를 독립적으로 다시 돌린다. 아래는 참고용 실행 결과다.

| Gate | 명령 | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` | 0 | PASS |
| lint | `npm run lint` | 0 | PASS |
| test | `npm run test` | 0 | PASS |

원본 출력: `gate-build.log` · `gate-lint.log` · `gate-test.log`
(`.loop-local/self-check/gates/<gate>/stdout.log`를 그대로 옮긴 것이다.)

## 테스트 수

- vitest: **Test Files 10 passed (10) · Tests 70 passed (70)**
  - 이 Task가 추가한 파일: `src/screens/recordingsView.test.ts` ·
    `src/screens/settingsView.test.ts` · `src/screens/failureView.test.ts` ·
    `tests/screen-boundary.test.ts`
- cargo test: 44 + 12 + 5 + 16 + 13 + 7 passed, 0 failed (변경 없음 — Rust는 건드리지 않았다)

## 첫 실행에서 잡힌 것

첫 self-check에서 lint가 FAIL 했다 (`react-hooks/set-state-in-effect`):
effect 본문에서 `setView(LOADING_*)`를 동기로 부르고 있었다.
다시 읽기 전에 loading으로 되돌리는 일을 effect가 아니라 "다시 시도" 사용자 동작
(`retry` · `retryLoad`)으로 옮겨서 해결했다. 규칙을 끄거나 완화하지 않았다.
