// mercury4win-linux/src-tauri/db/feed_store.rs
// Feed persistence — CRUD + sync status updates

use deadpool_sqlite::Pool;
use rusqlite::params;
use crate::db::models::{Feed, FeedWithCounts};

pub async fn list_feeds(pool: &Pool) -> Result<Vec<FeedWithCounts>, String> {
    let pool = pool.clone();
    let feeds = pool
        .get()
        .await
        .map_err(|e| e.to_string())?
        .interact(|conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT f.id, f.title, f.feed_url, f.site_url, f.last_fetched_at, f.created_at,
                            COALESCE(SUM(CASE WHEN e.is_read = 0 AND e.is_deleted = 0 THEN 1 ELSE 0 END), 0) as unread_count,
                            COALESCE(SUM(CASE WHEN e.is_deleted = 0 THEN 1 ELSE 0 END), 0) as total_count
                     FROM feed f
                     LEFT JOIN entry e ON e.feed_id = f.id
                     GROUP BY f.id
                     ORDER BY f.title ASC",
                )
                .map_err(|e| e.to_string())?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(FeedWithCounts {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        feed_url: row.get(2)?,
                        site_url: row.get(3)?,
                        last_fetched_at: row.get(4)?,
                        created_at: row.get(5)?,
                        unread_count: row.get(6)?,
                        total_count: row.get(7)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?;

    feeds
}

pub async fn get_feed(pool: &Pool, feed_id: i64) -> Result<Option<Feed>, String> {
    let pool = pool.clone();
    let feed = pool
        .get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.query_row(
                "SELECT id, title, feed_url, site_url, feed_parser_version, last_fetched_at, created_at
                 FROM feed WHERE id = ?1",
                params![feed_id],
                |row| {
                    Ok(Feed {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        feed_url: row.get(2)?,
                        site_url: row.get(3)?,
                        feed_parser_version: row.get(4)?,
                        last_fetched_at: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            )
        })
        .await
        .map_err(|e| e.to_string())?;

    match feed {
        Ok(f) => Ok(Some(f)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub async fn insert_feed(pool: &Pool, feed_url: &str, title: Option<&str>, site_url: Option<&str>) -> Result<Feed, String> {
    let feed_url = feed_url.to_string();
    let title = title.map(|s| s.to_string());
    let site_url = site_url.map(|s| s.to_string());
    let pool = pool.clone();

    let feed = pool
        .get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO feed (title, feed_url, site_url) VALUES (?1, ?2, ?3)",
                params![title, feed_url, site_url],
            )
            .map_err(|e| e.to_string())?;

            conn.query_row(
                "SELECT id, title, feed_url, site_url, feed_parser_version, last_fetched_at, created_at
                 FROM feed WHERE feed_url = ?1",
                params![feed_url],
                |row| {
                    Ok(Feed {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        feed_url: row.get(2)?,
                        site_url: row.get(3)?,
                        feed_parser_version: row.get(4)?,
                        last_fetched_at: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                },
            )
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    Ok(feed)
}

pub async fn update_feed_meta(
    pool: &Pool,
    feed_id: i64,
    title: Option<&str>,
    site_url: Option<&str>,
    parser_version: Option<i32>,
) -> Result<(), String> {
    let title = title.map(|s| s.to_string());
    let site_url = site_url.map(|s| s.to_string());
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE feed SET title = COALESCE(?1, title), site_url = COALESCE(?2, site_url),
                 feed_parser_version = COALESCE(?3, feed_parser_version)
                 WHERE id = ?4",
                params![title, site_url, parser_version, feed_id],
            )
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn touch_feed_sync(pool: &Pool, feed_id: i64) -> Result<(), String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE feed SET last_fetched_at = datetime('now') WHERE id = ?1",
                params![feed_id],
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_feed(pool: &Pool, feed_id: i64) -> Result<(), String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute("DELETE FROM feed WHERE id = ?1", params![feed_id])
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}
