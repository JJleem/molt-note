# TASK-014 — Acceptance Criteria 대응

변경한 파일은 `docs/ADR-0003-recording-engine.md` 하나다. 아래 §는 전부 그 문서의 절 번호다.

---

## AC1 — 실행 방법이 정확한 명령/조작 순서로 적혀 있고, 저장소의 실제 스크립트·구조와 일치한다

**어디에**: §12.1 (0~8단계) · §12.1.1

문서가 적은 명령/경로와 그 근거를 하나씩 대조한 표다. **지어낸 명령이 없다는 것이 이 표의 목적이다.**

| 문서가 적은 것 | 저장소 근거 | 확인 |
| --- | --- | --- |
| `npm install` | `package.json` 존재 | ✔ |
| `npm run tauri build` | `package.json` `scripts.tauri = "tauri"` | ✔ 존재하는 스크립트 |
| "`beforeBuildCommand`가 `npm run build`(= `tsc && vite build`)를 먼저 돌린다" | `src-tauri/tauri.conf.json` `build.beforeBuildCommand: "npm run build"` · `package.json` `scripts.build: "tsc && vite build"` | ✔ 두 파일에 그대로 있다 |
| `src-tauri/target/release/bundle/macos/Molt Note.app` | `tauri.conf.json` `productName: "Molt Note"`. **경로 형태는 `docs/ADR-0002-macos-microphone-usage-description.md` §5.2가 이미 같은 문자열로 기록한 것을 재사용했다** — 새로 지어내지 않았다 | ✔ |
| `ls -d …` / `find src-tauri/target/release/bundle -maxdepth 2 -name "*.app"` | 표준 도구. 경로가 다를 경우의 대비책으로 넣었다 | ✔ |
| "`bundle.targets`가 `"all"`이라 다른 산출물도 함께 만들어진다" | `tauri.conf.json` `bundle.targets: "all"` | ✔ |
| `plutil -p "…/Contents/Info.plist" \| grep NSMicrophoneUsageDescription` | **ADR-0002 §5.2의 명령을 그대로 인용** | ✔ |
| `tccutil reset Microphone com.moltnote.app` | `tauri.conf.json` `identifier: "com.moltnote.app"`. **문서가 "이 저장소는 실행해 본 적이 없다 [U]"로 명시했다** | ✔ identifier 일치 |
| `open -R "…app"` (실행이 아니라 Finder에서 보여주기) · 더블클릭으로 실행 | §12.1.1의 이유. 실행 자체는 Finder로 한정했다 | ✔ |
| 사이드바 `Recordings` · `Recording` · `Settings`, 기본 화면은 `Recordings` | `src/navigation/routes.ts` — `SIDEBAR_SCREENS = ['recordings','recording','settings']` · `ROUTES[*].title` = `Recordings` / `Recording` / `Settings` · `HOME_ROUTE = { screen: 'recordings' }` | ✔ |
| 화면 맨 위 `Phase 2A spike — temporary surface.` | `src/screens/RecordingScreen.tsx` `spike-notice` 문단의 문자열 그대로 | ✔ |
| `Input device` 그룹 · `(default)` 표시 · `Reload devices` · `No input devices found.` | `src/screens/RecordingScreen.tsx` 의 실제 라벨 문자열 | ✔ |
| `Capture` 그룹 · `Start` · `Stop` · 상태 `Idle` / `Recording…` / `Finished` | `RecordingScreen.tsx` · `src/screens/captureSpikeView.ts` `STATUS_TEXT` | ✔ |
| `Result`의 네 값 `Device` · `File` · `Format` · `Size` | `RecordingScreen.tsx` 의 `<dt>` 라벨 그대로 | ✔ |
| `The file is empty — nothing was written.` | `RecordingScreen.tsx` (`isEmptyFile`일 때) | ✔ |
| 출력 경로 `<app_data_dir>/recordings/capture-<unix seconds>.wav` | `src-tauri/src/platform/app_data_dir.rs` `RECORDINGS_DIR_NAME = "recordings"` · `src-tauri/src/audio/capture.rs` `file_stem()` = `capture-{unix_seconds}` · `EXTENSION = "wav"` | ✔ |
| "같은 이름이 있으면 `capture-<초>-2.wav`처럼 비켜 간다" | `capture.rs` `output_path()` — `attempt 0`이면 `{stem}.wav`, 아니면 `{stem}-{attempt+1}.wav` | ✔ |
| "app_data_dir의 macOS 해석은 확인하지 않았다 [U] → Result의 File 값을 쓴다" | `app_data_dir.rs`는 `PathResolver::app_data_dir()`만 부르고 OS 경로 문자열을 갖지 않는다. **그래서 문서가 경로를 지어내지 않고 화면 값을 쓰게 했다** | ✔ |
| `ls -l "$FILE"` / `afplay` / `afinfo` / `ffprobe` | 표준 macOS 도구. `ffprobe`는 "ffmpeg가 있는 경우"로 조건을 달았고 §14.1(ffmpeg 8.1.1)을 인용했다 | ✔ |
| 권한 프롬프트가 닿는 두 지점 (`list_input_devices` / `start_capture`) | `RecordingScreen.tsx`의 `useEffect` → `listInputDevices()` · `start()` → `startCapture()`. 두 command는 `src-tauri/src/lib.rs`에 등록되어 있다 | ✔ |
| "MSRV 1.85 이상" | PRODUCT-SPEC §14.3 (인용) | ✔ |
| "Xcode Command Line Tools로 충분" | PRODUCT-SPEC §14.2 (인용) | ✔ |

**존재하지 않는 명령/경로를 적지 않았다.** 불확실한 것은 지어내는 대신 대비책(`find`)이나
UNVERIFIED 표기([U])로 처리했다.

---

## AC2 — 번들된 `.app`에서 확인해야 하는 이유가 적혀 있고, spike가 번들 앱에서 실행 가능한 경로로 문서화되어 있다

**어디에**: §12.1.1 · §12.1의 4단계

§14.3과의 대조:

| 문서의 주장 | PRODUCT-SPEC §14.3의 근거 |
| --- | --- |
| macOS 권한은 실행 주체에 귀속되므로, 터미널에서 띄우면 권한이 터미널 앱에 붙어 제품이 겪을 상황과 달라진다 | §14.3 "마이크 권한 — 플랫폼 차이" + `phase-prompt/02a-recording-engine-validation.md` 86–88행이 같은 제약을 명시한다. ADR-0003 §5.9의 "공통 제약"도 같은 문장이다 |
| Tauri **#11951**(open) — dev 실행에서 마이크 권한 프롬프트가 뜨지 않는다 | §14.3 후보 A "알려진 문제" 행 · §14.3 "Phase 2에서 반드시 실제 확인할 것" 2번 |
| `NSMicrophoneUsageDescription`은 `src-tauri/Info.plist`에 있고 **번들을 만들 때** Tauri CLI 생성값과 병합된다 → 프롬프트 문구는 번들 `.app`에만 있다 | §14.3 macOS 행 (VERIFIED) · ADR-0002 §4 · §5.2 |

세 이유를 **각각 독립된 이유로** 적었다 — 하나만으로도 번들 실행이 필요하다.

**번들 앱에서 실행 가능한 경로**: §12.1.1 마지막 문단이 근거를 저장소 구조로 적었다 —
세 command(`list_input_devices` · `start_capture` · `stop_capture`)가 `src-tauri/src/lib.rs`의
`invoke_handler`에 등록되어 있고, spike 화면이 `src/screens/registry.ts`의 `recording` 라우트로
등록되어 있으므로 번들된 앱 하나만으로 5단계를 지날 수 있다.

동시에 **"실행 가능한 구조다"와 "실행해 봤다"를 구분해 적었다** — 후자는 §12의 표가 채워질 때
처음 사실이 된다고 명시했다.

---

## AC3 — Human Review 8개 항목이 빈 기록표로 존재하고, 어느 항목도 미리 채워져 있지 않다

**어디에**: §12

- 8개 항목이 모두 있다. `phase-prompt/02a-recording-engine-validation.md` 133–140행의
  8개와 1:1로 대응한다:
  1 권한 프롬프트 · 2 선택한 물리 마이크 사용 · 3 장치 열기 · 4 10~20초 녹음 성공 ·
  5 출력 파일 존재 · 6 음성 가청성 · 7 실제 컨테이너/코덱/PCM 포맷과 §14.4 대조 ·
  8 Stop 확정의 무손상
- **8개 항목의 `결과` 칸이 전부 `PENDING`이다.** `PASS`도 `FAIL`도 하나도 없다.
- `사람이 적는 관찰` 칸은 전부 비어 있다.
- 표 위의 메타 네 줄(확인 일자 · 확인한 사람 · 빌드한 커밋 · 기기/macOS 버전)도 `(미기입)`이다.
- "`PENDING`은 아직 하지 않았다는 뜻이며, Worker나 Gate가 이 칸을 대신 채우지 않는다"를
  표 바로 아래에 적었다.
- 짧은 smoke recording이면 충분하고 **1시간 테스트가 아니다**를 §12 본문에 적었다 (§12.1의
  5단계도 "10~20초"로 적혀 있다).

---

## AC4 — 자동 검증 근거가 P2~P4에서 실제로 증명된 것만으로 갱신되었고, 상태는 여전히 PROVISIONAL이며 근거의 세 구분이 유지된다

**어디에**: §4.2 (4.2.1 ~ 4.2.4). 부수적으로 §5.5 · §5.7 · §5.13 · §6 · §9 · 문서 머리말.

### 적은 것과 그 근거

| 문서가 적은 것 | 근거 파일 | 확인 |
| --- | --- | --- |
| `Cargo.toml`이 `cpal = "0.18"` 요구 | `src-tauri/Cargo.toml` | ✔ |
| 해석된 버전 **cpal 0.18.2** | `src-tauri/Cargo.lock` 421–422행 (`name = "cpal"` / `version = "0.18.2"`) | ✔ |
| `Cargo.toml`이 `hound = "3"` 요구 (TASK-012가 추가) | `src-tauri/Cargo.toml` | ✔ |
| 해석된 버전 **hound 3.5.1** | `src-tauri/Cargo.lock` 1340–1341행 | ✔ |
| 컴파일된다 — build/lint/test green | `.loop/evidence/TASK-011/` · `TASK-012/` · `TASK-013/` | ✔ 디렉터리 존재 |
| 확인된 `cpal` API 목록에 `default_input_config` · `build_input_stream` · `play` 추가 | `src-tauri/src/audio/system_capture.rs`가 실제로 부르는 호출이며 컴파일된다 | ✔ |
| 자동 테스트가 덮는 순수 로직 7행 | `audio/devices.rs` · `audio/capture.rs` · `src-tauri/tests/input_devices.rs` · `src-tauri/tests/capture.rs` · `tests/ipc-boundary.test.ts` · `src/screens/captureSpikeView.test.ts` · `tests/macos-bundle.test.ts` — 전부 존재 | ✔ |
| 하드웨어를 부르는 코드는 `system_devices.rs`와 `system_capture.rs` 두 파일뿐 | 소스 트리 확인 (`audio/` = capture · devices · mod · system_capture · system_devices) | ✔ |
| 코드가 고정하는 것: 컨테이너 WAV · 16-bit 정수 PCM | `capture.rs` `CONTAINER = "WAV"` · `BITS_PER_SAMPLE = 16` · `hound::SampleFormat::Int` | ✔ |
| 코드가 고정하지 않는 것: 샘플레이트 · 채널 (장치 값 그대로, 리샘플링·다운믹스 없음) | `system_capture.rs` — `config.sample_rate()` / `config.channels()`를 그대로 `CaptureFormat::pcm_16bit`에 넘긴다. 리샘플러가 소스에 없다 | ✔ |
| f32 → i16 변환이 감기지 않고 잘린다 | `system_capture.rs` `to_i16()` (`clamp`) + 그 테스트 | ✔ |

### 실행하지 않은 것을 사실로 적지 않았다

§4.2.4가 명시적으로 적는다:

- `npm run tauri build`를 **실행한 적이 없다** — 번들을 만든 적이 없다.
  (`tauri build`는 Gate가 아니다: `.loop/project.yaml`의 Gate는 build · lint · test 셋뿐이다 — 확인함)
- 앱을 실행한 적이 없다. 장치를 연 적도, 권한 프롬프트를 본 적도, 파일을 들어 본 적도 없다.
- 따라서 §12는 하나도 채워지지 않았다.

§12.1 머리말도 같은 말을 반복한다 — "이 절은 관찰 기록이 아니라 사람이 따라갈 절차다."

### 상태

- 머리말 `Status: PROVISIONAL — pending human device validation` — **바꾸지 않았다.**
- 머리말에 "§12의 8개 항목은 하나도 채워지지 않았다"를 덧붙였다.
- §2("왜 Accepted가 아닌가")는 그대로다.

### 근거 세 종류의 구분 유지

- §3의 태그 규칙표([R] · [A✓] · [A?] · [H] · [U])를 **바꾸지 않았다.**
- 새로 적은 모든 주장에 태그를 붙였고, §4.2.3의 제목은 `[A✓ — 코드에 대해서만]`으로
  "코드를 읽어 확인한 것"과 "만들어진 파일을 본 것"을 명시적으로 갈랐다.
- [A?] → [A✓]로 승격한 것은 실제로 코드/테스트가 생긴 항목뿐이며, 각각 어느 Task가
  만들었는지 날짜와 함께 적었다 (§5.5 · §5.7 · §5.13의 "2026-09-02 갱신" 인용문).
- **[H]로 남는 것을 승격하지 않았다.** §5.5 · §5.7 · §5.13의 갱신문이 각각
  "여전히 [H]인 것"을 명시한다.

### 정직하게 약화시킨 것 (실제 코드를 읽고 발견한 사실)

§6의 근거 1("whisper 입력 포맷을 변환 없이 만든다")은 **절반만 코드가 됐다.**
`16-bit WAV`는 코드가 고정하지만 `16kHz mono`는 장치가 정하고, 이 저장소에 리샘플링·
다운믹스가 없다. 이것을 §4.2.3 · §5.7 · §6에 그대로 적었다.

**결정을 바꾸지는 않았다** — 근거 1이 무너지는지는 여전히 §12 항목 7이 정한다는 기존 서술을
유지했다. §12.1의 7단계는 사람이 그 네 값을 각각 적도록 표로 갈라 두었다.

### 사실이 아니게 된 서술의 정정

| 이전 서술 | 정정 |
| --- | --- |
| §4.2 "**`hound`는 아직 추가하지 않았다**" | TASK-012가 추가했다. hound 3.5.1이 해석·컴파일됐다 |
| §9 "→ `cpal` 0.18.2는 설치·컴파일됐다. **`hound`는 아직 없다**" | 둘 다 해석·컴파일됐다 |
| §9 "디스크 쓰기 backpressure 시의 동작 [A?] — 아직 설계되지 않았다" | 정책이 생겼다 (`SAMPLE_QUEUE_CAPACITY`, 넘치면 조용히 버리지 않고 정지 시점에 실패로 알린다). **다만 실제 장치 부하에서 이 한도가 충분한가는 [U]로 남겼다** |

---

## AC5 — 문서만 변경했다

`.loop/evidence/TASK-014/changed-files.txt` 참조.

변경한 파일은 `docs/ADR-0003-recording-engine.md` **하나뿐이다.**
소스 · 매니페스트(`package.json` · `Cargo.toml` · `Cargo.lock`) · 설정(`tauri.conf.json` ·
`.loop/**`) · `docs/SYSTEM-MAP.md` — 어느 것도 건드리지 않았다.
git commit도 하지 않았다.
