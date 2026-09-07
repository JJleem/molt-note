# ADR-0010 — Manual AI Handoff는 §11의 산출물과 §9.5의 프롬프트에서 갈라지고, clipboard는 경계 하나다

```text
Status:   Accepted (결정) · **구현됨** (2026-09-06 · TASK-056 ~ TASK-064) — 계획과 실제가
          달라진 것은 §12에 있다. **§4~§9의 결정 절은 고쳐 쓰지 않았다.**
          ⚠️ 구현됐다는 것은 **자동 검증을 통과했다**는 뜻이며, 사람이 실제 외부 AI 채팅에서
          확인했다는 뜻이 아니다 (§12.4 · `docs/PHASE-5.5-HUMAN-REVIEW.md`).
          **2026-09-07에 크기와 나눔이 더해졌다 (§12.7 · §12.8)** — 긴 회의의 산출물이
          크기 때문에 조용히 실패하지 않게 한다. **§11 export 파일 형식은 바뀌지 않았다.**
          그것이 실제 AI 채팅에서 쓸 만한지도 여전히 사람이 판정한다
          (`docs/PHASE-5.6-HUMAN-REVIEW.md` HR-5).
Date:     2026-09-04 (결정) · 2026-09-06 (§12 확정) ·
          2026-09-07 (§12.7 command 표면 · §12.8 크기 결정)
Phase:    Phase 5.5 — Manual AI Handoff + UI Foundation (**로드맵에 없던 삽입** · §4) ·
          Phase 5.6 — Transcription Correctness + Reach (§12.7 · §12.8)
Task:     TASK-055 (작성) · TASK-065 (§12 확정) · TASK-073 (§12.7) · TASK-075 (§12.8)
Scope:    두 제품 방향 변경의 기록 · AI-ready 산출물 셋의 형식과 순서와 timestamp 표현 ·
          `export::markdown` 재사용 방식 · `ai::prompt` 상수 재사용 방식 ·
          clipboard 경계의 자리 · 이 Phase가 여는 command 이름과 개수 ·
          MH-1~MH-8의 판정 수단 · 벤더 중립 ·
          **산출물의 크기와 나눔의 예산 · 나눔 규칙 · 압축 모양 (§12.8 · 2026-09-07 추가)**
```

> **§4~§9는 결정이다.** 결정 절을 사후에 고쳐 쓰지 않는다 — 그것을 고치면 무엇을 정했고 무엇이
> 실제로 만들어졌는지가 구분되지 않는다 (ADR-0007 §16 · ADR-0008 §17 · ADR-0009 §15와 같은 방식).
> 구현이 결정과 달라지면 그 자리에 §12로 보내는 표시를 달고, 달라진 내용은 §12가 적는다.

---

## 1. Context

Phase 4는 provider 추상화를 세웠고 Phase 5는 나가는 문 두 개(Markdown export · Notion)를
만들었다. 그런데 지금 이 제품에서 AI Note를 얻으려면 사용자가 **Ollama를 설치하고 모델을 받아야
한다.** 대부분의 사람은 이미 쓰는 AI 채팅이 있다.

INV-8은 "AI가 없어도 제품이 동작한다"를 요구했다. Phase 5.5는 거기서 한 걸음 더 간다 —
**AI Provider가 하나도 없어도 AI의 값을 얻을 수 있게 한다.** 방법은 자동화가 아니라 **사람이
가져갈 수 있는 형태**다.

```text
Recording + current Transcript
        │
        ├──▶ Manual 프롬프트 텍스트 ──▶ (사람이) 붙여 넣기 ──▶ 사용자의 AI 채팅
        ├──▶ Transcript 텍스트 ───────▶ (사람이) 붙여 넣기 ──▶ 사용자의 AI 채팅
        └──▶ AI-ready Markdown 문서 ──▶ exports/…-ai-request.md ──▶ (사람이) 첨부
```

**앱은 이 경로에서 네트워크로 아무것도 보내지 않는다** (MH-3). 나가는 행위의 주체는 사람이고,
앱이 하는 일은 값에서 문자열을 만들고, clipboard에 쓰고, 파일 하나를 더하는 것까지다.

되돌리기 어려운 결정이 여섯 개 있다. 이 문서가 그 여섯 개를 확정한다. 전부 같은 질문에 답한다.

```text
이 결정이 틀리거나 바뀌면 무엇이 함께 무너지는가.
```

무너지는 범위가 **모듈 하나 · 상수 하나 · 파일 하나**로 끝나는 쪽을 택했다. 특히 이 Phase에는
**앱이 확인하지 못한 플랫폼 사실 위에 서는 자리가 하나** 있다(webview의 clipboard 쓰기 능력).
그 하나가 틀렸을 때 **조용히 아무 일도 일어나지 않는 대신 보이게 실패하고, 그때도 사용자가
목적을 달성할 다른 길이 남는가**가 §7의 설계 기준이 됐다.

---

## 2. Decision — 여섯 개 결정 요약

| | 항목 | 결정 |
| --- | --- | --- |
| **(1)** | 두 제품 방향 변경 | Manual AI Handoff를 정식 제품 기능으로 **추가**하고, Ollama를 '첫 provider · 기본 AI 경험'에서 '선택적 · 로컬 · 고급'으로 **재배치**한다. **삭제가 아니다.** 두 경로는 공존한다. Spec 본문(§9 · §14.5 · §21) 갱신은 이 Phase의 마지막 문서 Task(P11)가 한다 (§4) |
| **(2)** | AI-ready 산출물의 형식 | 산출물은 **셋**이다 — Manual 프롬프트 · Transcript 텍스트 · AI-ready 문서. 문서의 섹션은 **`# Molt Note AI Request` → `## Mode` → `## Instructions` → `## Recording` → `## Transcript`** 순서로 고정한다. timestamp는 §11과 **같은** `### HH:MM:SS`이며 [`format_timestamp_ms`]가 만든다. **두 번째 export 시스템도 두 번째 렌더링 규칙도 만들지 않는다** (§5) |
| **(3)** | `export::markdown` 재사용 | 입력은 기존 [`ExportDocument`]를 그대로 쓰고(`note`는 이 경로에서 언제나 `None`), transcript 본문·메타데이터 블록을 만드는 함수를 `pub(crate)`로 **끌어올려 한 자리에서** 쓴다. `markdown::render`가 내는 바이트는 하나도 바뀌지 않는다 — 기존 golden 테스트가 그것을 지킨다 (§5.4) |
| **(4)** | `ai::prompt` 재사용 | `MEETING_PROMPT` · `STUDY_PROMPT` · `SUMMARY_PROMPT` · `TRANSCRIPT_PLACEHOLDER` · `PROMPT_VERSION_*`을 **한 글자도 고치지 않는다.** Manual 프롬프트는 상수 원문에 **치환 두 번**만 적용해 만든다 — (a) 세 상수에 공통으로 있는 JSON 출력 계약 블록 하나를 Markdown 출력 계약으로, (b) 기존 seam인 `{{transcript}}`를 상황에 따라 transcript 본문 또는 "본문은 아래 `## Transcript`에 있다"는 한 문장으로. **두 번째 프롬프트 세트를 만들지 않는다.** 이 경로는 `promptVersion`을 만들지도 저장하지도 않는다 (§6) |
| **(5)** | clipboard 경계 | **프론트엔드의 작은 platform 모듈 하나** — `src/platform/clipboard.ts`. 새 의존성 없음 · 새 command 없음 · 새 `FailureKind` 없음. Rust command와 Tauri clipboard 플러그인은 탈락시키되 근거를 §7.3에 남긴다. **webview의 clipboard 쓰기 능력은 이 Run에서 확인하지 못했다 — UNVERIFIED다.** 그래서 실패는 보이는 상태가 되고 재시도 가능하며, **clipboard가 거절되는 환경에서도 Export for AI가 대체 경로로 남는다** (§7) |
| **(6)** | command 이름과 개수 | **세 개** — `get_ai_prompt` · `get_transcript_text` · `export_ai_request`. 등록 command는 28 → **31**이 된다. `tests/ipc-boundary.test.ts`의 **네 자리**가 이 이름들에서 깨진다 — 그것은 의도된 tripwire이며 §8.3이 어디가 왜 깨지는지 미리 적는다. 이름에도 산출물에도 core/domain에도 벤더 이름을 넣지 않는다 (§8 · §10) |

이 Phase에서 **하지 않는 것**은 §11에 따로 적었다.

[`format_timestamp_ms`]: ../src-tauri/src/export/markdown.rs
[`ExportDocument`]: ../src-tauri/src/export/markdown.rs

---

## 3. 근거의 종류 — 이 Run이 확인할 수 있었던 범위

**추측한 것을 확인한 것처럼 적지 않는다** (PRODUCT-SPEC §20.2 · ADR-0009 §3과 같은 표기를 쓴다).

| 표기 | 뜻 |
| --- | --- |
| **[E1] 직접 확인** | 이 Run에서 이 저장소의 실제 파일을 읽어 확인했다 |
| **[E4] UNVERIFIED** | 확인하지 못했다. **확인된 사실로 쓰지 않는다** |
| **[A] 앱이 고른 값** | 외부 사실이 아니라 이 앱이 정한 값 |

이 Run이 [E1]로 확인한 것:

```text
src-tauri/src/export/markdown.rs   ExportDocument · render · format_timestamp_ms ·
                                   MEETING/STUDY/SUMMARY_SECTIONS · TRANSCRIPT_SECTION
                                   transcript_blocks · heading_text · date_text (셋은 private)
src-tauri/src/export/filename.rs   export_file_name(created_at, title) · MARKDOWN_EXTENSION
src-tauri/src/export/run.rs        current_transcript_id만 고르는 순서 · 저장소에 쓰지 않는다
src-tauri/src/commands/export.rs   ensure_exports_dir → export::run::export → ExportedFilePayload
src-tauri/src/ai/prompt.rs         세 프롬프트 상수 · TRANSCRIPT_PLACEHOLDER ·
                                   PROMPT_VERSION_* 선언값 · computed_prompt_version ·
                                   prompt_version_is_bound_to_the_prompt_text 테스트
src-tauri/src/domain/mod.rs        NoteType::as_str = "meeting"|"study"|"summary"
src/ipc/failure.ts                 FailureKind 20종 · toFailure가 만드는 유일한 프론트 종류
tests/ipc-boundary.test.ts         REGISTERED_COMMANDS 28개 · outOfScope 정규식 · 표면별 assert
src-tauri/Cargo.lock · package-lock.json · tauri.conf.json · capabilities/default.json
                                   **clipboard 관련 항목이 하나도 없다** (grep 결과 0건)
```

이 Run이 [E4] UNVERIFIED로 남기는 것은 §7.4에 모아 두었다.

---

## 4. 결정 (1) — 로드맵에 없던 삽입과 두 제품 방향 변경 (2026-09-04)

### 4.1 이 Phase는 계획에 없었다

`PRODUCT-SPEC.md` §21의 Phase Roadmap에는 **Phase 5.5가 없다.** Manual AI Handoff라는 개념도
Product Spec에 없다. 이 Phase는 운영자가 **2026-09-04에** Phase 5와 Phase 6 사이에 넣기로 한
삽입이며, 그 사실을 지우지 않는다. 근거는 `phase-prompt/05.5-manual-ai-handoff-and-ui-foundation.md`
상단에 남아 있고, 이 문서가 결정 쪽을 맡는다.

**왜 지금 넣는가.** Phase 6(Windows 검증)과 Final Integration에서 사람이 앱을 처음 실제로 쓴다.
그 시점에 "AI Note를 보려면 먼저 Ollama를 설치하세요"가 유일한 길이면, 검증되는 것은 제품이
아니라 설치 안내다. §17.1이 정의한 core(AI 없이 성립하는 제품)는 이미 서 있으므로, 그 위에
**AI 없이 AI의 값을 얻는 길**을 하나 놓는 것이 Phase 6보다 먼저 올 값이 있다고 판단했다.

### 4.2 변경 1 — Manual AI Handoff를 제품 기능으로 **추가**한다

| | |
| --- | --- |
| 날짜 | 2026-09-04 |
| 결정 | D-1 · 운영자 Human Review에서 **APPROVED** |
| 무엇이 바뀌는가 | Transcript를 외부 AI 채팅에 **사람이 직접 넘기는 경로**가 제품 기능이 된다 |
| 근거 | §11(Markdown interoperability)의 연장이다 — 이미 "기록을 앱 밖으로 가져갈 수 있다"가 제품 원칙이고, 여기서 늘어나는 것은 **가져가는 형태 하나**다. 반면 "AI에 줄 형태"는 Spec에 없던 새 개념이므로 추가로 기록한다 |
| 무엇이 늘지 않는가 | provider도, 네트워크 경로도, 저장되는 데이터도 늘지 않는다. AI Note 레코드는 이 경로에서 만들어지지 않는다 (§6.4) |

**`A-HANDOFF-001`을 만들지 않는다** (D-4 · 2026-09-04). Manual AI Handoff는 외부 provider 없이
자동 검증되므로 `A-REC-001` · `A-TRANS-001` · `A-AI-001` · `A-NOTION-001`과 같은 hard deferred
assumption이 아니다. 대신 P11의 Human Review 항목 셋으로 남는다 — 그것은 **주관적 검토 항목이지
`A-` assumption이 아니다.**

**Open ChatGPT / Open Claude를 구현하지 않는다** (D-5 · 2026-09-04). 브라우저 열기는 이후의 작은
polish item이다. 본질은 딥링크 자동화가 아니라 **가져갈 수 있는 AI-ready 내용**이다.

### 4.3 변경 2 — Ollama를 '선택적 · 로컬 · 고급'으로 **재배치**한다

| | |
| --- | --- |
| 날짜 | 2026-09-04 |
| 결정 | D-2 · 운영자 Human Review에서 **APPROVED** |
| 무엇이 바뀌는가 | Ollama의 **제품상 위치**가 '첫 provider · 기본 AI 경험'에서 '선택적 · 로컬 · 고급'으로 내려간다. §14.5 · §21의 서술과 어긋나므로 P11이 Spec 본문을 갱신한다 |
| **삭제가 아니다** | `src-tauri/src/ai/provider.rs` · `ai/ollama/` · provider 설정 영속화 · 연결 확인 · 모델 선택 · 로컬/외부 표시(INV-5)를 **그대로 둔다.** 바뀌는 것은 Settings에서의 **자리와 언어**다 |
| 근거 | Phase 4가 만든 것은 "Ollama 기능"이 아니라 **provider 추상화**다. 그 추상화는 Manual Handoff가 생겨도 그대로 유효하고, 지우면 §21의 DEFERRED cloud provider가 돌아올 자리도 함께 사라진다. 유지 비용이 낮고(파일 몇 개 · 테스트가 이미 있다) 재배치 비용도 낮다 — **적극적으로 해로운 경우가 아니면 삭제하지 않는다** |
| 무엇이 늘지 않는가 | 새 cloud provider(OpenAI · Claude · Gemini · Codex)를 붙이지 않는다. 그것은 §16의 DEFERRED다 |

> **두 변경은 서로를 대체하지 않는다.** Connected Provider 경로와 Manual AI Handoff 경로는
> **공존한다** (MH-8). 사용자가 Ollama를 설정했다면 자동 생성이 그대로 동작하고, 설정하지
> 않았다면 그것이 오류가 아니라 정상 상태다 (INV-8 · §13).

### 4.4 Spec 본문은 여기서 고치지 않는다

이 Task는 **문서 하나만 만든다.** `PRODUCT-SPEC.md`의 §9 · §14.5 · §21 갱신, `SYSTEM-MAP.md`
반영, Human Review 절차 문서는 **P11(TASK-065)**의 몫이다. 그때도 규칙은 하나다 —
**소급해서 지우지 않는다.** 이전 서술이 무엇이었고 언제 왜 바뀌었는지가 남아야 한다.

---

## 5. 결정 (2)(3) — AI-ready 산출물의 형식과 `export::markdown` 재사용

### 5.1 산출물은 셋이고, 전부 같은 두 재료에서 나온다

```text
                    ┌─ ai::prompt의 상수 (고치지 않는다) ─────── §6
재료는 둘뿐이다 ────┤
                    └─ export::markdown의 렌더링 규칙 ────────── §5.4

Manual 프롬프트     = 상수 + 출력계약 치환 + {{transcript}} ← transcript 본문
Transcript 텍스트   = `## Transcript` 블록들 (§11과 같은 규칙)
AI-ready 문서       = 고정 섹션 넷 + `## Transcript` 블록들
```

**세 산출물 모두 결정론적 순수 함수의 결과다** (§18). 시계도 난수도 로캘도 저장소도 파일도
네트워크도 clipboard도 쓰지 않는다. 자리는 `src-tauri/src/export/ai_request.rs` 하나다 —
`markdown.rs` · `filename.rs`가 그러하듯 값에서 문자열을 만들 뿐이다.

### 5.2 AI-ready 문서의 섹션 구성과 순서 — 확정

```md
# Molt Note AI Request

## Mode
Study

## Instructions
<§6이 만드는 Manual 지시문. {{transcript}} 자리에는 본문 대신 아래를 가리키는 한 문장이 들어간다>

## Recording
Title: 3DGS Study #04
Date: 2026-09-01
Duration: 52:31

## Transcript

### 00:00:03
안녕하세요. 오늘은 3DGS를 봅니다.

### 00:00:06
먼저 splat 표현부터 보겠습니다.
```

확정 사항:

| | 결정 | 근거 |
| --- | --- | --- |
| 섹션 순서 | `Mode` → `Instructions` → `Recording` → `Transcript` **고정** | 읽는 쪽(모델과 사람)이 **무엇을 해 달라는 요청인지**를 먼저 만나고, 가장 긴 것이 마지막에 온다. 긴 입력의 앞뒤에 지시를 흩지 않는다 |
| 첫 줄 | `# Molt Note AI Request` **고정 문자열** | 이 문서는 "녹음 하나의 기록"이 아니라 **요청**이다. §11의 export(`# <제목>`)와 첫 줄에서 구분되므로, 파일 두 개가 섞여도 무엇이 무엇인지 즉시 보인다. "Molt Note"는 제품 이름이지 벤더가 아니다 (§10) |
| `## Mode` 값 | `Meeting` · `Study` · `Summary` — §9.5의 **표시 이름** | wire 값(`NoteType::as_str` = 소문자)은 경계를 지나는 값이고, 문서에 적히는 것은 사람이 읽는 이름이다. 둘을 섞지 않는다 |
| `## Recording` | `Title:` · `Date:` · `Duration:` 세 줄 | `Date:` · `Duration:` 두 줄은 **§11이 이미 만드는 블록 그대로**다(§5.4). `Title:`만 새 라벨이며, 값은 `heading_text`가 만드는 한 줄 제목이다 |
| `## Transcript` | §11의 블록을 **그대로** | 아래 5.3 |
| 블록 사이 | 빈 줄 **정확히 하나**, 문서는 개행 **하나**로 끝난다 | `markdown::render`와 같은 조립 규칙이다. "언제 빈 줄을 넣는가"를 각 자리가 따로 판단하지 않는다 |
| audio | `audio_path` · `audio_format` · 오디오 바이트가 **하나도 들어가지 않는다** | INV-6 · MH-4. 렌더러가 읽는 Recording 필드는 `title` · `created_at` · `duration_ms` **셋뿐**이다 |

### 5.3 timestamp 표현 — 새 형식을 만들지 않는다

```text
### 00:00:03      ← format_timestamp_ms(3_000)
```

**§11과 같은 함수, 같은 문자열이다.** [E1] `export::markdown::format_timestamp_ms`는 이미
`pub`이고 `export` 모듈에서 re-export되어 있다 — 그러므로 이 Phase는 **아무것도 새로 만들지
않고 부르기만 한다.**

그 함수가 이미 정해 둔 성질을 그대로 물려받는다: 시·분·초가 언제나 두 자리, 100시간이 넘으면
시간 자리가 세 자리, 음수 오프셋은 `0`, 자르지 않는다. **화면의 길이 표시(`52:31`,
`format_duration_ms`)와는 다른 형식이며 그 구분도 그대로다** — 녹음이 얼마나 긴가와 이 문장이
녹음의 어디인가는 다른 값이다.

> **timestamp 변환 규칙을 TypeScript로 옮기지 않는다.** `tests/screen-boundary.test.ts`가
> `src/` 전체에서 그것을 막고 있고, 예외로 허용된 파일은 `src/screens/transcriptView.ts`
> 하나다. Manual Handoff의 세 산출물은 **전부 Rust에서 완성된 문자열로 화면에 도착한다** —
> 화면이 조립할 것이 없으므로 이 검사는 손대지 않는다 (§8.3).

### 5.4 재사용 방식 — 병렬 export 시스템을 만들지 않는다

**입력 타입을 새로 만들지 않는다.**

```rust
// export/ai_request.rs
pub struct AiRequest<'a> {
    pub mode: NoteType,
    /// §11의 문서 입력 그대로다. 이 경로에서 `note`는 언제나 `None`이다.
    document: ExportDocument<'a>,
}

impl<'a> AiRequest<'a> {
    /// **노트를 받지 않는다.** 이 요청은 노트를 만들어 달라는 요청이므로,
    /// 이미 있는 노트를 실을 자리가 생성자에 없다.
    pub fn new(mode: NoteType, recording: &'a Recording, transcript: &'a Transcript) -> Self
}
```

`ExportDocument`가 이미 "한 문서로 만들 입력"이고 그 이상도 이하도 아니므로 두 번째 입력
타입을 만들 이유가 없다. `note: None`은 §11에서도 **정상 상태**이며(INV-8), 생성자가 노트를
받지 않으므로 이 경로에서 `Some`이 될 자리 자체가 없다.

**렌더링 규칙을 복제하지 않는다.** `markdown.rs`의 private 함수 중 **두 개를 `pub(crate)`로
끌어올려** 한 자리에서 쓴다.

```text
transcript 본문을 만드는 규칙   markdown.rs private → pub(crate)
  · segment는 받은 순서 그대로 (정렬하지 않는다)
  · 빈 텍스트 segment는 버린다
  · segment가 하나도 남지 않으면 raw_text 한 문단
  · 그마저 비면 아무것도 만들지 않는다

메타데이터 블록을 만드는 규칙   markdown.rs private → pub(crate)
  · "Date: <iso_date 또는 받은 값 또는 unknown-date>\nDuration: <format_duration_ms>"
  · 제목 한 줄 만들기(heading_text) — 빈 제목은 "제목 없음"
```

이 두 규칙이 **두 벌이 되면 조용히 갈라진다.** 그래서 AI-ready 문서는 §11과 같은 함수를 부르고,
Transcript 텍스트도 같은 함수를 부른다.

프롬프트에 끼우는 transcript는 `## Transcript` 제목 없이 본문만 필요하고, 문서와 Copy
Transcript는 제목까지 필요하다. 그 차이를 **문자열 자르기로 만들지 않는다** — 본문을 값으로
돌려주는 함수 하나를 두고, 제목을 붙이는 쪽이 붙인다.

```rust
pub(crate) enum TranscriptBody<'a> {
    /// 각 블록이 `### HH:MM:SS\n텍스트`
    Segments(Vec<String>),
    /// segment가 없을 때의 한 문단
    Raw(&'a str),
    /// 적을 것이 없다
    Empty,
}
pub(crate) fn transcript_body(transcript: &Transcript) -> TranscriptBody<'_>;
```

> **`markdown::render`가 내는 바이트는 한 글자도 바뀌지 않는다.** 이 리팩터가 §11의 산출물을
> 건드렸는지는 이미 있는 golden 테스트
> (`a_recording_with_an_ai_note_renders_exactly_the_document_of_section_11` 등)가 판정한다.
> 그 테스트를 고쳐야 한다면 리팩터가 잘못된 것이다.

**세 산출물의 조립 규칙**(구현은 P2 · TASK-056):

| 산출물 | 조립 |
| --- | --- |
| Manual 프롬프트 | §6의 치환 결과. `{{transcript}}` ← `Segments`면 블록들을 빈 줄로 이은 문자열, `Raw`면 그 문단 |
| Transcript 텍스트 | `## Transcript` + `transcript_body` (§11의 문서에 들어가는 그 블록 그대로) + 끝 개행 하나 |
| AI-ready 문서 | 5.2의 고정 섹션 넷 + `## Transcript` 블록들, 빈 줄 하나로 잇고 끝 개행 하나 |

Copy Transcript가 `## Transcript` 제목을 **포함하는** 이유: 제목을 떼면 "제목 없는 본문"이라는
**두 번째 조립 규칙과 두 번째 기대 문자열**이 생긴다. 붙여 넣은 사람에게도 그 한 줄은 무엇을
붙였는지 말해 준다. (제목을 떼는 안은 이 이유로 탈락시켰다.)

### 5.5 `TranscriptBody::Empty`는 산출물을 만들지 않는다

전사 본문이 하나도 없는 Recording에 대해 "AI에게 줄 요청"을 만드는 것은 **빈 요청을 만드는
일**이다. 순수 렌더러는 받은 값을 그대로 렌더하지만(결정론), **command 경계는 거절한다** —
`current_transcript_id`가 없거나 본문이 비어 있으면 §13의 `Failure`로 끝난다. 이것은
`export::run::export`가 이미 하는 판정과 같은 자리, 같은 모양이다.

### 5.6 Export for AI의 파일 이름과 자리

```text
디렉터리   exports/                         ← ADR-0009 §4가 정한 그 자리. 두 번째 디렉터리를 만들지 않는다
이름       2026-09-01-3dgs-study-04-ai-request.md
           └─ export_file_name(created_at, title)의 결과에서 확장자 앞에 `-ai-request`를 넣는다
충돌       write_new가 `-2` · `-3` … 을 붙인다   ← ADR-0009 §4.3 그대로. 덮어쓰지 않는다
```

- 슬러그 규칙 · 날짜 규칙 · `unknown-date` · Windows 예약 이름 처리는 **전부 기존
  `filename.rs` 한 자리에 그대로 있다.** 이 Phase가 더하는 것은 **고정 표식 하나**뿐이다.
- 표식이 필요한 이유: 같은 Recording의 Markdown export와 이름이 같으면, 서로 다른 문서 두 개가
  `-2` 접미사로만 구분된다. 그 상태에서는 파일 목록만 보고 무엇이 무엇인지 알 수 없다.
- 80바이트 슬러그 상한(`MAX_SLUG_BYTES`)은 그대로다. 표식 11바이트는 ADR-0009 §4.2가 이미
  남겨 둔 여유 안에 들어간다 [A].

---

## 6. 결정 (4) — `ai::prompt` 상수를 고치지 않고 재사용한다

### 6.1 왜 고칠 수 없는가 — 이것이 이 절의 근거 전부다

[E1] `ai/prompt.rs`는 프롬프트를 고치면 **테스트가 먼저 깨지도록** 설계돼 있다.

```text
프롬프트를 한 글자 고친다
  → computed_prompt_version(mode)의 hash8이 달라진다
  → 선언된 PROMPT_VERSION_{MEETING,STUDY,SUMMARY}와 다르다
  → prompt_version_is_bound_to_the_prompt_text 테스트가 깨진다
  → 선언값을 고치기 전에는 test Gate를 통과할 수 없다
```

여기서 **선언값을 고치는 것으로 넘어가면 더 나쁜 일이 일어난다.** 이미 저장된 AI Note의
`ai_notes.prompt_version`은 **그 노트를 만든 프롬프트**를 가리키는 provenance다 (§7.3 · §9.6).
상수를 고치고 선언값을 따라 올리면, 과거에 저장된 값이 가리키는 프롬프트는 이 저장소 어디에도
없게 된다 — **저장된 provenance가 거짓이 된다.**

> 그러므로 **Manual Handoff는 기존 프롬프트 상수를 고치지 않는다.** 고칠 이유도 없다 —
> Manual 경로가 필요로 하는 것은 다른 프롬프트가 아니라 **다른 출력 형태**뿐이다.

### 6.2 seam은 이미 있다 — 그리고 새로 필요한 것은 하나뿐이다

[E1] 세 상수는 **완전히 같은 문장 블록**으로 출력 형태를 요구한다.

```text
Return exactly one JSON object and nothing else. No prose before or after it, no code fence,
no explanation.
```

Manual 경로에서 이 문장은 그대로 둘 수 없다. 사람이 채팅에 붙여 넣고 **읽을** 답을 받는데,
JSON 한 덩어리는 읽히지 않는다. Phase Goal도 "출력이 구조화된 Markdown으로 오도록 요청하는
문구를 포함한다"를 요구한다.

**결정: 상수를 고치는 대신, 상수 원문에 치환 두 번을 적용해 Manual 프롬프트를 만든다.**

```rust
manual_prompt(mode, slot) =
    prompt_template(mode)                                    // 상수 원문 — 읽기만 한다
        .replace(JSON_OUTPUT_CONTRACT, markdown_contract(mode))  // (a) 정확히 1회여야 한다
        .replace(TRANSCRIPT_PLACEHOLDER, slot)                   // (b) 이미 있는 seam
```

| | 치환 | 성질 |
| --- | --- | --- |
| (a) | JSON 출력 계약 블록 → Markdown 출력 계약 블록 | 세 상수에 **글자까지 같은** 블록이 한 번씩 있다 [E1]. 새 모듈이 그 블록을 `const`로 들고, **치환이 정확히 1회 일어났는지 단언한다** |
| (b) | `{{transcript}}` → 상황에 맞는 내용 | **이미 있는 유일한 조립 자리**다. `every_prompt_has_exactly_one_place_for_the_transcript`가 "자리는 하나"를 이미 지키고 있다 |

**(a)가 실패하면 조용히 넘어가지 않는다.** 누군가 상수의 그 문장을 손대면 치환이 0회가 되고,
그때 만들어지는 것은 "Markdown으로 달라"와 "JSON만 달라"가 **동시에 들어간 모순된 프롬프트**다.
그래서 새 모듈은 치환 횟수를 단언한다 — `prompt_version_is_bound_to_the_prompt_text`가 상수 변경을
잡는 것과 **같은 태도**이며, 실패 메시지가 무엇을 해야 하는지 말한다.

`{{transcript}}` 자리에 들어가는 값은 산출물마다 다르다.

| 산출물 | `{{transcript}}` ← |
| --- | --- |
| Copy AI Prompt | transcript 본문 그대로 — **한 번의 붙여 넣기로 완결된다** |
| Export for AI의 `## Instructions` | `The transcript is in the "## Transcript" section of this document.` 같은 **가리키는 한 문장** — 같은 문서 안에서 본문이 두 번 나오지 않는다 |

Copy AI Prompt가 transcript를 **포함하는** 이유: 포함하지 않으면 사용자가 두 번 붙여 넣어야
하고, 그 사이에 채팅이 지시만 보고 답하기 시작한다. Copy Transcript는 **자기 지시를 직접 쓰고
싶은 사람**을 위한 별개의 길로 남는다.

### 6.3 Markdown 출력 계약이 요구하는 것 — 요구 사항을 여기서 고정한다

문장의 최종 표현은 P2가 정하되(모듈 안의 `const` 하나), **다음 다섯을 반드시 말해야 한다.**

```text
1. JSON이 아니라 Markdown으로 답한다. code fence도 앞뒤 설명도 없다.
2. 아래에 설명된 키 하나가 섹션 하나다. `## ` 제목을 그 순서대로 쓴다.
3. 제목은 이 목록의 문자열 그대로다 (§9.5의 출력 섹션 이름).
4. 목록인 섹션은 항목마다 `- ` 한 줄로 쓴다.
5. 내용이 없는 섹션은 제목도 쓰지 않는다.
```

**3번의 제목 목록은 손으로 적지 않는다** — `export::markdown::{MEETING,STUDY,SUMMARY}_SECTIONS`를
그대로 쓴다. 그 상수가 §9.5의 이름이고, `markdown::render`가 실제로 만드는 제목이며 [E1],
Notion으로 가는 문서의 제목이기도 하다.

그 결과가 이 결정의 핵심 성질이다.

```text
Manual 프롬프트가 요구하는 출력 모양  ==  markdown::render가 이미 만드는 모양

→ 사용자가 채팅에서 받아 온 답은 이 앱이 만드는 문서와 같은 모양이다
→ "AI에게 요구하는 형식"이라는 두 번째 규칙이 생기지 않는다
```

프롬프트가 키를 나열하는 순서와 `*_SECTIONS`의 순서는 **둘 다 §9.5 하나에서 왔고 i번째끼리
1:1로 대응한다** [E1]. P2는 그 대응(개수와 순서)을 테스트로 고정한다 — 어느 한쪽이 흔들리면
계약이 어긋난 채로 나가기 전에 깨진다.

### 6.4 이 경로는 `promptVersion`도 provenance도 만들지 않는다

Manual 프롬프트는 상수 원문과 **다른 문자열**이다. 그러므로 `PROMPT_VERSION_*`을 그 값에 붙이면
거짓이 된다. 붙이지 않는다 — 그리고 붙일 자리도 없다.

```text
Manual 경로가 저장소에 쓰는 것        없다 (MH-7)
Manual 경로가 만드는 ai_notes 행       없다
Manual 경로가 남기는 promptVersion     없다
```

저장하지 않으므로 provenance가 필요하지 않고, provenance가 없으므로 버전이 필요하지 않다.
**두 번째 프롬프트 세트도, 두 번째 버전 체계도 만들지 않는다.** 사용자가 채팅에서 받은 답을
앱에 다시 넣는 경로는 이 Phase에 없다 (§11).

### 6.5 이 경로에 파서가 없다

`ai::note::parse_note`도, `json_schema`도, `ContextBudget`/`prepare`의 초과 판정도 이 경로에
들어오지 않는다.

- **파서 없음** — 답을 읽는 것은 사람이다. 모델이 Markdown 계약을 어기고 JSON을 돌려줘도 앱에서
  깨지는 것은 없다. 그것은 **실패 상태가 아니라 품질 문제**이며, P11의 Human Review 항목이다.
- **context 예산 판정 없음** — 사용자의 채팅이 어떤 모델을 쓰는지 앱은 모른다. 모르는 값으로
  거절하지 않는다. `aiInputTooLarge`는 이 경로에 존재하지 않는다. **자르지 않는다**는 것만
  같다 — 전사는 통째로 나가고, 너무 길면 그 사실을 사용자의 채팅이 말한다.

---

## 7. 결정 (5) — clipboard 경계의 자리

### 7.1 지금 있는 것

[E1] `src/`와 `src-tauri/src/` 어디에도 clipboard 코드가 없다. `Cargo.lock` ·
`package-lock.json` · `tauri.conf.json` · `capabilities/default.json`에도 clipboard 관련 항목이
없다(grep 0건). **새 플랫폼 capability다.**

### 7.2 결정

> **`src/platform/clipboard.ts` 하나.** 저장소에서 clipboard를 실제로 부르는 자리는 이 파일
> 하나이며, 화면은 이 모듈을 통해서만 복사한다. 새 npm/cargo 의존성도, 새 Tauri 권한도, 새
> command 이름도, 새 `FailureKind`도 늘지 않는다.

```text
src/platform/clipboard.ts     webview의 clipboard 쓰기를 아는 유일한 자리
  · 입력은 문자열 하나, 결과는 성공 또는 Failure 값 하나
  · 실패를 console로 흘리지 않는다  (tests/screen-boundary.test.ts가 src/ 전체에서 막는다)
  · 테스트에서 test double로 대체된다 — 자동 테스트가 실제 시스템 clipboard를 건드리지 않는다
```

`src/platform/`은 새 디렉터리다. 이름을 그렇게 고른 이유는 `src-tauri/src/platform/`이
**이미 같은 뜻으로 쓰이고 있기 때문이다** — "플랫폼 지식이 갇혀 있는 경계"(INV-10 · §3.1).
경계의 성질이 같으면 이름도 같아야 다음 사람이 찾을 수 있다.

**복사 동작의 상태(아직 안 함 · 복사 중 · 복사됨 · 실패)와 그때 무엇을 보여줄지의 판정은
DOM을 모르는 순수 모듈에 둔다** — 기존 `src/screens/<name>View.ts` 규약 그대로. 그래야 clipboard
없이 vitest로 전부 판정된다. 성공/실패 표시는 **색만으로 말하지 않고 텍스트를 동반한다.**

**새 `FailureKind`를 만들지 않는다.** [E1] `FailureKind`는 Rust의 종류와 1:1이고, 예외는
프론트 경계에서만 만들어지는 `unexpected` 하나다. `tests/ipc-boundary.test.ts`가 양방향으로
그것을 강제한다(`declared.length === kinds.size + 1`). clipboard 실패는 Rust에서 나지 않으므로
`clipboard`라는 종류를 union에 더하면 **그 검사가 깨진다** — 그리고 깨뜨릴 이유가 없다.
clipboard 경계는 `kind: 'unexpected'`에 **clipboard가 무엇을 못 했는지 말하는 message**와
`sourceDataSafe: true` · `retryable: true`를 실어 돌려준다. §13의 세 질문에 그대로 답한다.

### 7.3 후보 비교와 탈락 근거

| | 후보 | 새 의존성 | 새 command | 판정 |
| --- | --- | --- | --- | --- |
| **A** | **프론트엔드 platform 모듈 하나** (`navigator.clipboard`) | 없음 | 없음 | **채택** |
| B | Rust command 하나 (`copy_to_clipboard`) | 필요 (아래) | +1 | 탈락 |
| C | Tauri clipboard 플러그인 | npm + cargo + 권한 항목 | 없음 | 탈락 (지금은) |

**A를 고른 이유**

1. 복사할 문자열은 **이미 webview 안에 있다.** `get_ai_prompt` · `get_transcript_text`가 방금
   돌려준 값이다. B는 그 문자열을 Rust로 **되돌려 보낸다** — 1시간짜리 전사면 수십만 글자가
   경계를 한 번 더 왕복한다. 얻는 것 없이 왕복만 는다.
2. 의존성이 늘지 않는다. Phase Goal이 "큰 서드파티를 들이지 않는다"고 했고, ADR-0009 §11이
   **확인하지 못한 crate feature 위에 서는 비용**을 이미 겪었다. 버튼 하나 때문에 그 비용을
   다시 지불하지 않는다.
3. IPC 표면이 늘지 않는다. 이 Phase가 여는 이름은 §8의 셋으로 끝난다.
4. 실패해도 **대체 경로가 이미 있다** (§7.5). A의 유일한 약점(UNVERIFIED한 플랫폼 능력)이
   막다른 길이 아니다.

**B를 탈락시킨 이유** — 위 1번의 왕복 비용에 더해, Rust에서 OS clipboard에 쓰려면 결국
**플러그인이나 OS별 crate가 필요하다**(Tauri v2 core가 clipboard를 노출하는지는 이 Run이 확인하지
못했다 · §7.4). 즉 B는 C의 비용을 그대로 지불하면서 왕복까지 더한다. 다만 B의 자리 자체는
아키텍처적으로 정당하다 — A가 플랫폼에서 막히면 **B/C가 그때의 대안**이다 (§7.5).

**C를 탈락시킨 이유** — 지금 시점에 **확인하지 못한 사실 위에 의존성을 얹는 선택**이기 때문이다.
이 Run은 플러그인의 정확한 crate/npm 이름, 이 저장소의 Tauri 2 핀과 호환되는 버전, 필요한
permission 식별자를 **하나도 확인하지 못했다** [E4]. 확인되지 않은 세 값 위에 의존성과 권한
설정을 더하는 것은 ADR-0009 §11이 남긴 교훈과 반대 방향이다. **필요가 증명되면 그때 얹는다** —
A가 실패하는 것이 그 증명이다.

### 7.4 UNVERIFIED — 확인하지 못한 것 (지어내지 않는다)

| | 확인하지 못한 것 |
| --- | --- |
| [E4] | Tauri v2 webview(macOS WKWebView · Windows WebView2)에서 `navigator.clipboard.writeText`가 **이 앱의 origin에서 실제로 동작하는지** |
| [E4] | 그 API가 요구하는 조건(secure context · 사용자 제스처)이 이 앱의 창에서 어떻게 판정되는지 |
| [E4] | 실패할 때 어떤 예외/거절 값이 오는지 (`NotAllowedError` 계열인지, 조용히 실패하는지) |
| [E4] | Tauri v2 core가 clipboard를 노출하는지, 아니면 플러그인이 필요한지 |
| [E4] | clipboard 플러그인의 정확한 crate/npm 이름 · 이 저장소의 Tauri 핀과 호환되는 버전 · permission 식별자 |
| [E4] | Windows에서의 동작 전반 — 이 저장소는 아직 Windows에서 검증되지 않았다 (Phase 6) |

**이 여섯을 확인된 사실처럼 쓰지 않는다.** 그래서 §7.2의 경계는 "동작한다"를 전제로 설계되지
않았다 — **"동작하지 않을 수 있다"를 전제로** 설계됐다. 확인은 실행 환경이 있어야 하며, P11이
그 시점의 사실을 §12에 적는다. **확인되지 않은 것을 DONE으로 적지 않는다.**

### 7.5 실패는 어떻게 보이고, 무엇이 남는가

```text
[ Copy AI Prompt ]  누른다
        │
        ├─ 성공 ──▶ "복사됨"  (텍스트 + 표시. 색만으로 말하지 않는다)
        │
        └─ 실패 ──▶ 무엇이 실패했는가 : "클립보드에 쓰지 못했다"
                    원본은 안전한가   : 안전하다 — 이 동작은 아무것도 읽지도 쓰지도 않았다
                    다시 시도할 수 있는가 : 있다 — 같은 버튼이 그대로 남는다
                            │
                            └──▶ 그리고 [ Export for AI ]가 옆에 그대로 있다
```

**clipboard가 거절되는 환경에서도 Manual AI Handoff는 성립한다.** Export for AI는 clipboard를
전혀 쓰지 않는다 — 파일 하나를 `exports/`에 쓰고 **실제로 쓰인 경로**를 돌려준다. 사용자는 그
파일을 자신의 AI 채팅에 첨부하거나 열어서 복사한다.

그러므로 §7.4의 UNVERIFIED 여섯이 전부 최악으로 판명돼도 **이 Phase의 성공 기준 1은 무너지지
않는다.** 무너지는 것은 세 동작 중 둘의 편의이며, 그때의 복구는 §7.3의 B 또는 C로 **모듈 하나를
교체하는 일**이다 — 부르는 자리가 하나이기 때문에 그렇다.

---

## 8. 결정 (6) — 이 Phase가 여는 command 이름과 개수

### 8.1 세 개다

| # | 이름 | 인자 | 돌려주는 것 | 하는 일 |
| --- | --- | --- | --- | --- |
| 1 | `get_ai_prompt` | `recordingId` · `mode` | 문자열 | Manual 프롬프트 (transcript 포함) |
| 2 | `get_transcript_text` | `recordingId` | 문자열 | 붙여 넣을 수 있는 Transcript 텍스트 |
| 3 | `export_ai_request` | `recordingId` · `mode` | `ExportedFilePayload` | AI-ready 문서를 `exports/`에 파일 하나로 쓴다 |

**등록 command는 28 → 31이 된다** [E1: 현재 `REGISTERED_COMMANDS`는 28개].

셋으로 나눈 이유 — **사용자 동작 하나에 이름 하나**다. 셋을 한 command로 묶으면 프롬프트와
문서와 전사가 **같은 전사를 세 벌** 실어 나른다. 1시간짜리 녹음에서 그것은 필요 없는 값의
세 배다. 반대로 넷 이상으로 쪼갤 것도 없다.

**늘지 않는 것:**

```text
저장된 것을 고치거나 지우는 이름       늘지 않는다 (INV-3 · MH-7)
provider · 설정을 보는 경로            없다 — 거절할 수단 자체가 없다 (MH-1 · MH-2)
transcriptId를 인자로 받는 이름        없다 ← 아래 8.2
벤더 이름이 들어간 이름                없다 (§10)
```

### 8.2 `transcriptId`가 아니라 `recordingId`를 받는다 (MH-5)

세 command 모두 **`Recording.current_transcript_id`가 가리키는 Transcript만** 읽는다 (§7.2).
그것을 규칙이 아니라 **계약의 모양**으로 만든다 — command가 `transcriptId`를 받지 않으므로,
화면이든 누구든 **실패했거나 대체된 옛 version을 고를 방법이 wire에 없다.**

(기존 `get_transcript(transcriptId)`는 이미 저장된 것을 id로 읽는 순수한 읽기이며 성격이 다르다.
Manual Handoff는 "지금 이 녹음의 전사"를 요구하는 동작이므로 id를 받지 않는다.)

이 순서는 `export::run::export`가 이미 하는 것과 **같다** [E1] — Recording을 읽고,
`current_transcript_id`가 없으면 거절하고, 있으면 그 Transcript를 읽는다.

### 8.3 `tests/ipc-boundary.test.ts`가 깨질 자리 — **예고**

이름이 세 개 늘면 그 파일의 **네 자리가 반드시 깨진다.** 그것은 버그가 아니라 **범위 초과를
잡으려고 일부러 걸어 둔 tripwire**다. 미리 적어 두는 이유는 하나다 — 깨진 것을 보고 **검사를
무르게 만드는 대신**, 이 Phase가 실제로 연 이름만 의식적으로 더하게 하기 위해서다.

| # | 깨지는 자리 | 무엇이 걸리는가 | 어떻게 다룬다 |
| --- | --- | --- | --- |
| 1 | `REGISTERED_COMMANDS` 상수 (정확히 같은 집합을 요구한다) | 새 이름 **셋 전부** | 셋을 더하고, **왜 늘었는지 주석으로 남긴다** — 지금까지의 Phase가 전부 그렇게 해 왔다 |
| 2 | `outOfScope` 정규식의 `export(?!_markdown\b)` | `export_ai_request` | 정규식을 **느슨하게 고치지 않는다.** 허용되는 export 이름이 둘이 됐다는 사실을 그대로 적는다 (예: `export(?!_(markdown\|ai_request)\b)`). PDF · DOCX · 일괄 export는 **여전히 막혀 있어야 한다** |
| 3 | `'Markdown export 표면은 파일 하나를 만드는 이름 하나뿐이다'` (`['export_markdown']` 기대) | `export_ai_request` | 기대값을 두 이름으로 만들고, 검사의 뜻을 갱신한다 — "파일 하나를 만드는 이름 **둘**". 일괄 export도 export한 것을 지우는 이름도 여전히 없다 |
| 4 | `'전사 표면은 한 건 시작 · 상태 조회 · 결과 읽기 셋뿐이다'` (`/transcri/i` 필터) | `get_transcript_text` | 기대 목록에 넣고, 그것이 **읽기 파생 하나이지 전사 큐가 아니라는 것**을 적는다. 큐를 막는다는 검사의 목적은 그대로다 |

**깨지지 않는 것도 확인해 둔다** [E1]:

```text
'AI 표면은 … 다섯뿐이다'   필터가 /ai_note|ai_provider/ 이므로 get_ai_prompt는 걸리지 않는다 → 그대로
'저장된 …를 고치거나 지우는 command가 없다' (transcript · ai_note · sync)   세 이름 전부 해당 없음 → 그대로
'wire 계약에 벤더가 없다'   새 payload/타입에 벤더 이름을 넣지 않는다 (§10) → 그대로
'자격증명은 한 방향으로만 지난다'   token이 이 경로에 없다 → 그대로
'src/ 아래에서 invoke를 부르는 곳은 ipc 모듈뿐이다'   clipboard 경계는 invoke를 쓰지 않는다 → 그대로
'frontend가 부르는 이름이 등록된 이름과 정확히 같다'   client 셋을 함께 더하면 그대로
```

> **검사를 삭제하거나 약화해서 통과시키지 않는다** (KERNEL §6). 위 네 자리는 "이 이름이 늘어도
> 되는가"를 사람이 한 번 판단하게 만드는 장치이고, 이 문서가 그 판단을 미리 기록한 것이다.

### 8.4 MH-1~MH-8 — 무엇으로 판정하는가

| 불변 | 판정 수단 | 어디서 |
| --- | --- | --- |
| **MH-1** AI Provider가 하나도 설정되지 않아도 동작한다 | (a) 새 Rust 모듈과 command에 `crate::ai::provider`·provider 설정 참조가 **없다** — 컴파일 단위에서 확인된다 (b) provider 없음 상태에서 세 동작이 전부 가능하다는 것을 순수 view 모듈 테스트가 값으로 판정한다 | P2 · P3 · P5 |
| **MH-2** Ollama를 요구하지 않는다 | MH-1과 같은 수단 + 새 코드 경로에 `ollama` 심볼이 도달하지 않는다. `ipc-boundary`의 벤더 부재 검사가 그대로 통과한다 | P2 · P3 |
| **MH-3** 네트워크로 자동 전송하지 않는다 | (a) 새 Rust 모듈이 HTTP 클라이언트(`ureq`)를 쓰지 않는다 (b) `ipc-boundary`의 `'src/ 아래에 네트워크로 나가는 통로가 없다'`가 그대로 통과한다 (c) 이 경로가 파일시스템에 닿는 자리는 `write_new` 하나다 | P2 · P3 |
| **MH-4** audio 바이트도 audio 경로도 산출물에 없다 | (a) 렌더러가 읽는 Recording 필드는 `title`·`created_at`·`duration_ms` 셋뿐이다 (b) 단위 테스트가 세 산출물 문자열에 `audio_path`·`audio_format` 값이 **없음**을 단언한다 (c) 새 타입에 audio를 담는 필드를 만들지 않는다 | P2 |
| **MH-5** 현재 성공한 Transcript를 쓴다 | (a) **wire에 `transcriptId` 인자가 없다** (§8.2) — 옛 version을 고를 수단 자체가 없다 (b) 옛 version이 함께 존재하는 저장소로 command를 돌려 current 쪽 문장이 나오는지 본다 | P3 |
| **MH-6** vendor 중립이다 | (a) 세 산출물 문자열에 벤더 이름이 없음을 단위 테스트가 단언한다 (b) `ipc-boundary`의 payload/타입 벤더 검사가 그대로 통과한다 (c) command 이름은 `outOfScope` 정규식이 이미 막는다 | P2 · P3 |
| **MH-7** 실패가 기존 데이터를 훼손하지 않는다 | (a) 새 Rust 모듈·command에 저장소 **쓰기** 호출이 없다(읽기 질의와 `write_new`뿐) (b) `write_new`는 기존 파일을 덮어쓰지 않는다 — ADR-0009 §4.3의 기존 테스트 (c) 실패는 `sourceDataSafe: true`로 돌아온다 | P3 · P4 |
| **MH-8** 기존 Connected Provider 경로가 그대로 살아 있다 | (a) `src-tauri/src/ai/`의 diff가 비어 있다 — 특히 `prompt.rs` (b) 기존 `aiNoteView` · `coreWithoutAi` · provider 설정/연결/모델 선택 테스트가 **고치지 않은 채로** 계속 통과한다 | P2 · P5 · P6 |

여덟 개 중 **`A-` deferred assumption으로 남는 것은 없다** (D-4). 전부 자동 검증으로 판정된다.
자동으로 판정할 수 없는 셋(§4.2)은 P11의 Human Review 항목이며 불변이 아니다.

---

## 9. 실패는 어디서 값이 되는가

```text
current transcript 없음 / 본문 비어 있음 → Failure (§13)          ← command 경계 · §5.5
저장소를 열지 못함                       → Failure(storage)        ← 기존 경로 그대로
exports 디렉터리를 만들지 못함            → Failure(storage)        ← ensure_exports_dir 그대로
이름이 이미 있음                          → 실패가 아니다 — `-2`가 붙고 쓰인 경로를 돌려준다
clipboard 거절                            → Failure(unexpected) + 재시도 + Export for AI (§7.5)
모델이 Markdown 계약을 어김                → 실패가 아니다 — 앱에 파서가 없다 (§6.5)
```

전부 §13의 세 질문에 답한다: **무엇이 실패했는가 · 원본은 안전한가 · 다시 시도할 수 있는가.**
어느 경우에도 Recording · Transcript · AINote · 이미 만들어진 export 파일은 그대로다.

---

## 10. 벤더 중립 — 결정

> **ChatGPT · Claude · Gemini · Ollama 같은 이름은 이 Phase의 산출물에도, core/domain에도,
> command 이름에도, payload 타입에도, frontend 타입에도 들어가지 않는다** (INV-9 · MH-6).

- 산출물 셋 어디에도 특정 채팅 서비스를 가정한 문장·형식·schema가 없다. 문서가 요구하는 것은
  **Markdown 하나**이며, 그것은 어느 채팅에나 붙여 넣을 수 있다.
- `## Mode` 값은 `Meeting` · `Study` · `Summary` — **출력 형태의 종류이지 벤더가 아니다**
  (`NoteType`의 doc comment가 이미 그렇게 말한다 [E1]).
- `# Molt Note AI Request`의 "Molt Note"는 **이 제품 자신**이다. 목적지가 아니다.
- Settings에 남는 Ollama 이름은 **provider가 스스로 말한 값**이며 adapter 안에서 온다 —
  그 규칙은 Phase 4가 이미 정했고 이 Phase가 바꾸지 않는다.

이것이 §4.2의 D-5(브라우저 열기를 하지 않는다)와 같은 뿌리다. 특정 채팅을 이름으로 아는 순간,
그 채팅이 바뀔 때 흔들리는 것이 adapter 하나가 아니라 **산출물 자체**가 된다.

---

## 11. 이 Phase에서 하지 않는 것

```text
사용자의 AI 채팅에 자동으로 보내는 것 — 자동 로그인 · 브라우저 자동화 · 메시지 주입 ·
  자동 업로드 · 스크래핑 · 비공식 세션/토큰 접근 (전부 D-5 · Phase Goal)
Open ChatGPT / Open Claude 같은 브라우저 열기 (D-5 — 이후의 작은 polish item)
채팅에서 받은 답을 앱으로 되돌려 넣는 경로 (import) — 파서도 저장 경로도 만들지 않는다
새 cloud AI provider (OpenAI · Claude · Gemini · Codex) — §16 DEFERRED
Ollama adapter · provider 추상화의 삭제나 축소 — 재배치이지 삭제가 아니다 (§4.3)
두 번째 프롬프트 세트 · 프롬프트 커스터마이즈 UI — §15 DEFERRED
두 번째 export 시스템 · 일괄 export · PDF/DOCX — §16 DEFERRED
Manual 산출물에 대한 promptVersion / provenance / 저장 (§6.4)
clipboard **읽기** — 이 Phase가 필요로 하는 것은 쓰기 하나다
```

---

## 12. 구현 대조 (P11 · TASK-065 · 2026-09-06)

**§4~§9는 고쳐 쓰지 않았다.** 이 절만 새로 적는다 — 무엇이 결정대로였고, 무엇이 달라졌고,
무엇이 **여전히 확인되지 않았는가**.

표기는 §3과 같다. **[E1]** 이 Run이 이 저장소의 실제 파일을 읽어 확인한 것 ·
**[E3] Gate 실행 결과** · **[E4] UNVERIFIED — 확인하지 못했다.**

### 12.1 만들어진 자리 [E1]

```text
src-tauri/src/export/ai_request.rs   순수 렌더러 — manual_prompt · transcript_text ·
                                     ai_ready_document. 파일도 clipboard도 네트워크도 없다
src-tauri/src/export/handoff.rs      저장소·파일시스템과 잇는 실행 순서 (export::run과 같은 자리)
src-tauri/src/export/filename.rs     AI_REQUEST_MARKER · ai_request_file_name (기존 파일에 추가)
src-tauri/src/export/markdown.rs     private → pub(crate) 끌어올리기 (아래 12.2-a)
src-tauri/src/commands/export.rs     command 셋 (기존 파일에 추가)
src/platform/clipboard.ts            webview clipboard 쓰기를 아는 유일한 자리 (새 디렉터리)
src/screens/copyView.ts              복사 한 번의 상태와 표시를 정하는 순수 모듈
src/screens/aiHandoffView.ts         AI Note 탭의 두 줄 · Export for AI 자리
src/screens/EmptyState.tsx           빈 상태 한 모양 (UI 기반)
src/screens/Loading.tsx              오래 걸리는 동작 한 줄 (UI 기반)
src-tauri/tests/manual_ai_handoff.rs           command 경계 통합 테스트
src-tauri/tests/manual_handoff_invariants.rs   MH-1~MH-8의 Rust 쪽 원문·값 검사
tests/manual-handoff-invariants.test.ts        MH-1~MH-8의 화면 쪽 원문 검사
tests/ui-foundation.test.ts                    UI 기반 다섯 검사
```

### 12.2 결정과 달라진 것 — 셋 [E1]

**(a) `markdown.rs`에서 끌어올린 것이 둘이 아니라 넷이다.**
§5.4는 "private 함수 중 **두 개**를 `pub(crate)`로"라고 적었다. 실제로 `pub(crate)`가 된 것은
`transcript_body` · `transcript_blocks` · `metadata_block` · `heading_text` **넷과**
`TranscriptBody` enum이다. §5.4가 말한 **규칙은 그대로 둘**(transcript 본문 · 메타데이터
블록)이지만, 그 규칙을 복제하지 않고 쓰려면 부르는 함수가 넷 필요했다 — 특히
`transcript_blocks`(제목 없이 블록 목록만)와 `heading_text`(제목 한 줄)는 §5.4 본문에 이미
이름이 있었으나 개수 세기에서 빠져 있었다. **복제가 생기지 않았다는 §5.4의 요구는 그대로
지켜졌고**, `markdown::render`의 바이트도 바뀌지 않았다 — Phase 5의 golden 테스트가 고쳐지지
않은 채로 통과한다 [E3].

**(b) `transcript_text`는 적을 것이 없으면 빈 문자열이다.**
§5.4의 조립표는 `## Transcript` + 본문 + 끝 개행이라고만 적었다. 본문이 하나도 없을 때
"제목만 남은 텍스트"를 만들지 않기로 했다 — 그것은 붙여 넣는 사람에게 아무것도 말하지 않는
한 줄이다. 빈 요청을 **거절하는** 것은 §5.5대로 command 경계(`handoff::request_input`)가 하며,
순수 렌더러는 빈 값을 그대로 낸다.

**(c) clipboard 실패가 두 갈래로 갈린다.**
§7.2는 `kind: 'unexpected'` + message + `sourceDataSafe` · `retryable`까지만 정했다. 구현은
그 위에 `detail`의 표식 하나(`clipboard=`)를 더해 **`unavailable`(이 창에 쓰는 능력이 아예
없다)** 와 **`rejected`(능력은 있는데 이번 쓰기가 거절됐다)** 를 갈랐다. 새 `FailureKind`는
만들지 않았으므로 §7.2의 결정과 `ipc-boundary`의 양방향 검사는 그대로다 [E3]. 가른 이유는
**사용자가 할 수 있는 일이 다르기 때문**이고, 부수 효과로 §7.4가 남긴 UNVERIFIED 하나(거절
값의 모양)를 실제 환경에서 기록할 자리가 생겼다 — `rejectedFailure`가 예외의 이름을 버리지
않고 `detail`에 옮긴다.

**그 밖에는 결정대로다.** 특히 §6(프롬프트 상수를 고치지 않는다) · §7.3(후보 A) ·
§8.1(이름 셋) · §8.2(`recordingId`만 받는다) · §10(벤더 중립)은 그대로 구현됐다.

### 12.3 확정된 값 [E1]

| 항목 | §4~§9의 결정 | 실제 |
| --- | --- | --- |
| command 이름 | `get_ai_prompt` · `get_transcript_text` · `export_ai_request` | **같다** (`lib.rs`의 `generate_handler!`) |
| 등록 개수 | 28 → 31 | **31** (`REGISTERED_COMMANDS`가 정확히 같은 집합을 요구한다) |
| 인자 | `recordingId`(+`mode`), `transcriptId` 없음 | **같다** (`commands.ts` · `commands/export.rs`) |
| 돌려주는 것 | 문자열 · 문자열 · `ExportedFilePayload` | **같다** — export 페이로드 타입을 새로 만들지 않았다 |
| 파일 이름 표식 | 확장자 앞에 `-ai-request` | **같다** — `AI_REQUEST_MARKER = "-ai-request"`, `2026-09-01-3dgs-study-04-ai-request.md` |
| 문서 첫 줄 | `# Molt Note AI Request` | **같다** (`AI_REQUEST_HEADING`) |
| 섹션 순서 | `Mode` → `Instructions` → `Recording` → `Transcript` | **같다** |
| timestamp | §11과 같은 `### HH:MM:SS` · `format_timestamp_ms` | **같다** — 새 함수를 만들지 않았다 |
| 프롬프트 상수 | 한 글자도 고치지 않는다 | **고치지 않았다** — `PROMPT_VERSION_*` 선언값 셋이 그대로이고 `prompt_version_is_bound_to_the_prompt_text`가 고쳐지지 않은 채 통과한다 |
| 치환 (a) | 정확히 1회 | **단언한다** — `the_json_output_contract_is_replaced_exactly_once_in_every_prompt` |
| clipboard | 새 의존성 · 새 command · 새 `FailureKind` 없음 | **셋 다 없다** — `navigator.clipboard`를 집는 자리는 `systemClipboard()` 한 줄이다 |

**Markdown 출력 계약의 최종 문장** (§6.3이 요구한 다섯을 말한다 ·
`export/ai_request.rs::markdown_output_contract`):

```text
Return the note as Markdown and nothing else. No JSON object, no code fence, no prose before or
after it.

Write one `## ` section for each key described below, in that order, using exactly these
headings and nothing else:

## <§9.5의 섹션 제목들 — MEETING/STUDY/SUMMARY_SECTIONS에서 그대로 온다>

Write a key whose value is a list as one `- ` line per item. Leave out the heading of a section
that has nothing in it.
```

AI-ready 문서에서 `{{transcript}}` 자리에 들어가는 문장은
`The transcript is in the "## Transcript" section of this document.` 이다.

### 12.4 §7.4의 UNVERIFIED 여섯 — 무엇이 확인됐는가

**하나도 확인되지 않았다. 여섯 전부 [E4]로 남는다.**

| | 확인하지 못한 것 | 이 Phase 이후 상태 |
| --- | --- | --- |
| [E4] | webview에서 `navigator.clipboard.writeText`가 이 앱의 origin에서 실제로 동작하는지 | **여전히 UNVERIFIED.** 자동 테스트는 언제나 test double을 넘기므로 실제 clipboard에 닿지 않는다 — 그것은 의도된 설계이지 확인이 아니다 |
| [E4] | secure context · 사용자 제스처 조건이 이 앱의 창에서 어떻게 판정되는지 | **여전히 UNVERIFIED** |
| [E4] | 거절할 때 어떤 예외/거절 값이 오는지 | **여전히 UNVERIFIED.** 다만 12.2-(c)로 **그 값을 기록할 자리**는 생겼다 — 실제 환경에서 나온 이름이 `detail`에 남는다 |
| [E4] | Tauri v2 core가 clipboard를 노출하는지, 플러그인이 필요한지 | **여전히 UNVERIFIED.** 후보 A를 택했으므로 이 Phase가 그것을 알 필요가 없었다 |
| [E4] | 플러그인의 crate/npm 이름 · 호환 버전 · permission 식별자 | **여전히 UNVERIFIED.** 의존성을 더하지 않았으므로 확인할 이유도 없었다 |
| [E4] | Windows에서의 동작 전반 | **여전히 UNVERIFIED** — Phase 6 |

```text
이 Phase에서 실제 환경의 clipboard에 문자열이 쓰인 적:  없다  (NOT RUN)
사람이 산출물을 실제 외부 AI 채팅에 붙여 넣은 적:      없다  (NOT RUN)
```

**그러므로 "clipboard가 동작한다"고 적지 않는다.** §7.5가 설계한 대로 clipboard가 거절되는
환경에서도 Export for AI가 남으며, 그 대체 경로는 파일 쓰기까지 자동 검증이 실제로 지나간다
(임시 디렉터리에 실물 파일을 만든다 · `manual_ai_handoff.rs`) [E3].

확인 절차와 **빈 기록표**는 `docs/PHASE-5.5-HUMAN-REVIEW.md`에 있다. **그 표가 비어 있는 동안
이 경로를 "실제 AI 채팅에서 검증됐다"고 표현하지 않는다.**

### 12.5 `tests/ipc-boundary.test.ts`의 네 자리 [E1]

§8.3이 예고한 네 자리가 **예고된 그대로** 깨졌고, 검사를 무르게 만들지 않고 갱신됐다.

| # | 자리 | 어떻게 갱신됐는가 |
| --- | --- | --- |
| 1 | `REGISTERED_COMMANDS` | 세 이름을 더하고 **왜 늘었는지 주석으로 남겼다** — "마지막 셋이 Phase 5.5의 Manual AI Handoff다 … 28 → 31". 여전히 부분집합이 아니라 **정확히 같은 집합**을 요구한다 |
| 2 | `outOfScope` 정규식 | `export(?!_markdown\b)` → `export(?!_(markdown\|ai_request)\b)`. §8.3이 적어 둔 모양 그대로이며 **느슨해지지 않았다** — `pdf` · `docx` · `queue` · `batch` · `schedule` · 벤더 이름은 그대로 막힌다 |
| 3 | `'Markdown export 표면은 …'` | 이름과 기대값이 **`'export 표면은 파일 하나를 만드는 이름 둘뿐이다'` · `['export_ai_request', 'export_markdown']`** 로 갱신됐다. "각 이름은 파일 하나를 만들 뿐"이라는 뜻은 그대로다 |
| 4 | `'전사 표면은 …'` | `get_transcript_text`가 기대 목록에 들어가고, 이름이 **`'… 결과 읽기 둘뿐이다'`** 로 갱신됐다. 주석이 그것이 **읽기 파생 하나이지 전사 큐가 아니라는 것**을 적는다 — 큐를 막는 목적은 그대로다 |

§8.3이 "깨지지 않는다"고 적은 여섯도 **실제로 깨지지 않았다** — 특히 `FailureKind`의
양방향 검사(`declared.length === kinds.size + 1`)와 `src/`의 `invoke` 호출자 검사가
고쳐지지 않은 채 통과한다 [E3].

### 12.6 Gate [E3]

`node tools/loop-runtime/loopctl.mjs self-check build lint test` — **build · lint · test 전부
exit 0.** 자동 테스트 **1,238개** (vitest 494 · Rust 744). Phase 5 종료 시점은 1,081개였다.

**Gate가 판정하지 않는 것은 §12.4에 있다.**

---

## 12.7 나중에 넓어진 것 — 크기와 나눔 (Phase 5.6 · TASK-073 · 2026-09-07)

**위 §8.1과 §12.3은 Phase 5.5 시점의 기록이며 지우지 않는다.** 아래는 그 뒤에 무엇이 왜
달라졌는가다.

**왜.** 2026-09-05의 첫 실사용에서 72분 녹음의 산출물(99 KB · 5,139줄)이 AI 채팅에 한 번에
들어가지 않았고, **앱은 그 사실을 말해 주지 않았다** (`phase-prompt/05.6` R-5 · 성공 기준 4).
크기와 나눔을 만드는 순수 모듈은 `src-tauri/src/export/portion.rs`에 있다 (TASK-072).

**이름은 늘지 않았다.** §8.1의 "사용자 동작 하나에 이름 하나"가 그대로다 —
`tests/ipc-boundary.test.ts`가 세는 표면은 여전히 서른둘이다. 넓어진 것은 **인자 하나와 응답의
모양**이다.

| # | 이름 | 인자 | 돌려주는 것 |
| --- | --- | --- | --- |
| 1 | `get_ai_prompt` | `recordingId` · `mode` · **`portion`** | **`HandoffTextPayload`** |
| 2 | `get_transcript_text` | `recordingId` · **`portion`** | **`HandoffTextPayload`** |
| 3 | `export_ai_request` | `recordingId` · `mode` · **`portion`** | **`ExportedAiRequestPayload`** |

- `portion`은 **1부터 세는 번호 하나**이며, 보내지 않으면 첫 조각이다. 없는 조각을 달라는
  요청은 빈 값이 아니라 실패다 — **잘린 것을 온전한 것이라고 말하는 자리를 만들지 않는다.**
- 두 응답 타입 모두 산출물 **전체의 크기**와 **조각의 자리**(`portion` · `portionCount` ·
  `portionSize`)를 싣는다. `ExportedAiRequestPayload`는 `ExportedFilePayload`를 **감싼다** —
  파일 타입을 두 벌로 만들지 않았다.
- §8.2는 그대로다: 셋 다 `transcriptId`를 받지 않는다 (MH-5). §10도 그대로다 — 새 인자도 새
  타입도 벤더를 알지 않으며, 나눔의 예산은 **이 앱이 고른 값이지 어떤 채팅의 확인된 한도가
  아니다** (`export::portion::PORTION_MAX_BYTES`의 주석).
- 파일은 **조각마다 하나**이고 이름이 그 자리를 말한다 —
  `2026-09-01-3dgs-study-04-ai-request-part-2-of-4.md`. 어느 조각도 앞서 쓴 파일을 덮어쓰지
  않는다 (ADR-0009 §4.3). 나뉘지 않은 문서의 이름은 §12.3의 그것 그대로다.

---

## 12.8 크기 결정 — 예산은 어디서 왔고, 무엇을 어떻게 나누는가 (Phase 5.6 · TASK-072 · 2026-09-07)

**§12.7은 command 경계가 어떻게 넓어졌는가를 적었다. 이 절은 그 아래에 있는 값과 규칙을
적는다.** §4~§9의 결정 절도, §12.1~§12.6의 Phase 5.5 기록도 **한 글자도 고치지 않았다.**

### 12.8.1 실측 — 무엇을 보고 이 결정을 했는가

**[관측된 사실 · 2026-09-05 운영자의 첫 실사용]** 72분 녹음 하나의 §11 Markdown export
(`phase-prompt/05.6` R-5):

```text
파일 크기    99 KB
줄 수        5,139
구조         segment 하나마다 `### HH:MM:SS` 제목 + 본문
             → 제목 줄만 1,711개
```

Phase 5.5의 성공 기준 1은 *"일반 AI 채팅에 붙여 넣을 수 있는 형태"* 였다. **99 KB는 한 번에
들어가지 않았고, 앱은 그 사실을 말해 주지 않았다.** 짧은 녹음에서는 보이지 않고 긴 회의에서만
드러나는 종류의 침묵이다.

> **이 세 값이 이 절의 유일한 측정치다.** 아래 어디에도 "얼마나 빨라졌다" · "몇 %가 줄었다"
> 같은 새 수치를 적지 않는다 — 이 저장소는 압축 전후의 크기를 실측한 적이 없다.

### 12.8.2 앱이 고른 예산 — **벤더 제약이 아니다**

```rust
// src-tauri/src/export/portion.rs
pub const PORTION_MAX_BYTES: usize = 40_000;
```

| | |
| --- | --- |
| **무엇에서 나왔는가** | 위 실측(99 KB) 위에서 **이 앱이 고른 값**이다 — UTF-8 40,000 바이트, 한글로 대략 13,000자. 72분 규모의 문서가 서너 조각이 되는 크기다 |
| **무엇에서 나오지 않았는가** | **어떤 AI 채팅이 한 번에 받는 텍스트의 한도에서 나오지 않았다.** 그 한도는 서비스마다 다르고 이 저장소가 확인한 primary source가 없다 — **[E4] UNVERIFIED다.** 그런 숫자를 코드에 적으면 그것은 벤더 지식이 되고, 이 제품은 산출물에도 코드에도 벤더를 담지 않는다 (§10 · MH-6 · INV-9) |
| **Notion의 예산과 어떤 관계인가** | **없다.** `notion::chunk::CHUNK_MAX_BYTES`(60,000)는 **VERIFIED된 Notion 요청 한도에서 유도한 벤더 제약의 표현**이고 (ADR-0009 §6), 이 값은 **사람이 한 번에 붙여 넣고 확인할 수 있는 크기**에 대한 이 앱의 판단이다. 두 숫자는 서로를 근거로 삼지 않으며 한쪽을 고쳐도 다른 쪽은 따라 움직이지 않는다 |

**그 독립성이 코드로 못박혀 있다** [E1 · `portion.rs`의 테스트
`the_budget_is_this_apps_choice_and_not_the_notion_api_constraint`]. 그 검사는 두 가지를 함께
본다 — 두 상수가 **같은 값이 아니라는 것**과, 이 모듈의 제품 코드가 `CHUNK_MAX_BYTES` ·
`CHUNK_MAX_BLOCK_UNITS` · `split_markdown` **어느 것에도 기대지 않는다는 것**(소스 원문 검사).

**틀렸다면 고칠 자리는 상수 한 줄이다.** 줄이는 것도 늘리는 것도 이 앱의 판단이며, 그 판단이
바뀌어도 나눔 규칙과 화면은 그대로다.

### 12.8.3 나눔 규칙 — 무엇도 잃지 않는다

```text
문자열 ─┬─→ measure ─→ TextSize { bytes, chars, lines }      얼마나 큰가
        └─→ split   ─→ [Portion { index, total, text }, …]    예산을 넘으면 순서대로
```

**지키는 성질 하나 — 재조립 동등성.**

```text
split(text) 의 조각들을 이어 붙이면 text 다   (바이트 단위로 같다)
```

돌려주는 조각은 전부 **입력의 연속된 부분 슬라이스**이며, 경계에서 공백을 먹지도 더하지도
않는다. **조각이 자기 자리를 말하는 방법도 값이지 본문에 적어 넣는 표식이 아니다**
(`index` · `total`) — 표식을 본문에 섞으면 이어 붙인 결과가 원본이 아니게 되고, 무엇을 어떻게
보여 줄지는 화면의 일이다 (§12.7의 `PortionView`가 그 자리다).

**나누는 자리는 가장 큰 단위부터 찾는다.** 문단으로 충분하면 줄까지 내려가지 않는다.

```text
1. 빈 줄(문단 경계)에서 나눈다                              ← 기본
2. 한 문단이 혼자 예산을 넘으면 그 안의 줄 경계에서
3. 한 줄이 혼자 예산을 넘으면 그 안의 문장 경계에서          ← 여기까지가 "문장을 자르지 않는다"
4. 한 문장이 혼자 예산을 넘으면 낱말 경계에서
5. 낱말 하나가 혼자 예산을 넘으면 글자 경계에서              ← 마지막 수단
```

| 경계 | 결정 | 왜 |
| --- | --- | --- |
| **4와 5에서도 버리지 않는다** | 예산보다 긴 문장을 만나면 **문장 안에서 나눈다.** 내용을 버리거나 실패를 돌려주지 않는다 | 사람이 요청한 것은 "가져가기"이며, 크기 때문에 아무것도 주지 않는 것은 그 요청에 대한 답이 아니다. **잘린 문서를 성공이라고 부르지 않는 것**과 **크다는 이유로 거절하는 것**은 다르다 |
| **글자는 쪼개지지 않는다** | 5에서도 자르는 자리는 문자 경계다 | 한글 한 글자는 3바이트다. 바이트 자리로 자르면 깨진 글자가 만들어진다 |
| **예산과 같은 크기는 나누지 않는다** | `bytes <= PORTION_MAX_BYTES`면 조각 하나 | "거의 맞으니 잘라 버린다"가 없다. 한 글자를 넘기면 그때 두 조각이 된다 |
| **빈 문자열은 조각이 0개다** | 빈 조각을 만들어 내지 않는다 | 가져갈 것이 없다는 사실을 조각 0개로 말한다. (command 경계에서는 그것이 조각 하나로 세어져 *"1번째 / 전체 0개"* 라는 읽을 수 없는 자리가 생기지 않는다 — §12.7) |
| **결정적이다** | 같은 입력은 언제나 같은 조각을 낸다 | 시계도 난수도 해시맵 순회도 없다 (§18). 사람이 "3번째 조각"을 다시 요청했을 때 지난번과 같은 것을 받는다 |

이 모듈은 **파일 시스템 · 저장소 · 네트워크 · clipboard · 시계 · 로캘을 알지 않는다.**
문자열에서 문자열 조각을 만드는 함수뿐이다.

### 12.8.4 AI 경로의 transcript 모양이 압축됐다 — **§11의 export 형식은 바뀌지 않았다**

크기의 이유는 실측이 그대로 말한다: **5,139줄 중 1,711줄이 `### HH:MM:SS` 제목이었다.**
제목 한 줄과 그것을 감싸는 빈 줄은 **사람이 읽는 파일에서는 구조지만, 채팅 창에 붙여 넣는
문자열에서는 내용 없이 늘어나는 크기**다.

```text
Sectioned   ### 00:00:03            §11의 export 파일 · Notion 본문
            안녕하세요.

Compact     00:00:03 안녕하세요.     AI Handoff의 세 산출물
```

| | |
| --- | --- |
| **압축해도 잃는 것이 없다** | segment 하나가 여전히 자기 timestamp를 갖고, 순서도 개수도 그대로이며, 문장은 한 글자도 지워지지 않는다. 사라지는 것은 `### ` 네 글자와 줄바꿈뿐이다 — **요약이 아니라 같은 내용의 다른 모양이다** |
| **규칙이 복제되지 않았다** (§2-(3) · §5.4) | 모양은 `markdown.rs`의 `TranscriptShape` enum 하나이고, 모양이 고르는 것은 `segment_block`의 한 줄뿐이다. **어느 segment를 · 어떤 순서로 · 없을 때 무엇으로 대체하는가는 모양과 무관하게 같은 함수에서 온다** (`transcript_body` · `transcript_blocks`). `ai_request.rs`는 그 함수들을 부르며 `Compact`를 상수 하나로 고를 뿐, timestamp를 만들거나 segment를 도는 코드가 없다 |
| **한 줄로 접는 이유가 하나 더 있다** | 압축 모양의 문장은 `single_line`으로 접힌다 — 한 segment가 한 줄이면 **줄 경계가 곧 문장 경계가 되어** §12.8.3의 2단계에서 깨끗하게 나뉜다. 접는 것은 공백뿐이라 글자는 잃지 않는다 |
| **★ §11의 파일 형식은 이 모양을 모른다** | `markdown::render`는 **언제나 `Sectioned`** 로 적는다. Phase 5의 golden 테스트(`### 00:00:03`을 기대 문자열에 담고 있는 것들)가 **한 글자도 고쳐지지 않은 채 통과한다** [E3] |

**§11 불변의 판정 근거** [E1 · `.loop/evidence/TASK-072/section-11-invariance.txt`]:

```text
markdown_export.rs · notion_adapter.rs · notion_chunking.rs · notion_and_export_invariants.rs
  → §11의 기대 문자열을 담은 파일을 이 변경이 건드리지 않았다

export::markdown::tests::the_section_11_document_is_never_written_in_the_compact_shape
  → render() 의 산출물에 "### 00:00:03" 이 있고 "00:00:03 안녕하세요" 는 없다

export::ai_request::tests::the_three_outputs_use_the_compact_shape_and_the_section_11_document_does_not
  → 같은 전사 하나로 두 자리를 함께 본다 — AI 경로에는 "### " 가 없고 §11 문서에는 있다

바뀐 기대 문자열은 AI Handoff 경로의 것 하나뿐이다 (manual_ai_handoff.rs 의 Copy Transcript)
```

**§2-(2)가 정한 것 중 바뀐 것과 바뀌지 않은 것을 분명히 적는다.** §2-(2)는 *"timestamp는
§11과 **같은** `### HH:MM:SS`이며 `format_timestamp_ms`가 만든다"* 로 적었다.

```text
바뀌지 않은 것   timestamp 를 만드는 함수는 여전히 format_timestamp_ms 하나다.
                 AI-ready 문서의 섹션 순서(Mode → Instructions → Recording → Transcript)도
                 첫 줄(`# Molt Note AI Request`)도 그대로다 (§12.3).
                 §11 export 파일의 형식도 그대로다.

바뀐 것          AI Handoff 세 산출물 안에서 segment 를 적는 **모양**이
                 `### HH:MM:SS` 제목 + 본문에서 `HH:MM:SS 문장` 한 줄로 바뀌었다.
                 이것이 §2-(2)와 달라진 유일한 자리이며, 이유는 §12.8.1의 실측이다.
```

### 12.8.5 이 결정이 틀리면 무엇이 무너지는가

| 틀리는 방식 | 무너지는 범위 | 복구 |
| --- | --- | --- |
| 40,000이 실제 채팅에 너무 크다(또는 불필요하게 작다) | **조각 개수만 달라진다.** 내용은 어느 쪽이든 온전하다 | `PORTION_MAX_BYTES` 한 줄 |
| 압축 모양이 AI에게 §11 모양보다 읽기 어렵다 | AI Handoff 세 산출물의 본문 모양 | `ai_request.rs`의 `const SHAPE` 한 줄. **§11 파일은 애초에 이 모양을 쓰지 않으므로 함께 흔들리지 않는다** |
| 조각을 사람이 순서대로 붙여 넣기 번거롭다 | 사용 경험 | 이 절이 답하지 않는다 — **자동 Gate가 판정할 수 없는 Human Review 항목**이며 `docs/PHASE-5.6-HUMAN-REVIEW.md` HR-5가 묻는다 |

### 12.8.6 이 갱신이 바꾼 것과 바꾸지 않은 것

**바꾸지 않은 것** — §2의 여섯 결정 중 철회된 것은 없다. §4~§9의 결정 절도 §12.1~§12.7도
**한 글자도 지우거나 다시 쓰지 않았다.** 특히 §6(프롬프트 상수를 고치지 않는다) · §7(clipboard
경계) · §8.1(사용자 동작 하나에 이름 하나) · §10(벤더 중립)은 그대로다.

| 언제 | 무엇을 | 왜 |
| --- | --- | --- |
| 2026-09-07 (TASK-073) | **§12.7을 추가** — 인자와 응답이 넓어진 자리 | 크기와 나눔이 사람이 쓰는 자리까지 도달해야 했다 |
| 2026-09-07 (TASK-075) | **§12.8을 추가** — 실측 · 예산의 출처 · 나눔 규칙 · 압축 모양과 §11 불변 | §12.7이 *어떤 이름이 무엇을 받는가*까지만 적었고, **그 아래의 값과 규칙이 어느 문서에도 없었다.** 특히 *"예산이 벤더 제약이 아니다"* 는 코드 주석과 테스트에만 있었다 |

**이 Task는 문서만 바꿨다.** `src-tauri/`도 `src/`도 설정도 테스트도 건드리지 않았다.

---

## 13. 이 결정이 틀리면 무엇이 무너지는가

| 결정 | 틀렸을 때 무너지는 범위 | 복구 |
| --- | --- | --- |
| (2) 문서의 섹션 구성·순서 | 이미 내보낸 `.md` 파일들과 새 파일의 모양이 갈린다 | 파일은 사용자 것이고 앱이 다시 읽지 않는다 — 새 규칙으로 바꾸고 그 사실을 적는다 |
| (3) `markdown.rs` 함수 끌어올리기 | §11의 산출물 바이트가 흔들리면 Notion 본문까지 함께 흔들린다 | **기존 golden 테스트가 먼저 깨진다** — 나가기 전에 잡힌다 |
| (4) 상수 치환 (a)가 0회가 됨 | 모순된 프롬프트가 사용자 채팅으로 나간다 | 치환 횟수 단언이 test Gate에서 먼저 깨진다 (§6.2) |
| (5) clipboard A안이 플랫폼에서 막힘 | 세 동작 중 둘의 **편의**가 사라진다. 기능은 남는다 | 모듈 하나(부르는 자리 하나)를 §7.3의 B 또는 C로 교체 (§7.5) |
| (6) command 이름 셋 | 이름은 만들어지면 남는다 — 화면·테스트·문서가 그 이름을 안다 | Phase 안이라면 싸다. 그래서 지금 확정한다 |
| (1) Ollama 재배치 | 되돌리기 가장 싸다 — Settings의 자리와 문구뿐이다 | 코드와 계약은 그대로 있다 (§4.3) |

---

## 14. Source of Truth

`docs/PRODUCT-SPEC.md` §2.1(INV-3 · INV-5 · INV-6 · INV-8 · INV-9 · INV-10) · §5 · §7.2 · §7.3 ·
§9.3 · §9.5 · §9.6 · §11 · §12 · §13 · §18 · §19 · §21
· `docs/ADR-0008-note-ai-provider.md` §8 · §9.2 · §10
· `docs/ADR-0009-notion-and-export.md` §4 · §11 · §14
· `phase-prompt/05.5-manual-ai-handoff-and-ui-foundation.md`
· `docs/SYSTEM-MAP.md`

**이 Phase가 §21의 로드맵과 §14.5의 Ollama 서술을 바꾼다** (§4). 소급해 지우지 않고 언제 왜
바뀌었는지 남긴다 — 그 갱신을 하는 것은 P11이다.
