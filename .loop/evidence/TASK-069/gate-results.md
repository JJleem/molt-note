# TASK-069 — Gate 실행 기록

`metal-feature-verification.md` §4가 가리키는 파일이다. **Gate는 이 저장소의
`.loop/project.yaml`이 정의한 명령을 Runtime 소유 진입점으로 돌린 것이며, 여기 적힌 결과는
참고용이다** — 완료 판정은 Runtime과 Verifier가 Worker 종료 후 독립적으로 다시 돌려서 한다
(self-check 출력 자체가 그렇게 적는다: *"advisory — the runtime reruns gates independently"*).

```text
lint  →  npm run lint
test  →  npm run test
```

---

## 1. 최종 상태에서의 실행 (이 Run이 직접 돌렸다)

```text
$ node tools/loop-runtime/loopctl.mjs self-check lint test

lint: PASS  exit=0   7.2s
test: PASS  exit=0  12.8s
Self-check: all gates passed
Artifacts: .loop-local/self-check/
```

기록 위치: `.loop-local/self-check/gates/{lint,test}/{stdout,stderr}.log`

이 실행이 대상으로 한 트리 상태 = `Cargo.toml`에 `features = ["metal"]`과 최종 주석이 모두
들어간 상태다. **AC1 · AC2가 요구하는 것이 이것이다** — Metal이 켜진 채로 whisper.cpp를 포함한
Rust 트리 전체가 컴파일되고 테스트가 통과한다.

### 1.1 이 실행은 cold build가 아니다 — 그 사실을 흐리지 않는다

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.22s
```

이미 Metal로 컴파일된 `target/`이 있는 상태의 **증분 빌드**이며, 이 실행 안에서 whisper.cpp가
다시 컴파일되지는 않았다. 그러므로 **위의 7.2초 · 12.8초를 "Metal을 켠 빌드에 걸리는 시간"으로
읽으면 안 된다.**

### 1.2 사슬이 이어지는 자리 — Gate가 실행한 바이너리가 검증 대상과 같은 파일이다

`test` Gate의 로그가 실행한 통합 테스트 바이너리를 지목한다:

```text
Running tests/transcription_engine.rs (src-tauri/target/debug/deps/transcription_engine-ae6a83e38b980483)
```

**`metal-feature-verification.md` §2.7이 열어 `ggml_metal_*` 심볼과 Metal framework 경로를 찾은
파일이 바로 이 파일이다.** 즉 *"Gate가 통과했다"* 와 *"Metal이 링크된 바이너리가 돌았다"* 가
같은 실행을 가리킨다.

`transcription_engine` 실행 결과: `ok. 15 passed; 0 failed; 0 ignored`.

---

## 2. whisper.cpp 재컴파일을 포함한 실행 — 앞선 Worker 실행의 기록

feature를 처음 켠 직후의 Gate 실행은 **whisper.cpp 재컴파일을 포함했고**, 그것을 관측한 것은
이 Task의 앞선 Worker 실행이다 (`after-metal-on.txt` §9):

```text
lint: PASS  exit=0  20.1s      ← 이 실행 안에서 whisper.cpp가 재컴파일됐다
test: PASS  exit=0  55.0s
```

**이 Run이 그 두 값을 다시 측정한 것은 아니다.** 다만 그 서술과 일치하는 산출물이 트리에
남아 있는 것은 이 Run이 직접 확인했다 — `libggml-metal.a`의 mtime이 15:12·15:13이고
(`after-metal-on.txt`가 적은 Gate 실행 시각과 같다), 그 파일은 before 디렉터리(9-03)에는 없다.

| | 값 | 근거 |
| --- | --- | --- |
| Gate timeout | `lint` · `test` 둘 다 **900초** | `.loop/project.yaml` |
| 재컴파일을 포함한 실행 | 20.1초 · 55.0초 | 앞선 Worker 실행의 기록 (`after-metal-on.txt` §9) |
| 판정 | **timeout을 넘지 않았다.** 조정이 필요하지 않다 | ADR-0007 §17.2.4가 *"feature를 켜는 Task가 Gate를 실제로 돌려 측정하고, 900초를 넘으면 그때 timeout을 조정한다"* 고 적었다 |

**이 값들은 빌드 시간이지 전사 속도가 아니다.** 이 Task는 전사 속도를 측정하지 않았다
(`metal-feature-verification.md` §3.4 · ADR-0007 §17.2.3).

### 2.1 측정하지 않은 것

| 항목 | 상태 |
| --- | --- |
| 빈 `target/`에서의 cold build 소요 시간 | **측정하지 않았다** |
| release 프로파일 빌드 | **측정하지 않았다** — Gate는 debug다 |
| Metal이 전사를 얼마나 빠르게 하는가 | **측정하지 않았다.** 배수도 소요 시간도 이 저장소 어디에도 적지 않는다 |
