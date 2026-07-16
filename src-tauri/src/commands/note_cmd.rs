// mercury4win-linux/src-tauri/commands/note_cmd.rs
// Entry notes Tauri commands

use deadpool_sqlite::Pool;
use tauri::State;
use crate::db::{note_store, models::EntryNote};

#[tauri::command]
pub async fn get_note(
    pool: State<'_, Pool>,
    entry_id: i64,
) -> Result<Option<EntryNote>, String> {
    note_store::get_note(&pool, entry_id).await
}

#[tauri::command]
pub async fn save_note(
    pool: State<'_, Pool>,
    entry_id: i64,
    markdown: String,
) -> Result<(), String> {
    note_store::save_note(&pool, entry_id, &markdown).await
}

#[tauri::command]
pub async fn delete_note(
    pool: State<'_, Pool>,
    entry_id: i64,
) -> Result<(), String> {
    note_store::delete_note(&pool, entry_id).await
}
