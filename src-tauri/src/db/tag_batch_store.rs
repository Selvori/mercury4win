//! Durable review staging, never article/tag writes during generation.
use crate::db::tag_store;
use deadpool_sqlite::Pool;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MIGRATION_V4: &str = "
ALTER TABLE tag_alias ADD COLUMN alias TEXT NOT NULL DEFAULT '';
UPDATE tag_alias SET alias=alias_normalized;
CREATE INDEX entry_tag_tag_entry ON entry_tag(tag_id,entry_id);
CREATE TABLE tag_batch_run (
 id TEXT PRIMARY KEY, status TEXT NOT NULL, scope_json TEXT NOT NULL,
 total INTEGER NOT NULL, configuration TEXT NOT NULL,
 kept_json TEXT, result_json TEXT, created_at TEXT NOT NULL DEFAULT(datetime('now'))
);
CREATE UNIQUE INDEX tag_batch_one_live ON tag_batch_run((1)) WHERE status IN ('running','ready');
CREATE TABLE tag_batch_item (
 run_id TEXT NOT NULL REFERENCES tag_batch_run(id) ON DELETE CASCADE,
 entry_id INTEGER NOT NULL, source_hash TEXT, names_json TEXT, error TEXT,
 PRIMARY KEY(run_id,entry_id)
);
";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchScope {
    pub from: Option<String>,
    pub to: Option<String>,
    pub unread_only: bool,
    pub skip_tagged: bool,
    pub skip_applied: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProposal {
    pub name: String,
    pub normalized_name: String,
    pub hits: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFailure {
    pub entry_id: i64,
    pub error: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchApplyResult {
    pub entries_processed: usize,
    pub tags_assigned: usize,
    pub skipped_deleted: usize,
    pub skipped_changed: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSnapshot {
    pub id: String,
    pub status: String,
    pub scope: BatchScope,
    pub total: usize,
    pub processed: usize,
    pub succeeded: usize,
    pub failures: Vec<BatchFailure>,
    pub proposals: Vec<BatchProposal>,
    pub result: Option<BatchApplyResult>,
}
#[derive(Debug, Clone)]
pub struct TaggingSource {
    pub title: String,
    pub author: String,
    pub content: String,
    pub hash: String,
}
fn source_fields(conn: &Connection, id: i64) -> rusqlite::Result<Option<Vec<Option<String>>>> {
    conn.query_row(
        "SELECT e.title,e.author,e.url,e.summary,e.feed_content,e.feed_content_type,
                c.markdown,CAST(c.markdown_version AS TEXT)
         FROM entry e LEFT JOIN content c ON c.entry_id=e.id
         WHERE e.id=?1 AND e.is_deleted=0",
        [id],
        |r| (0..8).map(|i| r.get(i)).collect(),
    )
    .optional()
}
fn fields_hash(source: &[Option<String>]) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(source).unwrap()))
}
pub fn source_hash(conn: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    Ok(source_fields(conn, id)?.map(|source| fields_hash(&source)))
}
/// Prompt input and its identity are read in one SQLite snapshot. A Feed/content
/// update between source loading and HTTP cannot masquerade as the analyzed source.
pub async fn load_source(pool: &Pool, id: i64) -> Result<TaggingSource, String> {
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let source = source_fields(conn, id)
                .map_err(|e| e.to_string())?
                .ok_or("Entry not found")?;
            Ok(TaggingSource {
                title: source[0].clone().unwrap_or_default(),
                author: source[1].clone().unwrap_or_default(),
                content: source[6]
                    .as_ref()
                    .filter(|s| !s.trim().is_empty())
                    .or(source[3].as_ref())
                    .cloned()
                    .unwrap_or_default(),
                hash: fields_hash(&source),
            })
        })
        .await
        .map_err(|e| e.to_string())?
}
fn selection(conn: &Connection, scope: &BatchScope) -> Result<Vec<i64>, String> {
    for date in [&scope.from, &scope.to].into_iter().flatten() {
        chrono::DateTime::parse_from_rfc3339(date)
            .map_err(|_| "Invalid batch date: expected RFC3339")?;
    }
    if let (Some(from), Some(to)) = (&scope.from, &scope.to) {
        if chrono::DateTime::parse_from_rfc3339(from).unwrap()
            > chrono::DateTime::parse_from_rfc3339(to).unwrap()
        {
            return Err("Batch start must precede end".into());
        }
    }
    conn.prepare("SELECT e.id FROM entry e WHERE e.is_deleted=0 AND (?1 IS NULL OR julianday(COALESCE(e.published_at,e.created_at))>=julianday(?1)) AND (?2 IS NULL OR julianday(COALESCE(e.published_at,e.created_at))<=julianday(?2)) AND (?3=0 OR e.is_read=0) AND (?4=0 OR NOT EXISTS(SELECT 1 FROM entry_tag et WHERE et.entry_id=e.id)) AND (?5=0 OR NOT EXISTS(SELECT 1 FROM tag_batch_item i JOIN tag_batch_run r ON r.id=i.run_id WHERE i.entry_id=e.id AND r.status='done' AND i.names_json IS NOT NULL AND i.error IS NULL)) ORDER BY COALESCE(e.published_at,e.created_at) DESC,e.id DESC")
 .map_err(|e|e.to_string())?.query_map(params![scope.from,scope.to,scope.unread_only,scope.skip_tagged,scope.skip_applied],|r|r.get(0)).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string())
}
pub async fn count(pool: &Pool, scope: BatchScope) -> Result<usize, String> {
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| selection(conn, &scope).map(|ids| ids.len()))
        .await
        .map_err(|e| e.to_string())?
}
pub async fn start(
    pool: &Pool,
    id: &str,
    scope: BatchScope,
    configuration: &str,
    confirmed_large: bool,
) -> Result<Vec<i64>, String> {
    let fence = crate::agent::runtime::current_commit_fence();
    let id = id.to_owned();
    let configuration = configuration.to_owned();
    pool.get().await.map_err(|e|e.to_string())?.interact(move|conn|{
 let tx=conn.unchecked_transaction().map_err(|e|e.to_string())?;let ids=selection(&tx,&scope)?;
 if ids.is_empty(){return Err("No matching entries".into());}
 if ids.len()>100 && !confirmed_large{return Err("Confirm batches over 100 entries before starting".into());}
 tx.execute("INSERT INTO tag_batch_run(id,status,scope_json,total,configuration) VALUES(?1,'running',?2,?3,?4)",params![id,serde_json::to_string(&scope).unwrap(),ids.len() as i64,configuration]).map_err(|e|e.to_string())?;
 for entry in &ids {tx.execute("INSERT INTO tag_batch_item(run_id,entry_id) VALUES(?1,?2)",params![id,entry]).map_err(|e|e.to_string())?;}
 fence.commit(tx)?;Ok(ids)
 }).await.map_err(|e|e.to_string())?
}
pub async fn stage(
    pool: &Pool,
    id: &str,
    entry: i64,
    hash: Option<String>,
    result: Result<Vec<String>, String>,
) -> Result<(), String> {
    let fence = crate::agent::runtime::current_commit_fence();
    let id = id.to_owned();
    pool.get().await.map_err(|e|e.to_string())?.interact(move|conn|{
 let tx=conn.unchecked_transaction().map_err(|e|e.to_string())?;
 let (names,error)=match result{Ok(names)=>(Some(serde_json::to_string(&names).unwrap()),None),Err(e)=>(None,Some(e))};
 tx.execute("UPDATE tag_batch_item SET source_hash=?1,names_json=?2,error=?3 WHERE run_id=?4 AND entry_id=?5 AND EXISTS(SELECT 1 FROM tag_batch_run WHERE id=?4 AND status='running')",params![hash,names,error,id,entry]).map_err(|e|e.to_string())?;
 fence.commit(tx)
 }).await.map_err(|e|e.to_string())?
}
pub async fn hash(pool: &Pool, entry: i64) -> Result<Option<String>, String> {
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| source_hash(conn, entry).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}
pub async fn finish(pool: &Pool, id: &str) -> Result<(), String> {
    let id = id.to_owned();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE tag_batch_run SET status='ready' WHERE id=?1 AND status='running'",
                [id],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
}
fn snapshot(conn: &Connection, id: &str) -> Result<BatchSnapshot, String> {
    let (status, scope, total, result): (String, String, i64, Option<String>) = conn
        .query_row(
            "SELECT status,scope_json,total,result_json FROM tag_batch_run WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| e.to_string())?;
    let rows:Vec<(i64,Option<String>,Option<String>)>=conn.prepare("SELECT entry_id,names_json,error FROM tag_batch_item WHERE run_id=?1 ORDER BY entry_id").map_err(|e|e.to_string())?.query_map([id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string())?;
    let mut failures = Vec::new();
    let mut succeeded = 0;
    let mut proposals = std::collections::BTreeMap::<String, BatchProposal>::new();
    for (entry, names, error) in rows {
        if let Some(error) = error {
            failures.push(BatchFailure {
                entry_id: entry,
                error,
            });
        } else if let Some(names) = names {
            succeeded += 1;
            for name in serde_json::from_str::<Vec<String>>(&names).map_err(|e| e.to_string())? {
                let normalized = tag_store::normalize(&name);
                if tag_store::resolve(conn, &normalized)
                    .map_err(|e| e.to_string())?
                    .is_none()
                {
                    let proposal = proposals
                        .entry(normalized.clone())
                        .or_insert(BatchProposal {
                            name,
                            normalized_name: normalized,
                            hits: 0,
                        });
                    proposal.hits += 1;
                }
            }
        }
    }
    Ok(BatchSnapshot {
        id: id.into(),
        status,
        scope: serde_json::from_str(&scope).map_err(|e| e.to_string())?,
        total: total as usize,
        processed: succeeded + failures.len(),
        succeeded,
        failures,
        proposals: proposals.into_values().collect(),
        result: result
            .map(|r| serde_json::from_str(&r))
            .transpose()
            .map_err(|e| e.to_string())?,
    })
}
pub async fn load(pool: &Pool, id: Option<String>) -> Result<Option<BatchSnapshot>, String> {
    pool.get().await.map_err(|e|e.to_string())?.interact(move|conn|{
 let id=match id{Some(id)=>Some(id),None=>conn.query_row("SELECT id FROM tag_batch_run WHERE status IN ('running','ready') ORDER BY created_at DESC LIMIT 1",[],|r|r.get(0)).optional().map_err(|e|e.to_string())?};
 match id {
 Some(id) => {let exists:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM tag_batch_run WHERE id=?1)",[&id],|r|r.get(0)).map_err(|e|e.to_string())?;if exists {snapshot(conn,&id).map(Some)}else{Ok(None)}},
 None=>Ok(None)
 }
 }).await.map_err(|e|e.to_string())?
}
pub async fn discard(pool: &Pool, id: &str) -> Result<(), String> {
    let id = id.to_owned();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            let changed = tx
                .execute(
                    "UPDATE tag_batch_run SET status='aborted' WHERE id=?1 AND status='ready'",
                    [&id],
                )
                .map_err(|e| e.to_string())?;
            if changed != 1 {
                return Err("Stop generation before discarding its review".into());
            }
            tx.execute("DELETE FROM tag_batch_item WHERE run_id=?1", [id])
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
}
pub async fn apply(
    pool: &Pool,
    id: &str,
    kept: Vec<String>,
    discarded: Vec<String>,
) -> Result<BatchApplyResult, String> {
    let id = id.to_owned();
    pool.get().await.map_err(|e|e.to_string())?.interact(move|conn|{
 let tx=conn.unchecked_transaction().map_err(|e|e.to_string())?;
 let review=snapshot(&tx,&id)?;
 if review.status=="done"{return review.result.ok_or("Missing applied result".into());}
 if review.status!="ready"{return Err("Generation must stop before review/apply".into());}
 let kept:std::collections::HashSet<_>=kept.into_iter().collect();let discarded:std::collections::HashSet<_>=discarded.into_iter().collect();
 if !kept.is_disjoint(&discarded){return Err("A proposal cannot be both kept and discarded".into());}
 let expected:std::collections::HashSet<_>=review.proposals.iter().map(|p|p.normalized_name.clone()).collect();
 let supplied:std::collections::HashSet<_>=kept.union(&discarded).cloned().collect();
 if supplied!=expected{return Err("Review every current new proposal before applying".into());}
 let rows:Vec<(i64,String,String)>=tx.prepare("SELECT entry_id,source_hash,names_json FROM tag_batch_item WHERE run_id=?1 AND names_json IS NOT NULL AND error IS NULL").map_err(|e|e.to_string())?.query_map([&id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string())?;
 let mut result=BatchApplyResult::default();
 for (entry,hash,names) in rows{
 let current=source_hash(&tx,entry).map_err(|e|e.to_string())?;
 let error=if current.is_none(){result.skipped_deleted+=1;Some("Entry deleted before apply")}else if current.as_deref()!=Some(&hash){result.skipped_changed+=1;Some("Article source changed; analyze it again")}else{None};
 if let Some(error)=error{tx.execute("UPDATE tag_batch_item SET error=?1 WHERE run_id=?2 AND entry_id=?3",params![error,id,entry]).map_err(|e|e.to_string())?;continue;}
 for name in serde_json::from_str::<Vec<String>>(&names).map_err(|e|e.to_string())? {
 let normalized=tag_store::normalize(&name);
 // Discard remains a discard even if a concurrent manual path created it.
 if discarded.contains(&normalized){continue;}
 if tag_store::resolve(&tx,&normalized).map_err(|e|e.to_string())?.is_none()&&!kept.contains(&normalized){return Err("Tag library changed; reload the review".into());}
 result.tags_assigned+=tag_store::assignment(&tx,entry,&name,"ai_batch")?.1;
 }
 result.entries_processed+=1;
 }
 tag_store::recount(&tx).map_err(|e|e.to_string())?;
 tx.execute("UPDATE tag_batch_run SET status='done',kept_json=?1,result_json=?2 WHERE id=?3",params![serde_json::to_string(&kept).unwrap(),serde_json::to_string(&result).unwrap(),id]).map_err(|e|e.to_string())?;
 // Keep tiny item markers for skip_applied; remove heavy suggestion texts.
 tx.execute("UPDATE tag_batch_item SET names_json='[]',source_hash=NULL WHERE run_id=?1 AND names_json IS NOT NULL",[id]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())?;Ok(result)
 }).await.map_err(|e|e.to_string())?
}
