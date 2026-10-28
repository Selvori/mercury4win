use super::*;
use crate::db::{
    entry_store::{self, EntryQueryParams},
    migrations, staged_search_store as store,
};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(crate) fn extra() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/staged_search/corpus_extra.json"
    ))
    .unwrap()
}

#[tokio::test]
async fn keyword_results_arrive_before_title_http_and_evolve_without_mutating_old_scope() {
    let p = pool().await;
    p.get().await.unwrap().interact(|c| {
        c.execute_batch("INSERT INTO entry(id,feed_id,title,summary,published_at) VALUES(700,1,'Release calendar for a local museum','A museum schedule.',datetime('now')),(701,1,'Community bulletin','A museum will release its floor plan.',datetime('now'));UPDATE entry SET title='EchoChess 1.2 decoder',summary='Checkpoints for multilingual reasoning' WHERE id=104;UPDATE content SET markdown='Fresh decoder checkpoints enable multilingual conversation. The new weights and tokenizer are available today.' WHERE entry_id=104;").unwrap();
    }).await.unwrap();
    let srv = server("models").await;
    let cfg = config(&p, &srv.url).await;
    let hold = srv.title_gate.clone().acquire_owned().await.unwrap();
    let events = Arc::new(Mutex::new(Vec::<Value>::new()));
    let sink = events.clone();
    let channel = Channel::new(move |body| {
        let raw = body.deserialize::<String>().unwrap();
        sink.lock()
            .unwrap()
            .push(serde_json::from_str(&raw).unwrap());
        Ok(())
    });
    let db = p.clone();
    let task = tokio::spawn(async move {
        run(
            &db,
            &reqwest::Client::new(),
            cfg,
            "新的AI模型".into(),
            manual(),
            None,
            false,
            "quick-first".into(),
            &channel,
        )
        .await
        .unwrap()
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if srv
                .calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| c["stage"] == "titles")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(!task.is_finished(), "title HTTP must still be gated");
    let first: Session = serde_json::from_value(
        events
            .lock()
            .unwrap()
            .iter()
            .find(|e| e["type"] == "search_snapshot")
            .unwrap()["session"]
            .clone(),
    )
    .unwrap();
    assert!(first.preliminary > 0);
    assert_eq!(first.matches, first.preliminary);
    assert_eq!(first.scan.as_ref().unwrap().title_screened, 0);
    assert_eq!(first.scan.as_ref().unwrap().matches, 0);
    let initial = result_ids(&p, &first, 3).await;
    assert!(
        initial.contains(&1) && initial.contains(&2),
        "cross-language keywords"
    );
    assert!(
        initial.contains(&700) && initial.contains(&701),
        "one event word matches without the subject/concept groups"
    );
    assert!(
        !initial.contains(&104),
        "semantic discovery cannot require a keyword match"
    );
    let mut filter = first.filter.clone();
    filter.ai_search_id = Some(first.id.clone());
    let page = entry_store::query_entries(
        &p,
        &EntryQueryParams {
            filter: filter.clone(),
            cursor: None,
            limit: 100,
        },
    )
    .await
    .unwrap();
    let order = page.entries.iter().map(|e| e.id).collect::<Vec<_>>();
    assert!(
        order.iter().position(|id| *id == 700).unwrap()
            < order.iter().position(|id| *id == 701).unwrap(),
        "title keyword matches outrank a single summary match"
    );
    assert!(page.search_info.iter().all(|s| s
        .ai
        .as_ref()
        .unwrap()
        .basis
        .as_ref()
        .unwrap()
        .starts_with("keyword_")));
    entry_store::mark_all_read_in_scope(&p, &filter)
        .await
        .unwrap();
    drop(hold);
    let done = task.await.unwrap();
    assert!(done.error.is_none(), "{:?}", done.error);
    assert_eq!(done.session.preliminary, 0);
    let final_ids = result_ids(&p, &done.session, 3).await;
    assert!(final_ids.contains(&104));
    assert!(!final_ids.contains(&700) && !final_ids.contains(&701));
    assert_eq!(
        result_ids(&p, &first, 3).await,
        initial,
        "old visible membership is immutable"
    );
    assert!(events
        .lock()
        .unwrap()
        .iter()
        .filter(|e| e["type"] == "search_snapshot")
        .any(|e| e["session"]["preliminary"].as_u64().unwrap_or(0) < first.preliminary as u64));
    srv.task.abort();
}

#[tokio::test]
async fn quick_matches_survive_failure_and_notes_require_explicit_field() {
    let p = pool().await;
    let srv = server("models").await;
    let cfg = config(&p, &srv.url).await;
    *srv.fail.lock().unwrap() = Some("titles".into());
    let r = run(
        &p,
        &reqwest::Client::new(),
        cfg,
        "新的AI模型".into(),
        manual(),
        None,
        false,
        "failed-quick".into(),
        &Channel::new(|_| Ok(())),
    )
    .await
    .unwrap();
    assert!(r.error.is_some());
    assert!(r.session.preliminary > 0);
    assert_eq!(r.session.scan.as_ref().unwrap().matches, 0);
    let recent = store::recent(&p).await.unwrap().unwrap();
    assert_eq!(recent.session.unwrap().preliminary, r.session.preliminary);
    for (field, expected) in [
        (crate::db::search_store::SearchField::All, 0),
        (crate::db::search_store::SearchField::Note, 1),
    ] {
        let mut filter = manual();
        filter.search_field = field;
        let mut plan: Plan = serde_json::from_value(topic_search::tests::plan("models")).unwrap();
        for route in &mut plan.routes {
            route.groups = vec![vec!["secret_note".into()]];
        }
        let scan = store::create(
            &p,
            format!("privacy-{expected}"),
            "config".into(),
            "SECRET_NOTE".into(),
            filter.clone(),
            plan,
            filter,
            "privacy".into(),
            false,
        )
        .await
        .unwrap();
        let session = store::publish(&p, &scan).await.unwrap();
        assert_eq!(session.preliminary, expected);
        if expected == 1 {
            let mut filter = session.filter.clone();
            filter.ai_search_id = Some(session.id.clone());
            p.get().await.unwrap().interact(|c|{c.execute("UPDATE entry_note SET markdown='Changed private synthetic note' WHERE entry_id=1",[]).unwrap();}).await.unwrap();
            assert!(entry_store::mark_all_read_in_scope(&p, &filter)
                .await
                .unwrap_err()
                .contains("AI_SEARCH_STALE"));
        }
    }
    srv.task.abort();
}

#[tokio::test]
async fn newest_100_applies_hard_scope_before_limit_and_retires_old_resumes() {
    let p = pool().await;
    p.get().await.unwrap().interact(|c| {
        for id in 2000..2150 {
            c.execute("INSERT INTO entry(id,feed_id,title,published_at) VALUES(?1,1,'Unrelated football match','2100-01-01T00:00:00Z')",[id]).unwrap();
            c.execute("INSERT INTO entry(id,feed_id,title,published_at) VALUES(?1,2,'A newer article from another source','2100-02-01T00:00:00Z')",[id+1000]).unwrap();
        }
    }).await.unwrap();
    let srv = server("models").await;
    *srv.fail.lock().unwrap() = Some("titles".into());
    let cfg = config(&p, &srv.url).await;
    let mut filter = manual();
    filter.feed_id = Some(1);
    filter.sort = crate::db::search_store::SearchSort::Oldest;
    let result = run(
        &p,
        &reqwest::Client::new(),
        cfg.clone(),
        "新的AI模型".into(),
        filter.clone(),
        None,
        false,
        "latest-100".into(),
        &Channel::new(|_| Ok(())),
    )
    .await
    .unwrap();
    assert_eq!(result.session.total, 100);
    let scan_id = result.session.scan.as_ref().unwrap().scan_id.clone();
    let captured = scan_id.clone();
    let ids = p
        .get()
        .await
        .unwrap()
        .interact(move |c| {
            c.prepare("SELECT entry_id FROM ai_topic_item WHERE scan_id=?1 ORDER BY entry_id")
                .unwrap()
                .query_map([captured], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        })
        .await
        .unwrap();
    assert_eq!(ids,(2050..2150).collect::<Vec<_>>(),"filter first; newest publication then ID; semantic clues and local oldest sort cannot pull older sources into scope");
    assert!(srv
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|v| v["stage"] == "titles")
        .flat_map(|v| v["titles"].as_array().unwrap())
        .all(|v| ids.contains(&v["entry_id"].as_i64().unwrap())));
    // Old full-population work stays in SQLite, but cannot be offered/resumed
    // under the new scope policy. Its immutable result token remains readable.
    let retired = scan_id.clone();
    p.get()
        .await
        .unwrap()
        .interact(move |c| {
            c.execute(
                "UPDATE ai_topic_scan SET algorithm='topic-search-staged-v2' WHERE id=?1",
                [retired],
            )
            .unwrap();
        })
        .await
        .unwrap();
    assert!(store::recent(&p).await.unwrap().is_none());
    assert_eq!(
        run(
            &p,
            &reqwest::Client::new(),
            cfg,
            "新的AI模型".into(),
            filter,
            Some(scan_id),
            false,
            "old-resume".into(),
            &Channel::new(|_| Ok(()))
        )
        .await
        .err()
        .unwrap()
        .code,
        "expired"
    );
    let token = result.session.id;
    p.get()
        .await
        .unwrap()
        .interact(move |c| {
            assert_eq!(store::session(c, &token).unwrap().total, 100);
        })
        .await
        .unwrap();
    srv.task.abort();
}

#[tokio::test]
async fn invalid_third_title_batch_retains_sources_within_latest_100() {
    let p = pool().await;
    p.get().await.unwrap().interact(|c| {
        for id in 1000..1538 {
            c.execute("INSERT INTO entry(id,feed_id,title,summary) VALUES(?1,1,'Home club wins another football match','Three goals decided the match.')", [id]).unwrap();
        }
    }).await.unwrap();
    let srv = server("models").await;
    *srv.fail.lock().unwrap() = Some("invalid_titles_after_2".into());
    let cfg = config(&p, &srv.url).await;
    let result = run(
        &p,
        &reqwest::Client::new(),
        cfg,
        "新的AI模型".into(),
        manual(),
        None,
        false,
        "invalid-title".into(),
        &Channel::new(|_| Ok(())),
    )
    .await
    .unwrap();
    assert!(result.error.is_none(), "{:?}", result.error);
    let progress = result.session.scan.as_ref().unwrap();
    assert_eq!(progress.title_screened, 100);
    assert_eq!(progress.title_fallback, 16);
    assert_eq!(progress.title_model_reviewed, 82);
    assert_eq!(progress.pending, 0);
    assert_eq!(progress.failed, 0);
    assert_eq!(
        result_ids(&p, &result.session, 100).await,
        vec![1, 2, 3, 4, 5, 6, 100, 101, 102, 103, 104, 107]
    );
    let scan_id = progress.scan_id.clone();
    p.get().await.unwrap().interact(move |c| {
        assert_eq!(c.query_row("SELECT count(*) FROM ai_topic_item WHERE scan_id=?1 AND title_model=2 AND stage IN ('title','rejected_title')",[&scan_id],|r|r.get::<_,i64>(0)).unwrap(),0);
        assert_eq!(c.query_row("SELECT count(*) FROM ai_topic_item WHERE scan_id=?1 AND entry_id=999999",[&scan_id],|r|r.get::<_,i64>(0)).unwrap(),0);
    }).await.unwrap();
    let scan = store::load(&p, &progress.scan_id).await.unwrap();
    // No fallback is cached as an AI title decision, even after content checks.
    let keys = p
        .get()
        .await
        .unwrap()
        .interact(|c| {
            c.prepare("SELECT result_json FROM ai_topic_cache WHERE stage='title'")
                .unwrap()
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        })
        .await
        .unwrap();
    assert_eq!(keys.len(), 84); // 82 valid model judgments plus two local blank-title keeps.
    assert_eq!(store::status(&p, &scan).await.unwrap().title_fallback, 16);
    let calls = srv.calls.lock().unwrap();
    let titles = calls
        .iter()
        .filter(|c| c["stage"] == "titles")
        .collect::<Vec<_>>();
    assert_eq!(
        titles.len(),
        7,
        "one request per finite batch; no retry for bad model output"
    );
    let bad_ids = titles[2]["titles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["entry_id"].as_i64().unwrap())
        .collect::<Vec<_>>();
    for id in bad_ids {
        assert!(
            calls.iter().any(|c| c["stage"] == "previews"
                && c["previews"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["entry_id"] == id)),
            "deferred source {id} never reached content review"
        );
    }
    srv.task.abort();
}

#[tokio::test]
async fn title_timeout_resumes_latest_100_in_smaller_batches_without_replanning() {
    let p = pool().await;
    p.get().await.unwrap().interact(|c| {
        for id in 1000..1538 {
            c.execute("INSERT INTO entry(id,feed_id,title,summary) VALUES(?1,1,'Home club wins another football match','Three goals decided the match.')", [id]).unwrap();
        }
    }).await.unwrap();
    let srv = server("models").await;
    let cfg = config(&p, &srv.url).await;
    let attempted = Arc::new(Mutex::new(vec![]));
    let sink = attempted.clone();
    let timeout: Arc<
        dyn Fn(
                &provider::ChatCompletionRequest,
            ) -> Result<Option<provider::ChatCompletionResponse>, String>
            + Send
            + Sync,
    > = Arc::new(move |request| {
        let data: Value = serde_json::from_str(&request.messages[1].content).unwrap();
        if data["stage"] == "titles" {
            sink.lock()
                .unwrap()
                .push(data["titles"].as_array().unwrap().len());
            return Err("Timeout: synthetic HTTP deadline".into());
        }
        Ok(None)
    });
    let first = provider::TEST_REPLAY
        .scope(
            timeout,
            run(
                &p,
                &reqwest::Client::new(),
                cfg.clone(),
                "新的AI模型".into(),
                manual(),
                None,
                false,
                "timeout".into(),
                &Channel::new(|_| Ok(())),
            ),
        )
        .await
        .unwrap();
    assert_eq!(first.error.unwrap().code, "timeout");
    assert_eq!(
        *attempted.lock().unwrap(),
        vec![16],
        "single bounded attempt, no automatic paid retry"
    );
    let progress = first.session.scan.unwrap();
    assert_eq!(progress.total, 100);
    assert_eq!(progress.failed, 16);
    assert_eq!(
        store::title_batch_limit(&p, &progress.scan_id)
            .await
            .unwrap(),
        8
    );
    // An older persisted 64-title failure also recovers using the new maximum.
    let scan_id = progress.scan_id.clone();
    p.get().await.unwrap().interact(move |c| {
        c.execute("UPDATE ai_topic_item SET error='timeout' WHERE scan_id=?1 AND entry_id IN(SELECT entry_id FROM ai_topic_item WHERE scan_id=?1 AND stage='title' ORDER BY entry_id LIMIT 64)", [&scan_id]).unwrap();
    }).await.unwrap();
    assert_eq!(
        store::title_batch_limit(&p, &progress.scan_id)
            .await
            .unwrap(),
        16
    );
    let scan_id = progress.scan_id.clone();
    p.get().await.unwrap().interact(move |c| {
        c.execute("UPDATE ai_topic_item SET error=NULL WHERE scan_id=?1", [&scan_id]).unwrap();
        c.execute("UPDATE ai_topic_item SET error='timeout' WHERE scan_id=?1 AND entry_id IN(SELECT entry_id FROM ai_topic_item WHERE scan_id=?1 AND stage='title' ORDER BY entry_id LIMIT 16)", [&scan_id]).unwrap();
    }).await.unwrap();
    let start = srv.calls.lock().unwrap().len();
    let done = run(
        &p,
        &reqwest::Client::new(),
        cfg,
        "新的AI模型".into(),
        manual(),
        Some(progress.scan_id),
        false,
        "resume-small".into(),
        &Channel::new(|_| Ok(())),
    )
    .await
    .unwrap();
    assert!(done.error.is_none(), "{:?}", done.error);
    let complete = done.session.scan.as_ref().unwrap();
    assert_eq!(complete.title_screened, 100);
    assert_eq!(complete.pending, 0);
    assert_eq!(complete.failed, 0);
    assert_eq!(
        result_ids(&p, &done.session, 100).await,
        vec![1, 2, 3, 4, 5, 6, 100, 101, 102, 103, 104, 107]
    );
    let calls = srv.calls.lock().unwrap();
    assert!(!calls[start..].iter().any(|c| c["stage"] == "understanding"));
    let batches = calls[start..]
        .iter()
        .filter(|c| c["stage"] == "titles")
        .collect::<Vec<_>>();
    assert!(batches.len() > 10);
    assert!(batches
        .iter()
        .all(|c| c["titles"].as_array().unwrap().len() <= 8));
    srv.task.abort();
}

#[tokio::test]
async fn every_labelled_topic_and_hard_scope_uses_finite_population() {
    for case in extra()["cases"].as_array().unwrap() {
        let p = pool().await;
        let srv = server(case["topic"].as_str().unwrap()).await;
        let cfg = config(&p, &srv.url).await;
        let r = run(
            &p,
            &reqwest::Client::new(),
            cfg,
            case["query"].as_str().unwrap().into(),
            manual(),
            None,
            false,
            "all-cases".into(),
            &Channel::new(|_| Ok(())),
        )
        .await
        .unwrap();
        assert!(r.error.is_none(), "{}: {:?}", case["key"], r.error);
        assert_eq!(
            result_ids(&p, &r.session, 4).await,
            case["expected"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap())
                .collect::<Vec<_>>(),
            "{}",
            case["key"]
        );
        let progress = r.session.scan.as_ref().unwrap();
        assert_eq!(progress.title_screened, progress.total);
        assert_eq!(progress.pending, 0);
        if case["topic"] != "hard" {
            assert_eq!(progress.total, 71);
        } else {
            assert!(progress.total < 71);
        }
        srv.task.abort();
    }
}

#[tokio::test]
async fn frozen_material_and_selected_field_caches_do_not_cross_contaminate() {
    let p = pool().await;
    async fn scan(p: &Pool, field: crate::db::search_store::SearchField, id: &str) -> store::Scan {
        let mut filter = manual();
        filter.search_field = field;
        let metadata = crate::agent::search_filter::metadata(p).await.unwrap();
        let (plan, filter) = topic_search::parse_plan(
            &crate::agent::topic_search::tests::plan("models").to_string(),
            "新的AI模型",
            &metadata,
            &filter,
        )
        .unwrap();
        store::create(
            p,
            id.into(),
            "config".into(),
            "新的AI模型".into(),
            manual(),
            plan,
            filter,
            id.into(),
            false,
        )
        .await
        .unwrap()
    }
    let first = scan(&p, crate::db::search_store::SearchField::Summary, "summary").await;
    let item = store::pending(&p, &first, "title", 100)
        .await
        .unwrap()
        .into_iter()
        .find(|i| i.title.entry_id == 1)
        .unwrap();
    let summary = store::material(&p, &first, &item).await.unwrap().unwrap();
    assert_eq!(summary.basis, "summary_only");
    assert!(!summary.text.contains("SECRET_NOTE"));
    let note = scan(&p, crate::db::search_store::SearchField::Note, "note").await;
    let note_item = store::pending(&p, &note, "title", 100)
        .await
        .unwrap()
        .into_iter()
        .find(|i| i.title.entry_id == 1)
        .unwrap();
    let material = store::material(&p, &note, &note_item)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(material.basis, "note_full");
    assert!(material.text.contains("SECRET_NOTE"));
    assert_ne!(material.text, summary.text);
    let body = scan(&p, crate::db::search_store::SearchField::Body, "body").await;
    let items = store::pending(&p, &body, "title", 100).await.unwrap();
    let captured = items.iter().find(|i| i.title.entry_id == 1).unwrap();
    let old = store::material(&p, &body, captured).await.unwrap().unwrap();
    p.get().await.unwrap().interact(|c|{c.execute("UPDATE content SET markdown='Updated entirely different text' WHERE entry_id IN(1,2)",[]).unwrap();c.execute("INSERT INTO entry(id,feed_id,title) VALUES(999,1,'New arrival outside frozen title population')",[]).unwrap();}).await.unwrap();
    assert_eq!(
        store::material(&p, &body, captured)
            .await
            .unwrap()
            .unwrap()
            .text,
        old.text,
        "already captured version must remain stable"
    );
    let uncaptured = items.iter().find(|i| i.title.entry_id == 2).unwrap();
    assert!(store::material(&p, &body, uncaptured)
        .await
        .unwrap()
        .is_none());
    assert_eq!(store::status(&p, &body).await.unwrap().changed, 1);
    assert_eq!(store::status(&p, &body).await.unwrap().total, 71);
}
pub(crate) fn articles() -> Vec<Value> {
    let mut articles = super::super::topic_search::tests::corpus()["articles"]
        .as_array()
        .unwrap()
        .clone();
    articles.extend(extra()["articles"].as_array().unwrap().clone());
    for i in 0..40 {
        articles.push(if i%2==0{json!({"id":300+i,"title":format!("Home club wins the away match {}",i),"summary":"The home supporters celebrated a decisive second goal.","body":"There were three goals before the whistle. The visitors defended the western end of the pitch, while the home side scored from two corners.","days":1,"starred":false,"feed":1,"topics":[]})}else{json!({"id":300+i,"title":format!("高温烤箱让面包表皮更酥脆 {}",i),"summary":"面包师记录了烘焙温度和面团含水量。","body":"面包师在烘焙前预热烤箱，记录面团发酵时间和含水量。较高的初始温度有助于形成更酥脆的表皮，最后十分钟降低温度使内部熟透。","days":2,"starred":false,"feed":2,"topics":[]})});
    }
    articles
}
pub(crate) fn seed(c: &mut rusqlite::Connection) {
    c.execute_batch("DELETE FROM entry_tag;DELETE FROM entry_note;DELETE FROM content;DELETE FROM entry;DELETE FROM feed;DELETE FROM tag;INSERT INTO feed(id,title,feed_url) VALUES(1,'技术来源','https://tech.test/rss'),(2,'新闻来源','https://news.test/rss');INSERT INTO tag(id,name,normalized_name) VALUES(1,'Rust','rust'),(2,'SQLite','sqlite');").unwrap();
    for a in articles() {
        let id = a["id"].as_i64().unwrap();
        c.execute("INSERT INTO entry(id,feed_id,guid,title,summary,published_at,is_starred,is_read) VALUES(?1,?2,?3,?4,?5,datetime('now',?6),?7,0)",rusqlite::params![id,a["feed"].as_i64().unwrap(),format!("staged-{id}"),a["title"].as_str(),a["summary"].as_str(),format!("-{} days",a["days"].as_i64().unwrap()),a["starred"].as_bool().unwrap()]).unwrap();
        if let Some(body) = a["body"].as_str() {
            let prefix = a["prefix"]
                .as_str()
                .unwrap_or("Routine infrastructure maintenance and storage notes. ");
            let body = format!(
                "{}{}{}",
                a["opening"].as_str().unwrap_or(""),
                prefix.repeat(a["prefix_repeat"].as_u64().unwrap_or(0) as usize),
                body
            );
            c.execute(
                "INSERT INTO content(entry_id,markdown) VALUES(?1,?2)",
                rusqlite::params![id, body],
            )
            .unwrap();
        }
    }
    c.execute_batch("INSERT INTO entry_note VALUES(1,'SECRET_NOTE not to be sent by default',datetime('now'));INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,1);").unwrap();
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
    crate::agent::topic_search::tests::manual()
}
pub(crate) async fn config(p: &Pool, url: &str) -> Configuration {
    crate::agent::topic_search::tests::config(p, url).await
}
pub(crate) async fn result_ids(p: &Pool, session: &Session, limit: u32) -> Vec<i64> {
    let mut f = session.filter.clone();
    f.ai_search_id = Some(session.id.clone());
    let mut ids = vec![];
    let mut cursor = None;
    loop {
        let page = entry_store::query_entries(
            p,
            &EntryQueryParams {
                filter: f.clone(),
                cursor,
                limit,
            },
        )
        .await
        .unwrap();
        ids.extend(page.entries.iter().map(|e| e.id));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    ids.sort();
    ids
}

// The loopback fixture uses HUMAN LABELS only to exercise transport and durable
// state. It does not establish model understanding; a separate live evaluation
// compares real model decisions to these labels.
pub(crate) struct Server {
    pub url: String,
    pub calls: Arc<Mutex<Vec<Value>>>,
    pub fail: Arc<Mutex<Option<String>>>,
    pub delay: Arc<std::sync::atomic::AtomicU64>,
    pub title_gate: Arc<tokio::sync::Semaphore>,
    pub task: tokio::task::JoinHandle<()>,
}
pub(crate) async fn server(topic: &str) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let calls = Arc::new(Mutex::new(vec![]));
    let sink = calls.clone();
    let fail = Arc::new(Mutex::new(None));
    let fail_loop = fail.clone();
    let delay = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let slow = delay.clone();
    let title_gate = Arc::new(tokio::sync::Semaphore::new(1));
    let gates = title_gate.clone();
    let topic = topic.to_owned();
    let task = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let sink = sink.clone();
            let fail = fail_loop.clone();
            let slow = slow.clone();
            let topic = topic.clone();
            let gate = gates.clone();
            tokio::spawn(async move {
                let mut bytes = vec![];
                let mut buf = [0; 16384];
                let request = loop {
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
                assert!(
                    request.get("max_tokens").is_none(),
                    "no fixed output cap or total token budget"
                );
                let data: Value =
                    serde_json::from_str(request["messages"][1]["content"].as_str().unwrap())
                        .unwrap();
                sink.lock().unwrap().push(data.clone());
                let stage = data["stage"].as_str().unwrap();
                let fail_at = fail.lock().unwrap().clone();
                let status = if fail_at.as_deref() == Some(stage)
                    || fail_at.as_deref() == Some("full_after_first")
                        && stage == "full"
                        && data["parts"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|p| p["index"].as_u64().unwrap() > 0)
                {
                    503
                } else {
                    200
                };
                let positive = |id: i64| {
                    articles().iter().find(|a| a["id"] == id).is_some_and(|a| {
                        a["topics"].as_array().unwrap().iter().any(|t| t == &topic)
                    })
                };
                let evidence = |key: &str, text: &str| json!({"key":key,"quote":text.chars().take(200).collect::<String>()});
                let mut reply = match stage {
                    "understanding" => {
                        let mut plan = crate::agent::topic_search::tests::plan(&topic);
                        if topic == "hard" {
                            plan["filter"] =
                                json!({"source":"技术来源","recent_days":7,"starred_only":true});
                        }
                        plan
                    }
                    "titles" => {
                        json!({"titles":data["titles"].as_array().unwrap().iter().map(|a|{let id=a["entry_id"].as_i64().unwrap();json!({"entry_id":id,"decision":if id==101||id==109{"unrelated"}else if generic_title(a["title"].as_str().unwrap()){"uncertain"}else if positive(id){"possible"}else{"unrelated"},"reason":"Controlled title classification"})}).collect::<Vec<_>>()})
                    }
                    "previews" => {
                        json!({"previews":data["previews"].as_array().unwrap().iter().map(|a|{let id=a["entry_id"].as_i64().unwrap();let chunk=a["chunks"].as_array().unwrap().iter().find(|c|c["text"].as_str().unwrap().chars().count()>=4).unwrap();json!({"entry_id":id,"decision":if id==101||id==109{"unrelated"}else if positive(id){"possible"}else if a["complete"]==false{"uncertain"}else{"unrelated"},"reason":"Controlled preview decision; misleading title is deliberately challenged","evidence":evidence(chunk["key"].as_str().unwrap(),chunk["text"].as_str().unwrap())})}).collect::<Vec<_>>()})
                    }
                    "full" => {
                        json!({"parts":data["parts"].as_array().unwrap().iter().map(|a|{let id=a["entry_id"].as_i64().unwrap();let text=a["text"].as_str().unwrap();let opposed=id==105&&a["index"].as_u64().unwrap()+1==a["total"].as_u64().unwrap();let kind=if opposed{"opposes"}else if positive(id)||id==105{"supports"}else{"background"};json!({"entry_id":id,"key":a["key"],"kind":kind,"reason":"Controlled complete-part finding","evidence":if kind=="background"{Value::Null}else{evidence(a["key"].as_str().unwrap(),text)},"context":if opposed{"The apparent launch was an old quotation; the new story concerns funding."}else if positive(id){"This frozen article directly addresses the topic."}else{"The material describes another subject."}})}).collect::<Vec<_>>()})
                    }
                    "final" => {
                        json!({"articles":data["articles"].as_array().unwrap().iter().map(|a|{let id=a["entry_id"].as_i64().unwrap();json!({"entry_id":id,"decision":if positive(id){"relevant"}else{"unrelated"},"score":if positive(id){95}else{5},"reason":"Controlled final article decision","evidence":if positive(id){a["evidence"][0].clone()}else{Value::Null},"context_resolution":if a["signals"]["opposes"].as_u64().unwrap_or(0)>0{"The old quotation is not a current release."}else{""}})}).collect::<Vec<_>>()})
                    }
                    _ => panic!("unknown stage"),
                };
                if fail_at.as_deref() == Some("invalid_titles_after_2")
                    && stage == "titles"
                    && sink
                        .lock()
                        .unwrap()
                        .iter()
                        .filter(|c| c["stage"] == "titles")
                        .count()
                        == 3
                {
                    // HTTP succeeds, but an extra foreign ID invalidates the
                    // whole batch. Production must retain original sources.
                    reply["titles"][0]["entry_id"] = json!(999999);
                }
                if stage == "titles" {
                    drop(gate.acquire().await.unwrap());
                }
                tokio::time::sleep(std::time::Duration::from_millis(
                    slow.load(std::sync::atomic::Ordering::SeqCst),
                ))
                .await;
                let body=json!({"choices":[{"message":{"role":"assistant","content":reply.to_string()},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":50,"total_tokens":150}}).to_string();
                let response=format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });
    Server {
        url,
        calls,
        fail,
        delay,
        title_gate,
        task,
    }
}

#[tokio::test]
async fn failed_long_part_resumes_checkpoint_and_keeps_published_scope_immutable() {
    let p = pool().await;
    let srv = server("models").await;
    *srv.fail.lock().unwrap() = Some("full_after_first".into());
    let cfg = config(&p, &srv.url).await;
    let channel = Channel::new(|_| Ok(()));
    let first = run(
        &p,
        &reqwest::Client::new(),
        cfg.clone(),
        "新的AI模型".into(),
        manual(),
        None,
        false,
        "failed".into(),
        &channel,
    )
    .await
    .unwrap();
    assert!(first.error.is_some());
    let progress = first.session.scan.as_ref().unwrap();
    assert_eq!(progress.status, "failed");
    assert_eq!(progress.title_screened, 71);
    assert!(progress.full_parts_done > 0 && progress.pending > 0 && progress.failed > 0);
    let original = result_ids(&p, &first.session, 3).await;
    assert!(!original.is_empty());
    let n = srv.calls.lock().unwrap().len();
    let scan_id = progress.scan_id.clone();
    let recent = store::recent(&p).await.unwrap().unwrap();
    assert_eq!(recent.progress.scan_id, scan_id);
    *srv.fail.lock().unwrap() = None;
    let done = run(
        &p,
        &reqwest::Client::new(),
        cfg,
        "新的AI模型".into(),
        manual(),
        Some(scan_id),
        false,
        "resume".into(),
        &channel,
    )
    .await
    .unwrap();
    assert!(done.error.is_none());
    assert_eq!(done.session.scan.as_ref().unwrap().pending, 0);
    assert_eq!(result_ids(&p, &first.session, 3).await, original);
    let calls = srv.calls.lock().unwrap();
    assert!(!calls[n..]
        .iter()
        .any(|c| ["understanding", "titles", "previews"].contains(&c["stage"].as_str().unwrap())));
    drop(calls);
    assert!(store::recent(&p).await.unwrap().is_none());
    srv.task.abort();
}

#[tokio::test]
async fn immutable_scope_sort_bulk_and_entry_versions() {
    let p = pool().await;
    let srv = server("models").await;
    let cfg = config(&p, &srv.url).await;
    let r = run(
        &p,
        &reqwest::Client::new(),
        cfg.clone(),
        "新的AI模型".into(),
        manual(),
        None,
        false,
        "scope".into(),
        &Channel::new(|_| Ok(())),
    )
    .await
    .unwrap();
    let count = srv.calls.lock().unwrap().len();
    let original = result_ids(&p, &r.session, 2).await;
    let mut f = r.session.filter.clone();
    f.ai_search_id = Some(r.session.id.clone());
    f.sort = crate::db::search_store::SearchSort::Oldest;
    let page = entry_store::query_entries(
        &p,
        &EntryQueryParams {
            filter: f.clone(),
            cursor: None,
            limit: 100,
        },
    )
    .await
    .unwrap();
    assert_eq!(page.entries.len(), original.len());
    assert!(!page.ai_stale);
    entry_store::mark_all_read_in_scope(&p, &f).await.unwrap();
    let ids = p
        .get()
        .await
        .unwrap()
        .interact(|c| {
            c.prepare("SELECT id FROM entry WHERE is_read=1 ORDER BY id")
                .unwrap()
                .query_map([], |r| r.get::<_, i64>(0))
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        })
        .await
        .unwrap();
    assert_eq!(ids, original);
    // Read state and unrelated arrivals do not alter the frozen membership or invalidate it.
    p.get().await.unwrap().interact(|c|{c.execute_batch("INSERT INTO entry(id,feed_id,title) VALUES(999,1,'Another just released AI language model');INSERT INTO content(entry_id,markdown) VALUES(999,'A new foundation model released today.');").unwrap();}).await.unwrap();
    assert_eq!(result_ids(&p, &r.session, 2).await, original);
    assert!(
        !entry_store::query_entries(
            &p,
            &EntryQueryParams {
                filter: f.clone(),
                cursor: None,
                limit: 100
            }
        )
        .await
        .unwrap()
        .ai_stale
    );
    p.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute(
                "UPDATE content SET markdown=markdown||' revised' WHERE entry_id=1",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();
    assert!(
        entry_store::query_entries(
            &p,
            &EntryQueryParams {
                filter: f.clone(),
                cursor: None,
                limit: 100
            }
        )
        .await
        .unwrap()
        .ai_stale
    );
    assert!(entry_store::mark_all_read_in_scope(&p, &f)
        .await
        .unwrap_err()
        .contains("AI_SEARCH_STALE"));
    f.starred_only = true;
    assert!(entry_store::query_entries(
        &p,
        &EntryQueryParams {
            filter: f,
            cursor: None,
            limit: 100
        }
    )
    .await
    .unwrap_err()
    .contains("AI_SEARCH_SCOPE_CHANGED"));
    assert_eq!(srv.calls.lock().unwrap().len(), count);
    srv.task.abort();
}

#[tokio::test]
async fn pause_drops_http_and_resume_owns_commits() {
    use crate::agent::runtime;
    let p = pool().await;
    let srv = server("models").await;
    let cfg = config(&p, &srv.url).await;
    // Save an initial failed checkpoint so the delayed request has a scan to pause.
    *srv.fail.lock().unwrap() = Some("titles".into());
    let first = run(
        &p,
        &reqwest::Client::new(),
        cfg.clone(),
        "新的AI模型".into(),
        manual(),
        None,
        false,
        "first".into(),
        &Channel::new(|_| Ok(())),
    )
    .await
    .unwrap();
    let checkpoint = first.session.scan.unwrap();
    let title_screened = checkpoint.title_screened;
    let scan_id = checkpoint.scan_id;
    *srv.fail.lock().unwrap() = None;
    srv.delay.store(5000, std::sync::atomic::Ordering::SeqCst);
    let rt = runtime::create_runtime();
    let r = rt.clone();
    let db = p.clone();
    let resume = scan_id.clone();
    let n = srv.calls.lock().unwrap().len();
    let task = tokio::spawn(async move {
        runtime::execute(
            &r,
            runtime::TaskType::SearchFilter,
            0,
            "pause",
            &Channel::new(|_| Ok(())),
            || async {
                Ok(run(
                    &db,
                    &reqwest::Client::new(),
                    cfg,
                    "新的AI模型".into(),
                    manual(),
                    Some(resume),
                    false,
                    "pause".into(),
                    &Channel::new(|_| Ok(())),
                )
                .await)
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while srv.calls.lock().unwrap().len() == n {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    rt.lock()
        .unwrap()
        .cancel(runtime::TaskType::SearchFilter, "pause", false)
        .unwrap();
    let stopped = store::stop(&p, "pause", "paused").await.unwrap().unwrap();
    assert_eq!(stopped.status, "paused");
    assert!(task.await.unwrap().is_err());
    assert_eq!(srv.calls.lock().unwrap().len(), n + 1);
    let recent = store::recent(&p).await.unwrap().unwrap();
    assert_eq!(recent.progress.title_screened, title_screened);
    let key = store::input_key(
        "新的AI模型",
        &manual(),
        &config(&p, &srv.url).await.fingerprint,
    );
    let old = store::claim(&p, scan_id.clone(), key.clone(), "old".into())
        .await
        .unwrap();
    let _new = store::claim(&p, scan_id, key, "new".into()).await.unwrap();
    assert_eq!(
        store::set_status(&p, &old, "complete", None, &[])
            .await
            .unwrap_err()
            .code,
        "cancelled"
    );
    srv.task.abort();
}

#[test]
fn strict_title_preview_part_and_final_contracts() {
    let titles = vec![Title {
        entry_id: 1,
        title: "Some title".into(),
        source: "Source".into(),
        published_at: None,
    }];
    let good = json!({"titles":[{"entry_id":1,"decision":"uncertain","reason":"Need content"}]});
    assert!(parse_titles(&format!("```json\n{good}\n```"), &titles).is_ok());
    assert!(parse_titles(&json!([good.clone()]).to_string(), &titles).is_ok());
    for bad in [
        json!([good.clone(), good.clone()]),
        json!([{"titles":good["titles"],"unknown":"unexpected"}]),
        json!([[good.clone()]]),
        json!({"titles":[]}),
        json!({"titles":[good["titles"][0].clone(),good["titles"][0].clone()]}),
        json!({"titles":[{"entry_id":999,"decision":"possible","reason":"X"}]}),
        json!({"titles":[{"entry_id":1,"decision":"possible","reason":"X","sql":"bad"}]}),
    ] {
        assert!(parse_titles(&bad.to_string(), &titles).is_err());
    }
    let m = Material {
        entry_id: 1,
        title: "Some title".into(),
        source: "S".into(),
        published_at: None,
        summary: "".into(),
        text: "An old quotation, contradicted by the current article.".into(),
        headings: vec![],
        basis: "full_body".into(),
        versions: Versions::default(),
    };
    let item = Item {
        title: titles[0].clone(),
        versions: Versions::default(),
        clue: false,
        strong_clue: false,
        generic: false,
        stage: "full".into(),
        material: None,
        parts_done: 0,
        parts_total: 1,
        context: "".into(),
        signals: Signals::default(),
        evidence: vec![],
    };
    let part = part(&m, &item).unwrap();
    let mut response = json!({"parts":[{"entry_id":1,"key":"full.1","kind":"opposes","reason":"Old quote","evidence":{"key":"full.1","quote":"An old quotation"},"context":"This is a historical quotation."}]});
    assert!(parse_parts(&response.to_string(), &[part.clone()]).is_ok());
    response["parts"][0]["evidence"]["quote"] = json!("Fabricated quotation");
    assert!(parse_parts(&response.to_string(), &[part]).is_err());
    let final_article = FinalInput {
        entry_id: 1,
        title: m.title,
        source: m.source,
        published_at: None,
        basis: m.basis,
        context: "old quote".into(),
        signals: Signals {
            opposes: 1,
            ..Default::default()
        },
        evidence: vec![Evidence {
            key: "full.1".into(),
            quote: "An old quotation".into(),
        }],
        parts: 1,
    };
    let mut f = json!({"articles":[{"entry_id":1,"decision":"relevant","score":90,"reason":"Relevant","evidence":{"key":"full.1","quote":"An old quotation"},"context_resolution":""}]});
    assert!(parse_final(&f.to_string(), &[final_article.clone()]).is_err());
    f["articles"][0]["context_resolution"] = Value::Null;
    assert!(
        parse_final(&f.to_string(), &[final_article.clone()]).is_err(),
        "null cannot bypass opposition resolution"
    );
    f["articles"][0]["context_resolution"] =
        json!("A reason that explicitly resolves contradiction");
    assert!(parse_final(&f.to_string(), &[final_article]).is_ok());
}

#[test]
fn real_synthetic_protocol_failures_replay_without_weakening_evidence_checks() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../../tests/staged_search/protocol_cases.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let request = &case["request"];
        let raw = case["response"].as_str().unwrap();
        if request["stage"] == "understanding" {
            let (plan, filter) = topic_search::parse_plan(
                raw,
                "新的AI模型",
                &search_filter::Metadata {
                    sources: vec![],
                    tags: vec![],
                },
                &manual(),
            )
            .unwrap();
            assert!(plan
                .routes
                .iter()
                .any(|r| r.groups.iter().any(|g| g.len() == 9)));
            assert_eq!(filter.recent_days, None);
        } else if request["stage"] == "final" {
            let articles: Vec<FinalInput> =
                serde_json::from_value(request["articles"].clone()).unwrap();
            assert_eq!(parse_final(raw, &articles).unwrap().len(), articles.len());
        } else if request["stage"] == "full" {
            let parts: Vec<Part> = serde_json::from_value(request["parts"].clone()).unwrap();
            let reviews = parse_parts(raw, &parts).unwrap();
            assert_eq!(reviews.len(), parts.len());
            let mut fabricated: Value = serde_json::from_str(raw).unwrap();
            fabricated["parts"][1]["evidence"] =
                json!("An invented quotation absent from this part");
            assert!(parse_parts(&fabricated.to_string(), &parts).is_err());
        } else {
            let previews: Vec<Preview> =
                serde_json::from_value(request["previews"].clone()).unwrap();
            assert_eq!(
                parse_previews(raw, &previews).unwrap().len(),
                previews.len()
            );
            let mut omitted: Value = serde_json::from_str(raw).unwrap();
            if omitted.is_array() {
                omitted.as_array_mut().unwrap().pop();
            } else {
                omitted["previews"].as_array_mut().unwrap().pop();
            }
            assert!(parse_previews(&omitted.to_string(), &previews).is_err());
        }
    }
}

#[test]
fn v9_upgrade_and_material_version_triggers_are_idempotent() {
    let mut c = migrations::historical_fixture(9);
    c.execute_batch("INSERT INTO feed(id,title,feed_url) VALUES(1,'Preserved','https://keep.test/rss');INSERT INTO entry(id,feed_id,title,summary) VALUES(1,1,'Keep','Summary');INSERT INTO entry_note VALUES(1,'Private preserved note',datetime('now'));INSERT INTO content(entry_id,markdown) VALUES(1,'Original body');").unwrap();
    let legacy_filter = serde_json::to_string(&manual()).unwrap();
    let legacy_plan = crate::agent::topic_search::tests::plan("models").to_string();
    c.execute(
        "INSERT INTO smart_view(name,filter_json) VALUES('Preserved view',?1)",
        [&legacy_filter],
    )
    .unwrap();
    c.execute("INSERT INTO ai_search_session(id,input_key,sentence,manual_json,plan_json,filter_json,anchor,revision,flags,published) VALUES('legacy-scope','legacy','旧搜索',?1,?2,?1,'2026-10-28T00:00:00Z',0,0,1)",rusqlite::params![legacy_filter,legacy_plan]).unwrap();
    c.execute_batch("INSERT INTO ai_search_judgment VALUES('legacy-scope',1,90,'Preserved decision','body.1','Original body',0);").unwrap();
    for _ in 0..2 {
        migrations::run_migrations(&mut c).unwrap();
    }
    assert_eq!(
        c.query_row("SELECT name FROM smart_view", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Preserved view"
    );
    assert_eq!(
        c.query_row(
            "SELECT reason FROM ai_search_judgment WHERE session_id='legacy-scope'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "Preserved decision"
    );
    assert_eq!(
        c.query_row(
            "SELECT plan_json FROM ai_search_session WHERE id='legacy-scope'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        legacy_plan
    );

    assert_eq!(
        c.query_row(
            "SELECT markdown FROM entry_note WHERE entry_id=1",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "Private preserved note"
    );
    let initial = store::versions(&c, 1).unwrap().unwrap();
    c.execute("UPDATE entry SET is_read=1,is_starred=1 WHERE id=1", [])
        .unwrap();
    assert_eq!(initial, store::versions(&c, 1).unwrap().unwrap());
    c.execute("UPDATE entry SET title='Changed' WHERE id=1", [])
        .unwrap();
    let title = store::versions(&c, 1).unwrap().unwrap();
    assert_eq!(title.title, initial.title + 1);
    assert_eq!(title.body, initial.body);
    c.execute(
        "UPDATE content SET markdown='New body' WHERE entry_id=1",
        [],
    )
    .unwrap();
    let body = store::versions(&c, 1).unwrap().unwrap();
    assert_eq!(body.body, title.body + 1);
    assert_eq!(body.title, title.title);
    c.execute(
        "UPDATE entry_note SET markdown='New note' WHERE entry_id=1",
        [],
    )
    .unwrap();
    let note = store::versions(&c, 1).unwrap().unwrap();
    assert_eq!(note.note, body.note + 1);
    assert_eq!(note.body, body.body);
    c.execute("UPDATE entry SET summary='New summary' WHERE id=1", [])
        .unwrap();
    let summary = store::versions(&c, 1).unwrap().unwrap();
    assert_eq!(summary.summary, note.summary + 1);
    assert_eq!(
        c.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}

#[tokio::test]
async fn automatic_full_title_scope_complete_parts_results_and_incremental_cache() {
    let p = pool().await;
    let srv = server("models").await;
    let cfg = config(&p, &srv.url).await;
    let events = Arc::new(Mutex::new(vec![]));
    let sink = events.clone();
    let channel = Channel::new(move |data| {
        sink.lock().unwrap().push(format!("{data:?}"));
        Ok(())
    });
    let r = run(
        &p,
        &reqwest::Client::new(),
        cfg.clone(),
        "新的AI模型".into(),
        manual(),
        None,
        false,
        "first".into(),
        &channel,
    )
    .await
    .unwrap();
    assert!(r.error.is_none(), "{:?}", r.error);
    let progress = r.session.scan.as_ref().unwrap();
    assert_eq!(progress.total, 71);
    assert_eq!(progress.title_screened, 71);
    assert_eq!(progress.pending, 0);
    assert_eq!(progress.status, "complete");
    assert_eq!(progress.summary_only, 1);
    assert!(progress.full_verified >= 11);
    assert_eq!(progress.insufficient, 1);
    assert_eq!(
        result_ids(&p, &r.session, 3).await,
        extra()["cases"][0]["expected"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_i64().unwrap())
            .collect::<Vec<_>>()
    );
    let calls = srv.calls.lock().unwrap().clone();
    assert!(!serde_json::to_string(&calls)
        .unwrap()
        .contains("SECRET_NOTE"));
    let title_ids = calls
        .iter()
        .filter(|c| c["stage"] == "titles")
        .flat_map(|c| c["titles"].as_array().unwrap())
        .map(|a| a["entry_id"].as_i64().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(title_ids.len(), 69);
    assert!(title_ids.contains(&104) && title_ids.contains(&339));
    assert!(calls
        .iter()
        .filter(|c| c["stage"] == "full")
        .flat_map(|c| c["parts"].as_array().unwrap())
        .any(|a| a["entry_id"] == 101));
    let full = calls
        .iter()
        .filter(|c| c["stage"] == "full")
        .flat_map(|c| c["parts"].as_array().unwrap())
        .filter(|a| a["entry_id"] == 100)
        .collect::<Vec<_>>();
    assert!(full.len() >= 4);
    assert_eq!(full[0]["start"], 0);
    for pair in full.windows(2) {
        assert_eq!(pair[0]["end"], pair[1]["start"]);
    }
    assert_eq!(
        full.len() as u64,
        full.last().unwrap()["total"].as_u64().unwrap()
    );
    let count = srv.calls.lock().unwrap().len();
    let cached = run(
        &p,
        &reqwest::Client::new(),
        cfg.clone(),
        "新的AI模型".into(),
        manual(),
        None,
        false,
        "cached".into(),
        &channel,
    )
    .await
    .unwrap();
    assert!(cached.cached);
    assert_eq!(srv.calls.lock().unwrap().len(), count);
    assert_eq!(
        result_ids(&p, &cached.session, 100).await,
        result_ids(&p, &r.session, 100).await
    );
    // A body edit reuses title screening but invalidates preview and full review.
    p.get().await.unwrap().interact(|c|{c.execute("UPDATE content SET markdown=markdown||' Additional release context.' WHERE entry_id=1",[]).unwrap();}).await.unwrap();
    let edited = run(
        &p,
        &reqwest::Client::new(),
        cfg,
        "新的AI模型".into(),
        manual(),
        None,
        false,
        "edited".into(),
        &channel,
    )
    .await
    .unwrap();
    assert!(edited.error.is_none());
    let new_calls = srv.calls.lock().unwrap()[count..].to_vec();
    assert!(!new_calls.iter().any(|c| c["stage"] == "titles"));
    assert!(new_calls.iter().any(|c| c["stage"] == "previews"));
    assert!(new_calls.iter().any(|c| c["stage"] == "full"));
    srv.task.abort();
}
