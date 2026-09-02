# TASK-013 Evidence — Phase 2A spike 표면

날짜: 2026-09-02
Run: RUN-20260902T072925Z-TASK-013

## 이 Run에서 바뀐 파일

| 파일 | 상태 | 내용 |
| --- | --- | --- |
| `src/screens/captureSpikeView.ts` | 새 파일 | spike 표면의 순수 상태 모듈 (DOM · Tauri · 하드웨어 없음) |
| `src/screens/captureSpikeView.test.ts` | 새 파일 | 위 모듈의 vitest 테스트 25개 (마이크 · 권한 전제 없음) |
| `src/screens/RecordingScreen.tsx` | 수정 | 임시 spike 패널 렌더링 (라우트 · registry 변경 없음) |
| `src/App.css` | 수정 | `.spike-notice` · `.spike-result` 두 클래스 추가 |

`src/ipc/*`의 변경은 이 Run의 것이 아니다 (TASK-012에서 만들어진 미커밋 상태다).
git commit은 하지 않았다.

## Acceptance Criteria 대응

| AC | 무엇이 판정하는가 | 결과 |
| --- | --- | --- |
| AC1 build | `npm run build` | PASS (`gates/build-stdout.log`) |
| AC2 lint | `npm run lint` (eslint + cargo clippy `-D warnings`) | PASS (`gates/lint-stdout.log`) |
| AC3 test | `npm run test` (vitest + cargo test) | PASS — 웹 96개 / Rust 139개 (`gates/test-stdout.log`) |
| AC4 네 값 표시 | `RecordingScreen.tsx`의 Result 절이 `deviceLabel` · `outputPath` · `formatText` · `sizeText`를 모두 그린다 | 아래 "표시 경로" 참조 |
| AC5 임시 표기 | `RecordingScreen.tsx`의 `<p className="spike-notice">`가 렌더 경로에 있다 (주석이 아니다) | 아래 참조 |
| AC6 순수 로직 · 규약 재사용 | 테스트 대상이 `captureSpikeView.ts`이고, 화면은 `src/ipc/commands.ts`와 `FailureNotice`만 쓴다 | 아래 참조 |

## 표시 경로 (AC4)

```text
listInputDevices()  →  loadedInputDevices()  →  radio 목록 (device.label · default 표기)
   ↓ 사용자가 고름
selectedInputDevice() → view.selectedKey
   ↓ Start
startCapture(view.selectedKey) → startedCapture() → captureStatusText('recording')
   ↓ Stop
stopCapture() → finishedCapture(report) → toCaptureResult(report)
   → <dl className="spike-result">
        Device : result.deviceLabel     (report.deviceLabel)
        File   : result.outputPath      (report.outputPath)
        Format : result.formatText      (report.format — Rust가 만든 문장 그대로)
        Size   : result.sizeText        (formatByteSize(report.byteSize))
      </dl>
   → byteSize가 0이면 "The file is empty — nothing was written." 가 함께 보인다
```

## 임시성 (AC5)

- 렌더 경로 안의 첫 요소가 임시 표기다:
  `<p className="spike-notice" role="note"><strong>Phase 2A spike — temporary surface.</strong> …
  Phase 2B replaces it with the real Recording screen.</p>`
- 최종 Recording UX를 만들지 않았다 — 경과 시간 · pause/resume · 재생 · 레코드 저장이 없다.
  (`.recording__elapsed`의 큰 타이머 표시를 쓰지 않는다.)
- navigation 구조는 그대로다: `src/navigation/routes.ts`와 `src/screens/registry.ts`는
  이 Run에서 수정되지 않았다 (`git status` 기준 변경 없음). 화면 4개 · 라우트 4개 그대로다.

## 규약 재사용 (AC6)

- backend 접근은 `src/ipc/commands.ts`의 `listInputDevices` · `startCapture` · `stopCapture`뿐이다.
  화면에 `invoke`가 없다 — `tests/ipc-boundary.test.ts`가 "invoke를 부르는 파일은
  `src/ipc/commands.ts` 하나"임을 소스 전체에 대해 검사하고, test Gate가 green이다.
- 실패 표시는 기존 `FailureNotice` / `failureView`를 그대로 쓴다. 새 실패 표현을 만들지 않았다.
- 새 테스트는 순수 모듈(`captureSpikeView.ts`)만 대상으로 한다 — React · jsdom · 마이크 ·
  마이크 권한 · 파일시스템을 쓰지 않는다.

## 이 Run이 판정하지 않은 것

ADR-0003 §12의 Human Review 8개 항목은 여기서 판정되지 않는다. 실제 마이크 권한 프롬프트 ·
실제 장치 열기 · 녹음 가청성 · 실제 파일 포맷은 사람이 번들된 `.app`에서 확인해야 한다.
자동 Gate가 green인 것은 그 확인이 끝났다는 뜻이 아니다.
