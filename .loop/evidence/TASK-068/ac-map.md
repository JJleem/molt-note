# TASK-068 — Acceptance Criterion → 판정 수단

각 AC가 **어떤 검사로** 판정되는지의 대응표다. Worker의 주장이 아니라 재실행 가능한 명령과
실제 코드 위치를 가리킨다.

| AC | 판정 수단 | 결과 |
| --- | --- | --- |
| AC1 `npm run lint` | Gate `lint` (`eslint .` + `cargo clippy --all-targets -- -D warnings`) | PASS exit=0 · `lint-stderr.log` |
| AC2 `npm run test` | Gate `test` (`vitest run` + `cargo test`) | PASS exit=0 · `test-language-selection.log` |
| AC3 `npm run build` | Gate `build` (`tsc && vite build`) | PASS exit=0 · `gate-results.md` |
| AC4 언어 선택이 값으로 엔진에 도달 · test double이 관측 | 아래 참조 | PASS |
| AC5 `whisper.rs`가 두 setter를 실제로 호출 · `Transcript.language`는 엔진 값 · `set_translate(false)` 유지 | 아래 참조 | PASS |

## AC4 — 선택이 엔진 경계에 값으로 도달한다

경로 (설정을 읽는 자리는 **하나**다):

```text
settings::load                    src-tauri/src/commands/transcriber.rs:250
  └─ LanguageChoice::from_setting(settings.transcription_language.as_deref())   :251
       └─ run::transcribe(..., &language)                                       :254-263
            └─ engine.transcribe(&input, &model, language)   src-tauri/src/transcription/run.rs:140
                 └─ TranscriptionEngine::transcribe(_, _, &LanguageChoice)      engine.rs:119-124
```

`LanguageChoice`는 `ModelChoice`와 같은 자리의 값 타입이다 (`transcription/engine.rs:68-96`).
`Detect` / `Chosen(String)` 두 갈래이며 `from_setting`이 공백만 있는 값을 `Detect`로 접는다.

test double이 그것을 관측한다 — `StubCall.language` (`transcription/testing.rs:32-36`):

| 테스트 | 위치 | 무엇을 판정하는가 |
| --- | --- | --- |
| `not_choosing_a_language_reaches_the_engine_as_detection` | `tests/transcription_language.rs:229` | 설정 없음 → 엔진이 받는 값은 `Detect`, `chosen()`은 `None` |
| `a_chosen_language_reaches_the_engine_as_that_language` | `tests/transcription_language.rs:244` | 설정 `"ko"` → `Chosen("ko")` |
| `a_blank_setting_is_the_same_as_not_having_chosen` | `tests/transcription_language.rs:254` | 공백만 있는 값 → `Detect` |
| `the_language_choice_reaches_the_engine_exactly_as_it_was_given` | `tests/transcription_run.rs` | `run::transcribe`가 값을 바꾸지 않는다 |
| `the_double_tells_detection_and_a_chosen_language_apart` | `transcription/testing.rs:223` | double이 두 갈래를 구분해 기록한다 |

**실제 whisper 없이 갈린다.** `transcription_language.rs`의 fixture는 `ggml-base.bin`이라는
이름의 16바이트 자리표시자와 0.1초짜리 무음 WAV뿐이고, 엔진 자리에는 `StubEngine`이 선다
(`Transcriber::with_engine`). Tauri 런타임도 창도 하드웨어도 모델도 없다.

**`whisper.rs`는 설정도 저장소도 읽지 않는다** — 그 사실 자체가 테스트다:
`the_real_engine_never_reads_settings_or_storage_to_decide_the_language`
(`tests/transcription_engine.rs:255`)가 주석을 제외한 소스에서 `settings` · `Settings` ·
`rusqlite` · `Connection` · `crate::db` · `app_data_dir::`를 찾아 하나라도 있으면 실패한다.

## AC5 — 두 setter 호출 · 결과 language의 출처 · set_translate(false)

`src-tauri/src/transcription/whisper.rs:164-180`:

```rust
match language.chosen() {
    Some(code) => { params.set_language(Some(code)); params.set_detect_language(false); }
    None       => { params.set_language(None);       params.set_detect_language(true);  }
}
```

`params.set_translate(false)`는 `whisper.rs:162`에 그대로 있다.

`the_real_engine_sets_the_language_instead_of_letting_whisper_default_to_english`
(`tests/transcription_engine.rs:230`)는 **모듈 문서가 아니라 실제 호출을 본다** — 소스에서
`params.`로 시작하는 줄만 모아 위 다섯 호출이 모두 있는지 확인한다.

`Transcript.language`는 여전히 **엔진이 보고한 값**이다:

- `whisper.rs:222` — `whisper_rs::get_lang_str(state.full_lang_id_from_state())`. 넘겨받은
  `language` 인자를 결과에 쓰는 경로가 없다 (인자는 위 `match`에서만 쓰인다).
- `run.rs:163` — `language: transcription.language` (= `parse::normalize`가 넘긴 엔진 값).
- `the_stored_language_is_the_one_the_engine_reported`
  (`tests/transcription_language.rs:263`): 고른 값 `"ja"`, 엔진 보고 `"ko"` → 저장된 값 `"ko"`.
- `detection_being_on_does_not_mean_a_language_is_always_known`
  (`tests/transcription_language.rs:281`): 감지를 켰어도 엔진이 말하지 못하면 `None`이며
  그 자리를 설정 값이나 짐작으로 채우지 않는다.

`whisper-rs` 0.16의 실제 시그니처 확인 수단은 **컴파일러 하나**였다 (이 Run에는
`~/.cargo/registry` 접근이 없다). `cargo clippy --all-targets -- -D warnings`가 이 호출들을
컴파일해 exit=0으로 끝났다 — `lint-stderr.log`.
