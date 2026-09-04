# TASK-048 — 긴 Markdown 문서를 순서대로 · 무손실로 나누는 순수 모듈

```text
Run:     RUN-20260904T051916Z-TASK-048
Date:    2026-09-04
Role:    impl
Gates:   lint PASS · test PASS  (self-check — 참고용이다. 판정은 Runtime이 다시 돌린다)
```

## 이 Run이 물려받은 상태

앞선 시도 `RUN-20260904T044650Z-TASK-048`이 **worker timeout으로 중단됐다** —
`.loop-local/runs/RUN-20260904T044650Z-TASK-048/`에 `worker-result.json`이 없고 evidence
디렉터리도 비어 있었다. 그러나 작업 트리에는 그 시도가 만든 `chunk.rs`와
`tests/notion_chunking.rs`가 남아 있었다.

이 Run은 그것을 **버리지도 무조건 믿지도 않았다.** 소스를 읽어 Acceptance Criteria와
ADR-0009 §6에 대조하고, Gate를 직접 돌려 실제로 통과하는지 확인한 뒤, **빠져 있던 것 하나**
(ADR-0009 §6.3이 요구하는 property 형태의 재조립 테스트)를 더했다.

## 만들어진 것

| 파일 | 무엇 |
| --- | --- |
| `src-tauri/src/notion/chunk.rs` | 순수 모듈 — 예산 상수 둘 · `AtomKind` · `OversizedAtom` · `split_markdown` |
| `src-tauri/tests/notion_chunking.rs` | 통합 테스트 10개. 입력은 §11 렌더러(`export::markdown::render`)의 실제 산출물이다 |
| `src-tauri/src/notion/mod.rs` | `pub mod chunk;` 와 재수출 (앞선 시도가 더한 줄) |

`split_markdown`이 돌려주는 chunk는 전부 **입력의 부분 슬라이스**(`Vec<&str>`)다. 새 문자열을
만들지 않으므로 "경계에서 개행을 먹거나 더한다"는 실패 양식이 타입 수준에서 거의 사라진다.

## Acceptance Criteria 대조

| AC | 무엇으로 판정되는가 | 결과 |
| --- | --- | --- |
| **P6-AC1** lint Gate | `npm run lint` = `eslint .` + `cargo clippy --all-targets -- -D warnings` | **PASS** exit=0 — `self-check.txt` · `gate-lint-*.log` |
| **P6-AC2** test Gate | `npm run test` = `vitest run` + `cargo test` | **PASS** exit=0 — `gate-test-*.log` |
| **P6-AC3** 재조립 동등성 + 1시간 규모 | 아래 표 | 테스트로 고정됨 |
| **P6-AC4** 원자 단위가 예산을 넘을 때 | 아래 표 | 명시적 `Err`. 조용한 절단 없음 |
| **P6-AC5** 예산 상수 표시 | `chunk.rs` 38~79행의 상수 주석 | 아래 §"예산 상수" |

### P6-AC3 — 이어 붙이면 원문과 정확히 같다

비교는 언제나 **원문 전체와 바이트 단위로** 한다 (`assert_eq!(chunks.concat(), markdown)`).
앞뒤 몇 글자나 길이만 보는 검사는 하나도 없다 — 가운데가 사라진 것을 놓치기 때문이다.

| Task가 요구한 입력 | 테스트 | 어디 |
| --- | --- | --- |
| **(a)** 예산보다 작아 나뉘지 않는 문서 | `a_document_under_the_budget_is_sent_as_one_piece` | `tests/notion_chunking.rs` |
| **(b)** 여러 chunk로 나뉘는 문서 | `a_document_over_the_budget_becomes_several_chunks_that_rejoin_into_the_original` | 같음 |
| **(c)** ★ **1시간 분량 transcript 규모** ★ | `an_hour_long_transcript_rejoins_byte_for_byte` | 같음 |
| **(d)** 원자 단위가 예산을 넘는 경우 | 아래 AC4의 넷 | 같음 |
| 임의의 문서 모양 (ADR-0009 §6.3의 property 형태) | `any_document_rejoins_into_exactly_what_went_in` | **이 Run이 더했다** |

**(c)가 정말 1시간 규모인가.** 손으로 지은 문자열이 아니라 실제 렌더러의 산출물이다 —
`duration_ms = 3,600,000`(1시간)인 Recording과 3초짜리 segment **1,200개**를 만들어
`export::markdown::render`에 넣고, 그 결과를 나눈다. 테스트가 먼저
`markdown.len() > 3 * CHUNK_MAX_BYTES`를 확인하므로 **한 chunk에 들어가 버려서 아무것도
검사하지 못하는 상태로 조용히 약해지지 않는다.** 이어서 chunk가 4개보다 많다는 것,
재조립이 원문과 같다는 것, segment 1,200개가 하나도 사라지지 않았다는 것
(`"번 구간입니다."` 출현 수 = 1,200)을 고정한다.

`any_document_rejoins_into_exactly_what_went_in`은 seed가 고정된 xorshift로 문서 64개를
만든다 — 길이 · 문단 · 코드 펜스 · 빈 줄 · 3바이트 문자 · **마지막 개행이 없는 문서**가 섞인다.
외부 crate를 들이지 않았고 seed가 고정이라 결과가 매번 같다.

### P6-AC4 — 담을 수 없는 단위를 만나면 자르지 않고 멈춘다

결정: **`Err(OversizedAtom)`**. 부분 결과를 성공으로 돌려주는 경로가 아예 없다
(반환형이 `Result<Vec<&str>, OversizedAtom>`이다). ADR-0009 §6.3-4의 결정 그대로다.

| 테스트 | 무엇을 고정하는가 |
| --- | --- |
| `a_line_that_cannot_fit_is_refused_instead_of_being_cut_silently` | 예산보다 긴 한 줄 → `Err`. 줄 번호 · 바이트 수 · 예산 · 종류(`Line`)를 말한다 |
| `nothing_is_truncated_when_an_atom_does_not_fit` | 앞뒤에 멀쩡한 문단이 있어도 `Ok(부분 목록)`이 나오지 않는다 — 나오면 `panic` |
| `a_code_block_that_cannot_fit_is_refused_too` | 펜스 안에서는 나누지 않으므로 (§6.3-3) 펜스 블록 하나도 원자 단위다 → `Err(CodeBlock)` |
| `an_atom_over_the_budget_is_the_only_reason_to_refuse` | **이 Run이 더했다.** 임의 문서 16개에 긴 줄을 섞어, 거절이 **오직 그 줄 때문에만** 일어나는지 (`error.bytes > CHUNK_MAX_BYTES`) 확인한다 |
| `the_failure_does_not_carry_the_document_into_its_message` | 실패 문장에 문서 내용이 실려 나가지 않는다 |

### P6-AC5 — 예산 상수는 [A]다

`CHUNK_MAX_BYTES = 60_000` · `CHUNK_MAX_BLOCK_UNITS = 300`. 두 주석이 전부 이렇게 적는다.

```text
⚠️ 이 값은 이 앱이 고른 값이다 (ADR-0009 §3의 [A]). 확인된 Notion API 한도가 아니다.
```

그리고 **무엇에서 나오지 않았는지**를 함께 적는다.

- markdown 엔드포인트 **전용** 본문 상한은 UNVERIFIED다 — 그런 값이 존재하는지조차 확인되지
  않았다. 750KB는 `chunk.rs` 57행에 **딱 한 번, "primary source에서 확인된 적이 없으므로 이
  상수의 근거가 아니다"라는 문장 안에서만** 나온다. API 사실로 적힌 자리도, 계산에 쓰인 자리도
  없다 — 다음 사람이 그 숫자를 어디선가 다시 만났을 때 "이미 확인해 봤고 근거가 아니다"를
  읽게 하려고 남긴 것이다.
- 옛 **2000자 rich text 규칙 · 100블록 배치 규칙**에서 유도하지 **않았다.** 그것은 블록 JSON
  경로(`children`/`content`)의 규칙이고 이 앱은 그 경로를 쓰지 않는다.

근거로 적힌 것은 VERIFIED된 일반 한도 하나뿐이다 — 요청당 500KB overall. 그 관계를
`the_byte_budget_stays_under_the_verified_general_request_limit`가 **컴파일 시점 `const` 검사**로
고정한다(`60,000 × 6 < 500,000`). 예산을 키우려는 다음 사람은 테스트를 돌리기 전에 막힌다.

## 이 모듈이 알지 않는 것

`the_chunking_module_knows_nothing_about_the_network_storage_or_files`가 `chunk.rs` 소스를
읽어 `ureq` · `std::net` · `TcpStream` · `std::fs` · `File::` · `rusqlite` · `Connection` ·
`SystemTime` · `Instant`가 **하나도 없음**을 확인한다. 같은 입력은 언제나 같은 결과를 낸다.

## 이 Run이 하지 않은 것

- chunk를 실제로 **보내는** 순서 · 재시도 · 영속화 — ADR-0009 §8 · §9이며 이 Task 밖이다.
- ADR-0009 §6.3의 경고 그대로, **나눠 보낸 결과가 한 번에 보낸 것과 같은 블록 구조가 되는지는
  이 모듈이 보장할 수 없다** [E4]. 그것은 Phase Goal의 Human Review 항목이며, 여기 있는 어떤
  자동 테스트도 그것을 통과했다고 말하지 않는다.

## 파일 목록

```text
self-check.txt            self-check 출력과 각 Gate가 실제로 돌린 명령
gate-lint-stdout.log      lint Gate 원문
gate-lint-stderr.log
gate-test-stdout.log      test Gate 원문 (전체)
gate-test-stderr.log      실행된 테스트 바이너리 목록 — notion_chunking 바이너리가 여기 있다
chunking-test-run.txt     위 원문에서 chunk 관련 줄만 추린 것 (원문 줄 번호 포함)
task-048-new-files.diff   chunk.rs · notion_chunking.rs 전체 (아직 추적되지 않는 새 파일이다)
changed-files.txt         git status --porcelain
```
