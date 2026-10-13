// mercury4win/src-tauri/reader/html_to_md.rs
// HTML to Markdown conversion via htmd (Turndown.js Rust port)

use markup5ever_rcdom::NodeData;
use std::rc::Rc;
use url::Url;

/// Only web URLs may supply the base of an untrusted article document.
pub fn web_url(value: &str) -> Option<Url> {
    Url::parse(value)
        .ok()
        .filter(|u| matches!(u.scheme(), "http" | "https"))
}

pub fn document_base_url(html: &str, response_url: Option<&str>) -> Option<String> {
    let response = response_url.and_then(web_url);
    let tree = htmd::HtmlToMarkdown::new().html_to_tree(html).ok()?;
    fn first_base(node: &Rc<htmd::Node>) -> Option<String> {
        if let NodeData::Element { name, attrs, .. } = &node.data {
            if name.local.as_ref() == "base" {
                if let Some(href) = attrs
                    .borrow()
                    .iter()
                    .find(|a| a.name.local.as_ref() == "href")
                {
                    return Some(href.value.to_string());
                }
            }
        }
        node.children.borrow().iter().find_map(first_base)
    }
    first_base(&tree)
        .and_then(|href| {
            response
                .as_ref()
                .and_then(|u| u.join(href.trim()).ok())
                .or_else(|| web_url(href.trim()))
        })
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .or(response)
        .map(|u| u.to_string())
}

/// Normalize URL-bearing DOM attributes before conversion, including Feed
/// fragments that intentionally bypass Readability. Raw HTML remains disabled
/// in the downstream renderer; no source base element reaches the iframe.
pub fn convert_html_to_markdown_with_base(
    html: &str,
    base: Option<&str>,
) -> Result<String, String> {
    let converter = htmd::HtmlToMarkdown::new();
    let tree = converter
        .html_to_tree(html)
        .map_err(|e| format!("htmd conversion failed: {e}"))?;
    let base = base.and_then(web_url);
    fn normalize(node: &Rc<htmd::Node>, base: Option<&Url>) {
        if let NodeData::Element { name, attrs, .. } = &node.data {
            let tag = name.local.as_ref();
            if matches!(tag, "a" | "img") {
                attrs.borrow_mut().retain_mut(|attr| {
                    let attribute = attr.name.local.as_ref();
                    if !((tag == "a" && attribute == "href")
                        || (tag == "img" && attribute == "src"))
                    {
                        return true;
                    }
                    let raw = attr.value.trim();
                    if tag == "a" && raw.starts_with('#') {
                        return true;
                    }
                    let url = Url::parse(raw)
                        .ok()
                        .or_else(|| base.and_then(|u| u.join(raw).ok()));
                    match url.filter(|u| {
                        matches!(u.scheme(), "http" | "https")
                            || (tag == "a" && u.scheme() == "mailto")
                    }) {
                        Some(url) => {
                            attr.value = url.as_str().into();
                            true
                        }
                        None => false,
                    }
                });
            }
        }
        for child in node.children.borrow().iter() {
            normalize(child, base);
        }
    }
    normalize(&tree, base.as_ref());
    Ok(converter.tree_to_markdown(&tree))
}

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
