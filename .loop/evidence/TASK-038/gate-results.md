# TASK-038 — Gate self-check (advisory)

명령: `node tools/loop-runtime/loopctl.mjs self-check build lint test`
실행 시각: 2026-09-04 (Attempt 2)
원본 로그: `.loop-local/self-check/gates/{lint,test}/{stdout,stderr}.log`

| Gate  | command         | exit | 결과 |
|-------|-----------------|------|------|
| build | `npm run build` | 0    | PASS |
| lint  | `npm run lint`  | 0    | PASS |
| test  | `npm run test`  | 0    | PASS |

## lint

```
> eslint .
> molt-note@0.1.0 lint:rust
> cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.42s
```

`-D warnings`로 clippy가 돌았고 경고가 하나도 나오지 않았다.

## test

web (vitest):

```
 Test Files  15 passed (15)
      Tests  208 passed (208)
```

rust (cargo test) — 이 Task가 여는 경계를 지나는 통합 테스트 두 개가 여기 들어 있다:

```
     Running tests/ai_note_commands.rs
     Running tests/ai_note_run.rs
     Running tests/command_boundary.rs
     Running tests/ollama_adapter.rs
```

실패 0, ignored 0. 전체 스위트에서 `0 failed`다.

## AC 대응

| AC  | 판정 수단 | 어디서 확인되는가 |
|-----|-----------|-------------------|
| AC1 | build Gate | 위 표 · exit 0 |
| AC2 | lint Gate  | 위 표 · exit 0 |
| AC3 | test Gate  | 위 표 · exit 0 |
| AC4 | verifier   | `commands/notes.rs` `provider_status`가 `Result`가 아니다 · `tests/ai_note_commands.rs`의 `a_provider_that_was_never_chosen_is_a_state_not_a_failure` · `asking_for_a_note_without_a_provider_is_accepted_and_answered_as_a_state` · `a_server_that_does_not_answer_and_a_server_without_models_are_different_states` |
| AC5 | verifier   | `commands/notes.rs`의 `spawn`이 `Transcriber::spawn`과 같은 구조(배경 스레드 · 자물쇠는 시작/끝만) · `tests/ai_note_commands.rs`의 `a_status_query_answers_while_the_provider_is_still_generating` · `other_commands_answer_while_a_note_is_being_generated` |
| AC6 | verifier   | `tests/ipc-boundary.test.ts`가 `commands/payload.rs`와 `src/ipc/types.ts` 소스에서 벤더 이름 부재와 `locality` 존재를 확인한다 (`pub locality: Option<String>` · `locality: AiProviderLocality \| null`) |
