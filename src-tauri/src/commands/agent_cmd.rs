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
    provider::test_connection(&client.0, &provider.base_url, &api_key, &model_name).await
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

    // Look up agent config
    let agent_profile = agent_store::get_agent_profile(&pool, "summary").await?;
    let model_id = agent_profile.as_ref().and_then(|p| p.primary_model_id.clone()).unwrap_or_default();
    let providers = agent_store::list_providers(&pool).await?;

    // Find active provider with models
    let (base_url, api_key, model_name) = if let Some(provider) = providers.iter().find(|p| p.is_enabled) {
        let models = agent_store::list_models(&pool, &provider.id).await?;
        let model = models.iter().find(|m| m.is_enabled && m.supports_summary);
        let key = credential_store::get_api_key(&app_handle, &provider.id).await?.unwrap_or_default();
        let mn = model.map(|m| m.model_name.clone()).unwrap_or_else(|| "gpt-4o-mini".to_string());
        (provider.base_url.clone(), key, mn)
    } else {
        return Err("No enabled AI provider configured. Add a provider in Settings.".into());
    };

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
    let providers = agent_store::list_providers(&pool).await?;
    let prompt_strategy = agent_profile.as_ref().and_then(|p| p.prompt_strategy.clone());

    let (base_url, api_key, model_name) = if let Some(provider) = providers.iter().find(|p| p.is_enabled) {
        let models = agent_store::list_models(&pool, &provider.id).await?;
        let model = models.iter().find(|m| m.is_enabled && m.supports_translation);
        let key = credential_store::get_api_key(&app_handle, &provider.id).await?.unwrap_or_default();
        let mn = model.map(|m| m.model_name.clone()).unwrap_or_else(|| "gpt-4o-mini".to_string());
        (provider.base_url.clone(), key, mn)
    } else {
        return Err("No enabled AI provider configured. Add a provider in Settings.".into());
    };

    crate::agent::translation::run_translation(
        &pool, &client.0,
        entry_id, &target_language,
        &base_url, &api_key, &model_name,
        prompt_strategy.as_deref(), on_event,
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

    let providers = agent_store::list_providers(&pool).await?;
    let (base_url, api_key, model_name) = if let Some(provider) = providers.iter().find(|p| p.is_enabled) {
        let models = agent_store::list_models(&pool, &provider.id).await?;
        let model = models.iter().find(|m| m.is_enabled && m.supports_tagging);
        let key = credential_store::get_api_key(&app_handle, &provider.id).await?.unwrap_or_default();
        let mn = model.map(|m| m.model_name.clone()).unwrap_or_else(|| "gpt-4o-mini".to_string());
        (provider.base_url.clone(), key, mn)
    } else {
        return Err("No enabled AI provider configured. Add a provider in Settings.".into());
    };

    crate::agent::tagging::run_tagging(
        &pool, &client.0, entry_id, &base_url, &api_key, &model_name,
    ).await
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
