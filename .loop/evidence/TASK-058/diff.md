# TASK-058 — 변경 파일

## 새로 만든 것 (4)

```text
src/platform/clipboard.ts        195줄  clipboard 쓰기 경계 하나 (ADR-0010 §7.2)
src/platform/clipboard.test.ts   209줄  경계의 값 판정 — 전부 test double로
src/screens/copyView.ts          424줄  복사 상태 네 갈래의 순수 판정 (DOM·clipboard 없음)
src/screens/copyView.test.ts     361줄  네 갈래 · 실패 갈래 · 대체 경로 · 문장 동반
```

`src/platform/`은 새 디렉터리다. 이름은 이미 같은 뜻으로 쓰이는 `src-tauri/src/platform/`에서
왔다 — **플랫폼 지식이 갇혀 있는 경계**다 (INV-10 · ADR-0010 §7.2).

## 고친 것 (1)

```text
tests/screen-boundary.test.ts    +84줄 / -0줄
```

기존 검사는 하나도 지우거나 약하게 하지 않았다 (`git diff --stat`: 84 insertions, 0 deletions).
더한 것은 `describe('clipboard는 경계 하나 뒤에 있다 (R-4 · INV-10)')` 하나이며 네 가지를 본다 —
호출 자리가 정확히 하나인지 · 그것이 React 컴포넌트가 아닌지 · 순수 모듈이 clipboard를 부르지
않는지 · 자동 테스트가 실제 clipboard를 집지 않는지.

## 건드리지 않은 것

```text
src-tauri/**                     Rust 변경 0 (cargo lib 436 tests 그대로)
package.json · Cargo.toml        새 의존성 0
tauri.conf.json · capabilities/  새 권한 항목 0
src/ipc/**                       새 command 0 — 이 경계는 invoke를 쓰지 않는다
src/screens/*.tsx                컴포넌트 변경 0 — 화면 연결은 P5(TASK-059)의 범위다
src/App.css                      변경 0 — UI 기반은 P7(TASK-061)의 범위다
```

`tests/ipc-boundary.test.ts`의 `'src/ 아래에서 command를 부르는 곳은 ipc 모듈뿐이다'`는 그대로
통과한다 — clipboard 경계는 `invoke`를 쓰지 않는다 (ADR-0010 §8.3의 예고 그대로).
