# TASK-036 — 이 Run이 바꾼 파일

Run: RUN-20260903T064637Z-TASK-036 · 2026-09-03

## 새로 만든 것

```text
src-tauri/src/ai/ollama/mod.rs        adapter 색인 — 무엇이 이 디렉터리 밖으로 나가지 않는가
src-tauri/src/ai/ollama/http.rs       실제 네트워크에 닿는 얇은 경계(trait) + 요청/응답/실패 타입
src-tauri/src/ai/ollama/network.rs    그 경계의 실제 구현 — 저장소에서 소켓을 여는 유일한 파일
src-tauri/src/ai/ollama/wire.rs       벤더 지식 — 엔드포인트 · 요청 필드 이름 · 응답 해석 (순수 함수)
src-tauri/src/ai/ollama/provider.rs   NoteAiProvider 이행 · 벤더 실패 → §13의 domain 공통 실패
src-tauri/src/ai/ollama/testing.rs    HTTP 경계의 결정론적 test double (StubServer)
src-tauri/tests/ollama_adapter.rs     계약 묶음 공유 · 실패 매핑 · 설정값 비노출 · 경계 스캔
```

## 고친 것

```text
src-tauri/Cargo.toml    ureq 3.4 (default-features = false) 추가 + 선택 이유 주석 (ADR-0008 §12.2)
src-tauri/Cargo.lock    위 의존성 해석 결과 (버전 · checksum 고정)
src-tauri/src/ai/mod.rs adapter 마운트 한 줄(`pub mod ollama;`)과 그 사실을 적은 주석
docs/ADR-0008-note-ai-provider.md   §17 — 구현이 계획에서 달라진 점 (문서가 스스로 요구한 절)
```

## 건드리지 않은 것

`domain/**` · `db/**` · `commands/**` · `transcription/**` · `src/**`(frontend) ·
`ai/{note,prompt,provider,testing}.rs` — 이 Task는 adapter를 더할 뿐 기존 경계를 고치지 않는다.
Runtime State(`.loop/tasks/**` · `.loop/policies/**` · `.loop/project.yaml`)도 건드리지 않았다.
