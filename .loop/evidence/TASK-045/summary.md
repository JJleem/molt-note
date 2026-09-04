# TASK-045 — Markdown 파일 쓰기 경계와 export command

작성: Worker (RUN-20260904T034150Z-TASK-045) · 2026-09-04

이 문서는 **무엇을 만들었고, 각 Acceptance Criterion이 어떤 검사로 판정되는가**만 적는다.
완료 판정은 Runtime과 Verifier의 몫이다.

---

## 1. 만든 것

```text
platform/app_data_dir.rs   exports_dir() · ensure_exports_dir()      (ADR-0009 §4.1 · INV-10)
export/file.rs             (디렉터리 · 이름 · 문자열) → 쓰인 파일     (ADR-0009 §4.3 · 덮어쓰지 않는다)
export/run.rs              저장소를 읽어 P2 렌더러를 파일로 잇는 순서 (§7.2 · INV-8 · INV-3 · INV-6)
commands/export.rs         Exporter (앱이 들고 있는 실행자)
commands/payload.rs        ExportedFilePayload { recordingId · path · fileName }
commands/mod.rs            export_markdown command
lib.rs                     Exporter 등록 + command 등록
src/ipc/types.ts           ExportedFile
src/ipc/commands.ts        exportMarkdown(recordingId)
tests/ipc-boundary.test.ts 등록 목록에 export_markdown 추가 · export 표면이 하나임을 고정
tests/markdown_export.rs   통합 테스트 12개 (전부 임시 디렉터리)
```

실행 순서는 Task가 요구한 그대로다.

```text
Recording 읽기 → current Transcript(§7.2) 읽기 → (있으면) 최신 AI Note 읽기
      → markdown::render → filename::export_file_name → file::write_new → 쓰인 경로 반환
```

## 2. 지킨 불변식과 그것을 지키는 방식

| 불변식 | 구조적 근거 (테스트만이 아니다) |
| --- | --- |
| INV-8 — AI 없이도 export | `export/run.rs`와 `commands/export.rs`에 provider·AI 설정에 대한 참조가 **없다.** 노트는 `Option<StructuredNote>` 선택 입력이며, 거절할 수단 자체가 없다 |
| INV-3 — 실패가 원본을 훼손하지 않는다 | `export/run.rs`가 `store`에서 부르는 것이 전부 `load_*`·`list_*`다. 쓰기 호출이 하나도 없으므로 어떤 실패 경로도 recording · transcript · ai_note를 고치거나 지울 수 없다 |
| INV-6 — 오디오를 복사하지도 읽지도 않는다 | 파일시스템에 닿는 자리는 `file::write_new` 하나이고, 그 함수가 받는 것은 Markdown 문자열뿐이다 |
| ADR-0009 §4.3 — 덮어쓰지 않는다 | `create_new`(이미 있으면 실패)로만 파일을 만든다. 기존 파일은 **열리지도 않는다.** 1000번까지 비켜 보고 못 찾으면 `FailureKind::Storage` |
| INV-10 — 경로를 아는 자리는 하나 | export 위치는 `AppDataDirectory::exports_dir()`에서만 나온다 |
| §13 — 실패는 세 질문에 답한다 | 디렉터리 실패·쓰기 실패·대상 없음·전사 없음 전부 `crate::domain::Failure` (kind · message · detail · sourceDataSafe · retryable) |

## 3. 이 Task가 내린 결정 하나 (ADR-0009가 정하지 않은 자리)

**current Transcript가 없는 Recording의 export는 거절한다** (`FailureKind::InvalidInput`,
retryable). ADR-0009는 이 경우를 다루지 않는다. 제목과 길이만 담긴 문서를 만들면 "export가
잘못됐다"처럼 보이는 파일이 사용자의 export 디렉터리에 실제로 쌓이므로, 빈 산출물 대신
무엇이 필요한지 말하는 쪽을 택했다. **INV-8과 충돌하지 않는다** — INV-8이 말하는 것은 AI이며,
AI Note가 없는 export는 성공한다 (AC5).

## 4. Acceptance Criteria → 판정 수단

| AC | 판정 |
| --- | --- |
| P3-AC1 build | `npm run build` exit=0 — `evidence/gate-results.md` |
| P3-AC2 lint | `npm run lint` (eslint + `cargo clippy --all-targets -- -D warnings`) exit=0 |
| P3-AC3 test | `npm run test` (vitest + cargo test) exit=0 · 306 web + 674 rust 전부 통과 |
| P3-AC4 파일·이름·내용 | `tests/markdown_export.rs::a_recording_becomes_a_markdown_file_whose_name_and_body_follow_the_spec` — 이름 `2026-09-01-3dgs-study-04.md`, 경로 `exports/` 아래, **문서 전체 문자열을 고정**한다. 이름의 안전성은 `a_title_full_of_dangerous_characters_still_lands_in_the_exports_directory` |
| P3-AC5 AI Note 없이 (INV-8) | `a_recording_with_no_ai_note_at_all_is_exported_as_a_valid_document` — 성공하고, 문서 전체가 고정되며, 빈 AI 섹션이 남지 않는 것까지 본다 |
| P3-AC6 실패와 INV-3 | `a_directory_that_cannot_be_created_fails_visibly_and_changes_nothing`(자리를 못 만듦) · `a_write_that_fails_is_a_domain_failure_that_leaves_the_stored_data_untouched`(쓰기 실패) — 둘 다 실패 전후의 recording · transcript · ai_note를 **한 값으로 찍어 비교**한다. `a_recording_that_is_not_there_is_refused_without_creating_a_file` · `a_recording_without_a_transcript_is_refused_instead_of_leaving_an_empty_document`도 같은 비교를 한다 |
| P3-AC7 이름 충돌 | `exporting_the_same_recording_again_never_overwrites_the_earlier_file` — 사용자가 손댄 첫 파일이 그대로 남고 `-2` · `-3`가 생긴다. `two_recordings_with_the_same_title_and_date_do_not_collapse_into_one_file` · unit test `export::file::tests::a_name_that_is_taken_is_stepped_over_instead_of_overwritten` · `running_out_of_names_is_a_visible_failure_rather_than_an_overwrite` |

## 5. AC4가 고정한 문서 (테스트가 문자열 전체를 비교한다)

```markdown
# 3DGS Study #04

Date: 2026-09-01
Duration: 52:31

## Short Summary
세 줄 요약

## Key Points
- 첫 번째
- 두 번째

## Transcript

### 00:00:03
첫 문장

### 00:01:05
둘째 문장
```

## 6. 범위 밖으로 두고 손대지 않은 것

- Recording Detail의 Export 버튼 · UI 통합 (다음 Task)
- Notion 전송 · SecretStore · ureq TLS (같은 Phase의 다른 Task)
- export 위치를 설정으로 노출하는 것 (ADR-0009 §4.1이 거절했다)
- `docs/SYSTEM-MAP.md` · ADR §15 (Phase가 DONE에 도달한 뒤의 일이다)
