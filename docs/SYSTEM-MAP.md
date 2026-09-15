# System Map — Molt Note

이 문서는 프로젝트의 **최상위 지도**다. 상세 구현 문서가 아니라 진입점이다.
상세는 §8의 문서로 넘긴다.

> **현재 상태: Phase 5.8까지 engineering DONE. 그 뒤 2026-09-08의 변경들은 Runtime을
> 거치지 않았다 (§5.9).**
>
> ⚠️ **2026-09-08은 이 저장소가 Runtime 밖에서 하루를 보낸 날이다.** Phase 5.9의 Plan은
> 만들어졌으나 승인되지 않았고, 운영자가 직접 구현으로 전환했다. 그날 들어온 것들은
> Gate 셋을 지났고 전용 테스트와 ADR 기록이 붙어 있으나 **Verifier를 지나지 않았다.**
> 무엇이 왜 그렇게 들어왔는지는 `ADR-0007 §23`과 `LOOP-RUNTIME-FIELD-NOTES` OBS-028에 있다.
>
> (직전 상태: Phase 5.6 · Phase 5.7 모두 engineering DONE (2026-09-07) — 두 Phase 다
> Human Review 미실행.)
> (직전 상태: Phase 5.5 완료 · 2026-09-06 · 그 앞은 Phase 5 완료 · 2026-09-04.
> **Phase 5.6은 2026-09-07 이 문서가 갱신되기 직전까지 부분 완료였다** — 남은 넷(TASK-072 ~
> TASK-075)이 Phase 5.7 뒤에 돌아 마무리됐다. 아래 별도 문단. 서술은 그 위에 쌓이며 앞 Phase의
> 기록을 지우지 않는다.)
> 앱 셸 · 로컬 영속성 · §7 데이터 모델 ·
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
> 그리고 그 문에서 **세 번째 갈래**가 났다 — **Manual AI Handoff**(Phase 5.5).
> AI Provider가 하나도 없어도 사람이 Transcript를 자기 AI 채팅으로 가져갈 수 있다
> (Copy AI Prompt · Copy Transcript · Export for AI). 같은 Phase에서 화면이 딛고 설
> **UI 기반**(타입·여백 스케일 · accent/상태색 · 보이는 focus · 빈 상태 · 로딩)이 생겼고,
> 로컬 Ollama provider는 **삭제가 아니라 '선택적 · 로컬 · 고급'으로 재배치**됐다.
>
> ⚠️ **그러나 실제 하드웨어/추론서버/워크스페이스/AI 채팅에서 확인된 적이 거의 없다.**
> 미확정 전제가 **넷**이다 — `A-REC-001`(실제 마이크 미검증) ·
> `A-TRANS-001`(실제 Whisper 추론이 **제품 경로에서 쓸 수 있게 동작한 적이 없다** — 아래) ·
> `A-AI-001`(실제 Ollama 미호출) ·
> `A-NOTION-001`(**실제 Notion 워크스페이스로 요청이 나간 적이 없다**).
> 넷 다 Final Integration의 hard human gate로 연기됐다.
> Phase 5.5는 여기에 **`A-` 가정을 더하지 않았다**(D-4) — 대신 자동으로 판정할 수 없는
> **Human Review 항목 셋**을 남겼다 (`docs/PHASE-5.5-HUMAN-REVIEW.md`. 기록표는 **비어 있다**).
>
> ⚠️ **2026-09-05에 운영자가 처음으로 앱을 실행해 실제 전사를 돌렸다** (72분 한국어 회의).
> 엔진 경로 자체는 지났으나(segments · timestamp · 재시작 생존) **언어가 설정되지 않아
> 한국어가 영어로 강제 디코딩됐고 결과는 사용할 수 없었다.** `A-TRANS-001`은 해소되지
> **않았다.** 기록은 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록, 수정은 Phase 5.6
> (`phase-prompt/05.6-transcription-correctness-and-reach.md`)이 맡았다.
>
> ⚠️ ~~**Phase 5.6은 부분 완료다 (2026-09-07 기준).**~~ **그것은 이 문서의 직전 갱신 시점
> 기록이며, 그 뒤 남은 넷이 돌았다.** 당시의 사실은 이랬다 — 열 Task 중 TASK-066 ~ TASK-071이
> DONE이고 TASK-072 ~ TASK-075는 TODO였으며, 그래서 이 문서에 Phase 5.6의 완료 기록이 없었다.
> **2026-09-07 늦게 TASK-072 ~ TASK-075가 돌아 열 Task 전부가 DONE이 됐다** (§5).
> 저장소에 들어온 것은 전사 언어 설정(저장소 → 화면) · 언어가 엔진에 도달하는 경로 ·
> `whisper-rs`의 `metal` feature · `transcripts.transcription_ms` · 내보낸 파일 자리를 여는
> command에 더해, **AI Handoff 산출물의 크기와 무손실 분할**(`export/portion.rs` → command →
> AI Note 탭)과 **이 Phase의 네 불변 전용 테스트**다.
> **engineering DONE이지 Human Review가 끝났다는 뜻이 아니다** — 한국어 전사 품질 · 모델 크기 ·
> Metal 체감 · 파일 도달 · handoff의 실제 사용은 전부 사람이 판정하며 기록표는 **비어 있다**
> (`docs/PHASE-5.6-HUMAN-REVIEW.md`).
>
> ⚠️ **2026-09-07에 운영자가 두 번째로 실제 전사를 돌렸다** (51분 한국어 회의).
> **Phase 5.6의 언어 수정은 동작했다** — 설정한 `ko`가 엔진에 도달했다. 그런데 그 아래에 있던
> 문제가 드러났다: **전사가 51분 전 구간에서 환각으로 채워졌고(고유 문장 2개 · 한 문장이 99.0%)
> 제품은 그것을 `done`으로 저장했다.** 그 녹음의 평균 입력 레벨은 성공한 9/4 녹음보다
> **16.4 dB 낮았다.** `A-TRANS-001`은 **여전히 열려 있다.**
>
> 그 두 침묵을 메운 것이 **Phase 5.7**이다 (로드맵에 없던 삽입 · 운영자 결정 2026-09-07).
> 녹음 중에 **입력 레벨과 그 판정이 화면에 보이고**, 쓸 수 없을 만큼 낮으면 **정지 전에**
> 경고가 뜬다. 전사는 **저장 직전에 붕괴 판정을 통과해야** 하며, 걸리면 Transcript를 남기지
> 않고 `failed`가 되고, 화면은 **무엇이 얼마나 반복됐는지와 다음에 할 일**을 문장으로 보여준다.
> **이 Phase는 마이크 게인도 오디오 정규화도 넣지 않았다** — 청크 분할 · state 재생성 ·
> 반복 차단 · VAD · 자동 언어 감지 개선도 마찬가지로 **다음 Phase 후보로 남았다** (§5).
>
> 갱신 이력:
> - 2026-09-01 Bootstrap — 개발 baseline과 Gate 확보
> - 2026-09-01 Requirements Delta — Windows 지원 대상 추가 · AI를 vendor 중립 Provider로 전환
> - 2026-09-01 Product Spec rev 3 — Transcript cardinality를 `Recording 1:N Transcript`로 확정
> - **2026-09-02 Phase 1 DONE** — §1 ~ §9를 실제 구현 기준으로 갱신
> - **2026-09-04 Phase 4 DONE** — §1 · §3 · §4 · §5 · §6 · §7 · §8 · §9에 AI Provider 경계 반영
> - **2026-09-04 Phase 5 DONE** — Markdown export · Notion sync · SecretStore 경계 반영
> - **2026-09-06 Phase 5.5 DONE** — Manual AI Handoff 경계 · clipboard 경계 · UI 기반 ·
>   Ollama 재배치 반영 (§1 · §2 · §3 · §5 · §6 · §7 · §8 · §9). Phase 5.5는 **로드맵에 없던
>   삽입**이며(운영자 결정 2026-09-04) `PRODUCT-SPEC.md` §9.7 · §14.5.1 · §21이 함께 갱신됐다
> - **2026-09-07 Phase 5.7 DONE** — 입력 레벨 경계 · 붕괴 판정 경계 · 그 둘의 화면 표현 반영
>   (§1 · §2 · §3 · §4 · §5 · §6 · §7 · §8 · §9). Phase 5.7도 **로드맵에 없던 삽입**이며
>   (운영자 결정 2026-09-07) **Phase 5.6이 부분 완료인 상태에서 그 앞으로 들어갔다.**
>   같은 갱신에서 **Phase 5.6의 실제 상태(TASK-066~071 DONE · TASK-072~075 TODO)** 를 적었다 —
>   이전 기록을 지운 것이 아니라 "PLANNED"였던 서술을 오늘의 사실로 바꿨다
> - **2026-09-07 Phase 5.6 engineering DONE** — 남은 넷(TASK-072 ~ TASK-075)이 Phase 5.7 뒤에
>   돌았다. AI Handoff 산출물의 크기와 무손실 분할 · 이 Phase의 네 불변 전용 테스트 ·
>   문서 마무리(ADR-0007 §19 · ADR-0009 §16 · ADR-0010 §12.8 ·
>   `docs/PHASE-5.6-HUMAN-REVIEW.md`)를 §1 · §2 · §3 · §4 · §5 · §6 · §7 · §8 · §9에 반영했다.
>   **위 Phase 5.7 줄과 그때의 "부분 완료" 서술은 지우지 않았다** — 이 Phase가 왜 두 번에 나눠
>   끝났는지가 그 기록에 남아 있다. **Human Review는 실행되지 않았다**

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
| **⚠️ 실제 추론은 두 번 실행됐고, 두 번 다 쓸 수 없었다** | 전사 경로 전체. 2026-09-05(언어 미설정 → 영어 강제)과 2026-09-07(입력 레벨 낮음 → 전 구간 환각)에 **실제 모델로 추론이 돌았다.** 엔진 경로는 지났으나 **사람이 읽을 수 있는 전사는 아직 한 번도 나오지 않았다** — `A-TRANS-001`은 **열려 있다** (§7 · `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록 · 부록 2) |
| **⚠️ 구현됐으나 실제 호출 미실행** | AI Note 경로 전체. 계약 · adapter · 화면이 자동 검증을 지나지만 **실제 Ollama에 요청을 보낸 적이 한 번도 없다** (`A-AI-001` · `docs/PHASE-4-AI-NOTE-REVIEW.md` §10.2) |
| **⚠️ 구현됐으나 실제 전송 미실행** | Notion sync 경로 전체. adapter · 분할 · SecretStore · 화면이 자동 검증을 지나지만 **실제 Notion 워크스페이스로 요청을 보낸 적이 없다** (`A-NOTION-001` · `docs/PHASE-5-NOTION-SMOKE-TEST.md` §10.1) |
| **지금 동작한다 (DONE · 자동 검증 기준) — Phase 5.5** | **AI Provider를 하나도 설정하지 않아도** Recording Detail에서 Manual 프롬프트와 Transcript 텍스트를 복사하고, AI-ready 문서를 `exports/`에 파일로 꺼낼 수 있다. 세 산출물은 결정론적 순수 함수의 결과이며 파일 쓰기까지 임시 디렉터리에서 실제로 검증된다. 화면은 타입·여백 스케일 · accent/상태색 · 보이는 focus · 공통 빈 상태 · 로딩 위에 선다 |
| **⚠️ 구현됐으나 실물 미확인 — Phase 5.5** | **clipboard 쓰기가 실제 webview에서 동작하는지 확인된 적이 없다** (ADR-0010 §7.4 · §12.4 — 여섯 항목 전부 UNVERIFIED). 자동 테스트는 언제나 test double을 쓴다. **사람이 산출물을 실제 외부 AI 채팅에 붙여 넣은 적도 없다.** clipboard가 거절돼도 Export for AI가 대체 경로로 남도록 설계돼 있다 (ADR-0010 §7.5) |
| **지금 동작한다 (DONE · 자동 검증 기준) — Phase 5.7** | **녹음 중에 입력 레벨이 화면에 보인다** — 파일에 쓰이는 것과 같은 통로에서 누적한 평균 RMS · 피크의 dBFS와 판정(쓸 만함 · 낮음 · 소리 없음)과 사람이 읽는 문장이 backend에서 만들어져 그대로 표시되고, 쓸 수 없을 만큼 낮으면 **정지 전에** 경고가 뜬다. 아직 재지 않은 것은 `null`이며 '낮음'이 아니다. **전사는 저장 직전에 붕괴 판정을 통과해야 한다** — 걸리면 Transcript를 추가하지 않고 `current_transcript_id`도 바꾸지 않은 채 `failed`가 되며, 화면이 무엇이 몇 번 반복됐는지와 다음에 할 일을 문장으로 보여준다 |
| **⚠️ 구현됐으나 실물 미확인 — Phase 5.7** | 위 둘 전부 **실제 마이크와 실제 전사에서 확인된 적이 없다.** 레벨은 가짜 `SampleSource`로, 붕괴 판정은 값으로 만든 segment 열로만 검증됐다. **"레벨 표시를 보고 사람이 실제로 마이크를 고칠 수 있는가" · "레벨을 올려 다시 녹음한 회의가 읽을 만하게 전사되는가"는 사람이 판정한다** (`docs/PHASE-5.7-HUMAN-REVIEW.md` — **기록표는 비어 있다**) |
| **지금 동작한다 (DONE · 자동 검증 기준) — Phase 5.6** | **전사 언어를 Settings에서 고를 수 있고, 고르지 않은 것이 '자동 감지'라는 정상 상태로 화면에 말해진다.** 그 선택이 엔진 경계까지 값으로 도달해 whisper.cpp의 기본값 `"en"`이 더 이상 조용히 쓰이지 않는다. `whisper-rs`의 `metal` feature가 켜져 빌드 산출물에 Metal 백엔드가 링크된다. **전사 한 건에 걸린 시간이 Transcript와 함께 저장되고** Transcript 탭에 문장으로 보인다(값이 없으면 그 줄이 없다). **내보낸 파일이 놓인 자리를 화면에서 열 수 있고**(경로 문자열은 그대로 남는다) 열리는 대상은 앱의 `exports/` 아래 파일로 제한된다. **긴 회의의 AI Handoff가 크기 때문에 조용히 실패하지 않는다** — 크기가 값으로 나오고, 예산을 넘으면 무손실로 순서대로 나뉘며, 각 조각이 자기 자리를 말한다 |
| **⚠️ 구현됐으나 사람이 판정하지 않음 — Phase 5.6** | **한국어 회의가 읽을 만하게 전사되는가 · `ggml-base`로 충분한가 · Metal 전후로 체감이 달라지는가 · 내보낸 파일을 실제로 열 수 있는가 · 긴 handoff를 실제 AI 채팅에 넣을 수 있는가** — 다섯 다 자동 Gate가 판정할 수 없다 (`docs/PHASE-5.6-HUMAN-REVIEW.md` — **기록표는 비어 있다**). **전사 속도는 이 저장소가 한 번도 측정한 적이 없고**(`ADR-0007` §19.2), `open -R` · `explorer /select,`는 실행된 적이 없으며(`ADR-0009` §16.2), handoff 예산 40,000 B는 **어떤 채팅의 확인된 한도도 아닌 이 앱이 고른 값이다**(`ADR-0010` §12.8.2) |
| **다음 단계 (PLANNED)** | Phase 6 — Cross-platform Validation & Hardening (Windows) |
| **미룬 것 (DEFERRED)** | **Cloud AI Providers (Claude · Gemini · Groq)** · search · tags · processing queue · menu bar (`PRODUCT-SPEC.md` §16) |
| **후보 (CANDIDATE)** | recording engine (§4) · whisper 통합 방식 (§4) · persistence crate (§4) · **전사 품질 후보 다섯과 입력 정규화 결정 하나** (§5의 Phase 5.7 절 — 청크 분할 · 청크마다 state 재생성 · 반복 차단 · VAD · 자동 언어 감지 개선 · P8 측정 뒤에 내릴 정규화 결정. **어느 것도 구현되지 않았다**) |

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
  ├─ 파일로 가는 통로(drain)의 샘플이 그대로 WAV로 쓰인다        ← 게인도 정규화도 없다
  └─ **같은 통로에서** 입력 레벨이 누적된다 (audio/level.rs)      ← 실시간 콜백이 아니다
        ↓  평균 RMS·피크의 dBFS · 판정 하나 · 사람이 읽는 문장   ← 오디오 샘플은 실리지 않는다
     status payload → 녹음 화면
        ↓  아직 잰 것이 없으면 값은 null이다 — '낮음'이 아니다
     낮음 · 소리 없음이면 **정지 전에** 경고가 보인다 (녹음을 막지도 멈추지도 않는다)
  ↓
Stop = 캡처 정지 → writer 확정 → 파일 존재·크기 확인 → Recording 영속화
  ↓  DB 실패 시 **audio를 지우지 않고** 경로를 담은 실패를 돌려준다 (INV-3 · INV-4)
Recordings 목록 → Recording Detail → 재생
                                   ↓ 파일이 없으면 레코드를 지우지 않고 알린다
  ↓
전사 (whisper-rs in-process · Metal · 배경 스레드)   ← 원본 오디오는 읽기만 한다
  ↓  시작할 때 설정에서 **모델과 언어를 한 번에** 읽는다 (commands/transcriber.rs)
  ↓    고른 언어 없음 → 감지를 켠다     고른 언어 있음 → 그 언어를 지정한다
  ↓    두 갈래 중 어느 쪽도 whisper.cpp의 기본값 "en"에 기대지 않는다
  ↓  걸린 시간을 단조 시계로 한 번 잰다 → 성공한 Transcript와 함께 저장된다
  ↓
**저장 직전 관문** — transcription/collapse.rs가 판정한다 (개수만 보지 않는다)
  ↓  빈 결과 · 붕괴 → Transcript를 만들지 않고 current도 그대로 둔 채 `failed`
  ↓                   실패 종류는 §13의 `TranscriptionOutputUnusable`이고,
  ↓                   문장에 무엇이 몇 번 반복됐는지가 수치와 함께 들어 있다
  ↓  쓸 수 있다     → Transcript를 **덧붙이고** current를 옮긴다
Detail의 Transcript 탭
  ↓  붕괴 실패는 별도 갈래로 보인다 — 다음에 할 일(레벨 확인 후 재녹음 · 다른 모델 · 재시도)과
  ↓  "아무것도 지워지지 않았다"가 함께 있다
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

━━━━━━━━━ Manual AI Handoff — provider가 하나도 없어도 여기는 열려 있다 (Phase 5.5) ━━━━━━━━━

AI Note 탭 (두 줄)
  ├─ 자동으로 만들기 : 연결된 provider로 생성            ← 위의 경로. 선택이다
  └─ 내 AI로 하기    : provider를 보지 않는다 (MH-1 · MH-2)
        ↓
   current Transcript (§7.2) ─→ export::ai_request (순수) ─┬─→ Manual 프롬프트 문자열
        ↑ 고르는 규칙은 export::run::current_input 하나     ├─→ Transcript 텍스트
                                                            └─→ AI-ready Markdown 문서
        ↓                                                          ↓
   platform/clipboard.ts (프론트 경계 하나)              export::file::write_new
        ↓  실패는 값으로 돌아온다 — 능력 없음 / 거절이 갈린다     ↓
   "복사됨" 또는 무엇이 실패했는지 + 재시도 + Export for AI   exports/…-ai-request.md
        ↓
   (사람이) 자기 AI 채팅에 붙여 넣거나 파일을 첨부한다   ← 앱은 여기서 아무것도 보내지 않는다 (MH-3)

━━━━━━━━━ 크기와 도달 — Phase 5.6이 위 두 갈래에 얹은 것 ━━━━━━━━━

세 산출물 ─→ export::portion (순수)  ─┬─→ 얼마나 큰가        (바이트 · 글자 · 줄)
                                       └─→ 예산을 넘으면      문단 → 줄 → 문장 → 낱말 → 글자
                                           순서대로 나눈다     순으로 자리를 찾는다
        ↓  이어 붙이면 원본이다 (한 글자도 잃지 않는다)
   화면이 크기와 "몇 번째 중 몇 번째"를 값으로 말한다 — 잘린 것을 온전하다고 말하는 상태가 없다
        ↓  파일로 갈 때는 조각마다 파일이고 이름이 그 자리를 말한다 (덮어쓰지 않는다)
   exports/…-ai-request-part-N-of-M.md
        ↓
   Show this file ─→ show_saved_file ─→ ① exports/ 아래인가  ② 파일인가  ─→ platform/file_manager
        ↑ 전체 경로 문자열은 그대로 남는다 — 여는 수단은 대체가 아니라 추가다
```

**이 갈래가 저장소에 쓰는 것은 없다** — `ai_notes` 행도 `promptVersion`도 만들지 않는다
(MH-7 · ADR-0010 §6.4). 파일시스템에 닿는 자리는 `write_new` 하나이며 기존 파일을 덮어쓰지
않는다. 채팅에서 받은 답을 앱으로 되돌리는 import 경로는 **없다.**

**아직 없는 것(PLANNED):** Windows 실동작 검증 (Phase 6).
*(2026-09-07 갱신 — 이 줄에 있던 "Phase 5.6의 남은 넷"은 그 뒤에 구현됐다 · §5)*

**이 Phase가 넣지 않은 것:** 마이크 게인 조정 · 오디오 정규화 · 청크 분할 · 청크마다 state
재생성 · 반복 차단 · VAD · 자동 언어 감지 개선. **전부 CANDIDATE이며 코드에 없다** (§5).
레벨 표시는 **알릴 뿐 녹음을 막지도 멈추지도 않고**, 붕괴 판정은 **저장을 막을 뿐 저장된
Transcript를 고치거나 지우지 않는다** (INV-2).

⚠️ 위 흐름 중 **AI 생성 · Notion 전송은 실제로 실행된 적이 없다** —
Ollama 호출(`A-AI-001`) · Notion 요청(`A-NOTION-001`)은 자동 검증에서 fixture와 stub으로만
지나간다. **전사는 2026-09-05과 2026-09-07에 두 번 실제로 실행됐으나 제품 경로가 쓸 수 있는
결과를 내지 못했다**(각각 언어 미설정 · 낮은 입력 레벨 · 위 상태 블록) — `A-TRANS-001`은
여전히 열려 있다. **위 흐름의 레벨 표시와 붕괴 관문도 실제 마이크·실제 전사에서 확인된 적이
없다** — 가짜 `SampleSource`와 값으로 만든 segment 열로만 검증됐다.
**Markdown 파일 export와 Manual AI Handoff의 Export for AI는 실제 파일 쓰기까지 자동 검증이
지나간다** — 임시 디렉터리에서 실물 파일을 만들고 내용을 확인한다. 이 두 경로에는 외부
의존이 없다. **다만 clipboard 쓰기는 그렇지 않다** — 자동 테스트는 언제나 test double을 쓰며,
실제 webview에서의 동작은 UNVERIFIED다 (ADR-0010 §12.4).
**Phase 5.6이 얹은 두 자리도 같은 성질이다** — 나눔과 크기는 순수 함수라 자동 검증이 끝까지
지나가지만, **OS 파일 관리자를 실제로 여는 것**(`open -R` · `explorer /select,`)은 자동 테스트가
double로 대신하고(부르면 검사가 도는 동안 창이 열린다) **실제로 열리는지는 UNVERIFIED다**
(ADR-0009 §16.2). **조각 하나가 실제 AI 채팅에 들어가는지도 마찬가지다** (ADR-0010 §12.8.2).

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
| **`export/ai_request.rs`** | **Manual AI Handoff의 순수 렌더러** — Manual 프롬프트 · Transcript 텍스트 · AI-ready 문서 셋을 만든다. 파일도 clipboard도 네트워크도 저장소도 시계도 없다. 프롬프트 상수는 **읽기만** 하고(치환 두 번), transcript·메타데이터 렌더링은 `markdown.rs`의 함수를 부른다 — 두 번째 렌더링 규칙이 없다 | **DONE** |
| **`export/handoff.rs`** | 그 셋을 저장소·파일시스템과 잇는 실행 순서. `current_transcript_id`가 가리키는 Transcript만 읽고(MH-5), 저장소에 쓰지 않으며, 파일은 `write_new` 하나로만 만든다 | **DONE** |
| **`commands/export.rs`의 세 이름** | `get_ai_prompt` · `get_transcript_text` · `export_ai_request`. 등록 command 28 → **31**. **`transcriptId`를 받지 않는다** — 옛 version을 고를 수단이 wire에 없다 (ADR-0010 §8.2) | **DONE** |
| **`src/platform/clipboard.ts`** | **webview의 clipboard 쓰기를 아는 유일한 자리** (INV-10). 새 의존성 · 새 command · 새 `FailureKind` 없이 `navigator.clipboard`를 한 줄에서 집는다. 실패는 던지지 않고 값으로 돌아오며 `unavailable`/`rejected`가 갈린다 | **DONE** (⚠️ 실제 webview 동작 UNVERIFIED · ADR-0010 §12.4) |
| **`screens/copyView.ts` · `screens/aiHandoffView.ts`** | 복사 한 번의 상태·표시와 AI Note 탭의 **두 줄**(자동으로 만들기 / 내 AI로 하기)을 정하는 순수 모듈. **입력 타입에 provider를 담을 자리가 없다** — provider 부재로 아래 줄이 막힐 수단 자체가 없다 (MH-1 · INV-8) | **DONE** |
| **UI 기반 (`src/App.css` · `EmptyState` · `Loading`)** | 타입 스케일 다섯 위계 · 여백 스케일 여섯 · accent/상태색 · 보이는 `:focus` · 공통 빈 상태와 로딩. 새 색 토큰은 dark 블록에도 정의된다. **radius(5px)는 이미 통일돼 있어 그대로 뒀다.** gradient · glassmorphism · 과한 그림자 · 장식 애니메이션은 테스트가 막는다 | **DONE** (⚠️ 실행 화면의 체감은 사람이 판정 · `docs/PHASE-5.5-HUMAN-REVIEW.md`) |
| **`audio/level.rs`** (Phase 5.7) | **입력 레벨의 계산과 판정이 사는 자리 하나** — i16 덩어리를 누적해 평균 RMS·피크를 dBFS로 옮기고, 판정 구간(`-36` / `-60` dBFS)과 **사람이 읽는 문장까지 여기서 만든다.** 장치도 파일도 스레드도 저장소도 모르고, **샘플을 바꾸는 함수가 없다.** 잰 것이 없으면 `None`이지 0이 아니다 | **DONE** (⚠️ 실제 마이크 미확인) |
| **`audio/capture.rs`의 레벨 갱신** (Phase 5.7) | 레벨을 **파일에 쓰이는 것과 같은 통로(`drain`)** 에서 갱신한다 — 실시간 오디오 콜백에서 계산하지 않는다. 일시정지 구간은 파일에도 레벨에도 도달하지 않는다. **캡처가 만드는 WAV의 샘플 값은 예전과 같다** | **DONE** |
| **`transcription/collapse.rs`** (Phase 5.7) | **붕괴 판정 규칙이 사는 자리 하나** — 정규화된 segment 열에서 문장 수 `n` · 고유 수 `u` · 최다 반복 `r`을 세고, `n < 20`은 판정하지 않으며 `u/n <= 0.20` 또는 `r/n >= 0.50`이면 붕괴다. **판정은 수치를 잃지 않는다.** 파일시스템 · DB · 네트워크 · 시계 · 엔진을 모른다 | **DONE** |
| **`transcription/run.rs`의 저장 직전 관문** (Phase 5.7) | 개수만 보던 자리(`segments.is_empty()`)에 그 판정을 더했다. **빈 결과 판정은 그대로 남아 있다.** 붕괴면 Transcript를 추가하지 않고 current도 옮기지 않은 채 `failed`가 되며, **새 실패 종류를 만들지 않고** 이미 있는 `TranscriptionOutputUnusable`을 쓴다. `run.rs`는 임계값도 비율도 스스로 계산하지 않는다 | **DONE** |
| **`commands`의 `SessionStatusPayload.level` · `src/ipc/types.ts`의 `InputLevel`** (Phase 5.7) | 수치 둘 · 판정 이름 하나 · 문장 하나만 나간다. **오디오 샘플을 담을 자리가 없고**(INV-6) 새 벤더 고유 개념도 없다(INV-9). 진행 중인 녹음이 없으면 `null`이다 | **DONE** |
| **`screens/recordingView.ts`의 레벨 표시** (Phase 5.7) | `inputLevelDisplay`(값 없음 · 낮음 · 소리 없음 · 쓸 만함)와 `inputLevelWarning`(녹음 중에만, 약할 때만). **dBFS 계산도 임계값도 여기에 없다** — backend가 준 값과 문장을 그대로 쓴다. `RecordingScreen.tsx`는 그리기만 한다 | **DONE** (⚠️ 실행 화면 미확인) |
| **`screens/transcriptView.ts`의 붕괴 갈래** (Phase 5.7) | `transcriptionOutputUnusable`이 `other`로 뭉개지지 않고 별도 원인이 된다. 그 갈래에 **다음에 할 일**(레벨 확인 후 재녹음 · 다른 모델 · 재시도)이 문장으로 있고, 재시도 수단과 `TRANSCRIPTION_PRESERVED_NOTICE`가 함께 남는다 | **DONE** |
| **`tests/level-and-collapse-boundary.test.ts`** (Phase 5.7) | 이 Phase의 다섯 불변을 저장소 원문으로 판정한다 — 레벨 경로에 오디오 샘플이 없다 · 붕괴 규칙이 한 자리에만 있다 · 화면에 dBFS 계산과 임계값이 없다 · 캡처가 샘플을 바꾸지 않는다 · 새 벤더 고유 개념이 없다 | **DONE** |
| **전사 언어 설정** (Phase 5.6) | `settings.transcription_language`(migration 9 · nullable) → `Settings` → payload → `settingsView.ts` → Settings 화면. **NULL은 '아직 고르지 않았다 = 자동 감지'라는 정상 상태**이며, 기본값 정책은 스키마가 아니라 `Settings::DEFAULT`가 갖는다. **앱이 사용자 로캘을 짐작해 언어를 굳혀 두지 않는다** | **DONE** |
| **`transcription::engine::LanguageChoice`** (Phase 5.6) | 무슨 언어로 들을지에 대한 **선택 하나**. 설정을 읽는 자리는 `commands/transcriber.rs` 하나(모델과 **같은 자리에서 한 번에** 읽는다), 엔진 호출로 옮기는 자리는 `transcription/whisper.rs` 하나다. 그 사이의 모듈은 값을 나르기만 하고 해석하지 않는다. **`Transcript.language`는 여전히 엔진이 보고한 값이며 설정 값을 베껴 넣지 않는다** | **DONE** (⚠️ 한국어 품질은 사람이 판정 · `docs/PHASE-5.6-HUMAN-REVIEW.md`) |
| **`whisper-rs`의 `metal` feature** (Phase 5.6) | Spec §14.4가 적은 대로 Metal을 켠다. **판정은 manifest 기재가 아니라 빌드 산출물이다** — `GGML_METAL` OFF→ON · `libggml-metal.a` 생성 · `Metal`/`MetalKit` 링크 지시 · Gate가 실행한 바이너리의 `ggml_metal_*` 심볼. CoreML · OpenMP는 켜지 않았다 | **DONE** (⚠️ **속도는 측정한 적이 없다** · 런타임 GPU 사용은 UNVERIFIED · `ADR-0007` §19.2) |
| **`transcripts.transcription_ms`** (Phase 5.6) | 전사 한 건에 걸린 시간. 재는 자리는 `transcription/run.rs`의 `Instant` 하나(단조 시계)이고 재는 구간은 모델 해석 → 오디오 읽기 → 엔진 → 정규화까지다. **사람이 읽는 문장은 Rust가 만든다**(`transcriptionLabel`). 값이 없는 옛 Transcript는 NULL이며 **화면이 그 줄을 그리지 않는다 — "0:00"이라고 말하지 않는다** | **DONE** |
| **`platform/file_manager.rs` · `commands/saved_file.rs`** (Phase 5.6) | 내보낸 파일이 **놓인 자리**를 여는 경계. **OS를 부르는 코드는 `file_manager.rs` 안에만 있고**(INV-10), 허용을 판정하는 자리는 `SavedFiles::show` 하나다 — 정규화된 경로가 앱의 `exports/` 아래의 **파일**일 때만 열린다(`..`·symlink·디렉터리·빈 경로는 거절). **파일을 만들지도 고치지도 지우지도 않고** 디렉터리도 만들지 않는다. 새 의존성도 새 capability 권한도 쓰지 않았다 | **DONE** (⚠️ `open -R` · `explorer /select,`가 실제로 여는지는 UNVERIFIED · `ADR-0009` §16.2) |
| **`export/portion.rs`** (Phase 5.6) | **AI Handoff 산출물의 크기와 나눔이 사는 자리 하나** — `measure`(바이트 · 글자 · 줄)와 `split`(문단 → 줄 → 문장 → 낱말 → 글자). **이어 붙이면 원본이다**(조각은 전부 입력의 부분 슬라이스이고 자리는 값이지 본문 표식이 아니다). 예산 `PORTION_MAX_BYTES = 40_000`은 **이 앱이 고른 값이지 벤더 제약이 아니다** — Notion의 `CHUNK_MAX_BYTES`에서 오지 않았다는 것을 테스트가 못박는다. 파일시스템 · 저장소 · 네트워크 · 시계를 모른다 | **DONE** (⚠️ 실제 채팅의 한도는 UNVERIFIED · `ADR-0010` §12.8.2) |
| **`markdown.rs`의 `TranscriptShape`** (Phase 5.6) | segment를 적는 **모양** 둘 — §11의 `### HH:MM:SS` 제목(`Sectioned`)과 AI Handoff의 `HH:MM:SS 문장` 한 줄(`Compact`). **어느 segment를 · 어떤 순서로 · 없으면 무엇으로 대체하는가는 모양과 무관하게 같은 함수에서 온다**(규칙이 복제되지 않았다). **`render`는 언제나 `Sectioned`이며 §11 export 파일 형식은 바뀌지 않았다** — Phase 5의 golden 테스트가 고쳐지지 않은 채 통과한다 | **DONE** |
| **`commands/export.rs`의 세 이름의 인자·응답** (Phase 5.6) | `get_ai_prompt` · `get_transcript_text` · `export_ai_request`가 `portion` 인자를 받고 **전체 크기와 조각의 자리**를 함께 돌려준다. **command 이름은 늘지 않았다**(ADR-0010 §8.1). 셋 다 여전히 `transcriptId`를 받지 않고(MH-5) 벤더 고유 개념도 없다(INV-9). 없는 조각을 달라는 요청은 빈 값이 아니라 실패다 | **DONE** |
| **`screens/copyView.ts` · `aiHandoffView.ts`의 크기·조각 표현** (Phase 5.6) | 얼마나 큰가(`handoffSize`)와 몇 번째 중 몇 번째인가(`portionTaken`)를 값으로 말한다. 두 모듈이 **같은 두 함수를 쓴다**(규칙이 두 벌이 되지 않는다). **잘린 결과를 온전한 것처럼 말하는 상태가 없고**, 나머지를 마저 가져가는 수단이 값으로 있다 | **DONE** (⚠️ 실제 채팅에서의 쓸모는 사람이 판정) |
| **`screens/savedFileView.ts`** (Phase 5.6) | 여는 동작과 그 실패의 순수 규칙. **어느 OS의 파일 관리자인지 말하지 않으며**(INV-10), 실패했을 때 *"전체 경로는 그대로 위에 있다"* 가 함께 있다 — 여는 수단은 경로의 대체가 아니라 추가다 | **DONE** |
| **`src-tauri/tests/transcription_and_reach_invariants.rs` · `tests/transcription-and-reach-invariants.test.ts`** (Phase 5.6) | 이 Phase의 네 불변 전용 테스트 — 언어가 영어로 강제되지 않는다 · §D의 language 설정이 실재하고 왕복한다 · 내보낸 파일에 도달하는 수단이 있고 exports 밖을 열지 못한다 · 긴 handoff가 크기 때문에 조용히 실패하지 않는다. **판정할 수 없는 것을 판정하는 척하지 않는다** — 한국어 품질과 Metal 체감은 사람의 몫이라고 테스트가 직접 적는다 | **DONE** |
| Windows 지원 검증 | §3.1 핵심 기능의 Windows 실동작 | **PLANNED** (Phase 6) |

---

## 4. External Dependency Boundary

| 구분 | 항목 | 비고 |
| --- | --- | --- |
| **선택됨 · 현재 사용 중** | Tauri v2 (2.11.5) · React 19 · Vite 7 · TypeScript 5.8 · ESLint · Vitest<br>**`rusqlite` 0.40.2 (`bundled`, SQLite 3.53.2)** — 제품 경로에서 실제로 쓰인다 | persistence 선택 근거는 `docs/ADR-0001-local-persistence.md` |
| **선택됨 · 추론은 돌았으나 쓸 수 있는 결과가 없다** | **`whisper-rs` 0.16 + `rubato` 5** — 제품 전사 경로에서 쓰인다. **Phase 5.6이 `whisper-rs`의 `metal` feature를 켰다** (TASK-069 · 새 crate를 들여오지 않는다) | ⚠️ 실제 모델로 두 번(2026-09-05 · 2026-09-07) 추론했으나 **읽을 만한 전사가 나온 적이 없다** (`A-TRANS-001`) |
| **선택됨 · 실제 호출 미실행** | **`ureq` 3.4.0 (`default-features = false, features = ["rustls"]`)** — 로컬 Ollama REST(`ai/ollama/network.rs`)와 Notion HTTPS(`notion/network.rs`)를 부르는 자리에서 쓰인다 | ⚠️ 실제 서버에 요청을 보낸 적이 없다 (`A-AI-001` · `A-NOTION-001`). **Phase 5가 `rustls`를 명시적으로 켰다** — ADR-0008 §12.2가 예고한 대로 HTTPS가 실제로 필요해진 시점이다 |
| **선택됨 · 실제 저장소 미접근** | **`keyring` 3.6.3 (`apple-native` + `windows-native`)** — Notion integration token을 담는 유일한 자리(`platform/secret_store.rs`) | ⚠️ 자동 테스트는 메모리 double만 쓰며 실제 OS 자격증명 저장소를 건드리지 않는다. feature 전체 목록은 **UNVERIFIED** — 확인된 것은 두 feature 이름이 실재하고 플랫폼 API가 들어왔다는 것까지다 (`ADR-0009` §15.2.4) |
| **선택됨 · 사용 중** | **`sha2` 0.10** — export 경로에서 쓰인다 | `Cargo.lock`이 고정한다 |
| **선택됨 · 실제로 나갔다 ✅** | **이 기기의 `claude` CLI** — `ai/claude_cli`가 프로세스로 부른다. 실행은 `platform/command_runner.rs` 하나에만 있다 (INV-10) | **2026-09-10에 실제로 노트를 만들었다.** 앱은 자격증명을 만지지 않는다 — 인증은 CLI가 자기 방식으로 한다. 서드파티가 구독 OAuth 토큰을 대신 쓰는 것은 Anthropic이 금지하며(2026-04-04 시행), 이 경로는 그것이 아니다 |
| **선택됨 · 번들** | **`wanted-sans` 1.0.3** (글꼴 · OFL 1.1) · **`lucide-react`** (아이콘) | 둘 다 빌드 시점에 앱 안으로 들어온다. **외부로 요청을 보내지 않는다** — 오프라인에서 그대로 돈다 (`src/fonts/LICENSE.md`) |
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

**Phase 5.5는 의존성을 하나도 더하지 않았다** — `package.json`도 `Cargo.toml`도 바뀌지
않았고, 새 Tauri 권한 항목도 없다. clipboard는 **플랫폼 능력**이지 라이브러리가 아니며,
webview가 이미 갖고 있(을 것으로 기대되)는 것을 경계 하나 뒤에서 부른다. Tauri clipboard
플러그인은 **확인하지 못한 세 값**(crate/npm 이름 · 호환 버전 · permission 식별자) 위에
의존성을 얹는 선택이라 탈락했다 — 필요가 증명되면 그때 얹는다 (ADR-0010 §7.3).
**Manual AI Handoff는 새 외부 경계를 만들지 않는다** — 나가는 행위의 주체가 사람이므로
§12의 세 단계 구분(완전 로컬 · 로컬 AI · 외부)이 그대로다.

**Phase 5.7도 의존성을 하나도 더하지 않았다** — `package.json`도 `Cargo.toml`도 바뀌지 않았고
새 Tauri 권한 항목도 없다. 입력 레벨은 **이미 캡처 경로를 지나는 샘플에서 산술로만** 만들어지고,
붕괴 판정은 **이미 파싱이 끝난 segment 열에서 세기만** 한다. 둘 다 **완전 로컬** 단계 안이며
기기 밖으로 나가는 통로가 없다.

**Phase 5.6이 `Cargo.toml`에서 바꾼 것은 한 줄이다** — `whisper-rs`에 `metal` feature를 켠 것.
**새 crate는 하나도 들어오지 않았고 `Cargo.lock`도 바뀌지 않았다** (이 feature는 새 패키지를
끌어오지 않는 빌드 스위치이며, 그래서 판정을 lock이 아니라 빌드 산출물에 걸었다 —
`ADR-0007` §19.2). **내보낸 파일의 자리를 여는 경계도 의존성을 더하지 않았다** — Tauri 플러그인
없이 표준 라이브러리의 프로세스 실행 하나를 쓰고, `src-tauri/capabilities/default.json`의
permissions는 `["core:default"]` 그대로다 (`ADR-0009` §16.2). AI Handoff의 크기와 나눔은
순수 Rust 산술이다. **이 Phase도 새 외부 경계를 만들지 않았다** — §12의 세 단계 구분이 그대로다.

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

### Phase 5.5 — Manual AI Handoff + UI Foundation · 2026-09-06 · **engineering DONE · Human Review 미실행**

**로드맵에 없던 삽입이다** (운영자 결정 · 2026-09-04). Phase 5와 Phase 6 사이에 들어갔고,
**두 가지 제품 방향 변경**을 담는다 — Manual AI Handoff를 정식 제품 기능으로 **추가**하고,
로컬 Ollama provider를 '첫 provider · 기본 AI 경험'에서 '선택적 · 로컬 · 고급'으로
**재배치**한다. **재배치는 삭제가 아니다** — Phase 4가 만든 provider 추상화 · adapter ·
설정 · 연결 확인 · 모델 선택은 **그대로 있다** (MH-8).

Task 11개(TASK-055 ~ TASK-065).

| Task | 결과물 |
| --- | --- |
| TASK-055 | `ADR-0010` — 두 방향 변경의 기록 · 산출물 형식 · 프롬프트 재사용 · clipboard 경계 · command 이름과 개수 · MH-1~MH-8의 판정 수단 |
| TASK-056 | AI-ready 산출물과 Manual 프롬프트를 만드는 **순수 모듈** (`export/ai_request.rs`) |
| TASK-057 | 세 command 경계와 IPC 계약 (`export/handoff.rs` · `commands/export.rs`) |
| TASK-058 | clipboard capability 경계 하나와 복사 상태의 순수 판정 (`src/platform/clipboard.ts` · `copyView.ts`) |
| TASK-059 | AI Note 탭의 위계 변경 — 자동 생성과 Manual Handoff **두 줄** (`aiHandoffView.ts`) |
| TASK-060 | Settings에서 로컬 provider를 '선택적 · 로컬 · 고급'으로 재배치 |
| TASK-061 | UI 기반 — 타입 · 여백 · accent · 상태 · focus 토큰과 최소 primitive |
| TASK-062 | 네 주요 화면을 그 기반 위에 올림 |
| TASK-063 | MH-1~MH-8 전용 자동 테스트 |
| TASK-064 | UI 기반 불변 전용 자동 테스트 |
| TASK-065 | Spec 방향 변경 반영 · ADR 확정 · SYSTEM-MAP · Human Review 절차 문서 |

핵심 성질:

```text
provider를 담을 자리가 없다  — Manual 경로의 Rust 모듈에도 순수 view 모듈의 입력 타입에도
                               provider가 없다. 거절할 수단 자체가 없다 (MH-1 · MH-2 · INV-8)
프롬프트 상수를 고치지 않았다 — 상수 원문에 치환 두 번만 적용한다. PROMPT_VERSION_* 선언값
                               셋이 그대로이고 이미 저장된 provenance가 거짓이 되지 않는다
렌더링 규칙이 한 벌이다      — transcript 본문 · 메타데이터 블록 · 제목 한 줄은 markdown.rs
                               한 자리에 있고, `render`가 내는 바이트는 바뀌지 않았다
clipboard를 부르는 자리가 하나 — src/platform/clipboard.ts. 새 의존성 · 새 command ·
                               새 FailureKind 없이. 실패는 값으로 돌아온다
파일은 덮어쓰지 않는다        — exports/…-ai-request.md. 이름이 겹치면 -2가 붙는다
저장소에 쓰지 않는다          — ai_notes 행도 promptVersion도 만들지 않는다 (MH-7)
```

검증: build/lint/test Gate green · **자동 테스트 1,238개** (vitest 494 · Rust 744).
Phase 종료 Gate는 **저장소 전체**에 세 개를 모두 돌렸다 (2026-09-04 운영자 결정).

**⚠️ IMPLEMENTED / VERIFIED로 기록하지 않는 것:**

```text
실제 webview에서 clipboard 쓰기가 동작한다        ← NOT RUN (ADR-0010 §7.4의 여섯 항목 전부)
거절될 때 어떤 예외/값이 오는가                    ← UNVERIFIED
사람이 산출물을 실제 외부 AI 채팅에 붙여 넣었다    ← NOT RUN
받아 온 답이 실제로 쓸 만한가                      ← NOT RUN
화면이 실제로 차분하고 읽기 쉬운가                 ← 사람이 판정한다
로컬 provider가 '선택적 · 고급'으로 읽히는가       ← 사람이 판정한다
Windows에서의 clipboard · 화면                     ← Phase 6
```

절차와 **빈 기록표**는 `docs/PHASE-5.5-HUMAN-REVIEW.md`.
**`A-` deferred assumption은 더하지 않았다** (D-4) — 위 셋은 주관적 Human Review 항목이지
`A-REC-001` 계열의 hard 가정이 아니다. 구현 대조는 `ADR-0010` §12.

이 Phase의 입력도 전부 fixture였다 — `A-TRANS-001` · `A-AI-001`이 여전히 유효하기 때문이다.

### Phase 5.6 — Transcription Correctness & Reach · 2026-09-07 · **engineering DONE · Human Review 미실행**

**로드맵에 없던 삽입이다** (운영자 결정 · 2026-09-05). 첫 실제 전사 실행이 드러낸 결함
(언어 미설정 · Metal 미사용 · 내보낸 파일에 도달 못 함 · 긴 handoff가 채팅에 안 들어감)을 메운다.
**새 제품 방향이 아니라 이미 Spec에 있던 것이 구현되지 않은 자리다** — 그래서 새 ADR을 쓰지 않고
기존 ADR을 갱신했다. Goal은 `phase-prompt/05.6-transcription-correctness-and-reach.md`,
실측 기록은 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록.

**이 Phase는 두 번에 나눠 끝났다.** TASK-066 ~ TASK-071이 먼저 돌았고, 2026-09-07의 두 번째
실사용이 더 급한 결함을 드러내 **Phase 5.7이 그 앞으로 들어갔으며**, 남은 넷(TASK-072 ~
TASK-075)이 그 뒤에 돌았다. **그 순서 자체가 기록이다** — 위의 갱신 이력과 Phase 5.7 절이
그것을 그대로 갖고 있다.

Task 10개(TASK-066 ~ TASK-075).

| Task | 결과물 |
| --- | --- |
| TASK-066 | `ADR-0007` §17 — 언어 처리 결정과 Metal 결정 |
| TASK-067 | 전사 언어 설정을 저장소부터 화면까지 (migration 9) |
| TASK-068 | 전사가 설정된 언어를 실제로 쓴다 — 영어 강제 제거 (`LanguageChoice`) |
| TASK-069 | `whisper-rs`의 `metal` feature — **판정은 빌드 산출물로** |
| TASK-070 | `transcripts.transcription_ms` — 걸린 시간 기록 (migration 10) |
| TASK-071 | 내보낸 파일이 있는 자리를 화면에서 연다 (`platform/file_manager.rs` · `commands/saved_file.rs`) |
| TASK-072 | AI Handoff 산출물의 크기와 순서대로 나누기 (순수 모듈 `export/portion.rs`) |
| TASK-073 | 그 크기와 나눔을 command 경계와 AI Note 탭까지 |
| TASK-074 | Phase 5.6 네 불변 전용 테스트 |
| TASK-075 | ADR 갱신(`ADR-0007` §19 · `ADR-0009` §16 · `ADR-0010` §12.8) · Human Review 절차 · SYSTEM-MAP |

핵심 성질:

```text
고르지 않음은 영어가 아니다   — 설정이 비면 감지를 켠다. 언어 파라미터를 건드리지 않는 코드
                                경로가 남지 않았다. whisper.cpp의 기본값 "en"이 조용히 쓰이던
                                자리가 사라졌다
읽는 자리 하나 · 옮기는 자리 하나 — 설정은 commands/transcriber.rs 에서 모델과 함께 한 번에
                                읽히고, 엔진 호출로 옮기는 것은 transcription/whisper.rs 하나다.
                                그 사이 모듈은 값을 나르기만 한다 (§13의 교체 경계가 그대로다)
결과는 여전히 엔진이 말한 값   — Transcript.language 에 설정 값을 베껴 넣지 않는다.
                                엔진이 말하지 못하면 비어 있다
켜졌다는 판정은 산출물로       — metal feature 를 켠 뒤 GGML_METAL 이 ON 이 되고
                                libggml-metal.a 가 생기고 링크된 바이너리에 심볼이 있다.
                                **manifest 에 이름이 적혔다는 것으로 판정하지 않았다**
모르는 것을 0이라고 하지 않는다 — 전사 소요 시간은 nullable 이고 DEFAULT 가 없다.
                                값이 없는 옛 Transcript 는 화면에서 그 줄이 아예 없다
여는 쪽이 범위를 정한다        — webview 가 임의 경로를 열 수 없다. 정규화된 경로가 앱의
                                exports/ 아래의 파일일 때만 열리고, 그 판정은 한 자리에 있다
경로는 사라지지 않았다        — 여는 수단은 대체가 아니라 추가다. 실패해도 전체 경로가 남는다
나눠도 잃지 않는다            — 조각을 이어 붙이면 원본이다. 자리는 값이지 본문 표식이 아니다
예산은 앱이 고른 값이다        — 40,000 B 는 어떤 채팅의 확인된 한도도 아니고 Notion 의
                                벤더 제약에서 오지도 않았다. 테스트가 그 독립성을 못박는다
§11 형식은 바뀌지 않았다      — AI 경로만 압축 모양을 고른다. render 는 언제나 Sectioned 이고
                                Phase 5 의 golden 테스트가 고쳐지지 않은 채 통과한다
```

검증: 이 Phase의 마지막 엔지니어링 Task(TASK-074)를 돌릴 때 **Runtime이 직접 실행한**
`build` · `lint` · `test` Gate가 셋 다 PASS · exit 0이고, 그 `test` 로그가
**자동 테스트 1,471개** (vitest 586 · Rust 885)를 적는다 — 원본은 Runtime 소유 경로
`.loop-local/runs/RUN-20260907T062848Z-TASK-074/`(`gate-report.json` · `gates/test/stdout.log`).
**마무리 Task(TASK-075)는 문서 전용이고 Gate를 선언하지 않았다** — 그 뒤로 바뀐 것이 `docs/`
아래뿐이므로 이 수치가 그대로 선다.

**⚠️ IMPLEMENTED / VERIFIED로 기록하지 않는 것:**

```text
한국어 회의가 읽을 만하게 전사된다             ← NOT RUN (사람이 판정한다)
ggml-base 로 충분한가 / 더 큰 모델이 필요한가   ← NOT RUN (A-TRANS-001 의 실질적 답)
Metal 을 켜서 전사가 빨라진다                  ← **측정한 적이 없다.** before 값이 저장소에 없다
런타임에 GPU 가 실제로 잡힌다                  ← UNVERIFIED (확인된 것은 바이너리에 링크됐다까지)
open -R · explorer /select, 가 그 창을 연다     ← NOT RUN (자동 테스트는 double 을 쓴다)
예산 40,000 B 가 실제 AI 채팅에서 맞는 크기다   ← UNVERIFIED (확인된 한도가 아니다)
조각을 순서대로 넣는 것이 사람에게 쓸 만하다    ← NOT RUN (사람이 판정한다)
whisper-rs 0.16.0 의 [features] 를 crate 소스에서 읽었다 ← 읽지 못했다. cargo 가 파싱해 남긴
                                                 declared_features 를 읽었다 (ADR-0007 §19.2)
```

절차와 **빈 기록표**는 `docs/PHASE-5.6-HUMAN-REVIEW.md`.
**`A-` deferred assumption을 더하지 않았다** — 이 Phase가 남긴 다섯은 주관적 Human Review
항목이며, `A-TRANS-001`은 **새로 만든 것이 아니라 계속 열려 있던 것**이다 (§7).
구현 대조는 `ADR-0007` §19 · `ADR-0009` §16 · `ADR-0010` §12.7 · §12.8.

**성공 기준 1(언어)은 실사용으로 한 번 확인됐다** — 2026-09-07의 두 번째 실행에서 설정한 `ko`가
엔진에 도달했고 `auto-detected language: en` 줄이 사라졌다 (`phase-prompt/05.7` R-1).
**그 확인이 "한국어가 읽을 만하게 전사됐다"는 뜻은 아니다** — 그 실행의 결과는 다른 이유로
붕괴했고(§5의 Phase 5.7 절), 품질 판정은 여전히 사람의 몫이다.

### Phase 5.7 — Recording Level + Transcription Collapse · 2026-09-07 · **engineering DONE · Human Review 미실행**

**로드맵에 없던 삽입이다** (운영자 결정 · 2026-09-07). Phase 5.6이 부분 완료인 상태에서 그 앞으로
들어갔고, **새 제품 방향이 아니다** — 2026-09-07 운영자의 두 번째 실사용이 드러낸 두 침묵을 메운다:
녹음 중에는 소리가 담기지 않는다는 것을 말해 주지 않았고, 전사 뒤에는 붕괴한 결과를 완료라고 말했다.

Task 9개(TASK-076 ~ TASK-084).

| Task | 결과물 |
| --- | --- |
| TASK-076 | `ADR-0007` §18(붕괴 판정 규칙) · `ADR-0003` §16(입력 레벨 경계) — 규칙과 임계값을 값으로 확정 |
| TASK-077 | 붕괴 판정 순수 모듈 (`transcription/collapse.rs`) |
| TASK-078 | 저장 직전 관문 — 붕괴한 전사를 `done`으로 저장하지 않는다 (`transcription/run.rs`) |
| TASK-079 | 붕괴 실패가 화면에서 별도 갈래로 읽힌다 (`screens/transcriptView.ts`) |
| TASK-080 | 입력 레벨 순수 모듈 (`audio/level.rs`) |
| TASK-081 | 파일에 쓰이는 통로에서 레벨을 갱신하고 status payload로 내보낸다 (`audio/capture.rs` · `commands`) |
| TASK-082 | 녹음 화면의 레벨 표시와 **정지 전** 경고 (`ipc/types.ts` · `screens/recordingView.ts` · `RecordingScreen.tsx`) |
| TASK-083 | 성공 기준 3의 측정 절차 — `PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록 2 |
| TASK-084 | 이 Phase의 다섯 불변 전용 테스트 (`tests/level-and-collapse-boundary.test.ts`) |

핵심 성질:

```text
레벨은 파일과 같은 통로에서 잰다  — 실시간 오디오 콜백이 아니라 drain이다. 일시정지 구간은
                                    파일에도 레벨에도 도달하지 않는다
샘플은 바뀌지 않는다              — 게인도 정규화도 없다. 들어온 샘플이 그대로 WAV로 쓰인다
모르는 것을 낮다고 말하지 않는다   — 잰 것이 없으면 null이고, 화면의 'unknown'은 판정이 아니다
판정 규칙이 한 자리다             — dBFS 환산·임계값은 level.rs, 붕괴 규칙은 collapse.rs.
                                    화면에도 run.rs에도 엔진에도 복제되지 않았다
저장을 막을 뿐 고치지 않는다      — 붕괴 판정은 저장 직전에 있고, 저장된 Transcript를
                                    지우거나 고치는 경로는 생기지 않았다 (INV-2)
새 실패 종류를 만들지 않았다      — 이미 있는 TranscriptionOutputUnusable을 쓴다 (§13)
원인을 단정하지 않는다            — 같은 붕괴가 낮은 레벨·잘못된 언어·부족한 모델 어디서도
                                    나오므로, 화면은 하나를 지목하지 않고 할 수 있는 일을 늘어놓는다
```

검증: test Gate green · **자동 테스트 1,405개** (vitest 571 · Rust 834).

**⚠️ IMPLEMENTED / VERIFIED로 기록하지 않는 것:**

```text
실제 마이크에서 레벨 표시가 맞는 값을 보인다        ← NOT RUN (가짜 SampleSource로만 검증)
사람이 그 표시를 보고 마이크나 자리를 고칠 수 있다   ← 사람이 판정한다
붕괴 판정이 실제 whisper 출력에서 옳게 걸린다        ← NOT RUN (값으로 만든 segment 열로만 검증)
임계값 셋(-36 dBFS · u/n 0.20 · r/n 0.50)이 옳은 값인가 ← 관측 두셋에서 고른 값이다. UNVERIFIED
레벨을 올리면 9/7 오디오가 구제되는가               ← **[미측정]** (성공 기준 3 · 아래)
레벨을 올려 다시 녹음한 회의가 읽을 만하게 전사되는가 ← NOT RUN (A-TRANS-001의 실질적 답)
```

**성공 기준 3은 절차까지만 왔다.** `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` **부록 2**가
원본을 읽기만 하고 레벨을 올린 사본을 만드는 절차 · 같은 조건으로 전사하는 절차 · 재는 방법 ·
9/7 원본값과 나란히 놓는 비교표를 확정했으나, **오른쪽 열은 전부 `[미측정]`이다** (부록2-4 ·
막은 것은 부록2-5). **그러므로 "레벨을 올리면 구제된다"고도 "구제되지 않는다"고도 적지 않는다.**

절차와 **빈 기록표**는 `docs/PHASE-5.7-HUMAN-REVIEW.md`.
**`A-` deferred assumption을 더하지 않았다** — 이 Phase가 남긴 넷은 주관적 Human Review 항목이며,
`A-TRANS-001`은 **새로 만든 것이 아니라 계속 열려 있던 것**이다 (§7).

#### 다음 Phase 후보 — **어느 것도 이 Phase에서 구현되지 않았다 (CANDIDATE)**

**아래는 계획도 결정도 아니다.** 코드에 없고, 이 표가 그것들을 DONE으로 만들지 않는다.

| # | 후보 | 왜 이 Phase에서 제외됐는가 (한 줄) |
| --- | --- | --- |
| **C-1** | **청크 분할** — 긴 오디오를 나눠 전사한다 | 9/4 실험에서 효과를 본 처방이지만 **이번 실패의 원인이 그것이 아니며**(R-3), 원인을 확인하기 전에 옮겨 오지 않기로 했다 |
| **C-2** | **청크마다 state 재생성** — 윈도우 사이로 환각이 번지는 것을 끊는다 | C-1과 같은 9/4 실험의 처방이고 **청크 분할 없이는 성립하지 않는다** — C-1이 정해지기 전에는 정할 것이 없다 |
| **C-3** | **반복 차단** — 같은 문장이 이어지면 디코딩을 멈춘다 | 같은 9/4 처방이며, **이 Phase는 반복을 막는 대신 반복을 판정해 저장을 막는 쪽**을 골랐다 (ADR-0007 §18) — 둘 다 하기 전에 붕괴 판정이 실사용에서 옳은지를 먼저 본다 |
| **C-4** | **VAD (음성 구간 검출)** | 무음 구간 환각을 줄이는 수단이지만 **2026-09-07의 붕괴는 무음 구간이 아니라 전 구간**에서 일어났다 — 이 실패를 설명하지 못한다 (9/5 부록 결과 5의 [미검증]이 그대로다) |
| **C-5** | **자동 언어 감지 개선** — 확신도 0.478의 `en` 오판 | 감지가 이 오디오에서 실패한 것은 기록해 뒀지만, **설정으로 언어를 지정하는 경로가 동작하므로 사람에게 막힌 길이 아니다** (Phase 5.6 R-1) |
| **C-6** | **입력 정규화를 제품에 넣는가** — 전사 직전에 레벨을 올려 넣을 것인가 | **측정 대기.** Phase Goal이 *"측정 전에 정규화 기능을 구현하지 않는다"* 로 못박았고, 그 측정(부록 2)의 결과 칸이 아직 `[미측정]`이다 |

C-6과 **녹음 시점의 마이크 게인을 앱이 조정할 것인가**는 서로 다른 결정이다 — 후자는 녹음 엔진
경계의 문제이며 `ADR-0003` §16이 이 Phase의 범위 밖으로 보류했다.

### Phase 6 — Cross-platform Validation & Hardening · **PLANNED**
---

### 2026-09-08 — **Runtime 밖의 하루** (Phase 아님)

⚠️ **이것은 Phase가 아니다.** Phase 5.9의 Plan(`PLAN-20260908T011945Z` · 10 Task)은
만들어졌으나 **승인되지 않았고**, 운영자가 직접 구현으로 전환했다. 아래는 그날 들어온
것이며, **Gate 셋을 지났고 전용 테스트와 ADR 기록이 붙어 있으나 Verifier를 지나지 않았다.**

무엇이 생겼는가:

```text
전사 품질
  hallucination.rs   창을 채운 상투구 제거 (20초 · 1.0 자/초)   ADR-0007 §22.3~22.4
  gain.rs            전사 입력만 증폭 (목표 -23 dBFS)          ADR-0003 §16.8
  vad.rs             Silero VAD — **효과를 보인 적이 없다**     ADR-0007 §22.2
  raw_text 수정      AI 노트가 차단 안 된 텍스트를 받던 것       ADR-0007 §23.4
  live.rs            녹음 중 전사의 창 규칙 — **2026-09-14에 연결됐다**

AI
  ai/anthropic/      NoteAiProvider 두 번째 구현 (Locality::External)
  net/               HTTP 계약을 notion/에서 승격 (두 번째 사용자가 생겼다)
  SecretKey          AnthropicApiKey

화면
  ErrorBoundary      화면이 죽어도 녹음이 살아 있다고 말한다      ADR-0007 §23.5
  한글화             화면 문자열 전부 + 한글 조판(keep-all · 행간 1.7)
  전사 화면          2,825줄 → 문단 + 검색
  레벨 막대 · 배경 작업 표시 · 제목 고치기 · 설정 저장 막대
```

무엇이 바뀌지 않았는가:

- **녹음 경로.** `cpal` + `hound` 그대로다 (ADR-0003). 증폭은 전사 입력 경로에만 있다
- **엔진 선택 · 청크 규칙 · 붕괴 판정.** ADR-0007 §17 · §18 · §20이 정한 값 전부 그대로
- **INV-1 ~ INV-10.** 하나도 완화되지 않았다

**아직 사람이 확인하지 않은 것:**

```text
Claude provider    **한 번도 실제로 노트를 받아본 적이 없다** — 키를 넣은 적이 없다
창 상투구 제거      값으로만 검증했다. 실제 전사를 다시 돌려 27개가 사라지는 것을 보지 않았다
VAD                효과를 본 적이 없다 (§22.2)
증폭               9/8 저레벨 파일에서 88.9%가 나왔으나, **같은 파일의 증폭 안 한 판을
                   돌리지 않아** 증폭의 효과라고 단정할 수 없다
```

### 2026-09-08 — 온라인 회의 녹음이 **범위에 들어왔고 코드가 붙었다** · **사람이 확인하지 않음**

PRODUCT-SPEC rev 12 · §22 · `ADR-0012`. 마이크와 macOS 시스템 오디오를 함께 녹음하고
**스테레오**로 저장한다 (L = 내 마이크 · R = 시스템).

spike가 잰 것과 아직 재지 않은 것:

```text
[A✓] Core Audio process tap + aggregate device → 마이크·시스템이 한 IOProc으로 온다
[A✓] cpal로는 그 장치를 읽을 수 없다 (config가 전부 1채널) → 기존 경로 재사용 불가
[U]  AirPods HFP · Zoom 점유 · 번들 권한 · tap 중 소리가 들리는지
```

**실제 회의에서 한 번도 돌아간 적이 없다.** 아래는 코드가 놓인 자리이지 동작의 증거가
아니다.

```text
platform/system_audio.rs   tap + aggregate device를 만들고 지운다. 그것뿐이다 (INV-10)
audio/meeting_mix.rs       채널 접기 규칙만 담은 순수 모듈 — L=채널 0, R=나머지 평균
audio/meeting_capture.rs   AudioDeviceIOProc → 접기 → try_send. SampleSource의 두 번째 구현
audio/capture.rs           CaptureMode { Microphone, Meeting } — 시작 시 정해지고 안 바뀐다
commands/mod.rs            meeting_source() — 이 저장소에서 cfg(target_os)가 있는 유일한 자리
Info.plist                 NSAudioCaptureUsageDescription (**화면 녹화 권한이 아니다**)
```

**`capture.rs`는 한 줄도 바뀌지 않았다.** `SampleSource`가 Phase 2B에 이미 있었기 때문이며,
그래서 파일 쓰기 · 일시정지 · 정지 · 레벨 측정 · 확정이 마이크 녹음과 **같은 경로**다
(ADR-0012 §9). 스테레오 WAV 쓰기와 전사 쪽 대응은 **새로 쓸 것이 없었다** — `WavFile`이
이미 `format.channels`를 헤더에 적고, `transcription/audio_input.rs`가 이미 다채널을
mono로 평균낸다.

정직하게 실패하도록 만든 자리 셋 (전부 전용 테스트가 있다):

```text
회의 소스가 없는 플랫폼   조용히 마이크로만 녹음하지 않는다 — 회의가 끝난 뒤에야 발견되므로
IOProc이 큐를 못 넘김     버린 프레임을 세어 두고 **정지할 때** 실패로 올린다
모르는 모드 이름          기본값이 되지 않고 거절된다
```

---

### 2026-09-10 — **구독으로 노트를 만들었다** · 그리고 잡은 버그 셋

이 제품이 처음으로 **끝에서 끝까지** 돌아간 날이다: 1시간 24분 회의 → 전사 → AI 노트.

```text
경로   audio → transcription(whisper) → ai::claude_cli → claude CLI → StructuredNote
과금   API 크레딧이 아니라 구독. 앱은 자격증명을 만지지 않는다
```

**전날 몰랐던 구분이 여기서 갈렸다.** 서드파티 앱이 구독 OAuth 토큰을 받아 대신 요청하는
것은 Anthropic이 금지한다(2026-04-04 시행). 그러나 **이 기기에 로그인된 공식 CLI를 부르는
것**은 그것이 아니다. `ai/anthropic`(HTTP · API 크레딧)과 `ai/claude_cli`(프로세스 · 구독)가
나란히 있는 이유가 이것이다.

같은 날 잡은 버그 셋 — 셋 다 **사용자가 실제로 막힌 자리**다:

```text
진행 중에 갇힘   앱이 죽으면 DB의 running 표시가 남아 껐다 켜도 안 풀렸다.
                 → 저장소를 열 때 정리한다 (앱 시작 시점에는 도는 작업이 있을 수 없다)
예산이 하나였다  컨텍스트 1M 모델에게 로컬 기본값 16,384를 씌워 44 토큰 차이로 거절했다.
                 → 예산을 provider가 말한다
안 쓰는 키 칸    needsApiKey가 "기기 밖에서 도는가"로 판정해, CLI 경로에도 키를 물었다.
                 → 자격증명 필요 여부를 provider의 사실로 옮겼다
```

화면도 이 날 성격을 얻었다 — 굵은 산세리프 하나(Wanted Sans), 보라 강조, 아이콘.
**장식 금지 규칙을 운영자가 걷어냈고**, 여백/타입 스케일 · focus · dark 규율은 남겼다.

---

### 2026-09-14 — **녹음하면서 받아 적는다** · 사람이 확인하지 않음

```text
audio/capture  파일을 쓴다 (헤더 크기는 0인 채)
      ↓ 읽기 전용으로 따라 읽는다 (INV-1)
growing_wav    data 청크 위치만 찾고 **지금 이 순간의 파일 길이**까지를 내용으로 본다
      ↓
live           전사할 구간이 있는가 — 30초 · 꼬리 1초는 물러선다
      ↓
live_run       창 하나만큼 나아간다. 시각을 전체 시간축으로 옮기고 꼬리를 문맥으로 남긴다
      ↓
whisper        `holding_the_model()` — 30초마다 1.6GB를 다시 올리지 않는다
      ↓
run::polish    배치와 **같은** 손질 (붕괴 판정 · 창 상투구 · 되풀이 차단 · 안쪽 축약)
      ↓
run::save_live 받아 적은 것이 곧 Transcript다 (운영자 결정 — 정지 뒤 재전사 없음)
```

**[실측 2026-09-14]** 지난 전사 4건으로 잰 속도는 **약 10배속**이다
(5095s→540s · 1176s→81s · 4371s→364s · 7433s→645s). 30초 창이면 2~3초다.

같은 날 **정지하고 UI가 7분 멈추는 사고**가 있었다. 남은 구간 전사를 정지 경로의
메인 스레드에서 돌렸고, 레코드 저장이 그 뒤에 있었다. 고친 순서가 지금의 규칙이다:

```text
파일 확정 → 레코드 저장 → 끝 → 남은 구간은 배경에서
```

잃은 것은 없었다 (83분 · 1,952문장 저장됨). 그 사고가 남긴 규칙:
**정지 경로에 기다리는 일을 끼우지 않는다.**

---

### 2026-09-15 — **확정된 녹음이 사라졌다** · 그리고 그 구멍을 막았다

15.7분짜리 회의 녹음이 디스크에 확정까지 끝난 채로 있는데 **목록에 없었다.** 사람이
DB를 직접 고쳐 되살려야 했다.

```text
정지 = 두 걸음    ① 파일을 확정한다      ② 레코드를 저장한다
                   그 사이에서 어긋나면 → 앱은 그 녹음을 모른다
```

반대 방향(레코드는 있는데 파일이 없다)은 `finalized::audio_is_present`가 이미 다루고
있었다. **그 짝이 없었다.**

이제 `Storage::open`이 여는 순간 되살린다 (`audio::orphaned`). 앱이 막 시작한 시점에는
쓰이는 중인 파일이 없으므로, 거기서 본 것은 전부 지난 실행이 남긴 것이다.

**`lib.rs`가 한 번 부르는 형태로 두지 않았다** — 그것은 부르는 쪽이 기억해야 하는
구조이고, 이 기능이 있어야 했던 이유가 바로 그런 종류의 실수였다. 같은 날, 고쳐 둔
빌드를 띄우지 않아 옛 바이너리가 돌고 있었던 것이 이 사고의 직접 원인이었다.

되살릴 때 지어내지 않는다: 제목은 파일 시각에서, **길이는 파일에서 읽고**(못 읽으면
되살리지 않는다), 장치 이름은 비운다. 파일은 어떤 경우에도 건드리지 않는다 (INV-1).

같은 날 녹음 화면에 **입력 레벨을 따라 일렁이는 강조**가 붙었다. 도는 스피너가 아니라
`usable · low · silent` 갈래로 세기가 갈리므로, 움직임이 곧 "지금 들리고 있다"는 신호다.

---

## 6. Validation Model

| | 무엇을 보장하는가 | 수단 |
| --- | --- | --- |
| **Automated validation** | 위의 전부 + **Markdown 렌더링 결정성 · 파일명 정규화 · 무손실 분할과 재조립 · Notion 요청 조립과 실패 변환 · `Retry-After` 준수 · 중복 페이지 없는 재시도 · SecretStore 경계** + **Manual AI Handoff 산출물 셋의 결정성과 기대 문자열 · 프롬프트 상수 불변 · MH-1~MH-8 · clipboard 실패의 값 · UI 기반 다섯 검사** + **입력 레벨의 dBFS·판정·문장(무음 · -42 · -26 · 풀스케일 · 빈 입력 · 덩어리 경계 불변) · 붕괴 판정의 임계값 경계와 수치 보존 · 붕괴한 출력이 Transcript를 남기지 않고 current를 바꾸지 않는다는 것 · 화면의 붕괴 갈래와 레벨 세 갈래 · 이 Phase의 다섯 불변** + **전사 언어가 엔진 경계까지 값으로 도달한다는 것(고르지 않으면 감지 · 고르면 지정) · 언어 설정의 왕복과 다른 설정 저장에도 지워지지 않음 · 전사 소요 시간의 저장·재조회와 '값 없음' 경로 · 여는 대상이 exports 아래로 제한되고 `..`·symlink·디렉터리·빈 경로가 거절된다는 것 · OS 호출이 `platform/` 안에만 있다는 것 · 나눔의 결정성·무손실·순서 보존·경계값 · §11 Markdown export 형식 불변 · Phase 5.6의 네 불변** — **자동 테스트 1,471개** (web `vitest` 586 · Rust `cargo test` 885 — TASK-074 Run에서 **Runtime이 직접 돌린** `test` Gate 로그의 값이다 · §5) | `build` · `lint` · `test` Gate + 독립 Verifier<br>Task별 Gate는 최소·관련 범위로 좁힐 수 있으나 **Phase 종료 시에는 저장소 전체에 세 Gate를 모두 돌린다** (2026-09-04 운영자 결정) |
| **Human validation / witness** | 실제 마이크 음질 · 재생 음질 · **실제 Whisper 추론이 읽을 만한 전사를 내는가(`A-TRANS-001`)** · **실제 Ollama 호출과 AI Note의 유용성(`A-AI-001`)** · **실제 Notion 전송과 페이지 품질 · 1시간 transcript 온전성(`A-NOTION-001`)** · export Markdown의 외부 도구 호환성 · **화면의 시각적 완성도** · **Windows 실동작** · **연기된 recording 장치 검증(ADR-0003 §12)** + **실제 webview에서의 clipboard 동작 · Manual AI Handoff 산출물이 실제 외부 AI 채팅에서 쓸 만한가 · 로컬 provider가 '선택적 · 고급'으로 읽히는가** (`docs/PHASE-5.5-HUMAN-REVIEW.md` — **기록표는 비어 있다**) + **레벨 표시로 "지금 소리가 안 담기고 있다"를 알 수 있는가 · 마이크나 자리를 고쳤을 때 레벨이 올라가는가 · 붕괴 실패 메시지로 무엇을 해야 할지 알 수 있는가 · 레벨을 올려 다시 녹음한 회의가 읽을 만하게 전사되는가** (`docs/PHASE-5.7-HUMAN-REVIEW.md` — **기록표는 비어 있다**) + **한국어 회의가 읽을 만하게 전사되는가 · `ggml-base`로 충분한가(`A-TRANS-001`의 실질적 답) · Metal 전후로 체감이 달라지는가(before 값을 직접 만들어야 한다) · 내보낸 파일을 실제로 열 수 있는가 · 긴 handoff를 실제 AI 채팅에 넣을 수 있는가** (`docs/PHASE-5.6-HUMAN-REVIEW.md` — **기록표는 비어 있다**) | 사람이 직접 확인 |

> Phase 1에서 사람이 확인한 것: 번들 `.app`의 Info.plist 병합(확인됨) ·
> 권한 문구(확인됨) · 화면 레이아웃(**소스 수준 검토만 — 실행 화면 확인은 사용자 몫**).

**자동으로 판정할 수 없는 것을 자동 PASS로 적지 않는다.** 상세는 `PRODUCT-SPEC.md` §18.

---

## 7. Known Boundaries / Deferred Work


### ⚠️ 2026-09-08에 만들어졌으나 **사람이 확인하지 않은 것**

이것들은 Gate 셋을 지났고 전용 테스트가 붙어 있다. **그것이 동작한다는 뜻은 아니다.**

- ~~**Claude provider가 노트를 만든 적이 없다**~~ — 2026-09-10에 해소됐다. 다만 **HTTP
  경로(`ai/anthropic`)가 아니라 CLI 경로(`ai/claude_cli`)로** 만들었다. HTTP 쪽은
  실제로 400(크레딧 부족)까지 갔고 그 위는 여전히 미실행이다
- **창 상투구 제거를 실제 전사로 확인하지 않았다.** 값으로만 검증했다 — 그 27개가
  실제로 사라지는 것을 본 적이 없다
- **VAD는 효과를 보인 적이 없다.** 켜고 돌린 결과가 오히려 0.4%p 낮았다 (`ADR-0007` §22.2)
- **증폭의 효과를 단정할 수 없다.** 같은 파일의 증폭하지 않은 판을 돌리지 않았다
- **화면이 왜 죽었는지 모른다.** 방어(ErrorBoundary)를 넣었을 뿐 원인을 고치지 않았다
  (`ADR-0007` §23.5)
- ~~**`live.rs`는 아무 데도 연결되지 않았다**~~ — 2026-09-14에 해소됐다. 실시간 전사가
  녹음 경로에 붙었다 (아래 참조). **다만 실제로 문장이 쌓이는 것을 아직 보지 못했다**
- **실시간 전사가 실제로 문장을 쌓는 것을 본 적이 없다.** 경로는 끝에서 끝까지 이어졌고
  테스트 33개가 붙었지만, 녹음하면서 화면에 문장이 뜨는 것을 아직 확인하지 못했다.
  **2026-09-14 첫 시도에서는 정지 시점까지 거의 전부가 밀려 있었다** — 첫 모델 적재가
  오래 걸려 못 따라잡았는지, 일찍 포기했는지 **구분되지 않았다.** 30초 넘겨 짧게 한 번
  녹음하면 갈린다
- **회의 모드가 실제 회의에서 돌아간 적이 없다.** 코드는 끝에서 끝까지 이어졌고 Gate 셋을
  지났지만, Meet도 Zoom도 열어 본 적이 없다. `ADR-0012` §7의 네 항목이 여전히 `[미검증]`이며,
  그중 *회의 중에 녹음이 시작되지 않는 실패*는 알아챌 때 이미 늦다
- **번들된 앱에서 오디오 권한 프롬프트가 뜨는지 모른다.** `NSAudioCaptureUsageDescription`을
  선언했을 뿐이다. 뜨지 않으면 tap 생성이 거부되고 회의 모드는 시작 시점에 실패한다
- **48 kHz는 관측 한 번에 근거한 고정값이다** (`audio/meeting_capture.rs`). 다른 기기나
  AirPods에서 다른 값이 나오면 WAV 헤더가 내용과 어긋난다
- **제목 고치기는 스펙에 적히지 않았다.** §5의 화면 스케치는 제목을 보여주는 것으로만
  적고 있다. 운영자 결정 한 줄이 필요하다

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
- ~~**AI Note를 얻으려면 로컬 추론 서버를 설치해야 한다**~~ — Phase 5.5에서 **완화됐다.**
  AI Provider가 하나도 없어도 Manual AI Handoff로 같은 값을 얻을 수 있고, 로컬 provider는
  '선택적 · 로컬 · 고급'으로 재배치됐다 (`PRODUCT-SPEC` §9.7 · §14.5.1).
  **삭제가 아니다** — 연결된 provider 경로는 그대로 있다 (MH-8).
- **⚠️ clipboard 쓰기가 실제 환경에서 확인되지 않았다** — `src/platform/clipboard.ts`가
  `navigator.clipboard`를 부르지만, **Tauri v2 webview(WKWebView · WebView2)에서 이 앱의
  origin에 그 능력이 실제로 있는지, 어떤 조건을 요구하는지, 거절할 때 어떤 값이 오는지는
  확인된 적이 없다** (ADR-0010 §7.4의 여섯 항목 · §12.4 — **여섯 전부 여전히 UNVERIFIED**).
  자동 테스트는 언제나 test double을 쓴다. 그래서 이 경계는 "동작한다"가 아니라 **"동작하지
  않을 수 있다"를 전제로** 설계됐다 — 실패가 보이는 값이 되고, 그때에도 **Export for AI가
  대체 경로로 남는다.** 확인은 `docs/PHASE-5.5-HUMAN-REVIEW.md`에서 사람이 한다.
  `A-` 가정으로 올리지 않은 이유는 D-4다 — 이것이 최악으로 판명돼도 Phase 5.5의 성공 기준은
  무너지지 않고, 복구는 모듈 하나를 갈아 끼우는 일이다 (ADR-0010 §7.5).
- **⚠️ Manual AI Handoff의 산출물이 실제 AI 채팅에서 쓸 만한지 확인되지 않았다** —
  프롬프트가 요구하는 출력 모양이 `markdown::render`가 이미 만드는 모양과 같다는 것은
  테스트가 지키지만, **받아 온 답의 유용성은 자동으로 판정할 수 없다.** 앱에 파서가 없으므로
  모델이 계약을 어겨도 깨지는 것은 없다 — 그것은 실패가 아니라 **품질 문제**이며 Human
  Review 항목이다 (ADR-0010 §6.5).
- **⚠️ 전사가 실제 실행에서 쓸 수 없는 결과를 냈다** (2026-09-05) — 엔진 경로는 지났으나
  `whisper.rs`가 언어를 설정하지 않아 한국어가 영어로 강제 디코딩됐다. Metal도 꺼져 있다.
  **`A-TRANS-001`은 해소되지 않았다.** 실측 기록은
  `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록, 수정은 Phase 5.6.
  **그 둘은 Phase 5.6의 TASK-067 ~ TASK-069로 실제로 고쳐졌고, 2026-09-07의 두 번째 실사용이
  그것을 확인했다** (설정한 `ko`가 엔진에 도달했다 · `phase-prompt/05.7` R-1).
- **⚠️ `A-TRANS-001`은 여전히 열려 있다 — 두 번째 실사용도 읽을 만한 전사를 내지 못했다**
  (2026-09-07). 51분 회의가 **고유 문장 2개**(한 문장이 99.0% · 30초 윈도우마다 하나씩)로
  나왔고 제품은 그것을 `done`으로 저장했다. 같은 마이크의 9/4 녹음보다 평균 입력 레벨이
  **16.4 dB 낮았다**(-42.2 vs -25.8 dBFS).
  **[유력한 설명]** 입력 레벨이 낮아 whisper가 음성을 찾지 못하고 빈 자리를 학습 데이터의
  잔재로 채웠다. **[미검증] 레벨이 유일한 원인인지, 레벨을 올리면 이 오디오가 구제되는지는
  확인되지 않았다** — 절차는 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록 2에 있고
  **결과 칸은 전부 `[미측정]`이다.**
  **이 가정이 닫히는 조건은 하나다 — 사람이 입력 레벨을 올려 다시 녹음한 회의가 제품 경로에서
  읽을 만하게 전사되는 것을 보는 것이다** (`docs/PHASE-5.7-HUMAN-REVIEW.md` HR-4).
  **Phase 5.7은 이 가정을 닫지 않는다.** 이 Phase가 한 것은 *붕괴를 성공이라 부르지 않는 것*과
  *녹음 중에 레벨을 보이는 것*이지, *읽을 만한 전사를 내는 것*이 아니다.
- **⚠️ 레벨 판정 구간과 붕괴 임계값은 관측 두셋에서 고른 값이다** — `-36 dBFS`(쓸 만함) ·
  `-60 dBFS`(소리 없음) · `u/n <= 0.20` · `r/n >= 0.50` · `n >= 20`. 근거는 9/4 · 9/5 · 9/7의
  세 실행뿐이고, 특히 `r/n`의 문턱은 가장 가까운 붕괴 관측(62.1%)과 **12.1%p** 떨어져 있을
  뿐이다 (`ADR-0007` §18.2 · `ADR-0003` §16.4). **설정 항목으로 열지 않았다** — 틀렸다면 고칠
  자리는 각 모듈의 상수다. **실사용에서 오판(정상 전사를 붕괴로 부르거나 그 반대)이 나오는지는
  아직 확인되지 않았다.**
- **⚠️ 누적 레벨은 녹음 도중의 개선을 곧바로 보여 주지 않는다** — 표시되는 값이 *지금 이 순간*이
  아니라 *지금까지 파일에 쓰인 전체*의 평균 RMS와 누적 피크다. 마이크를 고쳐도 평균은 천천히
  올라오고 **누적 피크는 아예 내려가지 않는다.** 최근 구간 미터가 필요한지는 이 Phase가 정하지
  않았다 (`ADR-0003` §16.3). **레벨이 낮다고 앱이 녹음을 막거나 멈추지 않는다** — 알릴 뿐이다.
- **⚠️ 이미 저장된 붕괴한 Transcript는 그대로 남아 있다** — 붕괴 판정은 **저장을 막는 자리**에
  있고, 저장된 것을 고치거나 지우는 경로는 생기지 않았다 (INV-2). 2026-09-07에 `done`으로 저장된
  그 Transcript도 `current`인 채로 있다. **소급 정리 기능은 만들지 않았다.**
- **⚠️ Metal은 켜졌지만 그것이 빨라졌다는 뜻은 아니다** — 확인된 것은 **빌드 산출물이
  달라졌다**까지다 (`GGML_METAL` OFF→ON · `libggml-metal.a` · 링크된 바이너리의 `ggml_metal_*`
  심볼 · `ADR-0007` §19.2). **이 저장소는 전사 속도를 한 번도 측정한 적이 없고**, 런타임에 GPU가
  실제로 잡히는지도 확인하지 않았다. **전사 소요 시간을 기록하는 열은 Metal을 켠 것과 같은
  Phase에 생겼으므로 "Metal 전"의 값이 저장소 어디에도 없다** — 비교하려면 사람이 빌드를 한 번
  되돌려 before를 직접 만들어야 한다 (`docs/PHASE-5.6-HUMAN-REVIEW.md` §4.2).
  2026-09-05의 26분도 2026-09-07의 4.30분도 **붕괴한 디코딩의 소요 시간이라 기준값이 아니다.**
- **⚠️ `whisper-rs`의 feature 목록을 crate 소스에서 읽지 못했다** — `metal`이라는 이름이 두
  crate에 선언돼 있다는 것은 **cargo가 manifest를 파싱해 fingerprint에 남긴 `declared_features`**
  에서 확인했다. **crate의 `Cargo.toml` 파일 자체를 연 것이 아니다** (`ADR-0007` §19.2).
  registry 접근도 네트워크도 없는 제약은 그대로다.
- **⚠️ 내보낸 파일의 자리를 여는 것이 실제로 동작하는지 확인되지 않았다** — `open -R`도
  `explorer /select,`도 이 저장소의 어떤 Run에서도 실행된 적이 없고, 두 이름과 플래그는 각
  플랫폼의 관례를 따른 것이다 (`ADR-0009` §16.2의 [E4]). 자동 테스트는 언제나 double을 쓴다 —
  **부르면 검사가 도는 동안 창이 열리기 때문에** 그것이 의도된 설계다. **확인하지 못한 것에
  제품을 걸지 않았다**: 실패하면 정의된 실패가 화면에 도착하고 **전체 경로는 그대로 남아**
  사람이 직접 찾아갈 수 있다. 확인은 `docs/PHASE-5.6-HUMAN-REVIEW.md` HR-4다.
- **⚠️ AI Handoff의 나눔 예산은 확인된 한도가 아니다** — `PORTION_MAX_BYTES = 40_000`은
  **이 앱이 고른 값**이며 어떤 AI 채팅의 확인된 입력 한도도 아니다 (`ADR-0010` §12.8.2).
  Notion의 `CHUNK_MAX_BYTES`에서 유도되지도 않았고, 그 독립성은 테스트가 못박는다.
  **틀렸다면 고칠 자리는 상수 한 줄이며**, 실제 채팅이 조각을 거절하는지는
  `docs/PHASE-5.6-HUMAN-REVIEW.md` HR-5 · §8.6에서 처음 관측된다.
- **⚠️ 한국어 전사 품질과 모델 크기는 여전히 판정되지 않았다** — Phase 5.6은 **여러 모델을 쓸
  수 있게** 만들었을 뿐 **어느 것이 충분한지 판정하지 않기로 했다** (`ADR-0007` §17.4 · §19.5).
  `PRODUCT-SPEC` §14.4의 *"한국어+영어 혼용 1시간 녹음에 `large-v3` / `large-v3-turbo`가 현실적"*
  이라는 서술은 **여전히 UNVERIFIED이며 실측된 적이 없다.** 첫 실측은
  `docs/PHASE-5.6-HUMAN-REVIEW.md` HR-2에서 나온다.
- **⚠️ 사용자가 고른 언어인지 엔진이 감지한 언어인지가 Transcript에 남지 않는다** —
  `ADR-0007` §17.1.4-2는 그 구분이 Transcript에서 보여야 한다고 적었지만, **구현은 출처를 담는
  값을 만들지 않았다.** 저장되는 것은 여전히 엔진이 보고한 코드 하나이며, 그 전사가 지정으로
  만들어졌는지 감지로 만들어졌는지는 **그때의 설정을 함께 봐야** 알 수 있다 (`ADR-0007` §19.4).
  필요해지면 열 하나를 더하는 일이다.
- **⚠️ 실시간 전사 진행률은 구현되지 않았다** — `phase-prompt/05.6`이 이 Phase의 성공 기준에서
  제외했고(R-6은 관측으로만 남아 있다), Phase 5.6이 책임진 것은 **시간이 기록으로 남는 데까지**다.
  긴 전사가 도는 동안 화면에 변화가 없는 것은 그대로다.
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
| `docs/ADR-0010-manual-ai-handoff.md` | **Manual AI Handoff 결정** — 두 제품 방향 변경의 기록 · AI-ready 산출물의 형식과 순서 · `export::markdown` 재사용 · `ai::prompt` 상수를 고치지 않는 재사용 · clipboard 경계의 자리와 탈락 후보 · command 이름 셋 · MH-1~MH-8의 판정 수단 · **§12 구현 대조와 남은 UNVERIFIED** |
| `docs/PHASE-5.5-HUMAN-REVIEW.md` | **자동으로 판정할 수 없는 셋(화면 · Manual AI Handoff의 유용성 · 로컬 provider의 위치)의 Human Review 절차와 빈 기록표.** §6이 이 Phase가 확인하지 **않은** 것의 정본이다 |
| `phase-prompt/05.6-transcription-correctness-and-reach.md` | 2026-09-05 실제 전사 실행이 드러낸 결함의 Goal · **R-1 ~ R-6의 실측 전제** (**engineering DONE** — §5) |
| `docs/ADR-0007-transcription-engine.md` **§17** | **전사 언어 결정과 Metal 결정** — whisper.cpp의 기본값이 `"en"`이고 감지가 꺼져 있다는 사실 · 붕괴의 원인이 디코더 설정이 아니라는 것 · 앞으로 무엇을 부를 것인가 · Spec §D와의 연결 |
| `docs/ADR-0007-transcription-engine.md` **§19** | **§17의 구현 결과** — 언어가 어디서 읽혀 어디까지 도달하는가 · Metal이 켜졌다는 판정의 근거(산출물) · 전사 소요 시간이 남는 자리 · **계획과 달라진 다섯** · `A-TRANS-001`이 여전히 열려 있다는 것 |
| `docs/ADR-0009-notion-and-export.md` **§16** | **내보낸 파일의 자리를 여는 결정** — 왜 경로를 보여 주는 것만으로 부족했는가(2026-09-05) · 무엇이 그 자리를 여는가 · **왜 앱의 `exports/` 아래로 제한되는가** · 확인하지 못한 것 |
| `docs/ADR-0010-manual-ai-handoff.md` **§12.7 · §12.8** | **크기와 나눔** — 인자와 응답이 넓어진 자리(이름은 늘지 않았다) · 실측(99 KB · 5,139줄 · 제목 1,711개) · **예산이 이 앱이 고른 값이지 벤더 제약이 아니라는 것** · 나눔 규칙 · **§11 export 형식이 바뀌지 않았다는 판정 근거** |
| `docs/PHASE-5.6-HUMAN-REVIEW.md` | **자동 Gate가 판정할 수 없는 다섯(한국어 전사 품질 · 모델 크기 · Metal 체감 · 파일 도달 · handoff의 실제 사용)의 Human Review 절차와 빈 기록표.** §4.2가 **Metal before 값을 직접 만드는 절차**이고, §7이 이 Phase가 확인하지 **않은** 것의 정본이다 |
| `phase-prompt/05.7-recording-level-and-transcription-collapse.md` | 2026-09-07 두 번째 실제 전사 실행이 드러낸 두 침묵의 Goal · **R-1 ~ R-5의 실측 전제**([관측된 사실] · [유력한 설명] · [미검증] 구분) |
| `docs/ADR-0003-recording-engine.md` **§16** | **입력 레벨 경계** — 재는 자리(실시간 콜백이 아닌 파일 통로) · 내보내는 값 · 판정 구간과 근거(-25.8 vs -42.2 dBFS) · **게인 조정과 정규화를 하지 않는다는 것** |
| `docs/ADR-0007-transcription-engine.md` **§18** | **붕괴 판정 규칙** — 무엇을 붕괴로 보는가(`n >= 20` · `u/n <= 0.20` · `r/n >= 0.50`) · 세 관측에서 임계값을 고른 근거 · 규칙이 사는 자리 · 대응하는 §13 실패 · **저장을 막는 자리이지 저장된 것을 고치는 자리가 아니라는 것**(INV-2) |
| `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` **부록 2** | **성공 기준 3의 측정 절차와 비교표.** 레벨을 올린 **사본**을 만드는 절차(원본은 읽기만 한다 · INV-1) · 같은 조건의 전사 · 재는 방법. **결과 칸은 `[미측정]`이며 왜 측정하지 못했는지가 부록2-5에 있다** |
| `docs/PHASE-5.7-HUMAN-REVIEW.md` | **자동 Gate가 판정할 수 없는 넷(레벨을 보고 아는가 · 고쳤을 때 올라가는가 · 붕괴 메시지로 할 일을 아는가 · 다시 녹음한 회의가 읽을 만하게 전사되는가)의 Human Review 절차와 빈 기록표.** §7이 이 Phase가 확인하지 **않은** 것의 정본이다 |
| `docs/ADR-0007-transcription-engine.md` **§22** | **무음 구간 환각** (2026-09-08) — 조용한 구간에서만 무너진다는 관측 · **VAD가 듣지 않았다는 기록** · 진짜 기전(창을 채운 상투구 · 30초 간격) · 왜 문구 목록이 아니라 말의 속도인가(오탐·누락을 둘 다 관측했다) · 임계값 20초 · 1.0 자/초의 근거(6,141 segment) |
| `docs/ADR-0007-transcription-engine.md` **§23** | **Runtime을 거치지 않고 들어온 변경들** (2026-09-07 · 09-08) — UTF-8 경계 · 반복 차단 연결 · 전사 중 미리보기 · `raw_text`가 차단을 지나지 않았던 것 · 화면이 죽어도 녹음은 죽지 않는다. **Gate는 지났으나 Verifier는 지나지 않았다는 사실을 포함한다** |
| `docs/ADR-0003-recording-engine.md` **§16.8** | **구간 레벨 실측과 전사 입력 증폭** (2026-09-08) — 한 파일 안에서 10.3 dB가 갈린다는 실측 · **누적 평균이 구간을 못 본다는 것** · 녹음이 아니라 **전사 입력만** 키운다는 경계 · 목표(-23 dBFS)·상한·하한의 근거 · 증폭의 기전은 `[미검증]`이라는 것 |
| `docs/ADR-0012-meeting-audio-capture.md` | **온라인 회의 녹음의 후보 비교와 spike 실측** (2026-09-08 · `PROVISIONAL`) — Core Audio process tap vs ScreenCaptureKit(**권한이 결정했다**) · `isExclusive` 없으면 무음만 나온다는 함정 · **cpal로는 그 장치를 읽을 수 없다는 측정** · AirPods HFP 처리 · 아직 재지 않은 넷. **구현은 없다** |
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
| 2026-09-04 (운영자 · D-1) | **Manual AI Handoff를 정식 제품 기능으로 추가하고, Phase 5.5를 로드맵에 삽입한다** | Phase 6과 Final Integration에서 사람이 앱을 처음 실제로 쓴다. 그때 AI Note로 가는 유일한 길이 "로컬 추론 서버를 설치하세요"이면 검증되는 것은 제품이 아니라 설치 안내다. §11(Markdown interoperability)의 연장이며 늘어나는 것은 **가져가는 형태 하나**다 | 채택 (`ADR-0010` §4.2 · `PRODUCT-SPEC` §9.7 · §21) |
| 2026-09-04 (운영자 · D-2) | **로컬 Ollama provider를 '첫 provider · 기본 AI 경험'에서 '선택적 · 로컬 · 고급'으로 재배치한다. 삭제가 아니다** | Phase 4가 만든 것은 "이 벤더의 기능"이 아니라 provider 추상화이며, Manual Handoff가 생겨도 유효하다. 지우면 DEFERRED cloud provider가 돌아올 자리도 사라진다. 유지 비용도 재배치 비용도 낮다 — **적극적으로 해로운 경우가 아니면 삭제하지 않는다** | 채택 (`ADR-0010` §4.3 · `PRODUCT-SPEC` §14.5.1) |
| 2026-09-04 (운영자 · D-3) | **§21 로드맵의 상태 열을 실제 구현 상태로 정정한다** | rev 8까지 Phase 1~5가 `PLANNED`로 남아 표가 저장소와 어긋나 있었다. 과거를 다시 쓰지 않고 **오늘의 상태만** 적으며, 각 Phase가 남긴 이력은 이 문서 §5에 그대로 있다 | 채택 (`PRODUCT-SPEC` §21) |
| 2026-09-04 (운영자 · D-4) | **`A-HANDOFF-001`을 만들지 않는다** | Manual AI Handoff는 외부 provider 없이 자동 검증되므로 `A-REC-001` 계열의 hard deferred assumption이 아니다. 자동으로 판정할 수 없는 셋은 **주관적 Human Review 항목**으로 남긴다 | 채택 (`docs/PHASE-5.5-HUMAN-REVIEW.md`) |
| 2026-09-04 (운영자 · D-5) | **Open ChatGPT / Open Claude(브라우저 열기)를 구현하지 않는다** | 본질은 딥링크 자동화가 아니라 **가져갈 수 있는 AI-ready 내용**이다. 특정 채팅을 이름으로 아는 순간 그 채팅이 바뀔 때 흔들리는 것이 adapter 하나가 아니라 산출물 자체가 된다 (INV-9) | 채택 (`ADR-0010` §10 · §11) |
| 2026-09-06 (Phase 5.5) | **clipboard 경계를 프론트엔드 platform 모듈 하나로 둔다.** Rust command도 Tauri 플러그인도 쓰지 않는다 | 복사할 문자열은 이미 webview 안에 있어 Rust로 되돌려 보낼 이유가 없고, 플러그인은 **확인하지 못한 세 값**(crate/npm 이름 · 호환 버전 · permission 식별자) 위에 의존성을 얹는 선택이다. A가 플랫폼에서 막히면 그때 B/C로 **모듈 하나를 갈아 끼운다** | 채택 · **플랫폼 능력은 UNVERIFIED** (`ADR-0010` §7 · §12.4) |
| 2026-09-06 (Phase 5.5) | **기존 프롬프트 상수를 고치지 않고 치환 두 번으로 재사용한다.** 두 번째 프롬프트 세트를 만들지 않는다 | 상수를 고치면 `PROMPT_VERSION_*`의 선언값과 계산값이 어긋나 테스트가 깨지고, 선언값을 따라 올리면 **이미 저장된 `ai_notes.prompt_version`이 가리키는 프롬프트가 저장소 어디에도 없게 된다** — 저장된 provenance가 거짓이 된다 | 채택 (`ADR-0010` §6) |
| 2026-09-06 (Phase 5.5) | **UI 기반은 "처음부터 만들기"가 아니라 "이미 있는 것을 규칙으로 끌어올리기"다.** 서드파티 디자인 시스템을 들이지 않는다 | 색 토큰 여섯과 dark 장치, radius 5px는 이미 있었다. 비어 있던 것은 타입·여백 스케일 · accent/상태색 · focus다. 값은 대부분 이미 쓰이던 것을 토큰으로 올린 것이며 새 크기를 발명하지 않았다 | 채택 (`phase-prompt/05.5` R-1 · `tests/ui-foundation.test.ts`) |
| 2026-09-05 (운영자) | **Phase 5.6을 로드맵에 삽입한다** | 첫 실사용에서 72분 한국어 회의가 통째로 못 쓰게 됐다. **새 제품 방향이 아니라 Spec과 구현 사이의 간극이다** — §D의 `language` 설정도 §14.4의 Metal도 Spec에 이미 있었다. 그래서 새 ADR을 쓰지 않고 기존 ADR을 갱신한다 | 채택 (`phase-prompt/05.6`) |
| 2026-09-06 (Phase 5.6) | **전사 언어를 설정으로 두되 "고르지 않음"을 자동 감지로 정의한다.** 로캘을 짐작해 굳혀 두지 않는다 | 결정이 없어서 whisper.cpp의 기본값(`"en"` · 감지 OFF)이 그대로 쓰였고 한국어 회의가 영어로 강제 디코딩됐다. **짐작한 값이 저장되면 그때부터 그것은 사용자가 고른 값처럼 보인다** — 앱 화면 언어는 말하는 언어가 아니다 | 채택 (`ADR-0007` §17.1 · §19.1) |
| 2026-09-06 (Phase 5.6) | **Spec §14.4가 적은 대로 Metal을 켠다. 켜졌다는 판정은 manifest 기재가 아니라 빌드 산출물로 한다** | Spec과 저장소가 어긋나 있었고 어긋난 쪽이 저장소였다. **켠 근거는 측정된 속도가 아니다** — 이 저장소는 전사 속도를 측정한 적이 없고 배수를 적지 않는다. `Cargo.lock`은 이 feature에서 바뀌지 않으므로 ADR-0009 §11.4의 lock 판정 대신 `GGML_METAL` · 링크 산출물 · 심볼로 판정했다 | 채택 · **속도 UNVERIFIED** (`ADR-0007` §17.2 · §19.2) |
| 2026-09-07 (Phase 5.6) | **전사에 걸린 시간을 Transcript와 함께 남긴다. 값이 없는 것을 0이라고 말하지 않는다** | Metal 전후를 사람이 비교하려면 비교할 값이 남아야 한다. 재지 않은 전사를 `0:00`이라고 말하면 그 비교가 거짓을 말하므로 열은 nullable이고 `DEFAULT`가 없으며 화면은 그 줄을 그리지 않는다. **실시간 진행률은 이 Phase의 성공 기준이 아니다** | 채택 (`ADR-0007` §19.3) |
| 2026-09-07 (Phase 5.6) | **내보낸 파일이 놓인 자리를 여는 수단을 더한다. 경로 문자열은 그대로 남긴다** | §4.1이 세운 전제(*"경로를 보여 주면 찾을 수 있다"*)가 macOS에서 틀렸다 — `~/Library`는 Finder 기본 숨김이다. **부족했던 것은 정보가 아니라 수단이므로 경로를 없애지 않고 수단을 더한다.** 여는 것은 파일이 아니라 자리이며, 허용은 앱의 `exports/` 아래로 제한된다 — **주소를 만드는 쪽이 아니라 여는 쪽이 범위를 정한다** | 채택 · **실제 동작 UNVERIFIED** (`ADR-0009` §16) |
| 2026-09-07 (Phase 5.6) | **AI Handoff 산출물의 나눔 예산을 40,000 B로 정한다 — 이 앱이 고른 값이며 벤더 제약이 아니다** | 72분 녹음의 산출물이 99 KB였고 채팅에 한 번에 들어가지 않았는데 **앱이 그 사실을 말해 주지 않았다.** 어떤 채팅의 한도도 확인된 적이 없으므로 그 숫자를 코드에 적으면 벤더 지식이 된다 (INV-9). Notion의 예산과도 근거가 다르며 서로를 따라 움직이지 않는다 | 채택 · **실제 채팅 한도 UNVERIFIED** (`ADR-0010` §12.8.2) |
| 2026-09-07 (Phase 5.6) | **AI 경로의 transcript만 압축 모양으로 적고, §11 export 파일 형식은 바꾸지 않는다** | 크기의 이유가 `### HH:MM:SS` 제목 1,711줄이었다. 그러나 §11 파일은 사람이 읽는 문서이고 Notion 본문이기도 하므로 모양을 바꾸면 함께 흔들린다. **렌더링 규칙은 한 자리에 두고 모양만 둘로 갈랐다** — 무엇을 어떤 순서로 적는가는 두 모양이 같은 함수에서 온다 | 채택 (`ADR-0010` §12.8.4) |
| 2026-09-07 (Phase 5.6) | **모델 크기 판정을 이 Phase가 하지 않는다. Human Review 다섯을 `A-` 가정으로 올리지 않는다** | 한국어 전사 품질 판정에는 사람의 귀가 필요하다 — Worker도 Gate도 Verifier도 *"이 전사가 읽을 만한가"* 를 판정할 수 없다. Phase 5.5의 D-4와 같은 판단이며, **다섯 중 둘(한국어 품질 · 모델 크기)은 이미 열려 있는 `A-TRANS-001`의 실질적 답이지 새 가정이 아니다** | 채택 (`ADR-0007` §17.4 · §19.5 · `docs/PHASE-5.6-HUMAN-REVIEW.md`) |
| 2026-09-07 (운영자) | **Phase 5.7을 로드맵에 삽입하고, 부분 완료인 Phase 5.6 앞에 둔다** | 두 번째 실사용에서 51분이 사라졌다. Phase 5.6의 언어 수정은 동작했으나 그 아래에 두 침묵이 있었다 — 녹음 중에 소리가 담기지 않는다는 것을 말해 주지 않았고, 붕괴한 전사를 완료라고 말했다. **제품이 실패를 실패라고 말하지 않는 것**이 남은 Phase 5.6 Task보다 급하다 | 채택 (`phase-prompt/05.7`) |
| 2026-09-07 (Phase 5.7) | **"쓸 수 없는 전사"를 값으로 정의하고, 저장 직전에서 판정한다** — `n >= 20` · `u/n <= 0.20` **또는** `r/n >= 0.50` | 저장 직전 검사가 `segments.is_empty()` 하나였고, 그래서 103개짜리 붕괴가 "있어서" 통과했다. Transcript는 immutable이므로(INV-2) 막을 수 있는 자리는 저장 직전뿐이다. **임계값은 세 관측에서 골랐고 그 약함(`r/n`은 가장 가까운 붕괴와 12.1%p)을 감추지 않는다** | 채택 · **임계값 UNVERIFIED** (`ADR-0007` §18.2 · §18.5) |
| 2026-09-07 (Phase 5.7) | **새 실패 종류를 만들지 않고 이미 있는 `TranscriptionOutputUnusable`을 쓴다** | §13의 갈래가 이미 그 뜻이다. 종류를 늘리면 화면 · IPC · 저장된 상태의 해석이 함께 늘어나는데, 늘어나는 정보는 *왜 쓸 수 없는가*의 수치뿐이며 그것은 `message`와 `detail`에 담긴다 | 채택 (`ADR-0007` §18.4) |
| 2026-09-07 (Phase 5.7) | **입력 레벨은 실시간 오디오 콜백이 아니라 파일에 쓰이는 통로(`drain`)에서 잰다** | 사람이 묻는 질문은 *"이 파일이 쓸 수 있는 소리를 담고 있는가"* 다. 콜백에서 재면 파일에 도달하지 않는 샘플(일시정지 구간)까지 세게 되고, 실시간 스레드에 일이 늘어난다 | 채택 (`ADR-0003` §16.2) |
| 2026-09-07 (Phase 5.7) | **판정은 평균 RMS로만 한다. 피크로 하지 않는다** | 9/7 파일의 전체 피크는 -19.4 dBFS로 무음이 아니었다 — 피크만 보면 "소리가 들어오고 있다"고 말하게 된다. 두 파일을 가른 것은 평균 RMS의 16.4 dB 차이였다 | 채택 (`ADR-0003` §16.4) |
| 2026-09-07 (Phase 5.7) | **재지 않은 것을 0이라고도 '낮음'이라고도 말하지 않는다** — 값이 없으면 `null`이고 화면의 `unknown`은 판정이 아니다 | 방금 시작한 녹음이 곧바로 "낮음"으로 보이면 그 뒤로 이 표시를 믿을 수 없게 된다. 경고가 배경이 되는 순간 이 Phase의 목적이 사라진다 | 채택 (`ADR-0003` §16.3 · `recordingView.ts`) |
| 2026-09-07 (Phase 5.7) | **게인 조정도 정규화도 넣지 않는다. 청크 분할 · state 재생성 · 반복 차단 · VAD · 자동 언어 감지 개선도 넣지 않는다** | 이번 실패의 원인이 확인되기 전에 9/4 실험의 처방을 옮겨 오지 않는다. 정규화는 Phase Goal이 **측정 뒤에** 정하기로 못박았고 그 측정은 아직 `[미측정]`이다 | 보류 (**CANDIDATE** · §5의 C-1 ~ C-6) |
| 2026-09-07 (Phase 5.7) | **Human Review 넷을 `A-` 가정으로 올리지 않는다** | Phase 5.5의 D-4와 같은 판단이다 — 셋은 주관적 판정이고, 넷째(*다시 녹음한 회의가 읽을 만하게 전사되는가*)는 **이미 열려 있는 `A-TRANS-001`의 실질적 답이지 새 가정이 아니다** | 채택 (`docs/PHASE-5.7-HUMAN-REVIEW.md`) |

---
| 2026-09-08 | **창을 채운 상투구를 말의 속도로 지운다** (20초 · 1.0 자/초) | 문구 목록은 오탐(`"구독제로…"`)과 누락(`GGG` · `-`)을 **둘 다 실제로 냈다**. 환각이 30초 창 경계에 정확히 놓인 것이 기전을 드러냈다. 전사 3건 6,141 segment로 재서 27개를 지웠고 그중 실제 발화는 0개 (`ADR-0007` §22.4) | 채택 · **사람이 다시 돌려 확인하지 않음** |
| 2026-09-08 | **전사 입력만 증폭한다** (목표 -23 dBFS) | 저레벨 녹음(-36.8 dBFS)이 +14 dB로 고유 88.9%를 냈다. 녹음 파일은 건드리지 않는다 — 만지는 것은 파생 버퍼뿐이다 (`ADR-0003` §16.8) | 채택 · **증폭 안 한 판을 돌리지 않아 효과를 단정할 수 없음** |
| 2026-09-08 | VAD를 켰다 | `whisper-rs`가 이미 노출하고 있었고 무음 환각의 처방으로 보였다 | ⚠️ **효과를 보인 적이 없다.** 같은 파일에서 94.7% → 94.3%로 오히려 내려갔다 (`ADR-0007` §22.2) |
| 2026-09-08 | Claude(Anthropic)를 두 번째 AI provider로 | 2시간 회의가 처음으로 읽을 만하게 전사됐고, 그때 앱이 AI를 부를 수단이 하나도 없다는 것이 드러났다. 그날 회의록은 사람이 손으로 만들었다 (`PRODUCT-SPEC` §16.1) | 채택 · **한 번도 실제로 노트를 받아본 적 없음** |
| 2026-09-08 | HTTP 계약을 `notion/`에서 `net/`으로 승격 | 그 파일의 첫 줄이 처음부터 *"이 계약은 Notion을 모른다"*고 적고 있었고, 두 번째 사용자가 생겨 그 말이 자리로도 참이 됐다 | 채택 |
| 2026-09-08 | 화면 문자열을 전부 한국어로 | Rust가 내는 문장은 이미 한국어였고 화면만 영어였다. 한글 조판(`keep-all` · 행간 1.7)도 함께 맞췄다 | 채택 |
| 2026-09-08 | **온라인 회의 녹음을 범위에 넣는다** (스테레오 저장) | 마이크와 시스템 오디오가 한 IOProc으로 온다는 것을 spike가 쟀다. 스테레오면 말한 사람을 두 갈래로 구분할 수 있으나 **그것은 §15의 화자 분리가 아니다** (`PRODUCT-SPEC` §22 · `ADR-0012`) | 결정만 · **구현 없음** · 미검증 4개 |
| 2026-09-08 | **Runtime 밖에서 구현했다** | Phase 5.9 Plan이 만들어졌으나 운영자가 시간을 이유로 직접 구현으로 전환했다 | ⚠️ 그날 변경 전부가 **Verifier를 지나지 않았다** (`ADR-0007` §23 · OBS-028) |

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
