use super::{identity, import_service, opml_import::OpmlImportEntry, parser};
use crate::db::{feed_store, migrations};
use deadpool_sqlite::{Config, Pool, Runtime};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn pool() -> Pool {
    let pool = Config::new(":memory:")
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    pool.get()
        .await
        .unwrap()
        .interact(|c| {
            c.execute_batch("PRAGMA foreign_keys=ON").unwrap();
            migrations::run_migrations(c).unwrap();
        })
        .await
        .unwrap();
    pool
}

fn rss(home: &str, start: usize, count: usize) -> String {
    let home = home.trim_end_matches('/');
    let items = (start..start+count).map(|i| format!("<item><guid>guid-{i}</guid><link>{home}/article/{i}</link><title>Article {i}</title></item>")).collect::<String>();
    format!("<rss version=\"2.0\"><channel><title>Shared name</title><link>{home}</link><description>Fixture</description>{items}</channel></rss>")
}

async fn server(
    replies: Vec<(u16, String)>,
) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    let task = tokio::spawn(async move {
        for (status, body) in replies {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).await.unwrap();
            calls.fetch_add(1, Ordering::SeqCst);
            socket.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nContent-Type: application/xml\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    (base, count, task)
}

fn entry(url: &str) -> OpmlImportEntry {
    OpmlImportEntry {
        title: "Imported title".into(),
        feed_url: url.into(),
        site_url: None,
    }
}

#[tokio::test]
async fn first_failure_retries_once_and_then_saves_articles() {
    let pool = pool().await;
    let (url, calls, task) = server(vec![
        (503, "temporary".into()),
        (200, rss("https://site.test", 0, 3)),
    ])
    .await;
    let mut statuses = vec![];
    let result =
        import_service::import_entries(&pool, &reqwest::Client::new(), vec![entry(&url)], |p| {
            statuses.push(p.status)
        })
        .await
        .unwrap();
    task.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(result.feeds_added, 1);
    assert!(result.errors.is_empty());
    assert_eq!(
        statuses,
        ["start", "processing", "retrying", "added", "done"]
    );
    assert_eq!(
        feed_store::list_feeds(&pool).await.unwrap()[0].total_count,
        3
    );
}

#[tokio::test]
async fn two_failures_leave_no_sidebar_feed_and_next_feed_continues() {
    let pool = pool().await;
    let (url, calls, task) = server(vec![
        (503, "first".into()),
        (503, "second".into()),
        (200, rss("https://site.test", 0, 3)),
    ])
    .await;
    let result = import_service::import_entries(
        &pool,
        &reqwest::Client::new(),
        vec![entry(&format!("{url}/bad")), entry(&format!("{url}/good"))],
        |_| {},
    )
    .await
    .unwrap();
    task.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(result.feeds_added, 1);
    assert_eq!(result.feeds_skipped, 0);
    assert_eq!(result.errors.len(), 1);
    let feeds = feed_store::list_feeds(&pool).await.unwrap();
    assert_eq!(feeds.len(), 1);
    assert!(feeds[0].feed_url.ends_with("/good"));
}

#[tokio::test]
async fn empty_invalid_and_zero_article_feeds_fail_without_placeholders() {
    for body in [
        String::new(),
        "<html>Not RSS</html>".into(),
        rss("https://site.test", 0, 0),
    ] {
        let pool = pool().await;
        let (url, calls, task) = server(vec![(200, body.clone()), (200, body)]).await;
        let result = import_service::import_entries(
            &pool,
            &reqwest::Client::new(),
            vec![entry(&url)],
            |_| {},
        )
        .await
        .unwrap();
        task.await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(result.feeds_added, 0);
        assert_eq!(result.errors.len(), 1);
        assert!(feed_store::list_feeds(&pool).await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn aliases_and_normalized_urls_skip_without_changing_saved_user_data() {
    let pool = pool().await;
    let (url, calls, task) = server(vec![
        (200, rss("https://site.test/blog", 0, 3)),
        (200, rss("http://www.site.test/blog/", 0, 6)),
    ])
    .await;
    let first = format!("{url}/official");
    import_service::import_entries(&pool, &reqwest::Client::new(), vec![entry(&first)], |_| {})
        .await
        .unwrap();
    pool.get().await.unwrap().interact(|c| c.execute_batch("UPDATE entry SET is_read=1,is_starred=1; INSERT INTO entry_note(entry_id,markdown,updated_at) SELECT id,'Saved note','2026-10-29' FROM entry;")).await.unwrap().unwrap();
    let mut statuses = vec![];
    let result = import_service::import_entries(
        &pool,
        &reqwest::Client::new(),
        vec![
            entry(&format!("{url}/mirror")),
            entry(&format!("{first}/#ignored")),
            entry(&first),
        ],
        |p| statuses.push((p.current, p.status)),
    )
    .await
    .unwrap();
    task.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!((result.feeds_added, result.feeds_skipped), (0, 3));
    assert_eq!(feed_store::list_feeds(&pool).await.unwrap().len(), 1);
    pool.get().await.unwrap().interact(|c| {
        assert_eq!(c.query_row("SELECT count(*) FROM entry e JOIN entry_note n ON e.id=n.entry_id WHERE e.is_read=1 AND e.is_starred=1 AND n.markdown='Saved note'", [], |r| r.get::<_,i64>(0)).unwrap(),3);
    }).await.unwrap();
    assert!(statuses.contains(&(3, "skipped".into())));
}

#[tokio::test]
async fn same_title_different_sites_or_different_columns_are_not_duplicates() {
    let pool = pool().await;
    let (url, _, task) = server(vec![
        (200, rss("https://site.test", 0, 3)),
        (200, rss("https://other.test", 0, 3)),
        (200, rss("https://site.test", 2, 3)),
    ])
    .await;
    let entries = (0..3).map(|i| entry(&format!("{url}/{i}"))).collect();
    let result = import_service::import_entries(&pool, &reqwest::Client::new(), entries, |_| {})
        .await
        .unwrap();
    task.await.unwrap();
    assert_eq!((result.feeds_added, result.feeds_skipped), (3, 0));
}

#[tokio::test]
async fn database_failure_rolls_back_feed_and_articles() {
    let pool = pool().await;
    pool.get().await.unwrap().interact(|c| c.execute_batch("CREATE TRIGGER fail_entry BEFORE INSERT ON entry BEGIN SELECT RAISE(ABORT,'fixture failure'); END;")).await.unwrap().unwrap();
    let (url, calls, task) = server(vec![(200, rss("https://site.test", 0, 3))]).await;
    let result =
        import_service::import_entries(&pool, &reqwest::Client::new(), vec![entry(&url)], |_| {})
            .await
            .unwrap();
    task.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1); // Database errors are not network retries.
    assert_eq!(result.errors.len(), 1);
    assert!(feed_store::list_feeds(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn old_failed_placeholder_is_recovered_or_removed_without_touching_valid_feeds() {
    let pool = pool().await;
    let (url, _, task) = server(vec![
        (503, "bad".into()),
        (503, "bad".into()),
        (200, rss("https://site.test", 0, 3)),
    ])
    .await;
    let bad = format!("{url}/bad");
    let good = format!("{url}/good");
    feed_store::insert_feed(&pool, &bad, Some("Old failed"), None)
        .await
        .unwrap();
    let (old, _) = feed_store::insert_feed(&pool, &good, Some("Old recoverable"), None)
        .await
        .unwrap();
    let result = import_service::import_entries(
        &pool,
        &reqwest::Client::new(),
        vec![entry(&bad), entry(&good)],
        |_| {},
    )
    .await
    .unwrap();
    task.await.unwrap();
    assert_eq!((result.feeds_added, result.errors.len()), (1, 1));
    let feeds = feed_store::list_feeds(&pool).await.unwrap();
    assert_eq!(feeds.len(), 1);
    assert_eq!(feeds[0].id, old.id);
    assert_eq!(feeds[0].total_count, 3);
    feed_store::remove_failed_import(&pool, old.id)
        .await
        .unwrap();
    assert_eq!(feed_store::list_feeds(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn concurrent_imports_recheck_duplicate_identity_in_transaction() {
    let pool = pool().await;
    let client = reqwest::Client::new();
    let (url, _, task) = server(vec![
        (200, rss("https://site.test", 0, 3)),
        (200, rss("https://site.test", 0, 3)),
    ])
    .await;
    let (a, b) = tokio::join!(
        import_service::import_entries(&pool, &client, vec![entry(&format!("{url}/a"))], |_| {}),
        import_service::import_entries(&pool, &client, vec![entry(&format!("{url}/b"))], |_| {}),
    );
    task.await.unwrap();
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.feeds_added + b.feeds_added, 1);
    assert_eq!(a.feeds_skipped + b.feeds_skipped, 1);
    assert_eq!(feed_store::list_feeds(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn timeout_retries_once_and_continues_with_later_feed() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let task = tokio::spawn(async move {
        for index in 0..3 {
            let (mut socket, _) = listener.accept().await.unwrap();
            count.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = [0; 4096];
                let _ = socket.read(&mut buf).await;
                if index < 2 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                let body = rss("https://site.test", 0, 3);
                let _=socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await;
            });
        }
    });
    let pool = pool().await;
    let result = import_service::import_with_timeout(
        &pool,
        &reqwest::Client::new(),
        vec![
            entry(&format!("{base}/slow")),
            entry(&format!("{base}/good")),
        ],
        &mut |_| {},
        std::time::Duration::from_millis(100),
    )
    .await
    .unwrap();
    task.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!((result.errors.len(), result.feeds_added), (1, 1));
}

#[test]
fn url_identity_preserves_meaningful_paths_and_query_parameters() {
    let parsed = parser::parse_feed_bytes(rss("https://site.test", 0, 3).as_bytes(), None).unwrap();
    assert_eq!(parsed.site_url.as_deref(), Some("https://site.test/"));
    assert_eq!(
        identity::url_key("http://www.Example.com/feed/#x"),
        identity::url_key("https://example.com/feed")
    );
    assert_ne!(
        identity::url_key("https://example.com/feed?a=1"),
        identity::url_key("https://example.com/feed?a=2")
    );
    assert_ne!(
        identity::url_key("https://example.com/A"),
        identity::url_key("https://example.com/a")
    );
    assert_ne!(
        identity::url_key("http://localhost:123/feed"),
        identity::url_key("http://localhost:456/feed")
    );
    assert!(identity::url_key("file:///feed").is_none());
    assert_ne!(
        identity::url_key("https://alice@example.com/feed"),
        identity::url_key("https://bob@example.com/feed")
    );
}

#[tokio::test]
#[ignore = "requires IMPORT_PUBLIC_SNAPSHOTS with previously fetched public feed bytes; no network"]
async fn recorded_ruanyifeng_and_appinn_aliases_are_deduplicated() {
    let dir = std::path::PathBuf::from(std::env::var("IMPORT_PUBLIC_SNAPSHOTS").unwrap());
    let pool = pool().await;
    let mut added = 0;
    for (index, url) in [
        (2, "https://www.ruanyifeng.com/blog/atom.xml"),
        (3, "http://feeds.feedburner.com/ruanyifeng"),
        (20, "http://feed.appinn.com/"),
        (26, "https://www.appinn.com/feed/"),
    ] {
        let bytes = std::fs::read(dir.join(format!("response-{index:02}.bin"))).unwrap();
        let parsed = parser::parse_feed_bytes(&bytes, Some(url)).unwrap();
        let (_, inserted) = feed_store::save_imported_feed(&pool, url, url, "Fallback", parsed)
            .await
            .unwrap();
        added += usize::from(inserted);
    }
    assert_eq!(added, 2);
    assert_eq!(feed_store::list_feeds(&pool).await.unwrap().len(), 2);
}
