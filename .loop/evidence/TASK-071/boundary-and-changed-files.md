# TASK-071 — 무엇을 어디에 만들었는가

`phase-prompt/05.6` 성공 기준 2 · R-4: **방금 내보낸 파일에 사람이 도달할 수 있다.**
경로를 글자로 보여 주는 것에 더해, 그 자리를 여는 수단이 화면에 있다.

## 1. 흐름

```text
화면(RecordingDetailScreen)
  └ ShowFileControl  ← 그리기만 한다. 상태 판정은 savedFileView.ts
      └ showSavedFile(path)                     src/ipc/commands.ts
          └ show_saved_file                     src-tauri/src/commands/mod.rs
              └ SavedFiles::show                src-tauri/src/commands/saved_file.rs
                  ① exports_dir 정규화 (없으면 거절 · 만들지 않는다)
                  ② 요청 경로 정규화 (`..`·symlink가 여기서 풀린다)
                  ③ starts_with(exports_dir) && is_file()   ← 허용을 결정하는 단 한 곳
                  └ FileManager::show           src-tauri/src/platform/file_manager.rs
                      macOS   open -R           (UNVERIFIED — 이 Run은 실행하지 못했다)
                      Windows explorer /select, (UNVERIFIED — 검증은 Phase 6)
                      그 밖    FileManagerError::Unsupported → §13의 Failure
```

## 2. 경계가 지키는 것

- **OS를 부르는 코드는 `src-tauri/src/platform/file_manager.rs` 안에만 있다** (INV-10).
  `process::Command`도 `cfg(target_os)`도 그 파일 밖에 없다는 것을
  `src-tauri/tests/show_saved_file.rs`가 소스에서 확인한다. 화면 · domain · export 실행 순서
  어디에도 없다.
- **webview가 임의 경로를 열 수 없다.** 열리는 것은 `AppDataDirectory::exports_dir()` 아래에
  실제로 있는 **파일**뿐이다. 판정은 정규화된 경로 하나로만 하며, 글자 비교는 실패 문장을
  고르는 데만 쓴다. 거절되는 것: 앱 데이터 안의 다른 파일 · 완전히 밖의 경로 ·
  `exports/../…` 탈출 · exports 밖을 가리키는 symlink · 디렉터리 자신 · 빈 문자열 ·
  더 이상 없는 파일.
- **파일을 만들지도 고치지도 지우지도 않는다** (INV-3). `ensure_exports_dir`을 쓰지 않으므로
  한 번도 내보낸 적 없는 사용자에게 빈 디렉터리를 만들지도 않는다.
- **Tauri 플러그인도 새 capability 권한도 쓰지 않았다.** `src-tauri/capabilities/default.json`은
  `["core:default"]` 그대로이고 `Cargo.toml`에 새 의존성이 없다 — 표준 라이브러리의 프로세스
  실행 하나로 끝나므로 필요하지 않았다. 필요했다면 이름으로 켜고 이유를 적었을 자리다.
- **command 이름이 표면 규칙을 흐리지 않는다.** `show_saved_file`에는 `export`도 `play`도
  벤더 이름도 없고, 어느 OS의 파일 관리자인지도 말하지 않는다. `tests/ipc-boundary.test.ts`의
  `outOfScope`에 `finder`·`explorer`를 더해 그 사실을 규칙으로 만들었다.
- **경로 문자열은 그대로 남는다.** 여는 수단은 대체가 아니라 추가이며,
  `tests/screen-boundary.test.ts`가 두 export 자리 모두에서 경로 줄과 여는 수단이 함께 있다는
  것을 원문으로 확인한다.

## 3. 이 Task가 만든 파일

```text
src-tauri/src/platform/file_manager.rs      OS 파일 관리자 경계 (+ 실행하지 않는 실제 구현 · double)
src-tauri/src/commands/saved_file.rs        허용을 판정하는 command 소유자
src-tauri/tests/show_saved_file.rs          행동 2각도 + 소스 2각도
src/screens/savedFileView.ts                여는 동작과 실패 표현의 순수 규칙
src/screens/savedFileView.test.ts           그 규칙의 값 검증 (OS도 DOM도 없이)
```

## 4. 이 Task가 고친 파일

```text
src-tauri/src/lib.rs                        SavedFiles 관리 · show_saved_file 등록 (주석 포함)
src-tauri/src/commands/mod.rs               모듈 선언 · 재수출 · command 함수
src-tauri/src/platform/mod.rs               경계 목록에 file_manager.rs 추가
src/ipc/commands.ts                         showSavedFile · 헤더의 command 개수 (31 → 32)
src/screens/exportView.ts                   done 상태에 show 값 · 입력에 show 시도
src/screens/aiHandoffView.ts                done 상태에 show 값 · 입력에 show 시도
src/screens/RecordingDetailScreen.tsx       beginShowFile · ShowFileControl · 두 자리에서 호출
src/App.css                                 .share__show (여백 스케일 위)
src/screens/exportView.test.ts              여는 수단의 값 검증 · 입력 필드 목록 갱신(주석 포함)
src/screens/aiHandoffView.test.ts           같은 검증 · 입력 필드 목록 갱신(주석 포함)
tests/ipc-boundary.test.ts                  허용 목록 · outOfScope · export 표면 검사 (주석 포함)
tests/screen-boundary.test.ts               순수 모듈이 OS를 알지 않는다 · 경로가 사라지지 않았다
tests/manual-handoff-invariants.test.ts     ManualHandoffInput 필드 목록 갱신(주석 포함)
tests/ui-foundation.test.ts                 manualHandoff 호출에 show 값 전달
```
