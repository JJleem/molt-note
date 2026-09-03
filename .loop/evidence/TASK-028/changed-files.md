# TASK-028 — 변경한 파일

`git status --porcelain` 기준. 아래 목록 밖의 dirty 항목(`Cargo.toml` · `Cargo.lock` ·
`domain/failure.rs` · `platform/app_data_dir.rs` · `src/ipc/failure.ts` · `docs/ADR-0007-*` ·
`src-tauri/tests/transcription_{engine,run}.rs`)은 이 Task 이전(TASK-023~027)의 것이며
이 Task가 건드리지 않았다.

## 새로 만든 파일

| 파일 | 무엇 |
| --- | --- |
| `src-tauri/src/commands/transcriber.rs` | 진행 중인 전사를 소유하는 Tauri managed state (`Transcriber`) · 배경 스레드 · 상태 한 값 |
| `src-tauri/tests/transcription_background.rs` | 동시성 계약 통합 테스트 10건 (멈춰 있는 test double 엔진) |

## 고친 파일

| 파일 | 무엇 |
| --- | --- |
| `src-tauri/src/commands/mod.rs` | `pub mod transcriber` · `pub use Transcriber` · command 둘(`start_transcription` · `transcription_status`) · 모듈 문서 |
| `src-tauri/src/commands/payload.rs` | `TranscriptionStatusPayload`(wire 계약)와 네 상태 생성자 |
| `src-tauri/src/lib.rs` | `app.manage(Transcriber::open_for(app))` · `generate_handler!`에 command 둘 등록 |
| `src-tauri/src/db/mod.rs` | 연결마다 `busy_timeout` 설정(전사 연결과 앱 연결이 겹칠 때 즉시 실패하지 않는다) + 그 테스트 |
| `src-tauri/src/transcription/mod.rs` | 모듈 문서 — "배경 실행과 command는 아직 없다"가 더는 사실이 아니다 |
| `src-tauri/src/transcription/run.rs` | 모듈 문서 — 스레드를 만드는 자리가 어디인지 가리킨다 |
| `src/ipc/types.ts` | `TranscriptionState` · `TranscriptionStatus` (payload와 1:1) |
| `src/ipc/commands.ts` | 타입 있는 client `startTranscription` · `transcriptionStatus` |
| `tests/ipc-boundary.test.ts` | 등록 command 집합에 둘 추가 · **전사 표면이 그 둘뿐인지 검사하는 테스트 추가** |

## 하지 않은 것

- Task 상태 · acceptance criteria · `.loop/` 설정 수정 없음 (Evidence 디렉터리에만 썼다)
- 테스트 삭제 · skip · 약화 없음. `tests/ipc-boundary.test.ts`의 out-of-scope 정규식에서
  `transcri`를 뺀 것은 이 Phase가 전사 표면을 여는 Phase이기 때문이며, 대신
  `queue|batch|schedule`을 추가하고 **전사 command가 정확히 둘인지 확인하는 검사를 새로 넣어**
  범위 검사를 더 좁혔다.
- 여러 Recording 동시 전사 큐 없음 (§16 DEFERRED) · 취소 command 없음 · 자동 전사 트리거 없음
  (TASK-029) · Transcript 화면 없음 (TASK-030)
