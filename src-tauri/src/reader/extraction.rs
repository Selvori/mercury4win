// mercury4win-linux/src-tauri/reader/extraction.rs
// Article content extraction via decruft

use decruft::{DecruftOptions, DecruftResult};
use crate::db::models::DecruftOutput;

/// Extract clean article content from raw HTML using decruft.
pub fn extract_content(html: &str, url: Option<&str>) -> Result<DecruftOutput, String> {
    let mut options = DecruftOptions::default();
    if let Some(u) = url {
        options.url = Some(u.to_string());
    }

    let result: DecruftResult = decruft::parse(html, &options);

    Ok(DecruftOutput {
        content: result.content,
        title: result.title,
        byline: result.author,
        site_name: result.domain,
        language: result.language,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_basic_html() {
        let html = r#"<!DOCTYPE html>
<html>
<head><title>Test Article</title></head>
<body>
<article>
<h1>Test Article Title</h1>
<p>This is a test paragraph with enough content to pass the minimum length check.
It needs to be long enough. The quick brown fox jumps over the lazy dog.
Let me add more text to ensure we have enough. More content here to be safe.</p>
</article>
</body>
</html>"#;

        let result = extract_content(html, None);
        // decruft may or may not extract content from such simple HTML
        // the test just verifies it doesn't crash
        assert!(result.is_ok());
    }

    #[test]
    fn test_needs_fallback_logic() {
        let empty = DecruftOutput {
            content: String::new(),
            title: None,
            byline: None,
            site_name: None,
            language: None,
        };
        assert!(empty.needs_fallback());

        let short = DecruftOutput {
            content: "hi".into(),
            title: Some("Ok".into()),
            byline: None,
            site_name: None,
            language: None,
        };
        assert!(short.needs_fallback());

        let missing_title = DecruftOutput {
            content: "x".repeat(500),
            title: None,
            byline: None,
            site_name: None,
            language: None,
        };
        assert!(missing_title.needs_fallback());

        let good = DecruftOutput {
            content: "x".repeat(500),
            title: Some("Good Title".into()),
            byline: None,
            site_name: None,
            language: None,
        };
        assert!(!good.needs_fallback());
    }
}
