// mercury4win/src-tauri/agent/summary.rs
// Summary agent — streaming LLM completion with 3 detail levels

use reqwest::Client;
use std::collections::HashMap;
use tauri::ipc::Channel;

use crate::agent::provider::{self, ChatCompletionRequest, ChatMessage};
use crate::agent::runtime::SharedAgentRuntime;
use crate::db::agent_result_store::{self, AgentResult, ResultSlot};

/// Run a streaming summary task. Pushes chunks through the Channel.
pub async fn run_summary(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    _runtime: &SharedAgentRuntime,
    entry_id: i64,
    target_language: &str,
    detail_level: &str,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
    on_event: Channel<String>,
) -> Result<(), String> {
    crate::agent::usage::scope(
        pool,
        entry_id,
        "summary",
        run_summary_inner(
            pool,
            client,
            _runtime,
            entry_id,
            target_language,
            detail_level,
            provider_base_url,
            api_key,
            model_name,
            on_event,
        ),
    )
    .await
}

async fn run_summary_inner(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    _runtime: &SharedAgentRuntime,
    entry_id: i64,
    target_language: &str,
    detail_level: &str,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
    on_event: Channel<String>,
) -> Result<(), String> {
    let source = agent_result_store::source_snapshot(pool, entry_id).await?;
    if source.markdown.trim().is_empty() {
        return Err("No summarizable content found".into());
    }
    let slot = ResultSlot::new(
        entry_id,
        "summary",
        target_language,
        detail_level,
        &source.hash,
    );
    let markdown = &source.markdown;

    // Load and render prompt template
    let template = crate::agent::configuration::template("summary", None)?;

    let mut params = HashMap::new();
    params.insert("title".into(), source.title);
    params.insert("byline".into(), source.author);
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
        temperature: Some(crate::agent::configuration::request_parameters("summary").temperature),
        top_p: None,
        max_tokens: Some(crate::agent::configuration::request_parameters("summary").max_tokens),
        stream: None,
    };

    // Stream via provider
    match provider::chat_completion_stream_with_content(
        client,
        provider_base_url,
        api_key,
        request,
        on_event.clone(),
    )
    .await
    {
        Ok((usage, text)) => {
            if text.trim().is_empty() {
                return Err("Empty summary response".into());
            }
            let actual_model = usage
                .route
                .as_ref()
                .map(|r| r.model_name.as_str())
                .unwrap_or(model_name);
            slot.save(
                pool,
                &AgentResult {
                    request_route: usage.route.clone(),
                    segment_routes: Vec::new(),
                    segment_ids: Vec::new(),
                    segment_errors: Vec::new(),
                    source_hash: source.hash,
                    text: Some(text),
                    segments: vec![],
                    complete: true,
                    model_name: actual_model.into(),
                },
            )
            .await?;
            // No more awaits after the terminal Channel event.
            let _ = on_event.send(r#"{"type":"done"}"#.to_string());
            Ok(())
        }
        Err(e) => {
            let _ = on_event.send(provider::summary_error_event(&e));
            Err(e)
        }
    }
}
