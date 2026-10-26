use super::*;
use crate::db::{entry_store, migrations, note_store, smart_view_store};
use deadpool_sqlite::{Config, Runtime};

async fn fixture() -> Pool {
    let pool = Config::new(":memory:")
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    pool.get().await.unwrap().interact(|conn| {
        migrations::run_migrations(conn).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;
            INSERT INTO feed(id,title,feed_url) VALUES(1,'Source A','https://a.test'),(2,'Source B','https://b.test');
            INSERT INTO tag(id,name,normalized_name) VALUES(1,'Rust','rust'),(2,'SQLite','sqlite');
            INSERT INTO entry(id,feed_id,guid,title,summary,published_at,is_read,is_starred) VALUES
                (1,1,'a','索引与 Rust::read_to_string','<p>中文摘要</p>','2026-10-25T12:00:00Z',0,1),
                (2,1,'b','Ordinary title','Brief','2026-10-24T12:00:00Z',1,1),
                (3,2,'c','回填资料','Short','2026-10-23T12:00:00Z',0,0),
                (4,2,'d','Rust solution','Short',NULL,0,0);
            INSERT INTO content(entry_id,markdown) VALUES(2,'只有正文包含双字方案与 TCP / read_to_string 代码。');
            INSERT INTO entry_note VALUES(3,'笔记独有回忆 async_fn <script>evil()</script>','now');
            INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,1),(1,2),(2,1),(3,2);").unwrap();
    }).await.unwrap();
    pool
}
async fn search(pool: &Pool, text: &str) -> EntryPage {
    entry_store::query_entries(
        pool,
        &EntryQueryParams {
            filter: EntryFilter {
                search: Some(text.into()),
                ..Default::default()
            },
            cursor: None,
            limit: 100,
        },
    )
    .await
    .unwrap()
}
fn ids(page: &EntryPage) -> Vec<i64> {
    page.entries.iter().map(|e| e.id).collect()
}

#[tokio::test]
async fn chinese_short_english_code_and_literal_operators() {
    let pool = fixture().await;
    for (q, expected) in [
        ("方案", vec![2]),
        ("中文", vec![1]),
        ("TCP", vec![2]),
        ("READ_TO_STRING", vec![1, 2]),
        ("Rust::read_to_string", vec![1]),
        ("回忆 async_fn", vec![3]),
        ("%", vec![]),
        ("_", vec![1, 2, 3]),
        ("\" OR *", vec![]),
    ] {
        assert_eq!(ids(&search(&pool, q).await), expected, "{q}");
    }
    // Demonstrate the bundled tokenizer's real two-character limitation.
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM entry_search WHERE entry_search MATCH '\"方案\"'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 0);
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn body_note_fields_snippets_and_no_ai_dependencies() {
    let pool = fixture().await;
    let page = search(&pool, "方案").await;
    assert_eq!(page.search_info[0].feed_title, "Source A");
    assert!(page.search_info[0].body_available);
    assert!(page.search_info[0]
        .hits
        .iter()
        .any(|h| h.field == "body" && h.parts.iter().any(|p| p.matched && p.text == "方案")));
    let page = search(&pool, "回忆").await;
    assert_eq!(page.search_info[0].hits[0].field, "note");
    let mut filter = EntryFilter {
        search: Some("Rust".into()),
        search_field: SearchField::Body,
        exclude_title: true,
        ..Default::default()
    };
    assert!(entry_store::query_entries(
        &pool,
        &EntryQueryParams {
            filter: filter.clone(),
            cursor: None,
            limit: 100
        }
    )
    .await
    .unwrap()
    .entries
    .is_empty());
    filter.search = Some("read_to_string".into());
    assert_eq!(
        ids(&entry_store::query_entries(
            &pool,
            &EntryQueryParams {
                filter,
                cursor: None,
                limit: 100
            }
        )
        .await
        .unwrap()),
        vec![2]
    );
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            for table in [
                "agent_provider_profile",
                "agent_task_run",
                "llm_usage_event",
            ] {
                assert_eq!(
                    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))
                        .unwrap(),
                    0
                );
            }
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn index_tracks_note_body_title_hide_delete_and_rebuild() {
    let pool = fixture().await;
    assert_eq!(ids(&search(&pool, "回忆").await), vec![3]);
    note_store::save_note(&pool, 3, "新的笔记标志")
        .await
        .unwrap();
    assert!(search(&pool, "回忆").await.entries.is_empty());
    assert_eq!(ids(&search(&pool, "标志").await), vec![3]);
    note_store::delete_note(&pool, 3).await.unwrap();
    assert!(search(&pool, "标志").await.entries.is_empty());
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            conn.execute(
                "UPDATE content SET markdown='replacement_body' WHERE entry_id=2",
                [],
            )
            .unwrap();
            conn.execute("UPDATE entry SET title='updated_title' WHERE id=1", [])
                .unwrap();
        })
        .await
        .unwrap();
    assert!(search(&pool, "方案").await.entries.is_empty());
    assert_eq!(ids(&search(&pool, "replacement_body").await), vec![2]);
    entry_store::delete_entry(&pool, 2).await.unwrap();
    assert!(search(&pool, "replacement_body").await.entries.is_empty());
    rebuild(&pool).await.unwrap();
    assert_eq!(ids(&search(&pool, "updated_title").await), vec![1]);
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            conn.execute_batch("DROP TABLE entry_search").unwrap();
        })
        .await
        .unwrap();
    rebuild(&pool).await.unwrap();
    assert_eq!(ids(&search(&pool, "updated_title").await), vec![1]);
    pool.get()
        .await
        .unwrap()
        .interact(|conn| conn.execute("DELETE FROM feed WHERE id=1", []).unwrap())
        .await
        .unwrap();
    assert!(search(&pool, "updated_title").await.entries.is_empty());
}

#[tokio::test]
async fn combined_filters_any_all_empty_keywords_and_scoped_bulk() {
    let pool = fixture().await;
    let mut filter = EntryFilter {
        tag_ids: Some(vec![1, 2]),
        starred_only: true,
        ..Default::default()
    };
    let get = |filter| EntryQueryParams {
        filter,
        cursor: None,
        limit: 100,
    };
    assert_eq!(
        ids(&entry_store::query_entries(&pool, &get(filter.clone()))
            .await
            .unwrap()),
        vec![1, 2]
    );
    filter.tag_mode = TagMatchMode::All;
    assert_eq!(
        ids(&entry_store::query_entries(&pool, &get(filter.clone()))
            .await
            .unwrap()),
        vec![1]
    );
    filter.is_read = Some(true);
    assert!(entry_store::query_entries(&pool, &get(filter.clone()))
        .await
        .unwrap()
        .entries
        .is_empty());
    filter.is_read = None;
    filter.has_note = Some(false);
    filter.feed_id = Some(1);
    filter.date_from = Some("2026-10-25".into());
    filter.date_to = Some("2026-10-25".into());
    entry_store::mark_all_read_in_scope(&pool, &filter)
        .await
        .unwrap();
    assert!(
        entry_store::get_entry_detail(&pool, 1)
            .await
            .unwrap()
            .unwrap()
            .entry
            .is_read
    );
    assert!(
        !entry_store::get_entry_detail(&pool, 3)
            .await
            .unwrap()
            .unwrap()
            .entry
            .is_read
    );
    filter = EntryFilter {
        has_note: Some(true),
        ..Default::default()
    };
    assert_eq!(
        ids(&entry_store::query_entries(&pool, &get(filter))
            .await
            .unwrap()),
        vec![3]
    );
}

#[tokio::test]
async fn ranking_oldest_newest_keysets_no_duplicates_or_missing_rows() {
    let pool = fixture().await;
    pool.get().await.unwrap().interact(|conn| {
        for id in 10..147 {
            conn.execute("INSERT INTO entry(id,feed_id,guid,title,summary,published_at) VALUES(?1,1,?2,?3,'needle',?4)",
                params![id,id.to_string(),if id%3==0 {"needle"} else {"other"},if id%7==0 {None}else{Some("2026-10-26")}]).unwrap();
        }
    }).await.unwrap();
    for sort in [
        SearchSort::Newest,
        SearchSort::Oldest,
        SearchSort::Relevance,
    ] {
        let filter = EntryFilter {
            search: Some("needle".into()),
            sort,
            ..Default::default()
        };
        let all = entry_store::query_entries(
            &pool,
            &EntryQueryParams {
                filter: filter.clone(),
                cursor: None,
                limit: 500,
            },
        )
        .await
        .unwrap();
        let mut q = EntryQueryParams {
            filter,
            cursor: None,
            limit: 7,
        };
        let mut actual = vec![];
        loop {
            let page = entry_store::query_entries(&pool, &q).await.unwrap();
            assert_eq!(page.total, 137);
            actual.extend(ids(&page));
            q.cursor = page.next_cursor;
            if q.cursor.is_none() {
                break;
            }
            assert!(actual.len() < 150);
        }
        assert_eq!(actual, ids(&all));
        assert_eq!(
            actual
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            137
        );
        if sort == SearchSort::Relevance {
            assert_eq!(actual[0] % 3, 0);
        }
    }
}

#[test]
fn dynamic_relative_boundary_and_validation() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run_migrations(&conn).unwrap();
    conn.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://test');
        INSERT INTO entry(feed_id,guid,published_at) VALUES(1,'a','2026-10-19T12:00:00Z'),(1,'b','2026-10-26T12:00:01Z');").unwrap();
    let filter = EntryFilter {
        recent_days: Some(7),
        ..Default::default()
    };
    for (anchor, count) in [
        ("2026-10-26T12:00:00Z", 1),
        ("2026-10-26T12:00:02Z", 1),
        ("2026-11-03T00:00:00Z", 0),
    ] {
        let c = compile(&filter, anchor).unwrap();
        let actual: i64 = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM entry e WHERE {}",
                    c.conditions.join(" AND ")
                ),
                rusqlite::params_from_iter(c.values),
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(actual, count);
    }
    assert!(validate(&EntryFilter {
        date_from: Some("2026-02-30".into()),
        ..Default::default()
    })
    .is_err());
    assert!(validate(&EntryFilter {
        recent_days: Some(0),
        ..Default::default()
    })
    .is_err());
}

#[test]
fn unicode_snippets_are_safe_text_with_context_and_overlaps() {
    let parts = snippet(
        "前文 İSTANBUL 中文方案 <img onerror=evil()> 后文",
        &["i̇stanbul".into(), "方案".into(), "img".into()],
    );
    assert!(parts.iter().any(|p| p.matched && p.text == "İSTANBUL"));
    assert!(parts.iter().any(|p| p.matched && p.text == "方案"));
    assert_eq!(
        parts.iter().map(|p| p.text.as_str()).collect::<String>(),
        "前文 İSTANBUL 中文方案 <img onerror=evil()> 后文"
    );
    assert_eq!(
        html_text("<script>secret</script><p>a &amp; b</p><a href='hidden'>visible</a>"),
        "a & b visible"
    );
}

#[tokio::test]
async fn saved_views_crud_persist_across_new_pool_and_preserve_relative_rule() {
    let path = std::env::temp_dir().join(format!("m4w-search-{}.db", uuid::Uuid::new_v4()));
    let make = || {
        Config::new(&path)
            .builder(Runtime::Tokio1)
            .unwrap()
            .max_size(1)
            .build()
            .unwrap()
    };
    let pool = make();
    pool.get()
        .await
        .unwrap()
        .interact(|c| migrations::run_migrations(c).unwrap())
        .await
        .unwrap();
    let filter = EntryFilter {
        search: Some("Rust".into()),
        recent_days: Some(7),
        has_note: Some(true),
        sort: SearchSort::Relevance,
        ..Default::default()
    };
    let id = smart_view_store::save(&pool, None, "  Rust 笔记  ".into(), filter.clone())
        .await
        .unwrap();
    assert!(
        smart_view_store::save(&pool, None, "Rust 笔记".into(), filter.clone())
            .await
            .is_err()
    );
    pool.close();
    drop(pool);
    let pool = make();
    let views = smart_view_store::list(&pool).await.unwrap();
    assert_eq!(views[0].filter.recent_days, Some(7));
    assert!(views[0].filter.date_from.is_none());
    assert_eq!(views[0].name, "Rust 笔记");
    smart_view_store::save(&pool, Some(id), "Renamed".into(), EntryFilter::default())
        .await
        .unwrap();
    assert_eq!(
        smart_view_store::list(&pool).await.unwrap()[0].name,
        "Renamed"
    );
    smart_view_store::delete(&pool, id).await.unwrap();
    assert!(smart_view_store::list(&pool).await.unwrap().is_empty());
    assert!(
        smart_view_store::save(&pool, Some(id), "Gone".into(), filter)
            .await
            .is_err()
    );
    pool.close();
    drop(pool);
    let _ = std::fs::remove_file(path);
}

#[test]
fn historical_upgrade_backfills_preserves_data_and_is_repeatable() {
    let conn = migrations::historical_fixture(3);
    conn.execute_batch("INSERT INTO feed(id,feed_url) VALUES(1,'https://test');
        INSERT INTO entry(id,feed_id,guid,title,is_read,is_starred) VALUES(1,1,'old','历史标题',1,1);
        INSERT INTO content(entry_id,markdown) VALUES(1,'历史正文双字');
        INSERT INTO entry_note VALUES(1,'历史笔记','now');").unwrap();
    migrations::run_migrations(&conn).unwrap();
    migrations::run_migrations(&conn).unwrap();
    let page = query(
        &conn,
        &EntryFilter {
            search: Some("双字".into()),
            ..Default::default()
        },
        None,
        100,
    )
    .unwrap();
    assert_eq!(ids(&page), vec![1]);
    assert!(page.entries[0].is_read && page.entries[0].is_starred);
    assert_eq!(
        conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}

/// Explicit opt-in artifacts for isolated UI tests; never reads an installed DB.
#[test]
fn export_search_ui_fixtures() {
    let Some(path) = std::env::var_os("M4W_SEARCH_NATIVE_DB") else {
        return;
    };
    assert!(
        !std::path::Path::new(&path).exists(),
        "Refuse to replace a database"
    );
    let old = migrations::historical_fixture(5);
    let conn = Connection::open(path).unwrap();
    let schema:Vec<String>=old.prepare("SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY rowid").unwrap()
        .query_map([],|r|r.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
    for sql in schema {
        conn.execute_batch(&sql).unwrap();
    }
    conn.execute_batch("PRAGMA foreign_keys=ON; INSERT INTO settings VALUES('schema_version','5');
        INSERT INTO feed(id,title,feed_url) VALUES(1,'搜索样例 A','https://search-a.test/feed'),(2,'搜索样例 B','https://search-b.test/feed');
        INSERT INTO tag(id,name,normalized_name) VALUES(1,'Rust','rust'),(2,'SQLite','sqlite');
        INSERT INTO entry(id,feed_id,guid,title,summary,feed_content,feed_content_type,published_at,is_starred) VALUES
            (1,1,'a','Rust 索引方案','<p>标题与摘要检索样例</p>','<p>本地全文搜索方案，rust::read_to_string 保留代码。</p>','html','2026-10-25T12:00:00Z',1),
            (2,1,'b','正文独有测试','<p>只有导读</p>','<p>这一方案只在正文中。TCP 与 read_to_string。</p>','html','2026-10-24T12:00:00Z',1),
            (3,2,'c','笔记独有测试','<p>另一来源</p>','<p>正常正文，没有笔记关键词。</p>','html','2026-10-23T12:00:00Z',1),
            (4,2,'d','未获取正文','<p>仅导读可搜索</p>',NULL,NULL,'2026-10-22T12:00:00Z',0),
            (5,2,'e','<img src=x onerror=evil()>方案','safe','<p>安全正文。</p>','html','2026-10-21T12:00:00Z',0);
        INSERT INTO entry_note VALUES(3,'笔记独有回忆 async_fn 与双字方案。','now');
        INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,1),(1,2),(2,1),(3,2);").unwrap();
    for id in 10..145 {
        conn.execute("INSERT INTO entry(id,feed_id,guid,title,summary,feed_content,feed_content_type,published_at) VALUES(?1,1,?2,?3,'合成分页','<p>分页样例的本地正文。</p>','html','2026-10-20T00:00:00Z')",params![id,format!("page-{id}"),format!("分页样例 {id}")]).unwrap();
    }
    // Native startup upgrades this genuine V5 DB. Browser fixtures use the same
    // source rows and production query/highlight serialization after migration.
    if let Some(path) = std::env::var_os("M4W_SEARCH_FIXTURE") {
        let backup_path =
            std::path::PathBuf::from(std::env::var_os("M4W_SEARCH_NATIVE_DB").unwrap());
        let export_path = backup_path.with_extension("export.db");
        assert!(!export_path.exists());
        drop(conn);
        std::fs::copy(&backup_path, &export_path).unwrap();
        let export = Connection::open(&export_path).unwrap();
        migrations::run_migrations(&export).unwrap();
        let mut pages = serde_json::Map::new();
        for text in ["", "方案", "回忆", "分页", "TCP", "没有匹配"] {
            pages.insert(
                text.into(),
                serde_json::to_value(
                    query(
                        &export,
                        &EntryFilter {
                            search: Some(text.into()),
                            sort: SearchSort::Relevance,
                            ..Default::default()
                        },
                        None,
                        100,
                    )
                    .unwrap(),
                )
                .unwrap(),
            );
        }
        let filter = EntryFilter {
            search: Some("分页".into()),
            sort: SearchSort::Relevance,
            ..Default::default()
        };
        let first = query(&export, &filter, None, 100).unwrap();
        pages.insert(
            "分页::next".into(),
            serde_json::to_value(query(&export, &filter, first.next_cursor.as_ref(), 100).unwrap())
                .unwrap(),
        );
        std::fs::write(path, serde_json::to_vec_pretty(&pages).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "explicit synthetic performance measurement"]
fn synthetic_performance() {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run_migrations(&conn).unwrap();
    conn.execute(
        "INSERT INTO feed(id,feed_url) VALUES(1,'https://synthetic.test')",
        [],
    )
    .unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    for id in 1..=10000 {
        tx.execute("INSERT INTO entry(id,feed_id,guid,title,summary,published_at) VALUES(?1,1,?2,?3,?4,'2026-10-26')",params![id,id.to_string(),format!("Article {id}"),"Synthetic summary"]).unwrap();
        tx.execute(
            "INSERT INTO content(entry_id,markdown) VALUES(?1,?2)",
            params![
                id,
                format!(
                    "{} {}",
                    "Ordinary text about readers and local databases. ".repeat(25),
                    if id % 17 == 0 {
                        "中文方案 rust::read_to_string"
                    } else {
                        "background"
                    }
                )
            ],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    let start = std::time::Instant::now();
    refresh(&conn).unwrap();
    eprintln!(
        "10,000 documents ~1.2KB each, initial index {:?}",
        start.elapsed()
    );
    for text in ["方案", "rust::read_to_string", "reader", "missing"] {
        let start = std::time::Instant::now();
        let page = query(
            &conn,
            &EntryFilter {
                search: Some(text.into()),
                sort: SearchSort::Relevance,
                ..Default::default()
            },
            None,
            100,
        )
        .unwrap();
        eprintln!(
            "query {text}: total={}, first 100+count+snippets {:?}",
            page.total,
            start.elapsed()
        );
    }
}

#[tokio::test]
async fn repeated_production_upserts_do_not_override_dirty_conflict_policy() {
    let pool = fixture().await;
    // Existing entry insert has already queued ID=1. Both insert and conflict
    // update paths must coexist with it, even under an outer OR REPLACE/UPSERT.
    note_store::save_note(&pool, 1, "first").await.unwrap();
    note_store::save_note(&pool, 1, "second").await.unwrap();
    pool.get().await.unwrap().interact(|conn| {
        conn.execute("INSERT INTO content(entry_id,markdown) VALUES(1,'new') ON CONFLICT(entry_id) DO UPDATE SET markdown=excluded.markdown",[]).unwrap();
        conn.execute("INSERT INTO content(entry_id,markdown) VALUES(1,'updated') ON CONFLICT(entry_id) DO UPDATE SET markdown=excluded.markdown",[]).unwrap();
        conn.execute("INSERT OR REPLACE INTO entry_note VALUES(1,'replacement','now')",[]).unwrap();
    }).await.unwrap();
    assert_eq!(ids(&search(&pool, "updated").await), vec![1]);
    assert_eq!(ids(&search(&pool, "replacement").await), vec![1]);
}

#[tokio::test]
async fn time_order_normalizes_offsets_ties_and_nulls_on_every_page() {
    let pool = fixture().await;
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            conn.execute_batch(
                "DELETE FROM entry; INSERT INTO entry(id,feed_id,guid,title,published_at) VALUES
          (1,1,'a','order','2026-10-26T08:00:00+08:00'),
          (2,1,'b','order','2026-10-26T00:00:01Z'),
          (3,1,'c','order','2026-10-26T00:00:00Z'),
          (4,1,'d','order',NULL);",
            )
            .unwrap();
        })
        .await
        .unwrap();
    for (sort, expected) in [
        (SearchSort::Newest, vec![2, 3, 1, 4]),
        (SearchSort::Oldest, vec![1, 3, 2, 4]),
    ] {
        let mut q = EntryQueryParams {
            filter: EntryFilter {
                sort,
                ..Default::default()
            },
            cursor: None,
            limit: 1,
        };
        let mut actual = vec![];
        loop {
            let page = entry_store::query_entries(&pool, &q).await.unwrap();
            actual.extend(ids(&page));
            q.cursor = page.next_cursor;
            if q.cursor.is_none() {
                break;
            }
        }
        assert_eq!(actual, expected);
    }
}
