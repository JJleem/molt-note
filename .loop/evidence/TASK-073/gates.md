# Gate 실행 결과 (Worker 로컬 self-check · 참고용)

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
시각: 2026-09-07 · Run `RUN-20260907T055932Z-TASK-073`

```text
[build] npm run build     build: PASS  exit=0  1.2s
[lint]  npm run lint      lint:  PASS  exit=0  2.0s
[test]  npm run test      test:  PASS  exit=0  6.5s

Self-check: all gates passed
```

원문 출력은 Runtime 소유 자리에 있다 — `.loop-local/self-check/gates/<gate>/stdout.log` ·
`stderr.log`. **이후의 self-check 실행이 그 파일을 덮어쓰므로** 아래에 판정에 쓰인 수치를
그대로 옮겨 적는다.

## build — `tsc && vite build`

```text
exit=0
```

`tsconfig.json`의 `include`는 `src`이며 `noUnusedLocals` · `noUnusedParameters` · `strict`가
켜져 있다. 즉 이 Gate가 프론트엔드 제품 코드의 타입을 전부 검사한다.

## lint — `eslint . && cargo clippy --all-targets -- -D warnings`

```text
exit=0
    Checking molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.38s
```

(위 `Checking` 두 줄은 이 Task의 Rust 변경이 실제로 다시 컴파일된 실행의 stderr다.
`--all-targets`이므로 통합 테스트 크레이트까지 포함한다.)

## test — `vitest run && cargo test`

```text
 Test Files  28 passed (28)
      Tests  582 passed (582)

cargo test:  "... ok" 872건 · failed 0 · 34개 test binary 전부 `test result: ok`
```

## Gate가 판정하지 않는 것

- 실제 72분 녹음을 앱에서 켜서 나눠 붙여 넣는 **사람의 확인**. 이 Task가 검증한 것은 규칙과
  값이며, `phase-prompt/05.6`의 Human Review 항목은 그대로 남아 있다.
- 어떤 AI 채팅이 한 번에 받는 텍스트의 실제 한도. 이 저장소는 그 값을 확인한 적이 없고
  (UNVERIFIED), 나눔의 예산은 이 앱이 고른 값이다 (`export::portion::PORTION_MAX_BYTES`).
