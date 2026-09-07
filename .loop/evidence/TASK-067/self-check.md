# TASK-067 self-check (advisory)

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
(Runtime 소유 진입점. 이 결과는 참고용이며 완료 판정이 아니다.)

실행 시각: 2026-09-06 (Run RUN-20260906T010345Z-TASK-067)

| Gate  | command         | 결과 | exit | 소요  |
| ----- | --------------- | ---- | ---- | ----- |
| build | `npm run build` | PASS | 0    | 1.1s  |
| lint  | `npm run lint`  | PASS | 0    | 5.4s  |
| test  | `npm run test`  | PASS | 0    | 40.4s (cold) · 4.2s (warm 재실행) |

`test` gate는 두 벌을 연달아 돌린다.

```text
vitest run                          Test Files 26 passed (26) · Tests 503 passed (503)
cargo test (lib + 통합 테스트 전부)  전부 ok · 0 failed
```

전체 출력은 `test-gate.log`에 있다.
