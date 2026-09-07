# TASK-080 — Gate 실행 기록

Runtime 소유 진입점으로 실행했다. 재실행 방법은 아래 명령 그대로다.

```text
node tools/loop-runtime/loopctl.mjs self-check test
node tools/loop-runtime/loopctl.mjs self-check build lint
```

`.loop/project.yaml`이 정의한 명령은 각각 `npm run build` · `npm run lint` · `npm run test`다.
아래는 self-check가 출력한 것 그대로이며, **판정이 아니라 참고용이다** — Runtime이 Worker
종료 후 Gate를 독립적으로 다시 돌린다.

## 최종 결과 (2026-09-07)

```text
[build] npm run build
[lint]  npm run lint
[test]  npm run test

build: PASS  exit=0   1.1s
lint:  PASS  exit=0   5.7s
test:  PASS  exit=0  43.9s

Self-check: all gates passed
```

`npm run test`는 `vitest run`(frontend)과 `cargo test`(backend)를 모두 돈다.
새 단위 테스트는 `src-tauri/src/audio/level.rs`의 `#[cfg(test)] mod tests`에 있으므로
`cargo test` 쪽에서 판정된다.

## 중간에 실제로 빨개진 것 두 개 — 무엇이 걸렸고 무엇을 고쳤나

두 실패 모두 **검사가 실제 결함을 잡은 경우**다. 테스트를 지우거나 약화해서 통과시키지 않았다.

### 1. `tests/audio-boundary.test.ts` — 저장소 전체 규칙에 걸렸다

```text
FAIL  tests/audio-boundary.test.ts > 실제 오디오 장치를 아는 코드는 두 파일 안에만 있다
AssertionError: expected [ …(3) ] to deeply equal [ …(2) ]
+   "/Users/molt/orca/projects/molt-note/src-tauri/src/audio/level.rs",
    "/Users/molt/orca/projects/molt-note/src-tauri/src/audio/system_capture.rs",
    "/Users/molt/orca/projects/molt-note/src-tauri/src/audio/system_devices.rs",
```

원인: `level.rs`의 순수성 검사가 장치 라이브러리 이름을 **테스트 코드 안의 문자열**로 들고
있었고, 저장소 전체를 보는 그 검사가 주석이 아닌 줄에서 그 이름을 발견했다.

고친 방법: 그 needle을 지웠다. **같은 규칙이 두 자리에 있을 이유가 없다** —
"장치를 아는 파일은 둘뿐"이라는 규칙은 `tests/audio-boundary.test.ts`가 이미 소유한다.
`level.rs`의 검사는 나머지 바깥 세계(파일 · 스레드 · 저장소 · 시계)만 본다.

### 2. `the_peak_is_reported_but_never_decides_the_verdict` — 테스트 자신의 신호가 틀렸다

```text
thread 'audio::level::tests::the_peak_is_reported_but_never_decides_the_verdict' panicked:
assertion `left == right` failed
  left: Usable
 right: Low
```

원인: 낮은 레벨 1,000 샘플에 풀스케일 샘플 **하나**를 더했더니 평균 RMS가 -29.7 dBFS까지
올라가 실제로 `쓸 만함`이 됐다. 제품 코드가 아니라 **테스트가 만든 신호가 의도와 달랐다.**

고친 방법: 그 하위 사례의 신호를 100,000 샘플로 바꿔 *"51분 녹음에서 큰 소리 하나가
판정을 뒤집지 않는다"* 를 실제로 성립하게 만들었다. 그 결과 평균이 **-42.2 dBFS**가 되어
ADR-0003 §16.1이 적은 9/7 관측값과 소수 첫째 자리까지 같아졌고, 판정은 `낮음`이다.
