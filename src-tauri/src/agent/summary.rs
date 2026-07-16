// mercury4win-linux/src-tauri/agent/summary.rs
// Summary agent — streaming LLM completion with 3 detail levels

use std::collections::HashMap;
use reqwest::Client;
use tauri::ipc::Channel;

use crate::agent::prompt_templates;
use crate::agent::provider::{self, ChatMessage, ChatCompletionRequest};
use crate::agent::runtime::SharedAgentRuntime;
use crate::db::{entry_store, content_store, usage_store};
use crate::db::models::LlmUsageEvent;

/// Run a streaming summary task. Pushes chunks through the Channel.
pub async fn run_summary(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    runtime: &SharedAgentRuntime,
    entry_id: i64,
    target_language: &str,
    detail_level: &str,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
    on_event: Channel<String>,
) -> Result<(), String> {
    // Load entry detail
    let detail = entry_store::get_entry_detail(pool, entry_id)
        .await?
        .ok_or("Entry not found")?;

    let content = content_store::get_content(pool, entry_id).await?;
    let markdown = content
        .as_ref()
        .and_then(|c| c.markdown.as_deref())
        .unwrap_or("");

    // Load and render prompt template
    let template = prompt_templates::load_builtin_template("summary", None)?;

    let mut params = HashMap::new();
    params.insert("title".into(), detail.entry.title.unwrap_or_default());
    params.insert(
        "byline".into(),
        detail.entry.author.unwrap_or_default(),
    );
    params.insert("content".into(), markdown.to_string());
    // Map locale codes to readable language names for the LLM
    let lang_name = match target_language {
        "zh-Hans" => "Simplified Chinese",
        "zh-Hant" => "Traditional Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        other => other,
    };
    params.insert("target_language".into(), lang_name.to_string());
    // Map detail_level to a human-readable name for the template conditional
    let detail_name = match detail_level {
        "brief" => "brief",
        "medium" => "medium",
        "detailed" => "detailed",
        other => other,
    };
    params.insert("detail_level".into(), detail_name.to_string());

    let rendered = template.render(&params);

    // Build messages
    let mut messages = Vec::new();
    if let Some(ref system) = rendered.system_message {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system.clone(),
        });
    }
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: rendered.user_message,
    });

    let request = ChatCompletionRequest {
        model: model_name.to_string(),
        messages,
        temperature: Some(0.5),
        top_p: None,
        max_tokens: Some(1024),
        stream: None,
    };

    // Stream via provider
    match provider::chat_completion_stream(
        client, provider_base_url, api_key, request, on_event.clone(),
    )
    .await
    {
        Ok(usage) => {
            // Signal completion to the frontend
            let _ = on_event.send(r#"{"type":"done"}"#.to_string());

            // Log usage
            let _ = usage_store::insert_usage_event(
                pool,
                &LlmUsageEvent {
                    id: 0,
                    task_run_id: None,
                    provider_name: Some("default".into()),
                    model_name: Some(model_name.into()),
                    agent_type: Some("summary".into()),
                    prompt_tokens: Some(usage.prompt_tokens as i64),
                    completion_tokens: Some(usage.completion_tokens as i64),
                    total_tokens: Some(usage.total_tokens as i64),
                    request_status: Some("success".into()),
                    created_at: String::new(),
                },
            )
            .await;
            Ok(())
        }
        Err(e) => {
            let _ = on_event.send(format!(r#"{{"type":"error","message":"{}"}}"#, e));
            Err(e)
        }
    }
}
