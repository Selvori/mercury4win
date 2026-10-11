//! Isolated probes for the 2026-10-12 audit; see mercury-porting-2026-10-12.md.
use crate::{agent, db, reader};
use deadpool_sqlite::{Config, Pool, Runtime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn fixture() -> Pool {
    let pool = Config::new(":memory:")
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
            db::migrations::run_migrations(conn).unwrap();
            conn.execute_batch(
                "INSERT INTO feed(id,feed_url) VALUES(1,'https://example.test/feed');
          INSERT INTO entry(id,feed_id,guid,title,published_at) VALUES
          (1,1,'new','Newest','2026-10-12'),(2,1,'old','Older','2026-10-11');
          INSERT INTO content(entry_id,markdown,markdown_version) VALUES(1,'First paragraph.',1);",
            )
            .unwrap();
        })
        .await
        .unwrap();
    pool
}

async fn count(pool: &Pool, table: &'static str) -> i64 {
    pool.get()
        .await
        .unwrap()
        .interact(move |conn| {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
        })
        .await
        .unwrap()
}

async fn mock_http(
    status: &str,
    content_type: &str,
    body: &str,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let response = format!("HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let n = stream.read(&mut buffer).await.unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..n]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse().ok())
                    })
                    .unwrap_or(0);
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    (url, task)
}

#[tokio::test]
async fn pagination_retains_older_entries_with_higher_ids() {
    let pool = fixture().await;
    let mut params = db::entry_store::EntryQueryParams {
        feed_id: None,
        unread_only: false,
        search: None,
        tag_ids: None,
        cursor: None,
        limit: 1,
    };
    let first = db::entry_store::query_entries(&pool, &params)
        .await
        .unwrap();
    assert_eq!(first.entries[0].id, 1);
    params.cursor = first.next_cursor.unwrap().parse().ok();
    let second = db::entry_store::query_entries(&pool, &params)
        .await
        .unwrap();
    assert_eq!(
        second.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![2]
    );
}

#[tokio::test]
async fn tag_merge_succeeds() {
    let pool = fixture().await;
    let source = db::tag_store::add_tag(&pool, 1, "Source").await.unwrap();
    let target = db::tag_store::add_tag(&pool, 2, "Target").await.unwrap();
    db::tag_store::merge_tags(&pool, source.id, target.id)
        .await
        .unwrap();
    assert_eq!(count(&pool, "tag").await, 1);
}

#[tokio::test]
async fn equivalent_tag_separators_reuse_canonical_tag() {
    let pool = fixture().await;
    let first = db::tag_store::add_tag(&pool, 1, "Machine-Learning")
        .await
        .unwrap();
    let second = db::tag_store::add_tag(&pool, 2, "machine learning")
        .await
        .unwrap();
    assert_eq!(first.id, second.id);
}

#[tokio::test]
async fn successful_translation_is_persisted() {
    let pool = fixture().await;
    let (url, server) = mock_http("200 OK", "application/json", r#"{"choices":[{"message":{"role":"assistant","content":"translated"}}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#).await;
    agent::translation::run_translation(
        &pool,
        &reqwest::Client::new(),
        1,
        "en",
        &url,
        "",
        "audit",
        None,
        3,
        tauri::ipc::Channel::new(|_| Ok(())),
    )
    .await
    .unwrap();
    server.await.unwrap();
    assert_eq!(count(&pool, "translation_result").await, 1);
    assert_eq!(count(&pool, "translation_segment").await, 1);
}

#[tokio::test]
async fn failed_translation_does_not_report_success() {
    let pool = fixture().await;
    let (url, server) = mock_http(
        "500 Internal Server Error",
        "application/json",
        r#"{"error":"audit failure"}"#,
    )
    .await;
    let result = agent::translation::run_translation(
        &pool,
        &reqwest::Client::new(),
        1,
        "en",
        &url,
        "",
        "audit",
        None,
        3,
        tauri::ipc::Channel::new(|_| Ok(())),
    )
    .await;
    server.await.unwrap();
    assert!(
        result.is_err(),
        "all segments failed but returned {result:?}"
    );
}

#[tokio::test]
async fn successful_summary_is_persisted() {
    let pool = fixture().await;
    let (url, server) = mock_http(
        "200 OK",
        "text/event-stream",
        "data: {\"choices\":[{\"delta\":{\"content\":\"audit summary\"}}]}\n\ndata: [DONE]\n\n",
    )
    .await;
    agent::summary::run_summary(
        &pool,
        &reqwest::Client::new(),
        &agent::runtime::create_runtime(),
        1,
        "en",
        "brief",
        &url,
        "",
        "audit",
        tauri::ipc::Channel::new(|_| Ok(())),
    )
    .await
    .unwrap();
    server.await.unwrap();
    assert_eq!(count(&pool, "summary_result").await, 1);
}

#[tokio::test]
async fn outdated_markdown_is_rebuilt_from_cleaned_html() {
    let pool = fixture().await;
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            conn.execute_batch(
                "UPDATE content SET markdown='obsolete text', markdown_version=0,
          cleaned_html='<p>Rebuilt article body</p>',readability_version=1 WHERE entry_id=1;",
            )
            .unwrap();
        })
        .await
        .unwrap();
    let html = reader::pipeline::build_reader_content(
        &pool,
        &reqwest::Client::new(),
        1,
        "classic-light",
        &reader::theme::FontConfig::default(),
    )
    .await
    .unwrap();
    assert!(
        html.contains("Rebuilt article body"),
        "stale markdown was reused"
    );
}

#[test]
fn fallback_html_is_visible_to_reader() {
    let html = reader::md_to_html::render_markdown(
        "<h1>Audit fallback title</h1>\n<p>Full article content is not available.</p>",
        "classic-light",
    );
    assert!(
        html.contains("Full article content is not available."),
        "fallback HTML omitted"
    );
}

#[test]
fn heading_failure_is_not_missing_content() {
    let html = reader::md_to_html::render_markdown("# Title\n\nContent here.", "classic-light");
    assert!(html.contains("Title</h1>"));
    assert!(html.contains("Content here."));
}

#[test]
fn rss_atom_and_json_feed_parse() {
    for feed in [
        r#"<rss version="2.0"><channel><title>Audit</title><link>https://example.test</link><description>Audit</description><item><guid>x</guid><title>Item</title></item></channel></rss>"#,
        r#"<feed xmlns="http://www.w3.org/2005/Atom"><id>urn:audit</id><title>Audit</title><updated>2026-10-12T00:00:00Z</updated><entry><id>urn:entry</id><title>Item</title><updated>2026-10-12T00:00:00Z</updated></entry></feed>"#,
        r#"{"version":"https://jsonfeed.org/version/1.1","title":"Audit","items":[{"id":"entry","content_text":"Hello"}]}"#,
    ] {
        assert_eq!(
            crate::feed::parser::parse_feed_bytes(
                feed.as_bytes(),
                Some("https://example.test/feed")
            )
            .unwrap()
            .entries
            .len(),
            1
        );
    }
}

#[tokio::test]
async fn note_storage_roundtrip_preserves_other_entries() {
    let pool = fixture().await;
    db::note_store::save_note(&pool, 1, "first note")
        .await
        .unwrap();
    db::note_store::save_note(&pool, 2, "second note")
        .await
        .unwrap();
    db::note_store::save_note(&pool, 1, "updated note")
        .await
        .unwrap();
    assert_eq!(
        db::note_store::get_note(&pool, 1)
            .await
            .unwrap()
            .unwrap()
            .markdown,
        "updated note"
    );
    assert_eq!(
        db::note_store::get_note(&pool, 2)
            .await
            .unwrap()
            .unwrap()
            .markdown,
        "second note"
    );
    db::note_store::delete_note(&pool, 1).await.unwrap();
    assert!(db::note_store::get_note(&pool, 1).await.unwrap().is_none());
}

#[tokio::test]
async fn reader_resolves_relative_images_against_article_url() {
    let pool = fixture().await;
    pool.get()
        .await
        .unwrap()
        .interact(|conn| {
            conn.execute_batch(
                "DELETE FROM content;
          UPDATE entry SET url='https://example.test/articles/item',
          summary='<p>Article body</p><img src=\"/image.png\" alt=\"Audit\">' WHERE id=1;",
            )
            .unwrap();
        })
        .await
        .unwrap();
    let html = reader::pipeline::build_reader_content(
        &pool,
        &reqwest::Client::new(),
        1,
        "classic-light",
        &reader::theme::FontConfig::default(),
    )
    .await
    .unwrap();
    assert!(
        html.contains("https://example.test/image.png")
            || html.contains("<base href=\"https://example.test/"),
        "article base URL was not applied"
    );
}

#[tokio::test]
async fn teaser_feed_does_not_replace_full_article() {
    let pool = fixture().await;
    let body = format!("<html><head><title>Article</title></head><body><article><h1>Article</h1><p>FULL_ARTICLE_MARKER {}</p></article></body></html>", "This is useful article text with sufficient detail and explanation. ".repeat(20));
    let (url, server) = mock_http("200 OK", "text/html", &body).await;
    pool.get()
        .await
        .unwrap()
        .interact(move |conn| {
            conn.execute_batch("DELETE FROM content;").unwrap();
            conn.execute(
                "UPDATE entry SET url=?1,summary='<p>Brief teaser only.</p>' WHERE id=1",
                [url],
            )
            .unwrap();
        })
        .await
        .unwrap();
    let html = reader::pipeline::build_reader_content(
        &pool,
        &reqwest::Client::new(),
        1,
        "classic-light",
        &reader::theme::FontConfig::default(),
    )
    .await
    .unwrap();
    server.abort();
    assert!(
        html.contains("FULL_ARTICLE_MARKER"),
        "feed teaser suppressed full article fetch"
    );
}
