# TASK-049 — Notion 전송 순서와 `NotionSync` 영속화

Run: `RUN-20260904T052652Z-TASK-049` · 2026-09-04

## 무엇을 만들었는가

```text
src-tauri/src/sync/mod.rs      새 모듈 — Notion 전송 순서의 경계 문서
src-tauri/src/sync/pace.rs     얼마나 기다리고 몇 번까지 다시 보내는가 (ADR-0009 §9.2) + Waiter 경계
src-tauri/src/sync/run.rs      실행 순서 하나 — 무엇을 언제 영속화하는지가 여기 있다 (§8.4)
src-tauri/src/commands/notion.rs  진행 중인 전송의 소유자 (`NotionSender`) — 배경 스레드 규약
src-tauri/tests/notion_sync.rs    통합 테스트 10개 (stub transport · 메모리 자격증명 · 임시 저장소)
```

바뀐 기존 파일:

```text
src-tauri/src/db/migrations.rs        migration 8 `add_notion_sync_progress` (ADR-0009 §8.4)
src-tauri/src/db/store.rs             save/load_notion_sync가 진행도 세 열을 함께 다룬다
src-tauri/src/domain/mod.rs           NotionSync + sent_chunks · total_chunks · content_fingerprint
src-tauri/src/export/run.rs           latest_note를 pub(crate)로 — 노트를 고르는 규칙을 한 벌로 유지
src-tauri/src/platform/secret_store.rs  app_secret_store() — 실제 구현을 세우는 자리를 경계 안에 둔다
src-tauri/src/lib.rs                  pub mod sync
src-tauri/src/commands/mod.rs         notion 모듈 마운트 + 재수출
src-tauri/Cargo.toml                  sha2 (content_fingerprint) — 이미 lock에 있던 전이 의존성
src-tauri/tests/domain_model.rs       notion_syncs의 새 열과 NotionSync 값 갱신
src-tauri/tests/settings_repository.rs  예전 스키마 fixture에 version 2의 notion_syncs를 더한다
```

## Gate

두 Gate 모두 로컬에서 직접 실행해 exit 0을 확인했다 (advisory — Runtime이 다시 돌린다).

```text
lint  npm run lint  (eslint . && cargo clippy --all-targets -- -D warnings)   PASS  exit=0
test  npm run test  (vitest run && cargo test)                                PASS  exit=0
```

- `gate-lint-stdout.log` · `gate-lint-stderr.log`
- `gate-test-stdout.log` · `gate-test-stderr.log`

`src-tauri/tests/notion_sync.rs`는 10 passed · 0 failed (test stdout 기준).

## Acceptance Criteria가 어디서 판정되는가

| AC | 판정 |
| --- | --- |
| P7-AC1 lint | `gate-lint-*.log` — exit 0 |
| P7-AC2 test | `gate-test-*.log` — exit 0 |
| P7-AC3 AI Note 없는 전송 (INV-8) | `tests/notion_sync.rs::a_recording_with_no_ai_note_at_all_is_sent_with_just_its_transcript` |
| P7-AC4 순차 전송 · 재시도가 중복을 만들지 않는다 | `…::a_long_document_goes_out_in_order_and_arrives_whole` · `…::a_retry_after_a_failed_second_chunk_continues_on_the_same_page_and_makes_no_duplicate` · `…::a_finished_recording_is_never_sent_again_without_being_asked_first` · `src/sync/run.rs`의 `plan` 단위 테스트 8개 |
| P7-AC5 실패 뒤 local data 온전 · status/error (INV-3 · §7) | `…::nothing_local_is_lost_or_changed_when_a_send_fails` (인증 실패 · 권한 없는 destination · 네트워크 없음 세 경로) |
| P7-AC6 오디오가 실리지 않는다 (INV-6) | `…::no_request_ever_carries_the_audio_bytes_or_even_the_path_to_them` |
| P7-AC7 배경 스레드 규약 | `…::asking_for_the_status_answers_immediately_while_a_send_is_still_running` · `src/commands/notion.rs` (Transcriber · NoteGenerator와 같은 구조) |

## ADR-0009 §8.4의 영속화 순서가 실제로 그 순서인가

`src/sync/run.rs`가 그대로 이행한다.

```text
2. 첫 요청 전에 행을 쓴다        send() 안의 persist(..., Running, ...) — transmit보다 먼저다
3. 페이지가 생기면 즉시 page_id  transmit()의 create_page 직후 persist — 다음 요청보다 먼저다
4. 성공한 요청만 센다            append 성공 뒤에만 progress.sent += 1
5. done + synced_at / failed + error
```

3번이 실제로 "다음 요청보다 먼저"라는 것은 `a_retry_after_a_failed_second_chunk…`가
관찰한다 — 두 번째 chunk에서 실패한 뒤 저장된 행에 `page_id`가 있고, 재시도가 그 페이지로
이어 붙이며 **새 페이지를 만드는 요청이 0건**이다.

## 확인하지 않은 것 / 하지 않은 것

- **실제 Notion API를 한 번도 호출하지 않았다.** 모든 왕복은 `notion::testing::StubServer`
  뒤에 있고, 실제 워크스페이스에 무엇이 만들어지는지는 이 Run이 확인할 수 없다
  (ADR-0009 §6.3의 "나눠 보낸 결과가 한 번에 보낸 것과 같은 블록 구조가 되는가"는 여전히
  UNVERIFIED이며 Human Review 항목이다).
- **실제 OS 자격증명 저장소를 열지 않았다.** token은 메모리 double 안에만 있었다.
- Tauri command 이름과 frontend 계약은 만들지 않았다 — TASK-050의 범위다. 이 Run이 만든
  표면은 실행 순서(`sync::run::send`)와 그 소유자(`commands::NotionSender`)까지다.
- `notion_syncs`가 `running`으로 남은 채 앱이 죽은 경우, ADR-0009 §8.5의 `running` 행 그대로
  전송을 거절한다. 그 상태를 푸는 경로는 이 Task가 만들지 않았다 (ADR의 결정 그대로다).

## 참고 — diff

`tracked-files.diff`는 git이 추적 중인 파일에 대한 diff다. **이 Run만의 변경이 아니다** —
Phase 5의 앞선 Task들(TASK-043~048)이 아직 커밋되지 않아 같은 파일의 변경이 함께 들어 있다.
이 Run이 만든 새 파일(`src/sync/**` · `src/commands/notion.rs` · `tests/notion_sync.rs`)은
추적되지 않은 파일이라 그 diff에 나타나지 않는다.
