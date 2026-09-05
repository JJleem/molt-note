# Gate self-check (AC-1 · AC-2 · AC-3)

Runtime 소유 진입점으로 세 Gate를 전부 돌렸다. 아래는 마지막 실행의 출력 그대로다.
(참고용이며 완료 판정이 아니다 — Runtime이 Worker 종료 후 독립적으로 다시 돌린다.)

```text
$ node tools/loop-runtime/loopctl.mjs self-check build lint test
Self-check (advisory — the runtime reruns gates independently after the worker finishes)
Gates: build, lint, test

[build] npm run build
[lint] npm run lint
[test] npm run test

build: PASS  exit=0  1.1s

lint: PASS  exit=0  1.9s

test: PASS  exit=0  4.6s

Self-check: all gates passed
Artifacts: .loop-local/self-check/  (advisory only — not a gate report)
```

각 Gate가 실제로 덮는 것 (`.loop/project.yaml`):

```text
build   tsc && vite build
lint    eslint . && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
test    vitest run && cargo test --manifest-path src-tauri/Cargo.toml
```

## AC-3의 뒷부분 — 순수 view 모듈의 기존 테스트

`vitest run`이 `src/**/*.test.ts`를 전부 돈다. 이 Task는 **순수 view 모듈을 한 줄도 고치지
않았으므로** (아래 `presentation-and-a11y.md`의 AC-6 절) 그 테스트들은 이전과 같은 코드를
판정하고 있고, 전부 통과했다.

소스 원문을 읽는 경계 테스트 셋도 함께 통과했다 — 이 Task가 마크업을 옮겼기 때문에 조용히
깨질 수 있었던 자리들이다.

```text
tests/screen-boundary.test.ts          token 입력란이 여전히 uncontrolled · ref={tokenInput}
                                       clipboard를 부르는 파일이 여전히 하나
                                       src/ 아래에 console 출력이 없다 (새 컴포넌트 둘 포함)
tests/manual-handoff-invariants.test.ts beginCopy · beginAiExport 블록이 그대로다
                                       벤더 이름을 아는 제품 파일이 여전히 하나뿐이다
                                       (새로 만든 EmptyState.tsx · Loading.tsx가 그 목록에 들어가지 않았다)
tests/ipc-boundary.test.ts             command 표면과 벤더 부재 검사
```
