# TASK-031 — 문서만 바뀌었다는 증거 (AC5)

```text
Run:   RUN-20260903T051135Z-TASK-031
작업 구간: 2026-09-03 14:19 ~ 14:23 (+0900)
```

## 1. mtime — 이 Run 구간에 바뀐 파일은 `docs/` 둘뿐이다

`git status`만으로는 이 Run이 무엇을 바꿨는지 알 수 없다. Phase 3의 앞선 Task들이 만든
변경이 아직 커밋되지 않은 채 남아 있기 때문이다 (`docs/GIT-WORKFLOW.md` — Phase commit은
운영자가 한다). 그래서 **수정 시각**으로 가른다.

```console
$ date "+%Y-%m-%dT%H:%M:%S%z"
2026-09-03T14:23:03+0900

$ ls -lT docs/ADR-0007-transcription-engine.md docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md \
         docs/SYSTEM-MAP.md src-tauri/tauri.conf.json src-tauri/Cargo.toml package.json \
         src-tauri/src/transcription/whisper.rs src/screens/transcriptView.ts
-rw-r--r--@ 1 molt  staff  46020 Sep  3 14:22:44 2026 docs/ADR-0007-transcription-engine.md
-rw-r--r--@ 1 molt  staff  27466 Sep  3 14:20:20 2026 docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md
-rw-r--r--@ 1 molt  staff  25492 Sep  3 10:08:11 2026 docs/SYSTEM-MAP.md
-rw-r--r--@ 1 molt  staff   1121 Sep  2 14:19:47 2026 package.json
-rw-r--r--@ 1 molt  staff   3313 Sep  3 12:20:35 2026 src-tauri/Cargo.toml
-rw-r--r--@ 1 molt  staff  10318 Sep  3 12:27:24 2026 src-tauri/src/transcription/whisper.rs
-rw-r--r--@ 1 molt  staff    771 Sep  2 19:15:25 2026 src-tauri/tauri.conf.json
-rw-r--r--@ 1 molt  staff  18323 Sep  3 14:03:37 2026 src/screens/transcriptView.ts
```

읽는 법:

| 파일 | mtime | 이 Run이 바꿨는가 |
| --- | --- | --- |
| `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` | **14:20:20** | **예 — 이 Run이 만들었다** |
| `docs/ADR-0007-transcription-engine.md` | **14:22:44** | **예 — 이 Run이 고쳤다** |
| `docs/SYSTEM-MAP.md` | 10:08:11 | 아니다 (이 Run 시작 전) |
| `package.json` | 09-02 14:19 | 아니다 |
| `src-tauri/Cargo.toml` | 12:20:35 | 아니다 (TASK-026) |
| `src-tauri/tauri.conf.json` | 09-02 19:15 | 아니다 |
| `src-tauri/src/transcription/whisper.rs` | 12:27:24 | 아니다 (TASK-026) |
| `src/screens/transcriptView.ts` | 14:03:37 | 아니다 (TASK-030) |

## 2. `git status --porcelain` (참고 — 이 Run 시작 시점과 종료 시점의 목록이 같다)

이 Run이 추가한 항목은 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 하나이며(`??`),
`docs/ADR-0007-transcription-engine.md`는 TASK-023이 만든 뒤 아직 커밋되지 않아
이 Run의 수정이 새 항목을 만들지 않는다. 그 밖의 `M`/`??` 항목은 전부
TASK-023~030이 남긴 것이며 **이 Run은 그중 어느 것도 열어서 고치지 않았다** (§1의 mtime).

## 3. 이 Run이 실행한 쓰기 작업 전부

```text
Write  docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md
Edit   docs/ADR-0007-transcription-engine.md   (8회 — 헤더 · §4.3 · §5 · §9.2 · §8.2.4 · §14 두 곳 · §16 추가)
Write  .loop/evidence/TASK-031/README.md
Write  .loop/evidence/TASK-031/changed-files.md
Write  .loop/evidence/TASK-031/doc-only-check.md
```

그 밖의 모든 도구 사용은 읽기(Read · grep · ls)였다.
