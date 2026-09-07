# TASK-075 — Acceptance Criteria가 무엇으로 판정되는가

Run: `RUN-20260907T065204Z-TASK-075` · 2026-09-07

**이 Task의 AC 다섯은 전부 `verification: verifier`다** (Gate는 하나도 enabled가 아니다 —
`stop_condition.gates: (none enabled)`). 그래서 이 문서는 **Verifier가 어디를 읽으면 되는지**를
가리키는 지도다.

---

## AC1 — ADR-0007 · ADR-0009 · ADR-0010에 이번 Phase의 결정과 구현 결과가 덧붙었고, 기존 문단이 삭제되지 않았다

| 문서 | 어디에 무엇이 덧붙었는가 | 무엇이 지워지지 않았는가 |
| --- | --- | --- |
| `docs/ADR-0007-transcription-engine.md` | **§19 신설** — §19.1 언어가 어디서 읽혀 어디까지 도달하는가 · §19.2 Metal이 켜졌는지와 그 판정 근거 · §19.3 전사 소요 시간이 어디에 남는가 · §19.4 §17과 달라진 다섯 · §19.5 `A-TRANS-001`의 상태 · §19.6 바꾼 것/바꾸지 않은 것 | §1~§18 전부. §14 · §16.3의 두 항목은 원래 문장을 취소선으로 **남긴 채** 갱신을 이어 붙였다 |
| `docs/ADR-0009-notion-and-export.md` | **§16 신설** — §16.1 왜 경로만으로 부족했는가(2026-09-05 관측) · §16.2 무엇이 그 자리를 여는가 · §16.3 왜 exports 아래로 제한되는가 · §16.4 틀렸을 때 · §16.5 바꾼 것/바꾸지 않은 것 | §1~§15 전부. 특히 §4(위치 · 파일명 · §4.3 충돌 정책)는 한 글자도 바뀌지 않았다 |
| `docs/ADR-0010-manual-ai-handoff.md` | **§12.8 신설** — §12.8.1 실측 · §12.8.2 예산의 출처 · §12.8.3 나눔 규칙 · §12.8.4 압축 모양과 §11 불변 · §12.8.5 틀렸을 때 · §12.8.6 바꾼 것/바꾸지 않은 것 | §1~§12.7 전부. §12.7(TASK-073이 쓴 command 표면 기록)도 그대로다 |

**덧붙은 내용이 실제 코드와 일치하는지 — 대조표**

| 문서의 서술 | 저장소의 자리 |
| --- | --- |
| 언어 설정이 migration 9의 nullable 열이고 NOT NULL도 DEFAULT도 없다 | `src-tauri/src/db/migrations.rs` version 9 |
| 기본값은 `Settings::DEFAULT`의 `None`이고 로캘을 짐작하지 않는다 | `src-tauri/src/domain/settings.rs` |
| 설정을 읽는 자리가 `commands/transcriber.rs` 하나이고 모델과 함께 읽는다 | `src-tauri/src/commands/transcriber.rs::transcribe_one` |
| 고르지 않음 = `Detect`, 공백만 있는 값도 `Detect` | `src-tauri/src/transcription/engine.rs::LanguageChoice::from_setting` |
| 두 갈래 모두 `set_language`와 `set_detect_language`를 부르고 `set_translate(false)`가 유지된다 | `src-tauri/src/transcription/whisper.rs::transcribe` |
| `Transcript.language`는 여전히 엔진이 보고한 값이다 | 같은 파일의 `get_lang_str(state.full_lang_id_from_state())` · `transcription/run.rs`의 `language: transcription.language` |
| **§17.1.4-2의 "출처 구분"은 Transcript에 값으로 남지 않았다** (§19.4의 달라진 것) | `src-tauri/src/domain/mod.rs`의 `Transcript`에 그런 필드가 없다 |
| `metal` feature가 켜져 있다 | `src-tauri/Cargo.toml`의 `whisper-rs = { version = "0.16", features = ["metal"] }` |
| 판정 근거가 빌드 산출물이다 (`GGML_METAL` OFF→ON · `libggml-metal.a` · 링크 지시 · 심볼) | `.loop/evidence/TASK-069/metal-feature-verification.md` §2.3 ~ §2.7 · `before-metal-off.txt` · `after-metal-on.txt` |
| feature 이름의 출처가 crate 소스가 아니라 cargo의 `declared_features`다 | `.loop/evidence/TASK-069/declared-features.txt` · 같은 문서 §1.1 · §3.2 |
| `Cargo.lock`이 바뀌지 않았고 그것이 정상이다 | 같은 문서 §3.3 |
| 재는 자리가 `Instant` 하나이고 영속화는 세지 않는다 | `src-tauri/src/transcription/run.rs::attempt`의 `started` · `elapsed_ms` |
| 열은 migration 10의 nullable이고 `DEFAULT 0`이 없다 | `src-tauri/src/db/migrations.rs` version 10 |
| 사람이 읽는 문장은 Rust가 만든다 | `src-tauri/src/commands/payload.rs`의 `transcription_label: … .map(format_duration_ms)` |
| 값이 없으면 화면이 그 줄을 그리지 않는다 | `src/screens/RecordingDetailScreen.tsx`의 `{tab.transcriptionLabel !== null && …}` |
| 여는 판정이 한 자리이고 exports 아래의 파일만 연다 | `src-tauri/src/commands/saved_file.rs::SavedFiles::show` |
| OS 호출이 `platform/` 안에만 있다 · `open -R` · `explorer /select,`가 실행된 적 없다 | `src-tauri/src/platform/file_manager.rs`(모듈 문서가 UNVERIFIED를 직접 적는다) · `src-tauri/tests/show_saved_file.rs` |
| 경로 문자열이 그대로 남는다 · 여는 수단은 추가다 | `src/screens/savedFileView.ts`의 `SHOW_FILE_TEXT` · `SHOW_FILE_RESOLUTION` · `tests/screen-boundary.test.ts` |
| 예산 40,000 B가 이 앱이 고른 값이고 Notion 제약이 아니다 | `src-tauri/src/export/portion.rs`의 `PORTION_MAX_BYTES`와 그 주석 · 테스트 `the_budget_is_this_apps_choice_and_not_the_notion_api_constraint` |
| 나눔이 문단 → 줄 → 문장 → 낱말 → 글자이고 이어 붙이면 원본이다 | 같은 파일의 `pack` · `Packer` · `assert_portions_are_sound` |
| 압축 모양이 있고 `render`는 언제나 `Sectioned`다 | `src-tauri/src/export/markdown.rs`의 `TranscriptShape` · `transcript_blocks` |
| §11 형식이 바뀌지 않았다 | `.loop/evidence/TASK-072/section-11-invariance.txt` |

---

## AC2 — `docs/PHASE-5.6-HUMAN-REVIEW.md`가 존재하고, 절차와 판정 항목이 구체적이며, 자동 Gate로 판정되지 않는다는 것을 문서가 직접 말한다

```text
docs/PHASE-5.6-HUMAN-REVIEW.md
```

| 요구 | 어디에 있는가 |
| --- | --- |
| **자동 Gate가 판정할 수 없다는 것을 문서가 직접 말한다** | **§0** — 다섯 항목마다 *"자동 검증이 이미 하는 것"* 과 *"왜 Gate가 판정할 수 없는가"* 를 나눠 적는다. 머리말의 ⚠️ 블록도 §8이 비어 있는 동안 무엇을 주장하지 않는지를 못박는다. §7이 확인하지 **않은** 것의 정본이다 |
| 사람이 실행할 절차 | §2(준비물 — 앱 실행 · 모델 · 언어 설정 · 녹음 조건) · §3(HR-1) · §4(HR-3) · §5(HR-2) · §6(HR-4 · HR-5) |
| **한국어 회의가 읽을 만하게 전사되는가** | §3 — 1a~1o. 판정은 1l(*"이 전사를 실제로 쓰겠는가"*) |
| **`ggml-base`로 충분한가** (`A-TRANS-001`의 실질적 답) | §5 — 같은 오디오를 모델만 바꿔 전사하고 9/5 · 9/7과 같은 방법으로 잰다. 판정은 2e |
| **Metal 전후로 체감이 달라지는가 (기록된 소요 시간 비교 방법 포함)** | §4 — §4.1이 **어디에 기록되는지**(화면 한 줄 · `transcripts.transcription_ms`의 SQL), §4.2가 **before를 만드는 절차**(Cargo.toml 한 줄을 되돌렸다 되돌린다)와 **왜 저장된 before가 없는지**, §4.3이 체감 판정 |
| **내보낸 파일을 실제로 열 수 있는가** | §6.1 — 4a~4j. 판정은 4d |
| **긴 회의의 handoff를 실제 AI 채팅에 넣을 수 있는가** | §6.2 · §6.3 · §6.4 — 5a~5o. 판정은 5g · 5h |
| 빈 기록표 | §8.1 ~ §8.7. **모든 칸이 `(비어 있음)` 또는 `☐ 미실행`이다** |
| 실패했을 때 무엇을 어디에 적는가 | §9 |

이 저장소의 기존 Human Review · smoke test 문서(`PHASE-3` · `PHASE-4` · `PHASE-5` ·
`PHASE-5.5` · `PHASE-5.7`)와 **같은 형태**다 — 머리말 블록 · ⚠️ 주장 금지 · 표기 등급 ·
판정하지 않는 것 · 절차 · 확인하지 않은 것의 정본 · 빈 기록표 · 기록 양식.

---

## AC3 — SYSTEM-MAP이 이번 Phase의 변경만 반영하고, Current / Planned / Deferred가 유지되며, 확인되지 않은 것이 완료로 적히지 않았다

| 요구 | 어디에서 확인하는가 |
| --- | --- |
| **이번 Phase의 변경만 반영** | 머리말 ⚠️ 문단 · 갱신 이력의 새 줄 · §1의 새 두 행 · §2의 새 흐름 블록과 전사 줄 · §3의 Phase 5.6 행 열 개 · §4의 Phase 5.6 문단 · §5의 Phase 5.6 절 · §6 두 칸 · §7의 새 여섯 항목 · §8의 새 다섯 행 · §9의 새 여덟 줄. **Phase 5.7 · 5.5 · 5 · 4 · 3 · 2 · 1의 절은 건드리지 않았다** |
| **Current / Planned / Deferred 구분 유지** | §1의 표에 `DONE(자동 검증 기준)` · `⚠️ 구현됐으나 사람이 판정하지 않음` · `PLANNED` · `DEFERRED` · `CANDIDATE` 행이 그대로 있다. §3의 새 행은 전부 `DONE`이되 **UNVERIFIED 단서가 괄호로 붙어 있다.** §5의 C-1 ~ C-6은 여전히 CANDIDATE다 |
| **사람이 확인하지 않은 것이 완료로 적히지 않았다** | §1의 새 행 하나가 통째로 *"⚠️ 구현됐으나 사람이 판정하지 않음 — Phase 5.6"* 이다. §5의 `⚠️ IMPLEMENTED / VERIFIED로 기록하지 않는 것` 블록에 여덟 줄이 있다(한국어 품질 · 모델 크기 · Metal 속도 · GPU 사용 · 파일 관리자 실행 · 예산 · 조각 사용성 · crate 소스). §7에 여섯 항목이 새로 있다. §6의 Human validation 칸에 다섯이 추가됐다 |
| **history를 덮어쓰지 않았다** | 머리말의 "부분 완료" 서술은 **그때의 기록으로 표시해 남겼다.** 갱신 이력의 Phase 5.7 줄도 그대로다. §5의 Phase 5.7 절과 그 자동 테스트 수(1,405)도 그대로다 |

---

## AC4 — 측정되지 않은 수치가 새로 들어가지 않았고, VERIFIED / UNVERIFIED가 구분돼 있다

**이 Run이 새로 적은 수치는 아래가 전부이며, 전부 출처가 있다.**

| 수치 | 출처 | 종류 |
| --- | --- | --- |
| 99 KB · 5,139줄 · 제목 1,711개 | `phase-prompt/05.6` R-5의 2026-09-05 실측 | 이미 기록된 관측 |
| 1,711 · 59 (3.4%) · 1,063회 (62.1%) · 72.85분 / 103 · 2 (1.9%) · 102회 (99.0%) · 50.99분 · 4.30분 | `ADR-0007` §18.1의 기존 표 (2026-09-05 · 2026-09-07) | 이미 기록된 관측 |
| `PORTION_MAX_BYTES = 40_000` · `CHUNK_MAX_BYTES = 60_000` | 저장소의 상수 | 코드에서 읽은 값 |
| migration version 9 · 10 | `src-tauri/src/db/migrations.rs` | 코드에서 읽은 값 |
| `libggml-metal.a` 1,681,096 B · `lint 20.1s` · `test 55.0s` · Gate timeout 900초 | `.loop/evidence/TASK-069/` · `.loop/project.yaml` | 이미 기록된 관측 |
| 자동 테스트 vitest **586** · Rust **885** (합계 1,471) | **Runtime이 TASK-074의 Run에서 스스로 실행한 `test` Gate의 로그** — `.loop-local/runs/RUN-20260907T062848Z-TASK-074/gates/test/stdout.log` (+ `gate-report.json`의 `"status": "PASS"` · `"exit_code": 0`). 확인 절차는 `gate-provenance.md` | **Runtime이 목격한 실행의 값** — 이 Run의 측정이 아니다 |

> ⚠️ **Attempt 1이 이 자리에서 틀렸고, 이번에 고쳤다.** Attempt 1은 같은 수치를 *"이 Run이 실제로
> 돌린 Gate의 출력"* 이라고 적었다. **TASK-075는 결정론적 Gate를 선언하지 않았고**
> (`gates: (none enabled)`) Runtime이 이 Run에 대해 목격한 실행이 없으므로, 그것은 목격되지 않은
> 실행을 근거로 든 진술이었다 (Verifier의 AC4 지적). 이번 Attempt는 **같은 수치를 Runtime이 직접
> 남긴 TASK-074의 gate 로그에 다시 걸었고**, 세 문서(`PHASE-5.6-HUMAN-REVIEW.md` §0.1 · §2.5 ·
> §7 · `SYSTEM-MAP.md` §5 · §6)가 전부 **"이 문서를 쓴 Run의 실행이 아니다"** 를 함께 적는다.
> 이 Run이 참고로 돌린 `loopctl self-check`는 스스로 *"advisory only — not a gate report"* 라고
> 말하며, **그 실행을 근거로 문서에 들어간 수치는 하나도 없다.**

**새로 지어낸 수치는 없다.** 특히:

```text
Metal 을 켜서 몇 배 빨라졌다        ← 적지 않았다. 측정한 적이 없다고 여러 자리에 적었다
전사가 몇 분 걸린다                 ← 적지 않았다. 기록된 4.30분·26분은 **붕괴한 디코딩**이라
                                      기준값이 아니라고 명시했다
압축으로 크기가 몇 % 줄었다         ← 적지 않았다. 압축 전후를 측정한 적이 없다
어떤 AI 채팅이 몇 자까지 받는다     ← 적지 않았다. UNVERIFIED 라고 적었다
```

`PHASE-5.6-HUMAN-REVIEW.md`의 화면 문구 예시에서도 **크기 숫자와 조각 번호는 자리표시자**로
두었다 (`About <글자 수> characters …` · `Part <몇 번째> of <전체>.`) — 이 문서가 아는 값이
아니기 때문이다.

**VERIFIED / UNVERIFIED 구분**

| 자리 | 어떻게 구분돼 있는가 |
| --- | --- |
| `ADR-0007` §19 | §3의 등급 표기([E1] ~ [E6])를 그대로 쓴다. §19.2는 feature 이름을 **[E1]로 올리지 않는다**고 명시하고 그 이유를 적는다. §19.5는 `A-TRANS-001`이 열려 있다고 적는다 |
| `ADR-0007` §14 · §16.3 | 갱신된 두 줄 모두 *"켜졌다는 것과 런타임에 GPU가 쓰인다는 것은 다른 진술"* 을 함께 적는다 |
| `ADR-0009` §16 | §16.2에 **[E4]** 문단(두 명령이 실행된 적 없다), §16.3은 [E1]로 코드 사실만 적는다 |
| `ADR-0010` §12.8 | §12.8.1이 실측을 [관측된 사실]로, §12.8.2가 채팅 한도를 **[E4] UNVERIFIED**로 적는다 |
| `PHASE-5.6-HUMAN-REVIEW.md` | §0.1의 등급 표([E1] ~ [E4]) · §7의 UNVERIFIED 정본 · `VERIFIED(자동)` / `VERIFIED(산출물)` / `UNVERIFIED` 블록 |
| `SYSTEM-MAP` | §5의 `⚠️ IMPLEMENTED / VERIFIED로 기록하지 않는 것` 블록 · §7의 새 여섯 항목 · §3 행마다 붙은 ⚠️ 단서 |

---

## AC5 — `docs/` 밖의 파일이 하나도 바뀌지 않았다

`changed-files.md` §1 · §2. 이 Run이 손댄 파일은 `docs/` 아래 다섯이며, working tree의 나머지
변경은 **앞선 Task들(TASK-066 ~ TASK-085)의 커밋되지 않은 산출물**이다.

참고로 **Gate는 이 Task에서 enabled가 아니다**(`gates: (none enabled)`). 문서 변경이 저장소를
깨뜨리지 않았다는 것을 보려고 `loopctl self-check`를 돌렸으나, 그것은 스스로 *"advisory only —
not a gate report"* 라고 말하는 실행이며 **완료 판정도 Evidence의 근거도 아니다** —
`self-check.md`가 그 성격을 적는다.
