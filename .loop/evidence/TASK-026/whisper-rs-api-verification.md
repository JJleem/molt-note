# TASK-026 — `whisper-rs` API 표면 확인 (컴파일러로 확인한 사실)

```text
Date:   2026-09-03
Task:   TASK-026 — 실제 whisper 실행 경계
근거:   이 기기에서 실제로 crate를 해석·빌드한 결과. 문서 인용이 아니다.
```

ADR-0007 §14는 다음 두 항목을 **[E4] UNVERIFIED**로 남기고 *"crate를 실제로 추가하는
TASK-026이 빌드로 확인한다"* 고 적었다. 이 파일이 그 확인 결과다.

## 1. 해석된 버전 — §14.4.1의 기록과 일치한다

`src-tauri/Cargo.lock` (이 Task가 `whisper-rs = "0.16"`을 추가한 뒤 cargo가 채운 값):

```text
name = "whisper-rs"      version = "0.16.0"
  checksum = 2088172d00f936c348d6a72f488dc2660ab3f507263a195df308a3c2383229f6
name = "whisper-rs-sys"  version = "0.15.0"
  checksum = 6986c0fe081241d391f09b9a071fbcbb59720c3563628c3c829057cf69f2a56f
```

PRODUCT-SPEC §14.4.1이 2026-09-03에 기록한 *"0.16.0 · `whisper-rs-sys` 0.15.0"* 과 같다.
그 기록은 이제 **이 저장소에서 실제로 해석되는 값**이다.

## 2. 실제 API 표면 — 기록과 다른 부분이 있었다

`WhisperState`의 segment 접근 API가 §14.4.1의 서술과 다르다. **추측으로 맞추지 않고
컴파일러가 알려준 실제 시그니처를 썼다** (`.loop/evidence/TASK-026/`의 이 기록이 그 출처다).

| 쓰는 것 | 실제 시그니처 | 확인 방법 |
| --- | --- | --- |
| 모델 적재 | `WhisperContext::new_with_params(&str, WhisperContextParameters) -> Result<_, WhisperError>` | 그대로 컴파일됐다 |
| 상태 생성 | `WhisperContext::create_state() -> Result<WhisperState, WhisperError>` | 그대로 컴파일됐다 |
| 추론 | `WhisperState::full(FullParams, &[f32]) -> Result<_, WhisperError>` | 그대로 컴파일됐다 — **f32 슬라이스를 직접 받는다** (§14.4.1과 일치) |
| segment 수 | `WhisperState::full_n_segments() -> i32` | 그대로 컴파일됐다 |
| **segment 접근** | **`WhisperState::get_segment(i32) -> Option<WhisperSegment<'_>>`** | `full_get_segment_text/_t0/_t1`은 **없다** (E0599). 타입은 컴파일 오류로 확인했다 |
| **segment 시각** | **`WhisperSegment::start_timestamp() -> i64` · `end_timestamp() -> i64`** | 같은 방법 (E0308이 `i64`를 보고했다) |
| **segment 텍스트** | **`WhisperSegment::to_str() -> Result<&str, WhisperError>`** | 같은 방법 |
| **언어 id** | **`WhisperState::full_lang_id_from_state() -> i32`** (`full_lang_id`가 아니다) | 같은 방법 |
| 언어 문자열 | `whisper_rs::get_lang_str(i32) -> Option<&'static str>` | 그대로 컴파일됐다 |

확인 절차: 시그니처를 모르는 호출마다 `let _: () = <호출>;`을 두고 컴파일해 **컴파일러가
실제 타입을 보고하게 했다.** 그 probe 코드는 결과를 옮겨 적은 뒤 제거했다. 최종 소스에는
남아 있지 않다 (`src-tauri/src/transcription/whisper.rs`).

ADR-0007 §5의 표는 `WhisperSegment::start_timestamp()`를 기록했고 **그 이름은 실재한다.**
다만 그 값을 얻는 경로가 `get_segment(i) -> Option<_>`라는 것은 이 확인으로 처음 드러났다.

## 3. 여전히 확인되지 않은 것 — 확인한 것처럼 적지 않는다

| 항목 | 상태 | 왜 |
| --- | --- | --- |
| **`start_timestamp()`의 단위가 실제로 센티초인가** | **UNVERIFIED** | 타입이 `i64`라는 것만 확인했다. 단위는 **실제 추론을 한 번 돌려야** 드러나며, 그것은 운영자 smoke test의 몫이다 (PRODUCT-SPEC §14.4.3 · TASK-031). 그래서 `parse.rs`의 계수(×10)를 **바꾸지 않았다** — 근거는 여전히 §14.4.1의 [E2] 기록이다 |
| **번들 whisper.cpp가 v1.8.3인가** | **UNVERIFIED** | 빌드는 성공했지만 이 Task는 번들 버전을 읽는 경로를 확인하지 않았다. 그래서 `engine_id`에 whisper.cpp 버전을 **적지 않는다** — 모르는 값을 provenance로 남기지 않는다 |
| **실제 추론이 성공하는가 (end-to-end)** | **DEFERRED** | 운영자 smoke test (§14.4.3). 이 Task의 자동 검증은 모델도 whisper 실행도 요구하지 않는다 |
| **codesign / notarization** | **DEFERRED** | 배포 검증 경계 (ADR-0007 §6) |
| **Windows 빌드** | **DEFERRED — Phase 6** | ADR-0007 §11 |

## 4. Gate 비용 — ADR-0007 §4.3의 UNVERIFIED 항목

ADR-0007 §4.3은 *"cold build가 whisper.cpp 컴파일을 포함한다 · 최초 1회 컴파일이 900초
한도를 넘을 수 있다 — [E4], TASK-026에서 관측한다"* 라고 적었다.

**관측값**: `whisper-rs`를 추가한 뒤 **처음 돌린** lint Gate
(`eslint . && cargo clippy --all-targets -- -D warnings`)가 **27.7초**에 끝났다
(그 실행은 이 Task의 코드 오류로 exit 101이었지만, `whisper-rs` · `whisper-rs-sys`는
그 안에서 **성공적으로 빌드됐다** — 실패는 우리 crate의 컴파일 단계에서 났다).

```text
관측: 27.7s   한도: 900s
```

**한도를 넘지 않았다.** 다만 다음은 이 관측이 말하지 않는다:

- 이 기기의 cargo registry 캐시에 tarball이 이미 있었는지 아닌지는 확인하지 않았다.
- 완전히 비어 있는 `target/`과 빈 registry에서의 시간은 측정하지 않았다.
- release 빌드(`cargo build --release` · 번들)는 이 Task가 돌리지 않았다.

그래서 결론은 **"이 환경에서 관측된 첫 실행은 한도 안이었다"** 까지다.
