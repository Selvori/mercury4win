// mercury4win/src/commands/settings_cmd.rs
// Settings and usage Tauri commands

use crate::db::usage_store;
use deadpool_sqlite::Pool;
use tauri::State;

#[tauri::command]
pub async fn get_setting(pool: State<'_, Pool>, key: String) -> Result<Option<String>, String> {
    let pool = pool.inner().clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let result = conn.query_row(
                "SELECT value FROM settings WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get::<_, String>(0),
            );
            match result {
                Ok(v) => Ok(Some(v)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        })
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_setting(pool: State<'_, Pool>, key: String, value: String) -> Result<(), String> {
    let pool = pool.inner().clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                rusqlite::params![key, value],
            )
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_usage_report(
    pool: State<'_, Pool>,
    window: String,
    dimension: Option<String>,
    filter: Option<usage_store::UsageFilter>,
) -> Result<usage_store::UsageReport, String> {
    usage_store::report(
        &pool,
        &window,
        dimension.as_deref().unwrap_or("provider"),
        filter.unwrap_or_default(),
    )
    .await
}
