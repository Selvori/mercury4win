// mercury4win/src-tauri/db/feed_store.rs
// Feed persistence — CRUD + sync status updates

use crate::db::models::{Feed, FeedWithCounts};
use deadpool_sqlite::Pool;
use rusqlite::params;

/// Return a URL-equivalent subscription and whether it has ever held valid data.
pub async fn find_import_url(pool: &Pool, url: &str) -> Result<Option<(i64, bool)>, String> {
    let url = url.to_owned();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| find_import_url_conn(conn, &url, &url))
        .await
        .map_err(|e| e.to_string())?
}

fn find_import_url_conn(
    conn: &rusqlite::Connection,
    requested: &str,
    resolved: &str,
) -> Result<Option<(i64, bool)>, String> {
    let mut stmt = conn.prepare("SELECT id, feed_url, last_fetched_at IS NOT NULL OR EXISTS(SELECT 1 FROM entry WHERE feed_id=feed.id) FROM feed ORDER BY id").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let requested_key = crate::feed::identity::url_key(requested);
    let resolved_key = crate::feed::identity::url_key(resolved);
    let mut empty = None;
    for row in rows {
        let (id, url, ready) = row.map_err(|e| e.to_string())?;
        let key = crate::feed::identity::url_key(&url);
        if url == requested
            || url == resolved
            || (key.is_some() && (key == requested_key || key == resolved_key))
        {
            if ready {
                return Ok(Some((id, true)));
            }
            empty = Some((id, false));
        }
    }
    Ok(empty)
}

/// The feed and its first articles become visible together, or not at all.
/// Recheck identity in the same write transaction to handle concurrent imports.
pub async fn save_imported_feed(
    pool: &Pool,
    requested: &str,
    resolved: &str,
    fallback_title: &str,
    parsed: crate::feed::parser::ParsedFeed,
) -> Result<(i64, bool), String> {
    let requested = requested.to_owned();
    let resolved = resolved.to_owned();
    let fallback_title = fallback_title.to_owned();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            let existing = find_import_url_conn(&tx, &requested, &resolved)?;
            if let Some((id, true)) = existing {
                return Ok((id, false));
            }
            let incoming_urls = crate::feed::identity::article_urls(&parsed.entries);
            let site = parsed
                .site_url
                .as_deref()
                .and_then(crate::feed::identity::url_key);
            let title = parsed.title.as_deref().unwrap_or(&fallback_title);
            let title_key = |s: &str| s.split_whitespace().collect::<String>().to_lowercase();
            let candidates: Vec<(i64, Option<String>, Option<String>)> = {
                let mut stmt = tx
                    .prepare("SELECT id,title,site_url FROM feed ORDER BY id")
                    .map_err(|e| e.to_string())?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                    .map_err(|e| e.to_string())?;
                rows.collect::<Result<_, _>>().map_err(|e| e.to_string())?
            };
            for (id, name, home) in candidates {
                if site.is_none()
                    || home
                        .as_deref()
                        .and_then(crate::feed::identity::url_key)
                        .is_some_and(|home| Some(home) != site)
                    || name.as_deref().map(&title_key) != Some(title_key(title))
                {
                    continue;
                }
                let urls = {
                    let mut stmt = tx
                        .prepare("SELECT url FROM entry WHERE feed_id=?1 AND url IS NOT NULL")
                        .map_err(|e| e.to_string())?;
                    let rows = stmt
                        .query_map([id], |r| r.get::<_, String>(0))
                        .map_err(|e| e.to_string())?;
                    let urls = rows
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| e.to_string())?;
                    urls.iter()
                        .filter_map(|u| crate::feed::identity::url_key(u))
                        .collect()
                };
                if crate::feed::identity::same_articles(&incoming_urls, &urls) {
                    if let Some((empty_id, false)) = existing {
                        tx.execute("DELETE FROM feed WHERE id=?1", [empty_id])
                            .map_err(|e| e.to_string())?;
                    }
                    tx.commit().map_err(|e| e.to_string())?;
                    return Ok((id, false));
                }
            }
            let id = if let Some((id, false)) = existing {
                id
            } else {
                tx.execute(
                    "INSERT INTO feed(title,feed_url,site_url) VALUES(?1,?2,?3)",
                    params![title, requested, parsed.site_url],
                )
                .map_err(|e| e.to_string())?;
                tx.last_insert_rowid()
            };
            crate::db::entry_store::upsert_entries_in_transaction(&tx, id, &parsed.entries)?;
            tx.execute(
                "UPDATE feed SET title=?1,site_url=?2,last_fetched_at=datetime('now') WHERE id=?3",
                params![title, parsed.site_url, id],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok((id, true))
        })
        .await
        .map_err(|e| e.to_string())?
}

/// Remove only an old, never-successful placeholder; never remove articles.
pub async fn remove_failed_import(pool: &Pool, id: i64) -> Result<(), String> {
    pool.get().await.map_err(|e| e.to_string())?.interact(move |conn| {
        conn.execute("DELETE FROM feed WHERE id=?1 AND last_fetched_at IS NULL AND NOT EXISTS(SELECT 1 FROM entry WHERE feed_id=feed.id)", [id]).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?
}

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

/// Insert a feed, returning the feed and whether it was newly inserted.
/// Uses INSERT OR IGNORE, so an existing feed_url yields `(existing_feed, false)`.
pub async fn insert_feed(
    pool: &Pool,
    feed_url: &str,
    title: Option<&str>,
    site_url: Option<&str>,
) -> Result<(Feed, bool), String> {
    let feed_url = feed_url.to_string();
    let title = title.map(|s| s.to_string());
    let site_url = site_url.map(|s| s.to_string());
    let pool = pool.clone();

    let feed = pool
        .get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let affected = conn.execute(
                "INSERT OR IGNORE INTO feed (title, feed_url, site_url) VALUES (?1, ?2, ?3)",
                params![title, feed_url, site_url],
            )
            .map_err(|e| e.to_string())?;
            let was_inserted = affected > 0;

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
            .map(|feed| (feed, was_inserted))
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
