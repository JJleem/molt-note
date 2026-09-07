# TASK-089 — 검사가 빈 단언이 아니라는 확인

새 검사가 **실제로 규칙 위반을 잡는지**를 규칙을 깨 보고 확인했다.
제품 파일은 고치지 않았다 — 위반을 담은 **임시 파일 둘**을 넣고 Gate를 돌린 뒤 지웠다
(TASK-084가 세운 선례와 같은 방법이다).

## 넣은 임시 파일

```text
src-tauri/src/transcription/_falsify_rule.rs
  // mod.rs에 등록하지 않으므로 cargo는 이 파일을 컴파일하지 않는다.
  pub const MAX_CONSECUTIVE_REPEATS: usize = 5;
  pub const CHUNK_SECONDS: u32 = 60;

src/screens/_falsifyChunk.ts
  export const offsetCentiseconds = 12000;
```

즉 **청킹 규칙의 두 번째 벌이 Rust 쪽에 하나 · 화면 쪽에 하나** 생긴 상태다.

## 결과 — 세 검사가 실패했다

`node tools/loop-runtime/loopctl.mjs self-check test`

```text
 FAIL  tests/transcription-chunking-boundary.test.ts
   × (a) 청킹 규칙이 한 자리에만 있다 > 세 값을 정의하는 파일이 정확히 하나다
   × (a) 청킹 규칙이 한 자리에만 있다 > 엔진 · 실행 경로 · 그 밖의 Rust 어디에도 청킹 값이 복제돼 있지 않다
   × (a) 청킹 규칙이 한 자리에만 있다 > 화면 쪽에 청킹 규칙이 없다

 AssertionError: .../src-tauri/src/transcription/_falsify_rule.rs에 청킹 규칙이 복제됐다:
   /CHUNK_SECONDS:/: expected 'pub const MAX_CONSECUTIVE_REPEATS: us…' not to match /CHUNK_SECONDS:/

 AssertionError: .../src/screens/_falsifyChunk.ts에 청킹 규칙이 있다:
   /offsetCentiseconds/i: expected 'export const offsetCentiseconds = 120…' not to match /offsetCentiseconds/i

 Test Files  1 failed | 29 passed (30)
      Tests  3 failed | 609 passed (612)
```

**"규칙을 아는 파일이 정확히 하나다"라는 집합 단언이 새 파일 하나에 즉시 반응한다.**
`chunking.rs`의 단위 테스트는 `include_str!`로 **이름을 적어 둔 다섯 파일**만 보므로 이 위반을
잡지 못한다 — 그 간극이 이 boundary 테스트가 저장소 전체를 원문으로 읽는 이유다.

## 되돌림 확인

```text
rm src-tauri/src/transcription/_falsify_rule.rs src/screens/_falsifyChunk.ts
```

지운 뒤 `self-check build lint test`가 전부 PASS다 (`gates.txt`). `git-status.txt`에도 두 임시
파일이 남아 있지 않다.

## 이 방법으로 확인하지 **않은** 검사

- **`(e) 새 의존성이 늘지 않았다`** — 위반을 만들려면 `src-tauri/Cargo.toml`을 고쳐야 하고,
  그것은 제품 설정이다(AC-6). 대신 이 검사는 `[dependencies]`를 **파싱한 이름 배열과 열한 개짜리
  리터럴을 통째로 비교**하므로, 이름이 하나라도 늘거나 줄거나 순서가 바뀌면 반드시 실패한다 —
  그리고 파서가 빈 배열을 냈다면 지금 통과할 수 없다. 검사가 값을 실제로 읽었다는 근거가
  통과 그 자체다.
- **`(d) 저장 직전의 붕괴 판정이 그대로 있다`** — 위반을 만들려면 `run.rs`를 고쳐야 한다.
  대신 같은 사실을 **행동으로** 판정하는 자리가 통합 테스트에 있다
  (`ch2_a_collapsed_result_is_blocked_even_when_it_arrived_in_chunks` — 130 문장짜리 붕괴가
  실제로 저장되지 않고 `TranscriptionOutputUnusable`로 끝난다). 원문 검사와 행동 검사가 같은
  규칙을 양쪽에서 본다.
