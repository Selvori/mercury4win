//! Per-source-block translation with bounded requests and durable recovery.
use futures_util::StreamExt;
use reqwest::Client;
use std::collections::HashMap;
use tauri::ipc::Channel;

use crate::agent::{
    provider::{self, ChatCompletionRequest, ChatMessage},
    translation_segments,
};
use crate::db::agent_result_store::{self, AgentResult, ResultSlot};

pub async fn run_translation(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    entry_id: i64,
    target_language: &str,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
    prompt_strategy: Option<&str>,
    concurrency: usize,
    on_event: Channel<String>,
) -> Result<(), String> {
    run_translation_segments(
        pool,
        client,
        entry_id,
        target_language,
        provider_base_url,
        api_key,
        model_name,
        prompt_strategy,
        concurrency,
        None,
        on_event,
    )
    .await
}

pub async fn run_translation_segments(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    entry_id: i64,
    target_language: &str,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
    prompt_strategy: Option<&str>,
    concurrency: usize,
    retry_segment_id: Option<&str>,
    on_event: Channel<String>,
) -> Result<(), String> {
    crate::agent::usage::scope(
        pool,
        entry_id,
        "translation",
        run_translation_segments_inner(
            pool,
            client,
            entry_id,
            target_language,
            provider_base_url,
            api_key,
            model_name,
            prompt_strategy,
            concurrency,
            retry_segment_id,
            on_event,
        ),
    )
    .await
}

async fn run_translation_segments_inner(
    pool: &deadpool_sqlite::Pool,
    client: &Client,
    entry_id: i64,
    target_language: &str,
    provider_base_url: &str,
    api_key: &str,
    model_name: &str,
    prompt_strategy: Option<&str>,
    concurrency: usize,
    retry_segment_id: Option<&str>,
    on_event: Channel<String>,
) -> Result<(), String> {
    let document = translation_segments::load_document(
        pool,
        entry_id,
        target_language,
        prompt_strategy,
        concurrency,
    )
    .await?;
    let segments = document.source_segments;
    if segments.is_empty() {
        return Err("No translatable content found".into());
    }
    let total = segments.len();
    let ids = segments
        .iter()
        .map(|s| s.source_segment_id.clone())
        .collect::<Vec<_>>();
    let selected = retry_segment_id
        .map(|id| {
            ids.iter()
                .position(|s| s == id)
                .ok_or_else(|| "Translation source segment changed or does not exist".to_string())
        })
        .transpose()?;
    let slot = ResultSlot::new(
        entry_id,
        "translation",
        target_language,
        &agent_result_store::translation_parameters(prompt_strategy, concurrency),
        &document.source_hash,
    );
    let mut saved = AgentResult {
        request_route: None,
        segment_routes: vec![None; total],
        source_hash: document.source_hash,
        text: None,
        segments: vec![None; total],
        complete: false,
        model_name: model_name.into(),
        segment_ids: ids,
        segment_errors: vec![None; total],
    };
    // Manual article regeneration starts afresh after a complete success; a retry
    // merges into the current slot. A checkpoint resumes only missing blocks.
    if let Some(cached) = document
        .result
        .filter(|c| !c.complete || selected.is_some())
    {
        saved = cached;
        saved.segment_errors.resize(total, None);
        saved.model_name = model_name.into();
    }
    if let Some(i) = selected {
        if saved.segments[i].is_some() {
            return Err("Only missing translation segments can be retried".into());
        }
    }
    saved.complete = false;
    saved.segment_routes.resize(total, None);
    let _ = on_event.send(serde_json::json!({"type":"source_segments","source_hash":saved.source_hash,"segments":segments,"html":document.html}).to_string());
    for (i, text) in saved.segments.iter().enumerate() {
        if let Some(text) = text {
            emit_segment(&on_event, &saved, i, text);
        }
    }
    let template = crate::agent::configuration::template("translation", prompt_strategy)?;
    let lang = match target_language {
        "zh-Hans" => "Simplified Chinese",
        "zh-Hant" => "Traditional Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "fr" => "French",
        "de" => "German",
        "es" => "Spanish",
        "en" => "English",
        other => other,
    };
    let concurrency = agent_result_store::normalize_translation_concurrency(concurrency);
    let indices: Vec<_> = (0..total)
        .filter(|i| saved.segments[*i].is_none() && selected.is_none_or(|s| s == *i))
        .collect();
    let mut usage = [0i64; 3];
    let mut storage_errors = Vec::new();
    let mut completed = saved.segments.iter().filter(|s| s.is_some()).count();
    let request = |i: usize, previous: Option<(String, String)>| {
        let mut params = HashMap::new();
        params.insert("target_language".into(), lang.to_string());
        params.insert("source_text".into(), segments[i].source_text.clone());
        if let Some((source, translation)) = previous {
            params.insert("previous_source_text".into(), source);
            params.insert("previous_translated_text".into(), translation);
        }
        let rendered = template.render(&params);
        let mut messages = Vec::new();
        if let Some(system) = rendered.system_message {
            messages.push(ChatMessage {
                role: "system".into(),
                content: system,
            });
        }
        messages.push(ChatMessage {
            role: "user".into(),
            content: rendered.user_message,
        });
        let channel = &on_event;
        let segment_id = &segments[i].source_segment_id;
        async move {
            let _ = channel.send(serde_json::json!({"type":"segment_started","index":i,"source_segment_id":segment_id}).to_string());
            let response = crate::agent::usage::PHASE
                .scope(
                    if retry_segment_id.is_some() {
                        "retry"
                    } else {
                        "normal"
                    },
                    provider::chat_completion(
                        client,
                        provider_base_url,
                        api_key,
                        &ChatCompletionRequest {
                            model: model_name.into(),
                            messages,
                            temperature: Some(
                                crate::agent::configuration::request_parameters("translation")
                                    .temperature,
                            ),
                            top_p: None,
                            max_tokens: Some(
                                crate::agent::configuration::request_parameters("translation")
                                    .max_tokens,
                            ),
                            stream: None,
                        },
                    ),
                )
                .await;
            (
                i,
                response.and_then(|r| {
                    if let Some(reason) = r.choices.first().and_then(|c| c.finish_reason.as_deref())
                    {
                        if reason != "stop" {
                            return Err(format!("Provider did not finish translation: {reason}"));
                        }
                    }
                    let text = r
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_default();
                    if text.trim().is_empty() {
                        Err("Provider returned an empty translation".into())
                    } else {
                        Ok((
                            text,
                            Some(provider::UsageInfo {
                                route: r.route,
                                ..r.usage.unwrap_or_default()
                            }),
                        ))
                    }
                }),
            )
        }
    };
    // Serial and parallel share reconciliation; every request is drained before
    // a terminal outcome. Serial context uses only the preceding source block.
    if concurrency == 1 {
        for i in indices {
            let previous = i.checked_sub(1).and_then(|p| {
                saved.segments[p]
                    .as_ref()
                    .map(|t| (segments[p].source_text.clone(), t.clone()))
            });
            let (i, result) = request(i, previous).await;
            reconcile(
                pool,
                &slot,
                &mut saved,
                i,
                result,
                &on_event,
                &mut usage,
                &mut completed,
                &mut storage_errors,
            )
            .await;
        }
    } else {
        let mut stream = futures_util::stream::iter(indices.into_iter().map(|i| request(i, None)))
            .buffer_unordered(concurrency);
        while let Some((i, result)) = stream.next().await {
            reconcile(
                pool,
                &slot,
                &mut saved,
                i,
                result,
                &on_event,
                &mut usage,
                &mut completed,
                &mut storage_errors,
            )
            .await;
        }
    }
    let succeeded = saved.segments.iter().filter(|s| s.is_some()).count();
    let failed = total - succeeded;
    let status = if failed == 0 && storage_errors.is_empty() {
        "complete"
    } else if succeeded == 0 {
        "all_failed"
    } else {
        "partial"
    };
    if status == "complete" {
        saved.complete = true;
        slot.save(pool, &saved).await?;
    }
    let _ = on_event.send(serde_json::json!({"type":"outcome","status":status,"succeeded":succeeded,"failed":failed,"total":total}).to_string());
    if status != "complete" {
        let errors = saved
            .segment_errors
            .iter()
            .flatten()
            .cloned()
            .chain(storage_errors)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "Translation {status}: {succeeded}/{total} segments completed. {errors}"
        ));
    }
    let _ = on_event
        .send(serde_json::json!({"type":"done","result":"translation_complete"}).to_string());
    Ok(())
}

fn emit_segment(channel: &Channel<String>, saved: &AgentResult, i: usize, text: &str) {
    let _ = channel.send(serde_json::json!({"type":"segment","index":i,"source_segment_id":saved.segment_ids[i],"text":text}).to_string());
}

async fn reconcile(
    pool: &deadpool_sqlite::Pool,
    slot: &ResultSlot,
    saved: &mut AgentResult,
    i: usize,
    result: Result<(String, Option<provider::UsageInfo>), String>,
    channel: &Channel<String>,
    usage: &mut [i64; 3],
    completed: &mut usize,
    storage_errors: &mut Vec<String>,
) {
    match result {
        Ok((text, tokens)) => {
            if let Some(tokens) = tokens {
                if let Some(route) = &tokens.route {
                    saved.segment_routes[i] = Some(route.clone());
                    let unique_models: std::collections::HashSet<_> = saved
                        .segment_routes
                        .iter()
                        .flatten()
                        .map(|r| r.model_name.as_str())
                        .collect();
                    saved.model_name = if unique_models.len() > 1 {
                        "mixed".into()
                    } else {
                        route.model_name.clone()
                    };
                }
                usage[0] += tokens.prompt_tokens as i64;
                usage[1] += tokens.completion_tokens as i64;
                usage[2] += tokens.total_tokens as i64;
            }
            saved.segments[i] = Some(text.clone());
            saved.segment_errors[i] = None;
            match slot.save(pool, saved).await {
                Ok(()) => emit_segment(channel, saved, i, &text),
                Err(e) => storage_errors.push(e),
            }
        }
        Err(error) => {
            saved.segment_errors[i] = Some(error.clone());
            if let Err(e) = slot.save(pool, saved).await {
                storage_errors.push(e);
            }
            let _ = channel.send(serde_json::json!({"type":"error","segment":i,"index":i,"source_segment_id":saved.segment_ids[i],"error":error}).to_string());
        }
    }
    *completed += 1;
    let _ = channel.send(
        serde_json::json!({"type":"progress","segment":completed,"total":saved.segments.len()})
            .to_string(),
    );
}

pub fn compute_source_content_hash(segments: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(segments).unwrap())
    )
}
