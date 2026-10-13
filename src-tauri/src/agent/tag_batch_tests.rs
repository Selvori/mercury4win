use super::*;
use crate::agent::{configuration, runtime};
use crate::db::{tag_batch_store as store, tag_store};
use std::sync::{Arc, Mutex};
use tauri::ipc::InvokeResponseBody;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
pub(crate) async fn fixture() -> Pool {
    let pool = deadpool_sqlite::Config::new(":memory:")
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    pool.get().await.unwrap().interact(|conn|{conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();crate::db::migrations::run_migrations(conn).unwrap();conn.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test'); INSERT INTO entry(id,feed_id,guid,title,summary,published_at) VALUES(1,1,'1','Intel CPUs','test summary','2026-10-11T00:00:00Z'),(2,1,'2','Rust SQLite','test summary','2026-10-12T00:00:00Z'),(3,1,'3','Intel Rust','test summary','2026-10-13T00:00:00Z')").unwrap();}).await.unwrap();
    pool
}
fn scope() -> BatchScope {
    BatchScope {
        from: None,
        to: None,
        unread_only: false,
        skip_tagged: false,
        skip_applied: false,
    }
}
fn channel() -> (Channel<String>, Arc<Mutex<Vec<serde_json::Value>>>) {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let output = messages.clone();
    (
        Channel::new(move |body| {
            let InvokeResponseBody::Json(json) = body else {
                panic!()
            };
            let json: String = serde_json::from_str(&json).unwrap();
            output
                .lock()
                .unwrap()
                .push(serde_json::from_str(&json).unwrap());
            Ok(())
        }),
        messages,
    )
}
async fn staged(pool: &Pool, id: &str) {
    store::start(pool, id, scope(), "frozen", true)
        .await
        .unwrap();
    for entry in 1..=3 {
        let hash = store::hash(pool, entry).await.unwrap();
        store::stage(
            pool,
            id,
            entry,
            hash,
            Ok(vec!["Edge-AI".into(), "edge_ai".into()]),
        )
        .await
        .unwrap();
    }
    store::finish(pool, id).await.unwrap();
}
#[tokio::test]
async fn staging_review_atomic_apply_and_idempotence() {
    let pool = fixture().await;
    staged(&pool, "review").await;
    assert!(tag_store::list_tags(&pool, None).await.unwrap().is_empty());
    assert!(tag_store::rename_tag(&pool, 1, "bad").await.is_err());
    assert!(store::apply(&pool, "review", vec![], vec![]).await.is_err());
    assert!(tag_store::list_tags(&pool, None).await.unwrap().is_empty());
    let result = store::apply(&pool, "review", vec!["edge ai".into()], vec![])
        .await
        .unwrap();
    assert_eq!(result.tags_assigned, 3);
    assert_eq!(result.entries_processed, 3);
    assert_eq!(tag_store::list_tags(&pool, None).await.unwrap().len(), 1);
    assert!(!tag_store::list_tags(&pool, None).await.unwrap()[0].is_provisional);
    assert_eq!(
        store::apply(&pool, "review", vec![], vec![])
            .await
            .unwrap()
            .tags_assigned,
        3
    );
    let mut skip = scope();
    skip.skip_applied = true;
    assert_eq!(store::count(&pool, skip).await.unwrap(), 0);
}
#[tokio::test]
async fn apply_discards_and_skips_deleted_or_changed_sources() {
    let pool = fixture().await;
    staged(&pool, "stale").await;
    pool.get().await.unwrap().interact(|c|c.execute_batch("UPDATE entry SET summary='changed' WHERE id=1;UPDATE entry SET is_deleted=1 WHERE id=2").unwrap()).await.unwrap();
    let result = store::apply(&pool, "stale", vec![], vec!["edge ai".into()])
        .await
        .unwrap();
    assert_eq!(result.skipped_deleted, 1);
    assert_eq!(result.skipped_changed, 1);
    assert_eq!(result.tags_assigned, 0);
    assert!(tag_store::list_tags(&pool, None).await.unwrap().is_empty());
}
#[tokio::test]
async fn apply_requires_current_review_and_resolves_manual_alias() {
    let pool = fixture().await;
    staged(&pool, "alias").await;
    let tag = tag_store::add_tag(&pool, 3, "AI").await.unwrap();
    // Existing canonical suggestion changed during review via allowed manual input.
    let snapshot = store::load(&pool, Some("alias".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.proposals[0].normalized_name, "edge ai");
    // Direct SQL represents a stale externally altered library; final resolver must cope.
    pool.get()
        .await
        .unwrap()
        .interact(move |c| {
            tag_store::attach_alias(c, tag.id, "Edge AI").unwrap();
        })
        .await
        .unwrap();
    assert!(store::apply(&pool, "alias", vec!["edge ai".into()], vec![])
        .await
        .is_err());
    let result = store::apply(&pool, "alias", vec![], vec![]).await.unwrap();
    assert_eq!(result.tags_assigned, 2);
    assert_eq!(tag_store::list_tags(&pool, None).await.unwrap().len(), 1);
}
#[tokio::test]
async fn scope_is_shared_frozen_and_maintenance_locked_until_discard() {
    let pool = fixture().await;
    let mut selected = scope();
    selected.from = Some("2026-10-12T00:00:00Z".into());
    selected.to = Some("2026-10-13T00:00:00Z".into());
    selected.unread_only = true;
    selected.skip_tagged = true;
    tag_store::add_tag(&pool, 2, "Rust").await.unwrap();
    assert_eq!(store::count(&pool, selected.clone()).await.unwrap(), 1);
    assert_eq!(
        store::start(&pool, "scope", selected, "frozen", false)
            .await
            .unwrap(),
        vec![3]
    );
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute("UPDATE entry SET is_read=1 WHERE id=3", [])
                .unwrap()
        })
        .await
        .unwrap();
    assert_eq!(
        store::load(&pool, Some("scope".into()))
            .await
            .unwrap()
            .unwrap()
            .total,
        1
    );
    assert!(tag_store::delete_tag(&pool, 1).await.is_err());
    assert!(store::discard(&pool, "scope").await.is_err());
    store::finish(&pool, "scope").await.unwrap();
    store::discard(&pool, "scope").await.unwrap();
    tag_store::delete_tag(&pool, 1).await.unwrap();
}
async fn config(url: &str) -> Configuration {
    Configuration {
        template: crate::agent::prompt_templates::load_builtin_template("tagging", None).unwrap(),
        routes: vec![configuration::Route {
            provider_id: "fixture".into(),
            provider_name: "fixture".into(),
            base_url: url.into(),
            model_id: "fixture".into(),
            model_name: "fixture".into(),
            api_key: String::new(),
        }],
        fingerprint: "test-frozen".into(),
        notice: None,
        prompt_strategy: None,
        concurrency: 1,
    }
}
#[tokio::test]
async fn production_batch_requests_progress_no_writes_and_cancel_stops_future_requests() {
    let pool = fixture().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sent, mut requests) = tokio::sync::mpsc::unbounded_channel();
    let (closed, mut disconnects) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let mut count = 0;
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            count += 1;
            let n = count;
            let sent = sent.clone();
            let closed = closed.clone();
            tokio::spawn(async move {
                let mut buffer = [0; 8192];
                socket.read(&mut buffer).await.unwrap();
                sent.send(n).unwrap();
                if n == 1 {
                    let body=serde_json::json!({"choices":[{"message":{"role":"assistant","content":"[\"Edge AI\"]"}}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}).to_string();
                    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
                } else {
                    let mut buf = [0; 1];
                    while socket.read(&mut buf).await.unwrap_or(0) > 0 {}
                    closed.send(()).unwrap();
                }
            });
        }
    });
    let state = runtime::create_runtime();
    let (events, messages) = channel();
    let cfg = config(&url).await;
    let running = state.clone();
    let db = pool.clone();
    let work = tokio::spawn(async move {
        let client = reqwest::Client::new();
        let result = runtime::execute(
            &running,
            runtime::TaskType::Tagging,
            0,
            "cancel-batch",
            &events,
            || generate(&db, &client, "cancel-batch", scope(), cfg, false, &events),
        )
        .await;
        store::finish(&db, "cancel-batch").await.unwrap();
        result
    });
    assert_eq!(requests.recv().await, Some(1));
    assert_eq!(requests.recv().await, Some(2));
    let live = crate::commands::tag_cmd::load_or_recover(&pool, &state, None)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        live.status, "running",
        "recovery must not interrupt an active batch"
    );
    state
        .lock()
        .unwrap()
        .cancel(runtime::TaskType::Tagging, "cancel-batch", false)
        .unwrap();
    assert!(work.await.unwrap().unwrap_err().contains("cancelled"));
    tokio::time::timeout(std::time::Duration::from_secs(3), disconnects.recv())
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), requests.recv())
            .await
            .is_err()
    );
    let snapshot = store::load(&pool, Some("cancel-batch".into()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.processed, 1);
    assert_eq!(snapshot.total, 3);
    assert_eq!(snapshot.status, "ready");
    assert!(tag_store::list_tags(&pool, None).await.unwrap().is_empty());
    assert!(messages
        .lock()
        .unwrap()
        .iter()
        .any(|e| e["type"] == "batch_progress" && e["processed"] == 1));
    assert_eq!(
        store::apply(&pool, "cancel-batch", vec!["edge ai".into()], vec![])
            .await
            .unwrap()
            .tags_assigned,
        1
    );
    let mut retry_scope = scope();
    retry_scope.skip_applied = true;
    assert_eq!(
        store::count(&pool, retry_scope).await.unwrap(),
        2,
        "unstarted cancelled items must remain retryable"
    );
    server.abort();
}

#[tokio::test]
async fn frozen_primary_backup_bounded_concurrency_and_per_item_failure_review() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let pool = fixture().await;
    let canonical = tag_store::add_tag(&pool, 1, "Machine Learning")
        .await
        .unwrap();
    tag_store::mutate(&pool, "alias", canonical.id, "ML")
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let (sink, inflight, highest) = (requests.clone(), active.clone(), peak.clone());
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (sink, inflight, highest) = (sink.clone(), inflight.clone(), highest.clone());
            tokio::spawn(async move {
                let mut bytes = Vec::new();
                let request;
                loop {
                    let mut buf = [0; 4096];
                    let n = socket.read(&mut buf).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(end) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                        let length = String::from_utf8_lossy(&bytes[..end])
                            .lines()
                            .find_map(|s| {
                                s.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|v| v.trim().parse::<usize>().ok())
                            })
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            request = serde_json::from_slice::<serde_json::Value>(
                                &bytes[end + 4..end + 4 + length],
                            )
                            .unwrap();
                            break;
                        }
                    }
                }
                sink.lock().unwrap().push(request.clone());
                let now = inflight.fetch_add(1, Ordering::SeqCst) + 1;
                highest.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                let primary = request["model"] == "primary";
                let failing_entry = request["messages"].to_string().contains("Rust SQLite");
                let (status, body) = if primary || failing_entry {
                    ("503 Error", "fixture failure".to_owned())
                } else {
                    ("200 OK",serde_json::json!({"choices":[{"message":{"role":"assistant","content":"[\"ML\",\"Edge_AI\",\"edge-ai\"]"}}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}}).to_string())
                };
                socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
                inflight.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });
    let mut cfg = config(&url).await;
    cfg.concurrency = 2;
    cfg.template.user = "FROZEN {{title}}".into();
    cfg.routes[0].model_name = "primary".into();
    let mut backup = cfg.routes[0].clone();
    backup.model_name = "backup".into();
    backup.provider_name = "Backup".into();
    cfg.routes.push(backup);
    let state = runtime::create_runtime();
    let (events, _) = channel();
    let client = reqwest::Client::new();
    let review = runtime::execute(
        &state,
        runtime::TaskType::Tagging,
        0,
        "routes",
        &events,
        || generate(&pool, &client, "routes", scope(), cfg, false, &events),
    )
    .await
    .unwrap();
    assert_eq!(review.processed, 3);
    assert_eq!(review.succeeded, 2);
    assert_eq!(review.failures.len(), 1);
    assert_eq!(review.proposals.len(), 1);
    assert_eq!(review.proposals[0].normalized_name, "edge ai");
    assert_eq!(tag_store::list_tags(&pool, None).await.unwrap().len(), 1);
    assert_eq!(
        tag_store::list_tags(&pool, None).await.unwrap()[0].usage_count,
        1
    );
    assert_eq!(peak.load(Ordering::SeqCst), 2);
    let calls = requests.lock().unwrap();
    assert_eq!(calls.len(), 10);
    assert!(calls
        .iter()
        .all(|r| r["messages"].to_string().contains("FROZEN")));
    assert_eq!(calls.iter().filter(|r| r["model"] == "backup").count(), 5);
    drop(calls);
    let usage=pool.get().await.unwrap().interact(|c|c.query_row("SELECT COUNT(*) FROM llm_usage_event WHERE provider_name='Backup' AND model_name='backup'",[],|r|r.get::<_,i64>(0)).unwrap()).await.unwrap();
    assert_eq!(usage, 5); // Request facts include failed fallback attempts too.
    let (total,succeeded,failed)=pool.get().await.unwrap().interact(|c|c.query_row("SELECT COUNT(*),SUM(request_status='succeeded'),SUM(request_status='failed') FROM llm_usage_event",[],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?))).unwrap()).await.unwrap();
    assert_eq!((total, succeeded, failed), (10, 2, 8));
    let applied = store::apply(&pool, "routes", vec!["edge ai".into()], vec![])
        .await
        .unwrap();
    assert_eq!(applied.tags_assigned, 3);
    server.abort();
}
#[tokio::test]
async fn durable_review_reopens_and_missing_pre_registration_batch_is_none() {
    let path = std::env::temp_dir().join(format!("m4w-010-{}.sqlite", uuid::Uuid::new_v4()));
    let make_pool = || {
        deadpool_sqlite::Config::new(&path)
            .builder(deadpool_sqlite::Runtime::Tokio1)
            .unwrap()
            .max_size(1)
            .build()
            .unwrap()
    };
    let pool = make_pool();
    pool.get().await.unwrap().interact(|c|{crate::db::migrations::run_migrations(c).unwrap();c.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test');INSERT INTO entry(id,feed_id,title) VALUES(1,1,'Durable')").unwrap();}).await.unwrap();
    let ids = store::start(&pool, "interrupted", scope(), "frozen", false)
        .await
        .unwrap();
    assert_eq!(ids, vec![1]);
    let hash = store::hash(&pool, 1).await.unwrap();
    store::stage(&pool, "interrupted", 1, hash, Ok(vec!["Durable".into()]))
        .await
        .unwrap();
    close_file_pool(pool).await;
    let reopened = make_pool();
    let snapshot = store::load(&reopened, None).await.unwrap().unwrap();
    assert_eq!(snapshot.status, "running");
    assert_eq!(snapshot.processed, 1);
    let ready =
        crate::commands::tag_cmd::load_or_recover(&reopened, &runtime::create_runtime(), None)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(ready.status, "ready");
    assert_eq!(ready.proposals[0].name, "Durable");
    assert!(
        store::load(&reopened, Some("cancelled-before-registration".into()))
            .await
            .unwrap()
            .is_none()
    );
    store::apply(&reopened, "interrupted", vec!["durable".into()], vec![])
        .await
        .unwrap();
    close_file_pool(reopened).await;
    let final_pool = make_pool();
    assert!(store::load(&final_pool, None).await.unwrap().is_none());
    assert_eq!(
        tag_store::list_tags(&final_pool, None).await.unwrap()[0].usage_count,
        1
    );
    close_file_pool(final_pool).await;
    std::fs::remove_file(path).unwrap();
}

// deadpool closes dropped SQLite handles on a background worker. Explicitly
// close this test-owned temporary DB before deleting its file on Windows.
async fn close_file_pool(pool: Pool) {
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            let old = std::mem::replace(conn, rusqlite::Connection::open_in_memory().unwrap());
            old.close().unwrap();
        })
        .await
        .unwrap();
}
