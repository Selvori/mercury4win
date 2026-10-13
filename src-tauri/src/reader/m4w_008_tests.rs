//! Production Reader regressions using isolated SQLite and local HTTP only.
use super::{html_to_md, pipeline, theme::FontConfig};
use crate::db::{content_store, migrations};
use deadpool_sqlite::{Config, Pool, Runtime};
use reqwest::Client;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn pool(url: Option<&str>, body: Option<&str>) -> Pool {
    let pool = Config::new(":memory:")
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    let url = url.map(str::to_owned);
    let body = body.map(str::to_owned);
    pool.get().await.unwrap().interact(move |c| {
        migrations::run_migrations(c).unwrap();
        c.execute("INSERT INTO feed(id,feed_url) VALUES(1,'https://example.test/feed')", []).unwrap();
        c.execute("INSERT INTO entry(id,feed_id,guid,url,feed_content,feed_content_type) VALUES(1,1,'a',?1,?2,'text/html')", rusqlite::params![url,body]).unwrap();
    }).await.unwrap();
    pool
}
async fn read(pool: &Pool, theme: &str, font: &FontConfig) -> Result<String, String> {
    pipeline::build_reader_content(
        pool,
        &Client::builder().no_proxy().build().unwrap(),
        1,
        theme,
        font,
    )
    .await
}
async fn sql(pool: &Pool, sql: &str) {
    let sql = sql.to_owned();
    pool.get()
        .await
        .unwrap()
        .interact(move |c| c.execute_batch(&sql).unwrap())
        .await
        .unwrap();
}

#[test]
fn base_resolution_and_url_safety_cover_fragments_entities_and_protocols() {
    let url = "https://example.test/articles/page";
    assert_eq!(
        html_to_md::document_base_url("<base href='../media/'>", Some(url)).as_deref(),
        Some("https://example.test/media/")
    );
    for href in [
        "javascript:alert(1)",
        "file:///C:/secret",
        "data:text/html,x",
    ] {
        assert_eq!(
            html_to_md::document_base_url(&format!("<base href='{href}'>"), Some(url)).as_deref(),
            Some(url)
        );
    }
    let md = html_to_md::convert_html_to_markdown_with_base(r##"<p><a href='../next?q=a&amp;b=c'>next</a><a href='#section'>section</a><a href='javascript:alert(1)'>bad</a><a href='file:///secret'>local</a><img src='/image.png' onerror='bad()'><img src='//cdn.example.test/p.png'><script>alert(1)</script></p>"##, Some(url)).unwrap();
    assert!(md.contains("https://example.test/next?q=a&b=c"), "{md}");
    assert!(md.contains("https://example.test/image.png"));
    assert!(md.contains("https://cdn.example.test/p.png"));
    assert!(md.contains("#section"));
    assert!(!md.contains("javascript:") && !md.contains("file:") && !md.contains("onerror"));
    let html = super::md_to_html::render_markdown(&md, "classic-light");
    assert!(!html.contains("<script") && !html.contains("onerror"));
}

#[tokio::test]
async fn redirect_base_persist_and_rebuild_offline_without_joining_base_twice() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let url = format!("{origin}/original");
    let article = format!("<html><head><title>Redirected article</title><base href='assets/'></head><body><article><h1>Redirected article</h1><p>{}<a href='next.html'>next</a><img src='picture.png'></p></article></body></html>", "Article text with meaningful sentences and detailed context. ".repeat(40));
    tokio::spawn(async move {
        for (status, headers, body) in [
            (302, "Location: /final/page\r\n", "".to_string()),
            (200, "", article),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            socket.read(&mut request).await.unwrap();
            socket.write_all(format!("HTTP/1.1 {status} Test\r\n{headers}Content-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    let pool = pool(Some(&url), None).await;
    let html = read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    assert!(
        html.contains(&format!("{origin}/final/assets/next.html")),
        "{html}"
    );
    assert!(html.contains(&format!("{origin}/final/assets/picture.png")));
    let c = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    assert_eq!(c.document_base_url, Some(format!("{origin}/final/assets/")));
    sql(
        &pool,
        "UPDATE content SET readability_version=0,markdown_version=0",
    )
    .await;
    let rebuilt = read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    assert!(rebuilt.contains(&format!("{origin}/final/assets/next.html")));
    assert!(!rebuilt.contains("/final/assets/assets/"));
}

#[tokio::test]
async fn markdown_rebuild_persists_and_invalidates_other_theme_caches() {
    let pool = pool(Some("https://example.test/a"), Some("<p>SOURCE_BODY</p>")).await;
    read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    read(&pool, "classic-dark", &FontConfig::default())
        .await
        .unwrap();
    sql(&pool,"UPDATE content SET markdown='STALE',markdown_version=0,cleaned_html='<p>FRESH_INTERMEDIATE</p>'").await;
    assert!(read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap()
        .contains("FRESH_INTERMEDIATE"));
    let c = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    assert!(c.markdown.unwrap().contains("FRESH\\_INTERMEDIATE"));
    assert_eq!(c.markdown_version, Some(pipeline::MARKDOWN_VERSION));
    assert!(
        pipeline::get_cached_reader_html(&pool, 1, "classic-dark:sans-serif:16")
            .await
            .unwrap()
            .is_none()
    );
    assert!(read(&pool, "classic-dark", &FontConfig::default())
        .await
        .unwrap()
        .contains("FRESH_INTERMEDIATE"));
}

#[tokio::test]
async fn stale_readability_overrides_current_markdown_and_cache_without_fetching() {
    let pool = pool(Some("http://127.0.0.1:1/a"), Some("<p>FRESH_SOURCE</p>")).await;
    read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    sql(&pool,"UPDATE content SET readability_version=NULL,cleaned_html='<p>STALE_HTML</p>',markdown='STALE_MARKDOWN'; UPDATE content_html_cache SET html='STALE_CACHE'").await;
    let html = read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    assert!(html.contains("FRESH_SOURCE") && !html.contains("STALE"));
    assert_eq!(
        content_store::get_content(&pool, 1)
            .await
            .unwrap()
            .unwrap()
            .readability_version,
        Some(pipeline::READABILITY_VERSION)
    );
}

#[tokio::test]
async fn cache_hit_does_not_write_and_render_only_miss_preserves_markdown() {
    let pool = pool(None, Some("<p>FULL_BODY</p>")).await;
    let first = read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    sql(&pool,"CREATE TRIGGER no_cache_write BEFORE INSERT ON content_html_cache BEGIN SELECT RAISE(FAIL,'unexpected cache write'); END;").await;
    assert_eq!(
        read(&pool, "classic-light", &FontConfig::default())
            .await
            .unwrap(),
        first
    );
    sql(&pool,"DROP TRIGGER no_cache_write; UPDATE content_html_cache SET reader_render_version=0; CREATE TRIGGER no_content_write BEFORE UPDATE ON content BEGIN SELECT RAISE(FAIL,'unexpected content write'); END;").await;
    assert_eq!(
        read(&pool, "classic-light", &FontConfig::default())
            .await
            .unwrap(),
        first
    );
    let other = read(
        &pool,
        "classic-dark",
        &FontConfig {
            family: "serif".into(),
            size: 22,
        },
    )
    .await
    .unwrap();
    assert!(
        other.contains("font-size: 22px") && other.contains("Georgia") && other.contains("#1a1a2e")
    );
    assert_ne!(other, first);
}

#[tokio::test]
async fn empty_is_terminal_but_legacy_empty_and_transient_failures_retry() {
    let pool = pool(None, None).await;
    assert!(read(&pool, "classic-light", &FontConfig::default())
        .await
        .is_err());
    assert_eq!(
        content_store::get_content(&pool, 1)
            .await
            .unwrap()
            .unwrap()
            .display_mode,
        "empty"
    );
    sql(&pool,"CREATE TRIGGER no_content_write BEFORE UPDATE ON content BEGIN SELECT RAISE(FAIL,'unexpected terminal write'); END;").await;
    assert_eq!(
        read(&pool, "classic-light", &FontConfig::default())
            .await
            .unwrap_err(),
        "No article content is available."
    );
    sql(&pool,"DROP TRIGGER no_content_write; UPDATE content SET readability_version=1,markdown_version=1; UPDATE entry SET feed_content='<p>RESTORED_BODY</p>'").await;
    assert!(read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap()
        .contains("RESTORED_BODY"));
    let failed = self::pool(Some("http://127.0.0.1:1/a"), None).await;
    for _ in 0..2 {
        assert!(read(&failed, "classic-light", &FontConfig::default())
            .await
            .is_err());
        assert!(content_store::get_content(&failed, 1)
            .await
            .unwrap()
            .is_none());
    }
}

#[tokio::test]
async fn failed_rebuild_keeps_persisted_layers_and_cache_unchanged() {
    let pool = pool(Some("http://127.0.0.1:1/a"), Some("<p>LAST_GOOD</p>")).await;
    read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    sql(&pool,"UPDATE entry SET feed_content=NULL; UPDATE content SET readability_version=0,html='<html><body>short</body></html>'").await;
    let before = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    let cache = pipeline::get_cached_reader_html(&pool, 1, "classic-light:sans-serif:16")
        .await
        .unwrap();
    assert!(read(&pool, "classic-light", &FontConfig::default())
        .await
        .is_err());
    let after = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    assert_eq!(after.markdown, before.markdown);
    assert_eq!(after.cleaned_html, before.cleaned_html);
    assert_eq!(after.readability_version, before.readability_version);
    assert_eq!(
        pipeline::get_cached_reader_html(&pool, 1, "classic-light:sans-serif:16")
            .await
            .unwrap(),
        cache
    );
}

#[tokio::test]
async fn legacy_version_one_entry_base_is_resolved_on_local_rebuild() {
    let pool = pool(
        Some("https://example.test/posts/page"),
        Some("<base href='media/'><p><a href='next.html'>next</a></p>"),
    )
    .await;
    read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    sql(&pool,"UPDATE content SET readability_version=1,markdown_version=1,document_base_url='https://example.test/posts/page'").await;
    let html = read(&pool, "classic-light", &FontConfig::default())
        .await
        .unwrap();
    assert!(html.contains("https://example.test/posts/media/next.html"));
    assert_eq!(
        content_store::get_content(&pool, 1)
            .await
            .unwrap()
            .unwrap()
            .document_base_url
            .as_deref(),
        Some("https://example.test/posts/media/")
    );
}
