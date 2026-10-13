//! Static, deliberately narrow Obsidian Publish resolution. Never run shell JS.
use comrak::{
    format_commonmark,
    nodes::{AstNode, NodeLink, NodeValue},
    parse_document, Arena, Options,
};
use markup5ever_rcdom::NodeData;
use reqwest::Client;
use std::rc::Rc;
use url::Url;

use super::html_to_md::web_url;

fn publish_host(host: &str) -> bool {
    host == "publish.obsidian.md"
        || host
            .strip_prefix("publish-")
            .and_then(|s| s.strip_suffix(".obsidian.md"))
            .is_some_and(|n| !n.is_empty() && n.len() <= 3 && n.bytes().all(|b| b.is_ascii_digit()))
}

fn endpoint(url: &Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return false;
    }
    #[cfg(test)]
    if url.scheme() == "http" && url.host_str() == Some("127.0.0.1") {
        return true;
    }
    url.scheme() == "https" && url.port().is_none() && url.host_str().is_some_and(publish_host)
}

fn access_parts(url: &Url) -> Option<(String, String)> {
    if !endpoint(url) {
        return None;
    }
    let parts: Vec<_> = url.path_segments()?.collect();
    if parts.len() < 3
        || parts[0] != "access"
        || parts[1].is_empty()
        || !parts[1]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || parts[2..]
            .iter()
            .any(|p| p.is_empty() || *p == "." || *p == "..")
    {
        return None;
    }
    Some((parts[1].to_string(), parts[2..].join("/")))
}

pub(super) fn markdown_url(value: &str) -> Option<Url> {
    let url = web_url(value)?;
    let (_, path) = access_parts(&url)?;
    path.to_ascii_lowercase().ends_with(".md").then_some(url)
}

// Extract a JSON value immediately after an exact assignment; expressions,
// template literals and dynamically computed URLs are outside this contract.
fn assignment<'a>(script: &'a str, name: &str) -> Option<&'a str> {
    let bytes = script.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"//") {
            i += 2;
            while i < bytes.len() && !matches!(bytes[i], b'\r' | b'\n') {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            while i < bytes.len() && !bytes[i..].starts_with(b"*/") {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else if matches!(bytes[i], b'\'' | b'"' | b'`') {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == quote {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i..].starts_with(name.as_bytes())
            && (i == 0
                || !matches!(bytes[i-1], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'$' | b'.'))
        {
            return Some(&script[i + name.len()..]);
        } else {
            i += 1;
        }
    }
    None
}

fn assigned_json(script: &str, name: &str) -> Option<serde_json::Value> {
    let tail = assignment(script, name)?
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let mut stream = serde_json::Deserializer::from_str(tail).into_iter();
    let value = stream.next()?.ok()?;
    (tail[stream.byte_offset()..].trim_start().starts_with(';')).then_some(value)
}

fn preload_literal(script: &str) -> Option<String> {
    let tail = assignment(script, "window.preloadPage")?
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let tail = tail
        .strip_prefix("fetch")
        .or_else(|| tail.strip_prefix('f'))?
        .trim_start()
        .strip_prefix('(')?
        .trim_start();
    // JSON double quotes support escaped slash and Unicode without evaluating JS.
    if tail.starts_with('"') {
        let mut stream = serde_json::Deserializer::from_str(tail).into_iter::<String>();
        let value = stream.next()?.ok()?;
        return matches!(
            tail[stream.byte_offset()..].trim_start().chars().next(),
            Some(')' | ',')
        )
        .then_some(value);
    }
    let tail = tail.strip_prefix('\'')?;
    let (value, rest) = tail.split_once('\'')?;
    (!value.contains('\\') && matches!(rest.trim_start().chars().next(), Some(')' | ',')))
        .then(|| value.to_string())
}

pub(super) fn resolve(html: &str, response_url: &str) -> Option<Url> {
    let tree = htmd::HtmlToMarkdown::new().html_to_tree(html).ok()?;
    fn collect(node: &Rc<htmd::Node>, base: &mut bool, scripts: &mut Vec<String>) {
        if let NodeData::Element { name, attrs, .. } = &node.data {
            if name.local.as_ref() == "base" {
                *base |= attrs.borrow().iter().any(|a| {
                    a.name.local.as_ref() == "href"
                        && web_url(a.value.trim())
                            .is_some_and(|u| u.as_str() == "https://publish.obsidian.md/")
                });
            }
            if name.local.as_ref() == "script"
                && !attrs
                    .borrow()
                    .iter()
                    .any(|a| a.name.local.as_ref() == "src")
            {
                scripts.push(
                    node.children
                        .borrow()
                        .iter()
                        .filter_map(|c| {
                            if let NodeData::Text { contents } = &c.data {
                                Some(contents.borrow().to_string())
                            } else {
                                None
                            }
                        })
                        .collect(),
                );
            }
        }
        for child in node.children.borrow().iter() {
            collect(child, base, scripts);
        }
    }
    let mut base = false;
    let mut scripts = Vec::new();
    collect(&tree, &mut base, &mut scripts);
    if !base {
        return None;
    }
    let site = scripts
        .iter()
        .find_map(|s| assigned_json(s, "window.siteInfo"))?;
    let uid = site.get("uid")?.as_str()?;
    let host = site.get("host")?.as_str()?;
    let reference = web_url(response_url)?;
    let literal = scripts.iter().find_map(|s| preload_literal(s))?;
    let url = reference.join(&literal).ok()?;
    let url = markdown_url(url.as_str())?;
    let (url_uid, _) = access_parts(&url)?;
    (uid == url_uid && host == url.host_str()?).then_some(url)
}

async fn fetch(client: &Client, url: &Url, accept: &str) -> Result<(String, Url), String> {
    let response = client
        .get(url.clone())
        .header("Accept", accept)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Obsidian HTTP error: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Obsidian HTTP error: {}", response.status()));
    }
    let final_url = response.url().clone();
    if !endpoint(&final_url) {
        return Err("Obsidian response redirected outside Publish".into());
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if content_type.contains("text/html") || content_type.contains("application/xhtml") {
        return Err("Obsidian response is HTML, not article data".into());
    }
    if accept.starts_with("text/") && content_type.contains("application/json") {
        return Err("Obsidian Markdown response is JSON".into());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Obsidian read error: {e}"))?;
    let body = String::from_utf8(bytes.to_vec()).map_err(|_| "Obsidian response is not UTF-8")?;
    let trimmed = body.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty()
        || trimmed.to_ascii_lowercase().starts_with("<!doctype html")
        || trimmed.to_ascii_lowercase().starts_with("<html")
        || trimmed.contains("_cf_chl_opt")
        || trimmed.contains("challenge-platform")
    {
        return Err("Obsidian response is empty or an HTML shell/challenge".into());
    }
    Ok((body.trim_start_matches('\u{feff}').to_string(), final_url))
}

fn resource_url(root: &Url, path: &str) -> Option<Url> {
    if path
        .split('/')
        .any(|p| p.is_empty() || matches!(p, "." | "..") || p.contains('\\'))
    {
        return None;
    }
    let mut url = root.clone();
    url.path_segments_mut()
        .ok()?
        .pop_if_empty()
        .extend(path.split('/'));
    Some(url)
}

fn media(target: &str) -> Option<bool> {
    let ext = target.rsplit_once('.')?.1.to_ascii_lowercase();
    if [
        "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "tif", "tiff", "avif", "heic", "heif",
    ]
    .contains(&ext.as_str())
    {
        Some(true)
    } else if [
        "mp4", "webm", "mov", "m4v", "mp3", "m4a", "wav", "ogg", "pdf",
    ]
    .contains(&ext.as_str())
    {
        Some(false)
    } else {
        None
    }
}

fn match_resource<'a>(target: &str, keys: &'a [String]) -> Option<&'a str> {
    if let Some(key) = keys.iter().find(|k| k.as_str() == target) {
        return Some(key);
    }
    let matches: Vec<_> = keys
        .iter()
        .filter(|k| k.ends_with(&format!("/{target}")))
        .collect();
    if matches.len() == 1 {
        return Some(matches[0]);
    }
    let basename = target.rsplit('/').next()?;
    let matches: Vec<_> = keys
        .iter()
        .filter(|k| {
            k.rsplit('/')
                .next()
                .is_some_and(|n| n.eq_ignore_ascii_case(basename))
        })
        .collect();
    (matches.len() == 1).then(|| matches[0].as_str())
}

fn decode_path(value: &str) -> Option<String> {
    let mut decoded = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let a = (*bytes.get(i + 1)? as char).to_digit(16)?;
            let b = (*bytes.get(i + 2)? as char).to_digit(16)?;
            decoded.push((a * 16 + b) as u8);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

pub(super) async fn build_markdown(_client: &Client, url: &Url) -> Result<String, String> {
    let original_uid = access_parts(url).ok_or("Invalid Obsidian Markdown URL")?.0;
    // A shared article client may follow unrestricted redirects. This branch
    // must reject an out-of-Publish hop before making that network request.
    let builder = Client::builder().redirect(reqwest::redirect::Policy::custom(move |attempt| {
        let target = attempt.url();
        let same_vault = access_parts(target).is_some_and(|p| p.0 == original_uid)
            || target.path() == format!("/cache/{original_uid}");
        if attempt.previous().len() >= 10 || !endpoint(target) || !same_vault {
            attempt.error("Obsidian redirect outside Publish vault")
        } else {
            attempt.follow()
        }
    }));
    #[cfg(test)]
    let builder = builder.no_proxy();
    let client = builder.build().map_err(|e| e.to_string())?;
    let (body, final_url) = fetch(&client, url, "text/markdown,text/plain;q=0.9").await?;
    let (uid, _) = access_parts(&final_url).ok_or("Invalid Obsidian Markdown redirect")?;
    if markdown_url(final_url.as_str()).is_none()
        || access_parts(url).map(|p| p.0) != Some(uid.clone())
    {
        return Err("Obsidian Markdown redirect changed vault or page type".into());
    }
    let mut root_url = final_url.clone();
    root_url.set_query(None);
    root_url.set_path(&format!("/access/{uid}/"));
    let mut index_url = root_url.clone();
    index_url.set_path(&format!("/cache/{uid}"));
    let has_embeds = {
        let arena = Arena::new();
        let document = parse_document(&arena, &body, &Options::default());
        document
            .descendants()
            .any(|n| matches!(&n.data.borrow().value, NodeValue::Text(t) if t.contains("![[")))
    };
    let keys: Vec<String> = if has_embeds {
        match fetch(&client, &index_url, "application/json").await {
            Ok((json, final_index)) if final_index.path() == index_url.path() => {
                serde_json::from_str::<serde_json::Value>(&json)
                    .ok()
                    .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
                    .unwrap_or_default()
            }
            _ => Vec::new(), // Preserve unresolved syntax rather than guessing assets.
        }
    } else {
        Vec::new()
    };
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    let document = parse_document(&arena, &body, &options);
    let nodes: Vec<_> = document.descendants().collect();
    if !nodes.iter().any(|n| match &n.data.borrow().value {
        NodeValue::Text(t) => !t.trim().is_empty(),
        NodeValue::Code(c) => !c.literal.trim().is_empty(),
        NodeValue::CodeBlock(c) => !c.literal.trim().is_empty(),
        NodeValue::Image(i) => !i.url.is_empty(),
        _ => false,
    }) {
        return Err("Obsidian Markdown has no readable content".into());
    }
    for node in nodes {
        let is_image = matches!(node.data.borrow().value, NodeValue::Image(_));
        let mut data = node.data.borrow_mut();
        match &mut data.value {
            NodeValue::Link(link) | NodeValue::Image(link) => {
                // Images and links both allow web URLs; renderer suppresses raw HTML.
                if link.url.starts_with('#') && !is_image {
                    continue;
                }
                link.url = final_url
                    .join(&link.url)
                    .ok()
                    .filter(|u| {
                        matches!(u.scheme(), "http" | "https")
                            || (!is_image && u.scheme() == "mailto")
                    })
                    .map(|u| u.to_string())
                    .unwrap_or_default();
            }
            NodeValue::Text(text) if !keys.is_empty() && text.contains("![[") => {
                let original = text.clone();
                drop(data);
                let mut tail = original.as_str();
                while let Some((before, rest)) = tail.split_once("![[") {
                    let Some((target_label, after)) = rest.split_once("]]") else {
                        break;
                    };
                    let (target, label) =
                        target_label.split_once('|').unwrap_or((target_label, ""));
                    let target = target
                        .split('#')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_matches('/')
                        .replace('\\', "/");
                    let decoded = decode_path(&target).unwrap_or_else(|| target.clone());
                    let replacement = media(&decoded).and_then(|image| {
                        match_resource(&target, &keys)
                            .or_else(|| match_resource(&decoded, &keys))
                            .and_then(|p| resource_url(&root_url, p))
                            .map(|u| (image, u))
                    });
                    node.insert_before(
                        arena.alloc(AstNode::from(NodeValue::Text(before.to_string()))),
                    );
                    if let Some((image, url)) = replacement {
                        let label = if label.trim().is_empty() {
                            target.rsplit('/').next().unwrap_or(&target)
                        } else {
                            label.trim()
                        };
                        let link = NodeLink {
                            url: url.to_string(),
                            title: String::new(),
                        };
                        let value = if image {
                            NodeValue::Image(link)
                        } else {
                            NodeValue::Link(link)
                        };
                        let new = arena.alloc(AstNode::from(value));
                        new.append(arena.alloc(AstNode::from(NodeValue::Text(label.to_string()))));
                        node.insert_before(new);
                    } else {
                        node.insert_before(arena.alloc(AstNode::from(NodeValue::Text(format!(
                            "![[{target_label}]]"
                        )))));
                    }
                    tail = after;
                }
                node.data.borrow_mut().value = NodeValue::Text(tail.to_string());
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    format_commonmark(document, &options, &mut out).map_err(|e| e.to_string())?;
    String::from_utf8(out).map_err(|e| e.to_string())
}
