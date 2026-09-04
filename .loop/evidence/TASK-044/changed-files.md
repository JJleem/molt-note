# TASK-044 — 바뀐 파일

```text
A  src-tauri/src/export/mod.rs        경계 문서 + 두 모듈이 공유하는 iso_date (private)
A  src-tauri/src/export/markdown.rs   §11 Markdown 렌더러 (순수)
A  src-tauri/src/export/filename.rs   ADR-0009 §4.2 파일명 정규화 (순수)
M  src-tauri/src/lib.rs               `pub mod export;` 한 줄
```

그 밖의 파일은 건드리지 않았다 — Cargo.toml · package.json · 기존 테스트 · 다른 Task의
Task 파일 · `.loop/` 아래 evidence 밖의 어떤 것도 바뀌지 않았다. 새 의존성도 없다.

## 두 모듈이 import하는 것 전부 (P2-AC3의 근거)

```text
export/mod.rs        (없음 — 표준 라이브러리조차 부르지 않는다)
export/markdown.rs   crate::ai::note::StructuredNote
                     crate::domain::{format_duration_ms, Recording, Transcript}
                     super::iso_date
export/filename.rs   super::iso_date
```

`std::fs` · `std::path` · `std::time` · `SystemTime` · `rusqlite` · `ureq` · `tauri` 가
세 파일 어디에도 없다. 저장소(`crate::db`)도, 제공자(`crate::ai::ollama`)도, 전사
엔진도 부르지 않는다. 그래서 실제 파일 하나 만들지 않고 전부 검증된다 (§18).

테스트 안에서도 파일을 만들지 않는다 — `tempfile`도 `std::fs`도 쓰지 않고, 값만 비교한다.
