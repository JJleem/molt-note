# 하드코딩된 font-size / px 여백 — 적용 전후 실측 (AC-4)

셋 다 이 저장소에서 실제로 돌린 명령이고, 출력을 그대로 옮겼다. 세는 대상은 `src/` 전체다.

## 세는 명령

```sh
# 타입 스케일을 쓰지 않는 font-size
grep -rhE "font-size:" src | grep -vc "var(--type"

# 스케일이 아닌 px 여백
grep -rhE "(padding|margin)[a-z-]*:[^;]*[0-9]+px" src | wc -l
```

## 세 시점

| 시점 | 하드코딩 font-size | px `padding`/`margin` |
| --- | --- | --- |
| HEAD (P7 이전, `git grep ... HEAD -- src`) | **17** | **54** |
| 이 Task 시작 시점 (TASK-061 적용 후, 미커밋 작업 트리) | **1** | **60** |
| 이 Task 적용 후 | **0** | **0** |

`gap`까지 포함한 px 여백은 이 Task 시작 시점에 70곳이었고 지금은 0곳이다.

```sh
grep -rnE "(padding|margin|gap)[a-z-]*:[^;]*[0-9]+px" src   # 출력 없음
grep -rnE "font-size:" src | grep -v "var(--type"           # 출력 없음
```

## 숫자에 대해 정직하게

- Task 서술의 `font-size 17곳`은 **HEAD 기준으로 정확히 맞는다** (위 표 첫 줄). 그 서술은
  proposal P8이 쓰인 시점, 즉 TASK-061이 들어오기 전의 작업 트리를 센 것이다.
- TASK-061이 그중 16곳을 타입 스케일로 올렸고, 이 Task 시작 시점에 남아 있던 하드코딩
  font-size는 `.recording__elapsed`의 `56px` 하나였다. 그 값은 TASK-061이 "스케일 밖의 예외"로
  의도해서 남긴 것이다.
- 이 Task는 그 하나를 지우지 않고 `--type-display` 토큰으로 올렸다 (`src/App.css` `:root`).
  **예외의 자리는 그대로 한 곳이지만, 값이 화면 규칙 안에 흩어져 있지는 않게 됐다.**
- Task 서술의 `px 여백 36곳`은 HEAD에서 실측한 54곳과 맞지 않는다. 세는 방식(단축 속성만
  세었는지, `gap`을 포함했는지)이 달랐던 것으로 보이며, 어느 기준으로 세도 지금은 0곳이다.

## 남아 있는 px — 여백도 타입도 아닌 값

`src/App.css`에 남은 px는 아래뿐이며, 여백 스케일이나 타입 스케일에 속하지 않는다.

```text
1px          hairline · 입력 테두리
2px          focus ring · 탭 활성 밑줄 · spinner 두께
12px         spinner의 지름 (사각 표면이 아니라 고리 하나)
92px         transcript timestamp 열의 폭
200px        sidebar 폭
520px 640px  읽는 폭의 상한
```
