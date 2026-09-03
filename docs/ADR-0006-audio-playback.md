# ADR-0006 — 저장된 녹음은 asset protocol로 재생하고, 열어 주는 자리는 녹음 디렉터리 하나다

```text
Status:   Accepted (실제 재생은 Human Review 대상)
Date:     2026-09-02
Phase:    Phase 2 — Reliable Recording
Task:     TASK-021
Scope:    Recording Detail의 재생 경로 · 파일 접근 범위 · 레코드는 있는데 파일이 없는 상태
```

---

## 1. Context

`phase-prompt/02-reliable-recording.md` 요구사항 7은 **Recording Detail에서 저장된 audio를
재생할 수 있을 것**을 요구한다. 그때까지 이 앱에서 오디오 파일을 아는 것은 Rust뿐이었고,
webview는 파일의 경로 문자열만 알 수 있었다 (PRODUCT-SPEC §12 · ADR-0001).

재생을 붙인다는 것은 **파일 내용이 webview에 도달하는 통로를 처음으로 여는 일**이다. 그래서
결정해야 하는 것이 둘이다 — 어떤 통로로 흐르게 할 것인가, 그리고 그 통로가 무엇까지 열게
둘 것인가.

## 2. Decision

1. 재생은 **Tauri v2의 asset protocol**로 한다 (`protocol-asset` feature).
   파일 바이트는 IPC를 지나지 않는다.
2. **설정만으로는 아무 자리도 열지 않는다.** `tauri.conf.json`의
   `app.security.assetProtocol`은 `enable: true` · `scope: []`이다.
3. 실제로 열리는 자리는 **녹음 디렉터리 하나**이며, 앱 시작 시 코드가 명시적으로 허용한다
   (`src-tauri/src/lib.rs`의 `asset_protocol_scope().allow_directory(&recordings_dir, false)`).
   경로는 파일을 쓰는 쪽과 같은 자리에서 온다 (`AppDataDirectory::recordings_dir` · INV-10).
   하위 디렉터리는 열지 않는다 — 녹음 파일은 그 디렉터리에 바로 놓인다.
4. 경로를 재생 주소로 바꾸는 자리는 **ipc 모듈 하나**다
   (`src/ipc/commands.ts`의 `recordingAudioSource`). 화면은 그 함수를 받아 쓴다.
5. **레코드는 있는데 파일이 없는 상태**는 기존 감지 수단(`list_missing_audio`)으로 판정하고
   화면의 독립된 상태로 보여준다. 새 command를 만들지 않았다 — command 표면은 그대로
   열세 개다.
6. 이 통로는 로컬 webview로만 향한다. **원본 audio가 기기 밖으로 나가는 경로는 만들지
   않았다** (PRODUCT-SPEC §12 · INV-6).

`tests/audio-boundary.test.ts`가 2 · 3 · 4 · 6을 소스와 설정에서 직접 검사한다.

## 3. 고려한 대안

| 대안 | 왜 고르지 않았나 |
| --- | --- |
| 파일 바이트를 돌려주는 command (`Vec<u8>` → Blob URL) | 한 시간짜리 녹음을 통째로 메모리에 올려야 한다. 캡처 형식은 장치가 정하므로(`CaptureFormat`) 48 kHz · 2ch · 16-bit이면 1시간이 약 690 MB다. Range 요청도 없어 탐색이 통째 로드에 묶인다. |
| `@tauri-apps/plugin-fs`로 파일을 읽는다 | 재생 하나를 위해 범용 파일 접근 표면을 새로 여는 것이다. 필요한 것은 "녹음 디렉터리의 파일을 webview가 읽는 것"뿐이다. |
| `assetProtocol.scope`에 glob(`$APPDATA/recordings/*`)을 적는다 | 설정 문자열이 실제로 어떤 자리로 확장되는지 이 Run에서 확인할 수 없었다(§4). 확인하지 못한 문자열에 접근 범위를 맡기는 대신, 파일을 쓰는 코드와 같은 경로 값을 쓰는 쪽을 골랐다. |

## 4. 확인한 것과 확인하지 못한 것 (VERIFIED / UNVERIFIED)

이 Run에서 실제로 확인할 수 있었던 범위를 그대로 적는다. **추측한 것을 확인한 것처럼 적지
않는다.** 이 Run에는 네트워크 접근이 없었으므로 근거는 전부 **설치된 패키지와 실제 빌드**다.

| 항목 | 상태 | 근거 |
| --- | --- | --- |
| 설치된 Tauri는 `2.11.5`, `@tauri-apps/api`는 `2.11.1` | **VERIFIED** | `src-tauri/Cargo.lock` · `node_modules/@tauri-apps/api/package.json` |
| 이 버전에 `protocol-asset` feature가 존재한다 | **VERIFIED** | feature를 켠 뒤 `cargo clippy --all-targets -- -D warnings`가 exit 0. 의존성 해석이 `http-range v0.1.5`를 추가했다 — asset protocol의 Range 처리에 쓰이는 crate다 (`.loop/evidence/TASK-021/protocol-asset-verification.md` §2) |
| `Manager::asset_protocol_scope()`와 `Scope::allow_directory(path, recursive)`가 이 버전에 있다 | **VERIFIED** | 위 빌드가 그 호출을 포함한 채 통과했다 |
| `app.security.assetProtocol`의 `enable` · `scope`가 이 버전의 설정 스키마에 있다 | **VERIFIED** | `tauri-build`(build.rs)가 그 설정을 담은 `tauri.conf.json`을 읽고 빌드가 통과했다 · `node_modules/@tauri-apps/api/core.d.ts`의 `convertFileSrc` 문서 |
| asset protocol에 별도의 capability permission이 필요하지 않다 | **VERIFIED (이 버전에 한해)** | 생성된 ACL(`src-tauri/gen/schemas/acl-manifests.json` · `desktop-schema.json`)에 `asset` 관련 permission 식별자가 없다. 접근 범위를 정하는 것은 scope다. capability 파일은 바꾸지 않았다 |
| CSP를 추가로 손볼 필요가 없다 | **VERIFIED** | `core.d.ts`는 **CSP를 설정했을 때** `asset:` · `http://asset.localhost`를 넣으라고 말한다. 이 앱의 `app.security.csp`는 `null`이다 |
| `convertFileSrc`가 설치된 API에 존재한다 | **VERIFIED** | `node_modules/@tauri-apps/api/core.d.ts:158` |
| **실제로 소리가 나는가 · 음질이 사람이 알아들을 수준인가** | **UNVERIFIED — Human Review** | 이 Run은 앱을 띄우지 않았다. 자동 테스트는 상태 전이만 판정한다 (`phase-prompt/02-reliable-recording.md`의 Human Review) |
| 앱을 껐다 켠 뒤에도 목록의 녹음이 재생되는가 | **UNVERIFIED — Human Review** | 위와 같다. Verification Boundary가 사람 확인 항목으로 두고 있다 |
| `scope`의 `$APPDATA` 같은 경로 변수가 이 버전에서 어떻게 확장되는가 | **UNVERIFIED** | 확인할 수단이 없었다. 그래서 §3처럼 설정 glob 대신 코드가 실제 경로를 허용하는 쪽을 골랐다 |

## 5. 파일이 없는 상태를 재생 실패로 만들지 않는다 (INV-3 · INV-4)

레코드는 있는데 오디오 파일이 없는 상태는 **조회 실패와 다른 사실**이다. 저장소는 정상적으로
답했고 레코드도 온전하다. 그래서 화면은 그것을 독립된 상태로 보여주고, 그 상태에서도 제목 ·
날짜 · 길이 · **레코드가 가리키던 경로**를 그대로 유지한다 — 지우지 않는 것은 사용자가 찾을
수 있어야 한다 (INV-1).

이 경로에는 레코드를 지우거나 고치는 동작이 없다. 파일을 다시 만드는 동작도 없다. 감지는
`list_missing_audio`가 하고, 그 command 역시 알리기만 한다
(`docs/ADR-0004-recording-session-lifecycle.md` §12 · R-004).

`src/screens/recordingDetailView.ts`는 이 판정을 DOM 없이 지나는 순수 모듈이며, 네 상태
(로딩 · 재생 가능 · 파일 없음 · 조회 실패)와 "그런 녹음이 없다"까지
`src/screens/recordingDetailView.test.ts`가 판정한다 (§18).

## 6. 결과

```text
Recording Detail
  ├─ get_recording ─────────→ 이 녹음이 무엇인가
  ├─ list_missing_audio ────→ 이 녹음의 파일이 지금 그 자리에 있는가
  └─ recordingAudioSource ──→ asset://…  (녹음 디렉터리 안의 파일만 열린다)
```

command 표면은 늘지 않았다. 늘어난 것은 **읽기 전용 통로 하나**이며, 그 통로가 볼 수 있는
자리는 이 앱이 녹음을 쓰는 디렉터리 하나다.
