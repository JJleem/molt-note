# TASK-035 — AI provider 설정의 로컬 영속화

## 무엇을 했는가

`settings` 테이블에 migration **version 6 (`add_ai_provider_settings`)** 을 목록 **끝에** 추가하고,
그 값이 domain → 저장소 → command payload → frontend 타입까지 한 줄로 이어지게 했다.

```text
ai_provider   TEXT   어떤 provider를 쓰는가   NULL = 아직 고르지 않았다 (정상)
ai_base_url   TEXT   어디에 연결하는가        NULL = 고른 적 없다 → 코드가 기본값을 답한다
ai_model      TEXT   어떤 모델로 만드는가     NULL = 아직 고르지 않았다 (정상)
```

세 열 모두 **NOT NULL도 DEFAULT도 없다.** 기본값 정책은 스키마가 아니라
`domain::settings::Settings::DEFAULT`가 갖는다는 이 테이블의 기존 규약(version 3의 주석) 그대로다.

## 변경한 파일

```text
src-tauri/src/db/migrations.rs           version 6 추가 + AC4·AC5 테스트
src-tauri/src/db/settings.rs             SELECT / INSERT ON CONFLICT에 세 열 추가
src-tauri/src/domain/settings.rs         Settings 세 필드 · DEFAULT · DEFAULT_AI_BASE_URL
src-tauri/src/domain/mod.rs              DEFAULT_AI_BASE_URL 재노출
src-tauri/src/commands/payload.rs        SettingsPayload 세 필드 + 양방향 변환
src-tauri/tests/settings_repository.rs   영속성 · 기본값 · 구버전 DB · INV-7
src-tauri/tests/command_boundary.rs      command 경계 왕복
src/ipc/types.ts                         Settings 세 필드 (string | null)
src/screens/settingsView.ts              편집하지 않는 세 값의 무손실 통과
src/screens/settingsView.test.ts         그 무손실 통과를 보는 테스트
```

## Acceptance Criteria 대응

| AC | 어디서 판정되는가 |
| --- | --- |
| AC1 build | self-check PASS — `.loop/evidence/TASK-035/gate-results.md` |
| AC2 lint | 같음 |
| AC3 test | 같음 |
| AC4 migration | `released_migrations_keep_their_version_and_name` (목록이 (1..6)으로 끝에만 붙었다) · `a_new_settings_column_is_added_by_a_new_migration_instead_of_editing_an_old_one`(새 열 셋이 옛 migration 안에 없다) · `no_migration_destroys_existing_data` · 통합 테스트 `a_database_written_before_the_ai_settings_existed_keeps_its_values`(version 5 DB를 올려도 행이 남는다). version 1~5의 version·name·sql은 `changes.diff`에서 한 글자도 바뀌지 않았다 — migrations.rs의 diff는 **목록 끝의 새 항목과 테스트 추가뿐이다**. `git diff --numstat`이 `50  0`(추가 50줄 · 삭제 0줄)이므로 기존 줄이 한 줄도 바뀌지 않았다 |
| AC5 INV-7 | 새 열은 `ai_provider` · `ai_base_url` · `ai_model` 셋뿐이다. `the_settings_schema_has_no_secret_columns`가 실제 `pragma_table_info('settings')` 결과를 목록과 정확히 비교한다(열이 하나라도 늘면 FAIL). `no_migration_creates_a_place_to_put_a_secret`(이 Run이 추가)이 모든 migration sql에서 key·token·secret·password·credential을 금지한다. `the_settings_api_does_not_accept_or_store_secrets`가 domain·db 소스의 코드 줄을 같은 기준으로 본다 |
| AC6 기본값·영속 | `Settings::DEFAULT.ai_provider == None`이고 `defaults_are_returned_when_nothing_has_been_saved` · `not_having_chosen_a_provider_is_the_normal_starting_state`가 그것을 판정한다 — 앱은 어떤 provider도 기본으로 굳혀 두지 않는다. 재시작(같은 경로로 DB를 닫고 다시 열기)은 `saved_values_survive_closing_and_reopening_the_database` · `a_chosen_provider_survives_closing_and_reopening_the_database`가 본다 |
| AC7 payload ↔ TS | `SettingsPayload`의 `ai_provider`/`ai_base_url`/`ai_model`(`Option<String>` + `serde(default)`, `rename_all = "camelCase"`)와 `src/ipc/types.ts`의 `aiProvider`/`aiBaseUrl`/`aiModel`(`string \| null`)이 기존 다섯 값과 **같은 규약**으로 짝을 이룬다. `updated_settings_are_stored_and_read_back`가 command 경계에서 셋의 왕복을 확인한다 |

## 판단이 필요했던 곳

**1. 연결 대상 기본값을 어디에 두는가.**
ADR-0008 §11.1은 `ai_base_url`이 NULL일 때 "adapter의 기본 주소를 쓴다"고 적지만, TASK-036은
adapter가 연결 대상을 **주입받으며 하드코딩하지 않는다**고 요구한다. 둘을 동시에 지키려면
기본값을 아는 자리가 설정 쪽에 있어야 한다. 그래서 `domain::settings::DEFAULT_AI_BASE_URL`
상수 **하나**와 그것을 적용하는 `Settings::ai_base_url_or_default()` **하나**를 두었다.
값 자체는 PRODUCT-SPEC §14.5가 primary source에서 확인해 기록하고 ADR-0008 §14가 그대로 옮긴
것(VERIFIED · 2026-09-01)이며, 테스트가 그 문자열을 고정해 기억으로 바뀌는 것을 막는다.
`Settings::DEFAULT.ai_base_url`은 여전히 `None`이다 — 기본 주소를 저장된 값으로 써 넣으면
"고른 적 없음"과 "마침 같은 주소를 고름"이 구분되지 않는다.

**2. `ai_context_tokens`를 더하지 않았다.**
ADR-0008 §11.1의 표에는 네 번째 열로 있지만, 이 Task의 request가 더하라고 적은 열은 셋이다
(provider 식별자 · 연결 대상 · 모델 식별자). 값을 읽는 코드가 아직 없는 열을 미리 만들지
않는 것이 이 저장소의 규약이기도 하다 (`domain/settings.rs`의 모듈 주석 · §20.6). 이 값을
실제로 읽는 자리(생성 요청)를 만드는 Task가 version 7로 더하면 된다 — `ContextBudget::DEFAULT`가
이미 그 기본값을 갖고 있어 지금 저장할 것이 없다.

**3. 설정 화면이 편집하지 않는 값을 지우지 않게 했다.**
UI는 이 Task의 범위가 아니므로 provider를 고르는 화면은 만들지 않았다. 다만 저장은 설정
**전체**를 보내므로, 폼이 모르는 세 값은 다른 설정을 한 번 저장하는 것만으로 `null`이 되어
사라진다. 그래서 `settingsView.ts`가 세 값을 **읽은 그대로 다시 내보내게** 했고
(`SettingsForm`의 통과용 필드), 그것을 보는 테스트를 두었다. 입력란도, 렌더링 변경도 없다.

## 남긴 것

`ai_base_url`은 secret이 아니지만 ADR-0008 §11.3은 설정값을 로그·실패 message·evidence에
남기지 않기를 요구한다. 이 evidence 파일들은 사용자 설정값을 적지 않으며, 테스트가 쓰는 값은
전부 실재하지 않는 고정 문자열이다.
