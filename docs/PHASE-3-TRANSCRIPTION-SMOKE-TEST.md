# Phase 3 — 운영자 전사 smoke test 절차와 검증 기록표

```text
Status:   절차 준비됨 · smoke test는 아직 실행되지 않았다
          **2026-09-03 운영자 결정으로 다음 Final/Integration review까지 연기됨 (DEFERRED)**
Date:     2026-09-03
Phase:    Phase 3 — Local Transcription
Task:     TASK-031 (문서 전용)
근거:     PRODUCT-SPEC §14.4.3 · phase-prompt/03-local-transcription.md
          Verification Boundary · docs/ADR-0007-transcription-engine.md
```

이 문서는 두 가지다.

1. **§1~§9** — 운영자가 그대로 따라 할 수 있는 실제 Whisper 추론 smoke test 절차.
2. **§10~§11** — 이 Phase가 **확인한 것과 확인하지 않은 것**을 구분하는 기록표.

> ⚠️ **이 문서가 절차를 적었다는 사실은 smoke test가 실행됐다는 뜻이 아니다.**
> §11의 실행 기록이 비어 있는 동안 Phase 3를 **"end-to-end 전사가 검증됐다"고 표현하지
> 않는다** (PRODUCT-SPEC §14.4.3).

## ⚠️ 2026-09-03 — 이 smoke test는 연기됐다 (DEFERRED)

운영자가 사정상 실제 음성 테스트를 **다음 Final/Integration review**에서 수행하기로 했다.

```text
Phase 3 engineering:              DONE
Automated verification:           PASS
Actual Whisper inference executed: NO
Actual Whisper inference:         DEFERRED
Actual transcription verified:    NO
Risk accepted by user:            YES  (2026-09-03)
```

**이것을 PASS로 간주하지 않는다.** `DEFERRED`는 "하지 않기로 하고 뒤로 미뤘다"는 뜻이며
"문제없다"는 뜻이 아니다. §1~§9의 절차는 그대로 유효하며 실행 시점만 옮겨졌다.

그때 반드시 다음 순서의 실제 E2E를 수행한다.

```text
Recording → Stop → 실제 Whisper 전사 → Transcript 표시 → AI Note 생성
```

**Whisper 단계가 실패하면 AI Note 품질/통합 테스트로 넘어가지 않는다.**
먼저 전사 문제를 고친다.

---

## 0. 표기 — 확인한 것과 확인하지 못한 것을 섞지 않는다

| 표기 | 뜻 |
| --- | --- |
| **[E1] 저장소에서 직접 확인** | 이 문서를 쓴 Run이 저장소의 실제 파일을 읽어 확인했다. 파일 경로를 함께 적는다 |
| **[E2] 저장소 문서의 기록** | PRODUCT-SPEC / ADR / 이전 Task의 Evidence가 기록한 값. 그 기록의 근거는 해당 문서에 있다 |
| **[E4] UNVERIFIED** | 이 Run에서 확인하지 못했다. 실행해 보면 드러난다 — 확인한 것처럼 적지 않는다 |

이 Run에는 **네트워크 접근도, 앱 실행도, macOS 도구 실행도 없었다.** 그러므로
**모델 다운로드 URL과 WAV를 만드는 macOS 명령은 [E4]다.** 저장소 안의 경로 · 파일명 ·
명령 · 화면 문구는 전부 [E1]이며 출처를 함께 적었다.

---

## 1. 이 smoke test가 판정하는 것 — 그리고 판정하지 않는 것

**판정하는 것은 하나다** (PRODUCT-SPEC §14.4.3):

```text
짧은 로컬 WAV → 파생 입력 변환 → 실제 whisper 엔진 → 실제 모델 파일
  → 추론 → segments + timestamp → Transcript로 저장 → 앱을 다시 켜도 남아 있다
```

> 이 통합이 **실제 모델을 올려 실제 오디오로 추론하고 timestamp 있는 결과를 만들어
> Transcript로 저장할 수 있는가.**

### 이것은 품질 벤치마크가 아니다

**필요하지 않은 것** (PRODUCT-SPEC §14.4.3 · `phase-prompt/03` Verification Boundary):

| 필요하지 않다 | 왜 |
| --- | --- |
| **실제 마이크로 한 회의/스터디 녹음** | 짧은 알려진 fixture면 충분하다 |
| **1시간 오디오** | 길이는 이 판정의 대상이 아니다. 5~30초면 된다 |
| **한국어 전사 품질 판정** | 품질은 Final Integration으로 연기됐다 (§10) |
| **한국어 + 영어 혼용 판정** | 같음 |
| **성능 측정 (소요 시간 · 속도 · 메모리)** | 같음 |

그러므로 다음은 **실패가 아니다.**

- 전사된 문장의 단어가 틀렸다 → **PASS**다. segment와 timestamp가 있고 저장되면 된다.
- 언어 표기(`language`)가 기대와 다르다 → PASS다. 다만 §9의 기록표에 그대로 적는다.
- 전사가 느리다 → PASS다. 시간을 재지 않는다.

**판정 기준은 §7에 있는 네 가지뿐이다.**

---

## 2. 준비물 — 사용자가 아니라 **개발 기기**의 요구사항이다

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 저장소 | 이 저장소의 루트에서 실행한다 | — |
| Node · npm | `package.json`의 스크립트를 쓴다 | [E1] `package.json` |
| Rust 툴체인 | `cargo`가 있어야 한다 | [E1] `src-tauri/Cargo.toml` |
| **CMake + C/C++ 툴체인** | `whisper-rs`가 빌드 시점에 whisper.cpp를 함께 빌드한다 | [E2] ADR-0007 §7 · PRODUCT-SPEC §14.4.1 |
| 모델 파일 하나 | §3에서 구한다 | [E2] ADR-0007 §8 |
| 짧은 WAV 하나 | §5에서 만든다 | PRODUCT-SPEC §14.4.3 |

**사용자가 설치할 것은 여전히 없다.** CMake는 이 저장소를 빌드하는 개발 기기의 요구이지
제품 사용자의 요구가 아니다 (ADR-0007 §7 · PRODUCT-SPEC §14.4.2).

### 2.1 sidecar 바이너리를 두는 절차는 **없다**

ADR-0007은 후보 A(Tauri sidecar + `whisper-cli`)가 아니라 **B(`whisper-rs`)** 를 골랐다.
그래서 이 절차에는 **바이너리를 얻어 어딘가에 두는 단계가 없다.**

| 확인 | 결과 | 근거 |
| --- | --- | --- |
| `tauri.conf.json`에 `bundle.externalBin`이 있는가 | **없다** | [E1] `src-tauri/tauri.conf.json` — `bundle`에 `active` · `targets` · `icon`뿐이다 |
| `src-tauri/binaries/` 같은 디렉터리가 있는가 | **없다** | [E1] `src-tauri/` 아래에 그런 디렉터리가 없다 |
| target triple 접미사 파일명 규약을 지켜야 하는가 | **아니다** | [E1] `src-tauri/src/transcription/whisper.rs` — 프로세스 실행이 없다 |

**저장소 밖에서 오는 것은 모델 파일 하나뿐이다.**

---

## 3. 모델 파일을 구한다

### 3.1 무엇을 받는가

smoke test에는 **가장 작은 모델이면 충분하다.** 품질을 판정하지 않기 때문이다.

| 모델 | 크기 | smoke test에 적합한가 |
| --- | --- | --- |
| `ggml-tiny.bin` | ≈75 MiB | 적합 — 가장 빨리 끝난다 |
| **`ggml-base.bin`** | ≈142 MiB | **권장** — 여전히 작고, 실제 단어가 나올 가능성이 tiny보다 높다 |
| `ggml-small.bin` 이상 | ≈466 MiB ~ | 필요 없다. 받는 시간과 추론 시간만 늘어난다 |

크기 출처: [E2] PRODUCT-SPEC §14.4 (`tiny` ≈75MiB · `base` ≈142MiB · `small` ≈466MiB ·
`medium` ≈1.5GiB · `large-v*` ≈2.9GiB).

### 3.2 어디서 받는가

저장소가 기록한 배포 위치는 다음 둘이다 [E2 · PRODUCT-SPEC §14.4].

```text
Hugging Face 저장소   huggingface.co/ggerganov/whisper.cpp
공식 스크립트         whisper.cpp 저장소의 models/download-ggml-model.sh
```

**받는 방법 (둘 중 하나):**

1. **브라우저** — `huggingface.co/ggerganov/whisper.cpp`의 Files 목록에서
   `ggml-base.bin`을 내려받는다. **이 방법이 확실하다.**
2. **명령줄** — 아래는 그 저장소의 통상적인 파일 주소 형태다.
   **이 URL은 이 Run에서 확인하지 못했다 [E4]** — 저장소가 기록한 것은 §3.2 위의 두 줄까지다.
   404가 나면 1번으로 간다.

   ```bash
   curl -L -o ~/Downloads/ggml-base.bin \
     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
   ```

받은 파일이 **0바이트가 아니고 수십 MiB 이상인지** 확인한다 — 중단된 다운로드는 앱이
`transcriptionModelUnusable`로 거절한다 [E1 · `src-tauri/src/transcription/model.rs`의
`an_empty_file_is_unusable_rather_than_a_model`].

```bash
ls -l ~/Downloads/ggml-base.bin
```

### 3.3 저장소에 커밋하지 않는다

**모델 파일을 이 저장소 트리 안에 두지 않는다.** `.gitignore`가 `/models/` · `*.bin` ·
`*.gguf`를 이미 제외하지만 [E1 · `.gitignore`], 규칙은 "무시된다"가 아니라
**"저장소 밖에 둔다"** 이다. 놓을 자리는 §4의 앱 데이터 디렉터리다.

---

## 4. 모델을 놓을 자리를 찾는다

### 4.1 앱 데이터 디렉터리

경로를 정하는 것은 Tauri의 `PathResolver::app_data_dir()`이며, 그 호출은 코드에서 한 곳에만
있다 [E1 · `src-tauri/src/platform/app_data_dir.rs`]. 그 아래 구조는 코드가 고정한다.

```text
<APP_DATA>/
├── molt-note.db          DATABASE_FILE_NAME   [E1 · app_data_dir.rs]
├── recordings/           RECORDINGS_DIR_NAME  [E1]
└── models/               MODELS_DIR_NAME      [E1]  ← 모델은 여기
```

**`<APP_DATA>`의 실제 문자열을 추측하지 않는다.** 앱을 한 번 켜면 DB 파일이 생기므로,
그 파일을 찾아서 확정한다. 파일 이름 `molt-note.db`는 코드가 고정한 값이다 [E1].

```bash
# 앱을 한 번이라도 실행한 뒤 (§6 참조)
find "$HOME/Library/Application Support" -maxdepth 3 -name molt-note.db 2>/dev/null
```

`.../com.moltnote.app/molt-note.db`가 나오면 그 부모 디렉터리가 `<APP_DATA>`다.
(`com.moltnote.app`은 `tauri.conf.json`의 `identifier` [E1]. macOS에서 Tauri가 그 값을
`~/Library/Application Support/` 아래에 놓는다는 것은 Tauri의 동작이며 **이 Run에서
재확인하지 않았다 [E4]** — 그래서 위 `find`로 확정한다.)

```bash
APP_DATA="$HOME/Library/Application Support/com.moltnote.app"   # find가 알려준 값으로 바꾼다
```

### 4.2 `models/` 디렉터리는 **앱이 만들어 주지 않는다**

`AppDataDirectory::ensure_models_dir()`은 존재하지만 **제품 코드에서 부르는 자리가 없다**
[E1 · `src-tauri/src/platform/app_data_dir.rs` — 호출자는 같은 파일의 테스트뿐이다].
그러므로 운영자가 직접 만든다.

```bash
mkdir -p "$APP_DATA/models"
cp ~/Downloads/ggml-base.bin "$APP_DATA/models/ggml-base.bin"
ls -l "$APP_DATA/models"
```

### 4.3 다른 자리에 두어도 된다

설정 값이 **절대 경로**면 그대로 쓰인다 [E1 · `model.rs`의
`an_absolute_path_is_used_as_the_user_gave_it`]. 수 GB짜리 파일을 옮기고 싶지 않으면
`models/`에 두지 않고 §7의 설정 화면에 절대 경로를 그대로 넣어도 된다.

```text
"ggml-base.bin"                  → <APP_DATA>/models/ggml-base.bin 으로 해석된다
"/Users/me/whisper/ggml-base.bin" → 그 경로를 그대로 쓴다
```

---

## 5. 짧은 WAV를 준비한다

### 5.1 앱이 실제로 받아 주는 형식 — 코드가 정한 값이다

[E1 · `src-tauri/src/transcription/audio_input.rs`의 `load()`]

| 조건 | 값 | 맞지 않으면 |
| --- | --- | --- |
| 컨테이너 | RIFF WAV (`hound`가 읽을 수 있어야 한다) | `storage` 실패 — "녹음 파일을 전사 입력으로 열지 못했다" |
| 샘플 형식 | **PCM 16-bit 정수** (`SampleFormat::Int` + `bits_per_sample == 16`) | `invalidInput` — "이 녹음 파일의 샘플 형식으로는 전사 입력을 만들 수 없다" |
| 채널 | **1 또는 2** (stereo는 앱이 평균으로 다운믹스한다) | `invalidInput` — "채널 수로는 전사 입력을 만들 수 없다" |
| 샘플레이트 | 0이 아니면 무엇이든 된다. **16 kHz가 아니면 앱이 리샘플한다** | — |
| 내용 | 비어 있지 않아야 한다 | `invalidInput` — "녹음 파일에 소리가 들어 있지 않다" |

**32-bit float WAV · MP3 · M4A는 받지 않는다.** 16-bit PCM WAV로 만들어야 한다.

smoke test에 적당한 길이는 **5~30초**다. 그 이상은 판정에 아무것도 더하지 않는다.

### 5.2 만드는 방법 (macOS 기본 도구)

> 아래 세 명령은 macOS에 기본 포함된 도구이며 **이 Run에서 실행해 보지 못했다 [E4].**
> 결과 파일이 §5.1의 조건을 만족하는지는 `afinfo`로 직접 확인한다.

**(a) 합성 음성으로 만든다 — 아무 오디오 파일도 없을 때**

```bash
say -o /tmp/molt-smoke.wav --file-format=WAVE --data-format=LEI16@16000 \
  "This is a short smoke test for local transcription. One, two, three."
```

**(b) 이미 있는 오디오를 변환한다**

```bash
afconvert -f WAVE -d LEI16@16000 -c 1 ~/Downloads/clip.m4a /tmp/molt-smoke.wav
```

**(c) 만들어진 파일을 확인한다 — 이 단계를 건너뛰지 않는다**

```bash
afinfo /tmp/molt-smoke.wav
```

`16-bit little-endian signed integer` · `1 ch` · 몇 초의 길이가 보이면 §5.1을 만족한다.
다른 값이 보이면 (a)나 (b)를 다시 한다. **형식이 틀린 채로 진행해도 앱이 §5.1의 문장으로
거절하므로 손상되는 것은 없지만, 그 실패는 전사 통합의 실패가 아니다.**

한국어 문장을 넣어도 되지만 **한국어 품질은 판정 대상이 아니다** (§1).

---

## 6. 앱을 실행하고 그 WAV를 Recording으로 만든다

### 6.1 실행 명령

저장소 루트에서:

```bash
npm install          # 처음 한 번
npm run tauri dev
```

`tauri` 스크립트는 `package.json`에 있다 [E1]. 이 명령은 `beforeDevCommand`로 `npm run dev`
(Vite)를 띄운 뒤 Rust를 빌드한다 [E1 · `src-tauri/tauri.conf.json`].

> **처음 빌드는 whisper.cpp 컴파일을 포함하므로 오래 걸릴 수 있다** (ADR-0007 §4.3).
> 기다린다. 이 시간은 측정 대상이 아니다.

창 제목은 `Molt Note`이고 왼쪽에 **Recordings · Recording · Settings** 세 화면이 있다
[E1 · `src/navigation/routes.ts`].

### 6.2 Recording 레코드를 하나 만든다

**앱에는 파일을 가져오는(import) 기능이 없다.** Transcript는 저장된 Recording 레코드
하나에 대해 만들어지고, 그 레코드는 앱의 녹음 흐름만이 만든다
[E1 · `src-tauri/src/commands/mod.rs` — `audio_path`는 `capture.output_path`에서 온다].
그래서 아래 두 경로 중 하나를 쓴다.

#### 경로 A — 앱으로 직접 짧게 녹음한다 (가장 짧다)

1. 왼쪽에서 **Recording** 화면을 연다.
2. **Record** 를 누른다. 처음이면 macOS 마이크 권한을 묻는다 — 허용한다
   (`docs/ADR-0005-microphone-permission.md`).
3. **10초쯤** 또박또박 말한다. (§5에서 만든 파일을 스피커로 재생해도 된다.)
4. **Stop** 을 누른다. 목록에 Recording 하나가 생긴다.

이 경로에서는 **§5의 WAV가 필요하지 않다** — 앱이 만든 녹음 자체가 "짧은 로컬 WAV"다
(장치 native 포맷의 PCM16 WAV [E1 · `src-tauri/src/audio/capture.rs`]).

#### 경로 B — §5에서 준비한 WAV를 그대로 쓴다 (입력을 고정하고 싶을 때)

경로 A로 **아주 짧게(3~5초) 한 번 녹음해 레코드를 만든 뒤**, 그 레코드가 가리키는 파일을
준비한 WAV로 바꾼다. 저장된 것은 경로 문자열이므로 [E1 · `recordings.audio_path` ·
`src-tauri/src/db/migrations.rs`] 같은 자리의 파일을 바꾸면 그 파일이 전사된다.

```bash
LATEST=$(ls -t "$APP_DATA/recordings"/capture-*.wav | head -1)
echo "$LATEST"                       # capture-<unix초>.wav  [E1 · capture::file_stem]
cp /tmp/molt-smoke.wav "$LATEST"
```

주의할 점:

- **전사를 시작하기 전에** 바꾼다. 녹음 중에는 하지 않는다.
- 목록에 보이는 길이·크기는 녹음 세션이 기록한 값이라 바뀐 파일과 다를 수 있다.
  **이것은 실패가 아니다.**
- 이것은 **운영자가 자기 테스트 fixture를 놓는 준비 작업**이다. INV-1은 *앱 코드가 원본을
  덮어쓰지 않는다*는 규칙이며 [E1 · `audio_input.rs`에는 파일을 쓰는 코드가 없다],
  그 규칙의 대상은 이 수동 조작이 아니다.

> 참고: Settings의 **Recordings directory** 값은 이 절차에 필요하지 않다. 녹음 파일이 놓이는
> 자리는 앱 데이터 디렉터리가 정한다 [E1 · `commands/mod.rs`가 `ensure_recordings_dir()`을
> 쓴다].

---

## 7. 모델을 고르고 전사를 시작한다

### 7.1 설정 (Settings 화면)

1. 왼쪽에서 **Settings** 를 연다.
2. **Transcription** 그룹의 **Whisper model** 칸에 다음 중 하나를 넣는다
   [E1 · `src/screens/SettingsScreen.tsx`].

   ```text
   ggml-base.bin                          <APP_DATA>/models/ 안의 파일 이름
   /Users/me/whisper/ggml-base.bin        절대 경로
   ```

3. **Save** 를 누른다. `Saved.` 가 보이면 저장된 것이다 [E1].

모델을 고르기 전에는 설정 화면이 *"No transcription model is set, so recordings cannot be
transcribed right now."* 를 보여준다 [E1 · `src/screens/settingsView.ts`]. 그 문장이 사라지면
모델이 지정된 것이다. (**Transcribe automatically after a recording is saved** 토글은
이 smoke test에 필요하지 않다. 켜면 다음 녹음이 끝날 때 자동으로 전사가 시작된다.)

### 7.2 전사 시작 (Recording Detail 화면)

1. **Recordings** 목록에서 §6에서 만든 녹음을 누른다.
2. **Transcript** 탭을 연다 (기본으로 열려 있다 [E1]).
3. **Start transcription** 버튼을 누른다 [E1 · `src/screens/transcriptView.ts`].
4. *"Transcribing… This keeps running in the background, so you can leave this screen."* 이
   보인다 [E1]. 끝날 때까지 기다린다 — 화면을 떠나도 된다.

---

## 8. 무엇을 보면 성공인가 — 네 가지 전부 만족해야 한다

### PASS-1 — segment와 timestamp가 있는 Transcript가 화면에 보인다

Transcript 탭에 **한 줄 이상**이 아래 모양으로 나온다
[E1 · `src/screens/RecordingDetailScreen.tsx` · `transcriptView.ts`].

```text
00:00:00 → 00:00:04   This is a short smoke test for local transcription.
00:00:04 → 00:00:07   One, two, three.
```

- 시각은 `HH:MM:SS → HH:MM:SS` 형태다.
- 문장이 비어 있지 않다. **내용이 정확할 필요는 없다** (§1).

### PASS-2 — provenance 한 줄이 실제 엔진과 실제 모델을 가리킨다

segment 위에 `language · engine · model` 이 점(`·`)으로 이어져 나온다 [E1].

```text
en · whisper-rs/0.16 · ggml-base.bin
```

- `engine`이 **`whisper-rs/0.16`** 이면 실제 엔진을 지났다는 뜻이다
  [E1 · `src-tauri/src/transcription/whisper.rs`의 `engine_id`].
- `model`이 **§7에서 지정한 파일의 이름**이어야 한다 [E1 · `model.rs`의 `id()`].
- `language`는 엔진이 판정한 값이며, **무엇이 나오든 PASS다.** 값만 §11에 적는다.

### PASS-3 — timestamp가 오디오 길이와 같은 자릿수다 (단위 확인)

**이 smoke test가 처음으로 관측하는 사실이다.** `whisper-rs`의 segment timestamp가 실제로
센티초인지는 지금까지 **UNVERIFIED**였고 (ADR-0007 §14 · `.loop/evidence/TASK-026/`),
정규화 계수 `×10`은 그 기록 위에 서 있다 [E1 · `src-tauri/src/transcription/parse.rs`].

```text
10초짜리 오디오의 마지막 end timestamp가
  00:00:08 ~ 00:00:11   → 단위 가정이 맞다.            PASS
  00:01:40 근처         → 10배 크다 (센티초가 아니다).  FAIL — §9에 그대로 적는다
  00:00:01 근처         → 10배 작다.                   FAIL — 같음
```

**어긋났다면 여기서 `parse.rs`를 고치지 않는다.** 관측값을 §9·§11에 적고 후속 Task로
넘긴다 — smoke test는 판정하는 자리이지 고치는 자리가 아니다.

### PASS-4 — 앱을 다시 켜도 그대로 있다

1. 앱을 종료한다 (`npm run tauri dev`를 돌린 터미널에서 `Ctrl+C`, 창도 닫는다).
2. `npm run tauri dev`로 다시 켠다.
3. 같은 녹음을 열고 Transcript 탭을 본다.
4. **같은 segment들이 그대로 보인다.**

(선택) 원본이 그대로인지도 확인할 수 있다 — 전사 전후로 해시가 같아야 한다.

```bash
shasum -a 256 "$LATEST"
```

---

## 9. 실패했을 때 무엇을 기록하는가

**어느 단계에서 멈췄든 아래를 그대로 남긴다. 실패를 PASS로 적지 않는다.**

앱은 실패를 화면에 문장으로 보여준다 — **무엇이 실패했는지 · 원본이 안전한지 ·
다시 시도할 수 있는지 · (모델 문제면) 무엇을 하면 되는지** [E1 ·
`src/screens/FailureNotice.tsx` · `transcriptView.ts`]. 그 문장을 요약하지 말고 그대로 옮긴다.

### 9.1 기록 양식

```text
날짜 / 시각        :
멈춘 단계          : §3 모델 / §5 WAV / §6 실행 / §7 시작 / §8 판정 (PASS-1~4 중 어디)
화면에 보인 문장   : headline + message + "원본은 그대로" 문장 + detail 줄까지 전부
실패 종류          : storage / invalidInput / transcriptionModelMissing /
                     transcriptionModelUnusable / transcriptionEngineFailed /
                     transcriptionOutputUnusable   (화면 문장이 어느 것인지 §9.2 표를 본다)
터미널 출력        : `npm run tauri dev`를 돌린 터미널의 마지막 30줄
모델               : 파일 이름 · 바이트 크기 (`ls -l`)
오디오             : `afinfo <파일>` 출력 전체 · 길이(초)
경로               : <APP_DATA> · 전사한 녹음의 audio_path
관측한 timestamp   : 첫/마지막 segment의 시각 (PASS-3에서 어긋났다면 특히)
앱이 죽었는가      : 창이 사라졌는지 / 터미널에 abort·signal이 찍혔는지
```

### 9.2 실패 종류를 가르는 표 [E1 · `src-tauri/src/domain/failure.rs` · `transcription/engine.rs`]

| 종류 | 언제 나오는가 | 먼저 볼 곳 |
| --- | --- | --- |
| `transcriptionModelMissing` | 모델을 고르지 않았거나 그 자리에 파일이 없다 | §4 · §7.1 |
| `transcriptionModelUnusable` | 파일이 아니거나 · 비어 있거나 · 열 수 없거나 · **엔진이 그 모델을 적재하지 못했다** | §3 (다시 받는다) |
| `transcriptionEngineFailed` | 엔진 시작 또는 추론 도중 실패 | 터미널 출력 · 모델과 오디오 조합 |
| `transcriptionOutputUnusable` | 엔진은 끝났는데 결과를 읽을 수 없다 | §9.1을 그대로 기록한다 — 통합 문제일 가능성이 높다 |
| `invalidInput` | WAV 형식·채널·내용이 §5.1과 맞지 않다 | §5 (파일을 다시 만든다) |
| `storage` | 파일을 열지 못했거나 DB에 쓰지 못했다 | 경로와 권한 |

### 9.3 어디에 적는가

| 무엇이 문제였나 | 어디로 |
| --- | --- |
| **제품 문제** (전사가 실패한다 · timestamp가 어긋난다 · 화면이 잘못 나온다) | Phase 3의 후속 Task. `docs/LOOP-RUNTIME-FIELD-NOTES.md`에 적지 않는다 |
| **절차 문제** (이 문서의 경로·명령이 실제와 다르다) | 이 문서를 고친다 |
| **Loop Runtime 문제** (Gate·Verifier·Worker·Plan 진행의 문제) | `docs/LOOP-RUNTIME-FIELD-NOTES.md` (`CLAUDE.local.md`의 Field Note Quality 규칙) |

그리고 어느 경우든 **§11의 실행 기록에 결과를 적는다.** 실패했으면 `FAIL`이라고 적고,
실행하지 않았으면 `NOT RUN`으로 남긴다.

---

## 10. Phase 3가 확인한 것 / 확인하지 않은 것

**확인하지 않은 것을 PASS로 적지 않는다** (PRODUCT-SPEC §20.2 · `phase-prompt/03`).

### 10.1 자동 검증이 확인한 것 (VERIFIED)

판정 수단이 저장소 안에 있고, 사람이 손으로 둔 파일 없이 재실행할 수 있는 것들이다.

| 항목 | 상태 | 판정 수단 |
| --- | --- | --- |
| whisper 통합 방식 결정과 탈락 근거가 기록됐다 | **VERIFIED** | `docs/ADR-0007-transcription-engine.md` |
| `whisper-rs` 0.16.0 / `whisper-rs-sys` 0.15.0이 실제로 해석된다 | **VERIFIED** | `src-tauri/Cargo.lock` · `.loop/evidence/TASK-026/whisper-rs-api-verification.md` |
| `whisper-rs`의 실제 API 시그니처 (컴파일러가 확인) | **VERIFIED** | 같음 — `full(&[f32])` · `get_segment(i32)` · `start_timestamp() -> i64` 등 |
| `rubato` 5.0.0의 실제 API와 지연 보정의 효력 | **VERIFIED** | `.loop/evidence/TASK-024/verification-log.md` (mutation 확인 포함) |
| WAV → 16 kHz mono f32 변환 · 다운믹스 · 거부 규칙 | **VERIFIED** | `cargo test` (`src-tauri/src/transcription/audio_input.rs`) |
| 센티초 → 밀리초 정규화가 한 자리에서만 일어나고 ×10/×100 회귀를 테스트가 잡는다 | **VERIFIED** | `src-tauri/src/transcription/parse.rs`의 테스트 · `.loop/evidence/TASK-025/` |
| Transcript가 §7의 필드대로 저장되고, 재전사가 **추가**이며, 실패해도 current가 유지된다 | **VERIFIED** | `src-tauri/tests/transcription_run.rs` |
| 전사 상태 전이(`pending`·`running`·`done`·`failed`)와 전사 중 UI 비차단 | **VERIFIED** | `src-tauri/tests/transcription_background.rs` |
| `automatic_transcription`이 별도 토글이고 기본 OFF이며, 모델이 없어도 앱이 그 값을 뒤집지 않는다 | **VERIFIED** | `src-tauri/tests/automatic_transcription.rs` |
| 네 가지 전사 실패가 서로 다른 종류로 화면에 도달한다 | **VERIFIED** | `src-tauri/tests/transcription_engine.rs` |
| Transcript 탭이 `HH:MM:SS → HH:MM:SS` 로 보여준다 | **VERIFIED** | `src/screens/transcriptView.test.ts` |
| 전사 경계에 네트워크도 자식 프로세스도 없다 (오디오가 기기 밖으로 나가지 않는다) | **VERIFIED** | `src-tauri/tests/transcription_engine.rs`의 경계 검사 |
| build · lint · test Gate가 green이다 | **VERIFIED (2026-09-03)** | `.loop/evidence/TASK-030/` (`self-check build lint test` 셋 다 exit 0). TASK-031은 문서만 바꿨다 |

### 10.2 확인되지 않은 것 — 실행되지 않았거나 연기됐다

| 항목 | 상태 | 왜 · 어디서 판정되는가 |
| --- | --- | --- |
| **실제 Whisper 추론이 한 번이라도 성공하는가 (end-to-end)** | **NOT RUN — 운영자 smoke test 대기** | 이 문서 §1~§9. **실행 전까지 "end-to-end 전사가 검증됐다"고 적지 않는다** (PRODUCT-SPEC §14.4.3) |
| `whisper-rs`의 segment timestamp가 실제로 **센티초**인가 | **UNVERIFIED** | 타입이 `i64`라는 것만 컴파일러가 확인했다. 단위는 §8의 PASS-3이 처음 관측한다 |
| 번들 whisper.cpp의 실제 버전 | **UNVERIFIED** | 읽는 경로를 확인하지 못해 `engine` provenance에 넣지 않았다 (ADR-0007 §16) |
| Metal 등 가속이 실제로 켜져 있는가 | **UNVERIFIED** | `Cargo.toml`이 feature를 지정하지 않는다 [E1] — 기본값이 무엇인지 확인하지 않았다 |
| release 빌드 · 번들된 `.app`에서의 동작 | **UNVERIFIED** | 이 Phase는 `dev` 실행만 다룬다 |
| **codesign / notarization** | **DEFERRED** | 배포 검증 경계 (ADR-0007 §6) |
| **Windows 빌드 · 실행** | **DEFERRED — Phase 6** | ADR-0007 §11 · `phase-prompt/03` Out of Scope |
| **실제 한국어 전사 품질** | **DEFERRED** | Final Integration (`phase-prompt/03` Human Review) |
| **한국어 + 영어 혼용 음성** | **DEFERRED** | 같음 |
| **timestamp가 실제 음성 위치와 맞는가** | **DEFERRED** | 같음. §8의 PASS-3은 **자릿수(단위)** 만 본다 — 음성 위치와의 일치는 보지 않는다 |
| **1시간 전사 소요 시간** | **DEFERRED** | 같음. smoke test는 시간을 재지 않는다 |
| 모델 디렉터리를 앱이 만들어 주는가 | **아니다 (확인된 한계)** | `ensure_models_dir()`의 제품 호출자가 없다 [E1]. 운영자가 §4.2에서 만든다 |

---

## 11. smoke test 실행 기록 — 운영자가 채운다

**아직 실행되지 않았다.** 아래 표의 `NOT RUN`을 실제 결과로 바꾸는 것은 이 문서를 쓴
Task가 아니라 **운영자의 실행**이다.

| | 값 |
| --- | --- |
| 실행 날짜 | *(NOT RUN)* |
| 실행자 | *(NOT RUN)* |
| 사용한 모델 파일 | *(NOT RUN)* |
| 오디오 (경로 A / B · 길이) | *(NOT RUN)* |
| PASS-1 segments + timestamp가 보인다 | **NOT RUN** |
| PASS-2 engine = `whisper-rs/0.16` · model = 지정한 파일 | **NOT RUN** |
| PASS-3 timestamp 자릿수 (단위 관측값) | **NOT RUN** |
| PASS-4 재시작 후에도 남아 있다 | **NOT RUN** |
| 관측된 `language` 값 | *(NOT RUN)* |
| 실패했다면 §9.1 기록 | *(NOT RUN)* |

**네 항목이 전부 PASS가 되기 전까지 Phase 3를 "end-to-end 전사가 검증됐다"고 적지 않는다.**
실행이 끝난 뒤 `docs/SYSTEM-MAP.md`를 갱신하는 것은 **Phase가 최종 DONE이 된 뒤 운영자의
일이다** (`CLAUDE.local.md`) — 이 문서를 만든 Task는 SYSTEM-MAP을 건드리지 않았다.

---

# 부록 — 실제 실행 결과 (2026-09-05)

**위 §10의 `NOT RUN` 표는 이 문서를 쓴 시점(Phase 3)의 상태다. 지우지 않는다.**
아래는 그 이후 **운영자가 실제로 실행한 결과**이며, `A-TRANS-001`에 대한 첫 실측 답이다.

## 실행 조건

```text
실행 날짜        2026-09-05
실행자           운영자 (사람이 직접 녹음 · 실행)
기기             Apple M5 · 물리 10코어
오디오           capture-1788522158.wav
                 48 kHz · 모노 · 16-bit PCM · 400 MB · 1시간 12분 51초
                 한국어 대화 (3인 · 실제 회의)
```

## 결과 1 — 제품 경로는 **FAIL**했다

앱(`npm run tauri dev`)으로 실행한 전사는 **사용 불가능한 결과**를 냈다.

```text
관측된 language     en          ← 한국어 음성인데 영어
소요                약 26분
segment             1,711
고유 문장            59  (3.4%)
한글이 나온 줄        0
최다 반복            1,063회 (62.1%)  But I was like, "What are you doing?"
상위 2문장           86.7%
```

00:28:10 ~ 00:56:55 약 30분간 같은 문장 하나만 출력됐다. 전사가 아니라 디코딩 붕괴다.

**원인 (코드에서 확인됨).** `src-tauri/src/transcription/whisper.rs`는 언어를 설정하지 않는다
(`set_language` · `set_detect_language` 모두 호출 없음). whisper.cpp 기본값은 자동 감지가
아니다:

```c
// whisper.cpp/src/whisper.cpp:5943 — whisper_full_default_params
/*.language        =*/ "en",
/*.detect_language =*/ false,
```

그러므로 DB에 남은 `language = en`은 **감지 결과가 아니라 아무도 바꾸지 않은 기본값**이다.

반복 방지 장치는 정상이었다 — whisper.cpp 기본값에 `no_context = true`,
`temperature_inc = 0.2`, `entropy_thold = 2.4`, `logprob_thold = -1.0`이 모두 켜져 있다.
붕괴는 디코더 설정 문제가 아니라 **틀린 언어를 강제한 결과**다.

## 결과 2 — 조건을 바꾸면 **PASS**한다

같은 오디오를 저장소 밖의 검증용 도구로 다시 전사했다
(제품 코드를 고치지 않고 조건만 바꿔 원인을 분리하기 위해서다).

```text
변경 1   params.set_language(Some("ko"))
변경 2   whisper-rs features = ["metal"]        (Spec §14.4가 이미 적어 둔 것)
변경 3   120초 청크 분할 + 청크마다 state 재생성 + 연속 반복 3회 초과 차단
모델     ggml-large-v3-turbo.bin (1.5 GB)
```

```text
소요                6.0분        (72.85분 오디오 · 실시간의 약 12배)
segment             1,749
고유 문장            1,643 (94.0%)
최다 반복            10회
한글 출력            정상
```

**세 변경의 기여도 (부분 실측):**

| 조건 | 모델 | 소요 | 결과 |
| --- | --- | --- | --- |
| 언어 미설정 (제품 현재) | base | 약 26분 | 영어 붕괴 · 고유 3.4% |
| `ko` + Metal | base | **1.3분** | 한국어 · 다만 반복 27% 잔존 |
| `ko` + Metal | large-v3-turbo | 6.1분 | 한국어 · 반복 구간 2곳 잔존 |
| `ko` + Metal + 청크분할 | large-v3-turbo | **6.0분** | 한국어 · 고유 94% |

## 결과 3 — Metal 실측 (Spec §14.4의 미검증 항목)

```text
Metal 미사용 (제품 현재)   build.rs가 GGML_METAL=OFF를 명시 · CPU + Accelerate만
Metal 사용                 GPU name: Apple M5 · use gpu = 1 · backends = 3
base 모델 72분 전사        26분 → 1.3분
```

**[관측된 사실]** 위 26분은 **붕괴한 디코딩의 소요 시간**이므로 순수 Metal 효과로 읽으면
안 된다. 언어 수정과 Metal 활성화가 함께 적용된 값이다. **두 요인을 분리한 측정은 하지
않았다.**

## 결과 4 — 모델 크기 (PRODUCT-SPEC:838의 UNVERIFIED 항목)

Spec은 "한국어+영어 혼용 1시간 녹음에 `large-v3` / `large-v3-turbo`가 현실적"이라고 적고
**UNVERIFIED · Phase 3에서 실측한다**고 표시했다. 그 실측 결과다.

```text
ggml-base (142 MB)            한국어는 나오지만 구어체에서 자주 무너진다
                              (언어 수정 후에도 반복 27% 잔존)
ggml-large-v3-turbo (1.5 GB)  실사용 가능한 품질. 72분에 6분.
```

**결론: `base`는 한국어 대화에 부족하다. `large-v3-turbo`는 충분하다.**
이것으로 Spec:838의 추론이 **이 기기·이 오디오에 한해 확인됐다.**

## 결과 5 — 무음 구간의 환각

`00:57 ~ 01:08` 약 10분(화자들이 자리를 비운 구간)에서 whisper가 학습 데이터의 잔재를
출력했다 — `한글자막 by 한효정`, `고추장은 너무 맛있게 잘 먹었습니다`,
`모종 입력을 제거합니다` 등.

청크 분할과 반복 차단으로 양은 크게 줄었으나 **완전히 사라지지 않았다.**
VAD(음성 구간 검출)는 이번 실행에 포함하지 않았다. **[미검증 · 다음 후보]**

## §10 표에 대한 답

| 항목 | 결과 |
| --- | --- |
| 실행 날짜 | 2026-09-05 |
| 사용한 모델 파일 | `ggml-base.bin` · `ggml-large-v3-turbo.bin` |
| 오디오 | `capture-1788522158.wav` · 1:12:51 · 한국어 3인 대화 |
| PASS-1 segments + timestamp가 보인다 | **PASS** |
| PASS-2 engine = `whisper-rs/0.16` · model = 지정한 파일 | **PASS** |
| PASS-3 timestamp 자릿수 | **PASS** — centisecond (1/100초) |
| PASS-4 재시작 후에도 남아 있다 | **PASS** — DB `transcripts` · `transcript_segments` 1,711행 |
| 관측된 `language` 값 | **`en`** — 설정되지 않은 기본값이며 **결함이다** |

**엔진 경로 자체는 동작한다 (PASS-1~4).** 그러나 **언어가 설정되지 않아 한국어 사용자에게는
제품이 동작하지 않는다.** 이 결함의 수정은 `phase-prompt/05.6-transcription-correctness-and-reach.md`가
맡는다.

**`A-TRANS-001`은 이 실행으로 해소되지 않는다.** 제품 경로가 고쳐지고 사람이 다시 확인할
때까지 열려 있다.

---

# 부록 2 — 레벨을 올린 사본으로 다시 전사하는 절차와 결과 (2026-09-07)

```text
기록 일자:  2026-09-07
기록 Task:  TASK-083 (문서 전용 — 제품 소스 · 설정 · 의존성 · 테스트를 바꾸지 않는다)
Phase:      5.7 — Recording Level + Transcription Collapse (성공 기준 3)
근거:       phase-prompt/05.7-recording-level-and-transcription-collapse.md R-2 · R-3
            docs/ADR-0003-recording-engine.md §16.1 · docs/ADR-0007-transcription-engine.md §18.1
Status:     절차는 적혔다 · **측정은 실행되지 않았다 ([미측정] — §부록2-5의 이유)**
정정:       2026-09-07 · TASK-090 (문서 전용) — 아래 **[정정 2026-09-07]** 블록
            capture-1788522158.wav 를 "전사가 성공한 파일"로 읽히게 한 서술 4곳.
            절차 · 표 · [미측정] 표시는 그대로 둔다.
```

> **위 §10의 표도, 2026-09-05 부록도 이 부록이 지우거나 다시 쓰지 않았다.**
> 이것은 그 뒤에 덧붙인 세 번째 기록이며, 9/5 부록이 세운 선례를 그대로 따른다.

> ⚠️ **이 부록이 절차를 적었다는 사실은 측정이 수행됐다는 뜻이 아니다.**
> §부록2-4의 결과 칸이 `[미측정]`인 동안, "레벨을 올리면 이 오디오가 구제된다"고도
> "구제되지 않는다"고도 적지 않는다.

**이 부록은 `[관측된 사실]` · `[유력한 설명]` · `[미검증]` 을 구분해 적는다**
(`05.7` §Preconditions의 표기를 그대로 쓴다). 그 구분을 지운 채 인용하지 않는다.

---

## [정정 2026-09-07] `capture-1788522158.wav`는 "전사가 성공한 파일"이 아니다

```text
정정한 Task   TASK-090 (문서 전용) · 2026-09-07 · Phase 5.8 성공 기준 3
              phase-prompt/05.8-transcription-chunking.md P-1 · Goal 3
틀린 서술이    TASK-083 (이 부록을 쓴 Task) · 2026-09-07 · §부록2 안 4곳
들어온 시점
정정 방식      덮어쓰지 않는다 — 원래 문장을 취소선으로 남기고 옆에 사실을 적는다.
              절차 · 측정 방법 · 결과 표 · [미측정] 표시는 하나도 지우지 않았다.
```

**무엇이 틀렸는가.** 이 부록은 §부록2-0 '대상과 조건'에서 `capture-1788522158.wav`를
**"9/4 녹음 · 전사가 성공한 파일"** 이라고 적었다. 그 파일이 **제품 경로에서 전사에
성공한 적은 없다.**

**무엇이 사실인가.** 같은 문서의 **"결과 1 — 제품 경로는 FAIL했다"** 가 그 파일의
제품 경로 실행을 이미 기록하고 있다.

```text
제품 경로 (앱 · npm run tauri dev)     FAIL
  관측된 language   en          ← 한국어 음성인데 영어 (설정되지 않은 기본값)
  segment           1,711
  고유 문장          59  (3.4%)
  최다 반복          1,063회 (62.1%)  But I was like, "What are you doing?"
  한글이 나온 줄      0
```

`고유 문장 94.0%`라는 PASS는 같은 문서 **"결과 2 — 조건을 바꾸면 PASS한다"** 의 값이며,
**저장소 밖의 검증용 도구에서 조건 셋(`set_language("ko")` · `features = ["metal"]` ·
120초 청크 분할 + state 재생성 + 반복 차단)을 바꿔** 얻은 것이다. 제품 코드가 낸 값이
아니다. 그러므로 그 파일에 대해 옳은 서술은 이것뿐이다.

> **`capture-1788522158.wav`는 제품 경로에서 FAIL했고, 저장소 밖 도구가 조건을 바꿨을 때
> 94.0%가 나온 파일이다.** "전사가 성공한 파일"이 아니라 **"목표 수치가 알려진 파일"** 이다.

### 이 정정이 무효로 만들지 않는 것

**`ADR-0003 §16.1`의 -25.8 dBFS는 여전히 유효하다.** 그 값은 전사 결과가 아니라 **파일의
레벨을 직접 잰 실측값**이다 — 40개 지점에서 2초씩 읽어 구한 평균 RMS 1685.5이며
(`[H✓ 2026-09-07 실측]`), 전사가 성공했는지 여부와 무관하게 성립한다. 이 정정으로
바뀌는 값이 아니다.

**§부록2의 P8 증폭 실험도 무효가 되지 않는다.** 이 실험이 목표로 삼은 것은 **레벨**이다 —
`capture-1788746454.wav`(-42.2 dBFS)를 **정상 레벨인 -25.8 dBFS까지 올렸을 때 같은 제품
경로에서 무엇이 나오는가**를 묻는다. 목표로 삼은 것이 "성공한 전사"였던 적은 없고,
증폭의 도달점은 dBFS 값 하나다. 그 dBFS 값이 실측이므로 실험 설계는 그대로 선다.

**틀린 것은 서술 하나다** — "그 파일은 전사가 성공했다". 그 파일을 레벨 기준으로 쓴 것은
옳고, 그 파일을 전사 성공 사례로 읽는 것이 틀리다.

### 이 문서 안에서 같은 오해를 만드는 서술 — 전수

세 곳은 `text` / `bash` 코드 블록 안이라 **취소선이 렌더되지 않는다.** 그 자리에서는
문구를 사실에 맞게 고치고 **원문을 바로 아래 `[정정]` 주석에 그대로 남겼다.** 마크다운
본문인 한 곳에서만 취소선을 쓴다.

| 위치 | 원래 서술 (전문은 각 자리의 `[정정]` 주석에 남아 있다) | 처리 |
| --- | --- | --- |
| §부록2-0 '대상과 조건' (`text` 블록) | `capture-1788522158.wav (9/4 녹음 · 전사가 성공한 파일)` | `9/4 녹음 · 레벨 비교 기준`으로 고치고, 원문 + 사실을 블록 안 `[정정]` 주석에 병기 |
| §부록2-1 '클리핑을 어떻게 피하는가' (`text` 블록) | `목표 이득  9/4 성공 녹음의 평균 RMS(-25.8 dBFS)에 맞춘다` | `9/4 녹음 파일의 평균 RMS`로 고치고, 원문 + 사실을 블록 안 `[정정]` 주석에 병기. **목표 -25.8은 그대로다** |
| §부록2-1 절차 스크립트의 주석 (`bash` 블록) | `TARGET_RMS_DBFS = -25.8  # 9/4 성공 녹음의 평균 RMS (ADR-0003 §16.1)` | 주석 문구를 `9/4 녹음 파일`로 고치고, 원문을 바로 아래 주석 줄에 남긴다. **값 -25.8과 스크립트 동작은 바뀌지 않았다** |
| §부록2-6 첫 `[미검증]` 항목 (본문) | `9/4 성공 실행은 ko + Metal + large-v3-turbo + 청크 분할…이 함께 걸린 저장소 밖 도구였고` | 취소선 + 사실 병기. 이 항목은 이미 "저장소 밖 도구"라고 적고 있었다 — 표현만 정정한다 |

**그 밖에 이 문서에 같은 오해를 만드는 서술은 없다.** 확인한 것들:

- 9/5 부록 '§10 표에 대한 답'의 `오디오 | capture-1788522158.wav` 행 — 그 표는 PASS-1~4가
  **엔진 경로**에 대한 판정임을 바로 아래에서 밝히고 있고, `language = en`을 **결함**이라고
  적었다. 정정하지 않는다.
- §부록2-1 보고서 확인 항목의 `9/4 수준까지 올리지 못했다` — 여기서 "9/4 수준"은 **레벨**을
  가리킨다. 전사 성공을 뜻하지 않으므로 정정하지 않는다.
- §부록2-7의 `9/5 실험` · `9/5가 효과를 본 세 가지` — 저장소 밖 실험임이 이미 명시돼 있다.

**다른 문서는 이 Task가 바꾸지 않았다.** `docs/ADR-0003-recording-engine.md` ·
`docs/ADR-0007-transcription-engine.md` · `phase-prompt/` 는 그대로다.

---

## 부록2-0. 이 측정이 답하려는 질문 하나

`05.7` R-3이 남긴 `[미검증]` 항목이다.

```text
이미 녹음된 capture-1788746454.wav 를 레벨만 올리면, 같은 제품 경로에서 전사가 되는가?
```

**이 질문이 묻지 않는 것:** 레벨이 붕괴의 *유일한* 원인인가. 이 실험은 한 파일에 한 조건을
바꿔 한 번 재는 것이며, 어느 쪽 결과가 나와도 원인을 하나로 단정할 근거가 되지 않는다
(§부록2-6).

### 대상과 조건 [관측된 사실 · `05.7` R-2 · R-3]

```text
원본            capture-1788746454.wav
                280 MB · WAV PCM · 16-bit · mono · 48,000 Hz · 50.99분
                recording_id fe5f738c732bdae0810972a66d233ccb
                평균 RMS 255.7 (-42.2 dBFS) · 전체 피크 3494 (-19.4 dBFS)
                  ← 40구간 표본 측정값 (ADR-0003 §16.1)

비교 대상        capture-1788522158.wav (9/4 녹음 · 레벨 비교 기준)
                평균 RMS 1685.5 (-25.8 dBFS) · 전체 피크 32767 (0.0 dBFS)
                차이 16.4 dB (RMS 6.6배)

  ── [정정 2026-09-07 · TASK-090] ───────────────────────────────────────────
  원래 서술 (TASK-083):  "capture-1788522158.wav (9/4 녹음 · 전사가 성공한 파일)"
  사실:  이 파일의 **제품 경로 전사는 FAIL**이다 — 같은 문서 '결과 1'이
         segment 1,711 · 고유 59 (3.4%) · 같은 문장 1,063회로 기록하고 있다.
         고유 94.0%는 '결과 2'의 값이며 **저장소 밖 검증용 도구에서 조건 셋을
         바꿔** 얻은 것이다.
  이 자리에서 이 파일이 비교 대상인 이유는 **레벨**이다 — -25.8 dBFS는
  ADR-0003 §16.1의 실측값이며 이 정정으로 바뀌지 않는다.
```

---

## 부록2-1. 레벨을 올린 **사본**을 만든다 — 원본은 읽기만 한다 (INV-1)

**규칙 세 개를 절차가 지킨다.**

1. **원본은 `'rb'`로만 연다.** 원본 경로를 쓰기 모드로 여는 단계가 이 절차에 없다.
2. **원본을 옮기지도 이름을 바꾸지도 않는다.** `mv` · `rm` · in-place 편집이 없다.
3. **사본은 `recordings/` 밖에 놓는다.** 앱이 그 파일을 Recording으로 착각할 자리에 두지 않는다.

쓰는 도구는 **macOS에 기본 포함된 `python3`의 표준 라이브러리(`wave` · `array` · `math`)**
뿐이다. 새 의존성을 설치하지 않으며, 이것은 `ADR-0003 §16.1`의 레벨 측정이 이미 쓴 것과
같은 도구다. **[미검증]** 아래 스크립트는 이 Run에서 실행해 보지 못했다 (§부록2-5).

### 클리핑을 어떻게 피하는가 — 두 겹으로 막는다

```text
목표 이득    9/4 녹음 파일의 평균 RMS(-25.8 dBFS)에 맞춘다
상한         올린 뒤의 **전체 피크가 -1 dBFS를 넘지 않는다**
적용 이득    min(목표 이득, 상한 이득)      ← 둘 중 작은 쪽만 쓴다
마지막 방어   샘플별 ±32767 클램프 + 클램프된 샘플 수를 세어 보고한다

  ── [정정 2026-09-07 · TASK-090] ───────────────────────────────────────────
  원래 서술 (TASK-083):  "목표 이득  9/4 성공 녹음의 평균 RMS(-25.8 dBFS)에 맞춘다"
  그 파일은 제품 경로에서 전사에 성공한 적이 없다 (위 [정정] 블록 · '결과 1').
  **목표 수치 -25.8 dBFS는 그대로다** — 이 실험이 맞추려는 것은 전사 결과가
  아니라 **레벨**이고, 그 값은 ADR-0003 §16.1의 실측값이다.
```

**표본 값으로 미리 계산하면** 목표 이득은 `1685.5 / 255.7 ≈ 6.6배 (+16.4 dB)`이고, 그때
피크는 `3494 × 6.6 ≈ 23,000 (-3.1 dBFS)`으로 상한 아래다. **실제로 쓰는 값은 스크립트가
전체 파일을 훑어 다시 구한다** — 40구간 표본과 전 구간 측정은 다를 수 있고 [미검증],
판단 근거는 표본이 아니라 전 구간 값이어야 한다.

### 절차

```bash
APP_DATA="$HOME/Library/Application Support/com.moltnote.app"   # §4.1의 find로 확정한 값
SRC="$APP_DATA/recordings/capture-1788746454.wav"
DST="$HOME/molt-level-test/capture-1788746454-gain.wav"          # recordings/ 밖이다

mkdir -p "$(dirname "$DST")"
ls -l "$SRC"                       # 280 MB · 이 파일은 이 절차 내내 바뀌지 않는다
shasum -a 256 "$SRC" | tee "$HOME/molt-level-test/original.sha256"
```

마지막 줄의 sha256은 **절차가 끝난 뒤 다시 찍어 같은 값인지 확인하기 위한 것**이다
(§부록2-3의 마지막 단계).

```bash
python3 - "$SRC" "$DST" <<'PY'
import array, math, os, sys, wave

SRC, DST = sys.argv[1], sys.argv[2]
TARGET_RMS_DBFS  = -25.8     # 9/4 녹음 파일의 평균 RMS (ADR-0003 §16.1 실측)
                             # [정정 2026-09-07 · TASK-090] 원래 주석은 "9/4 성공 녹음의
                             # 평균 RMS"였다. 그 파일은 제품 경로에서 전사에 성공한 적이
                             # 없다. 값 -25.8은 레벨 실측값이므로 바뀌지 않았다.
PEAK_CEILING_DBFS = -1.0     # 올린 뒤 피크가 이 위로 가지 않는다
FULL_SCALE = 32768.0         # 16-bit 정수 PCM의 풀스케일 (ADR-0003 §16.1이 고정한 기준)

# INV-1: 원본을 여는 유일한 자리이며 모드는 'rb'다. 아래 어디에도 SRC를 쓰는 코드가 없다.
assert os.path.abspath(SRC) != os.path.abspath(DST), "사본 경로가 원본과 같다"
assert "/recordings/" not in os.path.abspath(DST), "사본을 recordings/ 안에 두지 않는다"

with wave.open(SRC, "rb") as w:
    assert w.getsampwidth() == 2, "16-bit PCM이 아니다"
    assert w.getnchannels() == 1, "mono가 아니다"
    rate, frames = w.getframerate(), w.getnframes()
    samples = array.array("h")
    samples.frombytes(w.readframes(frames))
if sys.byteorder == "big":
    samples.byteswap()

peak = max(max(samples), -min(samples))
sumsq = 0
for s in samples:                      # 147M 샘플 — 수 분 걸린다. 표본이 아니라 전 구간이다
    sumsq += s * s
rms = math.sqrt(sumsq / len(samples))

gain_rms  = 10 ** (TARGET_RMS_DBFS  / 20) * FULL_SCALE / rms
gain_peak = 10 ** (PEAK_CEILING_DBFS / 20) * FULL_SCALE / peak
gain = min(gain_rms, gain_peak)        # 클리핑을 막는 것은 이 min 하나다

clamped = 0
out = array.array("h", bytes(2 * len(samples)))
for i, s in enumerate(samples):
    v = int(round(s * gain))
    if v > 32767:
        v, clamped = 32767, clamped + 1
    elif v < -32768:
        v, clamped = -32768, clamped + 1
    out[i] = v
if sys.byteorder == "big":
    out.byteswap()

with wave.open(DST, "wb") as w:        # 쓰기는 사본 경로 하나뿐이다
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(rate)
    w.writeframes(out.tobytes())

def dbfs(x):
    return 20 * math.log10(x / FULL_SCALE) if x > 0 else float("-inf")

print(f"원본        rate={rate} frames={frames} 길이={frames/rate/60:.2f}분")
print(f"원본 레벨    RMS {rms:.1f} ({dbfs(rms):.1f} dBFS) · 피크 {peak} ({dbfs(peak):.1f} dBFS)")
print(f"이득        목표 {gain_rms:.3f}x · 상한 {gain_peak:.3f}x · 적용 {gain:.3f}x "
      f"({20*math.log10(gain):.1f} dB)")
print(f"사본 예상    피크 {peak*gain:.0f} ({dbfs(peak*gain):.1f} dBFS) · 클램프된 샘플 {clamped}")
print(f"사본        {DST}")
PY
```

**보고서에서 확인할 것 두 가지.**

- `클램프된 샘플 0` — 0이 아니면 클리핑이 일어났다. 그 사본으로 전사하지 않고
  `PEAK_CEILING_DBFS`를 더 낮춰 다시 만든다.
- `적용 이득`이 `상한`이 아니라 `목표` 쪽에서 왔는가 — 상한에 걸렸다면 9/4 수준까지
  올리지 못했다는 뜻이며, 그 사실을 §부록2-4 표에 함께 적는다.

### 사본을 형식 검사한다

```bash
afinfo "$DST"      # 16-bit little-endian signed integer · 1 ch · 약 3059초
ls -l "$DST"       # 원본과 비슷한 280 MB 근처
```

`§5.1`의 조건(RIFF WAV · PCM 16-bit · mono)을 만족한다. 48 kHz는 그대로 두어도 된다 —
앱이 16 kHz로 리샘플한다 [E1 · `src-tauri/src/transcription/audio_input.rs`].
**원본과 같은 조건에서 레벨 하나만 다른 입력**이어야 비교가 성립하므로 리샘플하지 않는다.

---

## 부록2-2. 그 사본을 **제품 경로로** 전사한다 (§6.2 경로 B의 선례)

전사 조건은 **2026-09-07 원본 실행과 같아야 한다**. 다른 것은 오디오 레벨 하나뿐이다.

```text
경로     제품 (npm run tauri dev)   — 저장소 밖의 검증용 도구가 아니다
언어     ko                         (Settings의 Language 칸 [E1 · SettingsScreen.tsx])
모델     ggml-large-v3-turbo.bin    (2026-09-07 원본 실행과 같은 파일)
청크분할  없음                       (9/5 실험의 처방을 옮겨 오지 않는다 · `05.7` §하지 않는 것)
```

### 1) 설정 (Settings 화면)

1. **Whisper model** 칸에 `ggml-large-v3-turbo.bin` — `<APP_DATA>/models/`에 있어야 한다
   (없으면 §4의 절차로 놓는다. 절대 경로를 넣어도 된다 · §4.3).
2. **Language** 칸에 `ko` — **비워 두지 않는다.** 빈 값은 자동 감지이며, 이 오디오에서
   자동 감지는 확신도 0.478로 `en`을 골랐다 [관측된 사실 · `05.7` R-1].
3. **Save** → `Saved.`

### 2) 사본을 담을 **새 레코드**를 만든다 — 9/7 레코드를 건드리지 않는다

§6.2 경로 B를 그대로 쓰되, **바꿔 넣는 대상은 방금 만든 버리는 레코드의 파일이다.**
원본 `capture-1788746454.wav`와 그 레코드(`fe5f738c...`)는 이 절차에서 **읽지도 쓰지도
않는다.**

1. **Recording** 화면에서 **3~5초** 녹음하고 **Stop** — 새 레코드가 하나 생긴다.
2. 그 레코드의 파일을 사본으로 바꾼다.

```bash
NEW=$(ls -t "$APP_DATA/recordings"/capture-*.wav | head -1)
echo "$NEW"        # 방금 만든 레코드의 파일인지 눈으로 확인한다

case "$NEW" in
  *capture-1788746454.wav) echo "중단: 이것은 원본이다"; exit 1 ;;
esac

cp "$DST" "$NEW"   # 덮어쓰는 것은 방금 만든 버리는 레코드의 3초짜리 파일이다
```

- **전사를 시작하기 전에** 바꾼다.
- 목록의 길이·크기가 3~5초로 보이는 것은 녹음 세션이 기록한 값이다. **실패가 아니다**
  (§6.2의 주의와 같다).
- 이것은 **운영자가 자기 fixture를 놓는 준비 작업**이며, INV-1이 금지하는 것은 *앱 코드가
  원본을 덮어쓰는 것*이다 (§6.2). 그 원본은 이 절차에서 손대지 않는다.

### 3) 전사를 시작하고 **시계로 시간을 잰다**

1. 새 레코드의 상세 화면에서 **전사 시작**.
2. 시작·종료 시각을 적는다. **소요 시간을 시계로 재는 이유:** 붕괴로 판정되면 Transcript가
   저장되지 않아 `transcripts.transcription_ms`가 남지 않는다
   [E1 · `src-tauri/src/transcription/run.rs` — 붕괴는 저장 **전에** 실패로 돌아간다].

> **[관측된 사실 · E1]** 현재 작업 트리에는 붕괴 판정이 들어와 있다
> (`src-tauri/src/transcription/collapse.rs` · `n >= 20` 이고 `u/n <= 0.20` 또는
> `r/n >= 0.50`이면 붕괴). 그러므로 이번 실행은 **`done`이 아니라 실패로 끝날 수 있고,
> 그것 자체가 정상 동작이다.** 두 경우의 측정 방법이 §부록2-3에 모두 적혀 있다.

---

## 부록2-3. 결과를 어떻게 재는가 — 9/5 · 9/7과 **같은 방법**이어야 한다

재는 값은 네 개다. 문장을 세는 규칙은 `ADR-0007 §18.2`가 정한 그대로다 —
**앞뒤 공백을 걷고 내부 연속 공백을 하나로 줄인 문자열이 문장이며, 빈 문자열은 세지 않는다.**

```text
총 segment 수        전사가 낸 segment의 개수
고유 문장 u 와 u/n    서로 다른 문장의 개수와 그 비율 (n = 빈 문장을 뺀 문장 총 개수)
최다 반복 점유율 r/n  가장 많이 나온 문장의 출현 횟수 r 을 n 으로 나눈 값
소요 시간            transcripts.transcription_ms  또는  시계로 잰 값
```

### 경우 A — `done`으로 저장됐을 때 (DB에서 읽는다 · 읽기 전용으로 연다)

```bash
python3 - "$APP_DATA/molt-note.db" <<'PY'
import collections, sqlite3, sys

db = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)   # 읽기 전용
tid, lang, model, ms = db.execute(
    "SELECT id, language, model, transcription_ms FROM transcripts "
    "ORDER BY created_at DESC LIMIT 1").fetchone()
rows = db.execute(
    "SELECT text FROM transcript_segments WHERE transcript_id = ? ORDER BY ordinal",
    (tid,)).fetchall()

segments = len(rows)
keys = [k for k in (" ".join(t.split()) for (t,) in rows) if k]   # ADR-0007 §18.2의 규칙
n = len(keys)
counter = collections.Counter(keys)
u = len(counter)
top, r = counter.most_common(1)[0] if counter else ("", 0)

print(f"transcript   {tid} · language={lang} · model={model}")
print(f"segment 총 개수 {segments}")
print(f"문장 n        {n}")
print(f"고유 문장 u    {u}  ({u / n * 100:.1f}%)" if n else "고유 문장 u    0")
print(f"최다 반복      {r}회 ({r / n * 100:.1f}%)  {top!r}" if n else "최다 반복      0")
print(f"소요          {ms / 60000:.2f}분" if ms is not None else "소요          (기록 없음)")
PY
```

### 경우 B — 붕괴로 실패했을 때 (화면의 기술적 표현에서 읽는다)

붕괴하면 Transcript가 저장되지 않으므로 경우 A의 질의에 새 행이 없다. 대신 실패에 붙는
기술적 표현(detail)에 **판정을 재현할 수 있는 값이 전부 들어 있다**
[E1 · `src-tauri/src/transcription/run.rs`의 `collapsed_output`].

```text
segments=… · n=… · u=… · uniqueRatio=… · r=… · topRepeatShare=… · Collapsed{…} · anomalies=…
```

이 줄의 `segments` · `n` · `u` · `uniqueRatio` · `r` · `topRepeatShare`를 그대로 표에
옮긴다. **소요 시간은 이때 시계로 잰 값을 쓴다.**

### 사본의 레벨도 함께 기록한다

§부록2-1 스크립트의 보고서에서 `원본 레벨` · `적용 이득` · `사본 예상 피크`를 그대로 옮긴다.
**표에 적히는 레벨 값이 어떤 방법으로 나온 것인지 함께 적는다** — ADR-0003 §16.1의 값은
40구간 표본이고 이 스크립트의 값은 전 구간이라 **두 값은 다를 수 있다 [미검증]**.

### 마지막 — 원본이 그대로인지 확인한다 (INV-1)

```bash
shasum -a 256 "$SRC"
diff <(shasum -a 256 "$SRC" | cut -d' ' -f1) \
     <(cut -d' ' -f1 "$HOME/molt-level-test/original.sha256")   # 차이가 없어야 한다
```

**해시가 달라졌다면 절차 어딘가가 원본을 건드린 것이다.** 그 실행의 결과를 쓰지 않고
무엇이 원본을 썼는지 먼저 찾는다.

---

## 부록2-4. 결과 — **[미측정]**

**아래 표의 오른쪽 열은 이 Task가 채우지 못했다.** 왼쪽 열은 이미 기록된 실측값이고,
오른쪽 열은 **추정으로 채우지 않는다** (§부록2-5의 이유).

| 항목 | 2026-09-07 **원본** (제품 경로 · `ko` · `large-v3-turbo`) | 2026-09-07 **레벨 보정 사본** (같은 조건) |
| --- | --- | --- |
| 오디오 | `capture-1788746454.wav` · 50.99분 | `capture-1788746454-gain.wav` · 같은 길이 |
| 적용 이득 | 없음 (원본) | **[미측정]** |
| 평균 RMS | 255.7 (**-42.2 dBFS**) · 40구간 표본 | **[미측정]** |
| 전체 피크 | 3494 (**-19.4 dBFS**) · 40구간 표본 | **[미측정]** |
| 총 segment 수 | **103** | **[미측정]** |
| 문장 n | **103** (원 기록이 비율의 분모로 쓴 값 · ADR-0007 §18.1) | **[미측정]** |
| 고유 문장 u (u/n) | **2 (1.9%)** | **[미측정]** |
| 최다 반복 r (r/n) | **102회 (99.0%)** — `한글자막 by 한효정` | **[미측정]** |
| 소요 시간 | **4.30분** (실시간의 약 11.9배) | **[미측정]** |
| 저장된 상태 | `done` (붕괴 판정이 들어오기 전의 실행이다) | **[미측정]** |
| 사람의 판정 | **붕괴** | **[미측정]** |

> 왼쪽 열의 출처: `phase-prompt/05.7` R-2 · R-3 (운영자 실사용 실측) ·
> `docs/ADR-0007-transcription-engine.md` §18.1 · `docs/ADR-0003-recording-engine.md` §16.1.
> `1.9%`와 `99.0%`는 그 기록들이 `2 / 103` · `102 / 103`으로 계산해 둔 값이다.

---

## 부록2-5. 왜 측정하지 못했는가 — 막은 것

**[관측된 사실]** 이 Task(TASK-083)를 실행한 Run에 허용된 명령은
`node tools/loop-runtime/loopctl.mjs self-check` 하나였다. 오디오 변환·앱 실행·전사를
실행할 수단이 이 Run에 없었다.

**[관측된 사실]** 저장소 밖 경로에 대한 접근이 거부됐다. `~/Library/Application Support/`
아래를 나열하려는 시도는 승인 필요로 막혔다.

```text
$ ls -d "$HOME/Library/Application Support/"*molt*
This Bash command contains multiple operations. The following parts require approval: …
```

**[관측된 사실]** 그러므로 이 Run은 **원본 WAV(280 MB)와 `ggml-large-v3-turbo.bin`이 지금
이 기기에 있는지도 확인하지 못했다.** 있다고도 없다고도 적지 않는다.

**[관측된 사실]** §부록2-2의 전사는 GUI 앱(`npm run tauri dev`)을 사람이 조작해야 한다.
이 Run은 비대화형이며 앱을 띄우거나 화면을 조작할 수 없다.

**[관측된 사실]** 시간은 막은 이유가 **아니다.** 원본 실행이 4.30분이었으므로 사본도
그 정도가 예상되지만, 실행 수단 자체가 없어 시간이 문제가 될 자리에 닿지 못했다.

**막지 않은 것:** 절차·측정 방법·비교표는 전부 이 Run에서 확정했다. 남은 것은 §부록2-1 →
§부록2-3을 **사람이 한 번 실행해 오른쪽 열을 채우는 일**이다.

---

## 부록2-6. 결과를 읽을 때 — 레벨이 유일한 원인이라고 단정하지 않는다

**[관측된 사실]** 같은 마이크로 만든 두 파일의 레벨이 16.4 dB 다르고, 낮은 쪽의 전사가
고유 1.9%로 붕괴했다 (`05.7` R-3).

**[유력한 설명]** 입력 레벨이 낮아 whisper가 음성을 찾지 못했고, 빈 자리를 학습 데이터의
잔재로 채웠다. 붕괴가 무음 구간이 아니라 **전 구간**에서 일어났다는 것이 이 설명과 맞는다.

**[미검증] — 이 한 번의 측정으로는 답이 나오지 않는 것들.**

- **두 실행이 레벨만 다르지 않다.** ~~9/4 성공 실행은~~ **[정정 2026-09-07 · TASK-090]
  9/4 파일에서 94.0%를 낸 실행은** `ko` + Metal + `large-v3-turbo` +
  **청크 분할·state 재생성·반복 차단**이 함께 걸린 저장소 밖 도구였고 (같은 파일의
  **제품 경로 실행은 FAIL이다** — '결과 1'), 9/7 실행은
  청크 분할이 없는 제품 경로였다 (9/5 부록 결과 2 · `05.7` R-2). 두 파일은 회의·화자·
  마이크 거리·방도 다르다. **비교 대상이 한 변수로 정리되어 있지 않다.**
- **선형 이득은 SNR을 바꾸지 않는다.** 신호와 잡음을 같은 배수로 키운다. 붕괴가 신호 대
  잡음비 때문이라면 이 사본은 구제되지 않으며, **그때에도 "레벨은 무관하다"는 결론은
  나오지 않는다** — 절대 진폭이 원인인 경우와 구분되지 않기 때문이다.
- **whisper.cpp가 입력을 자체적으로 정규화하는지 이 Run은 확인하지 않았다.** 확인되면 이
  실험의 예상 결과가 달라진다. 확인 전에 예상을 결과처럼 적지 않는다.
- **`05.7` R-1이 기록한 자동 감지 실패(확신도 0.478 → `en`)가 레벨과 관련 있는지도
  확인되지 않았다.**

**그러므로 결과가 어느 쪽이든 이렇게만 적는다.**

```text
구제된다      →  [관측된 사실] 이 파일은 이득 N배에서 고유 X%로 전사됐다.
                 레벨이 이 실패에 기여했다는 것이 이 한 파일에서 확인됐다.
                 유일한 원인인지는 여전히 [미검증]이다.

구제되지 않는다 →  [관측된 사실] 이득 N배에서도 고유 X%였다.
                 레벨만 올리는 것으로는 이 파일이 구제되지 않는다.
                 레벨이 원인이 아니라는 뜻은 아니다 (선형 이득은 SNR을 바꾸지 않는다).
```

---

## 부록2-7. 이 Task가 내리지 않는 결정 — 다음 후보

**측정 결과에 따른 제품 결정은 이 Task가 하지 않는다.** `05.7`의 성공 기준 3이
*"답이 '구제된다'면 제품이 그 처리를 할지는 그 측정 뒤에 정한다 — 측정 전에 정규화 기능을
구현하지 않는다"* 라고 적은 그대로다. **이 Task는 제품 소스·설정·의존성·테스트를 바꾸지
않았고, 정규화나 게인 조정을 제품에 넣지 않았다.**

측정이 끝난 뒤 **사람이** 결정할 후보들이다.

| 후보 | 무엇을 정하는 결정인가 | 지금 상태 |
| --- | --- | --- |
| 전사 입력에 정규화/게인을 넣는가 | 앱이 전사 직전에 레벨을 올려 넣을 것인가 | **측정 대기** — 이 부록의 오른쪽 열 |
| 녹음 시점의 입력 게인을 앱이 조정하는가 | 녹음 엔진 경계의 문제다 (ADR-0003 §16) | `05.7` §하지 않는 것이 이미 보류했다 |
| VAD(음성 구간 검출) | 무음 구간 환각을 줄인다 | 9/5 부록 결과 5의 **[미검증 · 다음 후보]** 그대로 |
| 청크 분할 · state 재생성 · 반복 차단 | 9/5 실험이 효과를 본 세 가지의 제품 이식 | `05.7` §하지 않는 것 — **다음 Phase 후보** |
| 자동 언어 감지 개선 | 확신도 0.478의 `en` 오판 | `05.7` §하지 않는 것 — 설정 경로가 열려 있다 |

**`A-TRANS-001`은 이 부록으로도 해소되지 않는다.** 사람이 레벨을 올려 다시 녹음한 회의가
읽을 만하게 전사되는 것을 볼 때까지 열려 있다 (`05.7` §Human Review).

---

# 부록 3 — Phase 5.8 성공 기준 1을 사람이 판정하는 절차와 기록표 (2026-09-07)

```text
기록 일자:  2026-09-07
기록 Task:  TASK-091 (문서 전용 — 제품 소스 · 설정 · 의존성 · 테스트를 바꾸지 않는다)
Phase:      5.8 — Transcription Chunking (성공 기준 1)
근거:       phase-prompt/05.8-transcription-chunking.md Goal 1 · P-2 · P-4 · Human Review
            docs/ADR-0007-transcription-engine.md §20 · §21
            이 문서의 2026-09-05 부록 (결과 1 · 결과 2) · [정정 2026-09-07] 블록
Status:     절차는 적혔다 · **측정은 실행되지 않았다 ([미측정] — §부록3-5의 이유)**
```

> **§10 · §11의 표도, 2026-09-05 부록도, 부록 2도, [정정 2026-09-07] 블록도 이 부록이
> 지우거나 다시 쓰지 않았다.** 이것은 그 뒤에 덧붙인 네 번째 기록이며, **§부록2-4가 세운
> 선례 그대로** 결과 칸을 전부 `[미측정]`으로 둔다.

> ⚠️ **이 부록이 절차를 적었다는 사실은 측정이 수행됐다는 뜻이 아니다.**
> §부록3-4 · §부록3-6의 칸이 `[미측정]`인 동안, **"제품 경로가 94%에 닿는다"고도
> "닿지 않는다"고도 적지 않는다.** 같은 이유로 Phase 5.8을 *"제품 경로가 읽을 만한 한국어
> 전사를 낸다"* 고 표현하지 않는다 (`phase-prompt/05.8` Goal 1).

**이 부록도 `[관측된 사실]` · `[유력한 설명]` · `[미검증]` 을 구분해 적는다** (부록 2와 같다).

## 부록3-0. 이 측정이 답하려는 질문 하나

`phase-prompt/05.8`의 성공 기준 1이다.

```text
제품 경로가 capture-1788522158.wav 에서 읽을 만한 한국어 전사를 내는가 —
그리고 그 수치가 2026-09-05에 실측된 고유 94.0% 근처에 닿는가?
```

**이 질문이 묻지 않는 것:** 어느 조건이 얼마나 기여했는가. 이 실험은 **한 파일에 대해 오늘의
제품 경로를 한 번 돌리는 것**이며, 기여도 분리는 2026-09-05에도 되지 않았다 (§부록3-7).

### 대상과 조건 — 2026-09-05와 **맞출 수 있는 만큼 맞춘다**

```text
대상 오디오   capture-1788522158.wav
              48 kHz · mono · 16-bit PCM · 400 MB · 1:12:51 (72.85분)
              한국어 3인 실제 회의 · 평균 RMS 1685.5 (-25.8 dBFS · 40구간 표본)
                ← 레벨 값의 출처는 ADR-0003 §16.1이다 (전사 결과가 아니라 파일의 실측 레벨)

경로          제품 (`npm run tauri dev`)   — 저장소 밖의 검증용 도구가 아니다
언어          ko                           (Settings의 Language 칸 · 비워 두지 않는다)
모델          ggml-large-v3-turbo.bin      (2026-09-05가 94.0%를 낸 것과 같은 파일)
청크 분할      제품에 있다                   (ADR-0007 §21.1 — 120초 · 겹침 0 · 청크마다 state 재생성)
```

> **[정정 2026-09-07 · TASK-090]을 이 부록도 그대로 따른다.** `capture-1788522158.wav`는
> **"전사가 성공한 파일"이 아니라 "목표 수치가 알려진 파일"이다** — 같은 문서 '결과 1'이
> 그 파일의 **제품 경로 실행을 FAIL로** 기록하고 있다. 이 부록이 그 파일을 1차 대상으로 쓰는
> 이유는 **도달 가능한 목표가 실측으로 알려진 유일한 파일**이기 때문이다 (`05.8` P-4).

### 조건이 2026-09-05와 **완전히 같지는 않다** — 그 차이를 먼저 적는다

**[관측된 사실 · E1 · ADR-0007 §21.2]** 2026-09-05가 함께 건 세 조건 중 **연속 반복 차단은
오늘의 제품 경로에 연결되어 있지 않다.** `chunking::block_consecutive_repeats`는 순수 함수로
존재하고 테스트가 그 규칙을 값으로 고정하지만, **그 함수를 부르는 제품 코드가 없다** —
저장되는 Transcript는 차단 전 열이다.

| 2026-09-05가 건 조건 | 오늘의 제품 경로 |
| --- | --- |
| `set_language(Some("ko"))` | **있다** (ADR-0007 §17.1 · §19.1) |
| `features = ["metal"]` | **있다** (ADR-0007 §17.2 · §19.2) |
| 120초 청크 분할 + 청크마다 state 재생성 | **있다** (ADR-0007 §21.1) |
| 연속 반복 3회 초과 차단 | **없다 — 부르는 자리가 없다** (ADR-0007 §21.2) |

**그래서 수치를 읽을 때 이것을 함께 읽는다.** 차단은 *연속으로* 이어진 같은 문장을 지우는
규칙이므로, 차단이 없으면 **`u/n`은 낮게 · `r/n`은 높게 나오는 방향**으로 작용한다
— **[미검증]** 그 크기가 얼마인지는 이 프로젝트가 잰 적이 없다 (ADR-0007 §20.6.2의
[미검증] 항목과 같은 이유다: 94.0%는 차단이 켜진 채 잰 값이라 차단 전 값이 알려져 있지 않다).
**추정치를 적지 않는다.**

---

## 부록3-1. 원본은 읽기만 한다 (INV-1)

**이 절차에 원본 오디오를 쓰기 모드로 여는 단계가 없다.**

1. 원본 `capture-1788522158.wav`는 **읽기(`cp`의 원본 · `shasum`)로만 열린다.**
2. 덮어쓰는 대상은 **§6.2 경로 B로 만든 버리는 레코드의 3~5초짜리 파일** 하나다.
3. 절차 앞뒤로 원본의 sha256을 찍어 **같은 값인지 확인한다** (§부록3-3의 마지막 단계).

```bash
APP_DATA="$HOME/Library/Application Support/com.moltnote.app"   # §4.1의 find로 확정한 값
SRC="$APP_DATA/recordings/capture-1788522158.wav"

mkdir -p "$HOME/molt-chunking-test"
ls -l "$SRC"                       # 400 MB · 이 파일은 이 절차 내내 바뀌지 않는다
shasum -a 256 "$SRC" | tee "$HOME/molt-chunking-test/original.sha256"
```

**사본을 만들지 않는다.** 부록 2와 달리 이 실험은 오디오를 가공하지 않는다 — 바뀌는 것은
**제품 코드 쪽(청크 분할)**이고 입력은 9/5와 같은 파일 그대로여야 한다.

---

## 부록3-2. 실행 — 제품 경로로 전사한다

### 1) 설정 (Settings 화면 · §7.1의 절차)

1. **Whisper model** 칸에 `ggml-large-v3-turbo.bin` — `<APP_DATA>/models/`에 있어야 한다
   (없으면 §4의 절차로 놓는다. 절대 경로도 된다 · §4.3).
2. **Language** 칸에 `ko` — **비워 두지 않는다.** 빈 값은 자동 감지이며, 이 실험은 9/5와
   조건을 맞추기 위해 언어를 지정한다.
3. **Save** → `Saved.`

### 2) 원본을 담을 **버리는 레코드**를 만든다 (§6.2 경로 B)

```bash
# 3~5초 녹음하고 Stop 한 직후에 실행한다.
NEW=$(ls -t "$APP_DATA/recordings"/capture-*.wav | head -1)
echo "$NEW"        # 방금 만든 레코드의 파일인지 눈으로 확인한다

case "$NEW" in
  *capture-1788522158.wav|*capture-1788746454.wav)
    echo "중단: 이것은 원본이다"; exit 1 ;;
esac

cp "$SRC" "$NEW"   # 읽는 것은 원본, 쓰는 것은 방금 만든 버리는 레코드의 파일이다
```

- **전사를 시작하기 전에** 바꾼다.
- 목록의 길이·크기가 3~5초로 보이는 것은 녹음 세션이 기록한 값이다. **실패가 아니다** (§6.2).
- 이것은 **운영자가 자기 fixture를 놓는 준비 작업**이며, INV-1이 금지하는 것은 *앱 코드가
  원본을 덮어쓰는 것*이다 (§6.2와 같은 판단).

### 3) 전사를 시작하고 **시계로도 시간을 잰다**

1. 새 레코드의 상세 화면에서 **Start transcription**.
2. 시작·종료 시각을 적는다. **시계로도 재는 이유:** 붕괴로 판정되면 Transcript가 저장되지
   않아 `transcripts.transcription_ms`가 남지 않는다
   [E1 · `src-tauri/src/transcription/run.rs` — 붕괴는 저장 **전에** 실패로 돌아간다].

> **[관측된 사실 · E1]** 저장 직전 붕괴 판정이 들어와 있다 (`transcription/collapse.rs` ·
> `n >= 20`이고 `u/n <= 0.20` 또는 `r/n >= 0.50`이면 붕괴). **이번 실행이 `done`이 아니라
> 실패로 끝날 수 있고, 그것 자체가 정상 동작이다.** 두 경우의 측정 방법이 §부록3-3에 있다.

---

## 부록3-3. 결과를 어떻게 재는가 — **§부록2-3과 같은 방법이다**

**새 계산법을 만들지 않는다.** 재는 값도, 문장을 세는 규칙도, 읽는 자리도 §부록2-3이 이미
정한 그대로다 (그 규칙의 출처는 `ADR-0007 §18.2`다 — 앞뒤 공백을 걷고 내부 연속 공백을 하나로
줄인 문자열이 문장이며, 빈 문자열은 세지 않는다).

```text
총 segment 수        전사가 낸 segment의 개수
문장 n               빈 문장을 뺀 문장의 총 개수
고유 문장 u 와 u/n    서로 다른 문장의 개수와 그 비율
최다 반복 r 과 r/n    가장 많이 나온 문장의 출현 횟수와 그 비율
최다 반복 문장        그 문장 자체 (2026-09-05 · 2026-09-07에도 문장을 함께 적었다)
소요 시간            transcripts.transcription_ms  또는  시계로 잰 값
```

| 경우 | 어디서 읽는가 | 스크립트 |
| --- | --- | --- |
| **A — `done`으로 저장됐다** | DB를 **읽기 전용**으로 열어 가장 최근 transcript를 센다 | **§부록2-3 '경우 A'의 스크립트를 그대로 쓴다.** 여기 다시 쓰지 않는다 |
| **B — 붕괴로 실패했다** | 화면의 기술적 표현(detail) 한 줄에 판정을 재현할 값이 전부 있다 | **§부록2-3 '경우 B'** — `segments=… · n=… · u=… · uniqueRatio=… · r=… · topRepeatShare=… · Collapsed{…}` 를 그대로 표에 옮기고, **소요 시간은 시계로 잰 값을 쓴다** |

**경우 A에서도 최다 반복 문장을 함께 적는다.** *무엇이* 되풀이됐는지가 사람에게는 가장 빠른
단서이며, 2026-09-05(`한글자막 by 한효정`)와 2026-09-05 제품 경로
(`But I was like, "What are you doing?"`)가 그렇게 기록됐다.

### 마지막 — 원본이 그대로인지 확인한다 (INV-1)

```bash
diff <(shasum -a 256 "$SRC" | cut -d' ' -f1) \
     <(cut -d' ' -f1 "$HOME/molt-chunking-test/original.sha256")   # 차이가 없어야 한다
```

**해시가 달라졌다면 절차 어딘가가 원본을 건드린 것이다.** 그 실행의 결과를 쓰지 않고
무엇이 원본을 썼는지 먼저 찾는다.

---

## 부록3-4. 결과 — **[미측정]**

**오른쪽 열은 이 Task가 채우지 못했다.** 왼쪽 두 열은 이미 기록된 실측값이고, 오른쪽 열은
**추정으로 채우지 않는다** (§부록3-5의 이유 · §부록2-4가 세운 선례 그대로다).

| 항목 | 2026-09-05 **제품 경로** (언어 미설정 → `en` · `base`) | 2026-09-05 **저장소 밖 도구** (`ko` + Metal + `large-v3-turbo` + 청크분할·차단) | **오늘의 제품 경로** (`ko` · `large-v3-turbo` · 청크분할) |
| --- | --- | --- | --- |
| 오디오 | `capture-1788522158.wav` · 72.85분 | 같은 파일 | 같은 파일 |
| 실행 날짜 · 실행자 | 2026-09-05 · 운영자 | 2026-09-05 · 운영자 | **[미측정]** |
| 관측된 `language` | `en` (설정되지 않은 기본값 · **결함**) | (기록 없음 — `ko`로 지정했다) | **[미측정]** |
| 총 segment 수 | **1,711** | **1,749** | **[미측정]** |
| 문장 n | **1,711** (원 기록이 비율의 분모로 쓴 값) | **1,749** (같음) | **[미측정]** |
| 고유 문장 u (u/n) | **59 (3.4%)** | **1,643 (94.0%)** ← **기준선** | **[미측정]** |
| 최다 반복 r (r/n) | **1,063회 (62.1%)** | **10회 (0.6%)** | **[미측정]** |
| 최다 반복 문장 | `But I was like, "What are you doing?"` | (기록 없음) | **[미측정]** |
| 소요 시간 | 약 **26분** | **6.0분** (실시간의 약 12배) ← **기준선** | **[미측정]** |
| 저장된 상태 | `done` (붕괴 판정이 들어오기 전의 실행이다) | (제품이 아니다) | **[미측정]** |
| 사람의 판정 | **붕괴** (한글 0줄) | **쓸 수 있다** | **[미측정]** |

> 왼쪽 두 열의 출처: 이 문서의 2026-09-05 부록 '결과 1' · '결과 2' ·
> `docs/ADR-0007-transcription-engine.md` §18.1. `3.4%` · `94.0%` · `62.1%` · `0.6%`는
> 그 기록들이 `59/1,711` · `1,643/1,749` · `1,063/1,711` · `10/1,749`로 계산해 둔 값이다.
> **가운데 열은 저장소 밖의 검증용 도구가 낸 값이며 제품이 낸 값이 아니다.**

**판정 기준 — 사람이 무엇을 보고 성공이라고 말하는가.**

```text
기준선   고유 94.0% · 6.0분   (2026-09-05 · 저장소 밖 도구 · 조건 넷)
비교선   고유 3.4%            (2026-09-05 · 제품 경로 · 언어 미설정)

이 실행의 자리를 그 둘 사이에 놓는다. **숫자 하나로 PASS/FAIL을 선언하지 않는다** —
조건이 9/5와 하나 다르고(§부록3-0), 관측이 한 번뿐이며, 읽을 만한가는 §부록3-5가
사람에게 묻는다.
```

---

## 부록3-5. Human Review — **사람이 답하는 칸** (자동 Gate가 판정할 수 없다)

`phase-prompt/05.8`의 Human Review 항목이며, **왜 자동 검증이 이것들에 답하지 못하는지는
`docs/ADR-0007-transcription-engine.md` §21.3에 있다.** 아래 답 칸은 **비어 있다.**

| # | 사람이 답할 질문 | 무엇을 보고 답하는가 | 답 |
| --- | --- | --- | --- |
| **HR-A** | **그 전사가 한국어로 읽히는가** (자동 수치가 아니라 사람이 읽어서) | Transcript 탭을 처음부터 끝까지 훑는다. 한글이 나오는가 · 회의에서 실제로 오간 말로 읽히는가 | **[미기입]** |
| **HR-B** | **청크 경계에서 시각이 되감기지 않는가** | 2분(00:02:00) · 4분 · 6분 … 즉 **120초의 배수** 자리에서 시각이 앞으로만 가는지 본다. 되감긴다면 그 자리의 두 줄을 그대로 옮겨 적는다 | **[미기입]** |
| **HR-C** | **청크 경계에서 문장이 잘리거나 중복되지 않는가** | 같은 자리에서 말이 자연스럽게 이어지는지 읽는다. **겹침이 0이므로 경계에 걸친 문장이 두 조각으로 잘릴 수 있다** — 그것이 실제로 얼마나 일어나는지는 [미검증]이다 (ADR-0007 §20.4) | **[미기입]** |
| **HR-D** | **소요 시간이 2026-09-05의 6.0분과 크게 다르지 않은가** | §부록3-4의 소요 시간을 6.0분과 비교한다. 크게 길다면 그 사실만 적는다 — **원인을 단정하지 않는다** (청크마다의 state 재생성 비용은 이 프로젝트가 잰 적이 없다 · ADR-0007 §20.10) | **[미기입]** |

**답을 적을 때의 규칙.** *"괜찮다"* 로 적지 않는다 — **무엇을 보고 그렇게 판단했는지**를 함께
적는다 (본 구간 · 옮겨 적은 줄 · 잰 시각). 그 근거가 없으면 다음 사람이 같은 판단을 처음부터
다시 해야 한다.

### 왜 이 Task가 채우지 못했는가 — 막은 것

**[관측된 사실]** 이 Task(TASK-091)를 실행한 Run에 허용된 명령은
`node tools/loop-runtime/loopctl.mjs self-check` 하나였다. 오디오를 다루거나 앱을 띄우거나
전사를 실행할 수단이 이 Run에 없었다.

**[관측된 사실]** §부록3-2의 전사는 GUI 앱(`npm run tauri dev`)을 **사람이 조작해야 한다.**
이 Run은 비대화형이며 창을 띄우거나 화면을 조작할 수 없다.

**[관측된 사실]** 이 Run은 저장소 밖 경로를 확인하지 않았다 — **원본 WAV와
`ggml-large-v3-turbo.bin`이 지금 이 기기에 있는지도 확인하지 못했다.** 있다고도 없다고도
적지 않는다 (`05.8` P-4는 2026-09-07 기준으로 그것들이 있다고 기록했다).

**막지 않은 것:** 절차 · 측정 방법 · 기준선 · 기록표는 전부 이 Run에서 확정했다. 남은 것은
§부록3-1 → §부록3-3을 **사람이 한 번 실행해 오른쪽 열과 답 칸을 채우는 일**이다.

---

## 부록3-6. `capture-1788746454.wav` — **목표 수치를 세우지 않는다. 관측만 기록한다**

**[관측된 사실 · `05.8` P-4]** 이 파일은 레벨이 16.4 dB 부족하다(-42.2 dBFS). **그 파일에서
무엇이 가능한지는 알려져 있지 않으며, `phase-prompt/05.8`이 여기에 목표 수치를 세우지
않기로 정했다.** 그러므로 아래 표에는 **기준선 열이 없다.**

같은 절차(§부록3-2 · §부록3-3)를 이 파일에 대해 돌린다면 그 결과를 여기에 적는다. **돌리지
않아도 된다** — 성공 기준 1의 대상이 아니다.

| 항목 | 값 |
| --- | --- |
| 실행 여부 | **[미측정]** (실행하지 않았다면 그렇게 적는다) |
| 실행 날짜 · 조건 | **[미측정]** (조건은 §부록3-0과 같아야 비교가 성립한다) |
| 총 segment 수 | **[미측정]** |
| 문장 n | **[미측정]** |
| 고유 문장 u (u/n) | **[미측정]** |
| 최다 반복 r (r/n) · 그 문장 | **[미측정]** |
| 소요 시간 | **[미측정]** |
| 저장된 상태 (`done` / 붕괴 실패) | **[미측정]** |
| 사람이 읽은 인상 | **[미측정]** |

**참고 — 이미 기록된 이 파일의 값** (비교를 위해 옮겨 적을 뿐, 목표가 아니다):
2026-09-07 제품 경로 · `ko` · `large-v3-turbo` · 청크 분할 **없던** 시점 —
segment 103 · 고유 2 (1.9%) · `한글자막 by 한효정` 102회 (99.0%) · 4.30분 · `done`으로 저장 ·
사람의 판정 **붕괴** (`05.7` R-2 · R-3 · ADR-0007 §18.1).

**레벨 실험(부록 2)과 섞지 않는다.** 부록 2가 묻는 것은 *레벨을 올리면 구제되는가*이고,
여기서 묻는 것은 *오늘의 제품 경로가 이 파일에서 무엇을 내는가*다. **둘은 다른 실험이며,
어느 쪽도 다른 쪽의 답이 되지 않는다.**

---

## 부록3-7. 결과를 읽을 때 — 이 한 번의 실행이 답하지 않는 것

**[관측된 사실]** 2026-09-05에 같은 오디오에서 조건 넷을 함께 걸었을 때 고유 94.0%가 나왔고,
그 도구는 **저장소 밖의 검증용 도구**였다 (이 문서 '결과 2' · [정정 2026-09-07]).

**[관측된 사실 · E1]** 오늘의 제품 경로에는 그 조건 중 셋이 있고 하나(연속 반복 차단)가
없다 (§부록3-0 · ADR-0007 §21.2).

**[미검증] — 이 한 번의 측정으로는 답이 나오지 않는 것들.**

- **네 조건의 개별 기여도.** 2026-09-05도 분리하지 않았고 (ADR-0007 §20.10), 이 실행도
  분리하지 못한다. **어느 수치가 나오든 "청크 분할이 X%를 만들었다"고 적지 않는다.**
- **차단이 없다는 것이 수치를 얼마나 낮추는가.** 방향은 말할 수 있어도 크기는 알려져 있지
  않다 (§부록3-0).
- **이 결과가 다른 오디오 · 다른 모델 · 다른 언어에서도 같은가.** 관측은 한 기기 · 한 사람 ·
  한 파일이다.
- **붕괴 판정의 임계값이 이 실행에 맞는가.** 근거는 여전히 세 관측뿐이다 (ADR-0007 §18.7).

**그러므로 결과가 어느 쪽이든 이렇게만 적는다.**

```text
94% 근처에 닿았다     →  [관측된 사실] 오늘의 제품 경로는 이 파일에서 고유 X% · Y분이었다.
                        조건 하나(반복 차단)가 9/5와 다르다는 사실과 함께 적는다.
                        A-TRANS-001을 닫는 것은 이 수치가 아니라 **사람의 판정**이다.

닿지 않았다           →  [관측된 사실] 고유 X%였다. 무엇이 부족한지는 이 실행이 말하지 않는다.
                        (차단 미연결 · 모델 · 레벨 · 그 밖의 조건이 구분되지 않는다.)
                        **원인을 단정하지 않고** 다음 후보를 §부록3-8에 적는다.
```

## 부록3-8. 이 부록이 내리지 않는 결정 — 다음 후보

**측정 결과에 따른 제품 결정은 이 Task가 하지 않는다.** 이 Task는 제품 소스 · 설정 · 의존성 ·
테스트를 바꾸지 않았다.

| 후보 | 무엇을 정하는 결정인가 | 지금 상태 |
| --- | --- | --- |
| 연속 반복 차단을 제품 경로에 연결하는가 | ADR-0007 §20.6.2가 정한 자리(붕괴 판정 뒤 · 저장 앞)에 호출을 두는 일 | **규칙과 함수는 있고 부르는 자리가 없다** (ADR-0007 §21.2) |
| 겹침을 두는가 | 경계에서 잘리는 문장을 줄일 것인가 | ADR-0007 §20.4가 **0으로 확정**했다. 바꾸려면 병합 규칙과 비교 관측이 먼저 필요하다 |
| 청크 길이 · 반복 임계값을 바꾸는가 | 120초 · 3회 말고 다른 값 | 근거가 관측 하나뿐이다 (ADR-0007 §20.10). 재기 전에 바꾸지 않는다 |
| 엔진 교체 (Qwen3-ASR) | 제품 경로의 기준선을 만든 **뒤**의 결정 | ADR-0008 · Phase 5.9 이후 (`05.8` §하지 않는 것) |
| VAD · 디코딩 파라미터 · 녹음 레벨 정규화 | 각각 별개의 처방 | `05.7` · `05.8`이 이미 보류했다 (ADR-0007 §17.4 · §18.6 · §20.9) |

**`A-TRANS-001`은 이 부록으로도 자동으로 해소되지 않는다.** §부록3-4의 오른쪽 열과 §부록3-5의
답 칸이 채워지고 **사람이 "읽을 만하다"고 판정할 때** 그것을 닫는 Task가 `ADR-0007 §16.3.1`과
`docs/SYSTEM-MAP.md` §7을 함께 고친다 (ADR-0007 §21.4).
