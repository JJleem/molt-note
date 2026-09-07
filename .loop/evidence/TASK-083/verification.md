# TASK-083 — Evidence

```text
Run       RUN-20260907T045640Z-TASK-083
Task      TASK-083 (impl · 문서 전용)
대상      docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md
```

## 1. 이 Run이 바꾼 것 — 파일 하나, 삭제 0줄

```text
$ git diff --numstat -- docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md
416     0       docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md
```

**추가 416줄 · 삭제 0줄.** 기존 내용을 지우거나 다시 쓴 자리가 없다 (AC-1).
전체 diff는 `appendix.diff`에 있다.

작업 트리에는 이 Run 이전부터 다른 Phase 5.7 Task들이 만든 수정이 남아 있다
(Run 시작 시점의 `git status` 스냅샷에 이미 있던 것들이다). **이 Run이 손댄 파일은
위 한 개뿐이다.**

## 2. 보존 확인 (AC-1)

부록을 붙인 뒤에도 §10 표와 2026-09-05 부록이 원래 자리에 그대로 있다.

```text
$ grep -n "^## 10\. \|^### 10\.1\|^### 10\.2\|^## 11\. \|^# 부록" docs/PHASE-3-TRANSCRIPTION-SMOKE-TEST.md
488:## 10. Phase 3가 확인한 것 / 확인하지 않은 것
492:### 10.1 자동 검증이 확인한 것 (VERIFIED)
512:### 10.2 확인되지 않은 것 — 실행되지 않았거나 연기됐다
531:## 11. smoke test 실행 기록 — 운영자가 채운다
555:# 부록 — 실제 실행 결과 (2026-09-05)
689:# 부록 2 — 레벨을 올린 사본으로 다시 전사하는 절차와 결과 (2026-09-07)
```

기존 헤딩의 줄 번호가 변경 전과 같다(488 · 492 · 512 · 531 · 555). 새 부록은 문서 **뒤**
(689줄)에만 붙었다.

## 3. 제품 경계 (AC-4)

- `src-tauri/**` · `src/**` · `package.json` · `Cargo.toml` · `.loop/**` 설정 — **이 Run에서
  수정하지 않았다.** 정규화·게인 조정 기능을 제품에 넣지 않았다.
- 부록의 절차는 원본 WAV를 `'rb'`로만 열고, `mv`/`rm`/in-place 편집이 없으며, 사본을
  `recordings/` 밖에 만든다. 경로 B에서 덮어쓰는 대상은 **운영자가 방금 만든 버리는
  레코드의 파일**이고, 절차에 원본 파일명(`capture-1788746454.wav`)이면 중단하는 가드가
  들어 있다. 마지막 단계에서 원본 sha256을 다시 대조한다.

## 4. 측정을 실행하지 못한 이유 (AC-3)

부록 §부록2-5에 기록한 것과 같다. **관측된 사실만 적었다.**

- 이 Run에 허용된 명령은 `node tools/loop-runtime/loopctl.mjs self-check` 하나였다.
  오디오 변환·앱 실행·전사를 실행할 수단이 없었다.
- 저장소 밖 경로 접근이 승인 필요로 거부됐다:

  ```text
  $ ls -d "$HOME/Library/Application Support/"*molt*
  This Bash command contains multiple operations. The following parts require approval: …
  ```

  그래서 원본 WAV와 `ggml-large-v3-turbo.bin`의 존재 여부도 **확인하지 못했다** —
  있다고도 없다고도 문서에 적지 않았다.
- 전사는 GUI 앱(`npm run tauri dev`)을 사람이 조작해야 하며 이 Run은 비대화형이다.

따라서 §부록2-4 비교표의 **오른쪽 열은 전부 `[미측정]`이다.** 왼쪽 열의 값
(103 · 2 · 1.9% · 102회 99.0% · 4.30분 · -42.2 dBFS · -19.4 dBFS)은 전부 기존 기록에서
왔으며 출처를 표 아래에 적었다:

- `phase-prompt/05.7-recording-level-and-transcription-collapse.md` R-2 · R-3
- `docs/ADR-0007-transcription-engine.md` §18.1
- `docs/ADR-0003-recording-engine.md` §16.1

**이 Run이 새로 측정해 만든 수치는 하나도 없다.**

## 5. 절차가 저장소에서 확인한 것 (AC-2의 근거)

| 절차에 적힌 값 | 확인한 자리 |
| --- | --- |
| `<APP_DATA>/molt-note.db` · `recordings/` · `models/` | `src-tauri/src/platform/app_data_dir.rs` |
| identifier `com.moltnote.app` | `src-tauri/tauri.conf.json` |
| Settings의 **Language** 칸 (빈 값 = 자동 감지) | `src/screens/SettingsScreen.tsx:440-453` |
| 입력 형식 조건 (PCM 16-bit · mono/stereo · 리샘플) | 같은 문서 §5.1 · `transcription/audio_input.rs` |
| `transcripts(transcription_ms)` · `transcript_segments(ordinal, text)` | `src-tauri/src/db/migrations.rs:69-88` · `:310` |
| 문장 세는 규칙 (앞뒤 공백 제거 + 내부 연속 공백 축약 + 빈 문장 제외) | `docs/ADR-0007-transcription-engine.md` §18.2 · `transcription/collapse.rs` |
| 붕괴 시 저장 전에 실패로 돌아가고 detail에 수치가 남는다 | `src-tauri/src/transcription/run.rs`의 `collapsed_output` |
| 붕괴 임계값 `n>=20` · `u/n<=0.20` · `r/n>=0.50` | `src-tauri/src/transcription/collapse.rs:65-78` |
| 40구간 표본 레벨 측정 · dBFS 기준 32768 | `docs/ADR-0003-recording-engine.md` §16.1 |

## 6. Gate

Task의 `stop_condition`이 `gates: (none enabled)`이므로 이 Run에서 Gate를 실행하지 않았다.
변경은 마크다운 문서 한 개뿐이며 build · lint · test의 입력에 닿지 않는다.
