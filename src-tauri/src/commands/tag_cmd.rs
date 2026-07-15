// mercury4win-linux/src-tauri/commands/tag_cmd.rs
// Tag management Tauri commands

use deadpool_sqlite::Pool;
use tauri::State;
use crate::db::{tag_store};
use crate::db::models::{Tag, BatchTagResult};

#[tauri::command]
pub async fn list_tags(
    pool: State<'_, Pool>,
    search: Option<String>,
) -> Result<Vec<Tag>, String> {
    tag_store::list_tags(&pool, search.as_deref()).await
}

#[tauri::command]
pub async fn add_tag(
    pool: State<'_, Pool>,
    entry_id: i64,
    name: String,
) -> Result<Tag, String> {
    tag_store::add_tag(&pool, entry_id, &name).await
}

#[tauri::command]
pub async fn remove_tag(
    pool: State<'_, Pool>,
    entry_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    tag_store::remove_tag(&pool, entry_id, tag_id).await
}

#[tauri::command]
pub async fn merge_tags(
    pool: State<'_, Pool>,
    source_id: i64,
    target_id: i64,
) -> Result<(), String> {
    tag_store::merge_tags(&pool, source_id, target_id).await
}

#[tauri::command]
pub async fn rename_tag(
    pool: State<'_, Pool>,
    tag_id: i64,
    new_name: String,
) -> Result<(), String> {
    tag_store::rename_tag(&pool, tag_id, &new_name).await
}

#[tauri::command]
pub async fn delete_tag(pool: State<'_, Pool>, tag_id: i64) -> Result<(), String> {
    tag_store::delete_tag(&pool, tag_id).await
}

#[tauri::command]
pub async fn batch_tag(
    pool: State<'_, Pool>,
    time_range: crate::db::models::TimeRange,
    tag_ids: Vec<i64>,
) -> Result<BatchTagResult, String> {
    tag_store::batch_tag(&pool, &time_range.from, &time_range.to, &tag_ids).await
}
