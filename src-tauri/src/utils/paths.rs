// mercury4win-linux/src-tauri/utils/paths.rs
// Tauri Path API wrapper - no hardcoded paths anywhere

use std::path::PathBuf;
use tauri::Manager;

/// Get the app data directory for persistent storage.
pub fn app_data_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("failed to resolve app data dir")
}

/// Get the app config directory for settings.
pub fn app_config_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .expect("failed to resolve app config dir")
}

/// Get the database path inside app data dir.
pub fn database_path(app: &tauri::AppHandle) -> PathBuf {
    app_data_dir(app).join("mercury.db")
}

/// Get the prompts directory.
pub fn prompts_dir(app: &tauri::AppHandle) -> PathBuf {
    app_data_dir(app).join("prompts")
}

/// Get the digest templates directory.
pub fn digest_templates_dir(app: &tauri::AppHandle) -> PathBuf {
    app_data_dir(app).join("digest_templates")
}

/// Get the export directory (defaults to Documents).
pub fn default_export_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .document_dir()
        .unwrap_or_else(|_| app_data_dir(app))
}
