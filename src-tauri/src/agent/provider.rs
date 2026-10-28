// mercury4win/src-tauri/agent/provider.rs
// OpenAI-compatible LLM client with SSE streaming

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::ipc::Channel;

mod sse;

#[cfg(test)]
tokio::task_local! {
    // Opt-in synthetic paid evaluations persist before/after every request.
    pub(crate) static TEST_AUDIT: std::sync::Arc<dyn Fn(serde_json::Value) -> Result<(), String> + Send + Sync>;
    pub(crate) static TEST_REPLAY: std::sync::Arc<dyn Fn(&ChatCompletionRequest) -> Result<Option<ChatCompletionResponse>,String> + Send + Sync>;
}

#[cfg(test)]
#[path = "m4w_017_tests.rs"]
mod m4w_017_tests;

#[derive(Debug, Clone, Serialize)]
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
    #[serde(skip)]
    pub route: Option<RequestRoute>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UsageInfo {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
    #[serde(skip)]
    pub route: Option<RequestRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRoute {
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub model_name: String,
}

fn request_route(fallback: bool) -> Option<RequestRoute> {
    crate::agent::configuration::CONFIGURATION
        .try_with(|c| {
            c.routes.get(usize::from(fallback)).map(|r| RequestRoute {
                provider_id: r.provider_id.clone(),
                provider_name: r.provider_name.clone(),
                model_id: r.model_id.clone(),
                model_name: r.model_name.clone(),
            })
        })
        .ok()
        .flatten()
}

/// A bounded diagnostic label; never includes response bodies or credentials.
pub(crate) fn failure_class(error: &str) -> String {
    if let Some(status) = error
        .strip_prefix("API error (HTTP ")
        .and_then(|s| s.split(')').next())
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|s| (100..=599).contains(s))
    {
        return format!("http_{status}");
    }
    match error {
        "HTTP request failed" => "transport",
        "HTTP response body failed" => "response_transport",
        "Invalid completion JSON" => "completion_json",
        "Invalid completion response" => "completion_fields",
        "Provider returned empty content" => "empty_completion",
        s if s.starts_with("Timeout:") => "timeout",
        s if s.starts_with("Provider did not finish completion:") => "unfinished_completion",
        _ => "provider_failure",
    }
    .into()
}

/// Non-streaming chat completion.
async fn chat_completion_once(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request: &ChatCompletionRequest,
    fallback: bool,
) -> Result<ChatCompletionResponse, String> {
    #[cfg(test)]
    if let Ok(replayed) = TEST_REPLAY.try_with(|replay| replay(request)) {
        if let Some(response) = replayed? {
            return Ok(response);
        }
    }
    #[cfg(test)]
    if let Ok(result) = TEST_AUDIT.try_with(|audit| audit(serde_json::json!({"event":"started","phase":crate::agent::usage::PHASE.try_with(|p|*p).unwrap_or("unknown"),"request":request}))) { result?; }
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let outbound = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(Duration::from_secs(120))
        .json(request)
        .build()
        .map_err(|_| "Invalid HTTP request".to_string())?;
    let mut guard = crate::agent::usage::RequestGuard::start(
        request_route(fallback),
        base_url,
        &request.model,
        fallback,
    );
    let result = async {
        let response = client.execute(outbound).await.map_err(|e| {
            if e.is_timeout() {
                "Timeout: HTTP request".to_string()
            } else {
                "HTTP request failed".to_string()
            }
        })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.map_err(|e| {
                if e.is_timeout() {
                    "Timeout: response body".to_string()
                } else {
                    "HTTP response body failed".to_string()
                }
            })?;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(tokens) = value.get("usage") {
                    guard.tokens(tokens);
                }
            }
            let message = if api_key.is_empty() {
                body
            } else {
                body.replace(api_key, "[redacted]")
            };
            return Err(format!("API error (HTTP {status}): {message}"));
        }

        let value: serde_json::Value = response.json().await.map_err(|e| {
            if e.is_timeout() {
                "Timeout: response body".to_string()
            } else {
                "Invalid completion JSON".to_string()
            }
        })?;
        if let Some(tokens) = value.get("usage").filter(|v| !v.is_null()) {
            guard.tokens(tokens);
        }
        let response: ChatCompletionResponse =
            serde_json::from_value(value).map_err(|_| "Invalid completion response".to_string())?;
        validate_completion(response)
    }
    .await;
    guard.finish(&result).await;
    #[cfg(test)]
    if let Ok(saved) = TEST_AUDIT.try_with(|audit| audit(match &result {
        Ok(response) => serde_json::json!({"event":"finished","response":response.choices[0].message.content,"usage":response.usage.as_ref().map(|u|serde_json::json!({"prompt_tokens":u.prompt_tokens,"completion_tokens":u.completion_tokens,"total_tokens":u.total_tokens}))}),
        Err(error) => serde_json::json!({"event":"failed","failure_class":failure_class(error),"usage":"see usage ledger; unknown if provider did not return usage"})
    })) { saved?; }
    result
}

fn validate_completion(response: ChatCompletionResponse) -> Result<ChatCompletionResponse, String> {
    if response
        .choices
        .first()
        .is_none_or(|c| c.message.content.trim().is_empty())
    {
        return Err("Provider returned empty content".to_string());
    }
    if let Some(reason) = response
        .choices
        .first()
        .and_then(|c| c.finish_reason.as_deref())
    {
        if reason != "stop" {
            return Err(format!("Provider did not finish completion: {reason}"));
        }
    }
    Ok(response)
}

/// Requests fail over only inside the active execution future. Dropping that
/// future on cancellation never reaches the fallback branch.
pub async fn chat_completion(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse, String> {
    match chat_completion_once(client, base_url, api_key, request, false).await {
        Ok(mut value) => {
            value.route = request_route(false);
            Ok(value)
        }
        Err(primary) => {
            let Some(route) = crate::agent::configuration::fallback() else {
                return Err(primary);
            };
            let mut request = request.clone();
            request.model = route.model_name;
            chat_completion_once(client, &route.base_url, &route.api_key, &request, true)
                .await
                .map(|mut value| {
                    value.route = request_route(true);
                    value
                })
                .map_err(|fallback| {
                    format!(
                        "Primary request failed: {primary}; fallback request failed: {fallback}"
                    )
                })
        }
    }
}

/// Streaming completion. Sends JSON chunk events; only a complete [DONE] succeeds.
pub async fn chat_completion_stream(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request: ChatCompletionRequest,
    on_event: Channel<String>,
) -> Result<UsageInfo, String> {
    chat_completion_stream_with_content(client, base_url, api_key, request, on_event)
        .await
        .map(|(usage, _)| usage)
}

pub async fn chat_completion_stream_with_content(
    client: &Client,
    base_url: &str,
    api_key: &str,
    request: ChatCompletionRequest,
    on_event: Channel<String>,
) -> Result<(UsageInfo, String), String> {
    let first = chat_completion_stream_once(
        client,
        base_url,
        api_key,
        request.clone(),
        on_event.clone(),
        false,
    )
    .await
    .and_then(|value| {
        if value.1.trim().is_empty() {
            Err("Empty summary response".into())
        } else {
            Ok(value)
        }
    });
    match first {
        Ok(mut value) => {
            value.0.route = request_route(false);
            Ok(value)
        }
        Err(primary) => {
            if primary.starts_with("Channel error:") {
                return Err(primary);
            }
            let Some(route) = crate::agent::configuration::fallback() else {
                return Err(primary);
            };
            on_event.send(serde_json::json!({"type":"reset","reason":"fallback","provider_id":route.provider_id,"model_name":route.model_name}).to_string())
                .map_err(|e| format!("Channel error: {e}"))?;
            let mut request = request;
            request.model = route.model_name;
            chat_completion_stream_once(
                client,
                &route.base_url,
                &route.api_key,
                request,
                on_event,
                true,
            )
            .await
            .and_then(|value| {
                if value.1.trim().is_empty() {
                    Err("Empty fallback summary response".into())
                } else {
                    Ok(value)
                }
            })
            .map(|mut value| {
                value.0.route = request_route(true);
                value
            })
            .map_err(|fallback| {
                format!("Primary request failed: {primary}; fallback request failed: {fallback}")
            })
        }
    }
}

async fn chat_completion_stream_once(
    client: &Client,
    base_url: &str,
    api_key: &str,
    mut request: ChatCompletionRequest,
    on_event: Channel<String>,
    fallback: bool,
) -> Result<(UsageInfo, String), String> {
    request.stream = Some(true);

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let mut body = serde_json::to_value(&request).map_err(|_| "Invalid request".to_string())?;
    body["stream_options"] = serde_json::json!({"include_usage":true});
    let outbound = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .timeout(Duration::from_secs(300))
        .json(&body)
        .build()
        .map_err(|_| "Invalid HTTP request".to_string())?;
    let mut guard = crate::agent::usage::RequestGuard::start(
        request_route(fallback),
        base_url,
        &request.model,
        fallback,
    );
    let result = async {
        let response = client.execute(outbound).await.map_err(|e| {
            if e.is_timeout() {
                "Timeout: HTTP request".to_string()
            } else {
                "HTTP request failed".to_string()
            }
        })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.map_err(|e| {
                if e.is_timeout() {
                    "Timeout: response body".to_string()
                } else {
                    "HTTP response body failed".to_string()
                }
            })?;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(tokens) = value.get("usage") {
                    guard.tokens(tokens);
                }
            }
            let message = if api_key.is_empty() {
                body
            } else {
                body.replace(api_key, "[redacted]")
            };
            return Err(format!("API error (HTTP {status}): {message}"));
        }

        let mut stream = response.bytes_stream();
        let mut decoder = sse::SseDecoder::default();
        let mut content = String::new();
        let mut usage = UsageInfo {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            route: None,
        };
        use futures_util::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                if e.is_timeout() {
                    "Timeout: response stream".to_string()
                } else {
                    "Stream error: transport failure".to_string()
                }
            })?;
            if decoder.push(&chunk, |data| {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(tokens) = value.get("usage").filter(|v| !v.is_null()) {
                        guard.tokens(tokens);
                    }
                }
                process_stream_data_collect(data, &mut usage, &on_event, &mut content)
            })? {
                return if content.trim().is_empty() {
                    Err("Empty summary response".into())
                } else {
                    Ok((usage, content))
                };
            }
        }

        Err(decoder.eof_error())
    }
    .await;
    guard.finish(&result).await;
    result
}

#[cfg(test)]
fn process_stream_data(
    data: &str,
    usage: &mut UsageInfo,
    on_event: &Channel<String>,
) -> Result<bool, String> {
    process_stream_data_collect(data, usage, on_event, &mut String::new())
}

fn process_stream_data_collect(
    data: &str,
    usage: &mut UsageInfo,
    on_event: &Channel<String>,
    collected: &mut String,
) -> Result<bool, String> {
    if data == "[DONE]" {
        return Ok(true);
    }
    let sse: serde_json::Value =
        serde_json::from_str(data).map_err(|e| format!("Invalid SSE JSON: {e}"))?;
    if let Some(error) = sse.get("error").filter(|v| !v.is_null()) {
        let message = error
            .get("message")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| error.to_string());
        return Err(format!("API stream error: {message}"));
    }
    if let Some(u) = sse.get("usage").filter(|v| !v.is_null()) {
        usage.prompt_tokens = u
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(usage.prompt_tokens);
        usage.completion_tokens = u
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(usage.completion_tokens);
        usage.total_tokens = u
            .get("total_tokens")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(usage.total_tokens);
    }
    if let Some(choices) = sse.get("choices").and_then(|v| v.as_array()) {
        for choice in choices {
            if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
                if reason != "stop" {
                    return Err(format!("Provider did not finish completion: {reason}"));
                }
            }
            if let Some(content) = choice
                .get("delta")
                .and_then(|d| d.get("content"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                collected.push_str(content);
                on_event
                    .send(serde_json::json!({"type":"chunk", "content": content}).to_string())
                    .map_err(|e| format!("Channel error: {e}"))?;
            }
        }
    } else if sse.get("usage").filter(|v| !v.is_null()).is_none() {
        return Err("Invalid SSE completion event: expected choices or usage".into());
    }
    Ok(false)
}

pub(super) fn summary_error_event(message: &str) -> String {
    serde_json::json!({"type":"error", "message":message}).to_string()
}

#[cfg(test)]
pub(super) fn translation_error_event(segment: usize, error: &str) -> String {
    serde_json::json!({"type":"error", "segment":segment, "error":error}).to_string()
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
