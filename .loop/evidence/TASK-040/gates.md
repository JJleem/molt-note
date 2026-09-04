# TASK-040 — Gate self-check 결과

Runtime 소유 진입점으로 실행했다. **참고용이며 완료 판정이 아니다** — Runtime이 Worker 종료
후 같은 Gate를 독립적으로 다시 돌린다.

```text
$ node tools/loop-runtime/loopctl.mjs self-check build lint test

[build] npm run build      → PASS  exit=0   0.9s
[lint]  npm run lint       → PASS  exit=0   1.7s
[test]  npm run test       → PASS  exit=0   2.4s

Self-check: all gates passed
```

Runtime이 남긴 원본 출력: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(worker-local · Runtime 소유 디렉터리).

## 각 Gate가 실제로 돌린 것

| Gate | 명령 (package.json) | 덮는 범위 |
| --- | --- | --- |
| build | `tsc && vite build` | TypeScript 타입 검사 + 실제 번들 |
| lint | `eslint .` → `cargo clippy --all-targets -- -D warnings` | frontend + backend |
| test | `vitest run` → `cargo test` | frontend + backend |

## build stdout (요지)

```text
> tsc && vite build
vite v7.3.6 building client environment for production...
✓ 49 modules transformed.
dist/assets/index-BpT9izWt.js   235.35 kB │ gzip: 72.20 kB
✓ built in 282ms
```

빈 실행이 아니다 — 실제로 번들이 만들어졌다.

## test stdout (요지)

```text
> vitest run
 Test Files  17 passed (17)
      Tests  289 passed (289)

> cargo test --manifest-path src-tauri/Cargo.toml
test result: ok. 277 passed; 0 failed   (unit)
test result: ok.  14 passed; 0 failed   (tests/ai_note_commands.rs)
test result: ok.  17 passed; 0 failed   (tests/ai_note_run.rs)
test result: ok.  12 passed; 0 failed   (tests/ollama_adapter.rs)
test result: ok.  21 passed; 0 failed   (tests/settings_repository.rs)
… 그 밖의 통합 테스트 전부 0 failed
```

이 Task 전의 vitest 수는 261이었고 (`aiProviderSettings.test.ts` 28건 추가) 289가 됐다.
Rust 쪽은 이 Task가 건드리지 않았으므로 수가 그대로다.

## 이 Task가 삭제하거나 약화한 테스트

없다. `settingsView.test.ts`의 기대값 세 곳은 **폼 표현이 바뀌어서** 고쳤다 —
`aiProvider · aiBaseUrl · aiModel`이 `null`에서 빈 문자열로 바뀌었고
(`<select>`·`<input>`은 `null`을 담을 수 없다), 단언의 강도는 그대로다. 왕복 테스트
(`toSettings(toForm(settings))`)는 손대지 않았고 여전히 `null` 왕복을 검사한다.
