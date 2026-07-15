// mercury4win-linux/src-tauri/commands/feed_cmd.rs
// Feed CRUD Tauri commands

use deadpool_sqlite::Pool;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use crate::db::{feed_store, entry_store};
use crate::db::models::{Feed, FeedWithCounts, SyncResult, ImportResult};
use crate::feed::use_cases;
use crate::feed::opml_import;

pub struct AppClient(pub reqwest::Client);

impl AppClient {
    pub fn new() -> Self {
        Self(reqwest::Client::new())
    }
}

#[tauri::command]
pub async fn load_feeds(pool: State<'_, Pool>) -> Result<Vec<FeedWithCounts>, String> {
    feed_store::list_feeds(&pool).await
}

#[tauri::command]
pub async fn add_feed(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    url: String,
) -> Result<Feed, String> {
    use_cases::subscribe(&pool, &client.0, &url).await
}

#[tauri::command]
pub async fn update_feed(
    pool: State<'_, Pool>,
    id: i64,
    title: Option<String>,
) -> Result<(), String> {
    feed_store::update_feed_meta(&pool, id, title.as_deref(), None, None).await
}

#[tauri::command]
pub async fn delete_feed(pool: State<'_, Pool>, id: i64) -> Result<(), String> {
    feed_store::delete_feed(&pool, id).await
}

#[tauri::command]
pub async fn sync_feed(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    id: i64,
) -> Result<SyncResult, String> {
    use_cases::refresh_single(&pool, &client.0, id).await
}

#[tauri::command]
pub async fn sync_all_feeds(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
) -> Result<Vec<SyncResult>, String> {
    use_cases::refresh_all(&pool, &client.0).await
}

#[tauri::command]
pub async fn import_opml(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    file_path: String,
) -> Result<ImportResult, String> {
    let path = std::path::Path::new(&file_path);
    let entries = opml_import::parse_opml_file(path)?;

    let mut added = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    for entry in entries {
        match feed_store::insert_feed(
            &pool,
            &entry.feed_url,
            Some(&entry.title),
            entry.site_url.as_deref(),
        )
        .await
        {
            Ok(feed) => {
                // Try initial sync
                if let Err(e) = use_cases::refresh_single(&pool, &client.0, feed.id).await {
                    errors.push(format!("{}: {}", entry.feed_url, e));
                }
                added += 1;
            }
            Err(_) => {
                skipped += 1;
            }
        }
    }

    Ok(ImportResult {
        feeds_added: added,
        feeds_skipped: skipped,
        errors,
    })
}

#[tauri::command]
pub async fn export_opml(
    pool: State<'_, Pool>,
    file_path: String,
    feed_ids: Option<Vec<i64>>,
) -> Result<(), String> {
    let feeds = feed_store::list_feeds(&pool).await?;

    let feeds_to_export: Vec<&FeedWithCounts> = match feed_ids {
        Some(ref ids) => feeds.iter().filter(|f| ids.contains(&f.id)).collect(),
        None => feeds.iter().collect(),
    };

    let feed_data: Vec<crate::feed::opml_export::OpmlFeedData> = feeds_to_export
        .iter()
        .map(|f| crate::feed::opml_export::OpmlFeedData {
            title: f.title.clone().unwrap_or_else(|| "Untitled".into()),
            feed_url: f.feed_url.clone(),
            site_url: f.site_url.clone(),
        })
        .collect();

    crate::feed::opml_export::export_opml_file(std::path::Path::new(&file_path), &feed_data, &[])
}
