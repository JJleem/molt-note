# TASK-087 Gate 실행 결과 (Worker self-check · 참고용)

실행: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
일시: 2026-09-07
원본 로그: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(Runtime이 Worker 종료 후 Gate를 독립적으로 다시 돌린다. 아래는 완료 판정이 아니다.)

| Gate | command | exit | 소요 | 결과 |
| --- | --- | --- | --- | --- |
| build | `npm run build` (tsc && vite build) | 0 | 1.3s | PASS |
| lint | `npm run lint` (eslint . && cargo clippy --all-targets -- -D warnings) | 0 | 5.9s | PASS |
| test | `npm run test` (vitest run && cargo test) | 0 | 46.9s | PASS |

lib 단위 테스트 총계 (cargo test):

```
test result: ok. 555 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

새 모듈 이전(2026-09-07 이 Run 시작 시점)의 lib 테스트는 525개였고, 이 Run이 더한
`transcription::chunking::tests` 30개가 그 안에서 실제로 돌았다 — 목록은
`test-chunking.txt`에 있다.

## 첫 실행의 실패와 그 원인 (숨기지 않는다)

첫 self-check에서 `test` gate가 exit 101로 실패했다.

```
---- transcription::chunking::tests::this_module_does_not_know_the_outside_world stdout ----
panicked at src/transcription/chunking.rs:855:13:
청킹 모듈에 바깥 세계가 들어왔다: whisper_rs
```

순수성 검사가 **자기 모듈의 문서 주석**에 걸린 것이다 — 주석이 "`whisper_rs`가 여기 없다"고
적고 있었고, 검사는 원문에서 그 문자열을 찾는다. 검사를 약화하지 않고 주석의 표현을
"whisper 라이브러리 타입이 여기 없다"로 고쳤다. **검사 대상도 임계값도 그대로다.**
