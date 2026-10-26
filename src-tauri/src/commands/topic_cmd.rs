use crate::{
    agent::{
        self,
        runtime::{self, SharedAgentRuntime, TaskType},
    },
    commands::{agent_cmd::execution_configuration, feed_cmd::AppClient},
    db::{
        entry_store::EntryFilter,
        topic_store::{self, Detail, Topic},
    },
};
use deadpool_sqlite::Pool;
use tauri::{ipc::Channel, AppHandle, State};

#[tauri::command]
pub async fn topic_model_info(
    pool: State<'_, Pool>,
    app_handle: AppHandle,
) -> Result<serde_json::Value, String> {
    let config = agent::configuration::load(
        &pool,
        &crate::utils::paths::prompts_dir(&app_handle),
        "summary",
    )
    .await?;
    Ok(serde_json::json!({"routes":config.routes}))
}

#[tauri::command]
pub async fn list_topics(pool: State<'_, Pool>) -> Result<Vec<Topic>, String> {
    topic_store::list(&pool).await
}
#[tauri::command]
pub async fn save_topic(
    pool: State<'_, Pool>,
    id: Option<i64>,
    name: String,
    description: String,
    archived: bool,
    rule: Option<EntryFilter>,
    rule_name: Option<String>,
) -> Result<i64, String> {
    topic_store::save(&pool, id, name, description, archived, rule, rule_name).await
}
#[tauri::command]
pub async fn delete_topic(pool: State<'_, Pool>, id: i64) -> Result<(), String> {
    topic_store::delete(&pool, id).await
}
#[tauri::command]
pub async fn load_topic(pool: State<'_, Pool>, id: i64) -> Result<Detail, String> {
    topic_store::detail(&pool, id).await
}
#[tauri::command]
pub async fn update_topic_members(
    pool: State<'_, Pool>,
    id: i64,
    entry_ids: Vec<i64>,
    remove: bool,
    group_name: Option<String>,
) -> Result<(), String> {
    topic_store::members(&pool, id, entry_ids, remove, group_name).await
}
#[tauri::command]
pub async fn save_topic_notes(pool: State<'_, Pool>, id: i64, notes: String) -> Result<(), String> {
    topic_store::notes(&pool, id, notes).await
}
#[tauri::command]
pub async fn generate_topic_brief(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    runtime_state: State<'_, SharedAgentRuntime>,
    app_handle: AppHandle,
    id: i64,
    request_id: String,
    language: String,
    on_event: Channel<String>,
) -> Result<Option<i64>, String> {
    // No configuration/credential access and no HTTP when the successful scope is unchanged.
    let d = topic_store::detail(&pool, id).await?;
    if !d.changed {
        return Ok(None);
    }
    agent::topic::check_scope(&d.input)?;
    let config = execution_configuration(&pool, &app_handle, "summary").await?;
    topic_store::start_run(&pool,request_id.clone(),d.input.clone(),serde_json::json!({"routes":config.routes,"fingerprint":config.fingerprint,"prompt_version":"topic-comparison-v1","language":language})).await?;
    let result = runtime::execute(
        &runtime_state,
        TaskType::Topic,
        id,
        &request_id,
        &on_event,
        || {
            agent::topic::generate(
                &pool,
                &client.0,
                d.input,
                d.history.first().cloned(),
                config,
                request_id.clone(),
                language,
                &on_event,
            )
        },
    )
    .await;
    if let Err(e) = &result {
        topic_store::finish_run(
            &pool,
            request_id,
            if e.contains("cancel") {
                "cancelled"
            } else {
                "failed"
            }
            .into(),
            Some(e.clone()),
        )
        .await?;
    }
    result.map(Some)
}
#[tauri::command]
pub async fn export_topic_brief(
    pool: State<'_, Pool>,
    id: i64,
    brief_id: i64,
    export_path: String,
) -> Result<(), String> {
    let d = topic_store::detail(&pool, id).await?;
    let brief = d
        .history
        .iter()
        .find(|b| b.id == brief_id)
        .ok_or("Briefing no longer exists")?;
    if !export_path.to_lowercase().ends_with(".md") {
        return Err("Choose a Markdown (.md) file".into());
    }
    tokio::fs::write(export_path, agent::topic::markdown(brief, &d.topic.notes))
        .await
        .map_err(|e| e.to_string())
}
