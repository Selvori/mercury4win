// mercury4win-linux/src-tauri/agent/tagging.rs
// Tagging agent — non-streaming LLM completion for tag suggestions

use std::collections::HashMap;
use reqwest::Client;
use crate::agent::prompt_templates;
use crate::agent::provider::{self, ChatMessage, ChatCompletionRequest};
use crate::db::{entry_store, content_store, tag_store, usage_store};
use crate::db::models::LlmUsageEvent;

/// Run tagging agent. Returns a list of suggested tag names.
/// Retries up to 3 times when DeepSeek returns empty responses (known occasional quirk).
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
        max_tokens: Some(512),  // enough for a JSON array of 8 tags + formatting
        stream: None,
    };

    // Retry up to 3 times for empty responses (DeepSeek quirk) or transient network errors
    let mut raw = String::new();
    let mut last_usage = None;
    let mut last_error: Option<String> = None;
    for attempt in 0u32..3 {
        match provider::chat_completion(
            client, provider_base_url, api_key, &request,
        ).await {
            Ok(response) => {
                let content = response
                    .choices
                    .first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default();

                log::info!("[tagging] attempt={attempt}, raw LLM response (len={}): {:.300}", content.len(), content);

                if !content.trim().is_empty() {
                    raw = content;
                    last_usage = response.usage;
                    last_error = None;
                    break;
                }

                log::warn!("[tagging] empty response on attempt={attempt}, retrying...");
                last_error = Some("Empty response from LLM".into());
            }
            Err(e) => {
                log::warn!("[tagging] error on attempt={attempt}: {e}");
                last_error = Some(e);
            }
        }
        if attempt < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        }
    }

    // Log usage from the successful response
    if let Some(ref usage) = last_usage {
        let _ = usage_store::insert_usage_event(
            pool,
            &LlmUsageEvent {
                id: 0,
                task_run_id: None,
                provider_name: Some("default".into()),
                model_name: Some(model_name.into()),
                agent_type: Some("tagging".into()),
                prompt_tokens: Some(usage.prompt_tokens as i64),
                completion_tokens: Some(usage.completion_tokens as i64),
                total_tokens: Some(usage.total_tokens as i64),
                request_status: Some("success".into()),
                created_at: String::new(),
            },
        ).await;
    }

    // Parse tag names from LLM response.
    let tags: Vec<String> = parse_tag_response(&raw);

    // Deduplicate: remove tags that share the same normalized (lowercase) form
    let mut seen = std::collections::HashSet::new();
    let tags: Vec<String> = tags.into_iter().filter(|t| seen.insert(t.to_lowercase())).collect();

    // If all parsing failed, return the raw response as error so user can see it
    if tags.is_empty() {
        if raw.trim().is_empty() {
            // If we had a persisted error from all attempts, surface it
            if let Some(ref err) = last_error {
                return Err(format!("Tagging failed after 3 attempts: {}", err));
            }
            return Err("DeepSeek returned empty responses after 3 retries. Please try again.".into());
        }
        let preview: String = raw.chars().take(500).collect();
        return Err(format!("Failed to parse tags from LLM response. Raw output:\n{}", preview));
    }

    Ok(tags)
}

/// Parse tag names from the LLM response. Tries JSON array first, falls back to
/// comma/newline-separated plain text, and strips common noise (bullet points,
/// numbering, markdown fences).
fn parse_tag_response(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();

    // Strip markdown code fences if present
    let inner = if trimmed.starts_with("```") {
        trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .and_then(|s| s.strip_suffix("```"))
            .map(|s| s.trim())
            .unwrap_or(trimmed)
    } else {
        trimmed
    };

    let clean_tag = |s: &str| -> String {
        let t = s.trim();
        // Strip outermost quotes (single, double, backtick)
        let t = t.trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
        // Strip leading bracket remnants
        let t = t.trim_start_matches(|c: char| c == '[' || c == '(' || c.is_whitespace());
        let t = t.trim_end_matches(|c: char| c == ']' || c == ')' || c.is_whitespace());
        // Remove any remaining standalone quote marks
        let t = t.replace('"', "").replace('\'', "").replace('`', "");
        t.trim().to_string()
    };

    // Try JSON array first
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(inner) {
        let tags: Vec<String> = arr
            .into_iter()
            .map(|s| clean_tag(&s))
            .filter(|s| !s.is_empty() && s.len() <= 50)
            .take(8)
            .collect();
        if !tags.is_empty() {
            return tags;
        }
    }

    // Also try extracting a JSON array embedded in surrounding text
    if let Some(start) = inner.find('[') {
        if let Some(end) = inner.rfind(']') {
            if start < end {
                let slice = &inner[start..=end];
                // Try direct parse first
                if let Ok(arr) = serde_json::from_str::<Vec<String>>(slice) {
                    let tags: Vec<String> = arr
                        .into_iter()
                        .map(|s| clean_tag(&s))
                        .filter(|s| !s.is_empty() && s.len() <= 50)
                        .take(8)
                        .collect();
                    if !tags.is_empty() {
                        return tags;
                    }
                }
                // Repair truncated JSON: try appending "] and parsing
                let repaired = format!("{}\"]", &slice[..slice.len()-1]);
                if let Ok(arr) = serde_json::from_str::<Vec<String>>(&repaired) {
                    let tags: Vec<String> = arr
                        .into_iter()
                        .map(|s| clean_tag(&s))
                        .filter(|s| !s.is_empty() && s.len() <= 50)
                        .take(8)
                        .collect();
                    if !tags.is_empty() {
                        return tags;
                    }
                }
            }
        }
    }

    // Fallback: comma or newline separation, strip bullet markers and numbering
    inner
        .split(|c: char| c == ',' || c == '\n')
        .map(|s| {
            let cleaned = s.trim();
            // Strip leading bullet markers: "- ", "* ", "• ", "+ "
            let cleaned = cleaned
                .strip_prefix("- ")
                .or_else(|| cleaned.strip_prefix("* "))
                .or_else(|| cleaned.strip_prefix("• "))
                .or_else(|| cleaned.strip_prefix("+ "))
                .unwrap_or(cleaned);
            // Strip leading number like "1. " or "1) "
            let cleaned = strip_number_prefix(cleaned);
            // Strip quotes and bracket remnants
            let cleaned = cleaned.trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c == '[' || c == ']');
            // Remove internal quotes
            let cleaned = cleaned.replace('"', "").replace('\'', "").replace('`', "");
            cleaned.trim().to_string()
        })
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 50
                && !s.starts_with("```")
                && s.chars().any(|c| c.is_alphanumeric())
                && !s.chars().all(|c| c == '-' || c == '*' || c == '#')
        })
        .take(8)
        .collect()
}

/// Strip a leading number prefix like "1. " or "1) " from a string slice.
fn strip_number_prefix(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() {
        let rest = &s[i..];
        if let Some(stripped) = rest.strip_prefix(". ") {
            return stripped;
        }
        if let Some(stripped) = rest.strip_prefix(") ") {
            return stripped;
        }
    }
    s
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
