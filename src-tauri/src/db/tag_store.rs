//! Canonical tag persistence. Every assignment and proposal uses this resolver.
use crate::db::models::{Tag, TagAlias};
use deadpool_sqlite::Pool;
use rusqlite::{params, Connection, OptionalExtension};

pub fn normalize(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| c.is_whitespace() || "-_.".contains(c))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
fn valid_name(name: &str) -> Result<String, String> {
    let normalized = normalize(name);
    if normalized.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
        return Err(
            "Tag name must contain text, at most 80 characters, without control characters".into(),
        );
    }
    Ok(normalized)
}
fn row_tag(row: &rusqlite::Row<'_>) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        name: row.get(1)?,
        normalized_name: row.get(2)?,
        is_provisional: row.get(3)?,
        usage_count: row.get(4)?,
    })
}
pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Tag> {
    conn.query_row(
        "SELECT id,name,normalized_name,is_provisional,usage_count FROM tag WHERE id=?1",
        [id],
        row_tag,
    )
}
pub fn resolve(conn: &Connection, normalized: &str) -> rusqlite::Result<Option<i64>> {
    conn.query_row("SELECT id FROM tag WHERE normalized_name=?1 UNION ALL SELECT tag_id FROM tag_alias WHERE alias_normalized=?1 LIMIT 1", [normalized], |r| r.get(0)).optional()
}
pub fn recount(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("UPDATE tag SET usage_count=(SELECT COUNT(*) FROM entry_tag et JOIN entry e ON e.id=et.entry_id WHERE et.tag_id=tag.id AND e.is_deleted=0)", [])?;
    conn.execute("UPDATE tag SET is_provisional=0 WHERE usage_count>=2 OR EXISTS(SELECT 1 FROM entry_tag et WHERE et.tag_id=tag.id AND et.source='manual')", [])?;
    Ok(())
}
pub fn assignment(
    conn: &Connection,
    entry_id: i64,
    name: &str,
    source: &str,
) -> Result<(Tag, usize), String> {
    let normalized = valid_name(name)?;
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM entry WHERE id=?1 AND is_deleted=0)",
            [entry_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !exists {
        return Err("Entry not found".into());
    }
    let id = match resolve(conn, &normalized).map_err(|e| e.to_string())? {
        Some(id) => id,
        None => {
            conn.execute(
                "INSERT INTO tag(name,normalized_name,is_provisional) VALUES(?1,?2,?3)",
                params![name.trim(), normalized, source != "manual"],
            )
            .map_err(|e| e.to_string())?;
            conn.last_insert_rowid()
        }
    };
    let inserted = conn
        .execute(
            "INSERT OR IGNORE INTO entry_tag(entry_id,tag_id,source) VALUES(?1,?2,?3)",
            params![entry_id, id, source],
        )
        .map_err(|e| e.to_string())?;
    if source == "manual" {
        conn.execute(
            "UPDATE entry_tag SET source='manual',confidence=NULL WHERE entry_id=?1 AND tag_id=?2",
            params![entry_id, id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("UPDATE tag SET is_provisional=0 WHERE id=?1", [id])
            .map_err(|e| e.to_string())?;
    }
    recount(conn).map_err(|e| e.to_string())?;
    Ok((get(conn, id).map_err(|e| e.to_string())?, inserted))
}
pub fn assert_mutable(conn: &Connection) -> Result<(), String> {
    let active: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tag_batch_run WHERE status IN ('running','ready'))",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if active {
        return Err("Finish or discard the tag batch before changing the tag library".into());
    }
    Ok(())
}
/// Alias conflicts never silently reassign another canonical record.
pub fn attach_alias(conn: &Connection, id: i64, alias: &str) -> Result<(), String> {
    let normalized = valid_name(alias)?;
    let canonical = get(conn, id).map_err(|e| e.to_string())?;
    if normalized == canonical.normalized_name {
        return Err("Alias matches canonical tag".into());
    }
    if resolve(conn, &normalized)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("Alias is already used by a tag or alias".into());
    }
    conn.execute(
        "INSERT INTO tag_alias(tag_id,alias_normalized,alias) VALUES(?1,?2,?3)",
        params![id, normalized, alias.trim()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
pub fn merge(conn: &Connection, source: i64, target: i64) -> Result<(), String> {
    if source == target {
        return Err("Cannot merge a tag into itself".into());
    }
    let old = get(conn, source).map_err(|e| e.to_string())?;
    let new = get(conn, target).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR IGNORE INTO entry_tag(entry_id,tag_id,source,confidence) SELECT entry_id,?1,source,confidence FROM entry_tag WHERE tag_id=?2",params![target,source]).map_err(|e|e.to_string())?;
    // Preserve manual intent even when the target already had an AI association.
    conn.execute("UPDATE entry_tag SET source='manual',confidence=NULL WHERE tag_id=?1 AND entry_id IN (SELECT entry_id FROM entry_tag WHERE tag_id=?2 AND source='manual')",params![target,source]).map_err(|e|e.to_string())?;
    conn.execute("DELETE FROM entry_tag WHERE tag_id=?1", [source])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tag_alias SET tag_id=?1 WHERE tag_id=?2",
        params![target, source],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM tag_alias WHERE tag_id=?1 AND alias_normalized=?2",
        params![target, new.normalized_name],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tag WHERE id=?1", [source])
        .map_err(|e| e.to_string())?;
    if normalize(&old.name) != new.normalized_name {
        match resolve(conn, &normalize(&old.name)).map_err(|e| e.to_string())? {
            Some(id) if id != target => {
                return Err("Merged name conflicts with another alias".into())
            }
            Some(_) => {}
            None => attach_alias(conn, target, &old.name)?,
        }
    }
    if !old.is_provisional {
        conn.execute("UPDATE tag SET is_provisional=0 WHERE id=?1", [target])
            .map_err(|e| e.to_string())?;
    }
    recount(conn).map_err(|e| e.to_string())
}
/// v4 upgrades existing spellings deterministically (lowest ID wins); aliases
/// colliding with a canonical name merge their old owner, preserving relations.
pub fn migrate_normalization(conn: &Connection) -> rusqlite::Result<()> {
    let tags: Vec<(i64, String)> = conn
        .prepare("SELECT id,name FROM tag ORDER BY id")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (id, _) in &tags {
        conn.execute(
            "UPDATE tag SET normalized_name=?1 WHERE id=?2",
            params![format!("\u{0}v4:{id}"), id],
        )?;
    }
    for (id, name) in tags {
        if get(conn, id).is_err() {
            continue;
        }
        let normalized = normalize(&name);
        let normalized = if normalized.is_empty() {
            format!("legacy tag {id}")
        } else {
            normalized
        };
        let target: Option<i64> = conn
            .query_row(
                "SELECT id FROM tag WHERE normalized_name=?1",
                [&normalized],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(target) = target {
            merge(conn, id, target).map_err(|_| rusqlite::Error::InvalidQuery)?;
        } else {
            conn.execute(
                "UPDATE tag SET normalized_name=?1 WHERE id=?2",
                params![normalized, id],
            )?;
        }
    }
    let aliases: Vec<(i64, i64, String)> = conn
        .prepare("SELECT id,tag_id,alias FROM tag_alias ORDER BY id")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    for (id, _, _) in &aliases {
        conn.execute(
            "UPDATE tag_alias SET alias_normalized=?1 WHERE id=?2",
            params![format!("\u{0}alias:{id}"), id],
        )?;
    }
    for (id, _, alias) in aliases {
        let owner: Option<i64> = conn
            .query_row("SELECT tag_id FROM tag_alias WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        let Some(owner) = owner else {
            continue;
        };
        let normalized = normalize(&alias);
        if normalized.is_empty() {
            conn.execute("DELETE FROM tag_alias WHERE id=?1", [id])?;
            continue;
        }
        if let Some(target) = resolve(conn, &normalized)? {
            conn.execute("DELETE FROM tag_alias WHERE id=?1", [id])?;
            if owner != target {
                merge(conn, owner, target).map_err(|_| rusqlite::Error::InvalidQuery)?;
            }
        } else {
            conn.execute(
                "UPDATE tag_alias SET alias_normalized=?1 WHERE id=?2",
                params![normalized, id],
            )?;
        }
    }
    recount(conn)
}

pub async fn list_tags(pool: &Pool, search: Option<&str>) -> Result<Vec<Tag>, String> {
    let search = format!(
        "%{}%",
        normalize(search.unwrap_or(""))
            .replace('\\', "\\\\")
            .replace('%', "\\%")
    );
    pool.get().await.map_err(|e|e.to_string())?.interact(move|conn| {
        conn.prepare("SELECT id,name,normalized_name,is_provisional,usage_count FROM tag WHERE normalized_name LIKE ?1 ESCAPE '\\' OR id IN (SELECT tag_id FROM tag_alias WHERE alias_normalized LIKE ?1 ESCAPE '\\') ORDER BY usage_count DESC,name,id")
            .map_err(|e|e.to_string())?.query_map([search],row_tag).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())
    }).await.map_err(|e|e.to_string())?
}
pub async fn add_tag(pool: &Pool, entry_id: i64, name: &str) -> Result<Tag, String> {
    let name = name.to_owned();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            let (tag, _) = assignment(&tx, entry_id, &name, "manual")?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(tag)
        })
        .await
        .map_err(|e| e.to_string())?
}
pub async fn remove_tag(pool: &Pool, entry_id: i64, tag_id: i64) -> Result<(), String> {
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            tx.execute(
                "DELETE FROM entry_tag WHERE entry_id=?1 AND tag_id=?2",
                params![entry_id, tag_id],
            )
            .map_err(|e| e.to_string())?;
            recount(&tx).map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
}
pub async fn mutate(pool: &Pool, op: &str, id: i64, value: &str) -> Result<usize, String> {
    let op = op.to_owned();
    let value = value.to_owned();
    pool.get().await.map_err(|e|e.to_string())?.interact(move|conn|{
        let tx=conn.unchecked_transaction().map_err(|e|e.to_string())?;
        assert_mutable(&tx)?;
        let count=match op.as_str(){
            "merge"=>{merge(&tx,id,value.parse().map_err(|_|"Invalid target")?)?;1},
            "rename"=>{
                let normalized=valid_name(&value)?;let old=get(&tx,id).map_err(|e|e.to_string())?;
                if let Some(owner)=resolve(&tx,&normalized).map_err(|e|e.to_string())?{if owner!=id{return Err("Name conflicts with another tag or alias".into());}}
                tx.execute("DELETE FROM tag_alias WHERE tag_id=?1 AND alias_normalized=?2",params![id,normalized]).map_err(|e|e.to_string())?;
                tx.execute("UPDATE tag SET name=?1,normalized_name=?2 WHERE id=?3",params![value.trim(),normalized,id]).map_err(|e|e.to_string())?;
                if old.normalized_name!=normalized{attach_alias(&tx,id,&old.name)?;}1
            },
            "alias"=>{attach_alias(&tx,id,&value)?;1},
            "unalias"=>tx.execute("DELETE FROM tag_alias WHERE id=?1",[id]).map_err(|e|e.to_string())?,
            "promote"=>tx.execute("UPDATE tag SET is_provisional=0 WHERE id=?1",[id]).map_err(|e|e.to_string())?,
            "delete"=>tx.execute("DELETE FROM tag WHERE id=?1",[id]).map_err(|e|e.to_string())?,
            "cleanup"=>{recount(&tx).map_err(|e|e.to_string())?;tx.execute("DELETE FROM tag WHERE NOT EXISTS(SELECT 1 FROM entry_tag et JOIN entry e ON e.id=et.entry_id WHERE et.tag_id=tag.id AND e.is_deleted=0)",[]).map_err(|e|e.to_string())?},
            _=>return Err("Unknown tag mutation".into())
        };
        recount(&tx).map_err(|e|e.to_string())?;tx.commit().map_err(|e|e.to_string())?;Ok(count)
    }).await.map_err(|e|e.to_string())?
}
pub async fn merge_tags(pool: &Pool, source_id: i64, target_id: i64) -> Result<(), String> {
    mutate(pool, "merge", source_id, &target_id.to_string())
        .await
        .map(|_| ())
}
pub async fn rename_tag(pool: &Pool, id: i64, name: &str) -> Result<(), String> {
    mutate(pool, "rename", id, name).await.map(|_| ())
}
pub async fn delete_tag(pool: &Pool, id: i64) -> Result<(), String> {
    mutate(pool, "delete", id, "").await.map(|_| ())
}
pub async fn list_aliases(pool: &Pool, id: i64) -> Result<Vec<TagAlias>, String> {
    pool.get().await.map_err(|e|e.to_string())?.interact(move|conn|{
        conn.prepare("SELECT id,tag_id,alias_normalized,alias FROM tag_alias WHERE tag_id=?1 ORDER BY alias_normalized")
        .map_err(|e|e.to_string())?.query_map([id],|r|Ok(TagAlias{id:r.get(0)?,tag_id:r.get(1)?,alias_normalized:r.get(2)?,alias:r.get(3)?})).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string())
    }).await.map_err(|e|e.to_string())?
}
pub async fn canonical_names(pool: &Pool, names: Vec<String>) -> Result<Vec<String>, String> {
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();
            for name in names {
                let Ok(normalized) = valid_name(&name) else {
                    continue;
                };
                let name = match resolve(conn, &normalized).map_err(|e| e.to_string())? {
                    Some(id) => get(conn, id).map_err(|e| e.to_string())?.name,
                    None => name.trim().to_owned(),
                };
                if seen.insert(normalize(&name)) {
                    result.push(name);
                }
            }
            Ok(result)
        })
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "tag_store_tests.rs"]
mod tests;
