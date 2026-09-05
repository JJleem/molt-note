# TASK-061 — `src/App.css` UI 기반

변경 파일은 하나다: `src/App.css`.

서드파티 디자인 시스템을 들이지 않았다 — `package.json`의 dependency는 늘지 않았다.
화면 마크업(`src/screens/**`)은 건드리지 않았다. 그것은 TASK-062의 범위다.

---

## AC-4 — 토큰

### 타입 스케일 (`:root`)

| 토큰 | 값 | 위계 |
| --- | --- | --- |
| `--type-page-title` | 20px | page title |
| `--type-section-title` | 16px | section title |
| `--type-body` | 14px | body |
| `--type-secondary` | 13px | secondary |
| `--type-caption` | 12px | caption |

값은 새로 발명한 것이 아니라 이미 이 파일에서 쓰이던 크기다 (20px = `.header__title`,
16px = `.detail__title`, 14px = `:root` 기준, 13px = 15곳). 굵기(`--weight-regular`
`--weight-medium` `--weight-strong`)와 줄높이(`--line-tight` `--line-normal`)가 함께 있다.

**하드코딩된 `font-size`가 17곳 → 1곳**이 됐다 (`grep -n "font-size: [0-9]" src/App.css`가
이제 56px 한 줄만 잡는다). 15곳의 `13px`, `.header__title`의 `20px`, `.detail__title`의 `16px`,
`:root`의 기준 `14px`을 전부 토큰으로 옮겼고 **계산값은 그대로**다 (같은 px을 가리키는
토큰으로만 바꿨으므로 렌더 결과가 변하지 않는다). `:root`의 `line-height: 1.5`도 같은 이유로
`var(--line-normal)`이 됐다 — 기준값이 스케일 밖에 따로 있으면 규칙이 두 벌이 된다.
남은 하나는 `.recording__elapsed`의 56px이며,
"이 화면에서 가장 큰 것은 상태와 경과 시간"(§19)이라는 이유를 주석으로 그 자리에 못박은
의도적 예외다 — 스케일을 이 한 곳에 맞춰 늘리지 않았다.

### 여백 스케일 (`:root`)

`--space-1: 4px` · `--space-2: 8px` · `--space-3: 12px` · `--space-4: 16px` ·
`--space-5: 24px` · `--space-6: 32px`.

새로 더한 규칙(`.btn` · `.input` · `.toggle` · `.status` · `.empty-state` · `.loading` ·
`.field__error` · `.field__hint`)의 여백은 전부 이 여섯 안에서만 쓴다. 기존 화면 규칙의
여백을 이번에 옮기지는 않았다 — 20px · 28px처럼 스케일 밖의 값이 섞여 있고, 그것을 어느 칸으로
보낼지는 화면 위계의 판단이라 TASK-062가 화면과 함께 결정한다. 이 Task가 값을 임의로 바꾸면
렌더가 조용히 달라진다.

### accent와 상태색

| 토큰 | light | dark |
| --- | --- | --- |
| `--accent` | `#0060df` | `#0a6cff` |
| `--accent-strong` | `#0050bb` | `#3b8bff` |
| `--accent-text` | `#0060df` | `#6aa9ff` |
| `--on-accent` | `#ffffff` | `#ffffff` |
| `--success` | `#1a7f37` | `#3fb950` |
| `--warning` | `#9a6700` | `#d29922` |
| `--danger` | `#c0362c` | `#ff6b5e` |

`--radius: 5px`도 함께 올렸다 (값 변경 없음).

accent를 쓰는 자리는 셋뿐이다 — 주 동작(`.btn--primary`), 의미 있는 상태 강조
(`.input:focus-visible` 테두리), 그리고 focus ring. 활성 네비게이션(`.sidebar__item--active` ·
`.tabs__tab--active`)은 기존의 `--selected` / `--text` 표현을 그대로 뒀다: 이 Task는 CSS 기반을
만드는 일이고 화면의 강조 배치를 바꾸는 것은 다음 Task다.

면(`--accent`)과 글자(`--accent-text`)를 나눈 이유: dark에서 흰 글자를 얹을 면과, 어두운 표면
위에서 글자로 읽힐 파랑의 대비 조건이 서로 다르다. 하나로 두면 둘 중 하나가 반드시 흐려진다.

대비 (WCAG 2.1 상대 휘도 계산):

```text
light  --accent   #0060df 위의 --on-accent #ffffff   5.67:1   (AA 본문 통과)
light  --success  #1a7f37 on #ffffff                 5.13:1
light  --warning  #9a6700 on #ffffff                 4.86:1
light  --danger   #c0362c on #ffffff                 5.52:1
dark   --accent   #0a6cff 위의 --on-accent #ffffff   4.56:1   (AA 본문 통과)
dark   --accent-text #6aa9ff on --surface #1c1c1e    7.15:1
dark   --success  #3fb950 on #1c1c1e                 6.70:1
dark   --warning  #d29922 on #1c1c1e                 ~6.9:1
dark   --danger   #ff6b5e on #1c1c1e                 6.19:1
```

### dark 장치가 깨지지 않았다

- 기존 색 토큰 6개(`--text` `--text-muted` `--surface` `--surface-sidebar` `--hairline`
  `--selected`)는 light · dark 양쪽에서 **값도 순서도 그대로**다. 한 줄도 지우거나 바꾸지 않았다.
- 새로 더한 색 토큰 7개는 `@media (prefers-color-scheme: dark)` 블록에도 전부 정의돼 있다.
- 크기 · 여백 · 굵기 · radius 토큰은 테마와 무관하므로 dark 블록에서 다시 정의하지 않았다.
  그 이유를 dark 블록 위 주석에 적었다.
- 테마 시스템으로 번지게 하지 않았다 — 토글도, 저장되는 테마 설정도, `data-theme` 속성도 없다.

---

## AC-5 — focus · 컨트롤 · 상태 · 빈 상태 · 로딩

### 보이는 focus

저장소에 `:focus` 스타일이 **0곳**이었다. 이제 전역 규칙 하나가 있다:

```css
:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
```

`:focus`가 아니라 `:focus-visible`이므로 마우스로 누를 때는 뜨지 않고 **키보드로 왔을 때만**
보인다. `.sidebar`와 `.list`는 스크롤/경계 안이라 바깥으로 2px 나간 링이 잘리므로
`outline-offset: -2px`로 안쪽에 그린다. 이 규칙은 새 클래스가 아니라 요소 상태에 붙으므로
**기존 화면의 semantic button/input이 그대로 혜택을 받는다** — 마크업 변경이 필요 없다.

### 버튼 3종 + Danger

`.btn` (공통) · `.btn--primary` · `.btn--secondary` · `.btn--ghost` · `.btn--danger` ·
`.btn:disabled`. hover/active는 `:not(:disabled)`로 막아 꺼진 버튼이 반응하지 않는다.
Danger는 붉은 **면**이 아니라 붉은 **글자**다 — 화면이 경보처럼 보이지 않게 한다.
disabled는 `opacity: 0.5`로 흐리되 레이블이 사라지지 않는다.

### 입력

`.input` · `.select`(같은 규칙) · `.toggle` + `.toggle__input` · `.field__error` ·
`.field__hint`. 검증 실패는 `[aria-invalid='true']` 테두리 색과 `.field__error` 문장이
**함께** 말한다 — 색만으로 말하지 않는다. toggle은 네이티브 checkbox를 그대로 쓰고
`accent-color`만 맞춘다 (키보드 조작과 상태 전달이 이미 옳으므로 다시 만들지 않는다).
base 규칙의 `font: inherit; color: inherit`에 `select`와 `textarea`를 더해, 지금
`SettingsScreen.tsx`가 쓰는 `<select>` 세 개가 앱 글꼴과 어긋나지 않게 했다.

### 상태 6종

`.status` + `--working` · `--success` · `--warning` · `--error` · `--disabled`.
idle은 조용한 기본값이므로 modifier 없이 `.status` 자체다 (그래서 여섯 갈래가 전부 표현된다).
`.status`는 inline-flex 텍스트 컨테이너이며 **상태를 말하는 글자를 담는 자리**다 —
점이나 뱃지를 넣지 않았다. 색만으로 상태를 말하는 자리를 만들지 않기 위해서다 (요구 12).

### 빈 상태 · 로딩

`.empty-state` / `__title` / `__body` — 경고색도 테두리도 없다. 무엇이 없는가 한 줄,
무엇을 하면 되는가 한 줄. `.loading` + `.spinner` — 고리는 언제나 글자와 함께 온다.

---

## AC-6 — 넣지 않은 것

```text
gradient          0곳  (`linear-gradient` · `radial-gradient` 없음)
box-shadow        0곳
backdrop-filter   0곳  (glassmorphism 없음)
blur              0곳
```

`grep -n "gradient\|box-shadow\|backdrop\|blur(" src/App.css`가 잡는 것은 "두지 않는다"고
적은 주석 두 줄뿐이다.

### 애니메이션 2개 — 둘 다 상태 전이다

1. `.btn`의 `transition` 0.12s (background-color · border-color · color).
   눌렀는가 · 위에 있는가 · 꺼졌는가가 튀지 않고 갈리게 하는 것뿐이다.
2. `.spinner`의 `spinner-turn` 회전. 요구된 "로딩 표시"이며, 오래 걸리는 동작이 얼어붙은
   것처럼 보이지 않게 한다.

장식적 애니메이션(등장 효과 · 강조 펄스 · 호버 확대 등)은 없다.
`@media (prefers-reduced-motion: reduce)`에서 둘 다 끈다 — spinner는 멈춘 고리로 남고,
글자가 함께 있으므로 뜻은 그대로다.

### border-radius

```text
var(--radius) (= 5px)   2곳   새로 더한 .btn · .input/.select
5px 리터럴              6곳   기존 규칙 — 한 줄도 바꾸지 않았다
50%                     1곳   .spinner
```

`--radius`는 5px이므로 **사각 표면의 radius는 여전히 전부 5px 하나**다. 50%는 고리 하나뿐이고,
"5px는 사각 표면의 규칙이고 고리는 원이어야 한다"는 이유를 그 자리 주석에 적었다.

---

## 기존 것을 깨뜨리지 않았다

- 기존 클래스 81개 중 **삭제되거나 이름이 바뀐 것이 없다.** 새 규칙은 전부 새 클래스
  (`.t-*` · `.btn*` · `.input` · `.select` · `.toggle*` · `.status*` · `.empty-state*` ·
  `.loading` · `.spinner` · `.field__error` · `.field__hint`)이거나 요소 상태
  (`:focus-visible`)에 붙는다. 기존 화면은 그대로 렌더된다.
- 새 클래스 이름이 기존 이름과 겹치지 않는다 — 기존의 빈 상태 클래스는 `.empty`이고 새것은
  `.empty-state`, 기존 입력은 `.field__input`이고 새것은 `.input`이다. 어느 쪽도 서로의
  스타일을 덮지 않는다.
- 기존 규칙에서 실제로 바뀐 선언은 전부 **같은 값을 가리키는 토큰으로의 치환**이다:
  `font-size: 13px` 15곳 · `20px` 1곳 · `16px` 1곳 · `:root`의 `14px`과 `line-height: 1.5`.
  base 규칙에 `select, textarea`가 더해진 것이 유일한 동작상 추가다.
