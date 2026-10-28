// mercury4win/src-tauri/db/migrations.rs
// Database migration that mirrors the original Mercury schema

use rusqlite::Connection;

const MIGRATION_V1: &str = "
-- Feed subscriptions
CREATE TABLE feed (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT,
    feed_url TEXT NOT NULL UNIQUE,
    site_url TEXT,
    feed_parser_version INTEGER,
    last_fetched_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Entry items
CREATE TABLE entry (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_id INTEGER NOT NULL REFERENCES feed(id) ON DELETE CASCADE,
    guid TEXT,
    url TEXT,
    title TEXT,
    author TEXT,
    published_at TEXT,
    summary TEXT,
    is_read INTEGER NOT NULL DEFAULT 0,
    is_starred INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX idx_entry_feed_guid ON entry(feed_id, guid);
CREATE UNIQUE INDEX idx_entry_feed_url ON entry(feed_id, url);

-- Content pipeline artifacts (layered persistence)
CREATE TABLE content (
    entry_id INTEGER PRIMARY KEY REFERENCES entry(id) ON DELETE CASCADE,
    html TEXT,
    cleaned_html TEXT,
    readability_title TEXT,
    readability_byline TEXT,
    readability_version INTEGER,
    markdown TEXT,
    markdown_version INTEGER,
    display_mode TEXT NOT NULL DEFAULT 'cleaned',
    document_base_url TEXT,
    pipeline_type TEXT NOT NULL DEFAULT 'default',
    resolved_intermediate_content TEXT
);

-- Rendered reader HTML cache (keyed by theme + entry)
CREATE TABLE content_html_cache (
    theme_id TEXT NOT NULL,
    entry_id INTEGER NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    html TEXT NOT NULL,
    reader_render_version INTEGER,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (theme_id, entry_id)
);

-- Tags
CREATE TABLE tag (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    is_provisional INTEGER NOT NULL DEFAULT 0,
    usage_count INTEGER NOT NULL DEFAULT 0
);

-- Tag aliases (merge support)
CREATE TABLE tag_alias (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tag_id INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    alias_normalized TEXT NOT NULL UNIQUE
);

-- Entry-tag junction
CREATE TABLE entry_tag (
    entry_id INTEGER NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    source TEXT NOT NULL DEFAULT 'manual',
    confidence REAL,
    PRIMARY KEY (entry_id, tag_id)
);

-- Entry notes
CREATE TABLE entry_note (
    entry_id INTEGER PRIMARY KEY REFERENCES entry(id) ON DELETE CASCADE,
    markdown TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- AI provider profiles
CREATE TABLE agent_provider_profile (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    display_name TEXT,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_archived INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- AI model profiles (per provider)
CREATE TABLE agent_model_profile (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES agent_provider_profile(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    model_name TEXT NOT NULL,
    temperature REAL,
    top_p REAL,
    max_tokens INTEGER,
    is_streaming INTEGER NOT NULL DEFAULT 1,
    supports_summary INTEGER NOT NULL DEFAULT 1,
    supports_translation INTEGER NOT NULL DEFAULT 1,
    supports_tagging INTEGER NOT NULL DEFAULT 1,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_archived INTEGER NOT NULL DEFAULT 0,
    last_tested_at TEXT
);

-- Agent profiles (one per agent type)
CREATE TABLE agent_profile (
    agent_type TEXT PRIMARY KEY,
    primary_model_id TEXT REFERENCES agent_model_profile(id),
    fallback_model_id TEXT REFERENCES agent_model_profile(id),
    target_language TEXT,
    detail_level TEXT,
    prompt_strategy TEXT,
    concurrency_degree INTEGER
);

-- Agent task runs
CREATE TABLE agent_task_run (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_id INTEGER NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    task_type TEXT NOT NULL,
    status TEXT NOT NULL,
    agent_profile_id TEXT,
    provider_profile_id TEXT,
    model_profile_id TEXT,
    target_language TEXT,
    duration_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- LLM usage events
CREATE TABLE llm_usage_event (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_run_id INTEGER REFERENCES agent_task_run(id),
    provider_name TEXT,
    model_name TEXT,
    agent_type TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    request_status TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Translation results (keyed by entry + language)
CREATE TABLE translation_result (
    entry_id INTEGER NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    target_language TEXT NOT NULL,
    source_content_hash TEXT NOT NULL,
    segmenter_version TEXT NOT NULL,
    run_status TEXT NOT NULL,
    PRIMARY KEY (entry_id, target_language)
);

-- Translation segments
CREATE TABLE translation_segment (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entry_id INTEGER NOT NULL,
    target_language TEXT NOT NULL,
    source_segment_id TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    source_text_snapshot TEXT NOT NULL,
    translated_text TEXT,
    FOREIGN KEY (entry_id, target_language) REFERENCES translation_result(entry_id, target_language) ON DELETE CASCADE
);

-- Summary results
CREATE TABLE summary_result (
    entry_id INTEGER NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    target_language TEXT NOT NULL,
    detail_level TEXT NOT NULL,
    text TEXT NOT NULL,
    output_language TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (entry_id, target_language, detail_level)
);

-- Settings store (key-value)
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// Current schema version. Increment when migrations are added.
pub const SCHEMA_VERSION: i32 = 10;

const MIGRATION_V3: &str = "
CREATE TABLE agent_result_slot (
    slot_key TEXT PRIMARY KEY,
    entry_id INTEGER NOT NULL REFERENCES entry(id) ON DELETE CASCADE,
    task_type TEXT NOT NULL,
    target_language TEXT NOT NULL,
    parameters TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    success_json TEXT,
    checkpoint_json TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX agent_result_entry ON agent_result_slot(entry_id, task_type);
";

const MIGRATION_V2: &str = "
ALTER TABLE entry ADD COLUMN feed_content TEXT;
ALTER TABLE entry ADD COLUMN feed_content_type TEXT;
-- V1 merged explicit bodies and teasers into summary; their origin cannot be
-- reconstructed. Keep that raw field, but discard only identifiable derived
-- reader artifacts so an old teaser is not reused as successful full text.
DELETE FROM content_html_cache WHERE entry_id IN (
    SELECT c.entry_id FROM content c JOIN entry e ON e.id = c.entry_id
    WHERE c.html = e.summary AND trim(e.summary) <> ''
);
DELETE FROM content WHERE entry_id IN (
    SELECT c.entry_id FROM content c JOIN entry e ON e.id = c.entry_id
    WHERE c.html = e.summary AND trim(e.summary) <> ''
);
";

/// Run all migrations on a fresh or existing database.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    let version: i32 = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v.parse().unwrap_or(0))
        .unwrap_or(0);

    if version < 1 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(MIGRATION_V1)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('schema_version', ?1)",
            ["1"],
        )?;
        tx.commit()?;
    }

    if version < 2 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(MIGRATION_V2)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('schema_version', ?1)",
            ["2"],
        )?;
        tx.commit()?;
    }

    if version < 3 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(MIGRATION_V3)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('schema_version', ?1)",
            ["3"],
        )?;
        tx.commit()?;
    }

    if version < 4 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(crate::db::tag_batch_store::MIGRATION_V4)?;
        crate::db::tag_store::migrate_normalization(&tx)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('schema_version','4')",
            [],
        )?;
        tx.commit()?;
    }
    if version < 5 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(crate::db::usage_store::MIGRATION_V5)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('schema_version','5')",
            [],
        )?;
        tx.commit()?;
    }
    if version < 6 {
        let tx = conn.unchecked_transaction()?;
        crate::db::search_store::migrate(&tx)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('schema_version','6')",
            [],
        )?;
        tx.commit()?;
    }
    if version < 7 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(crate::db::topic_store::MIGRATION_V7)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('schema_version','7')",
            [],
        )?;
        tx.commit()?;
    }
    if version < 8 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(crate::db::topic_auto_store::MIGRATION_V8)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('schema_version','8')",
            [],
        )?;
        tx.commit()?;
    }
    if version < 9 {
        let tx = conn.unchecked_transaction()?;
        crate::db::topic_search_store::migrate(&tx)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('schema_version','9')",
            [],
        )?;
        tx.commit()?;
    }
    if version < 10 {
        let tx = conn.unchecked_transaction()?;
        crate::db::staged_search_store::migrate(&tx)?;
        tx.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('schema_version','10')",
            [],
        )?;
        tx.commit()?;
    }
    Ok(())
}

/// Build an actual historical schema, rather than downgrading a current DB's
/// version marker while leaving later columns/tables in place.
#[cfg(test)]
pub(crate) fn historical_fixture(version: i32) -> Connection {
    assert!((1..=9).contains(&version));
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    conn.execute_batch(MIGRATION_V1).unwrap();
    if version >= 2 {
        conn.execute_batch(MIGRATION_V2).unwrap();
    }
    if version >= 3 {
        conn.execute_batch(MIGRATION_V3).unwrap();
    }
    if version >= 4 {
        conn.execute_batch(crate::db::tag_batch_store::MIGRATION_V4)
            .unwrap();
        crate::db::tag_store::migrate_normalization(&conn).unwrap();
    }
    if version >= 5 {
        conn.execute_batch(crate::db::usage_store::MIGRATION_V5)
            .unwrap();
    }
    if version >= 6 {
        crate::db::search_store::migrate(&conn).unwrap();
    }
    if version >= 7 {
        conn.execute_batch(crate::db::topic_store::MIGRATION_V7)
            .unwrap();
    }
    if version >= 8 {
        conn.execute_batch(crate::db::topic_auto_store::MIGRATION_V8)
            .unwrap();
    }
    if version >= 9 {
        crate::db::topic_search_store::migrate(&conn).unwrap();
    }
    conn.execute(
        "INSERT INTO settings(key,value) VALUES('schema_version',?1)",
        [version.to_string()],
    )
    .unwrap();
    conn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_migration() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn).unwrap();

        // Verify key tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"feed".to_string()));
        assert!(tables.contains(&"entry".to_string()));
        assert!(tables.contains(&"content".to_string()));
        assert!(tables.contains(&"tag".to_string()));
        assert!(tables.contains(&"entry_tag".to_string()));
        assert!(tables.contains(&"agent_provider_profile".to_string()));
        assert!(tables.contains(&"translation_result".to_string()));
        assert!(tables.contains(&"translation_segment".to_string()));
        assert!(tables.contains(&"summary_result".to_string()));
    }

    #[test]
    fn test_version_tracking() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn).unwrap();

        let version: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(version, SCHEMA_VERSION.to_string());
    }

    #[test]
    fn test_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run_migrations(&conn).unwrap();
        // Running again should not error
        run_migrations(&conn).unwrap();
    }

    #[test]
    fn v2_preserves_legacy_data_and_invalidates_only_merged_reader_artifacts() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        conn.execute_batch(MIGRATION_V1).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO settings VALUES ('schema_version', '1');
            INSERT INTO feed(id,feed_url) VALUES(1,'https://example.test/feed');
            INSERT INTO entry(id,feed_id,guid,summary,is_read,is_starred) VALUES
                (1,1,'teaser','legacy snippet',1,1),
                (2,1,'fetched','another snippet',0,0),
                (3,1,'full','legacy complete Feed body',0,0);
            INSERT INTO content(entry_id,html,markdown) VALUES
                (1,'legacy snippet','wrong snippet markdown'),
                (2,'<article>Downloaded body</article>','Downloaded markdown'),
                (3,'legacy complete Feed body','legacy full markdown');
            INSERT INTO content_html_cache(theme_id,entry_id,html,updated_at) VALUES
                ('light',1,'wrong snippet','now'),('light',2,'full page','now'),
                ('light',3,'legacy full page','now');
            INSERT INTO entry_note VALUES(1,'keep note','now');
            INSERT INTO tag(id,name,normalized_name) VALUES(1,'Keep','keep');
            INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,1);
            INSERT INTO summary_result VALUES(1,'zh','brief','saved AI summary','zh','now');
            INSERT INTO translation_result VALUES(1,'zh','hash','1','completed');
            INSERT INTO translation_segment(entry_id,target_language,source_segment_id,
                order_index,source_text_snapshot,translated_text)
                VALUES(1,'zh','p1',0,'source','saved translation');
        "#,
        )
        .unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
        let state: (String, bool, bool, Option<String>) = conn
            .query_row(
                "SELECT summary,is_read,is_starred,feed_content FROM entry WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(state, ("legacy snippet".into(), true, true, None));
        let legacy_body: String = conn
            .query_row("SELECT summary FROM entry WHERE id=3", [], |r| r.get(0))
            .unwrap();
        assert_eq!(legacy_body, "legacy complete Feed body");
        for table in [
            "feed",
            "content",
            "content_html_cache",
            "entry_note",
            "tag",
            "entry_tag",
            "summary_result",
            "translation_result",
            "translation_segment",
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 1, "{table} must preserve unrelated/user data");
        }
        let fetched: String = conn
            .query_row("SELECT markdown FROM content WHERE entry_id=2", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(fetched, "Downloaded markdown");
        let violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(violations, 0);
    }
}
