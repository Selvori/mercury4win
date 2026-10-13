//! Production pagination over isolated SQLite; no user data or network.
use super::*;
use crate::db::migrations;
use deadpool_sqlite::{Config, Runtime};
use std::collections::BTreeSet;

async fn fixture() -> Pool {
    let pool = Config::new(":memory:")
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    pool.get().await.unwrap().interact(|conn| {
        migrations::run_migrations(conn).unwrap();
        conn.execute_batch("INSERT INTO feed(id, feed_url) VALUES(1,'https://one.test'),(2,'https://two.test');
            INSERT INTO tag(id,name,normalized_name) VALUES(1,'test','test');").unwrap();
        for id in 1..=237 {
            // Insert older/newer rows out of date order, tied dates and NULLs.
            let date = if id % 5 == 0 { None } else {
                Some(format!("2026-10-{:02}T00:00:00Z", (id * 7) % 11 + 1))
            };
            conn.execute("INSERT INTO entry(id,feed_id,guid,title,summary,published_at,is_read,is_deleted)
                VALUES(?1,?2,?3,?4,'needle',?5,?6,?7)",
                params![id, if id % 3 == 0 {2} else {1}, format!("guid-{id}"), format!("article-{id}"), date, id % 2, i32::from(id % 17 == 0)]).unwrap();
            if id % 4 == 0 {
                conn.execute("INSERT INTO entry_tag(entry_id,tag_id) VALUES(?1,1)", [id]).unwrap();
            }
        }
    }).await.unwrap();
    pool
}

fn query(limit: u32) -> EntryQueryParams {
    EntryQueryParams {
        filter: EntryFilter::default(),
        cursor: None,
        limit,
    }
}

async fn expected(pool: &Pool, condition: &'static str) -> Vec<i64> {
    pool.get().await.unwrap().interact(move |conn| {
        conn.prepare(&format!("SELECT id FROM entry WHERE is_deleted=0 {condition} ORDER BY published_at DESC,id DESC"))
            .unwrap().query_map([], |r| r.get(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap()
    }).await.unwrap()
}

async fn traverse(pool: &Pool, mut q: EntryQueryParams, expected: Vec<i64>) {
    let mut ids = vec![];
    loop {
        let page = query_entries(pool, &q).await.unwrap();
        assert_eq!(
            page.total,
            expected.len() as i64,
            "count must exclude cursor"
        );
        assert!(!page.entries.is_empty());
        assert!(page.entries.len() <= q.limit as usize);
        let wire = serde_json::to_value(&page).unwrap();
        if let Some(ref c) = page.next_cursor {
            assert_eq!(wire["next_cursor"]["id"], c.id);
            let decoded: EntryCursor = serde_json::from_value(wire["next_cursor"].clone()).unwrap();
            assert_eq!(
                decoded.published_at,
                page.entries.last().unwrap().published_at
            );
            assert_eq!(decoded.id, page.entries.last().unwrap().id);
        }
        ids.extend(page.entries.iter().map(|e| e.id));
        assert!(ids.len() <= expected.len(), "pagination did not terminate");
        q.cursor = page.next_cursor;
        if q.cursor.is_none() {
            break;
        }
    }
    assert_eq!(ids, expected);
    assert_eq!(
        ids.iter().copied().collect::<BTreeSet<_>>().len(),
        ids.len()
    );
}

#[tokio::test]
async fn out_of_order_ties_nulls_and_terminal_pages() {
    let pool = fixture().await;
    let ids = expected(&pool, "").await;
    for limit in [1, 7, 100, ids.len() as u32] {
        traverse(&pool, query(limit), ids.clone()).await;
    }
}

#[tokio::test]
async fn scoped_count_and_cursor_share_filters() {
    let pool = fixture().await;
    let mut q = query(3);
    q.filter.feed_id = Some(1);
    q.filter.unread_only = true;
    q.filter.search = Some("needle".into());
    q.filter.tag_ids = Some(vec![1]);
    traverse(
        &pool,
        q,
        expected(
            &pool,
            "AND feed_id=1 AND is_read=0 AND id IN (SELECT entry_id FROM entry_tag WHERE tag_id=1)",
        )
        .await,
    )
    .await;
    let mut q = query(3);
    q.filter.search = Some("not present".into());
    let page = query_entries(&pool, &q).await.unwrap();
    assert_eq!(page.total, 0);
    assert!(page.entries.is_empty());
    assert!(page.next_cursor.is_none());
}

#[tokio::test]
async fn cursor_survives_boundary_deletion_and_late_older_insertion() {
    let pool = fixture().await;
    let all = expected(&pool, "").await;
    let mut q = query(1);
    let first = query_entries(&pool, &q).await.unwrap();
    q.cursor = first.next_cursor;
    let boundary = first.entries[0].id;
    pool.get().await.unwrap().interact(move |conn| {
        conn.execute("DELETE FROM entry WHERE id=?1", [boundary]).unwrap();
        conn.execute("INSERT INTO entry(id,feed_id,guid,published_at) VALUES(1000,1,'late','2020-01-01')", []).unwrap();
    }).await.unwrap();
    let mut remaining = all[1..].to_vec();
    // The newly inserted old dated row belongs immediately before NULL dates.
    let null_start = pool
        .get()
        .await
        .unwrap()
        .interact(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM entry WHERE is_deleted=0 AND published_at IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
        })
        .await
        .unwrap();
    remaining.insert(null_start as usize - 1, 1000);
    traverse(&pool, q, remaining).await;
}

#[tokio::test]
async fn zero_and_excessive_limits_are_bounded() {
    let pool = fixture().await;
    let page = query_entries(&pool, &query(0)).await.unwrap();
    assert_eq!(page.entries.len(), 1);
    assert!(page.next_cursor.is_some());
    let page = query_entries(&pool, &query(u32::MAX)).await.unwrap();
    assert_eq!(page.entries.len() as i64, page.total);
    assert!(page.next_cursor.is_none());
}
