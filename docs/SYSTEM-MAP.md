# System Map — Molt Note

이 문서는 프로젝트의 **최상위 지도**다. 상세 구현 문서가 아니라 진입점이다.
상세는 §8의 문서로 넘긴다.

> **현재 상태: Phase 5 완료 (2026-09-04).** 앱 셸 · 로컬 영속성 · §7 데이터 모델 ·
> 네 화면 navigation에 더해, **녹음 lifecycle(Record/Pause/Resume/Stop) · 파일 확정 ·
> Recording 영속화 · 재생**이 구현되고 자동 검증을 통과했다.
>
> 여기에 **로컬 전사**(whisper-rs in-process · 파생 입력 · timestamp 정규화 · Transcript
> 버전 관리)가 더해졌고, 그 위에 **vendor 중립 AI Note 경계**(`NoteAiProvider` 계약 ·
> 세 mode의 structured note · 로컬 Ollama adapter · AI Note 탭 · Settings provider 구역)가
> 더해졌다.
>
> 여기에 **나가는 문**이 더해졌다 — `export::Document` 하나에서 갈라지는 **Markdown 파일
> export**와 **Notion sync**(Markdown Content API · 무손실 분할 · SecretStore).
>
> ⚠️ **그러나 실제 하드웨어/추론/추론서버/워크스페이스에서 확인된 적이 없다.**
> 미확정 전제가 **넷**이다 — `A-REC-001`(실제 마이크 미검증) ·
> `A-TRANS-001`(실제 Whisper 추론 미실행) · `A-AI-001`(실제 Ollama 미호출) ·
> `A-NOTION-001`(**실제 Notion 워크스페이스로 요청이 나간 적이 없다**).
> 넷 다 Final Integration의 hard human gate로 연기됐다.
>
> 갱신 이력:
> - 2026-09-01 Bootstrap — 개발 baseline과 Gate 확보
> - 2026-09-01 Requirements Delta — Windows 지원 대상 추가 · AI를 vendor 중립 Provider로 전환
> - 2026-09-01 Product Spec rev 3 — Transcript cardinality를 `Recording 1:N Transcript`로 확정
> - **2026-09-02 Phase 1 DONE** — §1 ~ §9를 실제 구현 기준으로 갱신
> - **2026-09-04 Phase 4 DONE** — §1 · §3 · §4 · §5 · §6 · §7 · §8 · §9에 AI Provider 경계 반영
> - **2026-09-04 Phase 5 DONE** — Markdown export · Notion sync · SecretStore 경계 반영

---

## 상태 표기 규칙 (필수)

| 표기 | 뜻 |
| --- | --- |
| **DONE** | 저장소에 실제 구현이 있고, 요구된 검증을 통과했다 |
| **PLANNED** | 계획되었지만 아직 구현이 없다 |
| **DEFERRED** | 의도적으로 후속 scope로 미뤘다 |
| **CANDIDATE** | 검토 후보이며 선택되지도 구현되지도 않았다 |

**의존성이 설치되어 있다는 것은 기능이 구현됐다는 뜻이 아니다.**
패키지가 `package.json`이나 `Cargo.toml`에 있다는 것, 빌드가 통과했다는 것은 DONE의 근거가 아니다.
제품 경로에 통합되어 검증을 통과했을 때만 DONE이다.

---

## 1. What This System Is

Molt Note는 **개인용 Local-first Knowledge Recorder**다.
회의·스터디 음성을 녹음하고, 로컬에서 전사하고, 필요하면 AI로 읽기 좋은 기록으로 정리하고,
필요하면 Notion이나 Markdown으로 내보낸다.

플랫폼: **macOS(Apple Silicon)가 primary 개발 플랫폼, Windows(x64)가 지원 대상 플랫폼.**
Linux와 모바일은 범위 밖이다 (`PRODUCT-SPEC.md` §3).

책임 경계:

- **책임진다** — 녹음 원본의 보존, 로컬 전사, 원본과 AI 산출물의 분리, 외부 전송의 명시적 통제,
  그리고 **AI 없이도 완결되는 core pipeline**(INV-8).
- **책임지지 않는다** — 서버, 계정, 클라우드 저장, 실시간 처리, 화자 분리, 협업,
  AI 런타임의 번들링. 전체 목록은 `docs/PRODUCT-SPEC.md` §15.

| | |
| --- | --- |
| **지금 동작한다 (DONE · 자동 검증 기준)** | 앱이 실행되고 네 화면 사이를 이동한다. 로컬 SQLite에 §7 스키마가 있고 Recording 목록·Settings가 재시작 후에도 유지된다. **backend가 소유하는 녹음 session으로 Record/Pause/Resume/Stop이 동작하고, Stop은 파일 확정 후 Recording을 영속화하며, Detail에서 재생한다.** 실패가 화면에 표시되고 재시도된다. 그 위에서 **Transcript → structured note(Meeting · Study · Summary) → AI Note 탭**이 돌고, Settings에서 provider · 주소 · 모델을 고르고 연결을 확인한다 |
| **⚠️ 자동 검증됐으나 장치 미확인** | 위 녹음 경로 전체. 실제 마이크·권한 프롬프트·음질로 확인된 적이 **없다** (`ASSUMPTION A-REC-001`) |
| **⚠️ 구현됐으나 실제 추론 미실행** | 전사 경로 전체. `whisper-rs`가 링크돼 있고 자동 검증을 지나지만 **실제 모델로 추론한 적이 없다** (`A-TRANS-001`) |
| **⚠️ 구현됐으나 실제 호출 미실행** | AI Note 경로 전체. 계약 · adapter · 화면이 자동 검증을 지나지만 **실제 Ollama에 요청을 보낸 적이 한 번도 없다** (`A-AI-001` · `docs/PHASE-4-AI-NOTE-REVIEW.md` §10.2) |
| **⚠️ 구현됐으나 실제 전송 미실행** | Notion sync 경로 전체. adapter · 분할 · SecretStore · 화면이 자동 검증을 지나지만 **실제 Notion 워크스페이스로 요청을 보낸 적이 없다** (`A-NOTION-001` · `docs/PHASE-5-NOTION-SMOKE-TEST.md` §10.1) |
| **다음 단계 (PLANNED)** | Phase 6 — Cross-platform Validation & Hardening (Windows) |
| **미룬 것 (DEFERRED)** | **Cloud AI Providers (Claude · Gemini · Groq)** · search · tags · processing queue · menu bar (`PRODUCT-SPEC.md` §16) |
| **후보 (CANDIDATE)** | recording engine (§4) · whisper 통합 방식 (§4) · persistence crate (§4) |

---

## 2. Current System Flow

**현재 구현된 흐름만** 그린다.

```text
앱 시작
  ↓
AppDataDirectory 결정 → SQLite 열기 → migration 적용 (미적용분만)
  ↓  실패하면 panic하지 않고 Failure로 보존된다
React 셸 (sidebar + 4 화면)
  ↓
Recording 화면
  ↓
입력 장치 열거 → 마이크 선택 (Settings의 default microphone 반영)
  ↓
Record → Pause → Resume → Stop      ← 상태 기계는 순수 모듈, session은 backend 소유
  ↓
Stop = 캡처 정지 → writer 확정 → 파일 존재·크기 확인 → Recording 영속화
  ↓  DB 실패 시 **audio를 지우지 않고** 경로를 담은 실패를 돌려준다 (INV-3 · INV-4)
Recordings 목록 → Recording Detail → 재생
                                   ↓ 파일이 없으면 레코드를 지우지 않고 알린다
  ↓
전사 (whisper-rs in-process · 배경 스레드)      ← 원본 오디오는 읽기만 한다
  ↓  성공하면 Transcript를 **덧붙이고** current를 옮긴다. 실패하면 이전 current를 유지한다
Detail의 Transcript 탭
  ↓
AI Note 탭 → mode 선택(Meeting · Study · Summary) → 생성
  ↓
ai::run → provider 선택 → Transcript **텍스트만** 전달 → structured note
  ↓  provider를 고르지 않았다면: 오류가 아니라 "AI 기능이 아직 켜지지 않았다" (INV-8)
  ↓  응답이 schema와 어긋나면: 재시도 가능한 실패. Transcript와 Recording은 그대로다
AINote를 **새 레코드로 append** (provider · model · promptVersion · transcriptId · generatedAt)
  ↓
AI Note 탭이 이력을 보여준다. 재생성은 대체가 아니라 추가다
  ↓
Export Markdown ──→ export::Document ──→ Markdown renderer ──┬─→ 로컬 .md 파일
                                                              └─→ Notion Markdown Content API
  ↓  AI Note가 없어도 Transcript와 메타데이터만으로 유효한 문서가 나온다 (INV-8)
Send to Notion (명시적 opt-in · INV-5)
  ↓  긴 문서는 안전한 경계에서 나뉘어 순서대로 전송되고, 조용히 잘리지 않는다
  ↓  429/529는 Retry-After를 존중한다. 부분 실패 뒤 재시도는 같은 페이지에서 이어간다
NotionSync (recordingId · pageId · syncedAt · status · error) — Recording 1개 ↔ 페이지 1개
```

**아직 없는 것(PLANNED):** Windows 실동작 검증 (Phase 6).

⚠️ 위 흐름 중 **전사 · AI 생성 · Notion 전송은 실제로 실행된 적이 없다** —
`whisper` 추론(`A-TRANS-001`) · Ollama 호출(`A-AI-001`) · Notion 요청(`A-NOTION-001`)
모두 자동 검증에서는 fixture와 stub으로만 지나간다.
**Markdown 파일 export는 실제 파일 쓰기까지 자동 검증이 지나간다** — 임시 디렉터리에서
실물 파일을 만들고 내용을 확인한다. 이 경로에는 외부 의존이 없다.

전체 목표 흐름은 `docs/PRODUCT-SPEC.md` §4에 있다.

---

## 3. Major Components

| Component | 역할 | 상태 |
| --- | --- | --- |
| Tauri v2 앱 셸 (`src-tauri/`) | macOS 데스크톱 런타임 · Rust backend 진입점 | **DONE** |
| `platform/app_data_dir.rs` | 플랫폼별 앱 데이터 경로 결정을 한 곳에 가둔다 (INV-10). 테스트는 임시 경로를 주입한다 | **DONE** |
| `db/` (migrations · store · settings) | SQLite 열기 · 버전 기반 migration · 영속성. 사용자 데이터를 지우는 경로가 없다 | **DONE** |
| `domain/` (duration · failure · settings) | 순수 도메인 로직. 하드웨어·DB 없이 테스트된다 | **DONE** |
| `commands/` (Tauri command 경계) | `list_recordings` · `get_recording` · `create_recording` · `delete_recording` · `get_settings` · `update_settings`. **임의 SQL을 받는 command는 없다** | **DONE** |
| `src/navigation/` · `src/screens/` | 4화면 라우팅 · 화면별 view 로직(순수 모듈, DOM 없이 테스트) | **DONE** |
| `domain/session` (상태 기계) | idle→recording→paused→recording→stopped · 잘못된 전이 거부 · pause 제외 duration. 하드웨어 없이 테스트된다 | **DONE** |
| `audio/` (capture · devices · finalized) | 장치 열거 · cpal 캡처 · hound WAV writer · 파일 확정 | **DONE** (⚠️ 장치 미검증) |
| backend 소유 recording session | Tauri managed state가 session을 소유한다. 화면 컴포넌트가 소유하지 않는다 (R-001) | **DONE** |
| `platform/microphone` | 권한 상태 경계. 신뢰할 판정 수단이 없는 구간은 UNVERIFIED로 남긴다 | **DONE** (한계 명시됨) |
| `src/ipc/` | 타입 있는 command client · 실패 매핑 | **DONE** |
| `src-tauri/Info.plist` | macOS 마이크 권한 **선언** (packaging 요구사항이지 권한 구현이 아니다) | **DONE** |

| Transcription 엔진 통합 (`transcription/`) | `whisper-rs` in-process 실행 · 파생 입력 · timestamp 정규화 · Transcript 버전 관리 | **DONE** (⚠️ 실제 추론 미실행 · `A-TRANS-001`) |
| **`ai/provider.rs` (`NoteAiProvider` 계약)** | **vendor 중립 AI 계약** — `descriptor` · `availability` · `generate_note`. domain은 벤더를 모른다 (INV-9). 요청 타입에 오디오를 담을 자리가 없다 (INV-6) | **DONE** (⚠️ 실제 호출 미실행) |
| **`ai/note.rs` · `ai/prompt.rs`** | 세 mode의 structured note schema · 방어적 파싱 · 프롬프트에 묶인 `promptVersion` | **DONE** |
| **`ai/run.rs`** | AI Note orchestration — Transcript를 **읽기만** 하고 AINote를 append한다 (INV-2 · INV-3) | **DONE** |
| **`ai/ollama/`** | 로컬 Ollama adapter. 엔드포인트 · 요청 필드 · 상태 코드 해석이 **이 디렉터리 안에만** 있다 (INV-9) | **DONE** (⚠️ 실제 호출 미실행 · `A-AI-001`) |
| **`ai/testing.rs` (fake provider)** | **테스트 전용 test double.** adapter와 **같은** 계약 묶음을 통과해 추상화를 검증한다. 제품 UI 선택지가 아니다 | **DONE** |
| **`commands/notes.rs`** | AI Note command 경계 — provider 상태 · 생성 시작 · 진행 조회 · 노트 열람. 전사와 같은 배경 스레드 방식 | **DONE** |
| **Detail의 AI Note 탭 · Settings의 provider 구역** | mode 선택 · 생성 · 재생성 · 이력 · 연결 확인 · 모델 선택 · **로컬/외부 표시** (INV-5) | **DONE** (⚠️ 실행 화면 미확인) |
| **`export/` (markdown · filename · file · run)** | `export::Document` → Markdown 문자열(순수) → 실제 파일. 파일명 정규화는 슬래시 · 콜론 · 이모지 · 개행 · 경로 탈출을 막고, 기존 파일을 조용히 덮어쓰지 않는다 | **DONE** |
| **`notion/` (wire · client · http · network · chunk)** | Notion을 아는 유일한 자리. `markdown` body param · `insert_content`/`position:end` · `GET /v1/users/me` · `Notion-Version: 2026-03-11`. 무손실 분할이 순수 모듈로 분리돼 있다 | **DONE** (⚠️ 실제 전송 미실행 · `A-NOTION-001`) |
| **`sync/` (run · pace)** | Notion 전송 실행 순서와 `NotionSync` 영속화. `429`/`529`의 `Retry-After`를 정수 초로 존중하고 요청 간 최소 간격을 둔다 | **DONE** |
| **`platform/secret_store.rs`** | **SecretStore 경계 하나** — 닫힌 `SecretKey` · 재현되지 않는 `Secret` 타입. macOS 구현 · Windows 구현 자리 · 메모리 test double. token은 SQLite에도 frontend에도 없다 (INV-7 · INV-10) | **DONE** (⚠️ Windows 검증은 Phase 6) |
| **`commands/notion.rs`** | `start` · `status` · `check_connection` · `save_token` · `delete_token`. **저장된 token을 돌려주는 command가 없다** | **DONE** |
| **Detail의 Export/Send · 목록의 sync 상태 · Settings의 Notion 구역** | Markdown export · Send to Notion · sync 상태 표시 · connection test · destination 설정 | **DONE** (⚠️ 실행 화면 미확인) |
| Windows 지원 검증 | §3.1 핵심 기능의 Windows 실동작 | **PLANNED** (Phase 6) |

---

## 4. External Dependency Boundary

| 구분 | 항목 | 비고 |
| --- | --- | --- |
| **선택됨 · 현재 사용 중** | Tauri v2 (2.11.5) · React 19 · Vite 7 · TypeScript 5.8 · ESLint · Vitest<br>**`rusqlite` 0.40.2 (`bundled`, SQLite 3.53.2)** — 제품 경로에서 실제로 쓰인다 | persistence 선택 근거는 `docs/ADR-0001-local-persistence.md` |
| **선택됨 · 추론 미실행** | **`whisper-rs` 0.16 + `rubato` 5** — 제품 전사 경로에서 쓰인다 | ⚠️ 실제 모델로 추론한 적이 없다 (`A-TRANS-001`) |
| **선택됨 · 실제 호출 미실행** | **`ureq` 3.4.0 (`default-features = false, features = ["rustls"]`)** — 로컬 Ollama REST(`ai/ollama/network.rs`)와 Notion HTTPS(`notion/network.rs`)를 부르는 자리에서 쓰인다 | ⚠️ 실제 서버에 요청을 보낸 적이 없다 (`A-AI-001` · `A-NOTION-001`). **Phase 5가 `rustls`를 명시적으로 켰다** — ADR-0008 §12.2가 예고한 대로 HTTPS가 실제로 필요해진 시점이다 |
| **선택됨 · 실제 저장소 미접근** | **`keyring` 3.6.3 (`apple-native` + `windows-native`)** — Notion integration token을 담는 유일한 자리(`platform/secret_store.rs`) | ⚠️ 자동 테스트는 메모리 double만 쓰며 실제 OS 자격증명 저장소를 건드리지 않는다. feature 전체 목록은 **UNVERIFIED** — 확인된 것은 두 feature 이름이 실재하고 플랫폼 API가 들어왔다는 것까지다 (`ADR-0009` §15.2.4) |
| **선택됨 · 사용 중** | **`sha2` 0.10** — export 경로에서 쓰인다 | `Cargo.lock`이 고정한다 |
| **잠정 선택 · 장치 미검증** | **`cpal` 0.18.2 + `hound` 3.5.1** — **제품 녹음 경로에서 쓰인다** | ⚠️ `ADR-0003`은 **PROVISIONAL**이다. 실제 마이크에서 확인된 적이 없다 (`ASSUMPTION A-REC-001`) |
| **설치됨 · 미통합** | (없음) | scaffold의 `tauri-plugin-opener`는 Phase 1에서 **제거**했다 |
| **후보 · 미선택** | recording: `cpal`+`hound` / webview MediaRecorder / 커뮤니티 플러그인<br>transcription: whisper.cpp sidecar / `whisper-rs`<br>Notion: `@notionhq/client` | **설치되지 않았다.** 각각 이를 필요로 하는 Phase에서 검증과 함께 선택한다. 확인된 사실은 `PRODUCT-SPEC.md` §14 |
| **탈락** | `tauri-plugin-sql` | frontend가 임의 SQL executor가 된다. INV-7 · repository 경계와 어긋난다 (ADR-0001) |
| **미룸** | Cloud AI Providers (Claude · Gemini · Groq) | V1 성공 조건(§17.1)이 AI 없이 성립하도록 정의됐다. 추상화는 Phase 4에 있고 구현은 필요해질 때 (`PRODUCT-SPEC.md` §16) |
| **미룸** | Claude Agent SDK | 단일 요청/응답 변환에 agentic loop가 필요한 이유가 없다 (`PRODUCT-SPEC.md` §9.4) |
| **미룸** | Ollama 자체의 번들링 | 사용자가 실행 중인 인스턴스에 연결만 한다 (`PRODUCT-SPEC.md` §9.4 · §15) |
| **탈락** | `@notionhq/client` (공식 JS SDK) | 호출 주체가 Rust backend다 (ADR-0008 §5와 일관). frontend가 token을 만지지 않는 것이 INV-7의 실현 방법이다 |
| **탈락** | Notion 블록 JSON 직접 조립 | Markdown Content API가 VERIFIED가 되어 §11의 export 산출물을 그대로 보낸다 (`PRODUCT-SPEC` §14.9.1) |

외부 전송 경계는 `docs/PRODUCT-SPEC.md` §12가 정본이다.
**Phase 5에서 처음으로 인터넷으로 나가는 경로가 생겼다** — Notion sync다.
그것은 **명시적 opt-in**이며 (사용자가 token을 저장하고 destination을 고르고 Send를 눌러야
한다), 나가는 것이 무엇인지 화면에 드러난다 (INV-5). Phase 4가 만든 Ollama 경로는 여전히
사용자의 로컬 주소(기본 `http://localhost:11434`)를 향한다.

경계는 세 단계로 나뉜다 — **완전 로컬**(녹음·전사) · **로컬 AI**(Ollama) ·
**외부**(Cloud provider · Notion). 로컬 AI는 인터넷으로 나가지 않으므로 외부가 아니다.
사용자가 그 주소를 원격 호스트로 바꾸면 로컬이 아니게 되며, 그래서 provider의
**로컬/외부 구분이 화면까지 도달한다** (INV-5 · §12).

**오디오는 어느 경로로도 나가지 않는다** — AI provider 요청 타입에도 Notion 요청 타입에도
오디오를 담을 자리가 아예 없다는 것이 소스 수준에서 강제된다 (INV-6 ·
`audio_never_reaches_ai.rs` · `notion_and_export_invariants.rs`의
`an_audio_file_that_really_exists_reaches_neither_the_file_nor_the_request` ·
`nothing_in_the_notion_boundary_can_open_or_read_a_file`).

**Notion integration token은 저장소에 없다** — `platform/secret_store.rs`의 경계를 통해 OS
자격증명 저장소에만 담기고, SQLite에는 secret이 아닌 `notion_parent_page_id`만 있다 (INV-7).

---

## 5. Build Evolution Map

### Bootstrap — 2026-09-01 · **DONE**

무엇이 생겼는가:

- Loop Runtime control plane(`.loop/`, `.loop-local/`, `.gitignore`) 복원 —
  저장소 복사 과정에서 누락되어 있었다 (`LOOP-RUNTIME-FIELD-NOTES.md` 참조)
- Tauri v2 + React + TypeScript + Vite scaffold
- ESLint · Vitest 설정
- build / lint / test Gate 3개 활성화 — 세 command 모두 frontend와 Rust를 함께 검사한다
- `docs/PRODUCT-SPEC.md`, `phase-prompt/01~05` + `Goal.md`

무엇이 검증됐는가:

- `npm run build` · `npm run lint` · `npm run test` 각각 exit 0
- `./loopctl self-check` 3개 Gate 전부 PASS
- `./loopctl doctor` exit 0
- Loop Runtime 회귀 149 pass / 0 fail

**제품 기능은 구현되지 않았다.**

### Phase 1 — Application Foundation · 2026-09-02 · **DONE**

무엇이 생겼는가 (Task 9개, 전부 Gate PASS + 독립 Verifier PASS):

| Task | 결과물 |
| --- | --- |
| TASK-001 | `AppDataDirectory` 경계 — OS별 경로 지식을 한 모듈에 가둠 |
| TASK-002 | SQLite 초기화 · 버전 기반 migration · `ADR-0001` |
| TASK-003 | §7 스키마 — `recordings` · `transcripts` · `transcript_segments` · `ai_notes` · `notion_syncs` |
| TASK-004 | Recording repository · duration 포맷 |
| TASK-005 | Settings 영속화 (secret 필드 없음) |
| TASK-006 | Tauri command 경계 · 타입 있는 frontend client · 실패 계약 |
| TASK-007 | 4화면 navigation shell |
| TASK-008 | Recordings 실데이터 렌더 · 실패의 UI 표현 |
| TASK-009 | macOS `NSMicrophoneUsageDescription` 선언 · `ADR-0002` |

무엇이 검증됐는가:

- build · lint · test Gate 전부 PASS (세 명령 모두 TypeScript와 Rust를 함께 검사)
- 자동 테스트 **131개** — vitest 70 · Rust 61
- `Recording 1:N Transcript 1:N AINote` 카디널리티가 스키마와 테스트로 표현됨 (§7.1)
- 재시작 생존: DB를 닫았다 같은 경로로 다시 열어 데이터가 남는 것을 테스트로 확인
- 번들된 `.app`의 `Info.plist`에 `NSMicrophoneUsageDescription`이 **실제로 병합**됨
  (`npm run tauri build`로 생성 후 `PlistBuddy`로 확인 — Gate가 판정하지 못하는 항목)

무엇이 **구현되지 않았는가** (의도적):

```text
마이크 캡처 · MediaRecorder · cpal · 오디오 파일 · 재생
whisper · 전사 · 재전사 workflow · transcript 버전 선택 UI
AI Provider · Ollama · Claude · Gemini · Groq
Notion · Markdown export
Windows 빌드 · Windows 검증 · secret 저장소
```

실행 중 관찰된 Runtime 이슈는 `docs/LOOP-RUNTIME-FIELD-NOTES.md` OBS-015 ~ OBS-019.

### Requirements Delta — 2026-09-01 · **문서만 변경 · 구현 변화 없음**

Human Review 중 제품 요구사항이 바뀌어 Product Spec을 rev 2로 갱신했다.

- Windows를 **지원 대상 플랫폼**으로 추가 (rev 1에서는 non-goal)
- AI를 **vendor 중립 Provider 추상화**로 전환하고 **core requirement에서 제외** (INV-8 · INV-9)
- 첫 AI provider를 **로컬 Ollama**로, Cloud provider를 **DEFERRED**로
- 불변 규칙 INV-8 · INV-9 · INV-10 추가
- Phase Map에 Phase 6(Cross-platform Validation) 추가
- rev 1 §14.7의 근거 없는 VERIFIED 표기를 정정하고 §14.7(Persistence)을 신설

**구현은 하나도 바뀌지 않았다.** 여전히 Bootstrap만 DONE이다.

### Phase 2A — Recording Engine Validation · 2026-09-02 · **engineering DONE · 장치 검증 DEFERRED**

무엇이 생겼는가 (Task 5개, 전부 Gate PASS + 독립 Verifier PASS):

| Task | 결과물 |
| --- | --- |
| TASK-010 | `ADR-0003` — 후보 비교와 **잠정** 선택 (`cpal` + `hound`) |
| TASK-011 | 입력 장치 열거 경계 · 하드웨어 없는 정규화 로직 |
| TASK-012 | 캡처 → 정지 → 파일 확정과 결과 보고 |
| TASK-013 | 임시 spike 표면 (Recording 화면 안, 임시 표시) |
| TASK-014 | 번들 `.app` 실행 절차 문서화 |

**IMPLEMENTED로 기록할 수 있는 것:**

```text
cpal/hound 잠정 capture spike
입력 장치 열거 경계
짧은 캡처 / 파일 확정 구현
임시 spike 표면
```

**⚠️ IMPLEMENTED / VERIFIED로 기록하지 않는 것 — 실제 장치에서 확인된 적이 없다:**

```text
실제 마이크가 이 Mac에서 동작한다
권한 프롬프트가 실제로 뜬다
오디오 품질
장시간 녹음 안정성
production recording lifecycle 완성
```

운영자가 2026-09-02에 장치 검증을 **Final Integration으로 연기**하고 위험을 수용했다
(`ADR-0003` §12 · §12.A — `ASSUMPTION A-REC-001`). §12의 8개 항목은 전부 `DEFERRED`다.

### Phase 3 — Local Transcription · 2026-09-03 · **engineering DONE · 실제 추론 DEFERRED**

Task 9개, 전부 Gate PASS + 독립 Verifier PASS. **9개 모두 첫 시도 통과** (Worker timeout
1800초 조정 이후 — OBS-021).

| Task | 결과물 |
| --- | --- |
| TASK-023 | `ADR-0007` — 통합 방식 결정 |
| TASK-024 | 파생 전사 입력 (메모리 f32 · 원본 불변) |
| TASK-025 | timestamp 정규화 경계 |
| TASK-026 | 전사 실행 경계 · 모델 해석 · 실패 분류 |
| TASK-027 | orchestration · Transcript 버전 규칙 |
| TASK-028 | 비동기 실행 · command · IPC |
| TASK-029 | `automatic_transcription` 설정 |
| TASK-030 | Detail Transcript 탭 |
| TASK-031 | smoke test 절차 문서 · ADR 결과 정리 |

**채택된 통합 방식: `whisper-rs` 0.16 in-process 링크** (`ADR-0007`).
sidecar를 쓰지 않아 `bundle.externalBin`도 `src-tauri/binaries/`도 shell 권한도 없다 —
tauri#11992 notarization 위험을 구조적으로 피했고, **사용자는 whisper·CMake·Homebrew를
설치하지 않는다.** 저장소 밖에서 오는 것은 **모델 파일 하나뿐**이다.

핵심 성질:

```text
원본 오디오 불변 — 파생 입력은 파일이 아니라 메모리 f32 버퍼다.
                   TranscriptionInput에 경로 필드가 없고 File::create도 없다
timestamp        — parse.rs의 to_milliseconds() 한 곳에서만 센티초→밀리초, overflow 검사
재전사 실패      — 실패 경로가 set_current_transcript를 부르지 않으므로 current가 유지된다
자동 전사        — automatic_processing과 별개 값. 둘 다 기본 OFF
모델             — 앱 데이터 디렉터리. 저장소에 커밋하지 않는다
```

검증: build/lint/test Gate green · **자동 테스트 406개** (vitest 201 · Rust 205).

**⚠️ IMPLEMENTED / VERIFIED로 기록하지 않는 것:**

```text
실제 Whisper 추론이 한 번이라도 성공한다   ← NOT RUN
segment timestamp 단위가 실제로 센티초인가  ← UNVERIFIED
번들 whisper.cpp 버전 · Metal 가속 여부      ← UNVERIFIED
release 빌드 · 번들 .app에서의 동작          ← UNVERIFIED
한국어 품질 · 혼용 · 1시간 소요 시간         ← DEFERRED
```

절차는 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`. 가정은 `ADR-0007` §16.3.1
(`A-TRANS-001`).

### Phase 2B — Reliable Recording · 2026-09-03 · **engineering DONE · 장치 검증 DEFERRED**

Task 8개, 전부 Gate PASS + 독립 Verifier PASS.

| Task | 결과물 |
| --- | --- |
| TASK-015 | recording state machine (순수 모듈, 하드웨어 없이 테스트) |
| TASK-016 | backend가 소유하는 녹음 session — Record/Pause/Resume/Stop (R-001) |
| TASK-017 | Stop = 파일 확정 + Recording 영속화 · 보상 정책 (`ADR-0004`) |
| TASK-018 | Settings의 default microphone · 사라진 장치 처리 |
| TASK-019 | macOS 마이크 권한 경계 · 실패 매핑 (`ADR-0005`) |
| TASK-020 | Recording 화면 — Phase 2A 임시 spike 표면을 **남김없이** 대체 |
| TASK-021 | Recording Detail 재생 · 파일 없음 상태 |
| TASK-022 | 결정 기록 · 남은 UNVERIFIED 정리 |

검증: build/lint/test Gate green · **자동 테스트 285개** (vitest 154 · Rust 131).

핵심 성질:

```text
녹음 session은 backend가 소유한다 — 화면 이동이 캡처를 끊지 않는다 (R-001)
Stop 성공 = 파일 존재·크기 확인 + 영속화 (R-002)
DB 실패 시 audio를 지우지 않는다 — 경로를 담은 실패를 돌려준다 (INV-3 · INV-4)
파일이 사라져도 레코드를 지우지 않는다 — 알리기만 한다
권한 판정 수단이 없는 구간은 단정하지 않고 UNVERIFIED로 남긴다
```

**⚠️ IMPLEMENTED / VERIFIED로 기록하지 않는 것:**

```text
실제 마이크가 이 Mac에서 동작한다
실제 TCC 권한 프롬프트
선택한 물리 마이크가 실제로 쓰인다
실제 녹음 음질 · 재생 음질
장시간 녹음 안정성
```

전부 Final Integration의 hard human gate로 연기됐다.
Runtime 관찰은 `LOOP-RUNTIME-FIELD-NOTES.md` OBS-020 · OBS-021.

> Phase 2는 두 단계다. engine 확정에 필요한 증거 일부(실제 권한 프롬프트 · 실제 코덱 ·
> 실제 음질)는 자동 검증이 불가능해 사람이 앱을 실행해야 한다. 그래서 잠정 선택 + 최소
> spike(2A) → 사람의 장치 검증 → production 구현(2B) 순서로 나눴다 (`PRODUCT-SPEC.md` §6.1).
### Phase 4 — AI Provider System + Local AI · 2026-09-04 · **engineering DONE · 실제 호출 DEFERRED**

Task 11개(TASK-032 ~ TASK-042), 전부 Gate PASS + 독립 Verifier PASS.

| Task | 결과물 |
| --- | --- |
| TASK-032 | `ADR-0008` — provider 경계 · 호출 주체 · 구조화 출력 · schema · context 전략 · 재생성 정책 |
| TASK-033 | structured note schema · 방어적 파싱 · 프롬프트와 `promptVersion` |
| TASK-034 | `NoteAiProvider` 계약 · AI 실패 타입 · 테스트 전용 fake provider |
| TASK-035 | AI provider 설정 영속화 (migration 6 · INV-7) |
| TASK-036 | Ollama adapter — HTTP 경계 분리와 실패 매핑 |
| TASK-037 | AI Note 생성 orchestration — provenance와 원본 불변 |
| TASK-038 | AI Note command 경계와 frontend IPC client |
| TASK-039 | Recording Detail의 AI Note 탭 |
| TASK-040 | Settings — provider 설정 · 연결 확인 · 모델 선택 · 로컬/외부 표시 |
| TASK-041 | INV-8 · INV-6 교차 검증 |
| TASK-042 | Phase 4 결정 기록 정리와 Human Review 절차 |

**채택된 경계: 호출 주체는 Rust backend, 벤더 지식은 `ai/ollama/` 하나** (`ADR-0008`).
Ollama 기본 CORS 허용 origin에 Tauri webview origin이 없다는 §14.5의 사실이 결정적이었다.
구조화 출력은 Ollama의 `format` 수단을 쓰되 **거기에 기대지 않는다** — 응답이 기대 schema와
어긋나는 것은 로컬 소형 모델의 예외가 아니라 기본 경로라서, 파싱 실패가 별도의 재시도 가능
실패로 다뤄진다.

**재생성 정책은 append-only 이력이다** — 재생성이 기존 AINote를 대체하지 않고 새 레코드를
더한다. Transcript는 어느 경로로도 쓰이지 않는다 (INV-2).

⚠️ **다음은 이 Phase에서 한 번도 일어나지 않았다** (`A-AI-001`):

```text
실제 Ollama에 요청이 나간 적                  ← NOT RUN
로컬 모델이 만든 노트의 품질                  ← NOT RUN
한국어 + 영어 혼용 처리                        ← NOT RUN
1시간 분량 transcript 처리 시간                ← NOT RUN
Meeting / Study 출력의 유용성 차이             ← NOT RUN
§14.5 엔드포인트·파라미터의 오늘 기준 재확인   ← UNVERIFIED (2026-09-01 기록 그대로)
```

자동 검증은 전부 `ollama::testing::StubServer`와 fake provider로 돌며, 소켓을 여는 파일은
Gate가 **컴파일**할 뿐 실행하지 않는다. 절차와 기록표는
`docs/PHASE-4-AI-NOTE-REVIEW.md`, 가정은 그 문서 §10.2다.

이 Phase의 입력 Transcript는 전부 fixture였다 — `A-TRANS-001`이 여전히 유효하기 때문이다.

Runtime 관찰은 `LOOP-RUNTIME-FIELD-NOTES.md` OBS-022 · OBS-023.

### Phase 5 — Notion & Markdown Export · 2026-09-04 · **engineering DONE · 실제 전송 DEFERRED**

Task 12개(TASK-043 ~ TASK-054), 전부 Gate PASS + 독립 Verifier PASS. **12개 모두 첫 시도 통과.**

| Task | 결과물 |
| --- | --- |
| TASK-043 | `ADR-0009` — export 구조 · 파일명/충돌 정책 · Notion 경로 · 분할 정책 · 중복 sync · SecretStore · 실패 변환 · TLS |
| TASK-044 | Markdown renderer와 안전한 파일명 (순수 모듈) |
| TASK-045 | Markdown 파일 쓰기 경계와 export command |
| TASK-046 | SecretStore 경계와 비-secret Notion 설정 (migration 7) |
| TASK-047 | Notion adapter — 요청 조립 · 응답 해석 · 실패 변환 |
| TASK-048 | 긴 markdown 문서의 무손실 분할 (순수 모듈) |
| TASK-049 | Notion sync 실행 순서와 `NotionSync` 영속화 |
| TASK-050 | Notion command와 IPC 표면 |
| TASK-051 | Recording Detail · Recordings 목록 — Export와 Send to Notion |
| TASK-052 | Settings — Notion 구역과 connection test |
| TASK-053 | Phase 5 불변 규칙 전용 자동 테스트 |
| TASK-054 | ADR-0009 확정과 운영자 smoke test 절차 |

**채택된 경로: Notion Markdown Content API** (`ADR-0009`). §14.9가 UNVERIFIED로 남겨 둔
항목을 Phase 5 계획 시점에 확인해 **VERIFIED**가 됐고 (`PRODUCT-SPEC` §14.9.1),
그래서 블록 JSON을 조립하지 않는다 — §11의 export 산출물을 그대로 보낸다.
렌더링은 한 방향이다: `export::Document → Markdown → (파일 | Notion)`.
**Markdown을 되파싱하는 경로는 없다.**

**중복 sync 정책: Recording 하나 ↔ Notion 페이지 하나.** `notion_syncs.recording_id`가
PRIMARY KEY라 스키마가 이미 그것을 강제하며, 부분 전송 뒤 재시도는 같은 페이지에서
이어간다 (`a_retry_after_a_failed_second_chunk_continues_on_the_same_page_and_makes_no_duplicate`).

**분할 예산은 이 앱이 고른 값이다** — 확인된 Notion 한도가 아니다. VERIFIED된 일반 요청
한도(1000 blocks · 500KB) 아래에서 보수적으로 잡았고, markdown 엔드포인트 전용 상한은
**UNVERIFIED**로 남아 있다 (`PRODUCT-SPEC` §14.9.1).

⚠️ **다음은 이 Phase에서 한 번도 일어나지 않았다** (`A-NOTION-001`):

```text
실제 Notion 워크스페이스로 요청이 나간 적          ← NOT RUN
만들어진 페이지가 읽을 만한 구조인가                ← NOT RUN
1시간 transcript가 실물에서 온전한가                ← NOT RUN
export Markdown이 Obsidian/NotebookLM에서 쓸 만한가 ← NOT RUN
실제 OS 자격증명 저장소에 token이 담기는가          ← NOT RUN
markdown 엔드포인트 전용 본문 크기 상한             ← UNVERIFIED
```

자동 검증은 전부 stub transport와 메모리 SecretStore double로 돌며, 소켓을 여는 파일과
자격증명 저장소를 여는 파일은 Gate가 **컴파일**할 뿐 실행하지 않는다.
절차와 기록표는 `docs/PHASE-5-NOTION-SMOKE-TEST.md`, 가정은 그 문서 §10.1이다.

**Markdown 파일 export는 예외다** — 외부 의존이 없어 임시 디렉터리에 실물 파일을 쓰는
자동 검증이 실제로 지나간다.

이 Phase의 입력도 전부 fixture였다 — `A-TRANS-001` · `A-AI-001`이 여전히 유효하기 때문이다.

Runtime 관찰은 `LOOP-RUNTIME-FIELD-NOTES.md` OBS-022 · OBS-023 · OBS-024.

### Phase 6 — Cross-platform Validation & Hardening · **PLANNED**
---

## 6. Validation Model

| | 무엇을 보장하는가 | 수단 |
| --- | --- | --- |
| **Automated validation** | 위의 전부 + **Markdown 렌더링 결정성 · 파일명 정규화 · 무손실 분할과 재조립 · Notion 요청 조립과 실패 변환 · `Retry-After` 준수 · 중복 페이지 없는 재시도 · SecretStore 경계** — **자동 테스트 1,081개** (web `vitest` 384 · Rust `cargo test` 697) | `build` · `lint` · `test` Gate + 독립 Verifier<br>Task별 Gate는 최소·관련 범위로 좁힐 수 있으나 **Phase 종료 시에는 저장소 전체에 세 Gate를 모두 돌린다** (2026-09-04 운영자 결정) |
| **Human validation / witness** | 실제 마이크 음질 · 재생 음질 · **실제 Whisper 추론(`A-TRANS-001`)** · **실제 Ollama 호출과 AI Note의 유용성(`A-AI-001`)** · **실제 Notion 전송과 페이지 품질 · 1시간 transcript 온전성(`A-NOTION-001`)** · export Markdown의 외부 도구 호환성 · **화면의 시각적 완성도** · **Windows 실동작** · **연기된 recording 장치 검증(ADR-0003 §12)** | 사람이 직접 확인 |

> Phase 1에서 사람이 확인한 것: 번들 `.app`의 Info.plist 병합(확인됨) ·
> 권한 문구(확인됨) · 화면 레이아웃(**소스 수준 검토만 — 실행 화면 확인은 사용자 몫**).

**자동으로 판정할 수 없는 것을 자동 PASS로 적지 않는다.** 상세는 `PRODUCT-SPEC.md` §18.

---

## 7. Known Boundaries / Deferred Work

- ~~**오디오가 없다**~~ — Phase 2B에서 해소됐다. Stop이 파일을 확정하고 `audioPath`를 채운다.
  남은 미확정은 실제 장치 검증뿐이다 (`A-REC-001`).
- ~~**`currentTranscriptId`를 갱신하는 파이프라인이 없다**~~ — Phase 3에서 해소됐다.
  전사 성공이 Transcript를 덧붙이고 current를 옮기며, 실패는 이전 current를 유지한다.
  남은 미확정은 실제 추론뿐이다 (`A-TRANS-001`).
- ~~**secret 저장소가 없다**~~ — Phase 5에서 해소됐다. Notion integration token이 이 제품의
  **첫 진짜 secret**이며, `platform/secret_store.rs`의 경계 하나(닫힌 `SecretKey` ·
  재현되지 않는 `Secret`)를 통해 OS 자격증명 저장소에만 담긴다.
  **SQLite에는 여전히 secret 열이 없다** — migration 7이 더한 것은 비-secret인
  `notion_parent_page_id` 하나이고, `no_migration_creates_a_place_to_put_a_secret`이
  그대로 통과한다 (INV-7). 이 경계를 미래 Cloud provider 자격증명 체계로 일반화하지 않았다.
  **Windows 구현은 같은 trait 뒤의 자리로만 있고 검증은 Phase 6이다.**
- **⚠️ recording engine이 실제 장치에서 검증되지 않았다** — `cpal`/`hound` 경로는 컴파일되고
  순수 로직은 테스트되지만, 실제 마이크·권한·음질은 확인된 적이 없다.
  V1은 이 미확정 전제 위에 쌓인다 (`ASSUMPTION A-REC-001`).
  확정은 `phase-prompt/Goal.md`의 hard human gate에서만 일어난다.
- **recording engine이 미결정** — §6.1의 기준으로 Phase 2에서 실측과 함께 결정한다.
  문서만으로 결정하지 않기로 했다.
- **whisper 통합 방식이 미결정** — 이 기기에 `cmake`가 없어 whisper.cpp 소스 빌드가
  바로 되지 않는다. Phase 3에서 다룬다.
- **코드서명 / notarization 미검증** — Tauri #11992(sidecar)와 #11951(마이크 TCC 프롬프트)이
  알려진 위험이다. Phase 2·3에서 실제 번들로 확인한다.
- **App Store / Microsoft Store 배포는 범위 밖** (`PRODUCT-SPEC.md` §3).
- **Windows 개발/검증 환경이 아직 없다** — Phase 6에서 확보한다. 그때까지 Windows 지원은
  "차단하지 않는다"까지이며 "동작한다"가 아니다.
- **whisper 바이너리의 플랫폼 비대칭** — Windows는 공식 prebuilt가 있고 macOS는 없다.
  더 어려운 쪽이 primary 개발 플랫폼이다 (`PRODUCT-SPEC.md` §14.4).
- **Ollama 기본 context가 4096 토큰** — 1시간 transcript는 이를 초과한다.
  Phase 4는 `num_ctx`를 명시하고(기본 16384) **들어가지 않는 입력은 보내지도 자르지도 않는
  것**으로 결정했다. 청킹은 하지 않는다 (`ADR-0008` §8). 그 문턱을 사용자가 조정하는
  `ai_context_tokens` 설정은 **결정만 있고 구현되지 않았다** (`ADR-0008` §17.3.2).
- **⚠️ Notion 경로가 실제 워크스페이스에서 검증되지 않았다** — adapter · 분할 · SecretStore ·
  화면이 컴파일되고 자동 검증을 지나지만, **실제 Notion으로 요청이 나간 적이 한 번도 없다**
  (`ASSUMPTION A-NOTION-001`). markdown 엔드포인트 전용 본문 크기 상한도 **UNVERIFIED**이며,
  분할 예산은 그래서 **이 앱이 고른 보수적 값**이지 확인된 API 한도가 아니다.
  절차는 `docs/PHASE-5-NOTION-SMOKE-TEST.md`, 확정은 `phase-prompt/Goal.md`의 hard human
  gate에서만 일어난다.
- **⚠️ AI Note 경로가 실제 추론 서버에서 검증되지 않았다** — 계약 · adapter · 화면이
  컴파일되고 자동 검증을 지나지만, **실제 Ollama에 요청이 나간 적이 한 번도 없다**
  (`ASSUMPTION A-AI-001`). §14.5의 엔드포인트·파라미터 이름은 2026-09-01 기록이며 이 Phase가
  다시 확인하지 못했다(세 Run 연속 네트워크 도구 거부). 절차는
  `docs/PHASE-4-AI-NOTE-REVIEW.md`, 확정은 `phase-prompt/Goal.md`의 hard human gate에서만
  일어난다.

---

## 8. Architecture Documents

| 문서 | 무엇이 들어 있는가 |
| --- | --- |
| `docs/PRODUCT-SPEC.md` | 제품 사양 (source of truth). §14에 2026-09-01 기준 검증된 외부 사실 |
| `phase-prompt/01~06-*.md` | Phase Goal (Phase 2는 `02a` → `02` 두 단계) |
| `phase-prompt/Goal.md` | 최종 통합 Goal |
| `docs/ADR-0001-local-persistence.md` | persistence 엔진 선택 근거 · crate 확인 상태 · migration 모델 |
| `docs/ADR-0002-macos-microphone-usage-description.md` | 마이크 권한 선언의 위치와 성격 · 구현하지 않은 것의 경계 |
| `docs/ADR-0003-recording-engine.md` | recording engine 잠정 선택 · **연기된 장치 검증 기록표(§12)** · 실행 절차(§12.1) |
| `docs/ADR-0004-recording-session-lifecycle.md` | session 소유권 · Stop 확정 계약 · 어긋난 상태의 보상 정책 |
| `docs/ADR-0005-microphone-permission.md` | 권한 판정의 VERIFIED / UNVERIFIED 경계 |
| `docs/ADR-0008-note-ai-provider.md` | AI provider 경계 · 호출 주체(Rust backend) · 구조화 출력과 방어 경로 · structured note schema · context 전략 · **재생성 append-only 정책** · 설정값 노출 금지(§11.3) |
| `docs/PHASE-4-AI-NOTE-REVIEW.md` | **실제 Ollama Human Review 절차와 빈 기록표.** §10.2가 이 Phase가 확인하지 **않은** 것의 정본이다 (`A-AI-001`) |
| `docs/ADR-0009-notion-and-export.md` | Markdown 구조 · 파일명/충돌 정책 · Notion Markdown Content API 경로 · 분할 예산의 성격 · **중복 sync 정책** · SecretStore 경계 · 실패 변환 · `ureq` TLS |
| `docs/PHASE-5-NOTION-SMOKE-TEST.md` | **실제 Notion smoke test 절차와 빈 기록표.** §10.1이 `A-NOTION-001`의 정본이고 §12가 확인되지 **않은** 것의 목록이다 |
| `docs/GIT-WORKFLOW.md` | Git/GitHub 운영 정책 — Phase 단위 commit · public 저장소 안전 규칙 |
| `docs/LOOP-RUNTIME-FIELD-NOTES.md` | Runtime 운용 관찰 기록 |
| `CLAUDE.local.md` | 대화형 세션 운영 지침 |

---

## 9. Decision History

| 시점 | 결정 | 근거 | 현재 상태 |
| --- | --- | --- | --- |
| 2026-09-01 | 스택을 Tauri v2 + React + TS + Vite로 확정 | `PRODUCT-SPEC.md`의 macOS 전용 · 로컬 우선 요구. Tauri v2가 현재 stable line이며 desktop만 목표하면 Xcode CLT로 충분함을 확인 | 채택 |
| 2026-09-01 | Gate 3개가 frontend와 Rust를 **함께** 검사하도록 구성 | Phase 2~3의 실질 로직이 Rust에 들어간다. web만 검사하는 Gate는 완료 판정 근거가 되지 못한다 | 채택 |
| 2026-09-01 | 제품 라이브러리(SQLite · cpal · whisper · Anthropic · Notion)를 Bootstrap에서 설치하지 **않음** | Bootstrap은 개발 baseline까지다. 미래 의존성 선입금 금지 | 채택 |
| 2026-09-01 | recording engine 결정을 Phase 2로 미룸 | 문서상 후보는 좁혀졌으나(§4) 결정적 사실이 실측 필요 — WKWebView 코덱, TCC 프롬프트, 1시간 안정성 | 보류 (CANDIDATE) |
| 2026-09-01 (delta) | Windows를 지원 대상 플랫폼으로 추가 | 제품 요구사항 변경. core/domain이 OS 가정을 갖지 않도록 INV-10을 추가하되, 실제 플랫폼 차이가 있는 곳만 추상화한다 | 채택 |
| 2026-09-01 (delta) | AI를 vendor 중립 Provider 추상화로 전환 | 벤더·가격·티어 정책은 외부에서 바뀐다. 그때 흔들리는 것이 adapter 하나여야 한다 (INV-9) | 채택 |
| 2026-09-01 (delta) | AI를 core requirement에서 제외 (INV-8) | 제품의 본질은 녹음과 전사의 보존이다. AI 부재가 core pipeline을 막으면 Local-first 주장이 성립하지 않는다 | 채택 |
| 2026-09-01 (delta) | 첫 provider를 로컬 Ollama로, Cloud는 DEFERRED | V1 성공 조건(§17.1)이 AI 없이 성립하므로 cloud provider는 V1 최소 범위가 아니다. 다만 구현이 하나뿐인 추상화는 검증된 것이 아니므로 Phase 4가 test double로 계약을 함께 검증한다 | 채택 |
| 2026-09-01 (delta) | Ollama를 Rust backend에서 호출하는 방향 유력 | Ollama 기본 CORS 허용 origin에 Tauri webview origin이 없다. 프론트엔드 직접 호출은 사용자에게 환경변수 설정을 요구하게 된다 (§14.5) | **확정** (아래 Phase 4 항목) |
| 2026-09-01 (delta) | persistence를 Rust 내부(`rusqlite` 계열)로 두는 방향 유력 | frontend가 임의 SQL executor가 되는 것을 막고 domain/repository 및 secret 경계(INV-7)와 일관되게 한다. Ollama 호출 방향과도 일관된다 | **확정** (아래 Phase 1 항목) |
| 2026-09-02 (Phase 1) | **`rusqlite` 0.40.2 + `bundled` 채택 확정** | 로컬 빌드로 feature 존재와 SQLite 3.53.2 번들을 확인했다. `tauri-plugin-sql`은 frontend가 SQL executor가 되어 탈락 | 채택 (`ADR-0001`) |
| 2026-09-02 (Phase 1) | migration을 `PRAGMA user_version` + `schema_migrations` 이중 기록으로 관리 | 미적용분만 순서대로 적용하고, 스키마 변경이 사용자 데이터를 지우지 않도록 한다 (INV-4) | 채택 (`ADR-0001`) |
| 2026-09-02 (Phase 2A) | recording engine을 **잠정** `cpal` + `hound`로 선택 | §6.1의 14개 기준 비교. native 경로는 PCM/WAV를 직접 만들고 두 플랫폼에서 같은 포맷을 낸다 | **PROVISIONAL** (`ADR-0003`) |
| 2026-09-02 (운영자) | 실제 장치 Human Review를 **Final Integration으로 연기** | 개발 흐름 유지. 위험을 명시적으로 수용했다 — 가정이 틀리면 recording 구현에 rework | 채택 · 위험 수용 (`ADR-0003` §12.A) |
| 2026-09-02 (Phase 1) | `NSMicrophoneUsageDescription`을 `src-tauri/Info.plist`에 선언 | `tauri.conf.json` 키가 아니다. 번들 `.app`에서 CLI 생성값과 병합되는 것을 실제로 확인했다 | 채택 (`ADR-0002`) |
| 2026-09-01 (rev 3) | **Transcript cardinality를 `Recording 1:N Transcript`로 확정** | rev 1~2의 §7은 `1:1`, §8은 "재전사가 새 Transcript를 만든다"로 서로 모순이었다. 1:1은 재전사 시 overwrite를 강제하므로 INV-2(immutable source)와 양립할 수 없다. **1:1을 잘못된 명세로 판단해 정정했다** | 채택 (`PRODUCT-SPEC.md` §7.1) |
| 2026-09-01 (rev 3) | `Recording.currentTranscriptId` 도입 | 여러 Transcript version 중 무엇을 표시하고 후속 작업의 기본 입력으로 쓸지 명시해야 한다. 재전사 실패 시 기존 current를 유지해 유효한 Transcript를 잃지 않는다 (INV-3의 귀결) | 채택 (§7.2) |
| 2026-09-01 (rev 3) | `AINote.transcriptId`를 provenance에 추가 | Transcript가 1:N이 되면서 `recordingId`만으로는 어떤 version에서 나온 노트인지 구분할 수 없다 | 채택 (§7.3) |
| 2026-09-04 (Phase 4) | **AI 호출 주체를 Rust backend로 확정** | §14.5의 CORS 사실이 결정적이다. 프론트엔드 직접 호출은 사용자에게 Ollama 환경변수 설정을 요구하게 되고, Phase 5의 Notion 호출과도 어긋난다 | 채택 (`ADR-0008` §5) |
| 2026-09-04 (Phase 4) | **`ureq` 3.4를 TLS feature 없이 채택.** wrapper crate(`ollama-rs`)를 쓰지 않는다 | 저장소의 모든 경계가 동기이며 호출 하나 때문에 async runtime을 들이지 않는다. 이 Phase는 로컬 주소로만 나가므로 TLS는 선입금이다 — HTTPS가 필요한 Phase 5에서 켠다 | 채택 (`ADR-0008` §12.2) |
| 2026-09-04 (Phase 4) | **구조화 출력에 Ollama `format`을 쓰되 거기에 기대지 않는다** | 응답이 기대 schema와 어긋나는 것은 로컬 소형 모델에서 예외가 아니라 기본 경로다. 파싱 실패를 별도의 **재시도 가능** 실패로 두고 앱이 깨지지 않게 한다 | 채택 (`ADR-0008` §6) |
| 2026-09-04 (Phase 4) | **재생성은 append-only 이력.** 기존 AINote를 대체하지 않는다 | 대체는 되돌릴 수 없고, 어떤 프롬프트·모델이 어떤 노트를 만들었는지 provenance가 남지 않는다. Transcript는 어느 경로로도 쓰이지 않는다 (INV-2) | 채택 (`ADR-0008` §9) |
| 2026-09-04 (Phase 4) | **긴 입력은 보내지도 자르지도 않는다.** 청킹하지 않는다 | 잘린 transcript로 만든 노트는 조용히 틀린 노트다. `num_ctx`를 명시하고 들어가지 않으면 그 사실을 실패로 말한다. 사용자 조정 설정(`ai_context_tokens`)은 미구현 | 채택 (`ADR-0008` §8) |
| 2026-09-04 (Phase 5) | **Notion 전송에 Markdown Content API를 쓴다. 블록 JSON을 조립하지 않는다** | §14.9가 UNVERIFIED로 남긴 항목을 계획 시점에 primary source에서 확인해 VERIFIED가 됐다. §11의 export 산출물을 그대로 보내므로 렌더러가 하나로 유지된다 | 채택 (`ADR-0009` · `PRODUCT-SPEC` §14.9.1) |
| 2026-09-04 (Phase 5) | **옛 "2000자 rich text · 100블록 배치"를 제품 요구사항에서 내린다** | 그것은 블록 JSON 경로의 규칙이었다. 지켜야 하는 불변은 수단이 아니라 **"긴 transcript가 잘리거나 조용히 유실되지 않는다"** 이며, 분할 예산은 이 앱이 고른 값이지 확인된 API 한도가 아니다 | 채택 (운영자 Human Review 승인) |
| 2026-09-04 (Phase 5) | **SecretStore 경계를 도입한다** — 제품의 첫 진짜 secret | Phase 4는 Ollama가 token을 요구하지 않아 secret 보관 수단을 만들지 않았다. Notion token은 진짜 secret이므로 SQLite도 frontend도 아닌 OS 자격증명 저장소가 필요하다. Cloud provider 자격증명 체계로 일반화하지 않는다 (선입금 금지) | 채택 (`ADR-0009` §10 · INV-7 · INV-10) |
| 2026-09-04 (Phase 5) | **중복 sync는 기존 페이지를 이어 쓴다.** 새 페이지를 만들지 않는다 | `notion_syncs.recording_id`가 PRIMARY KEY라 스키마가 이미 1:1을 강제한다. 부분 전송 실패 뒤 재시도가 중복 페이지를 만들면 사용자가 치우게 된다 | 채택 (`ADR-0009` §15.4) |
| 2026-09-04 (Phase 5) | **`ureq`의 `rustls`를 켠다** | ADR-0008 §12.2가 "HTTPS가 실제로 필요해질 때 켠다"로 미뤄 둔 시점이 왔다. Notion은 HTTPS다 | 채택 (`Cargo.toml` · `ADR-0009` §15.2) |
| 2026-09-04 (운영자) | **Task별 Gate는 최소·관련 범위로. Phase 종료 시에는 저장소 전체에 세 Gate** | Rust 전용 Task에 frontend 전용 `npm run build`를 거는 것은 Task-local 근거를 더하지 못한다. 다만 Phase 완료 판정은 좁힌 Gate 집합에 기대지 않는다 | 채택 (Phase 5 Human Review) |
| 2026-09-04 (운영자) | **실제 Notion smoke test를 Final Integration으로 연기** | `A-REC-001` · `A-TRANS-001` · `A-AI-001`과 같은 판단이다. 위험을 명시적으로 수용한다 — 가정이 틀리면 adapter와 분할 정책에 rework | 채택 · 위험 수용 (`A-NOTION-001`) |
| 2026-09-04 (운영자) | **실제 Ollama Human Review를 Final Integration으로 연기** | `A-REC-001` · `A-TRANS-001`과 같은 판단이다. 개발 흐름을 유지하고 위험을 명시적으로 수용한다 — 가정이 틀리면 adapter와 프롬프트에 rework | 채택 · 위험 수용 (`A-AI-001` · `docs/PHASE-4-AI-NOTE-REVIEW.md` §10.2) |

---

## 10. Update Rule

이 문서는 **다음 경우에만** 갱신한다.

1. Phase가 최종 **DONE** 상태가 됐을 때
2. 의미 있는 architecture boundary가 바뀌었을 때
3. 외부 engine / adapter 선택이 확정되거나 교체됐을 때
4. 시스템 전체 흐름이 달라졌을 때

**Task마다 갱신하지 않는다.**

Phase 종료 시 §1 · §2 · §3 · §4 · §5 · §6 · §7 · §9를 확인한다.

**History를 덮어쓰지 않는다.** Current / Planned / Deferred 상태는 언제나 구분되어야 한다 —
설치된 의존성을 구현된 기능으로, 계획된 결정을 채택된 구현으로 적지 않는다.
