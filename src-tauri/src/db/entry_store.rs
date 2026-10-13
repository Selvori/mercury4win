// mercury4win/src-tauri/db/entry_store.rs
// Entry persistence — paginated queries, read/star/delete, batch operations

use crate::db::models::{Entry, EntryCursor, EntryDetail, EntryPage, EntryTagInfo};
use deadpool_sqlite::Pool;
use rusqlite::{params, OptionalExtension};

#[cfg(test)]
mod m4w_006_tests;
#[cfg(test)]
mod m4w_011_tests;

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagMatchMode {
    #[default]
    Any,
    All,
}

#[derive(Debug, Clone, Default)]
pub struct EntryFilter {
    pub feed_id: Option<i64>,
    pub unread_only: bool,
    pub starred_only: bool,
    pub search: Option<String>,
    pub tag_ids: Option<Vec<i64>>,
    pub tag_mode: TagMatchMode,
}

impl EntryFilter {
    fn sql(&self) -> (Vec<String>, Vec<rusqlite::types::Value>) {
        let mut conditions = vec!["e.is_deleted = 0".to_string()];
        let mut values = vec![];
        if let Some(id) = self.feed_id {
            conditions.push(format!("e.feed_id = ?{}", values.len() + 1));
            values.push(id.into());
        }
        if self.unread_only {
            conditions.push("e.is_read = 0".into());
        }
        if self.starred_only {
            conditions.push("e.is_starred = 1".into());
        }
        if let Some(q) = self
            .search
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            let index = values.len() + 1;
            conditions.push(format!(
                "(e.title LIKE ?{index} OR e.summary LIKE ?{index})"
            ));
            values.push(format!("%{q}%").into());
        }
        let mut ids = self.tag_ids.clone().unwrap_or_default();
        ids.sort_unstable();
        ids.dedup();
        if !ids.is_empty() {
            match self.tag_mode {
                TagMatchMode::Any => {
                    let placeholders = (0..ids.len())
                        .map(|i| format!("?{}", values.len() + i + 1))
                        .collect::<Vec<_>>()
                        .join(",");
                    conditions.push(format!(
                        "e.id IN (SELECT entry_id FROM entry_tag WHERE tag_id IN ({placeholders}))"
                    ));
                    values.extend(ids.into_iter().map(rusqlite::types::Value::from));
                }
                TagMatchMode::All => {
                    for id in ids {
                        conditions.push(format!(
                            "e.id IN (SELECT entry_id FROM entry_tag WHERE tag_id = ?{})",
                            values.len() + 1
                        ));
                        values.push(id.into());
                    }
                }
            }
        }
        (conditions, values)
    }
}

pub struct EntryQueryParams {
    pub filter: EntryFilter,
    pub cursor: Option<EntryCursor>,
    pub limit: u32,
}

pub async fn query_entries(pool: &Pool, params: &EntryQueryParams) -> Result<EntryPage, String> {
    let filter = params.filter.clone();
    let cursor = params.cursor.clone();
    let limit = params.limit.clamp(1, 500);
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let (mut conditions, mut bind_values) = filter.sql();
            // Count the entire filtered scope, independently of pagination.
            let count_sql = format!("SELECT COUNT(*) FROM entry e WHERE {}", conditions.join(" AND "));
            let total: i64 = {
                let mut stmt = conn.prepare(&count_sql).map_err(|e| e.to_string())?;
                let refs: Vec<&dyn rusqlite::types::ToSql> =
                    bind_values.iter().map(|b| b as &dyn rusqlite::types::ToSql).collect();
                stmt.query_row(refs.as_slice(), |row| row.get(0))
                    .map_err(|e| e.to_string())?
            };

            // SQLite puts NULL dates last in DESC order. Use the same complete
            // ordering tuple, including NULL, rather than insertion order alone.
            if let Some(c) = cursor {
                let id_index = bind_values.len() + 1;
                bind_values.push(c.id.into());
                if let Some(published_at) = c.published_at {
                    let date_index = bind_values.len() + 1;
                    bind_values.push(published_at.into());
                    conditions.push(format!(
                        "(e.published_at < ?{date_index} OR (e.published_at = ?{date_index} AND e.id < ?{id_index}) OR e.published_at IS NULL)"
                    ));
                } else {
                    conditions.push(format!("(e.published_at IS NULL AND e.id < ?{id_index})"));
                }
            }
            let where_clause = conditions.join(" AND ");

            // Paginated query
            let query_sql = format!(
                "SELECT e.id, e.feed_id, e.guid, e.url, e.title, e.author,
                        e.published_at, e.summary, e.is_read, e.is_starred,
                        e.is_deleted, e.created_at, e.feed_content, e.feed_content_type
                 FROM entry e
                 WHERE {}
                 ORDER BY e.published_at DESC, e.id DESC
                 LIMIT ?{}",
                where_clause,
                bind_values.len() + 1
            );
            bind_values.push((i64::from(limit) + 1).into());

            let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
            let refs: Vec<&dyn rusqlite::types::ToSql> =
                bind_values.iter().map(|b| b as &dyn rusqlite::types::ToSql).collect();

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
                        feed_content: row.get(12)?,
                        feed_content_type: row.get(13)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            let entries: Result<Vec<_>, _> = rows.collect();
            let mut entries = entries.map_err(|e| e.to_string())?;
            let has_more = entries.len() > limit as usize;
            entries.truncate(limit as usize);
            let next_cursor = if has_more {
                entries.last().map(|e| EntryCursor {
                    published_at: e.published_at.clone(),
                    id: e.id,
                })
            } else {
                None
            };

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
            let entry_result: Result<Option<(Entry, Option<String>)>, String> = conn
                .query_row(
                    "SELECT e.id, e.feed_id, e.guid, e.url, e.title, e.author,
                            e.published_at, e.summary, e.is_read, e.is_starred,
                            e.is_deleted, e.created_at, f.title, e.feed_content, e.feed_content_type
                     FROM entry e
                     LEFT JOIN feed f ON f.id = e.feed_id
                     WHERE e.id = ?1 AND e.is_deleted = 0",
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
                            feed_content: row.get(13)?,
                            feed_content_type: row.get(14)?,
                        };
                        let feed_title: Option<String> = row.get(12)?;
                        Ok((entry, feed_title))
                    },
                )
                .optional()
                .map_err(|e| e.to_string());

            match entry_result {
                Ok(None) => Ok(None),
                Err(e) => Err(e),
                Ok(Some((entry, feed_title))) => {
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
                    // Only invalidate on source changes. Repeated sync must not
                    // discard a downloaded full article or any user/AI data.
                    let source_changed: bool = tx.query_row(
                        "SELECT feed_content IS NOT ?1 OR feed_content_type IS NOT ?2
                         OR url IS NOT COALESCE(?3, url) FROM entry WHERE id = ?4",
                        params![e.feed_content, e.feed_content_type, e.url, existing_id],
                        |row| row.get(0),
                    ).map_err(|e| e.to_string())?;
                    if source_changed {
                        tx.execute("DELETE FROM content_html_cache WHERE entry_id = ?1", [existing_id])
                            .map_err(|e| e.to_string())?;
                        tx.execute("DELETE FROM content WHERE entry_id = ?1", [existing_id])
                            .map_err(|e| e.to_string())?;
                    }
                    tx.execute(
                        "UPDATE entry SET title = ?1, author = ?2, published_at = ?3,
                         summary = ?4, url = COALESCE(?5, url), feed_content = ?7, feed_content_type = ?8
                         WHERE id = ?6",
                        params![e.title, e.author, e.published_at, e.summary, e.url, existing_id, e.feed_content, e.feed_content_type],
                    )
                    .map_err(|e| e.to_string())?;
                    updated_count += 1;
                } else {
                    let affected = tx.execute(
                        "INSERT OR IGNORE INTO entry (feed_id, guid, url, title, author, published_at, summary, feed_content, feed_content_type)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        params![feed_id, e.guid, e.url, e.title, e.author, e.published_at, e.summary, e.feed_content, e.feed_content_type],
                    )
                    .map_err(|e| e.to_string())?;
                    if affected > 0 {
                        new_count += 1;
                    }
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
    pub feed_content: Option<String>,
    pub feed_content_type: Option<String>,
}

pub async fn mark_read(pool: &Pool, entry_ids: &[i64], is_read: bool) -> Result<(), String> {
    let entry_ids = entry_ids.to_vec();
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let placeholders: Vec<String> = entry_ids
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            let sql = format!(
                "UPDATE entry SET is_read = ?{} WHERE id IN ({})",
                entry_ids.len() + 1,
                placeholders.join(",")
            );
            let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = entry_ids
                .iter()
                .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
                .collect();
            params_vec.push(Box::new(is_read as i32));
            let refs: Vec<&dyn rusqlite::types::ToSql> =
                params_vec.iter().map(|b| b.as_ref()).collect();
            conn.execute(&sql, refs.as_slice())
                .map_err(|e| e.to_string())
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

/// Query-scoped update deliberately excludes page cursors and limits.
pub async fn mark_all_read_in_scope(pool: &Pool, filter: &EntryFilter) -> Result<(), String> {
    let filter = filter.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let (conditions, values) = filter.sql();
            let sql = format!(
                "UPDATE entry AS e SET is_read = 1 WHERE {}",
                conditions.join(" AND ")
            );
            conn.execute(&sql, rusqlite::params_from_iter(values))
                .map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| e.to_string())?
}

pub async fn delete_entry(pool: &Pool, entry_id: i64) -> Result<(), String> {
    pool.get().await.map_err(|e| e.to_string())?.interact(move |conn| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("UPDATE entry SET is_deleted = 1 WHERE id = ?1", [entry_id]).map_err(|e| e.to_string())?;
        tx.execute("UPDATE tag SET usage_count = (SELECT COUNT(*) FROM entry_tag et JOIN entry e ON e.id = et.entry_id WHERE et.tag_id = tag.id AND e.is_deleted = 0) WHERE id IN (SELECT tag_id FROM entry_tag WHERE entry_id = ?1)", [entry_id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }).await.map_err(|e| e.to_string())?
}
