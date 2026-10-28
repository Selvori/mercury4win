use super::{fetch, opml_import::OpmlImportEntry};
use crate::db::{
    feed_store,
    models::{ImportProgress, ImportResult},
};
use deadpool_sqlite::Pool;
use std::time::Duration;

pub async fn import_entries(
    pool: &Pool,
    client: &reqwest::Client,
    entries: Vec<OpmlImportEntry>,
    mut report: impl FnMut(ImportProgress),
) -> Result<ImportResult, String> {
    import_with_timeout(pool, client, entries, &mut report, fetch::FEED_TIMEOUT).await
}

pub(super) async fn import_with_timeout(
    pool: &Pool,
    client: &reqwest::Client,
    entries: Vec<OpmlImportEntry>,
    report: &mut impl FnMut(ImportProgress),
    timeout: Duration,
) -> Result<ImportResult, String> {
    let total = entries.len();
    report(ImportProgress {
        status: "start".into(),
        feed_url: String::new(),
        title: None,
        error: None,
        current: 0,
        total,
    });
    let mut result = ImportResult {
        feeds_added: 0,
        feeds_skipped: 0,
        errors: Vec::new(),
    };
    for (idx, entry) in entries.iter().enumerate() {
        let progress = |status: &str, error| ImportProgress {
            status: status.into(),
            feed_url: entry.feed_url.clone(),
            title: Some(entry.title.clone()),
            error,
            current: idx + 1,
            total,
        };
        report(progress("processing", None));
        let outcome = import_one(
            pool,
            client,
            entry,
            || report(progress("retrying", None)),
            timeout,
        )
        .await;
        match outcome {
            Ok((_, true)) => {
                result.feeds_added += 1;
                report(progress("added", None));
            }
            Ok((_, false)) => {
                result.feeds_skipped += 1;
                report(progress("skipped", Some("duplicate_feed".into())));
            }
            Err(error) => {
                result.errors.push(format!("{}: {error}", entry.feed_url));
                report(progress("error", Some(error)));
            }
        }
    }
    report(ImportProgress {
        status: "done".into(),
        feed_url: String::new(),
        title: None,
        error: None,
        current: total,
        total,
    });
    Ok(result)
}

pub(super) async fn import_one(
    pool: &Pool,
    client: &reqwest::Client,
    entry: &OpmlImportEntry,
    mut on_retry: impl FnMut(),
    timeout: Duration,
) -> Result<(i64, bool), String> {
    let existing = feed_store::find_import_url(pool, &entry.feed_url).await?;
    if let Some((id, true)) = existing {
        return Ok((id, false));
    }
    let mut fetched = Err("Feed fetch not attempted".to_string());
    for attempt in 0..2 {
        fetched = fetch::fetch_feed_with_timeout(client, &entry.feed_url, timeout)
            .await
            .and_then(|(parsed, resolved)| {
                if parsed.entries.is_empty() {
                    Err("Feed contains no articles".into())
                } else {
                    Ok((parsed, resolved))
                }
            });
        if fetched.is_ok() {
            break;
        }
        if attempt == 0 {
            on_retry();
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
    let (mut parsed, resolved) = match fetched {
        Ok(value) => value,
        Err(error) => {
            if let Some((id, false)) = existing {
                feed_store::remove_failed_import(pool, id).await?;
            }
            return Err(error);
        }
    };
    if parsed.site_url.is_none() {
        parsed.site_url = entry.site_url.clone();
    }
    match feed_store::save_imported_feed(pool, &entry.feed_url, &resolved, &entry.title, parsed)
        .await
    {
        Ok(saved) => Ok(saved),
        Err(error) => {
            if let Some((id, false)) = existing {
                feed_store::remove_failed_import(pool, id).await?;
            }
            Err(error)
        }
    }
}
