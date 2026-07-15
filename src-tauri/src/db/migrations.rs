// mercury4win-linux/src-tauri/db/migrations.rs
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
pub const SCHEMA_VERSION: i32 = 1;

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
        conn.execute_batch(MIGRATION_V1)?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('schema_version', ?1)",
            [SCHEMA_VERSION.to_string()],
        )?;
    }

    // Future migrations go here:
    // if version < 2 { ... }

    Ok(())
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
}
