# Falsification — 이 테스트들이 빈 단언이 아니라는 것 (AC-4)

날짜: 2026-09-07 · Task: TASK-084 · Run: RUN-20260907T050531Z-TASK-084

## 방법

제품 파일을 고치면 이 Task의 범위를 벗어나고 되돌리다 흔적을 남길 수 있다. 그래서
**규칙을 위반하는 임시 파일을 넣었다 지우는 방식**으로 확인했다. 임시 파일은 추적되지
않는 새 파일이므로 지워졌는지를 `git status`가 확실하게 말해 준다.

이 검사들은 디렉터리를 훑어 소스 전체를 보므로, 파일 하나가 새로 생기는 것만으로 규칙이
깨진다 — 그것이 이 검사들이 개별 모듈 옆이 아니라 `tests/`에 있는 이유이기도 하다.

## 넣은 임시 파일 6개

```rust
// src-tauri/src/audio/zz_scratch_probe.rs
pub const USABLE_AT_OR_ABOVE_DBFS: f64 = -36.0;      // 판정 상수를 둘째 자리에 복제

pub fn apply_gain(samples: &mut [i16]) {              // 게인 + 가변 샘플 슬라이스
    for sample in samples.iter_mut() {
        *sample = sample.saturating_mul(2);
    }
}

pub fn ship(url: &str) {                              // 레벨을 재는 자리에서 밖으로
    let _ = ureq::get("https://example.invalid").call();
    let _ = url;
}
```

```rust
// src-tauri/src/transcription/zz_scratch_probe.rs
pub const MINIMUM_SENTENCES_TO_JUDGE: usize = 20;     // 붕괴 임계값 복제
pub const COLLAPSED_UNIQUE_RATIO_AT_OR_BELOW: f64 = 0.20;

pub fn assess(n: usize, u: usize) -> bool {           // 판정을 둘째 자리에서 다시 만든다
    (u as f64 / n as f64) <= 0.20
}
```

```rust
// src-tauri/src/domain/zz_scratch_probe.rs
pub const PROVIDER: &str = "ollama";                  // core/domain에 벤더 (INV-9)
```

```rust
// src-tauri/src/export/zz_scratch_probe.rs
pub fn body(average_dbfs: f64) -> String {            // 밖으로 나가는 자리가 레벨을 안다
    format!("level {average_dbfs}")
}
```

```ts
// src/screens/zzScratchProbe.ts
export function scratchLevel(session: SessionStatus): string {
  const level = session.level;                        // 레벨을 읽는 둘째 모듈
  if (level === null) {
    return 'none';
  }
  const dbfs = 20 * Math.log10(level.averageDbfs / 32768);   // 화면의 dBFS 환산
  const uniqueRatio = 0.20;                                   // 화면의 붕괴 임계값
  return dbfs < -36 && uniqueRatio < 0.50 ? 'low' : 'usable';
}
```

```ts
// src/screens/zzScratchProbe2.ts
export const KNOWN_HALLUCINATIONS = ['한글자막 by 한효정'];   // 모델 고유 문자열
```

## 결과 — 29개 중 13개가 실패했다

```text
 ❯ tests/level-and-collapse-boundary.test.ts (29 tests | 13 failed) 72ms
     × 레벨을 재는 자리에 기기 밖으로 나가는 통로가 없다 6ms
     × 밖으로 나가는 요청을 만드는 자리가 입력 레벨을 알지 않는다 7ms
     × 임계값 상수를 정의하는 파일이 정확히 하나다 9ms
     × 세는 것도 나누는 것도 그 파일 하나에서만 일어난다 7ms
     × 전사 실행 경로와 엔진에 임계값이 복제돼 있지 않다 1ms
     × 화면 쪽에 붕괴 판정 규칙이 없다 9ms
     × 레벨 값을 직접 읽는 제품 모듈이 src/ 아래에 하나뿐이다 4ms
     × 판정 구간을 정하는 상수가 Rust의 한 파일에만 있다 9ms
     × src/ 아래에 RMS나 dBFS를 만드는 산술이 없다 6ms
     × 캡처 경로 어디에도 게인도 정규화도 리샘플도 없다 2ms
     × 샘플을 바꿀 수 있는 자리가 캡처 경로 표면에 없다 1ms
     × core/domain에 벤더 고유 이름이 없다 1ms
     × 붕괴 판정이 특정 모델이 낸 문자열을 알지 않는다 5ms

 Test Files  2 failed | 26 passed (28)
      Tests  15 failed | 556 passed (571)
```

같은 임시 파일이 기존 `tests/screen-boundary.test.ts`의 두 검사도 함께 깨뜨렸다
(`src/ 아래에 dBFS 환산이 없다` · `src/ 아래에 판정 임계값이 없다`). 그 둘이 여전히 살아
있다는 것도 이 자리에서 함께 확인된 셈이다.

## 이 절차가 찾아낸 실제 결함 하나

**첫 판에서 `apply_gain(samples: &mut [i16])`을 넣었는데 게인 검사가 통과했다.**

```text
(첫 판)   × ... 11 failed   —  '캡처 경로 어디에도 게인도 정규화도 리샘플도 없다'가 없다
(고친 뒤) × ... 12 failed   —  그 검사가 실패 목록에 들어왔다
```

원인: `/\bgain\b/i`는 `apply_gain`을 잡지 못한다. `_`는 낱말 문자이므로 `_`와 `g` 사이에
낱말 경계가 없다. 이름 뒤쪽에 붙는 `apply_gain` · `gain_factor`가 오히려 흔한 모양이므로
`/(?<![a-z])gain/i`로 고쳤다 — 앞이 글자가 아닌 자리만 보면 그 둘은 잡히고 `again` 같은
낱말은 잡히지 않는다. `boost`와 `amplif`도 같은 이유로 낱말 경계를 뗐다.

**검사를 통과시키려고 규칙을 약하게 한 것이 아니라, 검사가 놓치던 자리를 찾아 넓혔다.**

## 되돌리기

임시 파일 6개를 전부 지웠고 남은 것이 없다는 것을 확인했다.

```text
$ git status --short | grep -i scratch
(출력 없음)
```

지운 뒤 `self-check build lint test` 세 Gate가 모두 PASS다 (`gates.txt`).
이 Task가 남긴 변경은 `?? tests/level-and-collapse-boundary.test.ts` 하나뿐이다
(`git-status.txt`).
