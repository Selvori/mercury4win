// mercury4win-linux/src-tauri/agent/provider.rs
// OpenAI-compatible LLM client with SSE streaming

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use std::time::Duration;

// ── Request types ──

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// ── Response types ──

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct SseChunk {
    choices: Option<Vec<SseChoice>>,
}

#[derive(Debug, Deserialize)]
struct SseChoice {
    delta: Option<SseDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseDelta {
    content: Option<String>,
}

/// Non-streaming chat completion.
pub async fn chat_completion(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(120))
        .json(request)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error: {}", body));
    }

    response
        .json::<ChatCompletionResponse>()
        .await
        .map_err(|e| format!("Parse error: {}", e))
}

/// Streaming chat completion. Sends each content delta through the Tauri Channel.
pub async fn chat_completion_stream(
    client: &Client,
    base_url: &str,
    api_key: &str,
    mut request: ChatCompletionRequest,
    on_event: Channel<String>,
) -> Result<UsageInfo, String> {
    request.stream = Some(true);

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .timeout(Duration::from_secs(300))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("API error: {}", body));
    }

    let mut stream = response.bytes_stream();
    let mut prompt_tokens = 0u32;
    let mut completion_tokens = 0u32;
    let mut total_tokens = 0u32;

    use futures_util::StreamExt;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
        let text = String::from_utf8_lossy(&chunk);

        for line in text.lines() {
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return Ok(UsageInfo {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                    });
                }
                if let Ok(sse) = serde_json::from_str::<SseChunk>(data) {
                    if let Some(choices) = &sse.choices {
                        for choice in choices {
                            if let Some(ref delta) = choice.delta {
                                if let Some(ref content) = delta.content {
                                    let _ = on_event.send(content.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(UsageInfo {
        prompt_tokens,
        completion_tokens,
        total_tokens,
    })
}

/// Test a provider connection with a simple chat request.
pub async fn test_connection(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model_name: &str,
) -> Result<String, String> {
    let request = ChatCompletionRequest {
        model: model_name.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello, respond with just 'ok'.".to_string(),
        }],
        temperature: None,
        top_p: None,
        max_tokens: Some(10),
        stream: None,
    };

    let response = chat_completion(client, base_url, api_key, &request).await?;
    let reply = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "No response".to_string());

    Ok(reply)
}
