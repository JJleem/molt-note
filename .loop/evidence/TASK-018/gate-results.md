# Gate 실행 결과 (Worker self-check · 참고용)

Runtime이 Worker 종료 후 Gate를 독립적으로 다시 돌린다. 아래는 그 전에 로컬에서 실행한 결과다.

```text
$ node tools/loop-runtime/loopctl.mjs self-check build lint test

[build] npm run build     PASS  exit=0   0.9s
[lint]  npm run lint      PASS  exit=0   2.7s
[test]  npm run test      PASS  exit=0  10.2s

Self-check: all gates passed
```

원본 출력: `.loop-local/self-check/gates/{build,lint,test}/{stdout,stderr}.log`
(`.loop-local`은 Runtime 소유 디렉터리이므로 여기로 복사하지 않고 경로만 남긴다.)

## 각 Gate가 실제로 실행한 것

| Gate | command (`.loop/project.yaml`) |
| --- | --- |
| build | `npm run build` → `tsc && vite build` |
| lint | `npm run lint` → `eslint .` && `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` |
| test | `npm run test` → `vitest run` && `cargo test --manifest-path src-tauri/Cargo.toml` |

## test Gate — 이 Task와 관련된 결과

**vitest** (`.loop-local/self-check/gates/test/stdout.log:13`)

```text
Test Files  13 passed (13)
     Tests  120 passed (120)
```

13번째 파일이 이 Task가 추가한 `src/screens/defaultMicrophone.test.ts`다.
그 파일이 판정하는 것 — 세 가지 해석 결과가 서로 다른 값이라는 것, 없어진 장치가 첫 장치로
바뀌지 않는다는 것, 목록이 비어 있어도 `missing`과 `notChosen`이 갈린다는 것, 고른 값이
언제나 선택지 안에 있다는 것.

**cargo test** — `settings_repository` (10 passed)

```text
test defaults_are_returned_when_nothing_has_been_saved ... ok
test saved_values_survive_closing_and_reopening_the_database ... ok
test a_default_microphone_that_is_no_longer_plugged_in_is_still_the_saved_choice ... ok
test clearing_the_default_microphone_is_remembered_as_not_chosen ... ok
test a_database_written_before_the_default_microphone_existed_keeps_its_values ... ok
test the_settings_schema_has_no_secret_columns ... ok
test the_settings_api_does_not_accept_or_store_secrets ... ok
test reading_defaults_does_not_write_a_row ... ok
test a_toggle_turned_off_is_remembered_and_not_mistaken_for_an_unsaved_value ... ok
test saving_settings_does_not_disturb_other_stored_data ... ok

test result: ok. 10 passed; 0 failed
```

앞의 다섯 개가 **영속성**을 판정한다 — 저장한 키가 연결을 닫았다 다시 연 뒤에도 남고(재시작),
장치가 빠져 있어도 저장된 선택이 그대로이며, '고르지 않음'으로 되돌린 것도 기억되고,
default microphone이 없던 스키마(version 3)의 DB를 올려도 기존 값이 유지된다.

**cargo test** — `command_boundary`

```text
test a_default_microphone_that_no_longer_exists_is_stored_as_chosen_not_replaced ... ok
```

command 경계에서도 알아볼 수 없는 키가 다른 값으로 바뀌지 않는다는 것을 판정한다.

## 테스트를 약화시키지 않았다

이 Task가 고친 기존 테스트는 **새 필드가 생겨 구조체/객체 리터럴이 불완전해진 곳**과
**설정 테이블의 열 목록을 정확히 요구하는 검사** 둘뿐이다.

- `the_settings_schema_has_no_secret_columns`의 기대 열 목록에 `default_microphone`을 추가했다.
  검사의 성격(정확히 이 열들만 있어야 한다)은 그대로다 — 느슨하게 바꾸지 않았다.
- 삭제하거나 `#[ignore]`/`skip`을 붙인 테스트는 없다.
