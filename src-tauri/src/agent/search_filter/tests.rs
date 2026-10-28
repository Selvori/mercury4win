use super::*;
use crate::agent::{configuration, runtime};
use crate::db::{entry_store, smart_view_store};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn meta() -> Metadata {
    Metadata {
        sources: vec![
            Source {
                id: 1,
                name: "技术".into(),
                url: "https://a.test/rss".into(),
            },
            Source {
                id: 2,
                name: "News".into(),
                url: "https://b.test/rss".into(),
            },
        ],
        tags: vec![
            Label {
                id: 3,
                name: "Rust".into(),
            },
            Label {
                id: 4,
                name: "数据库".into(),
            },
        ],
    }
}
fn parse_value(v: Value) -> Result<EntryFilter, FilterError> {
    parse(&v.to_string(), &meta())
}
fn response(f: Value) -> Value {
    json!({"filter":f,"unsupported":[]})
}

#[test]
fn validates_combined_conditions_and_rejects_lossy_filters() {
    let f = parse_value(response(json!({"search":" Rust async ","source":"技术","tags":["rust","数据库","Rust"],"tag_mode":"all","is_read":false,"starred_only":true,"has_note":true,"recent_days":7,"search_field":"body","sort":"relevance","exclude_title":true}))).unwrap();
    assert_eq!(f.search.as_deref(), Some("Rust async"));
    assert_eq!(f.feed_id, Some(1));
    assert_eq!(f.tag_ids, Some(vec![3, 4]));
    assert_eq!(f.is_read, Some(false));
    assert!(f.starred_only);
    assert_eq!(f.recent_days, Some(7));
    for bad in [
        response(json!({"sql":"DELETE FROM entry"})),
        response(json!({"feed_id":99})),
        response(json!({"tag_ids":[3]})),
        response(json!({"read_at":"2026-10-01"})),
        response(json!({"starred_only":"true"})),
        response(json!({"recent_days":1.5})),
        response(json!({"recent_days":0})),
        response(json!({"recent_days":36501})),
        response(json!({"recent_days":7,"date_from":"2026-10-01"})),
        response(json!({"date_from":"2026-02-30"})),
        response(json!({"date_to":"2026-1-01"})),
        response(json!({"date_from":"2026-10-02","date_to":"2026-10-01"})),
        response(json!({"search_field":"semantic"})),
        response(json!({"sort":"read_at"})),
        response(json!({"tag_mode":"none"})),
        response(json!({"search":"a ".repeat(25)})),
        response(json!({"search":"中".repeat(1001)})),
        response(json!({"tags":null})),
        json!({"filter":{},"unsupported":[],"extra":true}),
        json!({"filter":null,"unsupported":[]}),
        json!({"filter":{}}),
    ] {
        assert_eq!(
            parse_value(bad.clone()).unwrap_err().code,
            "invalid",
            "{bad}"
        );
    }
    assert!(parse(
        r#"{"filter":{"search":"x","search":"y"},"unsupported":[]}"#,
        &meta()
    )
    .is_err());
    assert!(parse("```json\n{}\n```", &meta()).is_err());
    assert_eq!(
        parse_value(json!({"filter":{},"unsupported":["上周读过：没有阅读历史"]}))
            .unwrap_err()
            .code,
        "unsupported"
    );
    assert!(validate_input("").is_err());
    assert!(validate_input(&"中".repeat(1001)).is_err());
}

#[test]
fn source_and_tag_matching_never_guess_ids_or_duplicates() {
    assert_eq!(
        parse_value(response(json!({"source":"Ghost"})))
            .unwrap_err()
            .code,
        "unknownSource"
    );
    assert_eq!(
        parse_value(response(json!({"tags":["Ghost"]})))
            .unwrap_err()
            .code,
        "unknownTag"
    );
    let mut m = meta();
    m.sources[1].name = "技术".into();
    m.tags[1].name = "RUST".into();
    let error = parse(&response(json!({"source":"技术"})).to_string(), &m).unwrap_err();
    assert_eq!(error.code, "ambiguousSource");
    assert!(error.detail.contains("https://b.test/rss"));
    let guessed = EntryFilter {
        feed_id: Some(2),
        ..Default::default()
    };
    assert_eq!(
        validate_references(&guessed, "找技术文章", &m)
            .unwrap_err()
            .code,
        "ambiguousSource"
    );
    assert!(validate_references(&guessed, "找 https://b.test/rss 的文章", &m).is_ok());
    let deleted = EntryFilter {
        feed_id: Some(99),
        ..Default::default()
    };
    assert_eq!(
        validate_references(&deleted, "source", &m)
            .unwrap_err()
            .code,
        "metadata"
    );
    assert_eq!(
        parse(
            &response(json!({"source":"https://b.test/rss"})).to_string(),
            &m
        )
        .unwrap()
        .feed_id,
        Some(2)
    );
    assert_eq!(
        parse(&response(json!({"tags":["Rust"]})).to_string(), &m)
            .unwrap_err()
            .code,
        "ambiguousTag"
    );
}

async fn pool() -> Pool {
    let pool = deadpool_sqlite::Config::new(":memory:")
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    pool.get().await.unwrap().interact(|c| {
        crate::db::migrations::run_migrations(c).unwrap();
        c.execute_batch("INSERT INTO feed(id,title,feed_url) VALUES(1,'技术','https://a.test/rss'),(2,'News','https://b.test/rss');
            INSERT INTO tag(id,name,normalized_name) VALUES(3,'Rust','rust'),(4,'数据库','数据库');
            INSERT INTO entry(id,feed_id,title,published_at,is_starred,is_read) VALUES
            (1,1,'Rust async',datetime('now','-2 days'),1,0),
            (2,2,'Rust old',datetime('now','-20 days'),1,1),
            (3,1,'Python',datetime('now','-1 day'),0,0);
            INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,3);
            INSERT INTO entry_note VALUES(1,'private note not sent','now');
            INSERT INTO content(entry_id,markdown) VALUES(1,'private body not sent');").unwrap();
    }).await.unwrap();
    pool
}

async fn server(
    content: Value,
    delay_ms: u64,
    status: u16,
) -> (
    String,
    tokio::task::JoinHandle<Value>,
    tokio::sync::oneshot::Receiver<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        let mut chunk = [0u8; 4096];
        let value = loop {
            let n = s.read(&mut chunk).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&chunk[..n]);
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..end]);
                let len = header
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                if bytes.len() >= end + 4 + len {
                    break serde_json::from_slice::<Value>(&bytes[end + 4..end + 4 + len]).unwrap();
                }
            }
        };
        let _ = tx.send(());
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        let body=json!({"choices":[{"message":{"role":"assistant","content":content.to_string()},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}}).to_string();
        let reply=format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
        let _ = s.write_all(reply.as_bytes()).await;
        value
    });
    (url, task, rx)
}
async fn config(pool: &Pool, url: &str) -> Configuration {
    let mut c = configuration::load(pool, std::path::Path::new(""), "summary")
        .await
        .unwrap();
    c.routes = vec![configuration::Route {
        provider_id: "fixture".into(),
        provider_name: "Fixture".into(),
        base_url: url.into(),
        model_id: "fixture".into(),
        model_name: "mock-model".into(),
        api_key: "synthetic-only".into(),
    }];
    c
}
fn now() -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_rfc3339("2026-10-27T00:30:00+08:00").unwrap()
}

#[tokio::test]
async fn controlled_http_chinese_english_local_query_and_saved_view_share_conditions() {
    let p = pool().await;
    let m = metadata(&p).await.unwrap();
    for sentence in [
        "找最近一周我收藏的 Rust 文章",
        "Find starred unread Rust articles from 技术 in the past 7 days",
    ] {
        let output = response(
            json!({"search":"Rust","source":"技术","is_read":false,"recent_days":7,"starred_only":true}),
        );
        let (url, wire, _) = server(output, 0, 200).await;
        let f = crate::agent::usage::scope(
            &p,
            0,
            "search_filter",
            request(
                &reqwest::Client::new(),
                config(&p, &url).await,
                sentence,
                &m,
                now(),
            ),
        )
        .await
        .unwrap();
        let sent = wire.await.unwrap();
        let payload: Value =
            serde_json::from_str(sent["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(payload["sentence"], sentence);
        assert_eq!(payload["today_utc"], "2026-10-26");
        assert_eq!(payload["now_local"], "2026-10-27T00:30:00+08:00");
        assert!(payload["metadata"]["sources"][0].get("id").is_none());
        assert!(payload["metadata"]["tags"][0].is_string());
        assert_eq!(sent["stream"], false);
        assert!(!sent.to_string().contains("private body"));
        assert!(!sent.to_string().contains("private note"));
        assert!(!sent.to_string().contains("synthetic-only"));
        let id = smart_view_store::save(&p, None, sentence.into(), f.clone())
            .await
            .unwrap();
        let saved = smart_view_store::list(&p)
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.id == id)
            .unwrap()
            .filter;
        assert_eq!(
            serde_json::to_value(&f).unwrap(),
            serde_json::to_value(&saved).unwrap()
        );
        assert_eq!(saved.recent_days, Some(7));
        assert!(saved.date_from.is_none());
        let results = entry_store::query_entries(
            &p,
            &entry_store::EntryQueryParams {
                filter: saved,
                cursor: None,
                limit: 100,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            results.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![1]
        );
        let a = search_store::compile(&f, "2026-10-27T00:30:00+08:00").unwrap();
        let b = search_store::compile(&f, "2026-10-28T00:30:00+08:00").unwrap();
        assert_ne!(a.values, b.values);
    }
    let count = p.get().await.unwrap().interact(|c| c.query_row("SELECT COUNT(*) FROM llm_usage_event WHERE agent_type='search_filter' AND entry_id=0 AND request_status='succeeded'", [], |r| r.get::<_,i64>(0))).await.unwrap().unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn absolute_dates_and_empty_results_are_valid_not_parse_failures() {
    let p = pool().await;
    let f=parse_value(response(json!({"search":"does-not-exist","date_from":"2026-10-01","date_to":"2026-10-27","sort":"oldest"}))).unwrap();
    assert_eq!(f.date_to.as_deref(), Some("2026-10-27"));
    assert_eq!(
        entry_store::query_entries(
            &p,
            &entry_store::EntryQueryParams {
                filter: f,
                cursor: None,
                limit: 100
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
}

#[tokio::test]
async fn missing_ai_does_not_affect_ordinary_keyword_search() {
    let p = pool().await;
    let c = configuration::load(&p, std::path::Path::new(""), "summary")
        .await
        .unwrap();
    assert!(c.routes.is_empty());
    assert_eq!(
        request(&reqwest::Client::new(), c, "Rust", &meta(), now())
            .await
            .unwrap_err()
            .code,
        "configuration"
    );
    assert_eq!(
        entry_store::query_entries(
            &p,
            &entry_store::EntryQueryParams {
                filter: EntryFilter {
                    search: Some("Rust".into()),
                    ..Default::default()
                },
                cursor: None,
                limit: 100
            }
        )
        .await
        .unwrap()
        .total,
        2
    );
}

#[tokio::test]
async fn refusal_invalid_json_and_http_failure_are_not_applied() {
    let p = pool().await;
    for (reply, status, code) in [
        (json!("I cannot help"), 200, "invalid"),
        (response(json!({"feed_id":999})), 200, "invalid"),
        (json!({"error":"fixture"}), 503, "provider"),
    ] {
        let (url, wire, _) = server(reply, 0, status).await;
        assert_eq!(
            request(
                &reqwest::Client::new(),
                config(&p, &url).await,
                "Rust",
                &meta(),
                now()
            )
            .await
            .unwrap_err()
            .code,
            code
        );
        wire.await.unwrap();
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    assert_eq!(
        request(
            &reqwest::Client::new(),
            config(&p, &url).await,
            "Rust",
            &meta(),
            now()
        )
        .await
        .unwrap_err()
        .code,
        "provider"
    );
}

#[tokio::test]
async fn runtime_cancellation_drops_http_and_next_request_can_succeed() {
    let p = pool().await;
    let rt = runtime::create_runtime();
    let client = reqwest::Client::new();
    let (url, wire, received) = server(response(json!({"search":"stale"})), 200, 200).await;
    let c = config(&p, &url).await;
    let r = rt.clone();
    let task = tokio::spawn(async move {
        runtime::execute(
            &r,
            runtime::TaskType::SearchFilter,
            0,
            "old",
            &tauri::ipc::Channel::new(|_| Ok(())),
            || async { Ok(request(&client, c, "old", &meta(), now()).await) },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), received)
        .await
        .unwrap()
        .unwrap();
    rt.lock()
        .unwrap()
        .cancel(runtime::TaskType::SearchFilter, "old", false)
        .unwrap();
    assert!(task.await.unwrap().unwrap_err().contains("cancelled"));
    assert_eq!(
        rt.lock()
            .unwrap()
            .get_status(runtime::TaskType::SearchFilter)
            .state,
        "idle"
    );
    wire.await.unwrap();
    let (url, wire, _) = server(response(json!({"search":"latest"})), 0, 200).await;
    let result = runtime::execute(
        &rt,
        runtime::TaskType::SearchFilter,
        0,
        "new",
        &tauri::ipc::Channel::new(|_| Ok(())),
        || async {
            Ok(request(
                &reqwest::Client::new(),
                config(&p, &url).await,
                "latest",
                &meta(),
                now(),
            )
            .await)
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.search.as_deref(), Some("latest"));
    wire.await.unwrap();
    rt.lock()
        .unwrap()
        .cancel(runtime::TaskType::SearchFilter, "pre-cancel", false)
        .unwrap();
    let cancelled = runtime::execute(
        &rt,
        runtime::TaskType::SearchFilter,
        0,
        "pre-cancel",
        &tauri::ipc::Channel::new(|_| Ok(())),
        || async {
            panic!("must not execute");
            #[allow(unreachable_code)]
            Ok(())
        },
    )
    .await;
    assert!(cancelled.is_err());
}
