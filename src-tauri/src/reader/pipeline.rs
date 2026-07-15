// mercury4win-linux/src-tauri/reader/pipeline.rs
// Reader build pipeline with layered rebuild and version caching

use deadpool_sqlite::Pool;
use reqwest::Client;
use crate::db::{content_store, entry_store};
use crate::db::models::Content;
use crate::reader::{extraction, html_to_md, md_to_html, theme};

/// Pipeline version constants.
pub const READABILITY_VERSION: i32 = 1;
pub const MARKDOWN_VERSION: i32 = 1;
pub const READER_RENDER_VERSION: i32 = 1;

/// Build reader content for an entry. Implements the layered rebuild strategy.
pub async fn build_reader_content(
    pool: &Pool,
    client: &Client,
    entry_id: i64,
    theme_id: &str,
) -> Result<String, String> {
    let content = content_store::get_content(pool, entry_id).await?;

    match content {
        Some(ref c) if c.markdown.is_some() => {
            // Step 1: render from existing markdown
            let md = c.markdown.as_ref().unwrap();
            let html = md_to_html::render_markdown(md, theme_id);
            save_rendered_cache(pool, entry_id, theme_id, &html).await?;
            Ok(html)
        }
        Some(ref c) if c.cleaned_html.is_some() => {
            // Step 2: convert cleaned HTML to markdown then render
            let cleaned = c.cleaned_html.as_ref().unwrap();
            let md = html_to_md::convert_html_to_markdown(cleaned)?;
            let html = md_to_html::render_markdown(&md, theme_id);
            save_rendered_cache(pool, entry_id, theme_id, &html).await?;
            Ok(html)
        }
        Some(ref c) if c.html.is_some() => {
            // Step 3: source HTML stored but not yet processed
            run_full_pipeline(pool, entry_id, c.html.as_ref().unwrap(), theme_id).await
        }
        _ => {
            // Step 4: fetch fresh content — try article URL first, fall back to RSS summary
            let entry_detail = entry_store::get_entry_detail(pool, entry_id)
                .await?
                .ok_or("Entry not found")?;

            // Try fetching from article URL for full content with images
            let fetched = if let Some(ref url) = entry_detail.entry.url {
                fetch_article_html(client, url).await.ok()
            } else {
                None
            };

            // Use fetched HTML if available, otherwise fall back to RSS summary
            let source_html = fetched
                .or(entry_detail.entry.summary)
                .unwrap_or_default();

            if source_html.is_empty() {
                return Err("No content available for this entry".into());
            }

            run_full_pipeline(pool, entry_id, &source_html, theme_id).await
        }
    }
}

/// Fetch article HTML from URL with browser-like User-Agent.
/// Returns empty string on failure or Cloudflare challenge detection.
async fn fetch_article_html(client: &Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !response.status().is_success() {
        return Ok(String::new());
    }

    let body = response.text().await.map_err(|e| format!("Read error: {}", e))?;

    // Reject Cloudflare challenge pages
    if body.contains("_cf_chl_opt") || body.contains("challenge-platform") {
        return Ok(String::new());
    }

    Ok(body)
}

async fn run_full_pipeline(
    pool: &Pool,
    entry_id: i64,
    source_html: &str,
    theme_id: &str,
) -> Result<String, String> {
    // Try decruft extraction first
    let decruft_output = extraction::extract_content(source_html, None)?;

    let needs_fallback = decruft_output.needs_fallback();
    let is_rss_metadata = source_html.len() < 500 || source_html.contains("Article URL:");

    if needs_fallback || is_rss_metadata {
        // decruft could not extract meaningful article text.  Load the entry
        // detail so we can build a per-entry fallback page that shows the entry
        // metadata plus whatever plain-text we can scrape from the source.
        let detail = entry_store::get_entry_detail(pool, entry_id)
            .await?
            .ok_or("Entry not found")?;

        let title = detail.entry.title.unwrap_or_default();
        let author = detail.entry.author.unwrap_or_default();
        let published = detail.entry.published_at.unwrap_or_default();
        let url = detail.entry.url.unwrap_or_default();

        // Build a simple but per-entry HTML page so different entries don't
        // look identical.
        let mut body = String::new();
        body.push_str(&format!("<h1>{}</h1>\n", html_escape::encode_text(&title)));
        if !author.is_empty() {
            body.push_str(&format!("<p><em>{}</em></p>\n", html_escape::encode_text(&author)));
        }
        if !published.is_empty() {
            body.push_str(&format!("<p>{}</p>\n", html_escape::encode_text(&published)));
        }
        body.push_str("<hr>\n");
        body.push_str("<p><strong>Full article content is not available.</strong><br>\n");
        body.push_str("The source website may require JavaScript or block automated access (Cloudflare challenge).</p>\n");

        // Append any plain-text we can extract from the source HTML so that
        // there is *some* content visible (will be unique per entry if the RSS
        // description carries a snippet).
        let stripped = strip_html_tags(source_html);
        let trimmed = stripped.trim();
        if trimmed.len() > 20 {
            body.push_str("<hr>\n<blockquote>\n");
            body.push_str(&html_escape::encode_text(trimmed).replace("\n", "<br>\n"));
            body.push_str("\n</blockquote>\n");
        }

        if !url.is_empty() {
            body.push_str(&format!(
                "<p><a href=\"{}\">Open original article in browser</a></p>\n",
                html_escape::encode_text(&url)
            ));
        }

        let html = md_to_html::render_markdown(&body, theme_id);
        save_rendered_cache(pool, entry_id, theme_id, &html).await?;

        // Persist empty marker so we don't keep re-fetching
        let content = Content {
            entry_id,
            html: None,
            cleaned_html: None,
            readability_title: Some(title),
            readability_byline: if author.is_empty() {
                None
            } else {
                Some(author)
            },
            readability_version: Some(READABILITY_VERSION),
            markdown: None,
            markdown_version: None,
            display_mode: "empty".to_string(),
            document_base_url: None,
            pipeline_type: "default".to_string(),
            resolved_intermediate_content: None,
        };
        content_store::upsert_content(pool, &content).await?;
        return Ok(html);
    }

    // Use the better of: decruft output or source HTML
    let cleaned = decruft_output.content;

    // HTML → Markdown
    let md = html_to_md::convert_html_to_markdown(&cleaned)?;

    // Persist pipeline artifacts
    let content = Content {
        entry_id,
        html: Some(source_html.to_string()),
        cleaned_html: Some(cleaned),
        readability_title: decruft_output.title,
        readability_byline: decruft_output.byline,
        readability_version: Some(READABILITY_VERSION),
        markdown: Some(md),
        markdown_version: Some(MARKDOWN_VERSION),
        display_mode: "cleaned".to_string(),
        document_base_url: None,
        pipeline_type: "default".to_string(),
        resolved_intermediate_content: None,
    };
    content_store::upsert_content(pool, &content).await?;

    // Markdown → Reader HTML
    let html = md_to_html::render_markdown(
        content.markdown.as_ref().unwrap(),
        theme_id,
    );

    // Cache rendered HTML
    save_rendered_cache(pool, entry_id, theme_id, &html).await?;

    Ok(html)
}

async fn save_rendered_cache(
    pool: &Pool,
    entry_id: i64,
    theme_id: &str,
    html: &str,
) -> Result<(), String> {
    let theme_id = theme_id.to_string();
    let html = html.to_string();
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO content_html_cache
                 (theme_id, entry_id, html, reader_render_version, updated_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                rusqlite::params![theme_id, entry_id, html, READER_RENDER_VERSION],
            )
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Strips HTML tags, returning plain text content.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}
pub async fn get_cached_reader_html(
    pool: &Pool,
    entry_id: i64,
    theme_id: &str,
) -> Result<Option<String>, String> {
    let entry_id_val = entry_id;
    let theme_id_val = theme_id.to_string();
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let result = conn.query_row(
                "SELECT html FROM content_html_cache
                 WHERE entry_id = ?1 AND theme_id = ?2 AND reader_render_version = ?3
                 ORDER BY updated_at DESC LIMIT 1",
                rusqlite::params![entry_id_val, theme_id_val, READER_RENDER_VERSION],
                |row| row.get(0),
            );
            match result {
                Ok(v) => Ok(Some(v)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        })
        .await
        .map_err(|e| e.to_string())?
}
