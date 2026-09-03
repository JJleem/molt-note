# TASK-024 — 확인한 것 / 확인하지 못한 것

`CLAUDE.local.md`의 외부 의존성 규칙에 따라 **VERIFIED와 UNVERIFIED를 구분해서** 적는다.
추측한 것을 확인한 것처럼 적지 않는다.

---

## 1. rubato 5.0.0의 실제 API — VERIFIED (빌드로 확인)

ADR-0007 §9.2는 crate와 버전(`rubato` 5.0.0, 2026-08-10)만 기록했고 **타입 이름은 적지
않았다.** 이 Task는 crate를 실제로 추가해 컴파일러로 확인했다.

`src-tauri/Cargo.lock`이 잠근 값:

```text
name = "rubato"
version = "5.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "a7cb1ffaf8738df50aab642a7f6465df81c6ba9e2818268053487165298114be"
```

### 확인 방법

`use` 목록에 후보 이름을 넣고 `npm run lint`(= `cargo clippy -D warnings`)를 돌렸다.
`unresolved import`가 나면 없는 것이고, `unused import`가 나면 있는 것이다.

### 결과 — 흔히 인용되는 0.16 시절의 이름은 5.0.0에 **없다**

| 이름 | 5.0.0 루트에 있는가 | 근거 |
| --- | --- | --- |
| `FftFixedIn` · `FftFixedInOut` · `FftFixedOut` | **없다** | `error[E0432]: no 'FftFixedIn' in the root` |
| `SincFixedIn` · `SincFixedOut` · `FastFixedIn` | **없다** | 같은 E0432 |
| `VecResampler` · `Fixed` | **없다** | 같은 E0432 |
| `Fft` | **있다** | `unused import: rubato::Fft` |
| `FixedSync` · `FixedAsync` | **있다** | `unused import` |
| `Resampler` · `Sample` · `Indexing` · `Async` | **있다** | `unused import` |
| `ResampleError` · `ResampleResult` · `ResamplerConstructionError` | **있다** | `unused import` |
| `SincInterpolationParameters` · `SincInterpolationType` · `WindowFunction` · `PolynomialDegree` | **있다** | `unused import` |
| `rubato::audioadapter_buffers::direct::InterleavedSlice` | **있다** | 컴파일 통과 |
| 모듈 `fft` · `sinc` · `asynchro` · `synchro` · `windows` | **비공개** | `error[E0603]: module 'sinc' is private` |

### 확인된 시그니처

```rust
Fft::<f32>::new(
    sample_rate_input: usize,
    sample_rate_output: usize,
    chunk_size: usize,
    nbr_channels: usize,
    fixed: FixedSync,
) -> Result<Self, _>
```

인자 수와 타입은 컴파일러가 직접 알려줬다:

```text
error[E0061]: this function takes 5 arguments but 6 arguments were supplied
error[E0308]: arguments to this function are incorrect
    |                                         --  --  --  --  -- expected `FixedSync`, found `()`
    |                                         |   |   |   |
    |                                         |   |   |   expected `usize`, found `()`   (×4)
note: associated function defined here
   --> rubato-5.0.0/src/synchro.rs:214:12
```

5.0.0은 버퍼를 `audioadapter` 크레이트로 받는다:

```text
error[E0277]: the trait bound `[Vec<f32>; 1]: rubato::audioadapter::Adapter<f32>` is not satisfied
error[E0608]: cannot index into a value of type
              `rubato::audioadapter_buffers::owned::InterleavedOwned<f32>`
```

그래서 `process()`(→ `InterleavedOwned`) 대신
`process_into_buffer(&InterleavedSlice, &mut InterleavedSlice, None) -> (usize, usize)`를 쓴다.

### 리샘플러가 보고한 실측값 (48 kHz → 16 kHz, chunk 1024, 1채널)

```text
delay=43  in_next=1024  out_max=344  out_next=258
```

`output_delay()`가 **43 프레임**(16 kHz 기준 ≈ 2.7 ms)이다. 이 값은 코드에 상수로 박지 않고
매번 리샘플러에게 묻는다. `the_resampler_reports_the_delay_this_module_compensates_for`가
"0이 아니다"와 "입력 chunk가 고정이다"만 고정한다 — 43이라는 수 자체를 고정하면 crate 갱신이
곧 테스트 실패가 된다.

> ⚠️ 이 Run은 `~/.cargo/registry` 아래의 crate 소스를 읽을 권한이 없었다. 위 사실은 전부
> **컴파일러 진단과 런타임 값**에서 나왔지 문서를 읽어 옮긴 것이 아니다.

---

## 2. 지연 보정에 실제로 효력이 있는가 — VERIFIED (mutation 확인)

테스트가 장식이 아닌지 확인하기 위해 **코드를 일부러 망가뜨려 테스트가 잡는지 봤다.**

| 단계 | 변경 | 결과 |
| --- | --- | --- |
| 1 | 원래 코드 (`out.drain(..delay)`) | 전 테스트 통과 |
| 2 | `out.drain(..0)` 으로 지연 보정 제거 | **처음엔 그대로 통과했다** — 당시 테스트는 전환점 앞뒤 100프레임씩 여유를 뒀는데 실제 밀림이 43프레임이라 여유 안에 숨었다 |
| 3 | 테스트를 "소리가 시작하는 자리를 직접 잰다"로 바꿈 | 망가진 코드에서 **실패**: `소리가 시작하는 자리가 밀렸다: 1643 (기대 1600 ±20)` |
| 4 | `out.drain(..delay)` 복구 | 다시 통과 |

2단계가 이 Task에서 실제로 관측된 사실이다. **느슨한 검사는 43프레임 밀림을 통과시킨다.**
그래서 `a_sound_stays_where_it_was_in_time_after_resampling`은 앞뒤 구간 비교가 아니라
**onset 위치를 값으로** 잰다. DC 신호만 보는 검사(`resampling_carries_the_signal...`)로는
시간 밀림을 볼 수 없다는 것도 같은 이유로 확인됐다.

---

## 3. 원본 불변 — VERIFIED (테스트)

| 확인 | 수단 |
| --- | --- |
| 변환 전후 원본 바이트가 완전히 동일하다 | `the_source_file_is_byte_for_byte_identical_after_conversion` — `fs::read` 결과를 `assert_eq!` |
| 원본을 쓰기로 열지도 않았다 | 같은 테스트가 mtime(`metadata().modified()`)까지 비교한다 |
| 파생 파일이 원본 옆에 생기지 않는다 | `conversion_leaves_no_derived_file_anywhere_near_the_source` — 디렉터리 목록을 전후로 비교 |
| 파생 산출물의 경로가 원본 경로와 같아질 수 없다 | **구조로 보장된다** — `TranscriptionInput`에 경로 필드가 없고, `audio_input.rs`에 `File::create` · `fs::write` · `WavWriter`가 하나도 없다. 쓸 방법 자체가 없다 |
| 실패해도 원본이 안전하다 | 모든 실패 테스트가 `failure.source_data_safe`와 `path.is_file()`을 확인한다 |

---

## 4. 오디오 fixture를 커밋하지 않는다 — VERIFIED

- `.gitignore` 70행이 `*.wav`를 **경로와 무관하게** 제외한다 (`/models/` · `*.bin`과 같은 절).
- 테스트는 `std::env::temp_dir()` 아래 고유 디렉터리에 WAV를 **직접 합성**하고 `Drop`에서
  지운다 (`TempDir` · `pcm16_wav` · `raw_wav_header`). 저장소 안의 fixture를 읽지 않는다.
- `git status --porcelain` 결과에 오디오 파일이 없다:

```text
 M src-tauri/Cargo.lock
 M src-tauri/Cargo.toml
 M src-tauri/src/lib.rs
?? src-tauri/src/transcription/
```

(그 밖의 `??` 항목은 이 Task 이전부터 있던 TASK-023 산출물과 Task 파일이다.)

---

## 5. 확인하지 못한 것 — UNVERIFIED

이 Task가 **성립했다고 주장하지 않는** 것들이다.

| 항목 | 왜 확인하지 못했는가 |
| --- | --- |
| **`whisper-rs`가 이 버퍼를 실제로 받는가** | `whisper-rs`는 아직 의존성이 아니다 (TASK-026). 이 모듈이 맞춘 것은 ADR-0007 §9.1이 기록한 **16 kHz · mono · f32**라는 요구값이지, 실제 함수 호출이 아니다. ADR-0007 §14도 `whisper-rs`의 정확한 시그니처를 **[E4] UNVERIFIED**로 남겨 뒀다 |
| **실제 전사 품질** | 엔진도 모델도 없다. DEFERRED — ADR-0007 §14 · 운영자 smoke test |
| **실제 장치 녹음(48 kHz stereo)에 대한 동작** | 이 Run은 합성 WAV로만 검증했다. 실제 장치 검증은 ADR-0003 §12의 사람 몫으로 남아 있다 |
| **rubato의 리샘플 품질이 전사 정확도에 충분한가** | 신호가 유지되는지(DC 유지·onset 위치)는 확인했지만 **음성 인식 정확도에 미치는 영향**은 측정하지 않았다 |
| **3채널 이상 입력을 실제로 만드는 장치가 있는가** | 확인하지 않았다. 그래서 그 경우를 추측으로 지원하지 않고 정의된 실패로 돌려준다 (§아래) |

---

## 6. 이 Task가 내린 판단 (Verifier가 볼 자리)

| 판단 | 근거 |
| --- | --- |
| **새 `FailureKind`를 만들지 않았다** | `FailureKind`는 `src/ipc/failure.ts`의 union과 1:1이고 `failure.rs`의 테스트가 그것을 강제한다. 화면을 다루지 않는 이 Task가 종류를 늘리면 frontend가 모르는 값이 생긴다. 대신 `Storage`(못 읽음)와 `InvalidInput`(내용이 규칙에 안 맞음)을 뜻대로 나눠 썼다 |
| **3채널 이상을 평균으로 뭉개지 않고 거절한다** | ADR-0007 §9.2가 적은 다운믹스는 stereo → mono다. 3채널 이상은 배치마다 가중치가 다르고, ADR은 *"근거 없는 custom DSP를 직접 구현하지 않는다"* 고 못박았다. Task 서술의 *"예상과 다른 채널 수는 제품 실패로 돌려준다"* 와도 같은 방향이다 |
| **헤더가 말하는 길이로 메모리를 미리 잡지 않는다** | 손상된 헤더가 실제 파일과 무관한 수를 담을 수 있다. 그 수를 믿고 예약하면 잘린 파일 하나가 앱 메모리를 가져간다 |
| **읽으면서 바로 mono로 접는다 (중간 버퍼를 만들지 않는다)** | interleave된 원본을 통째로 `f32`로 펼쳤다가 접으면 최대 사용량이 두 배가 된다. 1시간 48 kHz stereo면 펼친 중간 버퍼만 약 1.4 GB이고, 접고 나면 어차피 쓰지 않는다. `phase-prompt/03` 요구 3이 "1시간 분량"을 명시적으로 다루므로 그 크기를 무시하지 않았다. **계산은 동일하다** — mono 입력에서는 나누는 수가 1이라 값이 그대로 남고, `an_input_that_is_already_16khz_mono_passes_through_untouched`가 그것을 정확한 값 비교로 확인한다 |
| **파생 입력을 파일로 만들지 않았다** | ADR-0007 §2.1이 고른 `whisper-rs`가 받는 것은 파일이 아니라 `f32` 슬라이스다 (§4.1 · §9.1 · §15). sidecar였다면 파일이 필요했겠지만 그 후보는 §12.1에서 탈락했다 |
