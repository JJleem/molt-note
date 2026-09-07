# TASK-072 — Gate 실행 결과 (Worker 로컬 실행 · 참고용)

실행 명령 (Runtime 소유 진입점):

```text
node tools/loop-runtime/loopctl.mjs self-check build lint test
```

실행 시각: 2026-09-07 (RUN-20260907T053900Z-TASK-072)
원본 출력: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`

| Gate | 명령 | 결과 | exit | 소요 |
| --- | --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | PASS | 0 | 1.1s |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | PASS | 0 | 1.5s |
| test | `npm run test` (`vitest run` + `cargo test`) | PASS | 0 | 5.7s |

```text
Self-check: all gates passed
```

위 소요 시간은 **같은 세션에서 앞서 한 번 돌린 뒤의 캐시된 실행**이다. 캐시 없이 처음
돌렸을 때는 test 51.8s · lint 6.3s였고(clippy가 `Checking molt-note`를 실제로 다시 돌렸다),
세 번 모두 exit 0이었다.

## test Gate 안의 두 갈래

```text
vitest run     Test Files  28 passed (28)   ·   Tests  571 passed (571)
cargo test     lib 단위 테스트  517 passed; 0 failed
               (이 Task가 더한 25개가 그 안에 있다 — portion-unit-tests.txt)
               통합 테스트 바이너리 전부 0 failed
```

프론트엔드는 이 Task가 건드리지 않았다 — command도 화면도 범위 밖이다 (Task 서술).
`vitest`의 571개는 변경 전과 같은 수이며, 이 Task의 변경은 전부 Rust 쪽에 있다.

## 이 Task는 Gate를 한 번에 통과했다

재시도 없이 build · lint · test가 처음 실행에서 전부 exit 0이었다. 테스트를 지우거나
`#[ignore]`를 붙이거나 약하게 만든 자리는 없다 — 기존 기대 문자열 중 바꾼 것은
**AI Handoff 경로의 것 하나**뿐이며(`tests/manual_ai_handoff.rs`), §11 export의 기대 문자열은
한 글자도 바뀌지 않았다 (`section-11-invariance.txt`).
