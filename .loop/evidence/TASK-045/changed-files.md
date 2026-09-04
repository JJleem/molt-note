# TASK-045 — 실제로 손댄 파일

`git status --short` · `git diff --stat` (2026-09-04, Worker 종료 시점)

## 새로 만든 파일

```text
src-tauri/src/export/file.rs          Markdown 문자열 → 파일 하나 (덮어쓰지 않는다)
src-tauri/src/export/run.rs           저장소 → 렌더러 → 파일 실행 순서
src-tauri/src/commands/export.rs      Exporter (앱이 들고 있는 실행자)
src-tauri/tests/markdown_export.rs    통합 테스트 12개
```

`src-tauri/src/export/`는 TASK-044가 만든 디렉터리라 git에는 아직 untracked로 잡힌다
(`?? src-tauri/src/export/`). 이 Task가 그 안에 더한 것은 `file.rs` · `run.rs`이며,
`mod.rs`의 모듈 문서와 `pub mod` 두 줄도 함께 고쳤다.

## 고친 파일 (`git diff --stat`)

```text
 src-tauri/src/commands/mod.rs          | 40 ++++++++++++---     export_markdown command · Exporter 재수출
 src-tauri/src/commands/payload.rs      | 35 +++++++++++++       ExportedFilePayload
 src-tauri/src/lib.rs                   | 16 +++++-             Exporter 등록 · command 등록
 src-tauri/src/platform/app_data_dir.rs | 92 ++++++++++++++++    exports_dir · ensure_exports_dir + 테스트 3
 src/ipc/commands.ts                    | 28 ++++++++++-        exportMarkdown()
 src/ipc/types.ts                       | 20 ++++++++          ExportedFile
 tests/ipc-boundary.test.ts             | 25 +++++++--         등록 목록 · export 표면 검사
 7 files changed, 244 insertions(+), 12 deletions(-)
```

전체 diff: `tracked-files.diff` (같은 디렉터리).
`src-tauri/src/export/mod.rs`는 위 diff에 나타나지 않는다 — 파일 전체가 아직 untracked이기
때문이며, 내용은 작업 트리에서 직접 볼 수 있다.

## 손대지 않은 것

- `.loop/` 아래 Task 파일 · 정책 · KERNEL (읽기만 했다)
- 다른 Task의 evidence
- 기존 테스트의 삭제·skip·완화 — 없다. `tests/ipc-boundary.test.ts`는 **추가**다:
  등록 목록에 `export_markdown` 한 줄이 늘었고, "아직 만들지 않은 기능" 정규식에서 `export`가
  `export(?!_markdown\b)`로 좁아지면서 `pdf`·`docx`가 새로 막혔다. 검사는 하나 늘었다
  (export 표면이 이름 하나뿐이라는 것).
