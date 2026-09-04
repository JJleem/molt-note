# Phase 4 — 운영자 AI Note Human Review 절차와 기록표

```text
Status:   절차 준비됨 · Human Review는 아직 실행되지 않았다
          **이 Phase에서 실제 Ollama를 호출한 적이 한 번도 없다** (§10.2)
Date:     2026-09-04
Phase:    Phase 4 — AI Provider System + Local AI
Task:     TASK-042 (문서 전용)
근거:     phase-prompt/04-ai-provider-system.md "Human Review 항목" ·
          docs/ADR-0008-note-ai-provider.md §16.3 · §17.3 ·
          PRODUCT-SPEC §9 · §12 · §13 · §14.5 · §18
```

이 문서는 두 가지다.

1. **§1~§8** — 운영자가 그대로 따라 할 수 있는 Human Review 절차.
2. **§9~§10** — 이 Phase가 **확인한 것과 확인하지 않은 것**을 구분하는 기록표.

> ⚠️ **이 문서가 절차를 적었다는 사실은 review가 실행됐다는 뜻이 아니다.**
> §9의 기록표가 비어 있는 동안 Phase 4를 **"로컬 모델로 쓸 만한 노트가 나온다"고 표현하지
> 않는다.** 이 문서의 어떤 문장도 실측 결과를 담고 있지 않다.

---

## 0. 표기 — 확인한 것과 확인하지 못한 것을 섞지 않는다

| 표기 | 뜻 |
| --- | --- |
| **[E1] 저장소에서 직접 확인** | 이 문서를 쓴 Run이 저장소의 실제 파일을 읽어 확인했다. 파일 경로를 함께 적는다 |
| **[E2] 저장소 문서의 기록** | PRODUCT-SPEC / ADR이 기록한 값. 확인 시점은 **2026-09-01**이며 이 Run이 다시 확인한 것이 아니다 |
| **[E4] UNVERIFIED** | 이 Run에서 확인하지 못했다. 실행해 보면 드러난다 — 확인한 것처럼 적지 않는다 |

**이 Run에는 네트워크 접근도, 앱 실행도, Ollama 실행도 없었다.** 그러므로:

- **Ollama를 설치하고 모델을 받는 구체적 명령은 [E4]다.** 이 문서는 그것을 지어내지 않고,
  공식 배포처에서 확인하라고만 적는다 (§2.2).
- 앱 안의 화면 문구 · 버튼 이름 · 실패 종류 · 기본 주소 · 계산된 문턱값은 전부 **[E1]**이며
  출처 파일을 함께 적었다.
- **노트의 품질 · 처리 시간 · 언어 처리 결과에 대한 값은 이 문서에 하나도 없다.** 그것을
  적는 것은 §9를 채우는 운영자의 실행이다.

---

## 1. 이 review가 판정하는 것 — 그리고 판정하지 않는 것

`phase-prompt/04-ai-provider-system.md`의 Human Review 항목 **다섯 개가 전부**다.

```text
R-1  실제 스터디/회의 녹음에 대해 로컬 모델이 만든 노트가 **읽을 가치가 있는가**
R-2  로컬 모델이 **한국어 + 영어 혼용** transcript를 다룰 수 있는가
R-3  **1시간 분량** transcript 처리 시간이 실사용 가능한 수준인가
R-4  **Meeting과 Study** mode의 출력이 실제로 서로 다른 유용성을 갖는가
R-5  provider가 **로컬인지 외부인지**가 사용자에게 분명한가
```

> R-1이 **이 Phase의 실질적 성공 조건**이며, 자동으로 판정할 수 없기 때문에 사람이 본다
> (ADR-0008 §16.3).

### 1.1 이것은 자동 검증의 재실행이 아니다

아래는 이미 자동 테스트가 판정했고 **이 review의 대상이 아니다** (§10.1).

| 대상이 아니다 | 어디서 이미 판정됐나 |
| --- | --- |
| 응답이 깨졌을 때 앱이 죽지 않는가 | `src-tauri/src/ai/note.rs`의 파싱 테스트 · `tests/ai_failure_recovery.rs` |
| 실패해도 Transcript · 오디오가 온전한가 | `src-tauri/tests/ai_note_run.rs` · `tests/ai_failure_recovery.rs` |
| provider 없이 녹음 · 전사 · 열람이 도는가 (INV-8) | `src-tauri/tests/core_pipeline_without_ai.rs` · `src/screens/coreWithoutAi.test.ts` |
| 오디오가 나갈 경로가 없는가 (INV-6) | `src-tauri/tests/audio_never_reaches_ai.rs` |
| provenance 일곱 값이 저장되는가 | `src-tauri/tests/ai_note_run.rs` |

**여기서 보는 것은 "동작하는가"가 아니라 "쓸 만한가"다.**

### 1.2 그러므로 다음은 실패가 아니다

- 노트의 문장이 어색하다 → **품질 관찰이지 통합 실패가 아니다.** §9에 그대로 적는다.
- 한 번 `The model answered in a shape this app could not read.`가 떴다 → 다시 생성해 본다.
  로컬 소형 모델에서 흔한 일이며 그래서 이 실패는 재시도 가능이다 (ADR-0008 §13.2).
  **몇 번 만에 읽을 수 있는 답이 나왔는지를 적는 것**이 관찰이다.
- 오래 걸린다 → R-3의 관찰 대상이다. 숫자를 적는다. 앱은 생성에 시간 제한을 두지 않는다
  [E1 · `src-tauri/src/ai/ollama/network.rs` — 연결 타임아웃 5초뿐이다].

---

## 2. 준비물 — **앱은 Ollama를 설치하지도 번들하지도 않는다**

### 2.1 이것이 전제다

```text
Molt Note는 Ollama를 installer에 넣지 않는다.
Molt Note는 Ollama를 띄우지도 죽이지도 않는다.
Molt Note는 사용자가 이미 실행 중인 Ollama에 연결만 한다.
```

[E2 · PRODUCT-SPEC §14.5 · §15 · `phase-prompt/04` 요구 7 · Out of Scope ·
ADR-0008 §15] — 그래서 아래 준비는 전부 **운영자가 앱 밖에서** 하는 일이며, 앱 안에는
"Ollama 설치" 같은 버튼이 없다 [E1 · `src/screens/SettingsScreen.tsx`].

이 review 중에 **Ollama는 계속 실행되어 있어야 한다.** 앱이 대신 켜 주지 않으므로, 서버가
꺼져 있으면 앱은 "The AI provider did not answer."를 보여 준다 [E1 ·
`src/screens/aiProviderSettings.ts`] — 그것은 앱의 고장이 아니다.

### 2.2 Ollama를 준비한다 — 명령을 지어내지 않는다

| 항목 | 값 | 등급 |
| --- | --- | --- |
| 배포처 | `github.com/ollama/ollama` (공식 저장소) · 공식 배포 페이지 | [E2 · §14.5] |
| 이 저장소가 기록한 최신 릴리스 | **v0.33.2 (2026-08-27)** — *오늘 기준 최신인지는 [E4]* | [E2] |
| 기본으로 듣는 주소 | **`http://localhost:11434`** (기본 bind `127.0.0.1:11434`) | [E2 · VERIFIED 2026-09-01] |
| 미실행일 때 | TCP 연결 거부 | [E2] |

> **설치 명령과 실행 명령을 이 문서에 적지 않는다.** 이 Run은 그것을 확인하지 못했고 [E4],
> 틀린 명령을 적으면 운영자가 그것을 확인된 값으로 읽는다. 공식 배포처의 현재 안내를 따른다.

**서버가 떠 있는지 확인하는 방법은 두 가지다.**

1. **앱으로 확인한다 (권장).** §3의 `Check the AI provider` 버튼이 그 일을 한다 — 앱이
   실제로 하는 것과 같은 호출(`GET /api/tags`)이므로, 이 버튼이 답하면 앱 경로 전체가
   답한 것이다 [E1 · `src-tauri/src/ai/ollama/wire.rs`].
2. **직접 확인한다.** 기본 주소로 헬스 체크가 있다 [E2 · §14.5 — `GET /` → 200
   `"Ollama is running"`].

   ```bash
   curl -s http://localhost:11434/
   curl -s http://localhost:11434/api/tags
   ```

### 2.3 모델을 고른다 — **어느 것이 적합한지는 이 문서가 정하지 않는다**

`phase-prompt/04`가 R-2("한국어+영어 혼용")를 Human Review로 남긴 이유가 이것이다.
저장소가 기록한 후보는 셋이며, **셋 중 무엇이 이 제품의 transcript에 맞는지는 UNVERIFIED다**
[E2 · PRODUCT-SPEC §14.5].

| 모델 | 저장소가 기록한 사실 | 등급 |
| --- | --- | --- |
| `exaone3.5` (LG AI Research) | **영어/한국어 이중언어를 명시.** 2.4B~32B | [E2] |
| `kanana` (Kakao) | 한국어/영어 이중언어 계열. 라이브러리 존재는 VERIFIED, **정확한 크기 태그는 UNVERIFIED** | [E2 · E4] |
| `qwen3` | 다국어. 0.6b(523MB) · 1.7b(1.4GB) · 4b(2.5GB) · 8b(5.2GB) · 14b(9.3GB) · 30b(19GB) · 32b(20GB) | [E2] |

**최소 두 개를 받아 두기를 권한다.** R-1과 R-2는 "이 모델이 좋다/나쁘다"가 아니라
**"이 제품에 쓸 만한 로컬 모델이 있는가"**를 묻기 때문에, 하나만 보면 모델 문제와 제품 문제를
가를 수 없다.

> ⚠️ **크기를 고를 때 §7을 먼저 읽는다.** 이 앱은 요청 context를 **16384로 고정**하며 사용자가
> 키울 수 없다 (ADR-0008 §17.3.2). 모델을 더 큰 것으로 바꿔도 앱이 보내는 예산은 그대로다.

### 2.4 전사가 먼저다 — A-TRANS-001

```text
ASSUMPTION A-TRANS-001
Phase 3의 local transcription은 구현됐고 자동 검증을 통과했지만,
**실제 Whisper 추론은 아직 한 번도 실행되지 않았다.**
```

[E2 · ADR-0007 §16.3.1 · PRODUCT-SPEC §14.4.4 · `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` §11]

**AI Note는 Transcript가 있어야 만들어진다** [E1 · `src-tauri/src/ai/run.rs` —
`current_transcript_id`가 없으면 `NoTranscriptYet`이다]. 그러므로 이 review는
`docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`의 smoke test **뒤에** 온다.

```text
Recording → Stop → 실제 Whisper 전사 → Transcript 표시 → **여기서부터 이 문서**
```

> **Whisper 단계가 실패하면 이 review로 넘어가지 않는다.** 먼저 전사 문제를 고친다 —
> `PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`가 그 순서를 명시한다
> (`Recording → Stop → 실제 Whisper 전사 → Transcript 표시 → AI Note 생성`).

### 2.5 녹음 재료 — R-1 · R-2 · R-3이 요구하는 것이 다르다

| 무엇을 위해 | 필요한 녹음 | 왜 |
| --- | --- | --- |
| **R-1 · R-4** | 실제 회의 하나 + 실제 스터디/강의 하나. **각각 10~20분이면 된다** | 지어낸 대본이 아니라 실제 발화여야 "읽을 가치"를 물을 수 있다 |
| **R-2** | 한국어 안에 영어 용어가 섞인 것 (기술 회의·강의가 대개 그렇다) | 혼용은 별도로 준비하는 것이 아니라 **R-1의 녹음이 이미 그럴 것**이다. 아니라면 하나 더 만든다 |
| **R-3** | **1시간 분량 하나** | 이 항목만 길이가 판정 대상이다. §7을 먼저 읽는다 |

---

## 3. 앱에서 provider를 설정한다 (Settings 화면)

아래 문구는 전부 실제 화면 문자열이다 [E1 · `src/screens/SettingsScreen.tsx` ·
`src/screens/aiProviderSettings.ts`].

1. 앱을 실행한다. 저장소 루트에서:

   ```bash
   npm install          # 처음 한 번
   npm run tauri dev
   ```

2. 왼쪽에서 **Settings** 를 연다. **AI Provider** 그룹을 찾는다.

3. **Provider** 를 고른다.

   ```text
   Not set — AI notes are off      ← 기본값. 이 상태가 오류가 아니다 (INV-8)
   Ollama — runs on this device    ← 이 review에서 고르는 것
   ```

   > **테스트 전용 fake provider는 이 목록에 없다** [E1 — 선택지는 위 둘뿐이다].
   > 목록에 그것이 보인다면 그 자체가 결함이며 §8에 적는다.

4. **전송 경계 문구 세 줄이 나타나는지 본다 — 이것이 R-5의 관찰 지점이다** (§5.5).

   ```text
   This provider runs on this device.
   The transcript text is sent to it and stays on this device.
   Audio is never sent. Only the transcript text is used, and the recording file stays on this device.
   ```

   provider를 고르기 전에는 대신 이 한 줄이 있다:
   `No AI provider is set, so nothing is sent anywhere.`

5. **Address (host and port)** — 기본 주소(`http://localhost:11434`)를 쓸 것이면
   **비워 둔다.** placeholder가 `Leave empty to use the built-in address`다.
   Ollama를 다른 곳에 띄웠다면 그 주소를 넣는다.

6. 화면 아래의 **Save** 를 누른다. `Saved.` 가 보이면 저장된 것이다.

   > **Save를 먼저 눌러야 한다.** 연결 확인은 편집 중인 값이 아니라 **저장된 값**을 묻는다 —
   > 화면도 그렇게 적어 둔다: `The check asks the settings that are already saved. Save first
   > to check a new provider, address, or model.` [E1]

7. **Check the AI provider** 를 누른다. 버튼이 `Checking…`이 되었다가 네 결과 중 하나가 된다.

   | 화면 문구 | 뜻 | 다음 |
   | --- | --- | --- |
   | `The AI provider answered.` | 서버가 응답했고 모델이 있다 | 8로 간다 |
   | `The AI provider answered, but no models are installed on it.` | 서버는 살아 있는데 모델이 없다 | §2.3에서 모델을 받고 다시 확인한다 |
   | `The AI provider did not answer.` | 서버가 응답하지 않는다 | Ollama를 켠다. 주소를 바꿨다면 Save 후 다시 확인 |
   | `The AI provider could not be checked.` | 확인 자체가 실패했다 | 문장을 그대로 §8에 적는다 |

8. **Model** 을 고른다. 목록은 **서버가 보고한 것**이다 [E1 — 앱이 지어내지 않는다].
   고르지 않은 상태에서는 `No model chosen yet. Pick one from the list the provider reported.`
   가 보인다.

9. 다시 **Save** 를 누른다. `Saved.`

> **AI 설정이 어떻게 끝나든 다른 설정은 그대로 저장된다** — 화면이 그 사실을 적어 둔다
> (`Every other setting on this screen still saves normally, whether or not the AI provider
> answers.`) [E1]. 그렇지 않다면 그것은 INV-8 위반이며 §8에 적는다.

---

## 4. 노트를 만든다 (Recording Detail 화면)

1. **Recordings** 목록에서 §2.5의 녹음을 연다.
2. **AI Note** 탭을 연다. 탭은 세 개다: `AI Note` · `Transcript` · `Recording` [E1].
3. mode 를 고른다 — **Meeting · Study · Summary** 세 버튼. 각 버튼 아래에 그 mode가 만드는
   섹션이 적혀 있다 [E1 · `src/screens/aiNoteView.ts`].

   ```text
   Meeting   Overview · Key Discussions · Decisions · Action Items · Open Questions
   Study     Overview · Key Concepts · Important Details · Questions · Things to Study · References Mentioned
   Summary   Short Summary · Key Points
   ```

4. **`Generate Meeting note`**(고른 mode에 따라 이름이 바뀐다)를 누른다.
5. 기다린다. 화면이 **2초마다 상태를 다시 묻는다** [E1 ·
   `src/screens/RecordingDetailScreen.tsx` — `AI_NOTE_REFRESH_MS = 2_000`].
   **화면을 떠나도 생성은 계속된다.**
6. 끝나면 노트가 섹션 구조로 그려지고, 아래에 provenance 한 줄이 붙는다 [E1].

   ```text
   Meeting · ollama · <모델 이름> · prompt v1.meeting.5c6b8a90 · <생성 시각> · transcript <id>
   ```

   - `prompt v1.meeting.…`의 hash8은 프롬프트와 schema에 묶여 있다 (ADR-0008 §10).
   - **재생성은 대체가 아니라 추가다.** 다시 누르면 `Generate Meeting note again`이며,
     새 행이 하나 더 생긴다. 화면은 그중 가장 최근 것을 보여 준다 (ADR-0008 §9.2).
7. 같은 녹음에 대해 **Meeting과 Study를 둘 다** 만든다 — R-4가 그 둘을 비교한다.

> **한 번에 한 건이다.** 생성 중에 다른 시작을 누르면 거절된다 (`이미 이 녹음의 노트를 만들고
> 있다.` 또는 `다른 녹음의 노트를 만들고 있다. 그것이 끝난 뒤에 시작할 수 있다.`) [E1 ·
> `src-tauri/src/commands/notes.rs`]. 이것은 결함이 아니라 설계다 (ADR-0008 §17.3.5).

---

## 5. 관찰 항목 다섯 — 무엇을 보고 무엇을 적는가

**판정은 PASS / FAIL 둘로 몰지 않는다.** 다섯 항목 모두 정도의 문제이므로 아래 세 값 중
하나로 적고, **그렇게 판단한 근거를 한두 문장으로 함께 적는다.** 근거 없는 판정은 기록이
아니다.

```text
USABLE        이대로 V1에 쓸 수 있다
BORDERLINE    쓸 수는 있는데 조건이 붙는다 (그 조건을 적는다)
NOT USABLE    이 상태로는 쓸 수 없다 (무엇이 문제인지 적는다)
```

### 5.1 R-1 — 노트가 읽을 가치가 있는가 (이 Phase의 실질적 성공 조건)

**대상**: §2.5의 실제 회의 녹음 하나 + 실제 스터디 녹음 하나. mode는 각각 Meeting · Study.

**무엇을 보는가** — "잘 썼는가"가 아니라 **"Transcript를 다시 읽는 것보다 나은가"**다.

| 질문 | 어떻게 판단하나 |
| --- | --- |
| 이 노트만 읽고 그 회의/수업에서 무슨 일이 있었는지 떠오르는가 | Transcript를 보지 않은 상태에서 노트만 읽어 본다 |
| **없는 것을 지어냈는가** | `Decisions` · `Action Items` · `References Mentioned`를 Transcript와 대조한다. 프롬프트가 지어내지 말라고 지시하지만 [E1 · `ai/prompt.rs`], 지켜지는지는 UNVERIFIED다 |
| 빈 배열이 정직하게 비어 있는가 | 결정이 없었던 회의에서 `Decisions`가 비어 있으면 **정상이다.** 화면은 `Nothing was recorded in this section.`으로 그린다 [E1] |
| 몇 번 만에 읽을 수 있는 답이 나왔는가 | `The model answered in a shape this app could not read.`가 몇 번 떴는지 센다 |

**적는 것**: 판정 + 근거 + `aiResponseUnusable` 재시도 횟수 + 지어낸 항목이 있었다면 그 항목.

### 5.2 R-2 — 한국어 + 영어 혼용을 다룰 수 있는가

**대상**: 한국어 발화에 영어 용어가 섞인 녹음.

| 질문 | 어떻게 판단하나 |
| --- | --- |
| 노트가 **transcript의 주 언어**로 나왔는가 | 프롬프트가 "Write the note in the main language of the transcript."를 요구한다 [E1 · `ai/prompt.rs`]. 한국어 회의인데 영어 노트가 나오면 그대로 적는다 |
| 영어 용어가 뭉개지지 않았는가 | 고유명사·기술 용어가 음차되거나 사라졌는지 본다 |
| 한 노트 안에서 언어가 오락가락하는가 | 섹션마다 언어가 바뀌면 적는다 |

**적는 것**: 판정 + 어느 모델에서 본 결과인지 + 실제 문장 예시 한두 개(그대로 옮긴다).

> **모델을 최소 둘 비교한다** (§2.3). 하나만 보고 "로컬 모델은 혼용을 못 한다"고 적지 않는다.

### 5.3 R-3 — 1시간 분량 처리 시간이 실사용 가능한가

**⚠️ 이 항목은 §7을 먼저 읽는다.** 1시간 transcript는 앱의 고정 context 예산에 걸려
**요청이 아예 나가지 않을 수 있다.** 그 경우도 이 항목의 결과이며, 시간 대신 그 사실을 적는다.

| 재는 것 | 방법 |
| --- | --- |
| **생성 시작 → 노트 표시까지의 벽시계 시간** | `Generate …` 를 누른 시각과 노트가 보인 시각. 화면이 2초마다 갱신하므로 오차는 그 정도다 |
| 그동안 앱이 멈췄는가 | 생성 중에 **Recordings 목록으로 가고 Settings를 열어 본다.** 멈추면 그것은 결함이며 §8에 적는다 (`commands/notes.rs`는 자물쇠를 쥐지 않는다 [E1]) |
| 기기가 어떤 상태였나 | 모델 크기 · 기기 · 다른 무거운 작업이 함께 돌았는지 |

**적는 것**: 초 단위 소요 시간 + 모델 + 기기 + transcript 길이(문자 수) + 판정.

> **비교 기준을 하나 둔다.** 같은 녹음의 Summary mode도 한 번 재 두면 "1시간이 느린 것"과
> "이 모델이 원래 느린 것"을 가를 수 있다.

### 5.4 R-4 — Meeting과 Study의 출력이 실제로 다른 유용성을 갖는가

**대상**: **같은 녹음 하나**에 대해 Meeting과 Study를 둘 다 생성한다 (§4-7).
같은 입력이어야 차이가 mode에서 왔다고 말할 수 있다.

| 질문 | 어떻게 판단하나 |
| --- | --- |
| 두 노트가 **서로 다른 것을 뽑았는가** | `Key Discussions` vs `Key Concepts`, `Action Items` vs `Things to Study`를 나란히 읽는다 |
| 회의 녹음에 Study를 걸면 어색한가, 아니면 쓸 만한가 | 어색해도 된다 — **다르다는 것 자체가 확인 대상**이다 |
| 섹션 이름만 다르고 내용이 같은가 | 같다면 mode 구분이 값을 못 하고 있는 것이다. 그대로 적는다 |

**적는 것**: 판정 + 두 노트에서 실제로 갈린 지점 한두 개 + 갈리지 않았다면 그 사실.

### 5.5 R-5 — provider가 로컬인지 외부인지가 분명한가

**대상**: Settings 화면 (§3-3 · §3-4). **노트를 만들지 않고도 판정할 수 있는 유일한 항목이다.**

| 질문 | 어디를 보나 |
| --- | --- |
| 선택지 자체가 로컬/외부를 말하는가 | `Ollama — runs on this device` |
| 고른 뒤 전송 경계 세 줄이 보이는가 | §3-4의 세 문장 |
| **오디오가 나가지 않는다**는 것이 보이는가 | `Audio is never sent. …` — 이 문장은 locality와 무관하게 언제나 같다 [E1] |
| 고르지 않았을 때도 분명한가 | `No AI provider is set, so nothing is sent anywhere.` |
| 노트에 무엇이 만들었는지 남는가 | §4-6의 provenance 줄에 `ollama`와 모델 이름이 있다 |

**적는 것**: 판정 + 문구가 모호하다고 느낀 자리(그 문구를 그대로 옮긴다).

> 이 항목은 **문구가 아니라 이해**를 묻는다. 화면이 사실을 적었더라도 사용자가 그것을
> 놓친다면 그것이 관찰 결과다.

---

## 6. 실패했을 때 — 실패 종류를 가르는 표

앱은 실패를 **무엇이 실패했는가 · 원본은 안전한가 · 다시 시도할 수 있는가**로 보여 준다
[E1 · `src/screens/aiNoteView.ts` · `FailureNotice.tsx`]. 문장을 요약하지 말고 그대로 옮긴다.

| 실패 종류 | 화면이 말하는 것 | 재시도 | 무엇을 하면 되는가 |
| --- | --- | --- | --- |
| `aiProviderNotConfigured` | **오류로 그리지 않는다.** `AI notes are off.` | — | Settings에서 provider와 **모델**을 고른다. 모델을 고르지 않은 것도 이 실패다 (ADR-0008 §17.3.4) |
| `aiProviderUnreachable` | `The AI provider did not answer. Start it, then try again.` | **가능** | Ollama를 켜고 다시 누른다 |
| `aiModelUnavailable` | `The chosen model is not installed on the AI provider. Install it or choose another model in Settings, then try again.` | 불가 | 모델을 받거나 다른 모델을 고른다 |
| `aiResponseUnusable` | `The model answered in a shape this app could not read. Generating again often gives a usable answer.` | **가능** | 다시 생성한다. **몇 번 만에 됐는지 센다** (§5.1) |
| `aiInputTooLarge` | `This transcript is longer than the model can take in one request, so nothing was sent. …` | 불가 | **§7을 읽는다** |
| `aiRequestFailed` | 요청이 거절되거나 처리되지 못했다 | 원인에 따라 다르다 | 화면 문장과 `detail`(`HTTP <코드>`)을 그대로 적는다 |

**어느 실패에서도 다음이 참이어야 한다** — 아니라면 그것은 결함이며 §8에 적는다.

```text
Transcript 탭이 그대로 열린다
재생이 그대로 된다
이미 만들어진 노트가 그대로 있다
녹음 파일이 그대로 있다
```

---

## 7. ⚠️ 1시간 분량과 고정 context 예산 — R-3 전에 반드시 읽는다

**앱은 요청 context를 16384 토큰으로 고정하며, 사용자가 그 값을 키울 수 없다.**
(ADR-0008 §17.3.2 — 결정된 설정 항목 `ai_context_tokens`가 구현되지 않았다.)

계산은 상수에서 나온다 [E1 · `src-tauri/src/ai/prompt.rs`]:

```text
입력에 쓸 수 있는 예산 = 16384 − 1024(프롬프트 예약) − 1536(출력 예약) = 13,824 토큰
크기 추정             = 문자 수 ÷ 2 (올림, 보수적 과대추정)
따라서 대략 27,648자를 넘는 transcript는 요청조차 나가지 않는다
```

**§14.5의 추정(1시간 ≈ 8~12K 단어 / 12~20K+ 토큰)에 비추면 1시간 분량이 이 문턱에 걸릴 수
있다** [E2 — 추정이며 측정값이 아니다]. **실제로 걸리는지는 [E4]이며, R-3이 처음 관측한다.**

### 걸렸을 때 — 이것은 실행 실패가 아니라 관측 결과다

```text
화면:  This transcript is longer than the model can take in one request, so nothing was sent.
        A model with a larger context window, or a shorter recording, is needed.
```

> ⚠️ **이 안내의 앞쪽 절반은 지금 구현에서 효과가 없다.** 예산이 모델과 무관하게 고정이므로
> **더 큰 context의 모델을 골라도 달라지지 않는다** (ADR-0008 §17.3.2).
> 실제로 남는 수단은 **더 짧은 녹음**뿐이다.

**그러면 무엇을 하는가.**

1. **여기서 코드를 고치지 않는다.** review는 판정하는 자리이지 고치는 자리가 아니다.
2. §9의 R-3 칸에 다음을 적는다.

   ```text
   R-3 결과: BLOCKED — aiInputTooLarge
   transcript 문자 수:
   추정 토큰(문자수÷2):
   화면에 보인 문장 전체:
   ```

3. **R-3을 시간으로 판정할 수 없었다는 사실**을 그대로 남긴다. "느리다"도 "빠르다"도 적지
   않는다 — 재지 못했기 때문이다.
4. 대신 **들어가는 길이를 찾아 둔다.** 30분 · 15분으로 줄여 가며 어디까지 되는지 재면,
   후속 Task가 예산을 열 때 그 값이 근거가 된다.

---

## 8. 결과를 어디에 어떤 형식으로 남기는가

### 8.1 관찰 하나마다 이 양식으로 적는다

```text
날짜 / 시각        :
녹음               : 종류(회의 / 스터디) · 길이(분) · transcript 문자 수
언어               : 한국어 / 영어 / 혼용 (혼용이면 대략의 비율)
Ollama             : 버전 · 실행 방식(직접 실행 / 서비스)
모델               : 이름과 태그 그대로 (예: exaone3.5:7.8b)
주소               : 기본값 그대로인가 / 다른 주소인가  ← **값 자체는 적지 않아도 된다**
mode               : meeting / study / summary
소요 시간          : 시작 시각 → 노트 표시 시각 (초)
재시도 횟수        : aiResponseUnusable이 몇 번 떴는가
실패했다면         : 실패 종류 + 화면 문장 전체(headline · message · 원본 안전 문장 · detail)
provenance 줄      : 화면에 보인 그대로
노트 본문          : 판단 근거가 되는 부분을 그대로 인용 (요약하지 않는다)
판정               : USABLE / BORDERLINE / NOT USABLE + 근거 한두 문장
```

> **provider 설정값(host · port)을 evidence 파일과 공개 기록에 적지 않는다** —
> `phase-prompt/04`의 Important Rules이며, 앱도 그것을 실패 문장에 넣지 않는다
> (ADR-0008 §11.3). "기본값" / "다른 주소"로 충분하다.

### 8.2 어디에 적는가

| 무엇이 문제였나 | 어디로 |
| --- | --- |
| **다섯 항목의 판정 결과** | **이 문서의 §9 기록표** — 그것이 이 문서가 존재하는 이유다 |
| **제품 문제** (노트가 비어 나온다 · 화면이 잘못 그린다 · 실패가 다른 종류로 보인다 · 생성 중 앱이 멈춘다) | Phase 4의 후속 Task. `docs/LOOP-RUNTIME-FIELD-NOTES.md`에 적지 **않는다** |
| **설계 결함** (§7의 고정 예산처럼, 코드는 의도대로인데 그 의도가 문제인 것) | `docs/ADR-0008-note-ai-provider.md` §17에 사실로 덧붙인다 |
| **절차 문제** (이 문서의 문구·버튼 이름이 실제와 다르다) | 이 문서를 고친다 |
| **Loop Runtime 문제** (Gate · Verifier · Worker · Plan 진행) | `docs/LOOP-RUNTIME-FIELD-NOTES.md` (`CLAUDE.local.md`의 Field Note Quality 규칙) |

### 8.3 하지 않는 것

- **실행하지 않은 항목을 추정으로 채우지 않는다.** 안 했으면 `NOT RUN`이다.
- **한 번 본 것을 일반화하지 않는다.** 모델 하나 · 녹음 하나의 결과는 그 모델 그 녹음의
  결과다. §9에 조건을 함께 적는다.
- **`docs/SYSTEM-MAP.md`를 이 review로 고치지 않는다.** 그것은 Phase가 최종 DONE이 된 뒤
  운영자의 일이다 (`CLAUDE.local.md`).

---

## 9. Human Review 실행 기록 — 운영자가 채운다

**아직 실행되지 않았다.** 아래의 `NOT RUN`을 실제 결과로 바꾸는 것은 이 문서를 쓴 Task가
아니라 **운영자의 실행**이다.

### 9.1 실행 환경

| | 값 |
| --- | --- |
| 실행 날짜 | *(NOT RUN)* |
| 실행자 | *(NOT RUN)* |
| Ollama 버전 | *(NOT RUN)* |
| 시험한 모델 (전부) | *(NOT RUN)* |
| 기기 (CPU / RAM / GPU) | *(NOT RUN)* |
| 앱 실행 방식 | *(NOT RUN — dev / 번들)* |
| 선행 조건: 실제 Whisper 전사가 성공했는가 | *(NOT RUN — `PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` §11 참조)* |

### 9.2 다섯 항목

| | 판정 | 근거 |
| --- | --- | --- |
| **R-1** 노트가 읽을 가치가 있는가 | **NOT RUN** | *(NOT RUN)* |
| **R-2** 한국어 + 영어 혼용 | **NOT RUN** | *(NOT RUN)* |
| **R-3** 1시간 분량 처리 시간 | **NOT RUN** | *(NOT RUN — §7의 문턱에 걸렸다면 그 사실을 적는다)* |
| **R-4** Meeting과 Study의 유용성 차이 | **NOT RUN** | *(NOT RUN)* |
| **R-5** 로컬/외부가 분명한가 | **NOT RUN** | *(NOT RUN)* |

### 9.3 그 밖에 처음 관측되는 것들

이 review가 실행되면 **저장소가 지금까지 확인하지 못한 값들이 처음으로 관측된다.**
관측했으면 여기에 적고, 관측하지 못했으면 `NOT RUN`으로 둔다.

| 항목 | 지금 상태 | 관측값 |
| --- | --- | --- |
| 실제 Ollama가 `format`(JSON Schema)을 존중하는가, 무시하는가 | UNVERIFIED (ADR-0008 §14.2 항목 16) | *(NOT RUN)* |
| **생성 응답에서 본문 텍스트가 담기는 필드 이름** | UNVERIFIED (§14.3) — 구현은 이름에 의존하지 않는 경로를 쓴다 (§17.1.2) | *(NOT RUN)* |
| 모델이 없을 때 생성 요청이 돌려주는 상태 코드 | UNVERIFIED (§14.2 항목 15) — 구현은 목록으로 먼저 판정한다 | *(NOT RUN)* |
| 16384 토큰을 이 기기·이 모델이 감당하는가 | UNVERIFIED (§14.2 항목 21) | *(NOT RUN)* |
| 첫 응답이 그대로 JSON으로 파싱되는 비율 | UNVERIFIED | *(NOT RUN)* |

**다섯 항목이 채워지기 전까지 Phase 4를 "로컬 모델로 쓸 만한 노트가 나온다"고 적지 않는다.**

---

## 10. Phase 4가 확인한 것 / 확인하지 않은 것

**확인하지 않은 것을 PASS로 적지 않는다** (PRODUCT-SPEC §20.2).

### 10.1 자동 검증이 확인한 것 (VERIFIED)

판정 수단이 저장소 안에 있고, **실제 Ollama도 실제 네트워크도 없이** 재실행할 수 있는
것들이다 [E1 · 파일 목록].

| 항목 | 판정 수단 |
| --- | --- |
| 세 mode의 structured note schema와 저장 봉투가 왕복한다 | `src-tauri/src/ai/note.rs` |
| 기대와 다른 응답(산문 · 코드펜스 · 필드 누락 · 타입 불일치 · 빈 본문)에서 죽지 않는다 | 같음 · `src-tauri/tests/ai_failure_recovery.rs` |
| 프롬프트나 schema가 바뀌면 `promptVersion`이 반드시 바뀐다 | `src-tauri/src/ai/prompt.rs`의 `prompt_version_is_bound_to_the_prompt_text` |
| 같은 계약을 Ollama adapter와 결정론적 test double이 **둘 다** 통과한다 | `src-tauri/src/ai/testing.rs` · `src-tauri/tests/ollama_adapter.rs` |
| 여섯 AI 실패가 서로 다른 종류로 화면에 도달하고 frontend union과 1:1이다 | `src-tauri/src/domain/failure.rs` · `tests/ipc-boundary.test.ts` |
| 재생성이 **추가**이며 기존 노트·Transcript·오디오를 건드리지 않는다 | `src-tauri/tests/ai_note_run.rs` |
| provider가 없어도 녹음 · 전사 · 열람이 정상 동작한다 (INV-8) | `src-tauri/tests/core_pipeline_without_ai.rs` · `src/screens/coreWithoutAi.test.ts` |
| 오디오가 AI로 나갈 경로가 코드 수준에 없다 (INV-6) | `src-tauri/tests/audio_never_reaches_ai.rs` |
| 벤더 지식이 `ai/ollama/` 밖으로 나가지 않는다 (INV-9) | `src-tauri/tests/ollama_adapter.rs` |

### 10.2 ⚠️ 확인되지 않은 것 — 실행되지 않았다

```text
ASSUMPTION A-AI-001                                      (운영자 기록 · 2026-09-04)

Phase 4의 AI Provider 경계와 Ollama adapter는 구현됐고 자동 검증을 통과했다.
그러나 실제 Ollama에 요청이 나간 적은 한 번도 없다.

따라서 다음은 UNVERIFIED다 — 엔드포인트·요청 필드·응답 형태가 오늘의 Ollama와
실제로 맞는가, 로컬 모델이 이 schema를 지킬 수 있는가, 노트가 읽을 가치가 있는가.

`A-REC-001`(실제 마이크) · `A-TRANS-001`(실제 Whisper 추론)과 같은 성격이며,
같은 이유로 Final Integration의 hard human gate로 연기됐다
(`phase-prompt/Goal.md` · `docs/SYSTEM-MAP.md` §7).

해소 절차: 이 문서 §2 ~ §8. 해소 기록: 이 문서 §9의 기록표.
```

| 항목 | 상태 | 어디서 판정되는가 |
| --- | --- | --- |
| **실제 Ollama를 한 번이라도 호출했는가** | **없다 — 이 Phase에서 한 번도 일어나지 않았다** [E4] | 이 문서 §3~§5. 자동 검증은 `ollama::testing::StubServer`로만 돈다. 소켓을 여는 파일(`ai/ollama/network.rs`)은 Gate가 **컴파일**할 뿐 실행하지 않는다 |
| **실제 Whisper 추론** (`ASSUMPTION A-TRANS-001`) | **여전히 유효하다 — 실행된 적 없다** | `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` §11 · ADR-0007 §16.3.1. 이 Phase의 모든 입력은 fixture Transcript였다 |
| 로컬 모델이 만든 노트의 **품질** | **NOT RUN** | §5.1 (R-1) |
| **한국어 + 영어 혼용** 처리 | **NOT RUN** | §5.2 (R-2) |
| **1시간 분량 처리 시간** | **NOT RUN** | §5.3 (R-3). §7의 문턱 때문에 시간을 재지 못할 수 있다 |
| Meeting / Study 출력의 유용성 차이 | **NOT RUN** | §5.4 (R-4) |
| 로컬/외부 구분이 사용자에게 분명한가 | **NOT RUN** | §5.5 (R-5) |
| §14.5 엔드포인트·파라미터 이름의 **오늘 기준** 재확인 | **[E2] 2026-09-01 그대로** | ADR-0008 §14.3 · §17.3.6 — 네트워크 도구가 세 Run 연속 거부됐다 |
| `ai_context_tokens` 설정 | **미구현 — 결정만 있다** | ADR-0008 §17.3.2 · 이 문서 §7 |
| Windows에서의 동작 | **DEFERRED — Phase 6** | `phase-prompt/04` Out of Scope |
| Cloud provider (Claude · Gemini · Groq) | **DEFERRED** | PRODUCT-SPEC §16 · ADR-0008 §15 |

> **이 표의 어느 칸도 "아마 될 것이다"로 채우지 않는다.** 실행되지 않은 것은 실행되지
> 않은 것이며, 이 문서가 대신 판정하지 않는다 (ADR-0008 §16.3).
