// mercury4win/src-tauri/reader/md_to_html.rs
// Markdown to Reader HTML via comrak (GFM-compatible renderer)

use crate::reader::theme;
use crate::reader::theme::FontConfig;
use comrak::{markdown_to_html_with_plugins, ComrakOptions, ComrakPlugins};

/// Render Markdown to reader-ready HTML with theme CSS (default font).
pub fn render_markdown(md: &str, theme_id: &str) -> String {
    render_markdown_with_font(md, theme_id, &FontConfig::default())
}

/// Render Markdown to reader-ready HTML with theme CSS and custom font settings.
pub fn render_markdown_with_font(md: &str, theme_id: &str, font_config: &FontConfig) -> String {
    let body_html = markdown_to_html(md);
    theme::wrap_with_theme_and_font(&body_html, theme_id, font_config)
}

/// Convert Markdown to HTML body content (no wrapper).
fn markdown_to_html(md: &str) -> String {
    let mut options = ComrakOptions::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.tagfilter = true;
    options.extension.header_ids = Some(String::new());

    let plugins = ComrakPlugins::default();
    markdown_to_html_with_plugins(md, &options, &plugins)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_paragraph() {
        let md = "Hello world";
        let html = markdown_to_html(md);
        assert!(html.contains("<p>Hello world</p>"));
    }

    #[test]
    fn test_bold() {
        let md = "**bold text**";
        let html = markdown_to_html(md);
        assert!(html.contains("<strong>bold text</strong>"));
    }

    #[test]
    fn test_strikethrough() {
        let md = "~~deleted~~";
        let html = markdown_to_html(md);
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn test_gfm_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let html = markdown_to_html(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>A</th>") || html.contains(">A<"));
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let html = markdown_to_html(md);
        assert!(html.contains("<code") || html.contains("<pre"));
    }

    #[test]
    fn test_link() {
        let md = "[example](https://example.com)";
        let html = markdown_to_html(md);
        assert!(html.contains("example.com"));
    }

    #[test]
    fn test_render_with_theme() {
        let md = "# Title\n\nContent here.";
        let html = render_markdown(md, "classic-light");
        assert!(html.contains("<article class=\"reader\""));
        let heading = html
            .split_once("<h1>")
            .unwrap()
            .1
            .split_once("</h1>")
            .unwrap()
            .0;
        assert!(
            heading.ends_with("Title"),
            "heading text must remain visible"
        );
        assert!(heading.contains("href=\"#title\""));
        assert!(heading.contains("id=\"title\""));
        assert!(heading.contains("aria-hidden=\"true\""));
        assert!(html.contains("<p>Content here.</p>"));
    }
}
