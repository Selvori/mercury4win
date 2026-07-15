// mercury4win-linux/src-tauri/db/content_store.rs
// Content pipeline persistence

use deadpool_sqlite::Pool;
use rusqlite::params;
use crate::db::models::Content;

pub async fn get_content(pool: &Pool, entry_id: i64) -> Result<Option<Content>, String> {
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            let result = conn.query_row(
                "SELECT entry_id, html, cleaned_html, readability_title, readability_byline,
                        readability_version, markdown, markdown_version, display_mode,
                        document_base_url, pipeline_type, resolved_intermediate_content
                 FROM content WHERE entry_id = ?1",
                params![entry_id],
                |row| {
                    Ok(Content {
                        entry_id: row.get(0)?,
                        html: row.get(1)?,
                        cleaned_html: row.get(2)?,
                        readability_title: row.get(3)?,
                        readability_byline: row.get(4)?,
                        readability_version: row.get(5)?,
                        markdown: row.get(6)?,
                        markdown_version: row.get(7)?,
                        display_mode: row.get(8)?,
                        document_base_url: row.get(9)?,
                        pipeline_type: row.get(10)?,
                        resolved_intermediate_content: row.get(11)?,
                    })
                },
            );

            match result {
                Ok(c) => Ok(Some(c)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        })
        .await
        .map_err(|e| e.to_string())?
}

pub async fn upsert_content(pool: &Pool, c: &Content) -> Result<(), String> {
    let entry_id = c.entry_id;
    let html = c.html.clone();
    let cleaned_html = c.cleaned_html.clone();
    let readability_title = c.readability_title.clone();
    let readability_byline = c.readability_byline.clone();
    let readability_version = c.readability_version;
    let markdown = c.markdown.clone();
    let markdown_version = c.markdown_version;
    let display_mode = c.display_mode.clone();
    let document_base_url = c.document_base_url.clone();
    let pipeline_type = c.pipeline_type.clone();
    let resolved = c.resolved_intermediate_content.clone();
    let pool = pool.clone();

    pool.get()
        .await
        .map_err(|e| e.to_string())?
        .interact(move |conn| {
            conn.execute(
                "INSERT INTO content (entry_id, html, cleaned_html, readability_title,
                 readability_byline, readability_version, markdown, markdown_version,
                 display_mode, document_base_url, pipeline_type, resolved_intermediate_content)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
                 ON CONFLICT(entry_id) DO UPDATE SET
                 html=excluded.html, cleaned_html=excluded.cleaned_html,
                 readability_title=excluded.readability_title,
                 readability_byline=excluded.readability_byline,
                 readability_version=excluded.readability_version,
                 markdown=excluded.markdown, markdown_version=excluded.markdown_version,
                 display_mode=excluded.display_mode, document_base_url=excluded.document_base_url,
                 pipeline_type=excluded.pipeline_type,
                 resolved_intermediate_content=excluded.resolved_intermediate_content",
                rusqlite::params![
                    entry_id, html, cleaned_html, readability_title,
                    readability_byline, readability_version, markdown, markdown_version,
                    display_mode, document_base_url, pipeline_type, resolved,
                ],
            )
            .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}
