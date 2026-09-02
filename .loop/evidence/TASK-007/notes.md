# TASK-007 — Navigation shell (Recordings · Recording · Recording Detail · Settings)

## 무엇을 만들었나

| 파일 | 역할 |
| --- | --- |
| `src/navigation/routes.ts` | 라우트 정의와 화면 전이. DOM·React·Tauri 의존성 없음 (순수 모듈) |
| `src/navigation/routes.test.ts` | 네 화면 등록 · 도달 · 전이 · 뒤로 가기 테스트 (12 tests) |
| `src/screens/types.ts` | 모든 화면이 받는 공통 props (`route` · `navigate` · `goBack`) |
| `src/screens/RecordingsScreen.tsx` | 목록 화면. 목록이 비어 있으므로 empty state를 보여준다 |
| `src/screens/RecordingScreen.tsx` | 녹음 화면 idle 상태 (경과 시간 `00:00` · 마이크 미선택) |
| `src/screens/RecordingDetailScreen.tsx` | AI Note / Transcript / Recording 탭 + 탭별 empty state |
| `src/screens/SettingsScreen.tsx` | Recording 그룹 + Transcription / AI Provider / Notion 섹션 자리 |
| `src/screens/registry.ts` | `Record<ScreenId, ComponentType<ScreenProps>>` — 라우트 → 컴포넌트 |
| `src/screens/registry.test.ts` | 모든 라우트가 실제 컴포넌트로 이어지는지 (3 tests, DOM 불필요) |
| `src/App.tsx` | scaffold 데모 화면을 대체. 사이드바 + header + 라우트별 화면 렌더링 |
| `src/App.css` | §19 방향의 최소 스타일 (hairline · 여백 · typography, card/shadow 없음) |

## 제거한 scaffold 잔재

- `src/App.tsx`의 Vite/Tauri/React 로고 데모 화면과 `invoke("greet")` 호출
- `src/assets/react.svg` · `public/tauri.svg` · `public/vite.svg`
- `index.html`의 `Tauri + React + Typescript` 제목과 vite 파비콘 링크

> `src-tauri`의 예제 `greet` command 자체는 이 Task의 범위가 아니다 (P6/TASK-006이 정리한다).
> 이 Run에서 `src-tauri/` 아래 파일은 하나도 건드리지 않았다.

## 새 의존성

없다. 런타임 라우팅 라이브러리를 추가하지 않았고, 테스트 전용 devDependency
(jsdom · testing-library)도 추가하지 않았다 — 라우트 정의와 전이가 순수 모듈이라
DOM 없이 판정되고, 컴포넌트 연결은 registry 표로 판정되기 때문이다.

## 범위 밖으로 두고 온 것 (의도적)

- 저장소에서 읽은 실제 목록 렌더링 · Settings 값 영속화 · 실패 상태 표시 → P8 (TASK-008)
- 실제 오디오 캡처와 Record / Pause / Resume / Stop → Phase 2
- Transcription / AI Provider / Notion 섹션의 실제 기능 → Phase 3·4·5
- secret(API key · integration token) 입력 필드 — Phase 1에서 다루지 않는다 (INV-7).
  `src/` 어디에도 secret 입력 필드나 provider 호출 코드가 없다.

## Gate 실행 결과

`node tools/loop-runtime/loopctl.mjs self-check build lint test` (2026-09-02)

| Gate | command | exit | 결과 |
| --- | --- | --- | --- |
| build | `npm run build` | 0 | PASS — `tsc && vite build`, 35 modules |
| lint | `npm run lint` | 0 | PASS — `eslint .` + `cargo clippy -D warnings` |
| test | `npm run test` | 0 | PASS — vitest 19 passed (3 files) + cargo 14 passed |

전체 출력은 `gate-build.stdout.log` · `gate-lint.stdout.log` · `gate-test.stdout.log`.

self-check는 참고용이다. 완료 판정은 Runtime과 Verifier가 한다.

## 사람 확인 항목 (자동 PASS로 적지 않는다)

- 화면 레이아웃이 §19의 방향(minimal · macOS-like · typography first)에 실제로 맞는지는
  실행 화면을 사람이 봐야 판단할 수 있다. 이 Run은 그것을 검증하지 않았다.
