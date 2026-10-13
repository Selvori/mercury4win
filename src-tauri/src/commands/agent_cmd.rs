// mercury4win/src-tauri/commands/agent_cmd.rs
// AI agent control Tauri commands

use crate::agent::runtime::{self, SharedAgentRuntime, TaskType};
use crate::agent::{credential_store, provider};
use crate::commands::feed_cmd::AppClient;
use crate::db::agent_store;
use crate::db::models::{
    AgentModelProfile, AgentProfile, AgentProviderProfile, AgentStatus, ProviderConfig,
};
use deadpool_sqlite::Pool;
use tauri::{ipc::Channel, AppHandle, State};

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
    validate_provider_config(&config)?;
    let id = agent_store::create_provider(
        &pool,
        &config.name,
        &config.base_url,
        config.display_name.as_deref(),
    )
    .await?;
    // Archive never removes a key. A failed DB operation cannot destroy it.
    if let Some(ref api_key) = config.api_key {
        credential_store::store_api_key(&app_handle, &id, api_key).await?;
    }
    agent_store::list_providers(&pool)
        .await?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| "Provider not found".into())
}

#[tauri::command]
pub async fn delete_provider(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    agent_store::delete_provider(&pool, &id).await
}

#[tauri::command]
pub async fn update_provider(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
    id: String,
    config: ProviderConfig,
) -> Result<(), String> {
    validate_provider_config(&config)?;
    agent_store::update_provider(
        &pool,
        &id,
        &config.name,
        &agent_store::normalize_base_url(&config.base_url)?,
        config.display_name.as_deref(),
    )
    .await?;
    if let Some(ref api_key) = config.api_key {
        credential_store::store_api_key(&app_handle, &id, api_key).await?;
    }
    Ok(())
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
    let provider = providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or("Provider not found")?;
    let api_key = credential_store::get_api_key(&app_handle, &provider_id)
        .await?
        .unwrap_or_default();
    provider::test_connection(&client.0, &provider.base_url, &api_key, &model_name).await
}

/// Test the values currently shown in the Settings form. When editing an
/// existing provider, an omitted API key intentionally reuses its saved key.
#[tauri::command]
pub async fn test_provider_config(
    client: State<'_, AppClient>,
    app_handle: AppHandle,
    provider_id: Option<String>,
    config: ProviderConfig,
    model_name: String,
) -> Result<String, String> {
    validate_provider_config(&config)?;
    let model_name = model_name.trim();
    if model_name.is_empty() {
        return Err("Model name is required".into());
    }

    let api_key = match config.api_key.as_deref() {
        Some(key) if !key.is_empty() => key.to_string(),
        _ => match provider_id.as_deref() {
            Some(id) => credential_store::get_api_key(&app_handle, id)
                .await?
                .unwrap_or_default(),
            None => String::new(),
        },
    };

    provider::test_connection(&client.0, &config.base_url, &api_key, model_name).await
}

fn validate_provider_config(config: &ProviderConfig) -> Result<(), String> {
    if config.name.trim().is_empty() {
        return Err("Provider name is required".into());
    }
    let url = config.base_url.trim();
    if url.is_empty() {
        return Err("Base URL is required".into());
    }
    agent_store::normalize_base_url(url)?;
    Ok(())
}

// Route checks never load credentials. Resolution errors are configuration
// failures and do not cause requests to an arbitrary model.
#[cfg(test)]
pub(crate) async fn resolve_agent_route(
    pool: &Pool,
    agent_type: &str,
    _capability: impl Fn(&AgentModelProfile) -> bool,
    _default_model: &str,
) -> Result<(String, String, String), String> {
    let config =
        crate::agent::configuration::load(pool, std::path::Path::new(""), agent_type).await?;
    let route = config.routes.first().ok_or_else(|| format!("No enabled compatible model configured for {agent_type}. Configure a model in Settings."))?;
    Ok((
        route.base_url.clone(),
        route.provider_id.clone(),
        route.model_name.clone(),
    ))
}

pub(crate) async fn execution_configuration(
    pool: &Pool,
    app: &AppHandle,
    agent: &str,
) -> Result<crate::agent::configuration::Configuration, String> {
    let mut config =
        crate::agent::configuration::load(pool, &crate::utils::paths::prompts_dir(app), agent)
            .await?;
    if config.routes.is_empty() {
        return Err(format!(
            "No enabled compatible model configured for {agent}. Configure a model in Settings."
        ));
    }
    for route in &mut config.routes {
        route.api_key = credential_store::get_api_key(app, &route.provider_id)
            .await?
            .unwrap_or_default();
    }
    Ok(config)
}

#[tauri::command]
pub async fn get_agent_fingerprint(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
    agent_type: String,
) -> Result<String, String> {
    Ok(crate::agent::configuration::load(
        &pool,
        &crate::utils::paths::prompts_dir(&app_handle),
        &agent_type,
    )
    .await?
    .fingerprint)
}

#[tauri::command]
pub async fn load_latest_summary(
    pool: State<'_, Pool>,
    entry_id: i64,
) -> Result<Option<crate::db::agent_result_store::SummaryRecord>, String> {
    crate::db::agent_result_store::latest_summary(&pool, entry_id).await
}

// ── Agent runtime commands ──

#[tauri::command]
pub async fn load_summary_result(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
    entry_id: i64,
    target_language: String,
    detail_level: String,
) -> Result<Option<crate::db::agent_result_store::AgentResult>, String> {
    let config = crate::agent::configuration::load(
        &pool,
        &crate::utils::paths::prompts_dir(&app_handle),
        "summary",
    )
    .await?;
    crate::agent::configuration::CONFIGURATION
        .scope(config, async {
            crate::db::agent_result_store::load_summary(
                &pool,
                entry_id,
                &target_language,
                &detail_level,
            )
            .await
        })
        .await
}

#[tauri::command]
pub async fn load_translation_result(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
    entry_id: i64,
    target_language: String,
) -> Result<Option<crate::db::agent_result_store::AgentResult>, String> {
    let config = crate::agent::configuration::load(
        &pool,
        &crate::utils::paths::prompts_dir(&app_handle),
        "translation",
    )
    .await?;
    let strategy = config.prompt_strategy.clone();
    let concurrency = config.concurrency;
    crate::agent::configuration::CONFIGURATION
        .scope(config, async {
            crate::db::agent_result_store::load_translation(
                &pool,
                entry_id,
                &target_language,
                strategy.as_deref(),
                concurrency,
            )
            .await
        })
        .await
}

#[tauri::command]
pub async fn load_translation_document(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
    entry_id: i64,
    target_language: String,
) -> Result<crate::agent::translation_segments::TranslationDocument, String> {
    let config = crate::agent::configuration::load(
        &pool,
        &crate::utils::paths::prompts_dir(&app_handle),
        "translation",
    )
    .await?;
    let strategy = config.prompt_strategy.clone();
    let concurrency = config.concurrency;
    crate::agent::configuration::CONFIGURATION
        .scope(config, async {
            crate::agent::translation_segments::load_document(
                &pool,
                entry_id,
                &target_language,
                strategy.as_deref(),
                concurrency,
            )
            .await
        })
        .await
}

#[tauri::command]
pub async fn run_summary(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    runtime_state: State<'_, SharedAgentRuntime>,
    app_handle: AppHandle,
    on_event: Channel<String>,
    entry_id: i64,
    request_id: String,
    target_language: String,
    detail_level: String,
) -> Result<(), String> {
    runtime::execute(
        &runtime_state,
        TaskType::Summary,
        entry_id,
        &request_id,
        &on_event,
        || async {
            let config = execution_configuration(&pool, &app_handle, "summary").await?;
            let route = config.routes[0].clone();
            if let Some(notice) = &config.notice {
                let _ = on_event
                    .send(serde_json::json!({"type":"notice","message":notice}).to_string());
            }
            crate::agent::configuration::CONFIGURATION
                .scope(config, async {
                    crate::agent::summary::run_summary(
                        &pool,
                        &client.0,
                        &runtime_state,
                        entry_id,
                        &target_language,
                        &detail_level,
                        &route.base_url,
                        &route.api_key,
                        &route.model_name,
                        on_event.clone(),
                    )
                    .await
                })
                .await
        },
    )
    .await
}

#[tauri::command]
pub async fn run_translation(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    runtime_state: State<'_, SharedAgentRuntime>,
    app_handle: AppHandle,
    on_event: Channel<String>,
    entry_id: i64,
    request_id: String,
    target_language: String,
    source_segment_id: Option<String>,
) -> Result<(), String> {
    runtime::execute(
        &runtime_state,
        TaskType::Translation,
        entry_id,
        &request_id,
        &on_event,
        || async {
            let config = execution_configuration(&pool, &app_handle, "translation").await?;
            let route = config.routes[0].clone();
            let strategy = config.prompt_strategy.clone();
            let concurrency = config.concurrency;
            if let Some(notice) = &config.notice {
                let _ = on_event
                    .send(serde_json::json!({"type":"notice","message":notice}).to_string());
            }
            crate::agent::configuration::CONFIGURATION
                .scope(config, async {
                    crate::agent::translation::run_translation_segments(
                        &pool,
                        &client.0,
                        entry_id,
                        &target_language,
                        &route.base_url,
                        &route.api_key,
                        &route.model_name,
                        strategy.as_deref(),
                        concurrency,
                        source_segment_id.as_deref(),
                        on_event.clone(),
                    )
                    .await
                })
                .await
        },
    )
    .await
}

#[tauri::command]
pub async fn run_tagging(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    runtime_state: State<'_, SharedAgentRuntime>,
    app_handle: AppHandle,
    on_event: Channel<String>,
    entry_id: i64,
    request_id: String,
) -> Result<Vec<String>, String> {
    runtime::execute(
        &runtime_state,
        TaskType::Tagging,
        entry_id,
        &request_id,
        &on_event,
        || async {
            let config = execution_configuration(&pool, &app_handle, "tagging").await?;
            let route = config.routes[0].clone();
            if let Some(notice) = &config.notice {
                let _ = on_event
                    .send(serde_json::json!({"type":"notice","message":notice}).to_string());
            }
            crate::agent::configuration::CONFIGURATION
                .scope(config, async {
                    crate::agent::tagging::run_tagging(
                        &pool,
                        &client.0,
                        entry_id,
                        &route.base_url,
                        &route.api_key,
                        &route.model_name,
                    )
                    .await
                })
                .await
        },
    )
    .await
}

#[tauri::command]
pub async fn cancel_agent_task(
    runtime_state: State<'_, SharedAgentRuntime>,
    task_type: String,
    request_id: String,
) -> Result<(), String> {
    runtime_state
        .lock()
        .unwrap()
        .cancel(TaskType::parse(&task_type)?, &request_id, false)
}

#[tauri::command]
pub async fn abandon_agent_waiting(
    runtime_state: State<'_, SharedAgentRuntime>,
    task_type: String,
    request_id: String,
) -> Result<(), String> {
    runtime_state
        .lock()
        .unwrap()
        .cancel(TaskType::parse(&task_type)?, &request_id, true)
}

// ── Custom prompt templates ──

/// Max size for an uploaded custom template file (1 MiB).
const MAX_TEMPLATE_SIZE: u64 = 1_048_576;

#[tauri::command]
pub async fn save_custom_template(
    app_handle: AppHandle,
    agent_type: String,
    source_path: String,
) -> Result<(), String> {
    use crate::utils::paths;

    // Validate agent_type to prevent path traversal via the filename.
    if !matches!(agent_type.as_str(), "summary" | "translation" | "tagging") {
        return Err(format!("Unknown agent type: {}", agent_type));
    }

    // Reject oversized files before reading them into memory.
    let metadata =
        std::fs::metadata(&source_path).map_err(|e| format!("Failed to read file: {}", e))?;
    if metadata.len() > MAX_TEMPLATE_SIZE {
        return Err(format!(
            "File too large: {} bytes (max {} bytes)",
            metadata.len(),
            MAX_TEMPLATE_SIZE
        ));
    }

    let content =
        std::fs::read_to_string(&source_path).map_err(|e| format!("Failed to read file: {}", e))?;
    crate::agent::prompt_templates::save_custom_template(
        &paths::prompts_dir(&app_handle),
        &agent_type,
        &content,
    )
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
    Ok(runtime_state.lock().unwrap().get_status(tt))
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
    supports_summary: Option<bool>,
    supports_translation: Option<bool>,
    supports_tagging: Option<bool>,
) -> Result<(), String> {
    agent_store::create_model(
        &pool,
        &provider_id,
        &name,
        &model_name,
        [supports_summary, supports_translation, supports_tagging],
    )
    .await
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
    let name = name.trim().to_owned();
    let model_name = model_name.trim().to_owned();
    if name.is_empty() || model_name.is_empty() {
        return Err("Model name is required".into());
    }
    let pool = pool.inner().clone();
    let changed = pool
        .get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "UPDATE agent_model_profile SET name = ?1, model_name = ?2,
                 supports_summary = ?3, supports_translation = ?4, supports_tagging = ?5
                 WHERE id = ?6 AND is_archived=0",
                rusqlite::params![
                    name,
                    model_name,
                    supports_summary,
                    supports_translation,
                    supports_tagging,
                    id
                ],
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Model not found".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_model(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    agent_store::archive_model(&pool, &id).await
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
    if !matches!(agent_type.as_str(), "summary" | "translation" | "tagging") {
        return Err("Unknown agent type".into());
    }
    for id in [&primary_model_id, &fallback_model_id]
        .into_iter()
        .flatten()
    {
        let mut valid = false;
        for provider in agent_store::list_providers(&pool).await? {
            if provider.is_enabled
                && agent_store::list_models(&pool, &provider.id)
                    .await?
                    .iter()
                    .any(|model| {
                        model.id == *id
                            && model.is_enabled
                            && crate::agent::configuration::supports(model, &agent_type)
                    })
            {
                valid = true;
                break;
            }
        }
        if !valid {
            return Err(format!(
                "Model is unavailable or does not support {agent_type}: {id}"
            ));
        }
    }
    let pool = pool.inner().clone();
    pool.get()
        .await
        .map_err(|e| e.to_string())?
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
                rusqlite::params![
                    agent_type,
                    primary_model_id,
                    fallback_model_id,
                    target_language,
                    detail_level,
                    prompt_strategy,
                    concurrency_degree
                ],
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_configuration_enabled(
    pool: State<'_, Pool>,
    id: String,
    entity: String,
    enabled: bool,
) -> Result<(), String> {
    agent_store::set_enabled(&pool, &id, &entity, enabled).await
}
