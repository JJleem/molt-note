# TASK-113 — docs/MEETING-NOTE-FORMAT.md changes

Changed file: `docs/MEETING-NOTE-FORMAT.md` only (no source/tests touched by this Run).
Note: Bash is restricted to `loopctl self-check`, so no `git diff` output could be captured here;
the verifier should run `git diff -- docs/MEETING-NOTE-FORMAT.md`. Other working-tree changes
(src-tauri/**, src/**) pre-existed this Run (from earlier Tasks of Phase 5.12).

## (1) Time-recording instructions → content-based
- §1 📌 notice: the "구간 with lots of missing speech" is now pointed to by what was being discussed, not time.
- §4 (old line 94): "전사의 시각을 `> 인용` 블록으로 안내" → "그 구간이 무슨 이야기 중이었는지를 `> 인용` 블록으로 안내 … 전사 시각은 적지 않는다".
- §6 (old line 120): "(전사 시각과 함께)" → "(무슨 이야기 중에 나온 말인지와 함께)".
- §6 (old line 122): "구간을 시각과 함께" → "무슨 내용 근처인지로 가리킨다 (예: 장비 예산 이야기 중 반복 인식 구간)".

## (2) Where tables go
- §4: tables live in the group's table field `topics[].groups[].table { columns, rows }`
  (matches `TopicGroup.table: Option<TopicTable>` in src-tauri/src/ai/note.rs), rendered under the
  topic as a separate table block with blank lines around it; never table syntax inside a bullet.

## (3) "하지 않는 것"
- Added: do not write transcript times (`[HH:MM:SS]` etc.) anywhere.
- Added: no table syntax inside bullet items — tables only in the table field.

Remaining time-like text is not transcript time: header `이벤트 시간`/`일시` (meeting time),
`[YYMMDD]` title date, `📅` due dates.
