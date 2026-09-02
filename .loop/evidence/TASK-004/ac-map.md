# TASK-004 — Acceptance Criterion별 판정 수단

각 AC가 **어떤 검사로 판정되는지** 대응시킨 표다. Worker의 성공 주장이 아니라,
제3자가 다시 실행해 확인할 수 있는 지점을 가리킨다.

```text
재실행: node tools/loop-runtime/loopctl.mjs self-check lint test
개별:   cargo test --manifest-path src-tauri/Cargo.toml <테스트 이름>
```

---

## AC1 — `npm run lint` 통과 (gate)

`.loop/evidence/TASK-004/gate-lint.log` — exit 0.
eslint와 `cargo clippy --all-targets -- -D warnings` 둘 다 통과한다.
`--all-targets`이므로 새로 추가한 테스트 파일도 clippy 검사 대상이다.

## AC2 — `npm run test` 통과 (gate)

`.loop/evidence/TASK-004/gate-test.log` — exit 0.
vitest 22개 + cargo test 65개(기존 41 + 이번 24) 전부 통과.

## AC3 — 네 연산을 모두 덮고, 오디오 파일·하드웨어 없이 실행된다

테스트 파일: `src-tauri/tests/recording_repository.rs`

| 연산 | 테스트 | 무엇을 assertion하는가 |
| --- | --- | --- |
| **생성** | `a_created_recording_is_read_back_by_its_id_with_a_human_readable_length` | `store::insert_recording` 후 다시 읽은 레코드가 저장한 구조체와 완전히 같은지(`assert_eq!`) 본다. |
| 생성(중복 거부) | `creating_the_same_id_twice_fails_and_the_stored_record_survives` | 같은 id로 다시 만들면 UNIQUE 위반으로 실패하고(오류 원문에 `UNIQUE` 포함), 원본이 그대로 남고 행 수가 1개 그대로다. |
| **목록 조회** | `the_list_returns_every_recording_newest_first_with_its_length_label` | 3건을 저장하고 `list_recordings`가 `["rec-new","rec-old","rec-long"]` 순(최근순)으로, 각 항목의 `duration_label`이 `["52:31","38:22","1:01:01"]`로 오는지 본다. |
| 목록 조회(빈 상태) | `the_list_is_empty_before_anything_is_recorded` | 아직 아무것도 없을 때 빈 목록이며 오류가 아니다. |
| **단건 조회** | `a_created_recording_is_read_back_by_its_id_with_a_human_readable_length` | `load_recording_view(id)`가 레코드 + `duration_label`을 돌려준다. |
| 단건 조회(**없는 id**) | `loading_an_id_that_was_never_stored_returns_none_instead_of_failing` | 다른 Recording이 있는 저장소와 빈 저장소 **두 경우 모두** 없는 id 조회가 `Ok(None)`이다 — 실패가 아니라 '값 없음'이다. |
| **삭제** | `deleting_a_recording_removes_only_that_one` | 두 건 중 하나를 지우면 반환값이 `true`, 지운 것은 조회되지 않고, 다른 하나는 목록에 그대로 남는다. |
| 삭제(없는 id) | `deleting_an_id_that_does_not_exist_changes_nothing` | 반환값이 `false`이고 다른 Recording을 대신 지우지 않는다. |
| 삭제(딸린 데이터) | `a_recording_that_has_a_transcript_is_not_deleted_silently` | Transcript가 있는 Recording 삭제는 FK로 거부되고(오류 원문에 `FOREIGN KEY` 포함), Recording과 Transcript 둘 다 그대로 남는다 (INV-2). |

**하드웨어·오디오 파일 비의존 근거:**

| 테스트 | 무엇을 확인하는가 |
| --- | --- |
| `every_operation_works_without_an_audio_file_or_a_microphone` | `audio_path`가 **존재하지 않는 경로**이고 `microphone`이 `None`인 상태에서 생성·단건·목록·삭제 네 연산이 모두 성공하며, 그 경로에 파일이 만들어지지도 지워지지도 않음을 확인한다. |
| `deleting_a_recording_does_not_remove_the_file_at_its_audio_path` | `audio_path`에 자리표시 파일(실제 오디오가 아니라 바이트 9개)을 두고 삭제한 뒤에도 파일과 내용이 그대로임을 확인한다 (INV-1 · INV-4). |
| `tests_never_write_outside_the_system_temp_directory` | 테스트 DB 경로가 `std::env::temp_dir()` 아래임을 확인한다 — 실제 사용자 앱 데이터 디렉터리를 만들거나 오염시키지 않는다. |

DB는 매 테스트마다 `TempDir`(시스템 임시 디렉터리 아래의 고유 디렉터리, Drop 시 삭제)의
`molt-note.db` 하나뿐이다. 마이크·오디오 코덱·Tauri 런타임·네트워크는 쓰지 않는다.

## AC4 — duration 포맷 단위 테스트 (52:31 + 경계 사례)

대상: `src-tauri/src/domain/duration.rs`의 `format_duration_ms`
(1시간 미만 `m:ss`, 1시간 이상 `h:mm:ss`, 초 미만은 버림).

| 테스트 | 입력 → 기대값 |
| --- | --- |
| `a_fifty_two_minute_recording_reads_as_52_31` | `3_151_000ms` → `"52:31"` (§5 A 화면의 예시 그대로) |
| `a_zero_length_recording_reads_as_0_00` | `0ms` → `"0:00"` (**0초 경계**) |
| `less_than_a_second_is_truncated_not_rounded_up` | `1` → `"0:00"` · `999` → `"0:00"` · `1_000` → `"0:01"` · `1_999` → `"0:01"` |
| `seconds_are_always_two_digits_below_a_minute` | `5_000` → `"0:05"` · `59_000` → `"0:59"` |
| `the_minute_boundary_carries_over` | `59_999` → `"0:59"` · `60_000` → `"1:00"` · `61_000` → `"1:01"` (**분 경계**) |
| `the_hour_boundary_switches_to_h_mm_ss` | `3_599_000` → `"59:59"` · `3_599_999` → `"59:59"` · `3_600_000` → `"1:00:00"` · `3_601_000` → `"1:00:01"` (**1시간 경계**) |
| `recordings_longer_than_an_hour_keep_two_digit_minutes_and_seconds` | `3_661_000` → `"1:01:01"` · `3_909_000` → `"1:05:09"` · `36_000_000` → `"10:00:00"` · `360_001_000` → `"100:00:01"` (**1시간 이상**) |
| `a_negative_length_is_treated_as_zero_rather_than_guessed` | `-1` → `"0:00"` · `-3_151_000` → `"0:00"` |
| `an_extreme_value_formats_without_overflowing` | `i64::MAX` → 패닉 없이 `h:mm:ss` 형태(콜론 2개) |
| `the_same_input_always_produces_the_same_text` | 같은 입력이 항상 같은 출력 — 시계·로케일을 보지 않는 순수 함수 |

## AC5 — 연결을 닫고 다시 연 뒤에도 그대로 조회된다

`recordings_survive_closing_and_reopening_the_database`
(`src-tauri/tests/recording_repository.rs`)

```text
db::open(path)
  → 3건 생성 (rec-a · rec-b · rec-c)
  → rec-c를 명시적으로 삭제
  → connection.close()   (실제로 닫고, 닫히지 않으면 테스트 실패)
db::open(같은 path)      ← 새 연결
  → list_recordings()        == ["rec-a", "rec-b"]       (최근순)
  → 각 항목의 duration_label == ["52:31", "38:22"]
  → load_recording_view(rec-a).recording == 저장 당시 구조체 그대로
  → load_recording_view(rec-c)           == None          (지운 것은 돌아오지 않는다)
```

`db::tests::rows_survive_closing_and_reopening_the_same_path`와
`domain_model.rs::domain_rows_survive_closing_and_reopening_the_database`가
같은 성질을 다른 각도(스키마 · §7 레코드 전반)에서 이미 검증하고 있다.

---

## UI 중복 구현 방지

조회 API가 돌려주는 `RecordingView`가 `duration_label`을 함께 담는다
(`src-tauri/src/domain/mod.rs`). 프론트엔드의 `RecordingListItem`
(`src/screens/RecordingsScreen.tsx`)은 이미 `durationLabel: string`을 **받는** 형태이므로
TypeScript 쪽에 초 → `mm:ss` 계산이 들어갈 자리가 없다. 이 Task는 프론트엔드를
수정하지 않았고, 포맷 규칙은 Rust `domain::duration` 한 곳에만 있다.

## INV-4 — 자동 삭제 경로 부재

`nothing_deletes_a_recording_on_its_own` (`src-tauri/tests/recording_repository.rs`)이
소스 자체를 검사한다:

- `src/lib.rs`(앱 시작 경로)에 `delete_recording` 문자열이 없다.
- `src/db/store.rs`의 `DELETE FROM` 문장이 정확히 하나이고,
  그 문장이 `DELETE FROM recordings WHERE id = ?1` — 호출자가 준 id 하나만 지운다.

즉 오래된 녹음을 스스로 정리하거나, 조건으로 여러 건을 지우는 경로가 없다.
`delete_recording`은 레코드만 지우며 original audio 파일은 건드리지 않는다.

## 범위 밖으로 두고 만들지 않은 것

전사 · AI · Notion 관련 동작, Tauri command 노출, UI 연결, 자동 삭제/정리 정책.
Transcript·AI Note가 딸린 Recording의 삭제 정책은 **결정하지 않고** 참조 무결성이
막도록 두었다 — 전사가 실제로 존재하는 Phase에서 정할 문제다. git commit도 하지 않았다.
