// mercury4win-linux/src-tauri/feed/opml_import.rs
// OPML import pipeline

use opml::{OPML, Outline};
use std::path::Path;

pub struct OpmlImportEntry {
    pub title: String,
    pub feed_url: String,
    pub site_url: Option<String>,
}

/// Parse an OPML file and extract feed entries.
pub fn parse_opml_file(path: &Path) -> Result<Vec<OpmlImportEntry>, String> {
    let xml = std::fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;
    let opml = OPML::from_str(&xml).map_err(|e| format!("OPML parse error: {}", e))?;

    let mut entries = Vec::new();
    collect_feeds(&opml.body.outlines, &mut entries);
    Ok(entries)
}

fn collect_feeds(outlines: &[Outline], entries: &mut Vec<OpmlImportEntry>) {
    for outline in outlines {
        if let Some(ref feed_url) = outline.xml_url {
            if !feed_url.is_empty() {
                entries.push(OpmlImportEntry {
                    title: outline.text.clone(),
                    feed_url: feed_url.clone(),
                    site_url: outline.html_url.clone(),
                });
            }
        }
        // Recurse into nested outlines (folders)
        if !outline.outlines.is_empty() {
            collect_feeds(&outline.outlines, entries);
        }
    }
}
