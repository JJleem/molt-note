# TASK-055 — Acceptance Criteria ↔ 문서 위치 대응

대상 문서: `docs/ADR-0010-manual-ai-handoff.md`
RUN-20260904T080544Z-TASK-055 · 2026-09-04

이 표는 **주장이 아니라 위치**다. 각 AC가 문서의 어느 절에서 판정되는지만 가리킨다.

| AC | 요구 | 문서 위치 |
| --- | --- | --- |
| AC-1 | 새 문서 하나만 생겼고 다른 파일은 하나도 안 바뀌었다 | `.loop/evidence/TASK-055/changed-files.txt` (git status 대조) |
| AC-2 | 두 방향 변경 · 날짜 · 근거 · 로드맵에 없던 삽입 · Ollama는 재배치 | §4 전체 — §4.1(삽입 사실 · 2026-09-04) · §4.2(변경 1 · D-1) · §4.3(변경 2 · D-2 · "삭제가 아니다" 행) · §4.4(Spec 본문은 P11이 갱신) · §2 요약표 (1) |
| AC-3 | 산출물 섹션 구성 · 순서 · timestamp 표현 확정 + `export::Document` 재사용 + 병렬 export 없음 | §5.1(재료 둘) · §5.2(섹션 순서 확정표 + 예시 문서) · §5.3(timestamp = `format_timestamp_ms` 재사용) · §5.4(`ExportDocument` 재사용 · private → `pub(crate)` 끌어올리기 · `render` 바이트 불변) · §5.5 · §5.6(파일 이름은 `export_file_name` 재사용) |
| AC-4 | 프롬프트 상수를 고치지 않는 재사용 방식 + promptVersion/provenance 근거 + 두 번째 세트 없음 | §6.1(고칠 수 없는 이유 — 선언값·계산값 결속과 저장된 provenance) · §6.2(치환 두 번 · 치환 1회 단언) · §6.3(Markdown 출력 계약 다섯 요구) · §6.4(promptVersion을 만들지도 저장하지도 않는다 · 두 번째 세트 없음) · §6.5(파서 없음) |
| AC-5 | clipboard 경계 확정 + 후보 비교와 탈락 근거 + UNVERIFIED 표시 + Export for AI 대체 경로 | §7.1(현황) · §7.2(결정: `src/platform/clipboard.ts` · 새 FailureKind 없음) · §7.3(후보 A/B/C 비교표와 탈락 근거) · §7.4(UNVERIFIED 여섯 · [E4]) · §7.5(실패 표시 · 재시도 · Export for AI가 남는다) |
| AC-6 | command 이름·개수 + MH-1~MH-8 판정 대응표 + 벤더 이름 배제 결정 | §8.1(세 이름 · 28 → 31) · §8.2(`transcriptId`를 받지 않는다) · §8.3(`tests/ipc-boundary.test.ts` 네 자리 tripwire 예고 + 깨지지 않는 자리) · §8.4(MH-1~MH-8 판정표) · §10(벤더 중립 결정) |

## 참고 — 문서가 스스로 표시한 근거 등급

§3이 [E1] 직접 확인 / [E4] UNVERIFIED / [A] 앱이 고른 값 세 표기를 쓴다
(ADR-0009 §3의 표기를 이어받되, 이 Run이 실제로 쓴 셋만 남겼다).

- [E1] 목록: §3의 코드 블록 (이 Run이 읽은 파일과 확인한 값)
- [E4] 목록: §7.4 (clipboard 관련 여섯 — 지어내지 않았다)
- [A]: §5.6의 파일 이름 표식 여유 계산
