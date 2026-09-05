# TASK-065 — Gate self-check (저장소 전체 · build · lint · test)

Phase 종료 요구 14(`phase-prompt/05.5` E-14)에 따라 **저장소 전체**에 세 Gate를 모두 돌렸다.
이 결과는 **참고용이다** — Runtime이 Worker 종료 뒤 Gate를 독립적으로 다시 돌린다.

## 실행한 명령

```bash
node tools/loop-runtime/loopctl.mjs self-check build lint test
```

Gate 정의(`.loop/project.yaml`)가 부르는 것:

```text
build   npm run build     → tsc && vite build
lint    npm run lint      → eslint . && cargo clippy --all-targets -- -D warnings
test    npm run test      → vitest run && cargo test
```

세 명령 모두 **TypeScript와 Rust를 함께** 검사한다 (SYSTEM-MAP §9 · 2026-09-01 결정).

## 결과 — 문서 변경 후 재실행 (최종 트리)

```text
build: PASS  exit=0  1.2s
lint:  PASS  exit=0  2.2s
test:  PASS  exit=0  5.7s

Self-check: all gates passed
```

## 자동 테스트 개수 (같은 실행의 stdout에서)

```text
web  (vitest)      Test Files  26 passed (26)
                   Tests      494 passed (494)

Rust (cargo test)  744 passed; 0 failed; 0 ignored
                   (crate 단위 test result 합계 — 436 + 308)

합계               1,238
```

Phase 5 종료 시점은 1,081개(vitest 384 · Rust 697)였다 (SYSTEM-MAP §6의 이전 값).

## 원본 출력

Runtime이 소유하는 자리에 남는다 — `.loop-local/self-check/gates/{build,lint,test}/stdout.log`
및 `stderr.log`. **이 Task는 그 디렉터리를 만들거나 고치지 않았다** (self-check 진입점이 쓴다).

## 이 Gate가 판정하지 **않는** 것

```text
실제 webview에서 clipboard 쓰기가 동작하는가          ← UNVERIFIED (ADR-0010 §12.4)
산출물이 실제 외부 AI 채팅에서 쓸 만한가              ← 사람이 판정한다
화면이 실제로 차분하고 읽기 쉬운가                    ← 사람이 판정한다
로컬 provider가 '선택적 · 고급'으로 읽히는가          ← 사람이 판정한다
```

절차와 **빈 기록표**는 `docs/PHASE-5.5-HUMAN-REVIEW.md`.
