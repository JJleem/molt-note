# TASK-064 — 검사가 실제로 무는지 확인한 기록 (negative control)

**통과하는 검사는 그 자체로 증거가 아니다.** 아무것도 읽지 못한 검사도 언제나 통과한다.
그래서 `src/App.css`에 위반을 **일부러 넣고** test Gate를 돌려 각 검사가 실제로 깨지는지
확인했다. 두 번 모두 확인 직후 원래대로 되돌렸고, 되돌린 뒤 build · lint · test 세 Gate가
전부 다시 PASS했다 (`gates.md`).

되돌림 확인:

```text
grep -n -e "danger-soft" -e "outline: none" -e "display: none" -e "backdrop-filter" \
        -e "box-shadow" -e "linear-gradient" -e "7px" -e "11px" src/App.css
→ 출력 없음
```

---

## 1회차 — 검사 1과 검사 4

`.field__hint`를 아래로 바꿨다.

```css
.field__hint {
  margin: 7px 0 0;                                  /* 여백 스케일 밖 */
  font-size: 11px;                                  /* 타입 스케일 밖 */
  color: var(--text-muted);
  background-image: linear-gradient(#fff, #eee);    /* gradient */
  backdrop-filter: blur(4px);                       /* glassmorphism */
  box-shadow: 0 2px 6px #0003;                      /* 그림자 */
  transition: opacity 0.4s ease;                    /* 허용되지 않은 움직임 */
}
```

결과 — `tests/ui-foundation.test.ts (26 tests | 7 failed)`

```text
× 임의의 하드코딩 font-size가 기준선을 넘지 않는다
× 임의의 하드코딩 여백이 기준선을 넘지 않는다
× 모든 여백이 스케일을 지나거나 0이다
× gradient가 없다
× glassmorphism이 없다
× 그림자가 없다                     .field__hint에 그림자가 있다
× 움직임이 허용된 두 자리에만 있다   .field__hint의 transition은 허용된 움직임이 아니다
```

## 2회차 — 검사 2 · 검사 3 · 검사 5

```css
:focus-visible { outline: none; }        /* 보이는 focus를 없앤다 */
:root { ... --danger-soft: #ffeeee; }    /* dark 블록에 없는 색 토큰 */
.status--success { color: ...; display: none; }  /* CSS가 뜻을 나르기 시작한다 */
```

결과 — `tests/ui-foundation.test.ts (26 tests | 4 failed)`

```text
× 아무 조건 없이 모든 focus에 걸리는 규칙이 하나 있다
× focus 링을 꺼 버리는 자리가 없다
× light의 색 토큰이 전부 dark 블록에도 정의돼 있다   --danger-soft이 dark 블록에 없다
× 상태를 나타내는 CSS가 색을 얹을 뿐 뜻을 나르지 않는다
                                    expected [ 'color', 'display' ] to deeply equal [ 'color' ]
```

---

## 값 수준 검사(5)가 비어 있지 않다는 것

원문을 읽는 검사와 달리 5번의 본체는 순수 view 모듈을 **실제로 불러서** 판정한다.
갈래를 하나도 부르지 못한 채 통과하는 것을 막기 위해, 각 검사가 "부른 갈래의 집합"을
정확히 고정한다.

```text
copyPanel      loading · nothingToCopy · notAsked · copying · copied · failed   (6갈래)
aiExport       loading · nothingToExport · notAsked · exporting · done · failed (6갈래)
AiConnection   notConfigured · running · noModels · notRunning · checkFailed    (5갈래)
statusBadge    none · pending · running · done · failed                         (5갈래)
sessionDisplay idle · recording · paused · stopped · (아직 모른다)              (5갈래)
```

집합이 어긋나면(갈래가 늘거나, 입력이 빗나가 갈래에 닿지 못하면) 그 검사가 먼저 깨진다.

## CSS 파서가 실제 규칙을 읽는다는 것

`rules`가 비었다면 4번의 금지 검사들은 조용히 통과한다. 그것을 막는 것은 같은 파일의
**양성 단언들**이다 — 이들은 파서가 실제 규칙을 읽어야만 통과한다.

```text
--type-display를 쓰는 규칙이 정확히 ['.recording__elapsed'] 하나다
focus 스타일이 0곳이 아니다
@keyframes 이름이 정확히 ['spinner-turn']이다
.status--* 규칙을 하나 이상 찾았다
활성 상태(--active · --live) 규칙을 하나 이상 찾았다
light의 색 토큰을 하나 이상 찾았다
```
