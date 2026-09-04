# TASK-040 — Acceptance Criterion마다 무엇이 판정하는가

## AC1 · AC2 · AC3 — build · lint · test Gate

`.loop/evidence/TASK-040/gates.md`. 셋 다 exit 0.

---

## AC4 — 연결 확인 수단이 있고, 실행 중 · 미실행 · 모델 없음이 서로 구분된다

**순수 모듈의 상태 값**: `src/screens/aiProviderSettings.ts`의 `AiConnection`.
일곱 갈래가 각자 다른 `kind`를 갖고, 갈래마다 필드 구성도 다르다.

```text
notChecked      아직 물어보지 않았다
checking        물어보는 중이다
notConfigured   고른 provider가 없다              resolution 있음 · failure 없음
running         응답했고 쓸 수 있는 모델이 있다    models 있음
noModels        응답했지만 모델이 하나도 없다      resolution 있음 · failure 필드 자체가 없음
notRunning      지금 응답하지 않는다              resolution 있음 · failure 있음
checkFailed     확인 요청 자체가 거절됐다          failure 있음 · resolution 없음
```

`checkedAiProvider(status)`가 backend의 네 상태(`AiProviderState`)를 1:1로 옮기며 다시
뭉치지 않는다. 확인 수단 자체는 `SettingsScreen.tsx`의 `checkProvider` — "Check the AI
provider" 버튼이 `ai_provider_status` command를 부르고, 결과가 화면에 남는다.

**뭉쳐 있지 않다는 것을 강제하는 테스트** (`src/screens/aiProviderSettings.test.ts`):

| 테스트 | 무엇을 못 하게 하는가 |
| --- | --- |
| `실행 중 · 모델 없음 · 미실행이 서로 다른 상태다` | 세 `kind`가 다르고 세 `text`가 서로 다름을 `Set(...).size === 3`으로 강제 |
| `모델이 없는 것은 실패가 아니다` | `noModels`에 `failure` 프로퍼티가 **존재하지 않음**을 단언. 문구에 error/failed/problem이 없음도 |
| `닿지 못한 것은 이유와 함께 오고, 무엇을 하면 되는지가 붙는다` | `notRunning`이 `Failure`와 안내를 함께 실어 옴 |
| `확인 요청이 거절된 것과 provider가 응답하지 않는 것이 다른 상태다` | 저장소 실패를 "서버를 켜세요"로 바꾸지 않음 |
| `아직 물어보지 않은 것과 응답이 없는 것이 다른 상태다` | 두 문구가 같은 값이 되는 것을 막음 |
| `고른 provider가 없으면 물어볼 대상이 없다고 말한다` | `notConfigured`에도 `failure`가 없음 (INV-8) |

**실제 Ollama 없이 돈다** — 확인 결과는 값(`AiProviderStatus`)으로 들어오고, 테스트는
네트워크도 DOM도 Tauri도 쓰지 않는다.

---

## AC5 — 로컬/외부가 화면에 드러나고, 오디오 미전송이 UI에 표현된다 (INV-5 · INV-6 · §12)

**값에서 문구가 나온다. 문구가 값을 정하지 않는다.**

```text
SELECTABLE_AI_PROVIDERS[].locality        ← provider마다 선언된 값
   │
   ├─ aiProviderChoices()  → choice.label = `${name} — ${LOCALITY_CHOICE_LABEL[locality]}`
   │                         choice.locality (값 그대로)
   │
   └─ aiProviderLocality(form.aiProvider)
          │
          └─ aiTransferNotice(locality) → { headline, transcriptText, audioText }
                 headline       = LOCALITY_HEADLINE[locality]        ← Record로 갈린다
                 transcriptText = LOCALITY_TRANSCRIPT_TEXT[locality] ← Record로 갈린다
                 audioText      = AUDIO_IS_NEVER_SENT                ← 양쪽 동일 (INV-6)
```

`SettingsScreen.tsx`의 AI Provider 그룹이 `transfer.headline` · `transfer.transcriptText` ·
`transfer.audioText` 셋을 그대로 그린다. 컴포넌트에는 locality를 보고 분기하는 문자열이
하나도 없다.

`audioText`가 locality에 따라 달라지지 않는 것은 의도다 — 오디오는 어느 쪽에서도 나가지
않으며, 그것이 계약으로 보장돼 있다 (`NoteRequest`에 오디오를 가리킬 필드가 없다 ·
ADR-0008 §4.2 · Rust 테스트 `the_generation_request_has_nowhere_to_put_audio`).
locality에 따라 갈리는 것은 headline과 transcriptText 둘이다.

**backend가 말한 locality도 화면에 도달한다** — `checkedAiProvider`가 `status.locality`를
`running` · `noModels` · `notRunning` 세 갈래에 그대로 싣는다. 이 모듈이 채워 넣지 않는다
(INV-9).

테스트: `전송 경계 (§12 · INV-5 · INV-6)` describe 블록 3건 +
`선택지 이름의 로컬/외부 표시가 provider의 locality에서 나온다` +
`provider가 스스로 말한 이름과 locality가 그대로 실려 온다`.

`aiTransferNotice(null)`이 `null`인 것도 테스트한다 — **모르는 provider에 대해 "나가지
않는다"고 말하지 않는다.**

---

## AC6 — AI 설정 실패나 provider 부재가 다른 설정 저장과 화면 사용을 막지 않는다 (INV-8)

**구조로 막았다.** `aiProviderSettings.ts`가 만드는 값 중 어떤 것도 `SettingsView`가 아니고,
저장 경로(`toSettings` → `updateSettings` → `savedSettings`)의 입력에 `AiConnection`이 없다.
`SettingsScreen.tsx`에서도 `connection`은 `view`와 **별도의 useState**다 — 장치 목록 실패를
설정 읽기 실패와 섞지 않는 기존 규칙과 같다.

**테스트** (`describe('AI가 안 되는 것이 나머지를 막지 않는다 (INV-8)')`):

| 테스트 | 무엇을 확인하는가 |
| --- | --- |
| `provider 확인이 실패한 상태에서도 다른 설정의 저장 경로가 그대로 동작한다` | `failedAiCheck(...)`가 `checkFailed`인 상태에서 편집 → `savingSettings` → `toSettings` → `savedSettings`가 전부 정상으로 끝나고 `saved === true` · `failure === null` |
| `provider를 고르지 않은 상태에서도 다른 설정이 그대로 저장된다` | `notConfigured`에서 `automaticProcessing` 저장이 나가고 `aiProvider`는 `null` 그대로 |
| `응답하지 않는 provider를 골라 둔 채로도 저장은 나간다` | `notRunning`에서 고른 provider · 주소 · 없는 모델이 **지워지지도 바뀌지도 않고** 그대로 나감 |
| `그 사실이 화면에 적을 수 있는 문장으로 있다` | `AI_SETTINGS_UNAFFECTED_NOTICE` — 화면에도 그 사실이 적힌다 |

화면 사용도 막지 않는다 — `connection`은 AI Provider 그룹 안에서만 읽히고, Save 버튼의
`disabled`는 `saving` 하나만 본다.

---

## AC7 — 테스트 전용 fake provider가 UI의 선택지로 노출되지 않는다

**선택지 목록을 만드는 코드**: `aiProviderSettings.ts`의 `SELECTABLE_AI_PROVIDERS`.

```ts
const SELECTABLE_AI_PROVIDERS: readonly SelectableAiProvider[] = [
  { id: 'ollama', name: 'Ollama', locality: 'local' },
];
```

리터럴 하나뿐이며 어디서도 모아 오지 않는다 — 모아 오는 경로가 있으면 테스트용 구현이
섞여 들어올 자리가 생긴다. `aiProviderChoices()`가 여기에 "고르지 않음" 하나만 앞에 붙인다.

frontend에는 `FakeNoteAiProvider`를 가리키는 값도 이름도 없다. Rust 쪽 double은
`crate::ai::testing`에만 있고 `ai::`에서 재수출되지 않으며, 그것을 강제하는 테스트가 이미
있다 (`ai::testing::tests::product_code_never_constructs_the_fake_provider` — 이번 실행에서도
통과).

**저장된 값으로 알 수 없는 식별자가 들어오면** 항목이 하나 더 붙지만 `usable: false`이고
`locality: null`이다 — 저장된 선택을 말없이 잃지 않으면서도 "고를 수 있는 것"이 되지는
않는다. `aiProviderLocality('fake') === null`.

테스트: `테스트 전용 fake provider가 선택지에 없다` ·
`지금 고를 수 있는 provider는 로컬 Ollama 하나뿐이다` ·
`이 앱이 세울 수 없는 저장된 값도 선택지에 남는다`.
