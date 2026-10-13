//! Request facts outlive result cancellation fences; no HTTP work is detached.
use crate::agent::provider::RequestRoute;
use crate::db::usage_store::{self, RequestEvent};
use deadpool_sqlite::Pool;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone)]
pub struct Context {
    pool: Pool,
    entry_id: i64,
    agent: String,
}
tokio::task_local! { static CONTEXT: Context; pub static PHASE: &'static str; }
static PENDING: AtomicU64 = AtomicU64::new(0);
pub static WRITE_FAILURES: AtomicU64 = AtomicU64::new(0);

pub async fn scope<T>(
    pool: &Pool,
    entry_id: i64,
    agent: &str,
    future: impl Future<Output = T>,
) -> T {
    CONTEXT
        .scope(
            Context {
                pool: pool.clone(),
                entry_id,
                agent: agent.into(),
            },
            future,
        )
        .await
}

pub struct RequestGuard {
    pool: Option<Pool>,
    event: Option<RequestEvent>,
}
impl RequestGuard {
    pub fn start(route: Option<RequestRoute>, base_url: &str, model: &str, fallback: bool) -> Self {
        let Some(context) = CONTEXT.try_with(Clone::clone).ok() else {
            return Self {
                pool: None,
                event: None,
            };
        };
        let endpoint = usage_store::sanitized_endpoint(base_url);
        let phase = if fallback {
            "fallback"
        } else {
            PHASE.try_with(|p| *p).unwrap_or("normal")
        };
        Self {
            pool: Some(context.pool),
            event: Some(RequestEvent {
                event_key: uuid::Uuid::new_v4().to_string(),
                entry_id: context.entry_id,
                provider_id: route.as_ref().map(|r| r.provider_id.clone()),
                model_id: route.as_ref().map(|r| r.model_id.clone()),
                provider_name: route.as_ref().map(|r| r.provider_name.clone()),
                model_name: model.into(),
                agent_type: context.agent,
                endpoint,
                request_phase: phase.into(),
                request_status: "cancelled".into(),
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
                started_at: chrono::Utc::now().to_rfc3339(),
                finished_at: None,
            }),
        }
    }
    pub fn tokens(&mut self, value: &serde_json::Value) {
        if let Some(event) = &mut self.event {
            let token = |key| value.get(key).and_then(|v| v.as_i64()).filter(|v| *v >= 0);
            if value.get("prompt_tokens").is_some() {
                event.prompt_tokens = token("prompt_tokens");
            }
            if value.get("completion_tokens").is_some() {
                event.completion_tokens = token("completion_tokens");
            }
            if value.get("total_tokens").is_some() {
                event.total_tokens = token("total_tokens");
            }
        }
    }
    pub async fn finish<T>(&mut self, result: &Result<T, String>) {
        if let (Some(pool), Some(event)) = (&self.pool, &mut self.event) {
            event.request_status = match result {
                Ok(_) => "succeeded",
                Err(error) if error.starts_with("Timeout:") => "timedOut",
                Err(_) => "failed",
            }
            .into();
            event.finished_at = Some(chrono::Utc::now().to_rfc3339());
            // Retain the event across this await: cancellation after HTTP end
            // must keep that HTTP outcome. event_key makes a dispatched insert
            // and Drop's insert idempotent, independent of CommitFence.
            let write = dispatch(pool.clone(), event.clone());
            if let Err(error) = write.await {
                WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
                log::error!("Usage writer task failed: {error}");
            }
            self.event = None;
        }
    }
}
async fn record(pool: &Pool, event: &RequestEvent) {
    if let Err(error) = usage_store::insert_request_event(pool, event).await {
        WRITE_FAILURES.fetch_add(1, Ordering::Relaxed);
        log::error!(
            "Usage event persistence failed ({}): {error}",
            event.event_key
        );
    }
}
// Poll writes independently: buffer_unordered may suspend another request
// while checkpoint reconciliation also needs the single SQLite pool lease.
// Keeping the lease in an independently polled DB task avoids that deadlock.
fn dispatch(pool: Pool, event: RequestEvent) -> tokio::task::JoinHandle<()> {
    struct Pending;
    impl Drop for Pending {
        fn drop(&mut self) {
            PENDING.fetch_sub(1, Ordering::SeqCst);
        }
    }
    PENDING.fetch_add(1, Ordering::SeqCst);
    // Capture the decrement guard before spawning: runtime shutdown can drop
    // this future before its first poll.
    let pending = Pending;
    tokio::spawn(async move {
        let _pending = pending;
        record(&pool, &event).await;
    })
}
impl Drop for RequestGuard {
    fn drop(&mut self) {
        if let (Some(pool), Some(mut event)) = (self.pool.take(), self.event.take()) {
            event
                .finished_at
                .get_or_insert_with(|| chrono::Utc::now().to_rfc3339());
            dispatch(pool, event);
        }
    }
}
/// Wait for already-dispatched cancellation writes. Does not retain sockets,
/// acquire runtime locks, or wait for active HTTP requests.
#[cfg(test)]
pub async fn flush() {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while PENDING.load(Ordering::SeqCst) != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("usage write drain timeout");
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
