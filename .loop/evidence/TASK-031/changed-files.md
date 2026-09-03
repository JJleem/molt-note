# TASK-031 — 이 Run이 바꾼 파일

```text
Run:  RUN-20260903T051135Z-TASK-031
Date: 2026-09-03
```

## 새로 만든 것

```text
docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md      운영자 smoke test 절차(§1~§9) + 검증 기록표(§10~§11)
.loop/evidence/TASK-031/README.md             이 Run의 기록
.loop/evidence/TASK-031/changed-files.md      이 파일
.loop/evidence/TASK-031/doc-only-check.md     문서만 바뀌었다는 증거
```

## 고친 것

```text
docs/ADR-0007-transcription-engine.md
  - 헤더 Status / Date / Task — 구현 결과 반영, §16과 smoke test 문서로 안내
  - §4.3   Gate 비용 [E4] → 관측값 27.7초 (TASK-026 evidence)
  - §5     API 표면 [E4] → 컴파일러 확인 / 번들 버전은 UNVERIFIED로 분리
  - §8.2.4 engine provenance 실제 값(`whisper-rs/0.16`)과 그 이유
  - §9.2   rubato 5.0.0의 실제 타입 이름
  - §14    표 갱신 — API 시그니처 VERIFIED · 번들 버전/단위/가속 UNVERIFIED ·
           cold build 관측값 · end-to-end 추론은 NOT RUN
  - §16    새 절: 구현 결과 · 계획과 달라진 것과 이유 · 남은 UNVERIFIED · 되돌리기 경로
```

## 건드리지 않은 것 (AC5)

```text
src-tauri/**              소스 · 테스트
src/**                    소스 · 테스트
src-tauri/tauri.conf.json
src-tauri/Cargo.toml · Cargo.lock
package.json · package-lock.json
docs/SYSTEM-MAP.md        Phase 최종 DONE 뒤 운영자가 갱신한다
.loop/tasks/** · .loop/policies/** · .loop/project.yaml · .loop/KERNEL.md · .loop/DESIGN.md
```
