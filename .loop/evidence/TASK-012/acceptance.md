# TASK-012 — Acceptance Criteria 대응

## AC1 · AC2 · AC3 — build · lint · test

`.loop/evidence/TASK-012/gates.md` 참조. 세 Gate 모두 exit 0.
기존 테스트는 깨지지 않았다 — 실패 0, skip 0, ignored 0.

## AC4 — 하드웨어 없이 파일 확정 경로 전체가 테스트된다

실제 장치에서 샘플을 받는 부분은 `audio::capture::SampleSource` trait 뒤에 있고,
실제 구현은 `audio::system_capture::SystemSampleSource` 하나다. 테스트는 그 자리에
가짜 샘플 소스(`FakeMicrophone`, `src-tauri/tests/capture.rs:58`)를 넣는다.
그 경계 바깥은 제품 코드가 그대로 실행된다 — 경로 결정 · WAV writer · 파일 확정 · 크기 읽기.

출력은 전부 `std::env::temp_dir()` 아래의 고유 디렉터리에 쓰고 Drop에서 지운다.
실제 앱 데이터 위치는 건드리지 않는다.

| AC4가 요구한 것 | 판정하는 테스트 |
|---|---|
| 출력 경로 결정 — 같은 입력이면 결정론적 | `audio::capture::tests::the_same_directory_and_stem_always_give_the_same_path` |
| 출력 경로 결정 — **기존 파일을 덮어쓰지 않음** | `audio::capture::tests::a_path_is_never_one_that_already_holds_a_file`<br>`audio::capture::tests::the_path_keeps_stepping_aside_while_names_are_taken`<br>`a_second_capture_does_not_overwrite_the_first` (tests/capture.rs) |
| 포맷 기술 문자열 (샘플레이트 · 채널 수 · 비트 심도 · 컨테이너) | `audio::capture::tests::the_format_sentence_carries_all_four_facts`<br>`audio::capture::tests::the_format_sentence_reports_the_channel_count_it_was_given` |
| 보고 값 조립 (장치명 · 경로 · 포맷 · 크기) | `a_capture_that_stops_reports_the_device_the_path_the_format_and_the_size` (tests/capture.rs) |
| 임시 디렉터리에 실제로 쓴 파일의 크기 읽기 | `audio::capture::tests::the_reported_size_is_read_from_the_file_that_was_written`<br>`audio::capture::tests::a_wav_file_that_is_finalized_can_report_a_size_larger_than_its_header` |
| 가짜 샘플 소스로 파일 확정 경로 **전체** | `tests/capture.rs`의 9개 테스트 전부 (start → stop → WAV finalize → size) |

`a_capture_that_stops_reports_...`는 보고된 크기를 파일시스템에서 다시 읽어 대조한다
(`tests/capture.rs:175`) — 계산한 값이 아니라 실제로 쓰인 파일의 크기다.

## AC5 — 보고 값의 필드와 실패 경로

보고 타입 `CaptureReport` (`src-tauri/src/audio/capture.rs:107`) →
wire 타입 `CaptureReportPayload` (`src-tauri/src/commands/payload.rs:140`) →
frontend `CaptureReport` (`src/ipc/types.ts`).

네 가지가 모두 있다: `device_label` · `output_path` · `format`(+ `sample_rate_hz` ·
`channels` · `bits_per_sample` · `container`) · `byte_size`.

실패는 전부 `domain::Failure`다. `source_data_safe` · `retryable`을 기존 계약대로 채운다:

| 실패 | kind | source_data_safe | retryable | 테스트 |
|---|---|---|---|---|
| 장치를 열지 못함 | `AudioDevice` | true (아무것도 안 씀) | true | `a_device_that_cannot_be_opened_reaches_the_user_as_the_shared_failure_contract` |
| 캡처 중단 | `AudioDevice` | true (파일은 확정됨) | true | `a_capture_that_was_cut_short_says_so_and_keeps_what_was_recorded` |
| 파일 쓰기/확정 실패 | `Storage` | **false** (`with_source_data_at_risk`) | true | `audio::capture::tests::a_file_that_cannot_be_created_becomes_a_failure_instead_of_a_panic` |
| 녹음 자리를 만들지 못함 | `Storage` | true | true | `a_place_that_cannot_hold_recordings_becomes_a_failure_the_user_can_read` |
| 녹음 중이 아닌데 정지 | `InvalidInput` | true | false | `stopping_without_starting_is_a_failure_not_a_panic` |
| 이미 녹음 중인데 시작 | `InvalidInput` | true | false | `starting_twice_does_not_throw_away_the_recording_already_running` |

끊긴 캡처는 실패로 보고하면서 **그때까지 녹음된 파일의 경로를 문장에 담는다**
(`capture.rs:187`) — 실패했다는 이유로 이미 녹음된 것을 숨기지 않는다.

panic 경로: 캡처·파일 확정 경로(`audio/capture.rs` · `audio/system_capture.rs` ·
`commands/mod.rs`의 `Capture`)에 `unwrap` · `expect` · `panic!`이 없다.
`#[cfg(test)]` 블록의 `expect`는 테스트 전제 조건이며 제품 경로가 아니다.
쓰기 스레드가 죽는 경우도 `writer.join()`의 `Err`를 실패 값으로 옮긴다
(`capture.rs:175`) — 앱이 죽지 않는다. mutex poisoning도 같은 방식이다.

## AC6 — 범위

만들지 않은 것: pause/resume · 재생 · 재시작 영속성 · Recording DB 행 생성 · 완성된 UX.
`tests/ipc-boundary.test.ts`의 `아직 만들지 않은 기능의 command가 등록되어 있지 않다`가
`(start|stop|pause|resume)_recording` · `(pause|resume)_capture` · `play` · `transcri` 등의
이름을 정규식으로 막고, `허용된 목록과 정확히 같은 command만 등록되어 있다`가 부분집합이
아니라 **정확히 같은 집합**을 요구한다. 등록된 command는 9개이며 이번에 늘어난 것은
`start_capture` · `stop_capture` 둘뿐이다.

`stop_capture`는 어떤 DB 행도 만들지 않는다 — `Capture`는 `Storage`를 알지 않는다
(`commands/mod.rs:205`, 필드에 저장소가 없다).

앱 데이터 경로를 결정하는 자리는 `src-tauri/src/platform/app_data_dir.rs` 한 곳이다.
`AppDataDirectory::recordings_dir()`가 유일한 파생 지점이고, `audio::capture::start`는
디렉터리를 **인자로 받는다** — 스스로 경로를 만들지 않는다 (`capture.rs:214`).
`the_output_file_lives_under_the_app_data_directory` (tests/capture.rs:185)가
출력이 `app_data_dir.recordings_dir()` 바로 아래에 놓이는지 확인한다.
사용자 데이터를 지우는 경로는 만들지 않았다 — `ensure_recordings_dir`은 기존 파일을 남긴다
(`ensuring_the_recordings_directory_keeps_what_is_already_there`).

## 확인하지 못한 것 (UNVERIFIED)

- 실제 장치가 어떤 샘플레이트·채널을 주는지, 그래서 만들어진 파일이 whisper 입력 요구
  (16kHz mono 16-bit)와 맞는지. 리샘플링·다운믹스는 이 코드에 없다.
- macOS TCC 마이크 권한 프롬프트가 실제로 뜨는지.
- 만들어진 WAV이 실제로 재생 가능한 소리인지.

이 세 가지는 `src-tauri/src/audio/capture.rs`와 `system_capture.rs`의 모듈 주석에
UNVERIFIED로 적혀 있고, 판정은 사람의 장치 검증 몫이다 (ADR-0003 §12).
**확인된 것**만 적었다: 이 코드는 `hound`로 16-bit PCM WAV(RIFF)를 쓰고,
샘플레이트·채널 수는 열린 장치가 알려준 값을 그대로 쓴다.
