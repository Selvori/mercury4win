// mercury4win/src-tauri/db/agent_store.rs
// Agent provider/model/profile persistence

use crate::db::models::{AgentModelProfile, AgentProfile, AgentProviderProfile};
use deadpool_sqlite::Pool;
use rusqlite::params;

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

pub async fn insert_provider(
    pool: &Pool,
    id: &str,
    name: &str,
    base_url: &str,
    display_name: Option<&str>,
) -> Result<(), String> {
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
    pool.get().await.map_err(|e| e.to_string())?.interact(move |conn| {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        if tx.execute("UPDATE agent_provider_profile SET is_archived=1,archived_at=datetime('now'),is_default=0 WHERE id=?1 AND is_archived=0", params![id]).map_err(|e|e.to_string())? == 0 {
            return Err("Provider not found".to_string());
        }
        tx.execute("UPDATE agent_model_profile SET is_archived=1,archived_at=datetime('now'),is_default=0 WHERE provider_id=?1 AND is_archived=0", params![id]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())
    }).await.map_err(|e|e.to_string())??;
    Ok(())
}

pub async fn update_provider(
    pool: &Pool,
    id: &str,
    name: &str,
    base_url: &str,
    display_name: Option<&str>,
) -> Result<(), String> {
    let id = id.to_string();
    let name = name.to_string();
    let base_url = base_url.to_string();
    let display = display_name.map(|s| s.to_string());
    let pool = pool.clone();
    let changed = pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE agent_provider_profile SET name = ?2, base_url = ?3, display_name = ?4 WHERE id = ?1 AND is_archived=0",
                params![id, name, base_url, display],
            )
        }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Provider not found".into());
    }
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
                 FROM agent_model_profile WHERE provider_id = ?1 AND is_archived = 0 AND EXISTS(SELECT 1 FROM agent_provider_profile p WHERE p.id=provider_id AND p.is_archived=0) ORDER BY name"
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

pub async fn get_agent_profile(
    pool: &Pool,
    agent_type: &str,
) -> Result<Option<AgentProfile>, String> {
    let at = agent_type.to_string();
    let pool = pool.clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let result = conn.query_row(
                "SELECT agent_type, primary_model_id, fallback_model_id, target_language,
                        detail_level, prompt_strategy, concurrency_degree
                 FROM agent_profile WHERE agent_type = ?1",
                params![at],
                |row| {
                    Ok(AgentProfile {
                        agent_type: row.get(0)?,
                        primary_model_id: row.get(1)?,
                        fallback_model_id: row.get(2)?,
                        target_language: row.get(3)?,
                        detail_level: row.get(4)?,
                        prompt_strategy: row.get(5)?,
                        concurrency_degree: row.get(6)?,
                    })
                },
            );
            match result {
                Ok(p) => Ok(Some(p)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        })
        .await
        .map_err(|e| e.to_string())?
}

/// Same endpoint and display identity can be reactivated. A different name
/// creates a new identity so it cannot implicitly inherit an archived account.
pub async fn create_provider(
    pool: &Pool,
    name: &str,
    base_url: &str,
    display: Option<&str>,
) -> Result<String, String> {
    let name = name.trim().to_owned();
    let base_url = normalize_base_url(base_url)?;
    let display = display.map(str::to_owned);
    pool.get().await.map_err(|e|e.to_string())?.interact(move |conn| {
        let tx=conn.unchecked_transaction().map_err(|e|e.to_string())?;
        let existing = {
            let mut stmt=tx.prepare("SELECT id,base_url FROM agent_provider_profile WHERE is_archived=1 AND name=?1 ORDER BY archived_at DESC,id").map_err(|e|e.to_string())?;
            let candidates=stmt.query_map(params![name],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
            candidates.into_iter().find(|(_,url)|normalize_base_url(url).is_ok_and(|u|u==base_url)).map(|(id,_)|id)
        };
        let id=existing.unwrap_or_else(||uuid::Uuid::new_v4().to_string());
        tx.execute("INSERT INTO agent_provider_profile(id,name,base_url,display_name) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET name=excluded.name,base_url=excluded.base_url,display_name=excluded.display_name,is_archived=0,archived_at=NULL,is_enabled=1",params![id,name,base_url,display]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())?; Ok(id)
    }).await.map_err(|e|e.to_string())?
}
pub fn normalize_base_url(value: &str) -> Result<String, String> {
    let url = url::Url::parse(value.trim()).map_err(|_| "Invalid base URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Base URL must be http(s) without credentials, query or fragment".into());
    }
    Ok(url.as_str().trim_end_matches('/').into())
}
pub async fn create_model(
    pool: &Pool,
    provider_id: &str,
    name: &str,
    model_name: &str,
    capabilities: [Option<bool>; 3],
) -> Result<(), String> {
    use rusqlite::OptionalExtension;
    let pid = provider_id.to_owned();
    let name = name.trim().to_owned();
    let model = model_name.trim().to_owned();
    if name.is_empty() || model.is_empty() {
        return Err("Model name is required".into());
    }
    pool.get().await.map_err(|e|e.to_string())?.interact(move |conn| {
        let tx=conn.unchecked_transaction().map_err(|e|e.to_string())?;
        let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM agent_provider_profile WHERE id=?1 AND is_archived=0)",params![pid],|r|r.get(0)).map_err(|e|e.to_string())?;
        if !active { return Err("Provider not found".into()); }
        let id:Option<String>=tx.query_row("SELECT id FROM agent_model_profile WHERE provider_id=?1 AND model_name=?2 AND is_archived=1 ORDER BY archived_at DESC,id LIMIT 1",params![pid,model],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
        tx.execute("INSERT INTO agent_model_profile(id,provider_id,name,model_name,supports_summary,supports_translation,supports_tagging) VALUES(?1,?2,?3,?4,COALESCE(?5,1),COALESCE(?6,1),COALESCE(?7,1)) ON CONFLICT(id) DO UPDATE SET name=excluded.name,is_archived=0,archived_at=NULL,is_enabled=1,supports_summary=COALESCE(?5,agent_model_profile.supports_summary),supports_translation=COALESCE(?6,agent_model_profile.supports_translation),supports_tagging=COALESCE(?7,agent_model_profile.supports_tagging)",params![id.unwrap_or_else(||uuid::Uuid::new_v4().to_string()),pid,name,model,capabilities[0],capabilities[1],capabilities[2]]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e|e.to_string())
    }).await.map_err(|e|e.to_string())?
}
pub async fn archive_model(pool: &Pool, id: &str) -> Result<(), String> {
    let id = id.to_owned();
    pool.get().await.map_err(|e|e.to_string())?.interact(move |c| {
        if c.execute("UPDATE agent_model_profile SET is_archived=1,archived_at=datetime('now'),is_default=0 WHERE id=?1 AND is_archived=0",params![id]).map_err(|e|e.to_string())?==0 { return Err("Model not found".into()); } Ok(())
    }).await.map_err(|e|e.to_string())?
}
pub async fn set_enabled(pool: &Pool, id: &str, entity: &str, enabled: bool) -> Result<(), String> {
    let table = match entity {
        "provider" => "agent_provider_profile",
        "model" => "agent_model_profile",
        _ => return Err("Unknown entity".into()),
    };
    let id = id.to_owned();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |c| {
            if c.execute(
                &format!("UPDATE {table} SET is_enabled=?2 WHERE id=?1 AND is_archived=0"),
                params![id, enabled],
            )
            .map_err(|e| e.to_string())?
                == 0
            {
                return Err("Configuration not found".into());
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
}
