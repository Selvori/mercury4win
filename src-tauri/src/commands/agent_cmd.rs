// mercury4win-linux/src-tauri/commands/agent_cmd.rs
// AI agent control Tauri commands

use deadpool_sqlite::Pool;
use tauri::{ipc::Channel, State, AppHandle};
use crate::commands::feed_cmd::AppClient;
use crate::agent::{provider, credential_store};
use crate::agent::runtime::{SharedAgentRuntime, TaskType};
use crate::db::{agent_store, entry_store, content_store};
use crate::db::models::{
    AgentProviderProfile, AgentModelProfile, AgentProfile, ProviderConfig, AgentStatus,
};

// ── Provider management ──

#[tauri::command]
pub async fn list_providers(pool: State<'_, Pool>) -> Result<Vec<AgentProviderProfile>, String> {
    agent_store::list_providers(&pool).await
}

#[tauri::command]
pub async fn add_provider(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
    config: ProviderConfig,
) -> Result<AgentProviderProfile, String> {
    let id = uuid::Uuid::new_v4().to_string();
    log::info!("[add_provider] id={id}, name={}, base_url={}, has_key={}",
        config.name, config.base_url, config.api_key.is_some());
    if let Some(ref k) = config.api_key {
        let masked = if k.len() > 8 { format!("{}...{}", &k[..4], &k[k.len()-4..]) } else { "***".into() };
        log::info!("[add_provider] storing key: {masked} (len={})", k.len());
    }
    if let Some(ref api_key) = config.api_key {
        credential_store::store_api_key(&app_handle, &id, api_key).await?;
    }
    agent_store::insert_provider(&pool, &id, &config.name, &config.base_url, config.display_name.as_deref()).await?;
    Ok(AgentProviderProfile { id, name: config.name, base_url: config.base_url, display_name: config.display_name, is_default: false, is_enabled: true, is_archived: false, created_at: chrono::Utc::now().to_rfc3339() })
}

#[tauri::command]
pub async fn delete_provider(pool: State<'_, Pool>, app_handle: AppHandle, id: String) -> Result<(), String> {
    credential_store::delete_api_key(&app_handle, &id).await?;
    agent_store::delete_provider(&pool, &id).await
}

#[tauri::command]
pub async fn test_provider_connection(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    app_handle: AppHandle,
    provider_id: String,
    model_name: String,
) -> Result<String, String> {
    let providers = agent_store::list_providers(&pool).await?;
    let provider = providers.iter().find(|p| p.id == provider_id)
        .ok_or("Provider not found")?;
    let api_key = credential_store::get_api_key(&app_handle, &provider_id).await?.unwrap_or_default();
    log::info!("[test_provider_connection] provider_id={}, base_url={}, key_len={}, model={}",
        provider_id, provider.base_url, api_key.len(), model_name);
    provider::test_connection(&client.0, &provider.base_url, &api_key, &model_name).await
}

// ── Resolve model for agent ──
// Looks up the agent_profile.primary_model_id first; falls back to first enabled
// provider with a compatible model. Returns (base_url, api_key, model_name).
async fn resolve_agent_model(
    pool: &Pool,
    app_handle: &AppHandle,
    agent_type: &str,
    capability: impl Fn(&AgentModelProfile) -> bool,
    default_model: &str,
) -> Result<(String, String, String), String> {
    let agent_profile = agent_store::get_agent_profile(pool, agent_type).await?;
    let providers = agent_store::list_providers(pool).await?;

    // 1) Use the profile's primary_model_id if set
    if let Some(ref profile) = agent_profile {
        if let Some(ref model_id) = profile.primary_model_id {
            for provider in &providers {
                if !provider.is_enabled {
                    continue;
                }
                let models = agent_store::list_models(pool, &provider.id).await?;
                if let Some(model) = models.iter().find(|m| m.id == *model_id && m.is_enabled) {
                    let key = credential_store::get_api_key(app_handle, &provider.id)
                        .await?
                        .unwrap_or_default();
                    log::info!(
                        "[resolve_agent_model] agent={} using profile model: provider={}, model={}",
                        agent_type, provider.id, model.model_name
                    );
                    return Ok((provider.base_url.clone(), key, model.model_name.clone()));
                }
            }
            log::warn!(
                "[resolve_agent_model] agent={} primary_model_id={} not found, falling back",
                agent_type, model_id
            );
        }
    }

    // 2) Fallback: first enabled provider with a compatible model
    for provider in &providers {
        if !provider.is_enabled {
            continue;
        }
        let models = agent_store::list_models(pool, &provider.id).await?;
        if let Some(model) = models.iter().find(|m| m.is_enabled && capability(m)) {
            let key = credential_store::get_api_key(app_handle, &provider.id)
                .await?
                .unwrap_or_default();
            log::info!(
                "[resolve_agent_model] agent={} using fallback: provider={}, model={}",
                agent_type, provider.id, model.model_name
            );
            return Ok((provider.base_url.clone(), key, model.model_name.clone()));
        }
    }

    // 3) Last resort: first enabled provider + hardcoded model name
    if let Some(provider) = providers.iter().find(|p| p.is_enabled) {
        let key = credential_store::get_api_key(app_handle, &provider.id)
            .await?
            .unwrap_or_default();
        log::warn!(
            "[resolve_agent_model] agent={} no compatible model found, using default={}",
            agent_type, default_model
        );
        return Ok((provider.base_url.clone(), key, default_model.to_string()));
    }

    Err("No enabled AI provider configured. Add a provider in Settings.".into())
}

// ── Agent runtime commands ──

#[tauri::command]
pub async fn run_summary(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    runtime_state: State<'_, SharedAgentRuntime>,
    app_handle: AppHandle,
    on_event: Channel<String>,
    entry_id: i64,
    target_language: String,
    detail_level: String,
) -> Result<(), String> {
    // Submit to runtime
    {
        let mut rt = runtime_state.lock().await;
        rt.submit(entry_id, TaskType::Summary);
    }

    let (base_url, api_key, model_name) = resolve_agent_model(
        &pool, &app_handle, "summary",
        |m| m.supports_summary,
        "gpt-4o-mini",
    ).await?;

    // Run summary agent
    crate::agent::summary::run_summary(
        &pool, &client.0, &runtime_state,
        entry_id, &target_language, &detail_level,
        &base_url, &api_key, &model_name, on_event,
    ).await
}

#[tauri::command]
pub async fn run_translation(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    runtime_state: State<'_, SharedAgentRuntime>,
    app_handle: AppHandle,
    on_event: Channel<String>,
    entry_id: i64,
    target_language: String,
) -> Result<(), String> {
    {
        let mut rt = runtime_state.lock().await;
        rt.submit(entry_id, TaskType::Translation);
    }

    let agent_profile = agent_store::get_agent_profile(&pool, "translation").await?;
    let prompt_strategy = agent_profile.as_ref().and_then(|p| p.prompt_strategy.clone());
    let concurrency = agent_profile
        .as_ref()
        .and_then(|p| p.concurrency_degree)
        .map(|c| if c <= 0 { 3 } else { c as usize })
        .unwrap_or(3);

    let (base_url, api_key, model_name) = resolve_agent_model(
        &pool, &app_handle, "translation",
        |m| m.supports_translation,
        "gpt-4o-mini",
    ).await?;

    crate::agent::translation::run_translation(
        &pool, &client.0,
        entry_id, &target_language,
        &base_url, &api_key, &model_name,
        prompt_strategy.as_deref(), concurrency, on_event,
    ).await
}

#[tauri::command]
pub async fn run_tagging(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    runtime_state: State<'_, SharedAgentRuntime>,
    app_handle: AppHandle,
    entry_id: i64,
) -> Result<Vec<String>, String> {
    {
        let mut rt = runtime_state.lock().await;
        rt.submit(entry_id, TaskType::Tagging);
    }

    let (base_url, api_key, model_name) = resolve_agent_model(
        &pool, &app_handle, "tagging",
        |m| m.supports_tagging,
        "gpt-4o-mini",
    ).await?;

    crate::agent::tagging::run_tagging(
        &pool, &client.0, entry_id, &base_url, &api_key, &model_name,
    ).await.map_err(|e| {
        log::error!("[run_tagging] failed for entry_id={entry_id}: {e}");
        e
    })
}

#[tauri::command]
pub async fn cancel_agent_task(
    runtime_state: State<'_, SharedAgentRuntime>,
    task_type: String,
) -> Result<(), String> {
    let tt = match task_type.as_str() {
        "summary" => TaskType::Summary,
        "translation" => TaskType::Translation,
        "tagging" => TaskType::Tagging,
        _ => return Err("Unknown task type".into()),
    };
    let mut rt = runtime_state.lock().await;
    rt.complete(tt);
    Ok(())
}

// ── Custom prompt templates ──

#[tauri::command]
pub async fn save_custom_template(
    app_handle: AppHandle,
    agent_type: String,
    source_path: String,
) -> Result<(), String> {
    use crate::utils::paths;
    let prompts_dir = paths::prompts_dir(&app_handle);
    std::fs::create_dir_all(&prompts_dir).map_err(|e| format!("mkdir: {}", e))?;

    let dest = prompts_dir.join(format!("{}.custom.yaml", agent_type));
    std::fs::copy(&source_path, &dest).map_err(|e| format!("copy: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn load_custom_template(
    app_handle: AppHandle,
    agent_type: String,
) -> Result<Option<String>, String> {
    use crate::utils::paths;
    let prompts_dir = paths::prompts_dir(&app_handle);
    match crate::agent::prompt_templates::load_custom_template(&prompts_dir, &agent_type) {
        Ok(Some(template)) => {
            let yaml = serde_yaml::to_string(&template).map_err(|e| e.to_string())?;
            Ok(Some(yaml))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub async fn get_agent_status(
    runtime_state: State<'_, SharedAgentRuntime>,
    agent_type: String,
) -> Result<AgentStatus, String> {
    let tt = match agent_type.as_str() {
        "summary" => TaskType::Summary,
        "translation" => TaskType::Translation,
        "tagging" => TaskType::Tagging,
        _ => return Err("Unknown agent type".into()),
    };
    let rt = runtime_state.lock().await;
    let status = rt.get_status(tt);
    Ok(AgentStatus { agent_type: status.agent_type, state: status.state, current_entry_id: status.current_entry_id, queue_depth: status.queue_depth })
}

// ── Model management ──

#[tauri::command]
pub async fn list_models(
    pool: State<'_, Pool>,
    provider_id: String,
) -> Result<Vec<AgentModelProfile>, String> {
    agent_store::list_models(&pool, &provider_id).await
}

#[tauri::command]
pub async fn add_model(
    pool: State<'_, Pool>,
    provider_id: String,
    name: String,
    model_name: String,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();
    let pool = pool.inner().clone();
    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT INTO agent_model_profile (id, provider_id, name, model_name)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, provider_id, name, model_name],
            )
        }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_model(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    model_name: String,
    supports_summary: bool,
    supports_translation: bool,
    supports_tagging: bool,
) -> Result<(), String> {
    let pool = pool.inner().clone();
    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE agent_model_profile SET name = ?1, model_name = ?2,
                 supports_summary = ?3, supports_translation = ?4, supports_tagging = ?5
                 WHERE id = ?6",
                rusqlite::params![name, model_name, supports_summary, supports_translation, supports_tagging, id],
            )
        }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_model(
    pool: State<'_, Pool>,
    id: String,
) -> Result<(), String> {
    let pool = pool.inner().clone();
    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute("DELETE FROM agent_model_profile WHERE id = ?1", rusqlite::params![id])
        }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok(())
}

// ── Agent profile ──

#[tauri::command]
pub async fn get_agent_profile(
    pool: State<'_, Pool>,
    agent_type: String,
) -> Result<Option<AgentProfile>, String> {
    agent_store::get_agent_profile(&pool, &agent_type).await
}

#[tauri::command]
pub async fn update_agent_profile(
    pool: State<'_, Pool>,
    agent_type: String,
    primary_model_id: Option<String>,
    fallback_model_id: Option<String>,
    target_language: Option<String>,
    detail_level: Option<String>,
    prompt_strategy: Option<String>,
    concurrency_degree: Option<i32>,
) -> Result<(), String> {
    let pool = pool.inner().clone();
    pool.get().await.map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT INTO agent_profile (agent_type, primary_model_id, fallback_model_id,
                 target_language, detail_level, prompt_strategy, concurrency_degree)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(agent_type) DO UPDATE SET
                   primary_model_id = excluded.primary_model_id,
                   fallback_model_id = excluded.fallback_model_id,
                   target_language = excluded.target_language,
                   detail_level = excluded.detail_level,
                   prompt_strategy = excluded.prompt_strategy,
                   concurrency_degree = excluded.concurrency_degree",
                rusqlite::params![agent_type, primary_model_id, fallback_model_id, target_language, detail_level, prompt_strategy, concurrency_degree],
            )
        }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    Ok(())
}
