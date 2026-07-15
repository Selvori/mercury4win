// mercury4win-linux/src-tauri/lib.rs

use tauri::Manager;

pub mod commands;
pub mod db;
pub mod feed;
pub mod reader;
pub mod agent;
pub mod digest;
pub mod utils;

use crate::db::connection;
use crate::commands::feed_cmd::AppClient;
use crate::agent::runtime::create_runtime;
use crate::utils::paths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialize database
            let db_path = paths::database_path(&app.handle());
            let pool = tauri::async_runtime::block_on(connection::create_pool(
                db_path.to_str().unwrap_or("mercury.db"),
            ))
            .expect("failed to create database pool");

            // Run migrations
            {
                let conn = tauri::async_runtime::block_on(pool.get())
                    .expect("failed to get db connection");
                tauri::async_runtime::block_on(conn.interact(|conn| {
                    crate::db::migrations::run_migrations(conn)
                }))
                .expect("interact failed")
                .expect("migration failed");
            }

            let _ = app.manage(pool);
            let _ = app.manage(AppClient::new());
            let _ = app.manage(create_runtime());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Feed
            commands::feed_cmd::load_feeds,
            commands::feed_cmd::add_feed,
            commands::feed_cmd::update_feed,
            commands::feed_cmd::delete_feed,
            commands::feed_cmd::sync_feed,
            commands::feed_cmd::sync_all_feeds,
            commands::feed_cmd::import_opml,
            commands::feed_cmd::export_opml,
            // Entry
            commands::entry_cmd::load_entries,
            commands::entry_cmd::mark_read,
            commands::entry_cmd::mark_starred,
            commands::entry_cmd::delete_entry,
            commands::entry_cmd::mark_all_read_in_feed,
            commands::entry_cmd::load_entry_detail,
            // Tags
            commands::tag_cmd::list_tags,
            commands::tag_cmd::add_tag,
            commands::tag_cmd::remove_tag,
            commands::tag_cmd::merge_tags,
            commands::tag_cmd::rename_tag,
            commands::tag_cmd::delete_tag,
            commands::tag_cmd::batch_tag,
            // Reader
            commands::reader_cmd::build_reader_content,
            commands::reader_cmd::get_cached_reader_content,
            commands::reader_cmd::get_reader_themes,
            // Agent
            commands::agent_cmd::list_providers,
            commands::agent_cmd::add_provider,
            commands::agent_cmd::delete_provider,
            commands::agent_cmd::test_provider_connection,
            commands::agent_cmd::run_summary,
            commands::agent_cmd::run_translation,
            commands::agent_cmd::run_tagging,
            commands::agent_cmd::cancel_agent_task,
            commands::agent_cmd::get_agent_status,
            // Digest
            commands::digest_cmd::generate_digest,
            commands::digest_cmd::export_digest,
            commands::digest_cmd::export_multi_digest,
            // Settings
            commands::settings_cmd::get_setting,
            commands::settings_cmd::set_setting,
            commands::settings_cmd::get_usage_report,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
