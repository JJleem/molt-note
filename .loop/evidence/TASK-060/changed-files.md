# TASK-060 — 이 Task가 바꾼 파일

```text
src/screens/aiProviderSettings.ts        표현 규칙 추가 (순수 모듈)      +124
src/screens/aiProviderSettings.test.ts   그 규칙에 대한 테스트 6개       +107
src/screens/SettingsScreen.tsx           AI 구역 재배치 (그리기만)   +139 −101
src/App.css                              .group__part · .group__subtitle (985–996행)
```

전체 diff: `diff-ai-section.patch` (tsx · ts · test.ts 세 파일).
App.css의 큰 diff는 이 Phase의 앞선 Task(TASK-055 UI foundation)의 것이며, 이 Task가 더한
것은 위 두 규칙뿐이다.

## 앞선 Run에 대한 사실

이 변경은 **중단된 앞선 Run(2026-09-04)이 만든 것**이고 그 Run은 `worker-result.json`을
남기지 못했다. 이 Run은 그것을 그대로 두고 다시 검증했다 — 코드는 바꾸지 않았고, Gate를
다시 돌렸으며(`gates.md`), AC-4 · AC-5 · AC-6을 다시 확인했다(`boundary.md`).

## src-tauri는 하나도 바뀌지 않았다 (AC-4)

`git status --short -- src-tauri/src/ai/`와 `git diff --stat -- src-tauri/src/ai/` 둘 다
출력이 없다. 작업 트리에 남아 있는 `src-tauri/**` 수정은 이 Phase의 앞선 Task(TASK-056~059)가
만든 것이며 `ai/` 아래에는 하나도 없다:

```text
 M src-tauri/src/commands/export.rs      M src-tauri/src/export/mod.rs
 M src-tauri/src/commands/mod.rs         M src-tauri/src/export/run.rs
 M src-tauri/src/commands/payload.rs     M src-tauri/src/lib.rs
 M src-tauri/src/export/filename.rs     ?? src-tauri/src/export/ai_request.rs
 M src-tauri/src/export/markdown.rs     ?? src-tauri/src/export/handoff.rs
                                        ?? src-tauri/tests/manual_ai_handoff.rs
                                        ?? src-tauri/tests/manual_handoff_invariants.rs
```

`src-tauri/src/ai/`의 파일 목록은 그대로다 —
`mod.rs · note.rs · prompt.rs · provider.rs · run.rs · testing.rs · ollama/{http,mod,network,provider,testing,wire}.rs`.
삭제도 축소도 없고 `tests/ollama_adapter.rs`가 계속 통과한다 (`gates.md`).
