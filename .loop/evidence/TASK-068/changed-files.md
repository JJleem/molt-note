# TASK-068 — 변경 파일

`git diff --numstat HEAD` (HEAD = `eaf84f4`) 중 **이 Task의 범위**에 해당하는 것만 적는다.
working tree에는 앞선 Task(TASK-066 설정 컬럼 · TASK-067 설정 화면)의 변경도 함께 있으며,
그것들은 이 목록에 넣지 않는다.

| 파일 | +/- | 무엇이 바뀌었나 |
| --- | --- | --- |
| `src-tauri/src/transcription/engine.rs` | +96 / -2 | `LanguageChoice` 값 타입 추가 (`Detect` / `Chosen`, `from_setting`, `chosen`) · `TranscriptionEngine::transcribe`에 `language: &LanguageChoice` 인자 추가 · 단위 테스트 3개 |
| `src-tauri/src/transcription/whisper.rs` | +46 / -3 | 넘어온 선택을 `set_language` / `set_detect_language` 호출로 옮긴다 · 모듈 문서의 API 목록과 파라미터 설명 갱신 · `set_translate(false)` 유지 |
| `src-tauri/src/transcription/run.rs` | +17 / -8 | `transcribe`가 `&LanguageChoice`를 받아 엔진으로 그대로 넘긴다 (해석하지 않는다) · 모듈 문서 갱신 |
| `src-tauri/src/transcription/testing.rs` | +39 / -7 | `StubCall.language` 추가 — double이 넘어온 선택을 관측한다 · 단위 테스트 |
| `src-tauri/src/transcription/mod.rs` | +9 / -3 | `LanguageChoice` 재수출 · 모듈 문서의 데이터 흐름 그림에 언어 경로 추가 |
| `src-tauri/src/commands/transcriber.rs` | +21 / -9 | 이미 모델을 읽던 자리에서 언어도 **같은 `settings::load` 한 번으로** 읽어 `LanguageChoice`로 넘긴다 · 모듈 문서 갱신 |
| `src-tauri/tests/transcription_language.rs` | 신규 (292줄) | 설정 → 엔진 경계까지의 통합 테스트 5개. 실제 whisper도 모델도 쓰지 않는다 |
| `src-tauri/tests/transcription_engine.rs` | +55 / -6 | `whisper.rs` 소스에 대한 두 테스트(두 setter를 실제로 부르는가 · 설정/저장소에 닿지 않는가) · 기존 호출부에 `LanguageChoice` 인자 추가 |
| `src-tauri/tests/transcription_run.rs` | +86 / -2 | 언어 선택이 그대로 지나가는가 · 저장된 language가 엔진 값인가 |
| `src-tauri/tests/transcription_background.rs` | +3 / -2 | 시그니처 변경에 따른 호출부 정리 |

범위 밖이라 손대지 않은 것:

- `.loop/**` (evidence 디렉터리 하나 제외) · `.loop-local/**` · Task 파일의 어떤 필드도
- `docs/**` — ADR-0007 §17의 서술은 working tree에 이미 있었고 이 Run이 고치지 않았다
- 설정 저장소 · 마이그레이션 · 설정 화면 (TASK-066 / TASK-067의 몫)

## 이 Run에 대한 사실 하나

이 working tree에는 **같은 Task의 앞선 Worker 실행이 만든 변경이 이미 있었다.**
`.loop/evidence/TASK-068/gate-results.md`가 15:04에 쓰여 있었고, 그 파일이 가리키는 두 로그
(`lint-stderr.log` · `test-language-selection.log`)는 존재하지 않았다 — 그 실행은 Result를
쓰기 전에 끊긴 것으로 보인다. 이 Run은 그 상태에서 시작해 세 Gate를 직접 다시 돌려
(build/lint/test 전부 exit=0) 결과를 확인하고, 빠져 있던 Evidence를 채우고 Result를 썼다.
소스에 대한 추가 변경은 하지 않았다.
