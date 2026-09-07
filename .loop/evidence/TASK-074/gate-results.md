# TASK-074 — Gate 결과 (self-check · 참고용)

Runtime이 Worker 종료 후 Gate를 독립적으로 다시 돌린다. 아래는 Worker가 로컬에서 실행한 결과다.

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`

마지막 실행 (falsification 되돌린 뒤, 최종 상태):

```text
build: PASS  exit=0   1.2s     npm run build   (tsc && vite build)
lint:  PASS  exit=0   5.7s     npm run lint    (eslint . && cargo clippy --all-targets -- -D warnings)
test:  PASS  exit=0  48.2s     npm run test    (vitest run && cargo test)

Self-check: all gates passed
```

## 도중에 실제로 잡힌 것 (전부 이 Task가 새로 쓴 테스트 쪽 문제였다)

| 회차 | Gate | 실패 | 원인 | 고친 것 |
| --- | --- | --- | --- | --- |
| 1 | test | vitest 3건 | `markdown.file.show` vs `forAi.show` — 두 done 상태에서 `show`가 놓인 자리가 다르다 | 테스트가 두 자리를 같은 모양으로 보도록 `doneStates()`를 고침 |
| 2 | test | cargo 컴파일 | 지역 변수 `sentences`가 같은 이름의 함수를 가림 (E0618) | 지역 변수를 `spoken`으로 |
| 2 | test | cargo 경고 | `TextSizePayload` 미사용 import | `TextSizePayload::from(...)`로 실제 사용 |
| 3 | lint | clippy `needless_borrows_for_generic_args` | `TranscriptId::new(&format!(...))` | `&` 제거 |
| 4 | build/lint/test | — | — | 전부 PASS |

제품 코드는 이 과정에서 바뀌지 않았다. 고친 것은 전부 이 Task가 새로 쓴 두 테스트 파일이다.
