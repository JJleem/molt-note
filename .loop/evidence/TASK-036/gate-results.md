# TASK-036 — self-check 결과 (참고용 · 완료 판정이 아니다)

Run: RUN-20260903T064637Z-TASK-036 · 2026-09-03

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
(Runtime이 Worker 종료 후 Gate를 독립적으로 다시 돌린다.)

```text
build: PASS  exit=0
lint:  PASS  exit=0
test:  PASS  exit=0
```

원본 출력: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(Runtime 소유 경로이므로 여기 복사하지 않고 경로로 가리킨다.)

## 각 Gate가 실제로 무엇을 돌렸나

| Gate | 명령 | 이 Task와의 관계 |
| --- | --- | --- |
| build | `tsc && vite build` | frontend. 이 Task는 frontend를 바꾸지 않았다 |
| lint | `eslint .` + `cargo clippy --all-targets -- -D warnings` | 새 Rust 코드 전부가 경고 없이 통과한다 |
| test | `vitest run` + `cargo test` | 아래 |

## 이 Task가 더한 테스트 (전부 PASS)

```text
src/ai/ollama/http.rs      3   경계 타입 — 상태 코드를 판정하지 않는다 · 오류에 문자열 자리가 없다
src/ai/ollama/wire.rs     13   엔드포인트 · 요청 본문 · 목록 해석 · 응답에서 노트 회수
src/ai/ollama/testing.rs   2   double이 두 경로에 답하고 요청된 mode를 schema로 읽는다
src/ai/ollama/provider.rs 12   가용성 · 생성 · 실패 매핑 (모듈 내부)
tests/ollama_adapter.rs   10   계약 묶음 공유 · 실패 매핑 · 설정값 비노출 · 경계 스캔
```

`cargo test` 전체: 264 passed (lib) + 통합 테스트 각 바이너리 all ok · 0 failed · 0 ignored.
**skip된 테스트가 없다** — 서버가 없다는 이유로 건너뛰는 테스트를 만들지 않았다 (§18).

## 이 Run이 실행하지 않은 것

- **실제 Ollama에 대한 호출.** `ai::ollama::network`는 컴파일되지만 실행되지 않는다.
  실제 서버와의 동작은 이 Phase의 Human Review 항목이며, 자동 Gate가 대신 판정하지 않는다
  (ADR-0008 §16.3 · PRODUCT-SPEC §14.4.3의 운영자 smoke test와 같은 성격이다).
- **실제 전사 결과를 쓰는 경로.** 테스트가 쓰는 transcript는 고정 fixture다
  (`ai::testing::CONTRACT_TRANSCRIPT` · `ASSUMPTION A-TRANS-001`).
