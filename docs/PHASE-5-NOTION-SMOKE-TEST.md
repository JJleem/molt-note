# Phase 5 — 운영자 Notion smoke test 절차와 검증 기록표

```text
Status:   절차 준비됨 · smoke test는 아직 실행되지 않았다
Date:     2026-09-04
Phase:    Phase 5 — Notion & Markdown Export
Task:     TASK-054 (문서 전용)
근거:     phase-prompt/05-notion-and-export.md 의 Human Review 항목 ·
          docs/ADR-0009-notion-and-export.md §15.5 · PRODUCT-SPEC §10 · §11 · §14.9 · §18
```

이 문서는 두 가지다.

1. **§1~§10** — 운영자가 그대로 따라 할 수 있는 **실제 Notion 워크스페이스** smoke test 절차.
2. **§11~§12** — 이 Phase가 **확인한 것과 확인하지 않은 것**을 구분하는 기록표.

> ⚠️ **이 문서가 절차를 적었다는 사실은 smoke test가 실행됐다는 뜻이 아니다.**
> §11의 실행 기록이 비어 있는 동안 Phase 5를 **"실제 Notion 전송이 검증됐다"고 표현하지
> 않는다** (ADR-0009 §15.5 · `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`와 같은 규칙).

**자동 테스트는 실제 Notion API를 한 번도 호출하지 않고, 실제 OS 자격증명 저장소를 한 번도
건드리지 않는다** (PRODUCT-SPEC §18 · ADR-0009 §10.2). 그것이 이 문서가 있는 이유다 —
**여기 있는 항목들은 자동 검증이 답할 수 없는 것들만 모은 것이다.**

---

## 0. 표기 — 확인한 것과 확인하지 못한 것을 섞지 않는다

| 표기 | 뜻 |
| --- | --- |
| **[E1] 저장소에서 직접 확인** | 이 문서를 쓴 Run이 저장소의 실제 파일을 읽어 확인했다. 화면 문구 · 경로 · 상수 · 명령이 여기 해당한다 |
| **[E2] 저장소 문서의 기록** | PRODUCT-SPEC · ADR-0009 · 앞선 Task의 evidence가 기록한 값 |
| **[E4] UNVERIFIED** | 이 Run에서 확인하지 못했다. **실행해 보면 드러난다 — 확인한 것처럼 적지 않는다** |

**이 Run에는 네트워크 접근도, 앱 실행도, Notion 계정도 없었다.** 그러므로:

- **Notion 웹/앱의 화면 절차(§3.1 · §3.2), NotebookLM이 받는 파일 형식(§7), Obsidian의
  동작(§7)은 전부 [E4]다.** 그 제품들의 UI는 이 Run이 보지 못했고, 바뀔 수도 있다.
- **저장소 안의 화면 문구 · 버튼 이름 · 경로 · 상수는 전부 [E1]이며** 아래에 출처를 함께
  적었다. 그것이 이 절차가 기대는 유일한 확실한 부분이다.

---

## 1. ★ token 규칙 — 실제 값을 문서에도 evidence에도 남기지 않는다

**이 절이 절차보다 먼저 있는 이유는, 이 규칙을 어기는 것이 되돌릴 수 없기 때문이다.**
커밋된 secret은 지워도 히스토리에 남는다.

### 1.1 절대 하지 않는 것

| 하지 않는다 | 이유 |
| --- | --- |
| **이 문서(또는 어떤 저장소 문서)에 실제 integration token을 적는다** | 저장소는 커밋된다. `phase-prompt/05` Important Rules · ADR-0009 §10.5 |
| **`.loop/evidence/**` 에 token을 적는다** | evidence도 커밋된다. ADR-0009 §10.5의 표에 명시돼 있다 |
| **token이 보이는 스크린샷을 저장소에 넣는다** | 같은 이유. 입력란은 `type="password"`라 화면에는 가려지지만 [E1 · `SettingsScreen.tsx`], 클립보드 관리자 · 터미널 히스토리 · 붙여넣기 전 편집기 화면은 가려 주지 않는다 |
| **`.env` · 소스 · 테스트 fixture · 셸 히스토리에 token을 둔다** | 앱에는 환경변수로 token을 읽는 경로 자체가 없다 (ADR-0009 §10.5). 만들지 않는다 |
| **로그 · 오류 출력을 확인 없이 통째로 붙여넣는다** | 설계상 token이 실릴 자리는 없지만(§1.3), **붙여넣기 전에 눈으로 확인하는 것이 마지막 방어선이다** |

### 1.2 기록해도 되는 것

```text
적어도 된다   PASS/FAIL · 화면에 나온 문장 · Failure 종류(kind) · 상태 코드 ·
              sentChunks / totalChunks · 걸린 대략의 시간 · 만들어진 파일 경로의 파일명 부분
적지 않는다   integration token · 그 일부 · 그 앞뒤 몇 글자
```

**부모 페이지 식별자와 워크스페이스 이름은 secret이 아니다** — 어디에 쓰는지일 뿐이고
SQLite에도 평문으로 저장된다 (ADR-0009 §8.4 · §10.5). 다만 개인 워크스페이스의 정보이므로
기록이 꼭 필요하지 않으면 `<parent page id>`처럼 가려도 된다. **판정에 필요한 것이 아니다.**

### 1.3 앱이 이미 막고 있는 것 — 그래도 확인은 사람이 한다

[E1]:

- **token을 되돌려 주는 command가 없다.** 화면이 아는 것은 "저장돼 있다"는 사실(`boolean`)
  뿐이며, 입력란에 저장된 값이 다시 채워지는 일이 없다 (`NotionConnection.tokenStored`).
  그래서 **앱 화면에서 token을 복사할 방법 자체가 없다.**
- 요청 헤더와 응답은 `Debug`로 값을 내지 않고, `TransportError`에는 문자열이 없다.
- `Failure`의 문장과 detail은 상태 코드 · Notion 오류 코드 · 진행도만 담는다.

**그래도 evidence에 무엇을 적기 전에 한 번 읽어 본다.** 규칙은 앱이 아니라 사람이 지킨다.

### 1.4 테스트가 끝난 뒤

- 설정 화면의 **`Remove the saved token`** 으로 지운다 [E1].
- 남아 있는지 확인하려면 macOS **Keychain Access**에서 서비스 **`molt-note`** · 계정
  **`notion-integration-token`** 항목을 본다 [E1 · `platform/secret_store.rs`].
  **항목의 값을 열어 보지 않는다.** 있는지 없는지만 본다.
- 가능하면 **테스트 전용 Notion 워크스페이스(또는 전용 부모 페이지)** 로 한다. 이 테스트는
  페이지를 **만들고**, 마지막 항목(§9)에서는 **일부러 하나 더 만든다.**

---

## 2. 이 smoke test가 판정하는 것 — 그리고 판정하지 않는 것

`phase-prompt/05`의 **Human Review 항목 세 가지**가 이 문서의 뼈대다.

| # | Human Review 항목 | 이 문서의 자리 |
| --- | --- | --- |
| **HR-1** | 생성된 Notion 페이지가 **실제로 읽을 만한 구조**인가 (§10의 섹션 구성이 살아 있는가) | §5 (PASS-1 · PASS-2) |
| **HR-2** | **긴 transcript가 Notion에서 실제로 온전한가** | §6 (PASS-3) |
| **HR-3** | export된 Markdown을 **Obsidian / NotebookLM에서 열었을 때 쓸 만한가** | §7 (PASS-4) |

여기에 Phase Goal의 Verification Boundary가 요구하는 두 가지를 더한다.

| # | 항목 | 자리 |
| --- | --- | --- |
| **PASS-5** | Notion 실패 시 **local data가 온전하고** 실패가 보이며 재시도할 수 있다 (INV-3) | §8 |
| **PASS-6** | **중복 sync 정책의 실제 동작**이 문서(ADR-0009 §15.4)와 같다 | §9 |

그리고 이 절차를 지나는 것만으로 **처음 실제로 확인되는 것**이 둘 있다 (ADR-0009 §15.5):

```text
macOS 자격증명 저장소에 실제로 저장 · 조회 · 삭제가 되는가        →  §4
실제 https://api.notion.com 으로 TLS 핸드셰이크가 서는가          →  §4 (connection test)
```

### 판정하지 않는 것

| 판정하지 않는다 | 왜 |
| --- | --- |
| 전사 품질 · AI 노트 품질 | Phase 3 · Phase 4의 대상이며, 이 문서의 판정 기준이 아니다 |
| 전송 속도 · 소요 시간 | 60KB 예산은 보수적이므로 긴 문서는 여러 요청이 된다. **느린 것은 실패가 아니다** (ADR-0009 §14) |
| Windows 동작 | Phase 6 |
| 실제 마이크 품질 · 장시간 녹음 안정성 | 이 문서의 대상이 아니다 |

---

## 3. 준비물

### 3.1 Notion integration과 token — **[E4] 절차 화면은 이 Run이 확인하지 않았다**

필요한 것은 하나다.

```text
integration token 하나   —  이 앱이 Authorization: Bearer <token> 으로 보낸다
                            (ADR-0009 §5.2 · [E2 · PRODUCT-SPEC §14.9.1])
```

Notion 쪽 화면에서 integration을 만들고 token을 얻는 절차는 **Notion의 UI이며 이 Run이
보지 못했다 [E4].** 개발자 설정(integrations)에서 internal integration을 하나 만들고 그
secret을 복사한다 — **화면 문구가 이 문서와 다르면 Notion 쪽을 따른다.**

> ⚠️ 복사한 값은 **아직 어디에도 붙여넣지 않는다.** 곧바로 §4의 입력란으로 간다.
> 중간에 편집기 · 메모 · 채팅에 붙여넣지 않는다 (§1.1).

### 3.2 부모 페이지 — 만들고, **integration에 공유하고**, 식별자를 얻는다 [E4]

이 앱은 **고른 부모 페이지 아래에 페이지를 만든다** (`parent.page_id` · ADR-0009 §5.1).

1. Notion에서 빈 페이지를 하나 만든다 (예: `Molt Note smoke test`).
2. **그 페이지를 §3.1의 integration에 공유한다.** 이 단계를 빠뜨리면 전송은
   **`404 object_not_found`** 계열로 실패한다 — 앱은 그것을 "보낼 위치를 찾지 못했다"로
   말한다 [E1 · `notion/client.rs`]. 실패 자체가 정상 동작이므로, §8에서 일부러 다시 쓴다.
3. 그 페이지의 **식별자**를 얻는다. 앱 화면의 안내 문구가 같은 말을 한다 [E1]:

   > *Open the Notion page you want new pages to be created under, share it with your
   > integration, and paste its page identifier here.*

   식별자를 URL에서 어떻게 읽는지는 Notion 쪽 규약이며 **[E4]** 다. 붙여넣은 값이 틀리면
   §4의 connection test 또는 첫 전송이 실패로 알려 준다 — **틀린 채로 조용히 성공하는
   경로는 없다.**

### 3.3 앱을 실행한다 [E1]

저장소 루트에서:

```bash
npm install          # 처음 한 번
npm run tauri dev
```

앱 데이터 디렉터리는 앱이 정한다. **경로를 추측하지 않고 DB 파일로 확정한다** [E1 ·
`platform/app_data_dir.rs` · `DATABASE_FILE_NAME`]:

```bash
find "$HOME/Library/Application Support" -maxdepth 3 -name molt-note.db 2>/dev/null
APP_DATA="$HOME/Library/Application Support/com.moltnote.app"   # find가 알려준 값으로 바꾼다
```

```text
<APP_DATA>/
├── molt-note.db
├── recordings/
├── models/
└── exports/        ← Markdown export가 여기에 쓴다 (없으면 export할 때 만들어진다) [E1]
```

### 3.4 Recording 두 개를 준비한다

| | 무엇 | 어디에 쓰나 |
| --- | --- | --- |
| **R-short** | 짧은 녹음 하나. **Transcript가 있어야 한다.** AI Note는 있어도 되고 없어도 된다 | §5 · §8 · §9 |
| **R-long** | **1시간 분량의 transcript가 있는 녹음 하나** | §6 |
| **R-noai** | **AI Note가 없는** 녹음 하나 (R-short가 AI Note 없이 만들어졌다면 그것을 그대로 쓴다) | §5.4 (INV-8) |

**Transcript가 없으면 보낼 것이 없다** — 화면이 그렇게 말한다 [E1]:
*"There is nothing to send from this recording yet." / "Transcribe this recording in the
Transcript tab first, then send it."* 이것은 **실패가 아니라 정상 상태다** (INV-8).

R-long을 만드는 것은 Phase 3의 전사 경로이며 이 문서의 판정 대상이 아니다. 1시간짜리 실제
전사가 아직 없다면, **먼저 그것을 만든다** — HR-2는 그것 없이는 판정할 수 없고,
**짧은 녹음으로 대신했다고 적지 않는다.**

---

## 4. 설정 — token 저장 · destination 저장 · connection test

화면: **Settings → `Notion` 그룹** [E1 · `SettingsScreen.tsx`].

### 4.1 절차

1. **`Integration token`** 입력란에 §3.1의 값을 붙여넣는다.
   화면이 미리 말한다 [E1]: *"The token is handed to this app once and kept in the operating
   system credential store. It is cleared from this box as soon as it is saved, and it is
   never written to the app database or the browser."*
2. **`Save the token`** 을 누른다. → **입력란이 비워진다.** 그것이 정상이다.
3. **`Parent page (destination)`** 에 §3.2의 식별자를 붙여넣는다.
4. 화면 아래의 **`Save`** 를 누른다. destination은 secret이 아니라 **다른 설정과 같은 폼
   값**이며 화면 전체의 Save 하나가 저장한다 [E1 · ADR-0009 §8.4].
   → 순서가 중요하다. 화면이 그렇게 적어 두었다 [E1]: *"The check uses the token and the
   parent page that are already saved. Save first to check a new parent page."*
5. **`Check the Notion connection`** 을 누른다.

### 4.2 PASS-0 — 무엇을 보면 통과인가

```text
연결됨 · 워크스페이스 이름이 보인다 (Notion이 이름을 말해 줬을 때)
```

이 한 번의 성공이 **동시에 세 가지를 처음으로 증명한다** (ADR-0009 §15.5):

| 증명되는 것 | 왜 여기서 증명되는가 |
| --- | --- |
| **실제 TLS 핸드셰이크가 선다** | `webpki-roots`의 번들 루트로 `https://api.notion.com`에 실제로 붙었다는 뜻이다. 자동 테스트는 소켓을 열지 않는다 |
| **macOS 자격증명 저장소에서 token을 꺼내 쓸 수 있다** | 확인 요청은 **저장된** token으로 나간다. 저장이 안 됐거나 읽지 못하면 요청 자체가 나가지 않는다 |
| **`GET /v1/users/me` 계약이 맞다** | connection test가 쓰는 호출이다 (ADR-0009 §5.1) |

### 4.3 ★ 앱을 껐다 켜고 한 번 더 확인한다 — 이것이 Keychain 판정이다

1. 앱을 완전히 종료한다.
2. 다시 실행한다 (`npm run tauri dev`).
3. Settings → **`Check the Notion connection`** 을 **token을 다시 입력하지 않고** 누른다.

**여기서 연결되면 token이 실제로 OS 자격증명 저장소에 남아 있다는 뜻이다.** 프로세스
메모리에만 있었다면 이 단계에서 "아직 token을 저장하지 않았다"로 돌아온다.

### 4.4 실패 갈래 — 무엇이 다른지 화면이 구분해서 말한다 [E1]

| 화면이 말하는 것 | 뜻 | 무엇을 고치나 |
| --- | --- | --- |
| 아직 token을 저장하지 않았다 (`notConfigured`) | 요청이 **나가지도 않았다** | §4.1-1·2 |
| Notion이 연결을 거절했다 — token을 다시 입력해야 한다 | `401`/`403` 계열 | §3.1의 token |
| 보낼 위치를 찾지 못했다 — 부모 페이지를 integration에 공유했는지 확인 | `403`/`404` 계열 | §3.2-2 · §3.2-3 |
| 네트워크로 닿지 못했다 | 연결 실패 · 타임아웃 | 네트워크. **TLS 구성 문제도 이 모습으로 보인다** (ADR-0009 §15.2.3) |

**설정 화면의 나머지는 Notion이 어떻게 답하든 그대로 저장된다** (INV-8) [E1].

---

## 5. 짧은 Recording 하나를 보낸다 — **HR-1** (PASS-1 · PASS-2)

화면: **Recording Detail** (R-short) [E1 · `RecordingDetailScreen.tsx`].

### 5.1 보내기 전에 화면이 무엇을 말하는가 (INV-5 · INV-6) [E1]

```text
What is sent to Notion
  · The title, date, and length of this recording
  · The transcript text
  · The AI note, when this recording has one
The audio file is never sent. Only this text leaves this device, and it goes to Notion only.
```

**이 문장이 화면에 실제로 보이는지 확인한다.** 전송되는 데이터가 UI에 드러나야 한다는 것이
요구사항이다 (INV-5).

### 5.2 PASS-1 — 실제 페이지가 만들어진다

1. **`Send to Notion`** 을 누른다.
2. 진행 중: *"Sending to Notion… This keeps running in the background, so you can leave this
   screen."*
3. 끝나면: *"This recording is in Notion."* 와 **진행도 문장** —
   *"All N parts of this document are on that page."*
4. **Notion에서 확인한다**: §3.2의 부모 페이지 아래에 **새 페이지 하나**가 생겼다.

```text
PASS-1 = 부모 페이지 아래에 페이지가 하나 생겼고, 화면이 done 상태와 진행도를 보인다.
```

### 5.3 PASS-2 — 그 페이지가 §10의 구조를 갖는가 (**HR-1의 핵심**)

**페이지 제목은 문서의 첫 `# h1`이다** — 앱은 `properties.title`을 보내지 않는다
(ADR-0009 §5.3). 그러므로 **페이지 제목이 Recording 제목과 같아야 한다.**

확인 항목 (PRODUCT-SPEC §10 · §11 · §9.5 · [E1 · `export/markdown.rs`]):

| 확인 | 기대 |
| --- | --- |
| 제목 | 페이지 제목이 Recording 제목이다 (제목이 비어 있으면 `제목 없음`) |
| 메타 | `Date: YYYY-MM-DD` 와 `Duration: 52:31`(분:초. 한 시간이 넘으면 `1:02:03`) 한 덩어리 |
| AI Note 섹션 (있을 때) | 노트 타입에 따라 **아래 표의 이름과 순서 그대로** `##` 제목으로 보인다 |
| Transcript | `## Transcript` 아래에 `### 00:00:03` 형태의 타임스탬프 제목들과 그 아래 문장 |
| 목록 | 목록 항목이 Notion에서 **목록으로** 보인다 (문단 하나로 뭉쳐 있지 않다) |
| 빈 껍데기 | **내용이 없는 섹션은 제목도 없어야 한다** — 빈 제목이 보이면 그것이 실패다 [E1 · INV-8] |

```text
Meeting   Overview · Key Discussions · Decisions · Action Items · Open Questions
Study     Overview · Key Concepts · Important Details · Questions · Things to Study · References Mentioned
Summary   Short Summary · Key Points
```

```text
PASS-2 = 위 표가 전부 만족된다. "읽을 만한 문서"로 보이며, 마크다운 기호(##, -, ###)가
         그대로 글자로 남아 있지 않다.
```

⚠️ **마크다운 기호가 글자로 보이면 그것은 실패다** — Notion이 markdown을 블록으로 해석하지
못했다는 뜻이고, 이 Phase 전체가 그 동작 위에 서 있다 (ADR-0009 §5.4).

### 5.4 AI Note가 없는 녹음도 보낸다 (INV-8)

R-noai로 §5.2를 한 번 더 한다.

```text
기대: 제목 · Date/Duration · Transcript 만으로 유효한 페이지가 만들어진다.
      AI 섹션의 빈 껍데기가 없다. "AI가 실패한 것처럼" 보이지 않는다.
```

**이것은 선택적 편의가 아니라 core 성공 기준이다** (PRODUCT-SPEC §17.1 · `phase-prompt/05` A-3).

---

## 6. 1시간 분량 transcript — **HR-2** (PASS-3). 이 문서에서 가장 중요한 항목

### 6.1 왜 이 항목이 따로 있는가

문서는 **60,000 바이트 · 300 block unit** 예산으로 나뉘어 **여러 요청으로 순차 전송된다**
(ADR-0009 §6). 그리고 **나눠 보낸 결과가 한 번에 보낸 것과 같은 구조가 되는지는 확인되지
않았다** — ADR-0009 §6.3과 §15.5가 그것을 [E4] UNVERIFIED로 남겼고, **이 항목만이 그것을
판정할 수 있다.**

> **자동 테스트가 판정하는 것은 `join(chunks) == 원본`까지다.** 그것은 "앱이 문자열을 자르지
> 않았다"이지 **"Notion 페이지가 온전하다"가 아니다.**

### 6.2 절차 — 로컬 Markdown을 **정답지**로 쓴다

두 산출물이 **같은 문자열**에서 나온다는 것이 이 Phase의 설계다 (ADR-0009 §14). 그래서
로컬 파일이 비교 기준이 된다.

1. R-long의 Recording Detail에서 **먼저 `Export Markdown`** 을 누른다.
   화면이 만들어진 파일의 **전체 경로**를 보여 준다 [E1].

   ```bash
   FILE="<화면이 보여 준 경로>"
   wc -c "$FILE"                 # 전체 바이트
   grep -c '^### ' "$FILE"       # transcript 타임스탬프 제목 개수
   tail -n 20 "$FILE"            # 문서의 마지막 타임스탬프와 마지막 문장
   ```

2. **`Send to Notion`** 을 누른다. 시간이 걸린다 — 요청 사이에 최소 350ms 간격이 있고
   (ADR-0009 §9.2-6) 조각 수만큼 순차로 나간다. **느린 것은 실패가 아니다.**

3. 끝나면 화면의 진행도 문장을 읽는다.

   ```text
   All N parts of this document are on that page.       ← N > 1 이어야 한다
   ```

   **N이 1이면 문서가 나뉘지 않았다는 뜻이고, 그때 이 항목은 판정되지 않았다** — 더 긴
   transcript가 필요하다. (60,000 바이트가 한 조각의 상한이다.)

4. **Notion 페이지와 로컬 파일을 대조한다.** 아래 네 가지가 판정이다.

| # | 확인 | 어떻게 | 실패의 모습 |
| --- | --- | --- | --- |
| **6a** | **끝이 있는가** | 로컬 파일의 **마지막 타임스탬프**(`tail`로 본 값)를 Notion 페이지 맨 아래에서 찾는다 | 페이지가 중간에서 끝난다 = **조용한 잘림** |
| **6b** | **개수가 같은가** | `grep -c '^### '` 의 값과 Notion 페이지의 타임스탬프 제목 수를 비교한다 (§6.3) | 개수가 적다 = 유실 · 개수가 많다 = 중복 |
| **6c** | **이음매가 멀쩡한가** | 조각 경계 근처를 본다 (§6.4) — 문단이 갈리거나 두 문단이 하나로 붙지 않았는지 | 경계에서 문장이 끊긴다 |
| **6d** | **구조가 유지되는가** | 페이지 전체를 훑어 `## Transcript` · `###` 제목 · 목록이 **끝까지** 블록으로 살아 있는지 본다 | 뒤쪽부터 글자로 보인다 = 이어붙이기가 markdown으로 해석되지 않았다 |

```text
PASS-3 = 6a · 6b · 6c · 6d 가 전부 만족된다.
```

### 6.3 개수를 세는 실용적인 방법

Notion 페이지에서 제목을 손으로 세는 것은 1시간 분량에서는 현실적이지 않다.
**Notion의 페이지 export(Markdown)를 써서 파일로 받은 뒤 같은 `grep`으로 세는 것이 가장
신뢰할 만하다** — 다만 **Notion의 export 기능과 그 결과 형식은 이 Run이 확인하지 않았다
[E4].**

```bash
grep -c '^### ' "$FILE"                       # 우리가 보낸 것
grep -c '^### ' "<Notion에서 받은 .md>"        # Notion에 있는 것
```

받은 파일의 형식이 달라 `###`로 세어지지 않으면 **숫자를 지어내지 말고**, 대신 6a와 6c로
판정하고 §11에 "개수 비교는 하지 못했다"고 그대로 적는다.

### 6.4 이음매를 찾는 방법

조각은 **빈 줄(문단 경계)** 에서 나뉘고, 한 조각은 최대 60,000 바이트다 (ADR-0009 §6.3).
그러므로 이음매는 대략 그 배수 근처에 있다.

```bash
# 60,000 바이트 지점 근처의 문맥을 뽑아 본다 (경계는 이 앞의 빈 줄에 있다)
head -c 61000 "$FILE" | tail -c 2000
```

거기서 눈에 띄는 문장 한 조각을 Notion 페이지에서 **검색**해서, 그 앞뒤가 로컬 파일과 같은
순서·같은 문단 구성인지 본다.

### 6.5 같은 문단이 두 번 보이면

**그것은 알려진 감수 사항이며 자동 실패가 아니다.** 실패한 요청은 보낸 것으로 세지 않으므로,
재시도가 있었다면 같은 조각이 두 번 반영될 수 있다 — 유실보다 중복이 낫다는 것이 명시된
결정이다 (ADR-0009 §8.4-4).

**다만 §11에 그대로 적는다**: 어디가 반복됐는지, 그때 화면에 재시도/대기가 보였는지.

---

## 7. export된 Markdown을 Obsidian / NotebookLM에서 연다 — **HR-3** (PASS-4)

파일은 `<APP_DATA>/exports/` 아래에 있고, 이름은 `2026-09-01-3dgs-study-04.md` 형태다
[E1 · ADR-0009 §4.2]. **같은 이름이 있으면 덮어쓰지 않고 `-2` · `-3`이 붙는다** [E1].

> ⚠️ **NotebookLM에 올리는 것은 외부 서비스로 데이터를 보내는 일이다.** 이 테스트는
> **테스트용 녹음**으로 한다. 앱은 NotebookLM과 연동하지 않는다 — 사람이 파일을 옮기는
> 것뿐이다 (PRODUCT-SPEC §15).

### 7.1 Obsidian [E4 — Obsidian의 동작은 이 Run이 확인하지 않았다]

`exports/` 안의 `.md` 파일을 vault로 복사(또는 그 폴더를 vault로 열기)한 뒤 확인한다.

| 확인 | 기대 |
| --- | --- |
| 제목 | 첫 줄이 h1 제목으로 보인다 |
| 아웃라인 | `## Overview` … `## Transcript` · `### 00:00:03`이 **문서 목차에 계층으로 선다** |
| 목록 | `- ` 항목이 목록으로 보인다 |
| 파일명 | **한글 제목의 파일이 깨지지 않는다** (슬러그는 한글을 남긴다 — ADR-0009 §4.2) |
| 읽기 | 앞에서 뒤까지 그냥 읽힌다. 편집기에서 손볼 필요가 없다 |

### 7.2 NotebookLM [E4 — 받는 형식과 화면 절차는 이 Run이 확인하지 않았다]

같은 파일을 소스로 추가한다.

| 확인 | 기대 |
| --- | --- |
| 업로드 | `.md`를 그대로 받는다. **받지 않으면 그 사실을 §11에 적는다** — 앱을 고칠 일인지 판단하는 근거가 된다 |
| 내용 | 전체가 들어간다 (특히 긴 transcript의 끝까지) |
| 쓸모 | 그 소스에 질문했을 때 transcript 내용이 실제로 답에 쓰인다 |

```text
PASS-4 = 두 도구 중 최소한 Obsidian에서 §7.1의 다섯 가지가 만족되고,
         NotebookLM 쪽 결과(받았는가 / 못 받았는가)가 기록됐다.
```

**이것은 "쓸 만한가"의 판정이며 자동 테스트가 답할 수 없다.** 어색한 점이 있으면 PASS/FAIL과
별개로 문장으로 적는다 — §11이 그 자리를 갖고 있다.

---

## 8. 실패해도 local data가 온전한가 — PASS-5 (INV-3)

**일부러 실패시킨다. 파괴적이지 않은 방법만 쓴다.**

### 8.1 세 가지 실패를 만든다

| | 만드는 방법 | 기대하는 실패 |
| --- | --- | --- |
| **F-1** | Settings에서 `Remove the saved token` 을 누른 뒤 R-short를 보낸다 | 아직 token이 없다 / 인증 실패 계열 |
| **F-2** | 부모 페이지를 integration에서 **공유 해제**하거나, destination에 없는 식별자를 넣고 Save 한 뒤 보낸다 | 보낼 위치를 찾지 못했다 (`403`/`404` 계열) |
| **F-3** | 네트워크(Wi-Fi)를 끄고 보낸다 | 네트워크로 닿지 못했다 |

각각에서 **화면에 실패가 보이고, 다시 시도할 수단이 있는지** 확인한다 [E1 —
*"This recording could not be sent to Notion."* + 실패 안내 + 재시도 버튼].

### 8.2 그 뒤에 확인하는 것 — 전부 그대로여야 한다

| 확인 | 기대 |
| --- | --- |
| Recording | 목록과 Detail에 그대로 있다 |
| Transcript | 그대로 읽힌다 |
| AI Note | 그대로 있다 |
| 오디오 | 재생된다 (**전송은 오디오를 건드리지 않는다** — 애초에 보내지도 않는다 · INV-6) |
| **Markdown export** | **여전히 동작한다** — Notion과 무관하다 (INV-8) |
| 앱 재시작 후 | 위 전부가 그대로다 |

```text
PASS-5 = F-1 · F-2 · F-3 각각에서 실패가 보이고, 위 여섯 가지가 전부 그대로다.
```

**그 뒤 F-1·F-2를 되돌린다** (token 다시 저장 · 부모 페이지 다시 공유 / 식별자 복구).

### 8.3 ★ 부분 전송 뒤의 실패 — 여기가 중복 정책의 실물 판정이다

만들 수 있으면 만든다 (없으면 §11에 "만들지 못했다"고 적는다).

```text
R-long을 보내는 도중에 네트워크를 끊는다 (여러 조각으로 나뉘므로 중간에 끊을 시간이 있다)
```

그 뒤 확인:

1. 화면이 **부분 전송을 드러내는가** — *"N of M parts of this document are already on that
   page."* [E1]
2. 다시 보내기 버튼이 **`Continue sending to the same page`** 인가 [E1] —
   *새 페이지를 만드는 버튼이 아니어야 한다.*
3. 네트워크를 복구하고 그 버튼을 누른다 → **새 페이지가 생기지 않고 같은 페이지가 이어진다.**
4. Notion에서 확인: **부모 페이지 아래 R-long의 페이지는 여전히 하나다.**

이것이 ADR-0009 §15.4 표의 "`failed` + `page_id` 있음 + 지문 일치 → 이어 보낸다" 줄의
실제 확인이다.

---

## 9. 중복 sync 정책의 실물 확인 — PASS-6 (PRODUCT-SPEC §10 · ADR-0009 §15.4)

**이미 보낸 것을 다시 보내면 무슨 일이 일어나는가.** 문서가 정한 답은 이것이다:
*"확인을 받은 뒤 새 페이지를 만들고, 기존 페이지는 건드리지 않는다."*

### 9.1 절차

1. §5에서 만든 R-short의 Notion 페이지를 열어 **아무 줄이나 하나 직접 편집한다**
   (예: 맨 위에 `-- 사람이 손댄 줄 --`을 적는다). 이것이 "앱이 사용자 문서를 건드리지
   않는다"의 판정 재료다.
2. 앱에서 R-short의 Recording Detail을 연다. 이미 보낸 녹음이므로 화면은 **`Send to Notion`이
   아니라** 다른 것을 보인다 [E1]:

   ```text
   This recording is in Notion.
   [ Create a new Notion page ]
   This recording already has a Notion page. Creating a new page leaves that page exactly
   as it is — nothing there is changed or deleted.
   ```

   **무슨 일이 일어나는지가 누르기 전에 적혀 있는지** 확인한다. 그것이 요구사항이다
   (예측 가능성 · `phase-prompt/05` 요구 8).
3. **누르지 않고 멈춘다** → 이 상태에서 Notion에 **아무 변화도 없어야 한다.**
4. 이제 **`Create a new Notion page`** 를 누른다 [E1 — 새 페이지를 만드는 갈래는 이 버튼
   하나뿐이며, 이것만이 확인을 실어 보낸다].

### 9.2 확인

| 확인 | 기대 |
| --- | --- |
| 3단계에서 | Notion에 아무 일도 일어나지 않았다 (**확인 없이는 중복이 없다**) |
| 4단계 뒤 | 부모 페이지 아래에 **두 번째 페이지**가 생겼다 |
| **1단계의 손댄 줄** | **첫 페이지에 그대로 있다.** 지워지지도 덮어써지지도 않았다 |
| 첫 페이지 | 삭제되지도 보관 처리되지도 않았다 |

```text
PASS-6 = 위 네 가지가 전부 만족된다.
```

**이 항목이 실패하면(확인 없이 페이지가 생기거나, 기존 페이지가 바뀌면) 그것은
Phase Goal이 금지한 "조용한 중복" 또는 되돌릴 수 없는 파괴다** — 반드시 §11에 적는다.

---

## 10. 실패했을 때 무엇을 기록하는가

### 10.1 기록 양식

```text
항목:          PASS-0 ~ PASS-6 중 무엇
무엇을 했나:   눌렀던 버튼 · 그 직전 상태
화면이 말한 것: 화면 문장 그대로 (token은 어디에도 없다 — 그래도 붙여넣기 전에 읽는다)
Failure kind:  화면/상태가 말한 실패 종류
진행도:        sentChunks / totalChunks (있으면)
Notion 쪽:     페이지가 생겼는가 · 몇 개인가 · 어디까지 들어갔는가
local data:    §8.2의 여섯 가지가 그대로인가
재현되는가:    같은 조작을 한 번 더 했을 때
```

⚠️ **§1을 다시 읽는다.** 이 기록은 저장소에 들어간다. **token · 그 일부 · 그 앞뒤 몇 글자를
적지 않는다.** 부모 페이지 식별자와 워크스페이스 이름은 secret이 아니지만 필요 없으면 가린다.

### 10.2 어디에 적는가

- **이 문서의 §11** — smoke test 실행 기록.
- 실패가 **제품 결함**이면 그 내용을 그대로 남기고, 고치는 것은 Runtime의 Task가 한다.
- 실패가 **Runtime / Planner의 문제**로 보이면 `docs/LOOP-RUNTIME-FIELD-NOTES.md`다.
  **제품 버그를 Runtime 문제로 적지 않는다** (`CLAUDE.local.md` — Field Note Quality).

---

## 10.1 ⚠️ ASSUMPTION A-NOTION-001 (운영자 기록 · 2026-09-04)

```text
ASSUMPTION A-NOTION-001

Phase 5의 Markdown export와 Notion sync 아키텍처는 구현됐고, 계약 검증 · 가짜 네트워크
경계 · 자동 테스트를 전부 통과했다.
그러나 실제 Notion 워크스페이스로 요청이 나간 적은 한 번도 없다.

따라서 다음은 UNVERIFIED다 — 엔드포인트·요청 형태·응답 해석이 오늘의 Notion과 실제로
맞는가, 만들어진 페이지가 읽을 만한 구조인가, 1시간 transcript가 실물에서 온전한가,
integration token이 실제 OS 자격증명 저장소에 제대로 담기는가.

`A-REC-001`(실제 마이크) · `A-TRANS-001`(실제 Whisper 추론) · `A-AI-001`(실제 Ollama 생성)과
같은 성격이며, 같은 이유로 Final Integration의 hard human gate로 연기됐다.

**자동 테스트 PASS를 "실제 Notion sync 검증 완료"로 읽지 않는다.**
stub transport를 지난 것은 계약이지 실물이 아니다.

해소 절차: 이 문서 §1 ~ §10.   해소 기록: 이 문서 §11의 기록표.
```

---

## 11. smoke test 실행 기록 — 운영자가 채운다

**아래 표가 비어 있는 동안 Phase 5는 "실제 Notion 전송이 검증됐다"가 아니다.**

```text
실행 날짜:            (비어 있음)
실행자:               (비어 있음)
앱 버전 / 커밋:       (비어 있음)
Notion 워크스페이스:  테스트 전용인가? (예 / 아니오)
```

| 항목 | 무엇을 판정하는가 | 결과 | 메모 |
| --- | --- | --- | --- |
| **PASS-0** | connection test 성공 (+ TLS · Keychain이 처음 증명된다) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **PASS-0b** | 앱 재시작 뒤에도 저장된 token으로 연결된다 (§4.3) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **PASS-1** | 실제 Notion 페이지가 만들어진다 (**HR-1**) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **PASS-2** | 그 페이지가 §10의 섹션 구성을 갖는다 (**HR-1**) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **PASS-2b** | AI Note 없는 녹음도 전송된다 (INV-8) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **PASS-3** | **1시간 transcript가 Notion에서 온전하다** (**HR-2**) | ☐ PASS ☐ FAIL ☐ 미실행 | 조각 수 N = |
| **PASS-4** | export된 Markdown이 Obsidian / NotebookLM에서 쓸 만하다 (**HR-3**) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **PASS-5** | 실패해도 local data가 온전하고 재시도할 수 있다 (INV-3) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **PASS-5b** | 부분 전송 뒤 재시도가 같은 페이지를 이어간다 (§8.3) | ☐ PASS ☐ FAIL ☐ 미실행 | |
| **PASS-6** | 중복 sync 정책이 문서와 같이 동작한다 | ☐ PASS ☐ FAIL ☐ 미실행 | |

**자유 기술** (어색했던 점 · 느렸던 점 · 문구가 헷갈렸던 점):

```text
(비어 있음)
```

---

## 12. 이 smoke test가 통과해도 확인되지 않는 것

| 사실 | 왜 여전히 확인되지 않는가 |
| --- | --- |
| **markdown 엔드포인트 전용 본문 크기 상한** | 이 절차는 60,000 바이트 예산으로만 보낸다. 그 상한이 존재하는지, 얼마인지는 여기서 드러나지 않는다 (ADR-0009 §6.1 · §15.5). **성공했다는 사실이 "우리 예산이 옳다"를 증명하지 않는다** |
| `ureq` · `keyring`의 feature 전체 목록 | 이 절차는 "지금 켠 구성이 동작한다"만 보인다 (ADR-0009 §15.5) |
| Windows 자격증명 저장소 · Windows 전반 | Phase 6 |
| 사내 프록시 등 **기기에 직접 넣은 루트 인증서** 환경 | 번들된 루트(`webpki-roots`)를 쓰므로 그 환경은 실패할 수 있다 (ADR-0009 §15.2.3). 그런 환경에서 실행하지 않는 한 드러나지 않는다 |
| 장시간 · 반복 사용의 안정성 | 이 절차는 각 항목을 한두 번 수행한다 |
| Notion 요금제별 workspace rate limit | 한도는 요금제에 따르며 모든 연결이 공유한다 [E2 · PRODUCT-SPEC §14.9.1]. 이 절차가 그것을 재지 않는다 |

**통과한 항목만 통과했다고 적는다.** 미실행을 PASS로 옮겨 적지 않는다.
