use super::*;
use crate::agent::{runtime, summary, tagging, translation, translation_segments};
use crate::db::agent_result_store;
use std::sync::{Arc, Mutex};
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Prompts(std::path::PathBuf);
impl Prompts {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("m4w-009-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn custom(&self, agent: &str, user: &str) {
        let mut template = prompt_templates::load_builtin_template(agent, None).unwrap();
        template.system = None;
        template.user = user.into();
        std::fs::write(
            self.0.join(format!("{agent}.custom.yaml")),
            serde_yaml::to_string(&template).unwrap(),
        )
        .unwrap();
    }
}
impl Drop for Prompts {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

async fn pool(url: &str) -> Pool {
    let pool = deadpool_sqlite::Config::new(":memory:")
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    let url = url.to_owned();
    pool.get().await.unwrap().interact(move |c| {
        crate::db::migrations::run_migrations(c).unwrap();
        c.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test/feed'); INSERT INTO entry(id,feed_id,title) VALUES(1,1,'Fixture Title'); INSERT INTO content(entry_id,markdown) VALUES(1,'one\n\ntwo');").unwrap();
        for (id,model) in [("primary","primary-model"),("backup","backup-model")] {
            c.execute("INSERT INTO agent_provider_profile(id,name,base_url,is_enabled) VALUES(?1,?1,?2,1)", rusqlite::params![id,url]).unwrap();
            c.execute("INSERT INTO agent_model_profile(id,provider_id,name,model_name,is_enabled,supports_summary,supports_translation,supports_tagging) VALUES(?1,?1,?1,?2,1,1,1,1)", rusqlite::params![id,model]).unwrap();
        }
        for agent in ["summary","translation","tagging"] {
            c.execute("INSERT INTO agent_profile(agent_type,primary_model_id,fallback_model_id,prompt_strategy,concurrency_degree) VALUES(?1,'primary','backup','hy-mt-optimized',1)", [agent]).unwrap();
        }
    }).await.unwrap();
    pool
}
fn channel() -> (Channel<String>, Arc<Mutex<Vec<serde_json::Value>>>) {
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
struct Server {
    url: String,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    task: tokio::task::JoinHandle<()>,
    disconnected: Arc<std::sync::atomic::AtomicUsize>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn http_server(mode: &'static str) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let sink = requests.clone();
    let disconnected = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let closed = disconnected.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let request;
            loop {
                let mut buf = [0; 4096];
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
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
                        sink.lock().unwrap().push(request.clone());
                        let primary = request["model"] == "primary-model";
                        if mode == "pause" || (mode == "pause_backup" && !primary) {
                            let mut b = [0];
                            let result = socket.read(&mut b).await;
                            assert!(result.is_err() || result.unwrap() == 0);
                            closed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            break;
                        }
                        let failed = mode == "fail" || (primary && mode != "success");
                        let streaming = request["stream"].as_bool().unwrap_or(false);
                        let partial = streaming && primary && mode == "partial";
                        let (status, body) = if partial {
                            ("200 OK", "data: {\"choices\":[{\"delta\":{\"content\":\"PRIMARY PARTIAL\"}}]}\n\n".into())
                        } else if failed {
                            ("500 Error", "fixture request error".into())
                        } else if streaming {
                            ("200 OK", "data: {\"choices\":[{\"delta\":{\"content\":\"BACKUP COMPLETE\"}}]}\n\ndata: [DONE]\n\n".into())
                        } else {
                            ("200 OK", serde_json::json!({"choices":[{"message":{"role":"assistant","content":"translated / tag"},"finish_reason":"stop"}]}).to_string())
                        };
                        socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
                        break;
                    }
                }
            }
        }
    });
    Server {
        url,
        requests,
        task,
        disconnected,
    }
}
async fn wait_requests(server: &Server, count: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while server.requests.lock().unwrap().len() < count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn m4w_009_prompt_policy_custom_priority_and_rejection_notice() {
    let prompts = Prompts::new();
    for agent in ["summary", "translation", "tagging"] {
        prompts.custom(agent, "CUSTOM");
        let (template, notice) =
            prompt_templates::resolve_template(&prompts.0, agent, Some("hy-mt-optimized")).unwrap();
        assert_eq!(template.user, "CUSTOM");
        assert!(notice.is_none());
        let old = std::fs::read(prompts.0.join(format!("{agent}.custom.yaml"))).unwrap();
        assert!(prompt_templates::save_custom_template(
            &prompts.0,
            agent,
            "version: 999\nuser: invalid"
        )
        .is_err());
        assert_eq!(
            std::fs::read(prompts.0.join(format!("{agent}.custom.yaml"))).unwrap(),
            old
        );

        for invalid in [
            "{{typo}}",
            "{{#content}}{{#content}}{{/content}}{{/content}}",
            "{{#content}}{{/content}}{{#content}}{{/content}}",
            "{{#content}}",
            "{{/content}}",
            "{{content",
        ] {
            let mut template = prompt_templates::load_builtin_template(agent, None).unwrap();
            template.user = invalid.into();
            assert!(prompt_templates::validate(&template, agent).is_err());
        }
        std::fs::write(
            prompts.0.join(format!("{agent}.custom.yaml")),
            "version: 999\nuser: invalid",
        )
        .unwrap();
        assert!(prompt_templates::resolve_template(&prompts.0, agent, None)
            .unwrap()
            .1
            .is_some());
    }
}

#[tokio::test]
async fn m4w_009_custom_messages_reach_all_three_actual_executors() {
    let server = http_server("success").await;
    let pool = pool(&server.url).await;
    let prompts = Prompts::new();
    let client = reqwest::Client::new();
    let runtime = runtime::create_runtime();
    for agent in ["summary", "translation", "tagging"] {
        prompts.custom(agent,match agent {"translation"=>"CUSTOM {{source_text}} {{#previous_source_text}}PREV={{previous_source_text}}/{{previous_translated_text}}{{/previous_source_text}}",_=>"CUSTOM {{title}} {{content}}"});
        let config = load(&pool, &prompts.0, agent).await.unwrap();
        let (events, _) = channel();
        CONFIGURATION
            .scope(config, async {
                match agent {
                    "summary" => {
                        summary::run_summary(
                            &pool,
                            &client,
                            &runtime,
                            1,
                            "en",
                            "detailed",
                            &server.url,
                            "",
                            "primary-model",
                            events,
                        )
                        .await
                    }
                    "translation" => {
                        translation::run_translation(
                            &pool,
                            &client,
                            1,
                            "en",
                            &server.url,
                            "",
                            "primary-model",
                            Some("hy-mt-optimized"),
                            1,
                            events,
                        )
                        .await
                    }
                    _ => tagging::run_tagging(&pool, &client, 1, &server.url, "", "primary-model")
                        .await
                        .map(|_| ()),
                }
                .unwrap();
            })
            .await;
    }
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    for request in requests.iter() {
        assert_eq!(request["messages"].as_array().unwrap().len(), 1);
        assert!(request["messages"][0]["content"]
            .as_str()
            .unwrap()
            .starts_with("CUSTOM"));
    }
    assert!(requests[2]["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains("PREV=one/translated / tag"));
}

#[tokio::test]
async fn m4w_009_partial_stream_fallback_reset_and_failure_preserves_success() {
    let server = http_server("partial").await;
    let pool = pool(&server.url).await;
    let prompts = Prompts::new();
    let config = load(&pool, &prompts.0, "summary").await.unwrap();
    let runtime = runtime::create_runtime();
    let (events, messages) = channel();
    CONFIGURATION
        .scope(
            config.clone(),
            summary::run_summary(
                &pool,
                &reqwest::Client::new(),
                &runtime,
                1,
                "en",
                "medium",
                &server.url,
                "",
                "primary-model",
                events,
            ),
        )
        .await
        .unwrap();
    let messages = messages.lock().unwrap();
    assert_eq!(messages.iter().filter(|e| e["type"] == "reset").count(), 1);
    assert!(
        messages.iter().position(|e| e["type"] == "reset").unwrap()
            > messages
                .iter()
                .position(|e| e["content"] == "PRIMARY PARTIAL")
                .unwrap()
    );
    drop(messages);
    let saved = CONFIGURATION
        .scope(
            config.clone(),
            agent_result_store::load_summary(&pool, 1, "en", "medium"),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.text.as_deref(), Some("BACKUP COMPLETE"));
    assert_eq!(saved.model_name, "backup-model");
    if saved.text.is_some() {
        assert_eq!(saved.request_route.as_ref().unwrap().model_id, "backup");
    } else {
        assert!(saved.segment_routes.iter().all(|route| route
            .as_ref()
            .is_some_and(|route| route.model_id == "backup" && route.provider_id == "backup")));
    }
    let failing = http_server("fail").await;
    let mut same = config;
    for route in &mut same.routes {
        route.base_url = failing.url.clone();
    }
    let (events, _) = channel();
    assert!(CONFIGURATION
        .scope(
            same.clone(),
            summary::run_summary(
                &pool,
                &reqwest::Client::new(),
                &runtime,
                1,
                "en",
                "medium",
                &failing.url,
                "",
                "primary-model",
                events
            )
        )
        .await
        .is_err());
    assert_eq!(failing.requests.lock().unwrap().len(), 2);
    let saved = CONFIGURATION
        .scope(
            same,
            agent_result_store::load_summary(&pool, 1, "en", "medium"),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.text.as_deref(), Some("BACKUP COMPLETE"));
}

#[tokio::test]
async fn m4w_009_translation_failed_segments_use_actual_backup_model() {
    let server = http_server("fallback").await;
    let pool = pool(&server.url).await;
    let prompts = Prompts::new();
    let config = load(&pool, &prompts.0, "translation").await.unwrap();
    let (events, _) = channel();
    CONFIGURATION
        .scope(
            config.clone(),
            translation::run_translation(
                &pool,
                &reqwest::Client::new(),
                1,
                "en",
                &server.url,
                "",
                "primary-model",
                None,
                1,
                events,
            ),
        )
        .await
        .unwrap();
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 4);
    assert_eq!(
        requests
            .iter()
            .map(|r| r["model"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "primary-model",
            "backup-model",
            "primary-model",
            "backup-model"
        ]
    );
    drop(requests);
    let saved = CONFIGURATION
        .scope(
            config,
            translation_segments::load_document(&pool, 1, "en", None, 1),
        )
        .await
        .unwrap()
        .result
        .unwrap();
    assert!(saved.complete);
    assert_eq!(saved.model_name, "backup-model");
    if saved.text.is_some() {
        assert_eq!(saved.request_route.as_ref().unwrap().model_id, "backup");
    } else {
        assert!(saved.segment_routes.iter().all(|route| route
            .as_ref()
            .is_some_and(|route| route.model_id == "backup" && route.provider_id == "backup")));
    }
}

#[tokio::test]
async fn m4w_009_actual_cancellation_drops_request_without_fallback() {
    for (mode, count) in [("pause", 1), ("pause_backup", 2)] {
        let server = http_server(mode).await;
        let pool = pool(&server.url).await;
        let prompts = Prompts::new();
        let config = load(&pool, &prompts.0, "summary").await.unwrap();
        let runtime = runtime::create_runtime();
        let rt = runtime.clone();
        let url = server.url.clone();
        let (events, _) = channel();
        let task = tokio::spawn(async move {
            runtime::execute(
                &rt,
                runtime::TaskType::Summary,
                1,
                "cancel",
                &events,
                || async {
                    CONFIGURATION
                        .scope(
                            config,
                            summary::run_summary(
                                &pool,
                                &reqwest::Client::new(),
                                &rt,
                                1,
                                "en",
                                "medium",
                                &url,
                                "",
                                "primary-model",
                                events.clone(),
                            ),
                        )
                        .await
                },
            )
            .await
        });
        wait_requests(&server, count).await;
        runtime
            .lock()
            .unwrap()
            .cancel(runtime::TaskType::Summary, "cancel", false)
            .unwrap();
        assert!(task
            .await
            .unwrap()
            .unwrap_err()
            .starts_with("Agent task cancelled:"));
        tokio::task::yield_now().await;
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while server
                .disconnected
                .load(std::sync::atomic::Ordering::SeqCst)
                == 0
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), count);
    }
}

#[tokio::test]
async fn m4w_009_capabilities_and_read_only_unconfigured_identity() {
    let pool = pool("http://127.0.0.1:1").await;
    let prompts = Prompts::new();
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE agent_model_profile SET supports_summary=0 WHERE id='primary'",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();
    assert!(load(&pool, &prompts.0, "summary")
        .await
        .err()
        .unwrap()
        .contains("primary model"));
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE agent_model_profile SET supports_summary=1 WHERE id='primary'",
                [],
            )
            .unwrap();
            c.execute(
                "UPDATE agent_model_profile SET supports_summary=0 WHERE id='backup'",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();
    assert!(load(&pool, &prompts.0, "summary")
        .await
        .err()
        .unwrap()
        .contains("fallback model"));
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute("UPDATE agent_model_profile SET supports_summary=1", [])
                .unwrap();
            c.execute(
                "UPDATE agent_provider_profile SET is_enabled=0 WHERE id='backup'",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();
    assert!(load(&pool, &prompts.0, "summary")
        .await
        .err()
        .unwrap()
        .contains("fallback model"));
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE agent_profile SET primary_model_id=NULL,fallback_model_id=NULL",
                [],
            )
            .unwrap();
            c.execute("UPDATE agent_model_profile SET supports_summary=0", [])
                .unwrap();
        })
        .await
        .unwrap();
    let config = load(&pool, &prompts.0, "summary").await.unwrap();
    assert!(config.routes.is_empty());
    assert!(crate::commands::agent_cmd::resolve_agent_route(
        &pool,
        "summary",
        |m| m.supports_summary,
        "invented-model"
    )
    .await
    .unwrap_err()
    .contains("compatible model"));
}

#[tokio::test]
async fn m4w_009_configuration_identity_invalidates_new_reads_and_preserves_history() {
    let server = http_server("success").await;
    let pool = pool(&server.url).await;
    let prompts = Prompts::new();
    let before = load(&pool, &prompts.0, "summary").await.unwrap();
    let (events, _) = channel();
    let runtime = runtime::create_runtime();
    CONFIGURATION
        .scope(
            before.clone(),
            summary::run_summary(
                &pool,
                &reqwest::Client::new(),
                &runtime,
                1,
                "en",
                "brief",
                &server.url,
                "",
                "primary-model",
                events,
            ),
        )
        .await
        .unwrap();
    prompts.custom("summary", "NEW {{content}}");
    let after = load(&pool, &prompts.0, "summary").await.unwrap();
    assert_ne!(before.fingerprint, after.fingerprint);
    assert!(CONFIGURATION
        .scope(
            after,
            agent_result_store::load_summary(&pool, 1, "en", "brief")
        )
        .await
        .unwrap()
        .is_none());
    let old = agent_result_store::latest_summary(&pool, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(old.detail_level, "brief");
    assert_eq!(old.result.text.as_deref(), Some("BACKUP COMPLETE"));
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE agent_model_profile SET model_name='changed' WHERE id='primary'",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();
    assert_ne!(
        before.fingerprint,
        load(&pool, &prompts.0, "summary")
            .await
            .unwrap()
            .fingerprint
    );
    let routed = load(&pool, &prompts.0, "summary").await.unwrap();
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE agent_provider_profile SET base_url='http://127.0.0.1:2' WHERE id='backup'",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();
    assert_ne!(
        routed.fingerprint,
        load(&pool, &prompts.0, "summary")
            .await
            .unwrap()
            .fingerprint
    );
    let source = agent_result_store::source_snapshot(&pool, 1).await.unwrap();
    let legacy = agent_result_store::ResultSlot::new(1, "summary", "ja", "detailed", &source.hash);
    legacy
        .save(
            &pool,
            &agent_result_store::AgentResult {
                source_hash: source.hash,
                text: Some("Legacy success".into()),
                segments: vec![],
                complete: true,
                model_name: "archived model".into(),
                request_route: None,
                segment_routes: vec![],
                segment_ids: vec![],
                segment_errors: vec![],
            },
        )
        .await
        .unwrap();
    let current = load(&pool, &prompts.0, "summary").await.unwrap();
    assert!(CONFIGURATION
        .scope(
            current,
            agent_result_store::load_summary(&pool, 1, "ja", "detailed")
        )
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        legacy
            .load(&pool, false)
            .await
            .unwrap()
            .unwrap()
            .text
            .as_deref(),
        Some("Legacy success")
    );
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute("UPDATE agent_provider_profile SET is_enabled=0", [])
                .unwrap();
            c.execute("DELETE FROM agent_profile", []).unwrap();
        })
        .await
        .unwrap();
    assert!(agent_result_store::latest_summary(&pool, 1)
        .await
        .unwrap()
        .is_some());
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute("UPDATE content SET markdown='changed source'", [])
                .unwrap();
        })
        .await
        .unwrap();
    assert!(agent_result_store::latest_summary(&pool, 1)
        .await
        .unwrap()
        .is_none());
    assert_eq!(server.requests.lock().unwrap().len(), 1); // no network on any read.
}
