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
