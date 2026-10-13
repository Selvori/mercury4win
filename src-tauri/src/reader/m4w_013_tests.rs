//! M4W-013: fixture scripts are data, loopback HTTP and in-memory DB only.
use super::{obsidian, pipeline, theme::FontConfig};
use crate::db::{content_store, migrations};
use deadpool_sqlite::{Config, Pool, Runtime};
use reqwest::Client;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const UID: &str = "8a529a39ad1753175d73ccc6abc0547e";

fn shell(origin: &str, path: &str) -> String {
    let host = url::Url::parse(origin)
        .unwrap()
        .host_str()
        .unwrap()
        .to_string();
    format!(
        r#"<!doctype html><html><head><base href="https://publish.obsidian.md"><script>
window.siteInfo={{"uid":"{UID}","host":"{host}","customurl":"example.test"}};
window.preloadPage=f("{origin}/access/{UID}/{path}");
</script></head><body class="theme-light"><div class="preload">Loading</div></body></html>"#
    )
}
async fn pool(url: &str) -> Pool {
    let pool = Config::new(":memory:")
        .builder(Runtime::Tokio1)
        .unwrap()
        .max_size(1)
        .build()
        .unwrap();
    let url = url.to_string();
    pool.get()
        .await
        .unwrap()
        .interact(move |c| {
            migrations::run_migrations(c).unwrap();
            c.execute(
                "INSERT INTO feed(id,feed_url) VALUES(1,'https://example.test/rss')",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO entry(id,feed_id,guid,title,url) VALUES(1,1,'a','Publish article',?1)",
                [url],
            )
            .unwrap();
        })
        .await
        .unwrap();
    pool
}
async fn sql(pool: &Pool, sql: &str) {
    let sql = sql.to_string();
    pool.get()
        .await
        .unwrap()
        .interact(move |c| c.execute_batch(&sql).unwrap())
        .await
        .unwrap();
}
async fn read(pool: &Pool) -> Result<String, String> {
    pipeline::build_reader_content(
        pool,
        &Client::builder().no_proxy().build().unwrap(),
        1,
        "classic-light",
        &FontConfig::default(),
    )
    .await
}
async fn server(
    responses: Vec<(String, u16, String, String)>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let replacement = origin.clone();
    let task = tokio::spawn(async move {
        for (path, status, headers, body) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 8192];
            let n = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..n]);
            assert_eq!(
                request.split_whitespace().nth(1),
                Some(path.as_str()),
                "{request}"
            );
            let body = body.replace("ORIGIN", &replacement);
            socket.write_all(format!("HTTP/1.1 {status} Test\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    (origin, task)
}

#[test]
fn m4w_013_resolver_real_upstream_shape_custom_domains_and_strict_targets() {
    let html = include_str!("../../../tests/obsidian_publish/upstream-shell.html");
    let url = obsidian::resolve(
        html,
        "https://chadnauseam.com/coding/tips/give-them-two-choices",
    )
    .unwrap();
    assert_eq!(
        url.as_str(),
        format!("https://publish-01.obsidian.md/access/{UID}/coding/tips/give-them-two-choices.md")
    );
    let escaped = shell(
        "https://publish-01.obsidian.md",
        "folder/中文%20x.md?q=a&b=c",
    )
    .replace("https://publish-01", "https:\\/\\/publish-01");
    assert!(
        obsidian::resolve(&escaped, "https://publish.obsidian.md/slug/page")
            .unwrap()
            .as_str()
            .contains("%E4%B8%AD%E6%96%87%20x.md")
    );
    let relative = shell("https://publish-01.obsidian.md", "folder/page.md").replace(
        &format!("https://publish-01.obsidian.md/access/{UID}/"),
        &format!("/access/{UID}/"),
    );
    assert!(obsidian::resolve(&relative, "https://publish-01.obsidian.md/final").is_some());
    for bad in [
        html.replace("window.siteInfo", "window.other"),
        html.replace("window.preloadPage=f(", "// window.preloadPage=f("),
        html.replace(
            "give-them-two-choices.md\");",
            "give-them-two-choices.md\" + suffix);",
        ),
        html.replace("publish-01.obsidian.md", "evil.example"),
        html.replace(&format!("/access/{UID}/"), "/access/wrong/"),
        html.replace(".md", ".html"),
        html.replace("<base href=\"https://publish.obsidian.md\">", ""),
        format!("<pre>{}</pre>", html_escape::encode_text(html)),
    ] {
        assert!(
            obsidian::resolve(&bad, "https://example.test/page").is_none(),
            "{bad}"
        );
    }
    assert!(obsidian::resolve(
        "<article>ordinary window.siteInfo text</article>",
        "https://example.test"
    )
    .is_none());
}

#[tokio::test]
async fn m4w_013_redirect_media_safety_persistence_cache_and_consumers() {
    let source =
        shell("http://127.0.0.1:12345", "old.md").replace("http://127.0.0.1:12345", "ORIGIN");
    let md = "# 中文正文\n\nPublish content. [next](../next?q=a&b=c) ![picture](../images/a%20b.png) [anchor](#中文) [unsafe](javascript:alert(1)) ![local](file:///secret)\n\n![[中文 image.png|截图]]\n\n![[中文%20image.png|编码截图]]\n\n![[duplicate.png]]\n\n`![[中文 image.png]]`\n\n```text\n![[中文 image.png]]\n```\n\n<script>alert(1)</script>\n";
    let (origin, task) = server(vec![
        (
            "/original".into(),
            302,
            "Location: /custom/page\r\n".into(),
            String::new(),
        ),
        (
            "/custom/page".into(),
            200,
            "Content-Type: text/html\r\n".into(),
            source,
        ),
        (
            format!("/access/{UID}/old.md"),
            302,
            format!("Location: /access/{UID}/folder/%E4%B8%AD%E6%96%87%20page.md\r\n"),
            String::new(),
        ),
        (
            format!("/access/{UID}/folder/%E4%B8%AD%E6%96%87%20page.md"),
            200,
            "Content-Type: text/plain; charset=utf-8\r\n".into(),
            md.into(),
        ),
        (
            format!("/cache/{UID}"),
            200,
            "Content-Type: application/json\r\n".into(),
            r#"{"images/中文 image.png":{},"a/duplicate.png":{},"b/duplicate.png":{}}"#.into(),
        ),
    ])
    .await;
    let pool = pool(&format!("{origin}/original")).await;
    let html = read(&pool).await.unwrap();
    task.await.unwrap();
    assert!(
        html.contains("中文正文")
            && html.contains(&format!("{origin}/access/{UID}/next?q=a&amp;b=c")),
        "{html}"
    );
    assert!(html.contains(&format!("{origin}/access/{UID}/images/a%20b.png")));
    assert!(
        html.contains(&format!(
            "{origin}/access/{UID}/images/%E4%B8%AD%E6%96%87%20image.png"
        )),
        "{html}"
    );
    assert!(html.contains("alt=\"编码截图\""), "{html}");
    assert!(
        !html.contains("<script>") && !html.contains("javascript:") && !html.contains("file:///")
    );
    assert!(
        html.contains("![[duplicate.png]]") && html.contains("<code>![[中文 image.png]]</code>")
    );
    let c = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    assert_eq!(c.pipeline_type, "obsidian");
    assert_eq!(c.document_base_url, Some(format!("{origin}/custom/page")));
    assert_eq!(
        c.resolved_intermediate_content,
        Some(format!("{origin}/access/{UID}/old.md"))
    );
    assert!(
        c.cleaned_html.is_none()
            && c.readability_version.is_none()
            && pipeline::has_current_markdown(&c)
    );
    let snapshot = crate::db::agent_result_store::source_snapshot(&pool, 1)
        .await
        .unwrap();
    assert!(snapshot.markdown.contains("Publish content"));
    let translation = crate::agent::translation_segments::load_document(&pool, 1, "zh", None, 3)
        .await
        .unwrap();
    assert_eq!(translation.source_hash, snapshot.hash);
    assert!(translation
        .source_segments
        .iter()
        .any(|s| s.source_text.contains("Publish content")));
    assert!(translation.html.contains("中文正文") && !translation.html.contains("<script>"));
    let preview = crate::digest::templates::generate_preview(
        &pool,
        &[1],
        "single_markdown",
        &crate::digest::templates::DigestOptions {
            include_body: true,
            include_note: false,
            include_summary: false,
        },
    )
    .await
    .unwrap();
    assert!(preview.missing.is_empty() && preview.content.contains("Publish content"));
    sql(&pool,"CREATE TRIGGER no_cache_write BEFORE INSERT ON content_html_cache BEGIN SELECT RAISE(FAIL,'unexpected write'); END;").await;
    assert_eq!(read(&pool).await.unwrap(), html);
    sql(
        &pool,
        "DROP TRIGGER no_cache_write; UPDATE content_html_cache SET reader_render_version=0;",
    )
    .await;
    assert_eq!(read(&pool).await.unwrap(), html);
    let after = crate::db::agent_result_store::source_snapshot(&pool, 1)
        .await
        .unwrap();
    assert_eq!(snapshot.hash, after.hash);
}

#[tokio::test]
async fn m4w_013_failed_markdown_pending_identity_retry_and_version_rebuild() {
    let source =
        shell("http://127.0.0.1:12345", "article.md").replace("http://127.0.0.1:12345", "ORIGIN");
    let (origin, task) = server(vec![
        (
            "/page".into(),
            200,
            "Content-Type: text/html\r\n".into(),
            source,
        ),
        (
            format!("/access/{UID}/article.md"),
            503,
            String::new(),
            "unavailable".into(),
        ),
        (
            format!("/access/{UID}/article.md"),
            200,
            "Content-Type: text/plain\r\n".into(),
            "# Restored\n\nComplete body.".into(),
        ),
        (
            format!("/access/{UID}/article.md"),
            200,
            "Content-Type: text/markdown\r\n".into(),
            "# Rebuilt\n\nNew body.".into(),
        ),
        (
            format!("/access/{UID}/article.md"),
            200,
            "Content-Type: text/plain\r\n".into(),
            "Recovered resolver identity.".into(),
        ),
    ])
    .await;
    let pool = pool(&format!("{origin}/page")).await;
    sql(&pool, "UPDATE entry SET summary='导读'").await;
    assert!(read(&pool)
        .await
        .unwrap()
        .contains(pipeline::SUMMARY_SOURCE_MARKER));
    let pending = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    assert_eq!(pending.pipeline_type, "obsidian");
    assert_eq!(pending.display_mode, "pending");
    assert!(pending.markdown.is_none() && pending.cleaned_html.is_none());
    assert!(
        pipeline::get_cached_reader_html(&pool, 1, "classic-light:sans-serif:16")
            .await
            .unwrap()
            .is_none()
    );
    assert!(read(&pool).await.unwrap().contains("Complete body"));
    sql(
        &pool,
        "UPDATE content SET markdown_version=0; UPDATE content_html_cache SET html='STALE'",
    )
    .await;
    assert!(read(&pool).await.unwrap().contains("New body"));
    sql(
        &pool,
        "UPDATE content SET resolved_intermediate_content=NULL,markdown_version=0",
    )
    .await;
    assert!(read(&pool)
        .await
        .unwrap()
        .contains("Recovered resolver identity"));
    assert_eq!(
        content_store::get_content(&pool, 1)
            .await
            .unwrap()
            .unwrap()
            .resolved_intermediate_content,
        Some(format!("{origin}/access/{UID}/article.md"))
    );
    task.await.unwrap();
}

#[tokio::test]
async fn m4w_013_html_empty_invalid_and_index_failure_do_not_poison_body() {
    for (headers, body) in [
        ("Content-Type: text/html\r\n", "<html>challenge</html>"),
        (
            "Content-Type: text/plain\r\n",
            "<!DOCTYPE html><html>shell</html>",
        ),
        ("", ""),
        (
            "Content-Type: application/json\r\n",
            "{\"error\":\"missing\"}",
        ),
        ("Content-Type: text/plain\r\n", "<script>alert(1)</script>"),
    ] {
        let (origin, task) = server(vec![(
            format!("/access/{UID}/a.md"),
            200,
            headers.into(),
            body.into(),
        )])
        .await;
        assert!(obsidian::build_markdown(
            &Client::new(),
            &url::Url::parse(&format!("{origin}/access/{UID}/a.md")).unwrap()
        )
        .await
        .is_err());
        task.await.unwrap();
    }
    let (origin, task) = server(vec![
        (
            format!("/access/{UID}/a.md"),
            200,
            String::new(),
            "Text\n\n![[missing.png]]".into(),
        ),
        (
            format!("/cache/{UID}"),
            503,
            String::new(),
            "failure".into(),
        ),
    ])
    .await;
    let md = obsidian::build_markdown(
        &Client::new(),
        &url::Url::parse(&format!("{origin}/access/{UID}/a.md")).unwrap(),
    )
    .await
    .unwrap();
    assert!(super::md_to_html::render_markdown(&md, "classic-light").contains("![[missing.png]]"));
    task.await.unwrap();
}

#[tokio::test]
async fn m4w_013_old_default_shell_cache_is_re_resolved_and_feed_bypasses_it() {
    let (origin, task) = server(vec![(
        format!("/access/{UID}/page.md"),
        200,
        "Content-Type: text/plain\r\n".into(),
        "Recovered old shell body".into(),
    )])
    .await;
    let pool = pool(&format!("{origin}/page")).await;
    let source = shell(&origin, "page.md");
    let source_clone = source.clone();
    pool.get().await.unwrap().interact(move |c| {
        c.execute("INSERT INTO content(entry_id,html,cleaned_html,readability_version,markdown,markdown_version,document_base_url) VALUES(1,?1,'Old shell',2,'Old Markdown',2,?2)", rusqlite::params![source_clone,format!("{origin}/page")]).unwrap();
        c.execute("INSERT INTO content_html_cache(theme_id,entry_id,html,reader_render_version,updated_at) VALUES('classic-light:sans-serif:16',1,'Old successful shell',4,datetime('now'))",[]).unwrap();
    }).await.unwrap();
    assert!(read(&pool)
        .await
        .unwrap()
        .contains("Recovered old shell body"));
    task.await.unwrap();
    sql(&pool,"DELETE FROM content; DELETE FROM content_html_cache; UPDATE entry SET feed_content='<p>Explicit Feed body</p>',feed_content_type='text/html'").await;
    assert!(read(&pool).await.unwrap().contains("Explicit Feed body"));
    assert_eq!(
        content_store::get_content(&pool, 1)
            .await
            .unwrap()
            .unwrap()
            .pipeline_type,
        "default"
    );
}

#[tokio::test]
async fn m4w_013_redirect_rejects_external_vault_change_and_invalid_utf8() {
    let forbidden = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let forbidden_port = forbidden.local_addr().unwrap().port();
    for target in [
        format!("http://localhost:{forbidden_port}/access/{UID}/a.md"),
        format!("/access/other/a.md"),
    ] {
        let (origin, task) = server(vec![(
            format!("/access/{UID}/a.md"),
            302,
            format!("Location: {target}\r\n"),
            String::new(),
        )])
        .await;
        assert!(obsidian::build_markdown(
            &Client::new(),
            &url::Url::parse(&format!("{origin}/access/{UID}/a.md")).unwrap()
        )
        .await
        .unwrap_err()
        .contains("HTTP error"));
        task.await.unwrap();
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), forbidden.accept())
            .await
            .is_err()
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "http://{}/access/{UID}/a.md",
        listener.local_addr().unwrap()
    );
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        socket.read(&mut request).await.unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nConnection: close\r\n\r\n\xff")
            .await
            .unwrap();
    });
    assert!(
        obsidian::build_markdown(&Client::new(), &url::Url::parse(&url).unwrap())
            .await
            .unwrap_err()
            .contains("UTF-8")
    );
    task.await.unwrap();
}

#[tokio::test]
async fn m4w_013_drop_aborts_request_without_success_and_http_timeout_is_retryable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let source = shell(&origin, "a.md");
    let pool = pool(&format!("{origin}/page")).await;
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (closed_tx, closed_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut shell_socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        shell_socket.read(&mut request).await.unwrap();
        shell_socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{source}",source.len()).as_bytes()).await.unwrap();
        let (mut md_socket, _) = listener.accept().await.unwrap();
        md_socket.read(&mut request).await.unwrap();
        started_tx.send(()).unwrap();
        let closed = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            md_socket.read(&mut request),
        )
        .await
        .unwrap()
        .unwrap();
        closed_tx.send(closed).unwrap();
        let (mut retry_socket, _) = listener.accept().await.unwrap();
        retry_socket.read(&mut request).await.unwrap();
        // The production per-request deadline is 15 seconds, including reading.
        let closed = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            retry_socket.read(&mut request),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(closed, 0);
    });
    let reading_pool = pool.clone();
    let reading = tokio::spawn(async move { read(&reading_pool).await });
    started_rx.await.unwrap();
    reading.abort();
    assert!(reading.await.unwrap_err().is_cancelled());
    assert_eq!(closed_rx.await.unwrap(), 0);
    let pending = content_store::get_content(&pool, 1).await.unwrap().unwrap();
    assert_eq!(pending.display_mode, "pending");
    assert!(pending.markdown.is_none());
    assert!(
        pipeline::get_cached_reader_html(&pool, 1, "classic-light:sans-serif:16")
            .await
            .unwrap()
            .is_none()
    );
    assert!(read(&pool).await.unwrap_err().contains("HTTP error"));
    task.await.unwrap();
    assert!(content_store::get_content(&pool, 1)
        .await
        .unwrap()
        .unwrap()
        .markdown
        .is_none());
}
