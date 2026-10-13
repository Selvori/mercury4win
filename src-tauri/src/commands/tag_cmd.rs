// mercury4win/src-tauri/commands/tag_cmd.rs
// Tag management Tauri commands

use crate::agent::runtime::{self, SharedAgentRuntime, TaskType};
use crate::commands::feed_cmd::AppClient;
use crate::db::models::{Entry, Tag, TagAlias};
use crate::db::tag_batch_store::{self, BatchApplyResult, BatchScope, BatchSnapshot};
use crate::db::tag_store;
use deadpool_sqlite::Pool;
use tauri::State;
use tauri::{ipc::Channel, AppHandle};

#[tauri::command]
pub async fn list_tags(pool: State<'_, Pool>, search: Option<String>) -> Result<Vec<Tag>, String> {
    tag_store::list_tags(&pool, search.as_deref()).await
}

#[tauri::command]
pub async fn add_tag(pool: State<'_, Pool>, entry_id: i64, name: String) -> Result<Tag, String> {
    tag_store::add_tag(&pool, entry_id, &name).await
}

#[tauri::command]
pub async fn remove_tag(pool: State<'_, Pool>, entry_id: i64, tag_id: i64) -> Result<(), String> {
    tag_store::remove_tag(&pool, entry_id, tag_id).await
}

#[tauri::command]
pub async fn merge_tags(
    pool: State<'_, Pool>,
    source_id: i64,
    target_id: i64,
) -> Result<(), String> {
    tag_store::merge_tags(&pool, source_id, target_id).await
}

#[tauri::command]
pub async fn rename_tag(
    pool: State<'_, Pool>,
    tag_id: i64,
    new_name: String,
) -> Result<(), String> {
    tag_store::rename_tag(&pool, tag_id, &new_name).await
}

#[tauri::command]
pub async fn delete_tag(pool: State<'_, Pool>, tag_id: i64) -> Result<(), String> {
    tag_store::delete_tag(&pool, tag_id).await
}

#[tauri::command]
pub async fn list_tag_aliases(pool: State<'_, Pool>, tag_id: i64) -> Result<Vec<TagAlias>, String> {
    tag_store::list_aliases(&pool, tag_id).await
}
#[tauri::command]
pub async fn add_tag_alias(
    pool: State<'_, Pool>,
    tag_id: i64,
    alias: String,
) -> Result<(), String> {
    tag_store::mutate(&pool, "alias", tag_id, &alias)
        .await
        .map(|_| ())
}
#[tauri::command]
pub async fn delete_tag_alias(pool: State<'_, Pool>, alias_id: i64) -> Result<(), String> {
    tag_store::mutate(&pool, "unalias", alias_id, "")
        .await
        .map(|_| ())
}
#[tauri::command]
pub async fn promote_tag(pool: State<'_, Pool>, tag_id: i64) -> Result<(), String> {
    tag_store::mutate(&pool, "promote", tag_id, "")
        .await
        .map(|_| ())
}
#[tauri::command]
pub async fn cleanup_unused_tags(pool: State<'_, Pool>) -> Result<usize, String> {
    tag_store::mutate(&pool, "cleanup", 0, "").await
}
#[tauri::command]
pub async fn local_tag_suggestions(
    pool: State<'_, Pool>,
    entry_id: i64,
) -> Result<Vec<String>, String> {
    local_suggestions(&pool, entry_id).await
}
pub async fn local_suggestions(pool: &Pool, entry_id: i64) -> Result<Vec<String>, String> {
    let detail = crate::db::entry_store::get_entry_detail(pool, entry_id)
        .await?
        .ok_or("Entry not found")?;
    let text = format!(
        "{} {}",
        detail.entry.title.unwrap_or_default(),
        detail.entry.summary.unwrap_or_default()
    );
    let normalized = tag_store::normalize(&text);
    let applied: std::collections::HashSet<_> = detail
        .tags
        .iter()
        .map(|t| tag_store::normalize(&t.name))
        .collect();
    let mut candidates = Vec::new();
    for tag in tag_store::list_tags(pool, None).await? {
        if normalized.contains(&tag.normalized_name) && !applied.contains(&tag.normalized_name) {
            candidates.push(tag.name);
        }
    }
    // Portable, deliberately coarse extraction; no OS-specific NLP or AI request.
    for word in text.split(|c: char| !c.is_alphanumeric() && c != '-') {
        if word.chars().count() >= 3
            && word.chars().count() <= 25
            && word
                .chars()
                .next()
                .is_some_and(|c| c.is_uppercase() || !c.is_ascii())
            && ![
                "The", "This", "That", "With", "From", "Here", "How", "What", "Why",
            ]
            .contains(&word)
        {
            candidates.push(word.into());
        }
    }
    Ok(tag_store::canonical_names(pool, candidates)
        .await?
        .into_iter()
        .filter(|n| !applied.contains(&tag_store::normalize(n)))
        .take(3)
        .collect())
}
#[tauri::command]
pub async fn related_entries(pool: State<'_, Pool>, entry_id: i64) -> Result<Vec<Entry>, String> {
    related(&pool, entry_id).await
}
pub async fn related(pool: &Pool, entry_id: i64) -> Result<Vec<Entry>, String> {
    let ids:Vec<i64>=pool.get().await.map_err(|e|e.to_string())?.interact(move|conn|{
 conn.prepare("SELECT other.entry_id FROM entry_tag own JOIN tag t ON t.id=own.tag_id JOIN entry_tag other ON other.tag_id=own.tag_id JOIN entry e ON e.id=other.entry_id WHERE own.entry_id=?1 AND other.entry_id<>?1 AND e.is_deleted=0 AND (t.is_provisional=0 OR own.source='manual') GROUP BY other.entry_id ORDER BY COUNT(*) DESC,COALESCE(e.published_at,e.created_at) DESC,other.entry_id DESC LIMIT 5").map_err(|e|e.to_string())?.query_map([entry_id],|r|r.get(0)).map_err(|e|e.to_string())?.collect::<Result<_,_>>().map_err(|e|e.to_string())
 }).await.map_err(|e|e.to_string())??;
    let mut entries = Vec::new();
    for id in ids {
        if let Some(detail) = crate::db::entry_store::get_entry_detail(pool, id).await? {
            entries.push(detail.entry);
        }
    }
    Ok(entries)
}
#[tauri::command]
pub async fn count_tag_batch(pool: State<'_, Pool>, scope: BatchScope) -> Result<usize, String> {
    tag_batch_store::count(&pool, scope).await
}
#[tauri::command]
pub async fn load_tag_batch(
    pool: State<'_, Pool>,
    runtime_state: State<'_, SharedAgentRuntime>,
    run_id: Option<String>,
) -> Result<Option<BatchSnapshot>, String> {
    load_or_recover(&pool, &runtime_state, run_id).await
}
pub async fn load_or_recover(
    pool: &Pool,
    runtime_state: &SharedAgentRuntime,
    run_id: Option<String>,
) -> Result<Option<BatchSnapshot>, String> {
    let snapshot = tag_batch_store::load(pool, run_id).await?;
    if let Some(run) = &snapshot {
        if run.status == "running" {
            let status = runtime_state.lock().unwrap().get_status(TaskType::Tagging);
            if status.current_request_id.as_deref() != Some(&run.id)
                && status.waiting_request_id.as_deref() != Some(&run.id)
            {
                tag_batch_store::finish(&pool, &run.id).await?;
                return tag_batch_store::load(&pool, Some(run.id.clone())).await;
            }
        }
    }
    Ok(snapshot)
}
#[tauri::command]
pub async fn batch_tag(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    runtime_state: State<'_, SharedAgentRuntime>,
    app_handle: AppHandle,
    scope: BatchScope,
    request_id: String,
    confirmed_large: bool,
    concurrency: usize,
    on_event: Channel<String>,
) -> Result<BatchSnapshot, String> {
    let result = runtime::execute(
        &runtime_state,
        TaskType::Tagging,
        0,
        &request_id,
        &on_event,
        || async {
            // Explicit profile selection prevents a batch from guessing a model.
            let profile = crate::db::agent_store::get_agent_profile(&pool, "tagging").await?;
            if profile.and_then(|p| p.primary_model_id).is_none() {
                return Err("Select a tagging model in Settings before starting a batch".into());
            }
            let mut config =
                crate::commands::agent_cmd::execution_configuration(&pool, &app_handle, "tagging")
                    .await?;
            if !(1..=5).contains(&concurrency) {
                return Err("Batch concurrency must be between 1 and 5".into());
            }
            config.concurrency = concurrency;
            config.fingerprint = serde_json::json!({"configuration": config.fingerprint, "concurrency": concurrency}).to_string();
            if let Some(notice) = &config.notice {
                let _ = on_event
                    .send(serde_json::json!({"type":"notice","message":notice}).to_string());
            }
            crate::agent::tag_batch::generate(
                &pool,
                &client.0,
                &request_id,
                scope,
                config,
                confirmed_large,
                &on_event,
            )
            .await
        },
    )
    .await;
    // execute drops all HTTP futures and fences outstanding SQLite jobs first.
    if result.is_err() {
        tag_batch_store::finish(&pool, &request_id).await?;
    }
    result
}
#[tauri::command]
pub async fn apply_tag_batch(
    pool: State<'_, Pool>,
    run_id: String,
    kept: Vec<String>,
    discarded: Vec<String>,
) -> Result<BatchApplyResult, String> {
    tag_batch_store::apply(&pool, &run_id, kept, discarded).await
}
#[tauri::command]
pub async fn discard_tag_batch(pool: State<'_, Pool>, run_id: String) -> Result<(), String> {
    tag_batch_store::discard(&pool, &run_id).await
}
