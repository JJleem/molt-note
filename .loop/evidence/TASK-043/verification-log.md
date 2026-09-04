# TASK-043 — 확인 기록

이 Task는 **문서 하나만 만든다** (`docs/ADR-0009-notion-and-export.md`).
소스 · 설정 · 의존성 · 테스트는 건드리지 않았다.

Gate: `.loop/project.yaml`에 이 Task로 켜진 Gate가 없다 (`stop_condition.gates: []`).
그래서 build / lint / test를 실행하지 않았고, 그 사실을 Result의 `notes`에도 적었다.

---

## 1. 네트워크 접근 — 시도했고 거부됐다

ADR §10.3 · §11.3이 요구하는 crate 사실(`ureq`의 TLS feature 이름 · `keyring`의 플랫폼
feature 구성)을 primary source에서 확인하려 했다. 전부 거부됐다.

```text
WebFetch  https://docs.rs/crate/ureq/3.4.0/features
          → "Claude requested permissions to use WebFetch, but you haven't granted it yet."

WebFetch  https://crates.io/api/v1/crates/keyring
          → "Claude requested permissions to use WebFetch, but you haven't granted it yet."

WebSearch "ureq 3.4 cargo features rustls native-tls platform-verifier default features"
          → "Claude requested permissions to use WebSearch, but you haven't granted it yet."
```

로컬 cargo registry도 이 세션의 작업 디렉터리 밖이라 읽을 수 없었다.

```text
ls /Users/molt/.cargo/registry/src
  → blocked: "For security, Claude Code may only list files in the allowed working
     directories for this session: '/Users/molt/orca/projects/molt-note'"
```

**결론**: `ureq`의 TLS feature 이름과 `keyring`의 feature 구성은 **UNVERIFIED**로 남았다.
ADR은 그 둘을 [E4]로 표시하고, 지어낸 이름을 적는 대신 **구현 Task가 확인할 방법과,
틀렸을 때 조용히 넘어가지 않게 하는 판정 수단**을 결정에 포함시켰다 (ADR §10.3-3 · §11.4).

Notion API 사실은 다시 조사하지 않았다. `docs/PRODUCT-SPEC.md` §14.9.1 · §14.9.2가
**이 Phase의 계획 시점(2026-09-04)에** primary source에서 확인해 기록한 값이며, ADR은 그것을
[E2]로 인용한다.

---

## 2. 저장소에서 직접 읽어 확인한 것 ([E1])

아래는 전부 이 Run에서 실제 파일을 읽어 확인했고, ADR §3.2의 표가 같은 내용을 담는다.

| 확인 | 파일 | 확인한 값 |
| --- | --- | --- |
| `notion_syncs` 스키마 | `src-tauri/src/db/migrations.rs` | `recording_id TEXT PRIMARY KEY REFERENCES recordings(id)` · `page_id` · `synced_at` · `status` CHECK(`none/pending/running/done/failed`) · `error` |
| 다음 migration 번호 | 같음 | 적용된 version은 1~6 → **다음은 7** |
| settings 규약 | 같음 (version 3~6 주석) | 열을 더한다 · `NOT NULL`/`DEFAULT` 없음 · NULL은 '아직 없음' · **secret 열을 만들지 않는다** |
| upsert 동작 | `src-tauri/src/db/store.rs` | `save_notion_sync`가 `ON CONFLICT (recording_id) DO UPDATE` |
| 시각 형식 | 같음 | `strftime('%Y-%m-%dT%H:%M:%fZ','now')` (UTC 텍스트) |
| 날짜 자르기 선례 | `src-tauri/src/commands/mod.rs::title_for` | `created_at[0..10]`/`[11..16]`을 그대로 쓰고 **시간대를 계산하지 않는다** |
| 경로 파생 | `src-tauri/src/platform/app_data_dir.rs` | 루트 하나에서 `molt-note.db` · `recordings/` · `models/`. 테스트는 임시 루트 주입 |
| platform 경계 | `src-tauri/src/platform/mod.rs` | 현재 모듈 셋 (`app_data_dir` · `clock` · `microphone`) |
| 파일명 충돌 선례 | `src-tauri/src/audio/capture.rs` | `output_path`가 `stem.wav` → `stem-2.wav` … `MAX_PATH_ATTEMPTS = 1_000`, 덮어쓰지 않고 실패 |
| HTTP 경계의 현재 모양 | `src-tauri/src/ai/ollama/http.rs` | `HttpMethod`는 `Get`·`Post`뿐 · `HttpRequest`에 헤더 자리 없음 · `HttpResponse`는 `status`+`body`뿐(**응답 헤더를 읽을 수 없다**) · `TransportError`에 문자열 없음 |
| 실제 소켓 파일의 취급 | `src-tauri/src/ai/ollama/network.rs` | "자동 검증은 이 파일을 실행하지 않는다 — Gate가 컴파일한다" |
| frontend 영속화 | `src/` 전체 grep | `localStorage` · `sessionStorage` 사용 **0건** |

### 2.1 TLS — 이 저장소는 지금 HTTPS를 열 수 없다

```text
$ grep -n -A16 'name = "ureq"' src-tauri/Cargo.lock
4211:name = "ureq"
4212-version = "3.4.0"
4214-checksum = "972d7902c8735f2695410b8aed7df6ed12a47394aa1c8d7af49f0497b731a94d"
4215-dependencies = [
4216- "base64 0.23.1",
4217- "log",
4218- "percent-encoding",
4219- "ureq-proto",
4220- "utf8-zero",
4221-]

$ grep -nE '^name = "(rustls|ring|webpki|native-tls|security-framework|keyring|...)' src-tauri/Cargo.lock
   → TLS 구현 crate 일치 없음 (deranged · derive_more 등 접두사 우연 일치만 나왔다)
```

`src-tauri/Cargo.toml`: `ureq = { version = "3.4", default-features = false }`

**이 결과가 ADR §11.4의 판정 기준의 before 값이다** — 구현 Task가 TLS feature를 켠 뒤
`Cargo.lock`에 TLS 구현 crate가 나타나지 않으면 켜지지 않은 것이다.

---

## 3. 이 Run이 만든 파일

```text
docs/ADR-0009-notion-and-export.md          (신규)
.loop/evidence/TASK-043/verification-log.md (이 파일)
```

`src-tauri/` · `src/` · `Cargo.toml` · `Cargo.lock` · `package.json` · 테스트는
**하나도 바뀌지 않았다** (P1-AC2).

---

## 4. Acceptance Criteria 대응

| AC | 어디에서 답하는가 |
| --- | --- |
| P1-AC1 (여덟 결정 + 근거) | ADR §2 요약표 · §4(export 위치/파일명/충돌) · §5(엔드포인트·헤더) · §6(chunk 예산) · §7(allow_async) · §8(중복 sync·영속화 시점) · §9(Retry-After) · §10(SecretStore·crate) · §11(ureq TLS). '나중에 정한다'로 남긴 항목 없음 — §10.3-4 · §11.3의 대체 경로는 **발동 조건이 명시된 결정**이다 |
| P1-AC2 (문서만 변경) | 위 §3 |
| P1-AC3 (chunk 예산은 앱이 고른 값) | ADR §3의 **[A] 표기 도입** · §6.1(VERIFIED/UNVERIFIED 분리) · §6.2("확인된 API 한도가 아니다") · §12 표. **750KB는 어디에도 사실로 적히지 않았고**, 언급되는 자리는 "확인되지 않았으므로 적지 않는다"는 문장뿐이다 |
| P1-AC4 (`Notion-Version`) | ADR §5.2 — 값은 `2026-03-11` 하나. 같은 절의 표가 `2026-09-01`·`2026-09-04`(조사 날짜)와 `2026-07-28`(MCP 프로토콜 버전)을 헤더 값이 아니라고 명시한다 |
| P1-AC5 (VERIFIED/UNVERIFIED 구분) | ADR §3의 등급표 · §3.1(네트워크 거부 원문) · §12의 전체 표. `keyring` feature 구성과 `ureq` TLS feature 이름은 **[E4] UNVERIFIED**로 적혔다 |
