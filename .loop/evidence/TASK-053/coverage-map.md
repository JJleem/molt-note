# TASK-053 — Phase 5 불변 ↔ 테스트 대응표

새로 만든 파일은 하나다: `src-tauri/tests/notion_and_export_invariants.rs` (8 test).
**제품 코드는 한 줄도 바뀌지 않았다** — 이 Task가 만든 변경은 테스트 파일 하나뿐이다
(`diff-stat.txt`).

모든 검사는 이미 있는 경로를 그대로 지난다: 문서를 만드는 자리는 `export::markdown::render`,
파일로 나가는 자리는 `commands::Exporter` · `export::run`, Notion으로 나가는 자리는
`sync::run::send`와 `commands::NotionSender`, 나누는 자리는 `notion::split_markdown`.

---

## (a) AI Note가 하나도 없는 Recording의 Markdown export와 Notion 전송이 성공한다 (INV-8 · §17.1)

| | |
| --- | --- |
| 새 검사 | `notion_and_export_invariants.rs::a_recording_with_no_ai_note_at_all_goes_out_both_doors_as_one_and_the_same_document` |
| 무엇이 새로운가 | 노트가 **하나도 없는 같은 Recording**이 두 출구를 동시에 지나고, 파일 본문 · Notion으로 나간 문서 · 렌더러 산출물 셋이 서로 **바이트까지 같다** (ADR-0009 §14). 세 AI 섹션 이름(§9.5)이 문서에 하나도 남지 않는 것까지 본다. |
| 이미 있는 검사 (다시 쓰지 않음) | 파일 쪽: `tests/markdown_export.rs::a_recording_with_no_ai_note_at_all_is_exported_as_a_valid_document` · Notion 쪽: `tests/notion_sync.rs::a_recording_with_no_ai_note_at_all_is_sent_with_just_its_transcript` — 각각 **출구 하나씩**을 판정한다. |

## (b) 1시간 규모 transcript가 분할되어 순차 전송되고, 이어 붙이면 원문과 정확히 같다

| | |
| --- | --- |
| 새 검사 | `notion_and_export_invariants.rs::an_hour_long_transcript_is_sent_in_pieces_that_reassemble_into_the_original_byte_for_byte` |
| 입력 규모 | `ONE_HOUR_MS / SEGMENT_MS = 1,200` segment. 테스트가 먼저 **문서가 `3 × CHUNK_MAX_BYTES`보다 크고 chunk가 4개보다 많다**는 것을 사전 조건으로 고정한다 — 한 요청에 들어가는 문서로는 아무것도 검사하지 못하기 때문이다. |
| 무엇을 비교하는가 | stub transport가 받은 **모든 요청 본문을 순서대로 꺼내 이어 붙인 문자열을 원문 전체와 비교한다** (`arrived.concat() == expected`). 길이나 앞뒤 몇 글자가 아니다. 조각별로도 `i`번째 요청 == `i`번째 chunk를 확인하고, transcript 1,200줄이 전부 나갔는지 표시를 세어 확인한다. |
| 순서 | 첫 요청만 `POST`(페이지 생성)이고 나머지는 전부 그 페이지에 대한 `PATCH`이며, 페이지 생성은 정확히 한 번이다. |
| 이미 있는 검사 (다시 쓰지 않음) | `tests/notion_chunking.rs::an_hour_long_transcript_rejoins_byte_for_byte` — **나누는 함수**의 무손실성. 여기서 보는 것은 그 뒤, 나눈 것이 실제로 요청이 되는 구간이다. |
| 실제로 실패할 수 있는가 | 확인했다 — `mutation-check.md` |

## (c) 실패 뒤에도 recording · transcript · ai_note가 그대로이고 재시도가 가능하다 (INV-3)

| | |
| --- | --- |
| 새 검사 | `notion_and_export_invariants.rs::every_way_a_send_can_fail_leaves_the_local_data_untouched_and_still_finishes_on_a_retry` |
| 네 가지 실패 | 인증 실패(`401 unauthorized`) · 권한 없는 destination(`404 object_not_found`) · 네트워크 없음(`TransportError::NotConnected`) · **중간 chunk 실패**(페이지 생성 뒤 `500`) |
| 무엇이 새로운가 | ① **중간 chunk 실패**를 같은 잣대(실패 전후 snapshot 동등성)로 본다 — 페이지가 이미 만들어지고 일부가 이미 Notion에 있는 상태다. ② 네 경우 모두 **재시도가 실제로 `done`까지 간다**: 이어 보낼 수 있는 경우는 확인 없이 이어 붙여 두 시도를 합치면 원문 하나가 되고, 결과를 모르는 경우는 `needsConfirmation=outcomeUnknown` 뒤 확인을 받아 끝난다. |
| 이미 있는 검사 (다시 쓰지 않음) | `tests/notion_sync.rs::nothing_local_is_lost_or_changed_when_a_send_fails` (페이지 생성 **전에** 갈리는 세 경우의 데이터 보존) · `tests/notion_sync.rs::a_retry_after_a_failed_second_chunk_continues_on_the_same_page_and_makes_no_duplicate` (중복 페이지 없음) |

## (d) 오디오 바이트도 오디오 경로도 Notion 요청 본문에 들어가지 않는다 (INV-6)

| | |
| --- | --- |
| 새 검사 (행동) | `notion_and_export_invariants.rs::an_audio_file_that_really_exists_reaches_neither_the_file_nor_the_request` |
| 무엇이 새로운가 | 오디오 파일을 **실제로 만들어 두고** (RIFF 헤더 + 고유 표시 + 4KB 바이트) 두 출구를 지난다. 어떤 요청 본문 · 주소에도, 내보낸 문서에도 그 표시 · `RIFF` · 파일 이름 · 경로 · 열 이름이 없다. 전송 뒤 오디오 파일 바이트가 그대로다. 그리고 **오디오를 지운 뒤에도 두 출구가 성립한다** — 읽는 코드가 있었다면 여기서 드러난다. |
| 새 검사 (소스) | `notion_and_export_invariants.rs::nothing_in_the_notion_boundary_can_open_or_read_a_file` — `src/notion/**` · `src/sync/**`의 제품 코드(주석과 `#[cfg(test)]` 제외)에 `std::fs` · `fs::` · `File::` · `OpenOptions` · `include_bytes!` · `read_dir` · `audio` · `PathBuf`가 하나도 없다. |
| 이미 있는 검사 (다시 쓰지 않음) | `tests/notion_sync.rs::no_request_ever_carries_the_audio_bytes_or_even_the_path_to_them` (파일이 **없는** 상태의 같은 관찰) · `tests/markdown_export.rs::exporting_reads_no_audio_and_copies_none` |

## (e) token이 SQLite 스키마 · frontend 소스 · 실패 message/detail 어디에도 없다 (INV-7)

| | |
| --- | --- |
| 새 검사 | `notion_and_export_invariants.rs::a_token_that_actually_sent_a_recording_is_nowhere_in_the_database_or_in_any_failure` |
| 무엇이 새로운가 | 메모리 double에 담긴 token으로 **product 경로(`commands::NotionSender`)가 실제로 전송을 성공시키고 실패시킨 뒤**에 본다: ① `sqlite_master` + `PRAGMA table_info`로 읽은 **실제로 만들어진 스키마의 모든 열 이름**에 `token`/`secret`/`api_key`/`password`/`credential`이 없다. ② 앱 데이터 디렉터리 **모든 파일의 바이트**(DB 본체 · WAL · export 포함)에 그 값이 없다. ③ 사용자가 보는 실패의 `message` · `detail`, 저장된 `notion_syncs.error`에도 없다. ④ 그 값은 여전히 자격증명 저장소 double 안에만 있다. |
| 이미 있는 검사 (다시 쓰지 않음) | frontend 소스: `tests/screen-boundary.test.ts`의 "token은 화면에 남지 않는다 (INV-7)" 4건 (브라우저 저장소 없음 · 입력란이 상태를 갖지 않음 · 되읽는 조회 경로 없음 · 순수 view가 값을 들지 않음) · `tests/ipc-boundary.test.ts`의 "자격증명은 이 경계를 한 방향으로만 지난다 (INV-7)" 3건 · 실패 값 자체: `tests/notion_adapter.rs::no_failure_on_any_path_carries_the_token_or_the_destination` · 자격증명 경계: `tests/secret_store.rs` 전체 · migration **SQL 문자열**: `src/db/migrations.rs::no_migration_creates_a_place_to_put_a_secret` |

## (f) 두 renderer가 provider 중립 Structured Note를 소비한다 (INV-9)

| | |
| --- | --- |
| 새 검사 (행동) | `notion_and_export_invariants.rs::both_renderers_consume_the_neutral_note_and_never_the_vendors_own_shape` |
| 무엇이 새로운가 | 세 mode(Meeting · Study · Summary) × 세 provider(`claude`/`gemini`/`ollama`와 각 모델 이름)로 같은 노트를 저장하고, **provider가 무엇이든 산출물이 같은지** 본다. 매번 파일과 Notion 문서가 서로 같고, 세 provider의 문서가 서로 같으며, 벤더 이름·모델 이름이 산출물에 나타나지 않고, 섹션 제목이 §9.5의 상수 그대로다. |
| 새 검사 (소스) | `notion_and_export_invariants.rs::no_source_on_the_way_out_knows_an_ai_vendor` — `src/export/**` · `src/notion/**` · `src/sync/**`의 제품 코드에 벤더 이름 9종이 하나도 없다. |
| 이미 있는 검사 (다시 쓰지 않음) | `tests/domain_invariants.rs::the_domain_does_not_know_any_ai_vendor` · `::the_ai_note_provider_is_a_free_form_identifier_not_a_vendor_enum` (domain · store · migration) · `tests/ipc-boundary.test.ts`의 "wire 계약에 벤더가 없다 (INV-9)" · `src/export/markdown.rs::the_renderer_consumes_the_provider_neutral_note_of_section_9_3` |

---

## §18 — 무엇을 쓰지 않는가

| 실제 자원 | 이 파일이 그 자리에 세우는 것 |
| --- | --- |
| Notion API / 워크스페이스 | `notion::testing::StubServer` (소켓을 열지 않는 순수 값 transport). 실제 왕복 구현 `notion::network`는 이 파일에서 한 번도 만들어지지 않는다. |
| OS 자격증명 저장소 (Keychain) | `platform::secret_store::testing::InMemorySecretStore`. `OsSecretStore`는 세우지 않는다 — `tests/secret_store.rs::no_automated_test_stands_up_the_real_credential_store`가 **모든 테스트 소스**에 대해 그 규칙을 검사하며, 이 파일도 그 검사 대상에 포함된다. |
| 사용자 디렉터리 | `std::env::temp_dir()` 아래의 고유 루트 하나 (`TempRoot`, Drop 때 삭제). DB · export · 오디오가 전부 그 아래에 있다. |
| 실제 대기(sleep) | `sync::pace::testing::RecordedWaits` — 기록만 한다. |
