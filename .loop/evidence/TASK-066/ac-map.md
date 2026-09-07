# TASK-066 — Acceptance Criteria ↔ 문서 위치 대응

```text
Run    RUN-20260906T005147Z-TASK-066
대상   docs/ADR-0007-transcription-engine.md   (이 Task가 바꾼 유일한 파일)
```

---

## AC1 — 언어 처리 결정

> ADR-0007에 언어 처리 결정이 있고, whisper.cpp 기본값이 `en`이며 감지가 꺼져 있다는 사실과
> 그 근거 위치, 그리고 앞으로 무엇을 부를 것인가(고르지 않음 = 자동 감지)가 적혀 있다.
> **관측 사실 · 원인 · 앞으로의 규칙이 구분되어** 있어야 한다.

세 가지가 **서로 다른 소절**로 갈려 있다:

| 요구 | 자리 | 무엇이 있는가 |
| --- | --- | --- |
| **관측 사실** | **§17.1.1** *관측된 사실 (2026-09-05 · [E5])* | 1,711 segment · 고유 문장 59 (3.4%) · 한글 0줄 · 최다 반복 1,063회(62.1%) · **상위 2문장 86.7%** · DB `language = en` · 붕괴 구간 00:28:10 ~ 00:56:55 |
| **원인** | **§17.1.2** *원인 — 코드에서 확인됐다 ([E1])* | `whisper.rs`가 설정하는 파라미터 전부를 나열하고 **`set_language`도 `set_detect_language`도 부르지 않는다**고 적었다. whisper.cpp 기본값을 `whisper_full_default_params` (whisper.cpp:5943) 인용으로 붙였다 — `language = "en"` · `detect_language = false`. **DB의 `en`은 감지 결과가 아니라 아무도 바꾸지 않은 기본값을 `full_lang_id_from_state()`로 되읽은 값**이라고 명시했다 |
| **원인이 아닌 것** | **§17.1.3** | `no_context = true` · `temperature_inc = 0.2` · `entropy_thold = 2.4` · `logprob_thold = -1.0`이 whisper.cpp 기본값으로 **이미 켜져 있었다**는 사실과, 따라서 **붕괴는 디코더 설정 문제가 아니라 틀린 언어를 강제한 결과**라는 판정 |
| **앞으로의 규칙** | **§17.1.4** *결정 — 이 앱이 앞으로 무엇을 부를 것인가* | 표: **고르지 않음 → 자동 감지**(`set_detect_language(true)`) · **골랐음 → 그 언어 지정**(`set_language(Some(..))`) · 어느 쪽이든 `set_translate(false)`는 유지. 규칙 4개(기본값에 기대지 않는다 / 출처를 구분한다 / 모르는 값을 지어내지 않는다 / 경계를 넓히지 않는다) |
| **Spec §D와의 연결** | **§17.1.5** | Spec §D의 `| Transcription | whisper model · language · … |` 인용 + `settings` 테이블에 언어 열이 없다는 실측 → **새 제품 결정이 아니라 Phase 3이 빠뜨린 Spec 항목**이라는 결론 |

**근거 위치**: 코드 근거는 §17.1.2가 파일·행(`whisper.rs` 137~147행 · 179행)으로,
기본값 근거는 `whisper.cpp/src/whisper.cpp:5943`으로 걸려 있다.
표기 구분은 §3의 [E1] / [E5]로 한다 ([E5]는 이번에 추가한 표기다).

---

## AC2 — Metal 결정 · feature 이름 · 속도 수치 없음

> feature 이름이 실제 crate 소스나 공식 문서에서 확인된 값으로 확인 경로와 함께 적혀 있다.
> **확인하지 못했으면 UNVERIFIED로 표시돼 있다.** 측정되지 않은 속도 수치나 배수가 없다.

| 요구 | 자리 | 결과 |
| --- | --- | --- |
| Metal 결정이 있다 | **§17.2** (특히 §17.2.1 끝) | *"**결정: Spec §14.4가 적은 대로 Metal을 켠다.**"* |
| Spec이 적었는데 `Cargo.toml`에 없다 | **§17.2.1** 표 | Spec §14.4(오늘 901행) vs `src-tauri/Cargo.toml` 48행 `whisper-rs = "0.16"` — feature 없음 [E1] |
| build.rs가 `GGML_METAL = OFF`를 정의한다 | **§17.2.1** 표 마지막 행 | `whisper-rs-sys-0.15.0/build.rs` 258~265행 [E5]. **이 Run은 그 파일을 다시 읽지 못했다**는 단서를 같은 칸에 붙였다 |
| **feature 이름** | **§17.2.2 — 제목 자체가 `**UNVERIFIED (2026-09-06)**`** | 시도한 확인 경로 4개(crate 소스 2 · 공식 문서 · lock)와 각각의 실패 이유를 표로 적었다. 버전(`whisper-rs` 0.16.0 / `whisper-rs-sys` 0.15.0 · lock checksum)도 함께 적었다. `features = ["metal"]`이 2026-09-05에 한 번 빌드됐다는 것은 **정황([E5])으로만** 적고 *"강한 정황과 확인된 이름은 다른 진술"* 이라고 명시했다 |
| §14 표에도 반영 | **§14** | 새 행: `**Metal을 켜는 whisper-rs 0.16의 정확한 feature 이름** | **UNVERIFIED** [E4] *(2026-09-06 추가)*` |
| cold build 대가(§4.3)를 결정 옆에 다시 적었다 | **§17.2.4** | §4.3의 "Gate 비용 증가"를 되짚고, timeout 900초 [E1 · `.loop/project.yaml`]와 TASK-026의 27.7초가 **Metal이 꺼진 구성의 값**이라는 단서를 붙였다 |
| **속도 수치·배수 없음** | **§17.2.3** | 제목이 *"켰을 때 무엇이 빨라지는가 — **이 프로젝트는 측정한 적이 없다**"*. 부록에 있는 `26분` · `1.3분` · `6.0분` · `6.1분` · `약 12배`를 **하나도 옮기지 않았다** (제외 목록은 `verification-log.md` §4). 대신 **왜 쓸 수 없는지**(언어와 Metal이 함께 바뀌었고 비교 대상이 붕괴한 디코딩이었다)를 적었다 |

**ADR에 남아 있는 시간 값의 성격** — 새로 들어간 속도 추정치가 아니다:
`900초`/`600초`는 Gate timeout 설정값 [E1], `27.7초`는 §4.3에 **이미 있던** 측정값,
`1:12:51`·`72분`·`00:28:10 ~ 00:56:55`는 **오디오의 길이와 위치**이지 속도가 아니다.

---

## AC3 — 기존 절이 삭제·축소되지 않았고, 이력이 덧붙어 있다

**`git diff --numstat` = `336 삽입 / 12 삭제`.** 삭제된 12줄은 전부 **그 자리에서 다시 쓴
줄**이며, 사라진 문단은 없다.

| 삭제된 줄 | 무엇이었나 | 어떻게 됐나 |
| --- | --- | --- |
| 5줄 | 문서 상단 `Status` / `Date` / `Phase` / `Task` / `Scope` 둘째 줄 | 같은 자리에 2026-09-06 갱신 이력을 **덧붙여** 다시 썼다. 기존 내용(TASK-023 · TASK-031 · Phase 3 · 기존 Scope 항목)은 전부 남아 있다 |
| 1줄 | §14의 `가속(Metal 등)이 실제로 켜져 있는가` 행 | 원문을 `~~취소선~~ (2026-09-03)`으로 남기고 `→ 2026-09-05 갱신: 꺼져 있다`를 덧붙였다. 근거 칸의 원래 문장도 *"결정 시점의 기록:"* 뒤에 그대로 있다 |
| 2줄 | §14의 `실제 Whisper 추론이…` · `실제 한국어 전사 품질…` 행 | 같은 방식. `NOT RUN` 원문을 취소선으로 남기고 2026-09-05 결과를 덧붙였다 |
| 4줄 | §16.3 표의 4개 행 (추론 실행 · timestamp 단위 · 번들 버전 · 가속) | 같은 방식. `*(2026-09-03 기준)*` 표시를 붙여 **어느 시점의 판정인지**가 보이게 했다 |

**순수 추가만 있는 자리 (삭제 0):**

- §2 뒤 — *"이 목록은 2026-09-03 결정 시점의 아홉 항목이다. 지우거나 다시 쓰지 않는다"* 와
  결정 10·11이 §17에 따로 있다는 안내
- §3 표 — **[E5] 2026-09-05 실사용 관측** 행 추가 ([E1]~[E4]는 그대로)
- 상단 안내 blockquote — §17이 왜 붙었는지 4줄 추가
- **§16.3.1 (ASSUMPTION A-TRANS-001)** — **본문 한 글자도 바꾸지 않았다.** 뒤에
  *"위 §16.3.1은 2026-09-03의 기록이며 그대로 둔다 → §17.3"* blockquote만 덧붙였다
- **§4.3** — **한 글자도 바꾸지 않았다.** §17.2.4가 그것을 인용해 되짚는다
- §17 전체 (문서 끝에 append)

**언제 왜 바뀌었는지의 이력**: **§17.5 "이 갱신이 바꾼 것과 바꾸지 않은 것"** 이
날짜·Task·이유를 표로 갖는다. 더해 상단 `Date:` 줄, §2 뒤 blockquote, §16.3.1 뒤
blockquote, §14·§16.3의 각 셀에도 `2026-09-06` / `2026-09-05` 표시가 붙어 있다.

**바꾸지 않았다고 명시한 것**: §17.5 첫 문단 — *"§2의 아홉 가지 결정은 하나도 철회되거나
수정되지 않았다"*, §13의 되돌리기 경로도 그대로.

---

## AC4 — `docs/` 밖의 파일이 하나도 바뀌지 않았다

```text
$ git status --porcelain
 M docs/ADR-0007-transcription-engine.md
?? .loop/tasks/TASK-066.yaml     ← Runtime이 만든 Task 파일 (이 Run이 만들지 않았다)
?? .loop/tasks/TASK-067.yaml
?? .loop/tasks/TASK-068.yaml
?? .loop/tasks/TASK-069.yaml
?? .loop/tasks/TASK-070.yaml
?? .loop/tasks/TASK-071.yaml
?? .loop/tasks/TASK-072.yaml
?? .loop/tasks/TASK-073.yaml
?? .loop/tasks/TASK-074.yaml
?? .loop/tasks/TASK-075.yaml

$ git diff --numstat
336     12      docs/ADR-0007-transcription-engine.md
```

**추적 파일 중 수정된 것은 markdown 한 개뿐이다.**
`src-tauri/src/**` · `src-tauri/Cargo.toml` · `src-tauri/Cargo.lock` · `package.json` ·
`src/**` · `tests/**` · `.loop/project.yaml` 어느 것도 바뀌지 않았다.
`?? .loop/tasks/TASK-0**.yaml` 열 개는 Plan 승인 시점에 **Runtime이 생성한** 파일이며
이 Run은 그중 어느 것도 만들거나 고치지 않았다 (KERNEL §2 · §6).

`.loop/evidence/TASK-066/` 아래에 쓴 것은 이 Run의 Evidence이며 Runtime이 그 자리를 미리
만들어 두었다.

**재확인 방법:**

```bash
git status --porcelain          # M 은 docs/ADR-0007-transcription-engine.md 하나뿐
git diff --numstat              # 같은 파일 하나만 나온다
git diff -- src-tauri src tests package.json    # 아무것도 출력되지 않는다
```
