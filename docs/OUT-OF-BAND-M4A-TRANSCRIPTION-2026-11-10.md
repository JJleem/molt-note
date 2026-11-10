# 외부 m4a를 제품 경로 밖에서 전사한 기록 — 2026-11-10

```text
Status:   운영자 실험 기록. **제품 기능이 아니다.** 저장소 코드는 바뀌지 않았다.
Date:     2026-11-10
Scope:    외부 오디오 파일 1건을 전사·요약하기 위해 무엇을 했는가 ·
          그 과정이 제품 경로에 대해 무엇을 확인해 주었는가 ·
          무엇이 여전히 UNVERIFIED인가
Related:  docs/ADR-0007-transcription-engine.md (§9 입력 포맷 · §18 전사 붕괴 · §20 청크)
          docs/GIT-WORKFLOW.md §7 (이 문서에 녹음 내용이 없는 이유)
```

> **녹음 내용은 이 문서에 없다.** 저장소는 공개다 (`docs/GIT-WORKFLOW.md` §1).
> §7이 "개인 녹음 · private transcript · 실제 회의 내용"을 push 금지 대상으로 못 박았으므로,
> 여기 남기는 것은 **방법과 측정값뿐**이다. 오디오 · 전사 · 요약 산출물은 저장소 밖
> 임시 디렉터리에만 있었고 커밋되지 않았다.

---

## 1. 무엇을 하려 했나

운영자가 외부에서 받은 **61분 05초 한국어 m4a**(AAC · 48 kHz · stereo) 한 건을
전사하고 요약하려 했다. 먼저 **제품(Molt Note)으로 처리할 수 있는지**를 확인했고,
안 된다는 결론이 나와 **제품 경로 밖에서** 처리했다.

## 2. 제품으로는 왜 안 되는가 — 두 가지 (VERIFIED, 코드 확인)

### 2.1 외부 오디오 파일을 받는 입구가 없다

`src-tauri/src/lib.rs`의 `generate_handler!`에 등록된 커맨드 가운데 **외부 파일을
가져오는 것이 없다.** 녹음 레코드는 앱이 직접 캡처한 결과에서만 생긴다
(`commands::save_capture` → `finish_recording`). `src/`에도 파일 선택 UI가 없다.

`create_recording`이 `audio_path`를 문자열로 그대로 받기는 한다
(`src-tauri/src/commands/payload.rs:87`). 즉 **DB에 손으로 레코드를 꽂는 우회는
물리적으로 가능**하지만, 그것은 제품 경로가 아니라 검증되지 않은 수동 주입이다.

### 2.2 전사 입력 디코더가 WAV 전용이다

`src-tauri/src/transcription/audio_input.rs`는 `hound`로 열고 다음만 받는다:

```text
sample_format == Int  &&  bits_per_sample == 16        (audio_input.rs:173)
1 <= channels <= 2                                     (audio_input.rs:183)
```

그 밖은 `FailureKind::InvalidInput`으로 돌아간다. **m4a(AAC) 디코더는 없다.**
이것은 누락이 아니라 결정이다 — ADR-0007 §9.2가 "사용자에게 ffmpeg 설치를
요구하지 않는다"를 선택했고, 그래서 제품 경로에 AAC 디코딩 단계가 없다.

## 3. 밖에서 어떻게 처리했나

제품이 쓰는 것과 **같은 모델**을 쓰되, 실행은 제품이 아니라 CLI로 했다.

```text
도구:   ffmpeg 8.1.1 (이미 설치돼 있었다)
        whisper-cpp 1.9.5  ← brew로 이번에 설치했다 (제품 의존성이 아니다)
모델:   ~/Library/Application Support/com.moltnote.app/models/
          ggml-large-v3-turbo.bin     (전사)
          ggml-silero-v5.1.2.bin      (VAD)
산출물: 저장소 밖 임시 디렉터리. 커밋하지 않았다.
```

### 3.1 변환

```bash
ffmpeg -i <입력>.m4a -ar 16000 -ac 1 -c:a pcm_s16le <출력>.wav
```

### 3.2 1차 전사 — 기본 설정으로 돌렸고, 실패했다

```bash
whisper-cli -m ggml-large-v3-turbo.bin -f <출력>.wav -l ko -otxt -osrt
```

```text
소요            254초 (Metal)
세그먼트        1,973개 — 그중 서로 다른 것 214개
붕괴            19:40부터 같은 문장 1,361회 반복, 41분 18초를 덮었다
실질 커버리지   앞 20분
```

### 3.3 2차 전사 — 컨텍스트를 끊고 VAD를 붙였다. 통과했다

```bash
whisper-cli -m ggml-large-v3-turbo.bin -f <출력>.wav -l ko \
  -mc 0 --vad -vm ggml-silero-v5.1.2.bin -otxt -osrt
```

```text
소요            108초 (1차의 42%)
세그먼트        915개 — 그중 서로 다른 것 903개
연속 반복 런    5개 이상인 구간 0건
커버리지        61분 전체
```

두 변경 중 **어느 쪽이 결정적이었는지는 분리 측정하지 않았다.** 둘을 같이 바꿨다.

### 3.4 구간 정밀 재전사 — 고유명사 교정

관심 구간 2개(각 5분 40초)를 잘라 beam search와 어휘 프롬프트를 주고 다시 돌렸다.

```bash
whisper-cli ... -bs 8 -bo 8 -mc 64 --prompt "<도메인 고유명사 목록>" --vad -vm ...
```

```text
소요            구간당 9~14초
효과            제품명 · 기술 용어가 교정됐다 (예: 1차에서 뜻이 깨졌던 문장이
                문맥상 올바른 용어로 복원됐다)
부작용          VAD가 긴 발화를 최대 2분 블록으로 묶어 타임스탬프 해상도가 떨어졌다
주의            `-mc 0`과 `--prompt`는 함께 쓰기 어렵다. 컨텍스트 토큰을 0으로 두면
                초기 프롬프트도 함께 버려질 수 있어 `-mc 64`로 타협했다.
                이 상호작용은 측정으로 확인하지 않았다 — UNVERIFIED.
```

요약은 전사 텍스트를 읽고 대화형 세션이 작성했다. 제품의 AI 노트 경로
(`ai/note.rs` · `ai/prompt.rs`)는 사용하지 않았다.

## 4. 이 실험이 제품 경로에 대해 확인해 준 것

**1차의 붕괴는 제품 결함의 증거가 아니다.** 제품은 1차와 같은 방식으로 돌리지 않는다.

| | 1차 CLI 실행 | 제품 경로 |
| --- | --- | --- |
| 입력 분할 | 61분을 한 번에 | 120초 청크 (`chunking.rs:81`) |
| 청크 간 컨텍스트 | 캐리오버 (기본값) | **청크마다 state 재생성** (`whisper.rs:330`) · 겹침 0 (`chunking.rs:96`) |
| VAD | 꺼짐 | **켜짐** · silero (`whisper.rs:379`~`395`) |
| 붕괴 판정 | 없음 | `collapse.rs` (ADR-0007 §18) |

즉 제품은 ADR-0007 §20이 정한 청크·state 규칙과 §18의 붕괴 판정으로 **이 실패
양상을 구조적으로 막아 두었다.** 1차 결과는 그 규칙이 장식이 아니라는 실측 근거다.

**단, 이 실험이 제품 경로로 61분 녹음을 전사해 본 것은 아니다.** 제품 경로의
실제 전사 품질은 이 문서가 말할 수 있는 범위가 아니다 (ADR-0007 §19.5 ·
`docs/PHASE-5.6-HUMAN-REVIEW.md`가 같은 경계를 이미 적어 두었다).

## 5. 후보 개선 — 근거는 하나뿐이므로 결정하지 않는다

**초기 프롬프트로 어휘를 주면 고유명사 정확도가 올라간다.** §3.4에서 관측했다.
제품 경로에는 그런 주입이 없다 (`whisper.rs`에 프롬프트 설정이 보이지 않는다).

제품에 넣는다면 고려해야 할 것:

```text
무엇을 어휘로 줄 것인가        사용자 설정? 노트 제목? 이전 전사?
청크마다 줄 것인가             state를 청크마다 버리는 규칙(§20)과의 관계
붕괴 위험                      프롬프트가 컨텍스트로 들어가면 §18의 조건이 바뀔 수 있다
```

**관측 1건이다.** `CLAUDE.local.md`의 Field-Test Principle에 따라 지금 구현하지
않는다. 반복 증거가 모이면 ADR-0007에 후속 절로 올린다.

## 6. 상태 정리

```text
VERIFIED     제품에 외부 오디오 import 경로가 없다 (코드 확인)
VERIFIED     전사 입력은 PCM16 WAV · 1~2채널만 받는다 (코드 확인)
VERIFIED     CLI 기본 설정으로 61분을 한 번에 돌리면 붕괴했다 (측정)
VERIFIED     `-mc 0` + VAD 조합에서는 붕괴하지 않았다 (측정)
UNVERIFIED   두 변경 중 무엇이 결정적이었는가 (분리 측정 안 함)
UNVERIFIED   `-mc 0`과 `--prompt`의 상호작용
UNVERIFIED   제품 경로(`whisper-rs` · 120초 청크)로 같은 파일을 돌린 결과
NOT DONE     외부 파일 import는 제품 기능으로 존재하지 않는다.
             필요하다면 Phase Goal로 올려야 하며, AAC 디코딩 결정은
             ADR-0007 §9.2를 건드린다.
```

## 7. 저장소에 남지 않은 것

```text
오디오 원본 · 변환 WAV · 전사 txt/srt · 요약       모두 임시 디렉터리에만 있었다
```

`.gitignore`가 `*.m4a` · `*.wav` · `/transcripts/`를 이미 제외한다. 저장소 루트에
있던 m4a 사본도 그 규칙으로 추적되지 않았고, 이 커밋에 포함되지 않았다
(`git check-ignore`로 확인 가능).
