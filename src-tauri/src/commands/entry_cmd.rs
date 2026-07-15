// mercury4win-linux/src-tauri/commands/entry_cmd.rs
// Entry query and mark Tauri commands

use deadpool_sqlite::Pool;
use tauri::State;
use crate::db::entry_store::{self, EntryQueryParams};
use crate::db::models::{EntryPage, EntryDetail};

#[tauri::command]
pub async fn load_entries(
    pool: State<'_, Pool>,
    feed_id: Option<i64>,
    unread_only: Option<bool>,
    search: Option<String>,
    tag_ids: Option<Vec<i64>>,
    tag_mode: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<EntryPage, String> {
    let cursor_id = cursor
        .and_then(|c| c.parse::<i64>().ok());

    entry_store::query_entries(
        &pool,
        &EntryQueryParams {
            feed_id,
            unread_only: unread_only.unwrap_or(false),
            search,
            tag_ids,
            cursor: cursor_id,
            limit: limit.unwrap_or(50),
        },
    )
    .await
}

#[tauri::command]
pub async fn mark_read(
    pool: State<'_, Pool>,
    entry_ids: Vec<i64>,
    is_read: bool,
) -> Result<(), String> {
    entry_store::mark_read(&pool, &entry_ids, is_read).await
}

#[tauri::command]
pub async fn mark_starred(
    pool: State<'_, Pool>,
    entry_id: i64,
    is_starred: bool,
) -> Result<(), String> {
    entry_store::mark_starred(&pool, entry_id, is_starred).await
}

#[tauri::command]
pub async fn delete_entry(pool: State<'_, Pool>, entry_id: i64) -> Result<(), String> {
    // Soft delete: mark is_deleted = 1
    let pool_clone = pool.inner().clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE entry SET is_deleted = 1 WHERE id = ?1",
                rusqlite::params![entry_id],
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn mark_all_read_in_feed(
    pool: State<'_, Pool>,
    feed_id: i64,
) -> Result<(), String> {
    entry_store::mark_all_read_in_feed(&pool, feed_id).await
}

#[tauri::command]
pub async fn load_entry_detail(
    pool: State<'_, Pool>,
    entry_id: i64,
) -> Result<EntryDetail, String> {
    entry_store::get_entry_detail(&pool, entry_id)
        .await?
        .ok_or_else(|| "Entry not found".to_string())
}
