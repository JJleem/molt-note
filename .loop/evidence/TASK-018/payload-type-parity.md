# Rust payload ↔ `src/ipc/types.ts` 대조 (AC4 (3))

`SettingsPayload`는 `#[serde(rename_all = "camelCase")]`이므로 wire 이름은 camelCase다.

## `SettingsPayload` (`src-tauri/src/commands/payload.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPayload {
    #[serde(default)]
    pub recordings_directory: Option<String>,
    pub automatic_processing: bool,
    #[serde(default)]
    pub default_microphone: Option<String>,
}
```

## `Settings` (`src/ipc/types.ts`)

```ts
export interface Settings {
  readonly recordingsDirectory: string | null;
  readonly automaticProcessing: boolean;
  readonly defaultMicrophone: string | null;
}
```

## 대조

| wire 이름 | Rust | TypeScript | 같은가 |
| --- | --- | --- | --- |
| `recordingsDirectory` | `Option<String>` | `string \| null` | 예 |
| `automaticProcessing` | `bool` | `boolean` | 예 |
| `defaultMicrophone` | `Option<String>` | `string \| null` | 예 |

양쪽 모두 필드가 셋이고, 한쪽에만 있는 필드는 없다.

`#[serde(default)]`는 `Option` 필드가 JSON에서 빠져 있어도 `None`으로 읽히게 한다 —
TypeScript가 `null`을 명시적으로 보내는 지금 경로에서도 결과는 같고, 값이 없는 것과
`null`이 다른 뜻이 되는 상태를 만들지 않는다.

## secret 필드가 없다 (AC4 (4) · INV-7)

새로 생긴 필드는 `defaultMicrophone` 하나이며 입력 장치의 선택 키다. API key ·
integration token · password를 담는 자리는 payload에도, `domain::Settings`에도,
`settings` 테이블에도 없다. 두 테스트가 그것을 계속 요구한다 —
`the_settings_schema_has_no_secret_columns`, `the_settings_api_does_not_accept_or_store_secrets`.

## 소비 경로

```text
get_settings / update_settings  →  SettingsPayload            (Rust)
                                →  Settings                   (src/ipc/types.ts)
                                →  SettingsForm.defaultMicrophone: string
                                     ''  = 고르지 않음 (select가 null을 담을 수 없다)
                                →  chosenMicrophone(value): string | null
                                →  resolveDefaultMicrophone(saved, devices)
                                     notChosen | available | missing
```

`''`와 `null` 사이의 변환은 `chosenMicrophone` 하나에만 있다.
