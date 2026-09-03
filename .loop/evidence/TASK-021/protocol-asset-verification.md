# 재생 경로가 설치된 Tauri에서 실제로 지원되는지 확인한 방법

이 Run에는 **네트워크 접근이 없었다** (WebFetch · WebSearch 모두 거부됨). 그래서 근거는
전부 **이 저장소에 설치된 패키지와 실제 빌드 출력**이다. 문서에서 읽었다고 적은 것이 아니라
여기서 실행해 본 것만 적는다.

## 1. 설치된 버전

```text
src-tauri/Cargo.lock:3347   name = "tauri"
src-tauri/Cargo.lock:3348   version = "2.11.5"

node_modules/@tauri-apps/api/package.json:3   "version": "2.11.1"
```

## 2. `protocol-asset` feature가 이 버전에 존재한다

`src-tauri/Cargo.toml`을 `features = ["protocol-asset"]`로 바꾼 뒤 lint Gate를 실행했다.
없는 feature였다면 cargo가 여기서 멈춘다.

```text
$ node tools/loop-runtime/loopctl.mjs self-check lint
[lint] npm run lint    →  eslint . && cargo clippy --all-targets -- -D warnings

    Updating crates.io index
     Locking 1 package to latest compatible version
      Adding http-range v0.1.5
 Downloading crates ...
  Downloaded http-range v0.1.5
   Compiling tauri v2.11.5
   Compiling molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)
    Checking http-range v0.1.5
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.32s

lint: PASS  exit=0  5.6s
```

feature를 켜자 **`http-range`가 의존성으로 추가됐다**. asset protocol이 Range 요청을
처리하는 데 쓰는 crate이며, 이것이 "파일을 통째로 메모리에 올리지 않고 재생·탐색한다"는
근거다. `src-tauri/Cargo.lock`의 diff에도 그대로 남아 있다 (`backend-diff.patch`).

`cargo test`도 같은 조합을 **check가 아니라 실제 컴파일로** 통과했다.

```text
   Compiling tauri v2.11.5
   Compiling http-range v0.1.5
   Compiling molt-note v0.1.0 (/Users/molt/orca/projects/molt-note/src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.75s
```

## 3. `asset_protocol_scope()` · `allow_directory()`가 이 버전에 존재한다

`src-tauri/src/lib.rs`가 부르는 것은 상상한 API가 아니라 아래 두 개다.

```rust
let _ = app
    .asset_protocol_scope()
    .allow_directory(&recordings_dir, false);
```

이 호출을 포함한 채 `cargo clippy --all-targets -- -D warnings`가 exit 0으로 끝났다
(`gate-results.txt`). 이름이나 시그니처가 달랐다면 컴파일에서 실패한다.

## 4. 설정 키가 이 버전의 스키마에 있다

`src-tauri/tauri.conf.json`에 아래를 넣은 뒤에도 빌드가 통과했다. `tauri-build`(build.rs)가
이 설정을 읽는다.

```json
"security": {
  "csp": null,
  "assetProtocol": { "enable": true, "scope": [] }
}
```

설치된 API의 타입 정의도 같은 두 가지를 요구한다 —
`node_modules/@tauri-apps/api/core.d.ts:130-134`:

```text
Note that `asset:` and `http://asset.localhost` must be added to `app.security.csp` ...
Additionally, `"enable" : "true"` must be added to `app.security.assetProtocol`
and its access scope must be defined on the `scope` array on the same `assetProtocol` object.
```

`convertFileSrc`는 `core.d.ts:158`에 선언되어 있다. **CSP는 손대지 않았다** — 이 앱의
`app.security.csp`는 `null`이고, 위 문서는 CSP를 설정했을 때의 요구사항이다.

## 5. capability에는 손댈 것이 없었다

생성된 ACL에 asset 관련 permission 식별자가 없다.

```text
$ grep -o '"[a-zA-Z:_-]*asset[a-zA-Z:_-]*"' src-tauri/gen/schemas/desktop-schema.json | sort -u
(출력 없음)

$ grep -o '"[a-zA-Z:_-]*asset[a-zA-Z:_-]*"' src-tauri/gen/schemas/acl-manifests.json | sort -u
(출력 없음)

$ grep -o '"core:asset-protocol[^"]*"' src-tauri/gen/schemas/desktop-schema.json
(출력 없음)
```

그래서 `src-tauri/capabilities/default.json`은 바꾸지 않았다 (`core:default` 그대로).
접근 범위를 정하는 것은 permission이 아니라 scope다.

## 6. 확인하지 못한 것

- **실제로 소리가 나는가 · 음질** — 이 Run은 앱을 띄우지 않았다. Human Review 항목이다.
- **`scope` 문자열의 경로 변수(`$APPDATA` 등)가 어떻게 확장되는가** — 확인할 수단이 없었다.
  그래서 설정 glob에 접근 범위를 맡기지 않고, 파일을 쓰는 코드와 같은 경로 값
  (`AppDataDirectory::recordings_dir`)을 코드에서 허용하는 쪽을 골랐다.
- 위 두 가지는 `docs/ADR-0006-audio-playback.md` §4에 UNVERIFIED로 적혀 있다.
