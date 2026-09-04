# TASK-047 — `ureq`의 TLS 경로 확인 기록 (ADR-0009 §11 · `phase-prompt/05` P-4)

ADR-0009 §11.3은 `ureq` 3.4.0의 TLS feature 이름을 **[E4] UNVERIFIED**로 남기고, 구현 Task가
pin된 버전에서 확인해 §11.4의 네 가지를 남기라고 적었다. 이 문서가 그 기록이다.

> **이 파일에는 실제 자격증명이 하나도 없다.** 이 Run은 Notion token을 만들지도 읽지도 않았고,
> 실제 Notion API를 한 번도 호출하지 않았다 (ADR-0009 §10.5 · `phase-prompt/05` Important Rules).

---

## 1. 확인 방법과 그 한계

ADR-0009 §3.1 · TASK-046이 기록한 제약이 이 Run에도 그대로 있었다.

```text
ls /Users/molt/.cargo/registry/src/
  → blocked: "For security, Claude Code may only list files in the allowed
     working directories for this session: '/Users/molt/orca/projects/molt-note'"
```

WebFetch · WebSearch · 임의 셸 명령도 이 Run에서 쓸 수 없었다. 실행할 수 있는 명령은
`node tools/loop-runtime/loopctl.mjs self-check [<gate> ...]` 하나뿐이며, 그것이 도는 Gate 명령이
`cargo clippy` · `cargo test`다.

**따라서 `ureq` 3.4.0이 제공하는 feature 전체 목록을 문서에서 읽어 확인하지 못했다 — 그 목록은
여전히 [E4] UNVERIFIED다.** 대신 ADR-0009 §11.4가 "핵심 판정"이라고 지목한 3번을 그대로 했다:
**켰다고 믿지 않고, `Cargo.lock`과 빌드 산출물이 실제로 무엇을 들여왔는지로 판정했다.**

---

## 2. §11.4가 요구한 네 가지

### 2.1 (1) pin된 버전에서 그 feature 이름이 유효한가

```toml
ureq = { version = "3.4", default-features = false, features = ["rustls"] }
```

`cargo`가 이 요구를 **해석에 성공했다.** 존재하지 않는 feature 이름은 cargo가 하드 에러로
거절하므로, 이것은 **`rustls`가 pin된 버전(3.4.0)에 실재한다**는 관찰이다 (TASK-046이
`keyring`의 `apple-native` · `windows-native`를 확인한 것과 같은 방법이다).

```text
해석된 버전:  ureq 3.4.0
checksum:     972d7902c8735f2695410b8aed7df6ed12a47394aa1c8d7af49f0497b731a94d
```

`cargo`가 자동으로 lock을 갱신하며 남긴 출력(`.loop-local/self-check/gates/lint/stderr.log`):

```text
    Locking 8 packages to latest compatible versions
      Adding ring v0.17.14
      Adding rustls v0.23.43
      Adding rustls-pki-types v1.15.1
      Adding rustls-webpki v0.103.15
      Adding subtle v2.6.1
      Adding untrusted v0.9.0
      Adding webpki-roots v1.0.9
      Adding windows-sys v0.52.0
```

⚠️ **이것은 "`rustls`가 이 버전의 유일한/최선의 TLS feature다"라는 확인이 아니다.**
확인된 것은 "이 이름이 실재하고, 켰더니 TLS 구현이 실제로 들어왔다"까지다.
`native-tls` 계열 feature의 존재 여부는 **확인하지 않았다** — ADR-0009 §11.3의 1순위가 성립했으므로
2순위를 시험할 이유가 없었다.

### 2.2 (2) `Cargo.toml`의 최종 한 줄과 고른 이유

`src-tauri/Cargo.toml`의 주석이 그대로 적고 있다 — 켠 feature 이름 · 확인한 버전 · 루트 인증서
출처 · 이 문서로 오는 표시. 고른 이유는 ADR-0009 §11.3의 1순위다: **순수 Rust TLS 구현**이라
사용자 기기에 아무것도 설치하지 않는다는 제품 규칙(PRODUCT-SPEC §14.4.2)과 맞고, macOS·Windows에서
같은 코드가 선다.

`default-features = false`는 **유지했다** (§11.2-1). 기본값을 다시 켜면 압축·쿠키·프록시처럼
이 앱이 쓰지 않는 것이 함께 들어온다.

**인증서 검증을 끄는 구성은 없다** (§11.2-3). `notion/network.rs`에는 TLS 설정을 만지는 코드
자체가 없으며, "invalid cert 허용" 류의 옵션은 설정으로도 개발용 플래그로도 두지 않았다.

### 2.3 ★ (3) 핵심 판정 — `Cargo.lock`이 실제로 TLS 구현을 들여왔는가

ADR-0009 §11.1이 기록한 **before**는 "lock에 TLS crate가 하나도 없다"였다. 이 Run이 그 상태를
직접 다시 확인하고 시작했다.

**before** (이 Task의 변경 전 `src-tauri/Cargo.lock`):

```text
ureq 3.4.0 의 dependencies:  base64 · log · percent-encoding · ureq-proto · utf8-zero
grep -cE '^name = "(rustls|ring|native-tls|webpki-roots|rustls-platform-verifier|aws-lc-rs)"'  →  0
```

**after** (지금의 `src-tauri/Cargo.lock`):

```text
[[package]]
name = "ureq"
version = "3.4.0"
dependencies = [
 "base64 0.23.1",
 "log",
 "percent-encoding",
 "rustls",              ← 들어왔다
 "rustls-pki-types",    ← 들어왔다
 "ureq-proto",
 "utf8-zero",
 "webpki-roots",        ← 들어왔다
]
```

```text
ring            0.17.14     (암호 구현)
rustls          0.23.43     (TLS 구현)
rustls-pki-types 1.15.1
rustls-webpki   0.103.15
webpki-roots    1.0.9       (번들된 루트 인증서)
```

**루트 인증서의 출처는 `webpki-roots`, 즉 번들된 루트 집합이다** — OS 신뢰 저장소를 쓰는 구성이
아니다. ADR-0009 §11.3이 "둘 중 무엇을 쓰는지 구현 Task가 evidence에 적는다"고 요구한 항목이
이것이다. 그 대가는 알려져 있다: 사용자가 자기 기기에 직접 넣은 루트(사내 프록시 등)는
이 경로에서 신뢰되지 않는다. 그런 상황이 실제로 보고되면 바뀌는 것은 `Cargo.toml` 한 줄이다.

**그리고 lock만 바뀐 것이 아니라 실제로 빌드됐다** — `cargo test`가 만든 산출물이 그 사실을 보인다:

```text
src-tauri/target/debug/deps/
  libring-100d17aca8706583.rlib
  librustls-3bc0dcb4fcfcd147.rlib
  librustls_pki_types-29d3af5910c32a3a.rlib
  libwebpki_roots-2444d7e7da375c24.rlib
  libureq-cc300b5572a5a8d7.rlib
```

**판정: 켜졌다.** §11.4가 "feature를 켰다고 적어 두고 lock이 그대로면 켜지지 않은 것이다"라고
적은 바로 그 검사를 통과했다.

### 2.4 (4) 빌드 exit code

```text
lint  (eslint . && cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings)   exit 0
test  (vitest run && cargo test --manifest-path src-tauri/Cargo.toml)                                 exit 0
build (tsc && vite build)                                                                             exit 0
```

기록: `.loop/evidence/TASK-047/self-check.txt` · `.loop-local/self-check/gates/*/`.

⚠️ **`build` Gate는 frontend 빌드다** (`tsc && vite build`). Rust를 컴파일하는 것은 `lint`와
`test` Gate이며, 위의 TLS 판정은 그 둘이 뒷받침한다.

---

## 3. 확인하지 못한 것 — 여전히 [E4] UNVERIFIED

| 사실 | 왜 확인하지 못했는가 |
| --- | --- |
| `ureq` 3.4.0이 제공하는 **feature 전체 목록** | registry source 접근이 막혔고 (§1) 네트워크 조회 수단이 없다. 확인한 것은 "`rustls`가 실재한다"이지 "이것이 유일하거나 최선이다"가 아니다 |
| OS 신뢰 저장소를 쓰는 feature(예: platform verifier 계열)의 이름과 존재 | 같은 이유. 1순위가 성립했으므로 시험하지 않았다 |
| **실제 `https://api.notion.com`으로 TLS 핸드셰이크가 서는가** | 자동 테스트가 실제 Notion에 요청하지 않는다는 것이 이 Phase의 규칙이다 (PRODUCT-SPEC §18 · `phase-prompt/05`). 이 Task는 그것을 확인했다고 말하지 않는다 — **Phase Goal의 Human Review 항목**이다 |
| `ureq` 3.4.0의 MSRV | TASK-036 이후 그대로 미확인 |

---

## 4. 이 결정이 틀렸을 때 무너지는 범위

```text
바뀌는 것:     src-tauri/Cargo.toml 의 한 줄
바뀌지 않는 것: notion/http.rs 의 경계 · wire · client · testing · 모든 테스트
               (notion/network.rs 도 TLS 설정을 만지지 않으므로 그대로다)
```

TLS 구성이 틀렸다는 것은 **런타임에 https 요청이 실패하는 모습**으로 드러나며, 그 실패는
`TransportError::NotConnected` · `Incomplete`를 지나 §13의 `notionRequestFailed`가 된다 —
조용히 성공한 것처럼 보이는 경로는 없다.
