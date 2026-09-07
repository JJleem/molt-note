# TASK-079 — 무엇을 바꿨는가

붕괴한 전사(`transcriptionOutputUnusable`)가 화면에서 `other`로 뭉개지던 것을 별도 갈래로
가르고, 그 갈래의 안내 문장을 순수 모듈에 두었다.

## 바뀐 파일

```text
src/screens/transcriptView.ts        규칙 — 갈래 하나 · 문장 하나 · 매핑 한 줄
src/screens/transcriptView.test.ts   고정 — 새 갈래 · 문장 · 다른 갈래가 흐려지지 않았다는 것
src/screens/RecordingDetailScreen.tsx  주석 한 곳 (그리는 코드는 바뀌지 않았다)
```

## `transcriptView.ts`

1. `TranscriptFailureCause`에 `'outputUnusable'`을 더했다. 다섯 갈래가 됐다.
2. `TRANSCRIPTION_COLLAPSED_NOTICE`를 export 상수로 두었다:

   > The transcription collapsed, so the result could not be used as a transcript.
   > Running it again unchanged gives the same result — check the recording input level
   > and record again, or choose a different model in Settings, then start the
   > transcription again.

   **무엇이 일어났는가**(collapsed · 결과를 전사로 쓸 수 없다)와 **무엇을 하면 되는가**
   (입력 레벨 확인 후 재녹음 · 다른 모델 · 다시 시도)가 한 문장 안에 있다. 원인을 하나로
   단정하지 않는다 — Rust도 그렇게 하지 않는다 (ADR-0007 §18.6).
3. `RESOLUTION`에 `outputUnusable: TRANSCRIPTION_COLLAPSED_NOTICE`를 더했다.
   `other`와 `unknown`은 `null` 그대로다.
4. `failureCause`에 `case 'transcriptionOutputUnusable': return 'outputUnusable';`를 더했다.

바꾸지 않은 것:

- `retry`는 모든 실패 갈래에서 그대로 만들어진다 (`failedTranscript`).
- `preservedNotice`(`TRANSCRIPTION_PRESERVED_NOTICE` — "Nothing was deleted")도 그대로다.
- `kept`(이미 있던 Transcript)도 그대로 실려 나간다.
- `FailureKind` union · wire 타입 · command 표면은 한 글자도 건드리지 않았다 (INV-9).

## `RecordingDetailScreen.tsx`

JSX 주석 한 곳만 고쳤다 — 갈래가 셋이 됐다는 사실을 적고, 어느 갈래가 어떤 문장을 갖는지
정하는 자리가 `transcriptView`라는 것을 남겼다. **조건 분기는 늘지 않았다.** 그리는 코드는
전과 같이 `{tab.resolution !== null && <p className="hint">{tab.resolution}</p>}` 한 줄이며,
컴포넌트는 `cause`를 읽지 않는다 (AC-5).

확인:

```text
$ grep -n "cause" src/screens/RecordingDetailScreen.tsx
(일치 없음 — 이 컴포넌트는 어떤 화면의 실패 갈래도 읽지 않는다)
```
