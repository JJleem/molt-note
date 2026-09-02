# Phase 2A — Recording Engine Validation

Implement Phase 2A of `docs/PRODUCT-SPEC.md`.

이것은 Phase 2의 **첫 번째 단계**다. Phase 2 전체 목표는
`phase-prompt/02-reliable-recording.md`에 있고, 이 단계는 그 앞에 놓인다.

## Goal

Recording engine 후보를 비교해 **잠정 선택**하고, 그 선택이 **이 Mac에서 실제로 성립하는지를
사람이 직접 확인할 수 있는 최소 spike**를 만든다.

이 단계의 성공 기준:

> 사람이 앱을 실행해 마이크를 고르고 10~20초를 녹음하면, 실제 오디오 파일이 만들어지고
> 그 파일의 장치 이름 · 경로 · 포맷 · 크기를 확인할 수 있다.
> 그리고 그 결과가 잠정 결정을 지지하는지 반박하는지 판단할 수 있다.

## Why This Stage Exists

Phase 2는 제품의 심장이고, engine 선택은 되돌리기 비싼 결정이다.

그런데 이 결정을 좌우하는 사실 중 일부는 **자동으로 확인할 수 없다.**
실제 마이크 권한 프롬프트, 실제로 만들어지는 컨테이너/코덱, 실제 음질은
사람이 앱을 실행해야만 알 수 있다.

engine을 확정한 뒤 캡처 · lifecycle · 영속성 · UI를 전부 구현하고 나서야 사람이 처음
실제 장치를 확인한다면, 가정이 틀렸을 때 되돌릴 것이 너무 많다.

```text
engine 확정 → 대량 구현 → 사람이 처음 확인 → 가정과 다름 → 대규모 재작업
```

그래서 **실제 장치 증거를 앞으로 당긴다.** 이 단계는 그 증거를 얻기 위한 최소한의 작업만 한다.

## Required Outcome

### A. 잠정 기술 결정

1. **후보를 §6.1의 기준 전체로 비교한다.**

   ```text
   WebView / MediaRecorder      vs      Rust/native audio path (예: cpal)
   ```

   근거는 세 종류이며 **ADR이 이를 구분해야 한다** (§18):

   | 근거 | 내용 |
   | --- | --- |
   | 저장소 근거 | `docs/PRODUCT-SPEC.md` §14.3에 이미 조사된 사실. **재조사하지 않고 인용한다** |
   | 자동 검증 | 이 저장소에서 컴파일·테스트로 확인되는 것 (의존성 해석 · 버전 · 장치 열거 결과 · 파일 포맷) |
   | Human Review | 실행 중인 앱과 실제 하드웨어가 있어야만 알 수 있는 것 |

2. **`docs/ADR-0003-recording-engine.md`를 작성하되 최종 확정으로 쓰지 않는다.**
   사람의 장치 검증이 아직 없으므로 상태를 그렇게 표기한다. 예:

   ```text
   Status: PROVISIONAL — pending human device validation
   ```

   ADR에 담을 것: 검토한 후보 · 잠정 근거 · 자동/저장소 근거 · **사람이 확인해야 할 항목** ·
   잠정 선택 · 탈락 후보와 이유 · 알려진 한계 · Windows 함의 · Phase 3 포맷 함의.

3. **평가의 비대칭을 명시한다.**
   native 경로의 핵심 근거는 자동 검증 가능하고, webview 경로의 핵심 미지수는 자동으로
   확인할 수 없다. **"검증하기 쉬운 쪽"이 그 이유만으로 선택되면 그것은 근거가 아니라 편향이다.**
   탈락 사유가 *확인된 사실* 때문인지 *확인하지 못해서*인지 구분해서 적는다.

### B. 최소 spike

4. **잠정 선택된 engine으로 최소 캡처 경로를 만든다.**

   ```text
   입력 장치 열거
   → 장치 하나 선택
   → 선택된 장치 열기
   → 약 10~20초 캡처
   → 정지
   → 로컬 오디오 파일 확정
   → 보고: 장치 이름 · 출력 경로 · 포맷 · 파일 크기
   ```

5. **사람이 실제로 실행할 수 있어야 한다.** 이것이 이 단계의 존재 이유다.
   실행 방법을 `docs/ADR-0003-recording-engine.md` 또는 별도 문서에 **정확한 명령/조작 순서로** 적는다.

   ⚠️ **macOS 권한 프롬프트는 번들된 `.app`에서 확인해야 의미가 있다.**
   터미널에서 실행하면 권한은 터미널 앱에 귀속되며, 제품이 겪을 상황과 다르다 (§14.3).
   따라서 spike는 **번들된 앱에서 실행 가능해야 한다.**

6. **spike 표면은 임시임이 명확해야 한다.** Phase 2B가 이것을 production UI로 대체한다.
   임시 표면을 최종 Recording 화면처럼 만들지 않는다.

7. **Phase 1 경계를 재사용한다** — `AppDataDirectory` · Tauri command 경계 · 실패 표현.
   같은 것을 새로 만들지 않는다.

8. build · lint · test Gate가 전부 통과한다.
   **자동 테스트는 실제 마이크나 마이크 권한의 존재를 전제하지 않는다.**
   Gate가 도는 환경에 장치가 없을 수 있고, 그때 빨개지는 것은 제품 결함이 아니다.

## Out of Scope — 이 단계에서 구현하지 않는다

```text
완성된 Recording 화면 UX
전체 pause / resume lifecycle
재시작 영속성 workflow
production playback UI
장시간 녹음 테스트
전체 실패 UX
Recording DB 영속화 파이프라인
```

전부 Phase 2B가 한다. 여기서 미리 만들면 **잘못된 engine 위에 쌓는 위험**이 되살아난다.

그리고 Phase 3 이후의 것 일체 — whisper · 전사 · AI Provider · Notion · Markdown export.

Windows 빌드 · 실행 · 권한 로직도 하지 않는다 (Phase 6). Windows는 §6.1의 **평가 근거로만** 다룬다.

## Verification Boundary

- `docs/ADR-0003-recording-engine.md`가 존재하고 §6.1 기준 전체를 다루며,
  상태가 최종 확정이 아니라 **사람 검증 대기**로 표기되어 있다.
- 자동 검증 · 저장소 근거 · Human Review가 ADR에서 구분되어 있고,
  사람만 알 수 있는 항목이 자동 검증된 것처럼 적혀 있지 않다.
- spike의 순수 로직(장치 목록 정규화 · 파일 경로 결정 · 포맷 기술)이
  하드웨어 없이 자동 테스트된다.
- 사람이 spike를 실행하는 방법이 문서에 정확히 적혀 있다.
- build / lint / test Gate가 green이고 기존 테스트가 깨지지 않는다.

### Human Review 항목 — 이 단계의 핵심 산출물

**이 목록이 다음 단계로 넘어가도 되는지를 결정한다.** 자동 Gate가 대신 판정하지 않는다.

1. macOS 마이크 권한 프롬프트가 실제로 뜨는가
2. 선택한 물리 마이크가 실제로 사용되는가
3. 장치가 정상적으로 열리는가
4. 10~20초 녹음이 성공하는가
5. 출력 파일이 실제로 존재하는가
6. 녹음된 음성이 알아들을 수 있는가
7. 실제 컨테이너 / 코덱 / PCM 포맷이 기대와 일치하는가 (§14.4의 whisper 입력 요구와 대조)
8. Stop / 확정이 손상된 파일 없이 끝나는가

**1시간 테스트가 아니다.** 짧은 smoke recording이면 충분하다.

## Source of Truth

`docs/PRODUCT-SPEC.md`, 특히 §2.1 · §3.1 · §6(R-001~R-005) · §6.1(평가 기준) ·
§13 · §14.3(확인된 사실) · §14.4(whisper 입력 요구) · §18 · §19.

`phase-prompt/02-reliable-recording.md`가 Phase 2 전체 목표다. 이 단계는 그것을 대체하지 않는다.

- Git commit / push는 이 단계의 작업이 아니다 (`docs/GIT-WORKFLOW.md`).
- 외부 라이브러리는 추측하지 말고 확인한다. 확인할 수 없으면 UNVERIFIED로 남긴다.

이 단계 밖으로 나가지 않는다.
