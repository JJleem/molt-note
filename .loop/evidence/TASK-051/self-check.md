# TASK-051 · self-check 결과

실행: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
(Runtime 소유 진입점. 아래 결과는 참고용이며 완료 판정이 아니다 — Runtime이 Worker 종료 후
Gate를 독립적으로 다시 돌린다.)

| Gate | 명령 (`.loop/project.yaml`) | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | 0 | PASS |
| lint | `npm run lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | 0 | PASS |
| test | `npm run test` (`vitest run` + `cargo test`) | 0 | PASS |

세 Gate 모두 이 Task의 변경을 포함한 상태에서 통과했다 (P9-AC1 · P9-AC2 · P9-AC3).

```text
Self-check (advisory — the runtime reruns gates independently after the worker finishes)
Gates: build, lint, test

[build] npm run build
[lint] npm run lint
[test] npm run test

build: PASS  exit=0  1.0s
lint:  PASS  exit=0  1.3s
test:  PASS  exit=0  3.7s

Self-check: all gates passed
```

## test Gate가 실제로 무엇을 돌렸는가

```text
vitest run     Test Files  20 passed (20)      (TASK-050 시점 18 → 이 Task가 2개를 더했다)
               Tests      360 passed (360)     (TASK-050 시점 316)
cargo test     lib unit    412 passed; 0 failed
               integration test binary 전부 ok; 0 failed
```

이 Task는 **frontend만 바꾼다.** Rust 쪽 변경이 없으므로 `cargo test`의 수는 TASK-050과 같다 —
그 사실 자체가 "이 Task가 backend 계약을 건드리지 않았다"는 증거다.

새로 더한 vitest 파일 둘:

```text
src/screens/exportView.test.ts       Export Markdown 자리의 상태 · P-3의 세 조건
src/screens/notionSyncView.test.ts   Send to Notion 자리의 상태 · 중복 sync 예측 · §13
```

그리고 기존 파일 하나에 검사 하나를 더했다:

```text
tests/screen-boundary.test.ts        exportView.ts 원문에 provider가 등장하지 않는다 (INV-8)
```

## 실제 Notion · 실제 파일시스템에 닿지 않았다 (§18)

두 테스트 파일은 순수 view 모듈만 부른다. `@tauri-apps/api`도, `node:fs`도, 네트워크도 쓰지
않으며 (`src/`는 브라우저 코드로 타입 검사되므로 node 모듈을 import할 수 없다), 입력은 전부
테스트 안에서 만든 값이다. 실제 Notion token은 어디에도 없다.

## 처음 실행에서 실패했던 것 (기록)

첫 self-check에서 `src/screens/notionSyncView.test.ts`의 두 검사가 실패했고, **둘 다 제품 쪽을
고쳐서** 통과시켰다(테스트를 약화하지 않았다).

```text
1. 저장된 notionStatus=pending 인데 상세의 상태 표시가 "Running"으로 보였다.
   → badgeStatus가 본문 종류가 아니라 **이 앱이 돌리는 전송(live)**만 예외로 두도록 고쳤다.
     저장된 다섯 상태는 목록과 글자 그대로 같은 값이 된다.
2. 두 번째 전송 안내 문구 검사의 정규식이 "changed or deleted"라는 안심 문구까지 잡았다.
   → 검사를 정확하게 바꿨다(파괴를 약속하는 표현만 금지, 정책 문구는 그대로 요구).
```

두 번째 self-check에서 `build`가 `TS2307: Cannot find module 'node:fs'`로 실패했다 —
`src/` 아래 테스트는 브라우저 타입으로 검사되기 때문이다. 원문을 읽는 검사를
`tests/screen-boundary.test.ts`(이미 `node:fs`를 쓰는 자리)로 옮겨서 해결했다.
