// mercury4win/src-tauri/commands/feed_cmd.rs
// Feed CRUD Tauri commands

use crate::db::feed_store;
use crate::db::models::{Feed, FeedWithCounts, ImportProgress, ImportResult, SyncResult};
use crate::feed::opml_import;
use crate::feed::use_cases;
use deadpool_sqlite::Pool;
use tauri::{ipc::Channel, State};

pub struct AppClient(pub reqwest::Client);

impl AppClient {
    pub fn new() -> Self {
        Self(reqwest::Client::new())
    }
}

#[tauri::command]
pub async fn load_feeds(pool: State<'_, Pool>) -> Result<Vec<FeedWithCounts>, String> {
    feed_store::list_feeds(&pool).await
}

#[tauri::command]
pub async fn add_feed(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    url: String,
) -> Result<Feed, String> {
    use_cases::subscribe(&pool, &client.0, &url).await
}

#[tauri::command]
pub async fn update_feed(
    pool: State<'_, Pool>,
    id: i64,
    title: Option<String>,
) -> Result<(), String> {
    feed_store::update_feed_meta(&pool, id, title.as_deref(), None, None).await
}

#[tauri::command]
pub async fn delete_feed(pool: State<'_, Pool>, id: i64) -> Result<(), String> {
    feed_store::delete_feed(&pool, id).await
}

#[tauri::command]
pub async fn sync_feed(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    id: i64,
) -> Result<SyncResult, String> {
    use_cases::refresh_single(&pool, &client.0, id).await
}

#[tauri::command]
pub async fn sync_all_feeds(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
) -> Result<Vec<SyncResult>, String> {
    use_cases::refresh_all(&pool, &client.0).await
}

/// Refresh all feeds while reporting per-feed progress to the UI.
#[tauri::command]
pub async fn sync_all_feeds_stream(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    on_progress: Channel<ImportProgress>,
) -> Result<(), String> {
    let feeds = feed_store::list_feeds(&pool).await?;
    let total = feeds.len();
    let _ = on_progress.send(ImportProgress {
        status: "start".into(),
        feed_url: String::new(),
        title: None,
        error: None,
        current: 0,
        total,
    });

    for (idx, feed) in feeds.iter().enumerate() {
        let current = idx + 1;
        let _ = on_progress.send(ImportProgress {
            status: "processing".into(),
            feed_url: feed.feed_url.clone(),
            title: feed.title.clone(),
            error: None,
            current,
            total,
        });

        match use_cases::refresh_single(&pool, &client.0, feed.id).await {
            Ok(result) if result.error.is_none() => {
                let _ = on_progress.send(ImportProgress {
                    status: "completed".into(),
                    feed_url: feed.feed_url.clone(),
                    title: feed.title.clone(),
                    error: None,
                    current,
                    total,
                });
            }
            Ok(result) => {
                let _ = on_progress.send(ImportProgress {
                    status: "error".into(),
                    feed_url: feed.feed_url.clone(),
                    title: feed.title.clone(),
                    error: result.error,
                    current,
                    total,
                });
            }
            Err(error) => {
                let _ = on_progress.send(ImportProgress {
                    status: "error".into(),
                    feed_url: feed.feed_url.clone(),
                    title: feed.title.clone(),
                    error: Some(error),
                    current,
                    total,
                });
            }
        }
    }

    let _ = on_progress.send(ImportProgress {
        status: "done".into(),
        feed_url: String::new(),
        title: None,
        error: None,
        current: total,
        total,
    });
    Ok(())
}

#[tauri::command]
pub async fn import_opml(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    file_path: String,
) -> Result<ImportResult, String> {
    let path = std::path::Path::new(&file_path);
    let entries = opml_import::parse_opml_file(path)?;

    let mut added = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    for entry in entries {
        match feed_store::insert_feed(
            &pool,
            &entry.feed_url,
            Some(&entry.title),
            entry.site_url.as_deref(),
        )
        .await
        {
            Ok((feed, was_inserted)) => {
                if was_inserted {
                    // New feed: try initial sync
                    if let Err(e) = use_cases::refresh_single(&pool, &client.0, feed.id).await {
                        errors.push(format!("{}: {}", entry.feed_url, e));
                    }
                    added += 1;
                } else {
                    // Duplicate feed: skip
                    skipped += 1;
                }
            }
            Err(e) => {
                // Database error
                errors.push(format!("{}: {}", entry.feed_url, e));
                skipped += 1;
            }
        }
    }

    Ok(ImportResult {
        feeds_added: added,
        feeds_skipped: skipped,
        errors,
    })
}

#[tauri::command]
pub async fn export_opml(
    pool: State<'_, Pool>,
    file_path: String,
    feed_ids: Option<Vec<i64>>,
) -> Result<(), String> {
    let feeds = feed_store::list_feeds(&pool).await?;

    let feeds_to_export: Vec<&FeedWithCounts> = match feed_ids {
        Some(ref ids) => feeds.iter().filter(|f| ids.contains(&f.id)).collect(),
        None => feeds.iter().collect(),
    };

    let feed_data: Vec<crate::feed::opml_export::OpmlFeedData> = feeds_to_export
        .iter()
        .map(|f| crate::feed::opml_export::OpmlFeedData {
            title: f.title.clone().unwrap_or_else(|| "Untitled".into()),
            feed_url: f.feed_url.clone(),
            site_url: f.site_url.clone(),
        })
        .collect();

    crate::feed::opml_export::export_opml_file(std::path::Path::new(&file_path), &feed_data, &[])
}

/// Streaming OPML import — sends per-feed progress events via Channel.
#[tauri::command]
pub async fn import_opml_stream(
    pool: State<'_, Pool>,
    client: State<'_, AppClient>,
    file_path: String,
    on_progress: Channel<ImportProgress>,
) -> Result<(), String> {
    let path = std::path::Path::new(&file_path);
    let entries = opml_import::parse_opml_file(path)?;
    let total = entries.len();

    let _ = on_progress.send(ImportProgress {
        status: "start".into(),
        feed_url: String::new(),
        title: None,
        error: None,
        current: 0,
        total,
    });

    for (idx, entry) in entries.iter().enumerate() {
        let current = idx + 1;

        // Send "processing" event
        let _ = on_progress.send(ImportProgress {
            status: "processing".into(),
            feed_url: entry.feed_url.clone(),
            title: Some(entry.title.clone()),
            error: None,
            current,
            total,
        });

        // Try to insert + sync
        match feed_store::insert_feed(
            &pool,
            &entry.feed_url,
            Some(&entry.title),
            entry.site_url.as_deref(),
        )
        .await
        {
            Ok((feed, was_inserted)) => {
                if was_inserted {
                    // New feed: try initial sync
                    match use_cases::refresh_single(&pool, &client.0, feed.id).await {
                        Ok(_) => {
                            let _ = on_progress.send(ImportProgress {
                                status: "added".into(),
                                feed_url: entry.feed_url.clone(),
                                title: Some(entry.title.clone()),
                                error: None,
                                current,
                                total,
                            });
                        }
                        Err(e) => {
                            // Added but sync failed
                            let _ = on_progress.send(ImportProgress {
                                status: "error".into(),
                                feed_url: entry.feed_url.clone(),
                                title: Some(entry.title.clone()),
                                error: Some(format!("Added but initial sync failed: {}", e)),
                                current,
                                total,
                            });
                        }
                    }
                } else {
                    // Duplicate feed: skip
                    let _ = on_progress.send(ImportProgress {
                        status: "skipped".into(),
                        feed_url: entry.feed_url.clone(),
                        title: Some(entry.title.clone()),
                        error: Some("Feed already exists".into()),
                        current,
                        total,
                    });
                }
            }
            Err(e) => {
                // Database error
                let _ = on_progress.send(ImportProgress {
                    status: "error".into(),
                    feed_url: entry.feed_url.clone(),
                    title: Some(entry.title.clone()),
                    error: Some(format!("Database error: {}", e)),
                    current,
                    total,
                });
            }
        }
    }

    // Final "done" event
    let _ = on_progress.send(ImportProgress {
        status: "done".into(),
        feed_url: String::new(),
        title: None,
        error: None,
        current: total,
        total,
    });

    Ok(())
}
