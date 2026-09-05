# TASK-057 — 경계가 지키는 것과 그것을 판정하는 검사 (AC-4 · AC-5 · AC-6)

## AC-4 — tripwire 네 자리를 무르게 만들지 않았다

ADR-0010 §8.3이 구현 **전에** 예고한 네 자리가 전부 깨졌고, 예고한 방식대로만 갱신했다.
`git diff tests/ipc-boundary.test.ts`가 원본이다.

| # | 자리 | 어떻게 다뤘는가 | 무르게 만들었는가 |
| --- | --- | --- | --- |
| 1 | `REGISTERED_COMMANDS` | 이름 셋을 더하고, 왜 늘었는지를 상단 doc 주석에 적었다 (28 → 31) | 아니다 — 여전히 **정확히 같은 집합**을 요구한다 (`toEqual`) |
| 2 | `outOfScope` 정규식 | `export(?!_markdown\b)` → `export(?!_(markdown\|ai_request)\b)` | 아니다 — 허용 이름이 **둘로 열거**됐을 뿐, `export_pdf` · `export_all` 같은 이름은 그대로 막힌다 |
| 3 | `'Markdown export 표면은 … 이름 하나뿐이다'` | 제목과 기대값을 두 이름으로 갱신 (`['export_ai_request', 'export_markdown']`) | 아니다 — 부분집합이 아니라 정확한 목록이며, 셋째 이름이 생기면 깨진다 |
| 4 | `'전사 표면은 …'` | 기대 목록에 `get_transcript_text`를 넣고, **읽기 파생 하나이지 전사 큐가 아니라는 것**을 주석으로 남겼다 | 아니다 — 목록은 여전히 정확하며, 큐 이름이 생기면 깨진다 |

**약화하지 않았다는 것의 형태**: `.sort()`가 붙은 자리는 순서 의존을 없앤 것이고
(같은 파일의 Notion · AI · 전사 검사가 이미 쓰는 방식이다), 어떤 정규식도 넓히지 않았고,
어떤 `expect`도 지우거나 `toContain`으로 바꾸지 않았다.

**손대지 않은 검사** (ADR-0010 §8.3이 "깨지지 않는다"고 예고한 그대로 통과했다):

```text
src/ 아래에 SQL 문장이 없다
src/ 아래에서 command를 부르는 곳은 ipc 모듈뿐이다
AI 표면은 provider 상태 · 생성 시작 · 진행 상태 · 저장된 노트 읽기 둘뿐이다   ← get_ai_prompt는 필터에 걸리지 않는다
저장된 Transcript · AI 노트 · Notion 전송 기록을 고치거나 지우는 command가 없다
wire 계약에 벤더가 없다 (INV-9)  ·  자격증명은 한 방향으로만 지난다 (INV-7)
Rust의 실패 종류가 frontend 타입에 전부 있다 / 그 반대도 (declared.length === kinds.size + 1)
frontend가 부르는 이름이 등록된 이름과 정확히 같다
```

## AC-5 — current만 고르고(MH-5) · 저장소에 쓰지 않고(MH-7) · provider를 보지 않는다(MH-1 · MH-2)

### MH-5 — 고르는 자리가 하나다

Transcript를 고르는 규칙은 `export::run::current_input` **한 함수**에 있고, Markdown export와
Manual Handoff가 그 함수를 부른다. 함수가 하는 일은 `Recording.current_transcript_id`를 읽는
것뿐이며, 다른 version을 찾는 질의가 이 경로에 없다.

```rust
let Some(transcript_id) = recording.current_transcript_id.clone() else { … };
let transcript = store::load_transcript(connection, &transcript_id)?…
```

wire에도 고를 수단이 없다 — 세 command 전부 `recording_id: String`만 받는다. 두 검사가
그것을 판정한다.

```text
only_the_transcript_that_current_points_at_reaches_any_of_the_three_outputs
  옛 version과 current를 함께 저장하고 세 산출물 전부에서 current 문장만 나오는지 본다
the_three_commands_take_a_recording_id_and_never_a_transcript_id
  commands/mod.rs의 세 시그니처에 transcript_id가 없다는 것을 소스로 못박는다
```

### MH-7 — 저장소에 쓰는 코드가 없다

이 경로가 부르는 저장소 함수는 `load_recording` · `load_transcript` 둘뿐이다. 파일시스템에
닿는 자리는 `file::write_new` 하나이며, 그것은 **새 파일 하나를 더할 뿐** 기존 파일을 열지도
지우지도 않는다 (`create_new`).

```text
a_recording_without_a_current_transcript_is_refused_and_nothing_changes
a_transcript_with_nothing_in_it_is_refused_instead_of_making_an_empty_request
asking_for_a_recording_that_is_not_there_leaves_the_others_alone
  → 세 실패 뒤에 recording · transcripts · ai_notes 스냅샷이 완전히 같고,
    exports/ 에 파일이 하나도 남지 않는다. 실패는 전부 source_data_safe = true다.
```

### MH-1 · MH-2 — provider가 없다는 이유로 거절할 수단이 없다

```text
all_three_outputs_are_produced_without_any_ai_provider_configured
  설정이 Settings::DEFAULT(provider · model · baseUrl 전부 None)인 저장소에서
  세 mode의 프롬프트 · 세 mode의 Export for AI · Transcript 텍스트가 전부 성공한다

nothing_in_the_manual_handoff_boundary_can_reach_a_provider_a_network_or_a_write
  handoff.rs · ai_request.rs의 **주석을 뺀 제품 코드**에 다음 문자열이 하나도 없다:
  provider · ai_base_url · ai_model · settings:: · Settings · ollama · ureq · http ·
  insert_ · update_ · delete_ · append_ · set_current · execute
```

`src-tauri/src/ai/`의 diff는 비어 있다 — provider 추상화도 프롬프트 상수도 그대로다 (MH-8).

## AC-6 — Export for AI는 Phase 5의 자리를 재사용하고 덮어쓰지 않는다

```text
디렉터리   AppDataDirectory::ensure_exports_dir()    ← Markdown export와 같은 호출, 같은 자리
쓰기       export::file::write_new(…)                ← 같은 함수. create_new이므로 덮어쓰지 않는다
이름       ai_request_file_name = export_file_name + `-ai-request` 표식 하나
돌려주는 값 ExportedFilePayload { recordingId, path, fileName }  ← 실제로 쓰인 경로와 이름
```

두 export는 `Exporter::write_file` 하나를 공유한다 — 자리를 준비하고, 저장소를 열고, 넘겨받은
실행 순서로 파일 하나를 쓴다. 두 번째 export 시스템도, 두 번째 디렉터리도 생기지 않았다.

```text
the_ai_request_lands_in_the_same_exports_directory_under_its_own_name
  두 파일이 같은 exports/ 아래에 놓이고, 이름은 `…-3dgs-study-04.md`와
  `…-3dgs-study-04-ai-request.md`이며, 먼저 만든 Markdown 파일이 그대로 남는다.
  첫 줄도 다르다 — `# 3DGS Study #04` / `# Molt Note AI Request`

a_second_ai_request_gets_a_number_instead_of_overwriting_the_first
  첫 파일을 사용자가 손댄 뒤 다시 내보내면 `…-ai-request-2.md`가 만들어지고,
  손댄 내용은 한 글자도 바뀌지 않는다 (ADR-0009 §4.3)

the_written_document_is_exactly_what_the_pure_renderer_makes
  파일 바이트 == AiRequest::ai_ready_document() — 파일이 되는 자리가 문자열을 다시 손대지 않는다
```

## 이 Task가 하지 않은 것

```text
clipboard 경계 (src/platform/clipboard.ts)    P4의 몫이다 — 이 경계는 문자열을 돌려줄 뿐이다
화면 · React 컴포넌트 · 순수 view 모듈        P5 이후의 몫이다 (src/screens/**의 diff가 비어 있다)
새 FailureKind · 새 payload 타입              필요하지 않았다 (diff.md)
프롬프트 상수 · provider 추상화의 변경        MH-8 — src-tauri/src/ai/의 diff가 비어 있다
```
