# TASK-081이 바꾼 것

## 주의 — `diff.patch`의 범위

`diff.patch`는 **작업 트리와 HEAD(eaf84f4)의 차이**를 네 파일에 대해 뜬 것이다. 이 저장소의
작업 트리에는 이 Task 이전의 커밋되지 않은 Phase 작업이 이미 들어 있었으므로
(`src-tauri/src/commands/mod.rs` · `payload.rs`는 시작 시점에 이미 `M` 상태였다),
그 파일들의 diff 전부가 이 Task의 것은 아니다. **이 Task가 실제로 더한 것은 아래가 전부다.**

## 1. `src-tauri/src/audio/capture.rs` — 재는 자리 (ADR-0003 §16.2)

- `drain(receiver, file, level: &Mutex<InputLevel>)` — 시그니처에 레벨이 하나 늘었다.
  `Packet::Samples`가 **파일에 쓰인 뒤에만** `level.push(&chunk)`를 부른다.
  일시정지 구간(`writing == false`)은 예전과 똑같이 아무것도 하지 않는다.
- `ActiveCapture`에 `level: Arc<Mutex<InputLevel>>` 필드와 `pub fn level(&self) -> Option<LevelReading>`.
- `start()`이 그 `Arc` 하나를 만들어 쓰는 쪽(스레드)과 읽는 쪽(`ActiveCapture`)에 나눠 준다.
- 새 스레드도, 새 통로도, 새 플랫폼 경계도 만들지 않았다.
- 단위 검사 둘 추가 (`the_level_sees_exactly_the_samples_that_reached_the_file` ·
  `a_capture_that_wrote_nothing_has_no_level_rather_than_zero`).

**샘플을 바꾸는 코드는 하나도 없다** (AC-4). `push`가 받는 것은 읽기 전용 슬라이스
(`&[i16]`)이고, `file.write(&chunk)`에 넘어가는 값은 이전과 완전히 같다 — 곱하거나 자르거나
정규화하는 자리가 새로 생기지 않았다. 그래서 만들어지는 WAV의 샘플 값도 예전과 같으며,
그 사실을 기존 검사(`the_whole_path_runs_start_pause_resume_stop_and_finishes_one_file`이
확정된 WAV를 다시 읽어 샘플 값을 직접 본다)가 그대로 지킨다.

## 2. `src-tauri/src/audio/system_capture.rs` — **바꾸지 않았다**

실시간 오디오 콜백에는 레벨 계산이 들어가지 않았다 (AC-4). 이 파일은 이 Task에서 한 줄도
바뀌지 않았다.

## 3. `src-tauri/src/commands/payload.rs` — 나가는 값

- `InputLevelPayload` 추가. 필드는 **넷뿐**이다:
  `averageDbfs` · `peakDbfs` · `verdict` · `message` (ADR-0003 §16.3이 정한 네 값).
  **샘플도, 샘플 배열도, 파형도, 스펙트럼도 없다** (INV-6 · AC-5).
- `verdict`의 wire 값은 `usable · low · silent` — `SessionState::as_str`과 같은 성질의
  중립적인 식별자다. **벤더 고유 개념이 아니다** (INV-9 · AC-5).
- `SessionStatusPayload`에 `level: Option<InputLevelPayload>` 추가.
  진행 중인 녹음이 없거나 아직 파일에 쓰인 샘플이 없으면 **`null`이다 — 0이 아니다.**
- `SessionStatusPayload`의 `Eq` derive를 뺐다. dBFS가 `f64`이므로 `Eq`를 만족할 수 없다.
  `PartialEq`는 그대로다.
- dBFS 환산도 임계값도 여기 없다. 값은 `crate::audio::level`이 만든 것을 옮기기만 한다.

## 4. `src-tauri/src/commands/mod.rs` — 실어 보내는 자리

- `Recorder::status()`가 진행 중인 캡처의 `capture.level()`을 payload에 싣는다.
  진행 중인 녹음이 없으면 `None`을 싣는다.
- `InputLevelPayload`를 `commands`에서 re-export.

## 5. `src-tauri/tests/recording_lifecycle.rs` — 마이크 없이 고정한 것

이미 있던 가짜 `SampleSource`(`ControlledMicrophone`)와 `TestClock`을 그대로 쓴다.

| 검사 | 무엇을 못박는가 |
| --- | --- |
| `the_status_answer_tells_a_low_input_level_apart_from_a_usable_one` | 평균 RMS 256(= -42.1 dBFS · 2026-09-07 수준)은 `low`, 1685(= -25.8 dBFS · 9/4 수준)는 `usable`로 **서로 다르게** 보고된다 |
| `the_input_level_does_not_move_while_the_recording_is_paused` | 멈춰 있는 동안 들어온 진폭 31000이 평균에도 **피크에도** 들어가지 않는다. 확정된 WAV의 샘플과 레벨이 같은 말을 한다 |
| `there_is_no_input_level_before_a_recording_and_none_again_after_it` | 녹음 전 · 시작했지만 아직 샘플이 없을 때 · 정지한 뒤 모두 `null`이다 |
| `the_status_that_reaches_the_screen_carries_numbers_and_a_sentence_but_no_audio` | 직렬화된 payload의 키가 정확히 네 개(`state` · `elapsedMs` · `elapsedLabel` · `level`)이고 `level`의 키도 정확히 네 개이며, **JSON 어디에도 배열이 없고 샘플 값(1685)이 나타나지 않는다** |

일시정지 검사는 **기다린 시간에 의존하지 않는다**: 표시와 샘플이 같은 통로를 지나므로,
재개 후 흘려보낸 디지털 무음이 값에 반영된 것을 본 시점에는 멈춰 있는 동안의 큰 소리도
이미 그 통로를 지나간 뒤다.

## 하지 않은 것

- 마이크 게인 조정 · 오디오 정규화 (ADR-0003 §16.5) — 그런 함수가 생기지 않았다.
- `src/ipc/types.ts`의 `SessionStatus`에 필드를 더하는 일 — **TASK-082의 범위다.**
  이 Task는 backend가 그 값을 살려 내보내는 데까지만 한다.
- 화면 · 컴포넌트 · UI 변경.
