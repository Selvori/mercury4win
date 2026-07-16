// mercury4win-linux/src-tauri/agent/translation.rs
// Translation agent — per-segment concurrency, hash-based cache invalidation
// Mirrors original Mercury Translation segmentation + hash contract

use std::collections::HashMap;
use std::sync::Arc;
use reqwest::Client;
use tauri::ipc::Channel;
use futures_util::StreamExt;

use crate::agent::prompt_templates;
use crate::agent::provider::{self, ChatMessage, ChatCompletionRequest};
use crate::db::{entry_store, content_store, usage_store};
use crate::db::models::{Content, LlmUsageEvent};

/// Run a translation task with per-segment concurrent streaming.
pub async fn run_translation(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    entry_id: i64,
    target_language: &str,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
    prompt_strategy: Option<&str>,
    on_event: Channel<String>,
) -> Result<(), String> {
    let detail = entry_store::get_entry_detail(pool, entry_id)
        .await?
        .ok_or("Entry not found")?;

    let content = content_store::get_content(pool, entry_id).await?;
    let markdown = content
        .as_ref()
        .and_then(|c| c.markdown.as_deref())
        .unwrap_or("");

    // Segment the content: split by double-newline (paragraph boundaries)
    let segments: Vec<String> = markdown
        .split("\n\n")
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .collect();

    if segments.is_empty() {
        return Err("No translatable content found".into());
    }

    let template =
        prompt_templates::load_builtin_template("translation", prompt_strategy)?;

    let total = segments.len();
    let concurrency = 3usize;
    let template = Arc::new(template);
    let target_language = Arc::new(target_language.to_string());
    let provider_base_url = Arc::new(provider_base_url.to_string());
    let api_key = Arc::new(api_key.to_string());
    let model_name = Arc::new(model_name.to_string());

    // Concurrent translation with buffer_unordered
    let mut stream = futures_util::stream::iter(
        segments.into_iter().enumerate().map(|(i, segment)| {
            let template = template.clone();
            let target_language = target_language.clone();
            let provider_base_url = provider_base_url.clone();
            let api_key = api_key.clone();
            let model_name = model_name.clone();
            let client = client.clone();
            let on_event = on_event.clone();
            let total = total;

            async move {
                let mut params = HashMap::new();
                params.insert("target_language".into(), (*target_language).clone());
                params.insert("source_text".into(), segment);

                let rendered = template.render(&params);

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
                    model: (*model_name).clone(),
                    messages,
                    temperature: Some(0.3),
                    top_p: None,
                    max_tokens: Some(2048),
                    stream: None,
                };

                let _ = on_event.send(format!(
                    r#"{{"type":"progress","segment":{},"total":{}}}"#,
                    i + 1,
                    total
                ));

                match provider::chat_completion(
                    &client, &provider_base_url, &api_key, &request,
                )
                .await
                {
                    Ok(response) => {
                        let translated = response
                            .choices
                            .first()
                            .map(|c| c.message.content.clone())
                            .unwrap_or_default();

                        let _ = on_event.send(format!(
                            r#"{{"type":"segment","index":{},"text":{}}}"#,
                            i,
                            serde_json::to_string(&translated).unwrap_or_default()
                        ));

                        Ok((i, translated, response.usage))
                    }
                    Err(e) => {
                        let _ = on_event.send(format!(
                            r#"{{"type":"error","segment":{},"error":"{}"}}"#,
                            i, e
                        ));
                        Err(e)
                    }
                }
            }
        })
    )
    .buffer_unordered(concurrency);

    let mut total_prompt = 0i64;
    let mut total_completion = 0i64;
    let mut total_tokens = 0i64;

    while let Some(result) = stream.next().await {
        if let Ok((_i, _translated, usage)) = &result {
            if let Some(ref u) = usage {
                total_prompt += u.prompt_tokens as i64;
                total_completion += u.completion_tokens as i64;
                total_tokens += u.total_tokens as i64;
            }
        }
    }

    // Log aggregated usage
    let _ = usage_store::insert_usage_event(
        pool,
        &LlmUsageEvent {
            id: 0,
            task_run_id: None,
            provider_name: Some("default".into()),
            model_name: Some((*model_name).clone()),
            agent_type: Some("translation".into()),
            prompt_tokens: Some(total_prompt),
            completion_tokens: Some(total_completion),
            total_tokens: Some(total_tokens),
            request_status: Some("success".into()),
            created_at: String::new(),
        },
    )
    .await;

    let _ = on_event.send(
        r#"{"type":"done","result":"translation_complete"}"#.to_string(),
    );

    Ok(())
}

/// Compute a sourceContentHash for translation cache invalidation.
/// Mirrors original Mercury's SHA-256 hash over all segment payloads.
pub fn compute_source_content_hash(segments: &[&str]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    for seg in segments {
        hasher.update(seg.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}
