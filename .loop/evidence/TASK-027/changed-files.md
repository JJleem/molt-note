# TASK-027 변경 파일

| 파일 | 상태 | 내용 |
| --- | --- | --- |
| `src-tauri/src/transcription/run.rs` | 신규 | 전사 orchestration — 상태 전이 · Transcript 추가 · current 갱신 · 실패 처리 |
| `src-tauri/src/transcription/mod.rs` | 수정 | `pub mod run;` 추가 · 재수출(`transcribe` · `Completed` · `ModelChoice`) · 모듈 지도와 "아직 없다" 문장 갱신 |
| `src-tauri/tests/transcription_run.rs` | 신규 | 통합 테스트 13건 (fake engine + 임시 DB + 임시 WAV) |

Task 범위 밖은 건드리지 않았다:

- Tauri command 표면 · `src-tauri/src/lib.rs` 등록 · frontend client → TASK-028
- 설정(`automatic_transcription` · 모델 설정) → TASK-029
- Recording Detail Transcript 탭 → TASK-030
- `db/store.rs` · `db/migrations.rs` · `domain/**` → 변경 없음. 기존 API
  (`store::update_recording_statuses` · `store::append_transcript` ·
  `store::set_current_transcript`)를 그대로 썼다.

## 공개 API 표면 (추가된 것 전부)

```rust
// src-tauri/src/transcription/run.rs
pub struct ModelChoice<'a> { pub models_dir: &'a Path, pub configured: Option<&'a str> }
pub struct Completed { pub transcript: Transcript, pub anomalies: Vec<Anomaly> }
pub fn transcribe(
    connection: &mut Connection,
    recording_id: &RecordingId,
    engine: &dyn TranscriptionEngine,
    model_choice: ModelChoice<'_>,
) -> Result<Completed, Failure>;
```
