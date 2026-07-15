// mercury4win-linux/src/commands/digest_cmd.rs
// Digest export Tauri commands

use deadpool_sqlite::Pool;
use tauri::State;
use crate::db::{entry_store, content_store};
use crate::digest::templates;

#[tauri::command]
pub async fn generate_digest(
    pool: State<'_, Pool>,
    entry_id: i64,
    template_name: String,
) -> Result<String, String> {
    let detail = entry_store::get_entry_detail(&pool, entry_id)
        .await?
        .ok_or("Entry not found")?;

    let content = content_store::get_content(&pool, entry_id).await?;
    let md = content
        .as_ref()
        .and_then(|c| c.markdown.as_deref())
        .unwrap_or("");

    templates::generate_entry_digest(
        &template_name,
        &detail.entry.title.unwrap_or_default(),
        detail.entry.author.as_deref(),
        md,
        detail.entry.url.as_deref(),
    )
    .await
}

async fn generate_entry_single(
    pool: &Pool,
    entry_id: i64,
    template_name: &str,
) -> Result<String, String> {
    let detail = entry_store::get_entry_detail(pool, entry_id)
        .await?
        .ok_or("Entry not found")?;

    let content = content_store::get_content(pool, entry_id).await?;
    let md = content
        .as_ref()
        .and_then(|c| c.markdown.as_deref())
        .unwrap_or("");

    templates::generate_entry_digest(
        template_name,
        &detail.entry.title.unwrap_or_default(),
        detail.entry.author.as_deref(),
        md,
        detail.entry.url.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn export_digest(
    pool: State<'_, Pool>,
    entry_id: i64,
    template_name: String,
    export_path: String,
) -> Result<(), String> {
    let content = generate_entry_single(&pool, entry_id, &template_name).await?;

    std::fs::write(&export_path, content)
        .map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn export_multi_digest(
    pool: State<'_, Pool>,
    entry_ids: Vec<i64>,
    template_name: String,
    export_path: String,
) -> Result<(), String> {
    let mut entries = Vec::new();

    for id in &entry_ids {
        let detail = entry_store::get_entry_detail(&pool, *id)
            .await?
            .ok_or_else(|| format!("Entry {} not found", id))?;

        let content = content_store::get_content(&pool, *id).await?;
        let md = content
            .as_ref()
            .and_then(|c| c.markdown.as_deref())
            .unwrap_or("");

        entries.push(templates::DigestEntryData {
            title: detail.entry.title.unwrap_or_default(),
            byline: detail.entry.author,
            content: md.to_string(),
            published_at: detail.entry.published_at,
            url: detail.entry.url,
        });
    }

    let content = templates::generate_multi_digest(
        &template_name,
        Some("Mercury Digest"),
        entries,
    )
    .await?;

    std::fs::write(&export_path, content)
        .map_err(|e| format!("Write error: {}", e))?;

    Ok(())
}
