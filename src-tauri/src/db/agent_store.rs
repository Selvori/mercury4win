// mercury4win-linux/src-tauri/db/agent_store.rs
// Agent provider/model/profile persistence

use deadpool_sqlite::Pool;
use rusqlite::params;
use crate::db::models::{AgentProviderProfile, AgentModelProfile, AgentProfile};

// ── Provider CRUD ──

pub async fn list_providers(pool: &Pool) -> Result<Vec<AgentProviderProfile>, String> {
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, base_url, display_name, is_default, is_enabled, is_archived, created_at
                 FROM agent_provider_profile WHERE is_archived = 0 ORDER BY name"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |row| {
                Ok(AgentProviderProfile {
                    id: row.get(0)?, name: row.get(1)?, base_url: row.get(2)?,
                    display_name: row.get(3)?, is_default: row.get::<_, i32>(4)? != 0,
                    is_enabled: row.get::<_, i32>(5)? != 0, is_archived: row.get::<_, i32>(6)? != 0,
                    created_at: row.get(7)?,
                })
            }).map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
}

pub async fn insert_provider(pool: &Pool, id: &str, name: &str, base_url: &str, display_name: Option<&str>) -> Result<(), String> {
    let id = id.to_string();
    let name = name.to_string();
    let base_url = base_url.to_string();
    let display = display_name.map(|s| s.to_string());
    let pool = pool.clone();
    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT INTO agent_provider_profile (id, name, base_url, display_name) VALUES (?1,?2,?3,?4)",
                params![id, name, base_url, display],
            )
        }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_provider(pool: &Pool, id: &str) -> Result<(), String> {
    let id = id.to_string();
    let pool = pool.clone();
    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| conn.execute("DELETE FROM agent_provider_profile WHERE id = ?1", params![id]))
        .await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok(())
}

// ── Model CRUD ──

pub async fn list_models(pool: &Pool, provider_id: &str) -> Result<Vec<AgentModelProfile>, String> {
    let pid = provider_id.to_string();
    let pool = pool.clone();
    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, provider_id, name, model_name, temperature, top_p, max_tokens,
                        is_streaming, supports_summary, supports_translation, supports_tagging,
                        is_default, is_enabled, is_archived, last_tested_at
                 FROM agent_model_profile WHERE provider_id = ?1 AND is_archived = 0 ORDER BY name"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![pid], |row| {
                Ok(AgentModelProfile {
                    id: row.get(0)?, provider_id: row.get(1)?, name: row.get(2)?,
                    model_name: row.get(3)?, temperature: row.get(4)?, top_p: row.get(5)?,
                    max_tokens: row.get(6)?, is_streaming: row.get::<_, i32>(7)? != 0,
                    supports_summary: row.get::<_, i32>(8)? != 0,
                    supports_translation: row.get::<_, i32>(9)? != 0,
                    supports_tagging: row.get::<_, i32>(10)? != 0,
                    is_default: row.get::<_, i32>(11)? != 0,
                    is_enabled: row.get::<_, i32>(12)? != 0,
                    is_archived: row.get::<_, i32>(13)? != 0,
                    last_tested_at: row.get(14)?,
                })
            }).map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
        }).await.map_err(|e| e.to_string())?
}

// ── Agent Profile ──

pub async fn get_agent_profile(pool: &Pool, agent_type: &str) -> Result<Option<AgentProfile>, String> {
    let at = agent_type.to_string();
    let pool = pool.clone();
    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            let result = conn.query_row(
                "SELECT agent_type, primary_model_id, fallback_model_id, target_language,
                        detail_level, prompt_strategy, concurrency_degree
                 FROM agent_profile WHERE agent_type = ?1",
                params![at],
                |row| Ok(AgentProfile {
                    agent_type: row.get(0)?, primary_model_id: row.get(1)?,
                    fallback_model_id: row.get(2)?, target_language: row.get(3)?,
                    detail_level: row.get(4)?, prompt_strategy: row.get(5)?,
                    concurrency_degree: row.get(6)?,
                }),
            );
            match result {
                Ok(p) => Ok(Some(p)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        }).await.map_err(|e| e.to_string())?
}
