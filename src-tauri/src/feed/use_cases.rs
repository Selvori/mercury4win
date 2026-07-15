// mercury4win-linux/src-tauri/feed/use_cases.rs
// Feed CRUD use cases — thin wrappers around db stores

use deadpool_sqlite::Pool;
use crate::db::{feed_store, entry_store};
use crate::db::models::{Feed, FeedWithCounts, SyncResult};
use crate::feed::sync_service;

/// Add a new feed, fetch its metadata, and return the feed.
pub async fn subscribe(
    pool: &Pool,
    client: &reqwest::Client,
    url: &str,
) -> Result<Feed, String> {
    // Fetch the feed first to validate and get title
    let response = client
        .get(url)
        .header("User-Agent", "Mercury/0.1 (RSS Reader)")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    let parsed = crate::feed::parser::parse_feed_bytes(&bytes, Some(url))?;

    let feed = feed_store::insert_feed(
        pool,
        url,
        parsed.title.as_deref(),
        parsed.site_url.as_deref(),
    )
    .await?;

    // Initial sync to populate entries
    let _ = entry_store::bulk_upsert_entries(pool, feed.id, parsed.entries).await?;
    feed_store::touch_feed_sync(pool, feed.id).await?;

    Ok(feed)
}

/// Refresh all feeds.
pub async fn refresh_all(pool: &Pool, client: &reqwest::Client) -> Result<Vec<SyncResult>, String> {
    let feeds = feed_store::list_feeds(pool).await?;
    let feed_urls: Vec<(i64, String)> = feeds
        .into_iter()
        .map(|f| (f.id, f.feed_url))
        .collect();

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
