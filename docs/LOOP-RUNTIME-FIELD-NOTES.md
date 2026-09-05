# Loop Runtime — Field Notes

> **이 문서는 Runtime 설계 근거(design provenance)다. 현재 프로젝트의 상태가 아니다.**
>
> 아래에 나오는 Task ID(`TASK-001` ~ `TASK-008`), Plan/Run/Execution ID, 비용($), 소요 시간,
> Run 수, 토큰 수치는 전부 **Loop Runtime 개발에 쓰인 canonical field-test 프로젝트**의
> Phase 1 실측 기록이다. Starter Pack을 복사해 새로 시작한 프로젝트의 Task 상태나
> 실행 결과를 뜻하지 않는다. 새 프로젝트에는 저 Task도, 저 Run도 존재하지 않는다.
>
> 이 수치들을 지우거나 일반화하지 않는다. Runtime의 어떤 결정이 **어떤 실측 때문에**
> 내려졌는지가 이 문서의 존재 이유이기 때문이다. 새 프로젝트에서 관찰한 것은
> `OBS-` 순번을 이어서 따로 기록한다.

이 문서는 실제 프로젝트에서 Loop Runtime을 사용하면서 발견한 **불편함, 반복 문제, 개선 후보, 운영상 제약**을 기록한다.

목적은 다음을 구분하는 것이다.

- 실제 Runtime 사용성 문제
- 프로젝트 고유 문제
- Planner / Task 분해 문제
- Gate / Verifier / Retry 동작 문제
- 과도한 토큰·비용·시간 사용
- 향후 자동화 후보
- 아직 필요성이 검증되지 않은 아이디어

> 원칙: 문제나 아이디어를 발견했다고 바로 Runtime을 수정하지 않는다. 먼저 실제 사례와 Evidence를 기록하고, 반복되거나 영향이 큰 문제가 확인된 뒤 Runtime 개선 대상으로 승격한다.

---

## 기록 규칙

새로운 관찰은 `OBS-001`, `OBS-002`처럼 순번을 붙인다.

가능하면 아래 정보를 함께 남긴다.

- 관련 Plan ID
- 관련 Task ID
- 관련 Run / Execution ID
- 실행한 명령
- 실제로 발생한 현상
- 기대했던 동작
- 현재 workaround
- 영향도
- 개선 아이디어

Runtime 내부 구현 아이디어만 있고 실제 사용 사례가 없다면 우선 `Idea`로 기록하고, 실제 사례가 생기기 전까지 구현 우선순위로 간주하지 않는다.

Observation의 `Status`는 다음 중 하나다. **관찰 본문은 상태가 바뀌어도 고쳐 쓰지 않는다** —
무엇을 보고 무엇을 고쳤는지가 남아야 하기 때문이다.

- `OBSERVED` — 관찰됨. 아직 Runtime에서 해결되지 않음.
- `VALIDATED` — 다른 관찰이 예측한 문제가 실제로 재현됨.
- `RESOLVED (V0.x)` — 해당 Runtime 버전에서 해소됨. 어떤 CI 항목으로 고쳤는지 함께 적는다.

---

# Observations

## OBS-001 — Template

**Date:**

**Project phase / Goal:**

**Plan / Task / Run / Execution:**

**Runtime stage:**

- Planner
- Approval
- READY / dependency
- Worker
- Gate
- Verifier
- Diagnose
- Retry
- Execute loop
- CLI / Bootstrap
- Other

### What happened

실제로 어떤 일이 발생했는지 적는다.

### Expected

어떤 동작이 더 자연스럽거나 유용했는지 적는다.

### Current workaround

현재는 어떻게 우회했는지 적는다.

### Impact

- Low
- Medium
- High

### Possible Runtime improvement

가능한 개선 방향이 있다면 적는다. 해결책이 명확하지 않으면 비워둬도 된다.

### Evidence

관련 명령, Plan / Task / Run / Execution ID, artifact path 등을 적는다.

### Status

`OBSERVED`

---

## OBS-002 — Worker가 명령을 실행할 수 없어 self-check도 외부 검증도 불가능했다

**Date:** 2026-08-26

**Project phase / Goal:** Phase 1 — Asset Inspection Foundation

**Plan / Task / Run / Execution:**
- PLAN-20260826T052332Z
- TASK-001 / RUN-20260826T052916Z-TASK-001 / EXEC-20260826T052916Z-TASK-001 (DONE, $2.0895)
- TASK-002 / RUN-20260826T053857Z-TASK-002 / EXEC-20260826T053857Z-TASK-002 (NEEDS_HUMAN, $2.7520)
- TASK-003 / RUN-20260826T055601Z-TASK-003 / EXEC-20260826T055601Z-TASK-003 (DONE, $4.6265)

**Runtime stage:** Worker

> **3연속 재현.** TASK-001·002·003 모든 Run에서 동일하게 발생했다. 일회성 환경 문제가 아니라 이 Runtime 배치의 상시 조건으로 봐야 한다.

### What happened

두 Run 모두에서 Worker가 셸 명령을 거의 실행하지 못했다. permission layer가 `This command requires approval`로 거부했고, 허용된 것은 `ls`/`grep`/`sed`/`sha256sum`/`git status`/`node --version` 수준이었다.

구체적 영향:

1. **TASK-001 (라이브러리 검증)** — Goal이 명시적으로 "실제 현재 버전과 공식 문서를 확인하라"고 요구했으나 `npm view`, `curl registry.npmjs.org`, WebFetch, WebSearch, npm 캐시 읽기가 전부 차단됐다. Worker는 추측으로 채우는 대신 three.js / @loaders.gl/ply / playcanvas / splat-transform의 모든 registry·문서 파생 필드를 `UNVERIFIED`로 표기하고, 결론이 그 미검증 필드에 의존하지 않도록 설계했다(`docs/PHASE-1-LIBRARY-DECISION.md` §0, §4.3, Appendix A).
2. **TASK-002 (구현)** — `npm test`, `npm run lint`, `npx tsc -b`, `node <file>` 모두 거부돼 Worker가 자기 코드를 **한 번도 실행해보지 못한 채** REVIEW를 요청했다. Worker note에 "build/lint/test 결과는 내가 확인한 것이 아니다"라고 명시했다.
3. **TASK-003 (구현)** — 거부 범위가 더 넓어졌다. `npm test`, `npx vitest`, `./node_modules/.bin/vitest`, `node <file>`에 더해 **`node -e`까지** 거부됐다. Worker note: "AC4/AC5는 Worker 측 증거가 없으며 통과를 주장하지 않는다. Runtime의 gate 실행만이 유일한 권위다." 대신 정적 리뷰로 strict 모드 타입 위험 2건을 제거했다.
4. 세 Run 모두 `.loop/evidence/<TASK>/`에 쓰기가 거부됐다. TASK-001은 증거를 결과 문서 Appendix로 인라인했고, TASK-002는 소스 경로 + sha256으로, TASK-003은 `.loop-local/runs/<RUN-ID>/`에 대신 기록했다 — 즉 **KERNEL이 지정한 evidence 경로가 세 번 다 사용 불가능했고, Worker마다 다른 우회책을 즉흥적으로 골랐다.** 증거 위치가 Run마다 달라지는 것 자체가 부수적 문제다.

### Expected

Worker가 최소한 프로젝트의 Gate 명령(`npm run build` / `lint` / `test`)과 `.loop/evidence/` 쓰기는 할 수 있어야 한다. Gate가 어차피 Runtime 측에서 다시 돌기 때문에 결과 자체는 보장되지만, Worker가 실행 피드백 없이 코드를 쓰면 Gate 실패 → retry 사이클이 늘어나고 그만큼 비용이 커진다.

### Current workaround

- Worker가 "확인하지 못했다"를 정직하게 note에 남기고 Gate에 판정을 위임 — 실제로 두 Task 모두 Gate가 첫 시도에 PASS해서 문제가 표면화되지 않았다.
- 네트워크 검증은 Phase 2로 연기(`PHASE-1-LIBRARY-DECISION.md` §5).

### Impact

Medium→High (3연속 재현으로 상향) — 세 Task 모두 Gate가 첫 시도에 PASS해서 아직 retry 비용으로 드러나지 않았다. 하지만 그건 운이고, 실행 피드백 없는 Worker는 retry 비용을 구조적으로 키운다. TASK-003은 912줄을 한 번도 돌려보지 않고 작성했다($4.6265). 그리고 "외부 검증"을 요구하는 Goal은 이 환경에서 **원리적으로 만족 불가능**한데, Verifier는 그걸 PASS로 판정했다(AC1이 `UNVERIFIED` fallback을 명시적으로 허용했기 때문). AC가 fallback을 허용하면 Goal의 핵심 요구가 조용히 무력화될 수 있다.

### Possible Runtime improvement

- Runtime이 Worker에게 부여된 실제 capability(명령 실행 / 네트워크 / evidence 쓰기)를 Worker Context에 명시적으로 선언한다. 지금은 Worker가 하나씩 시도해보고 거부당하며 알아낸다.
- Planner가 "외부 검증 필요" Task를 만들 때 Runtime이 네트워크 가용 여부를 사실로 알려주면, 애초에 만족 불가능한 AC를 만들지 않을 수 있다.
- 또는 Worker 시작 전에 required gate 명령의 실행 가능 여부를 preflight로 확인하고, 불가능하면 Worker Context에 "self-check 불가"를 사실로 넣는다.
- `.loop/evidence/` 쓰기 가능 여부도 같은 preflight에 포함하고, 불가능하면 Runtime이 대체 경로를 **지정**한다. 지금은 Worker가 매번 다른 곳을 고른다.

### Evidence

- `docs/PHASE-1-LIBRARY-DECISION.md` §0 표, Appendix A.1 (차단된 7개 시도 기록)
- `.loop-local/runs/RUN-20260826T053857Z-TASK-002/worker-result.json` → `notes`
- `.loop-local/runs/RUN-20260826T055601Z-TASK-003/worker-result.json` → `notes` (`node -e`까지 거부, evidence 경로 우회)
- `.loop-local/executions/EXEC-20260826T052916Z-TASK-001/execution-report.json`

### Status

`RESOLVED (V0.1)` — CI-003 + CI-007로 해소. Worker Context가 capability를 사실로 선언하고,
`worker/policy.mjs`가 `.loop/evidence/<TASK-ID>/` 쓰기를 실제로 열었다. V0.1 §1 · §2 참조.
관찰 당시의 실측(Phase 1 8 Task / 9 Run 전부 재현)과 예측된 retry 비용($4.04, OBS-007)은 위 본문에 그대로 둔다.

---

## OBS-003 — Gate 실행 중 사람이 만든 무관한 파일 하나가 Run 전체를 NEEDS_HUMAN으로 세웠다

**Date:** 2026-08-26

**Project phase / Goal:** Phase 1 — Asset Inspection Foundation

**Plan / Task / Run / Execution:**
- PLAN-20260826T052332Z
- TASK-002 / RUN-20260826T053857Z-TASK-002 / EXEC-20260826T053857Z-TASK-002

**Runtime stage:** Execute loop (Gate → Verifier 사이), Subject fingerprint

### What happened

`loopctl execute TASK-002`가 다음과 같이 진행됐다.

```
Worker   success -> REVIEW
Gate     PASS  (build PASS · lint PASS · test PASS)
Verifier (실행되지 않음)
Stop     NEEDS_HUMAN / RECOVERY_AMBIGUOUS
         "the repository changed after gates ran;
          the runtime cannot prove that rerunning gates is safe."
```

원인은 subject fingerprint 불일치였다.

| | sha256 | dirty entries |
|---|---|---|
| Gate 시점 | `7b4119a0…308707` | 99 |
| 정지 시점 | `281916c7…2b56df9f` | 100 |

정확히 파일 하나가 늘었다. 그 파일은 **`CLAUDE.local.md`** 이고, mtime은 `14:46:42.211`(= `05:46:42Z`)다. Gate 실행 구간은 `05:46:34.090Z ~ 05:46:49.466Z`였으므로 **Gate가 도는 도중에 생성됐다.**

이 파일은 Worker의 `changed_files`에 없고 (Worker는 `05:46:33.5`에 이미 REVIEW로 전이 완료), 내용도 대화형 세션 운영 지침이다. 즉 Run 바깥에서 사람이 만든, **제품 코드와 아무 관련 없는 파일**이다.

`subject.mjs`는 `.loop-local/`만 제외하고 `git status --untracked-files=all`이 보고하는 전부를 지문에 넣는다. 이 저장소는 초기 커밋 하나뿐이라 사실상 모든 파일이 untracked라서, 작업 트리 아무 곳의 어떤 변경이든 지문을 바꾼다.

결과: Worker와 Gate가 모두 성공했는데도 Verifier가 돌지 않았고, $2.7520을 쓴 Run이 사람 개입 대기 상태로 남았다.

### Expected

Runtime이 멈춘 판단 자체는 옳다 — Gate와 Verifier가 서로 다른 대상을 봤다면 검증은 무의미하다. 문제는 **정지가 유일한 선택지였다는 점**이다.

기대했던 동작: 무엇이 바뀌었는지를 Runtime이 스스로 보고하는 것. 지금은 "저장소가 바뀌었다"만 말하고, 어떤 경로가 추가/변경/삭제됐는지는 알려주지 않는다. `loopctl diagnose TASK-002`도 `No failure recorded / NO_ACTION`만 답한다(Worker도 Gate도 실패하지 않았으므로 진단할 실패가 없다). 그래서 사람이 gate-report의 `verification_subject`를 직접 꺼내 현재 subject를 재계산하고 mtime을 비교해야 원인을 알 수 있었다.

### Current workaround

원인 파일 확정 후 gate 재실행 → verify 순으로 수동 복구.

```
loopctl gate TASK-002 --rerun     # 현재 subject 기준으로 Gate 재실행 (AI 호출 0)
loopctl verify TASK-002           # Gate와 같은 subject에서 Verifier 실행
```

원인을 찾는 데 쓴 명령:

```
node -e "import('./tools/loop-runtime/subject.mjs').then(m=>console.log(m.computeSubject().sha256))"
node -e "console.log(require('./.loop-local/runs/<RUN>/gate-report.json').verification_subject)"
find . -newermt "<gate 시작 시각>" -not -path "./node_modules/*" -not -path "./.git/*" -not -path "./.loop-local/*"
```

### Impact

High — 발생 빈도가 높다. Runtime을 쓰는 동안 사람이 같은 작업 트리에서 메모를 쓰거나 파일을 열어보는 것은 정상적인 행동인데, 그것만으로 성공한 Run이 무효화되고 그때까지의 비용이 검증 없이 남는다. Field Notes 문서를 쓰는 행위(`docs/LOOP-RUNTIME-FIELD-NOTES.md` 편집)조차 실행 중이면 같은 문제를 일으킨다 — 이 Runtime의 운영 규칙 자체가 Runtime을 깨는 구조다.

### Possible Runtime improvement

증거 기준 우선순위:

1. **정지 메시지에 diff를 넣는다.** gate-report에 `verification_subject.entries`(경로 + 해시)를 저장하고, 정지 시 `ADDED / REMOVED / CHANGED` 경로 목록을 출력한다. 지금은 `dirty_entry_count`와 최종 sha256만 남아서 사람이 재구성해야 한다. — 가장 싸고 확실한 개선.
2. **명시적 복구 경로를 CLI로 제공한다.** `loopctl resume <RUN>` 같은 한 명령으로 "gate 재실행 → verify"를 잇는다. 지금은 사람이 두 명령의 순서와 `--rerun` 필요성을 알아야 한다.
3. **subject 범위를 좁힐 수 있게 한다.** `.loop/policies/`에 subject 제외 glob(예: `CLAUDE.local.md`, `docs/LOOP-RUNTIME-FIELD-NOTES.md`, `*.md` 중 Gate 대상이 아닌 것)을 선언 가능하게 한다. 단 이건 검증 엄밀성을 깎는 방향이므로 1·2번을 먼저 하고 반복 사례가 더 쌓인 뒤 판단한다.
4. (장기) per-Task worktree isolation — 이미 `Ideas` 섹션에 있는 항목인데, 이 관찰이 그 아이디어의 **첫 실제 근거**다. 작업 트리를 사람과 Runtime이 공유하는 한 이 문제는 구조적으로 남는다.

### Evidence

- `.loop-local/executions/EXEC-20260826T053857Z-TASK-002/execution-report.json` → `events[2]` (`stop / RECOVERY_AMBIGUOUS`)
- `.loop-local/runs/RUN-20260826T053857Z-TASK-002/gate-report.json` → `verification_subject` (sha `7b4119a0…`, 99 entries)
- `.loop-local/runs/RUN-20260826T053857Z-TASK-002/recovery/diagnosis.json` → `NO_ACTION`
- `tools/loop-runtime/subject.mjs` — `EXCLUDE_PREFIXES = ['.loop-local/']`
- 원인 파일: `CLAUDE.local.md`, mtime `2026-08-26 14:46:42.211 +0900`

### Status

`OBSERVED` — **미해소.** 근본 해결은 per-Task worktree 격리이며 CI-001 · CI-002와 함께 CANDIDATE로 남아 있다.
V0.1에서 고치지 않았다. shared working tree를 쓰는 동안에는 그대로 재현될 수 있다.

---

## OBS-004 — 수동 복구로 Task가 DONE이 돼도 Execution Report는 NEEDS_HUMAN으로 남는다

**Date:** 2026-08-26

**Project phase / Goal:** Phase 1 — Asset Inspection Foundation

**Plan / Task / Run / Execution:**
- TASK-002 / RUN-20260826T053857Z-TASK-002 / EXEC-20260826T053857Z-TASK-002

**Runtime stage:** Execute loop, CLI (status 표시)

### What happened

OBS-003의 `RECOVERY_AMBIGUOUS` 정지를 아래 두 명령으로 수동 복구했고, 복구는 정확히 의도대로 동작했다.

```
loopctl gate TASK-002 --rerun     # PASS  build 4.9s / lint 0.4s / test 9.1s
                                  # 이전 gate 증거는 gate-history/1/ 로 보존됨
loopctl verify TASK-002           # PASS  AC1·AC2·AC3,  51.8s,  $0.4646
                                  # TASK-002: REVIEW -> DONE
```

Gate와 Verifier가 **동일한 subject를 봤음이 기록으로 증명된다** — 양쪽 리포트 모두 `4d6361fa84bee57e…f430bf` (100 entries). 즉 subject 안정성 요구가 실제로 충족됐다.

그런데 `loopctl status` 출력은 이렇게 나온다.

```
DONE
  TASK-002             Create the Phase 1 type foundation and the input-...
      latest execution: NEEDS_HUMAN  (RECOVERY_AMBIGUOUS)
```

Task 상태(DONE)와 latest execution 요약(NEEDS_HUMAN)이 서로 모순돼 보인다. `EXEC-20260826T053857Z-TASK-002/execution-report.json`은 정지 시점에 봉인됐고, 그 뒤 `gate --rerun` / `verify`는 execute 루프 **밖에서** 실행됐으므로 Execution Report에 반영되지 않았다.

### Expected

두 가지 중 하나가 자연스럽다.

- 저수준 명령으로 Run이 최종 상태에 도달하면 Runtime이 해당 Execution Report에 후속 이벤트를 append하거나 `superseded_by` 같은 포인터를 남긴다.
- 또는 `status`가 Task 상태와 execution 요약이 불일치할 때 "수동 복구됨"으로 표시한다.

지금은 나중에 이 프로젝트를 다시 볼 때 "DONE인데 왜 NEEDS_HUMAN이지?"를 다시 조사해야 한다. Run 디렉터리를 열어 gate-report와 verification-report를 확인해야만 실제로 무슨 일이 있었는지 알 수 있다.

### Current workaround

없음. 실제 결과는 `.loop-local/runs/RUN-20260826T053857Z-TASK-002/verification/verification-report.json`이 정본이고, Execution Report는 "execute 루프가 어디서 멈췄는가"의 기록으로만 읽으면 된다. 이 노트가 그 해석을 남기는 역할을 한다.

### Impact

Low — 실제 상태(DONE)와 증거(gate/verify report)는 정확하다. 표시상의 혼동일 뿐이고 잘못된 PASS도 아니다. 다만 OBS-003이 자주 재발하면 이 혼동도 같은 빈도로 따라온다.

### Possible Runtime improvement

- Execution Report에 terminal 여부를 명시하고, 이후 저수준 명령으로 상태가 바뀌면 `superseded_by: <run/verification>`를 append한다.
- 또는 `status`에서 Task가 DONE인데 latest execution이 non-terminal이면 `latest execution: NEEDS_HUMAN (manually recovered)`로 표기한다.
- CI-002(`loopctl resume <RUN>`)가 구현되면 복구가 execute 루프 안에서 일어나므로 이 문제는 자연히 사라진다. → OBS-004는 CI-002의 추가 근거다.

### Evidence

- `.loop-local/runs/RUN-20260826T053857Z-TASK-002/gate-report.json` → `verification_subject.sha256 = 4d6361fa…f430bf`, 100 entries
- `.loop-local/runs/RUN-20260826T053857Z-TASK-002/verification/verification-report.json` → 동일 subject, PASS
- `.loop-local/runs/RUN-20260826T053857Z-TASK-002/gate-history/1/` (OBS-003 시점의 gate 증거)
- `.loop-local/executions/EXEC-20260826T053857Z-TASK-002/execution-report.json` → 여전히 `result: NEEDS_HUMAN`
- `loopctl status` 출력

### Status

`RESOLVED (V0.1)` — `loop/reconcile.mjs`가 사람이 끝낸 복구를 `origin: manual` + `supersedes`로
새 Execution Report에 남긴다. 앞선 Report는 고쳐 쓰지 않는다. V0.1 §5 참조.
(`loopctl resume`(CI-002)은 여전히 별개 CANDIDATE다.)

---

## OBS-005 — Task당 비용이 Run마다 단조 증가한다 (Runtime context는 일정한데 Worker 세션이 커진다)

**Date:** 2026-08-26

**Project phase / Goal:** Phase 1 — Asset Inspection Foundation

**Plan / Task / Run / Execution:** PLAN-20260826T052332Z, TASK-001 ~ TASK-004 전체

**Runtime stage:** Worker, Execute loop (비용/telemetry)

### What happened

Phase 1의 앞 네 Task는 모두 **첫 시도에 Gate·Verifier PASS**했다. retry도 실패도 없었다. 그런데 Task당 비용이 단조 증가했고, 증가폭 자체도 커졌다.

| Task | 결과 | 시간 | 총비용 | Worker | Verifier | Worker output tok | Worker cached_input tok |
|---|---|---|---|---|---|---|---|
| TASK-001 | DONE | 6.1m | $2.0895 | $1.4938 | $0.5957 | 18,679 | 1,275,383 |
| TASK-002 | NEEDS_HUMAN* | 7.9m | $2.7520 | $2.7520 | — | 35,693 | 2,467,434 |
| TASK-003 | DONE | 14.0m | $4.6265 | $3.9183 | $0.7082 | 58,488 | 2,939,290 |
| TASK-004 | DONE | 15.2m | $6.8281 | $6.0060 | $0.8222 | 66,097 | 6,214,735 |

\* TASK-002는 OBS-003으로 Verifier 미실행. 이후 수동 `verify`에 $0.4646 추가 → 실제 $3.2166. Phase 1 누계 **$16.76**.

핵심은 **어느 쪽이 커지는가**다.

- **Verifier는 안정적이다.** $0.5957 → $0.7082 → $0.8222, cached_input은 174,871 / 83,260 / 246,745로 등락만 있다. Verifier context는 고정 스냅샷이라(TASK-002 기준 26.9 KB / 622줄) 경계가 있다.
- **Worker가 전부 끌고 간다.** $1.49 → $2.75 → $3.92 → $6.01, **4배**. cached_input은 1.28M → 2.47M → 2.94M → **6.21M**으로 거의 5배.

그리고 **Runtime이 Worker에게 주는 context는 거의 일정하다.**

```
RUN-...TASK-001  context.md = 9,276 B
RUN-...TASK-002  context.md = 9,086 B
RUN-...TASK-003  context.md = 9,231 B
RUN-...TASK-004  context.md = 9,273 B
```

즉 비용 증가의 원인은 Runtime의 context 조립이 아니다. Worker가 **자기 agentic 세션 안에서 스스로 읽어들이는 양**이다. 작업 트리가 Task마다 커지므로(Phase 1 종료 시점 src/ 누적 1,700줄+) Worker가 기존 코드를 읽고 재확인하는 턴이 늘고, 턴마다 누적 컨텍스트가 다시 실려 cached_input이 제곱에 가깝게 불어난다.

### OBS-002와의 인과 관계 (추정, 미검증)

OBS-002(Worker가 명령을 전혀 실행할 수 없음)가 이 증가를 **증폭**시키고 있을 가능성이 높다. Worker는 테스트를 돌려보는 대신 정적 검증으로 보상하는데, 그 보상 행위가 정확히 컨텍스트를 키우는 행위다. TASK-004 Worker note가 그 증거다.

> "각 fixture의 헤더 크기를 reader의 1024바이트 초기 윈도에 맞춰 **손으로 계산**했다."
> "strict 모드 유무와 무관하게 타입 체크되도록 작성했다 — non-null assertion 없음, ..."

`npm test` 한 번이면 끝날 확인을 파일을 반복해 읽으며 손으로 대신하고 있다. 다만 이건 상관관계 관찰이며, Worker에게 실행 권한을 준 대조군이 없으므로 **인과로 확정하지 않는다.**

### Expected

- 첫 시도에 전부 통과하는 Task들의 비용이 Task 크기가 아니라 **Task 순번**에 따라 오르는 것은 직관에 반한다. 같은 난이도의 Task가 프로젝트 후반이라는 이유만으로 3배 비싸다면 장기 프로젝트에서 Runtime 사용이 성립하지 않는다.
- Runtime이 Worker telemetry를 이미 기록하고 있으므로(`loopctl usage`), **추세를 사람이 계산하지 않아도 보이게** 하는 편이 자연스럽다.

### Current workaround

없음. 사람이 execution-report의 `usage_summary.invocations`를 직접 집계해야 추세가 보인다. 이번에도 아래로 손으로 뽑았다.

```
for e in .loop-local/executions/EXEC-*/execution-report.json; do
  node -e "const r=require('$PWD/$e'); ..."   # task_id, result, duration, cost, tokens
done
```

### Impact

Medium (관찰 시점) → 남은 Task에서 재확인 필요. 추세가 유지되면 High.

TASK-005(얇은 진입점)는 추세가 꺾이는지 보는 대조군이 된다. TASK-007·008은 통합 Task라 가장 비쌀 것으로 예상되며, 이 추세대로면 Phase 1 총액은 $35~45 범위가 된다. **예측이 맞는지는 Phase 종료 후 이 노트에 실측으로 덧붙인다.**

### Possible Runtime improvement

증거가 더 필요한 순서대로:

1. **`loopctl usage`에 Task 간 추세를 보여준다.** 이미 데이터는 다 있다. Run별 worker/verifier 비용과 token을 표로 누적 출력하면 사람이 집계할 필요가 없다. — 가장 싸다.
2. **Task별 예산 상한과 경고.** `.loop/policies/limits.yaml`에 Worker 비용/토큰 상한을 두고 초과 시 경고하거나 정지한다. 지금 `limits.yaml`은 실패 횟수만 다룬다.
3. **Worker context에 "읽어야 할 파일" 힌트를 넣는다.** Planner가 이미 Task별 관련 경로를 알고 있다(P3 → plyHeader, P4 → plyHeader+plyAnalyzer). 그걸 Worker Context에 넘기면 Worker의 탐색 턴이 줄어든다. 단 Worker의 자율 탐색을 제한하는 방향이라 품질 영향을 같이 봐야 한다.
4. **OBS-002 해소(CI-003)가 이 문제도 완화하는지 측정한다.** Worker에게 gate 명령 실행 권한을 준 Run과 그렇지 않은 Run의 비용을 비교하면 위 인과 추정을 검증할 수 있다. → CI-003의 가치가 "편의"가 아니라 "비용"일 수 있다.

### Evidence

- `.loop-local/executions/EXEC-*/execution-report.json` → `usage_summary.invocations[].provider_cost_usd`, `.tokens`
- `.loop-local/runs/RUN-*/context.md` 크기 (9,086 ~ 9,276 B, 변동 2% 미만)
- `.loop-local/runs/RUN-20260826T061218Z-TASK-004/worker-result.json` → `notes` (손 계산 보상 행위)
- 비교 대상 Verifier context: TASK-002 verify 출력 `context: 26.9 KB (24,122 chars, 622 lines)`

### Status

`OBSERVED` — **미해소.** TASK-005 실측 반영, TASK-006 ~ 008로 계속 확인.
비용 추세 표시(CI-004)와 비용 상한(CI-005)은 V0.1에서 구현하지 않았다. 둘 다 CANDIDATE로 남아 있다.

---

## OBS-006 — 실행 중인 Run이 `loopctl status`에 보이지 않아, 세션이 끊기면 살아 있는지 사람이 `ps`로 판단해야 한다

**Date:** 2026-08-26

**Project phase / Goal:** Phase 1 — Asset Inspection Foundation

**Plan / Task / Run / Execution:** PLAN-20260826T052332Z, TASK-005, RUN-20260826T064142Z-TASK-005, EXEC-20260826T064141Z-TASK-005

**Runtime stage:** status / execution 조회 (관측성)

### What happened

`loopctl execute TASK-005 --adapter claude --timeout 1800`을 띄운 인터랙티브 세션이 조작 실수로 종료됐다. loopctl 프로세스 자체는 tty에서 분리돼 **살아남았다** — 이건 올바른 동작이고 실제로 실행은 정상 완료됐다.

문제는 새 세션에서 그 사실을 확인할 방법이 Runtime 안에 없었다는 점이다.

```
$ ./loopctl status
IN PROGRESS
  TASK-005
    latest run: (none)

$ ./loopctl execution TASK-005
no execution found for TASK-005
```

이 시점에 `.loop-local/runs/RUN-20260826T064142Z-TASK-005/`는 이미 존재했고 `context.md`(8,447 B)와 `manifest.json`이 쓰여 있었다. 그런데 `status`는 `latest run: (none)`, `execution`은 `no execution found`를 반환했다. Run/Execution 레코드가 **종료 시점에만** 기록되기 때문이다.

살아 있는지는 결국 Runtime 밖에서 확인했다.

```
$ ps aux | grep loopctl
99347  /bin/bash -c ... eval './loopctl execute TASK-005 ...'
99349  node tools/loop-runtime/loopctl.mjs execute TASK-005 --adapter claude --timeout 1800
99364  claude --print --output-format json --permission-mode acceptEdits ...
```

`.loop-local/leases/`는 비어 있었다(`.gitkeep`만). lease 디렉터리가 존재하는데 실행 중 Run이 lease를 남기지 않는다.

**부수 관측 — 좀비 프로세스가 생존 폴링을 오판시킨다.** 부모 세션이 죽은 뒤 execute가 끝나자 wrapper bash(99347)가 `Zs [bash] <defunct>`로 남았다. 이를 reap할 부모가 없어 `kill -0 99347`이 계속 성공한다. 실제로 이 세션에서 PID 생존을 폴링하는 감시를 걸었다가 실행이 끝난 뒤에도 8분간 "실행 중"으로 오판했고, run 디렉터리 mtime과 자식 PID(99349/99364) 소멸을 보고서야 정정했다. PID 폴링은 이 Runtime에서 신뢰할 수 없는 신호다.

### Expected

- Run 시작 시점에 execution 레코드가 생기고 `status` / `execution`이 `RUNNING`(run_id, 시작 시각, 경과, 타임아웃)을 보여줘야 한다. 종료 시에만 기록하면 크래시·세션 단절 후 상태 복원이 원리적으로 불가능하다.
- 생존 판정 근거를 Runtime이 제공해야 한다. `.loop-local/leases/`가 이미 있으니 PID + heartbeat + timeout을 여기에 남기면 `status`만으로 "돌고 있음 / 죽었음(stale lease)"을 구분할 수 있다.

### Current workaround

`ps aux | grep loopctl` + run 디렉터리 mtime + `worker-result.json` 존재 여부로 사람이 추론. 생존 확인은 **wrapper PID가 아니라 node loopctl PID와 worker PID**로 해야 한다(위 좀비 문제).

### Impact

Medium. 진행 자체는 막히지 않았고 실행은 16m 22s만에 정상 DONE으로 끝났다. 하지만 "죽었나 살았나"를 판단하는 데 프로세스 수준 지식과 사람 개입이 필요했다. 죽은 걸로 오판하고 재실행했다면 동일 Task를 이중 실행해 shared working tree에서 충돌했을 것이다 — OBS-003이 기록한 subject staleness와 정확히 같은 실패 모드로 이어진다.

### Possible Runtime improvement

- Run 시작 시 execution 레코드 선기록 (`result: RUNNING`), 종료 시 갱신.
- `.loop-local/leases/<TASK-ID>.json`에 `{pid, run_id, started_at, timeout_s, heartbeat_at}` 기록. `status`가 heartbeat로 RUNNING / STALE 판정.
- `loopctl watch <TASK>` — 진행 중 Run의 stage 전이를 따라가는 조회 명령.

### Evidence

- `.loop-local/runs/RUN-20260826T064142Z-TASK-005/` (실행 중 `context.md` + `manifest.json`만 존재)
- `.loop-local/leases/` (비어 있음)
- `.loop-local/executions/EXEC-20260826T064141Z-TASK-005/execution-report.json` (종료 후에야 생성)

### Status

`RESOLVED (V0.1)` — CI-008. 활성 표식이 `heartbeat_at` · `stage` · `run_id`를 유지하고
`loopctl status`가 RUNNING/STALE을 표시한다. 생존 판정은 heartbeat가 정본이고 PID는 보조다
— 좀비 PID를 active로 오판하던 문제가 여기서 사라졌다. V0.1 §4 참조.

---

## OBS-007 — Worker가 Gate를 못 돌려 타입 한 줄 때문에 Attempt 1 전체($4.04, 9분)가 폐기됐다 — OBS-002의 첫 실측 비용

**Date:** 2026-08-26

**Project phase / Goal:** Phase 1 — Asset Inspection Foundation

**Plan / Task / Run / Execution:**
- PLAN-20260826T052332Z, TASK-005, EXEC-20260826T064141Z-TASK-005
- attempt 1: RUN-20260826T064142Z-TASK-005 (gate FAIL)
- attempt 2: RUN-20260826T065104Z-TASK-005 (gate PASS, verifier PASS)

**Runtime stage:** Worker capability → Gate → Diagnose → Retry

> OBS-002(Worker가 명령을 실행할 수 없음)의 **비용 근거**다. TASK-001~004는 전부 첫 시도에 통과해서 이 제약이 retry 비용으로 표면화되지 않았다. TASK-005가 첫 실측이다.

### What happened

attempt 1의 Gate 결과는 lint PASS, test PASS(98/98, 5 files), **build만 FAIL(exit 2)**. 오류는 정확히 하나, 테스트 헬퍼의 파라미터 타입이 너무 넓었다.

```
src/analyzers/assetAnalyzer.test.ts(79,29): error TS2322:
Type 'Uint8Array<ArrayBufferLike>' is not assignable to type 'BlobPart'.
```

attempt 2가 한 일은 헬퍼 파라미터를 `Uint8Array` → `Uint8Array<ArrayBuffer>`로 좁힌 **타입 주석 한 줄 수정**이다. 프로덕션 소스 변경 없음, 테스트 추가·삭제·약화 없음.

그 한 줄의 값:

| | Worker attempt 1 | Worker attempt 2 | Verifier | 합계 |
|---|---|---|---|---|
| 비용 | $4.0375 | $2.9554 | $0.7380 | **$7.7309** |
| 시간 | 539.5s | 308.5s | 86.3s | 16m 22s |
| output tok | 44,309 | 21,962 | — | 72,029 |
| cached_input tok | 3,810,510 | 3,212,858 | — | 7,240,150 |

attempt 1의 **$4.04 / 9분이 전량 폐기**됐다. 낭비 비율 52%. 이 오류를 잡는 데 필요한 것은 `tsc -b` 한 번이었고, Runtime이 gate로 돌렸을 때 실제 소요는 **3.9초**였다(`gate-report` build gate `duration_ms: 3961`).

원인은 추정이 아니다. Worker가 직접 원인과 코드 위치를 적었다.

> "GATE EXECUTION LIMITATION: this Worker could not run the build/lint/test gates locally. The Runtime launches the Worker with `--permission-mode acceptEdits` (tools/loop-runtime/adapters/claude.mjs:67), which auto-approves file edits but not Bash, and the Run is non-interactive, so 'npm run build', 'npm test', 'npm run lint', 'npx tsc' and 'node_modules/.bin/tsc' all returned 'This command requires approval'."

읽기 전용 명령은 허용되므로, Worker는 대신 `node_modules/typescript/lib/lib.es5.d.ts`와 `lib.dom.d.ts`를 직접 읽어 **TypeScript 5.7부터 typed array 인터페이스가 제네릭이 되어 bare `Uint8Array`가 `Uint8Array<ArrayBufferLike>`로 기본 해석되고, 이는 `SharedArrayBuffer`를 허용하므로 `BufferSource`를 만족하지 못한다**는 근본 원인까지 정확히 규명했다. 진단은 완벽했다. 다만 그 진단을 **3.9초짜리 명령 대신 파일을 반복해 읽어서** 했다 — OBS-005가 가설로 제시한 "정적 검증으로 보상하는 행위가 컨텍스트를 키운다"의 직접 사례다(attempt 1 cached_input 3.81M).

### 잘 동작한 부분 (기록해둘 가치가 있음)

복구 경로 자체는 설계대로 작동했다. Diagnose가 `GATE_FAILURE → RETRY_WITH_HINT`로 정확히 분류했고(`llm_calls: 0`, 결정적 판정), `subject_check.matches: true`로 subject 무결성을 확인했으며, failure memo가 build stderr 발췌와 "테스트를 지우거나 약화시켜 gate를 통과시키지 말라"는 hint를 attempt 2에 넘겼다. attempt 2는 그 한 줄만 고쳤다. **Runtime의 retry는 문제가 아니다. 문제는 retry가 필요했다는 것이다.**

### Expected

`build` / `lint` / `test`는 Runtime이 이미 `stop_condition.gates`로 알고 있고, `gate_sources`에 AC 연결까지 기록한다. **그 명령만 allow-list로 Worker에게 열어주면** 이 유형의 retry는 Worker 세션 안에서 흡수된다. Worker가 임의 명령을 실행할 필요는 없다 — Runtime이 어차피 돌릴 명령만 미리 돌려보게 하면 된다.

### Current workaround

없음. Runtime retry가 흡수하며, 비용은 그대로 지불한다.

### Impact

**High.** CI-003(Worker capability 선언)을 넘어 **Worker에게 gate 명령 실행 권한을 주는 것**이 실제 금액으로 정당화된 첫 사례다. TASK-006~008은 UI·통합 Task라 타입/빌드 실패 가능성이 더 높고, 같은 낭비가 반복될 것으로 예상한다.

### Possible Runtime improvement

- adapter 설정에 gate 명령 allow-list를 넣어 Worker에게 `stop_condition.gates`에 선언된 명령만 실행 허용.
- 또는 `loopctl gate --self-check` — subject를 건드리지 않고 gate만 돌리는 Worker 전용 진입점.
- 최소한: Worker Context에 "이 Run에서 gate 명령을 실행할 수 없다"를 **사실로 선언**해 Worker가 하나씩 시도하며 알아내는 턴(과 그 컨텍스트)을 없앤다. (= CI-003)

### Evidence

- `.loop-local/runs/RUN-20260826T064142Z-TASK-005/gate-report.json` → `result: FAIL`, build exit 2, `duration_ms: 3961`
- `.loop-local/runs/RUN-20260826T064142Z-TASK-005/recovery/failure-memo.json` → `failure_class: GATE_FAILURE`, stderr 발췌
- `.loop-local/runs/RUN-20260826T064142Z-TASK-005/recovery/diagnosis.json` → `RETRY_WITH_HINT`, `llm_calls: 0`, `subject_check.matches: true`
- `.loop-local/runs/RUN-20260826T065104Z-TASK-005/worker-result.json` → `notes` (원인 규명 + GATE EXECUTION LIMITATION 원문)
- `.loop-local/executions/EXEC-20260826T064141Z-TASK-005/execution-report.json` → `usage_summary.invocations[]`

### Status

`RESOLVED (V0.1)` — CI-006. `loopctl self-check`로 Worker가 설정된 Gate 명령을 미리 돌려볼 수 있다.
$4.04 / 9분 폐기라는 실측은 위 본문에 그대로 둔다 — 이 기능의 근거이기 때문이다. V0.1 §2 참조.

---

## OBS-008 — Worker가 self-check 불가를 "AC를 글로 논증하기"로 보상한다 (8/9 Run에서 정착된 패턴)

**Date:** 2026-08-26

**Project phase / Goal:** Phase 1 — Asset Inspection Foundation (종료)

**Plan / Task / Run / Execution:** PLAN-20260826T052332Z, TASK-006 / RUN-20260826T070648Z, TASK-007 / RUN-20260826T071431Z, TASK-008 / RUN-20260826T072745Z

**Runtime stage:** Worker (OBS-002 / OBS-007의 행동적 귀결)

### What happened

Phase 1 마지막 세 Task는 전부 첫 시도에 Gate·Verifier PASS했다. 그런데 Worker note를 보면 **매 Run이 같은 서두로 시작한다.**

- TASK-006: `"GATES NOT RUN BY THIS WORKER."`
- TASK-007: `"GATES NOT RUN BY THIS WORKER (same environment constraint recorded by TASK-006)."`
- TASK-008: `"gate commands could not be run locally because this Run's sandbox denies command execution."`

Worker는 거부당한 명령을 **일일이 나열한다**. TASK-007만 해도 `npm test`, `npm test 2>&1`, `npm run lint`, `npx vitest run src/utils/fileSize.test.ts`, `node node_modules/vitest/vitest.mjs run ...`, `./node_modules/.bin/vitest run`, `node -e "console.log(1+1)"` — 일곱 가지를 시도하고 전부 거부당한 뒤에야 포기했다. 이 탐색은 **Run마다 처음부터 반복된다.** Runtime이 "이 Run에서는 명령을 실행할 수 없다"를 사실로 알려주지 않기 때문이다.

그 다음이 더 비싸다. 실행으로 확인할 수 없으니 Worker는 **AC를 문장으로 논증한다.** TASK-006/007 note는 `AC mapping. AC1: ... Covered by the tests '...'` 형태로 각 AC를 코드 인용과 함께 수천 자에 걸쳐 정당화한다. TASK-007 Worker는 자기가 무엇을 하고 있는지 정확히 적었다.

> "Because I could not execute the suite, every new test was written against behaviour I could verify by **reading** (Testing Library's getNodeText only sees direct text-node children, ...)."

즉 **테스트를 돌리는 대신 테스트 라이브러리의 동작을 읽어서 추론**하고 있다. 결과적으로 옳았지만(131개 테스트 전부 통과), 그 정확성은 `vitest run` 12.9초로 얻을 수 있는 것을 토큰으로 산 것이다.

### Expected

OBS-002가 제안한 preflight로 충분하다. Runtime이 Worker Context에 **"이 Run에서 실행 가능한 명령 = 없음 / gate 명령만 / 전부"**를 사실로 선언하면:

1. 매 Run 반복되는 7회짜리 거부 탐색이 사라진다.
2. Worker가 "실행으로 확인 못 함"을 처음부터 알고 논증 분량을 조절할 수 있다.
3. (CI-006이 구현되면) 애초에 이 보상 행위 자체가 불필요해진다.

### Current workaround

없음. Worker의 정직성 덕분에 **잘못된 PASS 주장은 한 번도 없었다** — 이건 기록해둘 가치가 있는 긍정 신호다. 모든 Worker가 "AC4/AC5는 Runtime gate만이 권위"라고 명시하고 판정을 위임했다. 위험한 실패 모드(Worker가 돌려보지도 않고 통과를 주장)는 발생하지 않았다.

### Impact

Medium. Phase 1에서 이 보상 행위가 **잘못된 결과로 이어진 적은 없다**(8 Task 중 gate 실패 1회, 그것도 타입 한 줄). 비용 측면의 낭비이지 정확성 문제는 아니다. 다만 OBS-005 실측상 Worker 비용이 Phase 비용의 88%($31.7 / $36.0)를 차지하므로, 이 보상 행위를 없애는 것이 비용 개선의 주 레버다.

### Possible Runtime improvement

CI-003(capability 선언) + CI-006(gate 명령 allow-list). 새로운 항목은 없다 — 이 관찰은 **두 후보의 우선순위를 올리는 추가 증거**다.

### Evidence

- `.loop-local/runs/RUN-20260826T070648Z-TASK-006/worker-result.json` → `notes` (거부 명령 목록, evidence 디렉터리 전수 확인)
- `.loop-local/runs/RUN-20260826T071431Z-TASK-007/worker-result.json` → `notes` ("written against behaviour I could verify by reading")
- `.loop-local/runs/RUN-20260826T072745Z-TASK-008/worker-result.json` → `summary`
- Phase 1 독립 재확인 (Runtime 정지 후 조작자 실행): `npm run build` exit 0, `npm test` 131 passed / 9 files, `npm run lint` exit 0 (warning 1건)

### Status

`RESOLVED (V0.1)` — CI-003 + CI-006. 단독 개선 항목은 아니었고 두 후보의 우선순위를 올린 증거였다.
self-check가 생기면서 "AC를 글로 논증하기"로 보상할 이유 자체가 없어졌다. V0.1 §1 · §2 참조.

---

## OBS-009 — Verifier가 "사람이 dev server로 수동 확인했다"는 근거 없는 주장을 PASS시켰다

**Date:** 2026-08-26

**Project phase / Goal:** Phase 1 — Asset Inspection Foundation (종료)

**Plan / Task / Run / Execution:** PLAN-20260826T052332Z, TASK-008, RUN-20260826T072745Z-TASK-008, EXEC-20260826T072744Z-TASK-008 (DONE, verifier PASS)

**Runtime stage:** Verifier

### What happened

TASK-008이 생성한 `docs/PHASE-1.md` §3에 다음 문장이 있다 (227~229행).

> "**Browser-only checks.** The app has been exercised through jsdom-based tests **and manual use in a dev server**; there is no cross-browser or large-file (GB-scale) measurement recorded anywhere in this repository."

**"manual use in a dev server"는 일어나지 않은 일이다.**

- Worker는 `npm run dev`는커녕 `node -e`도 실행할 수 없었다 (OBS-002 / OBS-007 / OBS-008, Phase 1 9개 Run 전부).
- 조작자(사람)도 Phase 1 동안 dev server를 띄운 적이 없다. 저장소 어디에도 그 흔적이 없다.
- TASK-008 Worker의 `worker-result.json`에는 `npm run dev`나 dev server 언급 자체가 없다.

문장의 **의도**는 제약을 밝히는 것이었고 뒷절("cross-browser·대용량 측정 기록 없음")은 정확하다. 하지만 앞절은 검증 범위를 실제보다 넓게 주장한다. 아이러니하게도 이 문장은 **"현재 제약" 섹션**에 있다 — 제약을 적으면서 하지 않은 검증을 했다고 적었다.

Verifier는 이 Task를 PASS시켰다. 문서 형식(PRODUCT-SPEC §50)과 코드-문서 일치는 검사했지만, **문서가 저장소 밖 세계에 대해 하는 주장은 검사 대상이 아니었다.**

### Expected

Verifier가 "저장소 안에 증거가 없는 사실 주장"을 잡아내야 한다. 특히 이번 배치처럼 **Worker가 명령을 실행할 수 없는 환경**이라면, 실행·수동 조작·측정을 주장하는 문장은 원리적으로 전부 근거가 없다. Runtime은 그 사실(Worker capability)을 알고 있으므로 Verifier에게 넘겨줄 수 있다.

이건 OBS-002가 기록한 것과 같은 실패 형태다. TASK-001에서도 Goal이 요구한 외부 검증이 환경적으로 불가능했는데 Verifier가 PASS했다 — 그때는 AC가 `UNVERIFIED` fallback을 명시적으로 허용해서 정당했지만, 이번엔 **아무도 허용하지 않은 주장이 그냥 통과했다.**

### Current workaround

조작자가 Phase 종료 시 산출 문서를 직접 읽는다. 이번에도 그렇게 발견했다. 자동 검출 경로는 없다.

### Impact

Medium. 제품 동작에는 영향이 없다 (코드·테스트는 정확하고 131개 테스트 전부 통과). 하지만 **Phase 산출 문서는 다음 Phase의 입력이자 사람의 판단 근거**다. "수동 확인됨"이 문서에 남으면 Phase 2에서 브라우저 실측을 건너뛸 근거로 오독될 수 있다. 문서가 검증 상태를 과장하는 것은 OBS-002가 경계한 "VERIFIED / UNVERIFIED 구분 붕괴"와 같은 종류의 문제다.

### Possible Runtime improvement

- Verifier Context에 **이 Run의 Worker capability**(명령 실행 불가 / 네트워크 불가)를 사실로 포함한다. 그러면 Verifier가 "실행했다 / 측정했다 / 수동 확인했다" 류 주장을 자동으로 의심할 수 있다. — CI-003의 Verifier 측 대응.
- 문서 산출 Task의 AC에 "저장소 안에 증거가 없는 사실 주장을 포함하지 않는다"를 Planner가 넣도록 하는 것도 방법이지만, 이건 Task마다 사람이 챙겨야 하므로 위쪽이 낫다.

### Evidence

- `docs/PHASE-1.md:227-229`
- `.loop-local/runs/RUN-20260826T072745Z-TASK-008/worker-result.json` (dev server 언급 없음)
- `.loop-local/runs/RUN-20260826T072745Z-TASK-008/verification/` (PASS 판정)
- 대조: OBS-002 (TASK-001 외부 검증 불가 + Verifier PASS, 단 AC가 fallback 허용)

### Status

`RESOLVED (V0.1)` — CI-009. Verifier Context에 WITNESSED EXECUTION / NOT WITNESSED가 들어가고
`evidence_basis`가 필수가 되었다. `unwitnessed_claim`에 PASS를 주면 Runtime이 결정론적으로 거부한다.
V0.1 §3 참조.

---

# V0.1 Maintenance Pass — 무엇이 고쳐졌는가

**Date:** 2026-08-26 (Phase 1 종료 직후, Phase 2 시작 전)

이 절은 위 관찰들에 대해 **실제로 무엇을 바꿨는지**를 증거와 함께 적는다.
근거 없이 Runtime을 고치지 않는다는 원칙에 따라, 여기 있는 항목은 전부 위쪽
Observation에서 실측된 것들이다. 추측으로 추가한 기능은 없다.

Runtime 회귀 스위트: **121 tests / 121 pass / 0 fail** (변경 전 74 → 새 회귀 47건 추가).
제품 회귀: `npm test` 131 passed / 9 files, `npm run build` exit 0, `npm run lint` exit 0.
`loopctl doctor` exit 0.

## 1. Worker Evidence 쓰기 정책 일치 (OBS-002 후속 · CI-007)

**증상.** fingerprint는 `.loop/evidence` 를 Worker 쓰기 영역으로 예외 처리하는데
deny 규칙은 `.loop/**` 를 통째로 막았다. KERNEL이 지시한 Evidence 쓰기가 Phase 1
**8 Task / 9 Run 전부**에서 거부됐고, `.loop/evidence/TASK-001~008` 은 전량 비어 있었다.

**고친 방법.** 두 정책의 출처를 `worker/policy.mjs` 하나로 합쳤다.

- deny 규칙은 이제 `.loop/` 를 열거해서 만든다 — **이 Task의 Evidence 디렉터리만 빼고** 전부.
- fingerprint 예외도 같은 함수에서 나온다. `.loop/evidence` 전체가 아니라
  `.loop/evidence/<이 Task>` 하나다. 다른 Task의 Evidence는 이제 fingerprint 대상이라
  손대면 policy violation으로 잡힌다.
- 예방(deny 열거)과 탐지(fingerprint)의 경계가 같은 사실에서 유도된다. 열거가 놓치는
  경우(Run 도중 새로 생긴 경로)는 fingerprint가 잡으므로 열거의 완전성을 주장하지 않는다.

**회귀 테스트** — `test/policy.test.mjs` 6건:
자기 Evidence 쓰기가 violation이 아님 · 남의 Evidence 쓰기는 violation ·
`.loop/**` 통째 deny 규칙이 사라졌음 · KERNEL 수정은 여전히 violation ·
Runtime이 Evidence 디렉터리를 미리 만들어 둠.

## 2. Worker Self-check (OBS-007 · OBS-008 · CI-006)

**증상.** Worker가 `npm run build` 를 못 돌려 타입 한 줄 때문에 TASK-005 Attempt 1
전체($4.04 / 9분, 낭비율 52%)가 폐기됐다. Gate가 그 오류를 잡는 데 걸린 시간은 3.9초였다.

**고친 방법.** Bash 전체를 열지 않는다. Runtime 소유 진입점 하나만 연다.

- `loopctl self-check [<gate> ...]` — **project.yaml에 설정된 Gate 명령만** 실행한다.
  인자는 Gate 이름일 뿐 명령 문자열이 아니다. 해석되지 않는 이름은 아무것도 실행하지 않는다.
- 실행에는 기존 `gate/runner.mjs` 의 `executeGate` 를 그대로 쓴다 — 같은 명령을 두 가지
  방식으로 실행하는 경로를 만들지 않기 위해서다. 산출물은 `.loop-local/self-check/` 로 가고
  **Gate Report를 만들지 않으며 Run 디렉터리에도 Task 상태에도 쓰지 않는다.**
- Worker permission의 allow 목록은 **정확히 한 줄**이다:
  `Bash(node tools/loop-runtime/loopctl.mjs self-check:*)`.
  `Bash(npm ...)` 같은 규칙은 넣지 않는다 — 그러면 Gate 설정을 우회하는 두 번째 출처가 생긴다.
- **정본 Gate 실행은 그대로다.** Runtime은 Worker 종료 후 Gate를 독립적으로 다시 돌리고,
  완료 판정은 그쪽만이 근거다. self-check 출력에도 `advisory` 라고 명시한다.

**회귀 테스트** — `test/policy.test.mjs` 7건:
allow 목록이 정확히 한 줄인지 · 설정되지 않은 Gate 이름 거부 ·
`build; touch pwned` / `$(touch pwned)` 같은 인자가 **명령이 되지 못하고** 거부되는지 ·
비활성 Gate에 PASS를 지어내지 않는지 · self-check가 Gate Report/Run/Task 상태를
만들지 않는지 · Worker 이후 Gate가 여전히 독립적으로 도는지.

## 3. Verifier 증거 요구 (OBS-009 · CI-009)

**증상.** `docs/PHASE-1.md` 가 "manual use in a dev server" 를 서술했고 Verifier가 PASS했다.
그 실행은 일어난 적이 없다 — Worker는 `node -e` 도 못 돌렸고 사람도 dev server를 띄우지 않았다.

**고친 방법.** 두 겹이다. Runtime이 사실을 주고, 계약이 그것을 강제한다.

*(a) Runtime이 목격한 실행을 사실로 선언한다.* Verifier context의 `RUNTIME FACTS` 에
`WITNESSED EXECUTION`(이 Run에서 Runtime이 실제로 돌린 명령 목록)과
`NOT WITNESSED BY THE RUNTIME`(수동 조작 · 브라우저 · dev server · 네트워크 · 외부 서비스 ·
실물 렌더링)을 명시한다. 이것은 Runtime 소유 사실이며 Worker의 주장이 아니다.
Verifier 격리(Input 분리)는 그대로다 — Worker 요약·narrative·stdout은 여전히 들어가지 않는다.

*(b) 판정 계약에 근거를 필수로 넣었다.* `criteria[]` 의 각 항목은 `evidence_basis` 를 갖는다:

| 값 | 의미 |
|---|---|
| `gate` | Runtime이 직접 실행한 Gate 결과 |
| `runtime_artifact` | Runtime이 만든 Run 산출물 (`evidence_refs` 필수) |
| `canonical_diff` | Runtime이 만든 변경 매니페스트/패치 |
| `repository_content` | 저장소에 실제로 존재하는 파일 (`evidence_refs` 필수) |
| `unwitnessed_claim` | 이 AC는 Runtime이 목격하지 못한 실행을 요구한다 |

**"Worker가 그렇게 말했다"에 해당하는 값은 존재하지 않는다.** 서술은 근거가 아니다.

결정론적 강제(Runtime이 직접 확인, LLM 판단 아님):

- `unwitnessed_claim` + `PASS` → **거부**. 이것이 OBS-009를 막는 규칙이다.
- `runtime_artifact` / `repository_content` + `PASS` → `evidence_refs` 가 비었거나
  존재하지 않는 경로를 가리키면 거부.
- `gate` + `PASS` → 이 Run이 Gate를 하나도 실행하지 않았으면 거부.
- `canonical_diff` + `PASS` → canonical diff가 비었으면 거부.
- FAIL 판정에는 근거 존재를 요구하지 않는다 — 없다는 것이 곧 실패 사유다.

`.loop/skills/verifier.md` 계약에 규칙 5b·5c를 추가했다: 산출물이 그런 실행을 했다고
**서술**하더라도 Runtime Facts가 뒷받침하지 않으면 사실로 받아들이지 않는다.

**회귀 테스트** — `test/verifier-evidence.test.mjs` 12건.
5개 `unwitnessed_kind` 전부에 대해 PASS 거부 · 존재하지 않는 artifact 인용 거부 ·
근거 없는 gate/canonical_diff 주장 거부 · 지어낸 basis 값(`worker_narrative` 등) 거부 ·
FAIL로 표시된 unwitnessed는 정상 경로 · Verifier 격리 불변 확인.

## 4. 진행 중인 실행의 가시성 (OBS-006 · CI-008)

**증상.** 세션이 끊긴 뒤 `status` 는 `latest run: (none)`, `execution` 은
`no execution found` 를 반환했다. 실제로는 16분짜리 실행이 돌고 있었다.
게다가 wrapper bash가 좀비(`Z`)로 남아 PID 폴링이 8분간 "실행 중"으로 오판했다.

**고친 방법.** 이미 있던 `.loop-local/executions/active/<TASK>.json` 표식을 실제 상태로 만들었다.

- 표식에 `heartbeat_at` · `stage` · `run_id` · `attempt` 를 넣고 **매 단계마다 갱신**한다.
- 생존 판정의 정본은 **heartbeat**다. PID는 보조 신호로만 본다.
  `classifyActiveMarker()` 는 heartbeat가 `HEARTBEAT_STALE_MS`(5분)를 넘으면
  **PID가 살아 있어도** `STALE` 로 판정한다 — 좀비 오판을 구조적으로 막는다.
- `status` 에 `ACTIVE EXECUTION` 섹션이 생겼다. RUNNING/STALE, 실행 ID, 현재 단계,
  Run ID, 판정 근거("heartbeat 2s ago")를 보여준다. STALE이면 회수 방법도 알려준다.
- `claimExecution()` 도 같은 판정을 쓴다. heartbeat가 살아 있고 PID도 살아 있을 때만
  중복 실행을 거부하고, 그 외에는 표식을 회수한다.

**회귀 테스트** — `test/operability.test.mjs` 5건:
살아 있는 PID + 끊긴 heartbeat = STALE(좀비 시나리오) · status의 RUNNING 표시와 단계 ·
STALE 표시와 회수 안내 · 살아 있는 표식은 거부하고 좀비 표식은 회수 · 종료 시 표식 제거.

## 5. 수동 복구 조정 (OBS-004 · CI-002 관련)

**증상.** `execute` 가 멈춘 뒤 사람이 `gate` + `verify` 로 Task를 DONE으로 만들면,
Task는 DONE인데 "latest execution"은 영원히 NEEDS_HUMAN으로 남았다.

**진단.** 표시 문제가 아니라 **기록의 공백**이었다. 그 복구를 수행한 실행이 어디에도
기록되지 않았다. 그래서 표시를 손대는 대신 기록을 채웠다.

**고친 방법 (`loop/reconcile.mjs`).**

- 사람이 CLI로 REVIEW → DONE을 만들면, 그것도 Execution Report로 남긴다.
  `origin: 'manual'` · `stop_reason: 'MANUAL_RECOVERY'` · `manual_stages: ['gate','verify']`.
- **앞선 Report는 절대 고쳐 쓰지 않는다.** 그것은 그때 실제로 일어난 일의 기록이다.
  새 Report가 `supersedes: <이전 EXEC-ID>` 로 관계를 명시한다.
- 사용량은 그 Run의 정본 telemetry에서만 모은다. 없는 값을 지어내지 않는다.
- 오케스트레이터가 몰고 있는 중이면(활성 표식 존재) 기록하지 않는다 — 그쪽이 자기 Report를 쓴다.
- 부수적으로 `status` 는 Report의 `final_task_status` 와 현재 Task 상태가 다르면
  `[superseded — task is now X]` 를 덧붙인다. 이건 Runtime 사실이지 표시 보정이 아니다.

**resume을 만들지 않은 이유.** 사람이 이미 끝낸 일을 사실대로 적는 것으로 이 관찰은
해소된다. Worker/Gate/Verifier를 다시 돌리지 않고 LLM도 부르지 않는다.
CI-002(`loopctl resume <RUN>`)는 별개 문제로 남겨 둔다 — 근거가 더 필요하다.

**회귀 테스트** — `test/operability.test.mjs` 5건:
수동 복구가 자기 Execution으로 기록됨 · 멈춘 Report를 status가 더 이상 최신으로
보고하지 않음 · 앞선 Report 파일이 **바이트 단위로 그대로**인지 · 아직 유효한 Report는
superseded로 표시하지 않음 · 오케스트레이션 실행은 수동 기록을 만들지 않음.

## 6. Plan 단위 순차 실행 (`loopctl execute-plan`)

Phase 1에서 조작자가 손으로 반복한 절차(`ready` → `execute` → 결과 확인 → 다음 `ready`)를
Runtime이 결정론적으로 수행한다. **새 오케스트레이션 로직은 없다** — Task 하나의
Worker · Gate · Verifier · Diagnose · Retry는 전부 기존 `executeTask` 가 그대로 소유한다.
`loop/plan-executor.mjs` 가 하는 일은 "다음에 무엇을 실행할지" 고르는 것 하나뿐이다.

- **승인된 Plan에만** 동작한다. `approval.json` 의 `created_task_ids` 를 쓴다.
  승인하지 않고, Goal 단위·다중 Phase 자동 승인도 하지 않는다.
- READY는 Runtime 의존성 규칙(`readyTasks`)을 그대로 쓴다. 순서는 Plan의 Task 생성 순서.
- **한 번에 Task 하나.** shared working tree이므로 동시에 실행하지 않는다.
- DONE이면 READY를 다시 계산하고 이어간다. 전부 DONE이면 `PLAN_COMPLETE`.
- 사람이 필요한 정지에서 **즉시** 멈춘다:
  NEEDS_HUMAN · STALLED · LIMIT_REACHED · BLOCKED · FAILED · INTERRUPTED · PAUSE ·
  `PLAN_TASK_MISSING` · `PLAN_TASK_INVALID` · `PLAN_TASK_BLOCKED` · `PLAN_NO_READY_TASK`.
  READY가 없으면 왜 못 가는지(무엇을 기다리는지) 사실대로 적고 멈춘다.
- Plan 실행 보고서를 `.loop-local/plans/<PLAN>/executions/PLANEXEC-<stamp>.json` 에 남긴다.
- **재시작은 상태가 필요 없다.** 매 순회마다 Task 상태를 디스크에서 다시 읽으므로
  같은 명령을 다시 실행하면 남은 Task부터 이어간다.
- **오케스트레이션 판단에 LLM을 쓰지 않는다.** 보고서의 `orchestration_llm_calls: 0` 이
  그 사실을 기록하고, 회귀 테스트가 그 값을 검사한다.

**회귀 테스트** — `test/plan-execution.test.mjs` 12건:
미승인 Plan 거부 · execute-plan이 승인하지 않음 · 의존 순서대로 전부 DONE ·
추가 LLM 호출 0 · 실행이 겹치지 않음(앞 실행 종료 후 다음 시작) · 실패 시 즉시 정지와
뒤 Task 무손상 · PAUSE 정지 · 재실행 시 남은 Task만 · 완료된 Plan은 no-op ·
BLOCKED Task 정지 · 존재하지 않는 Task 참조 거부.

## 부수 발견 — 이번 작업 중 드러난 것

**yaml-lite가 double-quoted scalar의 `\"` 이스케이프를 처리하지 않는다.**
테스트 fixture의 Gate 명령 `"node -e \"process.exit(0)\""` 이 파서를 통과하면 백슬래시가
그대로 남아 `/bin/sh` 에서 문법 오류가 났다. Phase 1에서 드러나지 않은 이유는 그 Gate가
**한 번도 실행된 적이 없었기** 때문이다(Task들의 `stop_condition.gates` 가 비어 있었다).

- 영향: `project.yaml` 의 Gate 명령에 큰따옴표를 쓰면 실행 불가능한 명령이 된다.
- 위험도: 낮음. 결과는 Gate ERROR/FAIL이므로 **fail-closed**다. 거짓 PASS는 만들지 않는다.
- 당시 처리(2026-08-26, V0.1 유지보수 중): 파서를 고치지 않았다. 요청된 6개 항목 밖이고
  파서 변경은 `project.yaml` 전체에 영향을 준다. fixture의 Gate 명령을 `node --version` 으로
  바꿔 회귀를 살렸다. CI-010으로 기록만 남겼다.
- **후속(2026-08-26, CI-010 최소 수정): 고쳤다.** 아래 절 참조. fixture의 Gate 명령도
  관찰된 원문(`"node -e \"process.exit(0)\""`)으로 되돌려, 이제 Gate를 실행하는 모든
  테스트가 이 이스케이프를 실제로 통과시킨다.

---

# CI-010 Minimal Fix — `yaml-lite` 큰따옴표 이스케이프

**Date:** 2026-08-26 (V0.1 유지보수 직후, Phase 2 시작 전)

위 부수 발견에 대한 최소 수정이다. 관찰 기록은 그대로 두고, 여기에 무엇을 바꿨는지만 적는다.

## 근본 원인 — 한 군데가 아니라 두 군데

한 사실(“큰따옴표 안의 `\"` 는 값의 일부다”)을 두 코드가 서로 다르게 알고 있었다.

1. **인용 구간 스캐너** (`stripComment`) — 큰따옴표 구간 안에서 `\"` 의 `"` 를 구간의 끝으로
   봤다. 그래서 `"node -e \"process.exit(0)\""` 의 인용이 `\"` 에서 일찍 닫혔다.
2. **큰따옴표 스칼라 디코딩** (`parseScalar`) — 바깥 따옴표만 떼고 본문을 그대로 돌려줬다.
   이스케이프를 해석하지 않으므로 백슬래시가 값에 남았다.

두 번째만 고치면 스캐너가 여전히 구간을 잘못 끊고, 첫 번째만 고치면 백슬래시가 그대로 남는다.
그래서 **둘의 이스케이프 집합을 하나의 상수(`DQ_ESCAPES`)로 묶었다.**

## 구현

`tools/loop-runtime/yaml-lite.mjs` 한 파일, 세 군데.

```js
// 1. 스캐너 — 큰따옴표 구간에서만 백슬래시가 다음 한 글자를 이스케이프한다.
if (quote === '"' && c === '\\' && i + 1 < s.length) { i += 1; continue; }

// 2. 인정하는 이스케이프 — 이것뿐이다. 스캐너와 디코더가 같은 집합을 본다.
const DQ_ESCAPES = { '"': '"', '\\': '\\' };

// 3. 디코더 — 큰따옴표 스칼라에만 적용한다. 작은따옴표는 손대지 않는다.
if (s.startsWith('"') && s.endsWith('"') && s.length > 1) {
  return decodeDoubleQuoted(s.slice(1, -1), lineNo);
}
```

`\\` 를 함께 지원하는 것은 기능 확장이 아니라 **정합성 요구**다. 스캐너가 어떤 `"` 가
이스케이프됐는지 알려면 `\\` 를 인식해야 하고(`"a\\"` 는 정상적으로 닫혀야 한다),
그렇다면 디코더도 같은 것을 해석해야 한다.

의도적으로 하지 않은 것:

- 작은따옴표 스칼라에는 이스케이프 해석을 적용하지 않았다. YAML의 작은따옴표에는 백슬래시
  이스케이프가 없다. 기존 동작 그대로다.
- 평문 스칼라의 아포스트로피 동작(`it's fine`)은 건드리지 않았다.
- `#` 가 인용 구간 안에서 주석이 되지 않는 성질도 그대로다.
- 전체 YAML 이스케이프 의미론을 넣지 않았다. anchor/alias · flow map · multi-document ·
  block scalar 동작 · 들여쓰기 의미는 전부 그대로다.

## 남아 있는 미지원 이스케이프 — 의도적이다

`\"` 와 `\\` 외의 이스케이프(`\n` · `\t` · `\u0041` 등)는 **에러**다.
백슬래시를 조용히 남기지 않는다.

이건 이 파서의 원칙(조용히 잘못 읽는 대신 명시적으로 실패한다)을 따른 것이고,
CI-010 자체가 정확히 "백슬래시가 그대로 남아 실행 불가능한 명령이 되는" 문제였기 때문이다.
`\n` 을 실제 줄바꿈으로 원하면 block scalar(`|`)를 쓴다 — 이미 지원한다.

## Before / After

| | before | after |
|---|---|---|
| YAML 원문 | `command: "node -e \"process.exit(0)\""` | 같음 |
| 파싱 결과 | `node -e \"process.exit(0)\"` | `node -e "process.exit(0)"` |
| 셸 실행 | `/bin/sh: 1: Syntax error: "(" unexpected` | exit 0 |
| Gate 판정 | ERROR/FAIL (fail-closed) | PASS |

**fail-closed는 유지된다.** 고친 것은 파싱이지 판정이 아니다. 실제로 깨진 명령은 여전히
PASS가 되지 않으며, 회귀 테스트가 그것을 검사한다.

## 회귀 증거

`tools/loop-runtime/test/yaml-lite.test.mjs` — 18건, 전부 통과.

- **Case A** — 관찰된 명령 원문이 `node -e "process.exit(0)"` 로 디코딩되고 백슬래시가
  남지 않는다. 디코딩된 명령을 실제 셸로 실행해 exit 0과 빈 stderr를 확인한다.
- **Case B** — `\"` 가 인용 구간을 일찍 닫지 않는다. 이스케이프 뒤에 `#` · `:` · `,` 가
  있어도 잘리지 않으며, 인용 밖의 진짜 주석은 여전히 제거된다. `\\` 도 구간 추적을 깨지 않는다.
- **Case C** — 평범한 큰/작은따옴표 · 인용 안의 `#` · 평문 아포스트로피 · 미종료 인용 실패 ·
  flow map · anchor/alias · multi-document · tab 들여쓰기 · 미종료 flow sequence ·
  block scalar와 스칼라 타입이 전부 그대로다. 파서 에러를 약화시키지 않았다.
- **Case D** (YAML → Gate 경계, 버그가 발견된 자리) — fixture의 `project.yaml` 이
  관찰된 원문을 그대로 담고, `loopctl gates` 가 디코딩된 명령을 보여주며(`\"` 를 노출하지
  않는다), Runtime이 그 명령을 실행해 PASS한다. 그리고 진짜로 깨진 명령은 여전히 실패한다.

fixture의 기본 Gate 명령을 `node --version` 에서 관찰된 원문으로 되돌렸으므로,
Gate를 실행하는 **모든** Runtime 테스트가 이 경로를 지난다. 회귀가 생기면 조용히 넘어가지 않는다.

## 검증 결과

| | V0.1 기준선 | CI-010 이후 |
|---|---|---|
| Runtime 회귀 | 121 pass / 0 fail | **139 pass / 0 fail** |
| 제품 테스트 | 131 passed / 9 files | 131 passed / 9 files |
| `npm run build` | exit 0 | exit 0 |
| `npm run lint` | exit 0 (warning 1) | exit 0 (warning 1) |
| `loopctl doctor` | exit 0 | exit 0 |

기준선을 회귀시키지 않았다. LLM 기반 Runtime 작업은 이 유지보수에 한 번도 쓰지 않았다.

---


# OBS-013 Sync — `yaml-lite` block scalar 본문 재해석

**Date:** 2026-08-27  ·  **출처:** canonical field-test 프로젝트에서 관찰·수정·검증됨

이 항목은 Starter Pack이 **관찰한 것이 아니라 동기화한 것**이다. 관찰과 수정의 전체 기록은
canonical 프로젝트의 Field Notes에 있으며, 여기에는 동기화 사실만 남긴다.

**증상.** Plan validation은 PASS이고 repository subject도 일치하는데 `plan-approve`가
fail-closed로 거부됐다.

```text
TASK-...: serialized task does not parse back - unterminated quote
```

**원인.** 승인 로직이 아니라 파서였다. `scanLines()`가 block scalar **본문**까지
`stripComment()`에 넘겨서, `'node:'` 처럼 콜론 뒤에 따옴표가 오는 평범한 기술 산문이
값을 여는 인용으로 오인되어 닫히지 않는 구간을 만들었다. 본문은 `readBlockScalar()`가
raw 줄에서 따로 읽으므로 `scanLines`가 만든 content는 **애초에 쓰이지도 않았다** —
해석할 이유가 없는 줄을 해석하다 실패한 것이다.

**동기화한 것.** canonical의 최소 수정을 그대로 가져왔다(byte-identical).

```text
tools/loop-runtime/yaml-lite.mjs            scanLines() 한 함수
tools/loop-runtime/test/yaml-lite.test.mjs  회귀 10건 추가
```

block scalar **바깥**의 인용 검사·주석 처리·미지원 문법 거부는 하나도 느슨해지지 않았고,
CI-010의 `\"` 이스케이프 동작도 그대로다. 회귀가 그 성질을 고정한다.

| | CI-010 이후 | OBS-013 동기화 이후 |
|---|---|---|
| Runtime 회귀 | 139 pass / 0 fail | **149 pass / 0 fail** |
| `loopctl doctor` | exit 0 | exit 0 |
| `loopctl validate` | exit 0 | exit 0 |

CI 번호는 부여하지 않았다 — canonical 쪽에서도 아직 부여되지 않았고, 이번 작업은
새 개선 후보를 만드는 것이 아니라 검증된 수정을 옮긴 것이다.

---


## OBS-014 — Starter Pack을 새 프로젝트로 복사할 때 `.loop/` 전체가 누락됐고, Runtime은 그 상태에서 시작조차 할 수 없었다

**Date:** 2026-09-01

**Project phase / Goal:** Molt Note — Bootstrap 이전, 최초 저장소 조사

**Plan / Task / Run / Execution:** 해당 없음 (Runtime 실행 전)

**Runtime stage:** CLI / Bootstrap

### What happened

새 프로젝트(`molt-note`)는 Starter Pack에서 복사되었으나, 저장소에 **점(.)으로 시작하는
항목 세 개가 통째로 없었다.**

```text
.loop/          (DESIGN.md · KERNEL.md · project.yaml · policies/ · skills/ · tasks/ · evidence/)
.loop-local/    (runs · leases · staging · plans)
.gitignore
```

보이는 파일(`README.md` · `START-HERE.md` · `CLAUDE.local.md` · `prompts/` · `loop-prompts/` ·
`docs/` · `tools/` · `loopctl` · `loopctl.cmd`)은 upstream과 **바이트 단위로 동일**했다.
숨김 항목만 사라진 형태이므로, `git clone`이 아니라 Finder 또는 `cp`로 보이는 항목만
복사했을 때 나타나는 패턴으로 보인다. (관찰된 사실: 누락. 복사 방법은 **추정**이다.)

또한 `git ls-files`가 0을 반환했다 — `Initial commit`이 존재하지만 추적 중인 파일이 하나도 없다.

이 상태에서:

```text
$ ./loopctl doctor
(eval):1: permission denied: ./loopctl        # exit 126
```

`loopctl`에 실행 권한이 없었다. `chmod +x` 후 다시 실행하면:

```text
$ ./loopctl doctor
MISS .loop/DESIGN.md
MISS .loop/KERNEL.md
MISS .loop/project.yaml
MISS .loop/skills/impl.md
MISS .loop/skills/verifier.md
MISS .loop/policies/limits.yaml
MISS .loop/skills/planner.md
MISS .loop/tasks
MISS .loop/evidence
MISS .loop-local/runs
MISS .loop-local/leases
MISS .loop-local/staging
MISS .loop-local/plans
error: ENOENT: no such file or directory, open '.../.loop/KERNEL.md'
  (set LOOPCTL_DEBUG=1 for the full stack trace)          # exit 1

$ ./loopctl status
error: ENOENT: no such file or directory, open '.../.loop/project.yaml'
  (set LOOPCTL_DEBUG=1 for the full stack trace)          # exit 0   ← 주목
```

`loopctl`에는 `init` 명령이 없다. 즉 **Runtime이 자기 자신을 복구할 경로가 없었다.**

### Expected

세 가지가 각각 어긋났다.

1. **`doctor`가 MISS를 13개 나열해 놓고도 예외로 죽었다.** 이미 무엇이 없는지 정확히
   알고 있으므로, "control plane이 없다 — Starter Pack에서 `.loop/`를 복원하라"처럼
   원인과 다음 행동을 말하고 깨끗하게 끝나는 편이 자연스럽다. 사용자가 보는 마지막 줄이
   `ENOENT ... KERNEL.md` 스택 트레이스일 필요는 없다.

2. **`status`가 에러를 출력하면서 exit 0을 반환했다.** 이건 실패를 성공으로 보고하는
   형태다. Runtime의 원칙("실패를 성공으로 기록하지 않는다")과 정면으로 어긋나며,
   스크립트나 CI가 `status`의 exit code를 신뢰할 수 없게 만든다.
   **이것이 이 관찰에서 가장 문제인 부분이다.**

3. **`loopctl`에 실행 권한이 없었다.** upstream 저장소의 파일 모드 자체가 `100644`이며
   (scratchpad에 clone해서 확인), `README.md`와 `START-HERE.md`는 일관되게
   `./loopctl doctor`를 첫 명령으로 안내한다. 새 사용자의 첫 명령이 exit 126으로 끝난다.

### Current workaround

- `chmod +x loopctl`
- upstream(`github.com/JJleem/molt-loop`)을 scratchpad에 clone하고 `.loop/` · `.loop-local/` ·
  `.gitignore`를 복사해 복원. 보이는 파일이 upstream과 동일함을 `diff -rq`로 먼저 확인해,
  복원 대상이 같은 세대의 Starter Pack임을 근거로 삼았다.
- 복원 후 `doctor` exit 0, Runtime 회귀 149 pass / 0 fail.

**`.loop/` 내용을 추측으로 재작성하지 않았다.** KERNEL.md · DESIGN.md · skills/ 는 Runtime의
계약 문서이며, 지어내면 Runtime이 강제하는 규칙 자체가 조용히 달라진다.

### Impact

**High.** 새 프로젝트가 아예 시작되지 않는다. 그리고 복구 경로가 저장소 안에 없다 —
upstream에 네트워크로 접근할 수 있어야만 복구된다. 오프라인이거나 upstream이 사라진
상황이라면, `.loop/`를 지어내는 것 외에 방법이 없고 그것은 Runtime 계약의 위조다.

`status`의 exit 0은 별개로 심각하다. 자동화가 Runtime 상태를 오판할 수 있다.

### Possible Runtime improvement

- **`status`가 설정을 읽지 못하면 non-zero로 끝나야 한다.** (가장 명확한 수정)
- `doctor`가 control plane 부재를 **예외가 아니라 진단 결과로** 보고하고,
  복원 방법을 한 줄 안내한 뒤 non-zero로 끝난다.
- `loopctl init` — 없거나 손상된 control plane을 Runtime에 내장된 정본에서 복원한다.
  이것이 있으면 upstream 네트워크 접근 없이 복구된다.
- 저장소의 `loopctl` 파일 모드를 `100755`로 커밋한다.
- `START-HERE.md`에 "복사가 아니라 clone" 또는 "복사했다면 `.loop/` 존재를 먼저 확인"을 명시.

넷 다 별개의 작은 수정이며, 첫 번째와 네 번째는 특히 비용이 낮다.

### Evidence

```text
./loopctl doctor              # 복원 전 exit 1 (13 MISS + ENOENT)
./loopctl status              # 복원 전 에러 출력 + exit 0
diff -rq . <upstream> --exclude=.git --exclude=.loop --exclude=.loop-local \
          --exclude=.gitignore --exclude=.DS_Store   # 차이 없음
ls -l <upstream>/loopctl      # -rw-r--r--   (upstream도 실행 권한 없음)
git ls-files | wc -l          # 0
./loopctl doctor              # 복원 후 exit 0
node --test "tools/loop-runtime/test/*.test.mjs"    # 149 pass / 0 fail
```

Worker / Verifier / Planner 호출 없음. 비용 0.

### Status

`OBSERVED`

---


## OBS-015 — Worker launch 실패가 이전 attempt의 TIMEOUT 분류를 물려받아 retry budget을 소비했다

**Date:** 2026-09-01 ~ 2026-09-02

**Project phase / Goal:** Molt Note Phase 1 — Application Foundation

**Plan / Task / Run / Execution:**
- PLAN-20260901T035729Z
- TASK-002 (로컬 영속 저장소 초기화와 migration 경로)
- Attempt 1: RUN-20260901T041022Z-TASK-002
- Attempt 2: RUN-20260901T043410Z-TASK-002
- EXEC-20260901T041022Z-TASK-002 / PLANEXEC-20260901T045106Z

**Runtime stage:** Diagnose (Worker launch)

### What happened

`execute-plan` 실행 중 두 attempt가 **서로 다른 이유로** 실패했으나, 실행 로그에는
같은 분류가 두 번 찍혔다.

```text
Attempt 1
  [Worker] failed: worker timed out after 900s; worker result file not found: ...
  [Diagnose] TIMEOUT -> RETRY_WITH_HINT

Attempt 2
  [Worker] failed: Worker could not be launched: worker adapter "claude" is not
           available: `claude --version` exited null
  [Diagnose] TIMEOUT -> RETRY_WITH_HINT      ← Worker가 실행조차 되지 않았다
```

Attempt 2는 worker timeout이 아니라 **adapter launch 실패**다. 프로세스가 시작되지 않았다.

artifact를 보면 메커니즘이 드러난다. Attempt 2의 Run 디렉터리에는 입력만 있고
결과가 없다.

```text
.loop-local/runs/RUN-20260901T043410Z-TASK-002/
  context.md
  manifest.json          ← 이게 전부다. envelope도 recovery도 없다
```

그리고 `diagnose`는 대상 Run을 이렇게 고른다.

```text
$ ./loopctl diagnose TASK-002
Resolved Run: RUN-20260901T041022Z-TASK-002  (latest completed worker run)
```

즉 **launch 실패는 "completed worker run"을 만들지 않으므로 진단에서 보이지 않는다.**
Runtime은 직전 attempt(=attempt 1)의 분류를 다시 해석해 그것을 이번 attempt의
분류인 것처럼 보고했다. 그 결과:

- attempt 카운트가 2/3으로 올라갔고
- `escalation.hint_retry_max` (1/1)가 소진됐으며
- Failure Memo에는 *"Spend less time exploring… write the result file early rather
  than at the very end"* 라는 hint가 남았다 — **Worker가 한 번도 실행되지 않은 실패에
  대해 Worker의 탐색 습관을 교정하라는 조언이다.**

근본 원인은 provider 일시 장애로 보인다. 같은 시간대에 이 저장소를 운영하던 대화형
세션에서도 `claude-sonnet-5[1m] is temporarily unavailable (timed out)` 오류가 관측됐다.
장애가 해소된 뒤에는 정상이다.

```text
$ claude --version   → 2.1.252 (Claude Code)   exit 0
$ ./loopctl adapters → claude   available
```

### Expected

Worker가 **실행되지 않은 것**과 Worker가 **실행됐으나 실패한 것**은 다른 사건이다.

launch 실패는 infrastructure 가용성 문제이며 Worker의 행동을 교정해서 해결되지 않는다.
따라서:

- 별도의 실패 종류(예: `ADAPTER_UNAVAILABLE`)로 분류되는 편이 자연스럽고,
- 그 경우 Worker 행동 교정 hint를 만들지 않는 편이 낫고,
- **retry budget(특히 hint_retry_max)을 소비하지 않는 편이 합리적이다.**
  hint retry는 "직전 실패에서 배운 것을 주입한 재시도"인데, 배울 것이 없는 실패다.

최소한 이전 attempt의 분류를 이번 attempt의 분류로 재사용하지는 않아야 한다.

### Current workaround

없음. attempt 2가 hint retry를 소진했고, 그 결과 TASK-002가 사람에게 올라왔다.
provider가 회복된 뒤 `execute-plan`을 다시 실행했으나 TASK-002는 여전히 dispatch되지
않았다 (OBS-017 참조).

### Impact

**Medium~High.** 일시적인 provider 장애 하나가 Task의 재시도 예산을 조용히 태우고,
사실과 다른 교훈을 Failure Memo에 남긴다. 장애가 attempt 경계에 걸치면 실제 작업
품질과 무관하게 Task가 정지한다.

이번 사례에서는 **Attempt 1이 사실상 작업을 완료한 상태**(OBS-016)였기 때문에
손실이 더 두드러졌다.

### Possible Runtime improvement

- launch 실패를 `ADAPTER_UNAVAILABLE`처럼 별도로 분류한다.
- 그 분류에서는 hint를 생성하지 않고 hint_retry_max를 소비하지 않는다.
- 진단 대상 Run 해석 시, "latest completed worker run"이 **이번 attempt가 아닐 때**
  그 사실을 출력에 드러낸다 (현재는 조용히 이전 Run의 분류를 보여 준다).
- 실행 직전 adapter 가용성을 확인하고, 불가하면 attempt를 소비하지 않고 정지한다.

### Evidence

```text
.loop-local/runs/RUN-20260901T043410Z-TASK-002/     # context.md · manifest.json 뿐
.loop-local/runs/RUN-20260901T041022Z-TASK-002/recovery/history/1/diagnosis.json
    { "action": "RETRY_WITH_HINT", "reason": "Worker exceeded the 900s worker timeout…",
      "attempt": 1 }
./loopctl diagnose TASK-002   # "Resolved Run: … (latest completed worker run)"
```

Worker 호출 2회 · Verifier 0회 · provider-reported cost (known) $2.3220.

### Status

`OBSERVED`

---

## OBS-016 — 900s worker timeout이 1428s에 SIGKILL됐고, Worker는 마지막 15분간 아무 출력도 내지 않았다

**Date:** 2026-09-01

**Project phase / Goal:** Molt Note Phase 1 — Application Foundation

**Plan / Task / Run / Execution:**
- PLAN-20260901T035729Z · TASK-002
- RUN-20260901T041022Z-TASK-002 (attempt 1)

**Runtime stage:** Worker

### What happened

설정된 worker timeout은 900초인데 실제로는 약 1428초가 지난 뒤 SIGKILL됐다.

```json
"process": {
  "exit_code": null, "signal": "SIGKILL",
  "timed_out": true, "timeout_seconds": 900
},
"started_at":  "2026-09-01T04:10:22.987Z",
"finished_at": "2026-09-01T04:34:10.783Z",
"duration_ms": 1427796
```

**약 528초(8분 48초)의 초과분이 있다.** timeout enforcement 자체가 관찰 대상이다.

또 `stdout.log`와 `stderr.log`가 **둘 다 0바이트**다. 23.8분 동안 Worker가 남긴
표준 출력이 하나도 없다.

그러나 파일 시스템 증거는 Worker가 **실제로는 잘 작동했다가 도중에 멈췄음**을 보여 준다.
(시각은 로컬, UTC+9)

```text
13:10:22  Worker 시작
13:12:46  src-tauri/Cargo.toml            rusqlite = { version = "0.40", features = ["bundled"] }
13:13:14  src-tauri/src/db/migrations.rs
13:14:16  src-tauri/src/db/mod.rs
13:17:57  docs/ADR-0001-local-persistence.md
13:18:11  .loop/evidence/TASK-002/gate-lint.log     ← Worker가 자기 self-check를 돌린 흔적
13:18:27  .loop/evidence/TASK-002/gate-test.log
13:18:51  .loop/evidence/TASK-002/changed-files.log ← 마지막 의미 있는 활동
   …      (15분 19초 동안 아무 일도 일어나지 않음)
13:34:10  SIGKILL
```

Worker는 **8분 29초 만에 Task를 사실상 끝내고** evidence까지 남긴 뒤,
`worker-result.json` 하나를 쓰지 못한 채 15분 19초를 멈춰 있다가 죽었다.

이후 대화형 세션에서 저장소 상태를 독립적으로 확인한 결과:

```text
$ ./loopctl self-check          build PASS · lint PASS · test PASS
$ cargo test --manifest-path src-tauri/Cargo.toml
                                14 passed; 0 failed
```

AC3가 요구한 세 가지(빈 디렉터리 초기화 · 닫았다 다시 열기 · migration 재실행)가
모두 실제 테스트로 존재하며 통과한다. AC4가 요구한 ADR도 작성되어 있고,
§14.7이 UNVERIFIED로 남겨 둔 `rusqlite`의 `bundled` feature를 로컬 빌드 산출물로
확인하면서 **공식 문서 확인은 UNVERIFIED로 정직하게 구분**해 두었다.

### Expected

두 가지가 어긋났다.

1. **timeout이 설정값 근처에서 집행되는 것.** 900초 설정에 1428초는 예산 계획을
   무너뜨린다. 사람이 `worker_timeout_seconds`로 통제할 수 있다고 믿는 값이
   실제 상한이 아니다.
2. 15분간 무출력 상태가 **hang으로 감지되는 것.** `limits.yaml`의 `stall` 절은
   `identical_tool_calls` · `unchanged_error_string` · `zero_diff_attempts`를 정의하지만
   "일정 시간 이상 아무 출력도 없음"은 정의하지 않는다. V0에서 stall은 감지만 하고
   자동 대응은 하지 않기로 되어 있으므로, 이 항목이 없다는 것 자체가 관찰 결과다.

### Current workaround

없음. 결과적으로 완료된 작업이 결과 파일 부재만으로 미완료 처리됐다.

**주의: 이것을 근거로 "900초가 짧다"고 결론짓지 않는다.** 증거는 반대를 가리킨다 —
Worker는 8분 29초 만에 작업을 마쳤다. 예산을 소진한 것은 작업량이 아니라 무응답 구간이다.
cold `rusqlite` bundled 빌드(SQLite C 컴파일)가 self-check 안에서 돌긴 했지만,
그 구간은 13:18:11~13:18:51에 이미 끝나 있었다.

### Impact

**Medium.** 완료된 작업이 버려졌고, 재시도 예산이 OBS-015와 겹쳐 소진됐다.
timeout 초과 집행은 비용 상한 추정을 어렵게 만든다.

### Possible Runtime improvement

- timeout 집행 경로를 점검한다. 설정값과 실제 kill 시점의 차이를 Envelope에
  드러내는 것만으로도 진단이 쉬워진다.
- Worker가 결과 파일을 **일찍, 점진적으로** 쓰도록 계약을 조정하는 방향
  (KERNEL §7의 Result를 마지막에 한 번 쓰는 현재 구조가 이 실패 양식에 취약하다).
- `stall`에 "무출력 지속 시간" 항목을 추가할지 검토한다 — 다만 실측 사례는 아직 이것 하나다.

### Evidence

```text
.loop-local/runs/RUN-20260901T041022Z-TASK-002/runtime-envelope.json
.loop-local/runs/RUN-20260901T041022Z-TASK-002/stdout.log   # 0 bytes
.loop-local/runs/RUN-20260901T041022Z-TASK-002/stderr.log   # 0 bytes
.loop/evidence/TASK-002/                                    # gate-lint.log · gate-test.log · …
docs/ADR-0001-local-persistence.md
```

### Status

`OBSERVED`

---

## OBS-017 — Runtime이 안내한 resume가 독립 Task를 진행시키면서, 막혀 있던 Task의 복구 경로를 없앴다

**Date:** 2026-09-02

**Project phase / Goal:** Molt Note Phase 1 — Application Foundation

**Plan / Task / Run / Execution:**
- PLAN-20260901T035729Z · TASK-002
- PLANEXEC-20260901T045106Z (1차) → PLANEXEC-20260902T032717Z (resume)

**Runtime stage:** Execute loop / Diagnose

### What happened

1차 실행이 TASK-002에서 멈추면서 Runtime이 다음 행동을 직접 안내했다.

```text
Inspect:
  loopctl status
  loopctl diagnose TASK-002
  re-running `loopctl execute-plan PLAN-20260901T035729Z` resumes from the remaining tasks
```

그 시점 TASK-002의 진단은 이랬다.

```text
Failure: TIMEOUT
Retryable: yes
Recommended action: RETRY_WITH_HINT
Subject bound: unchanged since the failure
```

안내대로 아무것도 바꾸지 않고 `execute-plan`을 다시 실행했다. Runtime은 TASK-002를
건너뛰고 의존성이 없는 TASK-007 · TASK-009를 진행해 **둘 다 DONE으로 만들었다.**
그 과정에서 working tree가 바뀌었다.

그 직후 TASK-002의 진단이 달라졌다.

```text
Failure: RECOVERY_AMBIGUOUS
Retryable: no
Recommended action: NEEDS_HUMAN
Reason: … The working tree has changed since this attempt, so a retry would be
        layered onto unrelated changes.
Subject bound: CHANGED or unknown
```

Plan 실행은 `NEEDS_HUMAN / PLAN_NO_READY_TASK`로 끝났다.

**즉 Runtime이 스스로 권한 resume 동작이, 막혀 있던 Task를 `retryable: yes`에서
`retryable: no`로 밀어냈다.**

### Expected

fail-closed 자체는 옳다. 다른 Task가 바꿔 놓은 tree 위에 예전 attempt의 재시도를
얹는 것은 실제로 위험하며, Runtime이 이를 거부하는 것은 설계 원칙(Subject 바인딩)에 맞다.

문제는 **안내와 결과의 불일치**다. `re-running … resumes from the remaining tasks`는
남은 Task를 이어서 한다는 뜻으로 읽히지, *막힌 Task의 복구 가능성이 그 대가로
사라진다*는 뜻으로 읽히지 않는다.

resume 전에 다음을 알 수 있었으면 좋았다.

```text
TASK-002는 이번 resume에서 dispatch되지 않는다.
이 resume는 working tree를 바꾸므로, 이후 TASK-002의 retry는 subject 불일치로 거부된다.
```

### Current workaround

없음. 현재 TASK-002는 `IN_PROGRESS` · `NEEDS_HUMAN`이며 Plan에 READY Task가 없다.
Phase 1의 나머지 5개 Task(TASK-003~006, 008)가 전부 TASK-002에 막혀 있다.

`loopctl transition TASK-002 TODO`로 상태를 되돌리는 것은 escalation 우회에 해당하므로
운영자 판단 없이 하지 않는다.

### Impact

**High.** Runtime이 권한 정상 경로를 그대로 따랐는데 상황이 나빠졌다.
Plan 전체가 사람 개입 없이는 더 진행되지 않는다.

### Possible Runtime improvement

- resume 시작 전에 "이번 실행에서 dispatch되지 않는 Task"와 "tree 변경으로 복구
  경로를 잃게 될 Task"를 미리 보여 준다.
- 정지 메시지의 resume 안내에 이 부작용을 한 줄 덧붙인다.
- 막힌 Task가 있는 Plan에서 독립 Task만 진행하는 것을 별도 의사결정으로 만든다
  (예: 확인 플래그, 또는 per-Task worktree 격리 — 후자는 이미 "아직 없는 것"에 있다).

### Evidence

```text
PLANEXEC-20260901T045106Z.json   # LIMIT_REACHED / TASK_STOPPED
PLANEXEC-20260902T032717Z.json   # NEEDS_HUMAN / PLAN_NO_READY_TASK
.loop-local/runs/RUN-20260901T041022Z-TASK-002/recovery/history/1/diagnosis.json
    { "action": "RETRY_WITH_HINT", … }        # resume 이전
.loop-local/runs/RUN-20260901T041022Z-TASK-002/recovery/diagnosis.json
    { "action": "NEEDS_HUMAN", "reason": "… The working tree has changed …" }  # resume 이후
```

resume 실행 비용: provider-reported $6.7420 (Worker 2 · Verifier 2 · gate runs 2).

### Status

`OBSERVED`

---


## OBS-018 — 정상 구현의 Gate가 환경 stall로 timeout됐고, 같은 subject에 대한 `gate --rerun`이 1.4초에 통과했다

**Date:** 2026-09-02

**Project phase / Goal:** Molt Note Phase 1 — Application Foundation

**Plan / Task / Run / Execution:**
- PLAN-20260901T035729Z · TASK-005 (Settings 영속화)
- RUN-20260902T040119Z-TASK-005
- EXEC-20260902T040119Z-TASK-005 → EXEC-20260902T050826Z-TASK-005 (수동 복구)

**Runtime stage:** Gate

### What happened

Worker는 성공했는데 Gate에서 멈췄다. 두 Gate 중 하나만 timeout이다.

```text
[Worker] success -> REVIEW
[Gate] FAIL  (lint TIMEOUT, test PASS)
[Diagnose] TIMEOUT -> NEEDS_HUMAN
```

`test` Gate는 통과했다. `test`도 `lint`도 둘 다 Rust를 컴파일하므로,
**컴파일이 느려서 생긴 문제라면 test가 먼저 걸렸어야 한다.**

정지 후 같은 명령을 사람이 직접 재보았다.

```text
1회차   npm run lint    4:14.79 wall  ·  0.94s user  ·  0.34s system  ·  0% cpu
          ├ lint:web (eslint)   0.568s
          └ lint:rust (clippy)  0.237s   "Finished dev profile in 0.79s"
2회차   npm run lint    0.852s   exit 0
3회차   npm run lint    0.806s   exit 0
        npm run test    1.164s   exit 0
```

**254초 동안 CPU를 거의 쓰지 않았다.** 계산이 오래 걸린 것이 아니라 무언가를 기다렸다.
child command를 따로 재면 각각 1초 미만이고, composite만 wall time이 길었다.
(원인은 특정하지 못했다. 새로 쓰인 실행 파일에 대한 macOS 보안 검사나 파일시스템 정체가
후보이나 **이 Run에서 확인하지 못했다 — UNVERIFIED.**)

Runtime의 진단 문구는 이 상황을 정확히 표현하고 있었다.

```text
Reason: Gate "lint" exceeded its timeout. A gate timeout is ambiguous between a
        hanging implementation and a slow environment; it is not retried automatically.
```

사람이 "환경 쪽"이라고 판단한 뒤 같은 subject에 대해 공식 rerun을 실행했다.

```text
$ ./loopctl gate TASK-005 --rerun
[PASS]    lint  1.4s
[PASS]    test  1.0s
Preserved previous gate evidence: gate-history/1/
Gate Result: PASS
Task remains REVIEW.  Ready for independent verification.
```

**900초를 초과하던 Gate가 같은 코드·같은 subject에서 1.4초에 통과했다.**
이후 Verifier도 PASS했고 TASK-005는 DONE이 됐다.

### 대조 증거 — retry 경로는 provider가 정상이면 실제로 수렴한다

같은 실행에서 TASK-006이 Worker timeout을 겪었으나 자동 복구됐다.

```text
Attempt 1  [Worker] failed: worker timed out after 900s
           [Diagnose] TIMEOUT -> RETRY_WITH_HINT
Attempt 2  [Worker] success -> REVIEW
           [Gate] PASS  (build, lint, test)
           [Verifier] PASS
TASK-006: DONE   attempts=2  19m 32s
```

이것은 OBS-015의 해석을 뒷받침한다. **900초 timeout 자체가 이 Task class에 부족한 것이
아니라**, TASK-002에서는 provider 장애가 두 번째 시도를 삼켰던 것이다.
`worker_timeout_seconds`는 이번 Phase 내내 900으로 유지했고 Phase는 완료됐다.

### Expected

Runtime 동작 자체는 **옳다.** gate timeout을 자동 재시도하면 진짜로 hang하는 구현을
무한히 되돌릴 수 있다. fail-closed가 맞다.

관찰 가치가 있는 것은 **환경 stall이 완성된 작업을 사람 개입 지점까지 밀어 올린다**는 사실과,
그것을 판별할 근거가 Gate 기록만으로는 부족하다는 점이다. 현재 gate-report에는
exit code와 소요 시간은 있으나 **CPU 시간이 없다.** wall time과 CPU time을 함께 기록하면
"느린 환경"과 "hang하는 구현"을 사람이 훨씬 빨리 가른다 — 이번 판별의 결정적 근거가
정확히 그 두 값의 차이였다.

### Current workaround

`./loopctl gate <TASK> --rerun`. subject가 바뀌지 않았다면 깨끗하게 복구된다.
이전 Gate 기록은 `gate-history/1/`에 보존되므로 실패 사실이 지워지지 않는다.

### Impact

**Medium.** 작업은 정상이었고 복구도 저렴했다(AI 호출 0회, 2.5초). 다만 사람이 개입해야
했고, 판별 근거를 사람이 직접 `time`으로 재서 만들어야 했다.

### Possible Runtime improvement

- gate-report에 **CPU time(user/system)** 을 wall time과 함께 기록한다.
  이번 사례에서 판별을 가능하게 한 유일한 신호다.
- gate timeout 정지 메시지에 `gate --rerun`을 복구 후보로 함께 안내한다
  (현재는 `status` · `diagnose`만 안내한다).

### Evidence

```text
.loop-local/runs/RUN-20260902T040119Z-TASK-005/gate-report.json
.loop-local/runs/RUN-20260902T040119Z-TASK-005/gate-history/1/     # 실패한 Gate 기록 보존
./loopctl diagnose TASK-005     # "ambiguous between a hanging implementation and a slow environment"
```

Gate rerun 비용 0 (AI 호출 없음). 이후 Verifier 1회 $2.2660.

### Status

`OBSERVED`

---

## OBS-019 — `execute-plan`이 REVIEW 상태의 Task를 이어받지 못해, 수동 Gate 복구 후 Plan이 스스로 진행하지 못했다

**Date:** 2026-09-02

**Project phase / Goal:** Molt Note Phase 1 — Application Foundation

**Plan / Task / Run / Execution:**
- PLAN-20260901T035729Z · TASK-005
- PLANEXEC-20260902T050658Z

**Runtime stage:** Execute loop

### What happened

OBS-018의 `gate --rerun` 이후 TASK-005의 상태는 이랬다.

```text
$ ./loopctl verify-ready
TASK-005     RUN-20260902T040119Z-TASK-005     REVIEW   GATES PASS

$ ./loopctl status
REVIEW
  TASK-005
    gates: PASS
    verifier: ready
```

Gate는 통과했고 Verifier만 남은, 명백히 진행 가능한 상태다.
그래서 Plan 단위 루프에 맡기려고 `execute-plan`을 실행했다.

```text
$ ./loopctl execute-plan PLAN-20260901T035729Z
Plan Result: NEEDS_HUMAN   stop_reason: PLAN_NO_READY_TASK
  no task is ready; outstanding: TASK-005 (REVIEW); TASK-006 (TODO) waiting on: TASK-005; …
Duration: 0s
Tasks executed this run: (none)
LLM invocations: 0
```

`execute-plan`은 **READY(=TODO이고 의존성이 충족된) Task만 dispatch한다.**
`REVIEW`에서 Verifier를 기다리는 Task는 그 조건에 들지 않으므로, Plan 루프에게는
"할 수 있는 일이 없는" 상태로 보인다. 정작 `verify-ready`는 바로 그 Task를 지목하고 있다.

복구는 사람이 Verifier를 직접 호출해야 했다.

```text
$ ./loopctl verify TASK-005
Verifier Result: PASS
TASK-005: REVIEW -> DONE
Recorded this manual recovery as EXEC-20260902T050826Z-TASK-005
  it supersedes EXEC-20260902T040119Z-TASK-005, which stays exactly as it was recorded
```

그 뒤에야 TASK-006이 READY가 되고 `execute-plan`이 정상적으로 이어졌다.

### Expected

`execute-plan`은 "Plan의 Task를 끝까지 진행시키는" 명령으로 읽힌다. 그런데 Plan 안에
**Verifier만 남은 Task**가 있을 때 그것을 이어받지 못하고 `PLAN_NO_READY_TASK`로 끝난다.

`verify-ready`가 그 Task를 명시적으로 나열하고 있으므로 Runtime은 무엇이 남았는지 이미 안다.
정지 메시지가 `no task is ready`라고만 말하는 대신 **"TASK-005는 verify 대기 중이다 —
`loopctl verify TASK-005`"** 를 안내하면 사람이 다음 행동을 추측하지 않아도 된다.

Task 단위 `execute`는 Worker → Gate → Verifier를 잇는데, Plan 단위 진입점은
그 중간 지점에서 시작할 수 없다는 비대칭이 있다.

### Current workaround

`./loopctl verify <TASK>`를 직접 호출한 뒤 `execute-plan`을 다시 실행한다.
비용은 들지 않았다(0초 · AI 호출 0회). 다만 실패한 `execute-plan` 실행도
Plan execution 기록으로 남는다.

### Impact

**Low~Medium.** 비용은 없고 복구도 한 줄이다. 그러나 수동 Gate 복구(OBS-018)와
반드시 짝을 이루어 발생한다 — `gate --rerun`은 Task를 항상 `REVIEW`에 남기므로,
그 경로를 쓴 사람은 **반드시** 이 막다른 길을 만난다.

### Possible Runtime improvement

- `execute-plan`이 `REVIEW` + gates PASS 상태의 Task를 Verifier 단계부터 이어받는다.
- 또는 `PLAN_NO_READY_TASK` 정지 메시지가 verify 대기 Task와 그 명령을 함께 안내한다.
- `gate --rerun` 출력의 "Ready for independent verification."에 실제 명령을 덧붙인다.

### Evidence

```text
PLANEXEC-20260902T050658Z.json        # NEEDS_HUMAN / PLAN_NO_READY_TASK · 0s · 0 LLM
./loopctl verify-ready                # 같은 시점에 TASK-005를 지목하고 있었다
```

### 부수 확인 — 수동 복구 기록은 정확했다

이번 Phase에서 수동 개입이 두 번 있었고(`transition TASK-002 TODO`, `gate TASK-005 --rerun`),
두 경우 모두 Runtime이 이전 기록을 지우지 않고 관계를 남겼다.

```text
TASK-002   status에 "[superseded — task is now TODO]" 표시
TASK-005   "it supersedes EXEC-…040119Z, which stays exactly as it was recorded"
```

OBS-004가 지적했던 "수동 복구 후 실행 보고서가 낡은 채로 남는다"는 문제는
V0.1 유지보수 패스 이후 **재현되지 않았다.** 새 결함이 아니라 확인 사항으로 남긴다.

### Status

`OBSERVED`

---


## OBS-020 — 중단된 Task는 Plan 루프로 스스로 돌아오지 못한다 (stale 회수 → RECOVERY_AMBIGUOUS → 사람의 transition)

**Date:** 2026-09-02 ~ 2026-09-03

**Project phase / Goal:** Molt Note Phase 2B — Reliable Recording

**Plan / Task / Run / Execution:**
- PLAN-20260902T080012Z · TASK-022 (문서 전용 Task)
- RUN-20260902T102750Z-TASK-022 · EXEC-20260902T102750Z-TASK-022
- EXEC-20260903T005041Z-TASK-022 (회수 시도) · PLANEXEC-20260903T005030Z

**Runtime stage:** Execute loop / Diagnose

### What happened

운영자가 퇴근 때문에 `execute-plan` 프로세스를 의도적으로 종료했다. 종료 시점에
TASK-022는 막 시작된 상태였고(run 디렉터리 생성 19:27:50, 종료 19:28), **산출물이 없었다.**

```text
.loop-local/runs/RUN-20260902T102750Z-TASK-022/
  context.md
  manifest.json          ← 이게 전부. envelope · gate · verification 없음
```

다음 날 재개할 때 Runtime의 상태 표현은 **정확했다.**

```text
ACTIVE EXECUTION
  TASK-022             STALE
      execution STALE: EXEC-20260902T102750Z-TASK-022  (no heartbeat for 51705s (limit 300s))
      the runtime stopped updating this marker; `loopctl execute TASK-022` will reclaim it
  (liveness comes from the runtime's own heartbeat, not from process liveness)
```

프로세스 생존이 아니라 **자체 heartbeat**로 판단한다는 설계 덕분에, 프로세스를 죽인 상태가
손상이 아니라 **스스로를 설명하는 상태**로 남았다. 여기까지는 좋다.

문제는 **거기서 자동으로 돌아올 방법이 없다는 것**이다. 세 단계가 전부 막혔다.

**1. `execute-plan`은 회수하지 못한다.**

```text
$ ./loopctl execute-plan PLAN-20260902T080012Z
Plan Result: NEEDS_HUMAN   stop_reason: PLAN_NO_READY_TASK
  no task is ready; outstanding: TASK-022 (IN_PROGRESS)
Duration: 0s        LLM invocations: 0
```

Plan 루프는 **READY(=TODO + 의존성 충족) Task만 dispatch한다.** `IN_PROGRESS`에 stale
마커가 붙은 Task는 그 조건에 들지 않는다. 정작 `status`는 그 Task를 지목하며 회수 명령까지
알려주고 있다.

**2. `execute <TASK>`는 마커를 회수하지만 거기서 멈춘다.**

```text
$ ./loopctl execute TASK-022
(reclaimed a stale execution marker from EXEC-20260902T102750Z-TASK-022)
Execution Result: NEEDS_HUMAN
Stop reason: RECOVERY_AMBIGUOUS
  TASK-022 is IN_PROGRESS but has no completed worker run;
  the runtime cannot tell whether a worker is still executing.
Attempts: 0   Duration: 0s   LLM invocations: 0
```

**이 거부 자체는 옳다.** Runtime 입장에서 "Worker가 어젯밤 죽었다"와 "Worker가 아직 돌고
있다"는 구분되지 않으며, 잘못 추측하면 살아 있는 Worker와 경쟁하게 된다.

**3. 결국 사람의 `transition`이 필요했다.**

```text
$ ./loopctl transition TASK-022 TODO
TASK-022: IN_PROGRESS -> TODO
$ ./loopctl ready
TASK-022  TODO  ...
```

그 뒤 `execute-plan`이 정상적으로 이어받아 TASK-022가 DONE이 됐다.

### Expected

**의도적이고 깨끗한 중단(산출물 0)조차 사람의 상태 전이 없이는 Plan 루프로 돌아오지 못한다.**

Runtime은 회수 시점에 이미 다음을 전부 알고 있었다.

```text
heartbeat가 51705초 동안 없었다 (limit 300s)
completed worker run이 없다 (runtime-envelope.json 부재)
Gate 결과가 없다
Verifier 결과가 없다
```

"heartbeat가 limit의 170배를 넘겼고 산출물이 하나도 없다"는 조합은 사실상 한 가지 상황만
가리킨다. 그럼에도 `Attempts: 0`인 Task를 TODO로 되돌리는 데 사람이 필요했다.

Worker가 살아 있을 가능성을 배제할 수 없다는 판단은 옳지만, **그 가능성을 좁힐 신호를
Runtime이 이미 갖고 있다.** 최소한 회수 명령이 그 신호를 근거로 제안이라도 할 수 있다.

### Current workaround

```text
loopctl execute <TASK>        # stale 마커 회수
loopctl transition <TASK> TODO
loopctl execute-plan <PLAN>
```

세 명령이며, 두 번째는 사람이 "정말 Worker가 없다"를 외부 지식으로 확인해야 한다.
이번에는 프로세스 목록 · run 디렉터리 내용 · 타임스탬프로 확인했다.

### Impact

**Medium.** 비용은 0이다(두 번의 실패한 시도 모두 LLM 호출 0회). 그러나
**계획된 중단조차 사람 없이는 재개되지 않는다.** 장시간 Plan을 밤새 돌리거나
자리를 비우는 운용에서 이 성질은 실질적인 제약이다.

OBS-019와 같은 뿌리다 — Plan 루프의 진입 조건이 READY 하나뿐이라 Task 생애주기의
중간 지점에서 다시 시작할 수 없다. OBS-019는 `REVIEW`에서, 이번은 stale `IN_PROGRESS`에서
같은 벽을 만났다.

### Possible Runtime improvement

- 회수 시 판단 근거를 함께 제시한다 — heartbeat 경과 · completed run 부재 ·
  Gate/Verifier 산출물 부재. 그리고 `Attempts: 0`이고 산출물이 없으면
  **TODO 복귀를 제안**한다 (자동 수행이 아니라 명시적 제안).
- `PLAN_NO_READY_TASK` 정지 메시지가 outstanding Task의 **다음 명령**을 안내한다.
  현재는 `no task is ready`라고만 말한다 (OBS-019와 동일한 지적).
- lease/heartbeat에 프로세스 식별자를 남겨 "그 프로세스가 살아 있는가"를
  Runtime이 직접 확인할 수 있게 한다.

### Evidence

```text
./loopctl status                       # STALE 표시와 회수 안내
PLANEXEC-20260903T005030Z.json         # PLAN_NO_READY_TASK · 0s · 0 LLM
EXEC-20260903T005041Z-TASK-022         # RECOVERY_AMBIGUOUS · Attempts 0 · 0 LLM
.loop-local/runs/RUN-20260902T102750Z-TASK-022/   # context.md · manifest.json 뿐
```

### Status

`OBSERVED`

---

## OBS-021 — 무거운 Task에서 900s worker timeout이 **작업 중인** 시도를 버린다 (8개 중 4개)

**Date:** 2026-09-02

**Project phase / Goal:** Molt Note Phase 2B — Reliable Recording

**Plan / Task / Run / Execution:** PLAN-20260902T080012Z · TASK-016 · TASK-017 · TASK-019 · TASK-021

**Runtime stage:** Worker

### What happened

Phase 2B의 8개 Task 중 **4개가 첫 시도에서 900초 timeout으로 폐기**됐고, 4개 모두
두 번째 시도에서 성공했다.

```text
TASK-015  state machine        attempts=1
TASK-016  backend session      attempts=2   ← timeout
TASK-017  Stop 확정·영속화     attempts=2   ← timeout
TASK-018  default microphone   attempts=1
TASK-019  macOS 권한           attempts=2   ← timeout
TASK-020  Recording 화면       attempts=1
TASK-021  Detail 재생          attempts=2   ← timeout
TASK-022  문서                 attempts=1
```

**중요한 것은 그 시도들이 놀고 있지 않았다는 점이다.** Runtime envelope의
subject fingerprint가 Run 전후로 전부 바뀌었다 — 즉 파일을 만들고 있었다.

| Task | timeout | 실제 duration | signal | dirty entries (전 → 후) |
| --- | --- | --- | --- | --- |
| TASK-016 | 900s | 900s | SIGKILL | 14 → **27** |
| TASK-017 | 900s | 900s | SIGKILL | 32 → **35** |
| TASK-019 | 900s | 900s | SIGKILL | 54 → **55** |
| TASK-021 | 900s | 900s | SIGKILL | 73 → **86** |

**이것은 OBS-016과 다른 현상이다.** OBS-016(Phase 1 TASK-002)에서는 Worker가 8분 29초
만에 작업을 마치고 **15분 19초 동안 아무것도 하지 않다가** 죽었다 — 무응답이 예산을
소진했다. 이번 넷은 **죽는 순간까지 산출물을 늘리고 있었다.**

부수적으로 **OBS-016이 지적한 timeout 초과 집행(900s 설정에 1428s 실행)은 재현되지 않았다.**
네 번 모두 정확히 900초에 SIGKILL됐다. OBS-016의 초과분은 timeout 로직의 결함이라기보다
그때의 provider 장애와 얽힌 현상이었을 가능성이 높아졌다 — **다만 이것은 추론이며
확정된 원인 규명이 아니다.**

### Expected

timeout은 "폭주하는 Worker를 끊는 안전장치"로 기능해야지 "정상 작업의 상한"이 되어서는
곤란하다. 지금은 후자에 가깝다 — 무거운 Task에서 **작업 중인 시도가 규칙적으로 버려진다.**

버려지는 비용이 작지 않다. 첫 시도가 만든 산출물은 다음 시도가 처음부터 다시 만든다.
Phase 2B는 8개 Task에 약 3시간이 걸렸고 그중 4번의 900초(총 1시간)가 폐기됐다.

무엇이 무거운지도 드러났다. UI 코드량이 아니라 **불확실성 아래서의 결정**이 시간을 쓴다 —
TASK-019(macOS 권한 판정 수단이 없음)와 TASK-021(Tauri asset protocol 범위 결정)은
겉보기에 작지만 둘 다 timeout에 걸렸다. Worker는 웹 조회를 할 수 없어(OBS-015 §What happened)
저장소 안에서 근거를 만들어야 하고, 그 과정이 길다.

### Current workaround

없다. 재시도가 자동으로 수렴하므로 사람 개입은 필요 없었다. 시간과 비용만 든다.

### Impact

**Medium.** 진행은 되지만 무거운 Task마다 예산이 사실상 두 배가 된다.
Phase 1 TASK-006, Phase 2A TASK-012에서도 같은 양상이 있었으므로 이 Plan에 국한되지 않는다.

### Possible Runtime improvement

- `worker_timeout_seconds`를 이 Task class에 맞게 올리는 것을 검토한다.
  **다만 무작정 올리면 진짜 폭주를 늦게 잡는다.** 이번 증거는 "900초가 부족하다"까지는
  지지하지만 적정값이 얼마인지는 말해 주지 않는다 — 네 시도 모두 상한에서 잘렸으므로
  **완료에 실제로 얼마가 필요했는지는 알 수 없다** [UNVERIFIED].
- Task별 timeout 재정의를 허용한다. 지금은 전역 값 하나다.
- 더 근본적으로는 Worker가 **결과 파일을 점진적으로 쓰게** 하는 것이 timeout에 강하다
  (OBS-016의 개선 후보와 같다). 지금은 마지막에 한 번 쓰므로 중간에 잘리면 전부 잃는다.

### Evidence

```text
.loop-local/runs/RUN-20260902T082429Z-TASK-016/runtime-envelope.json
.loop-local/runs/RUN-20260902T085101Z-TASK-017/runtime-envelope.json
.loop-local/runs/RUN-20260902T092655Z-TASK-019/runtime-envelope.json
.loop-local/runs/RUN-20260902T100954Z-TASK-021/runtime-envelope.json
   각각 process.timeout_seconds=900 · duration_ms≈900000 · signal=SIGKILL
   verification_subject_before/after 의 dirty_entry_count 증가
```

### Status

`OBSERVED`

---


## OBS-022 — PAUSE로 종결된 execution은 사람의 transition 없이 재개된다 (OBS-020과 다른 경로)

**Date:** 2026-09-03 중단 · 2026-09-04 재개

**Project phase / Goal:** Molt Note Phase 4 — AI Provider System + Local AI

**Plan / Task / Run / Execution:**
- PLAN-20260903T053705Z · TASK-038
- 중단: RUN-20260903T073115Z-TASK-038 · EXEC-20260903T073115Z-TASK-038
- 재개: RUN-20260904T005636Z-TASK-038 · EXEC-20260904T005636Z-TASK-038

**Runtime stage:** Execute loop / Diagnose / 재개

### What happened

OBS-020과 같은 상황(운영자가 퇴근으로 중단)이었지만 **재개 경로가 달랐다.**

OBS-020의 TASK-022는 Run이 **막 시작된** 상태에서 프로세스가 죽어, 다음 날 stale
execution 마커로 남았다. 이번 TASK-038은 Worker가 끝까지 돌다 timeout으로 SIGKILL됐고,
Runtime이 **그 자리에서 진단까지 마친 뒤** PAUSE 때문에 정지했다.

```text
EXEC-20260903T073115Z-TASK-038
  result:      NEEDS_HUMAN
  stop_reason: PAUSE_ACTIVE
  attempts[0]: worker=failed · gate=null · verifier=null
               diagnosis=TIMEOUT · action=RETRY_WITH_HINT
  final_task_status: IN_PROGRESS
```

즉 execution이 **종결된 채로** 남았고 stale 마커가 아니었다. `status`도 STALE을 말하지
않고 `latest execution: NEEDS_HUMAN (PAUSE_ACTIVE)`를 말했다.

그래서 재개는 두 단계로 끝났다.

```text
rm .loop-local/PAUSE
./loopctl execute TASK-038      → Attempt 2 → Gate PASS → Verifier PASS → DONE
```

**`RECOVERY_AMBIGUOUS`가 나오지 않았고 `transition TASK-038 TODO`도 필요 없었다.**
이미 기록된 `RETRY_WITH_HINT`를 Runtime이 그대로 이어받았다.

### Expected

이 동작이 옳다. 그리고 **OBS-020의 결론을 좁힌다.**

OBS-020은 "계획된 중단조차 사람 없이는 Plan 루프로 돌아오지 못한다"고 적었다. 이번 증거는
그것이 **중단 시점에 달렸다**는 것을 보여준다.

```text
Run이 시작만 된 채 죽었다   → 산출물도 진단도 없다 → stale 마커 → 사람의 transition 필요
Worker가 죽고 진단이 남았다 → RETRY_WITH_HINT 기록됨 → PAUSE 해제만으로 자동 재개
```

Runtime은 **자기가 진단을 남길 수 있었던 중단**에서는 사람 없이 돌아온다. 돌아오지
못하는 것은 진단조차 남기지 못한 중단이다. OBS-020의 개선 후보(회수 시 판단 근거 제시)는
여전히 유효하며, 적용 범위가 후자로 좁혀진다.

### Current workaround

없다. 사람이 한 일은 PAUSE 파일 삭제 하나이며, 그것이 PAUSE의 정의된 해제 방법이다
(README §운영 표). `.loop-local/`은 `.gitignore` 대상이라 PAUSE 삭제가 subject
fingerprint를 건드리지 않는다는 점도 이번에 확인됐다.

### Impact

**Low (긍정적 관측).** 비용 0, 사람 개입 1회(파일 삭제).

다만 **운영자가 어느 쪽 중단인지 미리 알 수 없다는 점**은 남는다. 이번에는 `status`와
execution report를 읽고 나서야 "transition이 필요 없는 쪽"임을 알았다. `status`가
`NEEDS_HUMAN (PAUSE_ACTIVE)` 옆에 다음 명령을 함께 말해 주면 그 판별이 필요 없다
(OBS-019 · OBS-020과 같은 지적).

### Possible Runtime improvement

- `status`의 정지 표시가 **다음 명령**을 함께 말한다 — `PAUSE_ACTIVE`이고 진단이
  기록돼 있으면 "PAUSE를 해제하고 `execute`하면 이어진다", 진단이 없으면 OBS-020의 경로.
- `loopctl pause` / `loopctl resume`가 CLI에 없다. 지금은 파일을 직접 만들고 지운다.
  README는 그렇게 안내하지만 `help`에는 나오지 않아 발견성이 낮다.

### Evidence

```text
.loop-local/executions/EXEC-20260903T073115Z-TASK-038/execution-report.json
   result=NEEDS_HUMAN · stop_reason=PAUSE_ACTIVE · attempts[0].action=RETRY_WITH_HINT
.loop-local/runs/RUN-20260903T073115Z-TASK-038/recovery/diagnosis.json
   failure_class=TIMEOUT · subject_check.matches=true
.loop-local/executions/EXEC-20260904T005636Z-TASK-038/execution-report.json
   result=DONE · attempts=2 · 3m42s · $4.5558
```

### Status

`OBSERVED`

---

## OBS-023 — timeout 초과 집행이 재현됐고(1800s 설정 / 8619s 실행), 잘린 Worker의 산출물을 shared working tree가 살렸다

**Date:** 2026-09-03 ~ 2026-09-04

**Project phase / Goal:** Molt Note Phase 4 — AI Provider System + Local AI

**Plan / Task / Run:** PLAN-20260903T053705Z · TASK-038 · RUN-20260903T073115Z-TASK-038

**Runtime stage:** Worker

### What happened — 두 가지가 함께 관측됐다

**(1) OBS-021이 "재현되지 않았다"고 적은 timeout 초과 집행이 재현됐다.**

```text
process.timeout_seconds : 1800
process.duration_ms     : 8619593   (2시간 23분 39초 · 설정의 4.8배)
process.timed_out       : true
process.signal          : SIGKILL
started_at              : 2026-09-03T07:31:15Z
finished_at             : 2026-09-03T09:54:55Z
```

OBS-021은 Phase 2B에서 네 번 모두 **정확히** 900초에 SIGKILL된 것을 근거로, OBS-016의
초과분(900s 설정 / 1428s 실행)을 "provider 장애와 얽힌 현상일 가능성"으로 정리했다.
이번 값은 그 설명으로 덮기에는 크다.

**[관측된 사실]** timeout 집행이 설정값의 4.8배 뒤에 일어났다.
**[가능한 설명 · 미검증]** 운영자가 같은 시각에 퇴근하며 Mac을 닫았다. 프로세스가 sleep
동안 정지했다면 벽시계 기준 경과와 timeout 타이머의 기준이 어긋난다. adapter가 남긴
`terminal_reason: api_error` — `Can't reach the API server (ENOTFOUND)` 도 그 시각의
네트워크 단절과 일치한다. **다만 sleep 여부를 이 Run의 산출물로 확인하지는 못했다.**
timeout 타이머가 monotonic clock을 쓰는지 wall clock을 쓰는지도 확인하지 않았다.

**(2) 잘린 Worker의 산출물이 다음 시도에서 그대로 쓰였다.**

Worker는 **작업을 사실상 끝낸 상태**에서 결과 파일만 못 쓰고 죽었다.

```text
observed_changes.count : 8   (제품 코드 · 테스트 · evidence 3종)
.loop/evidence/TASK-038/{summary,acceptance-map,gate-results}.md   ← 전부 작성됨
gate-results.md : self-check build/lint/test 세 개 모두 exit 0 기록
worker-result.json : 없음   ← Runtime 계약상 "산출물 없음"
provider_cost_usd : 16.01754
```

Runtime은 이것을 **실패한 시도**로 회계했다(`worker=failed` · `gate=null` · `verifier=null`).
그러나 파일은 shared working tree에 남았고, 다음 날 Attempt 2가 그 위에서 시작해
**3분 42초 · $4.56**에 DONE이 됐다. 처음부터 다시 만들었다면 다시 두 시간대였을 것이다.

### Expected

OBS-021의 개선 후보 — "Worker가 결과 파일을 점진적으로 쓰게 한다" — 에 대한 **직접적인
추가 근거다.** 이번 경우 잃은 것은 작업이 아니라 **작업했다는 기록 한 개**였다.
$16.02짜리 시도가 Runtime 회계에서 통째로 실패로 남은 이유가 그것뿐이다.

동시에 이것은 **shared working tree에 대한 반대 방향의 증거**이기도 하다.
OBS-003 이후 필드 노트는 per-Task worktree isolation을 후보로 적어 왔다. 이번에는
공유 트리가 **의도치 않은 복구 수단**으로 작동했다 — 격리된 worktree였다면 Attempt 1의
산출물은 폐기됐을 것이다. 두 성질은 맞바꿈 관계이며, isolation을 도입한다면
**잘린 시도의 산출물을 어떻게 넘길지**를 함께 정해야 한다.

### Current workaround

없다. 사람이 한 일은 재시도를 Runtime에 맡긴 것뿐이다.
**수동으로 `worker-result.json`을 만들지 않았다** — 그것은 Runtime 판정을 위조하는 것이다.

### Impact

**Medium.** 이번에는 공유 트리 덕분에 손실이 $16.02의 회계상 폐기에 그쳤다.
그러나 (a) 그 절약은 설계된 것이 아니라 부수 효과이고, (b) timeout이 설정값의 4.8배
뒤에 걸린다면 timeout은 폭주를 끊는 안전장치로 기능하지 못한다.

### Possible Runtime improvement

- **결과 파일을 점진적으로 쓴다** (OBS-016 · OBS-021과 같은 후보. 근거 1건 추가).
  최소한 Worker가 self-check를 통과한 시점에 부분 결과를 남길 수 있으면, 이번 시도는
  실패가 아니라 검증 대기로 남았을 것이다.
- **timeout 집행이 monotonic clock 기준인지 확인한다.** wall clock 기준이면 sleep /
  suspend에서 이번과 같은 초과가 구조적으로 발생한다. [확인 필요 · 미검증]
- envelope에 **timeout 초과분**을 명시적으로 기록한다. 지금은 `timeout_seconds`와
  `duration_ms`를 사람이 비교해야 초과를 알아챈다.
- worktree isolation을 검토할 때 **잘린 시도의 산출물 인계**를 함께 설계한다 (OBS-003 관련).

### Evidence

```text
.loop-local/runs/RUN-20260903T073115Z-TASK-038/runtime-envelope.json
   process.{timeout_seconds=1800, duration_ms=8619593, signal=SIGKILL, timed_out=true}
   adapter_meta.terminal_reason=api_error · worker_result_valid=false
   observed_changes.count=8 · usage.provider_cost_usd=16.01754
.loop-local/runs/RUN-20260903T073115Z-TASK-038/stdout.log
   "API Error: Can't reach the API server — check your internet or DNS (ENOTFOUND)"
.loop/evidence/TASK-038/gate-results.md   (Attempt 1이 남긴 self-check 기록)
.loop-local/executions/EXEC-20260904T005636Z-TASK-038/execution-report.json
   Attempt 2: 3m42s · $4.5558 · Gate PASS · Verifier PASS
```

### Status

`OBSERVED`

### 추가 증거 — Phase 5 (2026-09-04) · 재현 2건

같은 성질이 Phase 5에서 **두 번 더** 재현됐다. 새 observation을 만들지 않고 여기에 붙인다.

| Task | Run | 중단 시점까지 Worker가 쓴 제품 파일 | worker-result | 재시도 결과 |
| --- | --- | --- | --- | --- |
| TASK-048 | RUN-20260904T044650Z | `notion/chunk.rs` · `notion/mod.rs` · `tests/notion_chunking.rs` (3개) | 없음 | **7.6분 · $6.05** — 다른 Task 평균(약 20분)의 3분의 1 |
| TASK-052 | RUN-20260904T063452Z | `commands/notion.rs` · `notion/{client,mod,wire}.rs` · `screens/notionSettings.*` · `SettingsScreen.tsx` 등 **12개** | 없음 | 첫 시도 통과 |

두 Run 모두 `context.md` · `manifest.json`만 남고 `runtime-envelope.json` ·
`worker-result.json` · Gate · Verifier 산출물이 **전부 없었다.** Runtime 회계상 "산출물 없음"
이지만 **제품 코드는 공유 트리에 있었다.**

TASK-048의 재시도가 7.6분에 끝난 것이 이 성질을 수치로 보여준다 — 재시도 Worker는 이미
완성된 `chunk.rs`를 다시 만들 필요가 없었고, Gate(lint · test)와 Verifier가 그 파일들을
그대로 PASS로 판정했다. **잃은 것은 작업이 아니라 "작업했다는 기록" 하나였다** (CI-011).

Phase 4 TASK-038까지 합쳐 **근거 3건**이 됐다. 공유 트리가 매번 복구 수단으로 작동했다는
사실은 worktree isolation(OBS-003)을 도입할 때 **잘린 시도의 산출물 인계**를 함께 설계해야
한다는 근거도 그만큼 강해졌다는 뜻이다.

### 추가 증거 — Phase 5.5 (2026-09-04) · 재현 4건째 · **self-check 통과까지 끝낸 시도가 폐기됐다**

| Task | Run | Worker가 남긴 것 | worker-result | Runtime 분류 |
| --- | --- | --- | --- | --- |
| TASK-060 | RUN-20260904T092924Z | `src/screens/SettingsScreen.tsx` · `aiProviderSettings.ts` · `aiProviderSettings.test.ts` + evidence 2종(`changed-files.md` · `gates.md`) | 없음 | `worker=failed` · `TIMEOUT` -> `RETRY_WITH_HINT` |

```text
process.timeout_seconds   : 1800
process.duration_ms       : 7104251   (1시간 58분 24초 · 설정의 3.9배)
process.signal            : SIGKILL · timed_out=true
adapter_meta.terminal_reason : api_error · is_error=true
adapter_meta.duration_api_ms : 486794  (8분 7초 — 실제 API 활동 시간)
usage.provider_cost_usd      : 4.357417
stdout.log: "API Error: Can't reach the API server — check your internet or DNS (ENOTFOUND)"
```

**이전 3건(TASK-038 · 048 · 052)과 다른 점 세 가지가 관측됐다.**

**(1) 이번 Worker는 self-check까지 끝내고 그 기록을 남겼다.**
`.loop/evidence/TASK-060/gates.md`에 build · lint · test 세 Gate가 **모두 exit 0**으로
기록돼 있다(`Self-check: all gates passed`). Runtime이 "산출물 없음 · 실패"로 회계한 시도가
실제로는 **세 Gate를 통과한 상태**였다. 이 세션에서 사람이 `npm run typecheck`를 다시 돌려
exit 0을 확인했다 — 트리는 온전하다.

OBS-023이 적은 개선 후보 — "결과 파일을 점진적으로 쓴다" — 에 대한 **가장 강한 근거다.**
부분 결과를 남길 지점이 추측이 아니라 **실재했다**: self-check가 통과한 순간이다.
그 시점에 부분 결과가 있었다면 이 시도는 실패가 아니라 **검증 대기**로 남았을 것이다.

**(2) timeout 초과 집행이 다시 관측됐다** (1800s 설정 / 7104s 실행 · 3.9배).
OBS-023의 4.8배와 성질이 같고, **두 경우 모두 `api_error`와 함께** 일어났다.
**[가능한 설명 · 미검증]** 운영자 Mac의 sleep. **[미검증]** timeout 타이머가 monotonic clock을
쓰는지 wall clock을 쓰는지는 OBS-023 이후로도 확인하지 않았다. 근거 2건이 됐다.

**(3) `api_error` 실패가 `TIMEOUT`으로 분류된다.**
Diagnose는 `worker-result.json` 부재만 보고 원인을 구분하지 않는다. 그런데 원인은 이미
envelope 안에 있다 — `adapter_meta.terminal_reason=api_error`. 실제 API 활동은 8분 7초였고
(`duration_api_ms=486794`), 1800초 timeout과는 무관한 실패다.

**[관측된 사실]** 서로 다른 두 실패(무응답 / provider 연결 불가)가 같은 분류·같은 재시도
힌트를 받는다. OBS-015가 적은 "launch 실패가 TIMEOUT 분류를 물려받는다"와 **같은 성질이며,
`TIMEOUT` 분류가 세 번째 원인까지 흡수한 사례다.**

**[가능한 개선 · 미구현]** Diagnose가 `adapter_meta.terminal_reason`을 읽으면 `TIMEOUT`과
`PROVIDER_ERROR`를 구분할 수 있다. 구분되면 재시도 힌트의 내용이 달라진다 —
provider 연결 실패에 "작업을 더 작게 쪼개라"는 힌트는 도움이 되지 않는다.
또한 사람이 `stdout.log`를 열어야만 진짜 원인을 알 수 있는 현재 상태도 개선된다
(`loopctl status`는 `worker: failed (TIMEOUT)`만 보여준다).

**Status:** `OBSERVED` · 근거 4건 (TASK-038 · 048 · 052 · 060)

---

## OBS-024 — 살아 있는 Worker가 STALE로 표시된다 (heartbeat가 Worker 단계에서 갱신되지 않는다)

**Date:** 2026-09-04

**Project phase / Goal:** Molt Note Phase 5 — Notion & Markdown Export

**Plan / Task / Run / Execution:**
- PLAN-20260904T025945Z · TASK-045 (관측) · TASK-050 (재현)
- EXEC-20260904T034150Z-TASK-045 · EXEC-20260904T055314Z-TASK-050

**Runtime stage:** 활성 실행 표식 / `status`

### What happened

Phase 5 실행 도중 진행 상황을 보려고 `loopctl status`를 읽었다. Runtime은 이렇게 말했다.

```text
ACTIVE EXECUTION
  TASK-045             STALE
      execution STALE: EXEC-20260904T034150Z-TASK-045  (no heartbeat for 512s (limit 300s))
      the runtime stopped updating this marker; `loopctl execute TASK-045` will reclaim it
  (liveness comes from the runtime's own heartbeat, not from process liveness)
```

**그러나 모든 것이 살아 있었다.** 프로세스 목록으로 직접 확인했다.

```text
PID 16818  node .../loopctl.mjs execute-plan PLAN-20260904T025945Z   40분 14초 경과 · 생존
PID 24666  claude --print ...                                        9분 11초 경과 · CPU 1.3% · 생존
           └ 시스템 프롬프트에 "RUN-20260904T034150Z-TASK-045"가 박혀 있어 이 Task의 Worker임이 확정
PID 16821  caffeinate -ims                                           생존
```

원인은 활성 표식 자체에 있었다.

```json
{ "task_id": "TASK-045", "pid": 16818,
  "started_at":   "2026-09-04T03:41:50.249Z",
  "heartbeat_at": "2026-09-04T03:41:50.249Z",   ← 시작 이후 한 번도 갱신되지 않았다
  "stage": "starting", "run_id": null, "attempt": null }
```

`heartbeat_at`이 `started_at`과 **정확히 같고**, `stage`는 여전히 `"starting"`이며
`run_id`는 `null`이다. Worker가 9분째 돌고 있는데도 그렇다.

TASK-050에서도 같은 값으로 재현됐다(`stage=starting` · `run=None`).

### Expected

**Worker 단계가 300초를 넘기는 Task는 살아 있어도 예외 없이 STALE로 표시된다.**
이 저장소의 Task는 Phase 4·5에서 평균 15~26분이 걸렸으므로, 사실상 **거의 모든 Task가
실행 중에 STALE로 보인다.**

이것이 위험한 이유는 표시가 틀렸다는 것에 그치지 않는다. **Runtime이 그 상태에서
회수 명령을 제안한다.**

```text
`loopctl execute TASK-045` will reclaim it
```

그 제안을 따랐다면 **살아 있는 Worker와 경쟁하게 된다.** OBS-020이 "Runtime 입장에서
'Worker가 죽었다'와 '아직 돌고 있다'는 구분되지 않는다"며 `RECOVERY_AMBIGUOUS` 거부를
옳다고 평가했는데, 여기서는 Runtime이 스스로 그 위험한 행동을 **권한다.**

이번에는 운영자가 프로세스 목록을 직접 확인해서 오탐임을 알아냈다. 그 확인 수단이 없었다면
멀쩡히 돌고 있는 $15짜리 Worker를 회수하려 했을 것이다.

### ⚠️ CI-008에 대한 반대 증거

CI-008은 이렇게 적혀 있다.

```text
CI-008 | Run 시작 시 execution 레코드 선기록 + PID/heartbeat → status에 RUNNING/STALE 표시
        | OBS-006 | Medium | IMPLEMENTED (V0.1 §4 — 기존 executions/active/ 표식을 heartbeat 기반으로)
```

**표식과 PID 기록은 실제로 구현됐다.** 관측된 marker에 `pid: 16818`이 그대로 들어 있다.
구현되지 않은 것은 **그 두 가지를 실제로 쓰는 부분**이다.

```text
구현됨    Run 시작 시 표식 선기록 · pid 기록 · heartbeat 필드 · STALE 판정 로직
안 됨     Worker 단계 동안 heartbeat 갱신 · 기록된 pid로 프로세스 생존 확인
```

OBS-020은 이 설계를 "프로세스 생존이 아니라 자체 heartbeat로 판단한다"며 긍정적으로
평가했다. 그 평가는 **Worker가 죽은 경우에 대해서만 옳았다.** heartbeat가 갱신되지 않으면
그 신호는 "죽었다"와 "일하고 있다"를 구분하지 못한다 — OBS-020이 피하려던 바로 그 모호성이
표시 계층으로 옮겨온 것이다.

**CI-008을 IMPLEMENTED에서 되돌리지 않는다** — 표식·PID·판정 로직은 실제로 들어갔다.
대신 그 위에 남은 구멍을 CI-015로 새로 세운다. 기록을 나중에 고쳐 쓰지 않는다.

### Current workaround

운영자가 `ps`로 직접 확인한다.

```bash
pgrep -f "loopctl.mjs execute"   # 오케스트레이터 생존
pgrep -f "claude --print"        # Worker 생존
ps -o command= -p <pid> | grep -o 'RUN-[0-9TZ]*-TASK-[0-9]*'   # 어느 Run의 Worker인가
```

세 번째 명령이 결정적이다 — Worker의 시스템 프롬프트에 `run_id`가 들어 있어 **어느 Task의
Worker인지 확실히 알 수 있다.** Runtime이 이미 자기가 만든 정보다.

### Impact

**Medium-High.** 비용은 0이지만 **잘못된 행동을 유도한다.**

- 실행 중 `status`가 사실상 항상 STALE을 보여주므로 **진짜 STALE과 구분되지 않는다.**
  이번 Phase에서 실제로 두 번의 진짜 중단(TASK-048 · TASK-052)이 있었고, 그때의 표시가
  살아 있을 때의 표시와 **글자 그대로 같았다.**
- 표시를 믿고 회수하면 살아 있는 Worker와 경쟁한다.
- 자동화된 운영(밤새 실행 · 원격 모니터링)에서 이 신호는 쓸 수 없다.

### Possible Runtime improvement

- **Worker 단계 동안 heartbeat를 갱신한다.** 최소한 Run 시작 · 각 단계 진입 · 주기적 tick.
  `stage`와 `run_id`도 함께 채우면 `stage=starting` · `run_id=null`이 9분간 유지되는
  현상이 사라진다.
- **기록된 `pid`로 프로세스 생존을 확인한 뒤 STALE을 말한다.** marker에 이미 pid가 있다.
  heartbeat가 낡았어도 그 pid가 살아 있으면 STALE이 아니라 "heartbeat 지연"이다.
- **살아 있을 가능성이 있는 실행에는 회수 명령을 제안하지 않는다.** 지금은 조건 없이 권한다.
- Worker의 시스템 프롬프트에 `run_id`가 들어가는 점을 활용해, `status`가 실제로 어느 Worker
  프로세스가 어느 Run에 속하는지 보여줄 수 있다.

### Evidence

```text
.loop-local/executions/active/TASK-045.json   heartbeat_at == started_at · stage=starting · run_id=null · pid=16818
.loop-local/executions/active/TASK-050.json   같은 형태로 재현
ps -o command= -p 24666                        시스템 프롬프트에 RUN-20260904T034150Z-TASK-045
./loopctl status                               "STALE ... will reclaim it" (Worker 생존 중)
EXEC-20260904T034150Z-TASK-045                 이후 정상 DONE — 오탐이었음이 결과로 확인됨
EXEC-20260904T055314Z-TASK-050                 이후 정상 DONE
```

### Status

`OBSERVED`

---

## OBS-025 — 대화형 harness의 background 실행이 약 90분 시점에 두 번 외부 종료됐다 (원인 미확정)

**Date:** 2026-09-04

**Project phase / Goal:** Molt Note Phase 5 — Notion & Markdown Export

**Plan / Task:** PLAN-20260904T025945Z · TASK-048 · TASK-052

**Runtime stage:** 오케스트레이터 프로세스 수명 (Runtime 밖의 문제일 수 있다)

### What happened

`execute-plan`을 대화형 세션의 background 명령으로 띄웠고, **두 번 모두 프로세스 그룹 전체가
외부에서 종료됐다.**

| 회차 | 시작 | 종료 시점의 Task | 그때까지 DONE | 경과 |
| --- | --- | --- | --- | --- |
| 1 | 12:10 | TASK-048 (시작 직후) | 5개 | 약 96분 |
| 2 | 14:19 | TASK-052 (시작 직후) | 4개 더 | 약 89분 |

두 번 다 **함께** 죽었다.

```text
node loopctl.mjs execute-plan   죽음
claude --print (Worker)          죽음
caffeinate -ims (래퍼)           죽음
```

**Runtime의 자체 진단은 남지 않았다** — `diagnosis.json` · `failure-memo.json` ·
`execution-report.json`이 전부 없다. Runtime이 진단 단계에 닿기 전에 프로세스가 사라졌다.
남은 것은 `context.md` · `manifest.json`과 stale 표식뿐이었다(OBS-020과 같은 형태).

### 이것이 무엇의 증거가 **아닌지** 먼저 적는다

- **macOS sleep의 증거가 아니다.** 별도 터미널의 `caffeinate -dimsu`(PID 67148)는 두 번 다
  살아남았다. sleep이었다면 그것도 함께 멈췄을 이유가 없고, 무엇보다 **다른 프로세스는
  멀쩡했다.**
- **Runtime timeout의 증거가 아니다.** `worker_timeout_seconds`는 1800이고 두 Worker 모두
  그보다 훨씬 짧게 살았다(각각 약 12분 · 약 14분). timeout 집행이라면 SIGKILL과 함께
  envelope이 남았을 것이다 — OBS-023의 TASK-038이 그랬다.
- **Runtime 결함의 증거가 아니다.** Runtime은 자기가 죽는 것을 관측하거나 기록할 위치에
  있지 않았다.

**[관측된 사실]** 두 번, 약 90분 간격으로, 프로세스 그룹 전체가 진단 없이 사라졌다.
**[미확정]** 무엇이 종료시켰는지. 대화형 harness의 background 실행 수명 제한일 가능성이
있으나 **이 Run의 산출물로 확인하지 못했다.**

### 결정적인 대조 증거

이후 운영자가 **일반 터미널에서 직접** 같은 명령을 실행했다.

```bash
./loopctl execute-plan PLAN-20260904T025945Z
```

남은 3개 Task가 **38분 25초 동안 중단 없이 정상 완료**됐다. 같은 Runtime · 같은 Plan ·
같은 기기 · 같은 시간대다. **달라진 것은 실행을 감싼 프로세스 환경뿐이다.**

이것은 원인이 Runtime이 아니라 **실행을 감싼 쪽에 있다**는 것을 강하게 시사한다 —
다만 그 쪽이 정확히 무엇인지는 여전히 미확정이다.

### Impact

**Low (Runtime 관점).** Runtime은 두 번 다 **정확하게 복구 가능한 상태로 남았다.**
stale 표식 → `execute` 회수 → `RECOVERY_AMBIGUOUS` → 사람의 `transition` → 재개.
비용 손실은 회수 시도 2회 × $0.00이며, 중단된 Worker의 산출물은 공유 트리에 남아
재시도가 오히려 빨랐다(OBS-023 추가 증거).

**운영 관점에서는 Medium.** 장시간 Plan을 대화형 세션의 background로 돌리는 운용은
이 프로젝트에서 **두 번 다 실패했다.**

### Current workaround

**장시간 `execute-plan`은 지속되는 별도 터미널에서 직접 실행한다.**
`caffeinate ... execute-plan` 래퍼는 실행 harness와 함께 죽으므로 그것만 믿지 않는다.
sleep 방지가 필요하면 `caffeinate -dimsu`를 **독립 터미널 세션에서** 따로 띄운다.

### Possible Runtime improvement

Runtime 쪽에 고칠 것이 있는지는 **아직 분명하지 않다.** 다만 두 가지는 값이 있다.

- 오케스트레이터가 SIGTERM/SIGHUP을 받았을 때 **최소한의 종료 기록**(어느 단계에서 무엇을
  하다 멈췄는지)을 남기면, 이번 같은 외부 종료가 "진단 없음"이 아니라 "외부 종료됨"으로
  구분된다. 지금은 OBS-020의 깨끗한 중단과 구별되지 않는다.
- OBS-024가 고쳐지면 이런 중단의 판별이 훨씬 쉬워진다 — 표식의 pid로 "정말 죽었는지"를
  Runtime이 스스로 답할 수 있다.

**Field-Test Principle에 따라 지금은 Runtime을 고치지 않는다.** 근거 2건이며 원인이
Runtime 밖일 가능성이 크다. 같은 현상이 **독립 터미널 실행에서도** 재현되면 그때 다시 본다.

### Evidence

```text
.loop-local/runs/RUN-20260904T044650Z-TASK-048/   context.md · manifest.json 뿐
.loop-local/runs/RUN-20260904T063452Z-TASK-052/   context.md · manifest.json 뿐
   두 Run 모두 runtime-envelope.json · worker-result.json · recovery/ 없음
.loop-local/executions/active/TASK-048.json · TASK-052.json   stale 표식 (pid는 죽은 프로세스)
EXEC-20260904T051855Z-TASK-048   RECOVERY_AMBIGUOUS · attempts 0 · LLM 0 · $0.00
EXEC-20260904T065048Z-TASK-052   RECOVERY_AMBIGUOUS · attempts 0 · LLM 0 · $0.00
독립 터미널 재실행                 남은 3 Task · 38분 25초 · 중단 없음
```

### Status

`OBSERVED` — 원인 미확정. Runtime 밖일 가능성이 크다.

---


# Candidate Improvements

실제 사용 사례가 충분히 쌓인 항목만 이 표로 승격한다.

| ID | Improvement | Evidence | Priority | Status |
|---|---|---|---|---|
| CI-001 | 정지 사유에 subject diff(ADDED/REMOVED/CHANGED 경로) 포함 | OBS-003 | High | CANDIDATE |
| CI-002 | `loopctl resume <RUN>` — gate 재실행 → verify 복구 경로 | OBS-003, OBS-004 | Medium | CANDIDATE |
| CI-003 | Worker Context에 실제 capability(명령 실행/네트워크/evidence 쓰기) 선언 | OBS-002 (Phase 1 전체 8/8), OBS-005, OBS-007, OBS-008 | High | **IMPLEMENTED** (V0.1 §1·§2 — Result Protocol의 RUNTIME CAPABILITIES 절) |
| CI-004 | `loopctl usage`에 Task 간 비용/토큰 추세 표시 (근거 수정: 순번이 아니라 Task 크기 대비 비용) | OBS-005 (Phase 1 실측으로 순번 가설 반증) | Medium | CANDIDATE |
| CI-005 | `limits.yaml`에 Worker 비용/토큰 상한 (현재는 실패 횟수만) | OBS-005 | Low | CANDIDATE |
| CI-006 | Worker에게 `stop_condition.gates` 명령만 allow-list로 실행 허용 (또는 `loopctl gate --self-check`) | OBS-007 ($4.04 / 9분 폐기), OBS-008 (Worker가 Phase 비용의 88% 차지) | High | **IMPLEMENTED** (V0.1 §2 — `loopctl self-check`) |
| CI-007 | Worker deny list를 fingerprint `PROTECTED_EXCEPTIONS`와 일치시켜 `.loop/evidence/**` 쓰기 허용 | OBS-002 후속 (Phase 1 8 Task / 9 Run 전부 재현, evidence 전량 공백) | High | **IMPLEMENTED** (V0.1 §1 — `worker/policy.mjs`, Task 단위로 좁힘) |
| CI-008 | Run 시작 시 execution 레코드 선기록 + PID/heartbeat → `status`에 RUNNING/STALE 표시 | OBS-006 | Medium | **IMPLEMENTED** (V0.1 §4 — 기존 `executions/active/` 표식을 heartbeat 기반으로) |
| CI-009 | Verifier Context에도 Worker capability를 전달 — "실행/측정/수동확인했다"는 주장을 의심할 근거 | OBS-009 | Medium | **IMPLEMENTED** (V0.1 §3 — WITNESSED EXECUTION + `evidence_basis` 계약) |
| CI-010 | `yaml-lite`가 double-quoted scalar의 `\"` 이스케이프를 처리하지 않는다 | V0.1 부수 발견 (fail-closed, 근거 1건) | Low | **IMPLEMENTED** (CI-010 Minimal Fix 절 — 스캐너 + 디코더, 회귀 18건) |
| CI-011 | Worker가 결과 파일을 **점진적으로** 쓴다 (또는 self-check 통과 시점에 부분 결과를 남긴다) | OBS-016, OBS-021, **OBS-023 ($16.02 시도가 결과 파일 하나 때문에 실패로 회계됨)** | High | CANDIDATE |
| CI-012 | timeout 집행이 monotonic clock 기준인지 확인하고, envelope에 초과분을 명시 기록 | OBS-016, **OBS-023 (1800s 설정 / 8619s 실행 · 4.8배)** | Medium | CANDIDATE |
| CI-013 | `status`의 정지 표시가 **다음 명령**을 함께 말한다 (`PAUSE_ACTIVE` + 진단 있음 → PAUSE 해제 후 `execute`) | OBS-019, OBS-020, **OBS-022** | Medium | CANDIDATE |
| CI-014 | `loopctl pause` / `loopctl resume`를 CLI에 노출 (현재는 파일을 직접 만들고 지우며 `help`에 없다) | OBS-022 | Low | CANDIDATE |
| CI-015 | **Worker 단계 동안 heartbeat를 갱신하고, STALE을 말하기 전에 표식의 `pid`로 프로세스 생존을 확인한다.** 살아 있을 수 있는 실행에는 회수 명령을 제안하지 않는다 | **OBS-024 (CI-008이 남긴 구멍 — 표식·pid·판정 로직은 들어갔으나 갱신과 생존 확인이 없다)** | **High** | CANDIDATE |
| CI-016 | 오케스트레이터가 SIGTERM/SIGHUP에서 최소한의 종료 기록을 남겨 **외부 종료**를 깨끗한 중단과 구별한다 | OBS-025 (근거 2건 · 원인 미확정) | Low | CANDIDATE |

권장 Status:

- `CANDIDATE`
- `VALIDATED`
- `PLANNED`
- `IMPLEMENTED`
- `REJECTED`

`IMPLEMENTED` 항목의 구현 내용과 회귀 테스트는 위 **V0.1 Maintenance Pass** 절에 있다.

---

# Ideas — Not Yet Validated

아직 실제 문제로 확인되지 않은 아이디어를 임시로 적는다.

이 섹션의 항목은 **Runtime 개발 요구사항으로 간주하지 않는다.**

예시:

- `loopctl init`으로 신규 프로젝트 bootstrap 자동화
- dependency-aware `execute-plan`
- shared working tree 대신 per-Task worktree isolation — OBS-003이 첫 실제 근거
- Planner Task granularity 개선
- Gate 자동 탐지 / 제안
- Runtime 코드를 프로젝트 밖 개인 도구로 추출

---

# Review Checkpoint

3D Asset Compatibility Lab의 주요 Goal 또는 Phase 하나가 끝날 때마다 이 문서를 검토한다.

검토할 질문:

1. 같은 불편이 두 번 이상 발생했는가?
2. 사람이 반복적으로 개입해야 했는가?
3. Runtime이 잘못 멈추거나 불필요하게 재시도했는가?
4. Planner가 Task를 너무 크게 또는 너무 작게 나눴는가?
5. Gate / Verifier가 실제 완료 조건을 제대로 판별했는가?
6. token / provider cost / 실행 시간이 과도했는가?
7. 새 프로젝트 bootstrap 과정에서 반복 작업이 있었는가?
8. shared working tree 때문에 STALE / ambiguity가 자주 발생했는가?
9. 실제 사용 결과 Runtime V1에 넣을 가치가 확인된 기능은 무엇인가?

이 리뷰를 기반으로 다음 Runtime 개선 순서를 정한다.
