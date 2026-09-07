# TASK-066 — 무엇이 바뀌었는가 (재실행 가능한 확인)

```text
Run    RUN-20260906T005147Z-TASK-066
날짜   2026-09-06
```

## 1. 변경된 파일 — 하나뿐이다

```text
$ git status --porcelain
 M docs/ADR-0007-transcription-engine.md
?? .loop/tasks/TASK-066.yaml
?? .loop/tasks/TASK-067.yaml
?? .loop/tasks/TASK-068.yaml
?? .loop/tasks/TASK-069.yaml
?? .loop/tasks/TASK-070.yaml
?? .loop/tasks/TASK-071.yaml
?? .loop/tasks/TASK-072.yaml
?? .loop/tasks/TASK-073.yaml
?? .loop/tasks/TASK-074.yaml
?? .loop/tasks/TASK-075.yaml
```

`??` 열 개는 **Plan 승인 시점에 Runtime이 생성한 Task 파일**이다. 이 Run이 만들지도
고치지도 않았다 (KERNEL §2 · §6). Run 시작 시점의 git status에도 이미 있었다.

## 2. 변경 규모

```text
$ git diff --numstat
336     12      docs/ADR-0007-transcription-engine.md
```

**336줄 추가 · 12줄 삭제.** 삭제된 12줄은 전부 *그 자리에서 다시 쓴 줄*이며 사라진 문단은
없다 — 내역은 `ac-map.md`의 AC3 표에 줄 수까지 대응시켜 두었다.

```text
5줄  문서 상단 Status / Date / Phase / Task / Scope 둘째 줄  → 이력을 덧붙여 재작성
3줄  §14 표의 3개 행                                        → 원문을 취소선으로 남기고 갱신 덧붙임
4줄  §16.3 표의 4개 행                                      → 같은 방식
────
12줄
```

## 3. `docs/` 밖은 하나도 바뀌지 않았다 (AC4)

```text
$ git diff --numstat -- src-tauri src tests package.json package-lock.json .loop/project.yaml
(출력 없음)
```

즉 `src-tauri/src/transcription/whisper.rs` · `src-tauri/Cargo.toml` ·
`src-tauri/Cargo.lock` · frontend 소스 · 테스트 · Gate 설정 **전부 그대로다.**

## 4. 절 구조 — §1~§16은 자리도 그대로다 (AC3)

```text
$ grep -n "^## " docs/ADR-0007-transcription-engine.md
28:## 1. Context
54:## 2. Decision
88:## 3. 근거의 종류 — 이 Run이 확인할 수 있었던 범위
108:## 4. 후보 비교 — §14.4.2의 세 제약 아래에서
192:## 5. 릴리스 아티팩트 — 종류를 뭉뚱그리지 않는다
222:## 6. tauri-apps/tauri#11992 — 관찰된 packaging 위험
248:## 7. 엔진 확보 경로 — 사용자는 무엇도 설치하지 않는다
272:## 8. 모델 관리
314:## 9. 입력 포맷 변환 책임 — 그리고 원본을 건드리지 않는다는 규칙
366:## 10. timestamp — 실제 단위와, 정규화하는 단 한 곳
399:## 11. Windows에서 성립하는가 — 그리고 하지 않는 것
433:## 12. 탈락한 후보와 탈락 이유
474:## 13. 이 결정이 틀렸을 때 — 되돌리기 비용을 작게 유지한다
499:## 14. 확인한 것 / 확인하지 못한 것
530:## 15. 결과
553:## 16. 구현 결과 — 실제로 만들어진 것 / 달라진 것 / 여전히 모르는 것
655:## 17. 첫 실사용이 만든 결정 — 전사 언어와 가속(Metal)     ← 이 Run이 추가
```

§4.3(175행)과 §16.3.1(619행)은 **본문 한 글자도 바뀌지 않았다.** §16.3.1 뒤에는
§17.3으로 가는 안내 blockquote만 덧붙었다.

## 5. 제3자가 다시 확인하는 방법

```bash
git status --porcelain
git diff --numstat
git diff --numstat -- src-tauri src tests package.json package-lock.json .loop/project.yaml
git diff docs/ADR-0007-transcription-engine.md
grep -n "^## " docs/ADR-0007-transcription-engine.md
```

## 6. Gate

이 Task의 `stop_condition.gates`는 비어 있다 (`.loop/tasks/TASK-066.yaml`의 `gates: []`).
**Gate 명령을 실행하지 않았다.** 변경이 markdown 한 개이고 build / lint / test가 읽는
파일을 건드리지 않았다는 것은 §3의 명령으로 확인된다.
