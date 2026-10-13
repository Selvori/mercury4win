// mercury4win/src-tauri/reader/pipeline.rs
// Reader build pipeline with layered rebuild and version caching

use crate::db::models::Content;
use crate::db::{content_store, entry_store};
use crate::reader::theme::FontConfig;
use crate::reader::{extraction, html_to_md, md_to_html, obsidian};
use deadpool_sqlite::Pool;
use reqwest::Client;

/// Pipeline version constants.
pub const READABILITY_VERSION: i32 = 3;
pub const MARKDOWN_VERSION: i32 = 2;
pub const READER_RENDER_VERSION: i32 = 5;

/// A summary fallback is display-only and never a successful body/cache entry.
pub const SUMMARY_SOURCE_MARKER: &str =
    "<meta name=\"mercury-content-source\" content=\"summary\">";

/// Both pipeline types own canonical Markdown, but only default owns Readability.
pub fn has_current_markdown(c: &Content) -> bool {
    has_text(c.markdown.as_deref())
        && c.markdown_version == Some(MARKDOWN_VERSION)
        && if c.pipeline_type == "obsidian" {
            c.resolved_intermediate_content
                .as_deref()
                .and_then(obsidian::markdown_url)
                .is_some()
        } else {
            c.pipeline_type == "default" && c.readability_version == Some(READABILITY_VERSION)
        }
}

/// Build reader content for an entry. Implements the layered rebuild strategy.
pub async fn build_reader_content(
    pool: &Pool,
    client: &Client,
    entry_id: i64,
    theme_id: &str,
    font_config: &FontConfig,
) -> Result<String, String> {
    // Cache key includes theme + font family + font size to avoid serving wrong font.
    let cache_key = format!("{}:{}:{}", theme_id, font_config.family, font_config.size);

    let detail = entry_store::get_entry_detail(pool, entry_id)
        .await?
        .ok_or("Entry not found")?;
    let entry = &detail.entry;
    let content = content_store::get_content(pool, entry_id).await?;

    let result = async {
        if let Some(c) = content.as_ref() {
            if c.pipeline_type == "obsidian" {
                if has_current_markdown(c) {
                    if let Some(html) = get_cached_reader_html(pool, entry_id, &cache_key).await? {
                        return Ok(html);
                    }
                    let html = md_to_html::render_markdown_with_font(
                        c.markdown.as_deref().unwrap(),
                        theme_id,
                        font_config,
                    );
                    save_rendered_cache(pool, entry_id, &cache_key, &html).await?;
                    return Ok(html);
                }
                let url = c
                    .resolved_intermediate_content
                    .as_deref()
                    .and_then(obsidian::markdown_url)
                    .or_else(|| {
                        c.html.as_deref().and_then(|h| {
                            obsidian::resolve(
                                h,
                                c.document_base_url
                                    .as_deref()
                                    .or(entry.url.as_deref())
                                    .unwrap_or(""),
                            )
                        })
                    })
                    .ok_or("Obsidian shell has no static Markdown URL")?;
                return run_obsidian_pipeline(
                    pool,
                    client,
                    c.clone(),
                    &url,
                    theme_id,
                    font_config,
                    &cache_key,
                )
                .await;
            }
            let readability_current = c.readability_version == Some(READABILITY_VERSION);
            // Only stable absence of a source is terminal. Old empty rows may
            // represent network/extraction failures and are deliberately retried.
            if c.display_mode == "empty"
                && readability_current
                && c.markdown_version == Some(MARKDOWN_VERSION)
            {
                return Err("No article content is available.".to_string());
            }
            if readability_current
                && has_text(c.markdown.as_deref())
                && c.markdown_version == Some(MARKDOWN_VERSION)
            {
                if let Some(html) = get_cached_reader_html(pool, entry_id, &cache_key).await? {
                    return Ok(html);
                }
                let html = md_to_html::render_markdown_with_font(
                    c.markdown.as_deref().unwrap(),
                    theme_id,
                    font_config,
                );
                save_rendered_cache(pool, entry_id, &cache_key, &html).await?;
                return Ok(html);
            }
            if readability_current && has_text(c.cleaned_html.as_deref()) {
                let mut rebuilt = c.clone();
                rebuilt.markdown = Some(html_to_md::convert_html_to_markdown_with_base(
                    c.cleaned_html.as_deref().unwrap(),
                    c.document_base_url.as_deref().or(entry.url.as_deref()),
                )?);
                if !has_text(rebuilt.markdown.as_deref()) {
                    return Err("Article conversion produced no content".into());
                }
                rebuilt.markdown_version = Some(MARKDOWN_VERSION);
                content_store::upsert_content(pool, &rebuilt).await?;
                let html = md_to_html::render_markdown_with_font(
                    rebuilt.markdown.as_deref().unwrap(),
                    theme_id,
                    font_config,
                );
                save_rendered_cache(pool, entry_id, &cache_key, &html).await?;
                return Ok(html);
            }
            if has_text(c.html.as_deref()) {
                if entry.feed_content.is_none() {
                    if let Some(url) = obsidian::resolve(
                        c.html.as_deref().unwrap(),
                        c.document_base_url
                            .as_deref()
                            .or(entry.url.as_deref())
                            .unwrap_or(""),
                    ) {
                        let mut resolved = c.clone();
                        resolved.pipeline_type = "obsidian".into();
                        resolved.resolved_intermediate_content = Some(url.to_string());
                        resolved.cleaned_html = None;
                        resolved.markdown = None;
                        resolved.markdown_version = None;
                        resolved.readability_version = None;
                        resolved.readability_title = None;
                        resolved.readability_byline = None;
                        resolved.display_mode = "pending".into();
                        content_store::upsert_content(pool, &resolved).await?;
                        return run_obsidian_pipeline(
                            pool,
                            client,
                            resolved,
                            &url,
                            theme_id,
                            font_config,
                            &cache_key,
                        )
                        .await;
                    }
                }
                return run_full_pipeline(
                    pool,
                    entry_id,
                    c.html.as_deref().unwrap(),
                    theme_id,
                    font_config,
                    &cache_key,
                    entry.feed_content.is_some(),
                    c.document_base_url.as_deref().or(entry.url.as_deref()),
                    // Version 1 (M4W-005) stored only the entry URL. Other
                    // persisted bases already include the document's <base>.
                    c.document_base_url.is_some() && c.readability_version != Some(1),
                )
                .await;
            }
        }
        // Only an explicit syndication body can bypass the article fetch.
        // Plain-text Feed bodies must not be interpreted as HTML.
        let explicit_body = entry
            .feed_content
            .as_deref()
            .filter(|b| !b.trim().is_empty());
        if let Some(body) = explicit_body {
            let html = if entry.feed_content_type.as_deref() == Some("text/plain") {
                format!("<pre><code>{}</code></pre>", html_escape::encode_text(body))
            } else {
                body.to_string()
            };
            run_full_pipeline(
                pool,
                entry_id,
                &html,
                theme_id,
                font_config,
                &cache_key,
                true,
                entry.url.as_deref(),
                false,
            )
            .await
        } else if let Some(url) = entry.url.as_deref().filter(|s| !s.trim().is_empty()) {
            match fetch_article_html(client, url).await {
                Ok((source, final_url)) => {
                    if let Some(url) = obsidian::resolve(&source, &final_url) {
                        let resolved = Content {
                            entry_id,
                            html: Some(source),
                            cleaned_html: None,
                            readability_title: None,
                            readability_byline: None,
                            readability_version: None,
                            markdown: None,
                            markdown_version: None,
                            display_mode: "pending".into(),
                            document_base_url: Some(final_url),
                            pipeline_type: "obsidian".into(),
                            resolved_intermediate_content: Some(url.to_string()),
                        };
                        // Persist resolver identity only. Failed fetches are not successful bodies.
                        content_store::upsert_content(pool, &resolved).await?;
                        return run_obsidian_pipeline(
                            pool,
                            client,
                            resolved,
                            &url,
                            theme_id,
                            font_config,
                            &cache_key,
                        )
                        .await;
                    }
                    run_full_pipeline(
                        pool,
                        entry_id,
                        &source,
                        theme_id,
                        font_config,
                        &cache_key,
                        false,
                        Some(&final_url),
                        false,
                    )
                    .await
                }
                Err(error) => Err(error),
            }
        } else {
            if !has_text(entry.summary.as_deref()) {
                let empty = Content {
                    entry_id,
                    html: None,
                    cleaned_html: None,
                    readability_title: None,
                    readability_byline: None,
                    readability_version: Some(READABILITY_VERSION),
                    markdown: None,
                    markdown_version: Some(MARKDOWN_VERSION),
                    display_mode: "empty".into(),
                    document_base_url: None,
                    pipeline_type: "default".into(),
                    resolved_intermediate_content: None,
                };
                content_store::upsert_content(pool, &empty).await?;
            }
            Err("No article URL or explicit Feed body".into())
        }
    }
    .await;
    match result {
        Ok(html) => Ok(html),
        Err(error) => {
            if let Some(summary) = entry.summary.as_deref().filter(|s| !s.trim().is_empty()) {
                // Keep legacy merged data readable offline without claiming
                // its unknown origin is a full article. Never persist this
                // fallback, so a later open/retry can fetch the full body.
                Ok(render_summary_fallback(summary, theme_id, font_config))
            } else {
                Err(error)
            }
        }
    }
}

async fn run_obsidian_pipeline(
    pool: &Pool,
    client: &Client,
    mut content: Content,
    url: &url::Url,
    theme_id: &str,
    font_config: &FontConfig,
    cache_key: &str,
) -> Result<String, String> {
    if content.resolved_intermediate_content.as_deref() != Some(url.as_str()) {
        content.pipeline_type = "obsidian".into();
        content.resolved_intermediate_content = Some(url.to_string());
        content.markdown = None;
        content.markdown_version = None;
        content.cleaned_html = None;
        content.readability_version = None;
        content.readability_title = None;
        content.readability_byline = None;
        content.display_mode = "pending".into();
        content_store::upsert_content(pool, &content).await?;
    }
    let md = obsidian::build_markdown(client, url).await?;
    if md.trim().is_empty() {
        return Err("Obsidian conversion produced no content".into());
    }
    content.markdown = Some(md);
    content.markdown_version = Some(MARKDOWN_VERSION);
    content.display_mode = "cleaned".into();
    content.cleaned_html = None;
    content.readability_version = None;
    content.readability_title = None;
    content.readability_byline = None;
    content_store::upsert_content(pool, &content).await?;
    let html = md_to_html::render_markdown_with_font(
        content.markdown.as_deref().unwrap(),
        theme_id,
        font_config,
    );
    save_rendered_cache(pool, content.entry_id, cache_key, &html).await?;
    Ok(html)
}

fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|s| !s.trim().is_empty())
}

fn render_summary_fallback(summary: &str, theme_id: &str, font_config: &FontConfig) -> String {
    let text = strip_html_tags(summary);
    let text = html_escape::decode_html_entities(&text);
    let body = format!(
        "<pre><code>{}</code></pre>",
        html_escape::encode_text(&text)
    );
    // Use the same converter as successful bodies; comrak deliberately omits
    // raw HTML, so passing the <pre> wrapper directly would hide the snippet.
    let markdown = html_to_md::convert_html_to_markdown(&body).unwrap_or_else(|_| text.to_string());
    md_to_html::render_markdown_with_font(&markdown, theme_id, font_config).replacen(
        "</head>",
        &format!("{SUMMARY_SOURCE_MARKER}</head>"),
        1,
    )
}

/// Fetch article HTML from URL with browser-like User-Agent.
/// Reject failed/challenge responses instead of persisting them as an article.
async fn fetch_article_html(client: &Client, url: &str) -> Result<(String, String), String> {
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
        return Err(format!("Article HTTP error: {}", response.status()));
    }

    let final_url = response.url().to_string();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    // Reject Cloudflare challenge pages
    if body.contains("_cf_chl_opt") || body.contains("challenge-platform") {
        return Err("Article access challenge".into());
    }

    if body.trim().is_empty() {
        return Err("Empty article response".into());
    }
    Ok((body, final_url))
}

async fn run_full_pipeline(
    pool: &Pool,
    entry_id: i64,
    source_html: &str,
    theme_id: &str,
    font_config: &FontConfig,
    cache_key: &str,
    from_feed_content: bool,
    document_base_url: Option<&str>,
    base_already_resolved: bool,
) -> Result<String, String> {
    // Persisted resolved bases must not be joined to a relative <base> again
    // when rebuilding locally after a version bump.
    let base = if base_already_resolved {
        document_base_url
            .and_then(html_to_md::web_url)
            .map(|u| u.to_string())
    } else {
        html_to_md::document_base_url(source_html, document_base_url)
    };
    // Feed bodies are already article fragments; extraction would discard
    // legitimate short posts. Web pages still require meaningful extraction.
    let decruft_output = extraction::extract_content(source_html, base.as_deref())?;
    if !from_feed_content && decruft_output.needs_fallback() {
        return Err("Article extraction failed".into());
    }
    let cleaned = if from_feed_content {
        source_html.to_string()
    } else {
        decruft_output.content
    };

    // HTML → Markdown
    let md = html_to_md::convert_html_to_markdown_with_base(&cleaned, base.as_deref())?;
    if md.trim().is_empty() {
        return Err("Article conversion produced no content".into());
    }

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
        document_base_url: base,
        pipeline_type: "default".to_string(),
        resolved_intermediate_content: None,
    };
    content_store::upsert_content(pool, &content).await?;

    // Markdown → Reader HTML
    let html = md_to_html::render_markdown_with_font(
        content.markdown.as_ref().unwrap(),
        theme_id,
        font_config,
    );

    // Cache rendered HTML
    save_rendered_cache(pool, entry_id, cache_key, &html).await?;

    Ok(html)
}

async fn save_rendered_cache(
    pool: &Pool,
    entry_id: i64,
    cache_key: &str,
    html: &str,
) -> Result<(), String> {
    let cache_key = cache_key.to_string();
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
                rusqlite::params![cache_key, entry_id, html, READER_RENDER_VERSION],
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
    cache_key: &str,
) -> Result<Option<String>, String> {
    let entry_id_val = entry_id;
    let theme_id_val = cache_key.to_string();
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
