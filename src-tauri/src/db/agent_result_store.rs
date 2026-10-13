//! Durable success slots and translation checkpoints. No partial summary writes.
use crate::agent::runtime::{current_commit_fence, CommitFence};
use deadpool_sqlite::Pool;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const TRANSLATION_SEGMENTER: &str = "reader-html-block-v3";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentResult {
    pub source_hash: String,
    pub text: Option<String>,
    pub segments: Vec<Option<String>>,
    pub complete: bool,
    pub model_name: String,
    #[serde(default)]
    pub request_route: Option<crate::agent::provider::RequestRoute>,
    #[serde(default)]
    pub segment_routes: Vec<Option<crate::agent::provider::RequestRoute>>,
    #[serde(default)]
    pub segment_ids: Vec<String>,
    #[serde(default)]
    pub segment_errors: Vec<Option<String>>,
}

#[derive(Clone)]
pub struct SourceSnapshot {
    pub title: String,
    pub author: String,
    pub markdown: String,
    pub hash: String,
}

fn source(conn: &Connection, entry_id: i64) -> Result<SourceSnapshot, String> {
    conn.query_row(
        "SELECT e.title,e.author,e.url,e.summary,e.feed_content,e.feed_content_type,
                c.markdown,CAST(c.markdown_version AS TEXT),c.cleaned_html,CAST(c.readability_version AS TEXT)
         FROM entry e LEFT JOIN content c ON c.entry_id=e.id WHERE e.id=?1 AND e.is_deleted=0",
        [entry_id], |row| {
            let values: Vec<Option<String>> = (0..10).map(|i| row.get(i)).collect::<Result<_,_>>()?;
            let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&values).unwrap()));
            Ok(SourceSnapshot { title: values[0].clone().unwrap_or_default(), author: values[1].clone().unwrap_or_default(), markdown: values[6].clone().unwrap_or_default(), hash })
        }).map_err(|e| e.to_string())
}

pub async fn source_snapshot(pool: &Pool, entry_id: i64) -> Result<SourceSnapshot, String> {
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| source(conn, entry_id))
        .await
        .map_err(|e| e.to_string())?
}

pub fn translation_parameters(strategy: Option<&str>, concurrency: usize) -> String {
    serde_json::json!({"segmenter": TRANSLATION_SEGMENTER, "strategy": strategy.unwrap_or("default"), "concurrency": normalize_translation_concurrency(concurrency)}).to_string()
}

pub fn normalize_translation_concurrency(concurrency: usize) -> usize {
    if concurrency == 0 {
        3
    } else {
        concurrency.clamp(1, 5)
    }
}

#[derive(Clone)]
pub struct ResultSlot {
    entry_id: i64,
    task_type: String,
    language: String,
    parameters: String,
    source_hash: String,
    fence: CommitFence,
}

impl ResultSlot {
    pub fn new(
        entry_id: i64,
        task_type: &str,
        language: &str,
        parameters: &str,
        source_hash: &str,
    ) -> Self {
        Self {
            entry_id,
            task_type: task_type.into(),
            language: language.into(),
            parameters: crate::agent::configuration::fingerprint_parameters(parameters),
            source_hash: source_hash.into(),
            fence: current_commit_fence(),
        }
    }
    fn key(&self) -> String {
        serde_json::json!([
            self.entry_id,
            self.task_type,
            self.language,
            self.parameters,
            self.source_hash
        ])
        .to_string()
    }
    pub async fn load(&self, pool: &Pool, checkpoint: bool) -> Result<Option<AgentResult>, String> {
        let slot = self.clone();
        pool.get().await.map_err(|e|e.to_string())?.interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e|e.to_string())?;
            if source(&tx, slot.entry_id)?.hash != slot.source_hash { return Ok(None); }
            // Prefer an existing complete success over an incomplete regeneration.
            let json: Option<String> = tx.query_row("SELECT COALESCE(success_json, CASE WHEN ?2 THEN checkpoint_json END) FROM agent_result_slot WHERE slot_key=?1",params![slot.key(),checkpoint],|row|row.get(0)).optional().map_err(|e|e.to_string())?.flatten();
            json.map(|s| serde_json::from_str(&s).map_err(|e|e.to_string())).transpose()
        }).await.map_err(|e|e.to_string())?
    }
    pub async fn save(&self, pool: &Pool, result: &AgentResult) -> Result<(), String> {
        if result.source_hash != self.source_hash {
            return Err("Agent result source identity mismatch".into());
        }
        let slot = self.clone();
        let result = result.clone();
        pool.get().await.map_err(|e|e.to_string())?.interact(move |conn| {
            let tx = conn.transaction().map_err(|e|e.to_string())?;
            if source(&tx, slot.entry_id)?.hash != slot.source_hash { return Err("Article source changed during generation".into()); }
            let json = serde_json::to_string(&result).map_err(|e|e.to_string())?;
            tx.execute("INSERT INTO agent_result_slot(slot_key,entry_id,task_type,target_language,parameters,source_hash,success_json,checkpoint_json)
                VALUES(?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(slot_key) DO UPDATE SET
                success_json=COALESCE(excluded.success_json,success_json), checkpoint_json=excluded.checkpoint_json,updated_at=datetime('now')",
                params![slot.key(),slot.entry_id,slot.task_type,slot.language,slot.parameters,slot.source_hash,if result.complete {Some(&json)} else {None}, if result.complete {None} else {Some(&json)}]).map_err(|e|e.to_string())?;
            // No runtime mutex is acquired here. The fence lock spans only COMMIT.
            slot.fence.commit(tx)
        }).await.map_err(|e|e.to_string())?
    }
}

pub async fn load_summary(
    pool: &Pool,
    entry_id: i64,
    language: &str,
    detail: &str,
) -> Result<Option<AgentResult>, String> {
    let source = source_snapshot(pool, entry_id).await?;
    ResultSlot::new(entry_id, "summary", language, detail, &source.hash)
        .load(pool, false)
        .await
}

/// Historical successful summaries remain readable without a model/provider,
/// custom prompt or credentials. This does not assert current-config validity.
#[derive(Serialize)]
pub struct SummaryRecord {
    pub target_language: String,
    pub detail_level: String,
    pub configuration_fingerprint: Option<String>,
    pub result: AgentResult,
}

pub async fn latest_summary(pool: &Pool, entry_id: i64) -> Result<Option<SummaryRecord>, String> {
    pool.get().await.map_err(|e|e.to_string())?.interact(move |conn| {
        let hash = source(conn, entry_id)?.hash;
        let record: Option<(String,String,String)> = conn.query_row("SELECT target_language,parameters,success_json FROM agent_result_slot WHERE entry_id=?1 AND task_type='summary' AND source_hash=?2 AND success_json IS NOT NULL ORDER BY updated_at DESC,slot_key DESC LIMIT 1", params![entry_id,hash], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e|e.to_string())?;
        record.map(|(target_language,parameters,json)| {
            let identity = serde_json::from_str::<serde_json::Value>(&parameters).ok();
            let configuration_fingerprint = identity.as_ref().and_then(|p|p.get("configuration").and_then(|v|v.as_str()).map(str::to_owned));
            let detail_level = identity.as_ref().and_then(|p|p.get("parameters").and_then(|v|v.as_str()).map(str::to_owned)).unwrap_or(parameters);
            Ok(SummaryRecord { target_language, detail_level, configuration_fingerprint, result:serde_json::from_str(&json).map_err(|e|e.to_string())? })
        }).transpose()
    }).await.map_err(|e|e.to_string())?
}
pub async fn load_translation(
    pool: &Pool,
    entry_id: i64,
    language: &str,
    strategy: Option<&str>,
    concurrency: usize,
) -> Result<Option<AgentResult>, String> {
    let source = source_snapshot(pool, entry_id).await?;
    ResultSlot::new(
        entry_id,
        "translation",
        language,
        &translation_parameters(strategy, concurrency),
        &source.hash,
    )
    .load(pool, true)
    .await
}

#[cfg(test)]
#[path = "agent_result_store_tests.rs"]
mod tests;
