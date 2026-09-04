# TASK-032 검증 로그 — ADR-0008을 쓰면서 실제로 확인한 것

Run: RUN-20260903T054825Z-TASK-032 · 2026-09-03

이 Task는 문서만 만든다. 따라서 Evidence는 "테스트가 통과했다"가 아니라
**문서에 적은 사실이 어디서 왔는지**와 **저장소가 실제로 바뀐 범위**다.

---

## 1. 네트워크 접근 — 없었다 (이것이 근거 등급을 정한다)

Task는 §14.5의 값을 **도입 시점에 재확인**하라고 요구한다. 이 Run은 두 가지 도구로
시도했고 **둘 다 거부됐다.**

```text
WebFetch   https://raw.githubusercontent.com/ollama/ollama/main/docs/api.md
           → "Claude requested permissions to use WebFetch, but you haven't granted it yet."

WebSearch  "Ollama API docs structured outputs format JSON schema /api/generate stream false"
           → "Claude requested permissions to use WebSearch, but you haven't granted it yet."
```

**그래서 ADR-0008은 §14.5의 어떤 값도 "2026-09-03에 재확인했다"고 적지 않는다.**
그 항목들의 등급은 [E2]이고 **확인 시점은 2026-09-01**이며, 그 사실을 문서 §3.1과
§14.1에 명시했다.

> ADR-0007(TASK-023)의 [E2]는 확인 시점이 결정일과 같은 날(2026-09-03)이었다.
> 이 문서의 [E2]는 **이틀 전**이다. 그 차이를 문서 §3.1이 숨기지 않고 적는다.

**이 제약이 설계를 바꿨다.** §14.5의 세부값이 틀렸을 때 무너지는 범위를 줄이는 세 결정이
그것 때문에 들어갔다 (ADR-0008 §3.1).

1. 가용성을 문서가 아니라 실행 중인 서버에게 묻는다 (§6.4)
2. `format` 파라미터에 정확성을 걸지 않는다 (§6.2)
3. 모델 존재를 생성 응답의 상태 코드로 판정하지 않는다 (§13.3)

**추측으로 채운 값은 없다.** 엔드포인트 경로 · 파라미터 이름 · crate 버전 · API 시그니처를
확인된 것처럼 적지 않았다. 확인하지 못한 것은 ADR-0008 §14.2의 21개 항목 표에
UNVERIFIED로 남겼고, 구현 Task가 확인할 목록을 §14.3에 넘겼다.

---

## 2. 저장소에서 직접 읽어 확인한 것 ([E1])

| 확인한 사실 | 파일 |
| --- | --- |
| `ai_notes` 열 구성과 `content TEXT NOT NULL`(내용을 해석하지 않음), `(transcript_id, recording_id)` 복합 FK, 인덱스 `idx_ai_notes_transcript (transcript_id, generated_at)` | `src-tauri/src/db/migrations.rs` (migration 2) |
| 적용된 migration은 1~5이며 다음 번호는 **6**. 이미 적용된 migration의 version·name·sql 변경을 막는 테스트가 존재 (`released_migrations_keep_their_version_and_name`) | 같음 |
| `settings` 테이블이 단일 행이고 secret 열이 없으며, 새 설정 값은 **새 migration으로 열을 더하는** 방식으로만 추가돼 왔다 (version 3 → 4 → 5) | 같음 |
| **`ai_notes`를 UPDATE·DELETE하는 경로가 없다.** `insert_ai_note` · `load_ai_note` · `list_ai_notes_for_transcript` 셋뿐이고 `INSERT OR REPLACE`도 쓰지 않는다 | `src-tauri/src/db/store.rs` |
| `update_recording_statuses`가 세 상태와 `updated_at`만 바꾼다 (AI 실패가 건드릴 수 있는 유일한 열) | 같음 |
| `domain::AiNote.provider`가 벤더 enum이 아니라 자유 문자열이며 복원 시 목록과 대조하지 않는다 (INV-9). `NoteType {Meeting, Study, Summary}`가 이미 존재하고 문자열과 왕복한다 | `src-tauri/src/domain/mod.rs` · `store.rs::decode_ai_note` |
| 현재 `FailureKind`는 8종이며 **AI provider 실패는 하나도 없다.** 모듈 주석이 "만들지 않은 실패의 자리를 미리 만들어 두지 않는다"를 규칙으로 적어 두었다. `TranscriptionOutputUnusable`은 `retryable: false`("같은 입력은 같은 출력을 낸다") | `src-tauri/src/domain/failure.rs` · `transcription/engine.rs` |
| frontend `FailureKind` union이 Rust와 1:1이고 `unexpected`만 frontend 전용 | `src/ipc/failure.ts` |
| `Settings`에 secret 필드가 없고, 기본값 정책이 스키마가 아니라 `Settings::DEFAULT`에 있으며, "그 기능을 구현하는 Phase가 설정 항목을 함께 추가한다"가 명시돼 있다 | `src-tauri/src/domain/settings.rs` |
| **의존성에 HTTP 클라이언트도 async runtime도 없다.** `tauri` · `serde` · `serde_json` · `rusqlite 0.40` · `cpal 0.18` · `hound 3` · `whisper-rs 0.16` · `rubato 5` | `src-tauri/Cargo.toml` |
| 전사 경계가 **동기 trait + test double**(`TranscriptionEngine` / `testing::StubEngine`)이고 실행 경계와 파싱(`parse.rs`)이 분리돼 있다 | `src-tauri/src/transcription/engine.rs` |
| `.loop/evidence/**`는 gitignore 대상이 아니라 추적되는 경로다 (기존 TASK 증거가 커밋돼 있다) | `.gitignore` · `git ls-files .loop/evidence` |

이 목록이 ADR-0008 §3.2와 같다.

---

## 3. 변경 범위 — 문서 하나 + 이 Run의 Evidence (AC4)

`git status --porcelain` 실행 결과 (ADR 작성 후):

```text
?? .loop/tasks/TASK-032.yaml      ← Runtime이 만든 Task 파일 (이 Run 이전부터 untracked)
?? .loop/tasks/TASK-033.yaml
?? .loop/tasks/TASK-034.yaml
?? .loop/tasks/TASK-035.yaml
?? .loop/tasks/TASK-036.yaml
?? .loop/tasks/TASK-037.yaml
?? .loop/tasks/TASK-038.yaml
?? .loop/tasks/TASK-039.yaml
?? .loop/tasks/TASK-040.yaml
?? .loop/tasks/TASK-041.yaml
?? .loop/tasks/TASK-042.yaml
?? docs/ADR-0008-note-ai-provider.md   ← 이 Run이 만든 유일한 제품 문서
```

- **수정된 tracked 파일이 하나도 없다** (`M` 항목 없음).
- `src/**` · `src-tauri/src/**` · `Cargo.toml` · `package.json` · `tauri.conf.json` ·
  `docs/SYSTEM-MAP.md` 전부 그대로다.
- `docs/PRODUCT-SPEC.md`도 고치지 않았다.
- `.loop/tasks/*.yaml`은 Runtime이 Plan 승인 시점에 만든 파일이며 이 Run은 건드리지 않았다.
- **commit을 만들지 않았다.** `git log -1`은 이 Run 전후로 `f5a18b2`다.
- 이 Run이 추가로 쓴 파일은 Runtime이 허용한 `.loop/evidence/TASK-032/` 아래 두 개뿐이다
  (`verification-log.md` · `changed-files.txt`). 제품 파일이 아니라 Runtime 산출물이며,
  TASK-023이 같은 위치에 같은 종류의 파일을 남긴 전례가 있다.

## 4. Gate

이 Task에 활성화된 Gate가 없다 (`stop_condition.gates: (none enabled)`).
따라서 `loopctl self-check`를 실행하지 않았다. 문서만 바뀌었으므로 build · lint · test의
대상이 되는 파일에는 변경이 없다.

---

## 5. Acceptance Criteria가 문서의 어디에 있는가

| AC | 위치 |
| --- | --- |
| **AC1** (a)~(j) 열 개 항목의 결정과 근거 | ADR-0008 §2(요약표) · (a)§4 · (b)§5 · (c)§6 · (d)§7 · (e)§8 · (f)§9 · (g)§10 · (h)§11 · (i)§12 · (j)§13 |
| **AC2** 세 모드 schema 확정 (필드명·타입·필수 여부, §9.5 전 섹션 표현) | ADR-0008 §7.1(명명 규칙과 §9.3 예시와의 차이) · §7.2(세 표) · §7.4(검증) · §7.5(`ai_notes.content` 봉투) |
| **AC3** §14.5 재확인 결과의 VERIFIED / UNVERIFIED 구분 | ADR-0008 §3(등급 정의) · §3.1(재확인 불가 사실) · §14(21개 항목표 · 확인 시점 · 출처) · 이 파일 §1 |
| **AC4** 문서 외 파일 변경 없음 · commit 없음 | 이 파일 §3 |
| **AC5** cloud adapter · Agent SDK · Ollama 번들링 · 가격 의존을 계획에 넣지 않음 | ADR-0008 §15 (표) · §11.3 · §13.4 · §6.4 |
