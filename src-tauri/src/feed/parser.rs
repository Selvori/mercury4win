// mercury4win/src-tauri/feed/parser.rs
// feed-rs wrapper for RSS/Atom/JSON Feed parsing

use crate::db::entry_store::RawEntryData;
use feed_rs::parser;
use url::Url;

/// Parsed feed with metadata and entries
pub struct ParsedFeed {
    pub title: Option<String>,
    pub site_url: Option<String>,
    pub entries: Vec<RawEntryData>,
}

/// Parse feed content and extract entries.
pub fn parse_feed_bytes(bytes: &[u8], feed_url: Option<&str>) -> Result<ParsedFeed, String> {
    let feed = parser::parse(bytes).map_err(|e| format!("Feed parse error: {}", e))?;

    // Parse base URL for resolving relative links
    let base_url = feed_url.and_then(|u| Url::parse(u).ok());

    let title = feed.title.map(|t| t.content);
    let site_url = feed
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("alternate"))
        .map(|l| l.href.clone())
        .and_then(|href| resolve_url(&href, base_url.as_ref()));

    let entries: Vec<RawEntryData> = feed
        .entries
        .iter()
        .map(|e| {
            let guid = e.id.clone();
            let url = e
                .links
                .iter()
                .find(|l| l.rel.as_deref() == Some("alternate"))
                .or_else(|| e.links.first())
                .map(|l| l.href.clone())
                .and_then(|href| resolve_url(&href, base_url.as_ref()));

            let title = e.title.as_ref().map(|t| t.content.clone());
            // Preserve the syndication field's meaning. A description/summary
            // is a teaser even when it contains HTML or happens to be long.
            let summary = e.summary.as_ref().map(|s| s.content.clone());
            let content = e.content.as_ref().filter(|c| {
                matches!(
                    c.content_type.as_str(),
                    "text/html" | "application/xhtml+xml" | "text/plain"
                ) && c.body.as_ref().is_some_and(|b| !b.trim().is_empty())
            });
            let feed_content = content.and_then(|c| c.body.clone());
            let feed_content_type = content.map(|c| c.content_type.to_string());
            let published_at = e.published.or(e.updated).map(|d| d.to_rfc3339());
            let author = e.authors.first().map(|a| a.name.clone());

            RawEntryData {
                guid,
                url,
                title,
                author,
                published_at,
                summary,
                feed_content,
                feed_content_type,
            }
        })
        .collect();

    Ok(ParsedFeed {
        title,
        site_url,
        entries,
    })
}

/// Resolve a URL (possibly relative) against a base URL.
fn resolve_url(href: &str, base: Option<&Url>) -> Option<String> {
    // If href is already absolute, return it
    if let Ok(absolute) = Url::parse(href) {
        return Some(absolute.to_string());
    }

    // Try resolving as relative
    if let Some(base_url) = base {
        if let Ok(resolved) = base_url.join(href) {
            return Some(resolved.to_string());
        }
    }

    // Fallback: return original if all resolution fails
    Some(href.to_string())
}
