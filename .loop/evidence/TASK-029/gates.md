# TASK-029 — Gate 실행 결과 (Worker self-check)

Runtime 소유 진입점으로 실행했다. **이 결과는 참고용이며 완료 판정이 아니다** —
Runtime이 Worker 종료 후 같은 Gate를 독립적으로 다시 돌린다.

```
node tools/loop-runtime/loopctl.mjs self-check build lint test
```

| Gate | 명령 (`.loop/project.yaml`) | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` (`tsc && vite build`) | 0 | PASS |
| lint | `npm run lint` (`eslint . && cargo clippy --all-targets -- -D warnings`) | 0 | PASS |
| test | `npm run test` (`vitest run && cargo test`) | 0 | PASS |

원본 출력: `.loop-local/self-check/gates/<gate>/stdout.log` · `stderr.log`
(Runtime 소유 디렉터리이며 다음 self-check 실행이 덮어쓴다. 아래에 그 시점의 요약을 옮겨 둔다.)

## build

```
> tsc && vite build
✓ 46 modules transformed.
dist/index.html                   0.40 kB │ gzip:  0.27 kB
dist/assets/index-BYKmgwJd.css    4.18 kB │ gzip:  1.30 kB
dist/assets/index-CO6X6RNA.js   214.27 kB │ gzip: 66.52 kB
✓ built in 264ms
```

## lint

```
> eslint . && cargo clippy --all-targets -- -D warnings
    Checking molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.40s
```

경고 하나도 통과시키지 않는다(`-D warnings`). 새로 만든 test 파일도 `--all-targets`에 포함된다.

## test

```
> vitest run
 Test Files  14 passed (14)
      Tests  164 passed (164)

> cargo test --manifest-path src-tauri/Cargo.toml
unit (src/lib.rs)                  172 passed; 0 failed
automatic_transcription.rs           7 passed; 0 failed   ← 이 Task가 추가한 파일
capture.rs                           9 passed; 0 failed
command_boundary.rs                 15 passed; 0 failed
domain_invariants.rs                 5 passed; 0 failed
domain_model.rs                     16 passed; 0 failed
input_devices.rs                     6 passed; 0 failed
microphone_permission.rs             7 passed; 0 failed
recording_lifecycle.rs               5 passed; 0 failed
recording_repository.rs             13 passed; 0 failed
recording_session.rs                 4 passed; 0 failed
settings_repository.rs              15 passed; 0 failed
stop_persistence.rs                  6 passed; 0 failed
transcription_background.rs         10 passed; 0 failed
transcription_engine.rs             10 passed; 0 failed
transcription_run.rs                13 passed; 0 failed
```

failed는 어느 binary에서도 0이다. skip(`ignored`)도 0이다 — 모델이 없다는 이유로 건너뛰는
테스트를 만들지 않았다.
