//! Explicit opt-in public-feed diagnostic. Never reads an application database.
use crate::db::{entry_store, feed_store, migrations};
use crate::feed::{opml_import, parser};
use deadpool_sqlite::{Config, Runtime};
use futures_util::{stream, StreamExt};
use serde_json::json;
use std::{path::Path, time::Duration};

#[tokio::test]
#[ignore = "requires explicit OPML_DIAGNOSTIC_INPUT and OPML_DIAGNOSTIC_OUTPUT; public network"]
async fn public_opml_diagnostic() {
    let input = std::env::var("OPML_DIAGNOSTIC_INPUT").unwrap();
    let output = std::env::var("OPML_DIAGNOSTIC_OUTPUT").unwrap();
    let feeds = opml_import::parse_opml_file(Path::new(&input)).unwrap();
    let client = crate::commands::feed_cmd::AppClient::new().0;
    let rows: Vec<_> = stream::iter(feeds.into_iter().enumerate())
        .map(|(index, feed)| {
            let client = client.clone();
            let output = output.clone();
            async move {
                let started = std::time::Instant::now();
                let mut row = json!({"index":index+1,"title":feed.title,"url":feed.feed_url});
                let outcome = async {
                    let response = client
                        .get(&feed.feed_url)
                        .header("User-Agent", "Mercury/0.1 (RSS Reader)")
                        .send()
                        .await
                        .map_err(|e| format!("{e:?}"))?;
                    row["status"] = json!(response.status().as_u16());
                    row["final_url"] = json!(response.url().as_str());
                    row["content_type"] = json!(response
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok()));
                    let status = response.status();
                    let bytes = response.bytes().await.map_err(|e| format!("{e:?}"))?;
                    row["bytes"] = json!(bytes.len());
                    // Public response samples stay in ignored tmp/, not audit records.
                    let snapshot = Path::new(&output)
                        .parent()
                        .unwrap()
                        .join(format!("response-{:02}.bin", index + 1));
                    std::fs::write(snapshot, &bytes).map_err(|e| e.to_string())?;
                    if !status.is_success() {
                        return Err(format!("HTTP {status}"));
                    }
                    let parsed = parser::parse_feed_bytes(&bytes, Some(&feed.feed_url))?;
                    let count = parsed.entries.len();
                    row["parsed_entries"] = json!(count);
                    row["parsed_title"] = json!(parsed.title);
                    row["latest_published"] = json!(parsed
                        .entries
                        .iter()
                        .filter_map(|e| e.published_at.as_ref())
                        .max());
                    let pool = Config::new(":memory:")
                        .builder(Runtime::Tokio1)
                        .unwrap()
                        .max_size(1)
                        .build()
                        .unwrap();
                    pool.get()
                        .await
                        .unwrap()
                        .interact(|c| migrations::run_migrations(c))
                        .await
                        .unwrap()
                        .map_err(|e| e.to_string())?;
                    let (stored, _) =
                        feed_store::insert_feed(&pool, &feed.feed_url, Some(&feed.title), None)
                            .await?;
                    let (new, updated) =
                        entry_store::bulk_upsert_entries(&pool, stored.id, parsed.entries).await?;
                    let listing = entry_store::query_entries(
                        &pool,
                        &entry_store::EntryQueryParams {
                            filter: entry_store::EntryFilter {
                                feed_id: Some(stored.id),
                                ..Default::default()
                            },
                            cursor: None,
                            limit: 100,
                        },
                    )
                    .await?;
                    row["inserted"] = json!(new);
                    row["updated"] = json!(updated);
                    row["listed"] = json!(listing.entries.len());
                    assert_eq!(new, count, "entry loss in {}", feed.feed_url);
                    assert_eq!(listing.entries.len(), count.min(100));
                    Ok::<(), String>(())
                };
                match tokio::time::timeout(Duration::from_secs(40), outcome).await {
                    Ok(Ok(())) => row["outcome"] = json!("ok"),
                    Ok(Err(error)) => {
                        row["outcome"] = json!("error");
                        row["error"] = json!(error);
                    }
                    Err(_) => {
                        row["outcome"] = json!("timeout");
                        row["error"] = json!("diagnostic 40-second bound for raw AppClient probe");
                    }
                }
                row["elapsed_ms"] = json!(started.elapsed().as_millis());
                println!("FEED_DIAGNOSTIC {row}");
                row
            }
        })
        .buffered(4)
        .collect()
        .await;
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&json!({"at_utc":chrono::Utc::now().to_rfc3339(),"rows":rows}))
            .unwrap(),
    )
    .unwrap();
}
