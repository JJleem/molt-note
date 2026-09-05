# 네 화면에 적용한 것 (AC-4 · AC-5 · AC-6)

행 번호는 이 Task 적용 후의 것이다.

## 1. 네 화면이 타입 · 여백 스케일 위에 있다 (AC-4)

`src/App.css`의 화면별 절이 전부 토큰을 쓴다. 숫자는 `token-counts.md`에 있다.

- 타입: `--type-page-title · --type-section-title · --type-body · --type-secondary ·
  --type-caption`. 스케일 밖의 한 칸(`--type-display`)은 녹음 화면의 경과 시간 하나만 쓴다.
- 여백: `--space-1..6` 여섯 값. 그 밖의 값을 새로 만들지 않았다.
- 굵기 · 행간 · radius도 토큰(`--weight-* · --line-* · --radius`)으로 바꿨다.
- 화면 마크업의 임시 클래스 둘을 규칙 위로 올렸다 — `.action`은 사라지고 전부
  `.btn btn--primary|--secondary|--ghost|--danger`가 됐으며, `.field__input`은 사라지고
  foundation의 `.input` · `.select`가 됐다 (`grep -rn 'className="action"\|field__input' src`
  → 출력 없음).

### (1) Recordings — 행을 카드로 만들지 않았다

`src/App.css` `.list*` · `src/screens/RecordingsScreen.tsx`.

행은 여전히 **테두리 없는 button 하나 + hairline 하나**다. 카드로 만들지 않았고
(`box-shadow` 0곳, 행 테두리 0곳), 위계는 타이포 세 단이 만든다 — 제목은 body/medium,
날짜와 길이는 secondary, 세 상태는 caption. 가리키는 면만 `--surface-sidebar`로 옅게 뜬다.

### (2) Recording — 상태와 주 조작이 지배한다

`src/App.css` `.recording*` · `src/screens/RecordingScreen.tsx:213`.

경과 시간이 `--type-display`(56px)로 화면에서 가장 크고, 그 위에 상태 문장이 있다. 네 조작은
`--type-section-title` 크기의 버튼이며 본문보다 크다. Record 하나만 primary다. 못 누르는
버튼은 `disabled`로 흐려져 눈에도 갈린다. **어느 것을 누를 수 있는지 정하는 규칙은 여전히
`recordingView.ts`의 `recordingControls`에 있다.**

### (3) Recording Detail — 탭을 갈아엎지 않았다

`src/screens/RecordingDetailScreen.tsx:102` — `const TABS = ['AI Note', 'Transcript', 'Recording']`.
**세 탭의 이름도 순서도 그대로다.** 바뀐 것은 셋이다.

- 여백과 활성 표시 (`.tabs` · `.tabs__tab--active` — 밑줄 · 굵기 · 글자색 셋이 함께 말한다)
- 탭과 그 내용이 `id` / `aria-controls` / `role="tabpanel"` / `aria-labelledby`로 이어졌다
  (`:114`–`:116`의 `slug` · `tabId` · `panelId`, `:731`의 panel)
- 제목 → 날짜·길이 → 재생 → 내보내는 문 둘 → 탭 순서의 여백 위계

### (4) Settings — 단순한 그룹 목록

`src/App.css` `.group*` · `src/screens/SettingsScreen.tsx`.

설정마다 카드를 두지 않았다. 그룹 사이를 가르는 것은 `.group + .group`의 hairline 하나와
여백뿐이고, 그룹 안의 위계는 `.group__title` · `.group__subtitle` · 여백이 만든다. 화면
전체의 primary는 Save 하나다.

## 2. 빈 상태 다섯 갈래와 로딩 (AC-5)

다섯이 **같은 한 모양**을 쓴다 — `src/screens/EmptyState.tsx` (`role="status"`, 경고색도
테두리도 없다). 문장은 전부 순수 모듈이 만든 값 그대로다.

```text
1 녹음 없음        RecordingsScreen.tsx:75
2 transcript 없음  RecordingDetailScreen.tsx:1011   (transcriptTab의 'none')
3 AI Note 없음     RecordingDetailScreen.tsx:1187   (aiNoteTab의 'none')
4 provider 없음    RecordingDetailScreen.tsx:1167   (aiNoteTab의 'disabled')
5 Notion 미설정    SettingsScreen.tsx:600           (notionTokenState의 'notStored')
```

같은 모양을 쓰는 다른 "아직 재료가 없다" 자리 일곱 — 내보낼 것 없음 · 보낼 것 없음 ·
복사할 것 없음 · transcript를 가리키지 않는 노트 자리 · 대상 녹음 없음 · 목록에서 사라진 녹음
(`:628 · :664 · :784 · :888 · :1182 · :1306 · :1382`).

로딩은 `src/screens/Loading.tsx` 하나다 — **고리 하나와 언제나 함께 오는 글자**이며,
`prefers-reduced-motion`에서 고리가 멈춰도 뜻은 글자가 나른다. 열일곱 자리에 있고, 그중 오래
걸리는 아홉(전사 · 노트 생성 · Notion 전송 · 내보내기 둘 · 복사 · provider 확인 · Notion 확인)은
`live`로 소리에도 전해진다 (`grep -rn "<Loading" src/screens/`).

## 3. 접근성 기준선 (AC-5)

- **색만으로 상태를 말하는 자리가 없다.** 상태에는 언제나 문장이 있다 — 녹음 상태
  (`display.stateText`) · 목록의 세 상태(이름 + 값) · 활성 탭(`aria-selected` + 밑줄 + 굵기) ·
  활성 sidebar 항목(`aria-current` + 면 + 굵기) · 고른 mode(`aria-pressed` + 테두리 + 굵기) ·
  복사됨/저장됨(문장) · 실패(`FailureNotice`의 문장).
- **보이는 focus**: `:focus-visible` 규칙이 전역에 있고, 스크롤 영역 안에서는 안쪽으로
  그린다. 이 Task가 만든 새 컨트롤도 전부 semantic `button` / `input` / `select`이므로 그대로
  적용된다.
- **기존 role/aria를 깨뜨리지 않았다.** `role="alert"`(FailureNotice) · `role="status"` ·
  `role="tablist"` · `role="tab"` · `aria-selected` · `aria-live` · `aria-pressed` ·
  `aria-current` · `aria-invalid` · `aria-label`이 전부 그대로이며, 더한 것만 있다:
  `role="tabpanel"` + `aria-controls` + `aria-labelledby`, `role="group"`(녹음 조작 · 이미
  있던 mode group과 같은 규칙), `aria-label`(목록 · 재생), `aria-hidden`(spinner).
  이전에 `<div role="status">`였던 자리는 `EmptyState`가 같은 `role="status"`를 그대로 낸다.
- **disabled 구분 가능**: `.btn:disabled` · `.note__mode:disabled` · `.input:disabled`가
  흐려지되 사라지지 않는다.
- **합리적 대비**: `--text-muted`를 `#8a8a8e`(흰 표면 위 약 3.4:1)에서 `#6b6b70`(약 5.3:1)로
  낮춰 잡았다. 이 Task가 부제 · 상태 · caption을 이 색으로 옮기면서 그 글자의 양이 늘었기
  때문이다. dark 쪽은 이미 약 5.2:1이라 그대로 두었다.

## 4. 비즈니스 로직이 컴포넌트로 옮겨가지 않았다 (AC-6)

이 Task가 편집한 파일은 아홉이고, **`src/screens/*View.ts` 순수 view 모듈은 하나도 없다.**

```text
수정  src/App.css
수정  src/App.tsx
수정  src/screens/FailureNotice.tsx
수정  src/screens/RecordingsScreen.tsx
수정  src/screens/RecordingScreen.tsx
수정  src/screens/RecordingDetailScreen.tsx
수정  src/screens/SettingsScreen.tsx
신규  src/screens/EmptyState.tsx
신규  src/screens/Loading.tsx
```

- 순수 모듈의 판정 규칙도 문장도 하나 바뀌지 않았다. 화면에 나오는 문자열은 전부 여전히
  `body.text` · `body.hint` · `tab.text` · `tokenNotice.text`처럼 **모듈이 만든 값 그대로**이며,
  이 Task는 그것을 어느 상자에 넣을지만 바꿨다.
- 새 컴포넌트 둘은 판정을 하지 않는다. `EmptyState`는 `title` · `body` · `children`을,
  `Loading`은 `text` · `live`를 받아 그리기만 한다.
- 컴포넌트가 새로 보게 된 값은 셋이며 전부 **순수 모듈이 이미 만들어 둔 갈래**다 —
  `tokenState === 'notStored'` (`notionTokenState`가 만든다) · `connection.kind === 'checking'` ·
  `notion.kind === 'checking'`. 이 저장소가 이미 쓰는 규약(`body.kind`로 모양을 고른다)과 같은
  자리이며, 판정이 컴포넌트로 옮겨온 것이 아니라 같은 값에 모양 하나가 더 붙은 것이다.
- `tests/manual-handoff-invariants.test.ts`가 `beginCopy` · `beginAiExport` 블록의 원문과
  기존 생성 경로 호출을 그대로 확인한다 (통과).
