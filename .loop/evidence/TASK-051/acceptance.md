# TASK-051 · Acceptance Criteria가 무엇으로 판정되는가

Gate 세 개(P9-AC1~AC3)는 `self-check.md`에 있다. 이 문서는 verifier가 보는 넷(P9-AC4~AC7)이
**어느 파일의 어느 검사로** 판정되는지 적는다. 전부 순수 view 모듈 수준이며 실제 Notion ·
실제 파일시스템 · DOM 없이 돈다 (§18).

---

## P9-AC4 — P-3의 세 조건이 테스트로 남아 있다

`phase-prompt/05` P-3이 "약화하면 안 된다"고 못박은 셋이다. 셋 다
`src/screens/exportView.test.ts`의 **이름이 그 조건 그대로인 `describe` 블록** 안에 있다.

### (1) AI provider 없이 Markdown export가 된다 (INV-8)

`describe('P-3 (1) AI provider 없이 Markdown export가 된다 (INV-8)')`

| 검사 | 무엇을 판정하는가 |
| --- | --- |
| `이 자리의 입력에 AI provider를 담을 자리가 없다` | `ExportPanelInput`의 필드를 통째로 고정한다 (`['attempt','notes','recording']`). provider를 담을 자리가 생기면 여기서 먼저 드러난다 |
| `AI 상태가 무엇이든 내보내기 동작이 글자 하나 달라지지 않는다` | 저장된 AI 상태 다섯(`none`·`pending`·`running`·`done`·`failed`) 전부에서 view 값이 baseline과 **깊은 비교로 동일**하고 `ready`다 |
| `tests/screen-boundary.test.ts` → `export 자리의 순수 모듈에 provider도 AI 상태도 등장하지 않는다` | 값의 모양만 고정하면 "모듈 안에서 몰래 본다"는 경로가 남는다. `src/screens/exportView.ts` **원문**에 `AiProviderStatus`·`providerName`·`locality`·`aiStatus`가 없다는 것을 읽어서 확인한다 |

이 검사가 `tests/` 쪽에 있는 이유: `src/` 아래 테스트는 브라우저 타입으로 검사되어
`node:fs`를 import할 수 없다(그렇게 했더니 build Gate가 `TS2307`로 실패했다 — `self-check.md`).
`tests/screen-boundary.test.ts`는 이미 원문을 읽는 검사들이 모여 있는 자리다.

### (2) AI Note 없이 Markdown export가 된다 (INV-8 · §17.1)

`describe('P-3 (2) AI Note 없이 Markdown export가 된다 (INV-8 · §17.1)')`

| 검사 | 무엇을 판정하는가 |
| --- | --- |
| `노트가 하나도 없어도 내보낼 수 있다` | `notes: []`에서 body가 `ready`이고 동작이 이 녹음을 가리킨다 |
| `노트가 있든 없든 내보내기 동작이 같다` | `notes: [note]` · `notes: []` · `notes: null`의 body가 **깊은 비교로 동일**하다 — 노트는 문서의 내용일 뿐 내보내기의 조건이 아니다 |
| `노트가 없는 것이 결함처럼 적히지 않는다` | 안내 문구가 "그것도 완결된 문서"라고 말하고 `fail|error|cannot`을 담지 않는다 |
| `노트도 전사도 없는 녹음은 실패가 아니라 "아직 없다"로 남는다` | 전사가 없는 갈래가 실패가 아니라 `nothingToExport` 상태다 (§7.2) |

Rust 쪽의 같은 사실(노트 없이 **실제로** 유효한 파일이 만들어진다)은 앞선 Task가 이미
테스트로 남겼다 — `src-tauri/tests/markdown_export.rs`의
`a_recording_with_no_ai_note_at_all_is_exported_as_a_valid_document`. 이 Task는 그 위에서
**화면에서 그 경로가 막히지 않는다**를 본다. 두 층이 같은 조건을 각자의 자리에서 판정한다.

### (3) export 실패가 원본 데이터를 훼손하지 않는다 (INV-3)

`describe('P-3 (3) export 실패가 원본 데이터를 훼손하지 않는다 (INV-3 · §13)')`

| 검사 | 무엇을 판정하는가 |
| --- | --- |
| `세 실패 각각이 §13의 세 질문에 답하고 재시도 수단을 남긴다` | `invalidInput`·`storage`·`unexpected` 각각에서 (1) 무엇이 실패했는가(`headline`·`failure.kind`) (2) 원본은 안전한가(`preservedNotice` — `untouched` · `Nothing was deleted or changed`, `sourceDataSafe`) (3) 다시 시도할 수 있는가(`retry`) |
| `다시 시도해도 같은 실패에서도 재시도 수단은 남는다` | `retryable: false`에서도 재시도 수단과 `resolution`이 있다 |
| `실패한 뒤에도 녹음 · 전사 · 열람의 화면 값이 달라지지 않는다` | 같은 레코드로 만든 `loadedRecordingDetail` · `loadedRecordings` · `transcriptTab` 값이 실패 전후로 **깊은 비교로 동일**하다 |
| `실패가 오디오 파일을 건드렸다고 말하지 않는다` | 문구가 오디오 파일이 그대로임을 말하고 `recovered|cleaned` 같은 말을 하지 않는다 |

저장소가 **실제로** 그대로인지는 Rust 쪽 테스트가 판정한다 —
`src-tauri/tests/markdown_export.rs`의
`a_write_that_fails_is_a_domain_failure_that_leaves_the_stored_data_untouched` ·
`a_directory_that_cannot_be_created_fails_visibly_and_changes_nothing` ·
`exporting_reads_no_audio_and_copies_none`. 이 Task가 판정하는 것은 **화면이 그 사실을 그대로
말하고, 실패가 다른 화면 값을 흔들지 않는다**는 것이다.

---

## P9-AC5 — 상태가 목록과 Detail 양쪽에 보이고, 실패가 §13에 답하며 재시도 수단이 있다

### 목록 (이미 있던 것)

`src/screens/recordingsView.ts`가 항목마다 `Transcript · AI Note · Notion` 세 상태를 만든다.
검사는 `src/screens/recordingsView.test.ts`의
`항목마다 transcription · AI · Notion 세 상태를 보여준다`이며, 이 Task가 그것을 고치지 않았다.

### Detail (이 Task가 더한 것)

`src/screens/notionSyncView.test.ts` →
`describe('이 녹음의 Notion 상태가 Detail에 보인다 (§7 · 요구 9)')`

| 검사 | 무엇을 판정하는가 |
| --- | --- |
| `저장된 다섯 상태가 목록과 같은 문자열로 보인다` | 다섯 상태 각각에서 Detail의 상태 표시가 **목록이 만든 값과 `toEqual`** 이다. 두 화면이 같은 함수(`statusBadge`)를 쓴다 |
| `아직 보낸 적이 없다는 것이 오류처럼 적히지 않는다` | `none`이 `Not started`이고 `fail|error`가 아니다 (INV-8) |
| `레코드를 아직 읽지 못했으면 상태를 지어내지 않는다` | `loading`에서 상태 표시가 `null`이다 |
| `방금 시작한 전송은 저장된 값보다 새롭다` | 예외는 하나뿐이며 그 이유가 검사에 적혀 있다 |
| `다른 녹음의 전송은 이 자리에 보이지 않는다` · `다른 녹음의 전송 기록도 …` | 다른 녹음의 진행 상황·페이지 식별자가 이 자리에 새지 않는다 |

### 실패와 재시도

`describe('전송 실패가 화면에 남는 방식 (§13 · INV-3)')`

| 검사 | 무엇을 판정하는가 |
| --- | --- |
| `다섯 실패 각각이 §13의 세 질문에 답하고 재시도 수단을 남긴다` | 인증 실패 · 권한 없는 destination · 속도 제한 · 네트워크 없음 · 결과를 모름 각각에서 세 질문의 답과 재시도 동작이 값으로 있다 |
| `실패한 갈래마다 안내가 서로 다르다` | 다섯 갈래의 `resolution`이 전부 다른 문장이다 — Rust가 나눈 구분을 뭉개지 않는다 |
| `부분 전송 뒤의 실패는 어디까지 갔는지를 드러낸다` | `sentChunks=3 / totalChunks=7`에서 `3 of 7 parts … already on that page.`가 값으로 나온다 (요구 12) |
| `기록되기 전에 실패한 것은 숫자를 지어내지 않는다` | 진행도를 모르면 `null`이다 |
| `앱을 다시 켠 뒤 이유를 모르는 실패는 이유를 지어내지 않는다` | `failure: null` · `cause: 'unknown'`이며 재시도 수단은 남는다 |
| `거절된 요청은 전송 상태를 덮지 않고 그 옆에 남는다` | 접수되지 않은 요청이 조용히 사라지지 않는다 |

화면 쪽 대응: `RecordingDetailScreen.tsx`의 `NotionPanel`이 실패를 `FailureNotice`(§13의 세
질문을 그리는 기존 컴포넌트) + `preservedNotice` + `resolution` + `SendButton`으로 그린다.

---

## P9-AC6 — 전송되는 데이터가 UI에 드러나고 오디오가 나가지 않는다 (INV-5 · INV-6)

`src/screens/notionSyncView.test.ts` → `describe('무엇이 전송되는지 화면에 드러난다')`

| 검사 | 무엇을 판정하는가 |
| --- | --- |
| `나가는 것이 텍스트라는 것과, 오디오는 나가지 않는다는 것이 함께 있다` | `contents.items`가 제목·날짜·길이 · 전사 텍스트 · (있으면) AI 노트를 말하고, `audioNotice`가 `The audio file is never sent`를 말한다 |
| `오디오 파일 경로가 이 자리의 어떤 값에도 실리지 않는다` | 다섯 상태 × 부분 전송 × 실패를 만든 view 값 전체를 직렬화해 `audioPath`도 `.wav`도 들어 있지 않다는 것을 확인한다 |
| `본문이 어느 상태든 그 사실은 남는다` | 준비·전송 중·완료·실패 어디서도 이 문구가 사라지지 않는다 |

Markdown 쪽도 같은 사실을 말한다 — `src/screens/exportView.test.ts`의
`언제나 담기는 것과, 오디오는 복사되지 않는다는 사실이 함께 있다`
(`The audio file is not copied, and nothing is sent anywhere`).

화면 쪽 대응: 두 panel 모두 본문 아래에 `share__contents` 블록을 **언제나** 그린다.

---

## P9-AC7 — 두 번 보냈을 때의 결과가 ADR-0009와 일치하게, 누르기 전에 예측 가능하다

`src/screens/notionSyncView.test.ts` →
`describe('같은 Recording을 두 번 보내면 무슨 일이 일어나는가 (ADR-0009 §8.3 · §8.5)')`

ADR-0009 §8.5의 표와 화면 동작의 대조:

| 저장된 상태 (§8.5) | ADR의 동작 | 화면이 **누르기 전에** 말하는 것 | 검사 |
| --- | --- | --- | --- |
| 행이 없다 / `none` | 새 페이지를 만든다 | `NEW_PAGE_OUTCOME` — 부모 페이지 밑에 페이지 하나. `confirmation: 'notAsked'` | `보낸 적이 없으면 새 페이지 하나가 생긴다고 미리 말한다` |
| `running` | 아무것도 하지 않고 진행 중임을 보인다 | 보내는 중 상태에는 동작 자체가 없다 | `방금 시작한 전송은 저장된 값보다 새롭다` |
| `failed` · page 있음 | **같은 페이지에 이어 보낸다** (확인 없이) | `RESUME_OUTCOME` — `does not create a second page`, 내용이 바뀌었으면 먼저 묻는다 | `끝나지 않은 전송은 같은 페이지에 이어 보낸다 …` |
| `failed` · page 없음 (결과를 모름) | 확인을 받은 뒤 새 페이지 | `RETRY_OUTCOME` — 그대로 만들지 않고 **먼저 묻는다** | `페이지가 만들어졌는지 모르는 실패는 …` |
| fingerprint 불일치 | 이어 붙이지 않고 확인 뒤 새 페이지 | Rust가 보낸 `needsConfirmation=documentChanged`를 확인 요청으로 읽는다 | `확인을 물어 온 것은 실패가 아니라 고를 차례다 …` |
| `done` | **확인 뒤 새 페이지. 기존 페이지는 그대로 둔다** | `leaves that page exactly as it is` · `nothing there is changed or deleted`, 그리고 `overwrit|replaces`라는 말이 없다 | `이미 보낸 녹음은 **누르기 전에** …` |

정책이 새는 통로가 없다는 것도 함께 본다.

- `확인을 싣는 동작은 그 하나뿐이다 — 조용한 중복이 생길 통로가 없다`:
  새 페이지가 필요 없는 갈래의 동작은 전부 `confirmation: 'notAsked'`다. 화면에서
  `confirmation: 'newPage'`를 싣는 것은 **사용자가 안내를 읽고 누르는 버튼 하나**뿐이며
  (`newPageAction`), 그 사실이 `RecordingDetailScreen.tsx`의 `SendButton`에도 그대로 있다 —
  버튼은 `action.confirmation`을 그대로 넘길 뿐 스스로 고르지 않는다.
- `모르는 확인 사유를 확인 요청으로 읽지 않는다`: `needsConfirmation=` 뒤의 값이 Rust가 정한
  셋이 아니면 확인 요청으로 읽지 않는다. Rust 쪽 값의 출처는
  `src-tauri/src/sync/run.rs`의 `ConfirmBecause::as_str`이다.
- `전송이 끝났을 때` → `이어 보내 끝낸 것을 "새 페이지를 만들었다"고 말하지 않는다`:
  `createdPage`를 화면이 지어내지 않는다 (§8.2).

## 이 Task가 판정하지 않는 것

- 실제 Notion 페이지가 만들어지는지, 1시간 transcript가 실제로 온전한지는 **Human Review**
  항목이다 (`phase-prompt/05` · TASK-054가 절차를 만든다). 이 Task의 어떤 테스트도 그것을
  통과했다고 말하지 않는다.
- Settings의 token 입력·connection test는 TASK-052의 범위다.
