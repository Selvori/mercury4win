// mercury4win-linux/src-tauri/agent/tagging.rs
// Tagging agent — non-streaming LLM completion for tag suggestions

use std::collections::HashMap;
use reqwest::Client;
use crate::agent::prompt_templates;
use crate::agent::provider::{self, ChatMessage, ChatCompletionRequest};
use crate::db::{entry_store, content_store, tag_store};

/// Run tagging agent. Returns a list of suggested tag names.
pub async fn run_tagging(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    entry_id: i64,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
) -> Result<Vec<String>, String> {
    let detail = entry_store::get_entry_detail(pool, entry_id)
        .await?
        .ok_or("Entry not found")?;

    let content = content_store::get_content(pool, entry_id).await?;
    let markdown = content
        .as_ref()
        .and_then(|c| c.markdown.as_deref())
        .unwrap_or("");

    // Get existing tags for the template
    let existing = tag_store::list_tags(pool, None).await?;
    let existing_tags: Vec<String> = existing.iter().map(|t| t.name.clone()).collect();
    let existing_tags_str = existing_tags.join("\n");

    let template = prompt_templates::load_builtin_template("tagging", None)?;

    let mut params = HashMap::new();
    params.insert("title".into(), detail.entry.title.unwrap_or_default());
    params.insert(
        "byline".into(),
        detail.entry.author.unwrap_or_default(),
    );
    params.insert("content".into(), markdown.to_string());
    params.insert("existing_tags".into(), existing_tags_str);

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
        max_tokens: Some(256),
        stream: None,
    };

    let response = provider::chat_completion(
        client, provider_base_url, api_key, &request,
    )
    .await?;

    let raw = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    // Parse tag names from LLM response (expects comma or newline-separated)
    let tags: Vec<String> = raw
        .split(|c: char| c == ',' || c == '\n')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty() && s.len() <= 50)
        .take(8)
        .collect();

    Ok(tags)
}

/// Apply tag suggestions to an entry (creates provisional tags).
pub async fn apply_tag_suggestions(
    pool: &deadpool_sqlite::Pool,
    entry_id: i64,
    tag_names: &[String],
) -> Result<usize, String> {
    let mut applied = 0usize;
    for name in tag_names {
        match tag_store::add_tag(pool, entry_id, name).await {
            Ok(_) => applied += 1,
            Err(_) => {} // skip duplicates
        }
    }
    Ok(applied)
}
