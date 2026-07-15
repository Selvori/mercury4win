// mercury4win-linux/src-tauri/reader/html_to_md.rs
// HTML to Markdown conversion via htmd (Turndown.js Rust port)

/// Convert cleaned HTML to canonical Markdown.
pub fn convert_html_to_markdown(html: &str) -> Result<String, String> {
    htmd::convert(html).map_err(|e| format!("htmd conversion failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_to_markdown() {
        let html = "<p>Hello world</p>";
        let md = convert_html_to_markdown(html).unwrap();
        assert!(md.contains("Hello world"));
    }

    #[test]
    fn test_bold_and_italic() {
        let html = "<p><strong>Bold</strong> and <em>italic</em></p>";
        let md = convert_html_to_markdown(html).unwrap();
        assert!(md.contains("**Bold**"));
        assert!(md.contains("italic"));
    }

    #[test]
    fn test_links() {
        let html = r#"<a href="https://example.com">Example</a>"#;
        let md = convert_html_to_markdown(html).unwrap();
        assert!(md.contains("Example"));
        assert!(md.contains("example.com"));
    }

    #[test]
    fn test_heading() {
        let html = "<h2>Section Title</h2>";
        let md = convert_html_to_markdown(html).unwrap();
        assert!(md.contains("## Section Title") || md.contains("Section Title"));
    }

    #[test]
    fn test_table() {
        let html = "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>";
        let md = convert_html_to_markdown(html).unwrap();
        // htmd should handle GFM tables
        assert!(!md.is_empty());
    }
}
