# TASK-106 Evidence: capture_warning integration tests

## Self-check result

```
Self-check (advisory)
Gates: build, test

build: PASS  exit=0  1.8s
test:  PASS  exit=0  19.3s

Self-check: all gates passed
```

## Test file

`src-tauri/tests/capture_warning.rs`

## Four scenarios

| Scenario | Test name | What it verifies |
|---|---|---|
| (a) 큐 넘침 | `queue_overflow_saves_a_record_with_capture_warning` | SampleSource interrupted에 Full 실패 → 레코드 저장 + capture_warning 채워짐 (DB 재조회) |
| (b) 스트림 오류 | `stream_error_saves_a_record_with_capture_warning` | SampleSource interrupted에 스트림 오류 → 레코드 저장 + capture_warning 채워짐 (DB 재조회) |
| (c) 정상 | `clean_stop_saves_a_record_without_capture_warning` | 아무 사건 없이 stop → 레코드 저장 + capture_warning이 None (DB 재조회) |
| (d) 파일 확정 실패 | `writer_failure_leaves_no_record_and_reports_the_file_path` | 빈 파일(WAV 헤더만) → 레코드 없음 + Failure에 파일 경로 포함 |

## All four use fake SampleSource (AC4)

- `ControlledMicrophone` — 테스트가 직접 샘플을 보내는 가짜. `interrupted` 필드로 stop 시 실패를 정함
- `WriterFailMicrophone` — 아무 샘플도 보내지 않아 파일이 MIN_FINALIZED_BYTES 미달로 실패하는 가짜
- 실제 마이크, 마이크 권한, 하드웨어 접근 없음
