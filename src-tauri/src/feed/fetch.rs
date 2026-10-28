use super::parser::{parse_feed_bytes, ParsedFeed};
use std::time::Duration;

pub const FEED_TIMEOUT: Duration = Duration::from_secs(30);

/// Bound only feed requests; the shared client also serves long AI requests.
pub async fn fetch_feed(
    client: &reqwest::Client,
    url: &str,
) -> Result<(ParsedFeed, String), String> {
    fetch_feed_with_timeout(client, url, FEED_TIMEOUT).await
}

pub(super) async fn fetch_feed_with_timeout(
    client: &reqwest::Client,
    url: &str,
    timeout: Duration,
) -> Result<(ParsedFeed, String), String> {
    let response = client
        .get(url)
        .header("User-Agent", "Mercury/0.1 (RSS Reader)")
        .timeout(timeout)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Feed HTTP error: {}", response.status()));
    }
    let final_url = response.url().to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Read error: {e}"))?;
    if bytes.is_empty() {
        return Err("Empty response body".into());
    }
    let parsed = parse_feed_bytes(&bytes, Some(&final_url))?;
    Ok((parsed, final_url))
}
