# TASK-022 — 문서와 구현의 대조

문서에 적은 정책이 P2(TASK-016) · P3(TASK-017) · P5(TASK-019)의 실제 구현과 어긋나지 않는지
소스를 직접 읽어 대조한 결과다. **각 줄은 읽은 파일과 그 안의 이름을 가리킨다.**

```text
P2 = TASK-016  backend가 소유하는 녹음 session (Record · Pause · Resume · Stop)
P3 = TASK-017  Stop = 파일 확정 + Recording 영속화, 순서와 보상 정책
P4 = TASK-018  Settings의 default microphone
P5 = TASK-019  macOS 마이크 권한 경계
P6 = TASK-020  Recording 화면          P7 = TASK-021  재생
(각 Task의 proposal 번호는 `.loop/tasks/TASK-0NN.yaml` 머리말 주석에 있다. 전부 status: DONE이다)
```

## 1. session 소유권 (ADR-0004 §1~§9 · R-001)

| 문서가 적은 것 | 실제 구현 |
| --- | --- |
| 진행 중인 session은 `crate::commands::Recorder` 하나가 소유하고 Tauri managed state가 들고 있다 | `src-tauri/src/commands/mod.rs` — `pub struct Recorder { source, clock, microphone, app_data_dir, active: Mutex<Option<ActiveRecording>> }` · `Recorder::open_for` |
| 상태 기계와 열려 있는 캡처가 **한 값으로 묶여서만** 존재한다 | 같은 파일 — `struct ActiveRecording { session: RecordingSession, capture: ActiveCapture }` |
| 화면은 핸들을 갖지 않고 **물어본다** | `src/screens/RecordingScreen.tsx` — `captureStatus` 되풀이 조회, session 핸들 없음. 상태 규칙은 `src/screens/recordingView.ts`(테스트: `recordingView.test.ts`) |
| 경과 시간 문자열을 Rust가 만든다 | `Recorder::status` → `SessionStatusPayload::new(state, elapsed_ms, elapsed_label)` |
| 한 번에 하나만 녹음한다 — 진행 중이면 새 시작을 거절한다 | `Recorder::start` 앞부분 — `if active.is_some() { return Err(...InvalidInput, "이미 녹음 중이다...") }` |
| pause는 장치도 파일도 닫지 않는다 | `src-tauri/src/audio/capture.rs` — `enum Packet { Samples, Paused, Resumed }` (표시가 샘플과 같은 통로로 흐른다) |

## 2. 파일 확정과 레코드 저장의 순서 · 보상 (ADR-0004 §10~§13 · R-002 · INV-4)

| 문서가 적은 것 | 실제 구현 |
| --- | --- |
| 순서는 **확정 → 확인 → 레코드**이고, 그 순서를 아는 자리는 하나다 | `commands::finish_recording` — `recorder.stop()?` (확정 + `finalized::verify`) → `storage.save_capture(...)` |
| `save_capture`는 파일을 다시 확인하지 않는다 | `Storage::save_capture`에 파일 접근이 없다 (`commands/mod.rs`) |
| `verify`는 저장소를 알지 않는다 | `src-tauri/src/audio/finalized.rs` — `fs`와 `hound`만 쓴다 |
| 확인은 **경로 존재 · 최소 크기 · 파일에서 다시 읽은 형식 · 프레임 ≠ 0** 네 가지다 | `finalized::verify` · `MIN_FINALIZED_BYTES = 45` · `hound::WavReader::open` · `reader.duration()` |
| audio는 있는데 레코드가 없는 상태 → **보상하지 않고 경로를 담은 실패를 보낸다** | `finish_recording`의 `.map_err(|f| keeping_file(not_listed(f), &output_path))` |
| 실패 문장의 순서: "파일로 저장됐지만 목록에 추가하지 못했다: …" → "녹음된 파일은 남아 있다: …" | `fn not_listed` · `fn keeping_file` (같은 경로를 두 번 적지 않는 분기 포함) |
| 이 실패는 `source_data_safe`를 내리지 않는다 | `not_listed`는 `message`만 바꾸고 나머지 필드를 그대로 둔다 (`..failure`) |
| 레코드는 있는데 audio가 없는 상태 → **감지만 한다** | `Storage::missing_audio` → `finalized::audio_is_present` (읽기만 한다) · `MissingAudioPayload` |
| 어떤 실패 경로도 audio를 지우지 않는다 | 제품 코드에 오디오 파일 삭제 경로가 없다. `Storage::delete_recording`은 레코드만 지운다. 판정: `src-tauri/tests/stop_persistence.rs` |

## 3. 권한 판정의 VERIFIED / UNVERIFIED (ADR-0004 §15 · ADR-0005)

| 문서가 적은 것 | 실제 구현 |
| --- | --- |
| 권한을 **장치보다 먼저** 묻고, `Denied`면 디렉터리도 파일도 만들지 않는다 | `Recorder::start` — `self.microphone.request()` 뒤에 `access_denied()` 반환. `ensure_recordings_dir()`와 `capture::start`는 그 **아래**에 있다 |
| macOS에서 상태를 안다고 주장하지 않는다 (`Undetermined`) | `src-tauri/src/platform/microphone.rs` — `impl MicrophonePermission for SystemMicrophonePermission { fn status(&self) -> ... if cfg!(target_os = "macos") { Undetermined } else { Granted } }` |
| 폴백은 두 조건이 모두 참일 때만 분류를 바꾼다 | 같은 파일 — `explain_open_failure`: `if access != Undetermined \|\| failure.kind != AudioDevice { return failure; }` |
| 원래 원인은 `detail`에 남는다 | 같은 함수 — `Failure::retryable(MicrophonePermission, UNDETERMINED_MESSAGE).with_detail(detail)` |
| 판정은 마이크 없이 이뤄진다 | `src-tauri/tests/microphone_permission.rs` · `microphone.rs`의 `mod tests` |

## 4. 형식은 절반만 코드가 정한다 (ADR-0004 §16 · ADR-0003 §15.3)

| 문서가 적은 것 | 실제 구현 |
| --- | --- |
| 컨테이너 WAV와 16-bit는 코드가 고정한다 | `audio/capture.rs` — `pub const CONTAINER: &str = "WAV"` · `pub const BITS_PER_SAMPLE: u16 = 16` |
| 샘플레이트와 채널 수는 **장치가 정한다** | `audio/system_capture.rs` — `let config = device.default_input_config()?;` → `CaptureFormat::pcm_16bit(config.sample_rate(), config.channels())` (바로 위 주석: "형식을 지어내지 않는다 … 리샘플링·다운믹스는 여기에 없다") |
| `f32` → `i16`은 비트 심도 변환이지 리샘플링이 아니다 | 같은 파일 — `forward_f32`가 `to_i16`으로 옮길 뿐 샘플 수를 바꾸지 않는다 |
| `verify`는 "말한 것과 만든 것이 같은가"를 본다 (16 kHz mono 강요가 아니다) | `finalized::verify` — `spec.channels != expected.channels \|\| spec.sample_rate != expected.sample_rate_hz \|\| ...` |
| 레코드에는 컨테이너 식별자 하나(`wav`)만 남는다 | `Storage::save_capture` — `audio_format: capture::EXTENSION.to_string()`. 샘플레이트·채널은 `CaptureReportPayload`(응답)에만 있고 `NewRecording`에는 없다 (`commands/payload.rs`) |

## 5. Human Review 항목의 출처 (ADR-0004 §17)

`phase-prompt/02-reliable-recording.md`의 **Verification Boundary › Human Review 항목**을 그대로
옮겼다 — 음질 · 장시간 안정성(사람이 한 번) · 녹음 중 UI 상태 · dev와 번들 `.app` 양쪽의 권한
프롬프트. 여기에 같은 절의 성공 기준("앱을 완전히 종료한 뒤 다시 실행해도 재생된다")을
5번 항목으로 더했다. 전부 `PENDING`이며 `PASS`로 적지 않았다.

장시간 녹음을 자동 검증으로 만들지 않는다는 선도 같은 문서의 문장을 인용해 §17.1에 적었다.

## 6. ADR-0003의 상태가 승격되지 않았음

| 확인한 것 | 결과 |
| --- | --- |
| 머리말의 `Status:` | `PROVISIONAL — pending human device validation` — **바뀌지 않았다** |
| §12의 8개 항목 `결과` 칸 | 전부 `DEFERRED` — **한 칸도 바뀌지 않았다** |
| §12 위의 상태 블록 | `Human Review: DEFERRED` · `Engine: PROVISIONAL` — 그대로 |
| §12.A (A-REC-001) | 그대로. §15.4가 그것을 "유효"로 다시 가리킨다 |
| 이 Task가 §15에 더한 `[A✓]` | 전부 소스 파일을 읽어 확인한 것이며, 그 사실을 §15 머리말에 명시했다 |
| §15가 새로 만든 `[U]` 표기 | `cpal`의 스트림 pause API(쓰지 않기로 해서 확인하지 않음) · 장치가 주는 샘플 형식 · TCC 판정 수단 — 확인하지 않은 것을 확인한 것으로 적지 않았다 |
