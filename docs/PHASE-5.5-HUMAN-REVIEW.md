# Phase 5.5 — Human Review 절차와 검증 기록표

```text
Status:   절차 준비됨 · **Human Review는 아직 실행되지 않았다**
Date:     2026-09-06
Phase:    Phase 5.5 — Manual AI Handoff + UI Foundation (로드맵에 없던 삽입)
Task:     TASK-065 (문서 전용)
근거:     phase-prompt/05.5-manual-ai-handoff-and-ui-foundation.md 의 Human Review 항목 ·
          docs/ADR-0010-manual-ai-handoff.md §12.4 ·
          PRODUCT-SPEC.md §9.7 · §14.5.1 · §18 · §19 · §21
```

이 문서는 두 가지다.

1. **§1~§7** — 운영자가 그대로 따라 할 수 있는 확인 절차.
2. **§8** — 그 결과를 적는 **기록표. 지금은 비어 있다.**

> ⚠️ **이 문서가 절차를 적었다는 사실은 확인이 끝났다는 뜻이 아니다.**
> §8이 비어 있는 동안 Phase 5.5를 **"화면이 검증됐다" · "Manual AI Handoff가 실제 AI
> 채팅에서 검증됐다"고 표현하지 않는다** (`docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` ·
> `docs/PHASE-4-AI-NOTE-REVIEW.md` · `docs/PHASE-5-NOTION-SMOKE-TEST.md`와 같은 규칙).

**여기 있는 항목들은 자동 검증이 답할 수 없는 것들만 모은 것이다** (PRODUCT-SPEC §18).
자동 테스트가 판정하는 것은 산출물의 문자열 · 값의 갈래 · 원문의 규칙까지이며,
**"차분한가" · "쓸 만한가" · "선택으로 읽히는가"는 사람만 답할 수 있다.**

---

## 0. 표기 — 확인한 것과 확인하지 못한 것을 섞지 않는다

| 표기 | 뜻 |
| --- | --- |
| **[E1] 저장소에서 직접 확인** | 이 문서를 쓴 Run이 저장소의 실제 파일을 읽어 확인했다. 화면 문구 · 버튼 이름 · 상수 · 경로가 여기 해당한다 |
| **[E2] 저장소 문서의 기록** | PRODUCT-SPEC · ADR-0010 · 앞선 Task의 evidence가 기록한 값 |
| **[E3] Gate 실행 결과** | 이 Phase에서 실제로 돌린 build · lint · test |
| **[E4] UNVERIFIED** | 이 Run에서 확인하지 못했다. **실행해 보면 드러난다 — 확인한 것처럼 적지 않는다** |

**이 Run에는 앱 실행도, 실제 clipboard도, 외부 AI 채팅 계정도 없었다.** 그러므로:

- **아래에 인용된 화면 문구 · 버튼 이름 · 파일 이름 형태는 전부 [E1]이다.**
  출처가 함께 적혀 있고, 그것이 이 절차가 기대는 확실한 부분이다.
- **그 문구가 실제 화면에서 어떻게 보이는지, clipboard가 실제로 동작하는지,
  AI 채팅이 무엇을 돌려주는지는 전부 [E4]다.**

---

## 1. 이 문서가 판정하는 셋 — 그리고 판정하지 않는 것

`phase-prompt/05.5`의 **Human Review 항목 세 가지**가 이 문서의 뼈대다.

| # | Human Review 항목 | 이 문서의 자리 |
| --- | --- | --- |
| **HR-1** | 화면이 실제로 **차분하고 읽기 쉬운가** | §3 |
| **HR-2** | Manual AI Handoff로 받은 결과가 **실제 외부 AI 채팅에서 쓸 만한가** | §4 |
| **HR-3** | 로컬 provider가 **"선택적 · 고급"으로 읽히는가**, 아니면 여전히 기본처럼 보이는가 | §5 |

**이 셋은 `A-` deferred assumption이 아니다** (운영자 결정 D-4 · 2026-09-04 ·
`ADR-0010` §4.2). `A-REC-001` · `A-TRANS-001` · `A-AI-001` · `A-NOTION-001`은 **외부 자원
없이는 판정 자체가 불가능한 것**이고, 이 셋은 **판정 기준이 주관적인 것**이다. 성격이 다르므로
같은 이름으로 부르지 않는다. 그러나 **미실행을 PASS로 옮겨 적지 않는 규칙은 같다.**

### 판정하지 않는 것

| 판정하지 않는다 | 왜 |
| --- | --- |
| 전사 품질 · AI 노트 품질 | Phase 3 · Phase 4 · Phase 5.6의 대상이다. **AI 채팅이 돌려준 노트의 내용이 틀렸다면 그것은 전사의 문제일 수 있다** — 2026-09-05 실측에서 제품 전사 경로가 아직 한국어를 제대로 내지 못한다는 것이 드러났다 (`PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록) |
| 최종 비주얼 디자인 | 이 Phase의 목표는 **나중의 UI/UX 다듬기가 딛고 설 바닥**까지다 (`phase-prompt/05.5` Goal) |
| Windows에서의 화면과 clipboard | Phase 6 |
| dark mode의 완성도 | 이 Phase의 목표가 아니다. **이미 있는 장치를 깨뜨리지 않는 것**까지이며, 그것은 자동 테스트가 본다 [E3] |
| 픽셀 단위 일관성 | 픽셀/스크린샷 비교는 이 Phase가 명시적으로 금지했다 (`phase-prompt/05.5` Important Rules) |

---

## 2. 준비물

### 2.1 앱을 실행한다 [E1]

저장소 루트에서:

```bash
npm install          # 처음 한 번
npm run tauri dev
```

export 파일이 쓰이는 자리는 앱 데이터 디렉터리 아래의 `exports/`다 [E1 ·
`platform/app_data_dir.rs`]. 경로를 추측하지 않고 DB 파일로 확정한다:

```bash
find "$HOME/Library/Application Support" -maxdepth 3 -name molt-note.db 2>/dev/null
APP_DATA="$HOME/Library/Application Support/com.moltnote.app"   # find가 알려준 값으로 바꾼다
```

### 2.2 상태 두 가지를 각각 만든다

**HR-2와 HR-3은 provider가 없는 상태에서 먼저 본다.** 그것이 이 Phase의 성공 기준 1이
말하는 사용자다.

| | 상태 | 어떻게 |
| --- | --- | --- |
| **S-none** | **AI provider를 하나도 고르지 않은 상태** | Settings → `AI` → `Connected provider` 의 provider 목록에서 `Not set — AI notes are off` 를 고르고 저장한다 [E1] |
| **S-local** | 로컬 provider를 고른 상태 | §5.3에서만 쓴다. 실제로 서버를 띄울 필요는 없다 — **고른 뒤 화면이 무엇을 말하는지**가 판정 대상이다 |

### 2.3 Recording 두 개

| | 무엇 | 어디에 쓰나 |
| --- | --- | --- |
| **R-ok** | **Transcript가 있는 녹음 하나.** AI Note는 없어도 된다 | §3 · §4 |
| **R-empty** | **Transcript가 아직 없는 녹음 하나** | §3.4 · §4.2 (빈 상태와 거절의 모습) |

> Transcript가 없으면 가져갈 것이 없다 — 화면이 그렇게 말한다 [E1 · `aiHandoffView.ts`]:
> *"There is nothing to send to an AI from this recording yet."*
> **이것은 실패가 아니라 정상 상태다** (INV-8).

### 2.4 외부 AI 채팅 — HR-2에만 필요하다

**최소 하나, 가능하면 서로 다른 둘.** 어느 서비스든 상관없다 — 이 제품은 특정 채팅을
가정하지 않으며, 그것이 벤더 중립의 뜻이다 (MH-6 · INV-9).

> ⚠️ **이것은 외부 서비스로 텍스트를 보내는 일이다.** 앱이 보내는 것이 아니라 **사람이
> 붙여 넣는 것**이지만(MH-3), 나가는 것은 같다. **테스트용 녹음으로 한다.**
> 개인 정보나 회의 내용이 들어간 실제 녹음을 쓰지 않는다.

---

## 3. HR-1 — 화면이 차분하고 읽기 쉬운가

**자동 테스트가 이미 본 것을 다시 보지 않는다.** 타입/여백 스케일이 정의돼 있고 화면이
그것을 쓴다는 것, `:focus` 스타일이 존재한다는 것, 새 색 토큰이 dark 블록에도 있다는 것,
gradient · glassmorphism · 과한 그림자 · 장식 애니메이션이 없다는 것, 상태를 말하는 값에
언제나 글자가 함께 있다는 것은 `tests/ui-foundation.test.ts`가 판정한다 [E3].

**여기서 보는 것은 그 규칙이 실제로 "차분하고 읽기 쉬운 화면"이 됐는가다.**

### 3.1 네 화면을 순서대로 연다

`Recordings`(홈) → `Recording` → `Recording Detail` → `Settings` [E1 · `App.tsx`].

각 화면에서 같은 네 질문에 답한다.

| # | 질문 | 무엇을 보는가 |
| --- | --- | --- |
| **3a** | **한눈에 무엇이 가장 중요한지 보이는가** | 화면마다 분명한 primary 하나. 목록에서는 제목, 녹음 화면에서는 **상태와 경과 시간**, Detail에서는 지금 보고 있는 탭의 내용 |
| **3b** | **화면 사이에 규칙이 같은가** | 제목 크기 · 본문 크기 · 여백 · 버튼 모양이 화면을 옮길 때 튀지 않는다. 같은 뜻의 것이 같은 모양이다 |
| **3c** | **조용한가** | 경쟁하는 색이 여럿 있지 않다. accent가 **주 동작 · 활성 네비게이션 · 의미 있는 상태**에만 있다. 목록의 모든 행이 무거운 카드가 아니다 (§19 · 요구 10 · 11) |
| **3d** | **읽히는가** | 거드는 글자(`--text-muted`)가 실제로 읽힌다. 긴 문장이 답답하지 않다 |

### 3.2 녹음 화면은 따로 본다 (요구 10)

```text
녹음 중이라는 사실과 경과 시간이 가장 크고 명확한가 (PRODUCT-SPEC §5-B · §19)
Record / Pause / Resume / Stop 이 무엇을 하는지 헷갈리지 않는가
보기 좋으라고 기능적 명확성이 희생된 자리가 있는가   ← 있으면 그것을 적는다
```

### 3.3 키보드만으로 한 바퀴 돈다 (요구 12)

마우스를 쓰지 않고 `Tab` · `Shift+Tab` · `Enter` · `Space`만으로:

| # | 확인 | 실패의 모습 |
| --- | --- | --- |
| **3e** | **지금 어디에 있는지 언제나 보인다** | focus가 어디 있는지 알 수 없는 자리가 하나라도 있으면 FAIL |
| **3f** | 네 화면을 오가고, 녹음을 고르고, 탭을 바꾸고, 설정을 저장할 수 있다 | 마우스 없이는 닿지 못하는 동작이 있으면 그것을 적는다 |
| **3g** | **비활성(disabled) 컨트롤이 구분된다** | 눌리지 않는데 눌릴 것처럼 보이면 FAIL |

### 3.4 빈 상태와 로딩 (요구 9)

**없는 것이 고장처럼 보이면 FAIL이다.** 다섯 갈래를 각각 만든다.

| 갈래 | 어떻게 만드나 |
| --- | --- |
| 녹음 없음 | 녹음이 하나도 없는 상태의 Recordings 화면 |
| transcript 없음 | R-empty의 Detail → `Transcript` 탭 |
| AI Note 없음 | R-ok의 Detail → `AI Note` 탭 (S-none 상태) |
| provider 없음 | Settings → `AI` (S-none 상태) |
| Notion 미설정 | Settings → `Notion` (token을 저장하지 않은 상태) |

```text
확인: 다섯 자리가 같은 모양을 쓰는가 · 경고색이나 무거운 테두리로 겁주지 않는가 ·
      무엇이 없는지와 (할 일이 있다면) 무엇을 하면 되는지가 한 줄씩 있는가
```

**로딩**: 오래 걸리는 동작(전사 시작 · AI 노트 생성 · Notion 전송 · Export for AI)에서
화면이 **얼어붙은 것처럼 보이지 않는가.** 고리만 돌고 무엇을 기다리는지 모르겠으면 FAIL이다 —
글자가 언제나 함께 있어야 한다 [E1 · `Loading.tsx`].

### 3.5 색만으로 말하지 않는가 (요구 12)

**화면을 흑백으로 보고 같은 판단이 서는지 본다.** (macOS: 시스템 설정 →
손쉬운 사용 → 디스플레이 → 색상 필터 → 그레이스케일. **[E4] 이 Run은 그 경로를 확인하지
않았다** — 화면 문구가 다르면 OS 쪽을 따른다.)

```text
성공 · 실패 · 진행 중 · 비활성이 색을 지운 뒤에도 구분되는가
구분되지 않는 자리가 하나라도 있으면 그 자리를 적는다
```

### 3.6 dark mode를 깨뜨리지 않았는가

시스템을 dark로 바꾸고 네 화면을 다시 본다. **완성도를 판정하지 않는다** — 판정하는 것은
**깨진 자리가 있는가**다 (읽을 수 없는 글자 · 사라진 경계 · 흰 판).

---

## 4. HR-2 — Manual AI Handoff의 결과가 실제 AI 채팅에서 쓸 만한가

**이 절은 S-none(provider를 하나도 고르지 않은 상태)에서 시작한다.**

### 4.1 AI Note 탭이 무엇을 보이는가 [E1 · `aiHandoffView.ts`]

R-ok의 Recording Detail → `AI Note` 탭.

```text
Have it written for you                                        ← 위 줄
  A connected AI provider turns this transcript into a structured note, right here.
  This part is optional. You can skip it and use your own AI below — nothing here is
  missing or broken.

or

Use the AI you already have                                    ← 아래 줄
  Take this recording to the AI chat you already use — copy it, or write it to a file
  you can attach.
  These three work without setting up an AI provider. Nothing has to be installed first.
  Nothing is sent anywhere from here. The text goes to your clipboard or to a file on
  this device, and you decide where it goes next. The audio file is never included.

  [ Copy AI Prompt ]  [ Copy Transcript ]  [ Export for AI ]
```

| # | 확인 | 기대 |
| --- | --- | --- |
| **4a** | 위 줄이 비어 있는 것이 **오류나 설정 요구로 보이지 않는가** | 담담한 사실로 읽힌다 (INV-8 · 요구 6). 공격적인 설정 유도가 없다 |
| **4b** | **아래 줄 셋이 전부 눌리는가** | provider가 없어도 셋 다 쓸 수 있다 (MH-1 · MH-2) |
| **4c** | 앱이 아무 데도 보내지 않는다는 사실이 **누르기 전에** 적혀 있는가 | 위 인용의 마지막 문장 (MH-3 · INV-6 · INV-5) |

### 4.2 R-empty에서는 거절한다

R-empty의 같은 탭에서 셋이 **눌리지 않고**, 왜인지가 적혀 있는지 본다 [E1]:
*"There is nothing to send to an AI from this recording yet."*
**이것은 실패가 아니라 정상 상태다.**

### 4.3 ★ clipboard가 실제로 동작하는가 — **이 Phase가 확인하지 못한 것** [E4]

> **`docs/ADR-0010` §7.4의 여섯 항목은 전부 UNVERIFIED로 남아 있다** (§12.4).
> **이 절이 그것을 처음 실제로 확인하는 자리다.**

1. `Copy AI Prompt`를 누른다.
2. 화면이 무엇을 말하는지 **그대로 적는다** — 복사됐다고 하는가, 실패라고 하는가.
3. 아무 편집기에나 붙여 넣어 **실제로 들어왔는지** 본다.
4. `Copy Transcript`로 같은 것을 한다.

| 결과 | 무엇을 기록하나 |
| --- | --- |
| **성공** | 화면 문장 · 붙여 넣은 내용이 온전한가 (특히 긴 전사에서 **끝까지** 들어왔는가) |
| **실패** | 화면 문장 그대로 · 그 옆에 **다른 길(Export for AI)이 함께 보이는가** · 같은 버튼으로 **다시 시도할 수 있는가** [E1 · `copyView.ts` · `clipboard.ts`] |

> **실패해도 그것은 이 Phase의 실패가 아니다.** §7.5의 설계가 그 경우를 전제로 했고,
> Export for AI가 대체 경로로 남는다. **다만 그 사실이 그대로 기록돼야 한다** —
> 그래야 §7.3의 B(Rust command) 또는 C(Tauri 플러그인)로 갈아 끼울 근거가 생긴다.

**실패했다면 개발자 도구 콘솔이 아니라 화면이 말한 문장을 적는다.** 앱은 실패를 console로
흘리지 않으며, 거절 값의 이름(`NotAllowedError` 계열인지)은 실패의 `detail`에 실려 있다
[E1 · `clipboard.ts`의 `rejectedFailure`]. **그 이름이 §6의 [E4] 하나를 지운다.**

### 4.4 Export for AI — 파일 하나

1. `Export for AI`를 누른다.
2. 화면이 **실제로 쓰인 파일 경로**를 보여 준다 [E1].
3. 그 파일을 연다.

```bash
ls "$APP_DATA/exports/"          # …-ai-request.md 가 있다 [E1 · AI_REQUEST_MARKER]
```

| # | 확인 | 기대 [E1 · `export/ai_request.rs`] |
| --- | --- | --- |
| **4d** | 파일 이름 | `<날짜>-<제목 슬러그>-ai-request.md`. **같은 녹음의 Markdown export와 이름이 구분된다** |
| **4e** | 첫 줄 | `# Molt Note AI Request` — §11의 export(`# <제목>`)와 첫 줄에서 구분된다 |
| **4f** | 섹션 | `## Mode` → `## Instructions` → `## Recording` → `## Transcript` 순서 |
| **4g** | **오디오** | `audio_path` · `audio_format` · 오디오 파일 이름이 **어디에도 없다** (INV-6 · MH-4) |
| **4h** | **벤더 이름** | 어떤 AI 서비스의 이름도 **없다** (MH-6) |
| **4i** | 두 번 누르면 | 파일이 **하나 더 생긴다.** 있던 파일이 덮어써지지 않는다 (ADR-0009 §4.3) |

### 4.5 ★ 실제 AI 채팅에 넣는다 — HR-2의 핵심

**세 산출물은 서로 다른 쓰임이다. 셋을 각각 한 번씩 한다.**

| 경로 | 무엇을 하나 |
| --- | --- |
| **P-1** | `Copy AI Prompt`로 복사한 것을 **한 번에 붙여 넣는다.** 지시와 전사가 한 덩어리로 들어 있으므로 그것으로 끝나야 한다 |
| **P-2** | `Copy Transcript`를 붙여 넣고 **자기 지시를 직접 쓴다.** 전사 텍스트가 그 용도로 깨끗한가 |
| **P-3** | `Export for AI`로 만든 `.md` 파일을 **첨부한다** (첨부를 받지 않는 채팅이면 내용을 붙여 넣는다) |

각 경로에서 mode를 바꿔 가며 본다 — `Meeting` · `Study` · `Summary`는 **요구하는 출력
섹션이 다르다** [E1 · §9.5].

**무엇을 판정하는가:**

| # | 질문 | 기대 |
| --- | --- | --- |
| **4j** | 채팅이 **요청을 이해했는가** | 무엇을 해 달라는 것인지 되묻지 않는다 |
| **4k** | **Markdown으로 답했는가** | JSON 덩어리도, code fence로 감싼 것도 아니다 (프롬프트가 그렇게 요구한다) |
| **4l** | **섹션 제목이 요구한 그대로인가** | Meeting `Overview · Key Discussions · Decisions · Action Items · Open Questions` / Study `Overview · Key Concepts · Important Details · Questions · Things to Study · References Mentioned` / Summary `Short Summary · Key Points` [E1] |
| **4m** | **그 답이 이 앱이 만드는 문서와 같은 모양인가** | 같은 모양이면 사용자가 그것을 그대로 자기 노트에 붙일 수 있다. 이것이 §6.3 설계의 핵심 성질이다 |
| **4n** | **길이가 문제가 되지 않는가** | 전사가 잘리거나 채팅이 거절하지 않는가. 거절했다면 **몇 분짜리 전사에서 그랬는지** 적는다 |
| **4o** | **★ 그래서 그 노트가 실제로 쓸 만한가** | 이것이 HR-2다. 다른 항목이 전부 통과해도 **읽어서 쓸모가 없으면 FAIL이다** |

> **모델이 Markdown 계약을 어겨도 앱에서 깨지는 것은 없다** — 이 경로에 파서가 없다
> (`ADR-0010` §6.5). 그러므로 4k · 4l의 실패는 **오류가 아니라 품질 문제**이며,
> 고칠 자리는 프롬프트의 출력 계약 문장 하나다. 그 사실을 §8에 적는다.

**가능하면 서로 다른 채팅 둘에서 같은 것을 한다.** 한 채팅에서만 잘 되는 프롬프트는
벤더 중립이 아니다.

---

## 5. HR-3 — 로컬 provider가 "선택적 · 고급"으로 읽히는가

**이 판정은 문장을 세는 것이 아니라 인상을 보는 것이다.** 그러나 무엇을 보고 판단하는지는
적어 둔다.

### 5.1 Settings의 `AI` 구역을 위에서 아래로 읽는다 [E1 · `aiProviderSettings.ts`]

```text
AI
  AI notes are optional. Recording, transcription, Markdown export, and Notion all work
  with nothing set here.
  With nothing set here you can still take a recording to the AI chat you already use —
  those buttons are on the recording itself.

  Connected provider
    A provider connected here writes notes for you inside the app. Leaving it unset is a
    normal state, not something to fix.
    [ provider 목록 · 주소 · 연결 확인 · 모델 선택 ]        ← Phase 4가 만든 그대로다

  <provider 이름> · local · optional                        ← 뒤쪽에 온다
    Optional and advanced. It is one way to fill the provider above, not something the
    app needs.
    You install and run it on this machine yourself. Molt Note does not install it, does
    not start it, and does not need it to work.
    To use it: install and start it yourself, then choose it in the provider list above
    and save. …
```

| # | 질문 | 기대 |
| --- | --- | --- |
| **5a** | 이 구역을 처음 보는 사람이 **"이건 안 해도 되는 거구나"** 로 읽는가 | 맨 앞 문장이 그것을 말한다 |
| **5b** | 로컬 provider 부분이 **기본값처럼 보이지 않는가** | 뒤쪽에 있고, 제목에 `optional`이 붙어 있고, **누를 버튼도 편집할 값도 없다** |
| **5c** | **켜라는 요구가 아니라 켜는 방법으로 읽히는가** | *"To use it: …"* — 명령이 아니라 안내다 |
| **5d** | 고르지 않은 상태가 **결함처럼 보이지 않는가** | *"It is not the provider right now. Nothing in the app is waiting for it."* |
| **5e** | **지워진 것처럼 보이지도 않는가** | 고르고 · 확인하고 · 모델을 고르는 경로가 **위 부분에 그대로 있다** (MH-8). 재배치이지 삭제가 아니다 |

### 5.2 AI Note 탭에서 다시 본다

§4.1의 두 줄을 다시 보되, 이번에는 **위계**만 본다.

```text
5f  위 줄(연결된 provider)이 아래 줄(내 AI로 하기)보다 "진짜 방법"처럼 보이는가?
    → 그렇게 보이면 이 Phase의 목적이 절반만 달성된 것이다. 그 인상을 그대로 적는다.
```

### 5.3 로컬 provider를 골라 본다 (S-local)

Settings에서 목록의 로컬 provider를 고르고 저장한다. **서버를 띄우지 않아도 된다.**

| # | 확인 | 기대 |
| --- | --- | --- |
| **5g** | 뒤쪽 부분의 문장이 바뀐다 | *"It is the provider right now, so the controls above check it and list its models."* [E1] |
| **5h** | 연결 확인을 눌렀을 때 | **응답하지 않는다는 사실이 오류가 아니라 상태로 말해진다** — *"The AI provider did not answer."* + 어떻게 하면 되는지 [E1] |
| **5i** | 그 상태에서 **다른 설정이 그대로 저장되는가** | AI가 안 되는 것이 나머지를 막지 않는다 (INV-8) [E1 · `AI_SETTINGS_UNAFFECTED_NOTICE`] |

**끝나면 S-none으로 되돌린다** (provider 목록에서 `Not set — AI notes are off`).

---

## 6. 이 Phase가 확인하지 **않은** 것 — 정본

**아래는 Phase 5.5가 확인하지 않았다. 확인한 것처럼 적지 않는다.**
(원본은 `docs/ADR-0010-manual-ai-handoff.md` §7.4 · §12.4.)

| | 확인하지 않은 것 | 어디서 확인되나 |
| --- | --- | --- |
| [E4] | Tauri v2 webview(WKWebView · WebView2)에서 `navigator.clipboard.writeText`가 이 앱의 origin에서 **실제로 동작하는지** | §4.3 |
| [E4] | 그 API가 요구하는 조건(secure context · 사용자 제스처)이 이 앱의 창에서 어떻게 판정되는지 | §4.3 |
| [E4] | 거절할 때 **어떤 예외/거절 값이 오는지** | §4.3 (실패의 `detail`에 이름이 남는다) |
| [E4] | Tauri v2 core가 clipboard를 노출하는지, 플러그인이 필요한지 | 후보 A가 막혔을 때만 필요해진다 (`ADR-0010` §7.3) |
| [E4] | clipboard 플러그인의 crate/npm 이름 · 호환 버전 · permission 식별자 | 같음 |
| [E4] | **Windows에서의 clipboard와 화면 전반** | Phase 6 |
| [E4] | **사람이 산출물을 실제 외부 AI 채팅에 붙여 넣었을 때 무엇이 돌아오는지** | §4.5 |
| [E4] | 화면이 실제로 차분하고 읽기 쉬운지 | §3 |
| [E4] | 로컬 provider가 선택적·고급으로 읽히는지 | §5 |

**자동 검증이 확인한 것과 섞지 않는다** [E3]:

```text
VERIFIED (자동)   세 산출물의 문자열이 결정론적이고 기대와 같다 · 프롬프트 상수가 고쳐지지
                  않았다 · 산출물에 오디오도 벤더 이름도 없다 · current Transcript만 쓰인다 ·
                  실패가 저장된 것을 바꾸지 않는다 · Export for AI가 실물 파일을 만든다 ·
                  기존 Connected Provider 경로가 그대로 통과한다 · UI 기반 다섯 검사
                  (자동 테스트 1,238개 · build · lint · test 전부 exit 0)

UNVERIFIED        위 표 전부
```

---

## 7. 실패했을 때 무엇을 기록하는가

### 7.1 기록 양식

```text
항목:            HR-1 / HR-2 / HR-3 중 무엇 · 어느 소항목(3a … 5i)
무엇을 했나:     화면 · 눌렀던 버튼 · 그 직전 상태 (S-none인가 S-local인가)
화면이 말한 것:  화면 문장 그대로
무엇이 어색했나: 인상을 문장으로. **"그냥 별로"는 다음 사람이 쓸 수 없다**
재현되는가:      같은 조작을 한 번 더 했을 때
```

### 7.2 어디에 적는가

- **이 문서의 §8** — Human Review 실행 기록.
- 결과가 **제품 결함**이면 그 내용을 그대로 남기고, 고치는 것은 Runtime의 Task가 한다.
- 결과가 **Runtime / Planner의 문제**로 보이면 `docs/LOOP-RUNTIME-FIELD-NOTES.md`다.
  **제품 문제를 Runtime 문제로 적지 않는다** (`CLAUDE.local.md` — Field Note Quality).
- **§4.3의 clipboard 결과는 §6의 [E4] 항목을 지우거나 확정하는 값이다.** 그 결과가 나오면
  `ADR-0010` §12.4를 갱신하는 것은 그때의 Task이며, **이 문서가 대신 적지 않는다.**

> ⚠️ **개인 정보를 적지 않는다.** 실제 회의 내용 · 사람 이름 · AI 채팅의 계정 정보는
> 판정에 필요하지 않다. 필요하면 `<가림>`으로 대신한다.

---

## 8. Human Review 실행 기록 — 운영자가 채운다

**아래 표가 비어 있는 동안 Phase 5.5는 "사람이 확인했다"가 아니다.**

```text
실행 날짜:        (비어 있음)
실행자:           (비어 있음)
앱 버전 / 커밋:   (비어 있음)
OS / 기기:        (비어 있음)
사용한 AI 채팅:   (비어 있음)
```

### 8.1 HR-1 — 화면이 차분하고 읽기 쉬운가

| 항목 | 무엇을 판정하는가 | 결과 | 메모 |
| --- | --- | --- | --- |
| **3a** | 한눈에 무엇이 중요한지 보인다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3b** | 화면 사이에 규칙이 같다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3c** | 조용하다 — 경쟁하는 색이 없다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3d** | 거드는 글자가 읽힌다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3e** | 보이는 focus — 어디 있는지 언제나 안다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3f** | 키보드만으로 주요 동작을 할 수 있다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3g** | 비활성 컨트롤이 구분된다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3h** | 빈 상태 다섯이 고장처럼 보이지 않는다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3i** | 로딩이 얼어붙은 것처럼 보이지 않는다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3j** | 그레이스케일에서도 상태가 구분된다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **3k** | dark mode에 깨진 자리가 없다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **HR-1 종합** | **화면이 차분하고 읽기 쉬운가** | ☐ PASS ☐ FAIL ☐ 미실행 | |

### 8.2 HR-2 — Manual AI Handoff의 결과가 쓸 만한가

| 항목 | 무엇을 판정하는가 | 결과 | 메모 |
| --- | --- | --- | --- |
| **4a** | provider 부재가 오류로 보이지 않는다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **4b** | provider 없이 세 동작이 전부 눌린다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **4c** | 아무 데도 보내지 않는다는 사실이 미리 보인다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **4d~4i** | Export for AI 파일의 이름 · 첫 줄 · 섹션 · 오디오 부재 · 벤더 부재 · 덮어쓰지 않음 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **4j** | 채팅이 요청을 이해했다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **4k** | Markdown으로 답했다 (JSON도 code fence도 아니다) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **4l** | 섹션 제목이 요구한 그대로다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **4m** | 그 답이 이 앱이 만드는 문서와 같은 모양이다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **4n** | 길이가 문제가 되지 않는다 | ☐ PASS ☐ FAIL ☐ 미실행 | 전사 길이 = |
| **4o** | **★ 받은 노트가 실제로 쓸 만하다** | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **HR-2 종합** | **Manual AI Handoff가 실제 AI 채팅에서 쓸 만한가** | ☐ PASS ☐ FAIL ☐ 미실행 | |

**§4.3 clipboard — 이 Phase가 처음 확인하는 자리** (판정 항목이 아니라 **관측 기록**이다):

| 관측 | 결과 | 그대로 적는다 |
| --- | --- | --- |
| `Copy AI Prompt`가 실제로 clipboard에 썼는가 | ☐ 됐다 ☐ 안 됐다 ☐ 미실행 | 화면 문장: |
| `Copy Transcript`가 실제로 clipboard에 썼는가 | ☐ 됐다 ☐ 안 됐다 ☐ 미실행 | 화면 문장: |
| 실패했다면 어떤 갈래였는가 | ☐ 쓸 수 없다 ☐ 거절됐다 ☐ 해당 없음 | 실패 `detail`의 이름: |
| 실패했을 때 Export for AI가 대체 경로로 보였는가 | ☐ 보였다 ☐ 아니다 ☐ 해당 없음 | |
| 긴 전사가 **끝까지** 붙여 넣어졌는가 | ☐ 그렇다 ☐ 아니다 ☐ 미실행 | |

### 8.3 HR-3 — 로컬 provider가 선택적·고급으로 읽히는가

| 항목 | 무엇을 판정하는가 | 결과 | 메모 |
| --- | --- | --- | --- |
| **5a** | 이 구역이 "안 해도 되는 것"으로 읽힌다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **5b** | 로컬 provider 부분이 기본값처럼 보이지 않는다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **5c** | 켜라는 요구가 아니라 켜는 방법으로 읽힌다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **5d** | 고르지 않은 상태가 결함처럼 보이지 않는다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **5e** | **지워진 것처럼도 보이지 않는다** (재배치이지 삭제가 아니다) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **5f** | AI Note 탭에서 두 줄의 위계가 한쪽을 "진짜 방법"으로 만들지 않는다 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **5g~5i** | 골랐을 때의 문장 · 응답 없음의 표현 · 다른 설정의 독립성 | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **HR-3 종합** | **선택적 · 고급으로 읽히는가** | ☐ PASS ☐ FAIL ☐ 미실행 | |

### 8.4 자유 기술

어색했던 점 · 헷갈렸던 문구 · 느렸던 자리 · "이건 아닌데" 싶었던 것:

```text
(비어 있음)
```

---

**통과한 항목만 통과했다고 적는다. 미실행을 PASS로 옮겨 적지 않는다.**
