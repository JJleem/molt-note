# Molt Note — Product Specification

> 이 문서가 제품의 source of truth다.
> Phase Goal(`phase-prompt/`)과 Runtime Task는 이 문서를 참조하며, 이 문서를 넘어서 제품을
> 임의로 확장하지 않는다.
>
> 상태 표기: **DONE**(구현+검증 완료) · **PLANNED**(계획됨) · **DEFERRED**(의도적 보류) ·
> **CANDIDATE**(후보, 미선택). 설치된 의존성은 구현된 기능이 아니다.
>
> **개정 이력**
> - 2026-09-01 (rev 1) 최초 작성
> - 2026-09-01 (rev 2) Requirements Delta 반영 — Windows를 지원 대상 플랫폼으로 추가,
>   AI를 vendor 중립 Provider 추상화로 전환하고 core requirement에서 제외,
>   §14.7의 근거 없는 VERIFIED 표기 정정

---

## 1. Product Name / Direction

**Molt Note** — 개인용 Local-first Knowledge Recorder.

단순 녹음기가 아니다. 한 문장으로:

> 회의나 스터디에서 음성을 녹음하고, 로컬에서 전사한 뒤, 필요하면 AI로 읽기 좋은 기록으로
> 정리하고, 필요한 경우 Notion으로 내보낼 수 있는 개인용 도구.

서버 없이 혼자 동작한다. 외부 서비스는 사용자가 명시적으로 요청할 때만 호출된다.

개발과 검증은 macOS에서 먼저 하지만, **Windows도 지원 대상 플랫폼이다** (§3).

---

## 2. Product Principles

우선순위 순서다. 충돌하면 위쪽이 이긴다.

1. **Local First** — 네트워크 없이도 녹음·전사·열람·export가 동작한다.
2. **Simple** — 화면과 개념 수를 늘리지 않는다.
3. **Reliable Recording** — 녹음 신뢰성이 AI 기능보다 우선한다.
4. **Raw Data Preservation** — 원본 audio와 Raw Transcript는 불변으로 취급한다.
5. **AI는 보조 수단** — AI가 없어도 제품이 완전히 동작한다 (§6).
6. **Vendor 중립** — core/domain은 특정 AI 벤더에 의존하지 않는다 (§9).
7. **macOS First, Windows Supported** — macOS에서 먼저 개발·검증하되,
   architecture가 Windows 지원을 차단하지 않는다 (§3).
8. **서버 없이 사용 가능** — 계정도 클라우드 저장소도 없다.
9. **외부 연동은 명시적 opt-in** — 기본값은 전송하지 않는 것이다.

### 2.1 불변 규칙 (Domain Invariants)

이 규칙들은 어떤 Phase에서도 위반되지 않는다.

| ID | 규칙 |
| --- | --- |
| **INV-1** | AI는 원본 audio 파일을 수정하거나 삭제하지 않는다. |
| **INV-2** | AI는 Raw Transcript를 덮어쓰지 않는다. AI 산출물은 항상 별도 레코드다. |
| **INV-3** | 후처리(전사·AI·Notion) 실패는 원본 audio와 Recording 레코드에 영향을 주지 않는다. |
| **INV-4** | 사용자가 명시적으로 삭제하기 전까지 original audio를 자동 삭제하지 않는다. |
| **INV-5** | 외부로 나가는 데이터는 사용자가 그 사실을 UI에서 알 수 있어야 한다. |
| **INV-6** | Audio 파일 자체는 AI Provider나 Notion으로 전송하지 않는다. |
| **INV-7** | Secret(API key, integration token)은 frontend 소스와 저장소에 저장하지 않는다. |
| **INV-8** | **AI Provider의 부재·미설정·실패가 Recording · Transcript · Markdown Export 사용을 막지 않는다.** |
| **INV-9** | **core/domain은 특정 AI 벤더 타입에 의존하지 않는다.** 벤더 지식은 adapter 안에만 있다. |
| **INV-10** | **core/domain은 특정 OS에 의존하지 않는다.** 플랫폼 지식은 명시된 경계 안에만 있다 (§3.1). |

---

## 3. Target Platform / Constraints

| 구분 | 플랫폼 |
| --- | --- |
| **Primary development platform** | macOS (Apple Silicon) |
| **Supported target platform** | Windows (x64) |
| **Out of scope** | Linux · 모바일 |

| 항목 | 값 |
| --- | --- |
| 사용자 | 개인 1인 |
| 배포 | App Store / Microsoft Store 배포는 현재 범위 밖 |
| 서버 | 없음 |
| 계정 | 없음 |

### 3.1 Cross-platform 원칙

Windows에서 최종적으로 지원해야 하는 핵심 기능:

```text
Record · Pause · Resume · Stop
Local Audio Storage · Playback
Local Transcription
AI Note Generation
Markdown Export · Notion Sync
```

**Windows 실제 검증은 후속 Phase에서 수행한다** (§21의 Phase 6).
Phase 1에서 Windows 환경을 구축하거나 Windows E2E를 강제하지 않는다.

core/domain에 넣지 않는 macOS 가정 (INV-10):

```text
/Users/... 고정 경로            macOS filesystem 경로 규약
Apple Silicon 전용 binary 해석   macOS 전용 recording 타입의 domain 누출
macOS 권한 로직의 domain 누출     macOS 전용 process 경로 가정
```

대신 **실제 플랫폼 차이가 존재하는 곳에만** 경계를 둔다. 후보:

```text
AppDataDirectory   RecordingBackend   TranscriptionRunner
SidecarResolver    SecretStore        PlatformPermissions
```

> **미래를 예상해서 abstraction을 과도하게 만들지 않는다.**
> 현재 Phase에서 실제로 플랫폼이 갈리는 지점만 추상화한다.
> 구현이 하나뿐인 인터페이스를 "언젠가 필요할 것"이라는 이유로 만들지 않는다.

---

## 4. Core User Flow

```text
Molt Note 실행
  ↓
새 녹음 (제목 입력 · 마이크 선택)
  ↓
Record → Pause / Resume → Stop
  ↓
로컬 audio 파일 저장 (여기서 원본은 확정된다)
  ↓
Recording 레코드 생성
  ↓
Local Whisper transcription
  ↓
Raw Transcript 저장 (immutable)
  ↓
━━━━━━━━━ 여기까지가 AI 없이 완결되는 core pipeline (INV-8) ━━━━━━━━━
  ↓                                    ↓
[선택] Note AI Provider          Markdown Export
  ↓                                    (AI 없이 동작한다)
Structured Note
  ↓
[선택] Notion 전송
```

**핵심 두 가지**

1. `Stop` 이후 어느 단계가 실패하든 audio 원본과 Recording은 남는다.
   각 후처리 단계는 독립적으로 재시도 가능하다.
2. **AI Provider가 하나도 설정되어 있지 않아도** 녹음 → 전사 → 검토 → Markdown Export까지
   완전히 동작한다 (INV-8).

---

## 5. Screens

초기 제품은 화면 4개를 넘기지 않는다.

### A. Recordings (기본 화면)

녹음 목록. 항목마다: 제목 · 날짜 · 길이 · transcription 상태 · AI 정리 상태 · Notion sync 상태.

```text
Molt Note

Recordings
────────────────────────
3DGS Study #04
Sep 1 · 52:31
Transcript ✓   AI Note ✓
────────────────────────
Scene Scout Meeting
Aug 31 · 38:22
Transcript ✓
────────────────────────
```

### B. Recording (녹음 화면)

제목 · 선택된 microphone · 경과 시간 · Record / Pause / Resume / Stop.

녹음 중이라는 사실과 경과 시간이 가장 크고 명확해야 한다. 장식적 시각 효과를 넣지 않는다.

```text
Molt Note

3DGS Study #04

        ● REC

        42:18

     Pause       Stop

MacBook Microphone
```

### C. Recording Detail

재생 + `AI Note` / `Transcript` / `Recording` 탭 + `Send to Notion` / `Export Markdown`.

AI Provider가 없거나 AI Note가 없을 때도 이 화면은 정상 동작한다 (INV-8) —
Transcript 탭과 Export는 그대로 쓸 수 있어야 한다.

### D. Settings

| 그룹 | 항목 |
| --- | --- |
| Recording | default microphone · recordings directory |
| Transcription | whisper model · language · automatic transcription ON/OFF |
| **AI Provider** | **provider 선택 · provider별 설정 · 연결 확인 · automatic AI processing ON/OFF** |
| Notion | integration token · destination · connection test · automatic sync ON/OFF |

Secret 값은 INV-7을 따른다.

---

## 6. Recording Requirements

녹음은 이 제품에서 가장 중요한 기능이며 AI 기능보다 우선한다.

지원 범위: microphone enumeration · selection · permission · Record · Pause · Resume · Stop ·
duration · local file 생성 · playback.

| ID | 요구사항 |
| --- | --- |
| **R-001** | 녹음 중 앱 UI state가 변경되더라도 Recording session이 쉽게 유실되지 않아야 한다. |
| **R-002** | Stop 시 결과 파일이 실제 filesystem에 생성된 것이 확인되어야 한다. |
| **R-003** | 전사 / AI / Notion 실패와 무관하게 원본 audio는 유지된다. |
| **R-004** | 사용자가 명시적으로 삭제하기 전까지 original audio를 자동 삭제하지 않는다. |
| **R-005** | 앱 crash나 예상치 못한 오류에서 데이터 유실 가능성을 최소화하도록 lifecycle을 설계한다. |

### 6.1 Recording Engine — 결정해야 할 아키텍처 선택 (CANDIDATE)

구현 전에 가장 신뢰할 수 있는 recording architecture를 조사하고 선택한다.
**"브라우저에서 동작한다" 또는 "macOS에서 가장 쉽다"는 이유만으로 선택하지 않는다.**

평가 대상은 최소한 다음 둘이다.

```text
WebView / MediaRecorder      vs      Rust/native audio path (예: cpal)
```

평가 기준 (전부 ADR에 기록한다):

```text
recording reliability      microphone enumeration     microphone selection
pause/resume               file finalization          data-loss behavior
audio format               transcription compatibility
macOS permission behavior  Windows compatibility
Tauri integration          packaging
testability                maintenance cost
```

특히 **transcription compatibility**(§8의 whisper 입력 포맷), **Windows compatibility**(§3),
**data-loss behavior**(R-005)가 결정적이다.

이 결정은 **Phase 2에서 실제 검증과 함께 내리고 ADR로 기록한다.**
후보와 현재까지 확인된 사실은 §14.3에 있다.

---

## 7. Local Data Model

개념 모델이다. 구현 중 합리적인 변경은 가능하되, **변경 이유를 기록한다.**

```text
Recording
  id · title · createdAt · updatedAt
  duration
  audioPath · audioFormat
  microphone
  transcriptionStatus · aiStatus · notionStatus

Transcript                      (immutable · Recording 1:1)
  recordingId
  language
  segments[] { start · end · text }
  rawText
  createdAt
  engine · model

AINote                          (derived · 재생성 가능 · Recording 1:N)
  recordingId
  type            (meeting | study | summary)
  content         (§9.3의 provider 중립 structured note)
  provider        ← provenance
  model           ← provenance
  promptVersion   ← provenance
  generatedAt     ← provenance

NotionSync
  recordingId
  pageId · syncedAt · status · error
```

상태 필드(`transcriptionStatus` 등)는 최소한 다음을 구분할 수 있어야 한다:
`none` · `pending` · `running` · `done` · `failed`. 실패는 사용자에게 보이고 재시도 가능해야 한다.

**Transcript와 AINote는 서로 다른 테이블/레코드다.** INV-2에 의해 하나가 다른 하나를 덮어쓰지 않는다.

`Recording`은 **source data**, `AINote`는 **derived data**다. 이 구분이 INV-1~INV-3의 근거다.
AI Provider는 `Recording`의 AI 상태 필드 외에 recording metadata를 변경하지 않는다.

---

## 8. Transcription

Local processing이 기본이다. 원격 전사 API는 쓰지 않는다.

```text
audio file → (필요한 전처리) → whisper → timestamped transcript → local DB
```

Segment timestamp를 보존한다.

```text
00:02:14 → 00:02:21
그러면 이번에는 PLY 먼저 변환하고
그다음 SOG 변환 확인하면 될 것 같아요.
```

Transcript는 immutable source로 취급한다. 재전사는 새 Transcript를 만드는 행위이며,
기존 것을 몰래 수정하는 행위가 아니다.

whisper 통합 방식(sidecar / 바인딩), 모델 관리, 입력 포맷 변환 책임, 그리고
**macOS와 Windows 양쪽의 binary 확보 전략**은 §14.4의 확인된 사실을 근거로
**Phase 3에서 확정한다.**

---

## 9. Note AI Provider System

### 9.1 역할과 경계

AI는 transcription engine이 **아니다.** 역할은 하나다:

> Raw Transcript → 사람이 읽기 좋은 structured note

```text
Transcript  →  Note AI Provider  →  Structured Note
```

**core/domain은 Claude · Gemini · Ollama 같은 특정 벤더에 의존하지 않는다 (INV-9).**

```text
Molt Note domain   ≠   Claude-specific domain
```

벤더 지식(SDK · 엔드포인트 · 요청 형태 · 에러 코드)은 **adapter 안에만** 존재한다.
domain은 `NoteAIProvider` 계약과 §9.3의 structured note 타입만 안다.

### 9.2 Provider 계약 (설계 의도)

아래는 **의도 설명용**이다. 실제 architecture에 더 적합한 contract가 있으면 변경 가능하며,
변경 시 이유를 기록한다. 바뀌면 안 되는 것은 계약의 형태가 아니라 INV-9다.

```ts
interface NoteAIProvider {
  id: string;
  name: string;

  isAvailable(): Promise<boolean>;

  generateNote(input: {
    transcript: TranscriptInput;
    mode: "meeting" | "study" | "summary";
  }): Promise<StructuredNote>;
}
```

`isAvailable()`이 존재하는 이유는 INV-8이다 — 제품은 provider가 없는 상태를
**정상 상태로** 다뤄야 하며, 없는 것을 오류로 표시하지 않는다.

### 9.3 Structured Note — provider 중립 데이터

Provider가 돌려준 임의의 Markdown을 core data model로 쓰지 않는 것을 **우선 검토한다.**
Provider-independent structured data를 권장한다.

Meeting:

```json
{ "overview": "...", "keyPoints": [], "decisions": [], "actionItems": [], "questions": [] }
```

Study:

```json
{ "overview": "...", "keyConcepts": [], "importantDetails": [],
  "questions": [], "thingsToStudy": [], "references": [] }
```

Summary: 짧은 요약과 key points.

렌더링 방향은 하나로 흐른다:

```text
Provider → Structured Note → UI renderer
                           → Markdown renderer
                           → Notion renderer
```

**최종 schema는 해당 Phase(§21의 Phase 4)에서 확정한다.**
구조화 데이터를 얻는 수단(모델의 structured output 기능 / 응답 파싱)은 provider마다 다를 수 있으며,
그 차이는 adapter가 흡수한다. 어느 쪽이든 **응답이 예상과 다를 때 앱이 깨지지 않아야 한다.**

### 9.4 Provider Strategy

**Primary local provider — Ollama.**

```text
Raw Transcript → Local Ollama → Structured Note
```

V1에서 **Ollama를 앱 installer에 bundle하지 않는다.**
사용자가 별도로 실행 중인 local Ollama instance에 연결하는 방식을 우선 검토한다.

**Optional cloud providers — DEFERRED.**
Claude · Gemini · Groq는 향후 adapter 후보다. V1에서 전부 구현하지 않는다.
Provider abstraction을 실제로 검증하는 데 필요한 최소 범위만 구현한다 (§21).

- **Claude를 필수 dependency로 만들지 않는다.**
- **Claude Agent SDK는 사용하지 않는다** — 단일 요청/응답 변환에 agentic loop가 필요한
  명확한 이유가 없다.
- **Free Tier나 현재 가격 정책을 architecture dependency로 만들지 않는다.**
  외부에서 바뀔 수 있는 값이다.

### 9.5 AI Modes

| Mode | 출력 섹션 |
| --- | --- |
| **Meeting** | Overview · Key Discussions · Decisions · Action Items · Open Questions |
| **Study** | Overview · Key Concepts · Important Details · Questions · Things to Study · References Mentioned |
| **Summary** | Short Summary · Key Points |

### 9.6 규칙

- AI 실패는 transcript와 recording에 어떤 영향도 주지 않는다 (INV-3).
- AI 결과는 항상 재생성 가능하다. 재생성이 원본을 훼손하지 않는다 (INV-2).
- **provenance를 남긴다** — `provider` · `model` · `promptVersion` · `generatedAt` (§7).
  나중에 "이 노트가 무엇으로 언제 만들어졌는지" 알 수 있어야 한다.
- Audio는 전송하지 않는다 (INV-6). 전송되는 것은 transcript 텍스트뿐이다.
- **Provider 부재·미설정·실패가 core pipeline을 막지 않는다 (INV-8).**

---

## 10. Notion Integration

AI Note 또는 Transcript를 Notion 페이지로 전송한다.

기본 페이지 구조:

```text
Title
Date · Duration
Summary
Key Points / Key Concepts
Decisions
Action Items
Transcript
```

Meeting / Study 타입에 따라 section 구성은 달라진다.
**AI Note가 없어도 Transcript만으로 전송 가능해야 한다** (INV-8).

### 규칙

- Notion sync 실패가 local data에 영향을 주면 안 된다 (INV-3).
- **중복 sync 정책을 명시적으로 결정하고 문서화한다** — 기존 페이지 업데이트인가,
  명시적 중복 생성인가. `NotionSync.pageId`가 이 정책의 근거 데이터다.
- 실패 시 `NotionSync.status`와 `error`가 사용자에게 보여야 한다.

---

## 11. Markdown Export

모든 Recording은 외부 서비스에 종속되지 않아야 한다. 최종 기록을 Markdown으로 export한다.

**Markdown Export는 core pipeline의 일부이며 AI Provider 없이 동작한다 (INV-8).**

```text
exports/2026-09-01-3dgs-study-04.md
```

```markdown
# 3DGS Study #04

Date: 2026-09-01
Duration: 52:31

## Overview
...

## Key Concepts
...

## Transcript

### 00:00:03
...
```

AI Note가 있으면 포함하고, 없으면 Transcript와 메타데이터만으로 유효한 문서가 나와야 한다.

Notion · NotebookLM · Obsidian 등에서 그대로 쓸 수 있는 형태를 목표로 한다.

**NotebookLM 자동 연동은 하지 않는다.** Markdown interoperability만 제공한다.

---

## 12. Privacy Boundary

데이터가 어디까지 가는지는 세 단계로 나뉜다.

```text
━━━━━━━━━━━ LOCAL — 기기 밖으로 나가지 않는다 ━━━━━━━━━━━
Microphone → Audio → Whisper → Raw Transcript
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

━━━━━━━━━ LOCAL — 사용자의 로컬 Ollama로만 간다 ━━━━━━━━━
Raw Transcript → Ollama → Structured Note
   (기기 내부 또는 사용자가 지정한 로컬 인스턴스. 인터넷으로 나가지 않는다)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

━━━━━━━━━━━━━━━━ EXTERNAL — 명시적 opt-in ━━━━━━━━━━━━━━━
Raw Transcript  ──→ 선택된 Cloud AI Provider (사용 시에만)
Selected Note / Transcript content ──→ Notion (사용 시에만)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

절대 나가지 않는 것:  Original Audio (INV-6)
```

**Local provider(Ollama)와 Cloud provider는 UI에서 구분되어야 한다.**
사용자는 자신이 고른 provider가 로컬인지 외부인지 알 수 있어야 한다 (INV-5).

---

## 13. Failure Handling

아래는 버그가 아니라 **정상적인 제품 상태**다. 각각에 대해 사용자는 세 가지를 알 수 있어야 한다:

```text
무엇이 실패했는가 · 원본 데이터는 안전한가 · 다시 시도할 수 있는가
```

| 영역 | 실패 |
| --- | --- |
| 권한/장치 | microphone permission denied · microphone disconnected |
| 녹음 | recording initialization failure · recording write failure · disk write failure |
| 전사 | transcription process failure · unsupported whisper model · 모델 파일 없음 |
| **AI Provider** | **provider 미설정 · provider 연결 불가(예: Ollama 미실행) · 모델 없음 · 요청 실패 · rate limit · 인증 실패 · 응답이 기대 schema와 다름** |
| Notion | authentication failure · sync failure |
| 앱 | application restart during processing |

**에러를 console에만 남기지 않는다.** 모든 실패는 사용자에게 보이는 상태로 표현된다.

**AI Provider 관련 상태 중 "provider가 없음"은 오류가 아니라 정상 상태다 (INV-8).**
경고나 에러가 아니라, AI 기능이 비활성이라는 사실을 담담히 알리는 것으로 표현한다.
provider별 에러 의미는 adapter가 domain 공통 실패 타입으로 변환한다 (INV-9).

---

## 14. Technical Direction

> 이 절의 외부 사실은 **2026-09-01에 실제로 확인한 값**이다.
> `VERIFIED`는 출처를 확인한 사실, `UNVERIFIED`는 아직 확인하지 못한 것이다.
> **확인되지 않은 것을 구현 근거로 쓰지 않는다.**

### 14.1 개발 환경 (실측 · VERIFIED 2026-09-01)

macOS가 primary development platform이다 (§3).

| | 값 | 확인 방법 |
| --- | --- | --- |
| macOS | 26.6.2 | `sw_vers -productVersion` |
| 아키텍처 | arm64 (Apple Silicon) | `uname -m` |
| Node.js | v24.14.0 | `node -v` |
| npm | 11.9.0 | `npm -v` |
| Rust | rustc 1.94.0 / cargo 1.94.0 | `rustc --version` |
| C 컴파일러 | Apple clang 17.0.0 | `clang --version` |
| Xcode | Command Line Tools — **full Xcode 아님** | `xcode-select -p` |
| ffmpeg | 8.1.1 | `ffmpeg -version` |
| cmake | **없음** | `cmake --version` → not found |
| Homebrew | 6.0.6 | `brew --version` |

> Windows 개발/검증 환경은 아직 존재하지 않는다. Phase 6에서 구축한다.

### 14.2 Stack

Tauri v2 + React + TypeScript + Vite + Rust backend.

| 항목 | 버전 / 사실 | 출처 |
| --- | --- | --- |
| Tauri는 v2가 현재 stable line (v3 core 릴리스 없음) | — | github.com/tauri-apps/tauri releases |
| `@tauri-apps/cli` | 2.11.4 | registry.npmjs.org |
| `@tauri-apps/api` | 2.11.1 | registry.npmjs.org |
| `tauri` crate | 2.11.5 (2026-07-01) | crates.io |
| Tauri v2 MSRV | Rust 1.77.2 | crates.io metadata + workspace `Cargo.toml` |
| scaffold | `npm create tauri-app@latest` → React → TypeScript | v2.tauri.app/start/create-project |

**macOS 요구사항**: desktop 전용이면 **Xcode Command Line Tools로 충분** (full Xcode는 iOS 타깃에만 필요).
출처: v2.tauri.app/start/prerequisites

**Windows 요구사항** (VERIFIED · v2.tauri.app/start/prerequisites · /distribute/windows-installer):

| 항목 | 사실 |
| --- | --- |
| WebView2 런타임 | **Windows 10 1803+ 및 Windows 11에 기본 설치되어 있다** |
| 빌드 도구 | Microsoft C++ Build Tools (Desktop development with C++ workload) |
| Rust 툴체인 | MSVC host triple — `rustup default stable-msvc` (`x86_64-pc-windows-msvc` 등) |
| 번들 타깃 | `msi` (WiX v3, **Windows에서만 빌드 가능**) · `nsis` (`-setup.exe`, 교차 빌드 가능) |

> `msi`는 Windows 머신 없이는 만들 수 없다. Phase 6의 packaging 계획에 직접 영향을 준다.

**Cross-platform 경로 API** (VERIFIED — §3.1의 `AppDataDirectory` 경계에 직접 쓰인다):

| 계층 | API |
| --- | --- |
| Rust | `tauri::path::PathResolver` — `app_data_dir()` · `app_local_data_dir()` · `app_config_dir()` · `app_cache_dir()` (docs.rs/tauri) |
| JS | `@tauri-apps/api/path` — `appDataDir()` · `appLocalDataDir()` |

Windows에서 `appDataDir()`는 `{FOLDERID_RoamingAppData}/{bundleIdentifier}`로 해석된다.
**즉 플랫폼별 경로를 직접 조합할 이유가 없다** — Tauri가 이미 제공한다.

### 14.3 Recording Engine — 후보와 확인된 사실 (CANDIDATE · 결정은 Phase 2)

#### 후보 A — Webview MediaRecorder

| | 확인된 사실 |
| --- | --- |
| 동작 여부 | WKWebView에서 `getUserMedia` / `MediaRecorder`는 원리상 동작한다 |
| **출력 포맷** | ⚠️ WebKit 기준이라 기본이 **MP4/AAC**다. WebM/Opus는 Safari 18.4(2025-03)부터, audio-only PCM/ALAC은 WebKit STP 214 경로. **PCM WAV를 직접 준다는 보장이 없다** → §14.4의 whisper 입력 요구와 충돌하며 변환 단계가 추가로 필요하다 |
| pause/resume | 스펙에 있고 Safari 14.1+ 지원 (VERIFIED). 2026년 macOS의 Tauri 번들 WKWebView 조합 실동작은 **UNVERIFIED** |
| 알려진 문제 | Tauri #11951(open): dev 실행에서 마이크 권한 프롬프트가 뜨지 않음. #5853: `getUserMedia`가 앱을 멈추게 한다는 보고 |
| **Windows** | webview가 WKWebView가 아니라 **WebView2(Chromium)** 이므로 코덱·동작이 macOS와 **다르다.** 즉 이 후보는 **플랫폼마다 다른 출력 포맷**을 낼 수 있다 — **UNVERIFIED**, Phase 2/6에서 확인 |

#### 후보 B — Rust 측 캡처 (`cpal` + `hound`)

| | 확인된 사실 |
| --- | --- |
| `cpal` | 0.18.2 (2026-08-16) |
| macOS 백엔드 | CoreAudio (기본), 입력 캡처 지원 |
| **Windows 백엔드** | **WASAPI (기본), 입력 캡처 지원.** ASIO·JACK은 선택 feature. 최소 Windows 8 |
| ⚠️ MSRV | cpal는 **Rust ≥ 1.85** 를 요구한다 — Tauri의 1.77.2보다 높다. 로컬 1.94.0은 만족 |
| 출력 | raw PCM 콜백. 파일 writer가 없으므로 WAV 작성에 `hound` 필요 |
| `hound` | 3.5.1 — 안정적이고 널리 쓰이나 **2023-09 이후 릴리스 없음**(dormant) |
| 장점 | PCM을 직접 다루므로 §14.4의 16kHz mono 16-bit WAV를 **변환 없이** 만들 수 있고, **두 플랫폼에서 같은 포맷**이 나온다 |

#### 후보 C — 커뮤니티 Tauri 플러그인

**공식 Tauri 오디오 녹음 플러그인은 없다.** 커뮤니티 플러그인 2개 모두 내부적으로 cpal + hound를 쓴다.

| 플러그인 | 상태 |
| --- | --- |
| `tauri-plugin-audio-recorder` (brenogonzaga) | ★19 · 2026-08-23 push (활성) · 0.1.x (pre-1.0) |
| `tauri-plugin-mic-recorder` (ayangweb) | ★26 · 2025-06-11 push (**1년 이상 미갱신**) |

> 둘 다 공식이 아니고 star 수가 적으며 개인 유지보수다. **관찰이지 결정이 아니다.**

#### 마이크 권한 — 플랫폼 차이

| 플랫폼 | 사실 |
| --- | --- |
| macOS | `src-tauri/Info.plist`에 `NSMicrophoneUsageDescription`을 넣으면 Tauri CLI 생성값과 **병합**된다 (VERIFIED · v2.tauri.app/distribute/macos-application-bundle). `tauri.conf.json` 키가 아니다. entitlements는 별개이며 `bundle.macOS.entitlements`를 쓴다 |
| Windows | 일반 Win32 데스크톱 앱(비패키지)에는 **manifest 선언이 불필요하다.** capability 선언은 UWP/MSIX-AppContainer에만 해당 (VERIFIED-by-secondary-corroboration — MS Learn 직접 fetch 실패) |
| Windows | 단, **Settings › Privacy & Security › Microphone 의 "Let desktop apps access your microphone" 토글이 OS 수준에서 차단할 수 있다.** 차단 시 앱이 받는 정확한 오류는 **UNVERIFIED** — 스트림 초기화 실패로 나타날 것으로 보이나 Phase 6에서 실측 필요 |

> **이 표가 §17(P8)의 근거다** — `NSMicrophoneUsageDescription`은 **macOS packaging 요구사항**이지
> Molt Note의 범용 권한 구현이 아니다.

#### Phase 2에서 반드시 실제 확인할 것 (현재 UNVERIFIED)

1. WKWebView MediaRecorder의 실제 컨테이너/코덱, pause/resume 동작
2. 번들된 `.app`에서 마이크 TCC 프롬프트가 우회 없이 뜨는가 (#11951)
3. 1시간 규모 녹음에서의 안정성과 crash 내성 (R-005)
4. 후보 A를 택할 경우 **WebView2와 WKWebView의 출력 포맷이 일치하는가**

### 14.4 Transcription — whisper.cpp (확인된 사실 · 결정은 Phase 3)

canonical repo `github.com/ggml-org/whisper.cpp`. 최신 릴리스 **v1.9.3 (2026-08-20)**.

| 항목 | 확인된 사실 |
| --- | --- |
| 빌드 | `cmake -B build` → `cmake --build build -j --config Release`. **CMake 필요** (Makefile 경로는 사라졌다) |
| ⚠️ macOS 제약 | §14.1 기준 이 기기에 **cmake가 없다** |
| 가속 (macOS) | Apple 플랫폼에서 **Metal이 기본 ON**. Core ML은 기본 OFF (`-DWHISPER_COREML=1` + 별도 변환 스크립트) |
| 가속 (Windows) | CUDA `-DGGML_CUDA=1` · Vulkan `-DGGML_VULKAN=1` · OpenBLAS `-DGGML_BLAS=1` · ROCm `-DGGML_HIP=1`. **전부 기본 OFF** — 기본 빌드는 CPU 전용 |
| CLI 바이너리 | **`whisper-cli`** (과거의 `main`이 아니다). 산출 위치 `./build/bin/whisper-cli` |
| **입력 포맷** | **16-bit WAV만** 받는다 (모델은 16kHz mono 기준). 기본 빌드는 miniaudio로 WAV만 디코드. 변환 예: `ffmpeg -i in.mp3 -ar 16000 -ac 1 -c:a pcm_s16le out.wav`. (`-D WHISPER_COMMON_FFMPEG=yes` 옵션이 있으나 기본 아님) |

**⚠️ prebuilt 바이너리의 플랫폼 비대칭 (VERIFIED · releases/tag/b4938):**

| 플랫폼 | prebuilt |
| --- | --- |
| **Windows x64** | **있다** — `whisper-bin-x64.zip`(CPU) · `whisper-blas-bin-x64.zip`(OpenBLAS) · `whisper-cublas-11.8.0-bin-x64.zip` · `whisper-cublas-12.4.0-bin-x64.zip`(CUDA). Vulkan prebuilt은 없다 |
| **macOS** | **없다** — 소스 빌드가 전제다 |

> 즉 **더 어려운 쪽이 primary 개발 플랫폼인 macOS다.** Phase 3의 binary 확보 전략은
> 이 비대칭을 전제로 세워야 한다.

**Timestamped 출력** (VERIFIED · `examples/cli/cli.cpp` 소스 직접 확인):
플래그 `-oj`/`--output-json`, `-ojf`(토큰 단위 포함), 그 외 `-osrt` · `-ovtt` · `-otxt`.

```text
{ systeminfo, model{...}, params{...}, result{ language },
  transcription: [
    { timestamps: { from: "HH:MM:SS,mmm", to: "HH:MM:SS,mmm" },
      offsets:    { from: <int>, to: <int> },     // 단위: 밀리초
      text: "..." }
  ] }
```

`-ojf`는 각 segment에 `tokens[]`(text · timestamps · id · 확률 `p`)를 추가한다.
Word-level은 `-ml 1` 또는 `-ojf`의 토큰 timestamp로 근사한다.

**모델** (`models/download-ggml-model.sh`, HF `huggingface.co/ggerganov/whisper.cpp`):
`tiny` ≈75MiB · `base` ≈142MiB · `small` ≈466MiB · `medium` ≈1.5GiB ·
`large-v1/v2/v3` ≈2.9GiB · `large-v3-turbo` · quantized 변형(`-q5_0` 등).
공식 스크립트에 `distil-*`는 없다.

> 한국어+영어 혼용 1시간 녹음에 `large-v3` / `large-v3-turbo`가 현실적이라는 것은
> **추론이며 whisper.cpp 문서의 명시적 주장이 아니다 (UNVERIFIED).** Phase 3에서 실측한다.

**Rust 바인딩**: `whisper-rs` 0.16.0 (whisper-rs-sys 0.15.0, Metal feature).
주 저장소가 **Codeberg(`codeberg.org/tazz4843/whisper-rs`)로 이전**했고 GitHub 쪽은 2025-07-30 archived.
번들된 whisper.cpp 커밋 핀은 UNVERIFIED.

**Tauri sidecar** (VERIFIED · v2.tauri.app/develop/sidecar):
`tauri.conf.json`의 `bundle.externalBin`에 `src-tauri/` 기준 상대 경로를 넣는다.
파일명에 **target triple 접미사가 필수**다:

```text
macOS  (Apple Silicon)  src-tauri/binaries/whisper-cli-aarch64-apple-darwin
Windows (x64)           src-tauri/binaries/whisper-cli-x86_64-pc-windows-msvc.exe
```

Windows는 **`.exe` 확장자를 포함한다.** 호출에는 capabilities에 shell 권한이 필요하다:

```json
{ "identifier": "shell:allow-execute",
  "allow": [{ "name": "binaries/whisper-cli", "sidecar": true }] }
```

⚠️ Tauri #11992 "Codesigning and notarization issue when using ExternalBin" 이슈 존재.
해결 여부 UNVERIFIED — Phase 3에서 확인할 위험 항목이다.

### 14.5 Note AI Provider — Ollama (확인된 사실 · 구현은 Phase 4)

**Ollama 최신 릴리스 v0.33.2 (2026-08-27)** · github.com/ollama/ollama/releases

| 항목 | 확인된 사실 (출처: ollama/ollama `docs/api.md` · `docs/faq.mdx` · `server/routes.go`) |
| --- | --- |
| 기본 주소 | **`http://localhost:11434`** — 기본 bind는 `127.0.0.1:11434`, `OLLAMA_HOST`로 변경 |
| 모델 목록 | `GET /api/tags` → `{"models":[{name, model, modified_at, size, digest, details{...}}]}` |
| 생성 | `POST /api/generate` · `POST /api/chat` 둘 다 존재. 기본은 스트리밍 JSON 시퀀스이며 **`"stream": false`** 면 단일 JSON |
| OpenAI 호환 | `/v1/chat/completions` 등 존재. 단 `tool_choice`·`logit_bias`·`n`·`logprobs` 미지원이고 **요청별 context 크기를 지정할 수 없다** |
| 헬스 체크 | **`GET /` → 200 `"Ollama is running"`** (서버 소스에서 직접 확인) · `GET /api/version` |
| 미실행 시 | TCP 연결 거부 (connection refused) |

**구조화 출력** (VERIFIED — Phase 4의 §9.3에 직접 쓰인다):
`format` 파라미터가 문자열 `"json"` **또는 완전한 JSON Schema 객체**를 받는다.
`/api/generate`와 `/api/chat` 양쪽에서 동작한다.

```json
{ "model": "...", "prompt": "...", "stream": false,
  "format": { "type": "object",
              "properties": { "overview": {"type":"string"},
                              "keyPoints": {"type":"array","items":{"type":"string"}} },
              "required": ["overview"] } }
```

> JSON-schema `format` 지원이 v0.5(2024-12)에 도입됐다는 것은 2차 출처 기반이다
> (VERIFIED-by-secondary-source). 현재 문서에 기능이 존재한다는 사실은 VERIFIED다.

**⚠️ 아키텍처에 직접 영향을 주는 두 가지 사실**

1. **CORS** — 기본 허용 origin은 `127.0.0.1`과 `0.0.0.0`뿐이다 (`OLLAMA_ORIGINS`로 확장).
   Tauri webview origin(`tauri://localhost` 등)은 기본 허용 목록에 **없다.**
   따라서 프론트엔드에서 직접 `fetch`하면 CORS로 막힐 가능성이 높고,
   사용자가 환경변수를 설정해야 하는 제품이 된다.
   **Rust backend에서 호출하면 이 문제가 발생하지 않는다** (브라우저 CORS 대상이 아님).
   → §9.4의 "호출 주체 결정"은 이 사실을 근거로 Phase 4에서 판단한다.

2. **기본 context window가 4096 토큰이다** (`options.num_ctx`로 요청별 지정,
   `OLLAMA_CONTEXT_LENGTH`로 서버 기본값 변경).
   **1시간 transcript(대략 8~12K 단어, 12~20K+ 토큰)는 기본값을 초과한다.**
   → Phase 4는 청킹하거나 `num_ctx`를 명시적으로 키워야 하며,
   키우는 것은 사용자 기기의 RAM/VRAM에 제약된다. **이것은 선택이 아니라 필수 설계 항목이다.**

**클라이언트 라이브러리**

| | 이름 | 버전 | 공식 여부 |
| --- | --- | --- | --- |
| JS/TS | `ollama-js` | 0.6.3 (2025-11-13) | **공식** (ollama org) |
| Python | `ollama-python` | 0.6.2 (2026-04-29) | **공식** |
| Rust | `ollama-rs` | 0.3.6 (2026-07-24) | **커뮤니티** (공식 Rust SDK 없음) |

> Rust backend에서 호출한다면 `reqwest`로 REST를 직접 쓰는 것도 합리적 선택지다.
> 공식 Rust SDK가 없으므로 wrapper crate 의존이 필수는 아니다.

**한국어+영어 혼용에 후보가 되는 로컬 모델** (Ollama 라이브러리에 존재함이 확인된 것):

| 모델 | 사실 |
| --- | --- |
| `exaone3.5` (LG AI Research) | **영어/한국어 이중언어를 명시**. 2.4B~32B. `exaone4.0` · `exaone-deep`도 존재 |
| `kanana` (Kakao) | 한국어/영어 이중언어 계열. 라이브러리 존재는 VERIFIED, **정확한 크기 태그는 UNVERIFIED** |
| `qwen3` | 다국어. 크기 확인됨 — 0.6b(523MB) · 1.7b(1.4GB) · 4b(2.5GB) · 8b(5.2GB) · 14b(9.3GB) · 30b(19GB) · 32b(20GB) |

> 어느 모델이 이 제품의 transcript에 실제로 적합한지는 **UNVERIFIED**다.
> Phase 4의 Human Review 항목이며, 문서가 대신 판정하지 않는다.

### 14.6 Cloud AI Providers — DEFERRED

V1에서 구현하지 않는다 (§16). 아래는 향후 adapter를 만들 때의 참고 사실이며,
**architecture dependency가 아니다.** 가격과 티어 정책은 외부에서 바뀐다 (§9.4).

2026-09-01 기준 Anthropic API 모델 (platform.claude.com/docs/en/models/overview):
`claude-fable-5` · `claude-opus-5` · `claude-sonnet-5` (각 context 1M / max output 128K) ·
`claude-haiku-4-5-20251001` (200K / 64K).
TypeScript SDK `@anthropic-ai/sdk` 0.122.0. **공식 Rust SDK는 없다.**
구조화 출력은 `output_config.format`(type `json_schema`)를 쓴다.
에러는 429 `rate_limit_error` 등이며, **지출 상한으로 인한 429에는 `retry-after`가 없다.**

Gemini · Groq는 조사하지 않았다 (**UNVERIFIED**). 필요해질 때 확인한다.

### 14.7 Local Persistence (확인된 사실 · 선택은 Phase 1)

> **이 절은 rev 1의 문서 결함을 고치기 위해 신설됐다.** rev 1의 검증 요약표는
> persistence crate 버전이 확인된 것처럼 적었으나, 실제 버전 값이 본문 어디에도 없었다.
> 근거는 아래에 두고, 요약표(§14.8)는 본문이 실제로 담고 있는 것만 주장한다.

| 후보 | 버전 | 확인 상태 |
| --- | --- | --- |
| `tauri-plugin-sql` (crate) | 2.4.1 (2026-08-31) | VERIFIED — crates.io |
| `@tauri-apps/plugin-sql` (npm) | 2.4.1 | VERIFIED — registry.npmjs.org |
| `rusqlite` (crate) | 0.40.2 (2026-08-08) | VERIFIED — crates.io |
| `rusqlite`의 `bundled` feature 이름 | `features = ["bundled"]` | **UNVERIFIED** — 오랜 관례이나 0.40.2 문서에 대해 재확인하지 않았다 |

`tauri-plugin-sql`은 sqlx 기반이며 SQLite·MySQL·PostgreSQL 드라이버를 지원하고
macOS를 지원 플랫폼으로 명시한다 (v2.tauri.app/plugin/sql).

**선택은 Phase 1이 한다.** 다만 §9·§12·INV-7과의 정합성 때문에 다음 방향이 유력하다:

> persistence ownership을 **Rust boundary**에 두고 Tauri command로 노출한다
> (`rusqlite` 계열). `tauri-plugin-sql`은 SQL 실행 주체가 프론트엔드가 되므로,
> frontend가 임의 SQL executor가 되는 것을 막고 domain/repository 경계와
> 향후 secret/privacy 경계를 일관되게 유지하려면 적합하지 않다.

이 방향의 근거:

```text
persistence ownership이 Rust boundary에 남는다
frontend가 arbitrary SQL executor가 되는 것을 방지한다
domain/repository boundary를 유지하기 쉽다
향후 secret/privacy boundary(INV-7)와 일관된다
Rust unit/integration test로 하드웨어·DOM 없이 검증 가능하다
§14.5의 "Ollama를 Rust backend에서 호출" 방향과도 일관된다
```

**단, 실제 crate와 버전은 사용 전에 현재 공식 출처에서 다시 확인하고 evidence를 남긴다.**
확인하지 못하면 UNVERIFIED로 남긴다.

### 14.8 검증 상태 요약

> 이 표는 **본문에 실제 근거가 있는 것만** VERIFIED로 적는다.

| 확정된 것 (VERIFIED · 본문에 근거 있음) | Phase에서 확인해야 하는 것 (UNVERIFIED) |
| --- | --- |
| Tauri v2 버전 · scaffold · MSRV · macOS는 Xcode CLT로 충분 (§14.2) | WKWebView MediaRecorder 실제 코덱 / pause·resume (P2) |
| Windows 요구사항 · WebView2 기본 설치 · msi는 Windows에서만 빌드 (§14.2) | WebView2와 WKWebView의 녹음 출력 포맷 일치 여부 (P2/P6) |
| Tauri cross-platform 경로 API (`PathResolver` · `appDataDir`) (§14.2) | 번들 `.app`에서 마이크 TCC 프롬프트 정상 동작 (P2) |
| cpal 백엔드 — macOS CoreAudio / Windows WASAPI · MSRV 1.85 (§14.3) | 1시간 녹음 안정성 · crash 내성 (P2) |
| macOS는 Info.plist 병합 · Windows Win32는 manifest 불필요 (§14.3) | Windows 마이크 privacy 토글 차단 시의 오류 형태 (P6) |
| whisper.cpp v1.9.3 · cmake 필요 · `whisper-cli` · 16-bit WAV · JSON 스키마 (§14.4) | macOS cmake 부재 해결 · sidecar 코드서명/notarization #11992 (P3) |
| **Windows prebuilt은 있고 macOS prebuilt은 없다** (§14.4) | 한국어+영어 혼용에 적합한 whisper 모델 (P3) |
| sidecar target triple 접미사 (macOS / Windows `.exe`) (§14.4) | 어떤 로컬 LLM이 이 transcript에 실제로 쓸 만한가 (P4) |
| Ollama 엔드포인트 · `format` JSON Schema · 헬스체크 (§14.5) | `kanana` 모델 크기 태그 (P4) |
| **Ollama CORS 기본 정책 · 기본 num_ctx 4096** (§14.5) | Gemini · Groq 관련 사실 일체 (DEFERRED) |
| Notion 버전 헤더 · SDK · 100블록/2000자 한도 (§14.9) | Notion `markdown` 직접 전송 지원 여부 (P5) |
| persistence 후보 crate 버전 (§14.7) | `rusqlite`의 `bundled` feature 이름 (P1) |

### 14.9 Notion API (확인된 사실 · 사용은 Phase 5)

| 항목 | 확인된 사실 |
| --- | --- |
| `Notion-Version` 헤더 | **`2026-03-11`** (developers.notion.com/reference/versioning) |
| 공식 JS SDK | `@notionhq/client` **5.26.0** (2026-08-20) |
| 인증 | internal integration token을 `Authorization: Bearer <secret>`로 전송 (+ `Notion-Version`, `Content-Type`) |
| 페이지 생성 | `POST /v1/pages`. `parent`는 `page_id` · `database_id` · `data_source_id` · `workspace` |
| 부모 페이지 밑 생성 | **page ID만 있으면 된다** (`parent: {page_id: ...}`) |
| ⚠️ 한도 | `children` 블록 **요청당 100개** · `rich_text` 배열 **100개** · 단일 `text.content` **2000자** |
| 100개 초과 | `PATCH /v1/blocks/{block_id}/children`를 **순차 반복**. append에는 배치/페이지네이션이 없다 |

**1시간 transcript는 이 한도에 반드시 걸린다.** Phase 5는 2000자 청킹과
100블록 배치 전송을 설계해야 한다.

**Markdown 직접 전송 (UNVERIFIED — 반드시 재확인)**: Notion이 블록 JSON 대신
`markdown` 문자열 필드를 받는다는 정황이 있으나 해당 가이드 페이지 직접 fetch가 실패했다.
사실이라면 §11의 Markdown export를 재사용할 수 있어 Phase 5가 크게 단순해진다.
**Phase 5는 이것을 가장 먼저 직접 확인하고**, 확인되지 않으면 블록 JSON을 직접 만든다.
확인 전까지 이 사실에 의존하는 설계를 확정하지 않는다.

---

## 15. Non-Goals

현재 프로젝트에서 **만들지 않는다.** 실제 사용 중 필요성이 확인되기 전까지 추가하지 않는다.

```text
화면 녹화              카메라 녹화            화상회의 기능
실시간 협업            사용자 계정            서버
Cloud DB               Cloud audio storage    모바일 앱
Linux 지원             실시간 AI assistant    실시간 transcription
화자 분리              NotebookLM 자동 API 연동
복잡한 RAG 시스템      Agent 기반 자동 업무 수행
Ollama 등 AI 런타임의 installer 번들링
```

이것들은 "나중에 할 일"이 아니라 **현재 제품 정의에서 제외된 것**이다.
Phase Goal이나 Task가 여기에 손대려 하면 그것은 scope 위반이다.

> rev 1에서 non-goal이었던 **Windows 지원은 §3에 따라 지원 대상으로 이동했다.**
> Linux와 모바일은 그대로 non-goal이다.

---

## 16. Deferred (DEFERRED — V1 성공 조건 아님)

실제 사용 후 필요성이 확인되면 검토한다. 근거 없이 앞당기지 않는다.

```text
Optional Cloud AI Providers (Claude · Gemini · Groq)
search · tags · processing queue · keyboard shortcuts · menu bar
better recording recovery · AI prompt customization
improved Notion sync · better export · UX polish
```

**Cloud Provider를 DEFERRED로 둔 근거**는 §21에 있다.
Architecture extensibility(provider 추상화가 존재하는 것)와
V1 feature count(provider를 몇 개 구현하는가)는 다른 문제다.

---

## 17. V1 Success Criteria (PoC 성공 판정)

### 17.1 Core (AI 없이 성립해야 한다 — INV-8)

```text
Molt Note 실행 → 제목 입력 → Record → 긴 녹음 → Stop
  → 앱에 녹음 저장
  → Local Whisper transcription
  → Transcript 검토
  → Markdown export
  → 나중에 다시 열고 원본 음성 / Transcript 확인
```

**AI Provider가 하나도 설정되지 않은 상태에서 이 흐름 전체가 동작하면 core는 성공이다.**

### 17.2 Full (AI 포함)

```text
위 흐름 + Local Ollama로 Study Note 생성 → 내용 확인 → Notion으로 전송
```

### 17.3 판정 시 특히 확인하는 것

1. 앱을 종료하고 재실행해도 모든 녹음과 산출물이 그대로 있다.
2. 후처리 중 하나가 실패해도 원본 audio와 Transcript는 남아 있고, 실패가 UI에 보이며 재시도된다.
3. 사용자가 요청하지 않은 외부 전송이 일어나지 않는다.
4. **AI Provider를 제거하거나 중지해도 §17.1이 그대로 동작한다.**
5. Windows에서 §3.1의 핵심 기능이 동작한다 (Phase 6에서 검증).

---

## 18. Validation Philosophy

완료 주장은 **실제로 실행한 명령의 결과**에 근거한다. Worker의 서술은 근거가 아니다.

| | 무엇을 보장하는가 | 수단 |
| --- | --- | --- |
| **Automated** | 도메인 로직 · 상태 기계 · 파싱 · 직렬화 · 영속성 · 실패 상태 처리 · provider 계약 준수 | build / lint / test Gate + Verifier |
| **Human witness** | 실제 마이크 입력 품질 · 실제 재생 음질 · 실제 Notion 페이지 결과 · AI Note의 유용성 · UI 체감 · Windows 실동작 | 사람이 직접 확인 |

**하드웨어 · 네트워크 · 플랫폼 · 외부 프로세스 경계와 core logic을 분리하여 설계한다.**
실제 microphone capture나 실제 Ollama 호출처럼 자동 검증이 불가능한 영역은,
그 주변의 상태 기계 · 파일 처리 · 파싱 · 메타데이터 로직이 그것들 없이
테스트 가능하도록 경계를 긋는다.

자동으로 판정할 수 없는 것을 자동 PASS로 적지 않는다.

### 최소 자동 검증 대상

```text
Recording state machine       duration handling         persistence
recording metadata            transcript parsing        timestamp handling
structured note schema        provider contract         Markdown export
Notion payload creation       settings                  failure state handling
AI-부재 시 core pipeline 동작 (INV-8)
```

---

## 19. UI Direction

Molt Note는 개인 생산성 도구다. 과도한 dashboard UI를 만들지 않는다.

```text
minimal · macOS-like · calm · dense enough for productivity
recording state immediately visible · typography first
불필요한 card UI 최소화
```

Recording 화면에서 가장 중요한 것은 **녹음 상태와 경과 시간**이다.
AI 장식보다 recording reliability를 우선한다.

Windows에서도 같은 디자인 방향을 유지하되, 플랫폼별 UI 분기를 미리 만들지 않는다.

---

## 20. Development Principles

1. **증거 우선** — 실행하지 않은 명령을 PASS로 적지 않는다.
2. **추측 금지** — 외부 라이브러리/API 능력을 상상하지 않는다. 확인하거나 UNVERIFIED로 남긴다.
3. **원본 보존이 기능보다 우선** — 충돌하면 INV-1~INV-4가 이긴다.
4. **점진적 Phase** — 한 Phase에서 제품 전체를 만들지 않는다.
5. **미래 의존성 선입금 금지** — 지금 필요하지 않은 라이브러리를 미리 설치하지 않는다.
6. **추상화도 선입금하지 않는다** — 실제 두 번째 구현이나 실제 플랫폼 차이가 있을 때 추상화한다.
7. **결정은 기록한다** — 되돌리기 어려운 아키텍처 선택은 근거와 탈락 후보를 함께 남긴다.

---

## 21. Phase Roadmap

각 Phase의 Goal 본문은 `phase-prompt/` 아래에 있다.

| Phase | 파일 | 한 줄 목적 | 상태 |
| --- | --- | --- | --- |
| Bootstrap | (`prompts/PROJECT-BOOTSTRAP.md`) | 실행 가능한 개발 baseline과 실제 Gate 확보 | **DONE** (2026-09-01) |
| 1 | `01-application-foundation.md` | 앱 셸 · 로컬 저장소 · 데이터 영속성 · 플랫폼 경계 · 마이크 권한 선언 | PLANNED |
| 2 | `02-reliable-recording.md` | 실제 녹음 → 파일 → 재생, 재시작 후에도 살아남는다 | PLANNED |
| 3 | `03-local-transcription.md` | 로컬 whisper로 timestamped transcript 생성 | PLANNED |
| 4 | `04-ai-provider-system.md` | **Provider 추상화 + Local AI(Ollama) → Structured Note** | PLANNED |
| 5 | `05-notion-and-export.md` | Notion 전송 · Markdown export | PLANNED |
| 6 | `06-cross-platform-validation.md` | **Windows에서 핵심 기능 검증 및 hardening** | PLANNED |
| Final | `Goal.md` | 통합 · 정합성 정리 · V1 성공 기준 검증 | PLANNED |
| — | (DEFERRED) | Optional Cloud Providers (Claude · Gemini · Groq) | **DEFERRED** |

### 순서의 근거

- Phase 1은 **가장 작은 유용한 기반**이다. 녹음 파이프라인 전체를 욕심내지 않는다.
- Phase 2에서 **처음으로 진짜 end-to-end 능력**(녹음→파일→재생)이 증명된다.
  이것이 제품의 핵심이며 AI보다 먼저 온다.
- Phase 3~5는 각각 이전 Phase가 증명한 산출물 위에서만 동작한다.
  Transcript 없이 AI Note를 만들 수 없고, structured note 없이 Notion 렌더러를 만들 수 없다.
- Phase 6은 Windows가 **지원 대상 플랫폼**(§3)이므로 V1 범위 안에 있다.
  단 플랫폼 검증은 검증할 기능이 존재한 뒤에만 의미가 있으므로 마지막에 온다.
- `Goal.md`는 새 기능 영역이 아니라 **통합과 검증**이다.

### Cloud Provider를 Phase가 아니라 DEFERRED로 둔 근거

§17.1이 V1 성공 조건을 **AI 없이 성립하는 core**로 정의한다. Cloud provider는 그 조건에
포함되지 않으므로 V1 최소 범위가 아니다.

다만 **구현이 하나뿐인 추상화는 검증된 추상화가 아니다.** 그래서 Phase 4는
Ollama adapter 하나만 만드는 대신, **결정론적 test double(fake provider)** 로 같은 계약을
통과시켜 경계를 검증한다. 이렇게 하면 두 번째 벤더를 V1에 넣지 않고도
추상화가 실제로 벤더 중립인지 확인할 수 있다.

Cloud provider가 실제로 필요해지면 그때 Phase로 승격한다.

### 검토 대상으로 남겨둔 순서 문제

§17.1(AI 없이 완결되는 core)이 Markdown Export까지 포함하는데, 현재 Map에서
Markdown Export는 Phase 5에 있고 AI는 Phase 4에 있다. 즉 **AI-free core pipeline이
완성되는 시점이 AI 도입보다 나중이다.**

Markdown Export를 Phase 4 앞으로 옮기면 §17.1을 한 Phase 먼저 증명할 수 있다.
지금은 권장 Map을 그대로 따랐으며, 이 순서 변경은 Phase 4를 계획하기 전이라면
비용 없이 가능하다. **운영자 판단 사항으로 남긴다.**
