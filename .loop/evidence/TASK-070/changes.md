# TASK-070 변경 요약 — 전사에 걸린 시간이 기록으로 남는다

`phase-prompt/05.6` 성공 기준 3의 나머지 절반. 값 하나가 잰 자리에서 화면까지 도달하는 경로다.

```text
transcription/run.rs   Instant(단조 시계)로 1회 측정
      ↓ Transcript.transcription_ms: Option<i64>
db/migrations.rs       version 10  ALTER TABLE transcripts ADD COLUMN transcription_ms INTEGER
db/store.rs            append_transcript(INSERT) · load_transcript / list_transcripts(SELECT)
      ↓
commands/payload.rs    transcriptionMs + transcriptionLabel(= format_duration_ms, Rust가 만든다)
      ↓
src/ipc/types.ts       Transcript.transcriptionMs · transcriptionLabel  (둘 다 null 가능)
src/screens/transcriptView.ts        done 상태가 transcriptionLabel을 그대로 나른다
src/screens/RecordingDetailScreen.tsx  provenance 아래 줄. label이 null이면 **그 줄이 없다**
```

## 결정과 이유

- **이름**: `transcription_ms` / `transcriptionLabel`. `duration*` · `elapsed*`를 쓰지 않은 것은
  그 이름들이 이미 **녹음 길이**(§5 A의 `52:31`)와 **녹음 중 경과 시간**(§19)에 쓰이고 있고,
  `tests/screen-boundary.test.ts`가 그 두 이름으로 "길이 포맷은 Rust에만 있다"를 지키기 때문이다.
  같은 이름을 세 번째 의미로 재사용하면 그 검사가 무엇을 지키는지 말할 수 없게 된다.
- **재는 자리는 한 곳**: `transcription::run::attempt`의 `Instant::now()` 하나뿐이며, 테스트가
  원문에서 그 횟수를 센다. 재는 구간은 **모델 해석 → 오디오 읽기 → 엔진 → 정규화**까지이고
  영속화는 세지 않는다 — 저장이 느린 것은 전사가 느린 것이 아니다.
- **단조 시계**: 벽시계(`SystemTime`) 두 번을 빼면 시스템 시각 조정이 음수 소요 시간을 만든다.
  `Instant`는 뒤로 가지 않는다. 넘치는 값은 `i64::MAX`로 포화시킨다 — 오래 걸렸다는 이유로
  전사 결과를 잃지 않는다.
- **표현은 backend가 만든다**: `format_duration_ms`(`domain/duration.rs`) 하나를 그대로 쓴다.
  화면은 문장을 받아 그리기만 하며, 밀리초를 나누지 않는다. 새 포맷 규칙을 만들지 않았다.
- **없는 값은 없는 채로**: 열은 nullable이고 DEFAULT가 없다. payload는 `map`으로 문장을 만들지
  않은 채 보내고, 화면은 그 줄을 그리지 않는다. 재지 않은 전사를 `0:00`이라고 말하면 이 값으로
  하려던 Metal 전후 비교가 거짓을 말하기 때문이다.
- **이미 적용된 migration은 고치지 않았다**: version 2의 `transcripts`는 그대로 두고 목록 끝에
  version 10을 붙였다. Transcript는 immutable이므로(§7.1 · INV-2) 옛 행에 이 값을 나중에
  채워 넣는 경로도 만들지 않았다.

## 범위 밖 — 하지 않은 것

- 실시간 진행률 UI (Task 본문과 `phase-prompt/05.6`가 이 Phase의 성공 기준이 아니라고 명시).
- Metal 활성화 자체(별개 Task) · ADR 문서 갱신 · markdown/Notion 내보내기에 이 값 추가.

## 남은 사실 하나 (AC5 관련)

`src/screens/transcriptView.ts`에는 여전히 `formatTimestamp`가 있다. 이것은 **segment의 위치**
(`00:02:14 → 00:02:21`)를 만드는 기존 예외이며 이 Task 이전부터 있었고,
`tests/screen-boundary.test.ts`가 파일 경로 하나로 못 박아 관리한다. 이 Task는 그 예외를 늘리지
않았다 — 오히려 같은 검사에 `transcriptionMs`를 더해, 그 모듈이 전사 소요 시간 쪽으로 넘어오는
것까지 막았다.

## Evidence 파일에 대한 주의

`core-diff.patch`는 **HEAD 기준 diff**다. `run.rs`와 `payload.rs`는 이 Task 이전에 이미
Phase 5.6의 다른 Task가 손댄(커밋되지 않은) 상태였으므로, 그 patch에는 이 Task가 하지 않은
변경(전사 언어 관련)도 섞여 있다. 이 Task가 그 두 파일에 더한 것은 위 "변경 파일"에 적은
`Instant` 측정 · `elapsed_ms` · `transcription_ms` · `transcription_label`뿐이다.

## 변경 파일

```text
제품 코드
  src-tauri/src/domain/mod.rs                     Transcript.transcription_ms
  src-tauri/src/db/migrations.rs                  version 10 + 규약 테스트 2개
  src-tauri/src/db/store.rs                       TRANSCRIPT_COLUMNS · INSERT · 두 SELECT
  src-tauri/src/transcription/run.rs              Instant 측정 1곳 · elapsed_ms
  src-tauri/src/commands/payload.rs               transcription_ms · transcription_label
  src/ipc/types.ts                                Transcript 두 필드
  src/screens/transcriptView.ts                   done 상태의 transcriptionLabel
  src/screens/RecordingDetailScreen.tsx           null이면 보이지 않는 한 줄

테스트
  src-tauri/tests/recording_repository.rs         저장·재조회 + NULL 유지 (신규)
  src-tauri/tests/transcription_run.rs            측정 저장 · 측정 자리 1곳 (신규 2건)
  src-tauri/tests/command_boundary.rs             payload 문장 · 값 없음 (신규 1건 + 보강)
  src-tauri/tests/domain_model.rs                 transcripts 열 목록
  src-tauri/tests/settings_repository.rs          legacy fixture에 transcripts 테이블 추가
  src-tauri/tests/ai_failure_recovery.rs          INV-2 덤프에 새 열 포함
  src-tauri/tests/ai_note_run.rs                  INV-2 덤프에 새 열 포함
  src-tauri/src/export/{markdown,ai_request}.rs   fixture 필드
  src-tauri/tests/{ai_note_commands,audio_never_reaches_ai,manual_ai_handoff,
                   manual_handoff_invariants,markdown_export,notion_chunking,
                   notion_sync,notion_and_export_invariants}.rs   fixture 필드
  src/screens/{transcriptView,aiNoteView,aiHandoffView,coreWithoutAi}.test.ts  fixture + 신규 검사
  tests/screen-boundary.test.ts                   경계 검사 확장
```
