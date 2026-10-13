//! One outer tagging permit, bounded concurrent per-entry calls, durable review.
use crate::agent::configuration::{Configuration, CONFIGURATION};
use crate::db::tag_batch_store::{self, BatchScope, BatchSnapshot};
use deadpool_sqlite::Pool;
use futures_util::{stream, StreamExt};
use tauri::ipc::Channel;

pub async fn generate(
    pool: &Pool,
    client: &reqwest::Client,
    id: &str,
    scope: BatchScope,
    config: Configuration,
    confirmed_large: bool,
    on_event: &Channel<String>,
) -> Result<BatchSnapshot, String> {
    let route = config
        .routes
        .first()
        .cloned()
        .ok_or("No configured tagging route")?;
    let concurrency = config.concurrency.clamp(1, 5);
    let ids = tag_batch_store::start(pool, id, scope, &config.fingerprint, confirmed_large).await?;
    let total = ids.len();
    let mut processed = 0;
    let mut failed = 0;
    let work = ids.into_iter().map(|entry| {
        let config = config.clone();
        let route = route.clone();
        async move {
            CONFIGURATION
                .scope(config, async {
                    let response = crate::agent::tagging::run_tagging_with_source(
                        pool,
                        client,
                        entry,
                        &route.base_url,
                        &route.api_key,
                        &route.model_name,
                    )
                    .await;
                    let (hash, result) = match response {
                        Ok((hash, names)) => (Some(hash), Ok(names)),
                        Err(error) => (None, Err(error)),
                    };
                    let success = result.is_ok();
                    tag_batch_store::stage(pool, id, entry, hash, result).await?;
                    Ok::<_, String>(success)
                })
                .await
        }
    });
    let mut pending = stream::iter(work).buffer_unordered(concurrency);
    while let Some(result) = pending.next().await {
        if !result? {
            failed += 1;
        }
        processed += 1;
        let _=on_event.send(serde_json::json!({"type":"batch_progress","request_id":id,"processed":processed,"total":total,"failed":failed}).to_string());
    }
    tag_batch_store::finish(pool, id).await?;
    tag_batch_store::load(pool, Some(id.into()))
        .await?
        .ok_or("Batch missing".into())
}
#[cfg(test)]
#[path = "tag_batch_tests.rs"]
pub(crate) mod tests;
