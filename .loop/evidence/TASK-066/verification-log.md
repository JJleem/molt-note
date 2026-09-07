# TASK-066 — 확인 기록 (무엇을 어디서 읽었는가 / 무엇을 확인하지 못했는가)

```text
Run       RUN-20260906T005147Z-TASK-066
Task      TASK-066 — ADR-0007 갱신 (문서 전용)
날짜      2026-09-06
```

이 Task는 **문서만 바꾼다.** 아래는 그 문서에 적은 각 사실이 어디서 왔는지의 기록이다.
확인하지 못한 것은 확인하지 못했다고 적는다 (PRODUCT-SPEC §20.2).

---

## 1. 이 Run이 저장소에서 직접 읽어 확인한 것 — [E1]

| 사실 | 읽은 파일 | 확인된 값 |
| --- | --- | --- |
| `whisper.rs`가 `set_language` · `set_detect_language`를 **부르지 않는다** | `src-tauri/src/transcription/whisper.rs` (137~147행) | 설정하는 것은 `set_n_threads` · `set_translate(false)` · `set_print_special` · `set_print_progress` · `set_print_realtime` · `set_print_timestamps` 뿐이다 |
| DB의 `language`가 `full_lang_id_from_state()`를 되읽은 값이다 | 같은 파일 179행 | `let language = whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_owned);` |
| `Cargo.toml`에 `whisper-rs`의 feature 지정이 **없다** | `src-tauri/Cargo.toml` 48행 | `whisper-rs = "0.16"` — `features = [...]`가 붙어 있지 않다 |
| 잠긴 crate 버전 | `src-tauri/Cargo.lock` 4673~4685행 | `whisper-rs` **0.16.0** (checksum `2088172d00f936c348d6a72f488dc2660ab3f507263a195df308a3c2383229f6`) · `whisper-rs-sys` **0.15.0** (checksum `6986c0fe081241d391f09b9a071fbcbb59720c3563628c3c829057cf69f2a56f`) |
| Spec이 Metal feature를 적어 둔 자리 | `docs/PRODUCT-SPEC.md` **901행** (§14.4 "Rust 바인딩" 문단) | `**Rust 바인딩**: whisper-rs 0.16.0 (whisper-rs-sys 0.15.0, Metal feature).` |
| Spec §D가 `language` 설정을 요구한다 | `docs/PRODUCT-SPEC.md` 223~228행 (§5 D. Settings) | `| Transcription | whisper model · language · automatic transcription ON/OFF |` |
| Spec §14.4의 모델 크기 항목이 UNVERIFIED다 | `docs/PRODUCT-SPEC.md` 893~899행 | *"…`large-v3` / `large-v3-turbo`가 현실적이라는 것은 추론이며 … (UNVERIFIED). Phase 3에서 실측한다"* |
| Gate timeout | `.loop/project.yaml` 54~67행 | `build` 600초 · `lint` 900초 · `test` 900초. lint/test 주석: *"cold Rust 빌드를 포함할 수 있다"* |

### 줄 번호에 대한 단서

`phase-prompt/05.6` R-3은 Metal feature 줄을 **`PRODUCT-SPEC:836`** 으로 인용한다.
**이 Run이 오늘 같은 파일에서 다시 찾은 줄 번호는 901이다.** 인용된 문장 자체는 같다.
줄 번호는 문서가 자라면 움직이므로 ADR §17.2.1은 **줄 번호가 아니라 §14.4 문단**에 인용을
걸고, 이 어긋남을 그 자리에 적어 두었다. (§17.4의 `Spec:838` 인용도 같은 사정이다 —
오늘의 위치는 898~899행이다.)

---

## 2. 2026-09-05 실사용 관측에서 온 것 — [E5]

**이 Run이 관측한 것이 아니다.** 운영자가 실제로 앱을 실행해 얻은 값이며, 저장소 안의
아래 두 문서에 이미 기록돼 있다. ADR §17은 그것을 **관측으로** 옮겨 적었고 [E1]/[E2]로
올리지 않았다.

| 출처 | 무엇 |
| --- | --- |
| `docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md` 부록 (555~686행) | 실행 조건 · 결과 1~5 · §10 표에 대한 답 |
| `phase-prompt/05.6-transcription-correctness-and-reach.md` Preconditions R-1 ~ R-6 | 같은 실행의 정리 |

옮겨 적은 값:

```text
segment 총 개수      1,711
고유 문장             59  (3.4%)
한글이 나온 줄         0
최다 반복 문장       1,063회 (62.1%)
상위 2문장            86.7%
DB의 language         en
붕괴 구간            00:28:10 ~ 00:56:55
오디오               capture-1788522158.wav · 48 kHz mono PCM16 · 1:12:51 · 한국어 3인 회의
모델                 ggml-base.bin
PASS-1 ~ PASS-4      전부 통과 (timestamp 단위 = 센티초 확인 포함)
```

whisper.cpp 기본값 (`whisper.cpp/src/whisper.cpp:5943 — whisper_full_default_params`):

```c
/*.language        =*/ "en",
/*.detect_language =*/ false,
```

반복 방지 장치 (whisper.cpp 기본값): `no_context = true` · `temperature_inc = 0.2` ·
`entropy_thold = 2.4` · `logprob_thold = -1.0` — **전부 켜져 있었다.**

`whisper-rs-sys-0.15.0/build.rs` (258~265행)가 `GGML_METAL = OFF`를 정의한다.
CoreML · OpenMP도 OFF, Accelerate만 링크된다.

**이 Run은 위 두 C/Rust 파일(whisper.cpp 소스 · whisper-rs-sys build.rs)을 다시 읽지
못했다** — §3을 볼 것. 그래서 ADR은 이 항목들을 [E5]로만 적었다.

---

## 3. 확인하지 **못한** 것 — Metal feature의 정확한 이름 (**UNVERIFIED**)

Task가 요구한 것: *"실제 crate 소스나 그 crate의 공식 문서에서 확인해 확인 경로와 버전을
함께 적는다. 확인하지 못했으면 UNVERIFIED로 표시한다."*

**확인하지 못했다.** 시도한 경로와 각각의 실패 이유:

| 시도 | 결과 |
| --- | --- |
| `Read` — `/Users/molt/.cargo/registry/src/index.crates.io-*/whisper-rs-0.16.0/Cargo.toml` | **거부됨.** 이 Run은 작업 디렉터리(`/Users/molt/orca/projects/molt-note`) 밖의 파일을 읽을 권한이 없다 |
| `Bash` — cargo registry 디렉터리 나열 | **차단됨.** *"may only list files in the allowed working directories for this session"* |
| `WebFetch` — `https://docs.rs/crate/whisper-rs/0.16.0/features` | **거부됨.** 권한이 부여되지 않았다 (비대화형 세션) |
| `WebSearch` — crate feature 목록 | **거부됨.** 같은 이유 |
| `src-tauri/Cargo.lock` | 읽었다. **버전과 checksum은 확인됐지만 lock 파일은 crate가 어떤 feature를 갖는지 기록하지 않는다** |
| 저장소 안의 evidence (`.loop/evidence/**`) | `grep -ri metal` → **일치 없음.** 이전 Task들은 Metal을 다룬 적이 없다 |

**남은 정황 (정황이지 확인이 아니다):** 2026-09-05에 운영자의 **저장소 밖 검증용 도구**가
`whisper-rs`를 `features = ["metal"]`로 빌드해 실행했고 whisper.cpp가 GPU를 잡았다고
보고했다 (`GPU name: Apple M5` · `use gpu = 1` · `backends = 3`) — 부록 결과 2·3 [E5].
그 실행에서는 언어 · Metal · 청크 분할이 **함께** 바뀌었으므로 개별 기여도도 분리되지 않았다.

→ ADR §17.2.2가 이 표를 그대로 담고 **UNVERIFIED (2026-09-06)** 로 표시했다.
   §14의 표에도 `Metal을 켜는 whisper-rs 0.16의 정확한 feature 이름 | UNVERIFIED [E4]` 행을
   새로 추가했다.

---

## 4. 문서에 적지 **않은** 것 — 측정되지 않은 속도

Task와 `phase-prompt/05.6` R-3이 금지한 것: *"켰을 때의 속도 변화는 이 프로젝트에서
측정된 적이 없으므로 어떤 배수도 어떤 수치도 적지 않는다."*

부록에 존재하지만 **ADR에 옮기지 않은 값** (의도적으로 제외했다):

```text
26분 · 1.3분 · 6.0분 · 6.1분 · "실시간의 약 12배" · "26분 → 1.3분"
```

ADR §17.2.3은 그 대신 **왜 그 값들을 쓸 수 없는지**를 적었다 — 언어 수정과 Metal이 함께
적용됐고 비교 대상이 붕괴한 디코딩이었으며, 두 요인을 분리한 측정은 이루어지지 않았다.

ADR에 남아 있는 시간 값은 **이번에 새로 들어간 것이 아니거나 Metal 속도가 아닌 것**뿐이다:

| 값 | 무엇인가 | 출처 |
| --- | --- | --- |
| 900초 / 600초 | Gate timeout **설정값** | `.loop/project.yaml` [E1] |
| 27.7초 | TASK-026이 **이미 §4.3에 기록한** 측정값 (Metal이 꺼진 구성의 lint Gate) | §4.3 (기존 문장) |
| 1:12:51 · 72분 | **오디오의 길이** (속도가 아니다) | 부록 [E5] |
| 00:28:10 ~ 00:56:55 | 붕괴가 관측된 **오디오 위치** (속도가 아니다) | 부록 [E5] |

---

## 5. Gate

이 Task의 `stop_condition.gates`는 **비어 있다** (`.loop/tasks/TASK-066.yaml`).
그래서 이 Run은 Gate 명령을 실행하지 않았다. 변경은 `docs/` 아래 markdown 한 개뿐이며
빌드·린트·테스트 대상 파일을 건드리지 않았다 (§6의 파일 목록을 볼 것).
