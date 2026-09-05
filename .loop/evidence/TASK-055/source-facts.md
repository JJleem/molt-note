# TASK-055 — ADR-0010이 [E1]로 주장하는 저장소 사실과 그 출처

RUN-20260904T080544Z-TASK-055 · 2026-09-04 · HEAD `358ab79`

ADR-0010은 결정 문서이고, 그 결정은 **저장소에서 직접 읽은 값** 위에 서 있다.
아래는 이 Run이 읽은 자리와 값이다. 제3자가 같은 자리를 열어 대조할 수 있다.

## 1. `export::markdown` — 재사용 대상 (§5)

| 사실 | 자리 |
| --- | --- |
| `pub struct ExportDocument<'a> { recording, transcript, note: Option<&StructuredNote> }` | `src-tauri/src/export/markdown.rs:80-86` |
| `pub fn render(&ExportDocument) -> String` — 블록을 `\n\n`으로 잇고 끝에 개행 하나 | `src-tauri/src/export/markdown.rs:108-136` |
| `pub fn format_timestamp_ms(i64) -> String` — `00:00:03` · 음수는 0 · 100시간은 세 자리 | `src-tauri/src/export/markdown.rs:145-154` |
| `transcript_blocks` · `heading_text` · `date_text` · `single_line`는 **private** (그래서 §5.4가 `pub(crate)` 승격을 결정한다) | `markdown.rs:218-244` · `250-257` · `264-274` · `277-279` |
| `MEETING_SECTIONS` · `STUDY_SECTIONS` · `SUMMARY_SECTIONS` · `TRANSCRIPT_SECTION` (§9.5의 이름·순서) | `markdown.rs:52` · `61` · `71` · `74` |
| §11 문서 전체를 문자열로 고정하는 golden 테스트가 이미 있다 | `markdown.rs:373-410` · `415-441` |
| `format_timestamp_ms` · `render` · `ExportDocument`가 `export` 모듈에서 re-export된다 | `src-tauri/src/export/mod.rs:44-47` |
| 세그먼트 순서 보존 · 빈 항목 제거 · raw_text 대체 · 빈 섹션 미생성 규칙이 한 자리에 있다 | `markdown.rs:218-244` |

## 2. `ai::prompt` — 고치면 무엇이 깨지는가 (§6)

| 사실 | 자리 |
| --- | --- |
| `TRANSCRIPT_PLACEHOLDER = "{{transcript}}"` — 실행 중 조립되는 유일한 자리 | `src-tauri/src/ai/prompt.rs:48` |
| 세 프롬프트 상수 | `prompt.rs:51-73`(MEETING) · `76-99`(STUDY) · `102-120`(SUMMARY) |
| **세 상수에 글자까지 같은 JSON 출력 계약 블록**이 한 번씩 있다 — `"Return exactly one JSON object and nothing else. No prose before or after it, no code fence,\nno explanation."` | `prompt.rs:54-55` · `79-80` · `105-106` |
| 프롬프트가 키를 나열하는 순서가 `*_SECTIONS`의 순서와 i번째끼리 1:1 (overview→Overview, keyDiscussions→Key Discussions, …) | `prompt.rs:57-63` ↔ `markdown.rs:52-58` (Study·Summary도 같다) |
| `PROMPT_VERSION_MEETING/STUDY/SUMMARY`는 **손으로 선언한 값**이다 | `prompt.rs:126` · `129` · `132` |
| 선언값 ≠ 계산값이면 테스트가 깨진다 — "선언값을 계산값으로 고친다" | `prompt.rs:334-351` (`prompt_version_is_bound_to_the_prompt_text`) |
| 한 글자만 바뀌어도 버전이 달라진다 | `prompt.rs:354-371` |
| transcript 자리는 프롬프트마다 정확히 하나 | `prompt.rs:418-427` |
| 저장된 provenance가 이 값을 가리킨다 (`ai_notes.prompt_version`) | `prompt.rs:190-191` · PRODUCT-SPEC §7.3 · §9.6 |

→ 그러므로 상수를 고치면 (a) test Gate가 깨지고 (b) 선언값을 따라 올리면 **이미 저장된
provenance가 가리키는 프롬프트가 저장소에서 사라진다.** ADR-0010 §6.1의 근거가 이것이다.

## 3. current Transcript와 저장소 쓰기 부재 (§8.2 · MH-5 · MH-7)

| 사실 | 자리 |
| --- | --- |
| export는 `recording.current_transcript_id`가 가리키는 Transcript만 고른다 — 다른 version을 추측하지 않는다 | `src-tauri/src/export/run.rs:70-77` |
| export 경로에 저장소 **쓰기**가 없다 (`load_*` · `list_*`뿐) | `src-tauri/src/export/run.rs:20-30` (모듈 주석) |
| `ensure_exports_dir` → `export::run::export` → `ExportedFilePayload` 순서 | `src-tauri/src/commands/export.rs:94-101` |
| `export_file_name(created_at, title)` · `MAX_SLUG_BYTES = 80` · `MARKDOWN_EXTENSION` | `src-tauri/src/export/filename.rs:78-82` · `40` · `54` |
| 같은 이름이 있으면 덮어쓰지 않고 번호를 붙인다 | `src-tauri/src/export/file.rs` (ADR-0009 §4.3) |

## 4. command 표면과 tripwire (§8.1 · §8.3)

| 사실 | 자리 |
| --- | --- |
| `REGISTERED_COMMANDS`는 **28개**이고 검사는 부분집합이 아니라 **정확히 같은 집합**을 요구한다 | `tests/ipc-boundary.test.ts:70-99` · `138-140` |
| `outOfScope` 정규식에 `export(?!_markdown\b)`가 있다 | `tests/ipc-boundary.test.ts:162-163` |
| `'Markdown export 표면은 … 이름 하나뿐이다'`가 `['export_markdown']`을 기대한다 | `tests/ipc-boundary.test.ts:218-224` |
| `'전사 표면은 … 셋뿐이다'`가 `/transcri/i` 필터로 세 이름을 기대한다 | `tests/ipc-boundary.test.ts:170-181` |
| `'AI 표면은 … 다섯뿐이다'`의 필터는 `/ai_note|ai_provider/`다 → `get_ai_prompt`는 걸리지 않는다 | `tests/ipc-boundary.test.ts:194-206` |
| frontend가 부르는 이름과 등록 이름이 정확히 같아야 한다 | `tests/ipc-boundary.test.ts:252-258` |
| `src/`에서 `invoke`를 부르는 곳은 `src/ipc/commands.ts` 하나뿐이다 | `tests/ipc-boundary.test.ts:129-134` |
| `src/`에 `fetch`·XHR·WebSocket·EventSource가 없다 | `tests/ipc-boundary.test.ts:333-345` |

## 5. 실패 타입 (§7.2)

| 사실 | 자리 |
| --- | --- |
| Rust `FailureKind` 19종 → frontend union은 그 19종 + `unexpected` | `src-tauri/src/domain/failure.rs:143-161` · `src/ipc/failure.ts:52-72` |
| `unexpected`는 **frontend 경계에서만** 만들어진다 (`toFailure`) | `src/ipc/failure.ts:104-118` |
| 검사가 크기까지 본다 — `declared.length === kinds.size + 1` | `tests/ipc-boundary.test.ts:375-395` |

→ 그러므로 `clipboard`라는 종류를 union에 더하면 이 검사가 깨진다. ADR-0010 §7.2가
새 `FailureKind`를 만들지 않기로 한 근거가 이것이다.

## 6. clipboard는 저장소에 존재하지 않는다 (§7.1)

```text
grep -rn 'clipboard' src-tauri/Cargo.lock package-lock.json src-tauri/tauri.conf.json
  → 0건 (출력 없음)

ls node_modules/@tauri-apps
  → api  cli  cli-darwin-arm64  plugin-opener      (clipboard 플러그인 없음)

src-tauri/capabilities/default.json
  → "permissions": ["core:default"]                (clipboard 권한 없음)

src-tauri/src/platform/
  → app_data_dir.rs · clock.rs · microphone.rs · secret_store.rs   (clipboard 없음)

src/
  → platform 디렉터리가 없다 (screens · ipc · navigation뿐)
```

## 7. UNVERIFIED — 이 Run이 확인하지 **못한** 것

ADR-0010 §7.4에 [E4]로 적혀 있다. 여기서 다시 요약한다. **확인된 사실로 쓰지 않았다.**

```text
Tauri v2 webview에서 navigator.clipboard.writeText가 이 앱의 origin에서 동작하는지
그 API가 요구하는 조건(secure context · 사용자 제스처)이 이 앱의 창에서 어떻게 판정되는지
실패 시 어떤 거절 값이 오는지
Tauri v2 core가 clipboard를 노출하는지, 플러그인이 필요한지
clipboard 플러그인의 crate/npm 이름 · 호환 버전 · permission 식별자
Windows에서의 동작 전반 (Phase 6 이전)
```

이 Run은 네트워크 조회를 하지 않았고, 앱을 실행하지도 않았다. 위 여섯은 실행 환경이 있어야
확인되며, P11(TASK-065)이 그 시점의 사실을 ADR-0010 §12에 적는다.
