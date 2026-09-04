# ADR-0008 — Note AI Provider 경계는 Rust에 있고, 벤더 지식은 adapter 하나에만 있다

```text
Status:   Accepted (결정) · **구현됨, 실제 provider 호출은 아직 없다**
          §1~§16의 결정은 TASK-033 ~ TASK-041이 구현했고, 구현 결과와의 대조는 §17에 있다.
          **이 Phase에서 실제 Ollama를 호출한 적은 한 번도 없다** (§17.3.6).
Date:     2026-09-03 (결정) · 2026-09-04 (구현 대조 — §17.3)
Phase:    Phase 4 — AI Provider System + Local AI
Task:     TASK-032 (결정) · TASK-033~041 (구현) · TASK-042 (대조)
Scope:    NoteAIProvider 계약 · 호출 주체 · 구조화 출력 수단과 방어 경로 ·
          structured note 최종 schema · context window 전략 · AINote 재생성 정책 ·
          promptVersion 정책 · provider 설정 저장 위치 · HTTP 클라이언트 ·
          AI Provider 실패의 domain 매핑
```

> **§1~§16은 결정이고 §17은 보고다.** 결정 절을 사후에 고쳐 쓰지 않는다 — 그것을 고치면
> 무엇을 정했고 무엇이 실제로 만들어졌는지가 구분되지 않는다 (ADR-0007 §16과 같은 방식).
> 대신 구현이 결정과 달라진 자리에는 **그 자리에서 §17로 보내는 표시**를 달았다.
> **결정만 있고 구현되지 않은 항목이 구현된 것처럼 읽히지 않게 하는 것**이 그 표시의 목적이다.

---

## 1. Context

Phase 4는 Phase 3가 만든 Transcript를 **사람이 읽기 좋은 structured note**로 바꾼다
(`phase-prompt/04-ai-provider-system.md` · PRODUCT-SPEC §9).

```text
Transcript  →  Note AI Provider  →  Structured Note
                      ↑
              첫 구현: 사용자가 실행 중인 로컬 Ollama
```

그 전에 되돌리기 어려운 결정이 열 개 있다. 이 문서가 그 열 개를 전부 확정한다.
전부 하나의 경계에 관한 것이다.

```text
Molt Note domain   ≠   특정 AI 벤더의 domain      (INV-9)
```

벤더는 바뀐다. 바뀔 때 흔들리는 것이 **adapter 하나**여야지 제품 전체여서는 안 된다.
그래서 이 문서의 모든 결정은 같은 질문에 답한다 — **이 사실이 틀리거나 바뀌면 무엇이 함께
무너지는가.** 무너지는 범위가 adapter 하나로 끝나면 그 설계를 택했다.

전제 하나를 명시한다. **Phase 3는 실제 Whisper 추론을 한 번도 실행한 적이 없다**
(`ASSUMPTION A-TRANS-001` · PRODUCT-SPEC §14.4.4 · ADR-0007 §16.3.1). 따라서 이 Phase에서
실제 Transcript를 전제해야 하는 것은 **결정론적 fixture / mock Transcript**를 쓰며,
이 문서도 실제 전사 결과를 근거로 쓴 문장을 갖지 않는다.

---

## 2. Decision — 열 개 결정 요약

| | 항목 | 결정 |
| --- | --- | --- |
| **(a)** | `NoteAIProvider` 계약 | **Rust trait** `NoteAiProvider`. `isAvailable(): bool`을 3상태 `Availability`로 바꾸고, 실패는 `Result<_, Failure>`(domain 공통 실패 타입)로 돌려준다. 입력 타입에는 **오디오를 가리킬 수 있는 필드가 없다** (§4) |
| **(b)** | 호출 주체 | **Rust backend.** frontend는 Ollama에 직접 접근하지 않는다. Phase 5의 Notion 호출도 같은 경계에서 나간다 (§5) |
| **(c)** | 구조화 출력 수단 | Ollama `format`에 **모드별 JSON Schema**를 실어 보낸다. 단 **정확성은 그것에 의존하지 않는다** — 모든 응답은 신뢰하지 않는 텍스트로 취급해 파싱·검증한다 (§6) |
| **(d)** | structured note schema | 세 모드의 필드 이름 · 타입 · 필수 여부를 §7에서 확정한다. `ai_notes.content`에는 `{schemaVersion, mode, note}` 봉투를 **compact JSON 한 문자열**로 담는다 |
| **(e)** | context window | **`options.num_ctx`를 항상 명시**한다(서버 기본값에 기대지 않는다). 청킹도 조용한 절단도 하지 않고, **보내기 전에 크기를 계산해 넘치면 실패로 보여 준다** (§8). ⚠️ **그 값을 설정으로 노출하는 부분은 구현되지 않았다 — 언제나 16384 고정이다** (§17.3.2) |
| **(f)** | AINote 재생성 | **append-only 이력.** 재생성은 언제나 새 행을 INSERT한다. 기존 노트를 UPDATE·DELETE하지 않는다 — 저장소에 그 경로가 아예 없다 (§9) |
| **(g)** | `promptVersion` | 프롬프트 상수 + 스키마의 **해시를 테스트가 강제**한다. 프롬프트를 고치면 값을 바꾸기 전까지 Gate가 깨진다 (§10) |
| **(h)** | provider 설정 저장 | 기존 단일 행 `settings` 테이블에 **새 migration(version 6)으로 열을 더한다.** secret 열은 만들지 않는다 (INV-7 · §11). ⚠️ 실제로 더해진 열은 **셋**이다 — 넷째(`ai_context_tokens`)는 만들어지지 않았다 (§17.3.2) |
| **(i)** | HTTP 클라이언트 | `ollama-rs`를 쓰지 않고 **REST를 직접** 호출한다. crate는 **`ureq`**(blocking), 차점은 `reqwest`(blocking) (§12) |
| **(j)** | 실패 매핑 | §13의 AI Provider 실패를 **여섯 개 domain `FailureKind`**로 옮긴다. 각각의 `retryable`을 §13에서 확정한다 |

이 Phase에서 **하지 않는 것**은 §15에 따로 적었다 (cloud adapter · Claude Agent SDK ·
Ollama 번들링 · 가격/무료 티어 의존).

---

## 3. 근거의 종류 — 이 Run이 확인할 수 있었던 범위

**추측한 것을 확인한 것처럼 적지 않는다** (PRODUCT-SPEC §20.2). 문서 전체에서 아래 표기를 쓴다.

| 표기 | 뜻 |
| --- | --- |
| **[E1] 직접 확인** | 이 Run에서 이 저장소의 실제 파일을 읽어 확인했다 |
| **[E2] §14.5 (2026-09-01)** | PRODUCT-SPEC §14.5가 **2026-09-01에** primary source(ollama/ollama `docs/api.md` · `docs/faq.mdx` · `server/routes.go`)에서 확인해 기록한 값. **이 Run이 다시 확인한 것이 아니다** |
| **[E3] 2차 출처** | 1차 출처가 아닌 근거로만 확인된 값 |
| **[E4] UNVERIFIED** | 확인하지 못했다. **구현 근거로 쓰지 않는다** |

### 3.1 ⚠️ 이 Run에는 네트워크 접근이 없었다

Task는 §14.5의 값을 **도입 시점에 재확인**하라고 요구한다. 이 Run은 그것을 시도했고 **거부됐다.**

```text
WebFetch  https://raw.githubusercontent.com/ollama/ollama/main/docs/api.md
          → "Claude requested permissions to use WebFetch, but you haven't granted it yet."
WebSearch "Ollama API docs structured outputs format JSON schema /api/generate stream false"
          → "Claude requested permissions to use WebSearch, but you haven't granted it yet."
```

기록: `.loop/evidence/TASK-032/verification-log.md`.

따라서 이 문서는 **§14.5의 어떤 항목도 "2026-09-03에 재확인했다"고 적지 않는다.**
그 항목들의 확인 시점은 전부 **2026-09-01**이며 등급은 [E2]다. ADR-0007의 [E2]는
확인 시점이 결정일과 **같은 날**이었지만, 이 문서의 [E2]는 **이틀 전**이다.
그 차이를 숨기지 않는다.

**이 사실이 설계를 바꿨다.** §14.5의 세부값이 틀렸을 때 무너지는 범위를 줄이는 쪽으로
세 가지를 결정했다.

1. **가용성은 문서가 아니라 실행 중인 서버에게 묻는다** — 모델 목록 호출 하나로 판정한다 (§6.4).
2. **`format` 파라미터에 정확성을 걸지 않는다** — 서버가 무시하거나 거절해도 파싱 경로가 답을 낸다 (§6.2).
3. **모델 존재 여부를 생성 응답의 상태 코드로 판정하지 않는다** — 목록으로 먼저 판정한다 (§13.3).

전체 재확인 결과표는 **§14**에 있다.

### 3.2 이 Run이 저장소에서 직접 읽어 확인한 것 ([E1])

| 확인한 사실 | 파일 |
| --- | --- |
| `ai_notes`는 `id · recording_id · transcript_id · note_type · content · provider · model · prompt_version · generated_at`을 갖고, `content`는 해석되지 않는 `TEXT NOT NULL`이다. `(transcript_id, recording_id)` 복합 FK가 provenance 불일치를 막는다 | `src-tauri/src/db/migrations.rs` (migration 2) |
| 인덱스 `idx_ai_notes_transcript (transcript_id, generated_at)` · `idx_ai_notes_recording (recording_id)`가 이미 있다 | 같음 |
| 적용된 migration은 1~5이며 **다음 번호는 6**이다. 이미 적용된 migration의 version·name·sql을 고치지 못하게 하는 테스트가 있다 | 같음 (`released_migrations_keep_their_version_and_name`) |
| **저장소에 `ai_notes`를 UPDATE·DELETE하는 경로가 없다.** 있는 것은 `insert_ai_note` · `load_ai_note` · `list_ai_notes_for_transcript` 셋뿐이고, `INSERT OR REPLACE`도 쓰지 않는다 | `src-tauri/src/db/store.rs` |
| `domain::AiNote`의 `provider`는 벤더 enum이 아니라 자유 문자열이며, 복원 시 알려진 목록과 대조하지 않는다 (INV-9) | `src-tauri/src/domain/mod.rs` · `store.rs::decode_ai_note` |
| `NoteType {Meeting, Study, Summary}`가 이미 domain에 있고 `meeting/study/summary` 문자열과 왕복한다 | `src-tauri/src/domain/mod.rs` |
| 현재 `FailureKind`는 8종(`Storage` · `InvalidInput` · `AudioDevice` · `MicrophonePermission` · `Transcription*` 4종)이고, **AI provider 실패는 아직 하나도 없다** | `src-tauri/src/domain/failure.rs` |
| frontend `FailureKind` union이 Rust와 1:1이며 `unexpected` 하나만 frontend 전용이다 | `src/ipc/failure.ts` |
| `Settings`에 secret 필드가 없고 `settings` 테이블에도 secret 열이 없다 (INV-7). 기본값 정책은 스키마가 아니라 `Settings::DEFAULT`가 갖는다 | `src-tauri/src/domain/settings.rs` · `migrations.rs` (version 3~5) |
| **의존성에 HTTP 클라이언트가 없다.** `tauri` · `serde` · `serde_json` · `rusqlite` · `cpal` · `hound` · `whisper-rs` · `rubato`뿐이며 async runtime도 없다 | `src-tauri/Cargo.toml` |
| 전사 경계는 **동기 trait + test double** 형태다 (`TranscriptionEngine` / `testing::StubEngine`), 실행 경계와 파싱(`parse.rs`)이 분리돼 있다 | `src-tauri/src/transcription/engine.rs` |

이 문서의 결정 중 저장소 사실에 근거한 것은 전부 위 표에서 왔다.

---

## 4. (a) `NoteAIProvider` 계약의 최종 형태

### 4.1 결정

계약은 **Rust trait**이며 `src-tauri/src/ai/` 아래에 둔다. domain은 벤더를 알지 않는다 (INV-9).

```rust
/// provider가 자기 자신에 대해 말하는 것. 화면이 §12의 구분을 그리는 근거다 (INV-5).
pub struct ProviderDescriptor {
    pub id: String,        // provenance에 그대로 저장된다 (예: "ollama")
    pub name: String,      // 사람이 읽는 이름
    pub locality: Locality,
}

/// 전송되는 데이터가 기기 밖으로 나가는가 (§12 · INV-5).
pub enum Locality { Local, External }

/// provider가 지금 쓸 수 있는 상태인가.
pub enum Availability {
    /// 응답했고 쓸 수 있는 모델이 하나 이상 있다.
    Ready { models: Vec<String> },
    /// 응답했지만 설치된 모델이 하나도 없다 (§13 `모델 없음`).
    NoModels,
    /// 지금 쓸 수 없다. 이미 domain 공통 실패로 번역된 값이다.
    Unavailable(Failure),
}

/// provider에게 넘기는 입력. **오디오를 가리킬 수 있는 필드가 없다** (INV-6).
pub struct NoteRequest<'a> {
    pub mode: NoteType,                       // domain의 meeting|study|summary
    pub transcript: TranscriptText<'a>,       // 텍스트뿐이다
    pub context_budget: ContextBudget,        // §8
}

/// Transcript에서 **텍스트만** 뽑은 값. 경로도 id도 담지 않는다.
pub struct TranscriptText<'a> { pub text: &'a str }

/// provider가 돌려주는 것. 실제로 무엇이 만들었는지는 provider만 안다.
pub struct NoteGeneration {
    pub note: StructuredNote,   // §7
    pub model: String,          // provenance — 실제로 쓰인 모델
}

pub trait NoteAiProvider: Send + Sync {
    fn descriptor(&self) -> ProviderDescriptor;
    fn availability(&self) -> Availability;
    fn generate_note(&self, request: &NoteRequest<'_>) -> Result<NoteGeneration, Failure>;
}
```

### 4.2 §9.2의 예시에서 벗어난 부분과 그 이유

> §9.2는 **의도 설명용**이며 "더 적합한 contract가 있으면 바꿔도 되지만 바꾼 이유를 기록한다.
> 바뀌면 안 되는 것은 계약의 형태가 아니라 INV-9다"라고 명시한다.
> 아래 여섯 가지가 벗어난 부분이고, 여섯 개 전부 **INV-9를 강화하는 방향**이다.

| # | §9.2 | 이 문서 | 왜 |
| --- | --- | --- | --- |
| 1 | TypeScript `interface` | **Rust `trait`** | 호출 주체를 Rust backend로 정했기 때문이다 (§5). 계약은 그것을 이행하는 쪽에 있어야 하며, 프론트에 계약을 두면 벤더 지식이 두 언어에 걸친다 |
| 2 | `isAvailable(): Promise<boolean>` | **`availability() -> Availability`** (3상태) | boolean은 §13이 **서로 다른 제품 상태로 구분하라고 요구한** 세 가지를 한 값으로 뭉갠다 — provider 미설정 · 연결 불가 · 모델 없음. 사용자가 할 수 있는 일이 셋 다 다르다(고르기 · Ollama 실행 · 모델 받기). 화면이 그 셋을 구분해 안내하려면 계약이 먼저 구분해야 한다. 이것은 전사 실패 넷을 뭉치지 않은 것과 같은 규칙이다 (`transcription::engine`) |
| 3 | `Promise<StructuredNote>` (거절은 임의의 에러) | **`Result<NoteGeneration, Failure>`** | 벤더 에러가 계약을 타고 밖으로 나가면 INV-9가 깨진다. adapter가 §13의 domain 공통 실패로 **번역해서만** 나올 수 있게 반환 타입이 강제한다 |
| 4 | `StructuredNote`만 반환 | `note` + **`model`** | provenance(§7.3 · §9.6)는 "무엇이 만들었는가"를 요구하는데, **실제로 어떤 모델이 답했는지는 provider만 안다.** 호출자가 설정값을 그대로 적어 두면 그것은 기록이 아니라 추정이다 |
| 5 | `mode: "meeting"\|"study"\|"summary"` | **`NoteType`** (이미 있는 domain 타입) | 같은 세 값을 세는 타입을 두 개 두지 않는다. `NoteType`은 이미 저장·복원까지 왕복한다 [E1] |
| 6 | `transcript: TranscriptInput` (형태 미지정) | **`TranscriptText { text }`** | INV-6("audio는 전송하지 않는다")을 **코드 수준에서 보장**하라는 요구를 타입으로 답한다. adapter가 받는 값에 오디오 경로·바이트·파일 핸들을 가리킬 필드가 아예 없으므로, adapter는 보내고 싶어도 보낼 것이 없다. `&Transcript`를 그대로 넘기지 않은 이유도 이것이다 — 지금은 오디오를 가리키지 않지만, 계약이 domain 레코드에 묶이면 그 레코드가 자라는 순간 경계가 함께 자란다 |

### 4.3 계약과 함께 정해 두는 것

- **비동기가 아니다.** 이 저장소에는 async runtime이 없고 [E1], 오래 걸리는 일은 이미
  평범한 스레드로 UI 밖에서 돈다 (전사 · `TranscriptionEngine: Send + Sync`). AI 호출 하나
  때문에 런타임을 들이지 않는다 (§20.5 — 미래 의존성 선입금 금지). `Send + Sync`인 이유도
  전사와 같다.
- **"provider 미설정"은 이 계약의 상태가 아니다.** 그것은 **provider 객체가 없는 상태**
  (`Option<Arc<dyn NoteAiProvider>>` = `None`)로 표현한다. 없는 것에게 물어보지 않는 것이
  INV-8을 타입으로 적는 방법이며, 그래서 미설정은 adapter가 아니라 서비스 계층이 만드는
  실패다 (§13.1).
- **구현이 하나뿐인 추상화는 검증된 추상화가 아니다** (§21). 이 계약은 Ollama adapter와
  **결정론적 test double(fake provider)** 둘이 통과한다. fake는 테스트 전용이며 제품 UI의
  선택지가 아니다. 이것으로 두 번째 벤더를 V1에 넣지 않고 경계를 검증한다.
- **adapter 내부도 둘로 나눈다** — 프로세스/네트워크 경계(HTTP 왕복)와 요청 생성·응답
  파싱·검증(순수 함수). 전사가 `engine`(경계)과 `parse`(순수)로 나뉜 것과 같은 이유이며,
  §18이 요구하는 "실제 Ollama 없이 테스트"가 그래야 성립한다.

---

## 5. (b) 호출 주체 — Rust backend

### 5.1 결정

**Ollama 호출은 Rust backend에서 나간다.** frontend는 Ollama의 주소도 엔드포인트도 모르며,
webview에는 Ollama로 가는 HTTP 경로가 존재하지 않는다. 화면은 Tauri command만 부른다.

```text
React 화면 ──command──▶ Rust (ai::service) ──HTTP──▶ 사용자의 로컬 Ollama
                                 │
                                 └─ 벤더 지식은 전부 여기 안쪽에만 있다 (INV-9)
```

### 5.2 근거 — §14.5의 CORS 사실이 결정적이다

§14.5는 이 선택에 직접 영향을 주는 사실을 적어 두었다 [E2 · 2026-09-01].

> **CORS** — 기본 허용 origin은 `127.0.0.1`과 `0.0.0.0`뿐이다 (`OLLAMA_ORIGINS`로 확장).
> Tauri webview origin(`tauri://localhost` 등)은 기본 허용 목록에 **없다.**
> 따라서 프론트엔드에서 직접 `fetch`하면 CORS로 막힐 가능성이 높고, 사용자가 환경변수를
> 설정해야 하는 제품이 된다. **Rust backend에서 호출하면 이 문제가 발생하지 않는다**
> (브라우저 CORS 대상이 아님).

frontend를 택하면 **제품이 사용자에게 `OLLAMA_ORIGINS` 설정을 요구하게 된다.** 그것은
§14.4.2가 전사에 대해 이미 거절한 것과 같은 종류의 요구다 — 사용자에게 환경 구성을 시키지
않는다. 게다가 그 요구는 플랫폼마다 다른 방법(셸 프로필 · launchctl · 시스템 환경변수)으로
설명해야 하고, 설정 여부를 앱이 확인할 수도 없다.

**주의**: 이 근거는 "CORS로 반드시 막힌다"가 아니라 **"기본 허용 목록에 webview origin이
없다"**는 §14.5의 기록 위에 서 있다. 실제 차단 여부를 이 Run이 실행해 본 것은 아니다.
그러나 Rust 경로는 그 사실이 맞든 틀리든 영향을 받지 않으므로, **틀렸을 때 손해가 없는
쪽**을 택했다.

### 5.3 CORS 말고도 같은 방향을 가리키는 것

| 근거 | 내용 |
| --- | --- |
| **INV-6를 코드로 보장한다** | 오디오가 밖으로 나갈 수 없다는 것을 "그런 코드를 안 짰다"가 아니라 **"webview에 나갈 통로가 없다"**로 말할 수 있다. 외부로 나가는 자리가 Rust 한 곳이면 감사할 대상도 한 곳이다 |
| **저장소가 이미 그렇게 서 있다** | persistence ownership이 Rust 경계에 있고(§14.7 · ADR-0001), 전사도 Rust 안에서 돈다 [E1]. AI만 webview에서 나가면 이 앱에서 유일하게 다른 방향의 컴포넌트가 된다 |
| **§14.7이 이미 이 방향을 예고했다** | "§14.5의 'Ollama를 Rust backend에서 호출' 방향과도 일관된다"가 persistence를 Rust에 둔 근거 목록에 들어 있다 [E2] |
| **테스트 경계** | HTTP 왕복을 trait 뒤에 두면 §18이 요구하는 "실제 Ollama 없이" 검증이 Rust 단위 테스트로 끝난다. webview `fetch`를 대상으로 하면 DOM/네트워크 mocking이 필요하다 |

### 5.4 Phase 5의 Notion 호출과의 일관성

**같은 규칙을 Phase 5에도 적용한다 — 외부로 나가는 호출은 전부 Rust backend에서 나간다.**
이 문서는 Phase 5를 구현하지 않지만, 규칙을 여기서 정해 두는 이유는 세 가지다.

1. **INV-7이 그것을 요구한다.** Notion integration token은 secret이며 **frontend 소스와
   저장소에 저장하지 않는다.** 토큰을 다루는 코드가 webview에 있으면 토큰이 webview로
   가야 한다. 경계를 나중에 옮기는 것보다 처음부터 같은 쪽에 두는 것이 싸다.
2. **§12의 opt-in 경계가 한 곳이어야 감사된다.** "사용자가 요청하지 않은 외부 전송이
   일어나지 않는다"(§17.3-3)를 확인하려면 나가는 자리가 열거 가능해야 한다.
3. **§14.9의 한도 처리가 domain 로직이다.** 2000자 청킹과 100블록 배치 전송은 데이터를
   아는 쪽에 있어야 하며, 그쪽은 이미 Rust다.

즉 Phase 4의 이 결정은 Phase 5에서 뒤집히지 않는다. **HTTP 클라이언트를 하나만 갖는
근거이기도 하다** (§12).

---

## 6. (c) 구조화 출력 수단과, 응답이 기대와 다를 때의 방어 경로

### 6.1 결정

**둘 다 한다. 단 정확성은 파싱 쪽에만 걸려 있다.**

1. Ollama의 **`format` 파라미터에 모드별 JSON Schema 객체**를 실어 보낸다 (§7의 schema를
   그대로 JSON Schema로 옮긴 상수).
2. 그리고 **응답을 신뢰하지 않고 파싱·검증한다.** `format`이 없었던 것처럼 동작해도
   제품은 옳게 동작한다.

`format`은 **최적화이지 계약이 아니다.** 이 구분이 §3.1의 제약(오늘 재확인하지 못함)에
대한 답이다.

### 6.2 왜 `format`을 쓰는가, 그리고 왜 거기에 기대지 않는가

**쓰는 이유**

- §14.5가 `format`이 문자열 `"json"` 또는 **완전한 JSON Schema 객체**를 받고 `/api/generate`와
  `/api/chat` 양쪽에서 동작한다고 기록했다 [E2 · 2026-09-01]. Phase 4의 §9.3에 직접 쓰라고
  적힌 값이다.
- **로컬 소형 모델이 지시만으로 순수 JSON을 내지 않는 것은 예외가 아니라 흔한 일이다**
  (Goal 요구 6: "이것은 로컬 소형 모델에서 실제로 자주 일어나는 일이므로 예외가 아니라
  기본 경로다"). 디코딩 자체를 제약하면 첫 응답이 파싱되는 비율이 올라간다.

**기대지 않는 이유**

- 이 Run은 §14.5를 **재확인하지 못했다** (§3.1). 파라미터 이름·수용 형태·버전별 동작이
  기록과 다를 가능성을 0으로 둘 수 없다.
- §14.5의 "JSON-schema `format` 지원이 v0.5에 도입됐다"는 **2차 출처 기반이다** [E3].
  사용자의 Ollama가 그보다 낮을 수 있고, 앱은 그 버전을 강제할 수 없다.
- `format`은 **벤더 고유 수단이다.** 계약에 그것을 노출하면 INV-9가 깨진다. 그래서 §4의
  trait에는 `format`이라는 말이 없고, 수단의 차이는 adapter가 흡수한다.

따라서 **`format`이 무시되거나 거절되어도** adapter는 같은 파싱 경로로 답을 내거나,
같은 실패(`aiResponseUnusable`)로 끝난다. 어느 쪽도 앱을 깨뜨리지 않는다.

### 6.3 방어 경로 — 응답이 기대 schema와 다를 때

응답 처리는 **순수 함수 하나**로 모은다 (`ai::ollama::parse`). 네트워크를 모르므로 실제
Ollama 없이 테스트된다 (§18).

```text
HTTP 200 본문
  │
  1. 봉투 읽기 — 본문을 JSON으로 읽고 생성 결과 필드를 꺼낸다.
  │     실패 → aiResponseUnusable
  │
  2. 본문에서 JSON 뽑기 — 값을 trim한다. 그대로 JSON이면 그것을 쓴다.
  │     아니면 **딱 한 번** 회수를 시도한다: ``` 코드펜스를 벗기고
  │     바깥쪽 균형 잡힌 {...} 하나를 취한다.
  │     그 이상은 하지 않는다 — 필드 추측 · 정규식 짜맞추기 · 모델에게 재요청은 없다.
  │     실패 → aiResponseUnusable
  │
  3. 모드별 struct로 역직렬화 (serde).
  │     모르는 필드는 **무시한다** (모델이 덧붙인 필드 때문에 실패하지 않는다).
  │     필수 필드가 없으면 실패 → aiResponseUnusable
  │
  4. domain 규칙 검증 (§7.4).
  │     필수 문자열이 trim 후 비어 있으면 실패 → aiResponseUnusable
  │     배열 원소 중 빈 문자열은 버린다. 배열이 비는 것은 정상이다.
  │     크기 상한(§7.4)을 넘으면 잘라내지 않고 실패 → aiResponseUnusable
  │
  5. StructuredNote
```

**깨지지 않는다는 것의 뜻을 명시한다.**

- 어떤 응답도 `panic`이나 `unwrap`으로 앱을 끝내지 못한다. 위 다섯 단계는 전부 `Result`다.
- **검증에 실패한 노트는 저장되지 않는다.** 반쯤 채워진 노트를 남기지 않는다 —
  `ai_notes`에는 UPDATE 경로가 없으므로 [E1] 잘못 들어간 행은 고칠 수 없다.
- 실패는 `recordings.ai_status = failed`로만 나타나며 **Transcript와 Recording은 바이트
  하나 바뀌지 않는다** (INV-2 · INV-3).
- 사용자는 실패를 화면에서 보고 **다시 시도할 수 있다** — 이 실패는 `retryable: true`다
  (§13.2가 그 이유를 적는다).

### 6.4 그래서 adapter가 실제로 부르는 것

| 목적 | 호출 | 근거 |
| --- | --- | --- |
| 가용성 + 모델 목록 | **`GET /api/tags`** | §14.5 [E2]. **헬스 체크를 따로 부르지 않는다** — 이 한 번으로 "서버가 응답하는가"와 "모델이 있는가"가 동시에 답해지고, §13은 그 둘을 구분하라고만 요구한다. `GET /`나 `/api/version`을 더 부르면 왕복만 늘고 새 정보는 없다 |
| 노트 생성 | **`POST /api/generate`**, `stream: false` | §14.5 [E2]. `/api/chat`이 아니라 `/api/generate`인 이유: 한 번의 변환이고 대화 상태가 없다. **agentic loop가 필요한 이유가 없다**는 §9.4의 판단과 같은 방향이며, 스트리밍 응답 UI는 이 Phase의 범위 밖이다 |

요청 본문의 모양(§14.5의 예시를 그대로 따른다):

```json
{ "model": "<사용자가 고른 모델>",
  "prompt": "<프롬프트 + transcript 텍스트>",
  "stream": false,
  "format": { "type": "object", "properties": { ... }, "required": [ ... ] },
  "options": { "num_ctx": 16384 } }
```

**엔드포인트 경로 · 파라미터 이름은 전부 [E2]다.** 구현 Task는 코드를 쓰기 전에 이것을
다시 확인하고 evidence를 남긴다 (§14.3).

---

## 7. (d) Structured Note 최종 schema

### 7.1 명명 규칙 — 왜 §9.3의 예시와 다른 이름이 있는가

§9.3은 자신의 JSON을 예시라고 밝히고 **"최종 schema는 해당 Phase(§21의 Phase 4)에서
확정한다"**고 적는다. 그래서 이 문서가 확정하며, 규칙은 하나다.

> **필드 이름은 §9.5 표의 출력 섹션 이름을 camelCase로 옮긴 것이다.**

§9.5가 세 모드의 출력 섹션을 정한 **정본**이고, 렌더러(UI · Markdown · Notion)가 만들
제목이 곧 그 섹션 이름이기 때문이다. 이름이 섹션과 1:1이면 렌더러는 매핑 표를 갖지 않아도
되고, 필드가 늘거나 줄 때 §9.5만 보면 된다.

그 결과 §9.3 예시와 **세 군데**가 다르다.

| 모드 | §9.3 예시 | 확정 | 이유 |
| --- | --- | --- | --- |
| Meeting | `keyPoints` | **`keyDiscussions`** | §9.5의 섹션은 "Key Discussions"다. 또 `keyPoints`는 Summary의 섹션 이름이기도 해서, 같은 이름이 두 모드에서 다른 것을 뜻하게 된다 |
| Meeting | `questions` | **`openQuestions`** | §9.5의 섹션은 "Open Questions"다. Study의 `questions`("Questions")와 뜻이 다르다 — 하나는 미해결 쟁점, 하나는 학습용 질문이다 |
| Study | `references` | **`referencesMentioned`** | §9.5의 섹션은 "References Mentioned"다. "언급된 것"이라는 제한이 이름에 남아야 모델이 없는 참고문헌을 지어내지 않는다 |

### 7.2 세 모드의 확정 schema

**타입은 두 가지뿐이다** — `string`과 `string[]`. 중첩 객체를 쓰지 않는다 (§7.3).

#### Meeting (§9.5: Overview · Key Discussions · Decisions · Action Items · Open Questions)

| 필드 | 타입 | 키 필수 | 값 제약 |
| --- | --- | --- | --- |
| `overview` | `string` | **필수** | trim 후 비어 있으면 **무효** |
| `keyDiscussions` | `string[]` | **필수** | 빈 배열 허용. 원소는 trim 후 비어 있지 않은 `string` |
| `decisions` | `string[]` | **필수** | 빈 배열 허용 |
| `actionItems` | `string[]` | **필수** | 빈 배열 허용 |
| `openQuestions` | `string[]` | **필수** | 빈 배열 허용 |

#### Study (§9.5: Overview · Key Concepts · Important Details · Questions · Things to Study · References Mentioned)

| 필드 | 타입 | 키 필수 | 값 제약 |
| --- | --- | --- | --- |
| `overview` | `string` | **필수** | trim 후 비어 있으면 **무효** |
| `keyConcepts` | `string[]` | **필수** | 빈 배열 허용 |
| `importantDetails` | `string[]` | **필수** | 빈 배열 허용 |
| `questions` | `string[]` | **필수** | 빈 배열 허용 |
| `thingsToStudy` | `string[]` | **필수** | 빈 배열 허용 |
| `referencesMentioned` | `string[]` | **필수** | 빈 배열 허용 |

#### Summary (§9.5: Short Summary · Key Points)

| 필드 | 타입 | 키 필수 | 값 제약 |
| --- | --- | --- | --- |
| `shortSummary` | `string` | **필수** | trim 후 비어 있으면 **무효** |
| `keyPoints` | `string[]` | **필수** | 빈 배열 허용 |

**§9.5의 13개 출력 섹션이 13개 필드에 1:1로 대응하며, 빠진 섹션도 남는 섹션도 없다.**

### 7.3 이 형태로 정한 이유

| 결정 | 이유 |
| --- | --- |
| **키는 언제나 있고, 배열은 비어도 된다** | "결정된 것이 없었다"는 실제 결과다. 배열을 필수로 채우게 하면 모델이 **없는 결정을 지어낸다.** 반대로 키를 선택적으로 두면 렌더러가 "없음"과 "비어 있음"을 매번 구분해야 한다 — 키는 항상 있고 `[]`가 "없음"을 뜻하는 쪽이 읽는 코드가 단순하다 |
| **`overview` · `shortSummary`는 비어 있으면 무효** | 요약이 없는 요약 노트는 노트가 아니다. 이 하나가 "생성은 됐지만 쓸모없는 결과"를 걸러내는 최소 기준이고, 걸러진 것은 §13의 실패로 보인다 |
| **`actionItems`가 `{text, owner, dueDate}`가 아니라 `string[]`** | 담당자·기한 추출은 모델의 개체 인식 능력에 기대는 일이고, **로컬 소형 모델이 그것을 할 수 있는지는 UNVERIFIED다** [E4 · §14.5는 어느 모델이 적합한지도 UNVERIFIED로 둔다]. 확인되지 않은 능력 위에 스키마를 세우면 필드가 늘 비거나 틀린 값으로 찬다. 필요해지면 `schemaVersion`을 올려 바꾼다 (§7.5) — 추상화를 선입금하지 않는다 (§20.6) |
| **Markdown 문자열을 core data로 쓰지 않는다** | §9.3이 "provider가 돌려준 임의의 Markdown을 core data model로 쓰지 않는 것을 우선 검토한다"고 요구한다. 렌더링은 `Structured Note → UI / Markdown / Notion` 한 방향으로만 흐르며, Phase 5의 Notion 렌더러가 §14.9의 블록 한도를 다루려면 **구조가 남아 있어야 한다** — Markdown 덩어리는 다시 파싱해야 한다 |
| **모드마다 다른 struct** | 세 모드는 출력 섹션이 다르다. 하나의 옵셔널 필드 뭉치로 합치면 "Meeting에 `thingsToStudy`가 있는" 상태가 타입 수준에서 가능해진다 |

### 7.4 검증 규칙 (§6.3의 4단계가 적용하는 것)

- 문자열은 trim 후 비교한다. 필수 문자열이 비면 **무효**.
- 배열 원소 중 trim 후 빈 것은 **버린다** (실패가 아니다).
- 크기 상한: 배열 원소 **200개**, 원소 하나 **2,000자**, `overview`/`shortSummary` **4,000자**.
  넘으면 **잘라내지 않고 무효**로 본다 — 조용히 자른 노트는 완전해 보이지만 완전하지 않다.
  (2,000자는 §14.9의 Notion `text.content` 한도와 같은 값이다. Phase 5가 다시 자르지 않아도
  되도록 맞췄다.)
- 검증 실패는 전부 `aiResponseUnusable` 하나로 모인다 (§13.2).

### 7.5 `ai_notes.content`에 어떻게 담기는가

`ai_notes.content`는 `TEXT NOT NULL`이고 **스키마는 그 내용을 해석하지 않는다** [E1 —
migration 2의 주석이 "최종 schema는 Phase 4가 확정하므로 스키마는 내용을 해석하지 않는
텍스트로 둔다"고 적어 두었다]. 그 자리에 **아래 봉투를 compact JSON 한 문자열**로 담는다.

```json
{"schemaVersion":1,"mode":"meeting","note":{"overview":"...","keyDiscussions":[],"decisions":[],"actionItems":[],"openQuestions":[]}}
```

| 요소 | 왜 있는가 |
| --- | --- |
| `schemaVersion` (integer, 필수) | **저장된 노트는 schema보다 오래 산다.** `ai_notes`에는 UPDATE 경로가 없으므로 [E1] 옛 노트를 새 형태로 고쳐 쓸 수 없고, 고쳐 쓰는 것은 provenance를 지우는 일이다. 읽는 쪽이 형태를 판정할 값이 봉투 안에 있어야 한다. 첫 값은 **1**이다 |
| `mode` (`"meeting"\|"study"\|"summary"`, 필수) | 봉투 하나만으로 자기 형태를 말할 수 있게 한다. `note_type` 열과 중복되지만, 그 중복이 **열과 내용의 불일치를 검출**하고(읽을 때 다르면 `Storage` 실패), Phase 5가 DB 밖으로 내보낸 JSON도 그대로 해석된다 |
| `note` (객체, 필수) | §7.2의 모드별 필드. 키 순서는 §7.2 표의 순서로 **고정**한다 (serde의 선언 순서). 같은 노트가 언제나 같은 문자열이 되므로 테스트가 문자열을 그대로 비교할 수 있다 |

**읽을 때의 규칙**

- `schemaVersion`이 코드가 모르는 값이면 → **행을 건드리지 않고** `FailureKind::Storage`로
  "이 앱보다 새 버전이 만든 노트"라고 말한다. 지우지도, 다시 쓰지도 않는다.
  (migration의 `DatabaseError::AheadOfCode`와 같은 태도다 [E1].)
- `mode`와 `note_type` 열이 다르면 → 같은 `Storage` 실패. 추측해서 한쪽을 고르지 않는다.
- **읽기 실패는 AI 실패가 아니다.** provider와 무관하므로 §13.2의 AI 실패 종류로 옮기지 않는다.

---

## 8. (e) Context window 전략

### 8.1 §14.5가 강제하는 것

> **기본 context window가 4096 토큰이다** (`options.num_ctx`로 요청별 지정,
> `OLLAMA_CONTEXT_LENGTH`로 서버 기본값 변경). **1시간 transcript(대략 8~12K 단어,
> 12~20K+ 토큰)는 기본값을 초과한다.** → Phase 4는 청킹하거나 `num_ctx`를 명시적으로
> 키워야 하며, 키우는 것은 사용자 기기의 RAM/VRAM에 제약된다.
> **이것은 선택이 아니라 필수 설계 항목이다.** [E2 · 2026-09-01]

즉 아무것도 하지 않는 선택지는 없다. 아무것도 하지 않으면 1시간 transcript는
**조용히 잘린 채** 그럴듯한 노트가 되어 저장된다. 그것이 이 항목에서 가장 나쁜 결과다.

### 8.2 결정 — `num_ctx` 명시 + 사전 크기 판정. 청킹도 절단도 하지 않는다

1. **adapter는 모든 생성 요청에 `options.num_ctx`를 명시한다.** 서버 기본값에 기대지 않는다.
   사용자가 `OLLAMA_CONTEXT_LENGTH`를 만졌는지 앱은 알 수 없고, 알 수 없는 값 위에서
   결과가 달라지면 재현할 수 없기 때문이다.
2. 값은 설정 `ai_context_tokens`에서 온다 (§11). **기본값 16384.**
   > ⚠️ **이 항목은 구현되지 않았다.** 설정 열도 화면도 만들어지지 않았고, 값은 언제나
   > `ContextBudget::DEFAULT`(16384)다. 그래서 **사용자가 이 값을 키울 수 없다** — 그 사실이
   > §8.3 · §8.4 · §13.2 · §16.2의 "설정에서 값을 키우면 된다"를 지금은 성립하지 않게 만든다
   > (§17.3.2).
3. **요청을 보내기 전에** domain의 순수 함수가 프롬프트 크기를 보수적으로 추정하고
   예산과 비교한다. 넘치면 **요청을 보내지 않고** `aiInputTooLarge`로 실패한다 (§13.2).
4. **청킹(map-reduce)을 하지 않는다.**
5. **transcript를 잘라서 보내지 않는다.**

### 8.3 셋 중에서 이렇게 고른 이유

| 선택지 | 판정 | 이유 |
| --- | --- | --- |
| **절단 (truncate)** | **거절** | 잘린 transcript로 만든 노트는 **완전해 보이지만 완전하지 않다.** 사용자는 회의 후반이 통째로 빠진 것을 알 수 없다. §13은 실패를 숨기지 말고 상태로 보이라고 요구한다 — 조용한 절단은 그 반대다 |
| **청킹 (map-reduce)** | **이 Phase에서는 하지 않음 (DEFERRED)** | 모델 호출이 여러 번이 되고, 모드마다 **부분 결과 병합 정책**이 필요해진다(중복 결정 항목 합치기 · action item 순서 · overview 재요약). 그 병합의 품질은 **정확히 이 Phase에서 UNVERIFIED인 것**에 달려 있다 — 어느 로컬 모델이 이 transcript에 쓸 만한지는 §14.5가 UNVERIFIED로 두었고 Human Review 항목이다 [E4]. 검증되지 않은 것 위에 두 번째 미검증 층을 쌓지 않는다. **계약도 schema도 청킹을 막지 않는다** — 청킹은 adapter 내부의 일이므로 나중에 §4의 trait을 바꾸지 않고 넣을 수 있다 |
| **`num_ctx` 명시 + 사전 판정** | **채택** | 필요한 것을 전부 준다: 4096을 넘길 수 있고(§14.5의 요구), 서버 설정에 의존하지 않으며, 넘칠 때 **조용히 잘리는 대신 사용자에게 보인다**. 사용자가 할 수 있는 일도 분명하다 — 설정에서 값을 키우거나, 더 큰 context의 모델을 고르거나, 더 짧은 녹음을 고른다. ⚠️ **구현에서는 앞의 둘이 사용자에게 열려 있지 않다** — 설정 항목이 없고, 값이 모델과 무관하게 16384로 고정이라 더 큰 모델을 골라도 예산이 커지지 않는다. 남는 수단은 **더 짧은 녹음** 하나다 (§17.3.2) |

### 8.4 기본값 16384와 그 한계

- **4096보다 커야 한다**는 것은 §14.5가 준 사실이다 [E2].
- **1시간 transcript를 시도할 수 있어야 한다** — Phase 4의 Human Review가
  "1시간 분량 transcript 처리 시간이 실사용 가능한 수준인가"를 묻기 때문이다. §14.5의
  추정은 12~20K+ 토큰이므로, 16384는 그 구간을 **아래쪽부터 덮는다.**
- **상한은 앱이 알 수 없다.** §14.5가 "키우는 것은 사용자 기기의 RAM/VRAM에 제약된다"고
  적었고, 앱은 사용자의 메모리도 모델의 KV cache 크기도 모른다. 그래서 **값을 설정으로
  노출하고 기본값을 하나 고른다.**
  > ⚠️ **노출하는 쪽은 구현되지 않았다.** 지금 있는 것은 "기본값 하나"뿐이다 (§17.3.2).
- **16384는 검증된 용량 수치가 아니라 시작값이다** [E4]. 어떤 기기에서 어느 모델이 이 값을
  감당하는지는 이 Run이 확인하지 않았고 확인할 수도 없다. Phase 4 Human Review가
  판정할 항목이며, 판정 결과로 기본값이 바뀌면 이 절을 고친다.

### 8.5 크기 추정 — 무엇을 근거로 하는가

앱은 모델의 tokenizer를 갖고 있지 않다. 그래서 **토큰 수를 세지 않고 보수적으로 추정한다.**

```text
estimated_tokens ≈ ceil(chars / 2)
budget           = ai_context_tokens − PROMPT_RESERVE(1024) − OUTPUT_RESERVE(1536)
넘치면 → aiInputTooLarge (요청을 보내지 않는다)
```

- **`chars / 2`는 tokenizer 사실이 아니라 안전한 과대추정이다** [E4]. 영어는 실제보다 훨씬
  크게 잡히고, 한국어는 대략 비슷하게 잡힌다. 과대추정이어야 "들어간다"가 "거의 확실히
  들어간다"를 뜻한다. 과소추정하면 이 가드는 존재 이유를 잃는다.
- 이 비율을 **정확한 값처럼 쓰지 않는다.** 용도는 가드레일 하나이며, 노트 품질이나 비용
  계산에 쓰지 않는다.
- 예약값(1024 / 1536)은 프롬프트 지시문과 출력이 차지할 자리다. 출력 예약은 §7.4의 크기
  상한과 같은 방향에서 정했다.
- 추정 함수는 **domain의 순수 함수**다 — 네트워크도 모델도 없이 테스트된다 (§18).

---

## 9. (f) AINote 재생성 정책

### 9.1 저장소의 현재 사실 ([E1])

| 사실 | 근거 |
| --- | --- |
| `ai_notes`에 대한 쓰기 경로는 **`insert_ai_note` 하나뿐이다.** UPDATE도 DELETE도 없다 | `db/store.rs` |
| 읽기는 `load_ai_note`(단건) · `list_ai_notes_for_transcript`(`ORDER BY generated_at, id`) 둘 | 같음 |
| `INSERT OR REPLACE`를 **의도적으로 쓰지 않는다** — "이미 있는 id로 다시 쓰면 조용히 덮어쓰는 대신 실패한다" | `db/store.rs` 모듈 주석 |
| 인덱스 `idx_ai_notes_transcript (transcript_id, generated_at)`가 이미 있다 | `db/migrations.rs` |
| §7.1이 `Transcript 1:N AINote`를 **확정된 domain 규칙**으로 둔다 | PRODUCT-SPEC §7.1 |

### 9.2 결정 — append-only 이력. 대체하지 않는다

- **재생성은 언제나 새 `ai_notes` 행을 INSERT한다.** 기존 행은 어떤 경우에도 바뀌지 않는다.
- **이 Phase는 `ai_notes`를 UPDATE하거나 DELETE하는 경로를 만들지 않는다.**
- 화면(Recording Detail의 AI Note 탭)은 **`(transcript_id, note_type)`별 가장 최근 노트**를
  기본으로 보여 준다. 정렬 기준은 `(generated_at, id)`이며 기존 인덱스가 그대로 답한다.
- 이전 노트는 사라지지 않고 조회 가능하다. **이력 UI는 이 Phase의 요구가 아니다** —
  데이터가 이력을 지탱하되 화면을 미리 만들지는 않는다 (§5의 화면 4개 · §16 UX polish).

### 9.3 왜 대체가 아니라 이력인가

| 근거 | 내용 |
| --- | --- |
| **대체하려면 없는 경로를 새로 만들어야 한다** | 저장소에 UPDATE도 DELETE도 없다 [E1]. 즉 "대체"는 단순한 쪽이 아니라 **저장소에 파괴적 경로를 새로 여는 쪽**이다. 그 경로는 한번 생기면 이 테이블에만 쓰이지 않는다 |
| **대체는 provenance를 지운다** | §7.3 · §9.6은 "이 노트가 **어떤 Transcript version에서** 무엇으로 언제 만들어졌는지" 알 수 있어야 한다고 요구한다. 덮어쓰면 이전 `model` · `promptVersion` · `generatedAt`이 사라진다. 프롬프트를 바꾼 뒤 "전보다 나아졌는가"를 물을 수 없게 된다 — 그것이 `promptVersion`을 기록하는 이유 자체다 (§10) |
| **1:N이 이미 확정된 domain 규칙이다** | §7.1의 그림은 한 Transcript 아래 `AI Note A1` · `A2`를 그린다. 여러 행은 우회가 아니라 **모델링된 형태**다 |
| **INV-2의 정신** | 재생성이 원본을 훼손하지 않는다는 규칙은 Transcript에 대한 것이지만, 같은 이유(파생물도 한번 만들어지면 그 시점의 기록이다)가 여기에도 적용된다 |
| **비용이 작다** | 행 하나는 사용자가 버튼을 누를 때만 늘어난다. 자동으로 무한히 늘어나는 경로가 없다 |

### 9.4 재생성이 건드리는 것과 건드리지 않는 것

```text
바뀌는 것:   ai_notes에 새 행 1개
             recordings.ai_status · recordings.updated_at
바뀌지 않는 것: transcripts · transcript_segments · 기존 ai_notes 행 · audio 파일
             recordings의 그 밖의 모든 열
```

마지막 줄은 §7의 "AI Provider는 `Recording`의 AI 상태 필드 외에 recording metadata를
변경하지 않는다"와 INV-1 · INV-2 · INV-3을 그대로 옮긴 것이다. 상태 갱신은 이미 있는
`update_recording_statuses`를 쓴다 [E1] — 그 함수는 세 상태와 `updated_at`만 만진다.

**Transcript가 여럿인 Recording에서도 어느 Transcript도 변경되지 않는다.** 노트는
`transcript_id`로 자기 출처를 가리킬 뿐이며, 서로 다른 Transcript에서 나온 노트는
그 열로 구분된다 (§7.3).

---

## 10. (g) `promptVersion` 정책

### 10.1 요구

> **`promptVersion`이 실제로 기록된다. 프롬프트가 바뀌면 값이 바뀌어야 한다.**
> (Goal 요구 13)

"바꾸는 것을 잊지 말자"는 정책이 아니다. **잊을 수 없게 만드는 것**이 이 절의 결정이다.

### 10.2 결정

1. **프롬프트는 정적 상수다.** 모드별로 하나씩, `ai::prompt` 한 모듈에만 있다. 실행 중에
   조립되는 부분은 **transcript 텍스트를 끼워 넣는 자리 하나**뿐이다. (프롬프트 커스터마이즈
   UI는 DEFERRED다 — §16.)
2. **값의 형태**는 다음과 같다.

   ```text
   promptVersion = "v<SET>.<mode>.<hash8>"      예: "v1.meeting.9f3a2c1b"
   ```

   | 조각 | 무엇인가 |
   | --- | --- |
   | `SET` | 사람이 올리는 프롬프트 세트 버전. 첫 값은 `1` |
   | `mode` | `meeting` · `study` · `summary` |
   | `hash8` | **(프롬프트 템플릿 원문 ‖ 그 모드로 보내는 JSON Schema 직렬화 ‖ `schemaVersion`)** 의 해시 앞 8자리 hex |

3. **해시 대상에 JSON Schema와 `schemaVersion`을 포함한다.** 같은 문장이라도 요구한 출력
   형태가 달라지면 나온 노트가 달라지기 때문이다. 노트를 만든 것은 프롬프트 혼자가 아니다.
4. **테스트가 강제한다.** 각 모드마다 선언된 상수 `PROMPT_VERSION_MEETING` 등이 있고,
   테스트는 실제 상수들로부터 해시를 계산해 그 선언값과 비교한다.

   ```text
   프롬프트 한 글자를 고친다
     → 계산된 hash8이 달라진다
     → 선언값과 다르다
     → test Gate가 깨진다
     → 선언값을 고치기 전에는 통과할 수 없다
   ```

   이것이 "프롬프트가 바뀌면 값이 반드시 바뀐다"를 **관례가 아니라 Gate**로 만드는 방법이다.
   `store.rs`가 "Transcript를 갱신할 수 없다"를 API 표면으로 표현한 것과 같은 방식이다 [E1].

5. **해시 함수는 저장소 안에 직접 둔다** (FNV-1a 64bit, 수십 줄). 새 crate를 들이지 않는다.
   **`std::hash::DefaultHasher`를 쓰지 않는다** — 그 값은 Rust 버전 간 안정성이 보장되지
   않으므로, 툴체인을 올렸다는 이유로 저장된 provenance와 계산값이 어긋날 수 있다.
   영속되는 값은 재현 가능해야 한다.
6. **저장된 값은 그 노트가 만들어진 시점의 값이다.** append-only이므로(§9) 프롬프트를 바꿔도
   옛 노트의 `promptVersion`은 그대로 남는다. 그것이 "전 프롬프트와 비교"를 가능하게 한다.

### 10.3 이 값으로 하지 않는 것

- 프롬프트 버전으로 **노트를 자동 재생성하지 않는다.** 재생성은 사용자가 요청한다.
- 프롬프트 버전이 낮다는 이유로 옛 노트를 숨기거나 지우지 않는다.

---

## 11. (h) Provider 설정을 어디에 저장하는가 — 그리고 INV-7

### 11.1 결정

기존 **단일 행 `settings` 테이블**에 저장한다. `create_settings`(version 3)를 고치지 않고
**새 migration(version 6)으로 열을 더한다.** 이미 version 5까지 적용된 사용자 DB가 있을 수
있고, migration은 다시 실행되지 않기 때문이다 [E1 — 이 규칙은 테스트로 강제돼 있다].

| 열 | 타입 | NULL의 뜻 | 읽는 곳 | 구현 |
| --- | --- | --- | --- | --- |
| `ai_provider` | `TEXT` | **아직 고르지 않았다 — 정상 상태다 (INV-8)** | provider 선택 | **있다** |
| `ai_base_url` | `TEXT` | adapter의 기본 주소를 쓴다 | Ollama adapter | **있다.** 다만 기본 주소를 아는 자리는 adapter가 아니라 domain이다 (§17.3.3) |
| `ai_model` | `TEXT` | 아직 고르지 않았다 | 생성 요청 | **있다** |
| `ai_context_tokens` | `INTEGER` | 저장한 적 없음 → `Settings::DEFAULT`의 16384 (§8) | 생성 요청 | ⚠️ **없다 — 만들어지지 않았다** (§17.3.2) |

- 기본값 정책은 **스키마의 `DEFAULT` 절이 아니라 `Settings::DEFAULT`가 갖는다** — 이 테이블의
  기존 규약이다 [E1]. 그래야 "저장한 적 없음"과 "기본값과 같은 값을 저장함"이 구분된다.
- `ai_provider`가 `NULL`인 상태는 **오류가 아니다.** 그 상태에서 앱은 "AI 기능이 비활성"이라는
  담담한 상태를 보이고, 녹음 · 전사 · 열람은 전부 그대로 동작한다 (INV-8).
- 저장된 값이 지금 유효하지 않을 수 있다 — 고른 모델이 삭제됐거나 Ollama가 꺼져 있을 수 있다.
  그때 **앱이 저장된 선택을 조용히 바꾸거나 지우지 않는다.** `default_microphone`과
  `transcription_model`이 이미 그 규칙을 따른다 [E1]. 그 사실은 §13의 상태로 말한다.

### 11.2 이 Phase에서 **더하지 않는** 설정

- **`automatic_ai_note`(자동 AI 처리 토글)를 더하지 않는다.** §5-D의 목록에는 있지만
  Phase 4의 Required Outcome에는 자동 생성이 없다. 값을 읽는 코드가 없는 열은
  **자리를 미리 만들어 두는 것**이며, 이 저장소는 그것을 금지한다 (§20.6 —
  `domain/settings.rs`가 "그 기능을 실제로 구현하는 Phase가 함께 추가한다"고 적어 두었다 [E1]).
  자동 생성을 구현하는 Task가 그때 이 열을 더한다. 그때는 `automatic_transcription`이
  `automatic_processing`과 별개로 추가된 것과 **같은 이유로 별개의 열**이어야 한다 — 하나의
  boolean에 두 의미를 겹치지 않는다.

### 11.3 INV-7 — secret을 저장하지 않으며, 담을 열도 만들지 않는다

> **INV-7**: Secret(API key, integration token)은 frontend 소스와 저장소에 저장하지 않는다.

- **version 6 migration은 secret 열을 만들지 않는다.** `api_key` · `token` · `password` 류
  이름의 열이 없다. 이것은 version 3~5가 이미 지킨 규칙이며 [E1], 이 Phase도 깨지 않는다.
- **지금 secret이 필요 없다**는 것이 사실 관계다. Ollama는 사용자의 로컬 서버이며 §14.5에
  인증에 대한 기록이 없다. **그래서 더더욱 지금 자리를 만들지 않는다** — 빈 secret 열은
  언젠가 채워진다.
- **Cloud provider(DEFERRED)가 API key를 요구할 때 그 값은 이 테이블로 오지 않는다.**
  §3.1이 이미 `SecretStore`를 플랫폼 경계 후보로 열거해 두었다. 그 경계를 만드는 것은
  cloud adapter를 실제로 만드는 Phase의 일이며, 이 Phase는 그 경계를 미리 만들지도 않고
  (§20.6) 그 자리를 `settings`에 파 두지도 않는다.
- **`ai_base_url`은 secret이 아니다.** 그러나 Goal은 "provider 설정값을 로그 · 에러 메시지 ·
  evidence 파일에 남기지 않는다"고 요구한다. 그래서:

  | 어디 | 규칙 |
  | --- | --- |
  | `Failure::message` · `Failure::detail` | 설정된 host/port를 **넣지 않는다.** "로컬 AI 서버에 연결하지 못했다" 같은 문장과, 설정값이 아닌 기술적 원인만 담는다 |
  | 로그 | AI 경로는 설정값을 로그로 남기지 않는다 |
  | `.loop/evidence/**` | 실행 결과에 설정값을 적지 않는다 |
  | `ai_notes.model` · `ai_notes.provider` | **여기에는 남는다.** §7이 요구하는 provenance이며, 로그가 아니라 사용자의 로컬 DB다. 이것은 예외가 아니라 다른 종류의 저장이다 |

---

## 12. (i) HTTP 클라이언트

### 12.1 §14.5가 준 사실

| 사실 | 등급 |
| --- | --- |
| **공식 Rust SDK가 없다.** JS/TS `ollama-js`와 Python `ollama-python`은 공식, Rust `ollama-rs` 0.3.6은 **커뮤니티** | [E2 · 2026-09-01] |
| "Rust backend에서 호출한다면 `reqwest`로 REST를 직접 쓰는 것도 합리적 선택지다. **공식 Rust SDK가 없으므로 wrapper crate 의존이 필수는 아니다**" | [E2] |
| 현재 저장소에 HTTP 클라이언트도 async runtime도 **없다** | [E1] |

### 12.2 결정 — `ollama-rs`를 쓰지 않고, `ureq`로 REST를 직접 부른다

**먼저: wrapper crate를 쓰지 않는다.**

- 공식 SDK가 아니다 [E2]. 벤더 중립 경계를 지키자고 만든 adapter가 **3자 crate의 타입 모양에
  종속되면** 벤더 지식이 우리가 통제하지 못하는 곳으로 옮겨갈 뿐이다.
- 우리가 쓰는 것은 엔드포인트 **두 개**뿐이다 (§6.4). 그만큼을 위해 유지보수 주체가 다른
  의존성을 하나 더 들이지 않는다.
- ADR-0007이 `whisper-rs`에서 겪은 것과 같은 위험이다 — 번들 버전 지연과 유지보수 위치 이동은
  crate를 고를 때 실제로 비용이 됐다.

**그다음: 어떤 HTTP 클라이언트인가.**

| 후보 | 판정 | 이유 |
| --- | --- | --- |
| **`ureq`** | **채택** | 이 저장소의 모든 경계가 **동기**다 — `TranscriptionEngine`도 동기이고, 오래 걸리는 일은 평범한 스레드로 UI 밖에서 돈다 [E1]. blocking 호출 하나를 위해 async runtime을 들이지 않아도 되는 쪽을 고른다 (§20.5). Phase 5의 Notion(HTTPS)까지 같은 클라이언트 하나로 덮을 수 있다 — 외부 호출은 전부 Rust에서 나가므로(§5.4) 클라이언트가 둘일 이유가 없다 |
| **`reqwest`** (blocking) | **차점** | §14.5가 이름을 든 선택지이고 성숙하다. 다만 blocking API가 내부에 async runtime을 두는 것으로 알려져 있어(**[E4] — 이 Run에서 확인하지 못했다**) 지금 필요 없는 무게가 들어온다. `ureq` 채택이 §12.3의 확인에서 무너지면 **여기로 돌아온다** |
| **`std::net`으로 직접 HTTP 작성** | **거절** | HTTP/1.1 파싱 · chunked 인코딩 · 타임아웃을 직접 다루는 것은 이 제품이 풀 문제가 아니다. Phase 5의 TLS까지 가면 명백히 무리다 |
| **`ollama-rs`** | **거절** | 위 참조 |

**Phase 4에서 켜는 것**: HTTP만 있으면 된다 — Ollama는 사용자의 로컬 주소이고, 이 Phase는
인터넷으로 나가지 않는다 (§12의 privacy 경계). **TLS backend 선택은 Phase 5의 일이다.**
지금 켜 두면 그것이야말로 미래 의존성 선입금이다 (§20.5).

### 12.3 구현 Task가 반드시 확인할 것

이 Run은 네트워크가 없어 **crate 이름 외에는 아무것도 확인하지 못했다** (§3.1).

- 최신 버전 · 최근 릴리스 일자 · MSRV — **[E4] UNVERIFIED**. crates.io에서 확인하고 evidence를 남긴다.
- 동기 API가 async runtime을 요구하지 않는다는 것 — **[E4]**. 확인해서 어긋나면 `reqwest`로 간다.
- Phase 5에서 TLS를 순수 Rust 경로로 켤 수 있는가 — **[E4]**. Phase 5가 확인한다.

**어느 쪽을 택하든 §4의 계약도 §7의 schema도 바뀌지 않는다.** HTTP 왕복은 adapter 안쪽의
얇은 trait 뒤에 있고, 그 뒤를 바꾸는 것은 파일 하나를 바꾸는 일이다.

---

## 13. (j) §13의 AI Provider 실패 → domain 공통 실패 매핑

### 13.1 매핑 표

새로 더하는 `FailureKind`는 여섯이다. 문자열은 `src/ipc/failure.ts`의 union과 1:1로 유지된다
(기존 테스트가 그것을 강제한다 [E1]).

| §13의 실패 | `FailureKind` | 문자열 | 누가 만드는가 | `retryable` | `source_data_safe` |
| --- | --- | --- | --- | --- | --- |
| **provider 미설정** | `AiProviderNotConfigured` | `aiProviderNotConfigured` | **서비스 계층** (provider 객체가 없다 — adapter를 부르지 않는다) | **false** | true |
| **provider 연결 불가** (Ollama 미실행) | `AiProviderUnreachable` | `aiProviderUnreachable` | Ollama adapter (transport 오류를 번역) | **true** | true |
| **모델 없음** | `AiModelUnavailable` | `aiModelUnavailable` | Ollama adapter (`GET /api/tags` 결과 판정) | **false** | true |
| **요청 실패** | `AiRequestFailed` | `aiRequestFailed` | Ollama adapter (비2xx · 타임아웃 · 본문 읽기 실패) | **§13.3의 규칙** | true |
| **응답이 기대 schema와 다름** | `AiResponseUnusable` | `aiResponseUnusable` | adapter의 파싱/검증 (§6.3) | **true** | true |
| *(추가)* **입력이 context 예산을 넘음** | `AiInputTooLarge` | `aiInputTooLarge` | domain (요청 전 계산 · §8.5) | **false** | true |

**여섯 번째는 §13의 목록에 없다.** §14.5가 "이것은 선택이 아니라 필수 설계 항목"이라고 적은
상황이 실제 제품 상태로 존재하기 때문에 더했다. 이것을 `aiRequestFailed`로 뭉치면 사용자는
**설정에서 값을 키우면 된다**는 것을 알 수 없다.

### 13.2 `retryable`을 이렇게 정한 이유

| 실패 | 왜 그 값인가 |
| --- | --- |
| `aiProviderNotConfigured` — **false** | 같은 상태로 다시 눌러도 같다. 사용자가 **설정에서 provider를 골라야** 풀린다. 그리고 이것은 애초에 오류로 표시하지 않는다 — §13이 "provider가 없음은 오류가 아니라 정상 상태"라고 못박았고, 화면은 경고가 아니라 **AI 기능이 비활성이라는 담담한 상태**로 보인다 (INV-8). 이 실패 값은 사용자가 그 상태에서 굳이 생성을 요청했을 때의 답이다 |
| `aiProviderUnreachable` — **true** | 사용자가 Ollama를 켜고 **같은 버튼을 다시 누르면 성공한다.** 앱이 고칠 것은 없고 재시도가 의미를 갖는다 |
| `aiModelUnavailable` — **false** | 모델을 받아 오거나 다른 모델을 고르기 전에는 다시 시도해도 같다. `TranscriptionModelMissing`이 `permanent`인 것과 같은 판단이다 [E1] |
| `aiRequestFailed` — §13.3 | 원인이 일시적인지 아닌지에 따라 다르다 |
| `aiResponseUnusable` — **true** | **여기서 전사와 갈린다.** `TranscriptionOutputUnusable`은 `false`다 — "같은 입력은 같은 출력을 낸다"는 주석이 그 이유다 [E1]. 그러나 **LLM 생성은 결정론적이지 않다.** 같은 프롬프트가 다음 번에는 파싱 가능한 JSON을 낼 수 있고, 그것이 로컬 소형 모델에서 실제로 일어나는 일이다. 그래서 재시도는 헛수고가 아니다 |
| `aiInputTooLarge` — **false** | 입력도 예산도 그대로면 결과도 그대로다. 사용자가 `ai_context_tokens`를 키우거나 다른 모델·다른 녹음을 골라야 한다. ⚠️ **앞의 둘은 구현에 없다** — 예산을 키우는 설정이 없고 예산이 모델과 무관하므로, 지금 남는 수단은 더 짧은 녹음뿐이다 (§17.3.2) |

**여섯 개 전부 `source_data_safe: true`다.** AI 경로는 audio도 `transcripts`도
`transcript_segments`도 쓰지 않는다. 실패했을 때 바뀌는 것은 `recordings.ai_status`
하나뿐이며 (§9.4), 이것이 INV-3의 표현이다. 전사 경계가 네 실패 모두 `source_data_safe`를
내리지 않은 것과 같다 [E1].

### 13.3 `aiRequestFailed`의 재시도 규칙

| 상황 | `retryable` |
| --- | --- |
| 연결 자체가 안 됨 (connection refused 등) | — `aiProviderUnreachable`로 간다 |
| 타임아웃 | **true** |
| 5xx | **true** |
| 4xx | **false** — 우리가 보낸 요청이 잘못됐다는 뜻이며, 같은 요청을 다시 보내도 같다 |

**⚠️ 여기에 §3.1의 제약이 걸린다.** "모델이 없을 때 Ollama가 어떤 상태 코드와 본문을
돌려주는가"는 **[E4] UNVERIFIED다.** 그래서 설계가 그것에 의존하지 않게 했다.

> **모델 존재 여부는 생성 응답이 아니라 `GET /api/tags`로 먼저 판정한다** (§6.4).
> 생성 요청 전에 목록을 확인하고, 고른 모델이 목록에 없으면 그때 `aiModelUnavailable`이다.
> 생성 응답의 상태 코드 해석이 틀려도 **분류가 틀릴 뿐 제품이 깨지지 않으며**,
> 그 경우에도 `aiRequestFailed`라는 보이는 상태로 끝난다.

구현 Task가 실제 응답 형태를 확인하면 그때 이 규칙을 좁힌다.

### 13.4 §13에 있지만 지금 만들지 않는 실패

§13의 AI Provider 행에는 **`rate limit`과 `인증 실패`**도 있다. 이 Phase에서는 만들지 않는다.

- 이 Phase의 유일한 provider는 **사용자의 로컬 Ollama**다. 요금 제한도 인증도 없다
  (§14.5에 인증에 대한 기록이 없다).
- 그 둘은 **cloud provider의 실패이며 cloud provider는 DEFERRED다** (§16 · §15).
- `domain/failure.rs`는 이미 이 규칙을 적어 두었다 — "만들지 않은 실패의 자리를 미리 만들어
  두지 않는다 (§20.6)" [E1]. 전사 실패 넷도 전사가 실재하는 Phase에서 추가됐다.
- 만에 하나 로컬 Ollama가 401/429를 돌려주면 §13.3의 4xx 규칙에 따라 `aiRequestFailed`로
  간다. 사용자에게 보이고 원본은 안전하다.

### 13.5 어떤 실패도 core pipeline을 막지 않는다 (INV-8)

여섯 실패 중 어느 것도 녹음 · 전사 · 열람 · (Phase 5의) Markdown export 경로에 닿지 않는다.
provider가 하나도 설정되지 않은 상태에서 §17.1의 흐름 전체가 동작하는 것이
**이 Phase의 가장 중요한 검증 항목 중 하나**이며, 그것은 테스트로 확인된다 (Goal 요구 16).

---

## 14. §14.5 외부 사실 재확인 — VERIFIED / UNVERIFIED

### 14.1 재확인 시도와 결과

| | |
| --- | --- |
| 재확인 시도 시점 | **2026-09-03** (이 Run) |
| 결과 | **불가 — 네트워크 도구 권한 거부** (§3.1 · 증거: `.loop/evidence/TASK-032/verification-log.md`) |
| 따라서 아래 표의 확인 시점 | **2026-09-01** (PRODUCT-SPEC §14.5가 기록한 시점) |

**이 문서는 §14.5의 어떤 값도 2026-09-03에 확인했다고 주장하지 않는다.**

### 14.2 항목별 상태

| # | 항목 | 상태 | 확인 시점 | 근거(출처) |
| --- | --- | --- | --- | --- |
| 1 | 기본 주소 `http://localhost:11434` (기본 bind `127.0.0.1:11434`, `OLLAMA_HOST`로 변경) | **VERIFIED** | 2026-09-01 | PRODUCT-SPEC §14.5 — ollama/ollama `docs/api.md` · `docs/faq.mdx` · `server/routes.go` |
| 2 | 모델 목록 `GET /api/tags` → `{"models":[{name, model, modified_at, size, digest, details{...}}]}` | **VERIFIED** | 2026-09-01 | 같음 |
| 3 | 생성 `POST /api/generate` · `POST /api/chat` 둘 다 존재 | **VERIFIED** | 2026-09-01 | 같음 |
| 4 | `stream` 파라미터 — 기본은 스트리밍 JSON 시퀀스, `"stream": false`면 단일 JSON | **VERIFIED** | 2026-09-01 | 같음 |
| 5 | `format` 파라미터가 문자열 `"json"` **또는 완전한 JSON Schema 객체**를 받고, `/api/generate`·`/api/chat` 양쪽에서 동작 | **VERIFIED** | 2026-09-01 | 같음 (§14.5가 요청 본문 예시까지 기록) |
| 6 | JSON-schema `format` 지원이 **v0.5(2024-12)에 도입**됐다 | **VERIFIED-by-secondary-source** [E3] | 2026-09-01 | §14.5가 2차 출처 기반임을 명시. **도입 버전을 구현 근거로 쓰지 않는다** (§6.2) |
| 7 | 헬스 체크 `GET /` → 200 `"Ollama is running"` · `GET /api/version` | **VERIFIED** | 2026-09-01 | §14.5 — "서버 소스에서 직접 확인" |
| 8 | 미실행 시 TCP 연결 거부 (connection refused) | **VERIFIED** | 2026-09-01 | 같음 |
| 9 | **CORS** 기본 허용 origin은 `127.0.0.1`·`0.0.0.0`뿐이며 Tauri webview origin은 기본 목록에 **없다** (`OLLAMA_ORIGINS`로 확장) | **VERIFIED** | 2026-09-01 | 같음. **§5의 결정 근거** |
| 10 | **기본 context window 4096 토큰**, `options.num_ctx`로 요청별 지정, `OLLAMA_CONTEXT_LENGTH`로 서버 기본값 변경 | **VERIFIED** | 2026-09-01 | 같음. **§8의 결정 근거** |
| 11 | 1시간 transcript ≈ 8~12K 단어 / 12~20K+ 토큰 | **추정치** — 측정값이 아니다 | 2026-09-01 | §14.5가 "대략"으로 적은 추정. §8.4에서 시작값의 근거로만 쓴다 |
| 12 | **공식 Rust SDK가 없다.** `ollama-rs` 0.3.6은 커뮤니티 | **VERIFIED** | 2026-09-01 | 같음. **§12의 결정 근거** |
| 13 | Ollama 최신 릴리스 v0.33.2 (2026-08-27) | **VERIFIED (그 시점 기준)** | 2026-09-01 | github.com/ollama/ollama/releases. **오늘 기준 최신인지는 [E4]** |
| 14 | OpenAI 호환 `/v1/*`는 요청별 context 크기를 지정할 수 없다 | **VERIFIED** | 2026-09-01 | 같음. 이 문서는 `/v1/*`를 쓰지 않는다 (§8이 `num_ctx`를 요구하므로 애초에 부적합하다) |
| 15 | **모델이 없을 때 생성 요청이 돌려주는 상태 코드/본문** | **UNVERIFIED** [E4] | — | 어디에도 기록이 없다. **§13.3이 이것에 의존하지 않게 설계했다** |
| 16 | `format`을 지원하지 않는 서버가 그것을 무시하는지 거절하는지 | **UNVERIFIED** [E4] | — | **§6.2가 이것에 의존하지 않게 설계했다** |
| 17 | Tauri webview origin이 실제로 CORS에 막히는지 (실측) | **UNVERIFIED** [E4] | — | 실행해 보지 않았다. §5.2가 "막힌다"가 아니라 "기본 목록에 없다"에 근거한다고 명시했다 |
| 18 | 어느 로컬 모델이 이 제품의 한/영 혼용 transcript에 실제로 적합한가 | **UNVERIFIED** [E4] | — | §14.5가 UNVERIFIED로 두었다. **Phase 4 Human Review 항목이며 이 문서가 대신 판정하지 않는다** |
| 19 | `kanana`의 정확한 크기 태그 | **UNVERIFIED** [E4] | — | §14.5 · §14.8 |
| 20 | `ureq` / `reqwest`의 현재 버전 · MSRV · runtime 요구 | **UNVERIFIED** [E4] | — | 이 Run에 네트워크 없음. §12.3이 확인 항목으로 남겼다 |
| 21 | 16384 토큰을 어떤 기기·모델이 감당하는가 | **UNVERIFIED** [E4] | — | 측정하지 않았다. §8.4가 "검증된 용량 수치가 아니라 시작값"이라고 명시 |

### 14.3 구현 Task에 넘기는 재확인 의무

Ollama adapter를 실제로 쓰는 Task는 **코드를 쓰기 전에** 아래를 현재 공식 출처에서 확인하고
`.loop/evidence/<TASK-ID>/`에 결과를 남긴다. 확인하지 못하면 UNVERIFIED로 남기고 그 사실을
드러낸다 — **기억이나 추측으로 채우지 않는다.**

```text
기본 주소 · GET /api/tags 응답 필드 이름 · POST /api/generate 요청 필드 이름
stream 파라미터의 이름과 의미 · format이 받는 값의 형태 · options.num_ctx의 이름
생성 응답에서 본문 텍스트가 담기는 필드 이름
```

마지막 항목(생성 응답의 본문 필드 이름)은 §14.5에 **명시되어 있지 않다** [E4]. §6.3의
1단계가 그것을 읽으므로, 구현 전에 반드시 확인해야 하는 값이다.

---

## 15. 이 Phase의 범위에 넣지 않는 것

명시적으로 계획에서 제외한다 (`phase-prompt/04` Important Rules · Out of Scope · §15 · §16).

| 항목 | 상태 | 이 문서에서의 취급 |
| --- | --- | --- |
| **Cloud provider adapter (Claude · Gemini · Groq)** | **DEFERRED** (§16) | 구현하지 않는다. 이 문서는 그 adapter의 설계도 일정도 담지 않는다. §11.3과 §13.4에서 **"그때 가서 그 Phase가 한다"**고만 적었다 |
| **Claude Agent SDK** | **쓰지 않는다** (§9.4) | 단일 요청/응답 변환에 agentic loop가 필요한 이유가 없다. §6.4가 `/api/generate`를 고른 근거와 같은 방향이다 |
| **Ollama 번들링 · 설치 · 프로세스 생명주기 관리** | **Non-Goal** (§15) | 앱은 **사용자가 이미 실행 중인** Ollama에 연결만 한다. installer에 넣지 않고, 띄우지도 죽이지도 않는다 |
| **가격 정책 · Free Tier 의존** | **금지** (§9.4) | 이 문서의 어떤 결정도 가격이나 무료 티어를 근거로 하지 않는다. 로컬 Ollama에는 해당 개념이 없고, cloud가 오더라도 그것이 architecture dependency가 되지 않는다 |
| **AI prompt customization UI** | **DEFERRED** (§16) | §10이 프롬프트를 정적 상수로 두는 근거 중 하나다 |
| **스트리밍 응답 UI** | **Out of Scope** | `"stream": false`를 쓰는 이유다 (§6.4) |
| **RAG · 임베딩 · 여러 녹음 교차 질의** | **Non-Goal** (§15) | 그런 경계를 만들지 않는다 (§20.6) |
| **여러 Recording 일괄 AI 처리 큐** | **DEFERRED** (§16) | |
| **오디오를 AI Provider로 보내는 어떤 경로** | **금지** (INV-6) | §4.2-6이 타입으로 막는다 |

---

## 16. Consequences

### 16.1 얻는 것

- 벤더 지식이 **`ai::ollama` 한 모듈**에만 있다. 계약 · schema · 실패 타입 · 설정 · 저장소
  어디에도 Ollama라는 말이 필요 없다 (INV-9).
- **응답이 기대와 달라도 앱이 깨지지 않는다.** 그 경로가 예외가 아니라 기본 경로로 설계됐다.
- **조용히 잘린 노트가 생기지 않는다.** 넘치는 입력은 보이는 실패가 된다.
- **provenance가 지워지지 않는다.** 재생성이 이력을 남기고, 프롬프트 변경이 Gate로 강제된다.
- §14.5의 세부값이 틀렸을 때 **고쳐야 할 곳이 adapter 하나**다 (§3.1의 세 결정).

### 16.2 감수하는 것

| 대가 | 내용 |
| --- | --- |
| **1시간 transcript가 기본값으로 안 될 수 있다** | 16384가 부족하면 사용자가 설정을 키워야 한다. **대신 그 사실이 보인다** — 조용히 잘리지 않는다 (§8). ⚠️ **구현에는 키울 설정이 없다** — 보이기는 하지만 사용자가 그 자리에서 할 수 있는 일은 더 짧은 녹음을 고르는 것뿐이다 (§17.3.2). 이 대가는 예상보다 크다 |
| **재생성이 행을 쌓는다** | 지우는 경로가 없으므로 사용자가 노트를 삭제할 수 없다. 사용자 행동에만 비례해 늘고, 필요해지면 이력 UI와 함께 삭제 정책을 다시 결정한다 |
| **`format`을 쓰면서도 파싱을 전부 갖는다** | 코드가 두 겹이다. §3.1의 재확인 실패와 로컬 소형 모델의 실제 동작을 함께 감안한 값이며, 어느 한쪽만으로는 요구를 만족하지 못한다 |
| **HTTP 클라이언트 선택이 잠정이다** | §12.3의 확인이 어긋나면 `reqwest`로 바꾼다. 바뀌는 범위는 adapter 안쪽 파일 하나다 |
| **§14.5를 오늘 재확인하지 못했다** | 이 문서의 가장 큰 미결이다. §14.3이 그 의무를 구현 Task로 넘기며, 설계는 그 사이 틀림에 견디도록 세워졌다 |

### 16.3 이 문서가 답하지 않는 것 (Human Review · §18)

자동으로 판정할 수 없어 사람이 봐야 하는 것들이다. **문서가 대신 판정하지 않는다.**
**절차와 기록표는 `docs/PHASE-4-AI-NOTE-REVIEW.md`에 있으며, 2026-09-04 현재 실행되지
않았다** (§17.3.6).

- 로컬 모델이 만든 노트가 **읽을 가치가 있는가** — 이 Phase의 실질적 성공 조건이다.
- 로컬 모델이 한국어+영어 혼용 transcript를 다룰 수 있는가.
- 1시간 transcript 처리 시간이 실사용 가능한가.
- Meeting과 Study mode의 출력이 실제로 서로 다른 유용성을 갖는가.
- provider가 로컬인지 외부인지가 사용자에게 분명한가 (§4의 `Locality`가 재료를 주지만,
  화면이 그것을 분명히 보이는지는 사람이 본다).

---

## 17. 구현이 계획에서 달라진 점

> 이 문서의 머리말이 요구한 절이다 — 구현 Task가 자기 몫을 여기에 덧붙인다
> (ADR-0007 §16과 같은 방식). **§1~§16은 결정이고, 이 절은 보고다.**

### 17.1 TASK-036 — Ollama adapter (2026-09-03)

구현 위치는 `src-tauri/src/ai/ollama/` 하나이며, §16.1이 예고한 그대로다.
Evidence는 `.loop/evidence/TASK-036/`.

#### 17.1.1 §14.3의 재확인 의무 — **이 Run도 이행하지 못했다**

`WebFetch`가 다시 거부됐다 (TASK-032와 같은 제약 · 증거:
`.loop/evidence/TASK-036/verification-log.md`). 따라서 구현이 쓴 엔드포인트와 파라미터
이름의 등급은 **여전히 [E2] · 확인 시점 2026-09-01**이며, 이 Run이 새로 확인한 것은 없다.
쓴 값은 전부 §14.2의 VERIFIED 행에서 왔고 **기억이나 추측으로 채운 이름은 없다.**

#### 17.1.2 §6.3 1단계 — 필드 이름을 지어내지 않고 이름에 의존하지 않는 경로를 택했다

§14.3이 "구현 전에 반드시 확인해야 하는 값"으로 남긴 **생성 응답의 본문 텍스트 필드 이름**은
끝내 확인되지 않았다 (§14.2 항목 15·16과 같은 [E4]).

구현은 그 이름을 짓는 대신, **응답 본문(JSON 객체)의 최상위 문자열 값들을 후보로 놓고 각각에
§6.3의 2~4단계를 그대로 적용해 그 mode의 노트로 읽히는 첫 후보를 취한다**
(`ai::ollama::wire::generated_note`). 느슨해진 것은 *어느 이름에서 꺼내는가*뿐이고,
무엇을 노트로 받아들이는가는 `ai::note::parse_note`가 그대로 판정한다.

§3.1이 세운 원칙("틀렸을 때 무너지는 범위를 줄이는 쪽")의 네 번째 적용이다.
**이름이 확인되면 좁힐 자리는 함수 하나다.**

#### 17.1.3 §4.2-4의 `model` provenance — 응답이 아니라 요청이 근거다

응답이 자기가 쓴 모델을 어느 이름으로 말하는지도 [E4]다. 그래서 `NoteGeneration::model`에는
**그 요청이 지정한 모델**이 들어간다. 지정한 것이 adapter이므로 추정이 아니라 아는 사실이며,
§4.2-4가 막으려던 것("호출자가 설정값을 그대로 적어 두는 것")도 아니다.

#### 17.1.4 §12.3의 확인 항목 — 두 개는 답했고 하나는 남았다

| 항목 | 결과 |
| --- | --- |
| `ureq` 버전 | **3.4.0** — cargo가 crates.io 인덱스에서 실제로 해석했고 `Cargo.lock`이 checksum까지 고정한다 |
| 동기 API가 async runtime을 요구하는가 | **요구하지 않는다** — `default-features = false`로 컴파일되며 async 진입점이 없다. 따라서 §12.2의 차점(`reqwest`)으로 돌아가지 않았다 |
| MSRV | **여전히 [E4]** — crates.io 페이지를 열지 못했다 |

`default-features = false`로 **TLS를 켜지 않았다** (§12.2의 "Phase 4에서 켜는 것"). Phase 5가
필요해질 때 켠다.

#### 17.1.5 §8의 사전 크기 판정은 adapter에 넣지 않았다

`prompt::prepare`(넘치면 보내지 않는다)는 **adapter가 아니라 실행 순서를 만드는 쪽**에 놓인다.
§13.1의 여섯 번째 실패(`AiInputTooLarge`)가 아직 `FailureKind`에 없고, 만들지 않은 실패의
자리를 adapter가 다른 종류로 대신 채우면 사용자는 "설정에서 값을 키우면 된다"를 알 수 없게
되기 때문이다 (§20.6). adapter는 `prompt::build_prompt`로 프롬프트를 만들고 `num_ctx`를
언제나 명시한다 — §8.2의 1·2는 그대로 지켜진다.

#### 17.1.6 그 밖에 §1~§16을 바꾸지 않은 것들

- 부르는 엔드포인트는 §6.4 그대로 둘뿐이다 (`GET /api/tags` · `POST /api/generate`,
  `stream: false`). 헬스 체크를 따로 부르지 않는다.
- 모델 존재는 §13.3대로 **목록으로 먼저** 판정한다. 그 대가로 생성 한 번에 왕복이 하나 는다.
- 실패 매핑은 §13.1 · §13.3의 표 그대로이며, `AiProviderNotConfigured`는 adapter가 만들지 않는다.
- 설정값(host · port · 모델 이름)은 실패 `message`/`detail`/로그에 남지 않는다 (§11.3).
  이것은 관례가 아니라 타입이 지탱한다 — 경계 오류 타입에 문자열을 담을 자리가 없다.
- 연결 대상은 주입되며 adapter 안에 주소가 하나도 없다 (§11.1).

### 17.2 TASK-037 — AI 노트 생성 orchestration (2026-09-03)

구현 위치는 `src-tauri/src/ai/run.rs` 하나이며, 저장소에 닿는 자리도 여기뿐이다.
Evidence는 `.loop/evidence/TASK-037/`.

#### 17.2.1 §13.1의 여섯 번째 실패를 여기서 만들었다 — 그리고 provider의 다섯과 섞지 않았다

TASK-036이 남긴 자리(§17.1.5)를 이 Task가 채웠다. `FailureKind::AiInputTooLarge`가
`domain/failure.rs`에 추가됐고 frontend union도 함께 갱신됐다.

§13.1의 표는 이 실패를 만드는 주체를 **"domain (요청 전 계산 · §8.5)"**이라고 적었다. 실제
구현은 그것을 둘로 나눈다 — **계산**은 domain의 순수 함수(`prompt::prepare`)가 하고, 그 결과를
§13의 공통 실패로 **옮기는 것**은 실행 순서(`ai::run::input_too_large`)가 한다. 계약 모듈의
`AI_PROVIDER_FAILURE_KINDS`(다섯)에는 넣지 않았다: provider가 만들 수 있는 실패가 아니고,
넣으면 계약 준수 검사가 "adapter가 이 실패로 거절해도 된다"고 말하게 된다.

#### 17.2.2 입력으로 쓰는 것은 Transcript의 `raw_text`다

§7.2는 "current가 가리키는 Transcript"까지만 정하고 그 안의 **어느 필드**를 프롬프트에 싣는지는
정하지 않았다. 구현은 `raw_text`를 쓰고 `segments`를 다시 이어 붙이지 않는다 — 같은 텍스트를
두 방법으로 만들면 두 값이 언젠가 어긋나고, 그때 저장된 노트가 무엇에서 나왔는지 말할 수 없게
된다. 타임스탬프는 노트 schema(§7.2)가 요구하지 않으므로 싣지 않는다.

#### 17.2.3 §9.4의 "바뀌는 것" 목록에 **아무것도 바뀌지 않는 경우**를 하나 더한다

current Transcript가 없는 Recording은 실패가 아니라 `Outcome::NoTranscriptYet`이다 (§7.2 ·
INV-8). 이 경로는 provider를 부르지 않고 **`ai_status`도 `updated_at`도 건드리지 않는다** —
시도한 적 없는 일이 `failed`로 남으면 사용자는 자기가 하지 않은 실패를 보게 된다.
§9.4의 목록은 생성을 **시도한** 경우에 대한 것이며, 시도조차 하지 않는 경우가 하나 더 있다.

#### 17.2.4 `prepare`가 만든 프롬프트 문자열은 쓰이지 않는다

실제 요청 본문을 만드는 것은 adapter다 (§6.4 · §17.1.5). 실행 순서가 `prepare`를 부르는 이유는
둘이다 — §8.2-3의 **사전 크기 판정**과 `promptVersion`. 둘을 같은 함수에서 얻으므로 저장된
버전은 언제나 **크기를 잰 바로 그 프롬프트의 버전**이다. 대가는 넘치지 않는 입력에 대해
프롬프트 문자열이 한 번 더 만들어진다는 것이고, 그 값을 provenance의 정확성과 바꿨다.

#### 17.2.5 저장 직전에 mode를 한 번 더 확인한다

계약을 지키는 provider는 요청한 mode의 노트를 돌려주므로 이 검사는 정상 경로에서 통과만 한다.
그래도 두는 이유는 `ai_notes`에 UPDATE 경로가 없기 때문이다 — `note_type` 열과 봉투의 `mode`가
어긋난 행은 읽을 때마다 `Storage` 실패가 되고(§7.5) 고칠 수 없다. 저장 직전이 그것을 막을 수
있는 마지막 자리이며, 어긋난 응답은 `AiResponseUnusable`로 끝난다.

#### 17.2.6 §1~§16을 바꾸지 않은 것들

- 재생성은 §9.2 그대로 **언제나 INSERT**다. 이 모듈에도 저장소에도 `ai_notes`를 UPDATE·DELETE
  하는 경로가 없고, 그 사실을 소스 검사 테스트가 함께 고정한다.
- 상태 전이는 `pending → running → done | failed`이며, `updated_at` 갱신 여부는 기존 전사 경로의
  규약을 그대로 따랐다(`store::update_recording_statuses` 하나만 쓴다). 다른 후처리 상태는 읽은
  값을 그대로 다시 쓴다.
- provenance 일곱 값(§7 · §7.3)이 전부 저장되며 어느 것도 추정이 아니다 — `transcript_id`는
  실제로 입력에 쓴 Transcript, `model`은 provider가 답한 값(§4.2-4), `promptVersion`은 §17.2.4의
  것이다.
- 이 모듈에는 벤더 지식이 없다 (INV-9). provider는 주입되며, 엔드포인트·주소·모델 이름이 한 번도
  나오지 않는다.

#### 17.2.7 이 Run도 실제 Ollama를 부른 적이 없다

검증은 전부 `ai::testing::FakeNoteAiProvider`와 **fixture Transcript**로 돌았다. 실제 Whisper
추론 결과도 실제 Ollama 응답도 쓰지 않았으므로, `A-TRANS-001`과 §16.3의 Human Review 항목은
**여전히 유효하다.** 이 절의 어떤 문장도 실제 모델이 만든 노트의 품질에 대해 말하지 않는다.

### 17.3 TASK-042 — 결정과 구현의 대조 (2026-09-04)

> 이 절은 **새 결정이 아니다.** §1~§16의 각 항목을 저장소의 실제 소스와 대조한 결과이며,
> 어긋난 곳은 **코드가 아니라 이 문서를 고쳤다** (Task 범위: 문서 전용). 어긋남이 설계 결함으로
> 보이는 곳은 고치지 않고 **사실로 남겼다** — 판단은 그것을 고칠 Task의 몫이다.
> 대조 방법은 [E1] 하나다: 이 Run이 아래 파일들을 직접 읽었다.

#### 17.3.1 일곱 항목의 대조 결과

| 항목 | 결정 | 구현 | 판정 |
| --- | --- | --- | --- |
| **최종 schema** (§7.2) | 세 mode · 13개 필드 · `string`과 `string[]`뿐 · 키는 언제나 있고 배열은 비어도 된다 | `ai/note.rs`의 `MeetingNote` 5 · `StudyNote` 6 · `SummaryNote` 2 = **13**, 이름·순서·필수 여부가 §7.2 표와 같다. 상한(§7.4)도 `MAX_ITEMS 200` · `MAX_ITEM_CHARS 2_000` · `MAX_OVERVIEW_CHARS 4_000`으로 같다 | **일치** |
| **저장 봉투** (§7.5) | `{schemaVersion, mode, note}` compact JSON 한 문자열 · 모르는 `schemaVersion`과 `mode` 불일치는 `Storage` 실패 | `encode_content` / `decode_content`가 그대로다. `CONTENT_SCHEMA_VERSION = 1`, 키 순서는 선언 순서로 고정, 읽기 실패 셋(형태 · 새 버전 · mode 불일치)이 전부 `FailureKind::Storage` | **일치** |
| **구조화 출력 수단** (§6) | `format`에 mode별 JSON Schema를 싣되 정확성은 파싱에 건다 | `ollama/wire.rs::generate_body`가 `format`에 `note::json_schema(mode)`를 그대로 싣고 `stream: false`를 보낸다. 응답은 `note::parse_note`가 §6.3의 4단계로 검증한다 | **일치** |
| **호출 주체** (§5) | Rust backend. frontend에 Ollama로 가는 경로가 없다 | 소켓을 여는 파일은 `ai/ollama/network.rs` 하나이며, frontend는 Tauri command만 부른다 (`src/ipc/commands.ts`) | **일치** |
| **context 전략** (§8) | `num_ctx` 항상 명시 · 청킹도 절단도 없음 · 보내기 전 사전 판정 · **값은 설정에서 온다** | 앞의 셋은 그대로다 (`wire::generate_body`의 `options.num_ctx` · `prompt::prepare`). **넷째는 없다** — §17.3.2 | **셋 일치 · 하나 미구현** |
| **재생성 정책** (§9) | append-only. UPDATE·DELETE 경로를 만들지 않는다 | `ai/run.rs`가 부르는 저장소 쓰기는 `store::insert_ai_note` 하나뿐이고, 화면은 `(transcriptId, mode)`의 마지막 노트를 고른다 (`aiNoteView.ts::latestNote`) | **일치** |
| **`promptVersion` 정책** (§10) | 정적 프롬프트 상수 · `v<SET>.<mode>.<hash8>` · 프롬프트/schema/`schemaVersion` 해시 · 테스트가 강제 · FNV-1a | `ai/prompt.rs` 그대로다. 선언값은 `v1.meeting.5c6b8a90` · `v1.study.2cfad9a0` · `v1.summary.beca7d6c`이며, `prompt_version_is_bound_to_the_prompt_text`가 선언값과 계산값을 비교해 깨진다 | **일치** |
| **실패 매핑** (§13) | 여섯 `FailureKind` · 각각의 `retryable` · frontend union과 1:1 | `domain/failure.rs`에 여섯이 있고 문자열·`retryable`·`source_data_safe`가 §13.1 표와 같다. `src/ipc/failure.ts`의 union도 여섯을 그대로 갖는다. 4xx/그 밖의 비2xx 구분은 `ollama/provider.rs::status_failure`가, 타임아웃/연결 거부 구분은 `network.rs::classify`가 한다 | **일치** (덧붙은 사용처 하나 — §17.3.4) |

**§4.2가 적은 §9.2 예시로부터의 여섯 가지 이탈은 전부 구현에 그대로 있고, 여섯 개 모두 그
자리에 이유가 적혀 있다** — Rust `trait` · 3상태 `Availability` · `Result<_, Failure>` ·
`NoteGeneration{note, model}` · `NoteType` 재사용 · `TranscriptText{text}`. 새로 생긴 이탈은
없다 (`ai/provider.rs`).

#### 17.3.2 ⚠️ `ai_context_tokens`는 만들어지지 않았다 — 결정 중 유일하게 구현되지 않은 항목

| | |
| --- | --- |
| 결정 | §2(e) · §8.2-2 · §8.4 · §11.1 — 예산 값을 설정 열 `ai_context_tokens`로 노출하고 기본값 16384를 쓴다 |
| 구현 | migration version 6이 더한 열은 **`ai_provider` · `ai_base_url` · `ai_model` 셋**이다 (`db/migrations.rs`). `Settings`에도 그 필드가 없고 (`domain/settings.rs`), 생성 경로는 언제나 `ContextBudget::DEFAULT`(16384)를 넘긴다 (`commands/notes.rs`) |
| 상태 | **미구현** — 이 문서가 결정을 취소한 것이 아니다. 만들어지지 않았다는 사실을 적을 뿐이다 |

**이것이 설계 결함으로 보이는 이유를 그대로 남긴다.** §8은 "넘칠 때 조용히 잘리는 대신
사용자에게 보인다"를 채택한 근거로 **사용자가 할 수 있는 일이 분명하다**는 것을 들었고,
그 목록의 첫째가 "설정에서 값을 키운다"였다. 지금은 그 수단이 없다.

- 예산은 모델과 무관하게 고정이므로 **더 큰 context의 모델을 골라도 예산이 커지지 않는다.**
  화면이 보여 주는 안내(`aiNoteView.ts`의 `inputTooLarge` 문장)는 "더 큰 context window를 가진
  모델, 또는 더 짧은 녹음이 필요하다"고 말하지만, 앞쪽 절반은 이 구현에서 효과가 없다.
- 실제로 남는 수단은 **더 짧은 녹음** 하나다.
- 입력 예산은 `16384 − 1024 − 1536 = 13,824` 토큰이고 추정은 `문자수 ÷ 2`이므로, 대략
  **27,648자를 넘는 transcript는 요청조차 나가지 않는다.** §14.2 항목 11의 추정(1시간 ≈ 8~12K
  단어 / 12~20K+ 토큰)에 비추면 **1시간 분량이 이 문턱에 걸릴 수 있다** — 그리고 그것은
  §16.3의 Human Review 항목("1시간 분량 처리 시간이 실사용 가능한가")이 판정하려던 바로 그
  대상이다. `docs/PHASE-4-AI-NOTE-REVIEW.md` §7이 이 경우를 관찰 항목으로 다룬다.
- **이 문턱이 실제로 걸리는지는 [E4] UNVERIFIED다.** 실제 전사도 실제 생성도 실행된 적이
  없으므로 (§17.3.6), 여기서 "걸린다"고 단정하지 않는다. 계산은 상수에서 나온 값이다.

#### 17.3.3 기본 주소를 아는 자리는 adapter가 아니라 domain이다

§11.1의 표는 `ai_base_url`이 `NULL`일 때 "adapter의 기본 주소를 쓴다"고 적었다. 구현은
그 값을 **`domain::settings::DEFAULT_AI_BASE_URL`(`"http://localhost:11434"`) 한 곳**에 두고,
`Settings::ai_base_url_or_default()`가 답한다. adapter에는 주소가 하나도 없다
(`ai/ollama/provider.rs` — `base_url`은 생성자 인자다).

**§11.1이 요구한 것("이 값을 코드 여러 곳에 흩어 두지 않는다")은 지켜졌고, 그 한 곳이 어디인지만
다르다.** 벤더 중립성 관점에서는 논의할 여지가 있다 — `localhost:11434`는 Ollama의 값이며,
그것이 domain에 있다. 다만 그 문자열을 **해석하는** 지식(엔드포인트 · 파라미터 이름)은 여전히
`ai/ollama/` 안에만 있고, 소스 검사 테스트(`tests/ollama_adapter.rs`)가 그것을 고정한다.

#### 17.3.4 `AiProviderNotConfigured`가 실제로 뜻하는 것 둘

§13.1의 표는 이 실패를 **"provider 미설정"** 하나에 대응시켰다. 구현은 **같은 실패를 두 상황에
쓴다** (`commands/notes.rs::Providers::to_generate_with`).

```text
설정에 ai_provider가 없다        → AiProviderNotConfigured
설정에 ai_model이 없다           → AiProviderNotConfigured   ← 표에 없던 두 번째 상황
```

`AiModelUnavailable`을 쓰지 않은 것은 그 실패가 **"서버에 그 모델이 없다"**를 뜻하기 때문이다
(§13.1 · `ollama/provider.rs`) — 아직 아무것도 고르지 않은 것과 고른 것이 서버에 없는 것은
사용자가 할 일이 다르다. 둘 다 "설정에서 골라야 풀린다"는 점에서 `retryable: false`이며 같다.
**§13.1의 표에 그 상황이 없었다는 것이 이 절이 남기는 사실이다.**

#### 17.3.5 결정에 없던 구현 사실 셋

이 문서가 정하지 않았지만 구현이 정해야 했던 값들이다. 어느 것도 §1~§16을 뒤집지 않는다.

| 사실 | 어디 | 근거 |
| --- | --- | --- |
| **연결 타임아웃 5초, 생성에는 시간 제한 없음** | `ai/ollama/network.rs::CONNECT_TIMEOUT` · `commands/notes.rs` | 연결이 서지 않는 서버를 오래 붙들지 않되, 로컬 모델의 생성은 분 단위가 될 수 있으므로 앱이 임의로 끊지 않는다. 끊으면 사용자는 §13이 설명할 수 없는 실패를 본다 |
| **생성 중 화면이 2초마다 상태를 다시 묻는다** | `src/screens/RecordingDetailScreen.tsx::AI_NOTE_REFRESH_MS` | 상태 조회가 생성을 기다리지 않으므로(자물쇠를 쥐지 않는다) 폴링이 생성을 늦추지 않는다 |
| **한 번에 한 건. 생성 중 다른 시작 요청은 거절된다** | `commands/notes.rs::already_running` | 여러 Recording 일괄 처리 큐는 DEFERRED다 (§15). 재생성이 추가이므로 같은 입력에 노트가 이유 없이 둘 생기지 않게 한다 |
| **설정 화면의 연결 확인은 "저장된" 값을 묻는다** | `src/screens/aiProviderSettings.ts::AI_CHECK_USES_SAVED_SETTINGS` | provider를 만드는 것은 저장된 `Settings`이며, 화면의 편집 중인 값이 아니다. 화면이 그 사실을 문장으로 밝힌다 |

#### 17.3.6 ⚠️ 이 Phase는 실제 Ollama를 한 번도 호출하지 않았다 — 그리고 A-TRANS-001은 그대로다

```text
Phase 4 engineering:                    구현됨 (TASK-033 ~ TASK-041)
Automated verification:                 PASS  (실제 서버 · 실제 네트워크 없이)
실제 Ollama 호출이 일어난 적:           없다        UNVERIFIED [E4]
실제 Whisper 추론이 일어난 적:          없다        ASSUMPTION A-TRANS-001 유효
실제 transcript로 만든 노트의 품질:     판정 안 됨  Human Review 대기 (§16.3)
```

- **자동 검증은 `ai::testing::FakeNoteAiProvider`와 `ollama::testing::StubServer`로만 돈다.**
  `network.rs`(소켓을 여는 유일한 파일)는 Gate가 **컴파일**할 뿐 실행하지 않는다.
- **`A-TRANS-001`(실제 Whisper 추론 미실행)은 여전히 유효하다** — 이 Phase의 어떤 Task도
  실제 전사를 돌리지 않았고, 입력은 전부 fixture Transcript였다
  (`ADR-0007` §16.3.1 · `PRODUCT-SPEC` §14.4.4 · `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` §11).
- 따라서 **이 문서의 어떤 문장도 실제 모델이 만든 노트의 품질 · 처리 시간 · 한국어+영어 혼용
  처리 결과에 대해 말하지 않는다.** 그 다섯 질문은 `docs/PHASE-4-AI-NOTE-REVIEW.md`가 절차로
  적었고, 그 문서의 기록표는 아직 비어 있다.
- §14.3이 구현 Task에 넘긴 재확인 의무(엔드포인트 · 파라미터 이름 · 생성 응답의 본문 필드
  이름)도 **끝내 이행되지 않았다** (§17.1.1 · §17.1.2). 네트워크 도구가 세 Run 연속으로
  거부됐고, 이 Run(TASK-042)도 문서 전용이라 시도하지 않았다. 등급은 여전히
  **[E2] · 확인 시점 2026-09-01**이다.
