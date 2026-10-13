//! Production SQL and wire contracts against an isolated in-memory database.
use super::*;
use crate::db::{feed_store, migrations, tag_store};
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
        conn.execute_batch("INSERT INTO feed(id,feed_url,title) VALUES(1,'https://one.test','One'),(2,'https://two.test','Two');
          INSERT INTO tag(id,name,normalized_name) VALUES(1,'alpha','alpha'),(2,'beta','beta');").unwrap();
        for id in 1..=237 {
            conn.execute("INSERT INTO entry(id,feed_id,guid,title,summary,published_at,is_read,is_starred,is_deleted,feed_content,feed_content_type)
                VALUES(?1,?2,?3,?4,'needle',?5,?6,?7,?8,'<p>Feed text</p>','html')",
                params![id, if id % 3 == 0 {2} else {1}, format!("guid-{id}"), format!("Article {id}"), if id % 5 == 0 {None} else {Some("2026-10-12")}, id % 2, i32::from(id % 7 != 0), i32::from(id % 17 == 0)]).unwrap();
            if id % 4 == 0 { conn.execute("INSERT INTO entry_tag(entry_id,tag_id,source) VALUES(?1,1,'manual')", [id]).unwrap(); }
            if id % 6 == 0 { conn.execute("INSERT INTO entry_tag(entry_id,tag_id,source) VALUES(?1,2,'ai')", [id]).unwrap(); }
        }
        conn.execute("INSERT INTO entry_note(entry_id,markdown,updated_at) VALUES(12,'Test note','2026-10-12')", []).unwrap();
    }).await.unwrap();
    pool
}

async fn ids(pool: &Pool, filter: EntryFilter) -> Vec<i64> {
    let mut params = EntryQueryParams {
        filter,
        cursor: None,
        limit: 7,
    };
    let mut ids = vec![];
    let mut total = None;
    loop {
        let page = query_entries(pool, &params).await.unwrap();
        assert_eq!(*total.get_or_insert(page.total), page.total);
        ids.extend(page.entries.iter().map(|e| e.id));
        params.cursor = page.next_cursor;
        if params.cursor.is_none() {
            assert_eq!(ids.len() as i64, page.total);
            return ids;
        }
    }
}

#[tokio::test]
async fn all_any_and_all_filters_match_independent_sql_across_pages() {
    let pool = fixture().await;
    for mode in [TagMatchMode::Any, TagMatchMode::All] {
        let filter = EntryFilter {
            feed_id: Some(2),
            unread_only: true,
            starred_only: true,
            search: Some("  needle  ".into()),
            tag_ids: Some(vec![2, 1, 2]),
            tag_mode: mode,
        };
        let actual = ids(&pool, filter).await;
        let expected = pool.get().await.unwrap().interact(move |conn| {
            let tags = if matches!(mode, TagMatchMode::All) { "id % 12 = 0" } else { "(id % 4 = 0 OR id % 6 = 0)" };
            conn.prepare(&format!("SELECT id FROM entry WHERE is_deleted=0 AND feed_id=2 AND is_read=0 AND is_starred=1 AND summary LIKE '%needle%' AND {tags} ORDER BY published_at DESC,id DESC"))
                .unwrap().query_map([], |r| r.get(0)).unwrap().collect::<Result<Vec<i64>,_>>().unwrap()
        }).await.unwrap();
        assert_eq!(actual, expected);
    }
    assert_eq!(
        ids(
            &pool,
            EntryFilter {
                search: Some("   ".into()),
                tag_ids: Some(vec![]),
                ..Default::default()
            }
        )
        .await,
        ids(&pool, EntryFilter::default()).await
    );
    assert!(ids(
        &pool,
        EntryFilter {
            search: Some("does not exist".into()),
            ..Default::default()
        }
    )
    .await
    .is_empty());
    assert!(serde_json::from_str::<TagMatchMode>("\"invalid\"").is_err());
}

#[tokio::test]
async fn scoped_bulk_read_matches_listing_beyond_first_page_and_is_reversible() {
    let pool = fixture().await;
    let filter = EntryFilter {
        search: Some("needle".into()),
        starred_only: true,
        tag_ids: Some(vec![1, 2]),
        tag_mode: TagMatchMode::Any,
        ..Default::default()
    };
    let before = ids(&pool, filter.clone()).await;
    assert!(before.len() > 7);
    mark_all_read_in_scope(&pool, &filter).await.unwrap();
    let changed = pool
        .get()
        .await
        .unwrap()
        .interact(|conn| {
            conn.prepare("SELECT id FROM entry WHERE is_read=1 AND id % 2=0 ORDER BY id")
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<Vec<i64>, _>>()
                .unwrap()
        })
        .await
        .unwrap();
    let mut expected: Vec<_> = before.into_iter().filter(|id| id % 2 == 0).collect();
    expected.sort();
    assert_eq!(changed, expected);
    let empty = EntryFilter {
        search: Some("no match".into()),
        ..Default::default()
    };
    mark_all_read_in_scope(&pool, &empty).await.unwrap();
    mark_read(&pool, &[12], false).await.unwrap();
    assert!(
        !get_entry_detail(&pool, 12)
            .await
            .unwrap()
            .unwrap()
            .entry
            .is_read
    );
    mark_read(&pool, &[12], true).await.unwrap();
    assert!(
        get_entry_detail(&pool, 12)
            .await
            .unwrap()
            .unwrap()
            .entry
            .is_read
    );
}

#[tokio::test]
async fn delete_updates_visible_tag_feed_counts_and_hides_detail() {
    let pool = fixture().await;
    delete_entry(&pool, 12).await.unwrap();
    assert!(get_entry_detail(&pool, 12).await.unwrap().is_none());
    let tags = tag_store::list_tags(&pool, None).await.unwrap();
    for tag in tags {
        let expected = pool.get().await.unwrap().interact(move |conn| conn.query_row(
            "SELECT COUNT(*) FROM entry_tag et JOIN entry e ON e.id=et.entry_id WHERE e.is_deleted=0 AND et.tag_id=?1", [tag.id], |r| r.get::<_,i64>(0)).unwrap()).await.unwrap();
        assert_eq!(tag.usage_count, expected);
    }
    let feeds = feed_store::list_feeds(&pool).await.unwrap();
    assert_eq!(feeds.iter().map(|f| f.total_count).sum::<i64>(), 223);
}

#[tokio::test]
async fn detail_serialization_preserves_nested_entry_optional_fields_and_missing_error() {
    let pool = fixture().await;
    let detail = get_entry_detail(&pool, 12).await.unwrap().unwrap();
    let wire = serde_json::to_value(detail).unwrap();
    assert_eq!(wire["entry"]["id"], 12);
    assert_eq!(wire["entry"]["feed_content"], "<p>Feed text</p>");
    assert_eq!(wire["entry"]["feed_content_type"], "html");
    assert!(wire["entry"]["author"].is_null());
    assert!(wire.get("id").is_none());
    assert_eq!(wire["feed_title"], "Two");
    assert_eq!(wire["has_note"], true);
    assert_eq!(wire["tags"].as_array().unwrap().len(), 2);
    let missing = get_entry_detail(&pool, 9999)
        .await
        .unwrap()
        .ok_or_else(|| "Entry not found".to_string());
    assert_eq!(missing.unwrap_err(), "Entry not found");
    if let Ok(path) = std::env::var("M4W_DETAIL_WIRE_PATH") {
        std::fs::write(path, serde_json::to_string_pretty(&wire).unwrap()).unwrap();
    }
}
