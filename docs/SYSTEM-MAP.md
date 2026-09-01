# System Map — Molt Note

이 문서는 프로젝트의 **최상위 지도**다. 상세 구현 문서가 아니라 진입점이다.
상세는 §8의 문서로 넘긴다.

> **현재 상태: Bootstrap 완료. 제품 기능은 아직 하나도 구현되지 않았다.**
> 이 문서는 Bootstrap 시점(2026-09-01)에 **저장소에 실제로 존재하는 것만** 기록한
> 골격이다. Phase 1이 끝나면 실제 architecture로 채운다.
>
> **2026-09-01 갱신** — Requirements Delta로 제품 경계가 바뀌어 §1 · §4 · §5 · §7 · §9를
> 갱신했다. 구현 상태는 **바뀌지 않았다** (여전히 Bootstrap만 DONE).
> 바뀐 것은 계획된 경계이지 구현이 아니다.

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
| **지금 동작한다 (DONE)** | 개발 baseline뿐 — Tauri v2 앱 셸이 빌드되고, build/lint/test Gate가 실행된다 |
| **다음 단계 (PLANNED)** | Phase 1 — 앱 셸 · 로컬 영속성 · 데이터 모델 · 플랫폼 경계 · 마이크 권한 선언 |
| **미룬 것 (DEFERRED)** | **Cloud AI Providers (Claude · Gemini · Groq)** · search · tags · processing queue · menu bar (`PRODUCT-SPEC.md` §16) |
| **후보 (CANDIDATE)** | recording engine (§4) · whisper 통합 방식 (§4) · persistence crate (§4) |

---

## 2. Current System Flow

**현재 구현된 흐름은 없다.** Bootstrap이 만든 것은 빈 Tauri 앱 셸이다.

```text
(현재)  npm run tauri dev  →  빈 Tauri 창
```

목표 흐름은 `docs/PRODUCT-SPEC.md` §4에 있다. 아직 어느 단계도 구현되지 않았으므로
여기에 옮겨 적지 않는다 — 구현되는 대로 이 절을 채운다.

---

## 3. Major Components

| Component | 역할 | 상태 |
| --- | --- | --- |
| Tauri v2 앱 셸 (`src-tauri/`) | macOS 데스크톱 런타임 · Rust backend 진입점 | **DONE** (셸만) |
| React + Vite frontend (`src/`) | UI | **DONE** (scaffold 기본 화면만) |
| Bootstrap baseline test (`tests/`) | 설정 정합성 검사 | **DONE** |
| 로컬 영속성 계층 | Recording · Transcript · AINote · NotionSync 저장 | **PLANNED** (Phase 1) |
| `AppDataDirectory` 경계 | 플랫폼별 앱 데이터 경로 결정을 한 곳에 가둔다 | **PLANNED** (Phase 1) |
| Recording engine | 마이크 캡처 · pause/resume · 파일 생성 | **PLANNED** (Phase 2) |
| Transcription 엔진 통합 | whisper 실행 · JSON 파싱 | **PLANNED** (Phase 3) |
| **`NoteAIProvider` 경계** | **vendor 중립 AI 계약. domain은 벤더를 모른다 (INV-9)** | **PLANNED** (Phase 4) |
| **Ollama adapter** | **로컬 Ollama → Structured Note. 첫 provider 구현** | **PLANNED** (Phase 4) |
| Notion / Markdown renderer | Structured Note를 외부 형식으로 내보낸다 | **PLANNED** (Phase 5) |
| Windows 지원 검증 | §3.1 핵심 기능의 Windows 실동작 | **PLANNED** (Phase 6) |

---

## 4. External Dependency Boundary

| 구분 | 항목 | 비고 |
| --- | --- | --- |
| **선택됨 · 현재 사용 중** | Tauri v2 (2.11.5) · React 19 · Vite 7 · TypeScript 5.8 · ESLint · Vitest | 앱 셸과 개발 baseline에서 실제로 쓰인다 |
| **설치됨 · 미통합** | `@tauri-apps/plugin-opener` | scaffold 기본 포함. **기능이 아니다** |
| **후보 · 미선택** | recording: `cpal`+`hound` / webview MediaRecorder / 커뮤니티 플러그인<br>persistence: `rusqlite` / `tauri-plugin-sql`<br>transcription: whisper.cpp sidecar / `whisper-rs`<br>AI: 로컬 Ollama REST (`reqwest` 또는 `ollama-rs`)<br>Notion: `@notionhq/client` | **설치되지 않았다.** 각각 이를 필요로 하는 Phase에서 검증과 함께 선택한다. 확인된 사실은 `PRODUCT-SPEC.md` §14 |
| **미룸** | Cloud AI Providers (Claude · Gemini · Groq) | V1 성공 조건(§17.1)이 AI 없이 성립하도록 정의됐다. 추상화는 Phase 4에 있고 구현은 필요해질 때 (`PRODUCT-SPEC.md` §16) |
| **미룸** | Claude Agent SDK | 단일 요청/응답 변환에 agentic loop가 필요한 이유가 없다 (`PRODUCT-SPEC.md` §9.4) |
| **미룸** | Ollama 자체의 번들링 | 사용자가 실행 중인 인스턴스에 연결만 한다 (`PRODUCT-SPEC.md` §9.4 · §15) |

외부 전송 경계는 `docs/PRODUCT-SPEC.md` §12가 정본이다. 현재 외부로 나가는 경로는 **없다.**

경계는 세 단계로 나뉜다 — **완전 로컬**(녹음·전사) · **로컬 AI**(Ollama) ·
**외부**(Cloud provider · Notion). 로컬 AI는 인터넷으로 나가지 않으므로 외부가 아니다.

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

### Requirements Delta — 2026-09-01 · **문서만 변경 · 구현 변화 없음**

Human Review 중 제품 요구사항이 바뀌어 Product Spec을 rev 2로 갱신했다.

- Windows를 **지원 대상 플랫폼**으로 추가 (rev 1에서는 non-goal)
- AI를 **vendor 중립 Provider 추상화**로 전환하고 **core requirement에서 제외** (INV-8 · INV-9)
- 첫 AI provider를 **로컬 Ollama**로, Cloud provider를 **DEFERRED**로
- 불변 규칙 INV-8 · INV-9 · INV-10 추가
- Phase Map에 Phase 6(Cross-platform Validation) 추가
- rev 1 §14.7의 근거 없는 VERIFIED 표기를 정정하고 §14.7(Persistence)을 신설

**구현은 하나도 바뀌지 않았다.** 여전히 Bootstrap만 DONE이다.

### Phase 1 — Application Foundation · **PLANNED**
### Phase 2 — Reliable Recording · **PLANNED**
### Phase 3 — Local Transcription · **PLANNED**
### Phase 4 — AI Provider System + Local AI · **PLANNED**
### Phase 5 — Notion & Markdown Export · **PLANNED**
### Phase 6 — Cross-platform Validation & Hardening · **PLANNED**

---

## 6. Validation Model

| | 무엇을 보장하는가 | 수단 |
| --- | --- | --- |
| **Automated validation** | 현재는 개발 baseline 정합성만 (설정 파싱 · 버전 일치 · 컴파일 · lint) | `build` · `lint` · `test` Gate + Verifier |
| **Human validation / witness** | 실제 마이크 음질 · 재생 음질 · AI Note의 유용성 · Notion 페이지 품질 · UI 체감 · **Windows 실동작** | 사람이 직접 확인 |

**자동으로 판정할 수 없는 것을 자동 PASS로 적지 않는다.** 상세는 `PRODUCT-SPEC.md` §18.

---

## 7. Known Boundaries / Deferred Work

- **제품 기능이 전혀 없다** — Bootstrap은 개발 환경만 만든다. 의도된 상태다.
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
  Phase 4의 필수 설계 항목이다 (`PRODUCT-SPEC.md` §14.5).

---

## 8. Architecture Documents

| 문서 | 무엇이 들어 있는가 |
| --- | --- |
| `docs/PRODUCT-SPEC.md` | 제품 사양 (source of truth). §14에 2026-09-01 기준 검증된 외부 사실 |
| `phase-prompt/01~06-*.md` | Phase Goal |
| `phase-prompt/Goal.md` | 최종 통합 Goal |
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
| 2026-09-01 (delta) | Ollama를 Rust backend에서 호출하는 방향 유력 | Ollama 기본 CORS 허용 origin에 Tauri webview origin이 없다. 프론트엔드 직접 호출은 사용자에게 환경변수 설정을 요구하게 된다 (§14.5) | 검토 중 (Phase 4 확정) |
| 2026-09-01 (delta) | persistence를 Rust 내부(`rusqlite` 계열)로 두는 방향 유력 | frontend가 임의 SQL executor가 되는 것을 막고 domain/repository 및 secret 경계(INV-7)와 일관되게 한다. Ollama 호출 방향과도 일관된다 | 검토 중 (Phase 1 확정) |

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
