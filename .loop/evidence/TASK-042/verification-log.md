# TASK-042 — 대조 기록 (문서 ↔ 저장소 코드)

```text
Run    : RUN-20260904T015011Z-TASK-042
Task   : TASK-042 (문서 전용)
Date   : 2026-09-04
방법   : 이 Run이 저장소의 소스 파일을 직접 읽어 ADR-0008 §1~§16의 각 결정과 대조했다.
         앱 실행 · 네트워크 접근 · Ollama 호출은 하지 않았다.
```

## 1. 대조한 파일 (전부 읽기만 했다)

```text
src-tauri/src/ai/mod.rs
src-tauri/src/ai/note.rs
src-tauri/src/ai/prompt.rs
src-tauri/src/ai/provider.rs
src-tauri/src/ai/run.rs
src-tauri/src/ai/ollama/http.rs
src-tauri/src/ai/ollama/network.rs
src-tauri/src/ai/ollama/provider.rs
src-tauri/src/ai/ollama/wire.rs
src-tauri/src/commands/mod.rs
src-tauri/src/commands/notes.rs
src-tauri/src/db/migrations.rs
src-tauri/src/domain/failure.rs
src-tauri/src/domain/settings.rs
src-tauri/Cargo.toml · Cargo.lock
src/ipc/failure.ts
src/screens/aiNoteView.ts
src/screens/aiProviderSettings.ts
src/screens/RecordingDetailScreen.tsx
src/screens/SettingsScreen.tsx
```

## 2. 결과 요약

| 결정 항목 | 판정 | 근거 |
| --- | --- | --- |
| 최종 structured note schema (§7.2) | 일치 | `ai/note.rs` — 13 필드, 이름·순서·상한 동일 |
| 저장 봉투와 읽기 규칙 (§7.5) | 일치 | `ai/note.rs::encode_content` · `decode_content` |
| 구조화 출력 수단 (§6) | 일치 | `ai/ollama/wire.rs::generate_body` — `format` = `json_schema(mode)`, `stream:false` |
| 호출 주체 (§5) | 일치 | 소켓을 여는 파일은 `ai/ollama/network.rs` 하나 |
| context 전략 (§8) | **부분 불일치** | `num_ctx` 명시 · 무절단 · 사전 판정은 있음. **설정 항목 `ai_context_tokens`는 없음** |
| 재생성 정책 (§9) | 일치 | `ai/run.rs`는 `store::insert_ai_note`만 부른다 |
| promptVersion 정책 (§10) | 일치 | `ai/prompt.rs` — 선언 상수 + `prompt_version_is_bound_to_the_prompt_text` |
| 실패 매핑 (§13) | 일치 (+사용처 하나 추가) | `domain/failure.rs` 여섯 · `src/ipc/failure.ts` 여섯 |
| §9.2 예시로부터의 여섯 이탈 (§4.2) | 일치, 이유 기록됨 | `ai/provider.rs`가 여섯을 그대로 구현 |

## 3. 확인된 불일치 — 문서를 코드에 맞췄다

### 3.1 `ai_context_tokens` 미구현

```text
migrations.rs (version 6, name="add_ai_provider_settings")
  ALTER TABLE settings ADD COLUMN ai_provider TEXT;
  ALTER TABLE settings ADD COLUMN ai_base_url TEXT;
  ALTER TABLE settings ADD COLUMN ai_model  TEXT;
  → 넷째 열 ai_context_tokens 없음

domain/settings.rs::Settings
  → ai_context_tokens 필드 없음

commands/notes.rs::generate_one
  → run::generate(..., ContextBudget::DEFAULT)   // 언제나 16384 고정

grep 결과: 저장소 전체에 ai_context_tokens 문자열이 존재하지 않는다.
```

파생 사실(상수에서 계산): 입력 예산 = 16384 − 1024 − 1536 = 13,824 토큰,
추정식 `문자수 ÷ 2` → 약 27,648자 초과 transcript는 요청이 나가지 않는다.
**실제로 걸리는지는 실행해 본 적이 없으므로 UNVERIFIED다.**

기록 위치: ADR-0008 §2(e) · §2(h) · §8.2 · §8.3 · §8.4 · §11.1 · §13.2 · §16.2에
경고 표시, 사실은 §17.3.2. 절차 문서는 `docs/PHASE-4-AI-NOTE-REVIEW.md` §7.

### 3.2 기본 주소를 아는 자리

§11.1은 "adapter의 기본 주소"라고 적었으나, 실제로는
`domain/settings.rs::DEFAULT_AI_BASE_URL`("http://localhost:11434") 한 곳이며 adapter에는
주소가 없다 (`ai/ollama/provider.rs`의 `base_url`은 생성자 인자). → ADR-0008 §17.3.3.

### 3.3 `AiProviderNotConfigured`의 두 번째 사용처

`commands/notes.rs::Providers::to_generate_with` — `ai_provider` 미선택과 **`ai_model`
미선택** 둘 다 이 실패다. §13.1의 표에는 후자가 없었다. → ADR-0008 §17.3.4.

### 3.4 결정에 없던 구현 사실

- 연결 타임아웃 5초 · 생성에는 시간 제한 없음 (`ai/ollama/network.rs::CONNECT_TIMEOUT`)
- 생성 중 화면 폴링 2초 (`RecordingDetailScreen.tsx::AI_NOTE_REFRESH_MS`)
- 한 번에 한 건, 동시 시작 거절 (`commands/notes.rs::already_running`)
- 연결 확인은 저장된 설정을 묻는다 (`aiProviderSettings.ts::AI_CHECK_USES_SAVED_SETTINGS`)

→ ADR-0008 §17.3.5.

## 4. UNVERIFIED로 남긴 것

```text
실제 Ollama 호출          : 이 Phase에서 한 번도 일어나지 않았다 [E4]
실제 Whisper 추론         : A-TRANS-001 여전히 유효 [E4]
§14.5 값의 오늘 기준 재확인: 미이행 — 등급은 [E2] · 확인 시점 2026-09-01 그대로
노트 품질 · 처리 시간 · 한/영 혼용 : 측정한 적 없음 — 수치를 적지 않았다
```

이 Run은 문서 전용 Task이므로 네트워크 도구를 시도하지 않았다.

## 5. 이 Run이 실행한 명령

없다. 파일 읽기와 `git status` / `git log` 조회뿐이다.
Task의 `stop_condition.gates`가 `(none enabled)`이므로 Gate를 실행하지 않았고,
소스·설정·테스트를 바꾸지 않았으므로 Gate 결과가 달라질 수 있는 변경도 없다.
