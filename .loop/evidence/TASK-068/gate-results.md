# TASK-068 — Gate 실행 결과 (Worker self-check · 2026-09-06)

Runtime 소유 진입점으로 세 Gate를 **한 번의 호출에서** 실행했다. Worker의 주장이 아니라
명령의 exit code다. (Runtime이 Worker 종료 후 독립적으로 다시 돌린다 — 이것은 참고용이다.)

```text
$ node tools/loop-runtime/loopctl.mjs self-check build lint test

Self-check (advisory — the runtime reruns gates independently after the worker finishes)
Gates: build, lint, test

[build] npm run build
[lint] npm run lint
[test] npm run test

build: PASS  exit=0  1.2s
lint:  PASS  exit=0  2.1s
test:  PASS  exit=0  5.2s

Self-check: all gates passed
Artifacts: .loop-local/self-check/  (advisory only — not a gate report)
```

세 명령의 실체 (`package.json`):

| Gate | command | 실제로 도는 것 |
| --- | --- | --- |
| build | `npm run build` | `tsc && vite build` |
| lint | `npm run lint` | `eslint .` → `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` |
| test | `npm run test` | `vitest run` → `cargo test --manifest-path src-tauri/Cargo.toml` |

**시간이 짧은 것은 cargo 캐시가 더워져 있기 때문이다.** 위 실행에서 clippy와 cargo test는
`Finished ... in 0.40s` / `Finished ... in 0.20s`로 재컴파일 없이 끝났다 — 이 working tree의
소스에 대한 컴파일은 이미 한 번 성공한 상태였고, 소스가 바뀌었다면 cargo가 다시 빌드한다.
같은 소스에 대한 앞선 cold 실행은 lint 6.0s · test 41.4s였고 둘 다 exit=0이었다.

집계:

- vitest: **26 파일 · 503 테스트 통과 · 0 실패**
- cargo test: 모든 target `test result: ok`, **0 failed** (unit 442 + 통합 테스트 전부)

## `whisper-rs` 0.16 API는 컴파일러가 확인했다 (AC1의 요점)

`set_language(Option<&str>)`과 `set_detect_language(bool)`은 문서를 짐작해 쓴 것이 아니다.
이 Run에는 crate 소스 접근이 없었으므로 (`~/.cargo/registry`는 작업 디렉터리 밖이라 읽지
못한다 — ADR-0007 §17.2.2가 기록한 것과 같은 제약), **확인 수단은 컴파일러 하나뿐이었고
그것이 통과했다.** 시그니처가 달랐다면 `cargo clippy --all-targets -- -D warnings`가
`src-tauri/src/transcription/whisper.rs`에서 실패한다.

관련 원문 로그:

- `.loop/evidence/TASK-068/lint-stderr.log`
- `.loop/evidence/TASK-068/test-language-selection.log`
