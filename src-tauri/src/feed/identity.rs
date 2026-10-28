use crate::db::entry_store::RawEntryData;
use std::collections::HashSet;
use url::Url;

/// Compare HTTP/HTTPS and www variants, preserving path case and query semantics.
pub fn url_key(value: &str) -> Option<String> {
    let mut url = Url::parse(value.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    url.set_host(Some(&host)).ok()?;
    url.set_fragment(None);
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    let userinfo = if !url.username().is_empty() || url.password().is_some() {
        let password = url.password().map(|p| format!(":{p}")).unwrap_or_default();
        format!("{}{password}@", url.username())
    } else {
        String::new()
    };
    let path = url.path().trim_end_matches('/');
    let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
    Some(format!("{userinfo}{host}{port}{path}{query}"))
}

pub fn article_urls(entries: &[RawEntryData]) -> HashSet<String> {
    entries
        .iter()
        .filter_map(|e| e.url.as_deref().and_then(url_key))
        .collect()
}

/// A shared title alone, or a shared site alone, is not a feed identity.
/// Require at least three matching articles and 80% of the smaller window.
pub fn same_articles(left: &HashSet<String>, right: &HashSet<String>) -> bool {
    let smaller = left.len().min(right.len());
    let shared = left.intersection(right).count();
    shared >= 3 && shared * 5 >= smaller * 4
}
