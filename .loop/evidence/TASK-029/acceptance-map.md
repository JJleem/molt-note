# TASK-029 — Acceptance Criteria가 무엇으로 판정되는가

각 AC를 **재실행할 수 있는 검사** 하나 이상에 대응시킨다. 아래 테스트는 전부
`npm run test`(test Gate) 안에서 돌며, 결과는 `gates.md`에 있다.

## AC1 · AC2 · AC3 — build · lint · test

`gates.md` 참조. 셋 다 exit 0.

## AC4 — `automatic_transcription`이 `automatic_processing`과 별개의 값이고 기본값이 OFF다

| 무엇을 판정하는가 | 어디에서 |
| --- | --- |
| 저장된 값이 없을 때 자동 전사가 OFF다 | `src-tauri/tests/settings_repository.rs::defaults_are_returned_when_nothing_has_been_saved` |
| 두 토글의 **네 조합**이 각각 그대로 저장되고 그대로 복원된다 | `settings_repository.rs::the_two_automatic_toggles_are_stored_and_restored_independently` |
| 자동 전사를 켜는 것이 다른 설정 값을 건드리지 않는다 | `settings_repository.rs::turning_automatic_transcription_on_does_not_touch_the_other_settings` |
| command 표면(`update_settings` → `get_settings`)에서도 두 값이 섞이지 않는다 | `src-tauri/tests/command_boundary.rs::the_two_automatic_toggles_do_not_share_one_value` |
| 기본 설정 응답의 자동 전사가 `false`다 | `command_boundary.rs::an_empty_store_answers_with_an_empty_list_and_the_default_settings` |
| 후처리 토글을 켜도 정지 뒤에 전사가 걸리지 않는다 (의미가 겹치지 않는다) | `src-tauri/tests/automatic_transcription.rs::automatic_processing_is_a_different_toggle_and_does_not_start_a_transcription` |
| 화면 쪽에서도 두 토글이 서로를 켜지 않는다 | `src/screens/settingsView.test.ts` — `두 자동 토글은 서로를 켜지 않는다`, `읽은 값을 그대로 돌려보낼 수 있다`(네 조합) |

기본값 정책이 있는 자리: `src-tauri/src/domain/settings.rs`의 `Settings::DEFAULT`
(`automatic_transcription: false`). **스키마에는 DEFAULT 절이 없다** — 아래 AC5 참조.

## AC5 — 새 migration이 목록 끝에 붙었고, 이미 적용된 migration은 그대로이며, 저장된 값이 보존된다

| 무엇을 판정하는가 | 어디에서 |
| --- | --- |
| 선언된 migration의 version·name 목록이 1..5로 고정돼 있다 (새 것은 끝에 붙는다) | `src-tauri/src/db/migrations.rs::tests::released_migrations_keep_their_version_and_name` |
| 나중에 생긴 열이 이미 적용된 `create_settings`(version 3) 안에 들어가 있지 않다 | `migrations.rs::tests::a_new_settings_column_is_added_by_a_new_migration_instead_of_editing_an_old_one` |
| 어떤 migration도 데이터를 지우는 문장을 갖지 않는다 | `migrations.rs::tests::no_migration_destroys_existing_data` (기존 테스트) |
| **version 4까지 적용된 DB의 값이 올린 뒤에도 그대로다** | `src-tauri/tests/settings_repository.rs::a_database_written_before_the_transcription_settings_existed_keeps_its_values` |
| 올린 뒤 그 행에 새 값도 저장된다 (반쪽 스키마가 아니다) | 같은 테스트의 후반부 |
| version 3까지 적용된 DB도 여전히 그대로다 | `settings_repository.rs::a_database_written_before_the_default_microphone_existed_keeps_its_values` (기존 테스트) |

추가된 migration (`src-tauri/src/db/migrations.rs`, 목록의 마지막):

```sql
-- version 5, name = "add_transcription_settings"
ALTER TABLE settings ADD COLUMN automatic_transcription INTEGER
    CHECK (automatic_transcription IN (0, 1));
ALTER TABLE settings ADD COLUMN transcription_model TEXT;
```

`NOT NULL`도 `DEFAULT`도 두지 않았다. SQLite에서 이미 행이 있는 테이블에 NOT NULL 열을
더하려면 스키마에 DEFAULT를 적어야 하는데, **기본값 정책은 스키마가 아니라
`Settings::DEFAULT`가 갖는다**는 것이 이 테이블의 기존 규약(version 3 주석)이기 때문이다.
그래서 NULL을 허용하고, NULL은 "아직 저장한 적 없음"으로 읽어 기본값을 돌려준다
(`src-tauri/src/db/settings.rs::load`). 그 동작이 위의 migration 보존 테스트에서 판정된다.

## AC6 — 자동 전사가 ON일 때만 Stop 후 전사가 시작되고, 수동 전사는 설정과 무관하다

전부 `src-tauri/tests/automatic_transcription.rs`. 정지 경로는 제품 코드 그대로 지나고
(파일이 실제로 쓰이고 확인되고 레코드가 저장된다), 대체 구현은 마이크 · 시계 · 전사 엔진뿐이다.

| 테스트 | 판정 |
| --- | --- |
| `a_stop_starts_a_transcription_only_when_automatic_transcription_is_on` | ON이면 정지 뒤 전사가 걸리고 그 녹음의 Transcript가 하나 추가된다 (밀리초 단위까지 확인) |
| `a_stop_starts_nothing_when_automatic_transcription_is_off` | OFF면 상태가 `idle`에 머물고 Transcript도 생기지 않는다. 녹음 자체는 저장돼 있다 |
| `the_default_settings_do_not_start_a_transcription_after_a_stop` | 아무것도 저장한 적 없는 앱에서도 걸리지 않는다 (기본값 OFF) |
| `automatic_processing_is_a_different_toggle_and_does_not_start_a_transcription` | 후처리 토글은 자동 전사를 켜지 않는다 |
| `a_manual_transcription_works_while_automatic_transcription_is_off` | OFF인 채로 수동 전사가 끝까지 성공하고, 그 때문에 설정이 켜지지도 않는다 |
| `a_failed_stop_does_not_start_a_transcription` | 정지가 실패하면(빈 녹음) 전사도 걸리지 않는다 — 저장되지 않은 녹음을 전사하지 않는다 |

구현 자리: `src-tauri/src/commands/mod.rs::start_automatic_transcription`을
`finish_recording`이 **레코드 저장 뒤에** 부른다. 이 함수는 실패를 돌려주지 않는다 —
전사를 걸지 못했다는 이유로 이미 성공한 정지(R-002)를 실패로 바꾸지 않기 위해서다.

## AC7 — 모델이 없는 상태를 제품 상태로 알리고 설정 값을 임의로 바꾸지 않는다 · secret 열 없음 (INV-7)

| 무엇을 판정하는가 | 어디에서 |
| --- | --- |
| 모델이 없는 채로 자동 전사가 켜져 있으면 §13의 `transcriptionModelMissing`으로 실패하고, **토글은 켜진 채로 남고 모델도 대신 골라 주지 않는다** | `automatic_transcription.rs::a_missing_model_is_reported_as_a_failure_and_the_toggle_is_left_as_the_user_set_it` |
| 저장소가 없는 모델을 지우거나 다른 값으로 바꾸지 않는다 | `settings_repository.rs::a_model_file_that_is_no_longer_there_is_still_the_saved_choice` |
| command 경계도 마찬가지다 (앞뒤 공백만 다듬는다) | `command_boundary.rs::a_model_that_is_not_there_is_stored_as_chosen_not_replaced` |
| 모델이 없다는 사실과 **푸는 방법**이 화면 상태로 나온다 | `src/screens/settingsView.test.ts` — `모델이 없으면 지금 전사할 수 없다는 사실과 푸는 방법을 함께 보여준다` |
| 자동 전사가 켜져 있으면 "값은 그대로 남는다"는 사실도 함께 말하고, 값 자체는 바뀌지 않는다 | 같은 파일 — `자동 전사가 켜져 있으면 그 값이 그대로 남는다는 사실도 말한다`, `상태를 읽는 것이 폼 값을 바꾸지 않는다` |
| 모델을 골랐을 때 "있다"고 단정하지 않는다 (파일 존재는 화면이 알 수 없다) | 같은 파일 — `모델을 골랐으면 할 말이 없다`, `TranscriptionModel` 타입 주석 |
| **설정 테이블 열 목록이 정확히 여섯이며 secret 성격의 이름이 없다** | `settings_repository.rs::the_settings_schema_has_no_secret_columns` |
| 설정 API 소스에 secret 이름이 없다 | `settings_repository.rs::the_settings_api_does_not_accept_or_store_secrets` |

`the_settings_schema_has_no_secret_columns`가 요구하는 열 목록 (이 순서 그대로):

```
id · recordings_directory · automatic_processing · default_microphone
   · automatic_transcription · transcription_model
```

`transcription_model`은 **파일이 어디 있는지**이며 API key · token · password가 아니다.
검사는 `api_key · apikey · token · password · secret · credential` 여섯 단어를 열 이름과
설정 API 소스 코드에서 찾으며, 새 열 어느 것도 걸리지 않는다.

화면이 모델 파일을 찾아보지 않는다는 것도 그대로다 — 파일을 여는 자리는
`src-tauri/src/transcription/model.rs` 하나뿐이며, 이 Task는 그 자리를 늘리지 않았다.
