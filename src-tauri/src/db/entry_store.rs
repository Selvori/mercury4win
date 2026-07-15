// mercury4win-linux/src-tauri/db/entry_store.rs
// Entry persistence — paginated queries, read/star/delete, batch operations

use deadpool_sqlite::Pool;
use rusqlite::params;
use crate::db::models::{Entry, EntryDetail, EntryPage, EntryTagInfo};

pub struct EntryQueryParams {
    pub feed_id: Option<i64>,
    pub unread_only: bool,
    pub search: Option<String>,
    pub tag_ids: Option<Vec<i64>>,
    pub cursor: Option<i64>,
    pub limit: u32,
}

pub async fn query_entries(pool: &Pool, params: &EntryQueryParams) -> Result<EntryPage, String> {
    let feed_id = params.feed_id;
    let unread_only = params.unread_only;
    let search = params.search.clone();
    let tag_ids = params.tag_ids.clone();
    let cursor = params.cursor;
    let limit = params.limit;
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let mut conditions = vec!["e.is_deleted = 0".to_string()];
            let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

            if let Some(fid) = feed_id {
                conditions.push(format!("e.feed_id = ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(fid));
            }
            if unread_only {
                conditions.push("e.is_read = 0".to_string());
            }
            if let Some(ref q) = search {
                conditions.push(format!(
                    "(e.title LIKE ?{} OR e.summary LIKE ?{})",
                    bind_values.len() + 1,
                    bind_values.len() + 2
                ));
                let like = format!("%{}%", q);
                bind_values.push(Box::new(like.clone()));
                bind_values.push(Box::new(like));
            }
            if let Some(ref tids) = tag_ids {
                if !tids.is_empty() {
                    let placeholders: Vec<String> = tids
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("?{}", bind_values.len() + i + 1))
                        .collect();
                    conditions.push(format!(
                        "e.id IN (SELECT entry_id FROM entry_tag WHERE tag_id IN ({}))",
                        placeholders.join(",")
                    ));
                    for tid in tids {
                        bind_values.push(Box::new(*tid));
                    }
                }
            }
            if let Some(c) = cursor {
                conditions.push(format!("e.id < ?{}", bind_values.len() + 1));
                bind_values.push(Box::new(c));
            }

            let where_clause = conditions.join(" AND ");

            // Total count
            let count_sql = format!("SELECT COUNT(*) FROM entry e WHERE {}", where_clause);
            let total: i64 = {
                let mut stmt = conn.prepare(&count_sql).map_err(|e| e.to_string())?;
                let refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
                stmt.query_row(refs.as_slice(), |row| row.get(0))
                    .map_err(|e| e.to_string())?
            };

            // Paginated query
            let query_sql = format!(
                "SELECT e.id, e.feed_id, e.guid, e.url, e.title, e.author,
                        e.published_at, e.summary, e.is_read, e.is_starred,
                        e.is_deleted, e.created_at
                 FROM entry e
                 WHERE {}
                 ORDER BY e.published_at DESC, e.id DESC
                 LIMIT ?{}",
                where_clause,
                bind_values.len() + 1
            );
            bind_values.push(Box::new(limit as i64));

            let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
            let refs: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();

            let rows = stmt
                .query_map(refs.as_slice(), |row| {
                    Ok(Entry {
                        id: row.get(0)?,
                        feed_id: row.get(1)?,
                        guid: row.get(2)?,
                        url: row.get(3)?,
                        title: row.get(4)?,
                        author: row.get(5)?,
                        published_at: row.get(6)?,
                        summary: row.get(7)?,
                        is_read: row.get::<_, i32>(8)? != 0,
                        is_starred: row.get::<_, i32>(9)? != 0,
                        is_deleted: row.get::<_, i32>(10)? != 0,
                        created_at: row.get(11)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            let entries: Result<Vec<_>, _> = rows.collect();
            let entries = entries.map_err(|e| e.to_string())?;
            let next_cursor = entries.last().map(|e| e.id.to_string());

            Ok(EntryPage {
                entries,
                next_cursor,
                total,
            })
        })
        .await
        .map_err(|e| e.to_string())?
}

pub async fn get_entry_detail(pool: &Pool, entry_id: i64) -> Result<Option<EntryDetail>, String> {
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let entry_result: Result<(Entry, Option<String>), String> = conn
                .query_row(
                    "SELECT e.id, e.feed_id, e.guid, e.url, e.title, e.author,
                            e.published_at, e.summary, e.is_read, e.is_starred,
                            e.is_deleted, e.created_at, f.title
                     FROM entry e
                     LEFT JOIN feed f ON f.id = e.feed_id
                     WHERE e.id = ?1",
                    params![entry_id],
                    |row| {
                        let entry = Entry {
                            id: row.get(0)?,
                            feed_id: row.get(1)?,
                            guid: row.get(2)?,
                            url: row.get(3)?,
                            title: row.get(4)?,
                            author: row.get(5)?,
                            published_at: row.get(6)?,
                            summary: row.get(7)?,
                            is_read: row.get::<_, i32>(8)? != 0,
                            is_starred: row.get::<_, i32>(9)? != 0,
                            is_deleted: row.get::<_, i32>(10)? != 0,
                            created_at: row.get(11)?,
                        };
                        let feed_title: Option<String> = row.get(12)?;
                        Ok((entry, feed_title))
                    },
                )
                .map_err(|e| e.to_string());

            match entry_result {
                Err(e) if e.contains("QueryReturnedNoRows") => Ok(None),
                Err(e) => Err(e),
                Ok((entry, feed_title)) => {
                    // Load tags
                    let mut tag_stmt = conn
                        .prepare(
                            "SELECT t.id, t.name, et.source
                             FROM entry_tag et
                             JOIN tag t ON t.id = et.tag_id
                             WHERE et.entry_id = ?1",
                        )
                        .map_err(|e| e.to_string())?;

                    let tags: Vec<EntryTagInfo> = tag_stmt
                        .query_map(params![entry_id], |row| {
                            Ok(EntryTagInfo {
                                tag_id: row.get(0)?,
                                name: row.get(1)?,
                                source: row.get(2)?,
                            })
                        })
                        .map_err(|e| e.to_string())?
                        .filter_map(|r| r.ok())
                        .collect();

                    // Check for note
                    let has_note: bool = conn
                        .query_row(
                            "SELECT COUNT(*) FROM entry_note WHERE entry_id = ?1",
                            params![entry_id],
                            |row| row.get::<_, i64>(0),
                        )
                        .map(|c| c > 0)
                        .unwrap_or(false);

                    Ok(Some(EntryDetail {
                        entry,
                        feed_title,
                        tags,
                        has_note,
                    }))
                }
            }
        })
        .await
        .map_err(|e| e.to_string())?
}

pub async fn bulk_upsert_entries(
    pool: &Pool,
    feed_id: i64,
    entries: Vec<RawEntryData>,
) -> Result<(usize, usize), String> {
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let mut new_count = 0usize;
            let mut updated_count = 0usize;

            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

            for e in &entries {
                let exists: Option<i64> = if !e.guid.is_empty() {
                    match tx.query_row(
                        "SELECT id FROM entry WHERE feed_id = ?1 AND guid = ?2",
                        params![feed_id, e.guid],
                        |row| row.get(0),
                    ) {
                        Ok(id) => Some(id),
                        Err(rusqlite::Error::QueryReturnedNoRows) => None,
                        Err(e) => return Err(e.to_string()),
                    }
                } else if let Some(ref url) = e.url {
                    match tx.query_row(
                        "SELECT id FROM entry WHERE feed_id = ?1 AND url = ?2",
                        params![feed_id, url],
                        |row| row.get(0),
                    ) {
                        Ok(id) => Some(id),
                        Err(rusqlite::Error::QueryReturnedNoRows) => None,
                        Err(e) => return Err(e.to_string()),
                    }
                } else {
                    None
                };

                if let Some(existing_id) = exists {
                    tx.execute(
                        "UPDATE entry SET title = ?1, author = ?2, published_at = ?3,
                         summary = ?4, url = COALESCE(?5, url)
                         WHERE id = ?6",
                        params![e.title, e.author, e.published_at, e.summary, e.url, existing_id],
                    )
                    .map_err(|e| e.to_string())?;
                    updated_count += 1;
                } else {
                    tx.execute(
                        "INSERT INTO entry (feed_id, guid, url, title, author, published_at, summary)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![feed_id, e.guid, e.url, e.title, e.author, e.published_at, e.summary],
                    )
                    .map_err(|e| e.to_string())?;
                    new_count += 1;
                }
            }

            tx.commit().map_err(|e| e.to_string())?;
            Ok((new_count, updated_count))
        })
        .await
        .map_err(|e| e.to_string())?
}

pub struct RawEntryData {
    pub guid: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub summary: Option<String>,
}

pub async fn mark_read(pool: &Pool, entry_ids: &[i64], is_read: bool) -> Result<(), String> {
    let entry_ids = entry_ids.to_vec();
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let placeholders: Vec<String> = entry_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
            let sql = format!(
                "UPDATE entry SET is_read = ?{} WHERE id IN ({})",
                entry_ids.len() + 1,
                placeholders.join(",")
            );
            let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = entry_ids.iter().map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>).collect();
            params_vec.push(Box::new(is_read as i32));
            let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
            conn.execute(&sql, refs.as_slice()).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn mark_starred(pool: &Pool, entry_id: i64, is_starred: bool) -> Result<(), String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE entry SET is_starred = ?1 WHERE id = ?2",
                params![is_starred as i32, entry_id],
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn mark_all_read_in_feed(pool: &Pool, feed_id: i64) -> Result<(), String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE entry SET is_read = 1 WHERE feed_id = ?1 AND is_read = 0",
                params![feed_id],
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}
