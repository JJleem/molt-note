# Phase 3 — Local Transcription

Implement Phase 3 of `docs/PRODUCT-SPEC.md`.

## Goal

Phase 2가 만든 녹음 파일을 **로컬에서 전사**해서, timestamp가 포함된 Transcript를
Recording Detail 화면에서 볼 수 있게 만든다.

이 Phase의 성공 기준:

> 실제 Molt Note에서 녹음한 음성을 **로컬에서** 전사하고, timestamp가 포함된 transcript를
> 상세 화면에서 확인할 수 있다. 이 과정에서 오디오는 기기 밖으로 나가지 않는다.

```text
Recording (audio file) → [필요한 전처리] → whisper.cpp → timestamped transcript → local DB
```

## Why This Phase Exists

Transcript는 이 제품의 두 번째 핵심 산출물이며, Phase 4(AI Note)와 Phase 5(Notion/Export)의
**유일한 입력**이다. 그리고 이것이 §12 Privacy Boundary에서 "외부 전송 없음" 영역의 끝점이다.

전사가 로컬에서 동작하지 않으면 Molt Note는 Local-first 제품이 아니다.

## The Architecture Decision This Phase Must Make

`docs/PRODUCT-SPEC.md` §14.4에 whisper.cpp에 대해 확인된 사실이 정리되어 있다.
이 Phase는 **whisper 통합 방식을 선택하고 근거를 ADR로 기록해야 한다.**

결정해야 할 것:

1. **통합 방식** — 후보 셋을 §14.4.2의 **운영자 정책 아래에서** 비교한다.

   ```text
   A. Tauri sidecar + whisper-cli     진지한 후보
   B. whisper-rs (Rust 바인딩)         진지한 후보
   C. 사용자 설치 바이너리 참조        V1 정상 배포 경로로는 부적합
   ```

   **제품 정책: 사용자에게 whisper.cpp · Homebrew · CMake · Whisper CLI 설치를 요구하지 않는다.**

   - **C는 A와 B가 둘 다 실증된 blocker를 만났을 때만** 고려한다. 개발/디버그 fallback으로는
     남을 수 있으나, **cmake가 없었다는 이유로 선택하지 않는다.**
   - **cmake는 A와 B 양쪽 모두 요구하므로 둘을 가르는 기준이 아니다.**
     개발 Mac에 2026-09-03에 설치됐다 (§14.1 — cmake 4.4.3). 이것은 **개발 빌드 의존성이지
     사용자 의존성이 아니다.**
   - **A의 notarization 이슈(tauri#11992)는 packaging 위험이지 배포 불가의 증거가 아니다.**
     *"sidecar를 빼는 것 외에 우회가 없다"* 를 **확정된 사실로 적지 않는다.**
     관찰된 증거를 기록하고 최종 notarization 확인은 배포 검증 경계로 넘긴다.
   - **B를 "Rust니까"라는 이유로 선택하지 않는다.** 번들 whisper.cpp 버전 지연(v1.8.3 vs
     v1.9.3) · 유지보수 위치 이동(GitHub archived → Codeberg) · in-process crash 격리
     tradeoff를 함께 본다.

2. **바이너리 확보 경로 — 아티팩트 종류를 뭉뚱그리지 않는다.**

   ⚠️ upstream이 무엇을 배포하는지 **실제 release asset을 보고** 다음을 구분한다.

   ```text
   (a) macOS CLI 실행 파일 (whisper-cli)   ← Tauri sidecar로 쓸 수 있는 것
   (b) XCFramework / 라이브러리 아티팩트     ← CLI가 아니다
   (c) 소스 빌드만 가능
   (d) Windows 아티팩트
   ```

   **XCFramework가 있다고 해서 sidecar용 macOS `whisper-cli` 실행 파일이 있는 것은 아니다.**
   그 반대도 마찬가지다. §14.4의 기록은 2026-09-01 기준이며 **재확인 대상이다.**

   ```text
   Windows x64  →  공식 prebuilt 있음 (whisper-bin-x64.zip · blas · cublas)
   macOS        →  공식 prebuilt 없음. 소스 빌드가 전제이며 cmake가 필요한데
                   §14.1 기준 이 기기에 cmake가 없다
   ```

   **더 어려운 쪽이 primary 개발 플랫폼이다.** 이 제약을 어떻게 다룰지 결정해야 한다.
   sidecar를 택한다면 target triple 접미사가 플랫폼마다 다르다는 것도 반영해야 한다
   (§14.4 — Windows는 `.exe`를 포함한다).

3. **모델 관리** — 모델 파일은 466MB~3GB다. 앱에 번들할 것인가, 최초 실행 시 내려받을 것인가,
   사용자가 지정하게 할 것인가. 모델이 없을 때의 상태와 안내가 있어야 한다.

4. **원본 오디오는 절대 덮어쓰지 않는다 (INV-1 · INV-3).**

   Phase 2가 만든 raw recording은 **장치 native sample rate / channels의 PCM16 WAV**다
   (예: 48kHz stereo). 전사를 위해 그 파일을 직접 resample해서 덮어쓰지 않는다.

   ```text
   raw audio            immutable · 보존
   derived 전사 입력    재생성 가능 · 파생물
   ```

   변환이 필요하면 **파생 입력을 따로 만든다.** 원본을 건드리는 순간 INV-1 위반이다.

5. **입력 포맷 변환 책임** — 선택한 통합 방식이 실제로 무엇을 요구하는지 확인한다.
   (CLI는 16-bit WAV를 받고 모델은 16kHz mono 기준이라는 것이 §14.4의 기록이지만,
   Rust 바인딩은 f32 PCM을 직접 받을 수도 있다 — **통합 방식마다 다르므로 확인한다.**)

   변환 수단도 결정한다 — Rust native resampling / 외부 도구 / 통합 방식이 제공하는 전처리.
   **개발 Mac에 ffmpeg이 있다는 이유로 사용자 의존성으로 가정하지 않는다.**
   사용자에게 Homebrew 설치를 요구하는 구조라면 그 tradeoff를 명시한다.
   근거 없이 과도한 custom DSP를 직접 구현하지도 않는다.
   Phase 2가 고른 recording engine의 출력이 이미 그 포맷이면 변환은 불필요하고,
   아니면 변환 단계가 필요하다. ffmpeg에 의존할 것인지 결정하고,
   의존한다면 **사용자 기기에 ffmpeg이 없을 경우**를 다뤄야 한다.

§14.4의 UNVERIFIED 항목(sidecar 코드서명/notarization 이슈 #11992, 한국어+영어 혼용에
적합한 모델)은 **실제로 확인한 뒤 결정한다.** 문서만 읽고 결정하지 않는다.

### Windows에 대해 이 Phase가 하는 일 / 하지 않는 일

**한다**: 통합 방식이 Windows에서도 성립하는지 근거와 함께 평가하고 ADR에 남긴다.
바이너리 위치를 해석하는 지점이 플랫폼에 따라 갈리면 거기에만 경계를 둔다
(§3.1의 `TranscriptionRunner` · `SidecarResolver`).

**하지 않는다**: Windows 빌드 · Windows 바이너리 확보 · Windows 실행 검증. 전부 Phase 6이다.
**추상화를 선입금하지 않는다 (§20.6).**

## Required Outcome

1. **whisper 통합 방식이 선택되고 근거·검증 결과가 문서로 남는다.** 탈락한 후보와
   탈락 이유를 포함한다.

2. **Recording 하나에 대해 전사를 실행할 수 있다.** 수동 실행이 최소 요구사항이다.

   **자동 전사 설정은 기존 `automatic_processing`을 재사용하지 않고 별도 값으로 만든다**
   (운영자 결정 · 2026-09-03). 이름은 저장소 규약에 맞추되 의미는 `automatic_transcription`이다.

   근거: 전사와 AI 처리는 **독립적인 기능**이다. 이후 Phase에서 다음이 가능해야 한다.

   ```text
   automatic transcription = ON
   automatic AI note       = OFF     (그리고 그 반대도)
   ```

   **하나의 boolean에 여러 의미를 겹치지 않는다.**
   V1 기본값은 **OFF**로 한다 (Product Spec에 다른 기본값을 요구하는 명시 규칙이 없다).
   수동 전사는 항상 가능하다.
   **모델이 없는 상태는 설정을 조용히 바꾸는 것이 아니라 제품 상태로 표현한다.**

3. **전사 진행 상태가 `Recording.transcriptionStatus`에 반영**된다 —
   `none · pending · running · done · failed`. 상태는 Recordings 목록과 Detail 양쪽에 보인다.
   긴 전사(1시간 분량)가 도는 동안 앱이 응답 불가 상태가 되지 않아야 한다.

4. **Transcript가 §7 모델대로 저장**된다 — `language` · `segments[] {start, end, text}` ·
   `rawText` · `engine` · `model` · `createdAt`.
   ⚠️ **timestamp 단위는 통합 방식마다 다르다** (§14.4.1) — CLI JSON `offsets`는 **밀리초**,
   `whisper-rs`의 segment timestamp는 **센티초**다.
   **가정으로 정규화하지 않는다.** 선택한 방식의 실제 단위를 확인하고,
   **통합 경계에서 한 번만** `start_ms` · `end_ms`로 정규화한다.
   **테스트가 ×10 / ×100 단위 오류를 잡아야 한다** — 조용히 100배 어긋난 transcript가
   저장되면 Gate는 그것을 잡지 못한다.

5. **Transcript는 immutable · versioned다 (INV-2 · §7.1) — 이것은 확정된 규칙이다.**
   재전사는 기존 Transcript를 `UPDATE`하지 않고 **새 Transcript를 추가**한다.
   Phase 1이 만든 `Recording 1:N Transcript` 스키마가 여기서 실제로 쓰인다.

   **`Recording.currentTranscriptId`를 이 Phase가 갱신한다** (§7.2):
   전사가 성공하면 새 Transcript를 current로 올리고,
   **재전사가 실패하면 기존 current를 그대로 유지한다.**

   ```text
   Transcript A = success / current
           ↓
   re-transcription attempt  →  failed
           ↓
   currentTranscript = Transcript A        (그대로)
   ```

   **실패한 시도 때문에 이미 유효한 Transcript를 잃지 않는다.** 이것은 테스트로 확인한다.

6. **Recording Detail의 Transcript 탭에서 timestamp와 함께 볼 수 있다.**

   ```text
   00:02:14 → 00:02:21
   그러면 이번에는 PLY 먼저 변환하고
   그다음 SOG 변환 확인하면 될 것 같아요.
   ```

7. **전사 실패가 제품 상태로 다뤄진다** (§13) — transcription process failure ·
   unsupported whisper model · 모델 파일 없음 · 입력 포맷 변환 실패.
   실패해도 **원본 audio와 Recording은 그대로 남는다 (INV-3)**. 재시도가 가능해야 한다.

8. **Worker / Gate / Verifier가 1시간 오디오를 전사하도록 만들지 않는다.**
   자동 검증은 **짧고 결정론적인 fixture**로 한다. 장시간 실제 성능은 Final Integration이다.

9. **자동 테스트**: whisper 출력 파싱, timestamp 변환(→ `HH:MM:SS`),
   segment 경계 처리, 잘못된/잘린 출력에 대한 방어가 whisper 실행 없이 테스트된다.
   실제 whisper 바이너리 없이 돌 수 있도록 파싱 로직과 프로세스 실행 경계를 분리한다 (§18).

9. build · lint · test Gate가 전부 통과한다.

## Important Rules

- **오디오는 기기 밖으로 나가지 않는다.** 원격 전사 API를 쓰지 않는다 (§12).
- **INV-1 / INV-2 / INV-3**: 전사는 원본 audio를 수정·삭제하지 않고, 실패해도 Recording을
  건드리지 않는다.
- 수백 MB~수 GB 모델 파일을 저장소에 커밋하지 않는다.
- §14.4의 값은 2026-09-01 기준이다. 도입 시점에 재확인한다. 추측으로 CLI 플래그나
  JSON 필드명을 쓰지 않는다 — 실제 출력으로 확인한다.
- 전사가 느리다는 이유로 정확도가 낮은 모델을 조용히 강제하지 않는다. 선택은 사용자 설정이다.
- **Git commit / push는 이 Phase의 작업이 아니다.** Phase commit은 Phase가 완료되고
  검증된 뒤 운영자가 한다 (`docs/GIT-WORKFLOW.md`). Worker가 commit하면 HEAD가 바뀌어
  Gate·Verifier가 묶여 있는 subject fingerprint가 실행 도중에 흔들린다.
  저장소에 파일을 만들고 고치는 것까지가 Task의 일이다.

## Out of Scope

- Claude · AI Note (Phase 4)
- Notion · Markdown export (Phase 5)
- 화자 분리 (diarization) — 제품 non-goal (§15)
- 실시간/스트리밍 전사 — 제품 non-goal (§15)
- transcript 편집 UI, 검색, 번역
- 여러 Recording 동시 전사 큐 (DEFERRED, §16) — 단, 한 건 전사 중 UI가 멎지 않는 것은 범위 안이다
- **Windows 빌드 · Windows 바이너리 확보 · Windows 실행 검증** (Phase 6)

## Verification Boundary

- 실제 녹음 파일이 로컬에서 전사되어 timestamp가 있는 Transcript로 저장된다.
- 앱을 재시작해도 Transcript가 남아 있다.
- whisper JSON 파싱과 timestamp 변환에 자동 테스트가 있고 통과한다.
- 모델이 없거나 전사가 실패했을 때 원본 audio가 그대로 있고, 실패가 UI에 보이며 재시도된다.
- 통합 방식 결정이 근거와 함께 기록되어 있다.
- build / lint / test Gate가 green이다.

### ⚠️ Phase 3 통합 smoke test — **연기 대상이 아니다** (운영자 결정 · 2026-09-03)

**Phase 3가 실제 Whisper 추론을 한 번도 하지 않은 채 끝나서는 안 된다** (§14.4.3).

엔지니어링 Plan이 완료된 뒤 **운영자가** 짧은 통합 smoke test를 한 번 수행한다.

```text
짧은 로컬 WAV → (필요시 변환) → 실제 Whisper 엔진 → 실제 모델
  → 추론 → segments + timestamps → Transcript 영속화
```

**품질 벤치마크가 아니다.** 실제 마이크 녹음 · 1시간 오디오 · 한국어 품질 판정 ·
성능 측정은 필요하지 않다. 짧은 알려진 fixture면 충분하다.

**이 추론이 실행되지 않으면 "end-to-end 전사가 검증됐다"고 표현하지 않는다.**

따라서 이 Phase는 **운영자가 그 절차를 실제로 실행할 수 있도록 정확한 방법을 문서로 남겨야
한다** — 모델을 어디서 얻어 어디에 두는지, 어떤 명령/조작으로 실행하는지.
자동 Gate는 계획대로 하드웨어·모델 독립을 유지한다.

### Human Review 항목 — **Final Integration으로 연기됨 (DEFERRED)**

운영자가 실제 하드웨어/품질 검증을 Final Integration에 모으기로 했다
(`ADR-0003` §12.A · `phase-prompt/Goal.md`의 hard human gate). Phase 3도 같은 정책을 따른다.

**아래는 이 Phase에서 확인되지 않으며, 확인된 것처럼 적지 않는다.**

| | |
| --- | --- |
| 실제 한국어 전사 품질 | `DEFERRED` |
| 한국어 + 영어 혼용 음성 | `DEFERRED` |
| timestamp가 실제 음성 위치와 맞는가 | `DEFERRED` |
| 1시간 전사 소요 시간 | `DEFERRED` |

**품질 검증을 하지 않았으면 PASS라고 쓰지 않는다.** `DEFERRED`로 명시한다.

이 Phase가 자동으로 판정하는 것은 **엔지니어링**뿐이다 — 프로세스 통합 · 파서 ·
영속성 규칙 · 상태 전이 · UI 상태. **전사 결과의 품질은 판정 대상이 아니다.**

## Source of Truth

`docs/PRODUCT-SPEC.md`, 특히 §2.1(INV-1~INV-3 · INV-10) · §3.1(cross-platform 원칙) ·
§7 · §8 · §12 · §13 · §14.1(로컬 환경 제약) · §14.4(whisper 확인된 사실) · §18.

외부 도구·API는 추측하지 말고 실제 현재 동작을 확인한다.
확인할 수 없으면 UNVERIFIED로 남기고 그 사실을 드러낸다.

이 Phase 밖으로 나가지 않는다.
