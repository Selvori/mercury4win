// mercury4win-linux/src-tauri/feed/parser.rs
// feed-rs wrapper for RSS/Atom/JSON Feed parsing

use feed_rs::parser;
use crate::db::models::Feed;
use crate::db::entry_store::RawEntryData;

/// Parsed feed with metadata and entries
pub struct ParsedFeed {
    pub title: Option<String>,
    pub site_url: Option<String>,
    pub entries: Vec<RawEntryData>,
}

/// Parse feed content and extract entries.
pub fn parse_feed_bytes(bytes: &[u8], _feed_url: Option<&str>) -> Result<ParsedFeed, String> {
    let feed = parser::parse(bytes).map_err(|e| format!("Feed parse error: {}", e))?;

    let title = feed.title.map(|t| t.content);
    let site_url = feed
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("alternate"))
        .map(|l| l.href.clone());

    let entries: Vec<RawEntryData> = feed
        .entries
        .iter()
        .map(|e| {
            let guid = e.id.clone();
            let url = e
                .links
                .first()
                .map(|l| l.href.clone());

            let title = e.title.as_ref().map(|t| t.content.clone());
            let summary = e.summary.as_ref().map(|s| s.content.clone());
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
