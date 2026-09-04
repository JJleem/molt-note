# TASK-033 self-check 기록

Runtime 소유 진입점(`node tools/loop-runtime/loopctl.mjs self-check`)으로만 실행했다.
**이것은 참고용이며 완료 판정이 아니다** — Runtime이 Worker 종료 후 Gate를 독립적으로 다시 돌린다.

## 최종 실행 (구현 완료 후)

```text
$ node tools/loop-runtime/loopctl.mjs self-check build lint test

[build] npm run build          → build: PASS  exit=0  0.9s
[lint]  npm run lint           → lint:  PASS  exit=0  3.0s
[test]  npm run test           → test:  PASS  exit=0  13.9s

Self-check: all gates passed
```

- `npm run build` = `tsc && vite build`
- `npm run lint` = `eslint .` + `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
- `npm run test` = `vitest run` + `cargo test --manifest-path src-tauri/Cargo.toml`

확인용으로 test Gate를 한 번 더 돌렸다: `test: PASS exit=0 1.7s` (증분 빌드).

## 그 전의 두 번은 **일부러** 실패했다 — 이 Task의 설계가 그렇게 되어 있다

`promptVersion`은 손으로 선언한 상수이며, 계산값과 다르면 test Gate가 깨진다
(ADR-0008 §10.2-4). 상수를 처음 채울 때 그 Gate가 실제로 작동하는지가 아래 두 실행에 남아 있다.

```text
1회차  test: FAIL exit=101   205 passed; 2 failed
       ai::prompt::tests::prompt_version_is_bound_to_the_prompt_text
         left:  "v1.meeting.00000000"      (선언된 자리표시자)
         right: "v1.meeting.5c6b8a90"      (프롬프트+schema에서 계산된 값)
       ai::prompt::tests::the_hash_function_does_not_depend_on_the_toolchain
         (FNV-1a 알려진 값 하나를 잘못 적었다 — 계산 쪽이 아니라 기대값을 고쳤다)

2회차  test: FAIL exit=101   206 passed; 1 failed
       세 mode를 한 번에 보고하도록 테스트를 고친 뒤:
         meeting: 선언 v1.meeting.00000000 ≠ 계산 v1.meeting.5c6b8a90
         study:   선언 v1.study.00000000   ≠ 계산 v1.study.2cfad9a0
         summary: 선언 v1.summary.00000000 ≠ 계산 v1.summary.beca7d6c

3회차  선언 상수를 계산값으로 고친 뒤 → build · lint · test 전부 PASS
```

**관찰**: 프롬프트를 건드리면 선언값을 고치기 전에는 Gate를 통과할 수 없다는 것이
가정이 아니라 실행 기록으로 남았다 (AC6).
