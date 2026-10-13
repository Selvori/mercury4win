//! M4W-005: production parser/store/reader with isolated in-memory SQLite and
//! local HTTP responses. No real subscriptions, credentials or remote requests.
use crate::db::{content_store, entry_store, feed_store, migrations};
use crate::feed::{opml_export, opml_import, parser};
use crate::reader::{pipeline, theme::FontConfig};
use deadpool_sqlite::{Config, Pool, Runtime};
use reqwest::Client;
use std::collections::BTreeSet;
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
            c.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
            migrations::run_migrations(c).unwrap();
        })
        .await
        .unwrap();
    pool
}

fn rss(summary: &str, body: Option<&str>, url: &str) -> String {
    let content = body
        .map(|b| format!("<content:encoded><![CDATA[{b}]]></content:encoded>"))
        .unwrap_or_default();
    format!(
        r#"<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/"><channel>
        <title>Test</title><link>https://example.test</link><description>Test</description>
        <item><guid>stable-entry</guid><title>Test Article</title><link>{url}</link>
        <description><![CDATA[{summary}]]></description>{content}</item></channel></rss>"#
    )
}

async fn insert(pool: &Pool, xml: &str) -> i64 {
    let (feed, _) = feed_store::insert_feed(pool, "https://example.test/feed", Some("Test"), None)
        .await
        .unwrap();
    let parsed = parser::parse_feed_bytes(xml.as_bytes(), None).unwrap();
    entry_store::bulk_upsert_entries(pool, feed.id, parsed.entries)
        .await
        .unwrap();
    entry_store::query_entries(
        pool,
        &entry_store::EntryQueryParams {
            filter: entry_store::EntryFilter {
                feed_id: Some(feed.id),
                ..Default::default()
            },
            cursor: None,
            limit: 10,
        },
    )
    .await
    .unwrap()
    .entries[0]
        .id
}

async fn server(replies: Vec<(u16, String)>) -> (String, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/article", listener.local_addr().unwrap());
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    tokio::spawn(async move {
        for (status, body) in replies {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).await.unwrap();
            count.fetch_add(1, Ordering::SeqCst);
            let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    (url, requests)
}

fn article() -> String {
    format!("<html><head><title>Complete Article</title></head><body><article><h1>Complete Article</h1><p>FULL_ARTICLE_MARKER {}</p></article></body></html>",
        "Meaningful article prose with enough detail, sentences, and context to extract the complete body. ".repeat(25))
}

async fn read(pool: &Pool, id: i64) -> Result<String, String> {
    pipeline::build_reader_content(
        pool,
        &Client::builder().no_proxy().build().unwrap(),
        id,
        "classic-light",
        &FontConfig::default(),
    )
    .await
}

#[test]
fn all_formats_keep_teasers_separate_from_explicit_bodies() {
    let rss_full = rss(
        "RSS teaser",
        Some("<p>RSS full</p>"),
        "https://example.test/a",
    );
    let rss_teaser = rss("RSS teaser", None, "https://example.test/a");
    let atom_full = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Test</title><id>feed</id><updated>2026-10-12T00:00:00Z</updated><entry><id>a</id><title>A</title><updated>2026-10-12T00:00:00Z</updated><summary>Atom teaser</summary><content type="html">&lt;p&gt;Atom full&lt;/p&gt;</content></entry></feed>"#;
    let atom_teaser = atom_full.replace(
        "<content type=\"html\">&lt;p&gt;Atom full&lt;/p&gt;</content>",
        "",
    );
    let json_full = r#"{"version":"https://jsonfeed.org/version/1.1","title":"Test","items":[{"id":"a","summary":"JSON teaser","content_html":"<p>JSON full</p>"}]}"#;
    let json_teaser = json_full.replace(",\"content_html\":\"<p>JSON full</p>\"", "");
    for (source, expected_summary, expected_body) in [
        (rss_full.as_str(), "RSS teaser", Some("<p>RSS full</p>")),
        (rss_teaser.as_str(), "RSS teaser", None),
        (atom_full, "Atom teaser", Some("<p>Atom full</p>")),
        (atom_teaser.as_str(), "Atom teaser", None),
        (json_full, "JSON teaser", Some("<p>JSON full</p>")),
        (json_teaser.as_str(), "JSON teaser", None),
    ] {
        let parsed = parser::parse_feed_bytes(source.as_bytes(), None).unwrap();
        let entry = &parsed.entries[0];
        assert_eq!(entry.summary.as_deref(), Some(expected_summary));
        assert_eq!(entry.feed_content.as_deref(), expected_body);
        assert_eq!(
            entry.feed_content_type.as_deref(),
            expected_body.map(|_| "text/html")
        );
    }
}

#[tokio::test]
async fn teaser_fetches_full_article_and_cached_body_works_offline() {
    let pool = pool().await;
    let (url, count) = server(vec![(200, article())]).await;
    let id = insert(&pool, &rss("SHORT_TEASER", None, &url)).await;
    let html = read(&pool, id).await.unwrap();
    assert!(html.contains("FULL_ARTICLE_MARKER"));
    assert!(!html.contains(pipeline::SUMMARY_SOURCE_MARKER));
    let content = content_store::get_content(&pool, id)
        .await
        .unwrap()
        .unwrap();
    assert!(content
        .markdown
        .unwrap()
        .replace('\\', "")
        .contains("FULL_ARTICLE_MARKER"));
    assert_eq!(content.document_base_url.as_deref(), Some(url.as_str()));
    assert!(read(&pool, id)
        .await
        .unwrap()
        .contains("FULL_ARTICLE_MARKER"));
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn explicit_short_feed_body_is_readable_offline_and_invalidates_on_change() {
    let pool = pool().await;
    let id = insert(
        &pool,
        &rss(
            "teaser",
            Some("<p>SHORT_FULL_POST</p>"),
            "http://127.0.0.1:1/a",
        ),
    )
    .await;
    assert!(read(&pool, id).await.unwrap().contains("SHORT_FULL_POST"));
    assert!(
        pipeline::get_cached_reader_html(&pool, id, "classic-light:sans-serif:16")
            .await
            .unwrap()
            .unwrap()
            .contains("SHORT_FULL_POST")
    );
    entry_store::mark_read(&pool, &[id], true).await.unwrap();
    entry_store::mark_starred(&pool, id, true).await.unwrap();
    insert(
        &pool,
        &rss(
            "updated teaser",
            Some("<p>SHORT_FULL_POST</p>"),
            "http://127.0.0.1:1/a",
        ),
    )
    .await;
    assert!(content_store::get_content(&pool, id)
        .await
        .unwrap()
        .is_some());
    insert(
        &pool,
        &rss(
            "teaser",
            Some("<p>CHANGED_FULL_POST</p>"),
            "http://127.0.0.1:1/a",
        ),
    )
    .await;
    assert!(content_store::get_content(&pool, id)
        .await
        .unwrap()
        .is_none());
    assert!(
        pipeline::get_cached_reader_html(&pool, id, "classic-light:sans-serif:16")
            .await
            .unwrap()
            .is_none()
    );
    let detail = entry_store::get_entry_detail(&pool, id)
        .await
        .unwrap()
        .unwrap();
    assert!(detail.entry.is_read && detail.entry.is_starred);
    assert!(read(&pool, id).await.unwrap().contains("CHANGED_FULL_POST"));
}

#[tokio::test]
async fn failed_fetch_shows_safe_summary_without_persisting_and_can_retry() {
    let pool = pool().await;
    let (url, count) = server(vec![(503, "Unavailable".into()), (200, article())]).await;
    let id = insert(
        &pool,
        &rss(
            "<p>VISIBLE_TEASER &amp; 中文</p><img src=x onerror=alert(1)>",
            None,
            &url,
        ),
    )
    .await;
    let html = read(&pool, id).await.unwrap();
    assert!(html.contains("VISIBLE_TEASER &amp; 中文"));
    assert!(html.contains(pipeline::SUMMARY_SOURCE_MARKER));
    assert!(!html.contains("onerror"));
    assert!(content_store::get_content(&pool, id)
        .await
        .unwrap()
        .is_none());
    let cache_count: i64 = pool
        .get()
        .await
        .unwrap()
        .interact(|c| {
            c.query_row("SELECT COUNT(*) FROM content_html_cache", [], |r| r.get(0))
                .unwrap()
        })
        .await
        .unwrap();
    assert_eq!(cache_count, 0);
    assert!(read(&pool, id)
        .await
        .unwrap()
        .contains("FULL_ARTICLE_MARKER"));
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn offline_or_missing_url_keeps_legacy_summary_visible_without_claiming_full_text() {
    for url in ["http://127.0.0.1:1/a", ""] {
        let pool = pool().await;
        let id = insert(&pool, &rss("LEGACY_OFFLINE_BODY", None, url)).await;
        let html = read(&pool, id).await.unwrap();
        assert!(html.contains("LEGACY_OFFLINE_BODY"));
        assert!(html.contains(pipeline::SUMMARY_SOURCE_MARKER));
        assert!(content_store::get_content(&pool, id)
            .await
            .unwrap()
            .is_none());
    }
}

#[tokio::test]
async fn plain_text_feed_body_is_literal_and_no_summary_is_an_error() {
    let pool = pool().await;
    let json = r#"{"version":"https://jsonfeed.org/version/1.1","title":"Test","items":[{"id":"a","url":"http://127.0.0.1:1/a","content_text":"PLAIN_FULL <b>literal</b> 中文"}]}"#;
    let id = insert(&pool, json).await;
    let detail = entry_store::get_entry_detail(&pool, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        detail.entry.feed_content_type.as_deref(),
        Some("text/plain")
    );
    let html = read(&pool, id).await.unwrap();
    assert!(
        html.contains("PLAIN_FULL &lt;b&gt;literal&lt;/b&gt; 中文"),
        "{html}"
    );
    let empty_pool = self::pool().await;
    let id = insert(&empty_pool, &rss("", None, "http://127.0.0.1:1/a")).await;
    assert!(read(&empty_pool, id).await.is_err());
}

#[tokio::test]
async fn explicit_rss_atom_xhtml_and_json_bodies_preserve_short_posts_without_fetching() {
    let cases = [
        rss("", Some("<p>EXPLICIT_POST</p>"), "http://127.0.0.1:1/a"),
        r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Test</title><id>feed</id><updated>2026-10-12T00:00:00Z</updated><entry><id>a</id><title>A</title><updated>2026-10-12T00:00:00Z</updated><link rel="alternate" href="http://127.0.0.1:1/a"/><content type="html">&lt;p&gt;EXPLICIT_POST&lt;/p&gt;</content></entry></feed>"#.into(),
        r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Test</title><id>feed</id><updated>2026-10-12T00:00:00Z</updated><entry><id>a</id><title>A</title><updated>2026-10-12T00:00:00Z</updated><link rel="alternate" href="http://127.0.0.1:1/a"/><content type="xhtml"><div xmlns="http://www.w3.org/1999/xhtml"><p>EXPLICIT_POST</p></div></content></entry></feed>"#.into(),
        r#"{"version":"https://jsonfeed.org/version/1.1","title":"Test","items":[{"id":"a","url":"http://127.0.0.1:1/a","content_html":"<p>EXPLICIT_POST</p>"}]}"#.into(),
    ];
    for source in cases {
        let pool = pool().await;
        let id = insert(&pool, &source).await;
        let html = read(&pool, id).await.unwrap();
        assert!(html.contains("EXPLICIT_POST"));
        assert!(!html.contains(pipeline::SUMMARY_SOURCE_MARKER));
        assert!(content_store::get_content(&pool, id)
            .await
            .unwrap()
            .is_some());
    }
}

#[tokio::test]
async fn rejected_article_responses_do_not_poison_cache() {
    for body in [
        "",
        "<script>_cf_chl_opt</script>",
        "<html><body>too short</body></html>",
    ] {
        let pool = pool().await;
        let (url, _) = server(vec![(200, body.into())]).await;
        let id = insert(&pool, &rss("SAFE_TEASER", None, &url)).await;
        let html = read(&pool, id).await.unwrap();
        assert!(html.contains("SAFE_TEASER"));
        assert!(html.contains(pipeline::SUMMARY_SOURCE_MARKER));
        assert!(content_store::get_content(&pool, id)
            .await
            .unwrap()
            .is_none());
    }
}

#[tokio::test]
async fn fixed_opml_parse_store_repeat_partial_and_export_roundtrip() {
    use sha2::{Digest, Sha256};
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/opml/hn-popular10+1.opml");
    assert_eq!(
        format!("{:X}", Sha256::digest(std::fs::read(&fixture).unwrap())),
        "14BF86ADC6146B3430FE901B5E467CBB679609A54F38F2708097D098C49783F3"
    );
    let entries = opml_import::parse_opml_file(&fixture).unwrap();
    assert_eq!(entries.len(), 11);
    assert_eq!(entries.iter().filter(|e| e.site_url.is_some()).count(), 10);
    assert!(entries.iter().all(|e| e.feed_url.starts_with("https://")));
    let expected: BTreeSet<_> = entries.iter().map(|e| e.feed_url.clone()).collect();
    assert_eq!(expected.len(), 11);
    let pool = pool().await;
    for (round, expected_added) in [(0, 3), (1, 8), (2, 0)] {
        let subset = if round == 0 {
            &entries[..3]
        } else {
            entries.as_slice()
        };
        let mut added = 0;
        for e in subset {
            let (_, inserted) =
                feed_store::insert_feed(&pool, &e.feed_url, Some(&e.title), e.site_url.as_deref())
                    .await
                    .unwrap();
            added += usize::from(inserted);
        }
        assert_eq!(added, expected_added);
        if round == 0 {
            pool.get().await.unwrap().interact(|c| c.execute_batch("INSERT INTO entry(id,feed_id,guid,summary,is_read,is_starred) VALUES(1,1,'keep','keep',1,1); INSERT INTO entry_note VALUES(1,'keep note','now'); INSERT INTO tag(id,name,normalized_name) VALUES(1,'keep','keep'); INSERT INTO entry_tag(entry_id,tag_id) VALUES(1,1);")).await.unwrap().unwrap();
        }
    }
    let saved = feed_store::list_feeds(&pool).await.unwrap();
    assert_eq!(saved.len(), 11);
    let detail = entry_store::get_entry_detail(&pool, 1)
        .await
        .unwrap()
        .unwrap();
    assert!(detail.entry.is_read && detail.entry.is_starred && detail.has_note);
    assert_eq!(detail.tags.len(), 1);
    let exported: Vec<_> = saved
        .into_iter()
        .map(|e| opml_export::OpmlFeedData {
            title: e.title.unwrap(),
            feed_url: e.feed_url,
            site_url: e.site_url,
        })
        .collect();
    let path =
        std::env::temp_dir().join(format!("mercury4win-m4w005-{}.opml", uuid::Uuid::new_v4()));
    opml_export::export_opml_file(&path, &exported, &[]).unwrap();
    let roundtrip = opml_import::parse_opml_file(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        roundtrip
            .iter()
            .map(|e| e.feed_url.clone())
            .collect::<BTreeSet<_>>(),
        expected
    );
    let other = self::pool().await;
    for e in roundtrip {
        feed_store::insert_feed(&other, &e.feed_url, Some(&e.title), e.site_url.as_deref())
            .await
            .unwrap();
    }
    assert_eq!(feed_store::list_feeds(&other).await.unwrap().len(), 11);
}
