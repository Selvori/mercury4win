// mercury4win-linux/src-tauri/feed/opml_export.rs
// OPML export pipeline

use std::path::Path;

/// Generate OPML XML from feed data and write to file.
pub fn export_opml_file(
    path: &Path,
    feeds: &[OpmlFeedData],
    folders: &[OpmlFolderData],
) -> Result<(), String> {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>Mercury Feeds</title>
    <dateCreated>"#,
    );
    xml.push_str(&chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string());
    xml.push_str(
        r#"</dateCreated>
  </head>
  <body>
"#,
    );

    // Folderless feeds
    for feed in feeds {
        push_outline(&mut xml, "    ", feed);
    }

    // Folders with nested feeds
    for folder in folders {
        xml.push_str(&format!(
            "    <outline text=\"{}\" title=\"{}\">\n",
            escape_xml(&folder.name),
            escape_xml(&folder.name)
        ));
        for feed in &folder.feeds {
            push_outline(&mut xml, "      ", feed);
        }
        xml.push_str("    </outline>\n");
    }

    xml.push_str("  </body>\n</opml>\n");

    std::fs::write(path, xml).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}

fn push_outline(buf: &mut String, indent: &str, feed: &OpmlFeedData) {
    buf.push_str(&format!(
        "{}<outline text=\"{}\" title=\"{}\" type=\"rss\" xmlUrl=\"{}\" htmlUrl=\"{}\"/>\n",
        indent,
        escape_xml(&feed.title),
        escape_xml(&feed.title),
        escape_xml(&feed.feed_url),
        escape_xml(&feed.site_url.as_deref().unwrap_or(""))
    ));
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub struct OpmlFeedData {
    pub title: String,
    pub feed_url: String,
    pub site_url: Option<String>,
}

pub struct OpmlFolderData {
    pub name: String,
    pub feeds: Vec<OpmlFeedData>,
}
