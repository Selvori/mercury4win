//! M4W-012 production HTTP attempts + SQLite; all identities/keys synthetic.
use super::*;
use crate::agent::{configuration, provider, runtime, summary, tagging, translation};
use crate::db::{
    agent_store,
    usage_store::{self, UsageFilter},
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use tauri::ipc::Channel;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        c.execute_batch("PRAGMA foreign_keys=ON; INSERT INTO feed(id,feed_url) VALUES(1,'https://fixture.test'); INSERT INTO entry(id,feed_id,title) VALUES(1,1,'fixture'); INSERT INTO content(entry_id,markdown) VALUES(1,'one\n\ntwo');").unwrap();
        for id in ["primary","backup"] {
            c.execute("INSERT INTO agent_provider_profile(id,name,base_url) VALUES(?1,?1,?2)",rusqlite::params![id,url]).unwrap();
            c.execute("INSERT INTO agent_model_profile(id,provider_id,name,model_name) VALUES(?1,?1,?1,?1)",[id]).unwrap();
        }
        for agent in ["summary","translation","tagging"] {
            c.execute("INSERT INTO agent_profile(agent_type,primary_model_id,fallback_model_id) VALUES(?1,'primary','backup')",[agent]).unwrap();
        }
    }).await.unwrap();
    pool
}
async fn config(pool: &Pool, agent: &str, fallback: bool) -> configuration::Configuration {
    let mut c = configuration::load(pool, std::path::Path::new(""), agent)
        .await
        .unwrap();
    if !fallback {
        c.routes.truncate(1);
    }
    c
}
fn channel() -> Channel<String> {
    Channel::new(|_| Ok(()))
}
struct Server {
    url: String,
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    closed: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn server(mode: &'static str) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let sink = requests.clone();
    let closed = Arc::new(AtomicUsize::new(0));
    let disconnected = closed.clone();
    let task = tokio::spawn(async move {
        let mut handlers = tokio::task::JoinSet::new();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let sink = sink.clone();
            let disconnected = disconnected.clone();
            handlers.spawn(async move {
                let mut bytes=Vec::new();let request;
                loop {
                    let mut b=[0;4096];let n=socket.read(&mut b).await.unwrap();if n==0{return;}bytes.extend_from_slice(&b[..n]);
                    if let Some(end)=bytes.windows(4).position(|w|w==b"\r\n\r\n") {
                        let length=String::from_utf8_lossy(&bytes[..end]).lines().find_map(|line|line.to_lowercase().strip_prefix("content-length:").and_then(|s|s.trim().parse::<usize>().ok())).unwrap();
                        if bytes.len()>=end+4+length {request=serde_json::from_slice::<serde_json::Value>(&bytes[end+4..end+4+length]).unwrap();break;}
                    }
                }
                let number={let mut list=sink.lock().unwrap();list.push(request.clone());list.len()};
                if mode=="hang" {
                    let mut b=[0];let n=socket.read(&mut b).await;assert!(n.is_err()||n.unwrap()==0);disconnected.fetch_add(1,Ordering::SeqCst);return;
                }
                let tokens=match mode { "partial"=>serde_json::json!({"prompt_tokens":4}),"zero"=>serde_json::json!({"prompt_tokens":0,"completion_tokens":0,"total_tokens":0}),_=>serde_json::json!({"prompt_tokens":4,"completion_tokens":6,"total_tokens":10}) };
                let stream=request["stream"]==true;
                let fail=mode=="fallback"&&request["model"]=="primary";
                let (status,body)=if fail { ("500 Error",serde_json::json!({"usage":tokens,"error":"fixture"}).to_string()) }
                else if stream {
                    let mut s="data: {\"choices\":[{\"delta\":{\"content\":\"summary\"}}]}\n\n".to_string();
                    if mode!="missing" {s.push_str(&format!("data: {}\n\n",serde_json::json!({"choices":[],"usage":tokens})));}
                    if mode=="broken" {s.push_str("data: {invalid}\n\n");} else {s.push_str("data: [DONE]\n\n");}
                    ("200 OK",s)
                } else {
                    let content=if mode=="retry"&&number<=2 {""} else if mode=="parse" {"```"} else {"translated"};
                    let mut value=serde_json::json!({"choices":[{"message":{"role":"assistant","content":content},"finish_reason":"stop"}],"usage":tokens});
                    if mode=="missing" {value.as_object_mut().unwrap().remove("usage");}
                    ("200 OK",value.to_string())
                };
                socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            });
        }
    });
    Server {
        url,
        requests,
        closed,
        task,
    }
}
async fn rows(pool: &Pool) -> Vec<serde_json::Value> {
    pool.get().await.unwrap().interact(|c|{
        let mut stmt=c.prepare("SELECT provider_id,model_id,agent_type,request_phase,request_status,prompt_tokens,completion_tokens,total_tokens,usage_availability,endpoint,entry_id FROM llm_usage_event ORDER BY id").unwrap();
        stmt.query_map([],|r|Ok(serde_json::json!({"provider":r.get::<_,Option<String>>(0)?,"model":r.get::<_,Option<String>>(1)?,"agent":r.get::<_,String>(2)?,"phase":r.get::<_,String>(3)?,"status":r.get::<_,String>(4)?,"prompt":r.get::<_,Option<i64>>(5)?,"completion":r.get::<_,Option<i64>>(6)?,"total":r.get::<_,Option<i64>>(7)?,"availability":r.get::<_,String>(8)?,"endpoint":r.get::<_,String>(9)?,"entry":r.get::<_,i64>(10)?}))).unwrap().collect::<Result<Vec<_>,_>>().unwrap()
    }).await.unwrap()
}
#[tokio::test]
async fn m4w_012_stream_usage_missing_failure_and_no_task_aggregate() {
    for mode in ["success", "missing", "broken"] {
        let server = server(mode).await;
        let pool = pool(&server.url).await;
        let config = config(&pool, "summary", false).await;
        let result = configuration::CONFIGURATION
            .scope(
                config,
                summary::run_summary(
                    &pool,
                    &reqwest::Client::new(),
                    &runtime::create_runtime(),
                    1,
                    "en",
                    "medium",
                    &server.url,
                    "synthetic",
                    "primary",
                    channel(),
                ),
            )
            .await;
        assert_eq!(result.is_ok(), mode != "broken");
        let rows = rows(&pool).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(
            server.requests.lock().unwrap()[0]["stream_options"]["include_usage"],
            true
        );
        assert_eq!(
            rows[0]["status"],
            if mode == "broken" {
                "failed"
            } else {
                "succeeded"
            }
        );
        assert_eq!(rows[0]["provider"], "primary");
        if mode == "missing" {
            assert!(rows[0]["prompt"].is_null());
            assert_eq!(rows[0]["availability"], "missing");
        } else {
            assert_eq!(rows[0]["total"], 10);
        }
        if mode == "success" {
            agent_store::delete_provider(&pool, "primary")
                .await
                .unwrap();
            agent_store::delete_provider(&pool, "backup").await.unwrap();
            assert!(
                configuration::load(&pool, std::path::Path::new(""), "summary")
                    .await
                    .is_err()
            );
            let historical = crate::db::agent_result_store::latest_summary(&pool, 1)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(historical.result.text.as_deref(), Some("summary"));
            assert_eq!(server.requests.lock().unwrap().len(), 1);
        }
    }
}
#[tokio::test]
async fn m4w_012_translation_each_segment_and_fallback_no_cache_event() {
    let server = server("fallback").await;
    let pool = pool(&server.url).await;
    let config = config(&pool, "translation", true).await;
    for _ in 0..1 {
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            configuration::CONFIGURATION.scope(
                config.clone(),
                translation::run_translation(
                    &pool,
                    &reqwest::Client::new(),
                    1,
                    "en",
                    &server.url,
                    "synthetic",
                    "primary",
                    None,
                    2,
                    channel(),
                ),
            ),
        )
        .await
        .expect("request logs and checkpoints must not deadlock with pool size 1")
        .unwrap();
        configuration::CONFIGURATION
            .scope(
                config.clone(),
                crate::agent::translation_segments::load_document(&pool, 1, "en", None, 2),
            )
            .await
            .unwrap();
    }
    let rows = rows(&pool).await;
    assert_eq!(rows.len(), 4);
    assert_eq!(server.requests.lock().unwrap().len(), 4);
    assert_eq!(
        rows.iter()
            .filter(|r| r["status"] == "failed" && r["provider"] == "primary")
            .count(),
        2
    );
    assert_eq!(
        rows.iter()
            .filter(|r| r["status"] == "succeeded"
                && r["provider"] == "backup"
                && r["phase"] == "fallback")
            .count(),
        2
    );
    assert!(rows.iter().all(|r| r["total"] == 10));
}
#[tokio::test]
async fn m4w_012_tag_retries_and_business_parse_failure_keep_http_tokens() {
    for mode in ["retry", "parse"] {
        let server = server(mode).await;
        let pool = pool(&server.url).await;
        let config = config(&pool, "tagging", true).await;
        let result = configuration::CONFIGURATION
            .scope(
                config,
                tagging::run_tagging(
                    &pool,
                    &reqwest::Client::new(),
                    1,
                    &server.url,
                    "synthetic",
                    "primary",
                ),
            )
            .await;
        let rows = rows(&pool).await;
        if mode == "retry" {
            assert!(result.is_ok());
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[0]["status"], "failed");
            assert_eq!(rows[1]["phase"], "fallback");
            assert_eq!(rows[2]["phase"], "retry");
        } else {
            assert!(result.is_err());
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["status"], "succeeded");
        }
        assert!(rows.iter().all(|r| r["total"] == 10));
        assert_eq!(rows.len(), server.requests.lock().unwrap().len());
    }
}
#[tokio::test]
async fn m4w_012_cancel_socket_flush_and_waiting_cancellation_not_counted() {
    let server = server("hang").await;
    let pool = pool(&server.url).await;
    let rt = runtime::create_runtime();
    let client = reqwest::Client::new();
    let active_pool = pool.clone();
    let active_rt = rt.clone();
    let url = server.url.clone();
    let config = config(&pool, "summary", true).await;
    let task = tokio::spawn(async move {
        runtime::execute(
            &active_rt,
            runtime::TaskType::Summary,
            1,
            "A",
            &channel(),
            || async {
                configuration::CONFIGURATION
                    .scope(
                        config,
                        summary::run_summary(
                            &active_pool,
                            &client,
                            &active_rt,
                            1,
                            "en",
                            "medium",
                            &url,
                            "synthetic",
                            "primary",
                            channel(),
                        ),
                    )
                    .await
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while server.requests.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let queued_rt = rt.clone();
    let queued = tokio::spawn(async move {
        runtime::execute(
            &queued_rt,
            runtime::TaskType::Summary,
            1,
            "B",
            &channel(),
            || async {
                panic!("queued future must not run");
                #[allow(unreachable_code)]
                Ok::<_, String>(())
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while rt
            .lock()
            .unwrap()
            .get_status(runtime::TaskType::Summary)
            .waiting_request_id
            .as_deref()
            != Some("B")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    rt.lock()
        .unwrap()
        .cancel(runtime::TaskType::Summary, "B", false)
        .unwrap();
    assert!(queued.await.unwrap().is_err());
    rt.lock()
        .unwrap()
        .cancel(runtime::TaskType::Summary, "A", false)
        .unwrap();
    assert!(task.await.unwrap().is_err());
    flush().await;
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while server.closed.load(Ordering::SeqCst) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let rows = rows(&pool).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["status"], "cancelled");
    assert!(rows[0]["total"].is_null());
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn m4w_012_archive_reactivation_disabled_route_and_history() {
    let pool = pool("https://fixture.test/v1/").await;
    let before = configuration::load(&pool, std::path::Path::new(""), "summary")
        .await
        .unwrap()
        .fingerprint;
    agent_store::set_enabled(&pool, "primary", "model", false)
        .await
        .unwrap();
    assert!(
        configuration::load(&pool, std::path::Path::new(""), "summary")
            .await
            .is_err()
    );
    agent_store::set_enabled(&pool, "primary", "model", true)
        .await
        .unwrap();
    agent_store::delete_provider(&pool, "primary")
        .await
        .unwrap();
    assert_eq!(agent_store::list_providers(&pool).await.unwrap().len(), 1);
    assert!(agent_store::list_models(&pool, "primary")
        .await
        .unwrap()
        .is_empty());
    assert!(
        configuration::load(&pool, std::path::Path::new(""), "summary")
            .await
            .is_err()
    );
    let id = agent_store::create_provider(&pool, "primary", "https://FIXTURE.test:443/v1", None)
        .await
        .unwrap();
    assert_eq!(id, "primary");
    assert!(agent_store::list_models(&pool, "primary")
        .await
        .unwrap()
        .is_empty());
    agent_store::create_model(&pool, &id, "restored name", "primary", [None; 3])
        .await
        .unwrap();
    assert_eq!(
        agent_store::list_models(&pool, "primary").await.unwrap()[0].id,
        "primary"
    );
    assert_eq!(
        configuration::load(&pool, std::path::Path::new(""), "summary")
            .await
            .unwrap()
            .routes[0]
            .model_id,
        "primary"
    );
    agent_store::archive_model(&pool, "primary").await.unwrap();
    agent_store::delete_provider(&pool, "primary")
        .await
        .unwrap();
    let other =
        agent_store::create_provider(&pool, "different account", "https://fixture.test/v1", None)
            .await
            .unwrap();
    assert_ne!(other, "primary");
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            assert_eq!(
                c.query_row(
                    "SELECT primary_model_id FROM agent_profile WHERE agent_type='summary'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "primary"
            );
        })
        .await
        .unwrap();
    assert!(!before.is_empty());
}
#[tokio::test]
async fn m4w_012_report_history_timezone_boundaries_and_idempotent_write() {
    let pool = pool("https://fixture.test").await;
    let e = usage_store::RequestEvent {
        event_key: "unique".into(),
        entry_id: 1,
        provider_id: Some("primary".into()),
        model_id: Some("primary".into()),
        provider_name: Some("primary".into()),
        model_name: "primary".into(),
        agent_type: "summary".into(),
        endpoint: "https://fixture.test/chat/completions".into(),
        request_phase: "normal".into(),
        request_status: "succeeded".into(),
        prompt_tokens: Some(4),
        completion_tokens: Some(6),
        total_tokens: Some(10),
        started_at: "2026-10-12T20:00:00+08:00".into(),
        finished_at: Some("2026-10-12T12:01:00Z".into()),
    };
    usage_store::insert_request_event(&pool, &e).await.unwrap();
    usage_store::insert_request_event(&pool, &e).await.unwrap();
    let mut missing = e.clone();
    missing.event_key = "missing".into();
    missing.request_status = "cancelled".into();
    missing.prompt_tokens = None;
    missing.completion_tokens = None;
    missing.total_tokens = None;
    usage_store::insert_request_event(&pool, &missing)
        .await
        .unwrap();
    let mut renamed = e.clone();
    renamed.event_key = "renamed".into();
    renamed.provider_name = Some("old snapshot".into());
    renamed.request_status = "failed".into();
    usage_store::insert_request_event(&pool, &renamed)
        .await
        .unwrap();
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE agent_provider_profile SET name='renamed current' WHERE id='primary'",
                [],
            )
            .unwrap()
        })
        .await
        .unwrap();
    let mut outside = e.clone();
    outside.event_key = "outside".into();
    outside.started_at = "2026-10-13T00:00:00Z".into();
    usage_store::insert_request_event(&pool, &outside)
        .await
        .unwrap();
    pool.get().await.unwrap().interact(|c| {c.execute("INSERT INTO llm_usage_event(provider_name,model_name,agent_type,total_tokens,request_status,created_at) VALUES('default','old','translation',35,'success','2026-10-12 12:00:00')",[]).unwrap();}).await.unwrap();
    agent_store::delete_provider(&pool, "primary")
        .await
        .unwrap();
    let report = usage_store::report_range(
        &pool,
        "2026-10-12T12:00:00Z",
        "2026-10-13T00:00:00Z",
        "provider",
        UsageFilter::default(),
    )
    .await
    .unwrap();
    if let Ok(path) = std::env::var("M4W_USAGE_FIXTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    assert_eq!(report.totals.request_count, 3);
    assert_eq!(report.totals.total_tokens, 20);
    assert_eq!(report.totals.missing_usage, 1);
    assert_eq!(report.totals.cancelled, 1);
    assert_eq!(report.totals.legacy_events, 1);
    assert_eq!(report.totals.legacy_tokens, 35);
    assert!(report
        .groups
        .iter()
        .any(|g| g.id == "primary" && g.archived));
    let actual: Vec<_> = report.groups.iter().filter(|g| g.id == "primary").collect();
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].label, "renamed current");
    assert_eq!(actual[0].metrics.request_count, 3);
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE agent_model_profile SET name='renamed model' WHERE id='primary'",
                [],
            )
            .unwrap()
        })
        .await
        .unwrap();
    let filtered = usage_store::report_range(
        &pool,
        "2026-10-12T12:00:00Z",
        "2026-10-13T00:00:00Z",
        "model",
        UsageFilter {
            provider_id: Some("primary".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(filtered.totals.legacy_events, 0);
    assert_eq!(filtered.totals.request_count, 3);
    assert_eq!(filtered.groups.len(), 1);
    assert_eq!(filtered.groups[0].label, "renamed current / renamed model");
    assert_eq!(
        usage_store::sanitized_endpoint("https://person:secret@fixture.test/v1?secret=q#private"),
        "https://fixture.test/v1"
    );
}

#[test]
fn m4w_012_upgrade_v4_preserves_legacy_and_is_idempotent() {
    let c = crate::db::migrations::historical_fixture(4);
    c.execute_batch("INSERT INTO agent_provider_profile(id,is_archived,name,base_url) VALUES('provider',0,'kept','https://fixture.test');
        INSERT INTO agent_model_profile(id,is_archived,provider_id,name,model_name) VALUES('model',0,'provider','kept','old');
        INSERT INTO llm_usage_event(id,task_run_id,provider_name,model_name,agent_type,prompt_tokens,completion_tokens,total_tokens,request_status,created_at)
        VALUES(1,NULL,'default','old','translation',0,0,0,'success','2026-10-12 00:00:00');").unwrap();
    crate::db::migrations::run_migrations(&c).unwrap();
    crate::db::migrations::run_migrations(&c).unwrap();
    assert_eq!(
        c.query_row(
            "SELECT value FROM settings WHERE key='schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        crate::db::migrations::SCHEMA_VERSION.to_string()
    );
    assert_eq!(c.query_row("SELECT request_phase,usage_availability,event_key,total_tokens FROM llm_usage_event WHERE id=1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,i64>(3)?))).unwrap(),("legacy".into(),"legacy".into(),None,0));
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM agent_model_profile", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[tokio::test]
async fn m4w_012_archive_transaction_failure_keeps_configuration() {
    let pool = pool("https://fixture.test").await;
    pool.get().await.unwrap().interact(|c|c.execute_batch("CREATE TRIGGER archive_failure BEFORE UPDATE ON agent_model_profile WHEN NEW.is_archived=1 BEGIN SELECT RAISE(ABORT,'fixture archive error'); END;").unwrap()).await.unwrap();
    assert!(agent_store::delete_provider(&pool, "primary")
        .await
        .is_err());
    assert_eq!(agent_store::list_providers(&pool).await.unwrap().len(), 2);
    assert_eq!(
        agent_store::list_models(&pool, "primary")
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        configuration::load(&pool, std::path::Path::new(""), "summary")
            .await
            .is_ok()
    );
}
#[tokio::test]
async fn m4w_012_dispatched_finished_usage_survives_cancellation_and_fence() {
    let pool = pool("https://fixture.test").await;
    let held = pool.get().await.unwrap();
    let owner = pool.clone();
    let (sent, mut signal) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        scope(&owner,1,"summary",async {
        let mut guard=RequestGuard::start(None,"https://fixture.test","synthetic",false);
        guard.tokens(&serde_json::json!({"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}));
        // A completed HTTP outcome enters finish before its DB await.
        let mut finish=Box::pin(guard.finish(&Ok::<_,String>(())));
        tokio::select! { biased; _=&mut finish=>panic!("write must wait for held pool lease"), _=tokio::time::sleep(std::time::Duration::from_millis(30))=>{} }
        sent.send(()).unwrap();finish.await;
    }).await
    });
    signal.recv().await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(held);
    flush().await;
    let rows = rows(&pool).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["status"], "succeeded");
    assert_eq!(rows[0]["total"], 3);
}
#[tokio::test]
async fn m4w_012_write_failure_observable_and_no_preflight_request() {
    let pool = pool("https://fixture.test").await;
    let initial = WRITE_FAILURES.load(Ordering::Relaxed);
    pool.get().await.unwrap().interact(|c|c.execute_batch("CREATE TRIGGER usage_failure BEFORE INSERT ON llm_usage_event BEGIN SELECT RAISE(ABORT,'fixture write failed'); END;").unwrap()).await.unwrap();
    scope(&pool, 1, "summary", async {
        let mut guard = RequestGuard::start(None, "https://fixture.test", "synthetic", false);
        guard.finish(&Ok::<_, String>(())).await;
    })
    .await;
    assert!(WRITE_FAILURES.load(Ordering::Relaxed) > initial);
    assert!(
        usage_store::report(&pool, "7d", "agent", UsageFilter::default())
            .await
            .unwrap()
            .persistence_failures
            > 0
    );
    assert!(rows(&pool).await.is_empty());
    let request = provider::ChatCompletionRequest {
        model: "fixture".into(),
        messages: vec![],
        temperature: None,
        top_p: None,
        max_tokens: None,
        stream: None,
    };
    assert!(scope(
        &pool,
        1,
        "summary",
        provider::chat_completion(&reqwest::Client::new(), "not a URL", "synthetic", &request)
    )
    .await
    .is_err());
    assert!(rows(&pool).await.is_empty());
}

#[tokio::test]
async fn m4w_012_partial_and_real_zero_usage_are_distinct_and_http_timeout() {
    for mode in ["partial", "zero"] {
        let server = server(mode).await;
        let pool = pool(&server.url).await;
        let request = provider::ChatCompletionRequest {
            model: "primary".into(),
            messages: vec![],
            temperature: None,
            top_p: None,
            max_tokens: None,
            stream: None,
        };
        scope(
            &pool,
            1,
            "translation",
            provider::chat_completion(&reqwest::Client::new(), &server.url, "synthetic", &request),
        )
        .await
        .unwrap();
        let rows = rows(&pool).await;
        assert_eq!(rows[0]["status"], "succeeded");
        if mode == "partial" {
            assert_eq!(rows[0]["prompt"], 4);
            assert!(rows[0]["total"].is_null());
            assert_eq!(rows[0]["availability"], "missing");
        } else {
            assert_eq!(rows[0]["prompt"], 0);
            assert_eq!(rows[0]["total"], 0);
            assert_eq!(rows[0]["availability"], "actual");
        }
    }
    let server = server("hang").await;
    let pool = pool(&server.url).await;
    let request = provider::ChatCompletionRequest {
        model: "primary".into(),
        messages: vec![],
        temperature: None,
        top_p: None,
        max_tokens: None,
        stream: None,
    };
    let client = reqwest::Client::builder()
        .read_timeout(std::time::Duration::from_millis(40))
        .build()
        .unwrap();
    assert!(scope(
        &pool,
        1,
        "summary",
        provider::chat_completion(&client, &server.url, "synthetic", &request)
    )
    .await
    .unwrap_err()
    .starts_with("Timeout:"));
    let rows = rows(&pool).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["status"], "timedOut");
    assert!(rows[0]["total"].is_null());
}
#[tokio::test]
async fn m4w_012_translation_parallel_cancel_records_only_started_attempts() {
    let server = server("hang").await;
    let pool = pool(&server.url).await;
    pool.get().await.unwrap().interact(|c|c.execute("UPDATE content SET markdown='one\n\ntwo\n\nthree\n\nfour\n\nfive' WHERE entry_id=1",[]).unwrap()).await.unwrap();
    let config = config(&pool, "translation", true).await;
    let owner = pool.clone();
    let rt = runtime::create_runtime();
    let active = rt.clone();
    let url = server.url.clone();
    let task = tokio::spawn(async move {
        runtime::execute(
            &active,
            runtime::TaskType::Translation,
            1,
            "parallel",
            &channel(),
            || async {
                configuration::CONFIGURATION
                    .scope(
                        config,
                        translation::run_translation(
                            &owner,
                            &reqwest::Client::new(),
                            1,
                            "en",
                            &url,
                            "synthetic",
                            "primary",
                            None,
                            3,
                            channel(),
                        ),
                    )
                    .await
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while server.requests.lock().unwrap().len() != 3 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    rt.lock()
        .unwrap()
        .cancel(runtime::TaskType::Translation, "parallel", false)
        .unwrap();
    assert!(task.await.unwrap().is_err());
    flush().await;
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while server.closed.load(Ordering::SeqCst) != 3 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let rows = rows(&pool).await;
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|r| r["status"] == "cancelled"));
    assert_eq!(server.requests.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn m4w_012_model_create_and_reactivate_explicit_capabilities_and_legacy_defaults() {
    let pool = pool("https://fixture.test").await;
    agent_store::create_model(&pool, "primary", "default", "default", [None; 3])
        .await
        .unwrap();
    let defaults = agent_store::list_models(&pool, "primary")
        .await
        .unwrap()
        .into_iter()
        .find(|m| m.model_name == "default")
        .unwrap();
    assert!(
        defaults.supports_summary && defaults.supports_translation && defaults.supports_tagging
    );
    agent_store::create_model(
        &pool,
        "primary",
        "selected",
        "selected",
        [Some(false), Some(true), Some(false)],
    )
    .await
    .unwrap();
    let created = agent_store::list_models(&pool, "primary")
        .await
        .unwrap()
        .into_iter()
        .find(|m| m.model_name == "selected")
        .unwrap();
    assert_eq!(
        (
            created.supports_summary,
            created.supports_translation,
            created.supports_tagging
        ),
        (false, true, false)
    );
    agent_store::archive_model(&pool, &created.id)
        .await
        .unwrap();
    agent_store::create_model(
        &pool,
        "primary",
        "restored",
        "selected",
        [Some(true), Some(false), Some(true)],
    )
    .await
    .unwrap();
    let restored = agent_store::list_models(&pool, "primary")
        .await
        .unwrap()
        .into_iter()
        .find(|m| m.model_name == "selected")
        .unwrap();
    assert_eq!(restored.id, created.id);
    assert_eq!(
        (
            restored.supports_summary,
            restored.supports_translation,
            restored.supports_tagging
        ),
        (true, false, true)
    );
    agent_store::archive_model(&pool, &created.id)
        .await
        .unwrap();
    agent_store::create_model(&pool, "primary", "legacy restore", "selected", [None; 3])
        .await
        .unwrap();
    let legacy = agent_store::list_models(&pool, "primary")
        .await
        .unwrap()
        .into_iter()
        .find(|m| m.model_name == "selected")
        .unwrap();
    assert_eq!(legacy.id, created.id);
    assert_eq!(
        (
            legacy.supports_summary,
            legacy.supports_translation,
            legacy.supports_tagging
        ),
        (true, false, true)
    );
    let id = legacy.id.clone();
    pool.get()
        .await
        .unwrap()
        .interact(move |c| {
            c.execute(
                "UPDATE agent_profile SET primary_model_id=?1,fallback_model_id=NULL",
                [id],
            )
            .unwrap()
        })
        .await
        .unwrap();
    assert!(
        configuration::load(&pool, std::path::Path::new(""), "summary")
            .await
            .is_ok()
    );
    assert!(
        configuration::load(&pool, std::path::Path::new(""), "translation")
            .await
            .is_err()
    );
}
