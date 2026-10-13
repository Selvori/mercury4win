use super::*;
use crate::agent::translation;
use crate::db::agent_result_store::AgentResult;
use deadpool_sqlite::Pool;
use std::sync::{Arc, Mutex};
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn stable_html_blocks_avoid_nested_list_duplicates_and_excluded_content() {
    let html = "<article class='reader'><p><img src='https://fixture.test/a'></p><p>Lead <code>original_code</code></p><ul><li><p>One</p><ol><li>Nested</li></ol></li><li>Two</li></ul><ol><li>Ordered</li></ol><blockquote><p>Quote</p></blockquote><pre><code>Code</code></pre><table><tr><td><p>Cell</p></td></tr></table></article>";
    let (annotated, a) = extract_html(html).unwrap();
    let (_, b) = extract_html(html).unwrap();
    assert_eq!(a.len(), 5);
    assert_eq!(
        a.iter()
            .map(|s| s.segment_type.as_str())
            .collect::<Vec<_>>(),
        ["p", "ul", "ol", "p", "td"]
    );
    assert_eq!(
        a.iter().map(|s| &s.source_segment_id).collect::<Vec<_>>(),
        b.iter().map(|s| &s.source_segment_id).collect::<Vec<_>>()
    );
    assert_eq!(annotated.matches("data-source-segment-id=").count(), 5);
    assert!(a[0].source_text.contains("`original_code`"));
    assert!(a[1].source_text.contains("Nested"));
    assert_eq!(a[3].source_text, "Quote");
    assert_eq!(a[4].source_text, "Cell");
    let (_, changed) = extract_html(&html.replace("Lead", "Changed")).unwrap();
    assert_ne!(a[0].source_segment_id, changed[0].source_segment_id);
}

#[test]
fn concurrency_bounded_and_segmenter_identity_upgraded() {
    assert_eq!(agent_result_store::normalize_translation_concurrency(0), 3);
    assert_eq!(agent_result_store::normalize_translation_concurrency(99), 5);
    assert!(agent_result_store::translation_parameters(None, 3).contains("reader-html-block-v3"));
    assert_ne!(
        agent_result_store::translation_parameters(None, 3),
        serde_json::json!({"segmenter":"reader-html-block-v2","strategy":"default","concurrency":3}).to_string()
    );
    assert_eq!(
        agent_result_store::translation_parameters(None, 99),
        agent_result_store::translation_parameters(None, 5)
    );
}

#[test]
fn headings_quotes_cells_and_captions_are_covered_without_duplicate_children() {
    let html = "<html><head><title>Not article</title></head><body><nav><p>Navigation</p></nav><article><h1>Title</h1><h2>Section</h2><h3>Third</h3><h4>Fourth</h4><h5>Fifth</h5><h6>Sixth</h6><blockquote><p>Quoted <code>name</code></p><ul><li>Quoted list</li></ul></blockquote><table><caption>Table title</caption><tr><th>Header</th><td><p>Cell</p><ul><li>Cell list</li></ul></td><td>42</td></tr></table><figure><img src='x'><figcaption>Figure caption</figcaption></figure><dl><dt>Term</dt><dd><p>Definition</p></dd></dl><pre><code>Untouched code</code></pre><aside><p>Sidebar</p></aside><p><img src='x'></p></article></body></html>";
    let (annotated, segments) = extract_html(html).unwrap();
    assert_eq!(segments.len(), 15);
    assert_eq!(annotated.matches("data-source-segment-id=").count(), 15);
    assert!(segments[6].source_text.contains("Quoted `name`"));
    assert!(segments[10].source_text.contains("Cell\n- Cell list"));
    let text = segments
        .iter()
        .map(|s| s.source_text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for excluded in ["Not article", "Navigation", "Untouched code", "Sidebar"] {
        assert!(!text.contains(excluded));
    }
    assert!(annotated.contains("Untouched code"));
}

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
                serde_json::json!({"choices":[{"message":{"role":"assistant","content":text},"finish_reason":if text=="TRUNCATED" {"length"} else {"stop"}}]})
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
async fn all_failed_and_partial_drain_all_requests_persist_ids_and_retry_one() {
    for concurrency in [1, 3] {
        let pool = pool(":memory:").await;
        pool.get()
            .await
            .unwrap()
            .interact(|c| {
                c.execute("UPDATE content SET markdown='one\n\ntwo\n\nthree'", [])
                    .unwrap()
            })
            .await
            .unwrap();
        let (url, requests) = server(vec![
            (false, "first failed"),
            (true, ""),
            (true, "TRUNCATED"),
        ])
        .await;
        let (channel, events) = make_channel();
        let failed = translation::run_translation(
            &pool,
            &reqwest::Client::new(),
            1,
            "en",
            &url,
            "fixture",
            "fixture",
            None,
            concurrency,
            channel,
        )
        .await
        .unwrap_err();
        assert!(failed.contains("all_failed"));
        assert_eq!(requests.await.unwrap().len(), 3);
        let events = events.lock().unwrap();
        assert_eq!(events.iter().filter(|e| e["type"] == "error").count(), 3);
        assert!(!events.iter().any(|e| e["type"] == "done"));
        assert_eq!(events.last().unwrap()["status"], "all_failed");
        drop(events);
        let saved = load_document(&pool, 1, "en", None, concurrency)
            .await
            .unwrap()
            .result
            .unwrap();
        assert_eq!(saved.segment_ids.len(), 3);
        assert!(saved.segment_errors.iter().all(Option::is_some));
        let (url, requests) = server(vec![(true, "second translated")]).await;
        let (channel, events) = make_channel();
        let error = translation::run_translation_segments(
            &pool,
            &reqwest::Client::new(),
            1,
            "en",
            &url,
            "fixture",
            "fixture",
            None,
            concurrency,
            Some(&saved.segment_ids[1]),
            channel,
        )
        .await
        .unwrap_err();
        assert!(error.contains("partial: 1/3"));
        assert_eq!(requests.await.unwrap().len(), 1);
        assert_eq!(events.lock().unwrap().last().unwrap()["status"], "partial");
        let restored = load_document(&pool, 1, "en", None, concurrency)
            .await
            .unwrap()
            .result
            .unwrap();
        assert_eq!(restored.segments[1].as_deref(), Some("second translated"));
        assert!(restored.segment_errors[1].is_none());
        let (url, requests) =
            server(vec![(true, "first translated"), (true, "third translated")]).await;
        let (channel, events) = make_channel();
        translation::run_translation(
            &pool,
            &reqwest::Client::new(),
            1,
            "en",
            &url,
            "fixture",
            "fixture",
            None,
            concurrency,
            channel,
        )
        .await
        .unwrap();
        assert_eq!(requests.await.unwrap().len(), 2);
        assert!(events.lock().unwrap().iter().any(|e| e["type"] == "done"));
        assert!(
            load_document(&pool, 1, "en", None, concurrency)
                .await
                .unwrap()
                .result
                .unwrap()
                .complete
        );
    }
}

#[tokio::test]
async fn stable_mapping_survives_pool_reopen_and_source_change_rejects_retry() {
    let path = std::env::temp_dir().join(format!(
        "mercury4win-translation-{}.db",
        uuid::Uuid::new_v4()
    ));
    let first = pool(path.to_str().unwrap()).await;
    let (url, requests) = server(vec![(true, "saved"), (false, "missing")]).await;
    let (channel, _) = make_channel();
    translation::run_translation(
        &first,
        &reqwest::Client::new(),
        1,
        "en",
        &url,
        "fixture",
        "fixture",
        None,
        1,
        channel,
    )
    .await
    .unwrap_err();
    requests.await.unwrap();
    let document = load_document(&first, 1, "en", None, 1).await.unwrap();
    let ids = document.result.as_ref().unwrap().segment_ids.clone();
    drop(first);
    let reopened = pool(path.to_str().unwrap()).await;
    let restored = load_document(&reopened, 1, "en", None, 1).await.unwrap();
    assert_eq!(restored.result.unwrap().segment_ids, ids);
    let (url, requests) = server(vec![(true, "restored")]).await;
    let (channel, _) = make_channel();
    translation::run_translation_segments(
        &reopened,
        &reqwest::Client::new(),
        1,
        "en",
        &url,
        "fixture",
        "fixture",
        None,
        1,
        Some(&ids[1]),
        channel,
    )
    .await
    .unwrap();
    let body = requests.await.unwrap();
    // Default template retains its context entry points on serial retry.
    assert!(body[0]["messages"].to_string().contains("saved"));
    reopened
        .get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute("UPDATE content SET markdown='new first\n\nnew second'", [])
                .unwrap()
        })
        .await
        .unwrap();
    assert!(load_document(&reopened, 1, "en", None, 1)
        .await
        .unwrap()
        .result
        .is_none());
    let (channel, _) = make_channel();
    assert!(translation::run_translation_segments(
        &reopened,
        &reqwest::Client::new(),
        1,
        "en",
        "http://127.0.0.1:1",
        "fixture",
        "fixture",
        None,
        1,
        Some(&ids[1]),
        channel
    )
    .await
    .unwrap_err()
    .contains("changed"));
    drop(reopened);
    remove_temporary_database(path).await;
}

#[tokio::test]
async fn actual_http_parallelism_is_five_and_outcome_waits_for_last_response() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let pool = pool(":memory:").await;
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE content SET markdown='one\n\ntwo\n\nthree\n\nfour\n\nfive\n\nsix\n\nseven'",
                [],
            )
            .unwrap()
        })
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let release = Arc::new(tokio::sync::Notify::new());
    let released = Arc::new(AtomicBool::new(false));
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let (started_tx, mut started) = tokio::sync::mpsc::unbounded_channel();
    let server_release = release.clone();
    let server_released = released.clone();
    let server_peak = peak.clone();
    let server = tokio::spawn(async move {
        let mut tasks = Vec::new();
        for i in 0..7 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let release = server_release.clone();
            let released = server_released.clone();
            let active = active.clone();
            let peak = server_peak.clone();
            let started = started_tx.clone();
            tasks.push(tokio::spawn(async move {
                let mut bytes = Vec::new();
                loop {
                    let mut buf = [0; 8192];
                    let n = socket.read(&mut buf).await.unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]);
                        let len = headers.lines().find_map(|l| l.to_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok())).unwrap();
                        if bytes.len() >= end + 4 + len { break; }
                    }
                }
                let count=active.fetch_add(1,Ordering::SeqCst)+1; peak.fetch_max(count,Ordering::SeqCst);
                // Register notification before publishing the observed request.
                let notified=release.notified(); tokio::pin!(notified); notified.as_mut().enable();
                started.send(i).unwrap();
                if !released.load(Ordering::SeqCst) { notified.await; }
                let body=if i==0 { "failed".into() } else { serde_json::json!({"choices":[{"message":{"role":"assistant","content":format!("translated {i}")}}]}).to_string() };
                socket.write_all(format!("HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",if i==0 {"500 Error"} else {"200 OK"},body.len(),body).as_bytes()).await.unwrap();
                active.fetch_sub(1,Ordering::SeqCst);
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
    });
    let (channel, events) = make_channel();
    let agent_pool = pool.clone();
    let agent = tokio::spawn(async move {
        translation::run_translation(
            &agent_pool,
            &reqwest::Client::new(),
            1,
            "en",
            &url,
            "fixture",
            "fixture",
            None,
            99,
            channel,
        )
        .await
    });
    for _ in 0..5 {
        tokio::time::timeout(std::time::Duration::from_secs(2), started.recv())
            .await
            .unwrap()
            .unwrap();
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(40), started.recv())
            .await
            .is_err()
    );
    assert!(!events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e["type"] == "outcome" || e["type"] == "done"));
    released.store(true, Ordering::SeqCst);
    release.notify_waiters();
    let error = agent.await.unwrap().unwrap_err();
    assert!(error.contains("partial: 6/7"), "{error}");
    server.await.unwrap();
    assert_eq!(peak.load(Ordering::SeqCst), 5);
    let events = events.lock().unwrap();
    assert_eq!(events.last().unwrap()["status"], "partial");
    assert_eq!(events.iter().filter(|e| e["type"] == "progress").count(), 7);
    assert_eq!(events.iter().filter(|e| e["type"] == "segment").count(), 6);
}

#[tokio::test]
async fn hy_mt_serial_context_and_production_document_fixture() {
    let pool = pool(":memory:").await;
    let (url, requests) = server(vec![(true, "context translation"), (true, "second")]).await;
    let (channel, _) = make_channel();
    translation::run_translation(
        &pool,
        &reqwest::Client::new(),
        1,
        "zh-Hans",
        &url,
        "fixture",
        "fixture",
        Some("hy-mt-optimized"),
        1,
        channel,
    )
    .await
    .unwrap();
    let requests = requests.await.unwrap();
    assert!(requests[1]["messages"]
        .to_string()
        .contains("context translation"));
    assert!(requests[1]["messages"].to_string().contains("one"));
    let old_summary: AgentResult=serde_json::from_str(r#"{"source_hash":"old","text":"old summary","segments":[],"complete":true,"model_name":"old"}"#).unwrap();
    assert_eq!(old_summary.text.as_deref(), Some("old summary"));
    assert!(old_summary.segment_ids.is_empty());
    pool.get().await.unwrap().interact(|c|c.execute("UPDATE content SET markdown='# Title\n\nAuthor\n\nLead `foo()`.\n\n- First\n\n  - Nested\n\n- Second\n\n1. Ordered\n\n> Quoted original\n\n| Header | Other |\n| --- | --- |\n| Cell text | More text |\n\n```rust\nfn original() {}\n```\n\n![image](https://fixture.test/image.png)'",[]).unwrap()).await.unwrap();
    let document = load_document(&pool, 1, "en", None, 3).await.unwrap();
    assert_eq!(document.source_segments.len(), 10);
    assert_eq!(document.html.matches("data-source-segment-id=").count(), 10);
    let old_slot = agent_result_store::ResultSlot::new(1, "translation", "en", &serde_json::json!({"segmenter":"reader-html-block-v2","strategy":"default","concurrency":3}).to_string(), &document.source_hash);
    let old_result = AgentResult {
        source_hash: document.source_hash.clone(),
        text: None,
        segments: vec![Some("old omitted blocks".into()); 4],
        complete: true,
        model_name: "fixture".into(),
        segment_ids: vec![],
        segment_errors: vec![],
        request_route: None,
        segment_routes: vec![],
    };
    old_slot.save(&pool, &old_result).await.unwrap();
    assert!(load_document(&pool, 1, "en", None, 3)
        .await
        .unwrap()
        .result
        .is_none());
    let (url, requests) = server(vec![(true, "translated block"); 10]).await;
    let (channel, events) = make_channel();
    translation::run_translation(
        &pool,
        &reqwest::Client::new(),
        1,
        "en",
        &url,
        "fixture",
        "fixture",
        None,
        3,
        channel,
    )
    .await
    .unwrap();
    let requests = requests.await.unwrap();
    assert_eq!(requests.len(), document.source_segments.len());
    for source in &document.source_segments {
        assert!(requests.iter().any(
            |request| request["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|message| message["content"]
                    .as_str()
                    .unwrap()
                    .contains(&source.source_text))
        ));
    }
    assert_eq!(
        events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event["type"] == "segment")
            .count(),
        10
    );
    let document = load_document(&pool, 1, "en", None, 3).await.unwrap();
    assert!(document.result.as_ref().unwrap().complete);
    assert_eq!(document.result.as_ref().unwrap().segments.len(), 10);
    assert!(old_slot.load(&pool, false).await.unwrap().is_some());
    if let Ok(path) = std::env::var("M4W_TRANSLATION_FIXTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    }
}
