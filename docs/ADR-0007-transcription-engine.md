# ADR-0007 — 전사 엔진은 앱 안으로 링크하고, 밖에서 확보하는 것은 모델 하나뿐이다

```text
Status:   Accepted · 구현됨 (§16) — 실제 추론과 notarization은 여전히 미확인 (§14)
Date:     2026-09-03 (결정) · 2026-09-03 갱신 (구현 결과 반영 — §16)
Phase:    Phase 3 — Local Transcription
Task:     TASK-023 (결정) · TASK-031 (구현 결과 반영)
Scope:    whisper 통합 방식 · 엔진/바이너리 확보 경로 · 모델 관리 · 입력 포맷 변환 책임 ·
          timestamp 정규화 경계 · Windows 성립 여부
```

> **§1~§15는 결정 시점(구현 전)의 문서다.** 구현이 그 결정을 어떻게 실현했는지, 무엇이
> 계획과 달라졌는지, 무엇이 아직 UNVERIFIED로 남았는지는 **§16**에 있다.
> 운영자 smoke test 절차와 Phase 3 검증 기록표는
> `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`에 있다.

---

## 1. Context

Phase 3는 Phase 2가 만든 녹음 파일을 **기기 안에서** 전사해 timestamp가 있는 Transcript로
저장해야 한다 (`phase-prompt/03-local-transcription.md` · PRODUCT-SPEC §8 · §12).

그 전에 되돌리기 어려운 선택이 하나 있다. **whisper.cpp를 이 앱에 어떻게 들여올 것인가.**

```text
A. Tauri sidecar + whisper-cli      별도 실행 파일을 번들에 넣고 프로세스로 부른다
B. whisper-rs (Rust 바인딩)          엔진을 앱 바이너리 안으로 링크한다
C. 사용자 설치 바이너리 참조          사용자 기기에 이미 있는 whisper를 찾아 쓴다
```

이 선택이 결정하는 것은 통합 코드만이 아니다. **무엇을 배포물에 넣는가 · 저장소가 무엇을
재현할 수 있는가 · 사용자가 무엇을 직접 확보해야 하는가 · timestamp를 어떤 단위로 받는가**가
전부 여기에 걸려 있다. Phase 3의 나머지 Task(TASK-024~031)는 이 문서의 결정을 전제로 한다.

선택은 PRODUCT-SPEC §14.4.2의 운영자 정책 아래에서 한다.

```text
Molt Note 사용자는 전사를 쓰기 위해
whisper.cpp · Homebrew · CMake · Whisper CLI를 직접 설치하도록 요구받지 않는다.
```

---

## 2. Decision

1. **통합 방식은 B — `whisper-rs`다.** 엔진은 `cargo build`의 산출물 안으로 들어가며,
   배포물에 추가로 넣는 실행 파일은 없다. `bundle.externalBin`을 쓰지 않는다.
2. **저장소 밖에서 확보해야 하는 것은 모델 파일 하나뿐이다.** 바이너리도, 사용자 기기의
   whisper 설치도 필요하지 않다.
3. **모델은 앱에 번들하지 않고 자동으로 내려받지도 않는다.** V1은 **사용자가 지정한 모델
   파일 경로**를 설정에 저장한다. 기본 탐색 위치는 앱 데이터 디렉터리 아래 `models/`이며,
   그 경로는 이미 있는 `AppDataDirectory` 경계에서 온다 (INV-10). 자동 다운로드는 DEFERRED다 (§8).
4. **모델이 없는 상태는 오류 로그도 아니고 설정의 조용한 변경도 아니다.** §13의 제품 실패
   `모델 파일 없음`으로 표현하고, 자동 전사 토글 값을 앱이 임의로 뒤집지 않는다.
5. **입력 변환은 앱이 순수 Rust로 한다** — `hound`(읽기) + `rubato`(리샘플) + 수동 다운믹스.
   사용자에게 ffmpeg을 요구하지 않는다. 변환 결과는 **메모리 위의 16 kHz mono `f32` 버퍼**이며
   **원본 파일은 읽기 전용으로만 연다** (INV-1 · INV-3).
6. **timestamp 정규화는 코드의 한 자리에서만 한다** — `whisper-rs`가 주는 **센티초**를
   `start_ms` · `end_ms`로 바꾸는 곳은 파싱/정규화 모듈 하나다 (TASK-025). 실행 경계도,
   영속성도, 화면도 단위 변환을 하지 않는다.
7. **`SidecarResolver`를 만들지 않는다.** B에는 해석할 sidecar 경로가 없다. 이 Phase에서
   플랫폼이 실제로 갈리는 지점은 **모델 파일 위치** 하나이며 그것은 이미 있는 경계가 처리한다
   (PRODUCT-SPEC §3.1 · §20.6 — 추상화를 선입금하지 않는다).
8. **모델과 큰 바이너리는 저장소에 커밋하지 않는다.** 이미 있는 `.gitignore` 규칙이 이것을
   강제한다 (§8.3).
9. **C(사용자 설치 바이너리)는 V1 배포 경로가 아니다.** 개발/디버그 목적의 비교 수단으로만
   남으며, 그 경우에도 제품 코드 경로가 되지 않는다 (§12.2).

---

## 3. 근거의 종류 — 이 Run이 확인할 수 있었던 범위

**추측한 것을 확인한 것처럼 적지 않는다** (PRODUCT-SPEC §20.2). 아래 표기를 문서 전체에서 쓴다.

| 표기 | 뜻 |
| --- | --- |
| **[E1] 직접 확인** | 이 Run에서 저장소의 실제 파일을 읽어 확인했다 |
| **[E2] §14.4.1 재확인** | PRODUCT-SPEC §14.4.1이 **2026-09-03에 primary source에서 재확인**한 값. 오늘이 곧 도입 시점이다 |
| **[E3] §14.4 (2026-09-01)** | 2026-09-01 기록. §14.4.1과 어긋나면 §14.4.1이 우선한다 |
| **[E4] UNVERIFIED** | 확인하지 못했다. 구현 근거로 쓰지 않는다 |

> ⚠️ **이 Run에는 네트워크 접근이 없었다.** 외부 URL을 가져오는 시도는 거부됐다
> (`.loop/evidence/TASK-023/verification-log.md`). 따라서 **upstream 릴리스 · crates.io ·
> GitHub 이슈에 관한 어떤 항목도 [E1]로 올리지 않았다.** 그 항목들의 근거는 [E2]이며,
> [E2]의 확인 시점은 **2026-09-03 — 이 결정을 내리는 날과 같은 날**이다.
> 도입 시점 재확인 요구는 그 사실로 충족되지, 이 Run이 다시 확인해서 충족되지 않는다.

---

## 4. 후보 비교 — §14.4.2의 세 제약 아래에서

### 4.0 선택 근거로 쓰지 않은 것 (§14.4.2가 금지한 세 가지)

| 쓰지 않은 근거 | 왜 |
| --- | --- |
| **cmake의 유무** | **A와 B 양쪽 모두 CMake를 요구한다** [E2 · §14.4.1 "CMake는 두 후보 모두에 필요하다"]. 둘을 가르지 못한다. 개발 Mac에는 2026-09-03에 cmake 4.4.3이 설치됐고 [E2 · §14.1], 그것은 **개발 빌드 의존성이지 사용자 의존성이 아니다** |
| **"언어가 Rust라서"** | 통합 언어의 동질성은 이 문서의 어떤 표에도 가중치로 들어가지 않았다. B는 아래 §4.2의 **확보 경로 · 미확인 링크 수**로 선택됐고, 그 대가(§4.3)를 함께 기록했다 |
| **"cmake가 없어서 C"** | C는 §12.2대로 **A·B가 둘 다 실증된 blocker를 만났을 때만** 선택할 수 있다. 그런 blocker는 관측되지 않았다 — A의 문제도 B의 문제도 전부 **감수 가능한 위험이거나 미확인 항목**이지 실증된 blocker가 아니다 |

### 4.1 두 후보가 실제로 요구하는 것

| | **A. sidecar + `whisper-cli`** | **B. `whisper-rs`** |
| --- | --- | --- |
| 엔진 버전 | whisper.cpp v1.9.3 (태그 `b4938`, 2026-08-20) [E2] | 0.16.0 (2026-03-12), 번들 whisper.cpp **v1.8.3 — upstream보다 낮다** [E2] |
| 유지보수 위치 | ggml-org/whisper.cpp (GitHub) [E2] | GitHub 저장소는 2025-07-30 archived, 현재 **Codeberg** [E2] |
| CMake | 필요 (소스 빌드) [E2] | 필요 (macOS·Windows 양쪽) [E2] |
| **배포물에 넣는 것** | `whisper-cli` 실행 파일 + target triple 접미사 파일명 [E2 · §14.4] | **없다** — 앱 바이너리 하나 |
| 오디오 입력 | 16-bit WAV **파일**. 내부 리샘플링 없음 [E2] | `full(params, &[f32])` — **f32 PCM**. 리샘플링·다운믹스 없음 [E2] |
| timestamp 단위 | JSON `offsets` = **밀리초** [E2] | segment timestamp = **센티초** [E2] |
| Apple Silicon | Metal 기본 ON [E2] | `metal` feature flag [E2] |
| 프로세스 격리 | **있다** — 자식 프로세스가 죽어도 앱은 산다 | **없다** — 엔진 abort가 앱을 함께 죽인다 |

### 4.2 결정적 차이 — 저장소에서 동작하는 앱까지의 **미확인 링크 수**

두 후보 모두 §14.4.2의 사용자 설치 요구 금지를 **원리상** 만족한다. 갈리는 것은
**무엇을 근거로 그렇게 말할 수 있는가**다.

**A를 택하면 앱이 동작하기까지 다음이 전부 성립해야 한다.**

```text
1. 개발 Mac에서 whisper.cpp를 소스 빌드해 whisper-cli를 얻는다      [가능 · 절차 있음]
2. 그 산출물을 target triple 파일명으로 src-tauri/binaries/에 둔다   [규약 확인됨 · E2]
3. 그 파일은 저장소에 커밋하지 않는다                                 [정책 · §8.3]
4. externalBin이 그 파일을 번들에 넣고 실제로 실행된다               [문서상 확인 · E2]
5. 그 바이너리가 단독으로 실행 가능하다                               [E4 — 확인 못 함]
6. externalBin이 있는 앱이 codesign/notarize를 통과한다              [E4 — #11992 관측]
```

여기서 **5와 6이 [E4]다.**

- **5** — whisper.cpp 기본 빌드가 `whisper-cli` 하나로 완결되는지, 아니면 함께 만들어진
  공유 라이브러리(`libwhisper` · `libggml*`)를 옆에 요구하는지 이 Run에서 확인하지 못했다.
  **이것은 미확인 가설이며 A의 탈락 근거로 세지 않았다.** 다만 A를 택하면 **확인해야 할
  항목이 하나 더 생긴다**는 사실 자체는 확인된 것이다 — 확인 전에는 6을 시도할 수도 없다.
- **6** — §5의 tauri#11992. **배포 불가의 증거가 아니다.** 그러나 A만 짊어지는 위험이다.

그리고 **3의 귀결이 A의 진짜 비용**이다. 바이너리를 커밋하지 않으면
**저장소를 새로 clone해서 만든 앱에는 엔진이 없다.** 어떤 플래그로(Metal on/off ·
정적/동적 링크 · 어느 커밋에서) 빌드한 실행 파일이 배포물에 들어갔는지를 **저장소가
재현하지 못한다.** Gate도 그 파일 없이 도는 수밖에 없으므로, **자동 검증은 실제 엔진
경로를 영원히 덮지 못한다.**

**B를 택하면 같은 사슬이 이렇게 된다.**

```text
1. cargo build가 엔진을 함께 빌드해 앱 바이너리에 링크한다          [E2 — CMake 요구 확인됨]
2. 배포물은 앱 하나다. externalBin도, 파일명 규약도, 존재 여부 분기도 없다
```

**5·6이 사라진다.** 엔진 버전은 `Cargo.toml`의 핀 하나로 저장소가 재현하고, Gate는
사람이 파일을 옮겨 두지 않아도 실제 엔진을 링크한 채로 돈다.

**PRODUCT-SPEC §14.4.3이 이 Phase에 요구하는 실제 추론 smoke test도 같은 방향을 가리킨다.**
A에서는 운영자가 *소스 빌드 → 파일명 변경 → 배치* 를 먼저 통과해야 추론에 도달한다.
B에서는 **모델 파일 하나만 두면 된다.** 이것은 취향이 아니라 절차 단계 수의 차이다.

### 4.3 B가 지불하는 대가 — 감추지 않는다

| 대가 | 사실 | 어떻게 감당하는가 |
| --- | --- | --- |
| **프로세스 격리 상실** | ggml의 assert/abort는 in-process에서 **앱 전체를 죽인다** | 격리로 지키려던 것은 원본 데이터인데, **그것은 이미 다른 수단이 지킨다** — 원본 audio는 읽기 전용으로만 열고 파생 입력은 메모리에만 있다 (INV-1 · INV-3 · §9). 앱이 죽어도 audio와 Recording 레코드는 그대로다. 처리 중 앱이 죽는 경우는 §13의 `application restart during processing`이 이미 다루는 제품 상태이며, `running`에서 재시작한 전사는 재시도 가능해야 한다 (TASK-027) |
| **엔진 버전 지연** | 번들 whisper.cpp가 **v1.8.3**, upstream은 v1.9.3 [E2] | 우리가 쓰는 것은 모델 로드 · `full()` · segment timestamp · Metal이다. **v1.8.3에 그 표면이 있는지는 [E4]** — `whisper-rs`를 실제로 추가하는 TASK-026이 빌드로 확인한다. 확인 결과가 다르면 이 ADR을 갱신한다 (TASK-031) |
| **유지보수 위치 이동** | GitHub archived (2025-07-30) → Codeberg [E2] | 방치 위험은 실재한다. 완화는 **버전 핀 + 교체 비용을 작게 유지**하는 것이다 (§13) |
| **Gate 비용 증가** | cold build가 whisper.cpp 컴파일을 포함한다 | lint · test Gate의 timeout은 900초다 [E1 · `.loop/project.yaml`]. **관측됨 (2026-09-03 · TASK-026): `whisper-rs` 추가 후 첫 lint Gate가 27.7초에 끝났다 — 한도를 넘지 않았다** [`.loop/evidence/TASK-026/whisper-rs-api-verification.md` §4]. 다만 그 관측은 이 기기의 cargo 캐시 상태·빈 `target/`·release 빌드에 대해서는 아무 말도 하지 않는다 (§16.4) |

### 4.4 선택

**B를 선택한다.** 근거는 §4.2의 두 가지다 — 저장소에서 동작하는 앱까지의 **미확인 링크가
더 적고**, 그중 어느 것도 사람이 손으로 옮긴 파일에 의존하지 않는다. 대가(§4.3)는
전부 기록했고, 그중 실증된 blocker는 없다.

---

## 5. 릴리스 아티팩트 — 종류를 뭉뚱그리지 않는다

**(b)가 있다고 (a)가 있는 것이 아니다.** 아래는 §14.4.1이 **2026-09-03에 릴리스 asset 9개
전부를 열거해 확인한 결과**다 [E2]. 태그 `b4938` (v1.9.3, 2026-08-20).

| 종류 | macOS | 근거 · 확인 시점 |
| --- | --- | --- |
| **(a) CLI 실행 파일 (`whisper-cli`)** | **없다** | [E2] 2026-09-03 · asset 전수 확인. Apple 대상 asset은 `whisper-b4938-xcframework.zip` 하나뿐이다 |
| **(b) XCFramework / 라이브러리** | **있다** (`whisper-b4938-xcframework.zip`) | [E2] 2026-09-03. **Swift/ObjC 임베딩용 라이브러리이며 Tauri sidecar로 바로 쓸 수 없다** |
| **(c) 소스 빌드** | CLI를 얻는 유일한 경로 | [E2] 2026-09-03 · `cmake -B build` → `cmake --build build -j --config Release`, 산출 위치 `./build/bin/whisper-cli` [E3] |
| **(d) Windows 아티팩트** | 있다 — `whisper-bin-Win32.zip` · `whisper-bin-x64.zip` · `whisper-blas-bin-*` · `whisper-cublas-11.8.0/12.4.0-bin-x64.zip` | [E2] 2026-09-03 |

**(d)의 내용물** — zip 안이 CLI exe인지 DLL만인지는 **[E4] UNVERIFIED다** [E2 · §14.4.1이
그렇게 기록했다]. **"Windows에는 prebuilt CLI가 있다"고 적지 않는다.** 확인된 것은
*"Windows 대상 아티팩트가 존재한다"* 까지다.

| 그 밖의 확인 항목 | 값 | 근거 |
| --- | --- | --- |
| `whisper-rs` 최신 버전 | 0.16.0 (2026-03-12), `whisper-rs-sys` 0.15.0 | [E2] 2026-09-03 |
| 번들 whisper.cpp 버전 | **v1.8.3** | [E2] 2026-09-03 |
| 유지보수 위치 | Codeberg `codeberg.org/tazz4843/whisper-rs`. GitHub 미러는 2025-07-30 archived | [E2] 2026-09-03 |
| `whisper-rs`의 정확한 API 표면 (타입·함수 이름) | `WhisperState::full(params, &[f32])` · `WhisperSegment::start_timestamp()` | [E2]가 기록한 형태다. **2026-09-03 TASK-026이 컴파일러로 확인했다 — segment 접근 경로가 기록과 다르다 (§16.2).** 확인된 시그니처는 `.loop/evidence/TASK-026/whisper-rs-api-verification.md` |
| 번들 whisper.cpp v1.8.3이 위 표면을 그대로 갖는가 | 빌드가 성공했다 | 우리가 쓰는 표면으로 **컴파일·링크된다**는 것은 확인됐다 (TASK-026). **번들 버전이 실제로 v1.8.3인지는 여전히 [E4]** — 읽는 경로를 확인하지 못했다 (§16.3) |

> 참고: 이 문서는 sidecar를 택하지 않았으므로 **CLI 플래그와 JSON 필드명을 제품 근거로
> 쓰지 않는다.** §14.4가 기록한 `-oj`/`--output-json`과 `transcription[].offsets{from,to}`는
> [E2/E3]로 남아 있으며, §13의 되돌리기 경로가 그것을 쓴다.

---

## 6. tauri-apps/tauri#11992 — 관찰된 packaging 위험

**관찰된 사실** [E2 · 2026-09-03]:

- 이슈 *"MacOS - Codesigning and notarization issue when using ExternalBin"* 는 **현재 OPEN이다.**
- `externalBin`을 설정하면 메인 앱 바이너리의 notarization이 `invalid signature`로 실패하고,
  sidecar를 빼면 성공한다는 **보고**가 있다 (macOS 15.0.1 arm64 / Tauri 2.1.1에서 재현).

**확정된 사실로 적지 않는 것:**

- ❌ "sidecar를 빼는 것 외에 우회가 없다" — 이것은 **적지 않는다.** 공식 수정이나 문서화된
  우회를 **찾지 못했다**는 것과, **존재하지 않는다**는 것은 다른 진술이다. 확인된 것은 전자다 [E2].
- ❌ "sidecar는 배포할 수 없다" — 근거가 없다. 보고는 특정 버전 조합(Tauri 2.1.1)에서의
  재현이며, 이 저장소의 Tauri는 **2.11.5**다 [E1 · `src-tauri/Cargo.toml` · ADR-0006 §4].
  그 버전에서 같은 증상이 나는지는 **[E4]**다.

**이 결정에서의 위치:** B는 `externalBin`을 쓰지 않으므로 **이 이슈가 제품 경로에 놓이지
않는다.** 그러나 그것이 A의 탈락 사유는 아니다 — A는 §4.2의 확보 경로로 탈락했고,
#11992는 **A를 택했을 때 추가로 짊어졌을 미확인 항목**으로 기록된다.

**최종 notarization 확인은 배포 검증 경계로 넘긴다.** 무엇을 배포하든(externalBin이 없어도)
서명·notarization이 실제로 통과하는지는 이 Phase가 판정하지 않는다. §3의 배포 범위는
App Store 밖이며, 배포 검증은 Phase 6 / 배포 준비의 몫이다.

---

## 7. 엔진 확보 경로 — 사용자는 무엇도 설치하지 않는다

```text
개발자 기기                                   사용자 기기
──────────                                   ──────────
cargo build
  └ whisper-rs-sys → cmake → whisper.cpp     설치할 것: 없다
      └ 앱 바이너리에 링크                     실행할 것: Molt Note 하나
                                             확보할 것: 모델 파일 하나 (§8)
```

| §14.4.2가 금지한 사용자 설치 | B에서 필요한가 | 근거 |
| --- | --- | --- |
| whisper.cpp | **아니다** | 엔진이 앱 바이너리 안에 있다 |
| Homebrew | **아니다** | 설치할 도구가 없다 |
| CMake | **아니다** | **빌드 시점**에 개발자 기기에서만 쓰인다 [E2 · §14.4.1] |
| Whisper CLI | **아니다** | CLI를 부르지 않는다 |
| (추가) ffmpeg | **아니다** | 변환이 순수 Rust다 (§9) |

**개발자 기기의 요구사항**은 cmake 4.4.3 + Apple clang 17.0.0이며 둘 다 이미 있다
[E2 · §14.1]. Windows 쪽 요구는 §11에 있다.

---

## 8. 모델 관리

### 8.1 세 가지 중에서

| 방식 | 판정 | 근거 |
| --- | --- | --- |
| **앱 번들에 포함** | ❌ | 모델은 `small` ≈466MiB · `medium` ≈1.5GiB · `large-v3` ≈2.9GiB다 [E2 · §14.4]. 앱 하나에 수 GB를 넣는 것도 문제지만, 더 근본적으로 **모델 선택은 사용자 설정이다** — 번들은 하나를 고정해 버린다. `phase-prompt/03`은 *"전사가 느리다는 이유로 정확도가 낮은 모델을 조용히 강제하지 않는다"* 고 못박는다 |
| **최초 실행 시 자동 다운로드** | ❌ (V1) · DEFERRED | 수 GB 다운로드는 그 자체로 제품 기능이다 — 진행률 · 중단/재개 · 무결성 확인 · 디스크 부족 · 네트워크 실패가 전부 새 상태다. Phase 3의 범위(§Out of Scope)를 넘고, **§12의 privacy 경계에 네트워크 경로를 하나 여는 일**이라 별도 결정이 필요하다. 오디오가 나가는 것은 아니지만 그 판단은 이 Task의 것이 아니다 |
| **사용자가 지정 / 앱이 아는 위치에 둔다** | ✅ **선택** | 코드가 늘지 않고, 모델 선택권이 사용자에게 남으며, 네트워크 경로를 열지 않는다. 운영자 smoke test(§14.4.3)에도 이 경로가 필요하다 |

### 8.2 규칙

1. 설정에 **모델 파일 경로(또는 앱 모델 디렉터리 안의 파일명)** 를 저장한다 (TASK-029).
   INV-7에 따라 secret 열은 만들지 않는다 — 이것은 경로일 뿐이다.
2. 기본 탐색 위치는 **앱 데이터 디렉터리 아래 `models/`** 이며, 그 경로는
   `AppDataDirectory` 경계에서 온다 (INV-10 · 플랫폼별 경로를 직접 조합하지 않는다 [E2 · §14.2]).
3. **모델이 없는 상태는 제품 상태다** (§13 `모델 파일 없음`).
   - 설정 화면은 *모델이 없어서 지금은 전사할 수 없다*는 사실과 **해결 방법**을 보여준다.
   - **앱이 `automatic_transcription` 토글을 조용히 뒤집지 않는다.** 사용자가 켠 값은 켜진 채로
     남고, 실행이 불가능하다는 사실은 별도 상태로 표현된다.
   - 모델이 없어서 실패한 전사는 **다른 실패와 구분되어** 보인다 (TASK-030).
4. Transcript에 기록하는 `model`은 **실제로 쓴 모델 파일의 식별자**이고, `engine`은
   `whisper-rs` 버전 + 번들 whisper.cpp 버전이다 (§7 provenance · TASK-027).
   **구현 결과는 `whisper-rs/0.16`뿐이다** — 번들 whisper.cpp 버전을 읽는 경로를 확인하지
   못했고, **모르는 값을 provenance로 지어내지 않았다** (§16.3).

### 8.3 저장소에 넣지 않는 것 — 이미 강제되고 있다

`.gitignore`가 다음을 이미 제외한다 [E1 · 이 Run에서 파일을 직접 읽었다]:

```text
/models/     *.gguf     *.bin        ← whisper ggml 모델 (수백 MB ~ 수 GB)
*.wav *.mp3 *.m4a ...                ← 오디오는 어디에 있든 제외
```

**수백 MB~수 GB 모델도, 큰 실행 파일도 커밋하지 않는다.** B는 배포물에 넣을 바이너리가
없으므로 새 규칙이 필요하지 않다. 나중에 어떤 이유로든 바이너리를 두는 디렉터리가 생기면
**그때 규칙을 추가한다** (TASK-026). 테스트용 오디오는 커밋하지 않고 **테스트가 임시
디렉터리에 합성 WAV를 만든다** (TASK-024).

---

## 9. 입력 포맷 변환 책임 — 그리고 원본을 건드리지 않는다는 규칙

### 9.1 무엇이 필요한가

```text
Phase 2의 raw recording        장치가 정한 sample rate / channels의 PCM16 WAV
                               (16-bit는 코드가 고정하고, 16kHz mono는 장치가 정한다)
                               [E1 · src-tauri/src/audio/capture.rs의 CaptureFormat · ADR-0003 §4.2.3]
                                        ↓  변환 책임은 앱에 있다
whisper-rs가 요구하는 입력      16 kHz mono f32 PCM — 리샘플링·다운믹스를 해 주지 않는다 [E2]
```

**sidecar와 다른 점을 분명히 한다.** `whisper-cli`였다면 요구는 *16-bit WAV 파일*이었고
파생 **파일**을 만들어야 했다 [E2]. **B가 요구하는 것은 파일이 아니라 `f32` 슬라이스**이므로
**파생 입력은 디스크에 내려갈 필요가 없다.**

### 9.2 수단 — 순수 Rust, 사용자에게 ffmpeg을 요구하지 않는다

| crate | 역할 | 상태 |
| --- | --- | --- |
| `hound` 3.x | WAV 읽기 | **이미 의존성이다** [E1 · `src-tauri/Cargo.toml`] — Phase 2가 쓰기용으로 넣었다 |
| `rubato` 5.0.0 (2026-08-10) | 샘플레이트 변환 | TASK-024가 추가했다. **5.0.0의 실제 타입은 `Fft` + `FixedSync` + `audioadapter` 버퍼이며, 흔히 인용되는 `FftFixedIn`/`SincFixedIn`은 이 버전에 없다** [빌드로 확인 · `.loop/evidence/TASK-024/verification-log.md` · §16.2] |
| 수동 다운믹스 | stereo → mono | 코드 몇 줄. 근거 없는 custom DSP를 직접 구현하지 않는다 |

**WAV 읽기와 리샘플링을 한 번에 하는 crate는 없다** [E2 · §14.4.1]. 위 조합은 **전부 순수
Rust이며 외부 도구가 필요 없다** [E2].

**결론: 사용자에게 ffmpeg 설치를 요구하지 않는다.** 개발 Mac에 ffmpeg 8.1.1이 있지만
[E2 · §14.1] **그것을 사용자 의존성으로 가정하지 않는다** — 제품 경로는 ffmpeg을 부르지 않는다.

입력이 이미 16 kHz mono이면 변환하지 않는다 (TASK-024).

### 9.3 원본은 덮어쓰지 않는다 (INV-1 · INV-3)

```text
raw recording        immutable · 보존 · 읽기 전용으로만 연다
derived 전사 입력     재생성 가능 · 메모리 위의 f32 버퍼 · 앱이 죽으면 그냥 사라진다
```

규칙:

1. 변환 모듈은 원본 파일을 **읽기만 한다.** 덮어쓰지 · 지우지 · 이름을 바꾸지 않는다.
2. 변환 결과를 **원본 경로에 쓰지 않는다.** 기본은 메모리 버퍼다. 어떤 이유로 파생 파일이
   필요해지면 **녹음 디렉터리가 아닌 파생/임시 경로**에 별도 파일로 만들고, 그 정리 실패가
   전사 성공을 되돌리지 않는다 (TASK-027).
3. **어떤 실패 경로도 원본 audio와 Recording 레코드를 건드리지 않는다** (INV-3). 손상된 WAV ·
   빈 파일 · 예상과 다른 채널 수는 panic이 아니라 §13의 제품 실패로 매핑된다
   (`src-tauri/src/domain/failure.rs`의 기존 `Failure` 계약 [E1]).
4. 전사는 **재시도 가능하다.** 원본이 그대로이므로 파생 입력은 언제든 다시 만들 수 있다.

---

## 10. timestamp — 실제 단위와, 정규화하는 단 한 곳

| | 값 | 근거 |
| --- | --- | --- |
| **선택한 방식(`whisper-rs`)이 내는 단위** | **센티초 (1/100초)** | [E2 · §14.4.1 — `WhisperSegment::start_timestamp()`] |
| (참고) sidecar였다면 | 밀리초 (JSON `offsets`, 내부 t0 센티초 × 10) | [E2] |
| 저장 스키마 | `transcript_segments.start_ms` · `end_ms` (INTEGER) | [E1 · `src-tauri/src/db/migrations.rs` migration 2] |

**변환은 `× 10`이다. 그러나 이 문서가 그렇게 적었다는 이유로 코드가 그렇게 하지 않는다** —
crate를 실제로 추가하는 TASK-026이 **실제 값으로 확인**하고, 다르면 이 ADR을 갱신한다
(§14의 [E4] 항목 · TASK-031).

**정규화 경계 — 한 자리에서만 한다:**

```text
whisper-rs 원시 segment (센티초)
        │
        ▼
  parse/정규화 모듈  ←── 단위 변환은 여기서만 일어난다 (TASK-025)
        │                프로세스 실행도 라이브러리 호출도 없는 순수 모듈이므로
        ▼                whisper 바이너리·모델 없이 테스트된다 (§18)
  start_ms · end_ms  ──→ 실행 경계 · 영속성 · 화면은 단위를 다시 만지지 않는다
```

- 실행 경계(TASK-026)는 원시 값을 **그대로** 넘긴다.
- 영속성(TASK-027)은 이미 밀리초인 값을 저장한다.
- 화면(TASK-030)은 밀리초 → `HH:MM:SS`만 한다. 단위 변환이 아니라 표시 변환이다.

**테스트가 ×10 / ×100 어긋남을 잡아야 한다** — 1분 30초는 `90000`이며 `9000`도 `900000`도
아니다 (TASK-025). 조용히 100배 어긋난 transcript는 Gate가 잡지 못하므로 값 단언으로 잡는다.

---

## 11. Windows에서 성립하는가 — 그리고 하지 않는 것

**평가 (근거와 함께):**

| 항목 | Windows에서 | 근거 |
| --- | --- | --- |
| `whisper-rs` 빌드 | CMake + C/C++ 툴체인을 요구한다. Tauri가 이미 요구하는 **Microsoft C++ Build Tools** 와 같은 계열이다 | [E2 · §14.4.1 · §14.2] |
| 배포물 | sidecar가 없으므로 **`.exe` 접미사 규약도, target triple 파일명도, 바이너리 확보 절차도 필요 없다** | [E2 · §14.4의 sidecar 규약이 요구하던 것들이 B에는 없다] |
| 가속 | 기본 빌드는 **CPU 전용** (CUDA · Vulkan · BLAS · ROCm 전부 기본 OFF) | [E2 · §14.4] |
| 모델 경로 | `AppDataDirectory`가 이미 플랫폼 차이를 흡수한다 (`app_data_dir()`) | [E2 · §14.2] |
| 리샘플링 | `hound` · `rubato`는 순수 Rust — 플랫폼 분기가 없다 | [E2] |
| **실제 Windows 빌드가 통과하는가** | **[E4] UNVERIFIED** | Windows 개발/검증 환경이 아직 없다 [E2 · §14.1] |

**판단: B는 Windows에서 성립할 가능성이 A보다 높다** — A가 Windows에서 추가로 요구했을
것(prebuilt zip 내용물 확인 [E4] 또는 두 번째 소스 빌드 · `.exe` 파일명 규약 · 두 벌의
바이너리 관리)이 B에는 없기 때문이다. 이것은 **가능성의 평가이지 검증이 아니다.**

**이 Phase가 하지 않는 것 (전부 Phase 6):**

```text
Windows 빌드 · Windows 바이너리 확보 · Windows 실행 검증
```

**추상화를 선입금하지 않는다 (§20.6 · §3.1).**

- `SidecarResolver`를 **만들지 않는다** — 해석할 sidecar가 없다.
- 이 Phase에서 플랫폼이 실제로 갈리는 지점은 **모델 파일 위치** 하나이며, 그것은 이미 있는
  `AppDataDirectory`가 처리한다. **새 플랫폼 경계를 만들지 않는다.**
- 엔진 실행 경계에 trait을 두는 이유(TASK-026)는 **플랫폼이 아니라 테스트다** — 실제
  whisper 없이 검증하기 위한 두 번째 구현(test double)이 지금 실재한다 (§18).
  "언젠가 Windows에서 다를 것"은 근거가 아니다.

---

## 12. 탈락한 후보와 탈락 이유

### 12.1 A — Tauri sidecar + `whisper-cli`

| 이유 | 근거 |
| --- | --- |
| **배포물에 들어갈 실행 파일을 저장소가 재현하지 못한다.** macOS prebuilt CLI가 없으므로 [E2 · 전수 확인] 소스 빌드 산출물을 손으로 배치해야 하는데, 그 파일은 커밋하지 않는다. clone → build로 동작하는 앱이 나오지 않고, **Gate가 실제 엔진 경로를 덮지 못한다** | §4.2 |
| **미확인 링크가 둘 더 붙는다** — 바이너리 단독 실행 가능성 [E4]과 externalBin 앱의 notarization [E4 · #11992] | §4.2 · §6 |
| **운영자 smoke test까지의 절차가 길다** — 소스 빌드 · 파일명 · 배치를 통과해야 추론에 도달한다 | §14.4.3 |

**탈락 사유가 아닌 것:** cmake 요구(B도 같다) · notarization 이슈 단독(packaging 위험이지
배포 불가의 증거가 아니다) · 언어.

**A가 더 나은 점은 남는다** — 프로세스 격리, upstream 최신 버전, 밀리초 timestamp,
Gate 빌드 비용 없음. 이것들은 §4.3에서 대가로 지불했다.

### 12.2 C — 사용자 설치 바이너리 참조

| 이유 | 근거 |
| --- | --- |
| **§14.4.2가 금지한 바로 그것을 요구한다** — 사용자가 whisper.cpp / Whisper CLI를 직접 설치해야 한다 | §14.4.2 |
| **선택 조건이 성립하지 않는다.** C는 *A와 B가 둘 다 실증된 blocker를 만났을 때만* 고려할 수 있다. **그런 blocker는 관측되지 않았다** — A의 문제도 B의 대가도 전부 감수 가능한 위험이거나 미확인 항목이다 | §14.4.2 · §4 |
| 사용자 기기의 whisper 버전·빌드 플래그·모델 호환성을 제품이 통제하지 못한다 | 귀결 |

**개발/디버그 fallback으로서의 위치 (별도로 적는다):**

C는 **제품 배포 경로가 아니지만 개발자의 비교 수단으로는 유효하다.** 개발자가 자기 기기의
`whisper-cli`로 같은 오디오를 돌려 **우리 파이프라인의 출력과 대조**하는 것은 정당한 디버깅이다
(예: 정규화가 ×10 어긋났는지, 파생 입력이 제대로 만들어졌는지).

단, 그때도 다음을 지킨다:

1. **제품 코드에 "설치된 whisper를 찾는" 경로를 만들지 않는다.** 앱은 그런 것을 탐색하지 않는다.
2. 대조는 개발자가 저장소 밖에서 손으로 한다 — 설정 항목도, fallback 분기도 만들지 않는다.
3. 그 결과를 제품 검증 증거로 적지 않는다. 검증은 앱이 실제로 낸 결과로 한다.

**"cmake가 처음에 없었다"는 C의 근거가 아니었고, 지금도 아니다** — 그 제약은 2026-09-03에
해소됐으며 [E2 · §14.1] 애초에 A·B 공통이었다.

---

## 13. 이 결정이 틀렸을 때 — 되돌리기 비용을 작게 유지한다

`whisper-rs`가 방치되거나(Codeberg 이전 · §4.3), v1.8.3이 우리가 쓰는 표면을 갖지
않거나(§5 [E4]), in-process abort가 실제로 감당 불가한 것으로 드러나면 **A로 되돌린다.**

그 비용이 작게 유지되도록 Phase 3의 구조를 이렇게 둔다:

```text
                    ┌─────────────────────────────┐
raw audio ─→ 파생 입력 │  TranscriptionEngine (trait) │ ─→ 원시 출력 ─→ 정규화 ─→ Transcript
 (§9)               └─────────────────────────────┘         (§10)
                     교체 대상은 이 구현 하나 +
                     정규화의 단위 상수 한 자리
```

- **교체해야 하는 것**: 엔진 구현 1개(라이브러리 호출 → 프로세스 실행), 정규화 입력 타입과
  단위(센티초 → 밀리초, 즉 계수 1), 그리고 sidecar 배치/`externalBin` 설정.
- **교체하지 않아도 되는 것**: 파생 입력 생성(§9는 `f32`를 만든다 — WAV 파일이 필요해지면
  같은 버퍼를 `hound`로 쓰면 된다), 영속성 규칙, 상태 전이, 화면, 테스트 대부분.

**그래서 이 선택은 되돌릴 수 있다.** 되돌릴 수 없는 것은 §7의 데이터 모델 규칙이지
엔진 구현이 아니다.

---

## 14. 확인한 것 / 확인하지 못한 것

| 항목 | 상태 | 근거 |
| --- | --- | --- |
| macOS용 prebuilt `whisper-cli`가 upstream 릴리스에 없다 | **VERIFIED** [E2] | 2026-09-03 · asset 9개 전수 확인 (§5) |
| Apple 대상 asset은 XCFramework 하나이며 CLI가 아니다 | **VERIFIED** [E2] | 2026-09-03 (§5) |
| Windows 대상 asset이 존재한다 | **VERIFIED** [E2] | 2026-09-03 (§5) |
| **Windows asset zip 안에 CLI exe가 있는가** | **UNVERIFIED** [E4] | §14.4.1이 미확인으로 남겼다. 이 Run은 네트워크가 없었다 |
| `whisper-rs` 0.16.0 / 번들 v1.8.3 / Codeberg 이전 | **VERIFIED** [E2] | 2026-09-03 (§5) |
| **`whisper-rs`의 정확한 API 시그니처** | **VERIFIED (2026-09-03 · 컴파일러)** | TASK-026이 확인했다. 기록과 다른 부분이 있었다 — §16.2 · `.loop/evidence/TASK-026/whisper-rs-api-verification.md` |
| **번들 whisper.cpp가 실제로 v1.8.3인가** | **UNVERIFIED** [E4] | 빌드는 성공했지만 번들 버전을 읽는 경로를 확인하지 못했다 (§16.3) |
| **`whisper-rs` segment timestamp가 실제로 센티초인가** | **UNVERIFIED (기록은 [E2])** | 타입이 `i64`라는 것만 확인됐다. **단위는 실제 추론을 한 번 돌려야 드러난다** — 운영자 smoke test의 PASS-3이 처음 관측한다 (`docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` §8). 그래서 `parse.rs`의 계수(×10)를 바꾸지 않았다 |
| **가속(Metal 등)이 실제로 켜져 있는가** | **UNVERIFIED** [E4] | `Cargo.toml`이 `whisper-rs`의 feature를 지정하지 않는다 [E1] — 기본값이 무엇인지 확인하지 않았다 (§16.3) |
| CMake가 A·B 공통 요구다 / 개발 Mac에 cmake 4.4.3이 있다 | **VERIFIED** [E2] | §14.4.1 · §14.1 |
| `hound` + `rubato` + 수동 다운믹스가 순수 Rust 변환 경로다 | **VERIFIED** [E2] | §14.4.1 |
| tauri#11992가 OPEN이며 재현 보고가 있다 | **VERIFIED** [E2] | 2026-09-03 (§6) |
| **#11992가 Tauri 2.11.5에서도 재현되는가** | **UNVERIFIED** [E4] | 확인하지 못했다. B는 이 경로를 쓰지 않는다 |
| **무엇을 배포하든 실제 codesign/notarization이 통과하는가** | **DEFERRED** | 배포 검증 경계 (§6) |
| 이 저장소의 raw recording이 장치 native 포맷의 PCM16 WAV다 | **VERIFIED** [E1] | `src-tauri/src/audio/capture.rs`의 `CaptureFormat` · ADR-0003 §4.2.3 |
| `transcript_segments`가 `start_ms` · `end_ms`를 갖는다 | **VERIFIED** [E1] | `src-tauri/src/db/migrations.rs` |
| `.gitignore`가 모델(`/models/` · `*.bin` · `*.gguf`)과 오디오를 제외한다 | **VERIFIED** [E1] | `.gitignore` |
| 현재 `Cargo.toml`에 whisper 관련 의존성이 없고 `tauri.conf.json`에 `externalBin`이 없다 | **VERIFIED** [E1] | 두 파일을 직접 읽었다 |
| **cold build가 Gate timeout(900초) 안에 끝나는가** | **관측됨 — 27.7초 (2026-09-03, 이 기기)** | TASK-026 (§4.3). 빈 registry·빈 `target/`·release 빌드는 측정하지 않았다 |
| **실제 Whisper 추론이 한 번이라도 성공하는가 (end-to-end)** | **NOT RUN — 운영자 smoke test 대기** | PRODUCT-SPEC §14.4.3. 절차는 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`에 있고 **TASK-031은 그 절차를 문서로 남겼을 뿐 실행하지 않았다.** 실행 전까지 "end-to-end 전사가 검증됐다"고 적지 않는다 |
| 실제 한국어 전사 품질 / 한국어+영어 혼용 | **DEFERRED** | Final Integration (`phase-prompt/03` Human Review) |
| timestamp가 실제 음성 위치와 맞는가 / 1시간 전사 소요 시간 | **DEFERRED** | 같음 |
| Windows 빌드 · 바이너리 · 실행 | **DEFERRED — Phase 6** | §11 |

---

## 15. 결과

```text
Recording (raw PCM16 WAV · 장치 native)      ← 읽기만 한다. 영원히 그대로다 (INV-1)
        │
        ▼  hound + rubato + 다운믹스 (순수 Rust · 메모리)
16 kHz mono f32 PCM                          ← 파생물. 언제든 다시 만든다
        │
        ▼  whisper-rs (앱 바이너리 안 · 모델 파일 하나만 밖에서 온다)
원시 segment (센티초)
        │
        ▼  정규화 — 코드에서 단 한 자리
segments[{ start_ms, end_ms, text }] + language + rawText
        │
        ▼  새 Transcript를 추가한다 (INV-2 · §7.1). 실패하면 current는 그대로다 (§7.2)
Transcript
```

사용자가 설치하는 것은 없다. 저장소가 커밋하지 않는 것은 모델과 오디오다.
**저장소 밖에서 오는 것은 모델 파일 하나뿐이며, 그것이 없는 상태는 제품 상태다.**

---

## 16. 구현 결과 — 실제로 만들어진 것 / 달라진 것 / 여전히 모르는 것

```text
갱신:  2026-09-03 · TASK-031 (문서 전용)
범위:  TASK-024 ~ TASK-030이 만든 것을 이 ADR의 결정과 대조한다
```

**여기 적힌 "구현됐다"는 코드가 존재하고 자동 검증이 그것을 지난다는 뜻이지,
실제 추론이 성공했다는 뜻이 아니다** (§16.3).

### 16.1 §2의 결정이 어디에 실현됐는가

| 결정 (§2) | 실현된 자리 | 확인 |
| --- | --- | --- |
| 1. 통합 방식은 B(`whisper-rs`) · `externalBin`을 쓰지 않는다 | `src-tauri/Cargo.toml`의 `whisper-rs = "0.16"` · `src-tauri/src/transcription/whisper.rs` | `tauri.conf.json`에 `bundle.externalBin`이 없고 `src-tauri/binaries/`도 없다. 프로세스 실행·shell 권한도 없다 |
| 2. 저장소 밖에서 오는 것은 모델 파일 하나 | 같음 | 배포물에 넣는 실행 파일이 없다 |
| 3. 모델은 설정에 저장한 경로/파일명으로 찾는다 | `src-tauri/src/db/migrations.rs` migration 5 (`transcription_model`) · `src-tauri/src/transcription/model.rs` | 파일명은 모델 디렉터리 기준, 절대 경로는 그대로 (`model.rs` 테스트) |
| 4. 모델 없음은 제품 상태이며 토글을 뒤집지 않는다 | `transcription/engine.rs`의 `TranscriptionModelMissing` · `src/screens/settingsView.ts`의 안내 | `src-tauri/tests/automatic_transcription.rs`의 `a_missing_model_is_reported_as_a_failure_and_the_toggle_is_left_as_the_user_set_it` |
| 5. 입력 변환은 순수 Rust · 메모리 위의 f32 · 원본은 읽기 전용 | `src-tauri/src/transcription/audio_input.rs` | 이 파일에 `File::create`·`fs::write`·`WavWriter`가 없다. `TranscriptionInput`에 경로 필드가 없다 |
| 6. 단위 변환은 한 자리에서만 | `src-tauri/src/transcription/parse.rs`의 `MILLISECONDS_PER_CENTISECOND = 10` | 실행 경계(`whisper.rs`)는 원시 값을 그대로 넘기고, 영속성·화면은 밀리초를 다시 만지지 않는다 |
| 7. `SidecarResolver`를 만들지 않는다 | — | `transcription/` 어디에도 sidecar 경로 해석도 `cfg(target_os)`도 없다 |
| 8. 모델·큰 바이너리를 커밋하지 않는다 | `.gitignore` (변경 없음) | 새 규칙이 필요하지 않았다 — 배포물에 넣을 바이너리가 없다 |
| 9. C는 제품 경로가 아니다 | — | 설치된 whisper를 탐색하는 코드 경로가 없다 |

**이 Phase가 새로 만든 실행 구조** (§13의 되돌리기 경계가 실제로 그 모양이다):

```text
run.rs        전사 한 건의 순서 (상태 기록 → 파생 입력 → 엔진 → 정규화 → 영속화)
 ├ audio_input.rs   WAV → 16 kHz mono f32 (hound + 수동 다운믹스 + rubato)
 ├ model.rs         모델 파일을 해석하는 단 한 곳
 ├ engine.rs        TranscriptionEngine trait + §13의 네 가지 실패
 │   ├ whisper.rs   실제 구현 (교체 대상은 이것 하나다 — §13)
 │   └ testing.rs   test double (실제 whisper·모델 없이 도는 자동 검증 · §18)
 └ parse.rs         센티초 → 밀리초 정규화 · 이상값 처리
commands/transcriber.rs   배경 스레드 소유 · start_transcription / transcription_status
```

### 16.2 계획과 달라진 것 — 그리고 왜

| 무엇이 | 계획 | 실제 | 왜 |
| --- | --- | --- | --- |
| **`whisper-rs`의 segment 접근 경로** | §5는 `WhisperSegment::start_timestamp()`만 기록했다 | `full_n_segments() -> i32` → **`get_segment(i32) -> Option<WhisperSegment<'_>>`** → `start_timestamp()/end_timestamp() -> i64` · `to_str()`. 언어는 **`full_lang_id_from_state()`** (`full_lang_id`가 아니다) | 문서에서 옮겨 적지 않고 **컴파일러가 보고한 실제 시그니처**를 썼다. `full_get_segment_text/_t0/_t1`은 없다 (E0599) — `.loop/evidence/TASK-026/whisper-rs-api-verification.md` |
| **`rubato` 5.0.0의 타입 이름** | §9.2는 crate와 버전만 적었다 | `Fft` + `FixedSync` + `audioadapter_buffers::direct::InterleavedSlice`. `FftFixedIn`·`SincFixedIn`·`VecResampler`는 **5.0.0에 없다** | 같은 방법(빌드)으로 확인했다 — `.loop/evidence/TASK-024/verification-log.md` |
| **리샘플러의 필터 지연** | 이 ADR은 언급하지 않았다 | `output_delay()`가 보고하는 프레임 수(관측: 43 프레임 ≈ 2.7 ms)를 **버리고 이어 붙인다.** 상수로 박지 않고 매번 리샘플러에게 묻는다 | 보정하지 않으면 앞에 무음이 붙고 뒤가 잘린다. 효력은 mutation 확인으로 검증했다 (TASK-024) |
| **`engine` provenance 문자열** | §8.2.4: `whisper-rs` 버전 **+ 번들 whisper.cpp 버전** | **`whisper-rs/0.16`만 기록한다** | 번들 whisper.cpp 버전을 런타임에 읽는 경로를 확인하지 못했다. **모르는 값을 provenance로 지어내지 않는다** (§20.2) |
| **모델 디렉터리 생성** | §8.2.2: 기본 탐색 위치는 앱 데이터 디렉터리 아래 `models/` | 경로는 그대로지만 **앱이 그 디렉터리를 만들지 않는다** — `AppDataDirectory::ensure_models_dir()`은 있으나 제품 호출자가 없다 | 모델을 두는 것이 사용자의 행위이므로 빈 디렉터리를 미리 만드는 코드가 필요하지 않았다. **대신 운영자가 만들어야 한다** — smoke test 절차 §4.2가 그 단계를 갖는다 |
| **모델을 언제 여는가** | 이 ADR은 정하지 않았다 | `WhisperEngine`은 모델을 들고 있지 않고 **전사마다 연다** | 설정에서 모델을 바꾸면 다음 전사부터 바로 반영되고, 쓰지 않는 동안 수 GB를 붙들지 않는다. 대가는 전사마다 드는 적재 시간이다 |
| **오디오 입력 실패의 종류** | §9.3: §13의 제품 실패로 매핑한다 | **새 `FailureKind`를 만들지 않고** 기존 `Storage`(열지/읽지 못함)와 `InvalidInput`(형식이 규칙에 맞지 않음)으로 갈랐다 | `FailureKind`는 `src/ipc/failure.ts`의 union과 1:1이다. 그 계약을 넓히면 frontend가 모르는 종류가 조용히 생긴다 |

**결정 자체를 바꾼 것은 없다.** 위는 전부 결정을 실현하는 과정에서 드러난 사실이며,
§2의 아홉 항목 중 철회되거나 수정된 것은 없다.

### 16.3 여전히 UNVERIFIED / DEFERRED — 확인한 것처럼 적지 않는다

| 항목 | 상태 | 어디서 판정되는가 |
| --- | --- | --- |
| **실제 Whisper 추론이 한 번이라도 성공하는가** | **NOT RUN** | 운영자 smoke test — `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`. **TASK-031은 절차를 문서로 남겼을 뿐 실행하지 않았다** |
| **segment timestamp의 단위가 실제로 센티초인가** | **UNVERIFIED** | 같은 문서의 PASS-3. 어긋나면 `parse.rs`의 계수 한 자리를 고치고 이 ADR을 다시 갱신한다 |
| **번들 whisper.cpp의 실제 버전** | **UNVERIFIED** | 읽는 경로를 확인하지 못했다 (§16.2) |
| **가속(Metal 등)이 켜져 있는가** | **UNVERIFIED** | `Cargo.toml`이 feature를 지정하지 않는다 |
| **in-process abort가 실제로 앱을 죽이는가** | **UNVERIFIED** | §4.3이 감수한 대가다. 추론이 한 번도 돌지 않았으므로 관측된 적이 없다 |
| **release 빌드 · 번들된 `.app`에서의 동작** | **UNVERIFIED** | 이 Phase는 `npm run tauri dev`만 다뤘다 |
| **codesign / notarization** | **DEFERRED** | 배포 검증 경계 (§6). `externalBin`을 쓰지 않는다는 사실이 이것을 통과시켜 주지는 않는다 |
| **Windows 빌드 · 바이너리 · 실행** | **DEFERRED — Phase 6** | §11 |
| **한국어 품질 · 한국어+영어 혼용 · timestamp와 음성 위치의 일치 · 1시간 소요 시간** | **DEFERRED** | Final Integration (`phase-prompt/03` Human Review) |

### 16.3.1 ASSUMPTION A-TRANS-001 (사용자가 수용한 위험 · 2026-09-03)

```text
A-TRANS-001

Phase 3의 local transcription architecture는 구현됐고 자동 검증을 통과했다.
그러나 실제 Whisper 추론은 아직 한 번도 실행되지 않았다.

실제 전사 smoke 검증은 운영자의 다음 integration test로 연기됐다.
```

**이것은 PASS가 아니다.** §16.3의 "실제 Whisper 추론이 한 번이라도 성공하는가"는
여전히 **NOT RUN**이며, 연기 결정이 그 상태를 바꾸지 않는다.

이 가정이 틀리면 — 즉 실제 추론이 실패하면 — §16.4의 되돌리기 경계가 그대로 쓰인다.
교체 대상은 엔진 구현 하나(`transcription/whisper.rs`)와 정규화 계수 한 자리(`parse.rs`)다.

**Phase 4는 이 가정 위에서 진행할 수 있다.** 단 실제 Transcript가 있다고 가정해야 하는
Task는 **결정론적 fixture / mock Transcript를 쓰고, 실제 Whisper 실행 결과를 꾸며내지 않는다.**

절차와 기록표는 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`에 있다.

### 16.4 §13의 되돌리기 경로는 그대로다

구현된 모양이 §13이 그린 그림과 같다 — 교체 대상은 **엔진 구현 하나
(`transcription/whisper.rs`)** 와 **정규화 계수 한 자리(`parse.rs`)** 이고, 파생 입력 생성 ·
영속성 규칙 · 상태 전이 · 화면 · 테스트 대부분은 엔진을 몰라도 된다
(`TranscriptionEngine` trait 뒤에 있고, `testing::StubEngine`이 이미 두 번째 구현이다).
**A로 되돌려야 할 이유는 아직 관측되지 않았다.**
