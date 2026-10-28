// mercury4win/src-tauri/feed/use_cases.rs
// Feed CRUD use cases — thin wrappers around db stores

use crate::db::feed_store;
use crate::db::models::{Feed, SyncResult};
use crate::feed::sync_service;
use deadpool_sqlite::Pool;

/// Add a new feed, fetch its metadata, and return the feed.
pub async fn subscribe(pool: &Pool, client: &reqwest::Client, url: &str) -> Result<Feed, String> {
    let entry = super::opml_import::OpmlImportEntry {
        title: url.to_owned(),
        feed_url: url.to_owned(),
        site_url: None,
    };
    let (id, _) =
        super::import_service::import_one(pool, client, &entry, || {}, super::fetch::FEED_TIMEOUT)
            .await?;
    feed_store::get_feed(pool, id)
        .await?
        .ok_or_else(|| "Feed not found".into())
}

/// Refresh all feeds.
pub async fn refresh_all(pool: &Pool, client: &reqwest::Client) -> Result<Vec<SyncResult>, String> {
    let feeds = feed_store::list_feeds(pool).await?;
    let feed_urls: Vec<(i64, String)> = feeds.into_iter().map(|f| (f.id, f.feed_url)).collect();

    Ok(sync_service::sync_all_feeds(pool, client, feed_urls).await)
}

/// Refresh a single feed.
pub async fn refresh_single(
    pool: &Pool,
    client: &reqwest::Client,
    feed_id: i64,
) -> Result<SyncResult, String> {
    let feed = feed_store::get_feed(pool, feed_id)
        .await?
        .ok_or_else(|| "Feed not found".to_string())?;

    sync_service::sync_feed(pool, client, feed.id, &feed.feed_url).await
}
