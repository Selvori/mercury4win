use crate::db::{
    entry_store::EntryFilter,
    search_store,
    smart_view_store::{self, SmartView},
};
use deadpool_sqlite::Pool;
use tauri::State;

#[tauri::command]
pub async fn search_local_topic(
    pool: State<'_, Pool>,
    client: State<'_, crate::commands::feed_cmd::AppClient>,
    runtime_state: State<'_, crate::agent::runtime::SharedAgentRuntime>,
    app_handle: tauri::AppHandle,
    sentence: String,
    manual: EntryFilter,
    request_id: String,
    resume: Option<String>,
    force: bool,
    on_event: tauri::ipc::Channel<String>,
) -> Result<crate::agent::staged_search::Run, crate::agent::search_filter::FilterError> {
    use crate::agent::{runtime, search_filter::FilterError};
    let result = runtime::execute(
        &runtime_state,
        runtime::TaskType::SearchFilter,
        0,
        &request_id,
        &on_event,
        || async {
            Ok(async {
                let config = crate::commands::agent_cmd::execution_configuration(
                    &pool,
                    &app_handle,
                    "summary",
                )
                .await
                .map_err(|_| FilterError::new("configuration", ""))?;
                crate::agent::usage::scope(
                    &pool,
                    0,
                    "search_filter",
                    crate::agent::staged_search::run(
                        &pool,
                        &client.0,
                        config,
                        sentence,
                        manual,
                        resume,
                        force,
                        request_id.clone(),
                        &on_event,
                    ),
                )
                .await
            }
            .await)
        },
    )
    .await;
    result.map_err(|_| FilterError::new("cancelled", ""))?
}

#[tauri::command]
pub async fn stop_topic_search(
    pool: State<'_, Pool>,
    runtime_state: State<'_, crate::agent::runtime::SharedAgentRuntime>,
    request_id: String,
    mode: String,
) -> Result<Option<crate::agent::staged_search::Progress>, crate::agent::search_filter::FilterError>
{
    use crate::agent::{runtime, search_filter::FilterError};
    if !["paused", "cancelled"].contains(&mode.as_str()) {
        return Err(FilterError::new("input", ""));
    }
    runtime_state
        .lock()
        .map_err(|_| FilterError::new("cancelled", ""))?
        .cancel(runtime::TaskType::SearchFilter, &request_id, false)
        .map_err(|_| FilterError::new("cancelled", ""))?;
    crate::db::staged_search_store::stop(&pool, &request_id, &mode).await
}

#[tauri::command]
pub async fn recent_topic_search(
    pool: State<'_, Pool>,
) -> Result<
    Option<crate::db::staged_search_store::ResumeState>,
    crate::agent::search_filter::FilterError,
> {
    crate::db::staged_search_store::recent(&pool).await
}

#[tauri::command]
pub async fn parse_search_filter(
    pool: State<'_, Pool>,
    client: State<'_, crate::commands::feed_cmd::AppClient>,
    runtime_state: State<'_, crate::agent::runtime::SharedAgentRuntime>,
    app_handle: tauri::AppHandle,
    sentence: String,
    request_id: String,
    on_event: tauri::ipc::Channel<String>,
) -> Result<EntryFilter, crate::agent::search_filter::FilterError> {
    use crate::agent::{
        runtime,
        search_filter::{self, FilterError},
    };
    search_filter::validate_input(&sentence)?;
    // The summary route is shared, while cancellation occupies its own task slot.
    let result = runtime::execute(
        &runtime_state,
        runtime::TaskType::SearchFilter,
        0,
        &request_id,
        &on_event,
        || async {
            Ok(async {
                let config = crate::commands::agent_cmd::execution_configuration(
                    &pool,
                    &app_handle,
                    "summary",
                )
                .await
                .map_err(|_| FilterError::new("configuration", ""))?;
                let metadata = search_filter::metadata(&pool)
                    .await
                    .map_err(|_| FilterError::new("metadata", ""))?;
                let filter = crate::agent::usage::scope(
                    &pool,
                    0,
                    "search_filter",
                    search_filter::request(
                        &client.0,
                        config,
                        &sentence,
                        &metadata,
                        chrono::Local::now().fixed_offset(),
                    ),
                )
                .await?;
                let latest = search_filter::metadata(&pool)
                    .await
                    .map_err(|_| FilterError::new("metadata", ""))?;
                search_filter::validate_references(&filter, &sentence, &latest)?;
                Ok::<_, FilterError>(filter)
            }
            .await)
        },
    )
    .await;
    // Runtime's string errors are kept separate from the parser's typed errors.
    result.map_err(|_| FilterError::new("cancelled", ""))?
}

#[tauri::command]
pub async fn list_smart_views(pool: State<'_, Pool>) -> Result<Vec<SmartView>, String> {
    smart_view_store::list(&pool).await
}
#[tauri::command]
pub async fn save_smart_view(
    pool: State<'_, Pool>,
    id: Option<i64>,
    name: String,
    filter: EntryFilter,
) -> Result<i64, String> {
    smart_view_store::save(&pool, id, name, filter).await
}
#[tauri::command]
pub async fn delete_smart_view(pool: State<'_, Pool>, id: i64) -> Result<(), String> {
    smart_view_store::delete(&pool, id).await
}
#[tauri::command]
pub async fn rebuild_search_index(pool: State<'_, Pool>) -> Result<(), String> {
    search_store::rebuild(&pool).await
}
