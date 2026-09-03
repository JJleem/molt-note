# TASK-029 — 이 Run이 바꾼 파일

작업 트리에는 앞선 Task(023~028)의 커밋되지 않은 변경이 함께 있다. **아래는 이 Run이 실제로
편집하거나 새로 만든 파일뿐이다.**

## 저장소 · domain

| 파일 | 무엇이 바뀌었나 |
| --- | --- |
| `src-tauri/src/db/migrations.rs` | 목록 **끝에** migration 5 `add_transcription_settings` 추가 (`automatic_transcription` · `transcription_model` 두 열). 기존 1~4의 version·sql은 그대로. 새 테스트 둘 추가 |
| `src-tauri/src/db/settings.rs` | 두 열의 load/save. `automatic_transcription`의 NULL(=저장된 적 없음)은 `Settings::DEFAULT` 값으로 읽는다. `decode_toggle`이 어느 열인지 함께 받도록 일반화 |
| `src-tauri/src/domain/settings.rs` | `automatic_transcription: bool` · `transcription_model: Option<String>` 추가. `DEFAULT`는 각각 `false` · `None` |

## command 표면

| 파일 | 무엇이 바뀌었나 |
| --- | --- |
| `src-tauri/src/commands/payload.rs` | `SettingsPayload`에 `automaticTranscription` · `transcriptionModel` 추가. 공백뿐인 모델 값은 '고르지 않음'으로 정규화 (다른 값으로 바꾸지는 않는다) |
| `src-tauri/src/commands/mod.rs` | `finish_recording`이 `&Transcriber`를 받고, 레코드 저장 뒤 `start_automatic_transcription`을 부른다. `stop_capture` command가 `Transcriber` state를 함께 받는다 |
| `src-tauri/src/commands/transcriber.rs` | 어떤 모델을 쓸지 **전사를 시작할 때 설정에서 읽는다.** 생성 시점에 들고 있던 `configured_model` 필드와 `with_model`을 없앴다 (설정 하나가 유일한 출처가 된다) |
| `src-tauri/src/transcription/run.rs` · `model.rs` | TASK-029를 가리키던 주석을 실제 구조(설정을 읽는 자리는 `Transcriber`)로 갱신. **코드 변경 없음** |

Tauri command 표면의 **이름과 개수는 그대로다**(15개) — `tests/ipc-boundary.test.ts`가 그것을
계속 요구하며 통과한다.

## 화면

| 파일 | 무엇이 바뀌었나 |
| --- | --- |
| `src/ipc/types.ts` | `Settings`에 `automaticTranscription` · `transcriptionModel` 추가 |
| `src/screens/settingsView.ts` | 폼에 두 값 추가. `transcriptionModel()` · `transcriptionNotices()`와 문구 상수 셋 추가 — 모델이 없는 상태를 화면 상태로 표현한다. 토글을 뒤집는 경로는 없다 |
| `src/screens/SettingsScreen.tsx` | Transcription 그룹이 실제 입력이 됐다(모델 입력 · 자동 전사 체크박스 · 안내). Save 버튼은 화면 전체에 하나로 옮겼다 — 설정은 한 벌이고 한 번에 저장되기 때문이다 |

## 테스트

| 파일 | 무엇이 바뀌었나 |
| --- | --- |
| `src-tauri/tests/automatic_transcription.rs` | **새 파일.** 자동 전사 트리거 7개 (AC6 · AC7) |
| `src-tauri/tests/settings_repository.rs` | 기본값 · 독립성 · 모델 보존 · version 4 DB 보존 테스트 추가. 열 목록 단언에 새 두 열 반영 |
| `src-tauri/tests/command_boundary.rs` | 두 토글 독립성 · 없는 모델 보존 테스트 추가. `SettingsPayload` 리터럴 갱신 |
| `src-tauri/tests/stop_persistence.rs` | `finish_recording`의 새 인자에 맞춰 호출 갱신. 자동 전사가 꺼진 기본 상태이므로 엔진까지 가지 않으며, 그것을 드러내려고 **실패를 내는 double**을 둔다 |
| `src-tauri/tests/transcription_background.rs` | 모델을 `with_model` 대신 **설정에 저장**해서 지정한다 — 실제 앱과 같은 경로가 됐다 |
| `src/screens/settingsView.test.ts` | 새 폼 값 · 두 토글 독립성 · 모델 없음 상태 테스트 추가, 기존 단언 갱신 |

## 삭제하거나 약화한 것

없다. 지운 테스트도, `#[ignore]`도, skip도 없다. 제거한 제품 API는
`Transcriber::with_model` 하나이며, 그 역할(어떤 모델을 쓸지 정하는 것)은 설정 값이
가져갔다 — 그 경로는 `transcription_background.rs`와
`automatic_transcription.rs`가 실제로 지난다.
