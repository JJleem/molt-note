# TASK-077 — Gate 실행 결과 (Worker 로컬 실행 · 참고용)

실행 명령 (Runtime 소유 진입점):

```text
node tools/loop-runtime/loopctl.mjs self-check build lint test
```

실행 시각: 2026-09-07 (RUN-20260907T035331Z-TASK-077)
원본 출력: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`

| Gate | 명령 | 결과 | exit | 소요 |
| --- | --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | PASS | 0 | 1.2s |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | PASS | 0 | 5.2s |
| test | `npm run test` (`vitest run` + `cargo test`) | PASS | 0 | 42.5s |

```text
Self-check: all gates passed
```

## test Gate 안의 두 갈래

```text
vitest run     Test Files  27 passed (27)   ·   Tests  526 passed (526)
cargo test     lib 단위 테스트  475 passed; 0 failed
               (이 Task가 더한 16개가 그 안에 있다 — collapse-unit-tests.txt)
               통합 테스트 바이너리 전부 0 failed
```

## 이 실행 전에 두 번 실패했고, 무엇이었는지 감추지 않는다

| 회차 | Gate | 원인 | 처리 |
| --- | --- | --- | --- |
| 1 | test | `E0716` — 테스트 헬퍼에서 임시 값을 빌려 썼다 (`borrow(&distinct(20))`) | `let` 바인딩으로 수명을 늘렸다. 판정 코드는 건드리지 않았다 |
| 2 | lint | `clippy::useless_vec` — 테스트의 `vec![...]`를 배열로 바꾸라는 경고 | 배열로 바꿨다. 판정 코드는 건드리지 않았다 |

둘 다 **테스트 코드의 문제였고, 테스트를 지우거나 약하게 만들어 통과시키지 않았다.**
