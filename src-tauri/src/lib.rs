// mercury4win/src-tauri/lib.rs

use tauri::Manager;

pub mod agent;
pub mod commands;
pub mod db;
pub mod digest;
pub mod feed;
pub mod reader;
pub mod utils;

use crate::agent::runtime::create_runtime;
use crate::commands::feed_cmd::AppClient;
use crate::db::connection;
use crate::utils::paths;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // Always enable logging — writes to app data dir
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;

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
                tauri::async_runtime::block_on(
                    conn.interact(|conn| crate::db::migrations::run_migrations(conn)),
                )
                .expect("interact failed")
                .expect("migration failed");
                tauri::async_runtime::block_on(conn.interact(|conn| conn.execute("UPDATE topic_run SET status='interrupted',error='Application stopped before completion',finished_at=datetime('now') WHERE status='running'", [])))
                    .expect("interact failed").expect("topic recovery failed");
            }

            let _ = app.manage(pool);
            let _ = app.manage(AppClient::new());
            let _ = app.manage(create_runtime());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::topic_cmd::list_topics,
            commands::topic_cmd::topic_status,
            commands::topic_cmd::topic_model_info,
            commands::topic_cmd::save_topic,
            commands::topic_cmd::delete_topic,
            commands::topic_cmd::load_topic,
            commands::topic_cmd::update_topic_members,
            commands::topic_cmd::save_topic_notes,
            commands::topic_cmd::topic_feedback,
            commands::topic_cmd::mark_topic_read,
            commands::topic_cmd::generate_topic_brief,
            commands::topic_cmd::export_topic_brief,
            // Feed
            commands::feed_cmd::load_feeds,
            commands::feed_cmd::add_feed,
            commands::feed_cmd::update_feed,
            commands::feed_cmd::delete_feed,
            commands::feed_cmd::sync_feed,
            commands::feed_cmd::sync_all_feeds,
            commands::feed_cmd::sync_all_feeds_stream,
            commands::feed_cmd::import_opml,
            commands::feed_cmd::import_opml_stream,
            commands::feed_cmd::export_opml,
            // Entry
            commands::entry_cmd::load_entries,
            commands::entry_cmd::mark_read,
            commands::entry_cmd::mark_starred,
            commands::entry_cmd::delete_entry,
            commands::entry_cmd::mark_all_read_in_feed,
            commands::entry_cmd::mark_all_read_in_scope,
            commands::entry_cmd::load_entry_detail,
            commands::search_cmd::list_smart_views,
            commands::search_cmd::save_smart_view,
            commands::search_cmd::delete_smart_view,
            commands::search_cmd::rebuild_search_index,
            // Tags
            commands::tag_cmd::list_tags,
            commands::tag_cmd::add_tag,
            commands::tag_cmd::remove_tag,
            commands::tag_cmd::merge_tags,
            commands::tag_cmd::rename_tag,
            commands::tag_cmd::delete_tag,
            commands::tag_cmd::batch_tag,
            commands::tag_cmd::list_tag_aliases,
            commands::tag_cmd::add_tag_alias,
            commands::tag_cmd::delete_tag_alias,
            commands::tag_cmd::promote_tag,
            commands::tag_cmd::cleanup_unused_tags,
            commands::tag_cmd::local_tag_suggestions,
            commands::tag_cmd::related_entries,
            commands::tag_cmd::count_tag_batch,
            commands::tag_cmd::load_tag_batch,
            commands::tag_cmd::apply_tag_batch,
            commands::tag_cmd::discard_tag_batch,
            // Reader
            commands::reader_cmd::build_reader_content,
            commands::reader_cmd::get_cached_reader_content,
            commands::reader_cmd::get_reader_themes,
            // Agent
            commands::agent_cmd::list_providers,
            commands::agent_cmd::add_provider,
            commands::agent_cmd::update_provider,
            commands::agent_cmd::delete_provider,
            commands::agent_cmd::test_provider_connection,
            commands::agent_cmd::test_provider_config,
            commands::agent_cmd::run_summary,
            commands::agent_cmd::load_summary_result,
            commands::agent_cmd::load_latest_summary,
            commands::agent_cmd::get_agent_fingerprint,
            commands::agent_cmd::load_translation_result,
            commands::agent_cmd::load_translation_document,
            commands::agent_cmd::run_translation,
            commands::agent_cmd::run_tagging,
            commands::agent_cmd::cancel_agent_task,
            commands::agent_cmd::abandon_agent_waiting,
            commands::agent_cmd::get_agent_status,
            commands::agent_cmd::list_models,
            commands::agent_cmd::add_model,
            commands::agent_cmd::update_model,
            commands::agent_cmd::delete_model,
            commands::agent_cmd::set_configuration_enabled,
            commands::agent_cmd::get_agent_profile,
            commands::agent_cmd::update_agent_profile,
            commands::agent_cmd::save_custom_template,
            commands::agent_cmd::load_custom_template,
            // Digest
            commands::digest_cmd::preview_digest,
            commands::digest_cmd::generate_digest,
            commands::digest_cmd::export_digest,
            commands::digest_cmd::export_multi_digest,
            // Settings
            commands::settings_cmd::get_setting,
            commands::settings_cmd::set_setting,
            commands::settings_cmd::get_usage_report,
            // Notes
            commands::note_cmd::get_note,
            commands::note_cmd::save_note,
            commands::note_cmd::delete_note,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
