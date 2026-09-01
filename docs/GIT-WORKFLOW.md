# Git / GitHub Workflow — Molt Note

이 문서는 저장소 운영 정책이다. **제품 요구사항이 아니다.**
제품 사양은 `docs/PRODUCT-SPEC.md`, Runtime 운영 지침은 `CLAUDE.local.md`에 있다.

---

## 1. Repository

```text
https://github.com/JJleem/molt-note
Visibility:      PUBLIC
Default branch:  main
Remote:          origin
```

**이 저장소는 공개다.** 아래 모든 규칙의 근거가 그것이다.

---

## 2. Commit 단위 — Phase

**Task마다 commit하지 않는다.**

```text
ONE COMPLETED PHASE  =  ONE REVIEWED PHASE COMMIT
```

Runtime은 Task별 Git history를 필요로 하지 않는다. Worker의 중간 작업은
`.loop-local/`의 Run 산출물로 이미 남으며, 그것이 Runtime의 실행 기록이다.
Git은 **완료되고 검증된 상태의 snapshot**을 보존하는 수단이다.

## 3. Commit 시점

```text
Phase Plan → Human Review → Plan Approval → Tasks → Gates → Verifier
  → Phase Signoff → (필요시 Human Review) → PHASE COMPLETE
  → Git safety review → Commit → Push
```

**Phase가 완료되기 전에 final phase commit/push를 하지 않는다.**

다음 상태를 Phase 완료 commit으로 만들지 않는다:

```text
failing Gate · unfinished Task · verifier failure · unresolved required Human Review
```

## 4. Commit message

Conventional commit을 쓴다. **실제 구현보다 과장하지 않는다.**

```text
feat(phase-1): establish application foundation
feat(phase-2): add reliable local recording
feat(phase-3): add local whisper transcription
feat(phase-4): add local AI note provider
feat(phase-5): add notion sync and markdown export
feat(phase-6): validate windows support
chore(final): complete molt note v1 verification
```

Windows 검증이 끝나지 않았다면 `support Windows`라고 쓰지 않는다.
Phase 결과가 message와 다르면 **message를 고친다.**

## 5. Push

Phase commit 후 `main`에 push한다. 개인 프로젝트이므로 PR workflow는 기본 요구사항이 아니다.
협업이 필요해지면 그때 branch/PR 정책을 도입한다.

## 6. Force push 금지

```text
git push --force        금지
git push -f             금지
```

remote history를 재작성하지 않는다. Push conflict는 원인을 조사해서 정상적인 workflow로 푼다.
**이미 public에 push된 history는 사용자가 명시적으로 요청하지 않는 한 rewrite하지 않는다.**

---

## 7. Public Repository Safety — 매 push 전

`.gitignore`에 있다고 **가정하지 않는다.** staged 내용을 실제로 확인한다.

```bash
git status
git diff --cached --name-only
git diff --cached
```

staged content에 다음이 있으면 **push하지 않는다**:

```text
API token · password · API key (Anthropic · Gemini · Groq · Notion 등)
개인 녹음 · private transcript · 실제 회의/스터디 내용이 담긴 AI Note
local SQLite database · secret이 담긴 사용자 설정
.loop-local/ 의 실행 상태 · 예상치 못한 대용량 바이너리
```

`.gitignore` 규칙은 §9에 있고, 그 규칙이 실제로 동작하는지는
`git check-ignore <path>`로 확인할 수 있다.

---

## 8. Runtime state와 Git을 구분한다

**Git으로 Runtime 상태를 바꾸려 하지 않는다.**

```text
Plan state · Task state · READY state · Gate result · Verification result · Approval
```

이 전부는 `loopctl`이 소유한다. Git은 소스와 문서의 snapshot일 뿐이다.
Task 파일(`.loop/tasks/*.yaml`)이 저장소에 커밋되는 것은
**Runtime이 그것을 추적되는 프로젝트 상태로 정의하기 때문이며**,
Git에서 그 파일을 편집해 상태를 바꾸는 것은 허용되지 않는다.

### Worker는 commit하지 않는다

Phase commit은 **운영자(대화형 세션)의 작업**이다. Runtime Worker의 작업이 아니다.

Worker가 commit하면 HEAD가 바뀌어 Gate와 Verifier가 묶여 있는
subject fingerprint가 실행 도중에 흔들린다. `.loop/KERNEL.md` §6이 이미
`git push`·force push·history 재작성을 금지하며, **commit도 하지 않는다.**

---

## 9. `.gitignore` 정책

`.gitignore`가 최소한 다음을 제외한다. 실제 규칙은 저장소 루트의 `.gitignore`에 있다.

| 범주 | 대상 |
| --- | --- |
| Runtime local state | `.loop-local/` (단, 빈 `.gitkeep`은 유지 — §10) |
| Secrets | `.env` · `.env.*` (`.env.example`은 예외) |
| Node | `node_modules/` · `dist/` |
| Rust / Tauri | `/target/` · `/src-tauri/target/` · `/src-tauri/gen/schemas` |
| OS metadata | `.DS_Store` · `Thumbs.db` |
| Local application data | `*.db` · `*.db-shm` · `*.db-wal` · `*.sqlite` · `*.sqlite3` |
| 사용자 콘텐츠 | `/recordings/` · `/transcripts/` · `/exports/` |
| Audio | `*.wav` · `*.mp3` · `*.m4a` · `*.aac` · `*.flac` · `*.webm` · `*.ogg` |
| Local models | `/models/` · `*.gguf` · `*.bin` |

**경로가 루트에 고정(`/`)된 규칙이 여럿인 것은 의도적이다.**
`models/` · `recordings/` 같은 이름은 제품 소스 디렉터리로도 흔히 쓰인다
(예: `src/models/recording.ts`). 무앵커 규칙은 그런 소스를 조용히 삼킨다.

### Test fixture 예외

향후 테스트에 작은 media fixture가 필요하면 blanket 규칙을 우회해
`.gitignore`에서 **명시적으로 allowlist**한다.

```gitignore
!/tests/fixtures/synthetic-tone.wav
```

**실제 개인 녹음이 아니라 synthetic fixture만 허용한다.**

---

## 10. `.loop/` 와 `.loop-local/`

두 디렉터리는 정책이 정반대다. 혼동하지 않는다.

| | 내용 | Git |
| --- | --- | --- |
| `.loop/` | Runtime **contract / control plane** — `KERNEL.md` · `DESIGN.md` · `project.yaml` · `policies/` · `skills/` · `tasks/` | **커밋한다.** Runtime이 추적되는 프로젝트 상태로 정의한다 |
| `.loop-local/` | Runtime **실행 상태** — plans · runs · executions · leases · staging · 로그 | **커밋하지 않는다** |

단 `.loop-local/`의 **빈 `.gitkeep` 파일 5개는 커밋한다.**
Starter Pack의 `.gitignore`가 의도적으로 이것만 남기며,
`loopctl doctor`가 이 디렉터리들의 존재를 확인하기 때문이다 —
없으면 새로 clone한 저장소에서 doctor가 실패한다.
**이것은 실행 상태가 아니라 디렉터리 구조 자체다** (전부 0바이트).

`.loop-local/` 안의 생성된 Plan 상태 · 실행 기록 · 로그 · 임시 산출물은
어떤 경우에도 커밋하지 않는다.

---

## 11. Phase 완료 보고에 포함할 Git 항목

모든 Phase 완료 보고서는 다음을 포함한다. **push 결과를 누락하지 않는다.**

```text
Git

Commit:   <hash>
Message:  <commit message>
Branch:   main
Remote:   origin
Push:     PASS / FAIL / NOT ATTEMPTED
Working tree:  clean / dirty
Public repository safety check:  PASS / FAIL
```
