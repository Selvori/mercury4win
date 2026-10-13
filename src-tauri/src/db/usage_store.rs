//! Immutable request facts and history-inclusive reports.
use deadpool_sqlite::Pool;
use rusqlite::{params, types::Value};
use serde::{Deserialize, Serialize};
pub const MIGRATION_V5: &str = "
ALTER TABLE agent_provider_profile ADD COLUMN archived_at TEXT;
ALTER TABLE agent_model_profile ADD COLUMN archived_at TEXT;
ALTER TABLE llm_usage_event ADD COLUMN event_key TEXT;
ALTER TABLE llm_usage_event ADD COLUMN entry_id INTEGER;
ALTER TABLE llm_usage_event ADD COLUMN provider_id TEXT;
ALTER TABLE llm_usage_event ADD COLUMN model_id TEXT;
ALTER TABLE llm_usage_event ADD COLUMN endpoint TEXT;
ALTER TABLE llm_usage_event ADD COLUMN request_phase TEXT NOT NULL DEFAULT 'legacy';
ALTER TABLE llm_usage_event ADD COLUMN usage_availability TEXT NOT NULL DEFAULT 'legacy';
ALTER TABLE llm_usage_event ADD COLUMN started_at TEXT;
ALTER TABLE llm_usage_event ADD COLUMN finished_at TEXT;
CREATE UNIQUE INDEX idx_usage_event_key ON llm_usage_event(event_key);
CREATE INDEX idx_usage_created ON llm_usage_event(created_at);
CREATE INDEX idx_usage_provider ON llm_usage_event(provider_id, created_at);
CREATE INDEX idx_usage_model ON llm_usage_event(model_id, created_at);
CREATE INDEX idx_usage_agent ON llm_usage_event(agent_type, created_at);
";
#[derive(Clone)]
pub struct RequestEvent {
    pub event_key: String,
    pub entry_id: i64,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub provider_name: Option<String>,
    pub model_name: String,
    pub agent_type: String,
    pub endpoint: String,
    pub request_phase: String,
    pub request_status: String,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub started_at: String,
    pub finished_at: Option<String>,
}
/// Retain origin/path; never persist URL credentials/query/fragment.
pub fn sanitized_endpoint(base: &str) -> String {
    match url::Url::parse(&format!("{}/chat/completions", base.trim_end_matches('/'))) {
        Ok(mut url) => {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_query(None);
            url.set_fragment(None);
            url.into()
        }
        Err(_) => "invalid endpoint".into(),
    }
}
pub async fn insert_request_event(pool: &Pool, event: &RequestEvent) -> Result<(), String> {
    let e = event.clone();
    let availability =
        if e.prompt_tokens.is_some() && e.completion_tokens.is_some() && e.total_tokens.is_some() {
            "actual"
        } else {
            "missing"
        };
    pool.get().await.map_err(|e| e.to_string())?.interact(move |conn| {
        conn.execute("INSERT INTO llm_usage_event(event_key,entry_id,provider_id,model_id,provider_name,model_name,agent_type,endpoint,request_phase,request_status,prompt_tokens,completion_tokens,total_tokens,usage_availability,started_at,finished_at,created_at)
        VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?15) ON CONFLICT(event_key) DO NOTHING",
        params![e.event_key,e.entry_id,e.provider_id,e.model_id,e.provider_name,e.model_name,e.agent_type,e.endpoint,e.request_phase,e.request_status,e.prompt_tokens,e.completion_tokens,e.total_tokens,availability,e.started_at,e.finished_at]).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?
}
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct UsageFilter {
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub agent_type: Option<String>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UsageMetrics {
    pub request_count: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub timed_out: i64,
    pub missing_usage: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub legacy_events: i64,
    pub legacy_tokens: i64,
}
#[derive(Debug, Serialize)]
pub struct UsageGroup {
    pub id: String,
    pub label: String,
    pub archived: bool,
    pub metrics: UsageMetrics,
}
#[derive(Debug, Serialize)]
pub struct UsageReport {
    pub from_utc: String,
    pub to_utc: String,
    pub totals: UsageMetrics,
    pub groups: Vec<UsageGroup>,
    pub daily: Vec<UsageGroup>,
    pub persistence_failures: u64,
}
const METRICS: &str = "
SUM(CASE WHEN e.event_key IS NOT NULL THEN 1 ELSE 0 END),
SUM(CASE WHEN e.event_key IS NOT NULL AND e.request_status='succeeded' THEN 1 ELSE 0 END),
SUM(CASE WHEN e.event_key IS NOT NULL AND e.request_status='failed' THEN 1 ELSE 0 END),
SUM(CASE WHEN e.event_key IS NOT NULL AND e.request_status='cancelled' THEN 1 ELSE 0 END),
SUM(CASE WHEN e.event_key IS NOT NULL AND e.request_status='timedOut' THEN 1 ELSE 0 END),
SUM(CASE WHEN e.event_key IS NOT NULL AND e.usage_availability='missing' THEN 1 ELSE 0 END),
COALESCE(SUM(CASE WHEN e.event_key IS NOT NULL THEN e.prompt_tokens END),0),
COALESCE(SUM(CASE WHEN e.event_key IS NOT NULL THEN e.completion_tokens END),0),
COALESCE(SUM(CASE WHEN e.event_key IS NOT NULL THEN e.total_tokens END),0),
SUM(CASE WHEN e.event_key IS NULL THEN 1 ELSE 0 END),
COALESCE(SUM(CASE WHEN e.event_key IS NULL THEN e.total_tokens END),0)";
fn metrics(row: &rusqlite::Row<'_>, start: usize) -> rusqlite::Result<UsageMetrics> {
    let n = |i| row.get::<_, Option<i64>>(start + i).map(|v| v.unwrap_or(0));
    Ok(UsageMetrics {
        request_count: n(0)?,
        succeeded: n(1)?,
        failed: n(2)?,
        cancelled: n(3)?,
        timed_out: n(4)?,
        missing_usage: n(5)?,
        prompt_tokens: n(6)?,
        completion_tokens: n(7)?,
        total_tokens: n(8)?,
        legacy_events: n(9)?,
        legacy_tokens: n(10)?,
    })
}
pub async fn report(
    pool: &Pool,
    window: &str,
    dimension: &str,
    filter: UsageFilter,
) -> Result<UsageReport, String> {
    let days = match window {
        "7d" => 7,
        "14d" => 14,
        "30d" => 30,
        "90d" => 90,
        _ => return Err("Unknown usage window".into()),
    };
    let end = chrono::Utc::now();
    report_range(
        pool,
        &(end - chrono::Duration::days(days)).to_rfc3339(),
        &end.to_rfc3339(),
        dimension,
        filter,
    )
    .await
}
pub async fn report_range(
    pool: &Pool,
    from: &str,
    to: &str,
    dimension: &str,
    filter: UsageFilter,
) -> Result<UsageReport, String> {
    let (id,label,archived) = match dimension {
        "provider" => ("COALESCE(e.provider_id,'snapshot:'||COALESCE(e.provider_name,'unknown'))", "COALESCE(p.display_name,p.name,e.provider_name,'unknown')", "COALESCE(p.is_archived,0)"),
        "model" => ("COALESCE(e.model_id,'snapshot:'||COALESCE(e.provider_name,'unknown')||':'||COALESCE(e.model_name,'unknown'))", "COALESCE(p.display_name,p.name,e.provider_name,'unknown')||' / '||COALESCE(m.name,e.model_name,'unknown')", "MAX(COALESCE(m.is_archived,0),COALESCE(p.is_archived,0))"),
        "agent" => ("COALESCE(e.agent_type,'unknown')", "COALESCE(e.agent_type,'unknown')", "0"),
        _ => return Err("Unknown usage dimension".into()),
    };
    let from = from.to_owned();
    let to = to.to_owned();
    let mut where_sql = "e.created_at >= date(?1,'-1 day') AND e.created_at < date(?2,'+1 day') AND julianday(e.created_at)>=julianday(?1) AND julianday(e.created_at)<julianday(?2)".to_string();
    let mut values = vec![Value::Text(from.clone()), Value::Text(to.clone())];
    for (column, value) in [
        ("provider_id", filter.provider_id),
        ("model_id", filter.model_id),
        ("agent_type", filter.agent_type),
    ] {
        if let Some(value) = value {
            values.push(Value::Text(value));
            where_sql.push_str(&format!(" AND e.{column}=?{}", values.len()));
        }
    }
    let joins = "FROM llm_usage_event e LEFT JOIN agent_provider_profile p ON p.id=e.provider_id LEFT JOIN agent_model_profile m ON m.id=e.model_id";
    let totals_sql = format!("SELECT {METRICS} {joins} WHERE {where_sql}");
    let groups_sql=format!("SELECT {id}, {label}, {archived}, {METRICS} {joins} WHERE {where_sql} GROUP BY 1 ORDER BY 4 DESC, 1");
    let daily_sql=format!("SELECT date(e.created_at),date(e.created_at),0,{METRICS} {joins} WHERE {where_sql} GROUP BY 1 ORDER BY 1");
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            let totals = tx
                .query_row(&totals_sql, rusqlite::params_from_iter(&values), |r| {
                    metrics(r, 0)
                })
                .map_err(|e| e.to_string())?;
            let rows = |sql: &str| -> Result<Vec<UsageGroup>, String> {
                let mut stmt = tx.prepare(sql).map_err(|e| e.to_string())?;
                let result = stmt
                    .query_map(rusqlite::params_from_iter(&values), |r| {
                        Ok(UsageGroup {
                            id: r.get(0)?,
                            label: r.get(1)?,
                            archived: r.get::<_, i64>(2)? != 0,
                            metrics: metrics(r, 3)?,
                        })
                    })
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string());
                result
            };
            let groups = rows(&groups_sql)?;
            let daily = rows(&daily_sql)?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(UsageReport {
                from_utc: from,
                to_utc: to,
                totals,
                groups,
                daily,
                persistence_failures: crate::agent::usage::WRITE_FAILURES
                    .load(std::sync::atomic::Ordering::Relaxed),
            })
        })
        .await
        .map_err(|e| e.to_string())?
}
