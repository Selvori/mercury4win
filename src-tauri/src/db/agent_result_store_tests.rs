use super::*;
use crate::agent::{runtime, summary, translation};
use std::sync::{Arc, Mutex};
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn remove_temporary_database(path: std::path::PathBuf) {
    // deadpool-sqlite disposes connections on its blocking worker; Windows may
    // still hold the file briefly after the final pool owner is dropped.
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            match std::fs::remove_file(&path) {
                Ok(()) => break,
                Err(error) if error.raw_os_error() == Some(32) => {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await
                }
                Err(error) => panic!("temporary database cleanup: {error}"),
            }
        }
    })
    .await
    .unwrap();
}

async fn pool(path: &str) -> Pool {
    let pool = deadpool_sqlite::Config::new(path)
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    pool.get().await.unwrap().interact(|conn| {
        crate::db::migrations::run_migrations(conn).unwrap();
        conn.execute_batch("INSERT OR IGNORE INTO feed(id,feed_url) VALUES(1,'https://fixture.test/feed'); INSERT OR IGNORE INTO entry(id,feed_id,title,url) VALUES(1,1,'Title','https://fixture.test/article'); INSERT OR IGNORE INTO content(entry_id,markdown,markdown_version) VALUES(1,'one\n\ntwo',2);").unwrap();
    }).await.unwrap();
    pool
}
fn result(hash: &str, text: &str, complete: bool) -> AgentResult {
    AgentResult {
        request_route: None,
        segment_routes: Vec::new(),
        segment_ids: Vec::new(),
        segment_errors: Vec::new(),
        source_hash: hash.into(),
        text: Some(text.into()),
        segments: vec![
            Some(text.into()),
            if complete { Some("two".into()) } else { None },
        ],
        complete,
        model_name: "fixture".into(),
    }
}
fn make_channel() -> (Channel<String>, Arc<Mutex<Vec<serde_json::Value>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    (
        Channel::new(move |body| {
            let InvokeResponseBody::Json(json) = body else {
                panic!()
            };
            let text: String = serde_json::from_str(&json).unwrap();
            sink.lock()
                .unwrap()
                .push(serde_json::from_str(&text).unwrap());
            Ok(())
        }),
        events,
    )
}
async fn server(
    responses: Vec<(bool, &'static str)>,
) -> (String, tokio::task::JoinHandle<Vec<serde_json::Value>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (ok, text) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let request;
            loop {
                let mut buf = [0; 4096];
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buf[..n]);
                if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let len = headers
                        .lines()
                        .find_map(|l| {
                            l.to_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap();
                    if bytes.len() >= end + 4 + len {
                        request = serde_json::from_slice::<serde_json::Value>(
                            &bytes[end + 4..end + 4 + len],
                        )
                        .unwrap();
                        break;
                    }
                }
            }
            let streaming = request["stream"].as_bool().unwrap_or(false);
            requests.push(request);
            if text == "PAUSE" {
                let mut byte = [0];
                let _ = socket.read(&mut byte).await;
                continue;
            }
            let body = if !ok {
                text.to_owned()
            } else if streaming {
                format!(
                    "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                    serde_json::json!({"choices":[{"delta":{"content":text}}]}),
                    serde_json::json!({"choices":[{"delta":{"content":"尾段🛰️"}}]})
                )
            } else {
                serde_json::json!({"choices":[{"message":{"role":"assistant","content":text}}]})
                    .to_string()
            };
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        if ok { "200 OK" } else { "500 Error" },
                        body.len(),
                        body
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
        }
        requests
    });
    (url, task)
}

#[tokio::test]
async fn success_slots_checkpoint_and_source_parameter_identity() {
    let pool = pool(":memory:").await;
    let source = source_snapshot(&pool, 1).await.unwrap();
    let slot = ResultSlot::new(1, "summary", "en", "medium", &source.hash);
    slot.save(&pool, &result(&source.hash, "old", true))
        .await
        .unwrap();
    assert_eq!(
        load_summary(&pool, 1, "en", "medium")
            .await
            .unwrap()
            .unwrap()
            .text
            .as_deref(),
        Some("old")
    );
    assert!(load_summary(&pool, 1, "en", "brief")
        .await
        .unwrap()
        .is_none());
    assert!(load_summary(&pool, 1, "zh-Hans", "medium")
        .await
        .unwrap()
        .is_none());
    let params = translation_parameters(None, 1);
    let tr = ResultSlot::new(1, "translation", "en", &params, &source.hash);
    tr.save(&pool, &result(&source.hash, "checkpoint", false))
        .await
        .unwrap();
    assert!(!tr.load(&pool, true).await.unwrap().unwrap().complete);
    tr.save(&pool, &result(&source.hash, "success", true))
        .await
        .unwrap();
    tr.save(&pool, &result(&source.hash, "new partial", false))
        .await
        .unwrap();
    assert_eq!(
        tr.load(&pool, true).await.unwrap().unwrap().text.as_deref(),
        Some("success")
    );
    assert!(load_translation(&pool, 1, "en", Some("hy-mt"), 1)
        .await
        .unwrap()
        .is_none());
    assert!(load_translation(&pool, 1, "en", None, 3)
        .await
        .unwrap()
        .is_none());
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE entry SET feed_content='changed source' WHERE id=1",
                [],
            )
            .unwrap()
        })
        .await
        .unwrap();
    assert!(load_summary(&pool, 1, "en", "medium")
        .await
        .unwrap()
        .is_none());
    assert!(slot
        .save(&pool, &result(&source.hash, "stale", true))
        .await
        .unwrap_err()
        .contains("source changed"));
}

#[tokio::test]
async fn pool_reopen_restores_summary_and_translation_checkpoint() {
    let path = std::env::temp_dir().join(format!("mercury4win-002-{}.db", uuid::Uuid::new_v4()));
    let filename = path.to_str().unwrap();
    {
        let pool = pool(filename).await;
        let s = source_snapshot(&pool, 1).await.unwrap();
        ResultSlot::new(1, "summary", "en", "medium", &s.hash)
            .save(&pool, &result(&s.hash, "durable", true))
            .await
            .unwrap();
        ResultSlot::new(
            1,
            "translation",
            "en",
            &translation_parameters(None, 1),
            &s.hash,
        )
        .save(&pool, &result(&s.hash, "checkpoint", false))
        .await
        .unwrap();
    }
    {
        let pool = pool(filename).await;
        assert_eq!(
            load_summary(&pool, 1, "en", "medium")
                .await
                .unwrap()
                .unwrap()
                .text
                .as_deref(),
            Some("durable")
        );
        assert!(
            !load_translation(&pool, 1, "en", None, 1)
                .await
                .unwrap()
                .unwrap()
                .complete
        );
    }
    remove_temporary_database(path).await;
}

#[tokio::test]
async fn production_summary_collects_every_delta_and_failure_keeps_success() {
    let pool = pool(":memory:").await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let runtime = runtime::create_runtime();
    let (url, task) = server(vec![(true, "中文"), (false, "failure")]).await;
    for ok in [true, false] {
        let (channel, _) = make_channel();
        let actual = summary::run_summary(
            &pool, &client, &runtime, 1, "en", "medium", &url, "", "fixture", channel,
        )
        .await;
        assert_eq!(actual.is_ok(), ok);
    }
    assert_eq!(task.await.unwrap().len(), 2);
    assert_eq!(
        load_summary(&pool, 1, "en", "medium")
            .await
            .unwrap()
            .unwrap()
            .text
            .as_deref(),
        Some("中文尾段🛰️")
    );
}

#[tokio::test]
async fn production_translation_checkpoints_failure_then_resumes_only_missing_segment() {
    for concurrency in [1, 3] {
        let pool = pool(":memory:").await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let (url, task) = server(vec![(true, "第一段"), (false, "failure")]).await;
        let (channel, events) = make_channel();
        assert!(translation::run_translation(
            &pool,
            &client,
            1,
            "en",
            &url,
            "",
            "fixture",
            None,
            concurrency,
            channel
        )
        .await
        .is_err());
        task.await.unwrap();
        assert!(!events.lock().unwrap().iter().any(|v| v["type"] == "done"));
        let partial = load_translation(&pool, 1, "en", None, concurrency)
            .await
            .unwrap()
            .unwrap();
        assert!(!partial.complete);
        assert_eq!(partial.segments.iter().flatten().count(), 1);
        let (url, task) = server(vec![(true, "第二段")]).await;
        let (channel, _) = make_channel();
        translation::run_translation(
            &pool,
            &client,
            1,
            "en",
            &url,
            "",
            "fixture",
            None,
            concurrency,
            channel,
        )
        .await
        .unwrap();
        assert_eq!(task.await.unwrap().len(), 1);
        let saved = load_translation(&pool, 1, "en", None, concurrency)
            .await
            .unwrap()
            .unwrap();
        assert!(saved.complete);
        assert_eq!(saved.segments.iter().flatten().count(), 2);
        let (url, task) = server(vec![(false, "failure"), (false, "failure")]).await;
        let (channel, _) = make_channel();
        assert!(translation::run_translation(
            &pool, &client, 1, "en", &url, "", "fixture", None, 3, channel
        )
        .await
        .is_err());
        task.await.unwrap();
        // concurrency=1 success and concurrency=3 failed attempt are distinct slots.
        assert_eq!(
            load_translation(&pool, 1, "en", None, concurrency)
                .await
                .unwrap()
                .unwrap()
                .segments,
            saved.segments
        );
    }
}

#[tokio::test]
async fn cancellation_fences_already_dispatched_sqlite_transaction_and_preserves_committed_success()
{
    let pool = pool(":memory:").await;
    let source = source_snapshot(&pool, 1).await.unwrap();
    let slot = ResultSlot::new(1, "summary", "en", "medium", &source.hash);
    slot.save(&pool, &result(&source.hash, "old committed", true))
        .await
        .unwrap();
    let runtime = runtime::create_runtime();
    let (events, _) = make_channel();
    let runtime_task = runtime.clone();
    let pool_task = pool.clone();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let task = tokio::spawn(async move {
        runtime::execute(
            &runtime_task,
            runtime::TaskType::Summary,
            1,
            "late-write",
            &events,
            || async move {
                let fence = runtime::current_commit_fence();
                pool_task
                    .get()
                    .await
                    .unwrap()
                    .interact(move |conn| {
                        let tx = conn.transaction().unwrap();
                        tx.execute("UPDATE agent_result_slot SET success_json='corrupted'", [])
                            .unwrap();
                        ready_tx.send(()).unwrap();
                        release_rx.recv().unwrap();
                        fence.commit(tx)
                    })
                    .await
                    .unwrap()
            },
        )
        .await
    });
    ready_rx.await.unwrap();
    runtime
        .lock()
        .unwrap()
        .cancel(runtime::TaskType::Summary, "late-write", false)
        .unwrap();
    assert!(task.await.unwrap().unwrap_err().contains("cancelled"));
    release_tx.send(()).unwrap();
    assert_eq!(
        load_summary(&pool, 1, "en", "medium")
            .await
            .unwrap()
            .unwrap()
            .text
            .as_deref(),
        Some("old committed")
    );
    // A COMMIT linearized before cancellation remains durable.
    let (events, _) = make_channel();
    runtime::execute(
        &runtime,
        runtime::TaskType::Summary,
        1,
        "committed",
        &events,
        || async {
            slot.save(&pool, &result(&source.hash, "new committed", true))
                .await
        },
    )
    .await
    .unwrap();
    runtime
        .lock()
        .unwrap()
        .cancel(runtime::TaskType::Summary, "committed", false)
        .unwrap();
    assert_eq!(
        load_summary(&pool, 1, "en", "medium")
            .await
            .unwrap()
            .unwrap()
            .text
            .as_deref(),
        Some("new committed")
    );
}

#[test]
fn v3_upgrade_preserves_v2_and_legacy_payloads_and_repeats() {
    let conn = crate::db::migrations::historical_fixture(2);
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name='agent_result_slot'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    conn.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test'); INSERT INTO entry(id,feed_id,is_starred,feed_content,feed_content_type) VALUES(1,1,1,'saved Feed body','html'); INSERT INTO summary_result VALUES(1,'en','brief','legacy','en','now'); INSERT INTO entry_note VALUES(1,'note','now');").unwrap();
    crate::db::migrations::run_migrations(&conn).unwrap();
    crate::db::migrations::run_migrations(&conn).unwrap();
    assert_eq!(
        conn.query_row(
            "SELECT value FROM settings WHERE key='schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        crate::db::migrations::SCHEMA_VERSION.to_string()
    );
    assert_eq!(
        conn.query_row("SELECT text FROM summary_result", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "legacy"
    );
    assert_eq!(
        conn.query_row("SELECT markdown FROM entry_note", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "note"
    );
    assert_eq!(
        conn.query_row(
            "SELECT is_starred,feed_content,feed_content_type FROM entry",
            [],
            |r| Ok((
                r.get::<_, bool>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?
            ))
        )
        .unwrap(),
        (true, "saved Feed body".into(), "html".into())
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM agent_result_slot", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn production_cancel_keeps_checkpoint_then_resumes_and_keeps_prior_summary() {
    // Cancelling interact may discard a busy pooled connection; a named
    // temporary DB models production persistence across replacement connections.
    let path = std::env::temp_dir().join(format!(
        "mercury4win-002-cancel-{}.db",
        uuid::Uuid::new_v4()
    ));
    let pool = pool(path.to_str().unwrap()).await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let runtime = runtime::create_runtime();
    let (url, server_task) =
        server(vec![(true, "checkpoint before cancel"), (true, "PAUSE")]).await;
    let (channel, events) = make_channel();
    let job_pool = pool.clone();
    let job_runtime = runtime.clone();
    let job_client = client.clone();
    let job = tokio::spawn(async move {
        runtime::execute(
            &job_runtime,
            runtime::TaskType::Translation,
            1,
            "cancel-translation",
            &channel,
            || async {
                translation::run_translation(
                    &job_pool,
                    &job_client,
                    1,
                    "en",
                    &url,
                    "",
                    "fixture",
                    None,
                    1,
                    channel.clone(),
                )
                .await
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if load_translation(&pool, 1, "en", None, 1)
                .await
                .unwrap()
                .is_some()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    runtime
        .lock()
        .unwrap()
        .cancel(runtime::TaskType::Translation, "cancel-translation", false)
        .unwrap();
    assert!(job.await.unwrap().unwrap_err().contains("cancelled"));
    assert!(!events.lock().unwrap().iter().any(|e| e["type"] == "done"));
    // Cancellation may occur before the second socket opens; don't wait for an
    // unrequested fixture response. The first durable segment is authoritative.
    server_task.abort();
    let checkpoint = load_translation(&pool, 1, "en", None, 1)
        .await
        .unwrap()
        .unwrap();
    assert!(!checkpoint.complete);
    assert_eq!(
        checkpoint.segments[0].as_deref(),
        Some("checkpoint before cancel")
    );
    let (url, task) = server(vec![(true, "resumed after cancel")]).await;
    let (channel, _) = make_channel();
    translation::run_translation(
        &pool, &client, 1, "en", &url, "", "fixture", None, 1, channel,
    )
    .await
    .unwrap();
    assert_eq!(task.await.unwrap().len(), 1);
    assert!(
        load_translation(&pool, 1, "en", None, 1)
            .await
            .unwrap()
            .unwrap()
            .complete
    );
    let source = source_snapshot(&pool, 1).await.unwrap();
    ResultSlot::new(1, "summary", "en", "medium", &source.hash)
        .save(&pool, &result(&source.hash, "old summary", true))
        .await
        .unwrap();
    let (url, server_task) = server(vec![(true, "PAUSE")]).await;
    let (channel, _) = make_channel();
    let job_pool = pool.clone();
    let job_runtime = runtime.clone();
    let job = tokio::spawn(async move {
        runtime::execute(
            &job_runtime,
            runtime::TaskType::Summary,
            1,
            "cancel-summary",
            &channel,
            || async {
                summary::run_summary(
                    &job_pool,
                    &client,
                    &job_runtime,
                    1,
                    "en",
                    "medium",
                    &url,
                    "",
                    "fixture",
                    channel.clone(),
                )
                .await
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if runtime
                .lock()
                .unwrap()
                .get_status(runtime::TaskType::Summary)
                .current_request_id
                .as_deref()
                == Some("cancel-summary")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    runtime
        .lock()
        .unwrap()
        .cancel(runtime::TaskType::Summary, "cancel-summary", false)
        .unwrap();
    assert!(job.await.unwrap().is_err());
    server_task.abort();
    assert_eq!(
        load_summary(&pool, 1, "en", "medium")
            .await
            .unwrap()
            .unwrap()
            .text
            .as_deref(),
        Some("old summary")
    );
    drop(pool);
    remove_temporary_database(path).await;
}
