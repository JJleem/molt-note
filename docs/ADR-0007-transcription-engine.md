# ADR-0007 — 전사 엔진은 앱 안으로 링크하고, 밖에서 확보하는 것은 모델 하나뿐이다

```text
Status:   Accepted · 구현됨 (§16) — 실제 추론은 **2026-09-05에 처음 실행됐고 제품 경로는
          FAIL했다** (§17). 언어를 고친 뒤의 2026-09-07 실행도 **제품 결과는 붕괴했다** (§18).
          §17의 두 결정은 코드로 구현됐고 자동 검증을 통과했다 (§19) — **그것이 사람이
          읽을 만한 전사를 봤다는 뜻은 아니다** (§19.5 · `docs/PHASE-5.6-HUMAN-REVIEW.md`).
          notarization은 여전히 미확인 (§14)
          **전사 청크 분할 규칙은 §20에서 값으로 확정됐다 — 그 규칙은 아직 코드가 아니다** (§20)
          **§20의 규칙은 §21에서 코드가 됐다 — 다만 셋 중 하나(연속 반복 차단)는 순수 함수로만
          있고 그것을 부르는 제품 코드가 없다** (§21.2)
Date:     2026-09-03 (결정) · 2026-09-03 갱신 (구현 결과 반영 — §16) ·
          2026-09-06 갱신 (첫 실사용이 드러낸 언어 처리 결정과 Metal 결정 — §17) ·
          2026-09-07 갱신 (두 번째 실사용이 만든 전사 붕괴 판정 규칙 — §18) ·
          2026-09-07 갱신 (§17의 구현 결과 — §19) ·
          2026-09-07 갱신 (전사 청크 분할 규칙 — §20) ·
          2026-09-07 갱신 (§20의 구현 결과 — §21)
Phase:    Phase 3 — Local Transcription ·
          Phase 5.6 — Transcription Correctness + Reach (§17 · §19) ·
          Phase 5.7 — Recording Level + Transcription Collapse (§18) ·
          Phase 5.8 — Transcription Chunking (§20 · §21)
Task:     TASK-023 (결정) · TASK-031 (구현 결과 반영) · TASK-066 (§17) · TASK-076 (§18) ·
          TASK-075 (§19) · TASK-086 (§20) · TASK-091 (§21)
Scope:    whisper 통합 방식 · 엔진/바이너리 확보 경로 · 모델 관리 · 입력 포맷 변환 책임 ·
          timestamp 정규화 경계 · Windows 성립 여부 ·
          **전사 언어 결정 · 가속(Metal) 결정 (§17 · 2026-09-06 추가)** ·
          **전사 붕괴 판정 규칙 (§18 · 2026-09-07 추가)** ·
          **§17 두 결정의 구현 결과와 전사 소요 시간 기록 (§19 · 2026-09-07 추가)** ·
          **전사 청크 분할 규칙 — 청크 길이 · state 재생성 · 겹침 · 오프셋 · 연속 반복 차단
          (§20 · 2026-09-07 추가)** ·
          **§20 규칙의 구현 결과 · 자동 검증이 판정하지 못하는 것 · `A-TRANS-001`의 상태
          (§21 · 2026-09-07 추가)**
```

> **§1~§15는 결정 시점(구현 전)의 문서다.** 구현이 그 결정을 어떻게 실현했는지, 무엇이
> 계획과 달라졌는지, 무엇이 아직 UNVERIFIED로 남았는지는 **§16**에 있다.
> 운영자 smoke test 절차와 Phase 3 검증 기록표는
> `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`에 있다.
>
> **§17은 그 뒤에 붙었다 (2026-09-06 · TASK-066).** 2026-09-05에 운영자가 처음으로 실제
> 전사를 실행했고, 그 결과가 이 ADR이 정하지 않고 남겨 두었던 것 둘 — **전사 언어**와
> **가속(Metal)** — 을 결정으로 만들었다. **§1~§16을 다시 쓰지 않는다.** 그 절들은 각각
> 2026-09-03의 기록이며, 무엇이 언제 왜 바뀌었는지는 §17이 덧붙이는 형태로 적는다.
>
> **§18은 다시 그 뒤에 붙었다 (2026-09-07 · TASK-076).** §17이 정한 언어 처리가 구현된 뒤
> 운영자가 51분을 다시 녹음해 전사했고, 이번에는 **언어가 아닌 다른 이유로** 결과가 붕괴했다.
> §18은 그 실행이 만든 것 하나 — **무엇을 붕괴로 볼 것인가** — 를 값으로 확정한다.
> **§1~§17을 같은 규칙으로 다시 쓰지 않는다.**
>
> **§19는 §17의 구현 결과다 (2026-09-07 · TASK-075).** §16이 Phase 3의 결정(§2)에 대해 한 것을
> §19가 §17의 결정에 대해 한다 — 무엇이 실제로 만들어졌고, 무엇이 계획과 달라졌고, 무엇이
> 여전히 사람의 실행으로만 닫히는가. **번호가 §18보다 뒤인 것은 이 절이 §18보다 나중에
> 쓰였기 때문이며, 다루는 Phase는 §18(Phase 5.7)이 아니라 §17(Phase 5.6)이다.**
> **§1~§18을 고쳐 쓰지 않는다.**
>
> **§20은 다시 그 뒤에 붙었다 (2026-09-07 · TASK-086 · Phase 5.8).** §18이 *무엇을 붕괴로
> 볼 것인가*를 정했다면 §20은 **그 붕괴를 줄이려고 무엇을 할 것인가** — 전사를 어디서
> 자르고, 청크마다 무엇을 재생성하고, 무엇을 차단하는가 — 를 값으로 확정한다. 근거는
> 2026-09-05의 실험 하나이며 [E5], 그 실험이 재현한 조건 셋 중 **제품에 없는 하나**가
> 청크 분할이다 (`phase-prompt/05.8-transcription-chunking.md` P-2 · P-3).
> **§1~§19를 지우거나 다시 쓰지 않는다.**
>
> **§21은 §20의 구현 결과다 (2026-09-07 · TASK-091).** §16이 §2의 결정에 대해, §19가 §17의
> 결정에 대해 한 것을 §21이 §20에 대해 한다 — 무엇이 실제로 만들어졌고, 무엇이 계획과
> 달라졌고, **자동 검증이 무엇을 판정하고 무엇을 판정하지 못하며**, `A-TRANS-001`이 이 Phase
> 뒤에 어떤 상태인가. §20이 *"코드는 Phase 5.8의 다른 Task가 쓴다 … 실제 시그니처 · 실제
> 동작을 확인해 여기 적힌 것과 다르면 이 절에 되적는다"* 로 남긴 자리다 (§20.11).
> **§1~§20을 지우거나 다시 쓰지 않는다.**

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

> **2026-09-06 추가 — 이 목록은 2026-09-03 결정 시점의 아홉 항목이다. 지우거나 다시 쓰지
> 않는다.** 그 뒤 2026-09-05의 첫 실사용에서 결정 둘이 더해졌다 — **10. 전사 언어를 어떻게
> 정하는가**와 **11. 가속(Metal)을 켜는가**. 이 ADR은 그 둘을 이 자리에 끼워 넣지 않고
> **§17에 따로 적는다.** §2가 무엇을 결정했고 무엇을 결정하지 않았는지가 그대로 보여야
> 하기 때문이다 — 언어도 가속도 **결정한 적이 없어서** 기본값이 그대로 쓰였다는 것이
> §17이 기록하는 사실이다.

---

## 3. 근거의 종류 — 이 Run이 확인할 수 있었던 범위

**추측한 것을 확인한 것처럼 적지 않는다** (PRODUCT-SPEC §20.2). 아래 표기를 문서 전체에서 쓴다.

| 표기 | 뜻 |
| --- | --- |
| **[E1] 직접 확인** | 이 Run에서 저장소의 실제 파일을 읽어 확인했다 |
| **[E2] §14.4.1 재확인** | PRODUCT-SPEC §14.4.1이 **2026-09-03에 primary source에서 재확인**한 값. 오늘이 곧 도입 시점이다 |
| **[E3] §14.4 (2026-09-01)** | 2026-09-01 기록. §14.4.1과 어긋나면 §14.4.1이 우선한다 |
| **[E4] UNVERIFIED** | 확인하지 못했다. 구현 근거로 쓰지 않는다 |
| **[E5] 2026-09-05 실사용 관측** | *(2026-09-06 추가 · §17)* 운영자가 **실제로 앱을 켜서 녹음하고 전사한 결과** 관측된 값. 기록 위치는 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록과 `phase-prompt/05.6` Preconditions다. **관측이지 crate 소스나 공식 문서가 아니다** — [E1]/[E2]로 올리지 않는다 |
| **[E6] 2026-09-07 실사용 관측** | *(2026-09-07 추가 · §18)* 운영자가 **§17의 언어 수정이 들어간 뒤 다시 녹음하고 전사한 결과** 관측된 값. 기록 위치는 `phase-prompt/05.7` Preconditions다. **[E5]와 같은 성질이며 날짜와 대상 녹음만 다르다** — 관측이지 crate 소스도 공식 문서도 아니다 |

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
| **가속(Metal 등)이 실제로 켜져 있는가** | ~~**UNVERIFIED** [E4]~~ (2026-09-03) → ~~**2026-09-05 갱신: 꺼져 있다 — VERIFIED**~~ → **2026-09-07 갱신: 켜져 있다 — VERIFIED** | 결정 시점의 기록: `Cargo.toml`이 `whisper-rs`의 feature를 지정하지 않는다 [E1] — 기본값이 무엇인지 확인하지 않았다 (§16.3). **2026-09-05 실사용 조사가 그 기본값을 확인했다 — `whisper-rs-sys` build.rs가 `GGML_METAL = OFF`를 정의한다** [E5 · §17.2]. **2026-09-07: `features = ["metal"]`을 켠 뒤 `GGML_METAL`이 `ON`이 되고 `libggml-metal.a`가 생기며 Gate가 실행한 바이너리에 `ggml_metal_*` 심볼이 있다** [E1 · §19.2]. **켜졌다는 것과 실행 시 GPU가 실제로 쓰인다는 것은 다른 진술이다** — 뒤는 확인하지 않았다 |
| **Metal을 켜는 `whisper-rs` 0.16의 정확한 feature 이름** | ~~**UNVERIFIED** [E4]~~ (2026-09-06) → **2026-09-07 갱신: `metal` — 단, 출처가 crate 소스 파일이 아니다** | 결정 시점의 기록: 이 Run은 crate 소스도 공식 문서도 읽지 못했다. **이름을 지어내지 않는다** — §17.2.2가 시도한 확인 경로와 남은 정황을 적었다. **2026-09-07: cargo가 manifest를 파싱해 fingerprint에 남긴 `declared_features`에 두 crate 모두 `metal`이 있고 Metal 관련 이름은 그것 하나다** (§19.2). crate의 `Cargo.toml`을 연 것이 아니라 cargo가 그것을 읽고 남긴 기록을 열었으므로 **[E1]로 올리지 않는다** |
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
| **실제 Whisper 추론이 한 번이라도 성공하는가 (end-to-end)** | ~~**NOT RUN — 운영자 smoke test 대기**~~ (2026-09-03) → **2026-09-05 실행됨. 엔진 경로는 동작했고 제품 결과는 쓸 수 없었다** | 결정 시점의 기록: PRODUCT-SPEC §14.4.3. 절차는 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`에 있고 **TASK-031은 그 절차를 문서로 남겼을 뿐 실행하지 않았다.** 실행 전까지 "end-to-end 전사가 검증됐다"고 적지 않는다. **2026-09-05에 운영자가 실행했다 — PASS-1~4는 통과했으나 언어가 영어로 강제돼 결과가 쓸 수 없었다** [E5 · §17.1] |
| 실제 한국어 전사 품질 / 한국어+영어 혼용 | **DEFERRED** | Final Integration (`phase-prompt/03` Human Review). *(2026-09-06)* 제품 경로가 아직 한국어를 내지 못하므로 이 항목은 **열린 채로 남는다** (§17.3). *(2026-09-07)* 언어 경로는 고쳐졌으나 **품질 판정은 여전히 사람의 몫이다** — 절차와 빈 기록표는 `docs/PHASE-5.6-HUMAN-REVIEW.md` HR-1 · HR-2 (§19.5) |
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
| **실제 Whisper 추론이 한 번이라도 성공하는가** | **NOT RUN** *(2026-09-03 기준)* → **2026-09-05 실행됨 (§17)** | 운영자 smoke test — `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md`. **TASK-031은 절차를 문서로 남겼을 뿐 실행하지 않았다.** 실행은 2026-09-05에 이루어졌다 [E5] |
| **segment timestamp의 단위가 실제로 센티초인가** | **UNVERIFIED** *(2026-09-03 기준)* → **2026-09-05 VERIFIED — 센티초** | 같은 문서의 PASS-3. 어긋나면 `parse.rs`의 계수 한 자리를 고치고 이 ADR을 다시 갱신한다. **PASS-3은 통과했고 계수는 그대로다** [E5 · 부록] |
| **번들 whisper.cpp의 실제 버전** | **UNVERIFIED** | 읽는 경로를 확인하지 못했다 (§16.2). *(2026-09-06 기준으로도 여전히 열려 있다)* |
| **가속(Metal 등)이 켜져 있는가** | **UNVERIFIED** *(2026-09-03 기준)* → **2026-09-05 갱신: 꺼져 있다** → **2026-09-07 갱신: 켜졌다** | `Cargo.toml`이 feature를 지정하지 않는다. **그 결과 `whisper-rs-sys` build.rs가 `GGML_METAL = OFF`를 정의한다는 것이 확인됐다** [E5 · §17.2]. ~~**켜는 feature의 정확한 이름은 여전히 UNVERIFIED다** (§17.2.2)~~ → **2026-09-07: 이름은 `metal`이며 빌드 산출물로 켜진 것이 확인됐다** (§19.2). 출처가 crate 소스가 아니라는 단서와, **런타임에 GPU가 실제로 쓰이는지는 확인하지 않았다**는 사실이 함께 남는다 |
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

> **2026-09-06 추가 — 위 §16.3.1은 2026-09-03의 기록이며 그대로 둔다.** 그 뒤
> **2026-09-05에 그 실행이 실제로 이루어졌다.** 무엇이 드러났고 `A-TRANS-001`의 어느 부분이
> 아직 열려 있는지는 **§17.3**에 있다. 요약하면: 실행은 됐고, 가정은 **해소되지 않았다.**

### 16.4 §13의 되돌리기 경로는 그대로다

구현된 모양이 §13이 그린 그림과 같다 — 교체 대상은 **엔진 구현 하나
(`transcription/whisper.rs`)** 와 **정규화 계수 한 자리(`parse.rs`)** 이고, 파생 입력 생성 ·
영속성 규칙 · 상태 전이 · 화면 · 테스트 대부분은 엔진을 몰라도 된다
(`TranscriptionEngine` trait 뒤에 있고, `testing::StubEngine`이 이미 두 번째 구현이다).
**A로 되돌려야 할 이유는 아직 관측되지 않았다.**

---

## 17. 첫 실사용이 만든 결정 — 전사 언어와 가속(Metal)

```text
갱신:  2026-09-06 · TASK-066 (문서 전용 — 소스 · 설정 · 의존성 · 테스트를 바꾸지 않는다)
Phase: 5.6 — Transcription Correctness + Reach
근거:  2026-09-05 운영자의 첫 실사용 실행
       docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md 부록 · phase-prompt/05.6 Preconditions
```

**이 절은 §1~§16을 대체하지 않는다.** 그 절들은 2026-09-03의 기록이고 그대로 있다.
여기 적는 것은 **그 기록이 남겨 둔 빈자리 둘**이다.

§2는 아홉 가지를 결정했다. 그 아홉 가지 어디에도 **전사 언어를 어떻게 정하는가**와
**가속을 켜는가**가 없다. §14와 §16.3은 "가속이 켜져 있는가"를 UNVERIFIED로만 적어 두었다.
**결정하지 않은 자리에는 라이브러리의 기본값이 들어간다** — 2026-09-05는 그 기본값이
무엇이었는지를 제품 결과로 보여 준 날이다.

### 17.1 결정 10 — 전사 언어

#### 17.1.1 관측된 사실 (2026-09-05 · [E5])

운영자가 한국어 3인 회의(72분 51초 · 48 kHz mono PCM16)를 앱으로 녹음하고
`ggml-base.bin`으로 전사했다. 엔진 경로는 동작했다 — smoke test의 PASS-1 ~ PASS-4가 전부
통과했고, timestamp 단위가 센티초라는 것도 이때 처음 확인됐다 (§16.3 표 갱신).

그런데 **제품 결과는 쓸 수 없었다.**

```text
DB에 남은 language     en          ← 한국어 음성인데 영어
segment 총 개수        1,711
고유 문장               59  (3.4%)
한글이 나온 줄           0
최다 반복 문장         1,063회 (62.1%)
상위 2문장이 전체의     86.7%
```

00:28:10 ~ 00:56:55 구간에서는 **같은 문장 하나만** 출력됐다.
**이것은 전사가 아니라 디코딩 붕괴다.**

#### 17.1.2 원인 — 코드에서 확인됐다 ([E1] · 이 Run이 파일을 직접 읽었다)

`src-tauri/src/transcription/whisper.rs`가 `FullParams`에 설정하는 것은 이것이 전부다:

```text
set_n_threads · set_translate(false)
set_print_special · set_print_progress · set_print_realtime · set_print_timestamps
```

**`set_language`도 `set_detect_language`도 부르지 않는다.** 그리고 whisper.cpp의 기본값은
자동 감지가 아니다:

```c
// whisper.cpp/src/whisper.cpp:5943 — whisper_full_default_params   [E5]
/*.language        =*/ "en",     ← 기본이 "자동"이 아니라 "영어"
/*.detect_language =*/ false,    ← 감지도 꺼져 있다
```

그러므로 **DB에 남은 `language = en`은 감지 결과가 아니다.** 그것은 *아무도 바꾸지 않은
기본값*을 `whisper.rs`가 `full_lang_id_from_state()`로 되읽어 그대로 Transcript에 적은
값이다 [E1 · `whisper.rs`의 `let language = whisper_rs::get_lang_str(state.full_lang_id_from_state())`].

이 코드는 §16.2가 기록한 대로 **"엔진이 언어를 말하지 못하면 지어내지 않는다"** 는 규칙을
지키고 있었다. 규칙은 지켜졌지만 **엔진이 말한 것이 감지 결과가 아니었다.** 값을 읽는 쪽은
정직했고, 값을 정하는 쪽이 비어 있었다.

#### 17.1.3 원인이 **아닌** 것 — 디코더 설정 (반복 방지 장치는 정상이었다)

붕괴의 모양(같은 문장 반복)만 보면 디코더 설정을 의심하기 쉽다. **그 방향은 확인해 봤고,
원인이 아니다** [E5]. whisper.cpp 기본값에 반복 방지 장치가 **이미 전부 켜져 있다**:

```text
no_context      = true      ← 이전 segment를 다음 디코딩의 prompt로 넘기지 않는다
temperature_inc = 0.2       ← 실패한 디코딩을 더 높은 temperature로 다시 시도한다
entropy_thold   = 2.4       ← 엔트로피가 낮으면(=반복이면) 실패로 본다
logprob_thold   = -1.0      ← 평균 logprob이 낮으면 실패로 본다
```

**붕괴는 디코더 설정 문제가 아니라 틀린 언어를 강제한 결과다.** 한국어 음성을 영어로
디코딩하도록 강제하면 어떤 후보도 좋은 점수를 받지 못하고, 그 상태에서 모델은 학습 데이터의
흔한 영어 문장으로 무너진다. **고칠 자리는 디코더 파라미터가 아니라 언어다.**

> 이 구분을 남기는 이유가 있다. 파라미터를 만지는 것은 쉽고 그럴듯하며, 만지면 결과가
> *조금* 달라지므로 원인을 고친 것처럼 보인다. **원인이 아닌 것을 원인이 아니라고 적어
> 두지 않으면 다음 사람이 같은 자리를 다시 판다.**

#### 17.1.4 결정 — 이 앱이 앞으로 무엇을 부를 것인가

```text
사용자가 언어를 고르지 않았다   →  자동 감지를 켠다
                                  set_detect_language(true)에 해당하는 호출
                                  (기본값 "en" + detect_language=false를 그대로 두지 않는다)

사용자가 언어를 골랐다          →  그 언어를 지정한다
                                  set_language(Some("<선택한 코드>"))에 해당하는 호출

어느 경우에도                   →  set_translate(false)는 그대로다 (§2 · 번역이 아니라 전사다)
```

두 갈래를 정하는 규칙:

1. **기본값에 기대지 않는다.** "고르지 않음"도 하나의 선택이며, 그 선택이 뜻하는 것은
   **영어가 아니라 자동 감지**다. 언어 파라미터를 건드리지 않는 코드 경로는 남기지 않는다.
2. **감지 결과와 사용자 지정을 Transcript에서 구분할 수 있어야 한다.** §7의 `language`는
   지금까지 "엔진이 말한 값"이었다. 사용자가 골랐다면 그 값은 사용자가 정한 것이고,
   고르지 않았다면 엔진이 감지한 것이다 — **둘 다 `language`에 들어가지만 출처가 다르다.**
   Transcript는 immutable하므로(§7.1 · INV-2) 이 값은 그 전사가 어떤 조건에서 만들어졌는지의
   기록이기도 하다.
3. **모르는 값을 지어내는 규칙은 그대로다** (§16.2). 감지가 실패해 엔진이 언어를 말하지
   못하면 `language`는 여전히 비어 있다. **"감지를 켰다"가 "항상 값이 있다"는 뜻이 되지
   않는다.**
4. **이 결정은 `transcription::whisper` 안에서 끝난다.** §13이 정한 교체 지점 하나이며,
   그 경계를 넓히지 않는다.

**이 절은 무엇을 부를 것인가를 정한다. 부르는 코드는 이 Task가 쓰지 않는다** — 이 Task는
문서만 바꾼다. 구현은 Phase 5.6의 다른 Task가 하고, 그 Task가 실제 호출 시그니처를
컴파일러로 확인해 여기 적힌 것과 다르면 §16.2가 한 것처럼 이 절에 되적는다.

#### 17.1.5 PRODUCT-SPEC §D와의 연결 — 새 제품 결정이 아니다

**Spec은 이미 이것을 요구하고 있었다** [E1 · `docs/PRODUCT-SPEC.md` §D Settings]:

```text
| 그룹          | 항목                                                        |
| Transcription | whisper model · language · automatic transcription ON/OFF   |
                                 ^^^^^^^^
```

그런데 `settings` 테이블에는 언어 열이 없다 [E5 · `phase-prompt/05.6` R-2의 실측]:

```text
recordings_directory · automatic_processing · default_microphone
automatic_transcription · transcription_model
ai_provider · ai_base_url · ai_model · notion_parent_page_id
```

즉 **§17.1.4는 새 제품 방향이 아니라 Phase 3이 빠뜨린 Spec 항목을 메우는 것이다.**
`transcription_model`이 이미 §8.2대로 설정에 있는 것과 같은 자리에 `language`가 온다.
"고르지 않음"이 유효한 값이라는 것이 §17.1.4-1의 귀결이며, 그것이 이 설정의 기본 상태다.

### 17.2 결정 11 — 가속(Metal)

#### 17.2.1 관측된 사실 — Spec이 적은 것과 저장소가 하는 것이 다르다

| | 무엇이 | 근거 |
| --- | --- | --- |
| **Spec이 적은 것** | `whisper-rs` 0.16.0 (`whisper-rs-sys` 0.15.0, **Metal feature**) | [E1] `docs/PRODUCT-SPEC.md` §14.4 "Rust 바인딩" 문단. `phase-prompt/05.6` R-3은 이 줄을 **`PRODUCT-SPEC:836`** 으로 인용했고, **이 Run이 오늘 같은 파일에서 다시 찾은 줄 번호는 901이다** — 줄 번호는 문서가 바뀌면 움직이므로 인용은 §14.4 문단에 건다 |
| **이 ADR이 적은 것** | §4.1 후보 비교표: Apple Silicon에서 A는 "Metal 기본 ON", B는 "`metal` feature flag" | [E2 · 2026-09-03] — **후보를 비교한 기록이지 이 저장소가 그 feature를 켰다는 기록이 아니다** |
| **저장소가 실제로 하는 것** | `whisper-rs = "0.16"` — **feature 지정이 없다** | [E1] `src-tauri/Cargo.toml`. 이 Run이 파일을 직접 읽었다 |
| **그 결과** | `whisper-rs-sys` 0.15.0의 build.rs가 **`GGML_METAL = OFF`를 명시적으로 정의한다.** CoreML · OpenMP도 꺼지고 Accelerate만 링크된다 | [E5] 2026-09-05 실사용 조사가 `whisper-rs-sys-0.15.0/build.rs`(258~265행)를 읽어 확인했다 — `phase-prompt/05.6` R-3. **이 Run은 그 파일을 다시 읽지 못했다** (§17.2.2) |

**§14와 §16.3이 "가속이 켜져 있는가 — UNVERIFIED"로 남겨 두었던 항목의 답은 "꺼져 있다"이다.**
두 표를 그 자리에서 갱신했고, 원래 문장은 지우지 않고 남겨 두었다.

**결정: Spec §14.4가 적은 대로 Metal을 켠다.** Spec과 저장소가 어긋나 있었고, 어긋난 쪽은
저장소다. 이것도 §17.1과 같은 종류의 일이다 — 새 방향이 아니라 **Spec과 구현 사이의 간극**이다.

#### 17.2.2 켜는 feature의 정확한 이름 — **UNVERIFIED (2026-09-06)**

**이 Run은 이름을 확인하지 못했다. 그래서 이름을 결정으로 적지 않는다.**

| 시도한 확인 경로 | 결과 |
| --- | --- |
| **crate 소스** — `whisper-rs` 0.16.0의 `Cargo.toml` `[features]` (cargo registry의 추출된 소스) | **읽지 못했다.** 이 Run은 작업 디렉터리 밖의 파일에 접근 권한이 없다 |
| **crate 소스** — `whisper-rs-sys` 0.15.0의 `build.rs` · `Cargo.toml` | **같은 이유로 읽지 못했다.** §17.2.1의 `GGML_METAL = OFF`는 [E5]로만 남는다 |
| **공식 문서** — docs.rs의 `whisper-rs` 0.16.0 feature 목록 · Codeberg의 crate 저장소 | **가져오지 못했다.** 이 Run에는 네트워크 접근이 없다 (§3의 2026-09-03 Run과 같은 제약이다) |
| **저장소 안의 기록** — `src-tauri/Cargo.lock` | 버전은 확인했다 [E1] — `whisper-rs` **0.16.0** (checksum `2088172d…`) · `whisper-rs-sys` **0.15.0** (checksum `6986c0fe…`). 그러나 **lock 파일은 crate가 어떤 feature를 갖는지 기록하지 않는다** |

**정황은 있다. 정황은 crate 소스가 아니다.**

2026-09-05에 운영자가 **저장소 밖의 검증용 도구**를 `whisper-rs`에 `features = ["metal"]`을
준 채로 빌드해 같은 오디오를 돌렸고, 그 실행에서 whisper.cpp가 GPU를 잡았다고 보고했다
(`GPU name: Apple M5` · `use gpu = 1` · `backends = 3`) [E5 · 부록 결과 2·3].
**빌드가 성립했다는 것은 그 이름의 feature가 존재한다는 강한 정황이다.**

그럼에도 이 절은 그것을 VERIFIED로 올리지 않는다. §3이 정한 구분 그대로다 —
**"확인된 이름"과 "이 조합이 한 번 빌드됐다는 관측"은 다른 진술이다** (PRODUCT-SPEC §20.2).
그리고 이 도구는 제품 코드가 아니었고, 그 실행에서는 언어 · Metal · 청크 분할 **셋이 함께**
바뀌었으므로 개별 기여도도 분리되지 않았다 [E5].

**실제로 feature를 켜는 Task가 해야 할 일:**

1. `Cargo.toml`을 고치기 **전에** `whisper-rs` 0.16.0의 `[features]`를 직접 읽는다.
   확인 대상 버전은 위 표의 lock 값 그대로다.
2. 확인된 이름을 이 절에 **[E1]로 되적고** UNVERIFIED 표시를 걷는다. §14의 표도 같이 고친다.
3. feature를 켠 뒤 **`Cargo.lock`이 실제로 달라졌는지로 판정한다.** ADR-0009 §11.4가
   TLS feature에 대해 세운 규칙과 같다 — *"feature를 켰다고 적어 두고 lock이 그대로면
   켜지지 않은 것이다."* 링크된 결과(`GGML_METAL`이 ON인가)를 실행으로 한 번 확인한다.

#### 17.2.3 켰을 때 무엇이 빨라지는가 — **이 프로젝트는 측정한 적이 없다**

**어떤 배수도, 어떤 소요 시간 추정치도 이 문서에 적지 않는다.**

2026-09-05의 실행에서 관측된 시간 값들은 **Metal의 효과로 읽을 수 없다.** 언어 수정과
Metal 활성화가 **함께** 적용됐고, 비교 대상이 된 쪽은 **붕괴한 디코딩**이었기 때문이다
(같은 문장을 1,063회 출력하는 실행의 소요 시간은 정상 디코딩의 속도가 아니다).
**두 요인을 분리한 측정은 하지 않았다** [E5 · 부록 결과 3의 명시적 단서].

그러므로:

- Metal을 켜는 근거는 **"Spec이 적은 대로 되어 있지 않다"** 이지 측정된 속도가 아니다.
- 체감 차이의 판정은 **사람의 Human Review 항목**이다 (`phase-prompt/05.6` Human Review —
  *"Metal 활성화 전후로 체감이 달라지는가"*).
- 전사에 걸린 시간이 **기록으로 남아야** 사람이 비교할 수 있다는 것이 Phase 5.6의 성공
  기준 3에 들어 있다. **이 ADR은 숫자를 적는 대신 그 기록 경로가 생겨야 한다고 적는다.**

#### 17.2.4 이 결정이 지불하는 대가 — §4.3이 이미 적어 둔 것

§4.3의 **"Gate 비용 증가 — cold build가 whisper.cpp 컴파일을 포함한다"** 가 그대로 돌아온다.
feature를 바꾸면 `whisper-rs-sys`의 build.rs가 다른 CMake 정의로 다시 돌고,
**whisper.cpp가 재컴파일된다.**

| | 사실 | 근거 |
| --- | --- | --- |
| Gate timeout | `lint` · `test` 둘 다 900초. 두 주석 모두 *"cold Rust 빌드를 포함할 수 있다"* 로 적혀 있다 | [E1] `.loop/project.yaml` |
| 이미 관측된 값 | TASK-026이 `whisper-rs`를 추가한 뒤 첫 lint Gate는 27.7초에 끝났다 | §4.3 [E1 · TASK-026 evidence] |
| **그 값이 말하지 않는 것** | 그것은 **Metal이 꺼진 구성**의 빌드다. feature를 바꾼 뒤의 빌드에 대해서는 아무 말도 하지 않는다 | §4.3이 이미 같은 단서를 달았다 (cargo 캐시 · 빈 `target/` · release 빌드) |

**여기서도 시간을 추정해 적지 않는다.** feature를 켜는 Task가 Gate를 실제로 돌려 측정하고,
900초를 넘으면 그때 timeout을 조정한다 (`phase-prompt/05.6` Constraints).

### 17.3 `A-TRANS-001`은 어떻게 됐는가 — 실행은 됐고 가정은 해소되지 않았다

**§16.3.1을 지우지 않는다.** 그것은 2026-09-03에 사용자가 수용한 위험의 기록이며 그대로 있다.
여기 적는 것은 **그 뒤에 일어난 일**이다.

```text
2026-09-03   A-TRANS-001 수용
             "구현됐고 자동 검증을 통과했다. 그러나 실제 추론은 한 번도 실행되지 않았다."
             실측은 운영자의 다음 integration test로 연기됐다.

2026-09-05   그 실행이 이루어졌다.
```

**그 실행이 드러낸 것:**

| | 결과 |
| --- | --- |
| **엔진 경로 자체** | **동작한다.** PASS-1(segment + timestamp) · PASS-2(`engine = whisper-rs/0.16` · 지정한 모델) · PASS-3(timestamp 자릿수) · PASS-4(재시작 후 DB에 남아 있다)가 전부 통과했다 [E5] |
| **§13이 그린 구조** | 실물로 확인됐다 — `f32` 버퍼 → 엔진 → 정규화 → 영속화가 72분 오디오 하나를 통과했다 |
| **§16.3의 timestamp 단위** | **센티초로 확인됐다.** `parse.rs`의 계수(×10)를 바꿀 이유가 없다 [E5 · PASS-3] |
| **§16.3의 in-process abort** | 이 실행에서 앱은 죽지 않았다. **abort 자체가 관측되지 않았다는 뜻이며, 일어나지 않는다는 뜻이 아니다** — §4.3의 대가는 여전히 미확인이다 |
| **제품 결과** | **쓸 수 없었다.** 한국어 사용자가 한국어 회의를 녹음하면 아무것도 얻지 못한다 (§17.1.1) |

**아직 열려 있는 것:**

```text
A-TRANS-001 는 해소되지 않았다.

"실제 Whisper 추론이 한 번이라도 성공하는가"의 답은
  엔진 경로에 대해서는  →  YES  (PASS-1~4)
  제품에 대해서는       →  NO   (한국어가 영어로 강제 디코딩됐다)

이 가정은 제품 경로가 고쳐지고 사람이 다시 확인할 때까지 열려 있다.
```

**Gate가 녹색이었다는 사실이 이것을 조금도 막지 못했다는 것을 함께 기록한다.** 코드는
컴파일됐고 테스트는 통과했으며 Verifier도 통과했다. 자동 검증은 §18대로 실제 whisper 없이
`StubEngine`으로 돌기 때문이다 — **stub은 언어 파라미터가 비어 있다는 것을 알 수 없다.**
이것은 Gate의 결함이 아니라 **Gate가 덮는 범위의 경계**이며, §14의 마지막 줄
*"실행 전까지 'end-to-end 전사가 검증됐다'고 적지 않는다"* 가 정확히 이 경계를 지키려던
문장이었다. 그 문장은 지켜졌다.

### 17.4 이 Phase가 하지 않는 것 — 모델 크기 판정은 사람의 몫이다

PRODUCT-SPEC §14.4의 모델 문단은 *"한국어+영어 혼용 1시간 녹음에 `large-v3` /
`large-v3-turbo`가 현실적"* 이라고 적고 **UNVERIFIED · Phase 3에서 실측한다**고 표시했다
(`phase-prompt/05.6`은 이 줄을 `Spec:838`로 인용한다 — §17.2.1과 같은 줄 번호 단서가 붙는다).

**Phase 5.6은 그 판정을 하지 않는다.**

| | |
| --- | --- |
| **왜 하지 않는가** | 한국어 전사 품질의 판정에는 **사람의 귀**가 필요하다. Worker도 Gate도 Verifier도 "이 전사가 읽을 만한가"를 판정할 수 없다 |
| **대신 무엇을 하는가** | **여러 모델을 쓸 수 있게 만든다.** 모델은 §8.2대로 이미 설정에서 오는 경로이며 (`transcription_model`), 어느 모델이 충분한지를 앱이 고르지 않는다 — *"전사가 느리다는 이유로 정확도가 낮은 모델을 조용히 강제하지 않는다"* (§8.1) |
| **누가 판정하는가** | 사람. `phase-prompt/05.6`의 Human Review 항목 — *"ggml-base로 충분한가, 더 큰 모델이 필요한가 (A-TRANS-001의 실질적 답)"* |
| **이 ADR에 무엇을 적지 않는가** | **`large-v3`가 필요하다고도, `base`로 충분하다고도 적지 않는다.** 2026-09-05에 두 모델의 출력을 비교한 관측이 있지만 [E5 · 부록 결과 4], 그것은 **한 기기 · 한 오디오 · 한 사람의 판단**이며 제품 기본값을 정하는 근거로 이 ADR에 올리지 않는다 |

같은 이유로 **무음 구간의 환각**(부록 결과 5)과 **VAD 도입**도 이 절이 결정하지 않는다.
관측으로 기록돼 있고 [E5], 다음 후보로 남는다.

### 17.5 이 갱신이 바꾼 것과 바꾸지 않은 것

**바꾸지 않은 것 — §2의 아홉 가지 결정은 하나도 철회되거나 수정되지 않았다.**
통합 방식은 여전히 B(`whisper-rs`)이고, 배포물에 넣는 실행 파일은 없으며, 모델은 사용자가
두는 파일 하나다. §13의 되돌리기 경로도 그대로다 — **§17의 두 결정은 모두 `transcription::whisper`
안에서 끝나며, 그 파일은 §13이 이미 "교체 대상 하나"로 지정한 자리다.** 경계가 넓어지지 않았다.

**바꾼 것 — 결정하지 않고 남겨 두었던 자리 둘을 결정으로 채웠다.**

| 언제 | 무엇을 | 왜 |
| --- | --- | --- |
| 2026-09-06 (TASK-066) | **§17.1 언어 처리 결정을 추가** | 결정이 없어서 whisper.cpp의 기본값(`"en"` · 감지 OFF)이 그대로 쓰였고, 2026-09-05 실사용에서 한국어 회의가 영어로 강제 디코딩돼 붕괴했다 |
| 2026-09-06 (TASK-066) | **§17.2 Metal 결정을 추가** | Spec §14.4가 적은 Metal feature가 `Cargo.toml`에 없어 build.rs가 `GGML_METAL = OFF`를 정의하고 있었다. Spec과 저장소가 어긋났고 어긋난 쪽이 저장소였다 |
| 2026-09-06 (TASK-066) | **§14 · §16.3의 두 UNVERIFIED 항목을 갱신** | "실제 추론이 한 번이라도 성공하는가"와 "가속이 켜져 있는가"에 2026-09-05가 답을 줬다. **원래 문장을 지우지 않고 그 뒤에 갱신을 덧붙였다** |
| 2026-09-06 (TASK-066) | **§3에 [E5] 표기를 추가** | 실사용 관측을 [E1](저장소 파일 직접 확인)이나 [E2](primary source 확인)와 섞지 않기 위해서다 |

**이 Task는 문서만 바꿨다.** `src-tauri/src/transcription/whisper.rs`도, `Cargo.toml`도,
설정도, 테스트도 이 Task가 건드리지 않았다. §17.1.4와 §17.2가 정한 것은 **무엇을 부를
것인가**이며, 부르는 코드는 Phase 5.6의 다른 Task가 쓴다.

---

## 18. 두 번째 실사용이 만든 결정 — 전사 붕괴를 무엇으로 판정하는가

```text
갱신:  2026-09-07 · TASK-076 (문서 전용 — 소스 · 설정 · 의존성 · 테스트를 바꾸지 않는다)
Phase: 5.7 — Recording Level + Transcription Collapse
근거:  2026-09-07 운영자의 두 번째 실사용 실행  [E6]
       phase-prompt/05.7-recording-level-and-transcription-collapse.md Preconditions
       docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md 부록 (2026-09-05 · [E5])
```

> **번호 주의 — 이 문서의 §10 · §16.1 · §17.3이 괄호로 인용한 "(§18)"은 이 절이 아니다.**
> 그 셋은 `docs/PRODUCT-SPEC.md` §18(Validation Philosophy — 자동 검증이 실제 whisper 없이
> 돈다는 규칙)을 가리킨다. **그 문장들을 고치지 않았다.** 이 절은 이 ADR의 열여덟 번째
> 절이며, 이 절을 가리킬 때는 "ADR-0007 §18"로 적는다.

**이 절은 §1~§17을 대체하지 않는다.** §2의 아홉 결정도, §17의 두 결정도 그대로다.
여기 적는 것은 **§17이 고친 뒤에도 남아 있던 자리** 하나다.

§17은 "언어가 영어로 강제된다"를 고쳤고 **그 수정은 실제로 동작했다** [E6 · `05.7` R-1].
그런데 2026-09-07의 전사는 여전히 쓸 수 없었고, **제품은 그것을 `done`으로 저장했다.**
이 ADR도, 코드도, **"쓸 수 없는 전사"가 무엇인지 한 번도 정한 적이 없기 때문이다** —
저장 직전 검사가 물었던 것은 `segments.is_empty()` 하나였다 [E1 · `run.rs`].

### 18.1 관측된 사실 — 세 번의 전사가 남긴 값

**[관측된 사실]** 같은 방법(총 segment 수 · 고유 문장 수 · 최다 반복 문장의 출현 횟수)으로
기록된 세 실행이다.

| 날짜 | 대상 오디오 | 조건 | segment | 고유 문장 | 최다 반복 문장 | 사람의 판정 |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-09-05 [E5] | 9/4 녹음 (72.85분) | `ko` + Metal + `large-v3-turbo` + 청크분할 (**저장소 밖의 검증용 도구**) | 1,749 | 1,643 (**94.0%**) | 10회 (**0.6%**) | **쓸 수 있다** |
| 2026-09-05 [E5 · §17.1.1] | 9/4 녹음 (72.85분) | 제품 경로 · 언어 미설정(→ `en` 강제) · `base` | 1,711 | 59 (**3.4%**) | 1,063회 (**62.1%**) | **붕괴** |
| 2026-09-07 [E6] | 9/7 녹음 (50.99분) | **제품 경로** · `ko` · `large-v3-turbo` | 103 | 2 (**1.9%**) | 102회 (**99.0%**) | **붕괴** |

> **최다 반복 문장의 점유율 두 개는 이 절이 계산한 값이다** — `10 / 1,749 = 0.6%` ·
> `1,063 / 1,711 = 62.1%` · `102 / 103 = 99.0%`. 원 기록에 있는 것은 출현 횟수와
> 총 segment 수이며, 나눗셈만 여기서 했다.

**[관측된 사실]** 2026-09-07의 103개는 **정확히 30초 간격**으로 t=0부터 오디오 끝까지 빈틈이
없었다. 30초는 whisper의 처리 윈도우 크기이며, **모든 윈도우가 예외 없이 환각을 출력했다**
— 사람이 말하고 있었을 구간을 포함한 전 구간이 그렇다 [E6 · `05.7` R-2].
반복된 문장(`한글자막 by 한효정`)은 2026-09-05에도 나왔지만 그때는 **약 10분의 무음 구간에서만**
나왔다 [E5 · 부록 결과 5]. **양상이 다르다.**

**[관측된 사실]** 그 103개짜리 결과가 `transcription_status = done`으로 저장됐고
`current_transcript_id`가 그것을 가리켰다. 사람이 화면에서 본 것은 실패가 아니라 완료된
전사다 [E6 · `05.7` R-4].

**[관측된 사실]** 저장 직전 검사는 개수만 본다 [E1 · `src-tauri/src/transcription/run.rs`] —
1회차는 segment가 0이라 걸렸고, 2회차는 **103개가 "있어서" 통과했다.**
그 검사의 주석이 적은 목적은 **빈** Transcript가 immutable하게 남는 것을 막는 것이다(INV-2).
**고유 문장 2개짜리 Transcript를 막는 것은 그 검사의 목적이 아니었다** — 아래는 새로 정하는
규칙이지 그 검사의 버그가 아니다.

### 18.2 규칙 — 값으로 확정한다 (나중에 정하지 않는다)

```text
입력   이미 정규화된 segment 열 (§10의 parse가 낸 것 — 센티초 변환이 끝난 값)

문장   segment의 텍스트에서 앞뒤 공백을 걷고, 내부의 연속 공백을 하나로 줄인 문자열.
       대소문자와 문장부호는 건드리지 않는다.
       그렇게 만든 결과가 빈 문자열이면 문장으로 세지 않는다.

n      빈 문장을 뺀 문장의 총 개수
u      서로 다른 문장의 개수                      →  고유 비율        = u / n
r      가장 많이 나온 문장의 출현 횟수              →  최다 반복 점유율  = r / n

판정
  n == 0                                        →  쓸 수 없다 (빈 결과)
  n <  20                                       →  붕괴로 판정하지 않는다 (쓸 수 있다)
  n >= 20 이고  u/n <= 0.20                      →  붕괴
  n >= 20 이고  r/n >= 0.50                      →  붕괴
  그 밖                                          →  쓸 수 있다
```

**세 값을 왜 그 값으로 골랐는가 — 관측이 양쪽에 무엇을 놓았는가로 적는다.**

| 임계값 | 값 | 아래쪽(붕괴)에 있는 관측 | 위쪽(쓸 수 있음)에 있는 관측 | 여유 |
| --- | --- | --- | --- | --- |
| **고유 비율** | **20%** | 1.9% (2026-09-07) · 3.4% (2026-09-05) | 94.0% (2026-09-05) | 가장 가까운 붕괴의 **약 5.9배 위**, 유일한 정상 관측의 **약 4.7배 아래**. 두 관측을 로그 눈금에 놓았을 때 거의 가운데다 (√(3.4 × 94.0) ≈ 17.9) |
| **최다 반복 점유율** | **50%** | 99.0% (2026-09-07) · 62.1% (2026-09-05) | 0.6% (2026-09-05) | 정상 관측에서는 **83배 위**지만, 가장 가까운 붕괴(62.1%)까지는 **12.1%p뿐이다. 세 값 중 가장 약한 임계값이며 그 사실을 감추지 않는다** |
| **최소 segment 수** | **20** | — | — | 관측된 두 붕괴(103 · 1,711)는 이 값의 **5배 이상**이다. 20개 미만에서는 `u/n`의 눈금이 5%p보다 굵어지고, `r/n >= 0.50`이 "같은 말이 10번 나왔다"만으로 성립한다 — 짧고 반복적인 대화가 실제로 그럴 수 있다 |

**경계에서 어느 쪽으로 기우는가 — 이 판정은 저장을 막는다.** 거짓 양성의 대가는
**사람이 실제로 녹음한 회의의 전사를 버리는 것**이고, 거짓 음성의 대가는 §18.1이 이미 보여
준 것(쓸 수 없는 결과가 `done`으로 남는다)이다. 둘 다 나쁘지만 **되돌릴 수 없는 쪽은 앞이다**
— 원본 오디오는 남으므로 다시 전사할 수 있지만, 그것은 사람이 다시 4분을 기다린다는 뜻이다.
그래서 세 값은 전부 **통과 쪽으로 조금씩 기울여** 골랐다: 최소 개수 미만은 아예 판정하지
않고, 최다 반복 점유율은 관측된 정상값(0.6%)이 아니라 관측된 붕괴값(62.1%) 가까이에 두었다.

**두 조건을 OR로 묶은 이유, 그리고 그 근거가 어디까지인지.** 둘은 서로 다른 모양의 붕괴를
겨냥한다 — *문장의 가짓수 자체가 없는 것*과 *한 문장이 전사를 지배하는 것*이다.
**[미검증]** 다만 관측된 두 붕괴는 **둘 다 두 조건에 함께 걸린다.** 즉 어느 한쪽만으로
충분한지, 둘 다 필요한지를 **이 관측들은 가르지 못한다.** 둘을 함께 두는 것은
"한쪽만 걸리는 붕괴가 있을 수 있다"는 판단이며, 그 판단을 뒷받침하는 관측은 아직 없다.

### 18.3 이 규칙이 사는 자리 — 순수 모듈 하나

```text
정규화된 segment 열
        │
        ▼
  붕괴 판정 모듈  ←── 임계값과 비율 계산은 여기서만 일어난다
        │            파일시스템 · 저장소 · 네트워크 · 시계 · 엔진을 알지 않는다
        ▼            (예: src-tauri/src/transcription/collapse.rs)
  판정 하나 + 그 판정을 만든 수치 (n · u · u/n · r · r/n)
```

- **§10의 정규화 모듈이 앉은 자리와 같은 성질이다** — 프로세스 실행도 라이브러리 호출도
  없으므로 **실제 whisper도 모델도 없이 테스트된다** (PRODUCT-SPEC §18).
  §17.3이 기록한 Gate의 한계("stub은 언어 파라미터가 비어 있다는 것을 알 수 없다")가
  이 규칙에는 적용되지 않는다. 붕괴한 segment 열은 test double이 그대로 만들어 낼 수 있다.
- **같은 규칙을 `run.rs`에도, 엔진에도, 화면에도 두지 않는다.** 임계값이 두 자리에 있으면
  한쪽만 고쳐지는 날이 온다. 화면은 판정을 다시 하지 않고 backend가 준 수치와 문장을 쓴다.
- **판정은 수치를 잃지 않는다.** 사람이 읽는 실패 문장이 *무엇이 얼마나 반복됐는가*를 말할 수
  있어야 하고, 그 수치가 남지 않으면 다음 사람이 같은 붕괴를 처음부터 다시 재야 한다.
  §18.1의 표가 만들어질 수 있었던 것도 누군가 그 세 값을 손으로 셌기 때문이다.
- **부르는 자리는 하나다** — 저장 직전(§18.5). 엔진 구현 안에서 부르지 않는다.
  §13의 되돌리기 경계가 그대로 유지되어야 하기 때문이다: 엔진을 A로 바꿔도 이 규칙은
  바뀌지 않는다.

### 18.4 걸렸을 때 어떤 §13 실패가 되는가

```text
FailureKind::TranscriptionOutputUnusable
```

| | | 근거 |
| --- | --- | --- |
| **왜 이것인가** | 엔진은 정상적으로 끝났고 출력도 있다. 그 출력을 **전사 결과로 쓸 수 없다**는 것이 이 종류가 이미 뜻하는 바다 | [E1] `src-tauri/src/domain/failure.rs`의 주석 — *"엔진은 끝났지만 출력이 없거나 그 출력을 전사 결과로 해석할 수 없다"* |
| **§13과의 관계** | PRODUCT-SPEC §13 표의 전사 줄은 셋이고(`transcription process failure` · `unsupported whisper model` · `모델 파일 없음`), 이 저장소는 **넷째**로 이 종류를 두었다. §16.1이 그것을 "§13의 네 가지 실패"로 적었다 | [E1] `transcription/engine.rs` |
| **새 `FailureKind`를 만들지 않는다** | `FailureKind`는 `src/ipc/failure.ts`의 union과 1:1이다. 넓히면 frontend가 모르는 종류가 조용히 생긴다 — §16.2가 오디오 입력 실패에 대해 이미 같은 판단을 했다 | [E1] · §16.2 |
| **어떻게 만드는가** | 이미 있는 `engine::output_unusable(...)`을 그대로 쓴다. **새 실패 생성 경로를 만들지 않는다** | [E1] `transcription/engine.rs` |
| **세 질문에 대한 답** | *무엇이 실패했는가* — 전사가 붕괴했다(수치와 함께). *원본은 안전한가* — 안전하다(`source_data_safe = true`). *다시 시도할 수 있는가* — `Failure::permanent`이므로 `retryable = false`다: **같은 오디오를 같은 조건으로 다시 돌리면 같은 결과다.** 사용자가 바꿔야 하는 것은 입력(녹음 레벨)이거나 조건(모델 · 언어)이다 | [E1] `domain/failure.rs`의 `permanent` |

**사용자가 읽는 문장은 그 종류를 다시 뭉개지 않는다.** 모델 없음 · 모델 사용 불가 ·
엔진 실패 · 출력을 쓸 수 없음은 **사용자가 할 수 있는 일이 전부 다르기 때문에** 나뉘어 있다
(`failure.rs`의 주석이 그 이유를 적고 있다). 붕괴 실패의 안내 문장을 어디에 둘 것인가는
표현의 문제이며, 이 ADR이 정하는 것은 **어떤 종류로 도달하는가**까지다.

### 18.5 이 판정은 **저장을 막는 자리**에 있다 — 저장된 것을 고치는 자리가 아니다 (INV-2)

```text
파생 입력 → 엔진 → 정규화 → ┌─ 빈 결과인가        (이미 있는 판정)
                            └─ 붕괴인가          (§18.2 — 여기 더한다)
                                    │
                     쓸 수 있다 ────┴──── 붕괴
                          │                 │
                          ▼                 ▼
            새 Transcript를 추가하고    Transcript를 추가하지 않는다
            current를 그것으로 옮긴다   current_transcript_id를 바꾸지 않는다
                                      transcription_status = failed
```

- **Transcript는 immutable하다 (§7.1 · INV-2).** 그래서 이 판정은 **저장 직전**에 서 있다 —
  한 번 저장되면 지우거나 고치는 것이 이 제품의 규칙에 없기 때문이다.
  `run.rs`의 빈 결과 검사가 이미 그 자리에 서 있고 주석이 같은 이유를 적고 있다 [E1] —
  *"빈 Transcript가 한 번 저장되면 immutable하게 남기 때문이다 (INV-2) — 저장 직전이
  그것을 막을 수 있는 마지막 자리다."*
- **붕괴 판정은 그 검사를 대체하지 않고 더해진다.** 빈 결과(n == 0)의 판정은 그대로 남는다.
- **이미 저장된 붕괴한 Transcript를 지우지도 고치지도 않는다.** 2026-09-07에 저장된
  103개짜리 Transcript는 그대로 남는다. 그것은 그 전사가 어떤 조건에서 만들어졌는지의
  기록이기도 하다(§17.1.4-2와 같은 이유).
- **원본 오디오 · Recording 레코드 · 이미 있던 Transcript는 어떤 경우에도 그대로다**
  (INV-1 · INV-3). 붕괴는 실패지만, 실패 경로가 원본을 건드리지 않는다는 §9.3의 규칙 안에 있다.
- **판정을 화면 쪽으로 옮기지 않는다.** 화면에서 걸러 내면 DB에는 붕괴한 Transcript가
  `done`으로 남는다 — 그것이 §18.1이 기록한 바로 그 상태다.

### 18.6 이 규칙이 **하지 않는 것** — 이 Phase 밖이며 다음 Phase 후보다

| 하지 않는 것 | 왜 | 어디로 |
| --- | --- | --- |
| **청크 분할** · **청크마다 state 재생성** · **연속 반복 차단** | 2026-09-05의 실험이 이 셋의 효과를 관측한 것은 사실이다 [E5 · 부록 결과 2]. 그러나 **2026-09-07의 붕괴 원인은 확인되지 않았다** — 유력한 설명은 입력 레벨이며(ADR-0003 §16), 그것이 사실이라면 9/4의 처방은 이번 실패에 대한 처방이 아니다. **원인을 확인하기 전에 처방을 옮겨 오지 않는다** | 다음 Phase 후보 |
| **자동 언어 감지 개선** | 2026-09-07 1회차의 자동 감지는 확신도 0.478로 `en`을 골랐다 [E6 · `05.7` R-1]. 기록해 두지만, **설정으로 언어를 지정하는 경로가 동작하므로 사람에게 막힌 길이 아니다** | 다음 Phase 후보 |
| **VAD (무음 구간 처리)** | §17.4가 이미 같은 자리에 남겨 둔 항목이다. 이번 붕괴는 무음 구간이 아니라 **전 구간**에서 일어났으므로 이 실패의 처방으로 세울 수 없다 | 다음 Phase 후보 (§17.4) |
| **붕괴한 텍스트를 고치는 것** | 이 규칙은 **쓸 수 있는지 없는지만** 말한다. 반복을 걷어 내거나 문장을 다듬어 "구제된 Transcript"를 만들지 않는다 — 그것은 저장된 것을 고치는 일이고 INV-2가 막는다 | — |
| **임계값을 사용자 설정으로 만드는 것** | 근거가 세 관측뿐이다. 지금 설정 항목을 만들면 **아무도 무엇으로 바꿔야 하는지 모르는 손잡이**가 생긴다 (§20.6 — 추상화를 선입금하지 않는다). 틀렸다는 증거가 나오면 고칠 자리는 한 모듈의 상수 셋이다 | — |
| **원인을 단정하는 것** | 이 규칙은 붕괴를 **알아보는** 규칙이지 붕괴의 **원인을 아는** 규칙이 아니다. 같은 판정이 낮은 입력 레벨 · 잘못된 언어 · 부족한 모델 어느 쪽에서도 나올 수 있다 | ADR-0003 §16 · `05.7` 성공 기준 3 |
| **모델 크기 판정** | §17.4가 정한 그대로다 — 사람의 귀가 필요하다 | Human Review |

### 18.7 [관측된 사실] · [유력한 설명] · [미검증]

| 구분 | 내용 | 근거 |
| --- | --- | --- |
| **[관측된 사실]** | §18.1의 세 실행 값 · segment 경계가 정확히 30초 · 반복 문장이 2026-09-05에는 무음 10분에서만 나왔고 2026-09-07에는 51분 전체에서 나왔다 · 붕괴한 결과가 `done`으로 저장됐다 | [E5] · [E6] |
| **[관측된 사실]** | 저장 직전 검사가 `segments.is_empty()` 하나이며, 그 주석의 목적은 **빈** Transcript를 막는 것이다 · `TranscriptionOutputUnusable`이 이미 존재하고 `Failure::permanent`로 만들어진다 | [E1] — 이 Run이 파일을 직접 읽었다 |
| **[유력한 설명]** | 2026-09-07의 붕괴는 **입력 레벨이 낮아** whisper가 음성을 찾지 못하고 빈 자리를 학습 데이터의 잔재로 채운 결과다. 붕괴가 무음 구간이 아니라 전 구간에서 일어났다는 것이 이 설명과 맞는다 | `05.7` R-3 · ADR-0003 §16 |
| **[미검증]** | **레벨이 유일한 원인인지, 그리고 레벨을 올리면 이 오디오가 구제되는지는 확인하지 않았다.** Phase 5.7의 성공 기준 3이 그 확인이다. **확인 전에 원인을 단정해 적지 않는다** | `05.7` R-3 |
| **[미검증]** | 이 임계값들이 **다른 오디오 · 다른 모델 · 다른 언어**에서도 두 부류를 가르는가. 근거는 세 관측(정상 하나 · 붕괴 둘)뿐이며, 전부 **한 기기 · 한 사람 · 두 개의 녹음**에서 나왔다 | 이 절 |
| **[미검증]** | 두 조건(고유 비율 · 최다 반복 점유율) 중 **어느 쪽이 필요한가.** 관측된 두 붕괴는 둘 다 두 조건에 함께 걸린다 (§18.2) | 이 절 |
| **[미검증]** | **정상적이지만 반복이 많은 짧은 대화**가 이 규칙에 걸리는가. 최소 segment 수 20이 그것을 막으려는 값이지만, 그 값을 정당화하는 관측은 없다 | 이 절 |
| **[미검증]** | 이 판정이 붙은 뒤 사람이 **화면에서 무엇을 해야 할지 알 수 있는가** — 자동 Gate가 판정할 수 없는 Human Review 항목이다 | `05.7` Human Review |

### 18.8 이 갱신이 바꾼 것과 바꾸지 않은 것

**바꾸지 않은 것** — §2의 아홉 결정, §17.1의 언어 결정, §17.2의 Metal 결정은 하나도
철회되거나 수정되지 않았다. §17의 기록은 **한 글자도 지우거나 다시 쓰지 않았다.**
§13의 되돌리기 경로도 그대로다 — 붕괴 판정은 엔진 밖의 순수 모듈이므로, 엔진을 A로
되돌려도 이 규칙은 따라 움직이지 않는다. 경계가 넓어지지 않았다.

| 언제 | 무엇을 | 왜 |
| --- | --- | --- |
| 2026-09-07 (TASK-076) | **§18을 추가** — 붕괴 판정 규칙(임계값 셋 · 최소 개수 · 사는 자리 · §13 실패 종류 · 저장을 막는 위치) | 정한 적이 없어서 저장 직전 검사가 개수만 봤고, 103개짜리 붕괴가 `done`으로 저장됐다 [E6] |
| 2026-09-07 (TASK-076) | **§3에 [E6] 표기를 추가** | 2026-09-07 관측을 2026-09-05 관측([E5])이나 저장소 확인([E1])과 섞지 않기 위해서다 |
| 2026-09-07 (TASK-076) | **머리말의 Status · Date · Phase · Task · Scope에 이 갱신을 덧붙였다** | §17이 한 것과 같은 형태다. 원래 줄을 지우지 않았다 |

**이 Task는 문서만 바꿨다.** `src-tauri/src/transcription/` 아래의 어떤 파일도,
`Cargo.toml`도, 설정도, 테스트도 건드리지 않았다. §18이 정한 것은 **무엇을 붕괴로 볼
것인가**이며, 그것을 계산하는 코드와 저장을 막는 코드는 Phase 5.7의 다른 Task가 쓴다.
그 Task가 실제 값과 이 절이 적은 것이 다르면 §16.2·§17.2.2가 한 것처럼 **여기에 되적는다.**

---

## 19. §17의 구현 결과 — 실제로 만들어진 것 / 달라진 것 / 여전히 사람이 판정하는 것

```text
갱신:  2026-09-07 · TASK-075 (문서 전용 — 소스 · 설정 · 의존성 · 테스트를 바꾸지 않는다)
Phase: 5.6 — Transcription Correctness + Reach
범위:  TASK-067 ~ TASK-070이 만든 것을 §17의 두 결정과 대조한다
       (내보낸 파일 도달은 ADR-0009 §16, AI Handoff 크기는 ADR-0010 §12.8이 맡는다)
```

**§16이 §2에 대해 한 것을 이 절이 §17에 대해 한다.** §17은 *무엇을 부를 것인가*를 정하고
*부르는 코드는 이 Task가 쓰지 않는다*고 적었다 (§17.5). 그 코드가 쓰였고, 여기 적는 것은
**무엇이 실제로 만들어졌는가**다.

> **여기 적힌 "구현됐다"는 §16과 같은 뜻이다** — 코드가 존재하고 자동 검증이 그것을 지난다는
> 뜻이지, **사람이 읽을 만한 전사를 봤다는 뜻이 아니다** (§19.5).

### 19.1 언어가 어디서 읽혀 어디까지 도달하는가

**설정에서 읽는 자리 하나, 엔진 호출로 옮기는 자리 하나** — 그 사이의 모듈들은 값을 나르기만
한다. §17.1.4-4가 요구한 *"이 결정은 `transcription::whisper` 안에서 끝난다"* 가 실현된 모양이다
[E1 · 이 Run이 파일을 직접 읽었다].

```text
settings 테이블의 transcription_language      migration version 9 (열 하나 · nullable)
        │                                     NULL = 아직 고르지 않았다 = 자동 감지
        ▼
domain::settings::Settings::transcription_language      DEFAULT는 None
        │                                                로캘을 짐작해 굳히는 코드가 없다
        ▼
commands/transcriber.rs::transcribe_one          ★ 설정을 읽는 자리 — 여기 하나다
        │   LanguageChoice::from_setting(...)      모델 값과 **같은 자리에서 한 번에** 읽는다
        ▼
transcription::run::transcribe(… , &LanguageChoice)     해석하지 않고 지나간다
        ▼
TranscriptionEngine::transcribe(… , &LanguageChoice)    trait 경계도 해석하지 않는다
        ▼
transcription/whisper.rs                          ★ 엔진 호출로 옮기는 자리 — 여기 하나다
    Chosen(code) → set_language(Some(code)) · set_detect_language(false)
    Detect       → set_language(None)       · set_detect_language(true)
    어느 쪽이든  → set_translate(false)                          (§2 · 번역이 아니라 전사다)
```

| §17이 정한 것 | 실현된 자리 | 확인 |
| --- | --- | --- |
| §17.1.4-1 **언어 파라미터를 건드리지 않는 코드 경로를 남기지 않는다** | `whisper.rs`의 `match language.chosen()` — 두 갈래 모두 `set_language`와 `set_detect_language`를 **둘 다** 부른다 | 갈래가 둘뿐이고 어느 쪽도 두 호출을 건너뛰지 않는다 [E1] |
| §17.1.4-1 **고르지 않음은 영어가 아니라 자동 감지다** | `LanguageChoice::from_setting` — `None`과 **공백만 있는 값**이 함께 `Detect`가 된다 | `engine.rs`의 그 함수 하나가 "비어 있음"을 언어로 해석하지 않는다 [E1] |
| §17.1.4-3 **엔진이 말하지 못하면 지어내지 않는다** | `whisper.rs`의 `get_lang_str(state.full_lang_id_from_state())` — 결과는 여전히 `Option`이고 넘겨받은 선택을 베껴 넣지 않는다 | `run.rs`의 `language: transcription.language`가 설정이 아니라 정규화된 엔진 출력에서 온다 [E1] |
| §17.1.4-4 **`whisper.rs`가 스스로 정책을 정하지 않는다** | 그 파일에 설정도 저장소도 없다 — 넘어오는 것은 `LanguageChoice` 하나다 | `whisper.rs`가 `db`도 `settings`도 `use`하지 않는다 [E1] |
| §17.1.5 **Spec §D의 `language` 설정이 실재한다** | migration 9 · `Settings` 필드 · payload · `src/ipc/types.ts` · `settingsView.ts`의 폼 · Settings 화면의 Transcription 구역 | 저장 → 재조회 왕복과 **다른 설정을 저장해도 지워지지 않는다**가 자동 테스트로 고정돼 있다 [E3] |

**시그니처는 컴파일러가 확인했다.** §17.1.4가 *"실제 호출 시그니처를 컴파일러로 확인해 여기
적힌 것과 다르면 되적는다"* 고 적었고, **다르지 않았다** — `FullParams::set_language(Option<&str>)`
와 `FullParams::set_detect_language(bool)`이 그대로다 [E1 · `whisper.rs`의 모듈 문서 ·
lint Gate가 이 호출을 실제로 컴파일한다].

### 19.2 Metal — 켜졌는가, 그리고 그 판정을 무엇으로 했는가

**켜졌다.** 판정 근거는 **manifest에 이름이 적혔다는 것이 아니라 빌드 산출물이 달라졌다는
것**이다 — §17.2.2-3이 요구한 그대로다.

```toml
before:  whisper-rs = "0.16"
after:   whisper-rs = { version = "0.16", features = ["metal"] }
```

| 무엇을 봤는가 | before (feature 없음) | after (`metal`) | 등급 |
| --- | --- | --- | --- |
| build.rs가 CMake에 넘긴 정의 | `GGML_METAL:BOOL=OFF` | **`ON`** (`GGML_METAL_EMBED_LIBRARY` · `GGML_METAL_NDEBUG`도 `ON`) | [E1] |
| 만들어진 정적 라이브러리 | `libggml-metal.a`가 **없다** | **생겼다** (1,681,096 B). `libggml.a` · `libwhisper.a`의 크기도 달라졌다 | [E1] |
| build.rs의 링크 지시 | `Metal` · `MetalKit` · `Foundation` · `ggml-metal` 줄이 **없다** | **넷 다 붙었다** | [E1] |
| test Gate가 **실제로 실행한** 바이너리 | — | 그 파일 안에 `ggml_metal_*` 심볼과 Metal framework 경로가 있다 | [E1] |
| 다른 가속 옵션 | `WHISPER_COREML` · `GGML_OPENMP` · `GGML_CUDA` · `GGML_VULKAN` = OFF | **그대로 OFF** — 값이 달라진 키는 `GGML_METAL*` 뿐이다 | [E1] |
| `GGML_BLAS` · `GGML_ACCELERATE` | ON | ON — **이 Task가 켠 것이 아니라 원래부터 켜져 있었다** | [E1] |

원자료: `.loop/evidence/TASK-069/metal-feature-verification.md` ·
`before-metal-off.txt` · `after-metal-on.txt` · `declared-features.txt`.

**§17.2.2가 UNVERIFIED로 남긴 "정확한 feature 이름"은 좁혀졌다 — 그러나 출처가 crate 소스는
아니다.** 그 구분을 지우지 않고 적는다.

```text
확인된 것   whisper-rs 0.16.0 · whisper-rs-sys 0.15.0 두 crate 모두에 `metal` 이라는
            feature 이름이 선언돼 있고, Metal 관련 이름은 그 목록에 그것 하나다.
            coreml · openmp 는 별개 이름으로 존재하며 켜지지 않았다.

출처        cargo 가 crate manifest 를 파싱해 fingerprint 에 남긴 `declared_features`
            (src-tauri/target/**/.fingerprint/*.json)
            ← crate 의 Cargo.toml **파일 자체를 연 것이 아니다**

여전히 못 읽은 것   registry 에 추출된 crate 소스 · docs.rs · Codeberg
                    (작업 디렉터리 밖 접근 없음 · 네트워크 없음 — §17.2.2와 같은 제약)
```

그래서 §3의 표기 체계에서 이 항목을 [E1]으로 **올리지 않는다.** 확인된 것은
*"cargo가 파싱해 남긴 목록에 그 철자가 있다"* 이며, 그것과 *"crate 소스에서 읽었다"* 는 다른
진술이다 (PRODUCT-SPEC §20.2). §14의 해당 줄도 이 성격 그대로 갱신한다.

**`Cargo.lock`은 바뀌지 않았고, 그것이 정상이다.** §17.2.2-3은 ADR-0009 §11.4의 규칙
(*"켰다고 적어 두고 lock이 그대로면 켜지지 않은 것이다"*)을 인용했는데, **그 규칙은
`ureq`의 `rustls`처럼 feature가 새 crate를 들여오는 경우의 판정이었다.** `metal`은 새 crate를
끌어오지 않고 `whisper-rs-sys`로 전파되는 빌드 스위치이므로 lock이 바이트 단위로 같다.
**규칙의 태도는 지켰다** — 판정을 lock이 아니라 위 표의 산출물에 걸었다.

**속도는 이 저장소가 여전히 측정한 적이 없다** (§17.2.3 그대로). 배수도 소요 시간 추정치도
이 절에 없다. 켠 근거는 **측정된 속도가 아니라 Spec §14.4와 저장소 사이의 간극**이다.
체감 차이의 판정은 사람의 몫이며 절차는 `docs/PHASE-5.6-HUMAN-REVIEW.md` HR-3이다.

**§4.3 · §17.2.4가 적어 둔 대가는 실제로 발생했다** — feature를 바꾼 뒤 whisper.cpp가 다시
컴파일됐다 (`libggml-metal.a`의 생성과 두 라이브러리의 크기 변화가 그 관측이다).
**Gate timeout 900초를 넘지 않았다** [E1 · `.loop/evidence/TASK-069/after-metal-on.txt` §9 —
재컴파일을 포함한 실행이 `lint 20.1s` · `test 55.0s`로 기록돼 있다]. 그 값은 **이 기기의
그 실행에 대한 것**이며 빈 registry · 빈 `target/` · release 빌드에 대해서는 아무 말도 하지
않는다 (§4.3이 이미 단 단서 그대로다).

### 19.3 전사에 걸린 시간이 어디에 남는가

§17.2.3이 *"이 ADR은 숫자를 적는 대신 그 기록 경로가 생겨야 한다고 적는다"* 로 남긴 자리다.
그 경로가 생겼다 [E1].

```text
transcription/run.rs::attempt          ★ 재는 자리 — 여기 하나다
    let started = Instant::now();      단조 시계. 벽시계 두 번을 빼지 않는다
    …모델 해석 → 오디오 읽기 → 엔진 → 정규화…
    let transcription_ms = elapsed_ms(started);      영속화는 세지 않는다
        │
        ▼  Transcript.transcription_ms: Option<i64>
db/migrations.rs   version 10   ALTER TABLE transcripts ADD COLUMN transcription_ms INTEGER
        │                       NOT NULL 도 DEFAULT 도 없다
        ▼
commands/payload.rs   transcriptionMs (수치) + transcriptionLabel (사람이 읽는 문장)
        │             ★ 문장을 만드는 것은 Rust다 — 화면이 밀리초를 나누지 않는다
        ▼
src/screens/transcriptView.ts → Transcript 탭의 provenance 자리 (language · engine · model 옆)
```

| 규칙 | 어떻게 지켜졌는가 |
| --- | --- |
| **재는 자리는 한 곳** | `attempt`의 `Instant::now()` 하나이며, 자동 테스트가 원문에서 그 횟수를 센다 [E3] |
| **단조 시계** | `Instant`. 시스템 시각이 조정돼도 값이 뒤로 가지 않는다 — 음수 소요 시간이 저장될 수 없다 |
| **재는 구간** | 오디오를 문장으로 옮기는 일까지다. 저장이 느린 것은 전사가 느린 것이 아니다 |
| **값이 없는 것을 0이라고 말하지 않는다** | 열은 nullable이고 `DEFAULT 0`이 **없다.** 이 열이 생기기 전에 저장된 Transcript는 NULL이며 화면은 **그 줄을 그리지 않는다** — 재지 않은 전사를 `0:00`이라고 말하면 이 값으로 하려던 비교 자체가 거짓이 된다 |
| **이미 적용된 migration을 고치지 않는다** | version 2의 `transcripts`는 그대로 두고 목록 끝에 version 10을 붙였다. Transcript는 immutable이므로(§7.1 · INV-2) 옛 행에 나중에 채워 넣는 경로도 만들지 않았다 |

**실패한 전사의 시간은 남지 않는다.** 이 값은 Transcript와 함께만 존재하고, 실패 경로는
Transcript를 만들지 않는다. **Phase 5.7의 붕괴 판정이 그 성질을 그대로 물려받는다** — 붕괴로
막힌 전사도 Transcript를 남기지 않으므로 소요 시간도 남지 않는다 (§18.5). Metal 전후를
비교하려는 사람은 **성공한 전사끼리** 비교하게 된다.

**실시간 진행률은 만들지 않았다.** §17이 인용한 `phase-prompt/05.6`이 그것을 이 Phase의 성공
기준에서 제외했고, R-6의 관측(진행 중에 화면이 변하지 않는다)은 기록으로만 남아 있다.

### 19.4 §17과 달라진 것 — 그리고 왜

**결정 자체를 철회하거나 수정한 것은 없다.** 아래는 결정을 실현하는 과정에서 드러난 사실이다
(§16.2와 같은 성격이다).

| 무엇이 | §17이 적은 것 | 실제 | 왜 |
| --- | --- | --- | --- |
| **§17.1.4-2 — 감지 결과와 사용자 지정의 구분** | *"둘 다 `language`에 들어가지만 출처가 다르다 … Transcript에서 구분할 수 있어야 한다"* | **Transcript에 출처를 담는 값이 생기지 않았다.** 저장되는 것은 여전히 엔진이 보고한 코드 하나이며, 그 전사가 지정으로 만들어졌는지 감지로 만들어졌는지는 **그때의 설정을 함께 봐야** 알 수 있다 | 출처를 남기려면 `transcripts`에 열이 하나 더 필요하고, 그 값은 §17.1.4-3(엔진이 말한 것만 적는다)과 다른 축의 정보다. **구현이 그것을 만들지 않았고, 이 절은 만들어지지 않았다고 적는다** — 필요해지면 그때 열 하나를 더하는 일이다 (§20.6) |
| **`set_language(None)`을 부르는가** | §17.1.4는 감지 갈래에 `set_detect_language(true)`만 적었다 | 감지 갈래도 **`set_language(None)`을 함께 부른다** | §17.1.4-1의 *"언어 파라미터를 건드리지 않는 경로는 남기지 않는다"* 를 문자 그대로 지킨 결과다. 두 갈래가 같은 두 함수를 부르므로 "어느 쪽이 무엇을 건너뛰는가"를 읽는 사람이 세지 않아도 된다 |
| **빈 문자열의 취급** | §17은 정하지 않았다 | 공백만 있는 설정 값은 **고르지 않은 것과 같다**(`Detect`) | `model::resolve`가 모델 설정 값을 다루는 규칙과 같다. 사용자가 지우다 만 공백 하나가 언어 코드가 되지 않는다 |
| **Metal feature 이름의 등급** | §17.2.2-2는 *"확인된 이름을 [E1]로 되적고 UNVERIFIED 표시를 걷는다"* 를 요구했다 | **[E1]로 올리지 않았다.** 출처를 밝힌 별도 등급으로 남긴다 (§19.2) | crate 소스 파일을 연 것이 아니라 cargo가 그것을 읽고 남긴 기록을 열었다. **읽지 않은 것을 읽은 것처럼 적지 않는다** (PRODUCT-SPEC §20.2) |
| **`Cargo.lock`으로 판정하는가** | §17.2.2-3은 ADR-0009 §11.4의 lock 판정을 인용했다 | **lock이 아니라 빌드 산출물로 판정했다** — lock은 바뀌지 않았고 바뀔 것이 없었다 | §19.2의 마지막 문단. 규칙의 **태도**(적어 둔 것 말고 실제로 달라진 것을 본다)는 지켰고, 판정 **수단**만 이 feature의 성질에 맞게 바꿨다 |

### 19.5 `A-TRANS-001`은 이 Phase 뒤에 어떤 상태인가 — **여전히 사람의 실행으로만 닫힌다**

**§16.3.1과 §17.3을 지우지 않는다.** 여기 적는 것은 그 뒤의 상태다.

```text
2026-09-03   A-TRANS-001 수용 — 구현됐고 자동 검증을 통과했으나 추론이 한 번도 돌지 않았다
2026-09-05   첫 실행. 엔진 경로는 지났고 제품 결과는 쓸 수 없었다 (언어가 en으로 강제됐다)
2026-09-06   §17이 그 원인을 결정으로 바꿨다 (문서)
2026-09-07   Phase 5.6이 그 결정을 코드로 만들었다 (§19.1 · §19.2 · §19.3)
2026-09-07   두 번째 실행. **언어 수정은 동작했다** — 설정한 `ko`가 엔진에 도달했다.
             그러나 결과는 다시 붕괴했고, 원인은 언어가 아니었다 (§18)
```

**이 Phase가 한 것과 하지 않은 것을 섞지 않는다.**

| | |
| --- | --- |
| **Phase 5.6이 한 것** | 한국어가 **영어로 강제되지 않게** 했다. Spec §D가 요구하던 `language` 설정을 실재하게 했다. Spec §14.4가 적은 대로 Metal을 켰다. 전사에 걸린 시간을 **비교할 수 있는 기록으로** 남겼다 |
| **Phase 5.6이 하지 않은 것** | **읽을 만한 전사를 내는 것.** 자동 검증은 언제나 `StubEngine`으로 돌며 (PRODUCT-SPEC §18), **stub은 한국어가 한국어로 들렸는지를 알 수 없다** — §17.3이 기록한 Gate의 경계가 그대로다 |
| **그래서 `A-TRANS-001`은** | **열려 있다.** 2026-09-05의 실행은 이 Phase가 고친 결함을 드러냈고, 2026-09-07의 실행은 **그 아래에 있던 다른 결함**을 드러냈다. 두 번 다 사람이 읽을 수 있는 전사는 나오지 않았다 |

```text
이 가정이 닫히는 조건 — 하나다

  사람이 실제 회의를 녹음하고, 제품 경로로 전사한 결과가
  **읽을 만하다고 사람이 판정하는 것**

닫는 주체   이 절이 아니다. docs/PHASE-5.6-HUMAN-REVIEW.md §8이 채워지고
            docs/PHASE-5.7-HUMAN-REVIEW.md §8.4가 채워진 뒤의 Task가
            §16.3.1과 SYSTEM-MAP §7을 함께 고친다
```

**모델 크기 판정도 그대로 열려 있다** (§17.4). 이 Phase는 *여러 모델을 쓸 수 있게* 만들었을
뿐이며, **`ggml-base`로 충분한지도 `large-v3`가 필요한지도 이 문서에 적지 않는다.**
그 질문은 `docs/PHASE-5.6-HUMAN-REVIEW.md` HR-2다.

### 19.6 이 갱신이 바꾼 것과 바꾸지 않은 것

**바꾸지 않은 것** — §2의 아홉 결정, §17.1의 언어 결정, §17.2의 Metal 결정, §18의 붕괴 판정
규칙은 하나도 철회되거나 수정되지 않았다. **§1~§18의 어떤 문단도 지우지 않았다.**
§13의 되돌리기 경로도 그대로다 — §17의 두 결정은 각각 `transcription::whisper` 하나와
`Cargo.toml` 한 줄에서 끝났고, 전사 소요 시간은 엔진을 모르는 값 하나다. 경계가 넓어지지 않았다.

| 언제 | 무엇을 | 왜 |
| --- | --- | --- |
| 2026-09-07 (TASK-075) | **§19를 추가** — §17 두 결정의 구현 결과 · 전사 소요 시간의 기록 경로 · 계획과 달라진 다섯 · `A-TRANS-001`의 상태 | §17이 *"부르는 코드는 이 Task가 쓰지 않는다"* 로 남긴 자리가 채워졌고, **무엇이 실제로 만들어졌는지를 적는 자리가 §16처럼 필요했다** |
| 2026-09-07 (TASK-075) | **머리말의 Status · Date · Phase · Task · Scope와 읽기 안내에 이 갱신을 덧붙였다** | §17 · §18이 한 것과 같은 형태다. 원래 줄을 지우지 않았다 |

**이 Task도 문서만 바꿨다.** `src-tauri/` 아래의 어떤 파일도, `Cargo.toml`도, 설정도, 테스트도
건드리지 않았다. 이 절이 적은 것은 **이미 저장소에 있는 코드를 읽은 결과**이며, 코드와 이 절이
어긋난다면 **코드 쪽이 사실이고 이 절을 고쳐야 한다.**

---

## 20. 전사를 어디서 자르는가 — 청크 분할 · state 재생성 · 연속 반복 차단

```text
갱신:  2026-09-07 · TASK-086 (문서 전용 — 소스 · 설정 · 의존성 · 테스트를 바꾸지 않는다)
Phase: 5.8 — Transcription Chunking
근거:  2026-09-05 실험 [E5] — docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md 부록 결과 2 ·
       phase-prompt/05.8-transcription-chunking.md P-2 · P-3
       저장소 파일 직접 확인 [E1] — transcription/{whisper,parse,collapse,audio_input,engine}.rs
```

> **번호 주의 — 이 문서의 §2 · §11 · §18.6이 괄호로 인용한 "(§20.6 — 추상화를 선입금하지
> 않는다)"와 §19.4의 "(§20.6)" · §17.2.2의 "(PRODUCT-SPEC §20.2)"는 이 절이 아니다.**
> 그것들은 `docs/PRODUCT-SPEC.md` §20을 가리킨다. **그 문장들을 고치지 않았다.** 이 절은
> 이 ADR의 스무 번째 절이며, 이 절을 가리킬 때는 "ADR-0007 §20"으로 적는다.

**이 절은 §1~§19를 대체하지 않는다.** §18은 *무엇을 붕괴로 볼 것인가*를 정했고 그 규칙은
`collapse.rs`로 실재한다. 여기 적는 것은 그다음 질문이다 — **붕괴를 줄이려고 무엇을 할
것인가.**

§18이 안전망이라면 §20은 처방이다. **그리고 처방은 안전망을 지우지 않는다** (§20.6).

### 20.1 이 절이 값을 가져오는 단 하나의 실험 — 그 기록이 말한 것과 말하지 않은 것

**[관측된 사실]** 2026-09-05에 운영자가 **저장소 밖의 검증용 도구**로 같은 오디오
(`capture-1788522158.wav` · 72.85분 · 한국어 3인 회의)를 다시 전사했다 [E5].

```text
변경 1   params.set_language(Some("ko"))                    ← 제품에 있다 (§17.1 · §19.1)
변경 2   whisper-rs features = ["metal"]                    ← 제품에 있다 (§17.2 · §19.2)
변경 3   120초 청크 분할 + 청크마다 state 재생성 + 연속 반복 3회 초과 차단   ← **제품에 없다**
모델     ggml-large-v3-turbo.bin

결과     6.0분 · segment 1,749 · 고유 1,643 (94.0%) · 최다 반복 10회 · 한글 정상
```

같은 오디오에서 **청크 분할만 없는 조건**(`ko` + Metal + `large-v3-turbo`)은 6.1분에
*"반복 구간 2곳 잔존"* 으로 기록됐다 [E5 · 부록 결과 2의 기여도 표]. **그것은 수치가 아니라
사람의 서술이며**, 그 실행의 고유 문장 비율은 세어진 적이 없다.

**그 기록이 말하지 않는 것 — 이 절은 그 자리를 지어내지 않는다:**

| 말하지 않은 것 | 이 절이 하는 일 |
| --- | --- |
| **겹침(overlap)을 두었는가** | 기록에 **명시가 없다.** "120초 청크 분할"이 전부다. §20.4가 이 사실을 근거의 일부로 적는다 |
| 세 변경 중 무엇이 얼마나 기여했는가 | 분리되지 않았다. §20.3 · §20.10이 [미검증]으로 적는다 |
| 청크 길이 120초가 다른 값보다 나은가 | 60초도 180초도 측정된 적이 없다 (§20.2) |
| "연속 3회 초과 차단"의 문장 비교 규칙 | 기록에 없다. §20.6이 **이 저장소에 이미 있는 규칙**(`collapse::sentence_key`)을 쓴다고 정한다 |
| 차단이 §18의 붕괴 판정과 어떻게 만나는가 | 2026-09-05에는 `collapse.rs`가 없었다. §20.6이 순서를 정한다 |

**[관측된 사실]** 지금 제품은 오디오 전체를 한 번에 넘긴다 — `whisper.rs`가 전사 한 건에
state 하나를 만들고(`context.create_state()`) `state.full(params, &input.samples)`에
**전체 버퍼**를 넘긴다 [E1 · `src-tauri/src/transcription/whisper.rs`]. `transcription/`에
전사 청크는 없다 — `audio_input.rs`의 `chunk`는 **리샘플러 청크**이지 전사 청크가 아니다
(`05.8` P-3).

### 20.2 청크 길이 — **120초**

```text
CHUNK_SECONDS = 120            ← 이 절이 확정하는 값. 정수 초다
한 청크의 프레임 수 = 120 × 16,000 = 1,920,000
                       ^^^^^^  파생 입력은 언제나 16 kHz mono다 (§9 · audio_input.rs의
                               TARGET_SAMPLE_RATE_HZ) — 청킹 모듈은 그 값을 짐작하지 않는다

마지막 청크   남은 만큼이다. 짧다고 앞 청크에 합치지 않고, 무음으로 채우지도 않는다
전체가 120초 이하   청크는 하나다 — 지금과 **완전히 같은 실행**이 된다
```

**왜 120초인가 — 관측에서 온 값이지 추정이 아니다.**

| | |
| --- | --- |
| **근거** | 2026-09-05 실험이 **실제로 쓴 값**이며, 그 실행이 같은 오디오에서 고유 94.0%를 냈다 [E5 · `05.8` P-2] |
| **추정이 아니라는 뜻** | 이 값은 "30초 윈도우의 배수가 적당해 보인다" 같은 추론으로 고르지 않았다. **그 조건으로 72.85분 오디오 하나가 실제로 전사됐고 결과가 세어졌다** |
| **왜 다른 값을 고르지 않는가** | 고를 근거가 없다. 60초 · 180초 · 300초는 이 프로젝트가 **한 번도 재지 않았다.** 그리고 `05.8`이 *"측정되지 않은 처방을 함께 넣으면 무엇이 효과를 냈는지 다시 알 수 없게 된다"* 고 정했다 — 목표 수치(94.0%)가 **그 조건에서** 나온 값이므로, 조건을 바꾸면 비교 대상이 사라진다 |
| **참고 (근거가 아니다)** | 120초는 whisper의 처리 윈도우 30초(§18.1의 [E6] 관측 — segment 경계가 정확히 30초였다)의 4배다. **이것은 값의 성질이지 값을 고른 이유가 아니다** |

**초가 아니라 프레임으로 자르는 이유.** 실수 초로 나누면 청크마다 반올림이 쌓이고 경계가
어긋난다. `CHUNK_SECONDS`가 정수 초이므로 프레임 수도(× 16,000) 오프셋도(× 100 센티초)
정수로 떨어진다 — **나눗셈도 실수 연산도 없다** (§20.5).

### 20.3 청크마다 **state를 재생성한다** — 무엇을 재생성하고 무엇을 재사용하는가

```text
재생성한다   whisper state          청크 하나마다 새로 만든다
             (whisper-rs에서 WhisperContext::create_state()가 내는 값 —
              지금은 전사 한 건에 하나다 [E1 · whisper.rs])

재사용한다   모델 context           전사 한 건에 한 번 연다. 청크마다 다시 열지 않는다
             (적재된 가중치 · WhisperContext — §16.2가 "전사마다 연다"로 적은 그 자리)
```

**경계를 여기 두는 이유 — 무엇이 청크를 오염시킬 수 있는가로 가른다.**

| | 성질 | 그래서 |
| --- | --- | --- |
| **state** | 디코딩을 하는 동안 **값이 변한다.** 반복 루프에 빠진 디코딩의 흔적이 남는 자리도 여기다 | **끊는다.** 청크 하나의 붕괴가 다음 청크로 이어지지 않게 하는 것이 이 처방의 목적이다 |
| **모델 가중치** | 전사 내내 **변하지 않는다.** 읽기 전용이며 청크마다 달라질 것이 없다 | **잇는다.** 1.5 GB 모델을 청크 수만큼(72분이면 37번) 다시 읽는 비용이 붙고, 그 대가로 얻는 것이 없다 |

**[미검증] 이 처방이 `no_context = true` 위에 무엇을 더하는가.** whisper.cpp 기본값은 이미
이전 segment를 다음 디코딩의 prompt로 넘기지 않는다 (§17.1.3). state 재생성이 그 위에
무엇을 더 끊는지는 **이 저장소가 분리해 측정한 적이 없다** — 2026-09-05는 세 가지를 함께
바꿨다 [E5]. **효과가 있었다는 관측은 있고, 무엇의 효과인지는 모른다.**

**구현 Task가 확인할 것.** `create_state`의 실제 시그니처와 청크마다 state를 새로 만드는
비용은 **컴파일러와 실행이 말한다.** §17.1.4가 세운 규칙 그대로다 — 여기 적힌 것과 다르면
§16.2 · §19.4가 한 것처럼 **이 절에 되적는다.**

### 20.4 겹침(overlap) — **두지 않는다 (0초)**

**확정이다. "나중에 정한다"가 아니다.** 청크는 맞닿아 있고 겹치지 않는다 —
청크 k는 프레임 `[k·L, (k+1)·L)`이며 프레임 하나도 두 번 들어가지 않는다.

**세 가지 이유:**

1. **2026-09-05 기록에 겹침에 대한 명시가 없다** [E5 · `05.8` P-2 — 적힌 것은 "120초 청크
   분할 + 청크마다 state 재생성 + 연속 반복 3회 초과 차단"이 전부다]. 겹침을 두는 것은
   **관측되지 않은 조건을 하나 더 넣는 일**이고, 그러면 94.0%라는 목표 수치가 무엇의
   결과였는지 다시 알 수 없게 된다 (`05.8` — *"측정되지 않은 처방을 함께 넣지 않는다"*).
2. **겹침은 병합 규칙을 요구하는데, 그 규칙이 맞았는지 판정할 수단이 없다.** 겹친 구간에서
   두 청크가 낸 문장은 **글자가 다를 수 있다** — 경계 근처의 오디오는 양쪽에서 다르게
   들린다. 그러면 *무엇을 같은 문장으로 볼 것인가 · 어느 쪽을 남길 것인가 · timestamp를
   어느 쪽 것으로 할 것인가* 가 전부 새 규칙이 되고, 자동 검사가 확인할 수 있는 것은
   "중복이 남지 않았다"까지다. **"옳은 쪽이 남았다"는 사람만 판정한다.**
3. **`parse`가 이미 겹침을 다루는 규칙을 갖고 있고, 그 규칙과 부딪힌다.** `parse.rs`는
   앞 segment와 겹치는 구간을 **자르지 않고** `AnomalyKind::Overlap`으로 남긴다 —
   *"겹침은 엔진의 chunk 경계에서 나온다. 잘라내면 없던 경계를 만든다"* [E1]. 겹침을
   도입하면 같은 현상을 다루는 규칙이 두 자리에 생긴다.

**대가는 감추지 않는다.** 겹치지 않으므로 **청크 경계에 걸친 문장은 두 조각으로 잘리거나,
경계의 한 마디가 어느 쪽에도 남지 않을 수 있다.** 그것이 실제로 얼마나 일어나는지는
**[미검증]** 이며, `05.8`의 Human Review 항목이 바로 그 질문이다 — *"청크 경계에서 문장이
잘리거나 중복되지 않는가"*.

**나중에 겹침을 두기로 한다면 무엇을 적어야 하는가** (지금 적지 않는 것):
겹침 길이 · 무엇을 중복으로 볼 것인가 · 겹친 구간에서 어느 쪽 segment를 남기는가 ·
남긴 segment의 timestamp를 어느 청크 기준으로 잡는가, 그리고 **겹침 없는 실행과 비교한
관측**. 마지막 것이 없으면 이 절이 지금 하지 않는 일을 그때도 하지 못한다.

### 20.5 청크 로컬 timestamp를 전체 시간축으로 되돌린다 — **원시 단위(센티초)에서 더한다**

```text
청크 k (0부터)   프레임 [k·L, min((k+1)·L, n))          L = 120 × 16,000 = 1,920,000
오프셋            offset_cs(k) = k × 12,000  [센티초]     (120초 = 12,000 센티초 · 정수다)

되돌리기          청크가 낸 원시 timestamp(센티초) + offset_cs(k)  →  **여전히 원시 센티초**
```

**어느 값이 원시이고 어느 값이 정규화된 값인가 — 이것이 흐려지면 §10이 깨진다.**

| 값 | 단위 | 성질 | 어디에 있는가 |
| --- | --- | --- | --- |
| 청크가 낸 segment의 start/end | **센티초** | **원시** | 엔진이 청크마다 낸 값 |
| 청크 오프셋 `offset_cs(k)` | **센티초** | **원시** | 청킹 모듈이 청크 번호로 계산한다 |
| 둘의 합 | **센티초** | **원시** | `RawSegment::start_centiseconds` · `end_centiseconds` — 타입이 이미 "센티초"라고 적고 있다 [E1 · `parse.rs`] |
| `start_ms` · `end_ms` | 밀리초 | **정규화됨** | `parse.rs`의 `MILLISECONDS_PER_CENTISECOND = 10` — **여전히 단위 변환이 일어나는 유일한 자리다** |

**§10을 왜 깨지 않는가 — 세 문장이다.**

1. **같은 단위끼리의 덧셈은 단위 변환이 아니다.** 센티초 + 센티초 = 센티초다. §10이 한
   자리로 묶은 것은 *센티초 → 밀리초*이며, 그 변환은 여전히 `parse.rs`에서 한 번만 일어난다.
2. **trait 밖으로 나오는 값의 성질이 바뀌지 않는다.** 지금도 `RawTranscription`은 원시
   센티초를 담고, 청킹이 들어와도 그대로다. `parse`가 받는 것은 **전체 시간축의 원시 센티초**
   한 열이며, 청크가 몇 개였는지 `parse`는 알지 않는다.
3. **`× 10`을 두 번 하는 경로가 생기지 않는다.** 오프셋이 밀리초로 계산되는 자리는 어디에도
   없다. 청킹 모듈은 **밀리초라는 단위를 알지 않는다.**

**샘플 인덱스 → 시각 변환은 여기서 한 번 일어나고, 그것은 §10이 다루는 변환이 아니다.**
§10이 묶은 것은 *엔진이 낸 시각의 단위*이고, 여기 있는 것은 *오디오 위치를 시각으로
말하는 것*이다. 그 계산은 **청킹 모듈 한 곳**에서만 하며, `CHUNK_SECONDS`가 정수 초이고
sample rate가 16,000이므로 `1,920,000 프레임 × 100 / 16,000 = 12,000 센티초`가 **나머지 없이**
떨어진다. 단위 테스트가 이 정수 관계를 고정한다 (§20.8).

**넘침은 조용히 접지 않는다.** `parse.rs`는 이미 `× 10`에서 `i64` 넘침을 `Failure`로 만든다
(*"조용히 saturate하면 틀린 시각이 영구히 저장된다 (INV-2)"* [E1]). 오프셋은 그 앞에서
더해지므로 그 검사가 합쳐진 값을 그대로 본다. 청킹 모듈도 같은 태도를 지킨다 —
**표현할 수 없는 값을 saturate해서 그럴듯하게 만들지 않는다.**

**엔진 경계의 문서 주석 하나는 구현 Task가 손봐야 한다.** `engine.rs`의 trait 주석은
*"구현은 값을 옮기기만 한다 — 계산하지 않는다"* 라고 적고, **바로 이어서 그 계산이
timestamp 단위 변환을 뜻한다고 적는다** [E1]. 오프셋 덧셈은 단위를 바꾸지 않으므로 그
규칙을 어기지 않지만, **"청크로 나눠 부르고 오프셋을 더해 이어 붙인다"는 사실은 그 주석에
남아야 한다.** trait의 **형태**(시그니처)는 바뀌지 않는다 (§20.8).

### 20.6 연속 반복 차단 — 정의와 임계값

#### 20.6.1 무엇을 "연속 반복"으로 세는가

```text
문장     collapse::sentence_key 와 **같은 정규화**다 — 앞뒤 공백을 걷고 내부의 연속 공백을
         한 칸으로 줄인다. 대소문자와 문장부호는 건드리지 않는다 [E1 · collapse.rs]
         **두 번째 정의를 만들지 않는다.** 그 함수를 그대로 부른다 (§18.2와 같은 규칙이다)

연속     이어진 segment들의 문장이 같으면 한 묶음이다.
         다른 문장이 하나라도 끼면 묶음이 끊기고 세는 값이 1로 돌아간다.
         **떨어져서 다시 나오는 같은 문장은 세지 않는다** — 그것은 반복이지 연속이 아니다

임계값   MAX_CONSECUTIVE_REPEATS = 3

차단     한 묶음에서 **4번째 segment부터** 버린다. 앞의 3개는 남는다.
         2026-09-05가 적은 "연속 3회 초과 차단"이 뜻하는 것이 이것이다 —
         **3회까지 통과 · 4회째부터 차단** [E5]
```

**임계값은 코드의 한 자리에만 있다.** `collapse.rs`가 임계값 셋을 상수로 두고 **그 자리가
하나라는 것을 테스트가 원문에서 확인하는** 선례를 그대로 따른다 [E1]. `whisper.rs`에도
`run.rs`에도 화면에도 이 숫자가 나타나지 않는다 — 두 자리에 있으면 한쪽만 고쳐지는 날이 온다.

#### 20.6.2 어디에서 도는가 — **붕괴 판정 뒤다.** 그리고 그 순서가 중요하다

```text
parse가 낸 정규화된 segment 열   (전체 시간축 · 밀리초 · 차단 전)
        │
        ├─→ collapse::assess   ← §18의 붕괴 판정은 **이 열**로 한다 (차단 전)
        │                        임계값도 공식도 §18.2 그대로다. 이 절이 바꾸지 않는다
        │
        └─→ 판정을 통과하면, 연속 반복을 차단한 열을 저장한다
```

**왜 판정을 먼저 하는가 — 차단은 §18이 세는 증거를 정확히 지우기 때문이다.**

§18.1이 기록한 2026-09-05의 제품 경로 붕괴는 segment 1,711개 중 한 문장이 **1,063회**였다.
그 반복이 이어져 있었다면 차단 뒤에는 묶음마다 3개만 남는다 — **고유 비율이 올라가 §18.2의
20% 문턱을 넘어설 수 있다.** 즉 **차단을 판정보다 먼저 두면 붕괴한 전사가 통과한다.**
`05.8`이 *"청크 분할이 붕괴 판정을 대체하지 않는다 — 판정은 안전망으로 남는다"* 고 적은
것이 이것이며, 이 순서가 그 문장을 지키는 방법이다.

- **§18.2의 임계값 세 개와 문장 정규화 규칙을 이 절이 바꾸지 않는다.** 바꾼 것은 **어느 열을
  판정하는가**가 아니라 — 판정하는 열은 지금도 앞으로도 `parse`가 낸 열이다 — **차단이 그
  뒤에 온다는 것**이다.
- **지워진 개수를 잃지 않는다.** 사람에게 보이는 수치(n · u · r)는 판정에 쓴 열의 것이며,
  저장된 segment 수보다 클 수 있다. **그 차이가 차단으로 지워진 개수**이고, 그 값이 남지
  않으면 다음 사람이 두 수치의 차이를 설명하지 못한다 (`parse`가 `anomalies`를 남기는 것과
  같은 태도다).
- **[미검증] 거짓 양성.** 차단 전 열로 판정하면 사람이 읽을 만한 전사가 붕괴로 판정될 수
  있는가. 2026-09-05의 94.0%는 **차단이 켜진 채** 잰 값이므로(1,749 · 고유 1,643),
  **차단 전 값이 얼마였는지 아무도 모른다.** 이 위험이 실제로 나타나면 고칠 자리는 §18.2의
  임계값이지 **판정을 차단 뒤로 옮기는 것이 아니다** — 그러면 안전망이 사라진다.

#### 20.6.3 이 차단이 지불하는 대가 — **[미검증]**

**정상적으로 반복된 발화가 차단될 수 있다.** 같은 짧은 대답("네" · "맞아요")이 네 번 이상
연달아 나오는 회의는 실재하고, 그때 **4번째부터는 Transcript에 남지 않는다.** 원본 오디오는
그대로지만(INV-1) **저장된 Transcript는 immutable하므로**(INV-2) 그 문장이 나중에 돌아오는
경로는 없다 — 사람이 다시 전사해야 한다.

이 위험을 감수하는 근거는 **2026-09-05 관측 하나**뿐이다. 그 실행에서 최다 반복이 10회로
남았다는 것은 [E5] **떨어져 있는 반복은 지워지지 않았다**는 뜻이며(연속만 센다),
그 이상은 이 기록이 말하지 않는다. **연속 2회 · 5회와 비교한 측정은 없다.**

### 20.7 여러 청크를 하나의 `RawTranscription`으로 — `language` 보고 값

```text
청크마다   엔진이 language를 보고한다. 보고하지 못하면 None이다
           (RawTranscription::language: Option<String> — "엔진이 보고한 언어" [E1 · parse.rs])

합친 값    값을 보고한 **첫 청크**의 값. 아무 청크도 보고하지 않았으면 **None**
segment    청크 순서대로 이어 붙인다. 순서를 다시 정렬하지 않는다
           (`parse`의 `ordinal`은 "엔진이 낸 순서"이며 그 규칙이 그대로다 [E1])
```

**왜 첫 값인가.** 결정적이고, **엔진이 말한 적 없는 값을 만들지 않는다.** 다수결은
*엔진 중 누구도 말하지 않은 집계값*을 새로 만들고 동점 규칙까지 요구하는데, 그것을 정할
관측이 하나도 없다.

**§17.1.4-3은 그대로다 — 설정 값을 베껴 넣는 경로를 만들지 않는다.**

| | |
| --- | --- |
| **지키는 규칙** | 사용자가 `ko`를 골랐어도 **엔진이 아무 말도 하지 않으면 `language`는 비어 있다.** "감지를 켰다"가 "항상 값이 있다"는 뜻이 되지 않는 것과 같은 규칙이다 (§17.1.4-3) |
| **청킹이 그 규칙을 우회하지 않는 이유** | 청킹 모듈은 **설정을 보지 않는다** (§20.8). 합치는 함수에 들어오는 것은 청크마다의 `Option<String>`뿐이며, `LanguageChoice`도 `Settings`도 그 자리에 없다. 베껴 넣을 값 자체가 손에 없다 |
| **`Chosen(code)`일 때** | 청크마다 되읽는 값이 같으므로 첫 값이든 마지막 값이든 결과가 같다. **[유력한 설명]** — 엔진이 강제된 언어를 그대로 되읽기 때문이다 (§17.1.2가 관측한 그 경로다). **청크마다 되읽은 값을 비교한 관측은 없다** |
| **`Detect`일 때** | 청크마다 다른 언어를 감지할 수 있다. **첫 값이 이긴다** |

**청크들이 서로 다른 언어를 보고했다는 사실을 Transcript에 남기는 열을 만들지 않는다.**
§19.4가 언어의 *출처*(지정인가 감지인가)에 대해 내린 판단과 같다 — 필요해지면 그때 열
하나를 더하는 일이다 (PRODUCT-SPEC §20.6 — 추상화를 선입금하지 않는다).

### 20.8 이 규칙이 사는 자리 — 순수 모듈 하나

```text
파생 입력 (16 kHz mono f32 · 메모리)
        │
        │  ① 어디서 자르는가        plan(프레임 수) → 청크 구간 + 청크별 오프셋(센티초)
        ▼
청크마다:  state 재생성 → 엔진 호출 → 청크 로컬 원시 segment (센티초)
        │
        │  ② 어떻게 이어 붙이는가   merge(청크별 원시 출력) → RawTranscription 하나
        ▼                          (오프셋을 센티초에서 더한다 · language는 §20.7)
RawTranscription  ← **trait 밖으로 나오는 것은 지금과 같다. 하나다**
        │
        ▼  parse — 단위 변환은 여기 한 곳 (§10)
정규화된 segment 열
        │
        │  collapse::assess (§18 · 차단 전)  →  ③ 무엇을 차단하는가 (§20.6)
        ▼
저장되는 Transcript
```

- **①②③의 규칙은 파일시스템 · 저장소 · 네트워크 · 시계 · 엔진을 모르는 순수 모듈 하나에
  산다** (예: `src-tauri/src/transcription/chunking.rs`). `collapse.rs` · `parse.rs`가 세운
  선례 그대로이며, 프로세스 실행도 라이브러리 호출도 없으므로 **실제 whisper도 모델도 없이
  테스트된다** (PRODUCT-SPEC §18). 청크 경계도 오프셋도 반복 묶음도 **값으로 그대로 만들어
  낼 수 있다.**
- **오디오 샘플을 복사해 들고 있는 것은 이 모듈의 일이 아니다.** `plan`이 내는 것은
  **구간**이고, 그 구간으로 버퍼를 자르는 것은 부르는 쪽이다. 그래야 이 모듈이 오디오를
  모르는 채로 남는다.
- **`TranscriptionEngine` trait의 형태는 바뀌지 않는다** [`transcription/engine.rs:106` ·
  `05.8` Constraints]. 청킹은 **그 구현 안쪽**의 일이며, `testing::StubEngine`도 그대로다.
  §13의 되돌리기 경계도 그대로다 — 엔진을 A(sidecar)로 되돌려도 **교체 대상은 여전히 엔진
  구현 하나**이고, 이 순수 모듈은 그 구현이 부르는 자리에 남는다.
- **세 값은 이 모듈의 상수 셋이다** — `CHUNK_SECONDS = 120` · 겹침 없음(0) ·
  `MAX_CONSECUTIVE_REPEATS = 3`. `whisper.rs` · `run.rs` · payload · 화면 어디에도 복제되지
  않는다.
- **벤더 고유 개념을 core/domain · payload · frontend 타입에 새로 만들지 않는다** (INV-9).
  **청크는 화면에도 IPC에도 나타나지 않는다** — 밖에서 보면 전사 하나가 나올 뿐이다.
- **새 의존성을 들이지 않는다** — 2026-09-05 실험도 새 crate 없이 이 결과를 냈다 (`05.8`).

**단위 테스트가 고정해야 할 경계값** (구현 Task의 몫이며, 이 절은 무엇을 고정할지만 적는다):

```text
청크 개수    프레임 수가 L보다 작을 때 · 정확히 L일 때 · L+1일 때 · 2L일 때
마지막 청크  남은 길이 그대로다 · 무음으로 채우지 않는다 · 앞 청크에 합치지 않는다
오프셋       offset_cs(k) = 12,000 × k  — 12,000이 프레임 L에서 나머지 없이 나온다
             청크가 문장을 하나도 내지 못해도 **다음 청크의 오프셋이 밀리지 않는다**
차단         연속 3회는 남고 4회째부터 지워진다 · 다른 문장이 끼면 다시 1부터 센다 ·
             떨어져 있는 반복은 지워지지 않는다 · 공백만 다른 두 문장은 같은 문장이다
language     첫 값이 이긴다 · 아무도 보고하지 않으면 None이다 · 설정 값이 들어갈 자리가 없다
```

### 20.9 이 절이 **하지 않는 것**

| 하지 않는 것 | 왜 | 어디로 |
| --- | --- | --- |
| **엔진 교체 (Qwen3-ASR)** | 지금 갈아타면 결과가 좋아져도 **엔진 덕인지 청크 덕인지 구분할 수 없다.** 그 판단에는 제품 경로의 기준선이 필요하고, 이 Phase가 그 기준선을 만든다 | ADR-0008 · Phase 5.9 이후 (`05.8`) |
| **디코딩 파라미터 튜닝** — VAD · `no_speech_thold` · `entropy_thold` · `temperature` | 2026-09-05가 측정한 조건이 **아니다.** 그리고 whisper.cpp 기본값에 `no_context` · `temperature_inc` · `entropy_thold` · `logprob_thold`가 이미 켜져 있다 — 꺼져 있는 것이 아니다 (§17.1.3) | §17.4 · §18.6이 남긴 자리 그대로 |
| **모델 강제** | 어떤 모델을 쓸지는 설정에서 사람이 고른다. *"전사가 느리다는 이유로 정확도가 낮은 모델을 조용히 강제하지 않는다"* (§8.1 · §17.4) — **`large-v3-turbo`를 코드가 강제하지 않는다** | 설정 · Human Review |
| **녹음 레벨 정규화** | `05.7` 성공 기준 3의 P8 측정이 아직 `[미측정]`이다. **답이 나오기 전에 정규화를 구현하지 않는다** | ADR-0003 §16 |
| **붕괴 판정을 대체하는 것** | §18은 **안전망으로 남는다.** 차단은 판정 뒤에 오며(§20.6.2), §18.2의 임계값도 공식도 이 절이 바꾸지 않는다 | §18 |
| **이미 저장된 붕괴한 Transcript를 지우거나 고치는 것** | Transcript는 immutable하다 (INV-2). 2026-09-07에 저장된 103개짜리 Transcript도 그대로 남는다 (§18.5) | — |
| **청크 길이 · 임계값을 사용자 설정으로 여는 것** | 근거가 **관측 하나**뿐이다. 지금 손잡이를 만들면 아무도 무엇으로 바꿔야 하는지 모른다. 틀렸다는 증거가 나오면 고칠 자리는 한 모듈의 상수다 | §18.6과 같은 판단 (PRODUCT-SPEC §20.6) |
| **`capture-1788746454.wav`에 목표 수치를 세우는 것** | 레벨이 16 dB 부족한 파일에서 무엇이 가능한지 **알려져 있지 않다** (`05.8` P-4) | 관측만 기록한다 |
| **언어 · Metal 결정을 다시 건드리는 것** | §17 · §19가 끝낸 자리다 | — |
| **전사 결과를 "구제"하는 것** | 차단은 **연속 반복을 지우는 것**이지 문장을 다듬거나 붕괴한 텍스트를 복원하는 것이 아니다 (§18.6이 같은 자리에 이미 적었다) | — |

### 20.10 [관측된 사실] · [유력한 설명] · [미검증]

| 구분 | 내용 | 근거 |
| --- | --- | --- |
| **[관측된 사실]** | 2026-09-05에 `120초 청크 + 청크마다 state 재생성 + 연속 3회 초과 차단` + `ko` + Metal + `large-v3-turbo` 조건이 72.85분 오디오를 6.0분에 전사했고 segment 1,749 · 고유 1,643(94.0%) · 최다 반복 10회였다. **그 도구는 저장소 밖의 검증용 도구다** | [E5] |
| **[관측된 사실]** | 같은 오디오에서 **청크 분할만 없는** 조건은 6.1분에 *"반복 구간 2곳 잔존"* 으로 기록됐다 — **수치가 아니라 사람의 서술이며 고유 비율은 세어지지 않았다** | [E5] |
| **[관측된 사실]** | **2026-09-05 기록 어디에도 겹침에 대한 명시가 없다.** 적힌 것은 "120초 청크 분할"이 전부다 | [E5] · `05.8` P-2 |
| **[관측된 사실]** | 지금 제품은 오디오 전체를 한 번에 `full()`에 넘기고 전사 한 건에 state 하나를 만든다. `transcription/`에 전사 청크가 없다 | [E1] `whisper.rs` · `05.8` P-3 |
| **[관측된 사실]** | `collapse.rs`에 문장 정규화(`sentence_key`)와 임계값 상수 셋이 이미 있고, 그 자리가 하나라는 것을 테스트가 원문에서 확인한다 | [E1] |
| **[유력한 설명]** | 청크마다 state를 재생성하면 **반복 루프가 청크를 넘어 이어지지 못한다.** 2026-09-05에 반복이 크게 줄어든 것이 이 설명과 맞는다 | [E5] · §20.3 |
| **[미검증]** | **임계값의 근거가 2026-09-05 한 번의 실험뿐이다.** 120초도 연속 3회도 **다른 값과 비교된 적이 없다** — 60초 · 180초 · 연속 2회 · 연속 5회를 이 프로젝트는 잰 적이 없다 | 이 절 |
| **[미검증]** | **정상적으로 반복된 발화가 차단될 수 있는가.** 같은 짧은 대답이 네 번 이상 연달아 나오면 4번째부터 Transcript에 남지 않는다. 그 손실을 관측한 적도, 예외 규칙을 둔 적도 없다 | 이 절 · §20.6.3 |
| **[미검증]** | 세 변경(청크 분할 · state 재생성 · 반복 차단)의 **개별 기여도.** 2026-09-05는 셋을 함께 바꿨다 — `no_context = true` 위에 state 재생성이 무엇을 더하는지도 여기 포함된다 | [E5] · §20.3 |
| **[미검증]** | **겹침을 두면 경계에서 잘리는 문장이 줄어드는가.** 그리고 겹침 없이 경계에서 실제로 무엇이 잘리는가 — 이 저장소는 청크 경계를 관측한 적이 없다 | §20.4 · `05.8` Human Review |
| **[미검증]** | **차단 전 열로 §18을 판정하면 정상 전사가 붕괴로 판정될 수 있는가.** 94.0%는 차단이 켜진 채 잰 값이므로 차단 전 값이 알려져 있지 않다 | §20.6.2 |
| **[미검증]** | **제품 경로가 94.0%를 재현하는가.** 2026-09-05는 저장소 밖의 도구였다. 자동 Gate는 72분 오디오를 전사하지 않으므로 이것은 사람이 판정한다 | `05.8` Goal 1 · Human Review |
| **[미검증]** | 청크마다 state를 새로 만드는 **비용**(시간 · 메모리). 2026-09-05는 총 소요(6.0분)만 기록했다 | [E5] |
| **[미검증]** | `Detect` 갈래에서 **청크마다 다른 언어가 감지되는가.** 청크별 language 보고 값을 비교한 관측이 없다 | §20.7 |

### 20.11 이 갱신이 바꾼 것과 바꾸지 않은 것

**바꾸지 않은 것** — §2의 아홉 결정, §17.1의 언어 결정, §17.2의 Metal 결정, §18의 붕괴 판정
규칙, §19의 구현 기록은 **하나도 철회되거나 수정되지 않았다. §1~§19의 어떤 문단도 지우거나
다시 쓰지 않았다.** §10의 단위 변환 규칙도 그대로다 — 오프셋은 원시 단위에서 더해지고,
변환은 여전히 `parse` 한 자리다 (§20.5). §13의 되돌리기 경로도 그대로다 — 청킹은 엔진
구현이 부르는 순수 규칙이며 trait의 형태를 바꾸지 않는다 (§20.8). **경계가 넓어지지 않았다.**

| 언제 | 무엇을 | 왜 |
| --- | --- | --- |
| 2026-09-07 (TASK-086) | **§20을 추가** — 청크 길이(120초) · state 재생성의 경계 · 겹침 없음 · 오프셋을 센티초에서 더하는 규칙 · 연속 반복 차단(3) · 합쳐진 `language` 결정 · 규칙이 사는 자리 · 하지 않는 것 | 2026-09-05가 재현한 세 조건 중 **제품에 없는 하나**가 청크 분할이며 (`05.8` P-2 · P-3), 그것을 제품으로 옮기기 전에 **무엇을 어떤 값으로 할 것인가**가 정해져 있어야 한다. `05.8` Constraints가 이 갱신을 요구한다 |
| 2026-09-07 (TASK-086) | **머리말의 Status · Date · Phase · Task · Scope와 읽기 안내에 이 갱신을 덧붙였다** | §17 · §18 · §19가 한 것과 같은 형태다. **원래 줄을 지우지 않았다** |

**이 Task는 문서만 바꿨다.** `src-tauri/` 아래의 어떤 파일도, `Cargo.toml`도, 설정도, 테스트도
건드리지 않았다. §20이 정한 것은 **무엇을 부를 것인가와 어떤 값으로 할 것인가**이며, 그
코드는 Phase 5.8의 다른 Task가 쓴다. 그 Task가 실제 시그니처 · 실제 동작을 확인해 여기 적힌
것과 다르면 §16.2 · §17.2.2 · §19.4가 한 것처럼 **이 절에 되적는다** — 코드와 이 절이
어긋나면 **코드 쪽이 사실이다.**

---

## 21. §20의 구현 결과 — 실제로 만들어진 것 / 달라진 것 / 여전히 사람이 판정하는 것

```text
갱신:  2026-09-07 · TASK-091 (문서 전용 — 소스 · 설정 · 의존성 · 테스트를 바꾸지 않는다)
Phase: 5.8 — Transcription Chunking
범위:  §20이 값으로 확정한 규칙과 **오늘 저장소에 있는 코드**를 대조한다 [E1 — 이 Run이
       아래 파일들을 직접 읽었다]
       src-tauri/src/transcription/{chunking,whisper,run,engine,mod}.rs ·
       src-tauri/tests/transcription_chunking.rs · tests/transcription-chunking-boundary.test.ts
```

**§16이 §2에 대해, §19가 §17에 대해 한 것을 이 절이 §20에 대해 한다.** §20은 *무엇을 어떤
값으로 할 것인가*를 정하고 *그 코드는 Phase 5.8의 다른 Task가 쓴다*고 적었다 (§20.11).
그 코드가 쓰였고, 여기 적는 것은 **무엇이 실제로 만들어졌는가**다.

> **여기 적힌 "구현됐다"는 §16 · §19와 같은 뜻이다** — 코드가 존재하고 자동 검증이 그것을
> 지난다는 뜻이지, **72분짜리 회의가 읽을 만한 한국어로 전사되는 것을 봤다는 뜻이 아니다**
> (§21.3 · §21.4 · `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록 3).

### 21.1 규칙이 사는 자리와, 그것을 부르는 자리

**규칙은 순수 모듈 하나에 있고, 엔진 구현이 그것을 부른다** — §20.8이 그린 모양 그대로다
[E1].

```text
src-tauri/src/transcription/chunking.rs      ← ①②③의 규칙과 값 셋이 사는 단 한 자리
    CHUNK_SECONDS = 120 · CHUNK_OVERLAP_FRAMES = 0 · MAX_CONSECUTIVE_REPEATS = 3
    plan(total_frames, sample_rate_hz) -> Vec<AudioChunk>      ① 어디서 자르는가
    shift(segments, offset_centiseconds) · merge(chunks)       ② 어떻게 되돌려 잇는가
    block_consecutive_repeats(&[TranscriptSegment])            ③ 무엇을 차단하는가
        ↑ 이 함수만 **부르는 제품 코드가 없다** (§21.2)

src-tauri/src/transcription/whisper.rs       ← ①②를 부르는 단 한 자리
    chunking::plan(input.frames(), input.sample_rate_hz)?      샘플레이트를 짐작하지 않는다
    for chunk in &chunks {
        context.create_state()                                 청크마다 새 state (§20.3)
        state.full(params, &input.samples[chunk.range()])       구간으로 자르는 것은 부르는 쪽
    }
    ensure_usable(chunking::merge(transcribed)?)                밖으로 나가는 것은 하나다
```

| §20이 정한 것 | 실현된 자리 | 확인 |
| --- | --- | --- |
| §20.2 **청크 길이 120초 · 마지막 청크는 남은 만큼 · 전체가 120초 이하면 청크 하나** | `chunking::plan`과 `CHUNK_SECONDS` | 프레임 수는 `CHUNK_SECONDS × sample_rate_hz`로 나오고, 마지막 청크는 `min(남은 프레임)`이다. 무음으로 채우거나 앞 청크에 합치는 코드가 없다 |
| §20.2 **청킹 모듈이 샘플레이트를 짐작하지 않는다** | `plan`의 두 번째 인자 | `chunking.rs`에 `TARGET_SAMPLE_RATE_HZ`를 `use`하는 줄이 없다. 값을 아는 것은 부르는 쪽(`whisper.rs`)이다 |
| §20.3 **state는 청크마다 재생성 · 모델 context는 재사용** | `whisper.rs` — `WhisperContext::new_with_params`는 루프 **밖**, `context.create_state()`는 루프 **안** | 모델을 여는 호출이 전사 한 건에 한 번뿐이다 (§16.2의 "전사마다 연다"가 그대로다) |
| §20.4 **겹침 0 — 청크는 맞닿는다** | `CHUNK_OVERLAP_FRAMES`가 `plan`의 전진 계산에 실제로 들어간다 (`start_frame += frame_count - CHUNK_OVERLAP_FRAMES`) | 값이 상수로만 적혀 있고 산술에는 없는 상태가 아니다. 단위 테스트가 *모든 프레임이 정확히 한 번씩 들어간다*를 값으로 고정한다 |
| §20.5 **오프셋은 원시 단위(센티초)에서 더한다** | `chunking::shift` · `AudioChunk::offset_centiseconds` | `chunking.rs`에 밀리초라는 단위가 없다. `× 10` 변환은 여전히 `parse.rs`의 `MILLISECONDS_PER_CENTISECOND` 한 자리다 |
| §20.5 **넘침을 조용히 접지 않는다** | `offset_centiseconds()`의 `checked_mul` · `add_offset()`의 `checked_add` | 넘치면 `FailureKind::InvalidInput`으로 나간다. `saturating_*`가 없다 |
| §20.5 **오프셋은 청크 번호에서만 나온다** | `offset_centiseconds(index)` | 앞 청크가 문장을 하나도 내지 못해도 다음 청크의 오프셋이 밀리지 않는다 — 단위 테스트와 `tests/transcription_chunking.rs`가 각각 고정한다 |
| §20.6.1 **문장 비교는 `collapse::sentence_key` 하나** | `chunking.rs`가 그 함수를 `use`해서 부른다 | 두 번째 정규화 정의가 만들어지지 않았다 |
| §20.7 **language는 값을 보고한 첫 청크의 값 · 아무도 보고하지 않으면 None** | `chunking::merge` | 다수결도 마지막 값도 아니다. `merge`에 `LanguageChoice`도 `Settings`도 들어오지 않으므로 **설정 값을 베껴 넣을 값 자체가 손에 없다** (§17.1.4-3) |
| §20.8 **trait의 형태를 바꾸지 않는다** | `TranscriptionEngine::transcribe`의 시그니처가 그대로다 | 청킹은 그 구현 안쪽에서만 일어나고, `testing::StubEngine`도 그대로다. §13의 되돌리기 경계가 넓어지지 않았다 |
| §20.8 **청크가 화면에도 IPC에도 나타나지 않는다** | `domain` · `commands/payload.rs` · `src/ipc/types.ts`에 청크 개념이 없다 | `tests/transcription-chunking-boundary.test.ts` (f)가 원문으로 확인한다 (INV-9) |
| §20.8 **새 의존성을 들이지 않는다** | `Cargo.toml`이 늘지 않았다 | 같은 파일의 (e) |

### 21.2 §20과 달라진 것 — 그리고 왜

**결정 자체를 철회하거나 수정한 것은 없다.** 아래는 구현이 §20의 서술과 어긋나거나, §20이
정하지 않은 자리에서 값이 정해진 곳이다 (§16.2 · §19.4와 같은 성격이다).

| 무엇이 | §20이 적은 것 | 실제 | 왜 · 그래서 무엇이 달라지는가 |
| --- | --- | --- | --- |
| **③ 연속 반복 차단이 제품 경로에 연결되지 않았다** | §20.6.2: *"판정을 통과하면, 연속 반복을 차단한 열을 저장한다"* — 붕괴 판정 **뒤**, 저장 **앞**이 그 자리다 | **`chunking::block_consecutive_repeats`를 부르는 제품 코드가 없다** [E1]. `run.rs`는 `collapse::assess`를 지난 뒤 `transcription.segments`를 **그대로** `Transcript`에 담는다. 함수는 존재하고 `mod.rs`가 재수출하며 테스트가 값으로 고정하지만, **저장되는 Transcript는 차단 전 열이다** | 이 절은 이유를 알지 못한다 — 저장소에 남은 것은 "부르는 자리가 없다"는 사실뿐이다. **귀결은 적어 둔다:** ⑴ `MAX_CONSECUTIVE_REPEATS = 3`은 아직 제품 출력에 아무 영향도 주지 않는다. ⑵ §20.6.3이 감수한 대가(정상적으로 네 번 이상 연달아 나온 발화가 지워지는 것)도 아직 일어나지 않는다. ⑶ **제품은 2026-09-05가 함께 건 셋 중 둘만 갖고 있다** — 그 사실이 §21.3의 기준선 해석을 바꾼다 |
| **`engine.rs`의 trait 주석** | §20.5: *"'청크로 나눠 부르고 오프셋을 더해 이어 붙인다'는 사실은 그 주석에 남아야 한다"* | **그 주석에 남지 않았다.** `TranscriptionEngine`의 문서는 여전히 *"구현은 값을 옮기기만 한다 — 계산하지 않는다"*와 단위 변환만 말한다. 대신 **`whisper.rs`의 모듈 문서가** 청크 단위로 돈다는 사실 · 무엇을 재생성하고 무엇을 재사용하는지 · 규칙이 `chunking`에 있다는 것을 적는다 | trait의 **형태**는 §20.8이 요구한 대로 바뀌지 않았고, 규칙을 어긴 것도 아니다(오프셋 덧셈은 단위를 바꾸지 않는다 · §20.5). 어긋난 것은 **그 사실이 어느 파일의 주석에 적혔는가** 하나다 |
| **`plan`의 실패가 둘이다** | §20.5는 **오프셋 넘침**만 적었다 | `sample_rate_hz == 0`도 실패다 — 프레임을 시각으로 말할 수 없기 때문이다. 둘 다 기존 `FailureKind::InvalidInput`이다 | **새 `FailureKind`를 만들지 않는다**는 §16.2 · §18.4의 판단을 그대로 따랐다. `FailureKind`는 `src/ipc/failure.ts`의 union과 1:1이다 |
| **청크 하나가 실패하면 무엇이 남는가** | §20은 정하지 않았다 | **전사 전체가 실패하고, 몇 번째 청크였는지가 `detail`에 `chunk k/n`으로 남는다.** 앞선 청크의 결과를 부분 전사로 저장하지 않는다 | Transcript는 immutable하므로(INV-2) 중간까지만 있는 전사가 영구히 남는다. 72분 녹음에서 어디가 무너졌는지를 되짚을 수단은 남긴다 |
| **`whisper-rs` 0.16의 실제 API** | §20.3: *"`create_state`의 실제 시그니처와 청크마다 state를 새로 만드는 비용은 컴파일러와 실행이 말한다 … 다르면 이 절에 되적는다"* | **다르지 않았다.** `WhisperContext::create_state() -> Result<WhisperState, WhisperError>`이며 하나의 context에서 청크마다 부를 수 있다. §16.2 · §19.1이 적은 나머지 API(`full` · `full_n_segments` · `get_segment` · `start_timestamp` · `to_str` · `full_lang_id_from_state` · `set_language` · `set_detect_language`)도 그대로다 | **되적을 차이가 없다는 것도 기록한다** — 확인하지 않은 것과 확인해서 같았던 것은 다른 진술이다. 판정 수단은 문서가 아니라 컴파일러다: `lint` Gate가 `cargo clippy --all-targets -- -D warnings`를, `test` Gate가 `cargo test`를 돌리며 이 호출들을 실제로 컴파일한다 [E1 · `.loop/project.yaml` · `package.json`의 `lint:rust` · `test:rust` · 이 Run의 self-check에서 세 Gate가 전부 exit 0] |
| **state 재생성의 비용** | §20.3: *"청크마다 state를 새로 만드는 비용은 … 실행이 말한다"* | **측정되지 않았다.** 자동 검증은 실제 모델을 열지 않으므로 `create_state`가 한 번도 실행되지 않는다 | §20.10이 [미검증]으로 남긴 그대로다. 사람이 72분 오디오를 전사할 때의 소요 시간으로만 간접적으로 드러난다 (§21.3) |

### 21.3 자동 검증이 무엇을 판정하고, **무엇을 판정하지 못하는가**

**구현이 들어왔다는 사실은 `05.8`의 성공 기준 1이 충족됐다는 뜻이 아니다.** 성공 기준 1은
*제품 경로가 `capture-1788522158.wav`에서 읽을 만한 한국어 전사를 낸다*이며, 그 판정은
**사람이 앱으로 실행해야만** 나온다 (`phase-prompt/05.8` Goal 1 · Human Review).

**판정하는 것** [E1 · 이 Run의 self-check에서 `build` · `lint` · `test` 세 Gate가 전부 exit 0]:

| 어디가 | 무엇을 |
| --- | --- |
| `transcription/chunking.rs`의 단위 테스트 | 청크 경계값(0 프레임 · L보다 짧음 · 정확히 L · 배수 · 배수+1) · 청크가 맞닿고 모든 프레임이 정확히 한 번 들어간다 · `offset_cs(k) = 12,000 × k` · **프레임 → 센티초가 나머지 없이 떨어진다**는 정수 관계 · 넘침이 실패로 나간다 · 차단이 3회까지 남기고 4회째부터 지운다 · 떨어진 반복은 지우지 않는다 · 공백만 다른 두 문장은 같은 문장이다 · 첫 language가 이긴다 · **모듈이 바깥 세계를 모른다** · **값 셋이 한 자리에만 있다** |
| `src-tauri/tests/transcription_chunking.rs` | 청크 로컬 시각을 그대로 이으면 되감긴다는 것(CH-1) · 합쳐진 결과가 Transcript 하나로 저장되고 시각이 뒤로 가지 않는다 · 청크로 도착한 붕괴도 저장 직전에 막힌다(CH-2) · **차단을 먼저 하면 그 붕괴가 통과한다는 것**과 그래서 제품이 판정을 먼저 한다는 것 · 실패가 원본 오디오와 current Transcript를 건드리지 않는다(CH-3) · 묶음이 청크 경계를 넘어도 하나로 센다(CH-4) |
| `tests/transcription-chunking-boundary.test.ts` | 규칙과 값이 한 파일에만 있다 · 순수 모듈이 파일시스템 · DB · 네트워크 · `whisper_rs`를 모른다 · 센티초→밀리초 계수가 `parse.rs` 한 자리다 · `run.rs`의 붕괴 판정이 저장보다 앞에 있다 · 새 의존성이 없다 · 청크 개념이 payload와 frontend 타입에 없다 |

**판정하지 못하는 것 — 이유가 항목마다 다르므로 각각 적는다:**

| 판정되지 않는 것 | 왜 |
| --- | --- |
| **고유 문장 비율이 94.0%에 닿는가** | 자동 검증에는 **실제 모델도 실제 오디오도 없다.** 엔진 자리에는 언제나 `StubEngine`이 서고(PRODUCT-SPEC §18), stub이 내는 문장은 테스트가 값으로 만든 것이다 — 그 비율은 **whisper가 무엇을 들었는가**가 아니라 fixture가 무엇으로 쓰였는가를 잰다 |
| **한국어로 읽히는가** | 같은 이유다. **stub은 한국어가 한국어로 들렸는지를 알 수 없다** (§17.3 · §19.5가 기록한 Gate의 경계 그대로다) |
| **소요 시간이 6.0분과 크게 다르지 않은가** | Gate는 72분 오디오를 전사하지 않는다. `transcription_ms`가 기록되는 경로는 있지만(§19.3) **그 값은 실제 추론이 있어야 생긴다** |
| **청크 경계에서 문장이 잘리거나 중복되는가** | 겹침이 0이므로 경계에 걸친 문장이 두 조각으로 잘릴 수 있고, 그것이 실제로 얼마나 일어나는지는 §20.4가 [미검증]으로 남겼다. **자동 검사가 확인할 수 있는 것은 "시각이 되감기지 않는다"까지이며, "말이 자연스럽게 이어졌다"는 사람만 판정한다** |
| **실제 오디오 버퍼가 청크 구간으로 잘리는 자리** | 그 자리는 `WhisperEngine::transcribe` 안이며 모델 파일을 요구한다. 통합 테스트가 보는 것은 **합쳐진 다음**의 경로 전부다 (`tests/transcription_chunking.rs`의 모듈 문서가 같은 사실을 적는다) |
| **제품 경로가 2026-09-05의 94.0%를 재현하는가** | 그 실행은 **저장소 밖의 검증용 도구**였고 조건 셋을 함께 걸었다 [E5]. **제품에는 그중 둘만 있다** — 연속 반복 차단은 아직 부르는 자리가 없다 (§21.2). 조건이 완전히 같지 않다는 사실을 수치와 함께 읽어야 한다 |
| **청크마다 state를 새로 만드는 비용** | 자동 검증에서 `create_state`가 한 번도 실행되지 않는다 (§21.2) |

**절차와 빈 기록표는 `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록 3이다.** 그 표의 결과
칸이 `[미측정]`인 동안, **이 Phase를 "제품 경로가 읽을 만한 한국어 전사를 낸다"고 적지
않는다.**

### 21.4 `A-TRANS-001`은 이 Phase 뒤에 어떤 상태인가 — **여전히 사람의 실행으로만 닫힌다**

**§16.3.1 · §17.3 · §19.5를 지우지 않는다.** 여기 적는 것은 그 뒤의 상태다.

```text
2026-09-03   A-TRANS-001 수용 — 구현됐고 자동 검증을 통과했으나 추론이 한 번도 돌지 않았다
2026-09-05   첫 실행. 엔진 경로는 지났고 제품 결과는 쓸 수 없었다 (언어가 en으로 강제됐다)
2026-09-06   §17이 그 원인을 결정으로 바꿨다 (문서)
2026-09-07   Phase 5.6이 그 결정을 코드로 만들었다 (§19)
2026-09-07   두 번째 실행. 언어 수정은 동작했고, 결과는 **다른 이유로** 다시 붕괴했다 (§18)
2026-09-07   Phase 5.7이 그 붕괴를 실패로 말하게 만들었다 (collapse.rs)
2026-09-07   Phase 5.8이 청크 분할과 state 재생성을 제품 경로에 넣었다 (§21.1)
             — **세 번째 실행은 아직 없다**
```

| | |
| --- | --- |
| **Phase 5.8이 한 것** | 2026-09-05가 함께 건 조건 셋 중 **제품에 없던 하나**를 옮기기 시작했다 — 청크 분할과 청크마다의 state 재생성이 제품 경로에서 돈다. 청크 로컬 시각이 전체 시간축으로 되돌아오고, 그 규칙이 순수 모듈 하나에 산다 (`05.8` 성공 기준 2) |
| **Phase 5.8이 하지 않은 것** | **읽을 만한 전사를 내는 것.** 그리고 셋 중 하나(연속 반복 차단)는 **아직 제품 출력에 닿지 않는다** (§21.2). 자동 검증은 여전히 `StubEngine`으로 돈다 |
| **그래서 `A-TRANS-001`은** | **열려 있다.** 이 Phase는 *조건을 하나 더 갖췄을* 뿐이고, 사람이 읽을 수 있는 전사는 **아직 한 번도 나오지 않았다** |

```text
이 가정이 닫히는 조건 — §19.5가 적은 것과 같다. 하나다

  사람이 실제 회의를 녹음(또는 이미 녹음된 파일을 제품 경로로 전사)하고,
  그 결과가 **읽을 만하다고 사람이 판정하는 것**

닫는 주체   이 절이 아니다. docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md 부록 3의 결과 칸과
            Human Review 칸이 채워지고, docs/PHASE-5.6-HUMAN-REVIEW.md §8 ·
            docs/PHASE-5.7-HUMAN-REVIEW.md §8.4가 채워진 뒤의 Task가
            §16.3.1과 SYSTEM-MAP §7을 함께 고친다
```

**모델 크기 판정도 그대로 열려 있다** (§17.4 · §19.5). 이 절은 `large-v3-turbo`가 필요하다고도
`base`로 충분하다고도 적지 않는다 — 부록 3이 `large-v3-turbo`를 쓰는 이유는 **2026-09-05와
조건을 맞추기 위해서이지 제품 기본값을 정해서가 아니다.**

### 21.5 이 갱신이 바꾼 것과 바꾸지 않은 것

**바꾸지 않은 것** — §2의 아홉 결정, §17.1의 언어 결정, §17.2의 Metal 결정, §18의 붕괴 판정
규칙, §19의 구현 기록, §20의 청킹 규칙은 **하나도 철회되거나 수정되지 않았다. §1~§20의 어떤
문단도 지우거나 다시 쓰지 않았다.** §10의 단위 변환 규칙도 그대로이고(§21.1), §13의 되돌리기
경로도 그대로다 — 청킹은 엔진 구현이 부르는 순수 규칙이며 trait의 형태를 바꾸지 않았다.
**경계가 넓어지지 않았다.**

| 언제 | 무엇을 | 왜 |
| --- | --- | --- |
| 2026-09-07 (TASK-091) | **§21을 추가** — 규칙이 사는 자리와 부르는 자리 · §20과 달라진 여섯 · 자동 검증이 판정하지 못하는 일곱 · `A-TRANS-001`의 상태 | §20이 *"코드는 다른 Task가 쓴다"* 로 남긴 자리가 채워졌고, **무엇이 실제로 만들어졌는지를 적는 자리가 §16 · §19처럼 필요했다.** 특히 **차단이 제품 경로에 없다는 사실**은 적어 두지 않으면 다음 사람이 9/5와 같은 조건이라고 읽는다 |
| 2026-09-07 (TASK-091) | **머리말의 Status · Date · Phase · Task · Scope와 읽기 안내에 이 갱신을 덧붙였다** | §17 · §18 · §19 · §20이 한 것과 같은 형태다. **원래 줄을 지우지 않았다** |

**이 Task도 문서만 바꿨다.** `src-tauri/` 아래의 어떤 파일도, `Cargo.toml`도, 설정도, 테스트도
건드리지 않았다. 이 절이 적은 것은 **이미 저장소에 있는 코드를 읽은 결과**이며, 코드와 이 절이
어긋난다면 **코드 쪽이 사실이고 이 절을 고쳐야 한다.**

---

## 22. 무음 구간 환각 — 무엇이 원인이었고, 무엇이 그것을 고쳤는가 (2026-09-08)

§18이 **붕괴**를 정의하고 §20이 **청크 분할**을 넣은 뒤, 제품 경로가 처음으로 읽을 수 있는
전사를 냈다 (고유 94.7%). 그러자 남은 오류가 무엇인지도 처음으로 보였다 — **오류가 고르게
퍼져 있지 않고 한 곳에 몰려 있었다.**

### 22.1 관측 — 조용한 구간에서만 무너진다

**[관측된 사실 · H✓ 2026-09-08]** 94.7%를 낸 그 전사의 `00:57 ~ 01:08` 약 10분이 통째로
환각이었다.

```text
00:57:03 ~ 00:57:56   "멈추린 곳이."
00:58:00 ~ 01:07:30   "한글자막 by 한효정"      30초 간격으로 반복
01:00:00 ~ 01:03:30   "이 시각 세계였습니다."
01:04:00 ~ 01:05:57   "twohang" · "Northeast" · "-"
```

**`한글자막 by 한효정`은 2026-09-07에 51분 녹음을 통째로 날린 바로 그 문장이다** (§18.1).

**[관측된 사실 · H✓ 2026-09-08]** 그 구간의 입력 레벨을 쟀고, **레벨이 무너진 구간과
전사가 무너진 구간이 일치했다** (ADR-0003 §16.8.1 — 저레벨 창 20개가 전부 그 안에 있다).

**[유력한 설명]** whisper는 자막으로 학습했고, 알아들을 것이 없는 구간에서 디코더가 학습
데이터의 자막 상투구를 뱉는다. 말이 있는 구간은 정상이고 조용한 구간만 무너진다는 관측이
이 설명과 맞는다.

### 22.2 VAD를 켰다 — 그리고 **듣지 않았다**

`whisper-rs 0.16.0`이 노출하는 Silero VAD를 붙이고(§22.5), 같은 파일을 다시 돌렸다.

```text                 이전            VAD + 증폭
segment              1,678           1,638
고유 비율             94.7%           94.3%
한글자막 by 한효정     6회             8회
멈추린                3회             0회
twohang / Northeast   각 1회          0회
```

**[관측된 사실]** 일부는 사라졌으나 **주된 것은 그대로이거나 늘었고, 고유 비율은 오히려
0.4%p 내려갔다.**

**[유력한 설명]** 그 10분은 **디지털 무음이 아니다.** 너무 작고 멀어 알아들을 수 없는
말소리이며, VAD는 거기서 음성 활동을 찾아낸다. **VAD가 지우는 것은 침묵이지 웅얼거림이
아니다.**

**이 절이 이 실패를 지우지 않고 남긴다.** VAD는 이 저장소에서 **아직 한 번도 효과를 보인
적이 없다.**

### 22.3 진짜 기전 — **창을 채운 상투구**

**[관측된 사실 · H✓ 2026-09-08]** 환각 segment가 `00:58:00 · 00:58:30 · 00:59:00 ·
01:00:00` — **정확히 30초 간격**에 있었다. 30초는 whisper의 디코딩 창이다
(`WHISPER_CHUNK_SIZE`).

**창 하나에 알아들을 것이 없으면, 디코더는 아무것도 내놓지 않는 대신 외운 자막 한 줄로
창 전체를 채운다.**

그래서 판정 근거는 문구가 아니라 **말의 속도**다.

```text
 3.6 자/초   "- 아 일정 본인 일정이세요? …"  (20.1초 · 73자)   실제 발화
11.4 자/초   "구독제로 가야 될 거 아니야."    (1.4초 · 16자)   실제 발화
 0.37 자/초  "한글자막 by 한효정"            (30초 · 11자)    환각
 0.14 자/초  "GGG"                           (22초 ·  3자)    환각
 0.04 자/초  "-"                             (25.7초 · 1자)   환각
```

### 22.4 왜 문구 목록이 아닌가 — **두 실패를 다 관측했다**

**[관측된 사실 · H✓ 2026-09-08]** 알려진 문구를 막는 방식은 두 방향으로 실패한다.

```text
오탐   "구독제로 가야 될 거 아니야." (1.4초 · 실제 발화)가 "구독"에 걸렸다
누락   "GGG" · "-" · "twohang" 은 목록에 없었다. 그리고 목록은 언제나 뒤늦다
```

**둘 다 그날 실제로 일어났다.** 말의 속도로 판정하면 둘 다 일어나지 않으며, 어느 언어의
어떤 상투구든 창을 채우고 말이 없으면 걸린다.

#### 임계값 — **20초 · 1.0 자/초**

**[H✓ 2026-09-08 실측]** 전사 세 건(9/4 두 번 · 9/8 한 번 · 합계 6,141 segment)으로 쟀다.

```text
20초 · 1.0 자/초   27개 제거 — **전부 환각. 실제 발화 0개**
15초로 내리면       "그러면 일단 위클리를 미루고" (16.2초 · 실제 발화)가 걸린다
10초로 내리면       "아, 네." · "공윤은 어떻게 해야지." 까지 걸린다
```

**[미검증]** 다른 녹음 · 다른 언어에서도 20초가 맞는지는 재본 적이 없다. 라틴 문자는
음절당 글자 수가 달라 `1.0 자/초`가 그대로 맞지 않을 수 있다 — **이 저장소가 잰 것은
한국어뿐이다.**

### 22.5 규칙이 사는 자리

§18.3 · §20.8이 세운 선례 그대로, 각 규칙은 **파일시스템도 저장소도 네트워크도 엔진도
모르는 순수 모듈 하나**에 산다.

```text
transcription/hallucination.rs   창을 채운 상투구 판정 (§22.3 · §22.4)
transcription/gain.rs             전사 입력 증폭 (ADR-0003 §16.8)
transcription/vad.rs              VAD 모델 파일을 찾는다. **없는 것은 실패가 아니다**
transcription/live.rs             녹음 중 전사의 창 규칙 — **아무 데도 연결되지 않았다**
```

#### 실행 경로의 순서

```text
assess (붕괴 판정)            ← 차단 전 수치를 본다 (§20.6.2)
  ↓
hallucination::drop_windows_without_speech
  ↓
block_consecutive_repeats     ← 이어진 것이 같을 때 (§20.6.1)
  ↓
collapse_repeated_phrases     ← segment 하나 안쪽 (§23.2)
  ↓
parse::join_text 로 raw_text 재생성   ← §23.4
```

**환각 제거가 반복 차단보다 앞이다.** 이 상투구는 30초 창마다 하나씩 나오므로 *이어진*
것처럼 보이지만, 사이에 다른 문장이 끼면 반복 차단이 묶음을 성립시키지 못한다. 판정
근거가 다르므로(하나는 되풀이, 하나는 말의 속도) 서로를 대신하지 않는다.

#### VAD — 없는 것이 실패가 아니다

`enable_vad(true)`는 모델 경로가 없으면 **panic한다** (`whisper_params.rs:823`). 그래서
경로를 **먼저 넣고, 넣은 경우에만 켠다**. `tests/transcription_chunking.rs`가 그 순서를
소스에서 고정하며, 뒤집으면 실패한다.

`vad::find`가 `Result`가 아니라 `VadChoice`인 것도 같은 규칙이다 — **없는 것이 실패로
표현될 수 없는 형태**여야 모델을 받지 않은 기기에서 전사가 통째로 실패하지 않는다.

**[관측된 사실 · A✓ 2026-09-08]** VAD는 무음을 샘플 버퍼에서 들어낸 뒤 디코딩하고
(`whisper.cpp:6615-6790`), 압축된 시간축을 `vad_mapping_table`로 되돌린 다음
`whisper_full_get_segment_t0/t1`을 낸다 (`whisper.cpp:7912-7964`). **따라서 §20.5의
"원시 단위에서 청크 오프셋을 더한다"가 그대로 유효하다.**

### 22.6 [관측된 사실] · [유력한 설명] · [미검증]

| | |
| --- | --- |
| **[관측된 사실]** | 환각 구간과 저레벨 구간의 일치 · 30초 간격 · 문구 목록의 오탐과 누락 · 임계값 27/27 · VAD가 듣지 않았다는 것 |
| **[유력한 설명]** | 알아들을 것이 없는 창을 자막 상투구로 채운다 · VAD가 웅얼거림을 음성으로 본다 |
| **[미검증]** | 20초 · 1.0 자/초가 다른 언어·녹음에서도 맞는가 · 증폭의 기전 · VAD가 어떤 조건에서 듣는가 · 이 규칙이 실제 발화를 지운 적이 있는가 (사람이 읽어야 한다) |

### 22.7 이 절이 바꾼 것과 바꾸지 않은 것

| | |
| --- | --- |
| 바꾸지 않았다 | 엔진 선택 · 청크 길이(120초) · 겹침(0) · `MAX_CONSECUTIVE_REPEATS`(3) · 붕괴 판정과 그 임계값(§18) · timestamp 규칙(§10 · §20.5) · `TranscriptionEngine` trait의 형태 |
| 더했다 | 창 상투구 제거 · 전사 입력 증폭 · VAD(효과 미확인) · 녹음 중 전사의 창 규칙(미연결) |
| 지우지 않았다 | **VAD가 듣지 않았다는 관측**(§22.2). 뒤에 효과를 보이더라도 이 기록은 남는다 |

---

## 23. 2026-09-07 · 09-08에 Runtime을 거치지 않고 들어온 변경들

**이 절은 자랑이 아니라 기록이다.** 아래 변경들은 Gate(build · lint · test)는 지났으나
**Verifier도 Task 기록도 없이** 대화형 세션이 직접 넣었다. `self-check`는 판정이 아니다
(`CLAUDE.local.md`). 무엇이 왜 그렇게 들어왔는지 적어 두지 않으면, 이 저장소는 자기가
무엇을 갖고 있는지 문서로 말하지 못한다.

### 23.1 UTF-8 경계에서 잘린 글자 (2026-09-07)

**[관측된 사실]** 2026-09-07 실행이
`chunk 32/37: segment 2/14: Invalid UTF-8 detected in a string from Whisper. Index: 0, Length: 1`
로 죽었다. **그때까지 만든 31개 청크가 함께 버려졌다 — 40분치다.**

**원인:** whisper.cpp의 segment 경계는 **토큰 경계이지 글자 경계가 아니다.** 한글은 UTF-8로
글자당 3바이트이므로, 경계가 글자 한가운데 떨어지면 앞은 끝이 잘리고 뒤는 이어지는
바이트로 시작한다.

**수정:** `whisper.rs`가 `to_str()` 대신 `to_bytes()`로 읽고, `decode_segment_texts`가
잘린 꼬리를 다음 segment 앞으로 넘겨 디코딩한다. **lossy 변환을 쓰지 않았다** — 그러면
경계마다 글자가 하나씩 사라지고, 72분 녹음에서 그 경계는 수백 곳이다.

### 23.2 반복 차단이 실행 경로에 연결되지 않았다 (2026-09-07)

**[관측된 사실]** §20.6이 `block_consecutive_repeats`를 만들고 테스트까지 붙였으나
**부르는 제품 코드가 없었다.** `src-tauri/tests/transcription_chunking.rs`가 그 사실을
주석으로 적고 있었다.

**수정:** 판정(`assess`) **뒤에** 차단하도록 연결하고, 지워진 개수를
`Completed::removed_segments`로 남긴다.

그리고 §20.6.1의 차단은 **이어진 segment들**이 같을 때만 동작하는데, 관측된 붕괴는
**segment 하나의 텍스트 안에서** 일어났다 (`"엉덩이" × 13`). 그래서
`collapse_repeated_phrases`를 더했다 — 한도는 `MAX_CONSECUTIVE_REPEATS`를 인자로 받아
**값을 두 번 정의하지 않는다.**

### 23.3 전사 중 미리보기 (2026-09-07)

72분이 도는 동안 화면이 보여 줄 수 있는 것이 "돌고 있다" 한 마디뿐이었고, 그래서
2026-09-07 실행은 **40분을 돌고 실패할 때까지 사람이 한 글자도 보지 못했다.**

`transcription/progress.rs`가 조각이 끝날 때마다 문장을 쌓고 진행률을 함께 낸다.
진행률은 **"몇 번째 청크"가 아니라 오디오 진행 비율**이다 — §20.8이 청크를 화면·IPC에
드러내지 않기로 정했기 때문이다 (INV-9).

### 23.4 `raw_text`가 차단을 지나지 않았다 (2026-09-08)

**[관측된 사실 · H✓ 2026-09-08]** `run.rs`가 segment만 거르고 `raw_text`는 엔진이 준
것을 그대로 저장하고 있었다. 실측:

```text
raw_text  안의 되풀이   24회
segments  안의 같은 것    3회      (1,201자 차이)
```

**그리고 AI 노트가 `raw_text`를 쓴다** (`ai/run.rs`). 즉 §20이 만든 붕괴 차단이 **AI
노트에는 한 번도 닿은 적이 없었다.** `export/ai_request.rs`는 segment를 쓰므로 Manual
Handoff와 Export는 영향받지 않았다.

**수정:** `raw_text`를 차단·축약 뒤 segment에서 **`parse::join_text`로 다시 만든다** —
`parse`가 이미 세운 규칙과 **같은 함수**를 쓴다. 그 규칙을 두 번 정의하지 않는다.

**[관측된 사실]** 이미 저장된 Transcript는 고치지 않았다 (INV-2). 2026-09-08 이전에
만들어진 전사의 `raw_text`에는 그 되풀이가 그대로 남아 있다.

### 23.5 화면이 죽어도 녹음은 죽지 않는다 (2026-09-08)

전사 모듈의 변경은 아니지만 **같은 날 같은 방식으로 들어왔고, 녹음의 독립성에 대한
기록이므로 여기 남긴다.**

**[관측된 사실]** 녹음 중에 화면이 통째로 까맣게 됐다. React 트리가 에러 하나로
언마운트됐고 받아 낼 자리가 없었다. **그 뒤에서 녹음은 2시간 4분 동안 계속 돌았다** —
session을 Rust가 들고 있기 때문이다 (`commands/mod.rs:77` · R-001). 사람은 그 사실을 알 수
없었고 Stop에 손이 닿지 않았다.

**[미검증]** **무엇이 던졌는지는 끝내 확인되지 않았다.** 콘솔 기록도 crash report도
남지 않았다. **방어를 넣었을 뿐 원인을 고친 것이 아니다.**

### 23.6 이 절이 남기는 것

**위 다섯은 전부 Runtime을 거치지 않았다.** 지금은 각각을 지키는 전용 테스트가 붙어 있고
이 문서가 그것을 가리키지만, **그것이 Verifier를 지났다는 뜻은 아니다.**

`docs/LOOP-RUNTIME-FIELD-NOTES.md`의 OBS-028이 그날 Runtime 경로가 왜 중단됐는지를
따로 적고 있다.
