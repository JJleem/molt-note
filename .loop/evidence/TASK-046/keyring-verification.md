# TASK-046 — `keyring` crate 확인 기록 (ADR-0009 §10.3)

ADR-0009 §10.3은 세 가지를 **UNVERIFIED**로 남기고, 구현 Task가 그것을 확인해 증거를
남기라고 적었다. 이 문서가 그 기록이다.

> **이 파일에는 실제 자격증명이 하나도 없다.** 이 Run은 Notion token을 만들지도, 읽지도,
> 어떤 자격증명 저장소에 쓰지도 않았다 (ADR-0009 §10.5 · `phase-prompt/05` Important Rules).

---

## 1. 확인 방법과 그 한계

ADR-0009 §3.1이 기록한 제약이 이 Run에도 그대로 있었다.

```text
find ~/.cargo/registry/src -maxdepth 2 -name "keyring-3.6.3" -type d
  → blocked: "For security, Claude Code may only search files in the allowed
     working directories for this session: '/Users/molt/orca/projects/molt-note'"
```

WebFetch · WebSearch · 임의 셸 명령도 이 Run에서 쓸 수 없었다. 실행할 수 있는 명령은
`node tools/loop-runtime/loopctl.mjs self-check [<gate> ...]` 하나뿐이었다.

**따라서 `keyring`의 전체 feature 목록을 문서에서 읽어 확인하지 못했다 — 그 목록은 여전히
[E4] UNVERIFIED다.** 대신 ADR-0009 §10.3이 "핵심 판정"이라고 지목한 것(3번)을 그대로 했다:
**켰다고 믿지 않고, `Cargo.lock`이 실제로 무엇을 들여왔는지로 판정했다.**

---

## 2. 확인된 것 — [E1] 이 Run이 저장소에서 직접 확인했다

### 2.1 pin된 버전과 feature 이름

`src-tauri/Cargo.toml`에 적은 한 줄:

```toml
keyring = { version = "3", default-features = false, features = ["apple-native", "windows-native"] }
```

`cargo`가 이 요구를 **해석에 성공했다.** 존재하지 않는 feature 이름은 cargo가 하드 에러로
거절하므로, 이것은 **`apple-native`와 `windows-native`가 pin된 버전에 실재한다**는 관찰이다.

```text
해석된 버전:   keyring 3.6.3
checksum:      eebcc3aff044e5944a8fbaf69eb277d11986064cba30c468730e8b9909fb551c
```

`default-features = false`를 유지한 이유는 `ureq`와 같다 — 필요한 것만 이름으로 켠다.

### 2.2 ★ 핵심 판정 — `Cargo.lock`이 실제로 플랫폼 자격증명 API를 들여왔는가

ADR-0009 §10.3-4가 적은 대체 경로의 발동 조건은 "켰는데도 `Cargo.lock`에 그 경로가
나타나지 않으면"이었다. 나타났다.

**before** (`git show HEAD:src-tauri/Cargo.lock`) — ADR-0009 §3.2의 관찰 그대로:

```text
grep -cE '^name = "(security-framework|security-framework-sys|keyring)"'  →  0
```

**after** (지금의 `src-tauri/Cargo.lock`):

```text
[[package]]
name = "keyring"
version = "3.6.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "eebcc3aff044e5944a8fbaf69eb277d11986064cba30c468730e8b9909fb551c"
dependencies = [
 "byteorder",
 "log",
 "security-framework 2.11.1",
 "security-framework 3.7.0",
 "windows-sys 0.60.2",
 "zeroize",
]
```

```text
security-framework      2.11.1 · 3.7.0    (macOS Security framework — Keychain)
security-framework-sys  2.17.0
windows-sys             0.60.2            (Windows Credential Manager 쪽 경로)
```

그리고 test Gate의 컴파일 로그가 그것들이 **실제로 빌드됐다**는 것을 보인다:

```text
Compiling core-foundation-sys v0.8.7
Compiling core-foundation v0.10.1
Compiling security-framework-sys v2.17.0
Compiling security-framework v3.7.0
Compiling keyring v3.6.3
```

**판정: mock으로 조용히 떨어지지 않았다.** ADR-0009 §10.3-3이 요구한 증거가 이것이다.
`.loop/evidence/TASK-046/self-check.txt`와 `.loop-local/self-check/gates/test/stderr.log`가
같은 사실을 보인다.

### 2.3 확인한 API 표면

아래는 `cargo clippy --all-targets -- -D warnings`가 exit 0으로 통과했다는 사실로만
뒷받침된다 — **문서를 읽어서 확인한 것이 아니다.**

```text
keyring::Entry::new(service, account) -> Result<Entry, keyring::Error>
Entry::get_password()                 -> Result<String, keyring::Error>
Entry::set_password(&str)             -> Result<(), keyring::Error>
Entry::delete_credential()            -> Result<(), keyring::Error>   (deprecated 경고 없음)
keyring::Error::{NoEntry, NoStorageAccess(_), BadEncoding(_), Ambiguous(_), ..}
```

`delete_password`가 아니라 `delete_credential`을 쓴 것은 clippy가 `-D warnings`로 도는데
deprecated 경고 없이 통과했기 때문이다.

---

## 3. 확인하지 못한 것 — 여전히 [E4] UNVERIFIED

| 사실 | 왜 확인하지 못했는가 |
| --- | --- |
| `keyring` 3.6.3이 제공하는 **feature 전체 목록** | registry source 접근이 막혔고 (§1) 네트워크 조회 수단이 없다. 이 Run이 확인한 것은 "이 두 이름이 실재한다"이지 "이 둘이 최선/유일이다"가 아니다 |
| `apple-native`가 여는 것이 정확히 어떤 Keychain 항목 종류인가 (generic password 등) | 소스를 읽지 못했다. **자동 테스트가 실제 Keychain을 실행하지 않으므로 이 Run은 그것을 관찰하지도 않았다** |
| **macOS에서 실제로 저장·조회·삭제가 동작하는가** | 자동 테스트가 실제 자격증명 저장소를 건드리지 않는다는 것이 ADR-0009 §10.2의 결정이다. 따라서 이 사실은 **Phase Goal의 Human Review 항목**이며, 이 Task는 그것을 통과했다고 말하지 않는다 |
| Windows Credential Manager 동작 | Phase 6 (ADR-0009 §10.2) |

**§10.3-4의 대체 경로(`security-framework` 직접 사용)는 발동하지 않았다** — 발동 조건이
충족되지 않았기 때문이다 (§2.2).

---

## 4. 이 결정이 틀렸을 때 무너지는 범위

ADR-0009 §10.3이 노린 성질이 그대로 유지된다.

```text
바뀌는 파일:  src-tauri/src/platform/secret_store.rs  하나
              (+ Cargo.toml 한 줄)
바뀌지 않는 것: SecretStore trait · SecretKey · Secret · 호출부 · 모든 테스트
```

`keyring`이 실제 Keychain을 열지 못한다는 것이 나중에 드러나면, 고치는 것은 이 파일 안의
`mod os` 하나다. 그것이 이 경계를 만든 이유다.
