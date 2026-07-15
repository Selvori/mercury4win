// mercury4win-linux/src-tauri/db/usage_store.rs
// LLM usage event persistence

use deadpool_sqlite::Pool;
use rusqlite::params;
use crate::db::models::LlmUsageEvent;

pub async fn insert_usage_event(pool: &Pool, event: &LlmUsageEvent) -> Result<(), String> {
    let task_run_id = event.task_run_id;
    let provider = event.provider_name.clone();
    let model = event.model_name.clone();
    let agent_type = event.agent_type.clone();
    let prompt_tokens = event.prompt_tokens;
    let completion_tokens = event.completion_tokens;
    let total_tokens = event.total_tokens;
    let status = event.request_status.clone();
    let pool = pool.clone();

    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT INTO llm_usage_event (task_run_id, provider_name, model_name, agent_type,
                 prompt_tokens, completion_tokens, total_tokens, request_status)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![task_run_id, provider, model, agent_type, prompt_tokens, completion_tokens, total_tokens, status],
            )
        }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn get_usage_totals(pool: &Pool, window: &str) -> Result<(i64, i64, f64), String> {
    let window = window.to_string(); // "7d", "30d", "90d"
    let pool = pool.clone();

    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            let days: i64 = match window.as_str() {
                "7d" => 7, "30d" => 30, "90d" => 90, _ => 30,
            };
            conn.query_row(
                "SELECT COALESCE(SUM(prompt_tokens),0), COALESCE(SUM(completion_tokens),0), COUNT(*)
                 FROM llm_usage_event
                 WHERE created_at >= datetime('now', ?1)",
                params![format!("-{} days", days)],
                |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, f64>(2)?)),
            )
            .map_err(|e| e.to_string())
        }).await.map_err(|e| e.to_string())?
}
