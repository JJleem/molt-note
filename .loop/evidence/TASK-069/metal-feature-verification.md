# TASK-069 — `whisper-rs`의 `metal` feature가 **실제로 빌드에 영향을 주는가**

ADR-0007 §17.2.2가 "실제로 feature를 켜는 Task가 해야 할 일"로 적은 세 가지 중 3번이 이 문서의
목적이다 — *"feature를 켠 뒤 실제로 달라졌는지로 판정한다. ADR-0009 §11.4가 TLS feature에 대해
세운 규칙과 같다 — 켰다고 적어 두고 산출물이 그대로면 켜지지 않은 것이다."*

**`Cargo.toml`에 이름이 적혔다는 것은 이 문서의 판정 근거가 아니다.** 아래는 전부 이 저장소
안의 파일을 직접 읽어 얻은 관측이며, 관측과 추론을 §3에서 나눈다.

원자료 두 개가 같은 디렉터리에 있다 (명령 출력 그대로):

```text
.loop/evidence/TASK-069/before-metal-off.txt   feature 지정이 없던 상태
.loop/evidence/TASK-069/after-metal-on.txt     feature를 켠 뒤
.loop/evidence/TASK-069/cargo-toml.diff        변경 diff
```

---

## 1. 확인 방법과 그 한계 — 무엇을 읽을 수 없었는가

**crate 소스 파일 자체는 읽지 못했다.** ADR-0007 §17.2.2가 1번으로 요구한
*"`Cargo.toml`을 고치기 전에 `whisper-rs` 0.16.0의 `[features]`를 직접 읽는다"* 는 **문자 그대로는
성립하지 않았다.** 시도와 결과를 그대로 적는다.

| 시도한 경로 | 결과 (명령의 실제 출력) |
| --- | --- |
| registry에 추출된 crate 소스 — `ls /Users/molt/.cargo/registry/src/` | `blocked. For security, Claude Code may only list files in the allowed working directories for this session: '/Users/molt/orca/projects/molt-note'` |
| `cargo metadata --offline --format-version 1` (패키지별 feature 표를 얻는 경로) | 실행 불가 — 이 Run이 돌릴 수 있는 명령은 `node tools/loop-runtime/loopctl.mjs self-check [<gate>]` 뿐이며 이 명령은 승인되지 않았다 |
| docs.rs · Codeberg의 feature 문서 | 네트워크 접근이 없다 |
| `src-tauri/Cargo.lock` | 버전은 확인된다 — `whisper-rs` **0.16.0** (checksum `2088172d…`) · `whisper-rs-sys` **0.15.0** (checksum `6986c0fe…`). **lock은 crate가 어떤 feature를 갖는지 기록하지 않는다** |

TASK-047이 `ureq`의 TLS feature에 대해 만난 것과 **같은 제약이고 같은 대응**이다
(`.loop/evidence/TASK-047/ureq-tls-verification.md` §1): 문서에서 읽어 확인할 수 없으므로
**켰다고 믿지 않고 산출물로 판정했다.**

### 1.1 ★ 네 번째 경로는 열려 있었다 — cargo가 기록한 `declared_features`

위 네 경로는 이 Run에서도 전부 막혀 있다. 그러나 **다섯 번째 경로가 있었고 그것은 열려 있다.**

cargo는 crate를 컴파일할 때 fingerprint JSON에 `declared_features` 필드를 남긴다.
그 값은 **cargo가 그 crate의 manifest를 파싱해 얻은 `[features]` 표 그대로**이며,
파일은 이 저장소의 `src-tauri/target/` 안에 있으므로 **읽을 수 있다.**

```text
$ cat src-tauri/target/debug/.fingerprint/whisper-rs-2c6df6183aa4345e/lib-whisper_rs.json

  "features":         "[\"_gpu\", \"default\", \"metal\"]"
  "declared_features":"[\"_gpu\", \"coreml\", \"cuda\", \"default\", \"hipblas\", \"intel-sycl\",
                        \"log_backend\", \"metal\", \"openblas\", \"openmp\", \"raw-api\",
                        \"test-with-tiny-model\", \"tracing_backend\", \"vulkan\"]"

$ cat src-tauri/target/debug/.fingerprint/whisper-rs-sys-7f1b7ef590cc7566/lib-whisper_rs_sys.json

  "features":         "[\"metal\"]"
  "declared_features":"[\"coreml\", \"cuda\", \"force-debug\", \"hipblas\", \"intel-sycl\",
                        \"metal\", \"openblas\", \"openmp\", \"vulkan\"]"
```

원자료 전문: `.loop/evidence/TASK-069/declared-features.txt`

**이것이 바꾸는 것:**

1. `metal`은 두 crate 모두에 **선언된 feature 이름이다.** 더 이상 *"cargo가 거절하지 않았으니
   있을 것"* 이라는 추론이 아니다 — **cargo가 파싱한 이름 목록에 그 철자가 있다.**
2. **Metal 관련 이름은 그 목록에 하나뿐이다.** §3.2가 *"유일한/최선의 이름인지 확인하지
   않았다"* 로 남겼던 자리가 좁혀졌다.
3. `coreml` · `openmp`가 **별개의 선언된 feature**라는 것이 확인됐고, 켜진 목록(`features`)에
   그 둘이 없다. Task가 요구한 *"CoreML · OpenMP는 켜지 않는다"* 가 manifest 기재가 아니라
   **cargo의 기록으로** 확인된다.

**이것이 바꾸지 않는 것:** 이것은 crate의 `Cargo.toml` **파일을 연 것이 아니라 cargo가 그
파일을 읽고 남긴 기록을 연 것**이다. 그 구분은 §3.2에서 유지한다. 그리고 이 목록은 **이름만
말하고 각 feature가 무엇을 하는지는 말하지 않는다** — `metal`이 실제로 무엇을 바꾸는지의
판정은 여전히 §2.3~§2.7의 빌드 산출물이다.

---

## 2. 관측 — before / after

before = 2026-09-03에 만들어진 빌드 산출물(feature 지정 없음),
after = 2026-09-06에 `features = ["metal"]`로 다시 돈 빌드 산출물.
네 개의 `whisper-rs-sys-*/out` 디렉터리가 target에 남아 있고, **before 2개와 after 2개**다.

### 2.1 `Cargo.toml`의 한 줄

```text
before:  whisper-rs = "0.16"
after:   whisper-rs = { version = "0.16", features = ["metal"] }
```

### 2.2 cargo 자신이 기록한 feature 집합 (`.fingerprint/*.json`)

```text
whisper-rs-98d41d5c13749804/lib-whisper_rs.json         "features":"[\"default\"]"                  ← before
whisper-rs-d6c13803281996e9/lib-whisper_rs.json         "features":"[\"default\"]"                  ← before
whisper-rs-2c6df6183aa4345e/lib-whisper_rs.json         "features":"[\"_gpu\", \"default\", \"metal\"]"   ← after
whisper-rs-832fc319169ff7b4/lib-whisper_rs.json         "features":"[\"_gpu\", \"default\", \"metal\"]"   ← after

whisper-rs-sys-937543fec3ce9af6/lib-whisper_rs_sys.json "features":"[]"                             ← before
whisper-rs-sys-c774acbcc09ac244/lib-whisper_rs_sys.json "features":"[]"                             ← before
whisper-rs-sys-7f1b7ef590cc7566/lib-whisper_rs_sys.json "features":"[\"metal\"]"                    ← after
whisper-rs-sys-c4f9dc0cc6d5eba4/lib-whisper_rs_sys.json "features":"[\"metal\"]"                    ← after
```

**우리가 켠 이름은 `metal` 하나인데 cargo가 해석한 집합에는 `_gpu`가 함께 있고, 그것이
`whisper-rs-sys`에도 `metal`로 전파됐다.** 이것은 cargo가 crate의 feature 정의를 읽고 만든
기록이다 (§3.2에서 이 관측이 무엇을 뜻하고 무엇을 뜻하지 않는지 나눈다).

### 2.3 ★ build.rs가 CMake에 넘긴 정의 — `GGML_METAL`

```text
$ grep -E '^GGML_METAL' src-tauri/target/debug/build/whisper-rs-sys-*/out/build/CMakeCache.txt

3a8019dcfcfe9c4c  (Sep  3)   GGML_METAL:BOOL=OFF   GGML_METAL_EMBED_LIBRARY:BOOL=OFF   GGML_METAL_NDEBUG:BOOL=OFF
f9ac82e95845cca2  (Sep  3)   GGML_METAL:BOOL=OFF   GGML_METAL_EMBED_LIBRARY:BOOL=OFF   GGML_METAL_NDEBUG:BOOL=OFF
316e0b7185149455  (Sep  6)   GGML_METAL:BOOL=ON    GGML_METAL_EMBED_LIBRARY:BOOL=ON    GGML_METAL_NDEBUG:BOOL=ON
1e2c94f2f290a9d1  (Sep  6)   GGML_METAL:BOOL=ON    GGML_METAL_EMBED_LIBRARY:BOOL=ON    GGML_METAL_NDEBUG:BOOL=ON
```

**ADR-0007 §17.2.1이 [E5]로만 갖고 있던 `GGML_METAL = OFF`가 이 저장소의 빌드 산출물에서
직접 확인됐고(before 2개), feature를 켠 뒤 `ON`으로 바뀌었다(after 2개).**

### 2.4 이 Task가 켠 것은 Metal 하나뿐이다 — 다른 가속 옵션은 그대로다

같은 네 `CMakeCache.txt`를 가속 관련 키로 비교했다.

| 키 | before (2개) | after (2개) |
| --- | --- | --- |
| `GGML_METAL` · `GGML_METAL_EMBED_LIBRARY` · `GGML_METAL_NDEBUG` | OFF | **ON** |
| `WHISPER_COREML` · `WHISPER_COREML_ALLOW_FALLBACK` | OFF | OFF |
| `GGML_OPENMP` | OFF | OFF |
| `WHISPER_OPENVINO` · `GGML_CUDA` · `GGML_VULKAN` | OFF | OFF |
| `GGML_BLAS` (`GGML_BLAS_VENDOR=Apple`) | ON | ON |
| `GGML_ACCELERATE` | ON | ON |

**값이 달라진 키는 `GGML_METAL*` 뿐이다.** `GGML_BLAS`와 `GGML_ACCELERATE`는 **before에도 이미
켜져 있었다** — 이 Task가 켠 것이 아니다.

### 2.5 산출물 — `libggml-metal.a`가 새로 생겼다

```text
before  3a8019dcfcfe9c4c/out/lib (Sep  3 12:22)   libggml-base.a  libggml-blas.a  libggml-cpu.a  libggml.a  libwhisper.a
before  f9ac82e95845cca2/out/lib (Sep  3 12:24)   (같음)
after   316e0b7185149455/out/lib (Sep  6 15:12)   … + libggml-metal.a  1,681,096 B
after   1e2c94f2f290a9d1/out/lib (Sep  6 15:13)   … + libggml-metal.a  1,681,096 B
```

`libggml.a`와 `libwhisper.a`도 크기가 달라졌다 (491,392 → 492,240 · 4,734,648 → 4,734,976).
**whisper.cpp가 다시 컴파일됐다는 뜻이다** — ADR-0007 §4.3 · §17.2.4가 대가로 적어 둔 그것이다.

### 2.6 build.rs가 cargo에 준 링크 지시 (`build/*/output`)

```text
before (3a8019dcfcfe9c4c)                 after (316e0b7185149455)
cargo:rustc-link-lib=dylib=c++            cargo:rustc-link-lib=dylib=c++
cargo:rustc-link-lib=framework=Accelerate cargo:rustc-link-lib=framework=Accelerate
                                          cargo:rustc-link-lib=framework=Foundation   ← 새로
                                          cargo:rustc-link-lib=framework=Metal        ← 새로
                                          cargo:rustc-link-lib=framework=MetalKit     ← 새로
cargo:rustc-link-lib=static=whisper       cargo:rustc-link-lib=static=whisper
cargo:rustc-link-lib=static=ggml          cargo:rustc-link-lib=static=ggml
cargo:rustc-link-lib=static=ggml-base     cargo:rustc-link-lib=static=ggml-base
cargo:rustc-link-lib=static=ggml-cpu      cargo:rustc-link-lib=static=ggml-cpu
cargo:rustc-link-lib=static=ggml-blas     cargo:rustc-link-lib=static=ggml-blas
                                          cargo:rustc-link-lib=static=ggml-metal      ← 새로
```

같은 파일의 CMake 로그 (after에만 있다 — before에서 `METAL backend` 문자열의 개수는 **0**이다):

```text
-- Metal framework found
-- Including METAL backend
Embedding Metal library
```

### 2.7 ★ 사슬의 끝 — Gate가 **실제로 실행한 바이너리**에 Metal이 들어 있다

`test` Gate가 실행한 통합 테스트 바이너리를 직접 열어 봤다. 어떤 바이너리인지는 Gate 로그가
지목한다 (`.loop-local/self-check/gates/test/stderr.log`):

```text
Running tests/transcription_engine.rs (src-tauri/target/debug/deps/transcription_engine-ae6a83e38b980483)
```

그 파일에 대해:

```text
$ grep -ac ggml_metal_kargs_argsort  …/transcription_engine-ae6a83e38b980483
6

$ grep -aoE "/System/Library/Frameworks/Metal[A-Za-z]*\.framework/Versions/A/[A-Za-z]*" … | sort -u
/System/Library/Frameworks/Metal.framework/Versions/A/Metal
/System/Library/Frameworks/MetalKit.framework/Versions/A/MetalKit
```

**manifest → cargo가 해석한 feature → CMake 정의 → 정적 라이브러리 → 링크 지시 → 실제로
링크된 실행 파일**까지 사슬이 끊기지 않는다.

---

## 3. 판정 — 관측과 추론을 섞지 않는다

### 3.1 관측된 사실 [E1 · 이 Run이 파일을 직접 읽었다]

1. feature 지정이 없을 때 `GGML_METAL:BOOL=OFF`였고, `features = ["metal"]` 뒤에 `ON`이 됐다.
2. `libggml-metal.a`가 새로 생겼고 `libggml.a` · `libwhisper.a`의 크기가 달라졌다.
3. build.rs가 `Metal` · `MetalKit` · `Foundation` framework와 `ggml-metal` 정적 라이브러리를
   링크하라고 cargo에 지시한다 (before에는 그 줄이 없다).
4. `test` Gate가 실행한 바이너리 안에 `ggml_metal_*` 심볼과 Metal framework 경로가 있다.
5. 켠 것은 Metal 하나다 — CoreML · OpenMP · OpenVINO · CUDA · Vulkan은 before/after 모두 OFF다.
6. cargo의 fingerprint에 `whisper-rs = ["_gpu","default","metal"]` ·
   `whisper-rs-sys = ["metal"]`이 기록됐다.
7. **같은 fingerprint의 `declared_features`에 두 crate의 선언된 feature 이름 목록이 있고,
   그 안에 `metal`이 있다.** Metal 관련 이름은 그 목록에 하나뿐이며, `coreml` · `openmp`는
   별개 이름으로 존재하고 켜지지 않았다 (§1.1 · `declared-features.txt`).

**§17.2.2-3이 요구한 판정은 통과했다: 켰다고 적은 것과 산출물이 일치한다.**

> **이 여섯 관측(1~6)은 이 Task의 앞선 Worker 실행이 먼저 기록했고, 뒤이은 Run이 같은 파일을
> 다시 열어 독립적으로 재확인했다.** 재확인한 것: `CMakeCache.txt` 네 개의 `GGML_METAL`
> 값(OFF 2 / ON 2), `libggml-metal.a`의 존재와 크기(1,681,096 B)와 mtime,
> `libggml.a` · `libwhisper.a`의 크기 차이, 링크된 테스트 바이너리의
> `grep -ac ggml_metal_kargs_argsort` = **6**, `Cargo.lock`의 버전과 checksum
> (`whisper-rs` 0.16.0 `2088172d…` · `whisper-rs-sys` 0.15.0 `6986c0fe…`).
> **전부 앞선 기록과 일치했다.** 7번은 뒤이은 Run이 새로 찾은 것이다.

### 3.2 추론 — 여기부터는 관측이 아니다

| 진술 | 종류 | 근거의 한계 |
| --- | --- | --- |
| `metal`이라는 이름의 feature가 `whisper-rs` 0.16.0에 **실재한다** | ~~강한 추론~~ → **관측** (§1.1) | cargo가 파싱한 `declared_features` 목록에 그 이름이 있다. **crate의 `Cargo.toml` 파일 자체를 연 것은 아니다** — cargo가 그 파일을 읽고 남긴 기록을 읽었다 |
| `whisper-rs`의 `metal`이 `_gpu`를 함께 켜고 `whisper-rs-sys/metal`로 전파된다 | **강한 추론** | §2.2의 fingerprint는 cargo가 crate 정의를 읽고 만든 기록이다. 켜진 집합은 관측이지만, **`metal`이 `_gpu`를 켜는 원인이라는 것**(feature 정의의 내용)은 원본 정의를 읽어 확인한 것이 아니다 |
| `metal`이 이 crate에서 Metal을 켜는 **유일한** 이름이다 | **강한 추론** | §1.1의 선언된 이름 목록에 Metal 관련 이름이 `metal` 하나뿐이다. 다만 목록은 **이름만 말하고 각 이름이 무엇을 하는지는 말하지 않으므로**, 다른 이름이 Metal에 영향을 줄 가능성을 목록만으로 배제하지는 못한다. 다른 후보 이름을 시험해 보지는 않았다 |
| 실행 시 앱이 **실제로 GPU를 쓴다** | **확인하지 않았다** | 이 Task는 추론(inference)을 한 번도 돌리지 않았다. 링크된 바이너리에 Metal 백엔드가 들어 있다는 것과, 런타임에 그 백엔드가 선택돼 GPU가 잡힌다는 것은 **다른 진술**이다. 2026-09-05 운영자 관측(`GPU name: Apple M5`)은 [E5]이며 **저장소 밖의 별도 도구**에 대한 것이다 (ADR-0007 §17.2.2) |

### 3.3 `Cargo.lock` — **바뀌지 않았다**

Task는 "`Cargo.lock`을 함께 갱신하고"라고 적었다. **갱신되지 않았고, 갱신할 것이 없었다.**
사실과 이유를 나눠 적는다.

**관측:**

```text
$ git status --porcelain src-tauri/Cargo.toml src-tauri/Cargo.lock
 M src-tauri/Cargo.toml
                              ← Cargo.lock 은 줄 자체가 없다 (변경 없음)
```

`lint`(=`cargo clippy --all-targets`)와 `test`(=`cargo test`)가 이 lock을 읽고 두 번 다 exit=0으로
끝났다. cargo는 lock을 고치지 않았고, 새 패키지를 들여왔다는 `Adding …` 출력도 없다.

**해석 (추론):** `Cargo.lock`은 **해석된 패키지와 버전과 의존 관계**를 기록하지, **어떤 feature가
켜졌는지는 기록하지 않는다.** `metal`은 새 crate를 끌어오지 않고 `whisper-rs-sys`로 전파되는
빌드 스위치이므로 (§2.2 — `whisper-rs-sys`의 의존 목록은 before/after가 같다) lock이 바이트
단위로 같은 것이 **정상이고 예상된 결과**다.

**이것이 TASK-047과 다른 점을 분명히 적는다.** ADR-0009 §11.4의 규칙 — *"feature를 켰다고 적어
두고 lock이 그대로면 켜지지 않은 것이다"* — 은 `ureq`의 `rustls`처럼 **feature가 새 crate를
들여오는 경우**의 판정이었다 (그때 lock에 `rustls` · `ring` · `webpki-roots`가 새로 생겼다).
`metal`은 그런 종류가 아니다. **그래서 이 Task는 판정 기준을 lock이 아니라 빌드 산출물에 걸었고**,
그것이 §2.3~§2.7이다. 규칙의 태도(*"적어 둔 것 말고 실제로 달라진 것을 본다"*)는 그대로 지켰다.

### 3.4 속도 — **측정하지 않았다**

**이 Task는 Metal의 성능 효과를 측정하지 않았다.** 배수도, 소요 시간 추정치도, "빨라졌다"는
서술도 이 문서 · `Cargo.toml` 주석 · 어떤 문서에도 새로 적지 않았다 (ADR-0007 §17.2.3).
Metal을 켠 근거는 **측정된 속도가 아니라 PRODUCT-SPEC §14.4와 저장소 사이의 간극**이다.

아래 §4의 Gate 소요 시간은 **빌드 시간**이지 전사 속도가 아니다.

---

## 4. Gate 결과

전문은 `.loop/evidence/TASK-069/gate-results.md`에 있다. 요약만 여기 둔다.

```text
$ node tools/loop-runtime/loopctl.mjs self-check lint test     (최종 상태 · 뒤이은 Run)
lint: PASS  exit=0   7.2s
test: PASS  exit=0  12.8s
Self-check: all gates passed
```

앞선 Worker 실행도 같은 명령을 두 번 돌려 두 번 다 PASS했다 (`2.3s`/`5.2s` · `6.4s`/`5.3s`).
**최종 트리 상태(주석까지 들어간 상태)에 대한 판정 근거는 위의 실행이다.**

그 실행이 돌린 통합 테스트 바이너리는 §2.7에서 연 것과 **같은 파일이다**
(`transcription_engine-ae6a83e38b980483` — Gate 로그의 `Running tests/transcription_engine.rs`
줄이 지목한다). 주석 변경은 링크 결과를 바꾸지 않았다.

기록: `.loop-local/self-check/gates/{lint,test}/{stdout,stderr}.log` ·
`.loop/evidence/TASK-069/gate-results.md`.

**이 값들은 cold build가 아니다.** 이미 Metal로 컴파일된 `target/`이 있는 상태의 증분 빌드이며
(`Finished \`test\` profile … in 0.22s`), whisper.cpp를 다시 컴파일하지 않았다.

**whisper.cpp 재컴파일을 포함한 Gate 실행은 같은 Task의 앞선 Worker 실행이 관측했다** —
`after-metal-on.txt` §9가 `lint 20.1s` · `test 55.0s`로 적었고, 그 시각(15:12·15:13)이
`libggml-metal.a`의 mtime과 같다. **900초 timeout을 넘지 않았다** (`.loop/project.yaml`).
그 두 값을 이 Run이 다시 측정한 것은 아니다 — 산출물 mtime과 그 파일의 기록에 근거한 서술이다.

---

## 5. 여전히 UNVERIFIED — 확인한 것처럼 적지 않는다

| 항목 | 상태 | 왜 |
| --- | --- | --- |
| `whisper-rs` 0.16.0의 **`[features]` 전체 목록** | ~~[E4] UNVERIFIED~~ → **확인됨 (단, 출처가 crate 소스 파일이 아니다)** | §1의 네 경로는 여전히 막혀 있다. 그러나 cargo가 남긴 `declared_features`에 두 crate의 선언된 이름 목록 전체가 있다 (§1.1 · `declared-features.txt`). **crate의 `Cargo.toml`을 연 것이 아니라 cargo가 그것을 읽고 기록한 것을 열었다** — 이 구분을 지운 채로 [E1]이라고 적지 않는다 |
| **런타임에 GPU가 실제로 쓰이는가** | **확인하지 않았다** | 이 Task는 추론을 돌리지 않았다 (§3.2). 사람의 Human Review 항목이다 (`phase-prompt/05.6`) |
| **Metal이 전사를 얼마나 빠르게 하는가** | **측정하지 않았다** | §3.4 |
| 번들 whisper.cpp의 실제 버전 | **[E4] UNVERIFIED** | ADR-0007 §16.3이 남긴 별개 항목이며 이 Task의 범위가 아니다 |
| release 빌드 · 번들된 `.app`에서의 동작 | **UNVERIFIED** | Gate는 debug 프로파일이다 |

**ADR-0007 §17.2.2의 2번(“확인된 이름을 [E1]로 되적고 UNVERIFIED 표시를 걷는다”)은 이 Task가
하지 않았다 — ADR 본문 수정은 이 Task의 request 범위 밖이다** (request가 지목한 것은
`Cargo.toml` · `Cargo.lock` · 주석 · evidence이며 "제품 코드는 바꾸지 않는다"고 적혀 있다).

**다만 그 항목이 열려 있는 이유는 앞선 Worker 실행 때와 달라졌으므로, 그 사실을 남긴다.**
앞선 실행은 *"crate 소스에서 읽은 이름이 없어서 못 한다"* 고 적었다. 이제는 **이름 목록이
있다** (§1.1). 남은 것은 **그 출처를 어떻게 표기할 것인가**이며, 그것은 ADR을 고치는 사람의
판단이다:

```text
확인된 것:  whisper-rs 0.16.0 · whisper-rs-sys 0.15.0 두 crate 모두에
            `metal` 이라는 feature 이름이 선언돼 있다
출처:       cargo 가 manifest 를 파싱해 fingerprint 에 남긴 declared_features
            (crate 의 Cargo.toml 파일 자체가 아니다)
원자료:     .loop/evidence/TASK-069/declared-features.txt
```

**읽지 않은 것을 읽은 것처럼 적지 않는다** (PRODUCT-SPEC §20.2). ADR §3의 표기 체계에서
이것을 [E1]으로 올릴지, 출처를 밝힌 별도 표기로 둘지는 그 문서의 결정이다.

---

## 6. 이 결정이 틀렸을 때 무너지는 범위

```text
바뀌는 것:     src-tauri/Cargo.toml 의 한 줄 (features 배열)
바뀌지 않는 것: transcription/** 의 어떤 코드도 · 테스트 · 설정 · 화면
```

제품 코드는 `whisper-rs`의 API만 부르고 백엔드를 고르지 않는다. Metal을 되돌리는 것은
`features = ["metal"]`을 지우고 whisper.cpp를 한 번 다시 컴파일하는 일이다.
