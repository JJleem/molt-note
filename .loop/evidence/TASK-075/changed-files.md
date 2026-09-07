# TASK-075 — 이 Run이 바꾼 파일

Run: `RUN-20260907T065204Z-TASK-075` (Attempt 1) → `RUN-20260907T072028Z-TASK-075` (Attempt 2) ·
2026-09-07 · role `impl` (문서 전용 Task)

> **Attempt 2가 바꾼 것은 AC4 한 항목이다.** Verifier가 *"목격되지 않은 실행에 근거한 새 수치"* 로
> 지적한 자리 넷(`PHASE-5.6-HUMAN-REVIEW.md` §0.1 · §2.5 · §7 · `SYSTEM-MAP.md` §5 · §6)에서
> **수치의 근거를 Runtime이 스스로 남긴 TASK-074의 gate 로그로 옮기고**, 그 자리마다
> *"이 문서를 쓴 Run의 실행이 아니다"* 를 명시했다. 절차와 원자료 경로는 `gate-provenance.md`.
> **Attempt 1이 만든 나머지 문서 내용은 그대로다** — 파일 목록도 §1과 같다.

## 1. 이 Run이 만들거나 고친 파일 — **전부 `docs/` 아래다**

```text
수정
  docs/ADR-0007-transcription-engine.md     머리말 · 읽기 안내 · §14의 두 줄 · §16.3의 한 줄 · §19 신설
  docs/ADR-0009-notion-and-export.md        머리말 · 읽기 안내 · §16 신설
  docs/ADR-0010-manual-ai-handoff.md        머리말 · §12.8 신설
  docs/SYSTEM-MAP.md                        머리말 · 갱신 이력 · §1 · §2 · §3 · §4 · §5 · §6 · §7 · §8 · §9

신규
  docs/PHASE-5.6-HUMAN-REVIEW.md            Human Review 절차 §0~§9 + 빈 기록표 §8
```

**`docs/` 밖의 파일은 하나도 바꾸지 않았다** (AC5). 소스 · 설정 · 의존성 · 테스트 ·
`Cargo.toml` · `Cargo.lock` · `package.json` · `.loop/**`(evidence 제외) 전부 그대로다.

## 2. `git status`가 다른 파일도 M으로 보이는 이유

이 저장소의 working tree에는 **Phase 5.6과 Phase 5.7의 커밋되지 않은 변경이 함께 있다**
(TASK-066 ~ TASK-085). 그래서 `git status`에는 `src-tauri/**` · `src/**` · `tests/**` ·
`docs/ADR-0003` · `docs/LOOP-RUNTIME-FIELD-NOTES.md` · `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` ·
`docs/PHASE-5.7-HUMAN-REVIEW.md`가 함께 보인다.

**그것들은 앞선 Task들의 산출물이며 이 Run이 건드리지 않았다.** 이 Run이 손댄 자리는 §1의
다섯 파일뿐이다.

## 3. 지운 문단이 없다 (AC1 · AC3)

세 ADR 모두 **덧붙이는 형태**로 갱신했다.

| 파일 | 기존 기록을 어떻게 다뤘는가 |
| --- | --- |
| `ADR-0007` | **§1~§18을 고쳐 쓰지 않았다.** §19를 뒤에 새로 붙였다. §14 · §16.3의 두 항목은 원래 문장을 `~~취소선~~`으로 남기고 그 뒤에 갱신을 이어 붙였다 — 이 문서가 §17에서 이미 쓰던 방식 그대로다 |
| `ADR-0009` | **§1~§15를 고쳐 쓰지 않았다.** §16을 뒤에 새로 붙였다. §4.1이 세운 전제가 부족했다는 것도 §4를 고치는 대신 §16.1이 인용해 적는다 |
| `ADR-0010` | **§1~§12.7을 고쳐 쓰지 않았다.** §12.8을 §12.7 뒤에 붙였다 |
| `SYSTEM-MAP` | 머리말의 "Phase 5.6은 부분 완료다" 서술은 **지우지 않고** 그때의 사실로 표시한 뒤 오늘의 사실을 이어 적었다. 갱신 이력에도 Phase 5.7 줄을 그대로 두고 새 줄을 더했다. §5의 Phase 5.7 절과 그 안의 자동 테스트 수(1,405)도 그대로 두었다 — 그것은 그 Phase의 기록이다 |

세 머리말(Status · Date · Phase · Task · Scope)은 **원래 줄을 지우지 않고 이번 갱신을
덧붙이는 형태**로 고쳤다.
