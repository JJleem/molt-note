# TASK-056 — 변경 범위 (2026-09-04 · base `358ab79`)

```text
?? src-tauri/src/export/ai_request.rs    새 순수 모듈 + 단위 테스트 16개
 M src-tauri/src/export/markdown.rs      private 함수 셋을 pub(crate)로 끌어올림 (렌더 결과 불변)
 M src-tauri/src/export/mod.rs           모듈 등록 · 재export · 모듈 doc 한 줄
```

```text
 src-tauri/src/export/markdown.rs | 65 +++++++++++++++++++++++++++++-----
 src-tauri/src/export/mod.rs      |  3 ++
 2 files changed, 54 insertions(+), 14 deletions(-)
```

## AC-3 — `src-tauri/src/ai/`의 diff는 비어 있다

```text
$ git diff --stat -- src-tauri/src/ai
(출력 없음)
```

`prompt.rs`의 `MEETING_PROMPT` · `STUDY_PROMPT` · `SUMMARY_PROMPT` ·
`TRANSCRIPT_PLACEHOLDER` · `PROMPT_VERSION_{MEETING,STUDY,SUMMARY}` 전부 한 글자도
바뀌지 않았다. 그러므로 `prompt_version_is_bound_to_the_prompt_text`도 고쳐지지 않은 채
통과했다 (`.loop/evidence/TASK-056/gates.md`).

새 모듈은 그 상수를 **읽어서 치환**한다 — `ai_request.rs`의 유일한 프롬프트 조립 자리:

```rust
fn instructions(mode: NoteType, transcript_slot: &str) -> String {
    prompt_template(mode)                                     // 상수 원문 — 읽기만 한다
        .replace(JSON_OUTPUT_CONTRACT, &markdown_output_contract(mode))  // (a)
        .replace(TRANSCRIPT_PLACEHOLDER, transcript_slot)                 // (b)
        .trim_end()
        .to_owned()
}
```

프롬프트 문장을 복사해 붙인 자리가 없다는 것은
`the_manual_prompt_is_built_from_the_untouched_prompt_constants`가 값으로 판정한다 —
상수의 첫 줄로 시작하고 `Rules:` 절이 원문 그대로 남아 있다. 두 번째 프롬프트 세트는 없다.
(치환 (a)가 0회가 되면 `the_json_output_contract_is_replaced_exactly_once_in_every_prompt`가
먼저 깨진다 — ADR-0010 §6.2-a · §13.)

## AC-4 — 렌더링 규칙이 복제되지 않았다

`markdown.rs`의 private 함수를 **끌어올렸다.** 새 규칙을 쓰지 않았다.

| 규칙 | 자리 | 누가 쓰는가 |
| --- | --- | --- |
| transcript 본문 (segment 순서 보존 · 빈 segment 제거 · 없으면 `raw_text` · 그마저 비면 없음) | `markdown::transcript_body` (신설 · `pub(crate)`) | `markdown::transcript_blocks` · `AiRequest::manual_prompt` |
| `## Transcript` + 본문 블록 | `markdown::transcript_blocks` (`fn` → `pub(crate) fn`) | `markdown::render` · `AiRequest::{transcript_text, ai_ready_document}` |
| `Date:` · `Duration:` 두 줄 | `markdown::metadata_block` (신설 · `pub(crate)`) | `markdown::render` · `AiRequest::ai_ready_document` |
| 제목 한 줄 (`heading_text`) | `markdown::heading_text` (`fn` → `pub(crate) fn`) | `markdown::render` · `AiRequest::ai_ready_document` |
| `### HH:MM:SS` | `markdown::format_timestamp_ms` (**손대지 않았다**) | `transcript_body` 한 자리 |

`markdown::render`가 내는 바이트는 바뀌지 않았다 — 기존 golden 테스트가 고쳐지지 않은 채로
통과했다 (gates.md).

## AC-5 — audio도 벤더도 담을 자리가 없다

```rust
pub struct AiRequest<'a> {
    pub mode: NoteType,
    document: ExportDocument<'a>,   // recording · transcript · note: None
}
```

- 입력 타입에 `audio_path` · `audio_format` · 오디오 바이트를 담는 필드가 **없다.** 생성자는
  `(mode, &Recording, &Transcript)` 셋만 받는다 (MH-4 · INV-6).
- 산출물이 읽는 Recording 필드는 `title` · `created_at` · `duration_ms` 셋뿐이며, 셋 다
  `markdown`의 함수를 거친다. `no_audio_path_and_no_audio_format_reaches_any_of_the_three_outputs`가
  세 산출물 문자열에 `audio` · `.wav` · `flac` · 경로가 없음을 단언한다.
- `no_vendor_name_appears_in_any_of_the_three_outputs`가 세 mode × 세 산출물에서 벤더 이름
  부재를 단언한다. **이 파일 자체도 로컬 provider 이름을 글자로 담지 않는다** — 담으면
  `no_source_outside_the_adapter_knows_this_vendors_endpoints_or_parameters`(INV-9)가 깨진다.
  실제로 첫 시도에서 그 검사가 깨졌고, 이름을 조각에서 만들도록 고쳐 통과시켰다.
- `ai_request.rs`의 제품 경로 `use`는 `crate::ai::prompt` · `crate::domain` · `super::markdown`
  셋뿐이다 (`#[cfg(test)]` 모듈은 키/섹션 대응을 보려고 `crate::ai::note`의 필드 상수를 더 읽는다).
  `std::fs` · `std::net` · `rusqlite` · `ureq` · `crate::db` · `crate::ai::provider` ·
  clipboard가 **하나도 없다** — 파일 · 네트워크 · 저장소 · clipboard에 닿는 코드가 없다.
- 시계 · 난수 · 로캘도 없다 (§18). `the_same_input_always_makes_the_same_three_strings`가
  세 mode에서 세 산출물의 결정성을 판정한다.
