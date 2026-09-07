# TASK-075 — self-check 결과 (**advisory** · Gate 판정이 아니다 · 문서 수치의 근거도 아니다)

Run: `RUN-20260907T072028Z-TASK-075` (Attempt 2) · 2026-09-07

**이 Task의 `stop_condition`에는 enabled된 Gate가 없다** (`gates: (none enabled)`).
그러므로 **Runtime이 이 Run에 대해 목격한 Gate 실행은 없다.**

## 1. 실제 출력 (편집 후 · 이 Run)

```text
$ node tools/loop-runtime/loopctl.mjs self-check
Self-check (advisory — the runtime reruns gates independently after the worker finishes)
Gates: build, lint, test

[build] npm run build
[lint] npm run lint
[test] npm run test

build: PASS  exit=0  1.2s
lint:  PASS  exit=0  2.2s
test:  PASS  exit=0  8.0s

Self-check: all gates passed
Artifacts: .loop-local/self-check/  (advisory only — not a gate report)
```

## 2. 이 실행이 무엇이고 무엇이 아닌가

```text
이것이다        문서만 바꾼 변경이 저장소를 깨뜨리지 않았다는 참고 신호.
                세 Gate 중 어느 것도 docs/ 를 읽지 않으므로 결과가 같은 것이 예상된 결과다

이것이 아니다    완료 판정 — Runtime 이 Worker 종료 뒤 독립적으로 다시 판단한다
                Runtime 이 목격한 실행 — 도구가 스스로 "advisory only — not a gate report" 라고 말한다
                문서에 적힌 어떤 수치의 근거 — 이 실행에서 나온 수치는 문서에 하나도 들어가지 않았다
```

## 3. Attempt 1이 이 자리에서 틀렸던 것 — 고친 내용

Attempt 1은 이 파일의 self-check 출력에서 테스트 수를 세어 **vitest 586 · Rust 885 · 합계
1,471**을 문서 넷 자리에 넣고, 그것을 *"이 문서를 쓴 Run이 실제로 돌린 Gate"* 라고 적었다.
**Runtime이 목격하지 않은 실행을 근거로 든 진술이었고, Verifier가 AC4로 지적했다.**

Attempt 2는 **수치를 지우지 않고 근거를 옮겼다** — 같은 값이 **Runtime이 스스로 실행해
남긴 로그**에 그대로 있기 때문이다. 확인 절차와 원자료 경로는 `gate-provenance.md`.
문서 넷은 이제 전부 *"이 문서를 쓴 Run의 실행이 아니다"* 를 함께 적는다:

```text
docs/PHASE-5.6-HUMAN-REVIEW.md   §0.1 [E3] 정의 + 그 아래 ⚠️ 블록 · §2.5 · §7 VERIFIED(자동)
docs/SYSTEM-MAP.md               §5 Phase 5.6 절의 '검증:' · §6 Automated validation 행
```

## 4. 이 Gate들이 판정하지 **않는** 것

```text
한국어 회의가 읽을 만하게 전사되는가
ggml-base 로 충분한가
Metal 전후로 체감이 달라지는가
내보낸 파일을 실제로 열 수 있는가
긴 handoff 를 실제 AI 채팅에 넣을 수 있는가
```

**다섯 다 자동 Gate로 판정할 수 없다.** 절차와 빈 기록표는
`docs/PHASE-5.6-HUMAN-REVIEW.md`이며, 그 문서 §0이 항목마다 *왜* Gate가 답하지 못하는지를 적는다.
