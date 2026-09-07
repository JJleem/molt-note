# TASK-075 — 문서에 적힌 테스트 수치의 출처 (재확인 절차)

Attempt 1의 AC4 실패는 **수치가 틀렸다는 것이 아니라 근거가 목격되지 않은 실행이었다**는
지적이었다. 이 파일은 같은 수치를 **Runtime이 스스로 실행해 남긴 artifact**에 다시 걸고,
제3자가 그것을 재확인하는 절차를 적는다.

## 1. 어느 실행인가

```text
Run          RUN-20260907T062848Z-TASK-074
Task         TASK-074 — 이 Phase(5.6)의 마지막 엔지니어링 Task (네 불변 전용 테스트)
왜 이것인가   TASK-074 이후 이 Phase가 바꾼 것은 docs/ 아래뿐이고,
             build · lint · test 세 Gate 중 어느 것도 docs/ 를 읽지 않는다.
             그래서 그 실행의 결과가 오늘의 저장소에도 그대로 선다.
경로          .loop-local/runs/RUN-20260907T062848Z-TASK-074/   (Runtime 소유 · 이 Run은 읽기만 했다)
```

## 2. Gate 결과 — `gate-report.json`

세 Gate가 각각 `"status": "PASS"` · `"exit_code": 0`으로 기록돼 있다.

```bash
grep -n '"name"\|"status"\|"exit_code"' \
  .loop-local/runs/RUN-20260907T062848Z-TASK-074/gate-report.json
```

```text
"name": "build"   "status": "PASS"   "exit_code": 0
"name": "lint"    "status": "PASS"   "exit_code": 0
"name": "test"    "status": "PASS"   "exit_code": 0
```

## 3. 테스트 수 — `gates/test/stdout.log`

### 3.1 web (vitest)

```bash
grep -n "Tests .*passed" \
  .loop-local/runs/RUN-20260907T062848Z-TASK-074/gates/test/stdout.log
```

```text
13:  Test Files  29 passed (29)
14:       Tests  586 passed (586)
```

### 3.2 Rust (cargo test)

`test result: ok. N passed` 줄이 **35개**이며 그 합이 885다.

```bash
grep -c "test result: ok" \
  .loop-local/runs/RUN-20260907T062848Z-TASK-074/gates/test/stdout.log
# 35

grep -o "test result: ok\. [0-9]* passed" \
  .loop-local/runs/RUN-20260907T062848Z-TASK-074/gates/test/stdout.log
```

```text
525  0  3  14  17  6  7  9  26  5  5  16  6  13  15  12  7  16  8  10
 10 12  9  14   4  7 31  7   6 13 10  12  5  21  4
────────────────────────────────────────────────────────────
합계 885
```

### 3.3 합계

```text
vitest   586
Rust     885
────────────
합계   1,471
```

## 4. 이 Run(TASK-075)이 한 것과 하지 않은 것

```text
했다        위 경로의 파일을 읽었다. docs/ 아래 다섯 파일을 편집했다.
            참고로 loopctl self-check 를 한 번 돌렸다 (advisory · self-check.md)

하지 않았다  Gate 를 선언하지 않았다 (stop_condition: gates (none enabled))
            그러므로 Runtime 이 이 Run 에 대해 목격한 Gate 실행이 없다
            self-check 출력에서 새 수치를 만들어 문서에 넣지 않았다
            .loop-local/ 아래의 어떤 파일도 쓰거나 고치지 않았다
```

## 5. 문서에서 이 수치가 나오는 자리 — 전부 출처가 함께 적혀 있다

| 파일 | 자리 | 어떻게 적혀 있는가 |
| --- | --- | --- |
| `docs/PHASE-5.6-HUMAN-REVIEW.md` | §0.1 [E3] 정의 | *"이 문서를 쓴 Run의 실행이 아니다"* + TASK-074 Run 경로 |
| `docs/PHASE-5.6-HUMAN-REVIEW.md` | §0.1 아래 ⚠️ 블록 | 이 Task가 Gate를 선언하지 않았다는 것 · self-check가 advisory라는 것 |
| `docs/PHASE-5.6-HUMAN-REVIEW.md` | §2.5 | 블록 첫 줄이 Runtime 소유 경로 |
| `docs/PHASE-5.6-HUMAN-REVIEW.md` | §7 `VERIFIED (자동)` | *"TASK-074 Run에서 Runtime이 직접 돌린 … 이 문서를 쓴 Run의 실행이 아니다"* |
| `docs/SYSTEM-MAP.md` | §5 Phase 5.6 절 `검증:` | 같은 문장 + 경로 + *"마무리 Task(TASK-075)는 문서 전용이고 Gate를 선언하지 않았다"* |
| `docs/SYSTEM-MAP.md` | §6 Automated validation 행 | 괄호 안에 *"TASK-074 Run에서 Runtime이 직접 돌린 test Gate 로그의 값이다 · §5"* |

**앞선 Phase의 수치(406 · 285 · 1,238 · 1,405)는 지우지 않았다** — 그것은 그 Phase들의 기록이다.
