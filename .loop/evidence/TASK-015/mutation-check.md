# AC4 (3) — pause 구간을 포함하도록 구현을 바꾸면 테스트가 실패한다

Run: RUN-20260902T081554Z-TASK-015 · Task: TASK-015 · 2026-09-02

AC4의 세 번째 항목은 "pause 구간이 duration에서 빠지는 것을 검증하는 테스트가 있고,
pause를 포함하도록 구현이 바뀌면 그 테스트가 실패하게 되어 있다"이다.
주장 대신 **실제로 구현을 바꿔 보고 Gate가 빨개지는 것을 확인했다.**

## 가한 변경 (일시적 · 확인 후 되돌렸다)

`src-tauri/src/audio/session.rs`의 `RecordingSession::pause`에서 구간을 닫는 두 줄을 제거해
일시정지 구간이 계속 자라도록(= duration에 포함되도록) 만들었다.

```diff
     pub fn pause(&mut self, at_ms: i64) -> Result<(), Failure> {
         if self.state != SessionState::Recording {
             return Err(self.rejected("녹음 중이 아니어서 일시정지할 수 없다.", "pause"));
         }

-        self.accumulated_ms = self.accumulated_ms.saturating_add(self.open_segment_ms(at_ms));
-        self.segment_started_at_ms = None;
+        let _ = at_ms; // MUTATION EXPERIMENT — pause 구간을 duration에 포함시킨다
         self.state = SessionState::Paused;
         Ok(())
     }
```

## 결과 — `node tools/loop-runtime/loopctl.mjs self-check test`

```text
test: FAIL  exit=1
test result: FAILED. 81 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out

failures:
    audio::session::tests::many_pause_and_resume_cycles_keep_accumulating_only_the_recorded_spans
    audio::session::tests::the_paused_span_is_not_counted_in_the_duration
    audio::session::tests::the_whole_lifecycle_walks_idle_recording_paused_recording_stopped
```

실패한 단언 원문:

```text
---- audio::session::tests::the_paused_span_is_not_counted_in_the_duration stdout ----
panicked at src/audio/session.rs:384:9:
assertion `left == right` failed: 멈춰 있는 동안은 자라지 않는다
  left: 99000
 right: 3000

---- audio::session::tests::the_whole_lifecycle_walks_idle_recording_paused_recording_stopped stdout ----
panicked at src/audio/session.rs:295:9:
assertion `left == right` failed
  left: 2000
 right: 7000

---- audio::session::tests::many_pause_and_resume_cycles_keep_accumulating_only_the_recorded_spans stdout ----
panicked at src/audio/session.rs:414:9:
assertion `left == right` failed
  left: 30000
 right: 60000
```

(단위 테스트에서 실패했으므로 `tests/recording_session.rs`의
`the_paused_span_stays_out_of_the_duration_however_long_it_lasts`는 이 실행에서 실행되기 전에
멈췄다. 그 테스트도 같은 성질을 crate 공개 API 쪽에서 한 번 더 본다.)

## 되돌린 뒤

변경을 원래대로 되돌리고 build · lint · test Gate를 다시 돌려 전부 PASS를 확인했다
(`gates.md`). 현재 저장소에는 이 실험의 흔적이 남아 있지 않다.
