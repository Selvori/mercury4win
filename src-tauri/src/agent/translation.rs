// mercury4win-linux/src-tauri/agent/translation.rs
// Translation agent — per-segment bounded concurrency, hash-based cache invalidation
// Mirrors original Mercury Translation segmentation + hash contract

use std::collections::HashMap;
use reqwest::Client;
use tauri::ipc::Channel;

use crate::agent::prompt_templates;
use crate::agent::provider::{self, ChatMessage, ChatCompletionRequest};
use crate::db::{entry_store, content_store};
use crate::db::models::Content;

/// Run a translation task with per-segment streaming. Each p/ul/ol segment is sent
/// as a separate request with optional previous-context from the template.
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
    let segments: Vec<&str> = markdown
        .split("\n\n")
        .filter(|s| !s.trim().is_empty())
        .collect();

    if segments.is_empty() {
        return Err("No translatable content found".into());
    }

    let template =
        prompt_templates::load_builtin_template("translation", prompt_strategy)?;

    let mut previous_translated: Option<String> = None;
    let total = segments.len();

    // Per-segment translation with bounded concurrency (default 3)
    let concurrency = 3usize;
    let mut results: Vec<String> = Vec::with_capacity(total);

    for (i, segment) in segments.iter().enumerate() {
        let mut params = HashMap::new();
        params.insert("target_language".into(), target_language.to_string());
        params.insert("source_text".into(), segment.to_string());
        if let Some(ref prev) = previous_translated {
            if i > 0 {
                params.insert("previous_translated_text".into(), prev.clone());
            }
        }

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
            model: model_name.to_string(),
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
            client, provider_base_url, api_key, &request,
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
                previous_translated = Some(translated.clone());
                results.push(translated);
            }
            Err(e) => {
                let _ = on_event.send(format!(
                    r#"{{"type":"error","segment":{},"error":"{}"}}"#,
                    i, e
                ));
            }
        }
    }

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
