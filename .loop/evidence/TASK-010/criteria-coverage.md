# TASK-010 Evidence — §6.1 평가 기준 14개 ↔ ADR-0003 대조표

`docs/PRODUCT-SPEC.md` §6.1이 나열한 기준 목록(줄 250~255)과
`docs/ADR-0003-recording-engine.md`의 비교 항목을 하나씩 대조한 것이다.
제3자가 두 파일을 열어 그대로 재확인할 수 있다.

## §6.1 원문 목록

```text
recording reliability      microphone enumeration     microphone selection
pause/resume               file finalization          data-loss behavior
audio format               transcription compatibility
macOS permission behavior  Windows compatibility
Tauri integration          packaging
testability                maintenance cost
```

## 대조

| # | §6.1 기준 | ADR-0003 절 | 세 후보(A·B·C)가 모두 다뤄지는가 |
| --- | --- | --- | --- |
| 1 | recording reliability | §5.1 | 예 |
| 2 | microphone enumeration | §5.2 | 예 |
| 3 | microphone selection | §5.3 | 예 |
| 4 | pause/resume | §5.4 | 예 |
| 5 | file finalization | §5.5 | 예 |
| 6 | data-loss behavior | §5.6 | 예 |
| 7 | audio format | §5.7 | 예 |
| 8 | transcription compatibility | §5.8 | 예 |
| 9 | macOS permission behavior | §5.9 | 예 (A·B + 공통) |
| 10 | Windows compatibility | §5.10 | 예 (A·B·C + 공통) |
| 11 | Tauri integration | §5.11 | 예 |
| 12 | packaging | §5.12 | 예 (A·B·C + 공통) |
| 13 | testability | §5.13 | 예 |
| 14 | maintenance cost | §5.14 | 예 |

**누락 없음 — 14/14.**

§6.1이 "결정적"이라고 지목한 세 기준(transcription compatibility · Windows compatibility ·
data-loss behavior)은 각각 §5.8 · §5.10 · §5.6이며, 셋 모두 해당 절에서 "결정적"이라고
명시하고 잠정 선택의 근거 목록(ADR §6)과 연결했다.

## 후보 목록

| 후보 | ADR §4 | 근거 |
| --- | --- | --- |
| A — WebView / MediaRecorder | 있음 | Task 요구 · §6.1 · §14.3 후보 A |
| B — Rust/native (`cpal` + `hound`) | 있음 | Task 요구 · §6.1 · §14.3 후보 B |
| C — 커뮤니티 Tauri 플러그인 | 있음 | Task 요구 · §14.3 후보 C |
