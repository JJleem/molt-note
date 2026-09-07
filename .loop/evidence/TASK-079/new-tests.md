# TASK-079 — 새로 고정한 것 (`src/screens/transcriptView.test.ts`)

`describe('실패')` 안에 세 케이스가 늘었다. 전부 whisper도 모델도 오디오도 DOM도 없이 돈다
(§18).

## 1. `붕괴한 전사가 일반 실패로 뭉개지지 않고 할 일이 문장으로 보인다`

`transcriptionOutputUnusable` 실패 하나를 넣고 다음을 고정한다.

```text
view.cause                 === 'outputUnusable'   (그리고 !== 'other')
view.resolution            === TRANSCRIPTION_COLLAPSED_NOTICE
view.resolution            contains 'collapsed' · 'input level' · 'record again'
                                    · 'different model' · 'start the transcription again'
view.failure.message       contains '103개'       (수치는 Rust 문장에 남는다)
view.resolution            not contains '103'     (화면이 다시 세지 않는다)
view.retry                 === { kind: 'retry', label: 'Try transcription again', … }
view.preservedNotice       === TRANSCRIPTION_PRESERVED_NOTICE  (contains 'Nothing was deleted')
```

→ AC-4가 요구한 세 가지(할 일이 문장으로 보인다 · 재시도 수단 · 아무것도 지워지지 않았다)가
   같은 케이스 하나 안에서 함께 검사된다.

## 2. `붕괴 실패에서도 이미 있던 Transcript가 그대로 남는다`

붕괴 실패 + 이미 있는 Transcript. `view.kept`의 문장 둘이 그대로 나온다 —
붕괴 판정은 저장을 막는 자리이지 저장된 것을 지우는 자리가 아니다 (ADR-0007 §18.5 · INV-2).

## 3. `네 갈래가 서로 다른 원인과 서로 다른 안내로 갈린다`

다섯 입력을 한 번에 훑어 갈래가 흐려지지 않았음을 고정한다.

```text
transcriptionModelMissing    → 'modelMissing'
transcriptionModelUnusable   → 'modelUnusable'
transcriptionOutputUnusable  → 'outputUnusable'
transcriptionEngineFailed    → 'other'      · resolution === null
failure 없음 (앱 재시작)      → 'unknown'    · resolution === UNKNOWN_FAILURE_NOTICE
```

붕괴 갈래의 문장이 모델 갈래 둘의 문장과도, `unknown`의 문장과도 다르다는 것을 함께 못박는다.

## 이미 있던 케이스는 그대로 남았다

`modelMissing` · `modelUnusable` · `other`(해결 절차를 지어내지 않는다) · `unknown` ·
`재전사가 실패해도 이미 있던 Transcript가 그대로 보인다`를 지우거나 약화하지 않았다.
