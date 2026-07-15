// mercury4win-linux/src-tauri/db/tag_store.rs
// Tag persistence — CRUD, merge, alias management, batch tagging

use deadpool_sqlite::Pool;
use rusqlite::params;
use crate::db::models::{Tag, BatchTagResult};

pub async fn list_tags(pool: &Pool, search: Option<&str>) -> Result<Vec<Tag>, String> {
    let search = search.map(|s| format!("%{}%", s));
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let (sql, params_ref): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match search {
                Some(ref q) => {
                    let mut v: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
                    v.push(Box::new(q.clone()));
                    ("SELECT id, name, normalized_name, is_provisional, usage_count
                      FROM tag WHERE name LIKE ?1 ORDER BY usage_count DESC, name ASC", v)
                }
                None => {
                    ( "SELECT id, name, normalized_name, is_provisional, usage_count
                       FROM tag ORDER BY usage_count DESC, name ASC", vec![])
                }
            };

            let refs: Vec<&dyn rusqlite::types::ToSql> = params_ref.iter().map(|b| b.as_ref()).collect();
            let mut stmt = conn
                .prepare(sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(refs.as_slice(), |row| {
                    Ok(Tag {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        normalized_name: row.get(2)?,
                        is_provisional: row.get::<_, i32>(3)? != 0,
                        usage_count: row.get(4)?,
                    })
                })
                .map_err(|e| e.to_string())?;

            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
}

pub async fn add_tag(pool: &Pool, entry_id: i64, name: &str) -> Result<Tag, String> {
    let name = name.trim().to_string();
    let normalized = name.to_lowercase();
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

            // Upsert tag
            tx.execute(
                "INSERT INTO tag (name, normalized_name) VALUES (?1, ?2)
                 ON CONFLICT(normalized_name) DO UPDATE SET usage_count = usage_count + 1",
                params![name, normalized],
            )
            .map_err(|e| e.to_string())?;

            let tag: Tag = tx
                .query_row(
                    "SELECT id, name, normalized_name, is_provisional, usage_count
                     FROM tag WHERE normalized_name = ?1",
                    params![normalized],
                    |row| {
                        Ok(Tag {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            normalized_name: row.get(2)?,
                            is_provisional: row.get::<_, i32>(3)? != 0,
                            usage_count: row.get(4)?,
                        })
                    },
                )
                .map_err(|e| e.to_string())?;

            // Link to entry
            tx.execute(
                "INSERT OR IGNORE INTO entry_tag (entry_id, tag_id, source) VALUES (?1, ?2, 'manual')",
                params![entry_id, tag.id],
            )
            .map_err(|e| e.to_string())?;

            tx.commit().map_err(|e| e.to_string())?;
            Ok(tag)
        })
        .await
        .map_err(|e| e.to_string())?
}

pub async fn remove_tag(pool: &Pool, entry_id: i64, tag_id: i64) -> Result<(), String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM entry_tag WHERE entry_id = ?1 AND tag_id = ?2",
                params![entry_id, tag_id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "UPDATE tag SET usage_count = (SELECT COUNT(*) FROM entry_tag WHERE tag_id = ?1) WHERE id = ?1",
                params![tag_id],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn merge_tags(pool: &Pool, source_id: i64, target_id: i64) -> Result<(), String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            // Move all entry_tag associations
            tx.execute(
                "INSERT OR IGNORE INTO entry_tag (entry_id, tag_id, source)
                 SELECT entry_id, ?1, source FROM entry_tag WHERE tag_id = ?2",
                params![target_id, source_id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM entry_tag WHERE tag_id = ?2",
                params![source_id],
            )
            .map_err(|e| e.to_string())?;
            // Update usage counts
            tx.execute(
                "UPDATE tag SET usage_count = (SELECT COUNT(*) FROM entry_tag WHERE tag_id = ?1) WHERE id = ?1",
                params![target_id],
            )
            .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM tag WHERE id = ?1", params![source_id])
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn rename_tag(pool: &Pool, tag_id: i64, new_name: &str) -> Result<(), String> {
    let name = new_name.trim().to_string();
    let normalized = name.to_lowercase();
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE tag SET name = ?1, normalized_name = ?2 WHERE id = ?3",
                params![name, normalized, tag_id],
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_tag(pool: &Pool, tag_id: i64) -> Result<(), String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute("DELETE FROM tag WHERE id = ?1", params![tag_id])
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn batch_tag(
    pool: &Pool,
    from: &str,
    to: &str,
    tag_ids: &[i64],
) -> Result<BatchTagResult, String> {
    let from = from.to_string();
    let to = to.to_string();
    let tag_ids = tag_ids.to_vec();
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

            let entries_processed = {
                let mut stmt = tx
                    .prepare(
                        "SELECT id FROM entry
                         WHERE created_at BETWEEN ?1 AND ?2 AND is_deleted = 0",
                    )
                    .map_err(|e| e.to_string())?;
                let ids: Vec<i64> = stmt
                    .query_map(params![from, to], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .filter_map(|r| r.ok())
                    .collect();

                let count = ids.len();
                for entry_id in &ids {
                    for tag_id in &tag_ids {
                        tx.execute(
                            "INSERT OR IGNORE INTO entry_tag (entry_id, tag_id, source) VALUES (?1, ?2, 'batch')",
                            params![entry_id, tag_id],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
                count
            };

            // Update usage counts
            for tag_id in &tag_ids {
                tx.execute(
                    "UPDATE tag SET usage_count = (SELECT COUNT(*) FROM entry_tag WHERE tag_id = ?1) WHERE id = ?1",
                    params![tag_id],
                )
                .map_err(|e| e.to_string())?;
            }

            tx.commit().map_err(|e| e.to_string())?;
            Ok(BatchTagResult {
                entries_processed,
                tags_assigned: tag_ids.len(),
            })
        })
        .await
        .map_err(|e| e.to_string())?
}
