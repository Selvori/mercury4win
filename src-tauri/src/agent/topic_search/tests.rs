use super::*;
use crate::db::{
    entry_store::{self, EntryQueryParams},
    migrations, topic_search_store as store,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../../tests/topic_search/corpus.json")).unwrap()
}
pub(crate) fn seed(c: &mut rusqlite::Connection) {
    c.execute_batch("DELETE FROM entry_tag;DELETE FROM entry_note;DELETE FROM content;DELETE FROM entry;DELETE FROM feed;DELETE FROM tag;
        INSERT INTO feed(id,title,feed_url) VALUES(1,'技术来源','https://tech.test/rss'),(2,'新闻来源','https://news.test/rss');
        INSERT INTO tag(id,name,normalized_name) VALUES(1,'Rust','rust'),(2,'SQLite','sqlite');").unwrap();
    for a in corpus()["articles"].as_array().unwrap() {
        let body = format!(
            "{}{}",
            "Routine infrastructure maintenance and storage notes. "
                .repeat(a["prefix_repeat"].as_u64().unwrap_or(0) as usize),
            a["body"].as_str().unwrap()
        );
        c.execute("INSERT INTO entry(id,feed_id,guid,title,summary,published_at,is_starred,is_read) VALUES(?1,?2,?3,?4,?5,datetime('now',?6),?7,0)",rusqlite::params![a["id"].as_i64().unwrap(),a["feed"].as_i64().unwrap(),format!("topic-fixture-{}",a["id"]),a["title"].as_str().unwrap(),a["summary"].as_str().unwrap(),format!("-{} days",a["days"].as_i64().unwrap()),a["starred"].as_bool().unwrap()]).unwrap();
        c.execute(
            "INSERT INTO content(entry_id,markdown) VALUES(?1,?2)",
            rusqlite::params![a["id"].as_i64().unwrap(), body],
        )
        .unwrap();
    }
    c.execute_batch("INSERT INTO entry_note VALUES(1,'SECRET_NOTE: AI model release private annotation',datetime('now')); INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,1);").unwrap();
}
pub(crate) async fn pool() -> Pool {
    let p = deadpool_sqlite::Config::new(":memory:")
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    p.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
            migrations::run_migrations(c).unwrap();
            seed(c);
        })
        .await
        .unwrap();
    p
}
pub(crate) fn manual() -> EntryFilter {
    EntryFilter {
        sort: SearchSort::Relevance,
        ..Default::default()
    }
}

#[test]
fn whole_json_fences_and_consistent_duplicate_conditions_keep_strict_validation() {
    let m = search_filter::Metadata {
        sources: vec![],
        tags: vec![],
    };
    let mut raw = plan("models");
    raw["filter"] = json!({"recent_days":7,"starred_only":true});
    raw["routes"][0]["recent_days"] = json!(7);
    raw["routes"][0]["starred_only"] = json!(true);
    let (normalized, _) = parse_plan(
        &format!("```json\r\n{raw}\r\n```"),
        "recent models",
        &m,
        &manual(),
    )
    .unwrap();
    assert!(normalized
        .routes
        .iter()
        .all(|r| r.recent_days.is_none() && r.starred_only.is_none()));
    raw["routes"][0]["recent_days"] = json!(30);
    assert!(parse_plan(&raw.to_string(), "models", &m, &manual()).is_err());
    raw["routes"][0]["recent_days"] = json!(7);
    raw["routes"][0]["sql"] = json!("SELECT *");
    assert!(parse_plan(&raw.to_string(), "models", &m, &manual()).is_err());
    assert!(parse_plan(
        &format!("Explanation\n```json\n{}\n```", plan("models")),
        "models",
        &m,
        &manual()
    )
    .is_err());
    let c = Candidate {
        entry_id: 1,
        title: "release".into(),
        published_at: None,
        chunks: vec![Chunk {
            key: "body.1".into(),
            text: "A real database performance benchmark.".into(),
        }],
        limited: false,
    };
    let raw = json!({"judgments":[{"entry_id":1,"score":90,"reason":"Relevant benchmark","evidence":{"key":"body.1","quote":"real database performance"}}]});
    assert!(parse_judgments(&format!("```json\n{raw}\n```"), std::slice::from_ref(&c)).is_ok());
    assert!(parse_judgments(&format!("```json\n{raw}\n``` trailing prose"), &[c]).is_err());
}
pub(crate) fn plan(topic: &str) -> Value {
    let (intent,zh,en,event)=match topic{
        "database"=>("Database query performance and acceleration",vec!["数据库","查询","sql"],vec!["database","sql","postgresql","query"],vec!["performance","latency","benchmark","fast","加速","性能"]),
        "tools"=>("New releases of open-source build tools, not tutorials or hiring",vec!["开源","构建工具"],vec!["open-source","build tool","toolforge"],vec!["release","available","发布","新版"]),
        "none"=>("Subsea volcanic acoustic discoveries",vec!["海底火山","声学"],vec!["subsea volcano","acoustic monitoring"],vec!["discovery","发现"]),
        _=>("Newly announced or released AI/language/foundation model versions; exclude finance, tutorials and car models",vec!["人工智能","大语言模型","语言模型"],vec!["ai","language model","foundation model","multimodal model"],vec!["release","introduc","unveil","launch","发布","开放权重"]),
    };
    json!({"intent":intent,"entities":[],"routes":[{"language":"zh","groups":[zh.clone(),event.clone()]},{"language":"en","groups":[en.clone(),event]},{"language":"zh","groups":[zh]},{"language":"en","groups":[en]}],"filter":{},"unsupported":[]})
}
#[test]
fn plan_concepts_or_routes_bounds_hard_filters_and_unknown_entity() {
    let m = search_filter::Metadata {
        sources: vec![],
        tags: vec![],
    };
    let (p, f) = parse_plan(&plan("models").to_string(), "新的AI模型", &m, &manual()).unwrap();
    assert!(f.recent_days.is_none() && f.search.is_none());
    assert_eq!(f.sort, SearchSort::Relevance);
    assert!(p.routes.iter().any(|r| r.groups.len() == 1));
    let (p, _) = parse_plan(
        &plan("models").to_string(),
        "Nebula-Q47 新模型发布",
        &m,
        &manual(),
    )
    .unwrap();
    assert!(p
        .routes
        .iter()
        .any(|r| r.groups == vec![vec!["nebula-q47"]]));
    for field in [
        json!({"search":"AI 人工智能 launch 发布"}),
        json!({"ai_search_id":"invented"}),
        json!({"sql":"select *"}),
        json!({"recent_days":0}),
        json!({"search_field":"semantic"}),
        json!({"recent_days":7,"date_from":"2026-01-01"}),
    ] {
        let mut raw = plan("models");
        raw["filter"] = field;
        assert!(parse_plan(&raw.to_string(), "新的AI模型", &m, &manual()).is_err());
    }
    let mut raw = plan("models");
    raw["filter"] = json!({"search_field":"note"});
    assert_eq!(
        parse_plan(&raw.to_string(), "新的AI模型", &m, &manual())
            .unwrap_err()
            .code,
        "unsupported"
    );
    assert!(parse_plan(&raw.to_string(), "在笔记中查找新的AI模型", &m, &manual()).is_ok());
    let mut raw = plan("models");
    raw["routes"][0]["groups"][0] = json!([]);
    assert!(parse_plan(&raw.to_string(), "theme", &m, &manual()).is_err());
    raw["routes"][0]["groups"][0] =
        json!((0..17).map(|i| format!("literal-{i}")).collect::<Vec<_>>());
    assert!(parse_plan(&raw.to_string(), "theme", &m, &manual()).is_err());
    let mut raw = plan("models");
    raw["filter"] = json!({"recent_days":7});
    let mut base = manual();
    base.recent_days = Some(30);
    assert_eq!(
        parse_plan(&raw.to_string(), "theme", &m, &base)
            .unwrap_err()
            .code,
        "conflict"
    );
}
#[test]
fn judgments_require_every_real_candidate_unique_scores_and_exact_evidence() {
    let cs = vec![Candidate {
        entry_id: 1,
        title: "Release".into(),
        published_at: None,
        chunks: vec![Chunk {
            key: "body.1".into(),
            text: "We release a new language model.".into(),
        }],
        limited: false,
    }];
    let good = json!({"judgments":[{"entry_id":1,"score":95,"reason":"A release","evidence":{"key":"body.1","quote":"new language model"}}]});
    assert!(parse_judgments(&good.to_string(), &cs).is_ok());
    for change in [
        json!({"entry_id":99,"score":95,"reason":"x","evidence":null}),
        json!({"entry_id":1,"score":101,"reason":"x","evidence":null}),
        json!({"entry_id":1,"score":95,"reason":"x","evidence":{"key":"body.1","quote":"invented quotation"}}),
        json!({"entry_id":1,"score":95,"reason":"x","evidence":null}),
        json!({"entry_id":1,"score":"95","reason":"x","evidence":null}),
        json!({"entry_id":1,"score":0,"reason":"x","evidence":null,"sql":"bad"}),
    ] {
        assert!(parse_judgments(&json!({"judgments":[change]}).to_string(), &cs).is_err());
    }
    assert!(parse_judgments(
        &json!({"judgments":[good["judgments"][0].clone(),good["judgments"][0].clone()]})
            .to_string(),
        &cs
    )
    .is_err());
    assert!(parse_judgments("{\"judgments\":[]}", &cs).is_err());
}

pub(crate) async fn config(p: &Pool, url: &str) -> Configuration {
    let mut c = super::super::configuration::load(p, std::path::Path::new(""), "summary")
        .await
        .unwrap();
    c.routes = vec![super::super::configuration::Route {
        provider_id: "fixture".into(),
        provider_name: "Fixture".into(),
        base_url: url.into(),
        model_id: "fixture".into(),
        model_name: "fixture".into(),
        api_key: "synthetic".into(),
    }];
    c
}
pub(crate) async fn scope(
    p: &Pool,
    s: &store::Session,
    limit: u32,
    cursor: Option<crate::db::models::EntryCursor>,
) -> crate::db::models::EntryPage {
    let mut f = s.filter.clone();
    f.ai_search_id = Some(s.id.clone());
    entry_store::query_entries(
        p,
        &EntryQueryParams {
            filter: f,
            cursor,
            limit,
        },
    )
    .await
    .unwrap()
}

// Mock judgments below test transport/scope/progress, NOT language understanding.
// Actual model quality is measured separately against the human labels.
async fn server(
    topic: &str,
    fail_at: Option<usize>,
    delay: u64,
) -> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let calls = Arc::new(Mutex::new(vec![]));
    let sink = calls.clone();
    let topic = topic.to_owned();
    let task = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let sink = sink.clone();
            let topic = topic.clone();
            tokio::spawn(async move {
                let mut bytes = vec![];
                let mut buf = [0; 8192];
                let req = loop {
                    let n = stream.read(&mut buf).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&buf[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&bytes[..end]);
                        let len = header
                            .lines()
                            .find_map(|l| {
                                l.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap())
                            })
                            .unwrap();
                        if bytes.len() >= end + 4 + len {
                            break serde_json::from_slice::<Value>(&bytes[end + 4..end + 4 + len])
                                .unwrap();
                        }
                    }
                };
                let data: Value =
                    serde_json::from_str(req["messages"][1]["content"].as_str().unwrap()).unwrap();
                let at = {
                    let mut calls = sink.lock().unwrap();
                    calls.push(data.clone());
                    calls.len()
                };
                let reply = if let Some(cs) = data["candidates"].as_array() {
                    json!({"judgments":cs.iter().map(|c|{let id=c["entry_id"].as_i64().unwrap();let positive=corpus()["articles"].as_array().unwrap().iter().find(|a|a["id"]==id).is_some_and(|a|a["topics"].as_array().unwrap().iter().any(|t|t==&topic));let evidence=if positive{let chunk=c["chunks"].as_array().unwrap().iter().find(|chunk|chunk["text"].as_str().unwrap().chars().count()>=4).unwrap();json!({"key":chunk["key"],"quote":chunk["text"].as_str().unwrap().chars().take(200).collect::<String>()})}else{Value::Null};json!({"entry_id":id,"score":if positive{95}else{10},"reason":"Controlled protocol fixture","evidence":evidence})}).collect::<Vec<_>>()})
                } else {
                    let mut plan = plan(&topic);
                    if topic == "hard" {
                        plan["filter"] =
                            json!({"source":"技术来源","recent_days":7,"starred_only":true});
                    }
                    plan
                };
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                let body=json!({"choices":[{"message":{"role":"assistant","content":reply.to_string()},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":50,"total_tokens":150}}).to_string();
                let status = if fail_at == Some(at) { 503 } else { 200 };
                let out=format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                let _ = stream.write_all(out.as_bytes()).await;
            });
        }
    });
    (url, calls, task)
}
#[tokio::test]
async fn controlled_production_http_recall_judge_pages_cache_bulk_scope_and_no_notes() {
    for topic in ["models", "database", "tools", "none", "hard", "entity"] {
        let p = pool().await;
        let (url, calls, server) = server(topic, None, 0).await;
        let cfg = config(&p, &url).await;
        let query = if topic == "entity" {
            "Nebula-Q47 新模型发布"
        } else {
            "synthetic topic"
        };
        let channel = Channel::new(|_| Ok(()));
        let r = run(
            &p,
            &reqwest::Client::new(),
            cfg.clone(),
            query.into(),
            manual(),
            None,
            false,
            &channel,
        )
        .await
        .unwrap();
        assert!(r.error.is_none());
        assert_eq!(r.session.pending, 0, "{topic}");
        let mut found = Vec::new();
        let mut cursor = None;
        loop {
            let page = scope(&p, &r.session, 2, cursor).await;
            for info in &page.search_info {
                assert!(info.ai.is_some());
            }
            found.extend(page.entries.iter().map(|e| e.id));
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        let mut expected = corpus()["articles"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["topics"].as_array().unwrap().iter().any(|t| t == topic))
            .map(|a| a["id"].as_i64().unwrap())
            .collect::<Vec<_>>();
        found.sort();
        expected.sort();
        assert_eq!(
            found, expected,
            "{topic}: controlled recall/scope integration"
        );
        let before = calls.lock().unwrap().len();
        let cached = run(
            &p,
            &reqwest::Client::new(),
            cfg,
            query.into(),
            manual(),
            None,
            false,
            &channel,
        )
        .await
        .unwrap();
        assert!(cached.cached);
        assert_eq!(calls.lock().unwrap().len(), before);
        let payload = calls.lock().unwrap().clone();
        assert!(!serde_json::to_string(&payload)
            .unwrap()
            .contains("SECRET_NOTE"));
        if topic == "models" {
            let late = payload
                .iter()
                .filter_map(|v| v["candidates"].as_array())
                .flatten()
                .find(|c| c["entry_id"] == 5)
                .unwrap();
            assert!(late["chunks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|chunk| chunk["text"]
                    .as_str()
                    .unwrap()
                    .contains("releasing a new AI")));
            assert!(late["chunks"]
                .as_array()
                .unwrap()
                .iter()
                .all(|chunk| chunk["text"].as_str().unwrap().chars().count() <= 700));
        }
        let mut f = r.session.filter.clone();
        f.ai_search_id = Some(r.session.id.clone());
        entry_store::mark_all_read_in_scope(&p, &f).await.unwrap();
        let read = p
            .get()
            .await
            .unwrap()
            .interact(|c| {
                let mut s = c
                    .prepare("SELECT id FROM entry WHERE is_read=1 ORDER BY id")
                    .unwrap();
                s.query_map([], |r| r.get::<_, i64>(0))
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .unwrap()
            })
            .await
            .unwrap();
        assert_eq!(read, expected, "bulk must never become full library");
        assert!(
            crate::db::smart_view_store::save(&p, None, "must reject".into(), f.clone())
                .await
                .is_err()
        );
        f.has_note = Some(true);
        assert!(entry_store::query_entries(
            &p,
            &EntryQueryParams {
                filter: f,
                cursor: None,
                limit: 100
            }
        )
        .await
        .is_err());
        server.abort();
    }
}
#[tokio::test]
async fn partial_failure_resume_forks_without_changing_published_pages() {
    let p = pool().await;
    p.get().await.unwrap().interact(|c|{for id in 100..170{c.execute("INSERT INTO entry(id,feed_id,title,summary,published_at) VALUES(?1,1,'AI model launch commentary','Financing-only AI company news',datetime('now'))",[id]).unwrap();}}).await.unwrap();
    let (url, calls, server) = server("models", Some(3), 0).await;
    let cfg = config(&p, &url).await;
    let channel = Channel::new(|_| Ok(()));
    let first = run(
        &p,
        &reqwest::Client::new(),
        cfg.clone(),
        "新的AI模型".into(),
        manual(),
        None,
        false,
        &channel,
    )
    .await
    .unwrap();
    assert_eq!(first.session.checked, BATCH_SIZE);
    assert!(first.error.is_some());
    assert!(first.session.pending > 0);
    let parent_rows = scope(&p, &first.session, 100, None)
        .await
        .entries
        .iter()
        .map(|e| e.id)
        .collect::<Vec<_>>();
    let second = run(
        &p,
        &reqwest::Client::new(),
        cfg,
        "新的AI模型".into(),
        manual(),
        Some(first.session.id.clone()),
        false,
        &channel,
    )
    .await
    .unwrap();
    assert!(second.error.is_none());
    assert_ne!(first.session.id, second.session.id);
    assert!(second.session.checked > first.session.checked);
    assert_eq!(
        scope(&p, &first.session, 100, None)
            .await
            .entries
            .iter()
            .map(|e| e.id)
            .collect::<Vec<_>>(),
        parent_rows
    );
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|v| v.get("candidates").is_none())
            .count(),
        1,
        "resume reuses plan and committed judgments"
    );
    server.abort();
}
#[tokio::test]
async fn cancellation_stops_live_http_and_new_identity_isolated() {
    let p = pool().await;
    let (url, calls, server) = server("models", None, 5000).await;
    let cfg = config(&p, &url).await;
    let rt = super::super::runtime::create_runtime();
    let r = rt.clone();
    let db = p.clone();
    let task = tokio::spawn(async move {
        super::super::runtime::execute(
            &r,
            super::super::runtime::TaskType::SearchFilter,
            0,
            "cancel",
            &Channel::new(|_| Ok(())),
            || async {
                Ok(run(
                    &db,
                    &reqwest::Client::new(),
                    cfg,
                    "新的AI模型".into(),
                    manual(),
                    None,
                    false,
                    &Channel::new(|_| Ok(())),
                )
                .await)
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while calls.lock().unwrap().is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    rt.lock()
        .unwrap()
        .cancel(
            super::super::runtime::TaskType::SearchFilter,
            "cancel",
            false,
        )
        .unwrap();
    assert!(task.await.unwrap().is_err());
    assert_eq!(calls.lock().unwrap().len(), 1);
    server.abort();
}
