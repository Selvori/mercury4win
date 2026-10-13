// mercury4win/src-tauri/agent/tagging.rs
// Tagging agent — non-streaming LLM completion for tag suggestions

use crate::agent::provider::{self, ChatCompletionRequest, ChatMessage};
use crate::db::{tag_batch_store, tag_store};
use reqwest::Client;
use std::collections::HashMap;

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
    run_tagging_with_source(
        pool,
        client,
        entry_id,
        provider_base_url,
        api_key,
        model_name,
    )
    .await
    .map(|(_, tags)| tags)
}

pub async fn run_tagging_with_source(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    entry_id: i64,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
) -> Result<(String, Vec<String>), String> {
    crate::agent::usage::scope(
        pool,
        entry_id,
        "tagging",
        run_tagging_with_source_inner(
            pool,
            client,
            entry_id,
            provider_base_url,
            api_key,
            model_name,
        ),
    )
    .await
}

async fn run_tagging_with_source_inner(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    entry_id: i64,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
) -> Result<(String, Vec<String>), String> {
    let source = tag_batch_store::load_source(pool, entry_id).await?;

    // Get existing tags for the template
    let existing = tag_store::list_tags(pool, None).await?;
    let existing_tags: Vec<String> = existing.iter().map(|t| t.name.clone()).collect();
    let existing_tags_str = existing_tags.join("\n");

    let template = crate::agent::configuration::template("tagging", None)?;

    let mut params = HashMap::new();
    params.insert("title".into(), source.title);
    params.insert("byline".into(), source.author);
    params.insert("content".into(), source.content);
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
        temperature: Some(crate::agent::configuration::request_parameters("tagging").temperature),
        top_p: None,
        max_tokens: Some(crate::agent::configuration::request_parameters("tagging").max_tokens), // enough for a JSON array of 8 tags + formatting
        stream: None,
    };

    // Retry up to 3 times for empty responses (DeepSeek quirk) or transient network errors
    let mut raw = String::new();
    let mut last_error: Option<String> = None;
    for attempt in 0u32..3 {
        match crate::agent::usage::PHASE
            .scope(
                if attempt == 0 { "normal" } else { "retry" },
                provider::chat_completion(client, provider_base_url, api_key, &request),
            )
            .await
        {
            Ok(response) => {
                let content = response
                    .choices
                    .first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default();

                log::info!(
                    "[tagging] attempt={attempt}, raw LLM response (len={}): {:.300}",
                    content.len(),
                    content
                );

                if !content.trim().is_empty() {
                    raw = content;
                    last_error = None;
                    break;
                }

                log::warn!("[tagging] empty response on attempt={attempt}, retrying...");
                last_error = Some("Empty response from LLM".into());
            }
            Err(e) => {
                log::warn!("[tagging] request failed on attempt={attempt}");
                last_error = Some(e);
            }
        }
        if attempt < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
        }
    }

    // Parse tag names from LLM response.
    let tags: Vec<String> = parse_tag_response(&raw);

    // Deduplicate: remove tags that share the same normalized (lowercase) form
    let mut seen = std::collections::HashSet::new();
    let tags: Vec<String> = tags
        .into_iter()
        .filter(|t| seen.insert(tag_store::normalize(t)))
        .collect();

    // If all parsing failed, return the raw response as error so user can see it
    if tags.is_empty() {
        if raw.trim().is_empty() {
            // If we had a persisted error from all attempts, surface it
            if let Some(ref err) = last_error {
                return Err(format!("Tagging failed after 3 attempts: {}", err));
            }
            return Err(
                "DeepSeek returned empty responses after 3 retries. Please try again.".into(),
            );
        }
        let preview: String = raw.chars().take(500).collect();
        return Err(format!(
            "Failed to parse tags from LLM response. Raw output:\n{}",
            preview
        ));
    }

    Ok((source.hash, tag_store::canonical_names(pool, tags).await?))
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
                let repaired = format!("{}\"]", &slice[..slice.len() - 1]);
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
            let cleaned = cleaned
                .trim_matches(|c: char| c == '"' || c == '\'' || c == '`' || c == '[' || c == ']');
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
