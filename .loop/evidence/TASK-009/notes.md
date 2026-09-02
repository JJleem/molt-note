# TASK-009 — macOS `NSMicrophoneUsageDescription` 선언

## 무엇을 했나

| 파일 | 내용 |
| --- | --- |
| `src-tauri/Info.plist` (신규) | `NSMicrophoneUsageDescription` 선언 + 이것이 macOS packaging 요구사항이지 권한 구현이 아니라는 주석 |
| `tests/macos-bundle.test.ts` (신규) | 선언 존재 · 문구 비어 있지 않음 · `tauri.conf.json` 오배치 없음을 검사 |
| `docs/ADR-0002-macos-microphone-usage-description.md` (신규) | 결정 근거 · 경계(구현되지 않은 것) · 사람 확인 항목 |

권한 요청 코드, 장치 열거 코드, Windows 권한 로직은 만들지 않았다.
기존 파일은 하나도 수정하지 않았다 (`changed-files.txt` 참조).

## 선언 내용 (`src-tauri/Info.plist`)

```xml
<key>NSMicrophoneUsageDescription</key>
<string>Molt Note는 회의와 강의를 녹음해 텍스트로 변환하기 위해 마이크를 사용합니다. 녹음은 사용자가 직접 시작할 때만 이루어지며, 오디오는 이 Mac에 저장됩니다.</string>
```

## 왜 `tauri.conf.json`이 아닌가

`docs/PRODUCT-SPEC.md` §14.3 (line 747)이 확인해 둔 사실:

> macOS | `src-tauri/Info.plist`에 `NSMicrophoneUsageDescription`을 넣으면 Tauri CLI 생성값과
> **병합**된다 (VERIFIED · v2.tauri.app/distribute/macos-application-bundle).
> `tauri.conf.json` 키가 아니다.

`tauri.conf.json`은 이 Run에서 변경하지 않았고, 새 테스트가 그 파일에 해당 키가 들어오는 것을
회귀로 막는다.

## 검증 상태

### 자동 (test Gate로 판정됨 — `gate-test.stdout.log`)

- `NSMicrophoneUsageDescription` 키가 `src-tauri/Info.plist`에 존재한다
- 그 키의 `<string>` 값이 비어 있지 않다
- `tauri.conf.json`에 같은 키가 없다

vitest: Test Files 4 passed / Tests 22 passed. 4개 파일 중 하나가 이 Task가 추가한
`tests/macos-bundle.test.ts`다 (저장소의 제품 테스트 파일 전체 목록은 `gate-test.stdout.log` 하단).

### UNVERIFIED — 사람 확인 항목

- **번들된 `.app`의 `Contents/Info.plist`에 이 키가 실제로 병합되는가.**
  확인하려면 `npm run tauri build`가 필요한데 그것은 이 프로젝트의 Gate가 아니다
  (`.loop/project.yaml`의 Gate는 build · lint · test 셋이며 `npm run build`는 프론트엔드 빌드다).
  자동 판정 대상이 아니며 ADR-0002 §5.2에 사람 확인 항목으로 기록했다.
- 권한 프롬프트 문구가 사용자에게 납득 가능한가 (Phase 1 Human Review 항목).
- 번들 실행 시 TCC 프롬프트가 실제로 뜨는가 — 실제 캡처 코드가 필요하므로 Phase 2.

## 실행한 Gate

`node tools/loop-runtime/loopctl.mjs self-check lint test` → lint PASS (exit 0) · test PASS (exit 0).
자문용이며 완료 판정이 아니다.
