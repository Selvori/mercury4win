//! Local HTTP + production agents, memory SQLite, no user data/AI credentials.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::ipc::InvokeResponseBody;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, Semaphore};

fn channel() -> (Channel<String>, Arc<Mutex<Vec<serde_json::Value>>>) {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let sink = messages.clone();
    (
        Channel::new(move |body| {
            let InvokeResponseBody::Json(json) = body else {
                panic!("JSON expected")
            };
            let text: String = serde_json::from_str(&json).unwrap();
            sink.lock()
                .unwrap()
                .push(serde_json::from_str(&text).unwrap());
            Ok(())
        }),
        messages,
    )
}

async fn pool(markdown: &str) -> deadpool_sqlite::Pool {
    let pool = deadpool_sqlite::Config::new(":memory:")
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    let markdown = markdown.to_owned();
    pool.get()
        .await
        .unwrap()
        .interact(move |c| {
            crate::db::migrations::run_migrations(c).unwrap();
            c.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test/feed')")
                .unwrap();
            for id in 1..=4 {
                c.execute(
                    "INSERT INTO entry(id,feed_id,guid,title) VALUES(?1,1,?2,'fixture')",
                    rusqlite::params![id, id.to_string()],
                )
                .unwrap();
                c.execute(
                    "INSERT INTO content(entry_id,markdown) VALUES(?1,?2)",
                    rusqlite::params![id, markdown],
                )
                .unwrap();
            }
        })
        .await
        .unwrap();
    pool
}

struct Server {
    url: String,
    release: Arc<Semaphore>,
    count: Arc<AtomicUsize>,
    requested: mpsc::UnboundedReceiver<()>,
    closed: mpsc::UnboundedReceiver<()>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn server(fail: bool) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(Semaphore::new(0));
    let (sent, requested) = mpsc::unbounded_channel();
    let (disconnected, closed) = mpsc::unbounded_channel();
    let tally = count.clone();
    let semaphore = release.clone();
    let task = tokio::spawn(async move {
        let mut handlers = tokio::task::JoinSet::new();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (sent, disconnected, count, release) = (
                sent.clone(),
                disconnected.clone(),
                tally.clone(),
                semaphore.clone(),
            );
            handlers.spawn(async move {
                let mut bytes = Vec::new();
                let mut buf = [0; 4096];
                let request;
                loop {
                    let n = socket.read(&mut buf).await.unwrap();
                    if n == 0 { return; }
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]);
                        let length = headers.lines().find_map(|line| line.to_lowercase().strip_prefix("content-length:").and_then(|s| s.trim().parse::<usize>().ok())).unwrap_or(0);
                        if bytes.len() >= end + 4 + length {
                            request = serde_json::from_slice::<serde_json::Value>(&bytes[end+4..end+4+length]).unwrap();
                            break;
                        }
                    }
                }
                let streaming = request["stream"].as_bool().unwrap_or(false);
                if streaming && !fail {
                    socket.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\nContent-Type: text/event-stream\r\n\r\n").await.unwrap();
                    let first = "data: {\"choices\":[{\"delta\":{\"content\":\"partial 中文\"}}]}\n\n";
                    socket.write_all(format!("{:x}\r\n{first}\r\n", first.len()).as_bytes()).await.unwrap();
                }
                count.fetch_add(1, Ordering::SeqCst);
                sent.send(()).unwrap();
                // While withheld, detect reqwest's cancelled socket rather than
                // just asserting a logical runtime state.
                tokio::select! {
                    permit = release.acquire() => { permit.unwrap().forget(); }
                    _ = socket.read(&mut buf) => { let _ = disconnected.send(()); return; }
                }
                if streaming && !fail {
                    let done = "data: [DONE]\n\n";
                    let _ = socket.write_all(format!("{:x}\r\n{done}\r\n0\r\n\r\n", done.len()).as_bytes()).await;
                    return;
                }
                let body = if fail { "fixture failure".into() }
                    else if streaming { "data: {\"choices\":[{\"delta\":{\"content\":\"中文\"}}]}\n\ndata: [DONE]\n\n".into() }
                    else { serde_json::json!({"choices":[{"message":{"role":"assistant","content":"[\"fixture\"]"}}], "usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}).to_string() };
                let status = if fail { "500 Failed" } else { "200 OK" };
                let response = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: {}\r\n\r\n{body}", body.len(), if streaming { "text/event-stream" } else { "application/json" });
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    Server {
        url,
        release,
        count,
        requested,
        closed,
        task,
    }
}

async fn received(receiver: &mut mpsc::UnboundedReceiver<()>) {
    tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap();
}

async fn state(runtime: &SharedAgentRuntime, kind: TaskType, depth: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if runtime.lock().unwrap().get_status(kind).queue_depth == depth {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

fn launch(
    runtime: &SharedAgentRuntime,
    pool: &deadpool_sqlite::Pool,
    url: &str,
    kind: TaskType,
    id: &'static str,
    entry: i64,
    concurrency: usize,
) -> (
    tokio::task::JoinHandle<Result<(), String>>,
    Arc<Mutex<Vec<serde_json::Value>>>,
) {
    let (channel, messages) = channel();
    let (runtime, pool, url) = (runtime.clone(), pool.clone(), url.to_owned());
    let task = tokio::spawn(async move {
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        execute(&runtime, kind, entry, id, &channel, || async {
            match kind {
                TaskType::Summary => {
                    crate::agent::summary::run_summary(
                        &pool,
                        &client,
                        &runtime,
                        entry,
                        "en",
                        "brief",
                        &url,
                        "",
                        "fixture",
                        channel.clone(),
                    )
                    .await
                }
                TaskType::Translation => {
                    crate::agent::translation::run_translation(
                        &pool,
                        &client,
                        entry,
                        "en",
                        &url,
                        "",
                        "fixture",
                        None,
                        concurrency,
                        channel.clone(),
                    )
                    .await
                }
                TaskType::Tagging => {
                    crate::agent::tagging::run_tagging(&pool, &client, entry, &url, "", "fixture")
                        .await
                        .map(|tags| {
                            assert_eq!(tags, ["fixture"]);
                        })
                }
            }
        })
        .await
    });
    (task, messages)
}

#[tokio::test]
async fn actual_http_latest_waiting_and_success_promotion_for_every_kind() {
    for kind in [TaskType::Summary, TaskType::Translation, TaskType::Tagging] {
        let runtime = create_runtime();
        let pool = pool("one paragraph").await;
        let mut api = server(false).await;
        let (a, _) = launch(&runtime, &pool, &api.url, kind, "a", 1, 1);
        received(&mut api.requested).await;
        let (b, events_b) = launch(&runtime, &pool, &api.url, kind, "b", 2, 1);
        state(&runtime, kind, 1).await;
        let (c, events_c) = launch(&runtime, &pool, &api.url, kind, "c", 3, 1);
        assert!(b.await.unwrap().unwrap_err().contains("replaced_by_latest"));
        assert_eq!(
            api.count.load(Ordering::SeqCst),
            1,
            "waiting commands made HTTP requests"
        );
        assert_eq!(
            runtime
                .lock()
                .unwrap()
                .get_status(kind)
                .waiting_request_id
                .as_deref(),
            Some("c")
        );
        api.release.add_permits(1);
        a.await.unwrap().unwrap();
        received(&mut api.requested).await;
        assert_eq!(api.count.load(Ordering::SeqCst), 2);
        api.release.add_permits(1);
        c.await.unwrap().unwrap();
        assert_eq!(runtime.lock().unwrap().get_status(kind).state, "idle");
        assert!(events_b
            .lock()
            .unwrap()
            .iter()
            .any(|v| v["state"] == "cancelled"));
        assert!(events_c
            .lock()
            .unwrap()
            .iter()
            .any(|v| v["state"] == "running"));
    }
}

#[tokio::test]
async fn cancelled_http_closes_and_promotes_for_every_kind() {
    for kind in [TaskType::Summary, TaskType::Translation, TaskType::Tagging] {
        let runtime = create_runtime();
        let pool = pool("one paragraph").await;
        let mut api = server(false).await;
        let (a, events) = launch(&runtime, &pool, &api.url, kind, "a", 1, 1);
        received(&mut api.requested).await;
        if kind == TaskType::Summary {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if events.lock().unwrap().iter().any(|v| v["type"] == "chunk") {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        }
        let (b, _) = launch(&runtime, &pool, &api.url, kind, "b", 2, 1);
        state(&runtime, kind, 1).await;
        runtime.lock().unwrap().cancel(kind, "a", false).unwrap();
        assert!(a.await.unwrap().unwrap_err().contains("user_cancelled"));
        received(&mut api.closed).await;
        received(&mut api.requested).await;
        // A delayed old finish and old cancel cannot affect newly promoted B.
        runtime.lock().unwrap().finish(kind, "a");
        runtime.lock().unwrap().cancel(kind, "a", false).unwrap();
        assert_eq!(
            runtime
                .lock()
                .unwrap()
                .get_status(kind)
                .current_request_id
                .as_deref(),
            Some("b")
        );
        api.release.add_permits(1);
        b.await.unwrap().unwrap();
        let events = events.lock().unwrap();
        assert!(events.iter().any(|v| v["state"] == "cancelled"));
        assert!(!events.iter().any(|v| v["type"] == "done"));
    }
}

#[tokio::test]
async fn cancel_translation_drops_all_buffered_http_and_never_starts_more_segments() {
    for concurrency in [1, 3] {
        let runtime = create_runtime();
        let pool = pool("a\n\nb\n\nc\n\nd\n\ne\n\nf").await;
        let mut api = server(false).await;
        let (task, _) = launch(
            &runtime,
            &pool,
            &api.url,
            TaskType::Translation,
            "translation",
            1,
            concurrency,
        );
        for _ in 0..concurrency {
            received(&mut api.requested).await;
        }
        runtime
            .lock()
            .unwrap()
            .cancel(TaskType::Translation, "translation", false)
            .unwrap();
        assert!(task.await.unwrap().is_err());
        for _ in 0..concurrency {
            received(&mut api.closed).await;
        }
        assert_eq!(api.count.load(Ordering::SeqCst), concurrency);
        assert_eq!(
            runtime
                .lock()
                .unwrap()
                .get_status(TaskType::Translation)
                .state,
            "idle"
        );
    }
}

#[tokio::test]
async fn waiting_cancel_and_abandon_never_issue_http_and_active_leave_continues() {
    let kind = TaskType::Summary;
    let runtime = create_runtime();
    let pool = pool("paragraph").await;
    let mut api = server(false).await;
    let (a, _) = launch(&runtime, &pool, &api.url, kind, "a", 1, 1);
    received(&mut api.requested).await;
    runtime.lock().unwrap().cancel(kind, "a", true).unwrap();
    for (id, entry, waiting_only) in [("b", 2, false), ("c", 3, true)] {
        let (waiting, _) = launch(&runtime, &pool, &api.url, kind, id, entry, 1);
        state(&runtime, kind, 1).await;
        runtime
            .lock()
            .unwrap()
            .cancel(kind, id, waiting_only)
            .unwrap();
        assert!(waiting.await.unwrap().is_err());
    }
    // Reverse IPC ordering: abandon arrives before submit is registered.
    runtime.lock().unwrap().cancel(kind, "late", true).unwrap();
    let (late, _) = launch(&runtime, &pool, &api.url, kind, "late", 4, 1);
    assert!(late
        .await
        .unwrap()
        .unwrap_err()
        .contains("before registration"));
    assert_eq!(api.count.load(Ordering::SeqCst), 1);
    api.release.add_permits(1);
    a.await.unwrap().unwrap();
    assert_eq!(runtime.lock().unwrap().get_status(kind).state, "idle");
}

#[tokio::test]
async fn route_and_executor_failure_release_then_waiting_makes_one_request() {
    for route_failure in [true, false] {
        let runtime = create_runtime();
        let pool = pool("paragraph").await;
        let mut api = server(!route_failure).await;
        let (gate, mut wait) = watch::channel(false);
        let (channel, _) = channel();
        let rt = runtime.clone();
        let db = pool.clone();
        let url = api.url.clone();
        let first = tokio::spawn(async move {
            execute(&rt, TaskType::Summary, 1, "a", &channel, || async {
                wait.changed().await.unwrap();
                if route_failure {
                    return crate::commands::agent_cmd::resolve_agent_route(
                        &db,
                        "summary",
                        |m| m.supports_summary,
                        "fixture",
                    )
                    .await
                    .map(|_| ());
                }
                let client = reqwest::Client::builder().no_proxy().build().unwrap();
                crate::agent::summary::run_summary(
                    &db,
                    &client,
                    &rt,
                    1,
                    "en",
                    "brief",
                    &url,
                    "",
                    "fixture",
                    channel.clone(),
                )
                .await
            })
            .await
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            while runtime.lock().unwrap().get_status(TaskType::Summary).state == "idle" {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let (next, _) = launch(&runtime, &pool, &api.url, TaskType::Summary, "b", 2, 1);
        state(&runtime, TaskType::Summary, 1).await;
        gate.send(true).unwrap();
        if !route_failure {
            received(&mut api.requested).await;
            api.release.add_permits(1);
        }
        assert!(first.await.unwrap().is_err());
        received(&mut api.requested).await;
        api.release.add_permits(1);
        assert_eq!(next.await.unwrap().is_ok(), route_failure);
        assert_eq!(
            api.count.load(Ordering::SeqCst),
            if route_failure { 1 } else { 2 }
        );
        assert_eq!(
            runtime.lock().unwrap().get_status(TaskType::Summary).state,
            "idle"
        );
    }
}

#[tokio::test]
async fn different_kinds_execute_independently() {
    let runtime = create_runtime();
    let pool = pool("paragraph").await;
    let mut api = server(false).await;
    let mut tasks = Vec::new();
    for kind in [TaskType::Summary, TaskType::Translation, TaskType::Tagging] {
        tasks.push(launch(&runtime, &pool, &api.url, kind, kind.as_str(), 1, 1).0);
        received(&mut api.requested).await;
    }
    assert_eq!(api.count.load(Ordering::SeqCst), 3);
    api.release.add_permits(3);
    for task in tasks {
        task.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn dropped_or_panicking_executor_releases_guard_and_promotes() {
    for panic in [false, true] {
        let runtime = create_runtime();
        let (first_channel, _) = channel();
        let rt = runtime.clone();
        let (gate, mut signal) = watch::channel(false);
        let first = tokio::spawn(async move {
            execute(&rt, TaskType::Summary, 1, "a", &first_channel, || async {
                signal.changed().await.unwrap();
                if panic {
                    panic!("fixture executor panic");
                }
                std::future::pending::<Result<(), String>>().await
            })
            .await
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            while runtime.lock().unwrap().get_status(TaskType::Summary).state == "idle" {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let rt = runtime.clone();
        let (next_channel, _) = channel();
        let next = tokio::spawn(async move {
            execute(&rt, TaskType::Summary, 2, "b", &next_channel, || async {
                Ok::<_, String>(())
            })
            .await
        });
        state(&runtime, TaskType::Summary, 1).await;
        if panic {
            gate.send(true).unwrap();
        } else {
            first.abort();
        }
        assert!(first.await.is_err());
        next.await.unwrap().unwrap();
        assert_eq!(
            runtime.lock().unwrap().get_status(TaskType::Summary).state,
            "idle"
        );
    }
}

#[test]
fn request_identity_validation_and_stale_waiting_guard() {
    let mut rt = AgentRuntime::default();
    assert!(rt.submit(TaskType::Summary, 1, "").is_err());
    let _a = rt.submit(TaskType::Summary, 1, "a").unwrap();
    assert!(rt.submit(TaskType::Summary, 1, "a").is_err());
    let _b = rt.submit(TaskType::Summary, 2, "b").unwrap();
    let _c = rt.submit(TaskType::Summary, 3, "c").unwrap();
    rt.finish(TaskType::Summary, "b");
    assert_eq!(
        rt.get_status(TaskType::Summary)
            .waiting_request_id
            .as_deref(),
        Some("c")
    );
    rt.finish(TaskType::Summary, "a");
    assert_eq!(
        rt.get_status(TaskType::Summary)
            .current_request_id
            .as_deref(),
        Some("c")
    );
}
