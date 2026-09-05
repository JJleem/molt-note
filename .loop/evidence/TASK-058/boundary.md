# TASK-058 — clipboard 호출 지점 전수 검색 (AC-4 · AC-5 · AC-6)

기준 시각: 2026-09-04 · 이 Task의 변경이 모두 적용된 작업 트리.

## AC-4 — 부르는 자리가 정확히 하나다

`src/`와 `src-tauri/src/` 전수 검색.

```text
$ grep -rn "navigator\|ClipboardItem\|execCommand\|writeText" src

src/platform/clipboard.ts:106  return clipboardWriter(typeof navigator === 'undefined' ? null : navigator.clipboard);
                               ← webview의 clipboard를 실제로 집는 유일한 줄

src/platform/clipboard.ts:48   writeText(text: string): Promise<void>;      ClipboardWriter 인터페이스 선언
src/platform/clipboard.ts:91   await writer.writeText(text);                넘겨받은 writer(테스트에서는 double)
src/platform/clipboard.ts:120-126                                           writer 판정과 this 유지 호출
src/platform/clipboard.test.ts:28,37,125,155,164,172,181                    전부 test double의 정의/호출
```

`navigator` · `ClipboardItem` · `execCommand`가 나오는 곳은 위 106행 하나뿐이다.
React 컴포넌트(`*.tsx`)에는 한 건도 없다.

```text
$ grep -rni "clipboard\|pasteboard" src-tauri/src src-tauri/Cargo.toml \
      src-tauri/tauri.conf.json src-tauri/capabilities package.json

src-tauri/src/commands/mod.rs:1079,1122   주석 (clipboard가 이 경계 밖이라는 서술)
src-tauri/src/export/ai_request.rs:10,12  주석
src-tauri/src/export/handoff.rs:37        주석
```

Rust 쪽에 clipboard **코드**는 0건이다. 새 npm/cargo 의존성도, 새 Tauri 권한 항목도, 새 command
이름도 늘지 않았다 (ADR-0010 §7.2의 A안 그대로).

이 사실은 주장으로 남지 않고 자동 검사로 고정돼 있다 —
`tests/screen-boundary.test.ts`의 `'src/ 아래에서 clipboard를 부르는 파일이 정확히 하나다'`가
`src/` 전체 원문을 읽어 호출 자리 목록이 `['src/platform/clipboard.ts']`와 정확히 같은지 본다.
파일이 하나 새로 생겨 두 번째 자리가 되면 그 검사가 먼저 깨진다.

## AC-5 — 실패가 보이고, 재시도할 수 있고, 색만으로 말하지 않는다

실패는 **값으로** 돌아온다. `copyText`는 어떤 경우에도 예외를 던지지 않는다
(`clipboard.test.ts`의 `'Error가 아닌 값으로 거절돼도 던지지 않는다'` ·
`'동기적으로 던지는 writer에도 예외가 새어 나가지 않는다'`).

돌아오는 값은 기존 `Failure` 모양이며 §13의 세 답이 그 안에 있다.

```text
kind             'unexpected'          ← 새 FailureKind를 만들지 않았다 (ADR-0010 §7.2)
message          '이 창에서는 클립보드에 쓸 수 없다.' / '클립보드에 쓰지 못했다.'
detail           'clipboard=unavailable' / 'clipboard=rejected: NotAllowedError: …'
sourceDataSafe   true                  ← 아무것도 읽지도 쓰지도 않았다 (MH-7)
retryable        true
```

화면 값(`src/screens/copyView.ts`)에서 실패 상태가 들고 있는 것:

```text
headline           'The prompt could not be copied.'      ← 문장. 색이 아니다
failure            §13의 세 답 그대로
cause              clipboardUnavailable · clipboardRejected · nothingToCopy · storage · other
resolution         갈래마다 먼저 할 일 (문장)
preservedNotice    원본이 그대로라는 사실 (INV-3 · MH-7)
retry              CopyAction — 재시도 수단
alternative        { label: 'Export for AI', text: … }    ← 막혀도 남는 길 (§7.5)
```

성공 상태도 마찬가지로 문장을 동반한다 — `headline: 'Copied to the clipboard.'` + `text`.
`copyView.test.ts`의 `'여섯 갈래 전부가 읽을 수 있는 문장을 들고 있다'`가 모든 갈래에서
비어 있지 않은 문장을 요구한다. **색을 값으로 들고 있는 자리가 아예 없다.**

console로만 남는 경로는 없다 — `tests/screen-boundary.test.ts`의
`'src/ 아래에 실패를 console로 흘려보내는 경로가 없다'`가 새 두 모듈을 포함한 `src/` 전체를
검사하며 통과한다.

## AC-6 — 자동 테스트가 실제 시스템 clipboard를 건드리지 않는다

경계의 모양이 그것을 가능하게 한다.

```ts
copyText(text: string, writer: ClipboardWriter | null = systemClipboard()): Promise<CopyResult>
```

`writer`를 넘기면 시스템 clipboard를 집는 `systemClipboard()`는 **평가되지 않는다**(기본 인자).
`src/platform/clipboard.test.ts`는 모든 경우에 double을 넘긴다 —
쓴 것을 기억하는 double · 언제나 거절하는 double · 동기적으로 던지는 double ·
`this` 유지를 확인하는 double · 능력이 없는 경우(`null`).

`tests/screen-boundary.test.ts`의 `'자동 테스트가 실제 시스템 clipboard를 건드리지 않는다'`가
`src/**/*.test.ts(x)`와 `tests/*.ts` 전부에서 clipboard API를 집는 모양을 금지한다
(검사 자신은 정규식을 원문에 담고 있으므로 스스로를 제외한다 — 그 제외가 경로로 못박혀 있다).

## 확인하지 못한 것 (지어내지 않는다 · ADR-0010 §7.4)

이 Run은 실행 환경에서 앱을 띄우지 않았다. 따라서 다음은 **여전히 UNVERIFIED**다.

```text
Tauri v2 webview(WKWebView · WebView2)에서 이 앱의 origin이 clipboard 쓰기를 실제로 허용하는지
그 API가 요구하는 조건(secure context · 사용자 제스처)이 이 창에서 어떻게 판정되는지
거절할 때 실제로 오는 예외의 이름 (NotAllowedError 계열인지)
Windows에서의 동작
```

그래서 이 경계는 "동작한다"를 전제로 만들어지지 않았다 — 능력이 없는 경우(`unavailable`)와
거절된 경우(`rejected`)가 갈라진 값으로 돌아오고, 거절한 예외의 **이름을 버리지 않고**
`detail`에 남긴다(`'NotAllowedError'`를 확인하는 테스트가 그 보존을 고정한다). 실제 환경의
사실은 P11(TASK-065)이 ADR-0010 §12에 적는다.
