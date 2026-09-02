# ADR-0001 — 로컬 영속 저장소는 Rust 경계 안의 `rusqlite`로 한다

```text
Status:   Accepted
Date:     2026-09-01
Phase:    Phase 1 — Application Foundation
Task:     TASK-002
Scope:    persistence 기술 선택 · DB 위치 · migration 모델
```

---

## 1. Context

Phase 1은 "앱을 껐다 켜도 데이터가 그대로 있는" 기반을 요구한다
(`phase-prompt/01-application-foundation.md` Required Outcome 2).
`docs/PRODUCT-SPEC.md` §14.7은 후보 두 개(`tauri-plugin-sql` · `rusqlite`)를 조사해 두고
**선택 자체는 Phase 1에 위임**했으며, 사용 전에 crate와 버전을 다시 확인하고
확인하지 못한 것은 UNVERIFIED로 남기라고 요구한다.

이 문서는 그 선택과 근거, 그리고 **이 Run에서 실제로 확인한 것과 확인하지 못한 것**을 기록한다.

## 2. Decision

1. 로컬 영속 저장소는 **SQLite**이며, 접근은 **Rust 경계 안의 `rusqlite`** 가 소유한다.
   프론트엔드는 SQL을 실행하지 않는다.
2. `rusqlite`는 **`bundled` feature**로 쓴다 — SQLite C 소스를 함께 빌드하므로
   실행 환경에 설치된 SQLite 버전에 의존하지 않는다.
3. DB 파일 위치는 persistence 모듈이 정하지 않는다.
   `AppDataDirectory::database_path()`(TASK-001, INV-10 경계)가 돌려주는 경로를 받는다.
4. 스키마는 **코드에 선언된 순서 있는 migration 목록**으로 관리한다 (§5).

구현: `src-tauri/src/db/mod.rs` · `src-tauri/src/db/migrations.rs`.

## 3. 왜 `tauri-plugin-sql`이 아닌가

§14.7이 정리한 근거를 그대로 따른다.

| 근거 | 설명 |
| --- | --- |
| persistence ownership | SQL 실행 주체가 Rust에 남는다. `tauri-plugin-sql`은 프론트엔드가 SQL을 실행하는 모델이다. |
| domain/repository 경계 | 저장소 접근이 Rust 경계 하나로 모이면 §7의 domain 규칙(예: Transcript immutability, INV-2)을 한 곳에서 강제할 수 있다. 프론트엔드가 임의 SQL executor면 그 규칙은 관례에 불과해진다. |
| INV-7 (secret 경계) | 이후 Phase에서 secret·privacy 경계가 Rust 쪽에 생긴다. 저장소 경계가 같은 쪽에 있어야 일관된다. |
| §18 (검증 가능성) | 하드웨어·DOM 없이 Rust 단위 테스트만으로 저장소를 검증할 수 있다. 이 Task의 테스트가 그렇게 동작한다. |
| §14.5와의 일관성 | Ollama 호출도 Rust backend에 두는 방향이므로 외부 자원 접근 주체가 갈리지 않는다. |

> **UNVERIFIED:** "`tauri-plugin-sql`은 sqlx 기반이며 프론트엔드가 SQL을 실행한다"는
> 서술은 §14.7의 선행 조사(v2.tauri.app/plugin/sql · crates.io)를 인용한 것이다.
> TASK-002의 어느 시도에서도 웹 조회 도구 호출이 허용되지 않아 해당 공식 출처를 다시 열어
> 재확인하지 못했다 (2026-09-02 시도에서도 `WebFetch` 권한이 거부됐다).
> 다만 이 결정의 핵심 근거(persistence ownership을 Rust에 두고 싶다)는 외부 사실이 아니라
> 이 저장소의 불변 규칙에서 나오므로, 재확인 결과와 무관하게 유지된다.

## 4. 실제로 쓰인 crate — 확인 상태

`VERIFIED (local)`은 **이 저장소에서 재현 가능한 산출물**로 확인했다는 뜻이다.
`UNVERIFIED`는 확인하지 못했다는 뜻이며, 추측으로 메우지 않았다.

| 주장 | 값 | 상태 | 근거 |
| --- | --- | --- | --- |
| crate 이름 | `rusqlite` | VERIFIED (local) | `src-tauri/Cargo.toml` 의존성 선언 |
| 해석된 버전 | **0.40.2** | VERIFIED (local) | `src-tauri/Cargo.lock` — `name = "rusqlite"` / `version = "0.40.2"` / `checksum = 23f2a97d…b5b3` |
| feature 이름 | **`bundled`** | VERIFIED (local) | `features = ["bundled"]`로 선언한 상태에서 `cargo clippy`·`cargo test`가 exit 0으로 끝났다. cargo는 존재하지 않는 feature 이름에 대해 빌드를 실패시키므로, 이 feature가 0.40.2에 존재한다는 것은 빌드 결과로 확인된다. |
| `bundled`가 SQLite C 소스를 함께 빌드한다 | 그렇다 | VERIFIED (local) | 빌드 산출물 `src-tauri/target/debug/build/libsqlite3-sys-*/out/` 에 `sqlite3.o`와 `libsqlite3.a`가 생성됐다 (시스템 SQLite에 링크만 했다면 없을 산출물이다) |
| 전이 의존성 | `libsqlite3-sys` **0.38.2** | VERIFIED (local) | `src-tauri/Cargo.lock` |
| 실제로 번들된 SQLite 버전 | **3.53.2** | VERIFIED (local) | 생성된 바인딩 `…/out/bindgen.rs`: `SQLITE_VERSION = "3.53.2"` (`SQLITE_VERSION_NUMBER = 3053002`) |
| crates.io가 `^0.40`에 대해 주는 최신 버전 | 0.40.2 | VERIFIED (local) | 의존성을 처음 추가할 때 cargo가 crates.io 인덱스를 갱신하고 0.40.2를 해석·다운로드했다 (`.loop/evidence/TASK-002/gate-lint.log`: `Updating crates.io index` → `Adding rusqlite v0.40.2` → `Downloaded rusqlite v0.40.2`). cargo는 요구 범위 안의 가장 높은 버전을 고른다. |
| `rusqlite` 전체의 최신 버전 (0.41 이상의 존재 여부) | — | **UNVERIFIED** | 웹 조회 도구 사용이 허용되지 않아 crates.io 페이지/API를 직접 열지 못했다. 위 항목은 `^0.40` 범위 안의 최신만 말해 주며, 그보다 새로운 계열이 있는지는 확인하지 못했다. §14.7은 2026-09-01 조사에서 0.40.2를 최신으로 적었으나 **이 저장소 안에서 재확인한 것은 아니다.** |
| docs.rs의 `bundled` feature 공식 설명 | — | **UNVERIFIED** | 같은 이유로 docs.rs를 열지 못했다. 위의 feature 관련 두 항목은 **공식 문서가 아니라 로컬 빌드 산출물**로 확인한 것이다. |
| Windows에서의 빌드 | — | **UNVERIFIED** | macOS에서만 검증했다. Windows 검증은 Phase 6이 다룬다. |

> **외부 출처 재확인 시도 기록.** 2026-09-02 시도에서 `WebFetch`로 `crates.io/api/v1/crates/rusqlite`를
> 열어 위 UNVERIFIED 두 항목을 해소하려 했으나 도구 권한이 거부됐다. 추측으로 메우지 않고
> UNVERIFIED로 남긴다. 웹 조회가 가능한 환경에서 이 두 줄을 갱신하면 된다.

> §14.8은 `rusqlite`의 `bundled` feature 이름을 "P1에서 확인해야 하는 것"으로 남겨 뒀다.
> 이 Task에서 **로컬 빌드로** 확인했다. 공식 문서 재확인은 여전히 하지 않았으므로,
> 위 표는 그 둘을 구분해 적는다.

## 5. Migration 모델

```text
PRAGMA user_version   = 현재 스키마 버전 (판단 기준)
schema_migrations     = 무엇이 언제 적용됐는지에 대한 사람용 기록
MIGRATIONS: &[Migration] = 코드에 선언된 순서 있는 목록
```

- `open()`은 DB를 열고 `apply_pending()`을 호출한다. `user_version`보다 큰 버전의 migration만,
  선언된 순서대로 적용한다. 이미 적용된 것은 건너뛴다 — 그래서 **몇 번을 실행해도 안전하다.**
- migration 하나는 **트랜잭션 하나**다(스키마 변경 · 적용 기록 · `user_version` 갱신).
  중간에 실패하면 이전 버전 상태로 롤백되고, 부분 적용된 스키마가 남지 않는다.
- DB의 버전이 코드가 아는 최신 버전보다 **크면 열지 않고 멈춘다**
  (`DatabaseError::AheadOfCode`). 더 새로운 앱이 만든 데이터를 다운그레이드하며
  망가뜨리지 않기 위해서다.

### 5.1 데이터를 지우는 경로를 만들지 않는다 (INV-4의 정신)

스키마가 바뀔 때 저장소를 비우고 다시 만드는 경로는 **없다.**
migration 실행 코드에 `DROP TABLE` · `DELETE FROM` · DB 파일 삭제 후 재생성이 없고,
`no_migration_destroys_existing_data` 테스트가 선언된 migration SQL에 파괴적 문장이
들어오는 것을 막는다.

### 5.2 새 migration을 추가하는 규칙

```text
새 migration은 항상 MIGRATIONS 목록 끝에 추가한다.
이미 적용된 적이 있는 migration의 version·sql은 고치지 않는다.
잘못된 스키마는 "되돌리는" 대신 새 migration으로 앞으로 고친다.
```

이미 그 스키마로 저장된 사용자 DB가 존재할 수 있고, 그것을 지우는 것은 선택지가 아니다.

### 5.3 현재 목록

| version | name | 내용 |
| --- | --- | --- |
| 1 | `create_schema_migrations` | 적용 기록 테이블 생성 |
| 2 | `create_domain_tables` | §7 도메인 테이블 생성 (§5.4) |
| 3 | `create_settings` | §5 D의 설정 테이블 생성 (§5.5) |

### 5.5 설정 저장 (TASK-005)

> 이 절은 TASK-005(2026-09-02)에서 추가했다. §5.2의 규칙대로 **목록 끝에** migration 3을
> 추가했고, migration 1·2는 고치지 않았다.

구현: `src-tauri/src/domain/settings.rs`(설정 타입과 기본값 정책) ·
`src-tauri/src/db/settings.rs`(저장·복원). 검증: `src-tauri/tests/settings_repository.rs`.

| 결정 | 이유 |
| --- | --- |
| 설정은 **한 행짜리 테이블**이다 (`id INTEGER PRIMARY KEY CHECK (id = 1)`) | 설정 집합은 하나뿐이다. 행이 여럿 생겨 "어느 것이 진짜인가"를 물어야 하는 상태를 스키마가 애초에 허용하지 않는다. key-value 테이블 대신 이름 있는 열을 쓰므로, 저장 가능한 값의 목록이 스키마에 드러난다 — 임의의 키(예: token)가 조용히 들어올 자리가 없다 (INV-7). |
| 열에 `DEFAULT` 절을 두지 않는다. 기본값은 `Settings::DEFAULT`에 있다 | 기본값 정책이 코드에 있으면 정책이 바뀔 때 이미 만들어진 사용자 DB를 고치지 않아도 된다. 또 "저장된 적 없음"(행 없음)과 "기본값과 같은 값을 저장함"이 구분된다 — 끈 토글이 '저장한 적 없음'으로 오해되지 않는다. |
| 기본값: `recordings_directory = None` · `automatic_processing = false` | 사용자가 고르기 전에 임의의 디렉터리를 설정 값으로 굳히지 않는다. 값이 없을 때 실제로 어느 위치를 쓸지는 녹음 파일을 실제로 만드는 Phase가 정한다 — domain에 OS 경로를 박지 않는다 (INV-10). 자동 실행은 켜는 것이 사용자의 명시적 선택이므로 기본은 OFF다. |
| 저장은 `ON CONFLICT (id) DO UPDATE` | 지웠다 다시 넣지 않으므로 갱신 중간에 "설정이 없는 상태"가 생기지 않는다. |
| `automatic_processing`은 `INTEGER` + `CHECK (… IN (0, 1))` | SQLite에 boolean 타입이 없다. 0/1 외의 값이 조용히 저장되지 않게 하고, 읽을 때 모르는 값은 추측 없이 `DatabaseError::Decode`로 실패한다. |
| 토글은 **하나**다 | §5 D는 transcription · AI · Notion 각각의 automatic 토글을 적지만, 그 기능들은 이 Phase의 범위 밖이다. 값을 담을 자리만 미리 만들지 않는다 (§20.6). 필요한 Phase가 새 migration으로 앞으로 추가한다. |
| secret은 스키마에도 API에도 없다 (INV-7) | API key · integration token 입력과 보관은 Phase 1의 Out of Scope다. 저장하지 않는 값은 담을 곳도 없어야 한다. 테스트가 `settings` 테이블의 열 목록과 설정 모듈의 코드에 secret 성격의 이름이 없는지 확인한다. |

### 5.4 §7 데이터 모델을 스키마로 옮길 때의 결정 (TASK-003)

> 이 절은 TASK-003(2026-09-02)에서 추가했다.
> §7은 "개념 모델이며 합리적인 변경은 가능하되 **변경 이유를 기록한다**"라고 요구하고,
> §7.2는 "정확한 SQL FK와 migration 구현은 Phase 1의 architecture에 맞게 설계할 수 있다.
> 고정된 것은 domain semantics이지 특정 스키마 형태가 아니다"라고 적었다.
> 아래는 그 재량 안에서 내린 결정과 이유다.

구현: `src-tauri/src/domain/mod.rs`(domain 타입) · `src-tauri/src/db/store.rs`(저장·복원) ·
migration 2. 검증: `src-tauri/tests/domain_model.rs` · `src-tauri/tests/domain_invariants.rs`.

#### 바꾸지 않은 것 (확정된 domain 규칙)

- `Recording 1:N Transcript` · `Transcript 1:N AINote` (§7.1).
- Transcript는 immutable이다. **기존 Transcript 행을 UPDATE하는 경로를 만들지 않았다**
  (§7.1 · INV-2). 저장소가 노출하는 것은 `append_transcript` 하나뿐이고,
  `INSERT OR REPLACE`도 쓰지 않는다 — 같은 id로 다시 쓰면 조용히 덮어쓰는 대신 실패한다.
- `currentTranscriptId`의 '값 없음'은 정상 상태다 (§7.2). 스키마에서 NULL을 허용한다.
- 상태 필드는 `none · pending · running · done · failed` 다섯을 구분한다.
- `AINote.provider`는 벤더 중립 자유 식별자다 (INV-9). 허용 값 목록도, 벤더 enum도 없다.

#### §7의 표기와 달라진 것과 그 이유

| §7 표기 | 구현 | 이유 |
| --- | --- | --- |
| `segments[] { start · end · text }` | 자식 테이블 `transcript_segments (transcript_id, ordinal, start_ms, end_ms, text)` | 배열을 JSON 문자열로 한 열에 넣으면 저장소가 그 내용을 검증하지 못하고, 구간 단위 질의(§11의 타임스탬프 렌더링)가 문자열 파싱에 의존하게 된다. `ordinal`이 순서를 보존하므로 §7의 배열 의미는 그대로다. Transcript가 immutable이므로 이 자식 행들도 추가만 되고 갱신되지 않는다. |
| `duration` · `start` · `end` | `duration_ms` · `start_ms` · `end_ms` (INTEGER, 밀리초) | 단위를 이름에 박아 두면 초/밀리초 혼동이 생기지 않는다. 부동소수 대신 정수를 쓰므로 저장·복원이 정확히 일치한다(테스트가 값 동일성으로 판정한다). |
| `createdAt` · `updatedAt` · `generatedAt` · `syncedAt` | ISO-8601 **TEXT**이며 **호출자가 값을 준다** | 시간 crate를 지금 추가하지 않는다(필요해진 Phase에서 결정한다). domain이 시계를 갖지 않으므로 테스트가 결정론적이다. |
| `type (meeting \| study \| summary)` | 열 이름 `note_type` | `type`은 SQL 예약어와 혼동되기 쉽다. 값 집합은 그대로이며 CHECK로 고정했다. |
| `AINote` | Rust 타입 이름 `AiNote` | Rust 명명 규약(연속 대문자 회피). 개념은 동일하다. |
| `currentTranscriptId` 등 camelCase | 열은 snake_case (`current_transcript_id`) | SQL/Rust 관례. 이름만 다르고 대응은 1:1이다. |
| `content (§9.3의 structured note)` | 해석하지 않는 **TEXT** | §9.3은 최종 schema를 Phase 4가 확정한다고 적었다. 지금 구조를 고정하면 Phase 4의 결정을 앞당겨 잠그게 된다. domain은 이 값을 파싱하지 않는다. |
| (명시 없음) | 상태 열에 `CHECK (… IN ('none','pending','running','done','failed'))` | 다섯 상태의 구분을 관례가 아니라 스키마 제약으로 표현한다. 여섯 번째 값이 조용히 저장되지 않는다. |
| (명시 없음) | `recordings.current_transcript_id`는 **복합 FK** `(current_transcript_id, id) → transcripts (id, recording_id)` | §7.2의 "이 Recording의 성공한 Transcript"를 제약으로 강제한다. 남의 Recording의 Transcript를 current로 지정할 수 없다. SQLite는 FK 열 중 하나가 NULL이면 제약을 만족한 것으로 보므로 '값 없음' 상태는 그대로 허용된다. |
| (명시 없음) | `ai_notes`도 복합 FK `(transcript_id, recording_id) → transcripts (id, recording_id)` | §7.3의 provenance가 어긋나는 조합(노트의 recordingId와 transcriptId가 서로 다른 Recording을 가리키는 상태)을 저장할 수 없게 한다. |
| `language` · `microphone` · `pageId` · `syncedAt` · `error` | nullable | "아직 모른다 / 해당 없음"이 정상 상태인 값들이다. 모르는 값을 빈 문자열로 위장하지 않는다. |
| `NotionSync` | `recording_id`가 PK (Recording당 최대 하나) | §7이 `recordingId`만 식별자로 적었고 전송 이력이 아니라 **현재 전송 상태**를 뜻하므로 1:1로 뒀다. 이력이 필요해지면 새 migration으로 앞으로 고친다(§5.2). |

#### 지금 결정하지 않은 것

- **삭제 시 동작.** FK에 `ON DELETE` 절을 두지 않았다(SQLite 기본 동작 = 부모 행 삭제 거부).
  삭제 기능 자체가 아직 없고, §7도 삭제 의미를 정하지 않았다. INV-4가 자동 삭제를 금지하므로
  기본 거부가 더 안전한 출발점이다. 삭제를 구현하는 Phase가 이 결정을 다시 한다.
- **id 생성 규칙.** id는 문자열이며 형식을 강제하지 않는다. 생성 주체는 이 Task의 범위 밖이다.
- **AI 호출 · 전사 · Notion 전송.** 이 Task는 스키마와 저장/복원만 다룬다.

## 6. Consequences

- 프론트엔드는 SQL을 실행할 수 없다. 저장소가 필요한 화면은 Tauri command를 통해야 한다.
- `bundled`는 빌드 시 C 컴파일러를 요구한다. macOS에서는 §14.2가 확인한 Xcode Command Line
  Tools로 충분하며, 실제로 빌드됐다 (§4의 빌드 산출물). Windows 쪽은 UNVERIFIED다 (§4).
- SQLite 버전이 사용자 환경이 아니라 빌드에 고정되므로, 동작 차이가 사용자 머신마다 갈리지 않는다.
- 저장소 검증은 임시 디렉터리 DB만으로 가능하다. 실제 사용자 앱 데이터 디렉터리를 건드리는
  테스트가 없다.

## 7. 이 결정이 다루지 않는 것

- Settings 영속화 · Recording repository의 상위 사용 흐름 — 각각 후속 Task가 다룬다.
  (§7 도메인 스키마 자체는 TASK-003에서 migration 2로 추가했다 — §5.4.)
- 백업 · 암호화 · 원격 동기화 — Product Spec 범위 밖이며 지금 결정하지 않는다.
