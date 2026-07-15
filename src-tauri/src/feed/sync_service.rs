// mercury4win-linux/src-tauri/feed/sync_service.rs
// Feed sync service with rate limiting

use deadpool_sqlite::Pool;
use crate::db::{feed_store, entry_store};
use crate::db::models::SyncResult;
use crate::feed::parser::parse_feed_bytes;

/// Sync a single feed: fetch, parse, upsert entries.
pub async fn sync_feed(
    pool: &Pool,
    http_client: &reqwest::Client,
    feed_id: i64,
    feed_url: &str,
) -> Result<SyncResult, String> {
    let response = http_client
        .get(feed_url)
        .header("User-Agent", "Mercury/0.1 (RSS Reader)")
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    if bytes.is_empty() {
        return Ok(SyncResult {
            feed_id,
            new_entries: 0,
            updated_entries: 0,
            error: Some("Empty response body".into()),
        });
    }

    let parsed = parse_feed_bytes(&bytes, Some(feed_url))?;

    // Update feed metadata
    feed_store::update_feed_meta(
        pool,
        feed_id,
        parsed.title.as_deref(),
        parsed.site_url.as_deref(),
        None,
    )
    .await?;

    // Upsert entries
    let (new_count, updated_count) =
        entry_store::bulk_upsert_entries(pool, feed_id, parsed.entries).await?;

    // Touch last_fetched_at
    feed_store::touch_feed_sync(pool, feed_id).await?;

    Ok(SyncResult {
        feed_id,
        new_entries: new_count,
        updated_entries: updated_count,
        error: None,
    })
}

/// Sync all feeds with rate-limited sequential processing (3 per chunk).
pub async fn sync_all_feeds(
    pool: &Pool,
    client: &reqwest::Client,
    feed_urls: Vec<(i64, String)>,
) -> Vec<SyncResult> {
    let mut results = Vec::new();

    for chunk in feed_urls.chunks(3) {
        let mut chunk_results = Vec::new();
        for (id, url) in chunk {
            match sync_feed(pool, client, *id, url).await {
                Ok(r) => chunk_results.push(r),
                Err(e) => chunk_results.push(SyncResult {
                    feed_id: *id,
                    new_entries: 0,
                    updated_entries: 0,
                    error: Some(e),
                }),
            }
        }
        results.extend(chunk_results);
    }

    results
}
