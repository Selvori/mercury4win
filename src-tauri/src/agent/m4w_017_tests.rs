//! Local transport and production Channel regressions; no credentials or remote AI.
use super::*;
use std::sync::{Arc, Mutex};
use tauri::ipc::InvokeResponseBody;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn channel() -> (Channel<String>, Arc<Mutex<Vec<String>>>) {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let captured = messages.clone();
    let channel = Channel::new(move |body| {
        let InvokeResponseBody::Json(json) = body else {
            panic!("expected JSON string")
        };
        captured
            .lock()
            .unwrap()
            .push(serde_json::from_str(&json).unwrap());
        Ok(())
    });
    (channel, messages)
}

async fn server(chunks: Vec<Vec<u8>>, abrupt: bool) -> (String, tokio::task::JoinHandle<()>) {
    server_status(chunks, abrupt, "200 OK").await
}

async fn server_status(
    chunks: Vec<Vec<u8>>,
    abrupt: bool,
    status: &'static str,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buf = [0; 4096];
        loop {
            let n = socket.read(&mut buf).await.unwrap();
            assert_ne!(n, 0);
            request.extend_from_slice(&buf[..n]);
            if let Some(end) = request.windows(4).position(|s| s == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]);
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
        for chunk in chunks {
            if socket
                .write_all(format!("{:x}\r\n", chunk.len()).as_bytes())
                .await
                .is_err()
            {
                return;
            }
            if socket.write_all(&chunk).await.is_err() {
                return;
            }
            if socket.write_all(b"\r\n").await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        if !abrupt {
            let _ = socket.write_all(b"0\r\n\r\n").await;
        }
    });
    (url, task)
}

fn request() -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: "fixture".into(),
        messages: vec![],
        temperature: None,
        top_p: None,
        max_tokens: None,
        stream: None,
    }
}

async fn run(chunks: Vec<Vec<u8>>, abrupt: bool) -> (Result<UsageInfo, String>, Vec<String>) {
    let (url, task) = server(chunks, abrupt).await;
    let (channel, messages) = channel();
    let result = chat_completion_stream(
        &Client::builder().no_proxy().build().unwrap(),
        &url,
        "",
        request(),
        channel,
    )
    .await;
    task.await.unwrap();
    let messages = messages.lock().unwrap().clone();
    (result, messages)
}

#[tokio::test]
async fn network_utf8_split_and_json_shaped_content_survive() {
    let text = "中文🛰️\n\"引号\"";
    let wire = format!(
        "data: {}\n\ndata: [DONE]\n\n",
        serde_json::json!({"choices":[{"delta":{"content":text}}]})
    );
    let split = wire
        .as_bytes()
        .windows(3)
        .position(|s| s == "中".as_bytes())
        .unwrap()
        + 1;
    let (result, messages) = run(
        vec![
            wire.as_bytes()[..split].to_vec(),
            wire.as_bytes()[split..].to_vec(),
        ],
        false,
    )
    .await;
    result.unwrap();
    let collected: String = messages
        .iter()
        .map(|m| {
            serde_json::from_str::<serde_json::Value>(m)
                .ok()
                .and_then(|v| v["content"].as_str().map(str::to_owned))
                .unwrap_or_else(|| m.clone())
        })
        .collect();
    assert_eq!(collected, text);
}

#[tokio::test]
async fn network_multiline_data_optional_space_and_usage() {
    let wire = b": keepalive\r\nevent: message\r\ndata:{\r\ndata: \"choices\":[{\"delta\":{\"content\":\"ok\"}}],\r\ndata: \"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3,\"total_tokens\":5}}\r\n\r\ndata:[DONE]\r\n\r\n";
    let (result, messages) = run(wire.chunks(17).map(|c| c.to_vec()).collect(), false).await;
    let usage = result.unwrap();
    assert_eq!(
        (
            usage.prompt_tokens,
            usage.completion_tokens,
            usage.total_tokens
        ),
        (2, 3, 5)
    );
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn network_clean_eof_without_done_is_failure() {
    let (result, _) = run(vec![b"data: {\"choices\":[]}\n\n".to_vec()], false).await;
    assert!(result.is_err(), "early EOF must not be success");
}

#[test]
fn error_events_roundtrip_quotes_newlines_backslashes_unicode_and_controls() {
    let error = "upstream says \"bad\"\nnext line\\中文";
    let legacy = format!(r#"{{"type":"error","message":"{}"}}"#, error);
    assert!(serde_json::from_str::<serde_json::Value>(&legacy).is_err());
    for error in [error, "\t\r\0🛰️", "{\"type\":\"done\"}"] {
        let summary: serde_json::Value = serde_json::from_str(&summary_error_event(error)).unwrap();
        assert_eq!(summary["type"], "error");
        assert_eq!(summary["message"], error);
        let translation: serde_json::Value =
            serde_json::from_str(&translation_error_event(7, error)).unwrap();
        assert_eq!(translation["segment"], 7);
        assert_eq!(translation["error"], error);
    }
}

#[test]
fn every_byte_split_and_single_byte_chunks_preserve_all_event_boundaries() {
    let content = "中文🛰️\n{\"type\":\"done\"}";
    let payload = serde_json::json!({"choices":[{"delta":{"content":content}}]}).to_string();
    for newline in ["\n", "\r\n", "\r"] {
        let wire = format!("\u{feff}: heartbeat{newline}id: ignored{newline}event: message{newline}data:{payload}{newline}{newline}data: {{\"choices\":[],\"usage\":{{\"prompt_tokens\":4,\"completion_tokens\":5,\"total_tokens\":9}}}}{newline}{newline}data: [DONE]{newline}{newline}");
        for split in 0..=wire.len() {
            let mut decoder = sse::SseDecoder::default();
            let mut usage = UsageInfo {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                route: None,
            };
            let (channel, messages) = channel();
            let mut done = false;
            for bytes in [&wire.as_bytes()[..split], &wire.as_bytes()[split..]] {
                if !done {
                    done = decoder
                        .push(bytes, |data| {
                            process_stream_data(data, &mut usage, &channel)
                        })
                        .unwrap();
                }
            }
            assert!(done, "newline {newline:?}, split {split}");
            assert_eq!(usage.total_tokens, 9);
            let messages = messages.lock().unwrap();
            assert_eq!(messages.len(), 1);
            let chunk: serde_json::Value = serde_json::from_str(&messages[0]).unwrap();
            assert_eq!(chunk["type"], "chunk");
            assert_eq!(chunk["content"], content);
        }
        let mut decoder = sse::SseDecoder::default();
        let mut seen = Vec::new();
        for bytes in wire.as_bytes().chunks(1) {
            decoder
                .push(bytes, |data| {
                    seen.push(data.to_owned());
                    Ok(data == "[DONE]")
                })
                .unwrap();
        }
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0], payload);
        assert_eq!(seen[2], "[DONE]");
    }
}

#[test]
fn only_complete_events_dispatch_and_done_requires_blank_line() {
    let mut decoder = sse::SseDecoder::default();
    let mut seen = Vec::new();
    for bytes in [b"data: [DONE]".as_slice(), b"\r"] {
        assert!(!decoder
            .push(bytes, |data| {
                seen.push(data.to_owned());
                Ok(true)
            })
            .unwrap());
    }
    assert!(seen.is_empty());
    assert!(decoder.eof_error().contains("incomplete"));
    assert!(decoder
        .push(b"\n\r\nignored invalid trailing bytes\xff", |data| {
            seen.push(data.to_owned());
            Ok(true)
        })
        .unwrap());
    assert_eq!(seen, ["[DONE]"]);
}

#[test]
fn unfinished_line_and_event_buffers_are_bounded() {
    for bytes in [
        vec![b'x'; sse::EVENT_BYTE_LIMIT + 1],
        b"data: x\n".repeat(sse::EVENT_BYTE_LIMIT / 8 + 1),
    ] {
        let mut decoder = sse::SseDecoder::default();
        assert!(decoder
            .push(&bytes, |_| panic!("no complete event"))
            .unwrap_err()
            .contains("8 MiB limit"));
    }
}

#[test]
fn channel_delivery_failure_is_not_success() {
    let channel = Channel::new(|_| Err(tauri::Error::Io(std::io::Error::other("fixture closed"))));
    let mut usage = UsageInfo {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        route: None,
    };
    let result = process_stream_data(
        r#"{"choices":[{"delta":{"content":"text"}}]}"#,
        &mut usage,
        &channel,
    );
    assert!(result.unwrap_err().contains("Channel error"));
}

#[tokio::test]
async fn malformed_utf8_json_api_error_and_truncated_done_fail() {
    for (wire, expected) in [
        (b"data: \xff\n\n".to_vec(), "Invalid SSE UTF-8"),
        (b"data: {not json}\n\n".to_vec(), "Invalid SSE JSON"),
        (b"data: {}\n\n".to_vec(), "expected choices or usage"),
        (
            b"data: {\"error\":{\"message\":\"bad\\n\\\"quote\\\"\"}}\n\n".to_vec(),
            "bad\n\"quote\"",
        ),
        (b"data: [DONE]\n".to_vec(), "incomplete SSE event"),
        (b"data: \xe4\xb8".to_vec(), "incomplete SSE event"),
    ] {
        let (result, messages) = run(vec![wire], false).await;
        assert!(result.unwrap_err().contains(expected), "{expected}");
        assert!(messages.is_empty());
    }
}

#[tokio::test]
async fn transport_disconnect_preserves_partial_text_but_returns_error() {
    let (result, messages) = run(
        vec![b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n".to_vec()],
        true,
    )
    .await;
    assert!(result.unwrap_err().contains("Stream error"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&messages[0]).unwrap()["content"],
        "partial"
    );
}

async fn pool(markdown: &str) -> deadpool_sqlite::Pool {
    let pool = deadpool_sqlite::Config::new(":memory:")
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    let markdown = markdown.to_owned();
    pool.get().await.unwrap().interact(move |c| {
        crate::db::migrations::run_migrations(c).unwrap();
        c.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://example.test/feed'); INSERT INTO entry(id,feed_id,guid,title) VALUES(1,1,'a','fixture')").unwrap();
        c.execute("INSERT INTO content(entry_id,markdown) VALUES(1,?1)", [markdown]).unwrap();
    }).await.unwrap();
    pool
}

#[tokio::test]
async fn summary_real_agent_channel_has_chunk_done_or_parseable_error_never_false_done() {
    let pool = pool("fixture body").await;
    let client = Client::builder().no_proxy().build().unwrap();
    let runtime = crate::agent::runtime::create_runtime();
    for (wire, success) in [
        (
            "data: {\"choices\":[{\"delta\":{\"content\":\"中文🛰️\"}}]}\n\ndata: [DONE]\n\n",
            true,
        ),
        (
            "data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n",
            false,
        ),
        (
            "data: {\"error\":{\"message\":\"bad\\n\\\"quote\\\"\"}}\n\n",
            false,
        ),
    ] {
        let (url, task) = server(vec![wire.as_bytes().to_vec()], false).await;
        let (channel, messages) = channel();
        let result = crate::agent::summary::run_summary(
            &pool, &client, &runtime, 1, "en", "brief", &url, "", "fixture", channel,
        )
        .await;
        task.await.unwrap();
        assert_eq!(result.is_ok(), success);
        let events: Vec<serde_json::Value> = messages
            .lock()
            .unwrap()
            .iter()
            .map(|m| serde_json::from_str(m).unwrap())
            .collect();
        assert_eq!(
            events.last().unwrap()["type"],
            if success { "done" } else { "error" }
        );
        if !success {
            assert!(!events.iter().any(|m| m["type"] == "done"));
        }
    }
}

#[tokio::test]
async fn translation_serial_and_concurrent_errors_use_actual_channel_valid_json() {
    // One segment goes through concurrent mode; two with concurrency 1 go through serial mode.
    for markdown in ["one", "one\n\ntwo"] {
        let pool = pool(markdown).await;
        let (url, task) = server_status(
            vec!["bad \"quote\"\n中文\\error".as_bytes().to_vec()],
            false,
            "500 Fixture",
        )
        .await;
        let (channel, messages) = channel();
        let result = crate::agent::translation::run_translation(
            &pool,
            &Client::builder().no_proxy().build().unwrap(),
            1,
            "en",
            &url,
            "",
            "fixture",
            None,
            1,
            channel,
        )
        .await;
        task.await.unwrap();
        let events: Vec<serde_json::Value> = messages
            .lock()
            .unwrap()
            .iter()
            .map(|m| serde_json::from_str(m).unwrap())
            .collect();
        let error = events.iter().find(|m| m["type"] == "error").unwrap();
        assert_eq!(error["segment"], 0);
        assert!(error["error"]
            .as_str()
            .unwrap()
            .contains("bad \"quote\"\n中文\\error"));
        if markdown.contains("\n\n") {
            assert!(result.is_err());
        }
        // Concurrent error aggregation is the separate M4W-004 issue.
    }
}
