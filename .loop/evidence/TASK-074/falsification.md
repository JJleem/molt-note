# TASK-074 — Falsification: 되돌리면 실제로 실패하는가

AC4는 "해당 코드를 되돌렸을 때 그 검사가 실패할 것인지"를 묻는다. 읽고 판단하는 대신 **실제로
되돌려 돌렸다.** 아래는 관측된 사실이며, 확인하지 못한 것은 확인하지 못했다고 적었다.

되돌린 뒤에는 전부 원래대로 복구했고, `git diff --numstat`이 되돌리기 전 값과 정확히 일치하는
것으로 확인했다 (`db/settings.rs 20/4` · `export/handoff.rs 259/20` ·
`transcription/engine.rs 96/2` · `screens/exportView.ts 32/3`, 그리고 untracked인
`commands/saved_file.rs`는 해당 줄의 원문 재확인).

## 1. 화면 쪽 (TR-3) — VERIFIED

되돌린 것: `src/screens/exportView.ts`

```diff
-        show: showFile(attempt.file.path, show),
+        show: showFile('/somewhere/else.md', show),
```

관측된 결과 (`npm run test` 중 vitest):

```text
❯ tests/transcription-and-reach-invariants.test.ts (4 tests | 3 failed)
   × TR-3a  두 done 상태가 둘 다 여는 동작을 들고 있고, 그 대상은 backend가 준 경로다
   × TR-3b  실패한 뒤에도 두 자리에 다시 여는 동작과 전체 경로가 남는다
   × TR-3b  다른 파일을 열다 실패한 사실은 이 자리에 보이지 않는다
```

네 번째(TR-3c)는 그대로 통과했다 — 그것은 import 관계를 보는 원문 규칙이고 이 되돌리기는 그것을
건드리지 않았기 때문이다. **의도한 대로다**: 세 검사는 값을, 하나는 그 자리들의 목록을 지킨다.

## 2. Rust 쪽 (TR-1 · TR-2 · TR-3 · TR-4) — 되돌리기는 유효했으나, 통합 테스트 실행까지는 도달하지 못했다

되돌린 것 네 가지:

```diff
# TR-1  src-tauri/src/transcription/engine.rs
-            None => Self::Detect,
+            None => Self::Chosen("en".to_owned()),      # 2026-09-05에 무너진 그 상태

# TR-2  src-tauri/src/db/settings.rs  (UPDATE의 SET 절)
-                 notion_parent_page_id = excluded.notion_parent_page_id,
-                 transcription_language = excluded.transcription_language",
+                 notion_parent_page_id = excluded.notion_parent_page_id",

# TR-3  src-tauri/src/commands/saved_file.rs
-        if !file.starts_with(&exports_dir) || !file.is_file() {
+        if !file.is_file() {

# TR-4  src-tauri/src/export/handoff.rs
-    let count = portions.len().max(1);
+    let count = 1;
```

관측된 결과:

```text
test result: FAILED. 520 passed; 5 failed
  commands::saved_file::tests::a_path_outside_the_exports_directory_is_refused
  commands::saved_file::tests::a_symlink_that_points_outside_is_refused_too
  commands::saved_file::tests::walking_out_of_the_exports_directory_does_not_work
  export::handoff::tests::a_text_over_the_budget_comes_in_order_and_rejoins_into_the_original
  transcription::engine::tests::not_choosing_a_language_means_detection_rather_than_english
```

**관측된 사실**: 네 되돌리기 중 셋(TR-1 · TR-3 · TR-4)은 실제로 제품 동작을 바꿨고, 이미 있던
단위 테스트가 그것을 잡았다. TR-2의 되돌리기는 어떤 단위 테스트도 잡지 못했다 — 그 열을 저장
경로에서 빼는 일은 **지금까지 아무 검사도 보지 않던 자리**였다는 뜻이며, 그것이 이 Task가
`tr2_saving_a_different_setting_does_not_quietly_erase_the_chosen_language`를 쓴 이유다.

**확인하지 못한 것**: 이 되돌리기 아래에서
`tests/transcription_and_reach_invariants.rs`가 실제로 실패하는 것은 **관측하지 못했다.**
`cargo test`는 기본이 fail-fast이고, lib 단위 테스트 target이 먼저 실패하면서 통합 테스트
target이 실행되지 않았다. 이 Run에서 Worker가 쓸 수 있는 명령은 `loopctl self-check` 하나이므로
(`--no-fail-fast`를 붙이거나 target 하나만 돌릴 수 없다) 그 관측은 여기서 얻을 수 없었다.

그러므로 이 문서는 다음 둘을 구분해서 말한다.

```text
VERIFIED     되돌리기가 실제로 제품 동작을 바꾼다 (기존 단위 테스트 5건이 잡았다)
VERIFIED     화면 쪽 세 검사는 되돌리기에서 실제로 실패한다
UNVERIFIED   같은 되돌리기 아래에서 Rust 통합 검사가 실패하는 것 — 실행 자체에 도달하지 못했다
```

각 Rust 검사가 그 경로를 실제로 지난다는 근거는 실행 관측으로 남는다: 되돌리기 없이 돌린
`test` Gate가 PASS이며, 그 검사들은 stub 엔진이 **받은 호출**, 저장소에서 **다시 읽은 값**,
파일 관리자 double에 **도착한 경로**, 디스크에 **실제로 쓰인 조각 파일들**을 읽는다. 상수 비교나
파일 존재 확인으로 대체된 검사는 없다. 유일한 원문 검사 두 개
(`tr1_the_engine_boundary_never_leaves_the_library_default_in_place`,
TR-3c)는 각각 그 이유를 코드 주석에 적어 두었다 — 실제 whisper 추론과 "아직 없는 세 번째
export 표면"은 값으로 판정할 수 없는 대상이다.
