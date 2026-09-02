# TASK-011 — Acceptance Criteria와 판정 수단의 대응

Worker의 주장이 아니라 **어디를 보면 판정되는지**를 적는다.

| AC | 판정 수단 | 위치 |
| --- | --- | --- |
| AC1 `npm run build` | Gate (build) | `.loop/evidence/TASK-011/self-check.log` — PASS exit=0 |
| AC2 `npm run lint` (clippy `-D warnings`) | Gate (lint) | 같은 파일 — PASS exit=0 |
| AC3 `npm run test` · 기존 테스트 유지 | Gate (test) | 같은 파일 — vitest 70 passed / cargo lib 53 + 통합 테스트 6개 파일 전부 ok |
| AC4 순수 로직 자동 테스트 · 하드웨어 비의존 | 아래 §1 | `src-tauri/src/audio/devices.rs` · `src-tauri/tests/input_devices.rs` |
| AC5 기존 command 경계 · 기존 Failure 계약 | 아래 §2 | `src-tauri/src/commands/` · `src/ipc/` |
| AC6 ADR의 잠정 선택 경로 · 해석된 버전 기록 · PROVISIONAL 유지 | 아래 §3 | `docs/ADR-0003-recording-engine.md` · `crate-resolution.txt` |

---

## 1. AC4 — 하드웨어 없이 판정되는가

경계는 하나다.

```text
InputDeviceSource (trait)          src-tauri/src/audio/devices.rs
  └ SystemInputDevices             src-tauri/src/audio/system_devices.rs   ← cpal을 아는 유일한 파일
  └ 테스트가 넣는 구현              src-tauri/tests/input_devices.rs        (FixedDevices · UnavailableDevices)
```

`AudioDevices::with_source(...)`가 그 주입 지점이다. 앱만 `AudioDevices::system()`을 쓴다.

요구된 성질별 테스트:

| 성질 | 테스트 |
| --- | --- |
| 빈 목록이 정상 상태 | `an_empty_list_is_a_normal_answer_not_a_failure` · `a_machine_without_any_input_device_gets_an_empty_list_not_a_failure` |
| 이름이 중복된 장치 | `devices_with_the_same_name_are_told_apart` (키도 표시 이름도 갈린다) |
| 이름이 비어 있는 장치 | `a_device_without_a_name_still_gets_a_label_and_a_key` · `a_device_without_a_name_is_still_listed_with_something_to_choose` |
| 안정적인 선택 키 | `the_same_observation_always_produces_the_same_keys` · `a_key_belongs_to_the_name_it_was_made_from` · `asking_twice_gives_the_same_keys_back` |
| 기본 장치 표시 | `the_default_device_is_marked_and_shown_first` · `a_list_without_a_default_device_is_still_a_list` |
| 표시용 목록 정렬 | `the_display_order_does_not_depend_on_the_enumeration_order` |
| 공백만 다른 이름 | `surrounding_whitespace_does_not_create_a_second_device_name` |

**장치가 하나 이상 존재해야만 통과하는 단언은 없다.** 모든 테스트의 입력은 테스트가 만든
값이며, `cpal`을 부르는 코드(`system_devices.rs`)는 어떤 테스트도 실행하지 않는다.
마이크 권한을 요구하는 경로도 없다.

## 2. AC5 — 기존 경계를 그대로 쓰는가

| 요구 | 실제 |
| --- | --- |
| 기존 Tauri command 경계 | `src-tauri/src/commands/mod.rs`에 `list_input_devices` 하나를 더했다. 등록도 기존 방식대로 `lib.rs`의 `generate_handler![...]` 안이다 |
| 프론트엔드 경로 | `src/ipc/commands.ts`의 `listInputDevices()`. `invoke`를 부르는 파일은 여전히 이 하나뿐이며 `tests/ipc-boundary.test.ts`가 그것을 검사한다 |
| 실패 표현 | `domain::Failure` 하나뿐이다. 병렬 오류 타입을 만들지 않았다 — 경계 trait의 오류 타입 자체가 `Failure`다 |
| 새 `FailureKind` | `AudioDevice` 하나. `as_str()`이 `"audioDevice"`이고 `src/ipc/failure.ts`의 union에 같은 문자열이 있다. 이 1:1 대응은 `tests/ipc-boundary.test.ts`의 "Rust의 실패 종류가 frontend 타입에 전부 있다"가 자동으로 검사한다 |
| 앱 데이터 경로 | **새로 결정하지 않았다.** 이 Task는 파일을 만들지 않으므로 `AppDataDirectory`를 부르는 코드가 없다. 경로 결정은 여전히 `platform/app_data_dir.rs` 한 곳뿐이다 |
| 등록 command 목록 검사 | `tests/ipc-boundary.test.ts`의 허용 목록에 `list_input_devices`를 더했다. 검사 자체는 그대로 **정확히 같은 집합**을 요구한다(부분집합으로 약화하지 않았다) |

## 3. AC6 — ADR과의 일치

| 확인 | 결과 |
| --- | --- |
| ADR-0003의 잠정 선택 | 후보 B (Rust/native · `cpal`). 구현도 native 경로다 — 열거가 Rust에 있고 프론트엔드는 command로 목록을 받는다 |
| 기록된 버전이 매니페스트/락과 맞는가 | ADR §4.2에 `cpal` 0.18.2. `Cargo.toml`은 `cpal = "0.18"`, `Cargo.lock`은 `version = "0.18.2"` (`crate-resolution.txt`) |
| 상태 표기 | 문서 상단은 여전히 `Status: PROVISIONAL — pending human device validation`이다. §12의 8개 체크박스는 **하나도 채우지 않았다** |
| 승격하지 않은 것 | §5.2 갱신에 [A✓]로 올린 것은 "컴파일된다"와 "순수 로직이 테스트된다"뿐이다. 실제 열거 결과 · TCC 프롬프트 · 물리 마이크 대응은 [H]로 남겼다 |
