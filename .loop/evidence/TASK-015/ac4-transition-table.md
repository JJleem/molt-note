# AC4 — 상태 기계의 순수성과 전이표

Run: RUN-20260902T081554Z-TASK-015 · Task: TASK-015 · 2026-09-02

## 변경한 파일

```text
src-tauri/src/audio/session.rs        새 파일 — 상태 기계 + 단위 테스트 13개
src-tauri/src/audio/mod.rs            모듈 등록 · 재export · 모듈 설명 갱신
src-tauri/tests/recording_session.rs  새 파일 — 통합 테스트 4개 (구조 검사 포함)
```

캡처(`audio/capture.rs` · `audio/system_capture.rs`) · command · 프론트엔드는 건드리지 않았다.
commit하지 않았다.

## (1) 하드웨어 · 시계 · 파일시스템 의존이 없다

`src/audio/session.rs`의 `use`는 두 줄뿐이다.

```rust
use std::fmt;
use crate::domain::{format_duration_ms, Failure, FailureKind};
```

`cpal` · `hound` · `std::time` · `std::fs` · `std::thread` · `tauri`가 없다.
경과 시간은 전부 인자로 들어온다 — `start(at_ms)` · `pause(at_ms)` · `resume(at_ms)` ·
`stop(at_ms)` · `elapsed_ms(now_ms)` · `elapsed_label(now_ms)`.

이것을 **주장으로 두지 않고 테스트로 고정했다.** `tests/recording_session.rs`의
`the_state_machine_module_reaches_no_hardware_no_clock_and_no_filesystem`이
모듈 소스(`include_str!`)를 정규화해서 아래 문자열이 없음을 확인한다.
나중에 누가 모듈 안에서 시계를 직접 읽으면 그 순간 test Gate가 빨개진다.

```text
CPAL · HOUND · STD::TIME · INSTANT · SYSTEMTIME · NOW() ·
STD::FS · FS:: · PATHBUF · STD::THREAD · SYNCSENDER · TAURI
```

(`domain_invariants.rs`가 domain에 대해 하는 것과 같은 방식이다.)

## (2) 전이표 — 16칸 전부가 테스트로 판정된다

허용 5칸 · 거절 11칸.

```text
            start          pause          resume         stop
idle        → recording    거절            거절            거절
recording   거절            → paused       거절            → stopped
paused      거절            거절            → recording    → stopped
stopped     거절            거절            거절            거절
```

`every_cell_of_the_transition_table_is_covered_and_only_five_are_allowed`가
`SessionState::ALL` × `Action::ALL` 16칸을 전부 돌면서 각 칸에 대해 확인한다.

- 허용 칸: `Ok`이고 상태가 실제로 옮겨진다. (허용 칸이 정확히 5개임을 마지막에 단언한다 —
  나중에 거절 하나를 몰래 허용으로 바꾸면 이 수가 어긋나 실패한다.)
- 거절 칸: `Err(Failure)`이고 `kind == FailureKind::InvalidInput`이며 **상태가 그대로다.**

Task가 이름으로 지목한 네 가지 잘못된 전이는 각각 다음이 덮는다.

```text
녹음 중이 아닌데 pause     idle/paused/stopped + pause   (표 전체 + each_wrong_transition…)
진행 중이 아닌데 resume    idle/recording/stopped + resume
idle에서 stop              idle + stop                   (a_rejected_transition_is_a_failure_value…)
이미 녹음 중인데 start     recording/paused + start
```

거절이 **panic이 아니라 값**이라는 것은 `Result`의 `Err` 갈래를 읽는 것으로 판정된다.
`a_rejected_transition_is_a_failure_value_the_user_can_read`가 그 값의 내용까지 본다 —
사용자에게 보여줄 문장이 있고, `source_data_safe`이며, 재시도해도 같으므로 `retryable`이 아니고,
`detail`에 어느 상태에서 무엇이 거절됐는지 남는다(`"stop rejected in state=idle"`).
`each_wrong_transition_names_what_went_wrong`은 다섯 거절 문장이 서로 겹치지 않는지 본다.

또한 거절 뒤에도 session이 살아남는다는 것을 통합 테스트가 본다
(`a_wrong_transition_comes_back_as_a_failure_the_screen_can_show` — 거절 후 상태도 길이도
그대로이고, 이어서 resume·stop이 정상 동작한다). R-001의 방향이다.

## (3) pause 구간이 duration에서 빠진다

- `the_paused_span_is_not_counted_in_the_duration` — 3초 녹음 → 96초 일시정지 → 2초 녹음.
  벽시계로는 101초지만 길이는 5초다. 일시정지 중에는 `now_ms`를 아무리 키워도 길이가
  자라지 않는다는 것도 함께 본다.
- `many_pause_and_resume_cycles_keep_accumulating_only_the_recorded_spans` —
  10초 녹음 / 1000초 정지를 세 번 반복하고 마지막에 30초. 결과는 60초(`"1:00"`)다.
- `the_paused_span_stays_out_of_the_duration_however_long_it_lasts` (통합) —
  1초 녹음 후 한 시간 일시정지. 길이는 1초 그대로이고, 일시정지 상태에서 stop해도 1초다.

**구현을 바꾸면 이 테스트들이 실제로 실패하는지 직접 확인했다** — `mutation-check.md`.
`pause`가 구간을 닫지 않도록(= 일시정지 구간이 duration에 포함되도록) 고치자
test Gate가 exit 1로 떨어졌고, 세 테스트가 `99000 != 3000` · `2000 != 7000` ·
`30000 != 60000`으로 실패했다. 확인 후 되돌렸고 Gate는 다시 전부 PASS다.

## 사람이 읽는 길이 문자열

`elapsed_label` · `SessionSummary::duration_label`은 `crate::domain::format_duration_ms`를
그대로 부른다. 길이 포맷 규칙은 여전히 `src-tauri/src/domain/duration.rs` 한 곳에만 있고,
TypeScript에는 만들지 않았다 (`tests/screen-boundary.test.ts`가 계속 green이다 —
vitest 96 passed).
`the_human_readable_length_comes_from_the_one_place_that_rule_lives`가 두 값이 같은 함수에서
나온다는 것을 단언한다.

## 이 Task가 판정하지 않는 것

상태 기계는 아직 실제 캡처에 연결되어 있지 않다. pause/resume를 실제 장치에 붙이는 것,
command 노출, 화면 표시는 이 Task의 범위가 아니다 (Phase 2B의 다른 Task).
실제 마이크·권한·음질은 여전히 사람의 검증 대상이며 이 테스트가 대신 판정하지 않는다.
