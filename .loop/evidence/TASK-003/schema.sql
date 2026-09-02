-- TASK-003 — migration 2 `create_domain_tables`가 만드는 스키마 (PRODUCT-SPEC §7)
-- 원본: src-tauri/src/db/migrations.rs  (MIGRATIONS[1].sql)
-- 이 파일은 읽기용 사본이다. 실행 주체는 migration이며, 여기서 실행되지 않는다.
--
-- 실제로 만들어진 열 목록은 `PRAGMA table_info`로 다시 확인했다:
--   src-tauri/tests/domain_model.rs
--   → the_four_concepts_live_in_four_separate_tables_with_the_fields_section_7_lists

CREATE TABLE IF NOT EXISTS recordings (
    id                    TEXT    PRIMARY KEY,
    title                 TEXT    NOT NULL,
    created_at            TEXT    NOT NULL,
    updated_at            TEXT    NOT NULL,
    duration_ms           INTEGER NOT NULL,
    audio_path            TEXT    NOT NULL,
    audio_format          TEXT    NOT NULL,
    microphone            TEXT,
    -- §7.2: 현재 사용 중인 성공한 Transcript. NULL(값 없음)도 정상 상태다.
    current_transcript_id TEXT,
    transcription_status  TEXT    NOT NULL
        CHECK (transcription_status IN ('none','pending','running','done','failed')),
    ai_status             TEXT    NOT NULL
        CHECK (ai_status IN ('none','pending','running','done','failed')),
    notion_status         TEXT    NOT NULL
        CHECK (notion_status IN ('none','pending','running','done','failed')),
    FOREIGN KEY (current_transcript_id, id)
        REFERENCES transcripts (id, recording_id)
);

-- §7.1: Recording 1:N Transcript. immutable · versioned.
CREATE TABLE IF NOT EXISTS transcripts (
    id           TEXT PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings (id),
    language     TEXT,
    raw_text     TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    engine       TEXT NOT NULL,
    model        TEXT NOT NULL,
    UNIQUE (id, recording_id)
);

CREATE TABLE IF NOT EXISTS transcript_segments (
    transcript_id TEXT    NOT NULL REFERENCES transcripts (id),
    ordinal       INTEGER NOT NULL,
    start_ms      INTEGER NOT NULL,
    end_ms        INTEGER NOT NULL,
    text          TEXT    NOT NULL,
    PRIMARY KEY (transcript_id, ordinal)
);

-- §7.1: Transcript 1:N AINote. Transcript와 별개의 테이블이다 (INV-2).
-- §7.3: transcript_id는 provenance의 일부다.
CREATE TABLE IF NOT EXISTS ai_notes (
    id             TEXT PRIMARY KEY,
    recording_id   TEXT NOT NULL REFERENCES recordings (id),
    transcript_id  TEXT NOT NULL REFERENCES transcripts (id),
    note_type      TEXT NOT NULL CHECK (note_type IN ('meeting','study','summary')),
    content        TEXT NOT NULL,
    -- INV-9: 벤더 중립 자유 식별자다. 허용 값 목록을 두지 않는다.
    provider       TEXT NOT NULL,
    model          TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    generated_at   TEXT NOT NULL,
    FOREIGN KEY (transcript_id, recording_id)
        REFERENCES transcripts (id, recording_id)
);

CREATE TABLE IF NOT EXISTS notion_syncs (
    recording_id TEXT PRIMARY KEY REFERENCES recordings (id),
    page_id      TEXT,
    synced_at    TEXT,
    status       TEXT NOT NULL
        CHECK (status IN ('none','pending','running','done','failed')),
    error        TEXT
);

CREATE INDEX IF NOT EXISTS idx_transcripts_recording
    ON transcripts (recording_id, created_at);
CREATE INDEX IF NOT EXISTS idx_ai_notes_transcript
    ON ai_notes (transcript_id, generated_at);
CREATE INDEX IF NOT EXISTS idx_ai_notes_recording
    ON ai_notes (recording_id);
