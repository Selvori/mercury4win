//! Read-only digest composition: never fetch articles or generate AI results.
use crate::db::{agent_result_store, content_store, entry_store, note_store};
use deadpool_sqlite::Pool;
use handlebars::Handlebars;
use markup5ever_rcdom::NodeData;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DigestOptions {
    pub include_body: bool,
    pub include_note: bool,
    pub include_summary: bool,
    /// Omitted by older callers; their exports retain English headings.
    pub locale: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DigestMissing {
    pub entry_id: i64,
    pub title: String,
    pub field: String,
}

#[derive(Debug, Serialize)]
pub struct DigestPreview {
    pub content: String,
    pub format: String,
    pub missing: Vec<DigestMissing>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DigestEntryData {
    pub title: String,
    pub byline: String,
    pub url: String,
    pub content: Option<String>,
    pub note: Option<String>,
    pub summary: Option<String>,
    pub summary_attribution: Option<String>,
}

pub fn output_format(name: &str, count: usize) -> Result<&'static str, String> {
    match (name, count) {
        (_, 0) => Err("Select at least one entry".into()),
        ("single_markdown", 1) | ("multiple_markdown", _) => Ok("markdown"),
        ("single_text", 1) => Ok("text"),
        _ => Err("Unknown digest template or invalid entry count".into()),
    }
}

fn optional(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.trim().is_empty())
}

/// Metadata is one line and literal in Markdown; optional Markdown stays Markdown.
fn metadata(value: &str, markdown: bool) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if !markdown {
        return value;
    }
    let mut escaped = String::new();
    for ch in value.chars() {
        if "\\`*_{}[]()#+-.!|>".contains(ch) {
            escaped.push('\\');
        }
        match ch {
            '<' => escaped.push_str("&lt;"),
            '&' => escaped.push_str("&amp;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

/// Render Markdown safely, then extract DOM text, preserving paragraphs and URLs.
pub fn plain_text(markdown: &str) -> String {
    let html = comrak::markdown_to_html(markdown, &comrak::Options::default());
    fn text(node: &Rc<htmd::Node>) -> String {
        if let NodeData::Text { contents } = &node.data {
            return contents.borrow().to_string();
        }
        let tag = match &node.data {
            NodeData::Element { name, .. } => name.local.as_ref(),
            _ => "",
        };
        if matches!(tag, "script" | "style") {
            return String::new();
        }
        let mut body = node.children.borrow().iter().map(text).collect::<String>();
        if tag == "a" {
            if let NodeData::Element { attrs, .. } = &node.data {
                if let Some(href) = attrs
                    .borrow()
                    .iter()
                    .find(|a| a.name.local.as_ref() == "href")
                {
                    if body != href.value.as_ref() {
                        body.push_str(&format!(" ({})", href.value));
                    }
                }
            }
        }
        if matches!(tag, "p" | "li" | "br" | "pre" | "h1" | "h2" | "h3" | "tr") {
            body.push('\n');
        }
        body
    }
    htmd::HtmlToMarkdown::new()
        .html_to_tree(&html)
        .map(|tree| text(&tree).trim().to_string())
        .unwrap_or_default()
}

pub fn render_digest(
    name: &str,
    entries: Vec<DigestEntryData>,
    locale: Option<&str>,
) -> Result<String, String> {
    output_format(name, entries.len())?;
    let template = match name {
        "single_text" => include_str!("../../resources/digest_templates/single_text.hbs"),
        "single_markdown" => include_str!("../../resources/digest_templates/single_markdown.hbs"),
        "multiple_markdown" => {
            include_str!("../../resources/digest_templates/multiple_markdown.hbs")
        }
        _ => unreachable!(),
    };
    let mut engine = Handlebars::new();
    engine.register_escape_fn(handlebars::no_escape);
    engine.set_strict_mode(true);
    engine
        .register_template_string("digest", template)
        .map_err(|e| format!("Template error: {e}"))?;
    let labels = if locale == Some("zh-Hans") {
        serde_json::json!({"author":"作者", "source":"来源", "summary":"摘要", "note":"我的笔记", "article":"正文"})
    } else {
        serde_json::json!({"author":"Author", "source":"Source", "summary":"Summary", "note":"My Take", "article":"Article"})
    };
    let digest_title = if locale == Some("zh-Hans") {
        "Mercury 文摘"
    } else {
        "Mercury Digest"
    };
    engine
        .render(
            "digest",
            &serde_json::json!({"digest_title":digest_title, "labels": labels, "entries": entries}),
        )
        .map_err(|e| format!("Render error: {e}"))
}

pub async fn generate_preview(
    pool: &Pool,
    ids: &[i64],
    name: &str,
    options: &DigestOptions,
) -> Result<DigestPreview, String> {
    let format = output_format(name, ids.len())?;
    let markdown = format == "markdown";
    let mut entries = Vec::new();
    let mut missing = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &id in ids {
        if !seen.insert(id) {
            return Err("Duplicate digest entry".into());
        }
        let detail = entry_store::get_entry_detail(pool, id)
            .await?
            .ok_or_else(|| format!("Entry {id} not found"))?;
        let content = content_store::get_content(pool, id).await?;
        let title =
            optional(detail.entry.title).ok_or_else(|| format!("Entry {id} has no title"))?;
        let url =
            optional(detail.entry.url).ok_or_else(|| format!("Entry {id} has no original URL"))?;
        let byline = optional(detail.entry.author)
            .or_else(|| {
                content
                    .as_ref()
                    .and_then(|c| optional(c.readability_byline.clone()))
            })
            .or_else(|| optional(detail.feed_title))
            .unwrap_or_default();
        let body = if options.include_body {
            content
                .as_ref()
                .filter(|c| {
                    c.display_mode != "empty" && crate::reader::pipeline::has_current_markdown(c)
                })
                .and_then(|c| optional(c.markdown.clone()))
        } else {
            None
        };
        let note = if options.include_note {
            note_store::get_note(pool, id)
                .await?
                .and_then(|n| optional(Some(n.markdown)))
        } else {
            None
        };
        let summary_record = if options.include_summary {
            agent_result_store::latest_summary(pool, id).await?
        } else {
            None
        };
        let summary = summary_record
            .as_ref()
            .filter(|r| r.result.complete)
            .and_then(|r| optional(r.result.text.clone()));
        for (wanted, present, field) in [
            (options.include_body, body.is_some(), "body"),
            (options.include_note, note.is_some(), "note"),
            (options.include_summary, summary.is_some(), "summary"),
        ] {
            if wanted && !present {
                missing.push(DigestMissing {
                    entry_id: id,
                    title: title.clone(),
                    field: field.into(),
                });
            }
        }
        let transform = |s: String| if markdown { s } else { plain_text(&s) };
        entries.push(DigestEntryData {
            title: metadata(&title, markdown),
            byline: metadata(&byline, markdown),
            url: metadata(&url, markdown),
            content: body.map(transform),
            note: note.map(transform),
            summary: summary.map(transform),
            summary_attribution: summary_record.map(|r| {
                let attribution = if options.locale.as_deref() == Some("zh-Hans") {
                    let language = match r.target_language.as_str() {
                        "zh" | "zh-Hans" => "简体中文",
                        "en" => "英语",
                        "ja" => "日语",
                        "ko" => "韩语",
                        "fr" => "法语",
                        "de" => "德语",
                        "es" => "西班牙语",
                        other => other,
                    };
                    let detail = match r.detail_level.as_str() {
                        "brief" => "简短",
                        "medium" => "中等",
                        "detailed" => "详细",
                        other => other,
                    };
                    format!(
                        "由 {} 生成 · {}（{}）",
                        r.result.model_name, language, detail
                    )
                } else {
                    format!(
                        "Generated by {} in {} ({})",
                        r.result.model_name, r.target_language, r.detail_level
                    )
                };
                metadata(&attribution, markdown)
            }),
        });
    }
    Ok(DigestPreview {
        content: render_digest(name, entries, options.locale.as_deref())?,
        format: format.into(),
        missing,
    })
}

/// Refuse stale previews so database changes cannot silently alter the export.
pub async fn export_preview(
    pool: &Pool,
    ids: &[i64],
    name: &str,
    options: &DigestOptions,
    path: &str,
    expected: Option<&str>,
) -> Result<(), String> {
    let preview = generate_preview(pool, ids, name, options).await?;
    if expected.is_some_and(|text| text != preview.content) {
        return Err("Digest changed; refresh the preview before exporting".into());
    }
    tokio::fs::write(path, preview.content.as_bytes())
        .await
        .map_err(|e| format!("Write error: {e}"))
}

#[cfg(test)]
#[path = "m4w_007_tests.rs"]
mod tests;
