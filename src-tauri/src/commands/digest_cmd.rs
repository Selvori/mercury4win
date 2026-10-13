use crate::digest::templates::{self, DigestOptions, DigestPreview};
use deadpool_sqlite::Pool;
use tauri::State;

#[tauri::command]
pub async fn preview_digest(
    pool: State<'_, Pool>,
    entry_ids: Vec<i64>,
    template_name: String,
    options: DigestOptions,
) -> Result<DigestPreview, String> {
    templates::generate_preview(&pool, &entry_ids, &template_name, &options).await
}

#[tauri::command]
pub async fn generate_digest(
    pool: State<'_, Pool>,
    entry_id: i64,
    template_name: String,
    options: Option<DigestOptions>,
) -> Result<String, String> {
    Ok(templates::generate_preview(
        &pool,
        &[entry_id],
        &template_name,
        &options.unwrap_or_default(),
    )
    .await?
    .content)
}

#[tauri::command]
pub async fn export_digest(
    pool: State<'_, Pool>,
    entry_id: i64,
    template_name: String,
    export_path: String,
    options: Option<DigestOptions>,
    expected_content: Option<String>,
) -> Result<(), String> {
    templates::export_preview(
        &pool,
        &[entry_id],
        &template_name,
        &options.unwrap_or_default(),
        &export_path,
        expected_content.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn export_multi_digest(
    pool: State<'_, Pool>,
    entry_ids: Vec<i64>,
    template_name: String,
    export_path: String,
    options: Option<DigestOptions>,
    expected_content: Option<String>,
) -> Result<(), String> {
    templates::export_preview(
        &pool,
        &entry_ids,
        &template_name,
        &options.unwrap_or_default(),
        &export_path,
        expected_content.as_deref(),
    )
    .await
}
