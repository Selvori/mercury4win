// mercury4win/src-tauri/commands/entry_cmd.rs
// Entry query and mark Tauri commands

use crate::db::entry_store::{self, EntryFilter, EntryQueryParams, TagMatchMode};
use crate::db::models::{EntryCursor, EntryDetail, EntryPage};
use deadpool_sqlite::Pool;
use tauri::State;

#[tauri::command]
pub async fn load_entries(
    pool: State<'_, Pool>,
    feed_id: Option<i64>,
    unread_only: Option<bool>,
    starred_only: Option<bool>,
    search: Option<String>,
    tag_ids: Option<Vec<i64>>,
    tag_mode: Option<TagMatchMode>,
    cursor: Option<EntryCursor>,
    limit: Option<u32>,
) -> Result<EntryPage, String> {
    entry_store::query_entries(
        &pool,
        &EntryQueryParams {
            filter: EntryFilter {
                feed_id,
                unread_only: unread_only.unwrap_or(false),
                starred_only: starred_only.unwrap_or(false),
                search,
                tag_ids,
                tag_mode: tag_mode.unwrap_or_default(),
            },
            cursor,
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
    entry_store::delete_entry(&pool, entry_id).await
}

#[tauri::command]
pub async fn mark_all_read_in_feed(pool: State<'_, Pool>, feed_id: i64) -> Result<(), String> {
    entry_store::mark_all_read_in_feed(&pool, feed_id).await
}

#[tauri::command]
pub async fn mark_all_read_in_scope(
    pool: State<'_, Pool>,
    feed_id: Option<i64>,
    unread_only: Option<bool>,
    starred_only: Option<bool>,
    search: Option<String>,
    tag_ids: Option<Vec<i64>>,
    tag_mode: Option<TagMatchMode>,
) -> Result<(), String> {
    entry_store::mark_all_read_in_scope(
        &pool,
        &EntryFilter {
            feed_id,
            unread_only: unread_only.unwrap_or(false),
            starred_only: starred_only.unwrap_or(false),
            search,
            tag_ids,
            tag_mode: tag_mode.unwrap_or_default(),
        },
    )
    .await
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
