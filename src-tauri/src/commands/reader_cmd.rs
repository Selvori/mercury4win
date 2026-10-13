// mercury4win/src-tauri/commands/reader_cmd.rs
// Reader pipeline Tauri commands

use crate::commands::feed_cmd::AppClient;
use crate::reader::pipeline;
use crate::reader::theme::FontConfig;
use deadpool_sqlite::Pool;
use tauri::State;

/// Build a FontConfig from optional command params, falling back to defaults.
fn font_config_from(font_family: Option<String>, font_size: Option<u32>) -> FontConfig {
    let default = FontConfig::default();
    FontConfig {
        family: font_family.unwrap_or(default.family),
        size: font_size.unwrap_or(default.size),
    }
}

/// Build the cache key used to look up rendered HTML — must match the key
/// used by the pipeline when writing the cache.
fn cache_key_from(theme_id: &str, font_config: &FontConfig) -> String {
    format!("{}:{}:{}", theme_id, font_config.family, font_config.size)
}

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
    font_family: Option<String>,
    font_size: Option<u32>,
) -> Result<String, String> {
    let theme = theme_id.unwrap_or_else(|| "classic-light".to_string());
    let font_config = font_config_from(font_family, font_size);
    pipeline::build_reader_content(&pool, &client.0, entry_id, &theme, &font_config).await
}

#[tauri::command]
pub async fn get_cached_reader_content(
    pool: State<'_, Pool>,
    entry_id: i64,
    theme_id: Option<String>,
    font_family: Option<String>,
    font_size: Option<u32>,
) -> Result<Option<String>, String> {
    let theme = theme_id.unwrap_or_else(|| "classic-light".to_string());
    let font_config = font_config_from(font_family, font_size);
    let cache_key = cache_key_from(&theme, &font_config);
    pipeline::get_cached_reader_html(&pool, entry_id, &cache_key).await
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
