# TASK-018 — Settings의 default microphone과 사라진 장치 처리

Run: `RUN-20260902T091108Z-TASK-018` · 2026-09-02

## 무엇을 만들었나

`default microphone` 값이 **스키마 → domain → payload → TypeScript 타입 → Settings 화면**까지
이어지고, 저장된 장치가 지금 목록에 없는 경우가 **값으로 구분되는** 상태가 됐다.

```text
migrations.rs  version 4  add_default_microphone_to_settings   (목록 끝에 추가)
domain::Settings.default_microphone: Option<String>            (DEFAULT = None)
db::settings::load / save                                       (열 하나 더)
SettingsPayload.default_microphone: Option<String>              (camelCase: defaultMicrophone)
src/ipc/types.ts  Settings.defaultMicrophone: string | null
src/screens/defaultMicrophone.ts   ← 순수 해석 함수 (새 파일)
src/screens/settingsView.ts        폼 값 ↔ 설정 값
src/screens/SettingsScreen.tsx     열거된 장치 중에서 고르는 <select>
```

저장하는 값은 **표시 이름이 아니라 선택 키**다 (`InputDevice.key`). 이름이 같은 장치가 둘 있을
수 있어서 이름으로는 어느 것인지 말할 수 없다 (`src-tauri/src/audio/devices.rs`).

## 세 가지 해석 결과 — `resolveDefaultMicrophone(saved, devices)`

`src/screens/defaultMicrophone.ts`

```text
notChosen  saved === null              고른 적이 없다        (정상 상태)
available  saved가 목록에 있다          그 장치를 돌려준다
missing    saved가 목록에 없다          저장된 키를 그대로 들고 돌려준다
```

**조용한 대체가 없다는 것을 어디서 보장하는가**

- `resolveDefaultMicrophone`에는 목록에서 다른 장치를 꺼내는 경로가 없다 — 찾지 못하면
  `{ kind: 'missing', key: saved }`이며, 저장된 키를 버리지도 않는다.
- `microphoneOptions`는 `missing`일 때 **저장된 키를 위한 항목을 목록에 더한다.**
  그 항목이 없으면 `<select>`가 저장된 값을 표현하지 못해 브라우저가 다른 항목을 골라진
  것처럼 보여 준다 — 코드가 아니라 DOM이 대체하는 경로였다. 테스트:
  `고른 값이 언제나 목록 안에 있다`.
- 저장 경로(`SettingsPayload → Settings`)도 알아볼 수 없는 키를 바꾸지 않는다. 하는 일은
  빈 문자열을 `None`으로 만드는 것뿐이다 (`recordings_directory`와 같은 규칙).
- `db::settings::load`는 저장된 키가 지금 있는 장치인지 묻지 않는다 — 저장소는 장치를 알지 않는다.

## 해석 함수가 TypeScript에 있는 이유

두 화면이 이 결과를 소비한다 — 이 Task의 Settings 화면과 P6(TASK-020)의 녹음 화면이다.
command 표면(`generate_handler!`)은 열세 개로 고정돼 있고 `tests/ipc-boundary.test.ts`가
정확히 같은 집합을 요구하므로, Rust에 두면 **새 command를 만들거나 같은 규칙을 두 벌로
갖게 된다.** 규칙을 한 곳에만 두는 쪽을 택했다.

Rust 쪽이 이 판단을 필요로 하지 않는 것도 확인했다 — 실제로 장치를 여는 경로는 키를 직접
받고, 찾지 못하면 이미 `Failure`가 된다
(`src-tauri/src/audio/system_capture.rs:146` "고른 입력 장치를 찾지 못했다. 장치가 빠졌을 수 있다.").
즉 backend에는 대체 경로가 없고, 이 모듈은 **시작하기 전에 사용자에게 무엇을 보여 줄지**를 정한다.

## migration

`src-tauri/src/db/migrations.rs`

- version 1 · 2 · 3의 `version`·`name`·`sql`은 **한 글자도 고치지 않았다.**
- 목록 끝에 version 4를 추가했다: `ALTER TABLE settings ADD COLUMN default_microphone TEXT;`
- 이미 있는 행을 지우거나 다시 만들지 않는다. 없던 열은 NULL('아직 고르지 않음')로 시작한다.
- 기존 테스트 `no_migration_destroys_existing_data`(DROP/DELETE 금지)가 그대로 통과한다.

version 3까지만 적용된 DB를 만들어 두고 올리는 경로를 테스트로 직접 지난다:
`a_database_written_before_the_default_microphone_existed_keeps_its_values`.

## INV-7

새 필드는 `default_microphone` 하나이며 secret이 아니다.

- `the_settings_schema_has_no_secret_columns` — 설정 테이블의 열 목록이
  `id · recordings_directory · automatic_processing · default_microphone` **정확히 이 넷**임을 요구한다.
- `the_settings_api_does_not_accept_or_store_secrets` — `db/settings.rs`와 `domain/settings.rs`의
  코드 줄에 `api_key · apikey · token · password · secret · credential`이 없음을 요구한다.

두 테스트 모두 이 Task에서 약화시키지 않았다 — 열 목록 검사는 새 열을 **명시적으로 추가**하는
방향으로만 고쳤다.

## Acceptance Criteria 대응

| AC | 무엇으로 판정되는가 |
| --- | --- |
| AC1 (test) | `gate-results.md` — test PASS. 영속성: `settings_repository.rs`의 재시작 테스트 4개. 세 가지 해석 결과: `src/screens/defaultMicrophone.test.ts` |
| AC2 (lint) | `gate-results.md` — lint PASS (`eslint .` + `cargo clippy --all-targets -- -D warnings`) |
| AC3 (build) | `gate-results.md` — build PASS (`tsc && vite build`) |
| AC4 (verifier) | 위의 "조용한 대체가 없다는 것을 어디서 보장하는가" · "migration" · "INV-7" · `payload-type-parity.md` |

## 범위

녹음 화면(`RecordingScreen.tsx` · `captureSpikeView.ts`)은 건드리지 않았다 — 소비는 P6이 한다.
command 표면도 그대로다(열세 개). commit하지 않았다.
