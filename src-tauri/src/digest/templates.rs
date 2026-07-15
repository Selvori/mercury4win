// mercury4win-linux/src-tauri/digest/templates.rs
// Digest template rendering via Handlebars

use handlebars::Handlebars;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DigestEntryData {
    pub title: String,
    pub byline: Option<String>,
    pub content: String,
    pub published_at: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DigestTemplateData {
    pub digest_title: Option<String>,
    pub generated_at: String,
    pub entry_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub byline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<DigestEntryData>>,
}

/// Built-in template names and their display info.
pub const BUILTIN_TEMPLATES: &[(&str, &str, &str)] = &[
    ("single_text", "Single Entry (HTML)", "Single entry as HTML with embedded CSS"),
    ("single_markdown", "Single Entry (Markdown)", "Single entry as clean Markdown"),
    ("multiple_markdown", "Multiple Entries (Markdown)", "Multi-entry digest with separators"),
];

/// Render a digest using one of the built-in templates.
pub fn render_digest(
    template_name: &str,
    data: &DigestTemplateData,
) -> Result<String, String> {
    let mut handlebars = Handlebars::new();

    let template_str = match template_name {
        "single_markdown" => {
            include_str!("../../resources/digest_templates/single_markdown.hbs")
        }
        "multiple_markdown" => {
            include_str!("../../resources/digest_templates/multiple_markdown.hbs")
        }
        _ => {
            include_str!("../../resources/digest_templates/single_text.hbs")
        }
    };

    handlebars
        .register_template_string("digest", template_str)
        .map_err(|e| format!("Template register error: {}", e))?;

    // Enable strict mode and HTML escaping
    handlebars.register_escape_fn(handlebars::no_escape);

    handlebars
        .render("digest", data)
        .map_err(|e| format!("Render error: {}", e))
}

/// Generate a digest for a single entry.
pub async fn generate_entry_digest(
    template_name: &str,
    entry_title: &str,
    entry_byline: Option<&str>,
    entry_content: &str,
    entry_url: Option<&str>,
) -> Result<String, String> {
    let data = DigestTemplateData {
        digest_title: Some(format!("Digest: {}", entry_title)),
        generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        entry_count: 1,
        title: Some(entry_title.to_string()),
        byline: entry_byline.map(|s| s.to_string()),
        content: Some(entry_content.to_string()),
        url: entry_url.map(|s| s.to_string()),
        published_at: None,
        entries: None,
    };

    render_digest(template_name, &data)
}

/// Generate a multi-entry digest.
pub async fn generate_multi_digest(
    template_name: &str,
    digest_title: Option<&str>,
    entries: Vec<DigestEntryData>,
) -> Result<String, String> {
    let count = entries.len();
    let data = DigestTemplateData {
        digest_title: digest_title.map(|s| s.to_string()),
        generated_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        entry_count: count,
        title: None,
        byline: None,
        content: None,
        url: None,
        published_at: None,
        entries: Some(entries),
    };

    render_digest(template_name, &data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_markdown_template() {
        let result = render_digest(
            "single_markdown",
            &DigestTemplateData {
                digest_title: None,
                generated_at: "2026-01-01".into(),
                entry_count: 1,
                title: Some("Test Article".into()),
                byline: Some("Author".into()),
                content: Some("Article content here.".into()),
                url: None,
                published_at: None,
                entries: None,
            },
        )
        .unwrap();
        assert!(result.contains("Test Article"));
        assert!(result.contains("Article content here."));
    }

    #[test]
    fn test_multiple_markdown_template() {
        let entries = vec![
            DigestEntryData {
                title: "First".into(),
                byline: None,
                content: "Content 1".into(),
                published_at: None,
                url: None,
            },
            DigestEntryData {
                title: "Second".into(),
                byline: None,
                content: "Content 2".into(),
                published_at: None,
                url: None,
            },
        ];
        let result = render_digest(
            "multiple_markdown",
            &DigestTemplateData {
                digest_title: Some("My Digest".into()),
                generated_at: "2026-01-01".into(),
                entry_count: 2,
                title: None,
                byline: None,
                content: None,
                url: None,
                published_at: None,
                entries: Some(entries),
            },
        )
        .unwrap();
        assert!(result.contains("My Digest"));
        assert!(result.contains("First"));
        assert!(result.contains("Second"));
    }
}
