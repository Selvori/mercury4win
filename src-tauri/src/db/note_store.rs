// mercury4win-linux/src-tauri/db/note_store.rs
// Entry notes persistence

use deadpool_sqlite::Pool;
use rusqlite::params;
use crate::db::models::EntryNote;

pub async fn get_note(pool: &Pool, entry_id: i64) -> Result<Option<EntryNote>, String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let result = conn.query_row(
                "SELECT entry_id, markdown, updated_at FROM entry_note WHERE entry_id = ?1",
                params![entry_id],
                |row| {
                    Ok(EntryNote {
                        entry_id: row.get(0)?,
                        markdown: row.get(1)?,
                        updated_at: row.get(2)?,
                    })
                },
            );
            match result {
                Ok(n) => Ok(Some(n)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        })
        .await
        .map_err(|e| e.to_string())?
}

pub async fn save_note(pool: &Pool, entry_id: i64, markdown: &str) -> Result<(), String> {
    let md = markdown.to_string();
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT INTO entry_note (entry_id, markdown, updated_at)
                 VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(entry_id) DO UPDATE SET markdown=excluded.markdown, updated_at=datetime('now')",
                params![entry_id, md],
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_note(pool: &Pool, entry_id: i64) -> Result<(), String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute("DELETE FROM entry_note WHERE entry_id = ?1", params![entry_id])
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}
