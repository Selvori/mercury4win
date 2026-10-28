//! Explicitly opted-in real API test. Reads only routing tables and one key,
//! reuses the private credential decoder, writes only to an in-memory fixture.
use crate::agent::{configuration, search_filter, usage};
use crate::db::{entry_store, migrations, smart_view_store};
use deadpool_sqlite::Pool;
use rusqlite::{types::Value, Connection, OpenFlags};
use serde_json::json;
use std::path::Path;

pub(super) async fn fixture(app_dir: &Path) -> Pool {
    let pool = deadpool_sqlite::Config::new(":memory:")
        .builder(deadpool_sqlite::Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    let db = app_dir.join("mercury.db");
    pool.get().await.unwrap().interact(move |c| {
        migrations::run_migrations(c).unwrap();
        let source = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY).expect("Open routing database read-only");
        // An explicit allowlist: never read feed, entry, note or content tables.
        for table in ["agent_provider_profile", "agent_model_profile", "agent_profile"] {
            c.execute(&format!("DELETE FROM {table}"), []).unwrap();
            let predicate = if table == "agent_profile" { " WHERE agent_type='summary'" } else { "" };
            let mut statement = source.prepare(&format!("SELECT * FROM {table}{predicate}")).unwrap();
            let columns = statement.column_names().iter().map(|s| s.to_string()).collect::<Vec<_>>();
            assert!(columns.iter().all(|name| name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')));
            let sql = format!("INSERT INTO {table}({}) VALUES({})", columns.join(","), vec!["?";columns.len()].join(","));
            let rows = statement.query_map([], |r| (0..columns.len()).map(|i| r.get::<_,Value>(i)).collect::<rusqlite::Result<Vec<_>>>()).unwrap();
            for row in rows { c.execute(&sql, rusqlite::params_from_iter(row.unwrap())).unwrap(); }
        }
        c.execute_batch("INSERT INTO feed(id,title,feed_url) VALUES(1,'技术','https://a.test/rss'),(2,'News','https://b.test/rss');
            INSERT INTO tag(id,name,normalized_name) VALUES(3,'Rust','rust'),(4,'数据库','数据库');
            INSERT INTO entry(id,feed_id,title,published_at,is_starred,is_read) VALUES
            (1,1,'Rust async',datetime('now','-2 days'),1,0),
            (2,2,'Rust old',datetime('now','-20 days'),1,1),
            (3,1,'Python',datetime('now','-1 day'),0,0);
            INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,3);").unwrap();
    }).await.unwrap();
    pool
}

#[tokio::test]
#[ignore = "Paid API: requires explicit user authorization and M4W_NL_LIVE_APP_DIR / M4W_NL_LIVE_REPORT"]
async fn approved_live_natural_search() {
    let app_dir = std::path::PathBuf::from(
        std::env::var_os("M4W_NL_LIVE_APP_DIR").expect("Explicit app directory required"),
    );
    let report = std::path::PathBuf::from(
        std::env::var_os("M4W_NL_LIVE_REPORT").expect("Explicit new report path required"),
    );
    assert!(!report.exists(), "Refuse to overwrite a report");
    assert!(
        report.parent().is_some_and(Path::is_dir),
        "Report parent directory missing"
    );
    let pool = fixture(&app_dir).await;
    let mut config = configuration::load(&pool, &app_dir.join("prompts"), "summary")
        .await
        .expect("Load configured summary routing");
    assert!(!config.routes.is_empty(), "No summary model configured");
    // Six primary requests at most. No automatic paid retry or fallback in this test.
    config.routes.truncate(1);
    let route = config.routes.first_mut().unwrap();
    assert!(route
        .provider_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    let encrypted = std::fs::read_to_string(
        app_dir
            .join("credentials")
            .join(format!("{}.key", route.provider_id)),
    )
    .expect("Read configured credential");
    route.api_key = super::simple_decrypt(&encrypted);
    assert!(!route.api_key.is_empty(), "Configured key is empty");
    let model = route.model_name.clone();
    let metadata = search_filter::metadata(&pool).await.unwrap();
    let client = reqwest::Client::new();
    let cases = [
        ("basic_zh","找最近一周我收藏的 Rust 文章",None),
        ("combined_en","Find unread starred Rust articles from 技术, tagged Rust, published in the last 7 days, sorted oldest first.",None),
        ("reading_history","找上周我读过的 Rust 文章",Some("unsupported")),
        ("unknown_source","找来自不存在的源Alpha的 Rust 文章",Some("unknownSource")),
        ("unknown_tag","找带有不存在的标签Beta的 Rust 文章",Some("unknownTag")),
        ("ambiguous_source","找来自技术的 Rust 文章",Some("ambiguousSource")),
    ];
    let mut results = Vec::new();
    let only = std::env::var("M4W_NL_LIVE_CASE")
        .ok()
        .filter(|value| !value.is_empty());
    let cases = cases
        .into_iter()
        .filter(|(name, _, _)| {
            only.as_ref().is_none_or(|one| {
                one == name
                    || one == "metadata_errors"
                        && matches!(*name, "unknown_source" | "unknown_tag" | "ambiguous_source")
            })
        })
        .collect::<Vec<_>>();
    assert!(!cases.is_empty(), "Unknown live case");
    let expected_count = cases.len();
    for (name, sentence, error_code) in cases {
        let mut metadata = metadata.clone();
        if name == "ambiguous_source" {
            metadata.sources.push(search_filter::Source {
                id: 5,
                name: "技术".into(),
                url: "https://duplicate.test/rss".into(),
            });
        }
        let start = std::time::Instant::now();
        let raw = usage::scope(
            &pool,
            0,
            "search_filter",
            search_filter::completion(
                &client,
                config.clone(),
                sentence,
                &metadata,
                chrono::Local::now().fixed_offset(),
            ),
        )
        .await;
        let diagnostic = raw.as_ref().ok().cloned();
        let outcome = raw.and_then(|raw| {
            let filter = search_filter::parse(&raw, &metadata)?;
            search_filter::validate_references(&filter, sentence, &metadata)?;
            Ok(filter)
        });
        let mut passed = false;
        let result = match outcome {
            Ok(filter) => {
                let matches = entry_store::query_entries(
                    &pool,
                    &entry_store::EntryQueryParams {
                        filter: filter.clone(),
                        cursor: None,
                        limit: 100,
                    },
                )
                .await
                .unwrap();
                let ids = matches.entries.iter().map(|e| e.id).collect::<Vec<_>>();
                let saved_id = smart_view_store::save(&pool, None, name.into(), filter.clone())
                    .await
                    .unwrap();
                let views = smart_view_store::list(&pool).await.unwrap();
                let saved = &views.iter().find(|v| v.id == saved_id).unwrap().filter;
                let identical =
                    serde_json::to_value(saved).unwrap() == serde_json::to_value(&filter).unwrap();
                let common = error_code.is_none()
                    && filter.search.as_deref() == Some("Rust")
                    && filter.recent_days == Some(7)
                    && filter.starred_only
                    && filter.date_from.is_none()
                    && filter.date_to.is_none()
                    && ids == [1]
                    && identical;
                passed = common
                    && if name == "basic_zh" {
                        filter.feed_id.is_none()
                            && filter.tag_ids.as_ref().is_none_or(|t| t.is_empty())
                            && filter.is_read.is_none()
                    } else {
                        filter.feed_id == Some(1)
                            && filter.tag_ids == Some(vec![3])
                            && filter.is_read == Some(false)
                            && filter.sort == crate::db::search_store::SearchSort::Oldest
                    };
                json!({"filter":filter,"matching_fixture_ids":ids,"saved_view_identical":identical})
            }
            Err(error) => {
                if let Some(code) = error_code {
                    passed = error.code == code
                        || (matches!(name, "unknown_source" | "unknown_tag")
                            && error.code == "unsupported"
                            && (error.detail.contains("不存在")
                                || error.detail.contains("not found")))
                        || (name == "ambiguous_source"
                            && error.code == "unsupported"
                            && error.detail.contains("技术")
                            && error.detail.contains("URL"));
                }
                json!({"error":error})
            }
        };
        println!(
            "live case {name}: {} ({} ms)",
            if passed { "PASS" } else { "FAIL" },
            start.elapsed().as_millis()
        );
        results.push(json!({"case":name,"sentence":sentence,"passed":passed,"elapsed_ms":start.elapsed().as_millis(),"result":result,"model_response":diagnostic}));
        // Stop on transport failure; do not repeatedly spend against a broken route.
        if results.last().unwrap()["result"]["error"]["code"] == "provider" {
            break;
        }
    }
    usage::flush().await;
    let metrics = pool.get().await.unwrap().interact(|c| {
        c.query_row("SELECT COUNT(*),SUM(COALESCE(prompt_tokens,0)),SUM(COALESCE(completion_tokens,0)),SUM(COALESCE(total_tokens,0)),SUM(CASE WHEN usage_availability='missing' THEN 1 ELSE 0 END) FROM llm_usage_event WHERE agent_type='search_filter'",[],|r|Ok(json!({"requests":r.get::<_,i64>(0)?,"prompt_tokens":r.get::<_,i64>(1)?,"completion_tokens":r.get::<_,i64>(2)?,"total_tokens":r.get::<_,i64>(3)?,"missing_usage":r.get::<_,i64>(4)?})))
    }).await.unwrap().unwrap();
    let passed = results.len() == expected_count && results.iter().all(|v| v["passed"] == true);
    let evidence = json!({"date":chrono::Local::now().to_rfc3339(),"model":model,"primary_only":true,"synthetic_metadata_and_library":true,"passed":passed,"metrics":metrics,"cases":results});
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(report)
        .unwrap();
    use std::io::Write;
    output
        .write_all(serde_json::to_string_pretty(&evidence).unwrap().as_bytes())
        .unwrap();
    assert!(passed, "Live cases failed; inspect the sanitized report");
}
