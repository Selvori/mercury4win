// mercury4win-linux/src-tauri/feed/parser.rs
// feed-rs wrapper for RSS/Atom/JSON Feed parsing

use feed_rs::parser;
use url::Url;
use crate::db::models::Feed;
use crate::db::entry_store::RawEntryData;

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
            // Some RSS feeds (including antirez.com) put the useful article
            // body in content:encoded and omit RSS description. Preserve
            // either field so the reader can render the feed item directly.
            let summary = e
                .content
                .as_ref()
                .and_then(|c| c.body.clone())
                .or_else(|| e.summary.as_ref().map(|s| s.content.clone()));
            let published_at = e.published.or(e.updated).map(|d| d.to_rfc3339());
            let author = e.authors.first().map(|a| a.name.clone());

            RawEntryData {
                guid,
                url,
                title,
                author,
                published_at,
                summary,
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
