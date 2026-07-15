// mercury4win-linux/src-tauri/commands/reader_cmd.rs
// Reader pipeline Tauri commands

use deadpool_sqlite::Pool;
use reqwest::Client;
use tauri::State;
use crate::commands::feed_cmd::AppClient;
use crate::reader::pipeline;

#[derive(serde::Serialize)]
pub struct ReaderTheme {
    pub id: String,
    pub style: String,
    pub color_scheme: String,
    pub label: String,
}

#[tauri::command]
pub async fn build_reader_content(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    entry_id: i64,
    theme_id: Option<String>,
) -> Result<String, String> {
    let theme = theme_id.unwrap_or_else(|| "classic-light".to_string());
    pipeline::build_reader_content(&pool, &client.0, entry_id, &theme).await
}

#[tauri::command]
pub async fn get_cached_reader_content(
    pool: State<'_, Pool>,
    entry_id: i64,
    theme_id: Option<String>,
) -> Result<Option<String>, String> {
    let theme = theme_id.unwrap_or_else(|| "classic-light".to_string());
    pipeline::get_cached_reader_html(&pool, entry_id, &theme).await
}

#[tauri::command]
pub async fn get_reader_themes() -> Result<Vec<ReaderTheme>, String> {
    Ok(vec![
        ReaderTheme {
            id: "classic-light".into(),
            style: "classic".into(),
            color_scheme: "light".into(),
            label: "Classic Light".into(),
        },
        ReaderTheme {
            id: "classic-dark".into(),
            style: "classic".into(),
            color_scheme: "dark".into(),
            label: "Classic Dark".into(),
        },
        ReaderTheme {
            id: "paper-light".into(),
            style: "paper".into(),
            color_scheme: "light".into(),
            label: "Paper Light".into(),
        },
        ReaderTheme {
            id: "paper-dark".into(),
            style: "paper".into(),
            color_scheme: "dark".into(),
            label: "Paper Dark".into(),
        },
    ])
}
