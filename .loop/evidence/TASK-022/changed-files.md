# TASK-022 — 변경 범위

```text
Run:        RUN-20260903T005130Z-TASK-022
기록 일자:   2026-09-03
성격:        문서만 바꾸는 Task (소스 · 설정 · SYSTEM-MAP 변경 없음 · commit 없음)
```

## 1. 이 Task가 바꾼 파일 — 둘뿐이다

| 파일 | 무엇을 했는가 |
| --- | --- |
| `docs/ADR-0003-recording-engine.md` | 머리말 코드 블록의 `Task:`에 TASK-022를 더하고, 그 아래에 §15를 가리키는 주석 한 단락을 넣었다. §14 뒤에 **§15(Phase 2B 추가 기록)** 를 새로 붙였다 — 15.1 소비한 것 · 15.2 `[A?]` → `[A✓]` · 15.3 여전히 `[H]`/`[U]`인 것 · 15.4 그래서 상태는 어떻게 되는가 |
| `docs/ADR-0004-recording-session-lifecycle.md` | 머리말의 `Status` / `Date`를 갱신하고 문서 지도를 넣었다. §14 뒤에 **§15(권한 판정의 VERIFIED/UNVERIFIED) · §16(형식은 절반만 코드가 정한다) · §17(Human Review) · §18(다루지 않는 것)** 을 붙였다 |

## 2. 이 Task가 바꾸지 않은 것

| 대상 | 상태 |
| --- | --- |
| `docs/SYSTEM-MAP.md` | **건드리지 않았다.** Phase 종료 시 운영자가 한다 (Task 지시 · `CLAUDE.local.md`의 System Map 규칙) |
| `src-tauri/**` (Rust 소스 · `Cargo.toml` · `Cargo.lock` · `tauri.conf.json`) | 읽기만 했다 |
| `src/**` · `tests/**` (TypeScript 소스와 테스트) | 읽기만 했다 |
| `.loop/**` | `.loop/evidence/TASK-022/` 아래에만 썼다. Task 파일 · policy · project.yaml · KERNEL은 읽기만 했다 |
| git | **commit하지 않았다.** 이 Run은 git 명령을 실행할 수 없었다 (§3) |
| 다른 ADR (`ADR-0001` · `0002` · `0005` · `0006`) | 읽기만 했다 |

## 3. 이 Run의 실행 제약

- 이 Run에서 실행이 허용된 명령은 `node tools/loop-runtime/loopctl.mjs self-check [<gate> ...]`
  **하나뿐**이었다. `git` · `ls` · `cargo` · `npm`을 실행하지 않았다.
- `.loop/tasks/TASK-022.yaml`의 `stop_condition.gates`가 **비어 있으므로** Gate를 실행하지
  않았다. 문서만 바꾸는 Task이며, Gate는 Runtime이 Worker 종료 후 독립적으로 다시 돌린다.
- 그래서 이 Task가 문서에 적은 `[A✓]`는 **저장소 파일을 직접 읽어 확인했다**는 뜻이며,
  "build/lint/test가 통과했다"는 뜻이 아니다. 그 구분을 ADR-0003 §15의 머리말에 그대로 적었다.
- 확인에 쓴 파일 목록과 대조 결과는 `doc-code-crosscheck.md`에 있다.
