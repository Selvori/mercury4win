// mercury4win-linux/src-tauri/db/connection.rs
// SQLite connection pool (deadpool-sqlite, WAL mode)

use deadpool_sqlite::{Config, Pool, Runtime};
use std::time::Duration;

/// Create a new connection pool with WAL mode.
pub async fn create_pool(db_path: &str) -> Result<Pool, Box<dyn std::error::Error>> {
    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let config = Config::new(db_path);
    let pool = config
        .builder(Runtime::Tokio1)?
        .max_size(8)
        .build()?;

    // Initialize WAL mode on first connection
    let conn = pool.get().await?;
    conn.interact(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\
             PRAGMA synchronous=NORMAL;\
             PRAGMA foreign_keys=ON;\
             PRAGMA busy_timeout=5000;"
        )
    }).await??;

    Ok(pool)
}

/// Retry wrapper for SQLITE_BUSY errors.
pub fn with_retry<T, F>(
    _conn: &rusqlite::Connection,
    f: F,
) -> Result<T, rusqlite::Error>
where
    F: Fn() -> Result<T, rusqlite::Error>,
{
    for attempt in 0..5 {
        match f() {
            Ok(val) => return Ok(val),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::DatabaseBusy =>
            {
                std::thread::sleep(Duration::from_millis(300 * (attempt + 1)));
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    f() // Last attempt, propagate error
}
