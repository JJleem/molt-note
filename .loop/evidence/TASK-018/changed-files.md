# 변경 파일 (TASK-018)

commit하지 않았다. 아래는 작업 트리의 변경이다.

## 새로 만든 파일

```text
src/screens/defaultMicrophone.ts        순수 해석 함수 (세 가지 결과 · 선택지 목록 · 안내 문장)
src/screens/defaultMicrophone.test.ts   그 함수의 테스트 (장치 목록을 값으로 넣는다)
```

## 고친 파일

```text
src-tauri/src/db/migrations.rs          version 4를 목록 끝에 추가 (기존 항목 무수정)
src-tauri/src/domain/settings.rs        Settings.default_microphone + DEFAULT = None
src-tauri/src/db/settings.rs            load/save가 열 하나를 더 다룬다
src-tauri/src/commands/payload.rs       SettingsPayload.default_microphone + 빈 값 정규화
src/ipc/types.ts                        Settings.defaultMicrophone: string | null
src/screens/settingsView.ts             SettingsForm.defaultMicrophone + toSettings/toForm
src/screens/SettingsScreen.tsx          장치 열거 + <select> + 없어진 장치 안내
```

## 고친 테스트

```text
src-tauri/tests/settings_repository.rs  새 필드 반영 · 열 목록 갱신 · 영속성 테스트 4개 추가
src-tauri/tests/command_boundary.rs     새 필드 반영 · 없어진 키가 대체되지 않는 테스트 추가
src/screens/settingsView.test.ts        새 폼 필드 반영 · 편집/저장 경로 테스트 추가
```

## 건드리지 않은 것

```text
src/screens/RecordingScreen.tsx         녹음 화면은 이 Task의 범위가 아니다 (P6)
src/screens/captureSpikeView.ts         같은 이유
src-tauri/src/lib.rs                    command 표면은 그대로 열세 개
src-tauri/src/audio/**                  장치 열거 규칙은 그대로다
.loop/** (evidence 밖)                  Runtime 소유
```
