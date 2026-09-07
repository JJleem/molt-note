# TASK-067 · 실제로 바꾼 파일

`settings`의 전사 언어 값 하나를 저장소 → domain → command 경계 → 폼 → 화면까지 잇는다.
전사 실행 경로(`crate::transcription::whisper`)는 이 Task가 건드리지 않았다.

## 제품 코드

| 파일 | 무엇을 |
| --- | --- |
| `src-tauri/src/db/migrations.rs` | migration **version 9 `add_transcription_language`를 목록 끝에 추가**. `ALTER TABLE settings ADD COLUMN transcription_language TEXT;` — NOT NULL도 DEFAULT도 없다. 이미 적용된 migration의 version·sql은 손대지 않았다 (1~8 그대로). secret 열 없음 (INV-7) |
| `src-tauri/src/domain/settings.rs` | `Settings::transcription_language: Option<String>` 필드와 `DEFAULT`(=`None`) 추가. 로캘을 짐작해 굳혀 두지 않는 이유를 기존 필드와 같은 밀도로 주석에 남겼다 |
| `src-tauri/src/db/settings.rs` | `load`의 SELECT/튜플과 `save`의 INSERT·`ON CONFLICT DO UPDATE`에 열 추가. 값이 유효한 언어 코드인지 묻지 않고, 없다고 해서 지우거나 바꾸지 않는다 (`transcription_model`과 같은 규칙) |
| `src-tauri/src/commands/payload.rs` | `SettingsPayload::transcription_language` 추가 + 양방향 `From`. 공백뿐인 입력만 `None`으로 다듬는다 |
| `src/ipc/types.ts` | `Settings.transcriptionLanguage: string \| null` |
| `src/screens/settingsView.ts` | `SettingsForm.transcriptionLanguage` · `toForm` · `toSettings`. 폼을 지나가므로 다른 설정을 저장해도 값이 `null`이 되지 않는다. 표현 규칙(`transcriptionLanguage` 상태 · `transcriptionLanguageNotices` · `TRANSCRIPTION_LANGUAGE_PLACEHOLDER` 등)도 여기 있다 |
| `src/screens/SettingsScreen.tsx` | Transcription 구역 **안에** 입력란과 안내 문장 추가. Phase 5.5가 세운 구역 구성은 그대로다. 컴포넌트는 그리기만 한다 |

## 테스트

| 파일 | 무엇을 |
| --- | --- |
| `src-tauri/src/db/migrations.rs` (`mod tests`) | 기존 규약 테스트의 목록 갱신(버전 9 · 새 열 이름) + `the_transcription_language_column_lives_only_in_the_migration_that_added_it` · `the_transcription_language_column_carries_no_default_language` 추가 |
| `src-tauri/tests/settings_repository.rs` | 왕복 · 다른 설정 저장 시 유지 · 모르는 코드 유지 · 지움 · version 8 이전 DB 업그레이드 · 기본값이 스키마가 아니라 코드에 있음 — 6개 추가. 스키마 열 목록과 기본값 단언도 갱신 |
| `src-tauri/tests/command_boundary.rs` | `SettingsPayload` 왕복에 언어 포함, 공백뿐인 값은 '고르지 않음', 모르는 코드는 그대로 저장 |
| `src/screens/settingsView.test.ts` | `toForm`/`toSettings` 왕복 · 다른 설정 저장 시 유지 · 저장 뒤 재충전 유지 · 자동 감지 문구 · 모델과 언어가 서로를 정하지 않음 |
| `src/screens/aiProviderSettings.test.ts` | 공유 fixture(`DEFAULT_SETTINGS`)에 새 필드 |

## 이 Task가 하지 않은 것

- 전사 실행 경로(`set_language` / `set_detect_language` 호출)는 건드리지 않았다 — Task 범위 밖이다.
- 사용자 로캘을 읽는 코드를 넣지 않았다. 저장소·경계·폼·화면 어디에도 언어를 짐작해 채우는
  경로가 없다 (`grep`으로 확인: `src/`·`src-tauri/src/`에 locale 관련 호출 없음.
  `src/screens/recordingsView.ts`의 `Intl.DateTimeFormat('en-US', …)`는 고정된 날짜 형식이며
  언어 설정과 무관하다).
