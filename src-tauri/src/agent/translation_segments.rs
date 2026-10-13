//! Segment the same safe rendered HTML that the reader displays. An outer list
//! owns its descendant paragraphs/lists. Headings, quotes and table cells are
//! translated too; code blocks and non-content regions remain original.
use markup5ever_rcdom::NodeData;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::rc::Rc;

use crate::db::agent_result_store::{self, AgentResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceSegment {
    pub source_segment_id: String,
    pub order_index: usize,
    pub segment_type: String,
    pub source_html: String,
    pub source_text: String,
}

#[derive(Serialize)]
pub struct TranslationDocument {
    pub source_hash: String,
    pub segmenter_version: String,
    pub html: String,
    pub source_segments: Vec<SourceSegment>,
    pub result: Option<AgentResult>,
}

fn tag(node: &Rc<htmd::Node>) -> &str {
    match &node.data {
        NodeData::Element { name, .. } => name.local.as_ref(),
        _ => "",
    }
}

fn source_text(node: &Rc<htmd::Node>) -> String {
    if matches!(tag(node), "pre" | "script" | "style") {
        return String::new();
    }
    if let NodeData::Text { contents } = &node.data {
        return contents.borrow().to_string();
    }
    let body = node
        .children
        .borrow()
        .iter()
        .map(source_text)
        .collect::<String>();
    match tag(node) {
        "code" => format!("`{body}`"),
        "li" => format!("- {}\n", body.trim()),
        "p" | "br" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => format!("{body}\n"),
        _ => body,
    }
}

fn serialize(node: &Rc<htmd::Node>, marker: Option<&str>, children: String) -> String {
    match &node.data {
        NodeData::Text { contents } => {
            html_escape::encode_text(contents.borrow().as_ref()).into_owned()
        }
        NodeData::Element { name, attrs, .. } => {
            let tag = name.local.as_ref();
            // Styles come from the trusted reader renderer; HTML character
            // references are not decoded in a raw-text style element.
            let children = if tag == "style" {
                node.children
                    .borrow()
                    .iter()
                    .filter_map(|child| {
                        if let NodeData::Text { contents } = &child.data {
                            Some(contents.borrow().to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<String>()
            } else {
                children
            };
            let mut start = format!("<{tag}");
            for attr in attrs.borrow().iter() {
                start.push_str(&format!(
                    " {}=\"{}\"",
                    attr.name.local,
                    html_escape::encode_double_quoted_attribute(&attr.value)
                ));
            }
            if let Some(id) = marker {
                start.push_str(&format!(" data-source-segment-id=\"{id}\""));
            }
            start.push('>');
            if matches!(
                tag,
                "area"
                    | "base"
                    | "br"
                    | "col"
                    | "embed"
                    | "hr"
                    | "img"
                    | "input"
                    | "link"
                    | "meta"
                    | "param"
                    | "source"
                    | "track"
                    | "wbr"
            ) {
                start
            } else {
                format!("{start}{children}</{tag}>")
            }
        }
        _ => children,
    }
}

fn plain_html(node: &Rc<htmd::Node>) -> String {
    serialize(
        node,
        None,
        node.children.borrow().iter().map(plain_html).collect(),
    )
}

pub fn extract_html(html: &str) -> Result<(String, Vec<SourceSegment>), String> {
    let tree = htmd::HtmlToMarkdown::new()
        .html_to_tree(html)
        .map_err(|e| e.to_string())?;
    fn visit(
        node: &Rc<htmd::Node>,
        in_reader: bool,
        excluded: bool,
        segments: &mut Vec<SourceSegment>,
    ) -> String {
        let name = tag(node);
        let in_reader = in_reader || name == "article";
        let excluded =
            excluded || matches!(name, "pre" | "code" | "script" | "style" | "nav" | "aside");
        if in_reader
            && !excluded
            && matches!(
                name,
                "p" | "ul"
                    | "ol"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "th"
                    | "td"
                    | "caption"
                    | "figcaption"
                    | "dt"
                    | "dd"
            )
        {
            let text = source_text(node).trim().to_string();
            if !text.is_empty() {
                let html = plain_html(node);
                let index = segments.len();
                let identity = serde_json::json!([name, index, html, text]).to_string();
                let id = format!(
                    "seg_{index}_{}",
                    &format!("{:x}", Sha256::digest(identity))[..12]
                );
                segments.push(SourceSegment {
                    source_segment_id: id.clone(),
                    order_index: index,
                    segment_type: name.into(),
                    source_html: html,
                    source_text: text,
                });
                // Never traverse a selected list/paragraph a second time.
                return serialize(
                    node,
                    Some(&id),
                    node.children.borrow().iter().map(plain_html).collect(),
                );
            }
        }
        let children = node
            .children
            .borrow()
            .iter()
            .map(|n| visit(n, in_reader, excluded, segments))
            .collect();
        serialize(node, None, children)
    }
    let mut segments = Vec::new();
    let html = visit(&tree, false, false, &mut segments);
    Ok((html, segments))
}

pub async fn load_document(
    pool: &deadpool_sqlite::Pool,
    entry_id: i64,
    language: &str,
    strategy: Option<&str>,
    concurrency: usize,
) -> Result<TranslationDocument, String> {
    let source = agent_result_store::source_snapshot(pool, entry_id).await?;
    let html = crate::reader::md_to_html::render_markdown(&source.markdown, "classic-light");
    let (html, source_segments) = extract_html(&html)?;
    let ids: Vec<_> = source_segments
        .iter()
        .map(|s| s.source_segment_id.clone())
        .collect();
    let result =
        agent_result_store::load_translation(pool, entry_id, language, strategy, concurrency)
            .await?
            .filter(|r| {
                r.source_hash == source.hash
                    && r.segment_ids == ids
                    && r.segments.len() == ids.len()
            });
    Ok(TranslationDocument {
        source_hash: source.hash,
        segmenter_version: agent_result_store::TRANSLATION_SEGMENTER.into(),
        html,
        source_segments,
        result,
    })
}

#[cfg(test)]
#[path = "translation_tests.rs"]
mod tests;
