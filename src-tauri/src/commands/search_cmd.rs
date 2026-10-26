use crate::db::{
    entry_store::EntryFilter,
    search_store,
    smart_view_store::{self, SmartView},
};
use deadpool_sqlite::Pool;
use tauri::State;

#[tauri::command]
pub async fn list_smart_views(pool: State<'_, Pool>) -> Result<Vec<SmartView>, String> {
    smart_view_store::list(&pool).await
}
#[tauri::command]
pub async fn save_smart_view(
    pool: State<'_, Pool>,
    id: Option<i64>,
    name: String,
    filter: EntryFilter,
) -> Result<i64, String> {
    smart_view_store::save(&pool, id, name, filter).await
}
#[tauri::command]
pub async fn delete_smart_view(pool: State<'_, Pool>, id: i64) -> Result<(), String> {
    smart_view_store::delete(&pool, id).await
}
#[tauri::command]
pub async fn rebuild_search_index(pool: State<'_, Pool>) -> Result<(), String> {
    search_store::rebuild(&pool).await
}
