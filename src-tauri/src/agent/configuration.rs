//! A single immutable execution snapshot. Credentials never enter the identity.
use crate::agent::prompt_templates::{self, PromptTemplate};
use crate::db::{agent_store, models::AgentModelProfile};
use deadpool_sqlite::Pool;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone, Serialize)]
pub struct Route {
    pub provider_id: String,
    pub provider_name: String,
    pub base_url: String,
    pub model_id: String,
    pub model_name: String,
    #[serde(skip)]
    pub api_key: String,
}
#[derive(Clone)]
pub struct Configuration {
    pub template: PromptTemplate,
    pub routes: Vec<Route>,
    pub fingerprint: String,
    pub notice: Option<String>,
    pub prompt_strategy: Option<String>,
    pub concurrency: usize,
}
tokio::task_local! { pub static CONFIGURATION: Configuration; }

#[derive(Serialize)]
pub struct RequestParameters {
    pub temperature: f64,
    pub max_tokens: u32,
}
pub fn request_parameters(agent: &str) -> RequestParameters {
    let (temperature, max_tokens) = match agent {
        "summary" => (0.5, 1024),
        "translation" => (0.3, 2048),
        _ => (0.3, 512),
    };
    RequestParameters {
        temperature,
        max_tokens,
    }
}

pub fn template(agent: &str, strategy: Option<&str>) -> Result<PromptTemplate, String> {
    CONFIGURATION
        .try_with(|c| c.template.clone())
        .or_else(|_| prompt_templates::load_builtin_template(agent, strategy))
}
pub fn fingerprint_parameters(parameters: &str) -> String {
    CONFIGURATION
        .try_with(|c| {
            serde_json::json!({"parameters":parameters,"configuration":c.fingerprint}).to_string()
        })
        .unwrap_or_else(|_| parameters.into())
}
pub fn fallback() -> Option<Route> {
    CONFIGURATION
        .try_with(|c| c.routes.get(1).cloned())
        .ok()
        .flatten()
}
pub fn supports(model: &AgentModelProfile, agent: &str) -> bool {
    match agent {
        "summary" => model.supports_summary,
        "translation" => model.supports_translation,
        "tagging" => model.supports_tagging,
        _ => false,
    }
}
pub async fn load(pool: &Pool, prompts: &Path, agent: &str) -> Result<Configuration, String> {
    let profile = agent_store::get_agent_profile(pool, agent).await?;
    let strategy = profile.as_ref().and_then(|p| p.prompt_strategy.as_deref());
    let (template, notice) = prompt_templates::resolve_template(prompts, agent, strategy)?;
    let mut available = Vec::new();
    for provider in agent_store::list_providers(pool).await? {
        if !provider.is_enabled {
            continue;
        }
        for model in agent_store::list_models(pool, &provider.id).await? {
            if model.is_enabled && supports(&model, agent) {
                available.push(Route {
                    provider_id: provider.id.clone(),
                    provider_name: provider
                        .display_name
                        .clone()
                        .unwrap_or_else(|| provider.name.clone()),
                    base_url: provider.base_url.clone(),
                    model_id: model.id,
                    model_name: model.model_name,
                    api_key: String::new(),
                });
            }
        }
    }
    let mut routes = Vec::new();
    if let Some(id) = profile.as_ref().and_then(|p| p.primary_model_id.as_ref()) {
        routes.push(
            available
                .iter()
                .find(|r| &r.model_id == id)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "Configured primary model is unavailable or does not support {agent}: {id}"
                    )
                })?,
        );
    } else if let Some(route) = available.first() {
        routes.push(route.clone());
    }
    if let Some(id) = profile.as_ref().and_then(|p| p.fallback_model_id.as_ref()) {
        let route = available
            .iter()
            .find(|r| &r.model_id == id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Configured fallback model is unavailable or does not support {agent}: {id}"
                )
            })?;
        if routes.first().is_none_or(|r| r.model_id != route.model_id) {
            routes.push(route);
        }
    }
    // Empty routes are a valid read-only identity; execution reports an error.
    let parameters = request_parameters(agent);
    let identity = serde_json::json!([
        "agent-configuration-v1",
        agent,
        template,
        routes,
        parameters
    ]);
    let fingerprint = format!("{:x}", Sha256::digest(identity.to_string()));
    let prompt_strategy = profile.as_ref().and_then(|p| p.prompt_strategy.clone());
    let concurrency = crate::db::agent_result_store::normalize_translation_concurrency(
        profile
            .as_ref()
            .and_then(|p| p.concurrency_degree)
            .filter(|c| *c > 0)
            .unwrap_or(3) as usize,
    );
    Ok(Configuration {
        template,
        routes,
        fingerprint,
        notice,
        prompt_strategy,
        concurrency,
    })
}

#[cfg(test)]
#[path = "configuration_tests.rs"]
mod tests;
